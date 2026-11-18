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

/// The service configuration for one thread: composition-issued grants
/// expiring at `expires_wall_ms`, and the trusted snapshot.
fn config(
    root: &Path,
    host: &Path,
    prompt: &str,
    snapshot: workspace_snapshot_read::TrustedSnapshot,
    expires_wall_ms: i64,
) -> ServiceConfig {
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
        expires_wall_ms,
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
            snapshot,
            semantic: Some(serde_json::from_value(json!({"capture_id":"capture:input","compile_session_id":"session:compile","verify_session_id":"session:verify","successor_verify_session_id":"session:successor","compile_invocation_id":"invocation:compile","compile_attempt_id":"attempt:compile","candidate_verify_invocation_id":"invocation:verify","candidate_verify_attempt_id":"attempt:verify","successor_verify_invocation_id":"invocation:successor","successor_verify_attempt_id":"attempt:successor","challenge_id":"challenge:input","challenge_text":"synthetic","control_grant_ref":"grant:control","control_grant_revision":1,"successor_control_grant_ref":"grant:next","successor_control_grant_revision":1})).unwrap()),
            semantic_contexts: Vec::new(),
        }),
    }
}

/// The Host job parameters of one relay thread (identical across the arms
/// of one comparison).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct JobParams {
    routes: Value,
    max_attempts_per_request: u64,
    deadline_secs: u64,
    max_requests: u64,
    budget_microusd: u64,
    max_context_bytes: u64,
    max_tool_calls: u64,
    turn_deadline_secs: u64,
}
impl JobParams {
    /// The offline fixture's parameters; the routes are the observer's.
    fn offline() -> Self {
        Self {
            routes: json!([]),
            max_attempts_per_request: 2,
            deadline_secs: 900,
            max_requests: 8,
            budget_microusd: 8 * 1_320_001,
            max_context_bytes: 400_000,
            max_tool_calls: 16,
            turn_deadline_secs: 300,
        }
    }
}

