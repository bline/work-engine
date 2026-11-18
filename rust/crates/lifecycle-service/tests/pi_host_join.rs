// Compiled inside the actor's unit boundary (as `native_input_capture.rs` is)
// so the fixture drives the actual private actor and the Pi Host runner (Q).
// Offline: Pi runs in the Host's pinned container on the `claude-sdk`
// provider; Claude Code's requests leave through the Host to the scripted
// loopback Messages observer; no inference is used.
use super::*;
use crate::{
    GrantFence, NativeConfig, NativeProviderRoute, PrincipalConfig, QuestionOpenPayload,
    ResultPublishPayload, TaskCursorBinding, TaskInputBinding, TaskOperationCommit,
    TaskOperationError, ValidatedTaskOperationEntry,
};
use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn sha_file(path: &Path) -> String {
    sha(&std::fs::read(path).unwrap())
}
fn art(path: &Path) -> Value {
    json!({"path":path,"sha256":sha_file(path)})
}
/// The Host's inventory format: sorted `{path: {kind: file, sha256}}`.
fn inventory(root: &Path) -> Value {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Value>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.insert(rel, json!({"kind":"file","sha256":sha_file(&path)}));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    serde_json::to_value(out).unwrap()
}
fn write_json(path: &Path, value: &Value) -> PathBuf {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
    path.to_path_buf()
}

fn config(root: &Path, host: &Path, prompt: &str) -> ServiceConfig {
    let now = clock_sample().wall.get();
    let grants = [
        ("grant:turn", "native_turn"),
        ("grant:snapshot", "snapshot_read"),
        ("grant:input", "enqueue_input"),
        ("grant:result", "task_result_publish"),
    ]
    .map(|(id, scope)| crate::GrantConfig {
        grant_ref: id.into(),
        context_generation: "context:input".into(),
        scope: scope.into(),
        expires_wall_ms: now + 900_000,
        revision: 1,
    })
    .to_vec();
    ServiceConfig {
        profile: "retained-repository-analysis-v1".into(),
        store_root: root.join("store"),
        socket_path: root.join("service.sock"),
        peer_socket_path: root.join("unused.sock"),
        verifier_executable: None,
        verifier_ledger_path: None,
        proof_barrier_dir: None,
        trusted_issuer: "issuer:input".into(),
        principal: PrincipalConfig {
            uid: {
                use std::os::unix::fs::MetadataExt;
                std::fs::metadata(root).unwrap().uid()
            },
            principal_ref: "principal:input".into(),
            producer_ref: "producer:input".into(),
        },
        subject_id: "subject:input".into(),
        context_generation: "context:input".into(),
        build_id: "build:input".into(),
        proof_run_id: "proof:input".into(),
        grants,
        max_executions: 1,
        max_wait_ms: 1000,
        verifier_proof_ms: None,
        verifier_cleanup_ms: None,
        selected_restart: None,
        native: Some(NativeConfig {
            executable: host.to_path_buf(),
            root: root.join("native"),
            provider_route: NativeProviderRoute::PiHostV1,
            chatgpt_access_source: None,
            simulator_base_url: String::new(),
            session_id: "session:input".into(),
            invocation_id: "invocation:unused".into(),
            attempt_id: "attempt:unused".into(),
            prompt: prompt.into(),
            turn_grant_ref: "grant:turn".into(),
            turn_grant_revision: 1,
            snapshot_grant_ref: "grant:snapshot".into(),
            snapshot_grant_revision: 1,
            same_native_session_required: false,
            followup_turns: Vec::new(),
            snapshot: workspace_snapshot_read::TrustedSnapshot {
                id: "snapshot:input".into(),
                source_revision: "synthetic:input".into(),
                members: vec![workspace_snapshot_read::TrustedMember {
                    id: "member".into(),
                    sha256: CodecContract::BinaryArtifactV1
                        .digest_binary(b"synthetic")
                        .unwrap()
                        .hex(),
                    bytes: b"synthetic".to_vec(),
                }],
            },
            semantic: Some(serde_json::from_value(json!({"capture_id":"capture:input","compile_session_id":"session:compile","verify_session_id":"session:verify","successor_verify_session_id":"session:successor","compile_invocation_id":"invocation:compile","compile_attempt_id":"attempt:compile","candidate_verify_invocation_id":"invocation:verify","candidate_verify_attempt_id":"attempt:verify","successor_verify_invocation_id":"invocation:successor","successor_verify_attempt_id":"attempt:successor","challenge_id":"challenge:input","challenge_text":"synthetic","control_grant_ref":"grant:control","control_grant_revision":1,"successor_control_grant_ref":"grant:next","successor_control_grant_revision":1})).unwrap()),
            semantic_contexts: Vec::new(),
        }),
    }
}

