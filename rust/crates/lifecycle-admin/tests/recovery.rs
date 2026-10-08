use ed25519_dalek::{Signer, SigningKey};
use lifecycle_admin::recovery::{inspect_exact, retire_exact_no_retry};
use lifecycle_admin::selection::{
    FileLaunchSelectionPort, LaunchSelectionGuard, LaunchSelectionPort, SelectionRecord,
};
use lifecycle_admin::transfer::resume_transfer;
use lifecycle_core::{
    AttemptId, AuthorityExpiresAt, BuildId, ClockSample, Command, CommandAdmission, CommandId,
    CommandRequest, ContextGeneration, ControlledTextInput, EffectObservation, EffectSettlement,
    EvidenceId, ExecutionOutcome, GrantId, GrantScope, HandoffId, InputId, NativeInvocationPurpose,
    NativeSessionId, ProofRunId, ProviderThreadId, ProviderTurnId, Revision, RuntimeIncarnation,
    SnapshotId, SubjectId, TrustedGrant, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::{
    DispositionGrant, EffectCoverage, ExactAttemptSelector, NativeTurnRequest,
    SignedDispositionRequest, SqliteLifecycleStore,
};
use serde_json::json;
use std::fs;
use tempfile::tempdir;
use work_engine_types::CodecContract;

fn id<T>(text: &str, parse: impl FnOnce(String) -> Result<T, work_engine_types::IdError>) -> T {
    parse(text.to_owned()).unwrap()
}

fn clock(wall: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(wall),
        wait_budget: WaitBudgetMs::new(100),
    }
}

fn bootstrap(store: &mut SqliteLifecycleStore) -> SubjectId {
    let subject = id("subject-s7", SubjectId::parse);
    store
        .register_subject(
            subject.clone(),
            id("context-s7", ContextGeneration::parse),
            id("build-s7", BuildId::parse),
            id("proof-s7", ProofRunId::parse),
            clock(100),
        )
        .unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: id("grant-s7", GrantId::parse),
            issuer: "test-issuer".into(),
            principal: "operator:controlled".into(),
            subject: subject.clone(),
            context: id("context-s7", ContextGeneration::parse),
            build: id("build-s7", BuildId::parse),
            proof_run: id("proof-s7", ProofRunId::parse),
            scope: GrantScope::EnqueueInput,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    subject
}

fn enqueue(store: &mut SqliteLifecycleStore, subject: &SubjectId) {
    let input_id = id("input-s7", InputId::parse);
    let text = "disposable S7 source";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap();
    let basis = json!({
        "protocol_version":1,
        "principal_ref":"operator:controlled",
        "command_id":"command-s7",
        "subject_id":subject.as_str(),
        "context_generation":"context-s7",
        "build_id":"build-s7",
        "proof_run_id":"proof-s7",
        "grant_ref":"grant-s7",
        "expected_revision":"0",
        "kind":"enqueue_input",
        "payload":{
            "input_id":"input-s7",
            "producer_ref":"fixture:s7",
            "text":text,
            "text_digest":digest.hex()
        }
    });
    let request = CommandRequest::new(
        id("command-s7", CommandId::parse),
        subject.clone(),
        id("context-s7", ContextGeneration::parse),
        id("build-s7", BuildId::parse),
        id("proof-s7", ProofRunId::parse),
        id("grant-s7", GrantId::parse),
        Revision::new(0),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        Command::EnqueueInput {
            input: input_id.clone(),
        },
    )
    .unwrap();
    let admission = CommandAdmission::new(
        request,
        "operator:controlled".into(),
        basis,
        Some(ControlledTextInput::new(input_id, text.into(), digest).unwrap()),
    )
    .unwrap();
    store.apply_checked_command(admission, clock(101)).unwrap();
}

fn selector(
    store: &mut SqliteLifecycleStore,
    subject: &SubjectId,
    effect_id: String,
    attempt_id: Option<String>,
    incarnation_id: Option<String>,
) -> ExactAttemptSelector {
    let projection = store.subject_projection(subject, None, None).unwrap();
    ExactAttemptSelector {
        store_id: projection.store_id,
        source_lineage: None,
        subject_id: subject.as_str().into(),
        input_id: Some("input-s7".into()),
        delivery_id: None,
        effect_id: Some(effect_id),
        attempt_id,
        native_session_id: None,
        invocation_id: None,
        incarnation_id,
        operation_contract_id: None,
        operation_implementation_id: None,
        operation_executor_id: None,
        profile_digest: None,
        claim_revision: projection.revision,
        observation_revision: 0,
    }
}