/// The admitted Host job for one relay thread (as
/// `records/dev-vertical-pi-host-realization-20261118/serve.py` prepares
/// it), its serve configuration and `pi-host.json`; `deadline` in unix
/// seconds.
fn prepare_host_job(native: &Path, host: &Path, job: &JobParams, snapshot: &str, deadline: u64) {
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
        &json!({"snapshot_id":snapshot}),
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
    let (routes, attempts) = (&job.routes, job.max_attempts_per_request);
    let routes_digest = sha(
        json!({"schema":"work-engine.pi-hosted-routes.v1","routes":routes,
        "max_attempts_per_request":attempts})
        .to_string()
        .as_bytes(),
    );
    let (max_requests, budget) = (job.max_requests, job.budget_microusd);
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
    let (tool_calls, context, turn) = (
        job.max_tool_calls,
        job.max_context_bytes,
        job.turn_deadline_secs,
    );
    let job = write_json(
        &native.join("job.json"),
        &json!({
        "schema":"work-engine.pi-docker-custody.v1","job_id":job_id,"state_dir":state,
        "authority":art(&authority),"packet":art(&packet),"docker":art(Path::new("/usr/bin/docker")),
        "image":"node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da",
        "runtime":{"root":runtime,"manifest":art(&runtime_manifest)},
        "workspace":{"root":inputs.join("workspace"),"manifest":art(&ws_manifest)},
        "deadline_unix_seconds":deadline,"command_timeout_seconds":30,"idle_seconds":0,
        "coding":{"schema":"work-engine.pi-lifecycle-hosted.v1","shim":art(&shim),"max_tool_calls":tool_calls,
            "provider":{"lane":{"pi_config":{"root":inputs.join("pi-config"),"manifest":art(&cfg_manifest)}},
                "task":art(&task),"catalog":art(&catalog),"stream_adapter":art(&adapter),
                "hosted":{"routes":routes,"max_attempts_per_request":attempts},"provider":"claude-sdk",
                "model":"claude-haiku-5-5","max_requests":max_requests,"budget_microusd":budget,
                "max_context_bytes":context}}}),
    );
    let serve = write_json(
        &native.join("serve-config.json"),
        &json!({
        "schema":"work-engine.pi-host-serve.v1","job":art(&job),"turn_deadline_secs":turn}),
    );
    write_json(
        &native.join("pi-host.json"),
        &json!({
        "host_sha256":sha_file(host),"serve_config":serve,"serve_config_sha256":sha_file(&serve),
        "job_sha256":sha_file(&job),"state_dir":state,"provider":"claude-sdk",
        "model":"claude-haiku-5-5","turn_deadline_secs":turn}),
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires Docker, the Pi Host binary, its claude-sdk runtime and the Messages observer"]
async fn pi_host_causal_join() {
    join(None).await;
}

/// L2: the derived A4 research material (`request\ncard\nreconstruction`,
/// `WE_A4_FIXTURE`) is the snapshot member; Pi reads it through the Host and
/// the actor in 32 KiB `mcp__pi__snapshot_read` calls, then publishes a
/// recommendation. The actor's retained causal read and the owner selector
/// map are written for the task host's `verify_delivery` test
/// (`work-engine-task-host` `pi_host_q_delivery_verifies`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires Docker, the Pi Host binary, its claude-sdk runtime, the Messages observer and the A4 fixture"]
async fn pi_host_research_delivery() {
    join(Some(PathBuf::from(std::env::var("WE_A4_FIXTURE").unwrap()))).await;
}

const TASK: &str = "task:input";
const SUBJECT: &str = "subject:input";
const ARM_SCHEMA: &str = "work-engine.pi-host-arm.v1";

/// One live arm (`WE_PI_HOST_ARM`): the task's fields, the snapshot members
/// by path (each pinned by SHA-256) and the job parameters. `mapping` names,
/// for the advisory arm, the fact behind each recommendation dependency.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arm {
    schema: String,
    name: String,
    snapshot_id: String,
    source_revision: String,
    members: Vec<ArmMember>,
    kind: retained_task_workflow::SubjectKind,
    objective: String,
    engineering_question: String,
    facts: Vec<retained_task_workflow::FactInput>,
    #[serde(default)]
    mapping: BTreeMap<String, String>,
    job: JobParams,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArmMember {
    id: String,
    path: PathBuf,
    sha256: String,
}

/// H2 step 3f: one live arm on its own routes, with no observer. Refuses
/// to start unless `WORK_ENGINE_HOSTED_CREDENTIALS` names the credentials
/// file the Host inherits, and unless every route is `https:`. Keeps the
/// offline assertions except the scripted read count (recorded instead).
/// Writes `summary.json`, `grants.json` and, for an arm with a `mapping`,
/// `retained.json` and `mapping.json`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live inference (operator-authorized arms only): requires WE_PI_HOST_ARM, WORK_ENGINE_HOSTED_CREDENTIALS, Docker, the Pi Host binary and its claude-sdk runtime"]
async fn pi_host_live_arm() {
    let credentials = std::env::var_os("WORK_ENGINE_HOSTED_CREDENTIALS")
        .expect("refused: WORK_ENGINE_HOSTED_CREDENTIALS is not set");
    assert!(
        Path::new(&credentials).is_file(),
        "refused: the credentials file is absent"
    );
    let arm_path = PathBuf::from(std::env::var("WE_PI_HOST_ARM").unwrap());
    let arm_bytes = std::fs::read(&arm_path).unwrap();
    let arm: Arm = serde_json::from_slice(&arm_bytes).unwrap();
    assert_eq!(arm.schema, ARM_SCHEMA);
    let routes = arm.job.routes.as_array().cloned().unwrap_or_default();
    assert!(
        !routes.is_empty()
            && routes.iter().all(|r| r["endpoint"]
                .as_str()
                .is_some_and(|e| e.starts_with("https:"))),
        "refused: a live arm routes only to https endpoints"
    );
    let mut members = arm
        .members
        .iter()
        .map(|m| {
            let bytes = std::fs::read(&m.path).unwrap();
            assert_eq!(sha(&bytes), m.sha256, "member {} changed", m.id);
            (m.id.clone(), bytes)
        })
        .collect::<Vec<_>>();
    members.sort();
    let root = out_root();
    let owner = owner(
        &root,
        arm.kind,
        &arm.objective,
        &arm.engineering_question,
        &arm.snapshot_id,
        arm.facts.clone(),
    );
    let text = owner.input_text("input").unwrap().unwrap();
    let snapshot = snapshot(&arm.snapshot_id, &arm.source_revision, &members);
    let run = drive(&root, &owner, &text, snapshot, arm.job.clone(), None).await;
    let reads = run.kinds.iter().filter(|k| **k == "tool").count();
    let mut summary = json!({"arm":arm.name,"arm_sha256":sha(&arm_bytes),"task_id":TASK,
        "subject_id":SUBJECT,"relayed_calls":reads,"messages":run.kinds,
        "grants_sha256":sha_file(&root.join("grants.json")),"job":arm.job,
        "failures":run.failures,"container_left":run.container_left});
    // The observed outcome is retained before the assertions decide it.
    write_json(&root.join("summary.json"), &summary);
    run.assert_closed();
    let settled = settled(&run.actor, &owner);
    if !arm.mapping.is_empty() {
        let record = owner.observe(TASK).unwrap().unwrap();
        let mapping: BTreeMap<String, (String, _)> = arm
            .mapping
            .iter()
            .map(|(key, fact)| {
                let source = record.sources.iter().find(|s| &s.id == fact).unwrap();
                (key.clone(), (TASK.to_string(), source.reference()))
            })
            .collect();
        write_retained(&run.actor, &root, &mapping);
    }
    summary["settled"] = settled;
    write_json(&root.join("summary.json"), &summary);
}

fn out_root() -> PathBuf {
    let root = PathBuf::from(std::env::var("WE_PI_HOST_OUT").unwrap());
    for d in ["", "store", "native"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(root.join("native"), std::fs::Permissions::from_mode(0o700)).unwrap();
    root
}

/// The task owner's store with the submitted task (`input`).
fn owner(
    root: &Path,
    kind: retained_task_workflow::SubjectKind,
    objective: &str,
    question: &str,
    snapshot: &str,
    facts: Vec<retained_task_workflow::FactInput>,
) -> Arc<retained_task_workflow::TaskStore> {
    use retained_task_workflow::{SubmitTask, TaskStore};
    let owner =
        Arc::new(TaskStore::open(&root.join("tasks.sqlite"), "human:fixture".into()).unwrap());
    owner
        .submit(&SubmitTask {
            command_id: "input".into(),
            task_id: TASK.into(),
            subject_id: SUBJECT.into(),
            kind,
            objective: objective.into(),
            engineering_question: question.into(),
            snapshot_ref: snapshot.into(),
            facts,
        })
        .unwrap();
    owner
}

fn snapshot(
    id: &str,
    source_revision: &str,
    members: &[(String, Vec<u8>)],
) -> workspace_snapshot_read::TrustedSnapshot {
    workspace_snapshot_read::TrustedSnapshot {
        id: id.into(),
        source_revision: source_revision.into(),
        members: members
            .iter()
            .map(|(id, bytes)| workspace_snapshot_read::TrustedMember {
                id: id.clone(),
                sha256: CodecContract::BinaryArtifactV1
                    .digest_binary(bytes)
                    .unwrap()
                    .hex(),
                bytes: bytes.clone(),
            })
            .collect(),
    }
}

async fn join(research: Option<PathBuf>) {
    use retained_task_workflow::{FactInput, FactKind, SubjectKind};
    let research = research.map(|a4| {
        let read = |n: &str| std::fs::read(a4.join(n)).unwrap();
        let material = [
            read("request.json"),
            b"\n".to_vec(),
            read("card.json"),
            b"\n".to_vec(),
            read("reconstruction.json"),
        ]
        .concat();
        (material, read("target.txt"))
    });
    let root = out_root();
    let question =
        "Which synthetic option matches basis-1? Publish the answer with result_publish.";
    let facts = research
        .iter()
        .map(|(_, target)| FactInput {
            id: "target".into(),
            kind: FactKind::Decision,
            text: String::from_utf8(target.clone()).unwrap(),
            current: true,
        })
        .collect();
    let owner = owner(
        &root,
        SubjectKind::RepositoryAnalysis,
        question,
        question,
        "snapshot:input",
        facts,
    );
    let text = owner.input_text("input").unwrap().unwrap();
    let record = owner.observe(TASK).unwrap().unwrap();
    let refs = record
        .sources
        .iter()
        .map(|s| s.reference())
        .collect::<Vec<_>>();
    let (member, material, steps, mapping) = match &research {
        None => {
            let args = json!({"result_id":"result:pi-host","body":"pi host recommendation","source_refs":refs,"attribution":"native:claude-haiku-5-5"});
            let steps =
                json!([{"tool":"mcp__pi__result_publish","input":args},{"text":"published"}]);
            ("member", b"synthetic".to_vec(), steps, None)
        }
        Some((material, target)) => {
            let source = record
                .sources
                .iter()
                .find(|s| s.id == "target")
                .unwrap()
                .reference();
            let body = json!({"dependencies":{"decision.target":sha(target)},
                "text":"Unknown transfer; alternatives include defer. Synthetic evidence supports only a private audit; execution and approval are refused."});
            let args = json!({"result_id":"result:pi-host-research","body":body.to_string(),
                "source_refs":[source.clone()],"attribution":"native:claude-haiku-5-5"});
            let mut steps = (0..material.len())
                .step_by(32768)
                .map(|offset| json!({"tool":"mcp__pi__snapshot_read","input":{"snapshot_id":"snapshot:input",
                    "member_id":"research","offset":offset,"length":(material.len()-offset).min(32768)}}))
                .collect::<Vec<_>>();
            steps.push(json!({"tool":"mcp__pi__result_publish","input":args}));
            steps.push(json!({"text":"published"}));
            let mapping =
                BTreeMap::from([("decision.target".to_string(), (TASK.to_string(), source))]);
            ("research", material.clone(), json!(steps), Some(mapping))
        }
    };
    let snapshot = snapshot(
        "snapshot:input",
        "synthetic:input",
        &[(member.to_string(), material.clone())],
    );
    let run = drive(
        &root,
        &owner,
        &text,
        snapshot,
        JobParams::offline(),
        Some(&steps),
    )
    .await;
    run.assert_closed();
    let settled = settled(&run.actor, &owner);
    if let Some(mapping) = &mapping {
        write_retained(&run.actor, &root, mapping);
        let reads = run.kinds.iter().filter(|k| **k == "tool").count();
        assert_eq!(reads, material.len().div_ceil(32768) + 1);
    }
    let mut summary = settled;
    summary["messages"] = json!(run.kinds);
    summary["task_id"] = json!(TASK);
    summary["subject_id"] = json!(SUBJECT);
    std::fs::write(root.join("summary.json"), summary.to_string()).unwrap();
}

/// One thread driven to its end.
struct Run {
    actor: Actor,
    kinds: Vec<&'static str>,
    failures: Value,
    container_left: bool,
}
impl Run {
    /// The Host closed its thread in order: no container is left behind and
    /// it recorded no failure.
    fn assert_closed(&self) {
        assert!(!self.container_left, "container left behind");
        assert!(
            self.failures.as_array().is_some_and(Vec::is_empty),
            "{}",
            self.failures
        );
    }
}

/// Composes one relay thread over the actual actor, runner and Host and
/// drives it until the runner is done: offline with the scripted observer
/// (`steps`, which supplies the routes), live on the job's own routes.
async fn drive(
    root: &Path,
    owner: &Arc<retained_task_workflow::TaskStore>,
    text: &str,
    snapshot: workspace_snapshot_read::TrustedSnapshot,
    mut job: JobParams,
    steps: Option<&Value>,
) -> Run {
    let host = PathBuf::from(std::env::var("WE_PI_HOST_BIN").unwrap());
    // The observer and its stdout reader: the reader stays open for the
    // run, since the observer writes a line per request.
    let mut fixture = steps.map(|steps| {
        std::fs::write(root.join("steps.json"), steps.to_string()).unwrap();
        let observer = PathBuf::from(std::env::var("WE_PI_HOST_OBSERVER").unwrap());
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
        job.routes = json!([
            {"id":"primary","payer":"synthetic-subscription","endpoint":format!("loopback:{}",ready["primary"])},
            {"id":"secondary","payer":"synthetic-api","endpoint":format!("loopback:{}",ready["secondary"])}]);
        (fixture, observer_out)
    });
    let deadline = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + job.deadline_secs;
    prepare_host_job(&root.join("native"), &host, &job, &snapshot.id, deadline);
    // Composition-issued grants (O1), retained exactly with the run; they
    // expire with the job.
    let config = config(root, &host, text, snapshot, deadline as i64 * 1000);
    std::fs::write(
        root.join("grants.json"),
        serde_json::to_vec(&config.grants).unwrap(),
    )
    .unwrap();
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
    let until = tokio::time::Instant::now() + Duration::from_secs(job.deadline_secs + 60);
    let mut kinds = Vec::new();
    loop {
        let message = tokio::time::timeout_at(until, actor.native_messages.recv())
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
    let containers = std::process::Command::new("docker")
        .args(["ps", "-a", "--format", "{{.Names}}"])
        .output()
        .unwrap();
    let job: Value = serde_json::from_slice(
        &std::fs::read(root.join("native/pi-host-state/pi-state.json")).unwrap(),
    )
    .unwrap();
    let name = job["container_name"].as_str().unwrap();
    if let Some((fixture, _)) = fixture.as_mut() {
        let _ = fixture.kill();
        let _ = fixture.wait();
    }
    std::fs::write(
        root.join("messages.json"),
        serde_json::to_vec(&kinds).unwrap(),
    )
    .unwrap();
    Run {
        actor,
        kinds,
        failures: job["failures"].clone(),
        container_left: String::from_utf8_lossy(&containers.stdout).contains(name),
    }
}

/// The published result's causal record, eligible checked input under this
/// protocol and settled recovery; returns them for the summary.
fn settled(actor: &Actor, owner: &retained_task_workflow::TaskStore) -> Value {
    let result = owner
        .observe(TASK)
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
    let page = actor.native_causal_read(TASK, SUBJECT, 0, None);
    assert_eq!(page["operation_id"], result.operation_id, "{page}");
    let recovery = actor
        .store
        .native_recovery(&causal.invocation_id)
        .unwrap()
        .unwrap();
    assert_eq!(recovery.class, "CompletedSettled");
    json!({"operation":result.operation_id,"scope":causal.scope,"eligible":input.eligible,
        "protocol":input.protocol_digest,"binary":input.binary_sha256,"payloads":input.payloads.len(),
        "recovery":recovery.class,"first_page_operation":page["operation_id"]})
}

/// The actor's scoped causal read, paged as the task host's `acquire` pages
/// it over the service socket, and the owner selector map, for
/// `verify_delivery`.
fn write_retained(actor: &Actor, root: &Path, mapping: &impl Serialize) {
    use base64::Engine;
    let (mut bytes, mut digest) = (Vec::new(), None::<String>);
    loop {
        let chunk = actor.native_causal_read(TASK, SUBJECT, bytes.len(), digest.as_deref());
        let data = base64::engine::general_purpose::STANDARD
            .decode(chunk["bytes_base64"].as_str().expect(&chunk.to_string()))
            .unwrap();
        assert_eq!(chunk["offset"], bytes.len());
        digest = Some(chunk["content_sha256"].as_str().unwrap().into());
        bytes.extend(data);
        if bytes.len() == chunk["length"].as_u64().unwrap() as usize {
            break;
        }
    }
    assert_eq!(Some(sha(&bytes)), digest);
    std::fs::write(root.join("retained.json"), &bytes).unwrap();
    std::fs::write(
        root.join("mapping.json"),
        serde_json::to_vec(mapping).unwrap(),
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
    // Snapshot-read effects forward to the owner as `retained-task-workflow`'s
    // own port does (`port.rs`), so the actor's `snapshot_read` commits.
    fn lookup_completed_effect(
        &self,
        task_id: &str,
        effect_id: &str,
    ) -> Result<Option<crate::CompletedTaskEffectReadback>, TaskOperationError> {
        let Some(effect) = self.0.lookup_completed_effect(effect_id).map_err(error)? else {
            return Ok(None);
        };
        let record = self
            .0
            .observe(task_id)
            .map_err(error)?
            .ok_or(TaskOperationError::Uncertain)?;
        if !record.completed_effects.contains(&effect) {
            return Err(TaskOperationError::Invalid);
        }
        Ok(Some(crate::CompletedTaskEffectReadback {
            task_id: task_id.into(),
            logical_delivery_id: effect.delivery_id,
            effect_id: effect.effect_id,
            receipt_sha256: effect.receipt_digest,
            attribution: effect
                .attribution
                .map(|a| crate::CompletedTaskEffectAttribution {
                    parent_invocation_id: a.parent_invocation_id,
                    call_id: a.call_id,
                    contract_id: a.contract_id,
                    snapshot_id: a.snapshot_id,
                    member_id: a.member_id,
                    offset: a.offset,
                    requested_length: a.requested_length,
                    returned_length: a.returned_length,
                    result_sha256: a.result_sha256,
                }),
        }))
    }
    fn record_completed_effect(
        &self,
        effect: &crate::CompletedTaskEffect,
    ) -> Result<(), TaskOperationError> {
        use retained_task_workflow::{
            CompletedEffect, CompletedSnapshotReadAttribution, CompletedSnapshotResult as R,
            CompletedSnapshotUnavailable as U,
        };
        let result = match effect.result_bytes.as_deref() {
            None => R::Unavailable {
                reason: U::MissingResult,
            },
            Some(b)
                if b.len() as u64 != effect.returned_length || sha(b) != effect.receipt_sha256 =>
            {
                return Err(TaskOperationError::Invalid);
            }
            Some(b) => match std::str::from_utf8(b) {
                Err(_) => R::Unavailable { reason: U::NonUtf8 },
                Ok(_) if b.len() > 4 * 1024 => R::Unavailable {
                    reason: U::OverInlineLimit,
                },
                Ok(t) if serde_json::to_vec(t).unwrap().len() > 8 * 1024 => R::Unavailable {
                    reason: U::OverEscapedLimit,
                },
                Ok(t) => R::InlineUtf8 { text: t.into() },
            },
        };
        self.0
            .record_completed_effect(
                &effect.task_id,
                CompletedEffect {
                    effect_id: effect.effect_id.clone(),
                    delivery_id: effect.logical_delivery_id.clone(),
                    receipt_digest: effect.receipt_sha256.clone(),
                    attribution: Some(CompletedSnapshotReadAttribution {
                        schema: "completed-snapshot-read-attribution-v2".into(),
                        parent_invocation_id: effect.parent_invocation_id.clone(),
                        call_id: effect.call_id.clone(),
                        contract_id: effect.contract_id.clone(),
                        snapshot_id: effect.snapshot_id.clone(),
                        member_id: effect.member_id.clone(),
                        offset: effect.offset,
                        requested_length: effect.requested_length,
                        returned_length: effect.returned_length,
                        result_sha256: effect.receipt_sha256.clone(),
                        result,
                    }),
                },
                effect.expected_cursor_revision,
            )
            .map(|_| ())
            .map_err(error)
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
