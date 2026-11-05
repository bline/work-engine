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