#[test]
fn exact_readback_never_turns_entered_uncertainty_into_replay() {
    let temporary = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(temporary.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    enqueue(&mut store, &subject);
    let plan = store.prepare_next_input(&subject).unwrap();
    let prepared = selector(
        &mut store,
        &subject,
        plan.effect.as_str().into(),
        None,
        None,
    );
    let before = inspect_exact(&store, &prepared).unwrap();
    assert_eq!(before.classification, "PreparedNeedsGrant");
    assert_eq!(before.outcome, None);

    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-s7", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    let entered = selector(
        &mut store,
        &subject,
        plan.effect.as_str().into(),
        Some(entry.attempt().as_str().into()),
        Some(entry.incarnation().as_str().into()),
    );
    drop(store);

    let mut reopened = SqliteLifecycleStore::open(temporary.path(), "test-issuer".into()).unwrap();
    let detail = inspect_exact(&reopened, &entered).unwrap();
    assert_eq!(detail.classification, "EnteredUncertain");
    assert_ne!(detail.outcome.as_deref(), Some("completed"));
    assert!(reopened.prepare_next_input(&subject).is_err());
    assert!(
        reopened
            .claim_prepared_entry(
                &plan.effect,
                id("incarnation-s7-second", RuntimeIncarnation::parse),
                clock(103),
            )
            .is_err()
    );
}

