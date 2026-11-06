//! Offline bridge tests. Synthetic Host process records are not Docker evidence.
use lifecycle_core::*;
use lifecycle_store::*;
use pi_pressure_context_host::coding::CodingLifecycleGate;
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use work_engine_manual_host::{
    Artifact, ProcessRecord, digest,
    pi_docker::coding::{CodingEffectGate, CodingToolIntent},
};
fn clock() -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(100),
        wait_budget: WaitBudgetMs::new(100),
    }
}
fn fixture() -> (PathBuf, CodingLifecycleGate, CodingToolIntent) {
    let root = std::env::temp_dir().join(format!(
        "coding-bridge-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut store = SqliteLifecycleStore::open(&root, "issuer".into()).unwrap();
    store
        .register_subject(
            SubjectId::parse("s").unwrap(),
            ContextGeneration::parse("c").unwrap(),
            BuildId::parse("b").unwrap(),
            ProofRunId::parse("p").unwrap(),
            clock(),
        )
        .unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: GrantId::parse("g").unwrap(),
            issuer: "issuer".into(),
            principal: "operator".into(),
            subject: SubjectId::parse("s").unwrap(),
            context: ContextGeneration::parse("c").unwrap(),
            build: BuildId::parse("b").unwrap(),
            proof_run: ProofRunId::parse("p").unwrap(),
            scope: GrantScope::NativeTurn,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    let mut session = store
        .reserve_native_session(
            NativeSessionId::parse("n").unwrap(),
            SubjectId::parse("s").unwrap(),
            ContextGeneration::parse("c").unwrap(),
            RuntimeIncarnation::parse("i").unwrap(),
            &pi_coding_profile_digest(),
            PI_SDK_INDEX_SHA256,
            &pi_coding_protocol_digest(),
            SnapshotId::parse("snap").unwrap(),
            &"a".repeat(64),
            false,
            clock(),
        )
        .unwrap();
    store
        .bind_native_thread(
            &mut session,
            ProviderThreadId::parse("physical:pi").unwrap(),
        )
        .unwrap();
    let mut parent = store
        .claim_native_turn(
            &session,
            NativeTurnRequest {
                invocation_id: "coding".into(),
                attempt: AttemptId::parse("attempt:coding").unwrap(),
                grant: GrantId::parse("g").unwrap(),
                grant_revision: 1,
                purpose: NativeInvocationPurpose::DomainWork,
                prompt: b"repair".to_vec(),
            },
            "operator",
            clock(),
        )
        .unwrap();
    store.mark_native_send_intent(&parent).unwrap();
    store
        .bind_native_turn(&mut parent, ProviderTurnId::parse("turn:coding").unwrap())
        .unwrap();
    let gate = CodingLifecycleGate::new(
        store,
        parent,
        session.id().clone(),
        "host-wire".into(),
        0,
        clock,
    );
    let intent = CodingToolIntent {
        job_id: "job".into(),
        input_digest: "1".repeat(64),
        session_id: "physical:pi".into(),
        call_id: "one".into(),
        command_sha256: digest(b"bounded fixture edit"),
    };
    (root, gate, intent)
}
fn evidence(root: &std::path::Path, id: &str) -> (Artifact, ProcessRecord) {
    evidence_with_stdout(root, id, b"edited\n")
}
fn evidence_with_stdout(
    root: &std::path::Path,
    id: &str,
    stdout_bytes: &[u8],
) -> (Artifact, ProcessRecord) {
    let stdout = root.join("stdout");
    let stderr = root.join("stderr");
    fs::write(&stdout, stdout_bytes).unwrap();
    fs::write(&stderr, b"").unwrap();
    let result = json!({"type":"tool_result","id":id,"ok":true,"output":format!("Host command success: true\n{}\n[stderr]\n",String::from_utf8_lossy(stdout_bytes))});
    let path = root.join("tool-result.json");
    let bytes = serde_json::to_vec(&result).unwrap();
    fs::write(&path, &bytes).unwrap();
    (
        Artifact {
            path,
            sha256: digest(&bytes),
        },
        ProcessRecord {
            argv: vec![
                "/synthetic/docker",
                "exec",
                "--workdir",
                "/workspace",
                "fixture-tool",
                "/bin/sh",
                "-c",
                "bounded fixture edit",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            pid: Some(1),
            start: Some("fixture".into()),
            status: Some(0),
            stdout,
            stderr,
            stdout_sha256: Some(digest(stdout_bytes)),
            stderr_sha256: Some(digest(b"")),
            timed_out: false,
            leftovers: vec![],
        },
    )
}
#[test]
fn admitted_intent_exact_result_and_unknown_parent_refuse_replacement_and_replay() {
    let (root, mut gate, intent) = fixture();
    assert!(gate.replacement_ready().is_err());
    let mut count = 0;
    let outcome = gate
        .execute(&intent, &mut |_expires| {
            count += 1;
            fs::write(root.join("disposable-source"), b"edited").unwrap();
            Ok((true, b"edited\n".to_vec()))
        })
        .unwrap();
    assert!(outcome.0);
    assert_eq!(count, 1);
    assert_eq!(fs::read(root.join("disposable-source")).unwrap(), b"edited");
    assert!(gate.replacement_ready().is_err());
    let (artifact, process) = evidence(&root, &intent.call_id);
    gate.observed(&intent, &artifact, &process, &"f".repeat(64))
        .unwrap();
    let retained = gate.owner().result().unwrap().clone();
    assert_eq!(retained["artifact"]["sha256"], artifact.sha256);
    assert_eq!(retained["result"]["id"], "one");
    assert!(
        gate.replacement_ready().is_err(),
        "observing a tool result cannot settle its parent"
    );
    assert!(
        gate.execute(&intent, &mut |_expires| {
            count += 1;
            Ok((true, vec![]))
        })
        .is_err()
    );
    assert_eq!(count, 1);
    drop(gate);
    let reopened = SqliteLifecycleStore::open(&root, "issuer".into()).unwrap();
    assert_eq!(
        reopened.native_recovery("coding").unwrap().unwrap().class,
        "EnteredUncertain"
    );
    drop(reopened);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn process_artifact_and_foreign_result_cannot_settle_host_observation() {
    for case in 0..5 {
        let (root, mut gate, intent) = fixture();
        gate.execute(&intent, &mut |_expires| Ok((true, b"edited\n".to_vec())))
            .unwrap();
        let (artifact, mut process) = evidence(&root, if case == 0 { "foreign" } else { "one" });
        if case == 1 {
            fs::write(&process.stdout, b"changed").unwrap();
        }
        if case == 2 {
            process.timed_out = true;
        }
        if case == 3 {
            process.leftovers = vec![999];
        }
        if case == 4 {
            fs::write(&artifact.path, b"{}").unwrap();
        }
        assert!(
            gate.observed(&intent, &artifact, &process, &"f".repeat(64))
                .is_err(),
            "case {case}"
        );
        assert!(gate.owner().result().is_none());
        assert!(gate.replacement_ready().is_err());
        drop(gate);
        fs::remove_dir_all(root).unwrap();
    }
}

use pi_pressure_context_host::coding::{CodingConfig, CodingContinuity};
fn continuity_fixture() -> (PathBuf, CodingContinuity) {
    continuity_fixture_with(false, None)
}
/// `live` selects the separately versioned v4 profile and its honest budget
/// label; `mixed` deliberately registers a v4 profile with the v3 protocol.
fn continuity_fixture_with(live: bool, mixed: Option<()>) -> (PathBuf, CodingContinuity) {
    let (root, c) = continuity_try(live, mixed, None);
    (root, c.unwrap())
}
fn continuity_try(
    live: bool,
    mixed: Option<()>,
    class: Option<&str>,
) -> (
    PathBuf,
    Result<CodingContinuity, work_engine_manual_host::Error>,
) {
    continuity_try_mode(live, mixed, class, false)
}
fn continuity_try_mode(
    live: bool,
    mixed: Option<()>,
    class: Option<&str>,
    sol: bool,
) -> (
    PathBuf,
    Result<CodingContinuity, work_engine_manual_host::Error>,
) {
    let root = std::env::temp_dir().join(format!(
        "coding-protocol-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut store = SqliteLifecycleStore::open(&root, "fixture-root".into()).unwrap();
    let subject = SubjectId::parse("subject:pi:coding").unwrap();
    store
        .register_subject(
            subject.clone(),
            ContextGeneration::parse("context:old").unwrap(),
            BuildId::parse("build:pi:coding").unwrap(),
            ProofRunId::parse("proof:pi:coding").unwrap(),
            clock(),
        )
        .unwrap();
    for (id, context, scope) in [
        ("grant:old", "context:old", GrantScope::NativeTurn),
        ("grant:new", "context:new", GrantScope::NativeTurn),
        (
            "grant:replace",
            "context:old",
            GrantScope::RequestReplacement,
        ),
        ("grant:successor", "context:new", GrantScope::EnqueueInput),
    ] {
        store
            .install_trusted_grant(TrustedGrant {
                id: GrantId::parse(id).unwrap(),
                issuer: "fixture-root".into(),
                principal: "operator:fixture".into(),
                subject: subject.clone(),
                context: ContextGeneration::parse(context).unwrap(),
                build: BuildId::parse("build:pi:coding").unwrap(),
                proof_run: ProofRunId::parse("proof:pi:coding").unwrap(),
                scope,
                expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
    }
    let staged = store.stage_artifact(subject.clone(), b"{}").unwrap();
    let published = store.publish_artifact(staged, clock()).unwrap();
    let initial = store
        .reserve_native_session(
            NativeSessionId::parse("session:old").unwrap(),
            subject,
            ContextGeneration::parse("context:old").unwrap(),
            RuntimeIncarnation::parse("incarnation:pi:coding").unwrap(),
            &if sol {
                pi_coding_sol_high_profile_digest()
            } else if live {
                pi_coding_live_profile_digest()
            } else {
                pi_coding_profile_digest()
            },
            PI_SDK_INDEX_SHA256,
            &if sol {
                pi_coding_sol_high_protocol_digest()
            } else if live && mixed.is_none() {
                pi_coding_live_protocol_digest()
            } else {
                pi_coding_protocol_digest()
            },
            SnapshotId::parse("snapshot:coding").unwrap(),
            &published.digest_hex,
            false,
            clock(),
        )
        .unwrap();
    let config = CodingConfig {
        sol_high: sol,
        task: live.then(|| "Fix the disposable crate so the acceptance tests pass.".to_string()),
        budget: json!({"schema":"work-engine.coding-budget-control.v1","classification":class.unwrap_or(if live {"live_provider_observation"} else {"provider_disabled_fixture"}),"available":true,"max_provider_requests":4,"max_tool_calls":1,"authority":"observation_only","sequence":1,"expires_ms":1000,"source":"synthetic-root-budget","generation":"fixture:1","observed_ms":100,"max_age_ms":300000}),
        expires_ms: 1000,
        pressure_basis_points: 9500,
    };
    (root, CodingContinuity::new(store, initial, config, clock))
}
fn snapshot(session: &str) -> serde_json::Value {
    json!({"sessionId":session,"idle":true,"observedMs":100,"observedAt":"2026-11-05T00:00:00Z","model":{"id":"fixture","provider":"fixture","contextWindow":32768},"usage":null})
}
fn control(c: &mut CodingContinuity, v: &serde_json::Value) {
    let mut entered = 0;
    c.dispatch(v, &mut |expires| {
        assert_eq!(expires, 1000);
        entered += 1;
        Ok(())
    })
    .unwrap_or_else(|e| panic!("dispatch {} id {}: {e}", v["type"], v["id"]));
    assert_eq!(entered, 1);
}
struct SyntheticSdk {
    session: String,
    messages: Vec<serde_json::Value>,
    branch: Vec<serde_json::Value>,
    requests: u64,
    tool_output_bytes: usize,
    system_mutation: u8,
    live: bool,
    preamble: bool,
    thinking: bool,
}
impl SyntheticSdk {
    fn new(session: &str, requests: u64) -> Self {
        Self {
            session: session.into(),
            messages: vec![],
            branch: vec![],
            requests,
            tool_output_bytes: 4096,
            system_mutation: 0,
            live: false,
            preamble: false,
            thinking: false,
        }
    }
    fn assistant(&self, mut m: serde_json::Value) -> serde_json::Value {
        if self.thinking {
            m["content"].as_array_mut().unwrap().insert(0,json!({"type":"thinking","thinking":"synthetic high reasoning","thinkingSignature":"{\"type\":\"reasoning\",\"id\":\"fixture\"}"}));
        }
        m
    }
    fn native(&self, c: &mut CodingContinuity, event: serde_json::Value) {
        assert!(
            c.frame(&json!({"type":"native_event","sessionId":self.session,"event":event}))
                .unwrap()
                .is_empty()
        );
    }
    fn append(&mut self, m: serde_json::Value) {
        let id = format!("{}:{}", self.session, self.branch.len());
        let parent = self
            .branch
            .last()
            .map(|r| r["id"].clone())
            .unwrap_or(serde_json::Value::Null);
        self.branch
            .push(json!({"type":"message","id":id,"parentId":parent,"message":m}));
        self.messages.push(m);
    }
    fn message(&mut self, c: &mut CodingContinuity, m: serde_json::Value) {
        self.native(c, json!({"type":"message_start","message":m}));
        self.native(c, json!({"type":"message_end","message":m}));
        self.append(m);
    }
    fn request(
        &mut self,
        c: &mut CodingContinuity,
        command: &serde_json::Value,
        workspace: Option<&serde_json::Value>,
    ) -> serde_json::Value {
        self.requests += 1;
        let mut frame = json!({"type":"coding_provider_request","producer":"pi-1.5.0","sessionId":self.session,"id":self.requests,"invocationId":command["invocationId"],"turnCorrelation":command["turnCorrelation"],"prompt_sha256":digest(command["prompt"].as_str().unwrap().as_bytes()),"messages":self.messages,"agentMessages":self.messages,"usage":null});
        match self.system_mutation {
            1 => {
                frame["messages"][0]["sections"]["preamble"] =
                    json!("altered predecessor instructions")
            }
            2 => frame["messages"]
                .as_array_mut()
                .unwrap()
                .push(json!({"role":"system","content":"old predecessor conversation"})),
            3 => {
                frame["messages"].as_array_mut().unwrap().remove(0);
            }
            _ => {}
        }
        if let Some(w) = workspace {
            frame["host_workspace"] = w.clone();
        }
        let commands = c.frame(&frame).unwrap();
        assert_eq!(commands.len(), 1);
        control(c, &commands[0]);
        commands[0].clone()
    }
    fn turn(
        &mut self,
        c: &mut CodingContinuity,
        command: &serde_json::Value,
        tool_root: Option<&std::path::Path>,
    ) -> Result<serde_json::Value, work_engine_manual_host::Error> {
        self.native(c, json!({"type":"agent_start"}));
        self.native(c, json!({"type":"turn_start"}));
        let user = json!({"role":"user","content":[{"type":"text","text":command["prompt"]}]});
        let mut generated = vec![];
        if self.messages.is_empty() {
            let system = json!({"role":"system","content":"","sections":{"preamble":"Fresh Pi coding assistant boilerplate","tools":"No inherited tools","cwd":"/workspace"},"timestamp":100});
            self.message(c, system.clone());
            generated.push(system);
        }
        generated.push(user.clone());
        self.message(c, user);
        let first = self.request(c, command, None);
        let response = if let Some(root) = tool_root {
            assert_eq!(first["mode"], if self.live { "live" } else { "tool" });
            let call = json!({"type":"toolCall","id":"one","name":"workspace_exec","arguments":{"command":"bounded fixture edit"}});
            let content = if self.preamble {
                json!([{"type":"text","text":"I will make the edit now."}, call])
            } else {
                json!([call])
            };
            let assistant = self
                .assistant(json!({"role":"assistant","content":content,"stopReason":"toolUse"}));
            self.message(c, assistant.clone());
            generated.push(assistant.clone());
            self.native(c,json!({"type":"tool_execution_start","toolCallId":"one","toolName":"workspace_exec","args":call["arguments"]}));
            let intent = CodingToolIntent {
                job_id: "job".into(),
                input_digest: "1".repeat(64),
                session_id: self.session.clone(),
                call_id: "one".into(),
                command_sha256: digest(b"bounded fixture edit"),
            };
            c.execute(&intent, &mut |_expires| Ok((true, b"edited\n".to_vec())))
                .unwrap();
            let (artifact, process) =
                evidence_with_stdout(root, "one", &vec![b'x'; self.tool_output_bytes]);
            c.observed(&intent, &artifact, &process, &"f".repeat(64))
                .unwrap();
            let result: serde_json::Value =
                serde_json::from_slice(&artifact.verify().unwrap()).unwrap();
            let mut tampered = result.clone();
            tampered["output"] = json!("forged output");
            assert!(
                c.dispatch(&tampered, &mut |_| panic!("tampered result must not send"))
                    .is_err()
            );
            control(c, &result);
            assert!(
                c.dispatch(&result, &mut |_| panic!("duplicate result must not send"))
                    .is_err()
            );
            let tool = json!({"role":"toolResult","toolCallId":"one","toolName":"workspace_exec","content":[{"type":"text","text":result["output"]}],"details":{"hostReportedSuccess":true},"isError":false});
            self.native(c,json!({"type":"tool_execution_end","toolCallId":"one","toolName":"workspace_exec","isError":false,"result":{"content":tool["content"],"details":tool["details"]}}));
            self.message(c, tool.clone());
            generated.push(tool.clone());
            self.native(
                c,
                json!({"type":"turn_end","message":assistant,"toolResults":[tool]}),
            );
            self.native(c, json!({"type":"turn_start"}));
            let workspace = json!({"manifest":{"path":"/synthetic/workspace.json","sha256":digest(b"{}")},"manifest_bytes":"{}"});
            self.request(c, command, Some(&workspace))
        } else {
            first
        };
        // Live: the provider supplies free-form text; nothing is manufactured.
        let reply = if self.live {
            json!(format!(
                "Model-authored summary for {} (not a checkpoint).",
                command["invocationId"]
            ))
        } else {
            response["reply"].clone()
        };
        let final_message = self.assistant(json!({"role":"assistant","content":[{"type":"text","text":reply}],"stopReason":"stop"}));
        self.message(c, final_message.clone());
        generated.push(final_message.clone());
        self.native(
            c,
            json!({"type":"turn_end","message":final_message,"toolResults":[]}),
        );
        self.native(
            c,
            json!({"type":"agent_end","messages":generated,"willRetry":false}),
        );
        self.native(c, json!({"type":"agent_settled","aborted":false}));
        let history = json!({"type":"coding_history","producer":"pi-1.5.0","sessionId":self.session,"invocationId":command["invocationId"],"messages":self.messages,"branch":self.branch,"header":{"id":self.session,"type":"session"},"leafId":self.branch.last().unwrap()["id"],"snapshot":snapshot(&self.session)});
        let commands = c.frame(&history)?;
        assert_eq!(commands.len(), 1);
        Ok(commands[0].clone())
    }
}
#[test]
fn complete_synthetic_coding_checkpoint_reset_exact_recovery_and_continuation() {
    let (root, mut c) = continuity_fixture();
    let commands = c
        .frame(
            &json!({"type":"ready","sessionId":"physical:old","snapshot":snapshot("physical:old")}),
        )
        .unwrap();
    let original = commands[0].clone();
    control(&mut c, &original);
    assert!(
        c.dispatch(&original, &mut |_| panic!("duplicate prompt must not send"))
            .is_err()
    );
    let mut sdk = SyntheticSdk::new("physical:old", 0);
    let reset = sdk.turn(&mut c, &original, Some(&root)).unwrap();
    assert_eq!(reset["type"], "bridge_replace");
    control(&mut c, &reset);
    assert!(
        c.dispatch(&reset, &mut |_| panic!("duplicate reset must not send"))
            .is_err()
    );
    let commands=c.frame(&json!({"type":"coding_replaced","sessionId":"physical:new","oldSessionId":"physical:old","result":{"cancelled":false},"messages":[],"branch":[],"header":{"type":"session","id":"physical:new"},"leafId":null,"autoCompactionEnabled":false,"tools":[],"snapshot":snapshot("physical:new")})).unwrap();
    let recovery = commands[0].clone();
    assert!(recovery["prompt"].as_str().unwrap().len() > 2048);
    assert!(
        recovery["prompt"]
            .as_str()
            .unwrap()
            .contains("coding_result_sha256")
    );
    control(&mut c, &recovery);
    let mut fresh = SyntheticSdk::new("physical:new", 2);
    let continued = fresh.turn(&mut c, &recovery, None).unwrap();
    assert_eq!(continued["invocationId"], "continued");
    control(&mut c, &continued);
    let close = fresh.turn(&mut c, &continued, None).unwrap();
    assert_eq!(close["type"], "bridge_close");
    control(&mut c, &close);
    let report = c.report().unwrap();
    assert_eq!(report["context"], "context:new");
    assert_eq!(report["owner"], "domain");
    assert_eq!(report["provider_requests"], 4);
    assert_eq!(report["continued_parent"], "CompletedSettled");
    assert!(report["recovery_receipt"].is_string());
    drop(c);
    let reopened = SqliteLifecycleStore::open(&root, "fixture-root".into()).unwrap();
    assert_eq!(
        reopened
            .native_recovery("continued")
            .unwrap()
            .unwrap()
            .class,
        "CompletedSettled"
    );
    drop(reopened);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tool_result_dispatch_requires_exact_custody_and_never_retries_failed_write() {
    let (root, mut c) = continuity_fixture();
    let commands = c
        .frame(
            &json!({"type":"ready","sessionId":"physical:old","snapshot":snapshot("physical:old")}),
        )
        .unwrap();
    control(&mut c, &commands[0]);
    let intent = CodingToolIntent {
        job_id: "job".into(),
        input_digest: "1".repeat(64),
        session_id: "physical:old".into(),
        call_id: "one".into(),
        command_sha256: digest(b"bounded fixture edit"),
    };
    let (artifact, process) = evidence(&root, "one");
    let result: serde_json::Value = serde_json::from_slice(&artifact.verify().unwrap()).unwrap();
    assert!(
        c.dispatch(&result, &mut |_| panic!("unobserved result must not send"))
            .is_err()
    );
    c.execute(&intent, &mut |_| Ok((true, b"edited\n".to_vec())))
        .unwrap();
    c.observed(&intent, &artifact, &process, &"f".repeat(64))
        .unwrap();
    let bytes = artifact.verify().unwrap();
    fs::write(&artifact.path, b"{}").unwrap();
    assert!(
        c.dispatch(&result, &mut |_| panic!("changed artifact must not send"))
            .is_err()
    );
    fs::write(&artifact.path, bytes).unwrap();
    let mut attempts = 0;
    assert!(
        c.dispatch(&result, &mut |_| {
            attempts += 1;
            Err(work_engine_manual_host::Error::Refused(
                "synthetic failed write".into(),
            ))
        })
        .is_err()
    );
    assert_eq!(attempts, 1);
    assert!(
        c.dispatch(&result, &mut |_| panic!("uncertain write must not replay"))
            .is_err()
    );
    drop(c);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn oversized_coding_checkpoint_cannot_admit_replacement() {
    let (root, mut c) = continuity_fixture();
    let commands = c
        .frame(
            &json!({"type":"ready","sessionId":"physical:old","snapshot":snapshot("physical:old")}),
        )
        .unwrap();
    control(&mut c, &commands[0]);
    let mut sdk = SyntheticSdk::new("physical:old", 0);
    sdk.tool_output_bytes = 65536;
    assert!(sdk.turn(&mut c, &commands[0], Some(&root)).is_err());
    drop(c);
    let reopened = SqliteLifecycleStore::open(&root, "fixture-root".into()).unwrap();
    assert_eq!(
        reopened.native_recovery("coding").unwrap().unwrap().class,
        "EnteredUncertain"
    );
    assert!(reopened.native_recovery("rehydrate").unwrap().is_none());
    drop(reopened);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recovery_and_continuation_reject_changed_extra_or_missing_system_input() {
    for target in [3, 4] {
        for mutation in 1..=3 {
            let (root, mut c) = continuity_fixture();
            let commands = c.frame(&json!({"type":"ready","sessionId":"physical:old","snapshot":snapshot("physical:old")})).unwrap();
            control(&mut c, &commands[0]);
            let mut old = SyntheticSdk::new("physical:old", 0);
            let reset = old.turn(&mut c, &commands[0], Some(&root)).unwrap();
            control(&mut c, &reset);
            let commands = c.frame(&json!({"type":"coding_replaced","sessionId":"physical:new","oldSessionId":"physical:old","result":{"cancelled":false},"messages":[],"branch":[],"header":{"type":"session","id":"physical:new"},"leafId":null,"autoCompactionEnabled":false,"tools":[],"snapshot":snapshot("physical:new")})).unwrap();
            let mut command = commands[0].clone();
            control(&mut c, &command);
            let mut fresh = SyntheticSdk::new("physical:new", 2);
            if target == 4 {
                command = fresh.turn(&mut c, &command, None).unwrap();
                control(&mut c, &command);
            }
            fresh.system_mutation = mutation;
            assert!(
                fresh.turn(&mut c, &command, None).is_err(),
                "request {target}, mutation {mutation}"
            );
            drop(c);
            let reopened = SqliteLifecycleStore::open(&root, "fixture-root".into()).unwrap();
            let invocation = if target == 3 {
                "rehydrate"
            } else {
                "continued"
            };
            assert_eq!(
                reopened.native_recovery(invocation).unwrap().unwrap().class,
                "EnteredUncertain"
            );
            drop(reopened);
            fs::remove_dir_all(root).unwrap();
        }
    }
}

fn run_complete(
    live: bool,
    preamble: bool,
) -> Result<(PathBuf, CodingContinuity), work_engine_manual_host::Error> {
    run_complete_mode(live, preamble, false)
}
fn run_complete_mode(
    live: bool,
    preamble: bool,
    sol: bool,
) -> Result<(PathBuf, CodingContinuity), work_engine_manual_host::Error> {
    let (root, c) = continuity_try_mode(live, None, None, sol);
    let mut c = c?;

    let commands = c
        .frame(
            &json!({"type":"ready","sessionId":"physical:old","snapshot":snapshot("physical:old")}),
        )
        .unwrap();
    let original = commands[0].clone();
    if live {
        assert!(
            original["prompt"]
                .as_str()
                .unwrap()
                .starts_with("Fix the disposable crate")
        );
    }
    control(&mut c, &original);
    let mut sdk = SyntheticSdk::new("physical:old", 0);
    sdk.live = live;
    sdk.thinking = sol;
    sdk.preamble = preamble;
    let reset = sdk.turn(&mut c, &original, Some(&root))?;
    assert_eq!(reset["type"], "bridge_replace");
    control(&mut c, &reset);
    let commands=c.frame(&json!({"type":"coding_replaced","sessionId":"physical:new","oldSessionId":"physical:old","result":{"cancelled":false},"messages":[],"branch":[],"header":{"type":"session","id":"physical:new"},"leafId":null,"autoCompactionEnabled":false,"tools":[],"snapshot":snapshot("physical:new")})).unwrap();
    let recovery = commands[0].clone();
    let text = recovery["prompt"].as_str().unwrap();
    if live {
        // Sole recovery input is the owner-derived checkpoint, not model text.
        assert!(text.contains(if sol {
            "work-engine.pi-coding-sol-high-continuity.v1"
        } else {
            "work-engine.pi-coding-live-continuity.v1"
        }));
        assert!(
            text.contains("coding_result_sha256") && text.contains("workspace_manifest_sha256")
        );
        assert!(!text.contains("Model-authored summary"));
        assert!(!text.contains("fixture"));
    }
    control(&mut c, &recovery);
    let mut fresh = SyntheticSdk::new("physical:new", 2);
    fresh.live = live;
    fresh.thinking = sol;
    let continued = fresh.turn(&mut c, &recovery, None).unwrap();
    control(&mut c, &continued);
    let close = fresh.turn(&mut c, &continued, None).unwrap();
    control(&mut c, &close);
    Ok((root, c))
}
#[test]
fn live_continuity_owner_derives_checkpoint_from_real_shaped_turns_and_recovers_once() {
    let (root, c) = run_complete(true, false).unwrap();
    let report = c.report().unwrap();
    assert_eq!(report["mode"], "live");
    assert!(
        report["standing"]
            .as_str()
            .unwrap()
            .contains("real inference")
    );
    assert_eq!(report["provider_requests"], 4);
    assert_eq!(report["continued_parent"], "CompletedSettled");
    let checkpoint: serde_json::Value =
        serde_json::from_str(report["checkpoint"].as_str().unwrap()).unwrap();
    assert_eq!(
        checkpoint["profile"],
        "work-engine.pi-coding-live-continuity.v1"
    );
    assert_eq!(checkpoint["tool_executions"], 1);
    assert_eq!(
        checkpoint["coding_result"],
        report["original_host_result"]["result"]
    );
    assert!(checkpoint["settled_turn"]["assistant_final_text_sha256"].is_string());
    assert!(checkpoint.get("fixture").is_none() && checkpoint.get("canonical_budget").is_none());
    drop(c);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn live_profile_accepts_text_before_the_single_tool_call_but_fixture_v3_does_not() {
    let (root, c) = run_complete(true, true).unwrap();
    drop(c);
    fs::remove_dir_all(root).unwrap();
    assert!(run_complete(false, true).is_err());
}
#[test]
fn live_and_fixture_budget_labels_and_mixed_identities_cannot_substitute() {
    // A fixture label cannot stand in for a live observation, nor the reverse.
    for (live, class) in [
        (true, "provider_disabled_fixture"),
        (false, "live_provider_observation"),
    ] {
        let (root, c) = continuity_try(live, None, Some(class));
        assert!(c.is_err(), "live={live} class={class}");
        fs::remove_dir_all(root).unwrap();
    }
    // v4 profile bound to the v3 protocol: the owner refuses the very first entry.
    let (root, mut c) = continuity_fixture_with(true, Some(()));
    let commands = c
        .frame(
            &json!({"type":"ready","sessionId":"physical:old","snapshot":snapshot("physical:old")}),
        )
        .unwrap();
    assert!(
        c.dispatch(&commands[0], &mut |_| panic!(
            "mixed identity must not enter"
        ))
        .is_err()
    );
    drop(c);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn sol_high_thinking_profile_resets_rehydrates_and_continues_exactly_once() {
    let (root, c) = run_complete_mode(true, true, true).unwrap();
    let report = c.report().unwrap();
    assert_eq!(report["provider_requests"], 4);
    assert_eq!(report["physical_resets"], 1);
    assert_eq!(report["continued_parent"], "CompletedSettled");
    let checkpoint: serde_json::Value =
        serde_json::from_str(report["checkpoint"].as_str().unwrap()).unwrap();
    assert_eq!(
        checkpoint["profile"],
        "work-engine.pi-coding-sol-high-continuity.v1"
    );
    drop(c);
    let reopened = SqliteLifecycleStore::open(&root, "fixture-root".into()).unwrap();
    assert_eq!(
        reopened
            .native_recovery("continued")
            .unwrap()
            .unwrap()
            .class,
        "CompletedSettled"
    );
    drop(reopened);
    fs::remove_dir_all(root).unwrap();
}
