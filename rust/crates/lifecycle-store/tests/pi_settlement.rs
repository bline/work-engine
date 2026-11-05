//! Synthetic protocol-unit fixtures only; they never authorize a transition.
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
fn messages() -> Vec<Value> {
    let u = json!({"role":"user","content":[{"type":"text","text":"prompt"}]});
    let a = json!({"role":"assistant","content":[{"type":"text","text":"fixture"}],"stopReason":"stop"});
    vec![
        json!({"type":"agent_start"}),
        json!({"type":"turn_start"}),
        json!({"type":"message_start","message":u}),
        json!({"type":"message_end","message":u}),
        json!({"type":"pi.provider_request","producer":"pi-1.5.0","sessionId":"physical:pi","invocationId":"inv","turnCorrelation":"local:1","prompt_sha256":hash(b"prompt"),"messages":[u],"agentMessages":[u]}),
        json!({"type":"message_start","message":{"role":"assistant"}}),
        json!({"type":"message_end","message":a}),
        json!({"type":"turn_end","message":a,"toolResults":[]}),
        json!({"type":"agent_end","messages":[u,a],"willRetry":false}),
        json!({"type":"agent_settled","aborted":false}),
        json!({"type":"pi.session_history","producer":"pi-1.5.0","sessionId":"physical:pi","header":{"type":"session","id":"physical:pi"},"leafId":"b","messages":[u,a],"branch":[{"type":"message","id":"a","parentId":null,"message":u},{"type":"message","id":"b","parentId":"a","message":a}]}),
    ]
}
#[test]
fn owner_rejects_incomplete_contradictory_and_unbound_pi_proofs() {
    for case in 0..19 {
        let root = tempdir().unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "issuer".into()).unwrap();
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
        let profile = if case == 1 {
            "a".repeat(64)
        } else {
            pi_profile_digest()
        };
        let protocol = if case == 2 {
            "b".repeat(64)
        } else {
            pi_protocol_digest()
        };
        assert!(store.reserve_native_session(
            NativeSessionId::parse("wrong-generation-intent").unwrap(),SubjectId::parse("s").unwrap(),
            ContextGeneration::parse("wrong-generation").unwrap(),RuntimeIncarnation::parse("i").unwrap(),
            &profile,PI_SDK_INDEX_SHA256,&protocol,SnapshotId::parse("snap").unwrap(),&"c".repeat(64),false,clock()).is_err());
        let mut session = store
            .reserve_native_session(
                NativeSessionId::parse("native").unwrap(),
                SubjectId::parse("s").unwrap(),
                ContextGeneration::parse("c").unwrap(),
                RuntimeIncarnation::parse("i").unwrap(),
                &profile,
                PI_SDK_INDEX_SHA256,
                &protocol,
                SnapshotId::parse("snap").unwrap(),
                &"c".repeat(64),
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
        assert!(store.claim_native_turn(&session,NativeTurnRequest {
            invocation_id:"stale-grant-attempt".into(),attempt:AttemptId::parse("stale-attempt").unwrap(),
            grant:GrantId::parse("g").unwrap(),grant_revision:0,purpose:NativeInvocationPurpose::DomainWork,prompt:b"prompt".to_vec()
        },"operator",clock()).is_err());
        let mut entry = store
            .claim_native_turn(
                &session,
                NativeTurnRequest {
                    invocation_id: "inv".into(),
                    attempt: AttemptId::parse("attempt").unwrap(),
                    grant: GrantId::parse("g").unwrap(),
                    grant_revision: 1,
                    purpose: NativeInvocationPurpose::DomainWork,
                    prompt: b"prompt".to_vec(),
                },
                "operator",
                clock(),
            )
            .unwrap();
        store.mark_native_send_intent(&entry).unwrap();
        store
            .bind_native_turn(&mut entry, ProviderTurnId::parse("local:1").unwrap())
            .unwrap();
        let mut v = messages();
        match case {
            3 => v[4]["prompt_sha256"] = json!("wrong"),
            4 => v[4]["sessionId"] = json!("wrong"),
            5 => {
                v.remove(8);
            }
            6 => {
                v.insert(8, v[8].clone());
            }
            7 => v.swap(7, 8),
            8 => v[10]["messages"] = json!([]),
            9 => v[10]["branch"] = json!([]),
            10 => v[8]["willRetry"] = json!(true),
            11 => v[7]["toolResults"] = json!([{"role":"toolResult"}]),
            12 => v[10]["branch"][1]["parentId"] = json!("missing"),
            13 => v[4]["messages"] = json!([]),
            14 => v[6]["message"]["stopReason"] = json!("error"),
            18 => {
                v.remove(9);
            }
            _ => {}
        }
        for (n, raw) in v.iter().enumerate() {
            store
                .record_native_ingress(
                    session.id(),
                    "transport",
                    n as u64 + 1,
                    &serde_json::to_vec(raw).unwrap(),
                    "qualified",
                    raw["type"].as_str(),
                    Some("physical:pi"),
                    Some("local:1"),
                    None,
                    clock(),
                )
                .unwrap();
        }
        let mut proof = PiSettlementProof {
            transport: "transport".into(),
            first_sequence: 1,
            history_sequence: v.len() as u64,
        };
        if case == 15 {
            proof.transport = "wrong".into();
        }
        if case == 16 {
            store
                .revoke_trusted_grant(&GrantId::parse("g").unwrap(), Revision::new(1), clock())
                .unwrap();
        }
        let mut settlement_clock = clock();
        if case == 17 {
            settlement_clock.wall = WallTimeMs::new(99);
        }
        let result = store.settle_pi_turn(&entry, &proof, settlement_clock);
        if case == 0 {
            result.unwrap();
            assert!(store.settle_pi_turn(&entry, &proof, clock()).is_err());
        } else {
            assert!(result.is_err(), "case {case} must fail");
            assert_ne!(
                store.native_recovery("inv").unwrap().unwrap().class,
                "CompletedSettled"
            );
        }
    }
}
