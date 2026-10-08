use ed25519_dalek::{Signer, SigningKey};
use lifecycle_admin::recovery::{inspect_exact, retire_exact_no_retry};
use lifecycle_core::{
    AttemptId, AuthorityExpiresAt, BuildId, ClockSample, ContextGeneration, GrantId, GrantScope,
    NativeInvocationPurpose, NativeSessionId, ProofRunId, ProviderThreadId, ProviderTurnId,
    Revision, RuntimeIncarnation, SnapshotId, SubjectId, TrustedGrant, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::{
    DispositionGrant, EffectCoverage, ExactAttemptSelector, NativeIngressApply, NativeTurnRequest,
    SignedDispositionRequest, SqliteLifecycleStore,
};
use tempfile::tempdir;

fn clock(wall: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(wall),
        wait_budget: WaitBudgetMs::new(100),
    }
}

fn fixture_sign(bytes: &[u8]) -> Vec<u8> {
    let hex = include_str!("fixtures/ed25519-rfc8032-vector1-seed.hex").trim();
    let seed: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
        .collect();
    SigningKey::from_bytes(&seed.try_into().unwrap())
        .sign(bytes)
        .to_bytes()
        .to_vec()
}

fn late_native_conflict_after_signed_retirement_preserves_original_evidence(
    late_method: &str,
    late_status: &str,
) {
    let temporary = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(temporary.path(), "test-issuer".into()).unwrap();
    let subject = SubjectId::parse("subject-late-conflict").unwrap();
    let context = ContextGeneration::parse("context-late-conflict").unwrap();
    let build = BuildId::parse("build-late-conflict").unwrap();
    let proof = ProofRunId::parse("proof-late-conflict").unwrap();
    store
        .register_subject(
            subject.clone(),
            context.clone(),
            build.clone(),
            proof.clone(),
            clock(100),
        )
        .unwrap();
    for (id, scope) in [
        ("grant-native-late", GrantScope::NativeTurn),
        ("grant-retire-late", GrantScope::RetireNoRetry),
    ] {
        store
            .install_trusted_grant(TrustedGrant {
                id: GrantId::parse(id).unwrap(),
                issuer: "test-issuer".into(),
                principal: "operator:late".into(),
                subject: subject.clone(),
                context: context.clone(),
                build: build.clone(),
                proof_run: proof.clone(),
                scope,
                expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
    }
    let digest = "a".repeat(64);
    let session_id = NativeSessionId::parse("session-late").unwrap();
    let incarnation = RuntimeIncarnation::parse("incarnation-late").unwrap();
    let mut session = store
        .reserve_native_session(
            session_id.clone(),
            subject.clone(),
            context,
            incarnation.clone(),
            &digest,
            &digest,
            &digest,
            SnapshotId::parse("snapshot-late").unwrap(),
            &digest,
            false,
            clock(101),
        )
        .unwrap();
    store
        .bind_native_thread(
            &mut session,
            ProviderThreadId::parse("thread-late").unwrap(),
        )
        .unwrap();
    let mut turn = store
        .claim_native_turn(
            &session,
            NativeTurnRequest {
                invocation_id: "invocation-late".into(),
                attempt: AttemptId::parse("attempt-late").unwrap(),
                grant: GrantId::parse("grant-native-late").unwrap(),
                grant_revision: 1,
                purpose: NativeInvocationPurpose::DomainWork,
                prompt: b"fixture prompt".to_vec(),
            },
            "operator:late",
            clock(102),
        )
        .unwrap();
    store.mark_native_send_intent(&turn).unwrap();
    store
        .bind_native_turn(&mut turn, ProviderTurnId::parse("turn-late").unwrap())
        .unwrap();
    let final_item = serde_json::json!({"type":"agentMessage","id":"final-late","text":"done"});
    let item = serde_json::json!({"method":"item/completed","params":{
        "threadId":"thread-late","turnId":"turn-late","item":final_item.clone()
    }});
    let completed = serde_json::json!({"method":"turn/completed","params":{
        "threadId":"thread-late","turn":{"id":"turn-late","status":"completed"}
    }});
    let history = serde_json::json!({"id":3,"result":{"thread":{
        "id":"thread-late","historyMode":"legacy","turns":[{
            "id":"turn-late","status":"completed","itemsView":"full","items":[final_item]
        }]
    }}});
    for (sequence, raw, method, thread, native_turn) in [
        (
            1,
            serde_json::to_vec(&item).unwrap(),
            Some("item/completed"),
            Some("thread-late"),
            Some("turn-late"),
        ),
        (
            2,
            serde_json::to_vec(&completed).unwrap(),
            Some("turn/completed"),
            Some("thread-late"),
            Some("turn-late"),
        ),
        (3, serde_json::to_vec(&history).unwrap(), None, None, None),
    ] {
        store
            .record_native_ingress(
                &session_id,
                "transport-late",
                sequence,
                &raw,
                "qualified",
                method,
                thread,
                native_turn,
                None,
                clock(103),
            )
            .unwrap();
    }
    store
        .settle_native_turn(
            &turn,
            "done",
            "observed_complete",
            "transport-late",
            3,
            clock(104),
        )
        .unwrap();
    let projection = store.subject_projection(&subject, None, None).unwrap();
    let selector = ExactAttemptSelector {
        store_id: projection.store_id,
        source_lineage: None,
        subject_id: subject.as_str().into(),
        input_id: None,
        delivery_id: None,
        effect_id: None,
        attempt_id: Some("attempt-late".into()),
        native_session_id: Some(session_id.as_str().into()),
        invocation_id: Some("invocation-late".into()),
        incarnation_id: Some(incarnation.as_str().into()),
        operation_contract_id: None,
        operation_implementation_id: None,
        operation_executor_id: None,
        profile_digest: Some(digest),
        claim_revision: projection.revision,
        observation_revision: 0,
    };
    let before = inspect_exact(&store, &selector).unwrap();
    assert_eq!(before.effect_coverage, EffectCoverage::AccountedNativeTurn);
    assert_eq!(before.outcome.as_deref(), Some("completed"));
    assert_eq!(before.settlement.as_deref(), Some("established"));
    let grant = DispositionGrant {
        codec: "json-exact-ed25519.v1".into(),
        domain: "lifecycle.disposition.v1".into(),
        grant_id: "grant-retire-late".into(),
        grant_revision: 1,
        issuer: "test-issuer".into(),
        principal: "operator:late".into(),
        store_id: selector.store_id.clone(),
        subject_id: selector.subject_id.clone(),
        claim_id: "native:invocation-late".into(),
        observation_revision: 0,
        scope: "retire_no_retry".into(),
        expires_wall_ms: 10_000,
        effect_coverage: EffectCoverage::AccountedNativeTurn,
    };
    let bytes = serde_json::to_vec(&grant).unwrap();
    let request = SignedDispositionRequest {
        command_id: "retire-late".into(),
        selector: selector.clone(),
        grant_bytes: bytes.clone(),
        signature: fixture_sign(&bytes),
        key_id: "test-rfc8032-1".into(),
        clock_wall_ms: 200,
        expected_store_fence_revision: store.admin_ownership().unwrap().revision,
    };
    let retired = retire_exact_no_retry(&mut store, &request).unwrap();
    assert_eq!(retired.original_outcome.as_deref(), Some("completed"));
    assert_eq!(retired.original_settlement.as_deref(), Some("established"));
    let fresh_contradiction = serde_json::json!({"method":late_method,"params":{
        "threadId":"thread-late","turn":{"id":"turn-late","status":late_status}
    }});
    assert_eq!(
        store
            .record_native_ingress(
                &session_id,
                "transport-late",
                4,
                &serde_json::to_vec(&fresh_contradiction).unwrap(),
                "qualified",
                Some(late_method),
                Some("thread-late"),
                Some("turn-late"),
                None,
                clock(201),
            )
            .unwrap(),
        NativeIngressApply::ConflictFenced
    );
    let mut fresh_selector = selector.clone();
    fresh_selector.claim_revision = store
        .subject_projection(&subject, None, None)
        .unwrap()
        .revision;
    let fresh = inspect_exact(&store, &fresh_selector).unwrap();
    assert!(
        !fresh.conflicts.is_empty(),
        "fresh-coordinate late adverse terminal must expose an incident"
    );
    assert_eq!(fresh.disposition.as_deref(), Some("retired_no_retry"));
    assert_eq!(fresh.disposition_command_id.as_deref(), Some("retire-late"));
    assert_eq!(fresh.outcome, before.outcome);
    assert_eq!(fresh.retired_original_outcome.as_deref(), Some("completed"));
    assert_eq!(
        fresh.retired_original_settlement.as_deref(),
        Some("established")
    );
    assert!(
        store
            .claim_native_turn(
                &session,
                NativeTurnRequest {
                    invocation_id: "invocation-late-followup".into(),
                    attempt: AttemptId::parse("attempt-late-followup").unwrap(),
                    grant: GrantId::parse("grant-native-late").unwrap(),
                    grant_revision: 1,
                    purpose: NativeInvocationPurpose::DomainWork,
                    prompt: b"followup after contradiction".to_vec(),
                },
                "operator:late",
                clock(202),
            )
            .is_err(),
        "late contradiction must fence followup admission"
    );
    let contradictory = serde_json::json!({"method":"turn/completed","params":{
        "threadId":"thread-late","turn":{"id":"turn-late","status":"failed"}
    }});
    assert_eq!(
        store
            .record_native_ingress(
                &session_id,
                "transport-late",
                2,
                &serde_json::to_vec(&contradictory).unwrap(),
                "qualified",
                Some("turn/completed"),
                Some("thread-late"),
                Some("turn-late"),
                None,
                clock(203),
            )
            .unwrap(),
        NativeIngressApply::ConflictFenced
    );
    let mut current_selector = selector;
    current_selector.claim_revision = store
        .subject_projection(&subject, None, None)
        .unwrap()
        .revision;
    let after = inspect_exact(&store, &current_selector).unwrap();
    assert_eq!(after.disposition.as_deref(), Some("retired_no_retry"));
    assert_eq!(after.disposition_command_id.as_deref(), Some("retire-late"));
    assert_eq!(after.outcome, before.outcome);
    assert_eq!(retired.original_settlement.as_deref(), Some("established"));
    assert_eq!(after.settlement.as_deref(), Some("unresolved"));
    assert_eq!(after.retired_original_outcome.as_deref(), Some("completed"));
    assert_eq!(
        after.retired_original_settlement.as_deref(),
        Some("established")
    );
    assert!(!after.conflicts.is_empty());
    assert_eq!(
        retire_exact_no_retry(&mut store, &request).unwrap(),
        retired
    );
    drop(store);
    let mut reopened = SqliteLifecycleStore::open(temporary.path(), "test-issuer".into()).unwrap();
    let recovered = inspect_exact(&reopened, &current_selector).unwrap();
    assert_eq!(recovered.disposition.as_deref(), Some("retired_no_retry"));
    assert_eq!(
        recovered.retired_original_outcome.as_deref(),
        Some("completed")
    );
    assert_eq!(
        recovered.retired_original_settlement.as_deref(),
        Some("established")
    );
    assert!(!recovered.conflicts.is_empty());
    assert!(
        reopened
            .claim_native_turn(
                &session,
                NativeTurnRequest {
                    invocation_id: "invocation-after-reopen".into(),
                    attempt: AttemptId::parse("attempt-after-reopen").unwrap(),
                    grant: GrantId::parse("grant-native-late").unwrap(),
                    grant_revision: 1,
                    purpose: NativeInvocationPurpose::DomainWork,
                    prompt: b"followup after reopen".to_vec(),
                },
                "operator:late",
                clock(204),
            )
            .is_err(),
        "late incident fence must survive store reopen"
    );
}

#[test]
fn late_failed_turn_after_signed_retirement_fences_followup() {
    late_native_conflict_after_signed_retirement_preserves_original_evidence(
        "turn/failed",
        "failed",
    );
}

#[test]
fn late_interrupted_turn_after_signed_retirement_fences_followup() {
    late_native_conflict_after_signed_retirement_preserves_original_evidence(
        "turn/interrupted",
        "interrupted",
    );
}
