use std::fs;

use ed25519_dalek::{Signer, SigningKey};
use lifecycle_admin::recovery::retire_exact_no_retry;
use lifecycle_admin::selection::{FileLaunchSelectionPort, SelectionRecord};
use lifecycle_admin::transfer::{resume_transfer, transfer_current};
use lifecycle_core::{
    AuthorityExpiresAt, BuildId, ClockSample, Command, CommandAdmission, CommandId, CommandRequest,
    ContextGeneration, ControlledTextInput, EffectInput, EffectObservation, EffectSettlement,
    EvidenceId, ExecutionOutcome, GrantId, GrantScope, HandoffId, InputId, ProofRunId,
    ProviderThreadId, ProviderTurnId, Revision, RuntimeIncarnation, SubjectId, TrustedGrant,
    WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::{
    DispositionGrant, EffectCoverage, ExactAttemptSelector, SignedDispositionRequest,
    SqliteLifecycleStore,
};
use serde_json::json;
use tempfile::tempdir;
use work_engine_types::CodecContract;

fn clock(wall: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(wall),
        wait_budget: WaitBudgetMs::new(100),
    }
}

fn register_fixture_subject(
    store: &mut SqliteLifecycleStore,
    suffix: &str,
    wall: i64,
) -> SubjectId {
    let subject = SubjectId::parse(format!("subject-{suffix}")).unwrap();
    let context = ContextGeneration::parse(format!("context-{suffix}")).unwrap();
    let build = BuildId::parse(format!("build-{suffix}")).unwrap();
    let proof = ProofRunId::parse(format!("proof-{suffix}")).unwrap();
    store
        .register_subject(
            subject.clone(),
            context.clone(),
            build.clone(),
            proof.clone(),
            clock(wall),
        )
        .unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: GrantId::parse(format!("grant-{suffix}")).unwrap(),
            issuer: "test-issuer".into(),
            principal: "operator:fixture".into(),
            subject: subject.clone(),
            context,
            build,
            proof_run: proof,
            scope: GrantScope::EnqueueInput,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    subject
}

fn enqueue(store: &mut SqliteLifecycleStore, suffix: &str, text: &str, wall: i64) {
    let input_name = format!("input-{suffix}");
    let command_name = format!("command-{suffix}");
    let subject_name = format!("subject-{suffix}");
    let input = InputId::parse(input_name.clone()).unwrap();
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap();
    let basis = json!({
        "protocol_version":1,"principal_ref":"operator:fixture","command_id":command_name,
        "subject_id":subject_name,"context_generation":format!("context-{suffix}"),
        "build_id":format!("build-{suffix}"),"proof_run_id":format!("proof-{suffix}"),
        "grant_ref":format!("grant-{suffix}"),"expected_revision":"0",
        "kind":"enqueue_input","payload":{"input_id":input_name,"producer_ref":"fixture:s7-transfer",
            "text":text,"text_digest":digest.hex()}
    });
    let request = CommandRequest::new(
        CommandId::parse(format!("command-{suffix}")).unwrap(),
        SubjectId::parse(format!("subject-{suffix}")).unwrap(),
        ContextGeneration::parse(format!("context-{suffix}")).unwrap(),
        BuildId::parse(format!("build-{suffix}")).unwrap(),
        ProofRunId::parse(format!("proof-{suffix}")).unwrap(),
        GrantId::parse(format!("grant-{suffix}")).unwrap(),
        Revision::new(0),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        Command::EnqueueInput {
            input: input.clone(),
        },
    )
    .unwrap();
    let admission = CommandAdmission::new(
        request,
        "operator:fixture".into(),
        basis,
        Some(ControlledTextInput::new(input, text.into(), digest).unwrap()),
    )
    .unwrap();
    store.apply_checked_command(admission, clock(wall)).unwrap();
}