#[test]
fn failed_unresolved_attempt_and_native_process_exit_keep_independent_custody() {
    let temporary = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(temporary.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    enqueue(&mut store, &subject);
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-failed-s7", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    store
        .apply_bound_observation(
            EffectObservation {
                source: id("evidence-failed-s7", EvidenceId::parse),
                effect: plan.effect.clone(),
                attempt: entry.attempt().clone(),
                incarnation: entry.incarnation().clone(),
                provider_thread: id("thread-failed-s7", ProviderThreadId::parse),
                provider_turn: id("turn-failed-s7", ProviderTurnId::parse),
                outcome: ExecutionOutcome::Failed,
                settlement: EffectSettlement::Unresolved,
            },
            clock(103),
        )
        .unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: id("grant-native-exit-s7", GrantId::parse),
            issuer: "test-issuer".into(),
            principal: "operator:controlled".into(),
            subject: subject.clone(),
            context: id("context-s7", ContextGeneration::parse),
            build: id("build-s7", BuildId::parse),
            proof_run: id("proof-s7", ProofRunId::parse),
            scope: GrantScope::NativeTurn,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    let session_id = id("session-exit-s7", NativeSessionId::parse);
    let incarnation = id("incarnation-native-exit-s7", RuntimeIncarnation::parse);
    let digest = "a".repeat(64);
    let mut session = store
        .reserve_native_session(
            session_id.clone(),
            subject.clone(),
            id("context-s7", ContextGeneration::parse),
            incarnation.clone(),
            &digest,
            &digest,
            &digest,
            id("snapshot-exit-s7", SnapshotId::parse),
            &digest,
            false,
            clock(104),
        )
        .unwrap();
    store
        .bind_native_thread(&mut session, id("thread-exit-s7", ProviderThreadId::parse))
        .unwrap();
    let mut turn = store
        .claim_native_turn(
            &session,
            NativeTurnRequest {
                invocation_id: "invocation-exit-s7".into(),
                attempt: id("attempt-exit-s7", AttemptId::parse),
                grant: id("grant-native-exit-s7", GrantId::parse),
                grant_revision: 1,
                purpose: NativeInvocationPurpose::DomainWork,
                prompt: b"disposable exit observation".to_vec(),
            },
            "operator:controlled",
            clock(105),
        )
        .unwrap();
    store.mark_native_send_intent(&turn).unwrap();
    store
        .bind_native_turn(&mut turn, id("turn-exit-s7", ProviderTurnId::parse))
        .unwrap();
    store
        .record_native_process_exit(
            &session_id,
            None,
            Some(1),
            false,
            "protocol_error",
            clock(106),
        )
        .unwrap();

    let failed_selector = selector(
        &mut store,
        &subject,
        plan.effect.as_str().into(),
        Some(entry.attempt().as_str().into()),
        Some(entry.incarnation().as_str().into()),
    );
    let failure = inspect_exact(&store, &failed_selector).unwrap();
    assert_eq!(failure.outcome.as_deref(), Some("failed"));
    assert_eq!(failure.settlement.as_deref(), Some("unresolved"));
    assert_eq!(inspect_exact(&store, &failed_selector).unwrap(), failure);
    let projection = store.subject_projection(&subject, None, None).unwrap();
    let native_selector = ExactAttemptSelector {
        store_id: projection.store_id,
        source_lineage: None,
        subject_id: subject.as_str().into(),
        input_id: None,
        delivery_id: None,
        effect_id: None,
        attempt_id: Some("attempt-exit-s7".into()),
        native_session_id: Some(session_id.as_str().into()),
        invocation_id: Some("invocation-exit-s7".into()),
        incarnation_id: Some(incarnation.as_str().into()),
        operation_contract_id: None,
        operation_implementation_id: None,
        operation_executor_id: None,
        profile_digest: Some(digest),
        claim_revision: projection.revision,
        observation_revision: 0,
    };
    let native = inspect_exact(&store, &native_selector).unwrap();
    assert_eq!(native.classification, "EnteredUncertain");
    assert_eq!(native.outcome, None);
    assert_eq!(native.settlement.as_deref(), Some("unresolved"));
    assert_eq!(inspect_exact(&store, &native_selector).unwrap(), native);
    drop(store);
    let mut reopened = SqliteLifecycleStore::open(temporary.path(), "test-issuer".into()).unwrap();
    assert_eq!(inspect_exact(&reopened, &failed_selector).unwrap(), failure);
    assert_eq!(inspect_exact(&reopened, &native_selector).unwrap(), native);
    assert!(
        reopened
            .claim_prepared_entry(
                &plan.effect,
                id("incarnation-failed-replay-s7", RuntimeIncarnation::parse),
                clock(107),
            )
            .is_err()
    );
}

fn fixture_sign(bytes: &[u8]) -> Vec<u8> {
    let hex = include_str!("fixtures/ed25519-rfc8032-vector1-seed.hex").trim();
    let seed: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
        .collect();
    let key = SigningKey::from_bytes(&seed.try_into().unwrap());
    key.sign(bytes).to_bytes().to_vec()
}

#[test]
fn signed_exact_prepared_retirement_keeps_original_outcome_and_rejects_forgery() {
    let temporary = tempdir().unwrap();
    let mut store =
        SqliteLifecycleStore::open(&temporary.path().join("source"), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .install_trusted_grant(TrustedGrant {
            id: id("grant-retire-s7", GrantId::parse),
            issuer: "test-issuer".into(),
            principal: "operator:controlled".into(),
            subject: subject.clone(),
            context: id("context-s7", ContextGeneration::parse),
            build: id("build-s7", BuildId::parse),
            proof_run: id("proof-s7", ProofRunId::parse),
            scope: GrantScope::RetireNoRetry,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    enqueue(&mut store, &subject);
    let plan = store.prepare_next_input(&subject).unwrap();
    let exact = selector(
        &mut store,
        &subject,
        plan.effect.as_str().into(),
        None,
        None,
    );
    let detail = inspect_exact(&store, &exact).unwrap();
    assert_eq!(detail.effect_coverage, EffectCoverage::KnownNotEntered);
    let grant = DispositionGrant {
        codec: "json-exact-ed25519.v1".into(),
        domain: "lifecycle.disposition.v1".into(),
        grant_id: "grant-retire-s7".into(),
        grant_revision: 1,
        issuer: "test-issuer".into(),
        principal: "operator:controlled".into(),
        store_id: exact.store_id.clone(),
        subject_id: exact.subject_id.clone(),
        claim_id: format!("effect:{}", plan.effect.as_str()),
        observation_revision: exact.observation_revision,
        scope: "retire_no_retry".into(),
        expires_wall_ms: 10_000,
        effect_coverage: EffectCoverage::KnownNotEntered,
    };
    let grant_bytes = serde_json::to_vec(&grant).unwrap();
    let request = SignedDispositionRequest {
        command_id: "retire-s7".into(),
        selector: exact.clone(),
        grant_bytes: grant_bytes.clone(),
        signature: fixture_sign(&grant_bytes),
        key_id: "test-rfc8032-1".into(),
        clock_wall_ms: 200,
        expected_store_fence_revision: store.admin_ownership().unwrap().revision,
    };
    let mut wrong_key = request.clone();
    wrong_key.command_id = "wrong-key".into();
    wrong_key.key_id = "request-installed-key".into();
    assert!(retire_exact_no_retry(&mut store, &wrong_key).is_err());
    let mut forged = request.clone();
    forged.command_id = "forged-signature".into();
    forged.signature[0] ^= 1;
    assert!(retire_exact_no_retry(&mut store, &forged).is_err());
    let mut expired = request.clone();
    expired.command_id = "expired-grant".into();
    expired.clock_wall_ms = 10_001;
    assert!(retire_exact_no_retry(&mut store, &expired).is_err());
    let mut rolled_back_clock = request.clone();
    rolled_back_clock.command_id = "rolled-back-clock".into();
    rolled_back_clock.clock_wall_ms = 50;
    assert!(retire_exact_no_retry(&mut store, &rolled_back_clock).is_err());
    store
        .install_trusted_grant(TrustedGrant {
            id: id("grant-retire-revoked", GrantId::parse),
            issuer: "test-issuer".into(),
            principal: "operator:controlled".into(),
            subject: subject.clone(),
            context: id("context-s7", ContextGeneration::parse),
            build: id("build-s7", BuildId::parse),
            proof_run: id("proof-s7", ProofRunId::parse),
            scope: GrantScope::RetireNoRetry,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    store
        .revoke_trusted_grant(
            &id("grant-retire-revoked", GrantId::parse),
            Revision::new(1),
            clock(102),
        )
        .unwrap();
    let mut revoked_grant = grant.clone();
    revoked_grant.grant_id = "grant-retire-revoked".into();
    let revoked_bytes = serde_json::to_vec(&revoked_grant).unwrap();
    let mut revoked = request.clone();
    revoked.command_id = "revoked-grant".into();
    revoked.signature = fixture_sign(&revoked_bytes);
    revoked.grant_bytes = revoked_bytes;
    assert!(retire_exact_no_retry(&mut store, &revoked).is_err());
    let mut wrong_domain_grant = grant.clone();
    wrong_domain_grant.domain = "lifecycle.disposition.v2".into();
    let wrong_domain_bytes = serde_json::to_vec(&wrong_domain_grant).unwrap();
    let mut wrong_domain = request.clone();
    wrong_domain.command_id = "wrong-domain".into();
    wrong_domain.signature = fixture_sign(&wrong_domain_bytes);
    wrong_domain.grant_bytes = wrong_domain_bytes;
    assert!(retire_exact_no_retry(&mut store, &wrong_domain).is_err());
    let mut wrong_observation_grant = grant.clone();
    wrong_observation_grant.observation_revision += 1;
    let wrong_observation_bytes = serde_json::to_vec(&wrong_observation_grant).unwrap();
    let mut wrong_observation = request.clone();
    wrong_observation.command_id = "wrong-observation".into();
    wrong_observation.signature = fixture_sign(&wrong_observation_bytes);
    wrong_observation.grant_bytes = wrong_observation_bytes;
    assert!(retire_exact_no_retry(&mut store, &wrong_observation).is_err());

    let capsule_before_retirement = temporary.path().join("capsule-before-retirement");
    let exported_before_retirement = store.export_current(&capsule_before_retirement).unwrap();
    assert_eq!(exported_before_retirement.source_schema, 4);
    let stale_handoff = HandoffId::parse("handoff-before-retirement").unwrap();
    let (_inactive, stale_adoption) = SqliteLifecycleStore::adopt_inactive(
        &exported_before_retirement,
        &temporary.path().join("destination-before-retirement"),
        &stale_handoff,
        "test-issuer".into(),
    )
    .unwrap();
    let accepted = retire_exact_no_retry(&mut store, &request).unwrap();
    assert!(
        store.export_current(&capsule_before_retirement).is_err(),
        "a signed retirement must invalidate an earlier complete capsule"
    );
    assert!(
        store.fence_source(&stale_handoff, &stale_adoption).is_err(),
        "a pre-retirement inactive destination must not fence the changed source"
    );
    let selector_root = temporary.path().join("selector-before-retirement");
    fs::create_dir(&selector_root).unwrap();
    let expected_selection = SelectionRecord {
        generation: 0,
        store_id: store.admin_store_identity().unwrap(),
        root: store.root().to_owned(),
        handoff_id: None,
        capsule_digest: None,
    };
    let mut selection =
        FileLaunchSelectionPort::initialize(&selector_root, expected_selection.clone()).unwrap();
    assert!(
        resume_transfer(
            &mut store,
            &mut selection,
            &expected_selection,
            &capsule_before_retirement,
            &temporary.path().join("destination-before-retirement"),
            &stale_handoff,
            "test-issuer".into(),
        )
        .is_err(),
        "resume must not promote a capsule invalidated by signed retirement"
    );
    assert_eq!(
        selection.lock().unwrap().read().unwrap(),
        expected_selection
    );
    assert_eq!(store.admin_ownership().unwrap().status, "active");
    assert_eq!(accepted.disposition, "retired_no_retry");
    assert_eq!(accepted.original_outcome, None);
    assert_eq!(
        retire_exact_no_retry(&mut store, &request).unwrap(),
        accepted
    );
    let after = inspect_exact(&store, &exact).unwrap();
    assert_ne!(after.outcome.as_deref(), Some("completed"));
    assert_eq!(after.disposition.as_deref(), Some("retired_no_retry"));
    assert_eq!(after.disposition_command_id.as_deref(), Some("retire-s7"));
    assert!(
        store
            .claim_prepared_entry(
                &plan.effect,
                id("incarnation-after-retirement", RuntimeIncarnation::parse),
                clock(201),
            )
            .is_err()
    );
}