/// The admitted Host job for one relay thread over the observer's loopback
/// routes (as `records/dev-vertical-pi-host-realization-20261118/serve.py`
/// prepares it), its serve configuration and `pi-host.json`.
fn prepare_host_job(native: &Path, host: &Path, ports: (u64, u64)) {
    let env = |k: &str| PathBuf::from(std::env::var(k).unwrap());
    let runtime_dir = env("WE_PI_HOST_RUNTIME_DIR");
    let receiver = env("WE_PI_HOST_RECEIVER_DIR");
    let inputs = native.join("inputs");
    let state = native.join("pi-host-state");
    for d in [
        &inputs,
        &state,
        &inputs.join("workspace"),
        &inputs.join("pi-config"),
    ] {
        std::fs::create_dir_all(d).unwrap();
    }
    write_json(
        &inputs.join("workspace/snapshot-identity.json"),
        &json!({"snapshot_id":"snapshot:input"}),
    );
    let ws_manifest = write_json(
        &inputs.join("workspace-manifest.json"),
        &inventory(&inputs.join("workspace")),
    );
    let cfg_manifest = write_json(
        &inputs.join("pi-config-manifest.json"),
        &inventory(&inputs.join("pi-config")),
    );
    let task = inputs.join("task.txt");
    std::fs::write(
        &task,
        "Pi Host lifecycle relay thread: prompts arrive from the controller.",
    )
    .unwrap();
    let shim = inputs.join("pi-coding.mjs");
    std::fs::copy(receiver.join("pi-coding.mjs"), &shim).unwrap();
    let adapter = inputs.join("pi-native-tools.mjs");
    std::fs::copy(receiver.join("pi-native-tools.mjs"), &adapter).unwrap();
    let runtime_manifest = inputs.join("runtime-manifest.json");
    std::fs::copy(runtime_dir.join("runtime-manifest.json"), &runtime_manifest).unwrap();
    let runtime = runtime_dir.join("root");
    let catalog =
        runtime.join("node_modules/@earendil-works/pi-ai/dist/providers/data/anthropic.json");
    let routes = json!([
        {"id":"primary","payer":"synthetic-subscription","endpoint":format!("loopback:{}",ports.0)},
        {"id":"secondary","payer":"synthetic-api","endpoint":format!("loopback:{}",ports.1)}]);
    let routes_digest = sha(
        json!({"schema":"work-engine.pi-hosted-routes.v1","routes":routes,
        "max_attempts_per_request":2})
        .to_string()
        .as_bytes(),
    );
    let (max_requests, budget) = (8u64, 8 * 1_320_001u64);
    let job_id = "lifecycle-service-pi-host-join";
    let authority = write_json(
        &inputs.join("authority.json"),
        &json!({
        "schema":"work-engine.pi-coding-runtime-admission.v1","job_id":job_id,
        "binary_sha256":sha_file(host),"model":"claude-haiku-5-5","provider":"claude-sdk",
        "pi_config_manifest_sha256":sha_file(&cfg_manifest),"task_sha256":sha_file(&task),
        "catalog_sha256":sha_file(&catalog),"max_requests":max_requests,"budget_microusd":budget,
        "allow_provider":true,"hosted_routes_sha256":routes_digest}),
    );
    let packet = write_json(
        &inputs.join("packet.json"),
        &json!({"schema":"work-engine.pi-host-serve-packet.v1"}),
    );
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 900;
    let job = write_json(
        &native.join("job.json"),
        &json!({
        "schema":"work-engine.pi-docker-custody.v1","job_id":job_id,"state_dir":state,
        "authority":art(&authority),"packet":art(&packet),"docker":art(Path::new("/usr/bin/docker")),
        "image":"node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da",
        "runtime":{"root":runtime,"manifest":art(&runtime_manifest)},
        "workspace":{"root":inputs.join("workspace"),"manifest":art(&ws_manifest)},
        "deadline_unix_seconds":deadline,"command_timeout_seconds":30,"idle_seconds":0,
        "coding":{"schema":"work-engine.pi-lifecycle-hosted.v1","shim":art(&shim),"max_tool_calls":16,
            "provider":{"lane":{"pi_config":{"root":inputs.join("pi-config"),"manifest":art(&cfg_manifest)}},
                "task":art(&task),"catalog":art(&catalog),"stream_adapter":art(&adapter),
                "hosted":{"routes":routes,"max_attempts_per_request":2},"provider":"claude-sdk",
                "model":"claude-haiku-5-5","max_requests":max_requests,"budget_microusd":budget,
                "max_context_bytes":400000}}}),
    );
    let serve = write_json(
        &native.join("serve-config.json"),
        &json!({
        "schema":"work-engine.pi-host-serve.v1","job":art(&job),"turn_deadline_secs":300}),
    );
    write_json(
        &native.join("pi-host.json"),
        &json!({
        "host_sha256":sha_file(host),"serve_config":serve,"serve_config_sha256":sha_file(&serve),
        "job_sha256":sha_file(&job),"state_dir":state,"provider":"claude-sdk",
        "model":"claude-haiku-5-5","turn_deadline_secs":300}),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires Docker, the Pi Host binary, its claude-sdk runtime and the Messages observer"]
async fn pi_host_causal_join() {
    use retained_task_workflow::{SubjectKind, SubmitTask, TaskStore};
    let root = PathBuf::from(std::env::var("WE_PI_HOST_OUT").unwrap());
    let host = PathBuf::from(std::env::var("WE_PI_HOST_BIN").unwrap());
    let observer = PathBuf::from(std::env::var("WE_PI_HOST_OBSERVER").unwrap());
    for d in ["", "store", "native"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root.join("native"), std::fs::Permissions::from_mode(0o700))
            .unwrap();
    }
    let owner =
        Arc::new(TaskStore::open(&root.join("tasks.sqlite"), "human:fixture".into()).unwrap());
    let question =
        "Which synthetic option matches basis-1? Publish the answer with result_publish.";
    owner
        .submit(&SubmitTask {
            command_id: "input".into(),
            task_id: "task:input".into(),
            subject_id: "subject:input".into(),
            kind: SubjectKind::RepositoryAnalysis,
            objective: question.into(),
            engineering_question: question.into(),
            snapshot_ref: "snapshot:input".into(),
            facts: vec![],
        })
        .unwrap();
    let text = owner.input_text("input").unwrap().unwrap();
    let record = owner.observe("task:input").unwrap().unwrap();
    let refs = record
        .sources
        .iter()
        .map(|s| s.reference())
        .collect::<Vec<_>>();
    let args = json!({"result_id":"result:pi-host","body":"pi host recommendation","source_refs":refs,"attribution":"native:claude-haiku-5-5"});
    let steps = json!([{"tool":"mcp__pi__result_publish","input":args},{"text":"published"}]);
    std::fs::write(root.join("steps.json"), steps.to_string()).unwrap();
    let mut fixture = std::process::Command::new(&observer)
        .env("HOSTED_OBSERVER_SCENARIO", "steps")
        .env("HOSTED_OBSERVER_STEPS", root.join("steps.json"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::fs::File::create(root.join("observer.stderr")).unwrap())
        .spawn()
        .unwrap();
    let mut line = String::new();
    let mut observer_out = std::io::BufReader::new(fixture.stdout.take().unwrap());
    observer_out.read_line(&mut line).unwrap();
    let ready: Value = serde_json::from_str(&line).unwrap();
    let ports = (
        ready["primary"].as_u64().unwrap(),
        ready["secondary"].as_u64().unwrap(),
    );
    prepare_host_job(&root.join("native"), &host, ports);
    let config = config(&root, &host, &text);
    let native = config.native.as_ref().unwrap();
    operations::pi_host::validate(native, &operations::pi_host::load(native).expect("load"))
        .expect("validate");
    operations::prepare_native_task_launch(native, config.principal.uid).expect("launch");
    let mut actor = Actor::new(
        config.clone(),
        Some(Arc::new(super::restart_epoch_tests::EmptyMaterial)),
        Some(Arc::new(Owner(owner.clone()))),
        None,
    )
    .unwrap();
    let db = rusqlite::Connection::open(root.join("store/lifecycle.sqlite")).unwrap();
    db.execute("INSERT INTO inputs(input_id,subject_id,sequence,delivery_id,producer_ref,text,text_digest,grant_id,state) VALUES('input:input','subject:input',1,'delivery:input','producer:input',?1,?2,'grant:input','queued')",rusqlite::params![text.as_bytes(),CodecContract::LifecycleTextInputV1.digest_binary(text.as_bytes()).unwrap().hex()]).unwrap();
    let launch = actor.native_launch.take().unwrap();
    assert!(launch.pi_host.is_some());
    let (work, pending) = mpsc::channel(4);
    actor.native_task_work = Some(work);
    let run = tokio::spawn(operations::pi_host::run_pi_host(
        config.native.clone().unwrap(),
        launch,
        "transport:pi-host".into(),
        actor.native_sender.clone(),
        actor.native_gate.clone(),
        Some(pending),
        None,
    ));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(600);
    let mut kinds = Vec::new();
    loop {
        let message = tokio::time::timeout_at(deadline, actor.native_messages.recv())
            .await
            .unwrap()
            .unwrap();
        let kind = match &message {
            NativeMessage::BindThread { .. } => "bind_thread",
            NativeMessage::PrepareTurn { .. } => "prepare_turn",
            NativeMessage::BindTurn { .. } => "bind_turn",
            NativeMessage::Ingress { .. } => "ingress",
            NativeMessage::Tool { .. } => "tool",
            NativeMessage::CaptureInput { .. } => "capture_codex",
            NativeMessage::CaptureHostInput { .. } => "capture_host",
            NativeMessage::Settle { .. } => "settle",
            NativeMessage::Done { .. } => "done",
        };
        kinds.push(kind);
        let reply = actor.handle_native_message(message).await;
        let _ = reply;
        if kind == "settle" {
            actor.native_task_work.take();
        }
        if kind == "done" {
            break;
        }
    }
    run.await.unwrap();
    // The Host closed its thread in order: no container is left behind.
    let containers = std::process::Command::new("docker")
        .args(["ps", "-a", "--format", "{{.Names}}"])
        .output()
        .unwrap();
    let job: Value = serde_json::from_slice(
        &std::fs::read(root.join("native/pi-host-state/pi-state.json")).unwrap(),
    )
    .unwrap();
    let name = job["container_name"].as_str().unwrap();
    assert!(
        !String::from_utf8_lossy(&containers.stdout).contains(name),
        "container {name} left behind"
    );
    assert!(
        job["failures"].as_array().unwrap().is_empty(),
        "{}",
        job["failures"]
    );
    let _ = fixture.kill();
    let _ = fixture.wait();
    std::fs::write(
        root.join("messages.json"),
        serde_json::to_vec(&kinds).unwrap(),
    )
    .unwrap();
    let result = owner
        .observe("task:input")
        .unwrap()
        .unwrap()
        .result
        .expect("result published through the Pi Host");
    let causal = actor
        .store
        .native_causal_evidence(&result.operation_id)
        .unwrap()
        .expect("causal record");
    assert_eq!(causal.scope, native_realization::pi_host::CAUSAL_SCOPE);
    let input = actor
        .store
        .native_input_observation(&causal.invocation_id)
        .unwrap()
        .unwrap();
    assert!(input.eligible, "{}", input.outcome);
    assert_eq!(
        input.protocol_digest,
        native_realization::pi_host::PROTOCOL_DIGEST
    );
    assert!(input.payloads.iter().all(|p| p.representation == "wire"));
    let page = actor.native_causal_read("task:input", "subject:input", 0, None);
    assert_eq!(page["operation_id"], result.operation_id, "{page}");
    let recovery = actor
        .store
        .native_recovery(&causal.invocation_id)
        .unwrap()
        .unwrap();
    assert_eq!(recovery.class, "CompletedSettled");
    assert!(kinds.contains(&"capture_host") && !kinds.contains(&"capture_codex"));
    std::fs::write(
        root.join("summary.json"),
        json!({"operation":result.operation_id,"scope":causal.scope,"eligible":input.eligible,
            "protocol":input.protocol_digest,"binary":input.binary_sha256,"payloads":input.payloads.len(),
            "recovery":recovery.class,"first_page_operation":page["operation_id"],"messages":kinds})
        .to_string(),
    )
    .unwrap();
}
// The unit-test crate and its library dependency have distinct trait
// identities; this adapter forwards to the real TaskStore.
struct Owner(Arc<retained_task_workflow::TaskStore>);
fn commit(c: retained_task_workflow::OperationCommit) -> TaskOperationCommit {
    TaskOperationCommit {
        operation_id: c.operation_id,
        task_id: c.task_id.clone(),
        owner_revision: c.owner_revision,
        owner_ref: c.task_id,
        sha256: c.digest,
        kind: c.kind,
    }
}
fn error(e: retained_task_workflow::TaskError) -> TaskOperationError {
    use retained_task_workflow::TaskError as E;
    match e {
        E::Stale | E::PendingQuestion => TaskOperationError::Stale,
        E::Unauthorized => TaskOperationError::Unauthorized,
        E::Busy => TaskOperationError::Busy,
        E::Invalid | E::Conflict => TaskOperationError::Invalid,
        _ => TaskOperationError::Uncertain,
    }
}
impl TaskOperationPort for Owner {
    fn original_result_operation(
        &self,
        t: &str,
        s: &str,
    ) -> Result<Option<String>, TaskOperationError> {
        let r = self
            .0
            .observe(t)
            .map_err(error)?
            .ok_or(TaskOperationError::Invalid)?;
        if r.subject_id != s {
            return Err(TaskOperationError::Invalid);
        }
        Ok(r.result.map(|r| r.operation_id))
    }
    fn lookup_completed_effect(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<crate::CompletedTaskEffectReadback>, TaskOperationError> {
        Ok(None)
    }
    fn record_completed_effect(
        &self,
        _: &crate::CompletedTaskEffect,
    ) -> Result<(), TaskOperationError> {
        Err(TaskOperationError::Invalid)
    }
    fn validate_input(&self, id: &str, text: &str) -> Result<TaskInputBinding, TaskOperationError> {
        let r = self.0.validate_input(id, text).map_err(error)?;
        Ok(TaskInputBinding {
            input_id: r.input_id,
            task_id: r.task_id,
            subject_id: r.subject_id,
            cursor_ref: r.cursor_ref,
            cursor_revision: r.cursor_revision,
            owner_revision: r.owner_revision,
            text_sha256: retained_task_workflow::sha256(text.as_bytes()),
        })
    }
    fn current_cursor(&self, t: &str) -> Result<TaskCursorBinding, TaskOperationError> {
        let r = self
            .0
            .observe(t)
            .map_err(error)?
            .ok_or(TaskOperationError::Invalid)?;
        Ok(TaskCursorBinding {
            task_id: r.task_id.clone(),
            subject_id: r.subject_id,
            cursor_ref: r.task_id,
            cursor_revision: r.cursor_revision,
            owner_revision: r.revision,
        })
    }
    fn open_question(
        &self,
        _: &ValidatedTaskOperationEntry,
        _: QuestionOpenPayload,
        _: &mut GrantFence<'_>,
    ) -> Result<TaskOperationCommit, TaskOperationError> {
        Err(TaskOperationError::Invalid)
    }
    fn publish_result(
        &self,
        e: &ValidatedTaskOperationEntry,
        p: ResultPublishPayload,
        f: &mut GrantFence<'_>,
    ) -> Result<TaskOperationCommit, TaskOperationError> {
        use retained_task_workflow::{OperationEntry, ResultPublishPayload as P, TaskError};
        if e.contract_id != "task.result.publish.v1" || e.cursor_ref != e.task_id {
            return Err(TaskOperationError::Invalid);
        }
        let entry = OperationEntry {
            operation_id: e.operation_id.clone(),
            task_id: e.task_id.clone(),
            subject_id: e.subject_id.clone(),
            delivery_id: e.logical_delivery_id.clone(),
            attempt_id: e.attempt_id.clone(),
            expected_task_revision: e.expected_task_revision,
            expected_cursor_revision: e.cursor_revision,
            grant_ref: e.grant_ref.clone(),
            grant_revision: e.grant_revision,
            payload_sha256: e.payload_sha256.clone(),
        };
        let p = P {
            result_id: p.result_id,
            body: p.body,
            source_refs: p.source_refs,
            attribution: p.attribution,
        };
        let mut fence = |commit: &mut dyn FnMut() -> Result<(), TaskError>| {
            f(&mut || commit().map_err(error)).map_err(|e| match e {
                TaskOperationError::Stale => TaskError::Stale,
                TaskOperationError::Unauthorized => TaskError::Unauthorized,
                TaskOperationError::Busy => TaskError::Busy,
                TaskOperationError::Invalid => TaskError::Invalid,
                TaskOperationError::Uncertain => TaskError::Uncertain,
            })
        };
        self.0
            .publish_result(&entry, &p, &mut fence)
            .map(commit)
            .map_err(error)
    }
    fn lookup_operation(
        &self,
        id: &str,
    ) -> Result<Option<TaskOperationCommit>, TaskOperationError> {
        self.0
            .lookup_operation(id)
            .map(|c| c.map(commit))
            .map_err(error)
    }
}