fn retire_prepared(
    store: &mut SqliteLifecycleStore,
    suffix: &str,
    source_lineage: Option<String>,
) -> String {
    let subject = SubjectId::parse(format!("subject-{suffix}")).unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: GrantId::parse(format!("retire-grant-{suffix}")).unwrap(),
            issuer: "test-issuer".into(),
            principal: "operator:fixture".into(),
            subject: subject.clone(),
            context: ContextGeneration::parse(format!("context-{suffix}")).unwrap(),
            build: BuildId::parse(format!("build-{suffix}")).unwrap(),
            proof_run: ProofRunId::parse(format!("proof-{suffix}")).unwrap(),
            scope: GrantScope::RetireNoRetry,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    let projection = store.subject_projection(&subject, None, None).unwrap();
    let selector = ExactAttemptSelector {
        store_id: projection.store_id.clone(),
        source_lineage,
        subject_id: subject.as_str().into(),
        input_id: Some(format!("input-{suffix}")),
        delivery_id: None,
        effect_id: Some(plan.effect.as_str().into()),
        attempt_id: None,
        native_session_id: None,
        invocation_id: None,
        incarnation_id: None,
        operation_contract_id: None,
        operation_implementation_id: None,
        operation_executor_id: None,
        profile_digest: None,
        claim_revision: projection.revision,
        observation_revision: 0,
    };
    let grant = DispositionGrant {
        codec: "json-exact-ed25519.v1".into(),
        domain: "lifecycle.disposition.v1".into(),
        grant_id: format!("retire-grant-{suffix}"),
        grant_revision: 1,
        issuer: "test-issuer".into(),
        principal: "operator:fixture".into(),
        store_id: projection.store_id,
        subject_id: subject.as_str().into(),
        claim_id: format!("effect:{}", plan.effect.as_str()),
        observation_revision: 0,
        scope: "retire_no_retry".into(),
        expires_wall_ms: 10_000,
        effect_coverage: EffectCoverage::KnownNotEntered,
    };
    let bytes = serde_json::to_vec(&grant).unwrap();
    let seed_hex = include_str!("fixtures/ed25519-rfc8032-vector1-seed.hex").trim();
    let seed: Vec<u8> = (0..seed_hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&seed_hex[index..index + 2], 16).unwrap())
        .collect();
    let key = SigningKey::from_bytes(&seed.try_into().unwrap());
    let request = SignedDispositionRequest {
        command_id: format!("retire-{suffix}"),
        selector,
        grant_bytes: bytes.clone(),
        signature: key.sign(&bytes).to_bytes().to_vec(),
        key_id: "test-rfc8032-1".into(),
        clock_wall_ms: 250,
        expected_store_fence_revision: store.admin_ownership().unwrap().revision,
    };
    assert_eq!(
        retire_exact_no_retry(store, &request).unwrap().disposition,
        "retired_no_retry"
    );
    plan.effect.as_str().into()
}

