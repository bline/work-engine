//! Synthetic protocol-unit fixtures; never original SDK evidence or authority.
use lifecycle_core::*;
use lifecycle_store::*;
use serde_json::{Value, json};
use tempfile::tempdir;
use work_engine_types::CodecContract;
fn clock() -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(100),
        wait_budget: WaitBudgetMs::new(100),
    }
}
fn hash(b: &[u8]) -> String {
    CodecContract::BinaryArtifactV1
        .digest_binary(b)
        .unwrap()
        .hex()
}
fn events(inv: &str, prior: Option<&Value>) -> Vec<Value> {
    let user = json!({"role":"user","content":[{"type":"text","text":inv}]});
    let assistant = json!({"role":"assistant","content":[{"type":"text","text":format!("reply:{inv}")}],"stopReason":"stop"});
    let mut input = prior
        .map(|p| p["messages"].as_array().unwrap().clone())
        .unwrap_or_default();
    input.push(user.clone());
    let mut full = input.clone();
    full.push(assistant.clone());
    let mut branch = prior
        .map(|p| p["branch"].as_array().unwrap().clone())
        .unwrap_or_default();
    let parent = branch
        .last()
        .map(|r| r["id"].clone())
        .unwrap_or(Value::Null);
    branch.push(json!({"type":"message","id":format!("{inv}:u"),"parentId":parent,"message":user}));
    branch.push(json!({"type":"message","id":format!("{inv}:a"),"parentId":format!("{inv}:u"),"message":assistant}));
    vec![
        json!({"type":"agent_start"}),
        json!({"type":"turn_start"}),
        json!({"type":"message_start","message":user}),
        json!({"type":"message_end","message":user}),
        json!({"type":"pi.provider_request","producer":"pi-1.5.0","sessionId":"physical:pi","invocationId":inv,"turnCorrelation":format!("turn:{inv}"),"prompt_sha256":hash(inv.as_bytes()),"messages":input,"agentMessages":input}),
        json!({"type":"message_start","message":{"role":"assistant"}}),
        json!({"type":"message_end","message":assistant}),
        json!({"type":"turn_end","message":assistant,"toolResults":[]}),
        json!({"type":"agent_end","messages":[user,assistant],"willRetry":false}),
        json!({"type":"agent_settled","aborted":false}),
        json!({"type":"pi.session_history","producer":"pi-1.5.0","sessionId":"physical:pi","header":{"type":"session","id":"physical:pi"},"leafId":format!("{inv}:a"),"messages":full,"branch":branch}),
    ]
}
fn setup(root: &std::path::Path, multi: bool) -> (SqliteLifecycleStore, NativeSessionEntry) {
    let profile = if multi {
        pi_multiturn_profile_digest()
    } else {
        pi_profile_digest()
    };
    let protocol = if multi {
        pi_multiturn_protocol_digest()
    } else {
        pi_protocol_digest()
    };
    setup_binding(root, &profile, &protocol)
}
fn setup_binding(
    root: &std::path::Path,
    profile: &str,
    protocol: &str,
) -> (SqliteLifecycleStore, NativeSessionEntry) {
    let mut s = SqliteLifecycleStore::open(root, "issuer".into()).unwrap();
    s.register_subject(
        SubjectId::parse("s").unwrap(),
        ContextGeneration::parse("c").unwrap(),
        BuildId::parse("b").unwrap(),
        ProofRunId::parse("p").unwrap(),
        clock(),
    )
    .unwrap();
    s.install_trusted_grant(TrustedGrant {
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
    let mut session = s
        .reserve_native_session(
            NativeSessionId::parse("n").unwrap(),
            SubjectId::parse("s").unwrap(),
            ContextGeneration::parse("c").unwrap(),
            RuntimeIncarnation::parse("i").unwrap(),
            profile,
            PI_SDK_INDEX_SHA256,
            protocol,
            SnapshotId::parse("snap").unwrap(),
            &"a".repeat(64),
            false,
            clock(),
        )
        .unwrap();
    s.bind_native_thread(
        &mut session,
        ProviderThreadId::parse("physical:pi").unwrap(),
    )
    .unwrap();
    (s, session)
}
fn claim(
    s: &mut SqliteLifecycleStore,
    session: &NativeSessionEntry,
    inv: &str,
) -> Result<NativeTurnEntry, StoreError> {
    s.claim_native_turn(
        session,
        NativeTurnRequest {
            invocation_id: inv.into(),
            attempt: AttemptId::parse(format!("attempt:{inv}")).unwrap(),
            grant: GrantId::parse("g").unwrap(),
            grant_revision: 1,
            purpose: NativeInvocationPurpose::DomainWork,
            prompt: inv.as_bytes().to_vec(),
        },
        "operator",
        clock(),
    )
}
fn record(
    s: &mut SqliteLifecycleStore,
    session: &NativeSessionEntry,
    inv: &str,
    values: &[Value],
    first: u64,
) -> (NativeTurnEntry, PiSettlementProof) {
    let mut turn = claim(s, session, inv).unwrap();
    s.mark_native_send_intent(&turn).unwrap();
    s.bind_native_turn(
        &mut turn,
        ProviderTurnId::parse(format!("turn:{inv}")).unwrap(),
    )
    .unwrap();
    for (i, v) in values.iter().enumerate() {
        s.record_native_ingress(
            session.id(),
            "wire",
            first + i as u64,
            &serde_json::to_vec(v).unwrap(),
            "qualified",
            v["type"].as_str(),
            Some("physical:pi"),
            Some(&format!("turn:{inv}")),
            None,
            clock(),
        )
        .unwrap();
    }
    (
        turn,
        PiSettlementProof {
            transport: "wire".into(),
            first_sequence: first,
            history_sequence: first + values.len() as u64 - 1,
        },
    )
}
#[test]
fn anchored_history_accepts_only_exact_owner_prefix() {
    for case in 0..10 {
        let root = tempdir().unwrap();
        let (mut s, session) = setup(root.path(), true);
        let first = events("first", None);
        let (t, p) = record(&mut s, &session, "first", &first, 1);
        s.settle_pi_multiturn(&t, &p, clock()).unwrap();
        let mut next = events("next", first.last());
        match case {
            1 => {
                next[4]["agentMessages"][0]["content"][0]["text"] = json!("rewrite");
            }
            2 => {
                next[4]["messages"][0]["content"][0]["text"] = json!("rewrite");
            }
            3 => {
                next[4]["agentMessages"].as_array_mut().unwrap().remove(0);
            }
            4 => {
                next[10]["branch"][0]["id"] = json!("fork");
                next[10]["branch"][1]["parentId"] = json!("fork");
            }
            5 => {
                next[10]["messages"][0]["content"][0]["text"] = json!("rewrite");
            }
            6 => {
                next[4]["sessionId"] = json!("other-session");
            }
            7 => {
                next[10]["branch"].as_array_mut().unwrap().swap(0, 1);
            }
            8 => {
                next[4]["agentMessages"].as_array_mut().unwrap().swap(0, 1);
            }
            9 => {
                next[10]["header"]["id"] = json!("other-session");
            }
            _ => {}
        }
        let (t, p) = record(&mut s, &session, "next", &next, 12);
        let result = s.settle_pi_multiturn(&t, &p, clock());
        if case == 0 {
            result.unwrap();
            assert!(s.settle_pi_multiturn(&t, &p, clock()).is_err());
        } else {
            assert!(result.is_err(), "case {case}");
            assert_eq!(
                s.native_recovery("next").unwrap().unwrap().class,
                "EnteredUncertain"
            );
        }
    }
}
#[test]
fn v1_still_rejects_retained_history() {
    let root = tempdir().unwrap();
    let (mut s, session) = setup(root.path(), false);
    let first = events("first", None);
    let (t, p) = record(&mut s, &session, "first", &first, 1);
    s.settle_pi_turn(&t, &p, clock()).unwrap();
    let next = events("next", first.last());
    let (t, p) = record(&mut s, &session, "next", &next, 12);
    assert!(s.settle_pi_turn(&t, &p, clock()).is_err());
    assert!(s.settle_pi_multiturn(&t, &p, clock()).is_err());
}
#[test]
fn unsettled_predecessor_and_unowned_prefix_are_refused() {
    let root = tempdir().unwrap();
    let (mut s, session) = setup(root.path(), true);
    let first = events("first", None);
    let (_t, _p) = record(&mut s, &session, "first", &first, 1);
    assert!(claim(&mut s, &session, "next").is_err());
    let other = tempdir().unwrap();
    let (mut s, session) = setup(other.path(), true);
    let next = events("next", first.last());
    let (t, p) = record(&mut s, &session, "next", &next, 1);
    assert!(s.settle_pi_multiturn(&t, &p, clock()).is_err());
}

#[test]
fn valid_generic_native_proof_cannot_bypass_either_pi_identifier() {
    let generic_profile = "a".repeat(64);
    let generic_protocol = "b".repeat(64);
    let bindings = [
        (generic_profile.clone(), generic_protocol.clone(), true),
        (
            pi_multiturn_profile_digest(),
            pi_multiturn_protocol_digest(),
            false,
        ),
        (
            pi_multiturn_profile_digest(),
            generic_protocol.clone(),
            false,
        ),
        (
            generic_profile.clone(),
            pi_multiturn_protocol_digest(),
            false,
        ),
        (
            pi_coding_profile_digest(),
            pi_coding_protocol_digest(),
            false,
        ),
        (pi_coding_profile_digest(), generic_protocol.clone(), false),
        (generic_profile.clone(), pi_coding_protocol_digest(), false),
        // Live continuity (v4) identities, including mixed bindings, are equally
        // unavailable to generic native settlement.
        (
            pi_coding_live_profile_digest(),
            pi_coding_live_protocol_digest(),
            false,
        ),
        (
            pi_coding_live_profile_digest(),
            generic_protocol.clone(),
            false,
        ),
        (
            generic_profile.clone(),
            pi_coding_live_protocol_digest(),
            false,
        ),
        (
            pi_coding_live_profile_digest(),
            pi_coding_protocol_digest(),
            false,
        ),
        (pi_profile_digest(), generic_protocol, false),
        (generic_profile, pi_protocol_digest(), false),
    ];
    for (profile, protocol, allowed) in bindings {
        let root = tempdir().unwrap();
        let (mut store, session) = setup_binding(root.path(), &profile, &protocol);
        let mut turn = claim(&mut store, &session, "generic").unwrap();
        store.mark_native_send_intent(&turn).unwrap();
        store
            .bind_native_turn(&mut turn, ProviderTurnId::parse("turn:generic").unwrap())
            .unwrap();
        let item = json!({"type":"agentMessage","id":"native-final","text":"generic result"});
        for (sequence, method, value) in [
            (
                1,
                Some("turn/completed"),
                json!({"method":"turn/completed","params":{"threadId":"physical:pi","turn":{"id":"turn:generic","status":"completed"}}}),
            ),
            (
                2,
                Some("item/completed"),
                json!({"method":"item/completed","params":{"threadId":"physical:pi","turnId":"turn:generic","item":item}}),
            ),
            (
                3,
                None,
                json!({"id":3,"result":{"thread":{"id":"physical:pi","historyMode":"legacy","turns":[{"id":"turn:generic","status":"completed","itemsView":"full","items":[item]}]}}}),
            ),
        ] {
            store
                .record_native_ingress(
                    session.id(),
                    "generic-wire",
                    sequence,
                    &serde_json::to_vec(&value).unwrap(),
                    "qualified",
                    method,
                    method.map(|_| "physical:pi"),
                    method.map(|_| "turn:generic"),
                    None,
                    clock(),
                )
                .unwrap();
        }
        let result = store.settle_native_turn(
            &turn,
            "generic result",
            "observed_complete",
            "generic-wire",
            3,
            clock(),
        );
        if allowed {
            result.unwrap();
            assert_eq!(
                store.native_recovery("generic").unwrap().unwrap().class,
                "CompletedSettled"
            );
        } else {
            assert!(matches!(result, Err(StoreError::Rejected)));
            let pending = store.native_recovery("generic").unwrap().unwrap();
            assert_eq!(pending.class, "EnteredUncertain");
            assert!(pending.send_intent);
        }
    }
}

#[test]
fn coding_effect_is_serialized_and_parent_uncertainty_survives_reopen() {
    let root = tempdir().unwrap();
    let (mut store, session) = setup_binding(
        root.path(),
        &pi_coding_profile_digest(),
        &pi_coding_protocol_digest(),
    );
    let mut parent = claim(&mut store, &session, "coding").unwrap();
    store.mark_native_send_intent(&parent).unwrap();
    store
        .bind_native_turn(&mut parent, ProviderTurnId::parse("turn:coding").unwrap())
        .unwrap();
    let intent = json!({"job_id":"job","input_digest":"1".repeat(64),"session_id":"physical:pi","call_id":"call:1","command_sha256":"2".repeat(64)});
    let raw = json!({"type":"pi.coding_tool_intent","invocationId":"coding","intent":intent});
    store
        .record_native_ingress(
            session.id(),
            "host-wire",
            1,
            &serde_json::to_vec(&raw).unwrap(),
            "qualified",
            Some("pi.coding_tool_intent"),
            Some("physical:pi"),
            Some("turn:coding"),
            None,
            clock(),
        )
        .unwrap();
    let mut entries = 0;
    let result = store
        .with_pi_coding_effect(&parent, "host-wire", 1, &intent, clock(), |_expires| {
            // A concurrent invalidator cannot pass the exact effect boundary while
            // this already-admitted actuator owns the write transaction.
            let other = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
            other.busy_timeout(std::time::Duration::ZERO).unwrap();
            assert!(
                other
                    .execute("UPDATE grants SET revoked=1 WHERE grant_id='g'", [])
                    .is_err()
            );
            entries += 1;
            Ok::<_, ()>(b"actual local fixture result".to_vec())
        })
        .unwrap()
        .unwrap();
    assert_eq!(entries, 1);
    assert_eq!(result, b"actual local fixture result");
    assert_eq!(
        store.native_recovery("coding").unwrap().unwrap().class,
        "EnteredUncertain"
    );
    assert!(claim(&mut store, &session, "next").is_err());
    drop(store);
    let mut reopened = SqliteLifecycleStore::open(root.path(), "issuer".into()).unwrap();
    assert_eq!(
        reopened.native_recovery("coding").unwrap().unwrap().class,
        "EnteredUncertain"
    );
    assert!(claim(&mut reopened, &session, "after-reopen").is_err());
}

#[test]
fn coding_effect_refuses_unbound_changed_expired_revoked_and_wrong_profile() {
    for case in 0..7 {
        let root = tempdir().unwrap();
        let profile = if case == 6 {
            pi_multiturn_profile_digest()
        } else {
            pi_coding_profile_digest()
        };
        let protocol = if case == 6 {
            pi_multiturn_protocol_digest()
        } else {
            pi_coding_protocol_digest()
        };
        let (mut store, session) = setup_binding(root.path(), &profile, &protocol);
        let mut parent = claim(&mut store, &session, "coding").unwrap();
        if case == 0 {
            let intent = json!({});
            let mut entered = false;
            assert!(
                store
                    .with_pi_coding_effect(&parent, "host-wire", 1, &intent, clock(), |_expires| {
                        entered = true;
                        Ok::<_, ()>(())
                    })
                    .is_err()
            );
            assert!(!entered);
            continue;
        }
        store.mark_native_send_intent(&parent).unwrap();
        store
            .bind_native_turn(&mut parent, ProviderTurnId::parse("turn:coding").unwrap())
            .unwrap();
        let mut intent = json!({"job_id":"job","input_digest":"1".repeat(64),"session_id":"physical:pi","call_id":"call:1","command_sha256":"2".repeat(64)});
        let raw = json!({"type":"pi.coding_tool_intent","invocationId":"coding","intent":intent});
        store
            .record_native_ingress(
                session.id(),
                "host-wire",
                1,
                &serde_json::to_vec(&raw).unwrap(),
                "qualified",
                Some("pi.coding_tool_intent"),
                Some("physical:pi"),
                Some("turn:coding"),
                None,
                clock(),
            )
            .unwrap();
        if case == 1 {
            intent["command_sha256"] = json!("3".repeat(64));
        }
        if case == 2 {
            intent["session_id"] = json!("foreign");
        }
        if case == 5 {
            store
                .contract_grant_command(
                    "cancel",
                    &SubjectId::parse("s").unwrap(),
                    "operator",
                    &GrantId::parse("g").unwrap(),
                    Revision::new(1),
                    b"cancel",
                    clock(),
                )
                .unwrap();
        }
        let mut sample = clock();
        if case == 3 {
            sample.wall = WallTimeMs::new(1000);
        }
        if case == 4 {
            sample.wall = WallTimeMs::new(99);
        }
        let mut entered = false;
        assert!(
            store
                .with_pi_coding_effect(&parent, "host-wire", 1, &intent, sample, |_expires| {
                    entered = true;
                    Ok::<_, ()>(())
                })
                .is_err(),
            "case {case}"
        );
        assert!(!entered, "case {case}");
        assert_eq!(
            store.native_recovery("coding").unwrap().unwrap().class,
            "EnteredUncertain"
        );
    }
}

fn coding_events(inv: &str) -> Vec<Value> {
    let command = "bounded fixture edit";
    let id = "one";
    let output = "Host command success: true\nedited\n[stderr]\n";
    let result = json!({"type":"tool_result","id":id,"ok":true,"output":output});
    let artifact_bytes = serde_json::to_string_pretty(&result).unwrap() + "\n";
    let intent = json!({"job_id":"job","input_digest":"1".repeat(64),"session_id":"physical:pi","call_id":id,"command_sha256":hash(command.as_bytes())});
    let owned = json!({"type":"pi.coding_tool_result","invocationId":inv,"intent":intent,"result":result,"artifact":{"path":"/fixture/result.json","sha256":hash(artifact_bytes.as_bytes())},"artifact_bytes":artifact_bytes,"stdout_bytes":b"edited".to_vec(),"stderr_bytes":[],"process":{"argv":["/fixture/docker","exec","--workdir","/workspace","tool","/bin/sh","-c",command],"pid":1,"start":"fixture","status":0,"timed_out":false,"leftovers":[],"stdout_sha256":hash(b"edited"),"stderr_sha256":hash(b"")},"tool_container":"f".repeat(64)});
    let workspace = json!({"type":"pi.coding_workspace","invocationId":inv,"manifest":{"path":"/fixture/workspace.json","sha256":hash(b"{}")},"manifest_bytes":"{}"});
    let checkpoint=json!({"successor_context":"context:new","coding_result":result,"coding_result_sha256":owned["artifact"]["sha256"],"workspace_manifest_sha256":workspace["manifest"]["sha256"],"tool_executions":1}).to_string();
    let user = json!({"role":"user","content":[{"type":"text","text":inv}]});
    let call =
        json!({"type":"toolCall","id":id,"name":"workspace_exec","arguments":{"command":command}});
    let assistant = json!({"role":"assistant","content":[call],"stopReason":"toolUse"});
    let tool = json!({"role":"toolResult","toolCallId":id,"toolName":"workspace_exec","content":[{"type":"text","text":output}],"details":{"hostReportedSuccess":true},"isError":false});
    let final_message = json!({"role":"assistant","content":[{"type":"text","text":checkpoint}],"stopReason":"stop"});
    let full = vec![
        user.clone(),
        assistant.clone(),
        tool.clone(),
        final_message.clone(),
    ];
    let request = |input: Vec<Value>| json!({"type":"pi.provider_request","producer":"pi-1.5.0","sessionId":"physical:pi","invocationId":inv,"turnCorrelation":format!("turn:{inv}"),"prompt_sha256":hash(inv.as_bytes()),"messages":input,"agentMessages":input});
    let mut branch = vec![];
    let mut parent = Value::Null;
    for (i, message) in full.iter().enumerate() {
        let id = format!("{inv}:{i}");
        branch.push(json!({"type":"message","id":id,"parentId":parent,"message":message}));
        parent = json!(id);
    }
    vec![
        json!({"type":"agent_start"}),
        json!({"type":"turn_start"}),
        json!({"type":"message_start","message":user}),
        json!({"type":"message_end","message":user}),
        request(vec![user.clone()]),
        json!({"type":"message_start","message":{"role":"assistant"}}),
        json!({"type":"message_end","message":assistant}),
        json!({"type":"tool_execution_start","toolCallId":id,"toolName":"workspace_exec","args":{"command":command}}),
        json!({"type":"pi.coding_tool_intent","invocationId":inv,"intent":intent}),
        owned,
        json!({"type":"tool_execution_end","toolCallId":id,"toolName":"workspace_exec","isError":false,"result":{"content":tool["content"],"details":tool["details"]}}),
        json!({"type":"message_start","message":tool}),
        json!({"type":"message_end","message":tool}),
        json!({"type":"turn_end","message":assistant,"toolResults":[tool]}),
        json!({"type":"turn_start"}),
        workspace,
        request(vec![user.clone(), assistant, tool]),
        json!({"type":"message_start","message":{"role":"assistant"}}),
        json!({"type":"message_end","message":final_message}),
        json!({"type":"turn_end","message":final_message,"toolResults":[]}),
        json!({"type":"agent_end","messages":full,"willRetry":false}),
        json!({"type":"agent_settled","aborted":false}),
        json!({"type":"pi.session_history","producer":"pi-1.5.0","sessionId":"physical:pi","header":{"type":"session","id":"physical:pi"},"messages":full,"branch":branch,"leafId":parent}),
    ]
}
#[test]
fn v3_tool_proof_requires_exact_host_custody_and_sdk_result_and_anchors_next_turn() {
    for case in 0..14 {
        let root = tempdir().unwrap();
        let (mut store, session) = setup_binding(
            root.path(),
            &pi_coding_profile_digest(),
            &pi_coding_protocol_digest(),
        );
        let mut values = coding_events("coding");
        match case {
            1 => values[9]["artifact"]["sha256"] = json!("0".repeat(64)),
            2 => values[9]["process"]["timed_out"] = json!(true),
            3 => values[9]["stdout_bytes"] = json!([0]),
            4 => values[10]["result"]["content"] = json!([]),
            5 => values[8]["intent"]["command_sha256"] = json!("0".repeat(64)),
            6 => {
                values.remove(15);
            }
            7 => {
                values.remove(9);
            }
            8 => {
                values.insert(9, values[8].clone());
            }
            9 => values[16]["agentMessages"] = json!([]),
            10 => values[22]["branch"][1]["parentId"] = json!("foreign"),
            11 => values[15]["manifest_bytes"] = json!("{\"changed\":true}"),
            12 => values[9]["process"]["argv"][7] = json!("different command"),
            13 => {
                fn alter(v: &mut Value) {
                    if v["role"] == "assistant" && v["stopReason"] == "stop" {
                        let mut checkpoint: Value =
                            serde_json::from_str(v["content"][0]["text"].as_str().unwrap())
                                .unwrap();
                        checkpoint["coding_result_sha256"] = json!("0".repeat(64));
                        v["content"][0]["text"] = json!(checkpoint.to_string());
                        return;
                    }
                    match v {
                        Value::Array(a) => {
                            for v in a {
                                alter(v)
                            }
                        }
                        Value::Object(o) => {
                            for v in o.values_mut() {
                                alter(v)
                            }
                        }
                        _ => {}
                    }
                }
                for v in &mut values {
                    alter(v);
                }
            }

            _ => {}
        }
        let (parent, proof) = record(&mut store, &session, "coding", &values, 1);
        let outcome = store.settle_pi_coding(&parent, &proof, clock());
        if case == 0 {
            outcome.unwrap();
            assert!(store.settle_pi_coding(&parent, &proof, clock()).is_err());
            let next = events("next", values.last());
            let (parent, proof) =
                record(&mut store, &session, "next", &next, values.len() as u64 + 1);
            store.settle_pi_coding(&parent, &proof, clock()).unwrap();
            assert_eq!(
                store.native_recovery("next").unwrap().unwrap().class,
                "CompletedSettled"
            );
        } else {
            assert!(outcome.is_err(), "case {case}");
            assert_eq!(
                store.native_recovery("coding").unwrap().unwrap().class,
                "EnteredUncertain"
            );
        }
    }
}

#[test]
fn coding_recovery_envelope_does_not_expand_domain_or_foreign_profile_prompts() {
    for (profile, protocol) in [
        (pi_coding_profile_digest(), pi_coding_protocol_digest()),
        (
            pi_multiturn_profile_digest(),
            pi_multiturn_protocol_digest(),
        ),
    ] {
        let root = tempdir().unwrap();
        let (mut store, session) = setup_binding(root.path(), &profile, &protocol);
        assert!(matches!(
            store.claim_native_turn(
                &session,
                NativeTurnRequest {
                    invocation_id: "oversized-domain".into(),
                    attempt: AttemptId::parse("attempt:oversized").unwrap(),
                    grant: GrantId::parse("g").unwrap(),
                    grant_revision: 1,
                    purpose: NativeInvocationPurpose::DomainWork,
                    prompt: vec![b'x'; 2049],
                },
                "operator",
                clock()
            ),
            Err(StoreError::Rejected)
        ));
        assert!(store.native_recovery("oversized-domain").unwrap().is_none());
    }
}

fn live_events(inv: &str, model: &str) -> Vec<Value> {
    fn swap(v: &mut Value, from: &str, to: &str) {
        match v {
            Value::String(s) if s == from => *s = to.into(),
            Value::Array(a) => a.iter_mut().for_each(|v| swap(v, from, to)),
            Value::Object(o) => o.values_mut().for_each(|v| swap(v, from, to)),
            _ => {}
        }
    }
    let mut values = coding_events(inv);
    let original = values[18]["message"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    values.iter_mut().for_each(|v| swap(v, &original, model));
    assert_eq!(values[18]["message"]["content"][0]["text"], model);
    values
}
fn open_live(root: &std::path::Path) -> (SqliteLifecycleStore, NativeSessionEntry) {
    setup_binding(
        root,
        &pi_coding_live_profile_digest(),
        &pi_coding_live_protocol_digest(),
    )
}

#[test]
fn v4_keeps_actual_final_text_and_owner_checkpoint_as_distinct_bound_materials_across_reopen() {
    let model = "Done. I edited the file as requested.";
    let root = tempdir().unwrap();
    let (mut store, session) = open_live(root.path());
    let values = live_events("coding", model);
    let (parent, proof) = record(&mut store, &session, "coding", &values, 1);
    store.settle_pi_coding(&parent, &proof, clock()).unwrap();
    let history_messages = values[22]["messages"].clone();
    let verify = |store: &mut SqliteLifecycleStore| -> String {
        // Generic projection: version 1, actual model text and its digest.
        let material = store.native_final_material("coding").unwrap().unwrap();
        assert_eq!(material.version, 1);
        assert_eq!(material.final_text, model.as_bytes());
        assert_eq!(material.final_sha256, hash(model.as_bytes()));
        // Owner checkpoint: separate bytes bound to the same invocation,
        // the same final digest and the same validated history.
        let checkpoint = store.pi_coding_live_checkpoint(&parent).unwrap();
        assert_ne!(checkpoint.as_bytes(), material.final_text.as_slice());
        let value: Value = serde_json::from_str(&checkpoint).unwrap();
        assert_eq!(value["profile"], "work-engine.pi-coding-live-continuity.v1");
        let turn = &value["settled_turn"];
        assert_eq!(turn["invocation_id"], "coding");
        assert_eq!(turn["assistant_final_text_sha256"], material.final_sha256);
        assert_eq!(turn["history_messages"], 4);
        assert_eq!(
            turn["history_messages_sha256"],
            hash(&serde_json::to_vec(&history_messages).unwrap())
        );
        assert_eq!(value["coding_result"], values[9]["result"]);
        assert_eq!(
            value["coding_result_sha256"],
            values[9]["artifact"]["sha256"]
        );
        assert!(!checkpoint.contains(model));
        checkpoint
    };
    let first = verify(&mut store);
    // A permitted duplicate at a new coordinate cannot extend the evidence
    // slice committed by the settled invocation.
    store
        .record_native_ingress(
            session.id(),
            "wire",
            values.len() as u64 + 1,
            &serde_json::to_vec(values.last().unwrap()).unwrap(),
            "qualified",
            Some("pi.session_history"),
            Some("physical:pi"),
            Some("turn:coding"),
            None,
            clock(),
        )
        .unwrap();
    assert_eq!(
        store.native_recovery("coding").unwrap().unwrap().class,
        "CompletedSettled"
    );
    assert_eq!(verify(&mut store), first);
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "issuer".into()).unwrap();
    assert_eq!(verify(&mut store), first);
    // The slice bound does not hide genuinely conflicting late evidence:
    // normal owner conflict fencing still invalidates dependent material.
    assert_eq!(
        store
            .record_native_ingress(
                session.id(),
                "wire",
                values.len() as u64 + 2,
                br#"{"type":"turn/failed"}"#,
                "qualified",
                Some("turn/failed"),
                Some("physical:pi"),
                Some("turn:coding"),
                None,
                clock(),
            )
            .unwrap(),
        NativeIngressApply::ConflictFenced
    );
    assert!(store.pi_coding_live_checkpoint(&parent).is_err());
}

#[test]
fn v4_tool_checkpoint_requires_initial_emission_boundary() {
    let root = tempdir().unwrap();
    let (mut store, session) = open_live(root.path());
    let values = live_events("coding", "Done.");
    let (parent, proof) = record(&mut store, &session, "coding", &values, 2);
    assert!(store.settle_pi_coding(&parent, &proof, clock()).is_err());
    assert_eq!(
        store.native_recovery("coding").unwrap().unwrap().class,
        "EnteredUncertain"
    );
}

#[test]
fn v4_checkpoint_refuses_tampered_material_and_other_profiles_are_unchanged() {
    let model = "Done. I edited the file as requested.";
    // Tampering with the committed final text breaks the binding to the
    // validated evidence; the checkpoint is refused rather than re-served.
    let root = tempdir().unwrap();
    let (mut store, session) = open_live(root.path());
    let values = live_events("coding", model);
    let (parent, proof) = record(&mut store, &session, "coding", &values, 1);
    store.settle_pi_coding(&parent, &proof, clock()).unwrap();
    store.pi_coding_live_checkpoint(&parent).unwrap();
    drop(store);
    let conn = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    conn.execute(
        "UPDATE native_turns SET final_text=?1 WHERE invocation_id='coding'",
        [b"substituted final text".as_slice()],
    )
    .unwrap();
    drop(conn);
    let mut store = SqliteLifecycleStore::open(root.path(), "issuer".into()).unwrap();
    assert!(store.pi_coding_live_checkpoint(&parent).is_err());
    // Tampered immutable ingress (the validated Host result) is refused too.
    let root = tempdir().unwrap();
    let (mut store, session) = open_live(root.path());
    let (parent, proof) = record(&mut store, &session, "coding", &values, 1);
    store.settle_pi_coding(&parent, &proof, clock()).unwrap();
    drop(store);
    let conn = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    conn.execute(
        "UPDATE native_ingress SET raw=?1 WHERE sequence=10",
        [serde_json::to_vec(&json!({"type":"tool_execution_end","toolCallId":"one","toolName":"workspace_exec","isError":true,"result":{}})).unwrap()],
    )
    .unwrap();
    drop(conn);
    let mut store = SqliteLifecycleStore::open(root.path(), "issuer".into()).unwrap();
    assert!(store.pi_coding_live_checkpoint(&parent).is_err());
    // v3 (fixture) keeps its contract: the final text is the checkpoint and the
    // v4 accessor refuses it.
    let root = tempdir().unwrap();
    let (mut store, session) = setup_binding(
        root.path(),
        &pi_coding_profile_digest(),
        &pi_coding_protocol_digest(),
    );
    let fixture = coding_events("coding");
    let (parent, proof) = record(&mut store, &session, "coding", &fixture, 1);
    store.settle_pi_coding(&parent, &proof, clock()).unwrap();
    let material = store.native_final_material("coding").unwrap().unwrap();
    assert_eq!(
        material.final_text,
        fixture[18]["message"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .as_bytes()
    );
    assert!(store.pi_coding_live_checkpoint(&parent).is_err());
}

#[test]
fn sol_high_v5_preserves_thinking_history_and_rejects_cross_profile_or_bad_blocks() {
    fn inject(v: &mut Value, malformed: bool) {
        match v {
            Value::Array(a) => a.iter_mut().for_each(|v| inject(v, malformed)),
            Value::Object(o) => {
                if o.get("role").is_some_and(|r| r == "assistant") {
                    if let Some(Value::Array(a)) = o.get_mut("content") {
                        a.insert(0, json!({"type":"thinking","thinking":"fixture reasoning", "thinkingSignature":if malformed {"not-json"} else {"{\"type\":\"reasoning\",\"id\":\"r-fixture\",\"encrypted_content\":\"fixture\"}"}}));
                    }
                }
                o.values_mut().for_each(|v| inject(v, malformed));
            }
            _ => {}
        }
    }
    for (sol, malformed) in [(true, false), (false, false), (true, true)] {
        let root = tempdir().unwrap();
        let (mut store, session) = if sol {
            setup_binding(
                root.path(),
                &pi_coding_sol_high_profile_digest(),
                &pi_coding_sol_high_protocol_digest(),
            )
        } else {
            open_live(root.path())
        };
        let mut values = live_events("coding", "Actual final text.");
        values.iter_mut().for_each(|v| inject(v, malformed));
        let (parent, proof) = record(&mut store, &session, "coding", &values, 1);
        let settled = store.settle_pi_coding(&parent, &proof, clock());
        if !sol || malformed {
            assert!(settled.is_err());
            assert!(store.native_final_material("coding").unwrap().is_none());
        } else {
            settled.unwrap();
            assert_eq!(
                store
                    .native_final_material("coding")
                    .unwrap()
                    .unwrap()
                    .final_text,
                b"Actual final text."
            );
            let checkpoint = store.pi_coding_live_checkpoint(&parent).unwrap();
            let value: Value = serde_json::from_str(&checkpoint).unwrap();
            assert_eq!(
                value["profile"],
                "work-engine.pi-coding-sol-high-continuity.v1"
            );
            assert_eq!(
                value["settled_turn"]["history_messages_sha256"],
                hash(&serde_json::to_vec(&values[22]["messages"]).unwrap())
            );
            drop(store);
            let mut reopened = SqliteLifecycleStore::open(root.path(), "issuer".into()).unwrap();
            assert_eq!(
                reopened.pi_coding_live_checkpoint(&parent).unwrap(),
                checkpoint
            );
        }
    }
}