#[test]
fn actual_store_capsule_transfer_restart_and_current_state_rollback() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let capsule_one = temporary.path().join("capsule-one");
    let candidate_root = temporary.path().join("candidate");
    let capsule_two = temporary.path().join("capsule-two");
    let rollback_root = temporary.path().join("rollback");
    let selector_root = temporary.path().join("selector");
    fs::create_dir(&selector_root).unwrap();

    let mut source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let subject = register_fixture_subject(&mut source, "transfer", 100);
    let old = source
        .stage_artifact(subject.clone(), b"old committed bytes")
        .unwrap();
    let old = source.publish_artifact(old, clock(101)).unwrap();
    enqueue(&mut source, "transfer", "completed source delivery", 102);
    let completed_plan = source.prepare_next_input(&subject).unwrap();
    let completed_entry = source
        .claim_prepared_entry(
            &completed_plan.effect,
            RuntimeIncarnation::parse("incarnation-completed").unwrap(),
            clock(103),
        )
        .unwrap();
    source
        .apply_bound_observation(
            EffectObservation {
                source: EvidenceId::parse("evidence-completed").unwrap(),
                effect: completed_plan.effect.clone(),
                attempt: completed_entry.attempt().clone(),
                incarnation: completed_entry.incarnation().clone(),
                provider_thread: ProviderThreadId::parse("thread-completed").unwrap(),
                provider_turn: ProviderTurnId::parse("turn-completed").unwrap(),
                outcome: ExecutionOutcome::Completed,
                settlement: EffectSettlement::Established(
                    EvidenceId::parse("settlement-completed").unwrap(),
                ),
            },
            clock(104),
        )
        .unwrap();
    let original = SelectionRecord {
        generation: 0,
        store_id: source.admin_store_identity().unwrap(),
        root: source.root().to_owned(),
        handoff_id: None,
        capsule_digest: None,
    };
    let mut selection =
        FileLaunchSelectionPort::initialize(&selector_root, original.clone()).unwrap();
    let first_handoff = HandoffId::parse("handoff-first").unwrap();
    let first = transfer_current(
        &mut source,
        &mut selection,
        &original,
        &capsule_one,
        &candidate_root,
        &first_handoff,
        "test-issuer".into(),
    )
    .unwrap();
    assert_eq!(first.capsule.artifacts.len(), 1);
    assert_eq!(first.selected.store_id, first.adoption.destination_store_id);
    assert_eq!(first.source_fence.source_store_id, original.store_id);
    assert_eq!(
        source
            .read_committed_artifact(&old.digest_hex)
            .unwrap()
            .unwrap(),
        b"old committed bytes"
    );
    drop(source);

    let mut source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let same = resume_transfer(
        &mut source,
        &mut selection,
        &original,
        &capsule_one,
        &candidate_root,
        &first_handoff,
        "test-issuer".into(),
    )
    .unwrap();
    assert_eq!(same, first);
    drop(source);

    let mut candidate = SqliteLifecycleStore::open(&candidate_root, "test-issuer".into()).unwrap();
    assert_eq!(
        candidate
            .read_committed_artifact(&old.digest_hex)
            .unwrap()
            .unwrap(),
        b"old committed bytes"
    );
    let new = candidate
        .stage_artifact(subject.clone(), b"new candidate bytes")
        .unwrap();
    let new = candidate.publish_artifact(new, clock(200)).unwrap();
    let uncertain_subject = register_fixture_subject(&mut candidate, "uncertain", 201);
    enqueue(
        &mut candidate,
        "uncertain",
        "entered candidate delivery",
        202,
    );
    let uncertain_plan = candidate.prepare_next_input(&uncertain_subject).unwrap();
    let uncertain_entry = candidate
        .claim_prepared_entry(
            &uncertain_plan.effect,
            RuntimeIncarnation::parse("incarnation-uncertain").unwrap(),
            clock(203),
        )
        .unwrap();
    let pending_subject = register_fixture_subject(&mut candidate, "pending", 204);
    enqueue(&mut candidate, "pending", "pending candidate delivery", 205);
    let _retired_subject = register_fixture_subject(&mut candidate, "retired", 206);
    enqueue(&mut candidate, "retired", "retired without retry", 207);
    let retired_effect =
        retire_prepared(&mut candidate, "retired", Some(original.store_id.clone()));
    let yielded_subject = register_fixture_subject(&mut candidate, "yielded", 208);
    enqueue(&mut candidate, "yielded", "yielded candidate delivery", 209);
    let yielded_plan = candidate.prepare_next_input(&yielded_subject).unwrap();
    let yielded_entry = candidate
        .claim_prepared_entry(
            &yielded_plan.effect,
            RuntimeIncarnation::parse("incarnation-yielded").unwrap(),
            clock(210),
        )
        .unwrap();
    candidate
        .apply_bound_observation(
            EffectObservation {
                source: EvidenceId::parse("evidence-yielded").unwrap(),
                effect: yielded_plan.effect.clone(),
                attempt: yielded_entry.attempt().clone(),
                incarnation: yielded_entry.incarnation().clone(),
                provider_thread: ProviderThreadId::parse("thread-yielded").unwrap(),
                provider_turn: ProviderTurnId::parse("turn-yielded").unwrap(),
                outcome: ExecutionOutcome::Yielded,
                settlement: EffectSettlement::Established(
                    EvidenceId::parse("settlement-yielded").unwrap(),
                ),
            },
            clock(211),
        )
        .unwrap();
    let candidate_counts = candidate.counts().unwrap();
    let candidate_entries = candidate.load_recovery_entries().unwrap();
    assert_eq!(candidate_counts, (5, 5, 3));
    let second_handoff = HandoffId::parse("handoff-rollback").unwrap();
    let rollback = transfer_current(
        &mut candidate,
        &mut selection,
        &first.selected,
        &capsule_two,
        &rollback_root,
        &second_handoff,
        "test-issuer".into(),
    )
    .unwrap();
    assert_ne!(
        rollback.capsule.capsule_sha256,
        first.capsule.capsule_sha256
    );
    assert_eq!(rollback.capsule.artifacts.len(), 2);
    drop(candidate);

    let mut rollback_store =
        SqliteLifecycleStore::open(&rollback_root, "test-issuer".into()).unwrap();
    assert_eq!(rollback_store.counts().unwrap(), candidate_counts);
    assert_eq!(
        rollback_store.load_recovery_entries().unwrap(),
        candidate_entries
    );
    assert_eq!(
        rollback_store
            .effect_projection(&completed_plan.effect)
            .unwrap()
            .outcome,
        ExecutionOutcome::Completed
    );
    let uncertain = rollback_store
        .effect_projection(&uncertain_plan.effect)
        .unwrap();
    assert_eq!(uncertain.attempt, *uncertain_entry.attempt());
    assert_ne!(uncertain.outcome, ExecutionOutcome::Completed);
    assert_eq!(
        rollback_store
            .effect_projection(&yielded_plan.effect)
            .unwrap()
            .outcome,
        ExecutionOutcome::Yielded
    );
    let retired_projection = rollback_store
        .subject_projection(&SubjectId::parse("subject-retired").unwrap(), None, None)
        .unwrap();
    let retired_selector = ExactAttemptSelector {
        store_id: rollback.selected.store_id.clone(),
        source_lineage: Some(first.selected.store_id.clone()),
        subject_id: "subject-retired".into(),
        input_id: Some("input-retired".into()),
        delivery_id: None,
        effect_id: Some(retired_effect),
        attempt_id: None,
        native_session_id: None,
        invocation_id: None,
        incarnation_id: None,
        operation_contract_id: None,
        operation_implementation_id: None,
        operation_executor_id: None,
        profile_digest: None,
        claim_revision: retired_projection.revision,
        observation_revision: 0,
    };
    assert_eq!(
        rollback_store
            .read_exact_attempt(&retired_selector)
            .unwrap()
            .disposition
            .as_deref(),
        Some("retired_no_retry")
    );
    assert_eq!(
        rollback_store
            .read_committed_artifact(&old.digest_hex)
            .unwrap()
            .unwrap(),
        b"old committed bytes"
    );
    assert_eq!(
        rollback_store
            .read_committed_artifact(&new.digest_hex)
            .unwrap()
            .unwrap(),
        b"new candidate bytes"
    );
    assert_eq!(
        rollback_store.admin_store_identity().unwrap(),
        rollback.selected.store_id
    );
    assert_ne!(rollback.selected.store_id, first.selected.store_id);
    let pending = rollback_store.prepare_next_input(&pending_subject).unwrap();
    let EffectInput::ControlledText(pending) = pending.input;
    assert_eq!(pending.input_id().as_str(), "input-pending");
}
