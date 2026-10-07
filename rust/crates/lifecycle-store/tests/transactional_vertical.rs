use lifecycle_core::{
    AuthorityExpiresAt, BuildId, ClockSample, Command, CommandAdmission, CommandId, CommandOutcome,
    CommandRequest, ContextGeneration, ControlledTextInput, EffectInput, EffectObservation,
    EffectSettlement, EvidenceId, ExecutionOutcome, GrantId, GrantScope, InputId, ProofRunId,
    ProviderThreadId, ProviderTurnId, RejectionCode, Revision, RuntimeIncarnation, SubjectId,
    TransitionFact, TransitionStage, TrustedGrant, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::{ObservationApply, SqliteLifecycleStore, StoreError};
use serde_json::json;
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

fn admission() -> CommandAdmission {
    admission_for("command-1", "input-1", "hello from owned custody\n", 0)
}

fn admission_for(
    command_id: &str,
    input_name: &str,
    text: &str,
    revision: u64,
) -> CommandAdmission {
    let input_id = id(input_name, InputId::parse);
    let text_digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap();
    let controlled =
        ControlledTextInput::new(input_id.clone(), text.into(), text_digest.clone()).unwrap();
    let basis = json!({
        "protocol_version": 1,
        "principal_ref": "operator:controlled",
        "command_id": command_id,
        "subject_id": "subject-1",
        "context_generation": "context-1",
        "build_id": "build-1",
        "proof_run_id": "proof-1",
        "grant_ref": "grant-1",
        "expected_revision": revision.to_string(),
        "kind": "enqueue_input",
        "payload": {
            "input_id": input_name,
            "producer_ref": "fixture:vertical",
            "text": text,
            "text_digest": text_digest.hex(),
        },
    });
    let digest = CodecContract::LifecycleCommandV1
        .digest_json(&basis)
        .unwrap();
    let request = CommandRequest::new(
        id(command_id, CommandId::parse),
        id("subject-1", SubjectId::parse),
        id("context-1", ContextGeneration::parse),
        id("build-1", BuildId::parse),
        id("proof-1", ProofRunId::parse),
        id("grant-1", GrantId::parse),
        Revision::new(revision),
        digest,
        Command::EnqueueInput { input: input_id },
    )
    .unwrap();
    CommandAdmission::new(
        request,
        "operator:controlled".into(),
        basis,
        Some(controlled),
    )
    .unwrap()
}

fn bootstrap(store: &mut SqliteLifecycleStore) -> SubjectId {
    let subject = id("subject-1", SubjectId::parse);
    store
        .register_subject(
            subject.clone(),
            id("context-1", ContextGeneration::parse),
            id("build-1", BuildId::parse),
            id("proof-1", ProofRunId::parse),
            clock(100),
        )
        .unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: id("grant-1", GrantId::parse),
            issuer: "test-issuer".into(),
            principal: "operator:controlled".into(),
            subject: subject.clone(),
            context: id("context-1", ContextGeneration::parse),
            build: id("build-1", BuildId::parse),
            proof_run: id("proof-1", ProofRunId::parse),
            scope: GrantScope::EnqueueInput,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(1_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    subject
}

fn replacement_admission(revision: u64) -> CommandAdmission {
    let basis = json!({
        "protocol_version":1,"principal_ref":"operator:controlled","command_id":"replace-1",
        "subject_id":"subject-1","context_generation":"context-1","build_id":"build-1",
        "proof_run_id":"proof-1","grant_ref":"grant-replace","expected_revision":revision.to_string(),
        "kind":"request_replacement","payload":{"reason":"controlled transition"}
    });
    let request = CommandRequest::new(
        id("replace-1", CommandId::parse),
        id("subject-1", SubjectId::parse),
        id("context-1", ContextGeneration::parse),
        id("build-1", BuildId::parse),
        id("proof-1", ProofRunId::parse),
        id("grant-replace", GrantId::parse),
        Revision::new(revision),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        Command::RequestReplacement,
    )
    .unwrap();
    CommandAdmission::new(request, "operator:controlled".into(), basis, None).unwrap()
}

fn interruption_admission(command_id: &str, attempt: &str, revision: u64) -> CommandAdmission {
    interruption_admission_for(
        "operator:controlled",
        "grant-interrupt",
        command_id,
        attempt,
        revision,
    )
}

fn interruption_admission_for(
    principal: &str,
    grant: &str,
    command_id: &str,
    attempt: &str,
    revision: u64,
) -> CommandAdmission {
    let basis = json!({
        "protocol_version":1,"principal_ref":principal,"command_id":command_id,
        "subject_id":"subject-1","context_generation":"context-1","build_id":"build-1",
        "proof_run_id":"proof-1","grant_ref":grant,"expected_revision":revision.to_string(),
        "kind":"request_interruption","payload":{"attempt_id":attempt}
    });
    let request = CommandRequest::new(
        id(command_id, CommandId::parse),
        id("subject-1", SubjectId::parse),
        id("context-1", ContextGeneration::parse),
        id("build-1", BuildId::parse),
        id("proof-1", ProofRunId::parse),
        id(grant, GrantId::parse),
        Revision::new(revision),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        Command::RequestInterruption {
            attempt: id(attempt, lifecycle_core::AttemptId::parse),
        },
    )
    .unwrap();
    CommandAdmission::new(request, principal.into(), basis, None).unwrap()
}

fn unknown_subject_admission() -> CommandAdmission {
    let input_id = id("input-unknown", InputId::parse);
    let text = "unknown custody";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap();
    let basis = json!({
        "protocol_version":1,"principal_ref":"operator:controlled","command_id":"command-unknown",
        "subject_id":"subject-unknown","context_generation":"context-1","build_id":"build-1",
        "proof_run_id":"proof-1","grant_ref":"grant-unknown","expected_revision":"0",
        "kind":"enqueue_input","payload":{"input_id":"input-unknown","producer_ref":"fixture:vertical","text":text,"text_digest":digest.hex()}
    });
    let request = CommandRequest::new(
        id("command-unknown", CommandId::parse),
        id("subject-unknown", SubjectId::parse),
        id("context-1", ContextGeneration::parse),
        id("build-1", BuildId::parse),
        id("proof-1", ProofRunId::parse),
        id("grant-unknown", GrantId::parse),
        Revision::new(0),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        Command::EnqueueInput {
            input: input_id.clone(),
        },
    )
    .unwrap();
    CommandAdmission::new(
        request,
        "operator:controlled".into(),
        basis,
        Some(ControlledTextInput::new(input_id, text.into(), digest).unwrap()),
    )
    .unwrap()
}

fn install_grant(store: &mut SqliteLifecycleStore, name: &str, scope: GrantScope) {
    store
        .install_trusted_grant(TrustedGrant {
            id: id(name, GrantId::parse),
            issuer: "test-issuer".into(),
            principal: "operator:controlled".into(),
            subject: id("subject-1", SubjectId::parse),
            context: id("context-1", ContextGeneration::parse),
            build: id("build-1", BuildId::parse),
            proof_run: id("proof-1", ProofRunId::parse),
            scope,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
}

#[test]
fn committed_command_and_entry_survive_lost_response_without_resend() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    let first = store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    assert!(matches!(first.outcome, CommandOutcome::Enqueued { .. }));
    assert_eq!(store.counts().unwrap(), (1, 1, 0));
    drop(store); // caller loses the response while committed bytes remain

    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert_eq!(
        store
            .apply_checked_command(admission(), clock(102))
            .unwrap(),
        first
    );
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-1", RuntimeIncarnation::parse),
            clock(103),
        )
        .unwrap();
    assert_eq!(entry.effect(), &plan.effect);
    assert_eq!(entry.subject(), &subject);
    assert_eq!(entry.input(), &plan.input);
    match entry.input() {
        EffectInput::ControlledText(input) => {
            assert_eq!(input.text(), "hello from owned custody\n");
            assert_eq!(
                input.digest().hex(),
                match &plan.input {
                    EffectInput::ControlledText(planned) => planned.digest().hex(),
                }
            );
        }
    }
    assert_eq!(store.counts().unwrap(), (1, 1, 1));
    drop(store); // entered but no provider observation

    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert_eq!(store.counts().unwrap(), (1, 1, 1));
    assert!(store.prepare_next_input(&subject).is_err());
    assert!(
        store
            .claim_prepared_entry(
                &plan.effect,
                id("incarnation-2", RuntimeIncarnation::parse),
                clock(104),
            )
            .is_err()
    );
}

#[test]
fn failed_outcome_remains_visible_while_settlement_is_unresolved_and_conflict_fences() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-1", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    let failed = EffectObservation {
        source: id("event-1", EvidenceId::parse),
        effect: plan.effect.clone(),
        attempt: entry.attempt().clone(),
        incarnation: entry.incarnation().clone(),
        provider_thread: id("thread-1", ProviderThreadId::parse),
        provider_turn: id("turn-1", ProviderTurnId::parse),
        outcome: ExecutionOutcome::Failed,
        settlement: EffectSettlement::Unresolved,
    };
    assert_eq!(
        store
            .apply_bound_observation(failed.clone(), clock(103))
            .unwrap(),
        ObservationApply::Applied
    );
    assert_eq!(
        store
            .apply_bound_observation(failed.clone(), clock(104))
            .unwrap(),
        ObservationApply::Duplicate
    );
    let projection = store.effect_projection(&plan.effect).unwrap();
    assert_eq!(projection.outcome, ExecutionOutcome::Failed);
    assert_eq!(projection.settlement, EffectSettlement::Unresolved);
    assert!(!projection.admission_fenced);
    assert!(store.prepare_next_input(&subject).is_err());

    let mut contradiction = failed;
    contradiction.source = id("event-2", EvidenceId::parse);
    contradiction.outcome = ExecutionOutcome::Completed;
    contradiction.settlement = EffectSettlement::Established(id("settlement-2", EvidenceId::parse));
    assert_eq!(
        store
            .apply_bound_observation(contradiction, clock(105))
            .unwrap(),
        ObservationApply::ConflictFenced
    );
    let projection = store.effect_projection(&plan.effect).unwrap();
    assert_eq!(projection.outcome, ExecutionOutcome::Failed);
    assert!(matches!(
        projection.settlement,
        EffectSettlement::Conflict(_)
    ));
    assert!(projection.admission_fenced);
    drop(store);
    let store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert!(
        store
            .effect_projection(&plan.effect)
            .unwrap()
            .admission_fenced
    );
}

#[test]
fn rejected_command_replays_stably_and_changed_bytes_conflict() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    bootstrap(&mut store);
    let rejected = store
        .apply_checked_command(
            admission_for("command-rejected", "input-x", "x", 99),
            clock(101),
        )
        .unwrap();
    assert_eq!(
        rejected.outcome,
        CommandOutcome::Rejected(RejectionCode::StaleRevision)
    );
    assert_eq!(store.counts().unwrap(), (1, 0, 0));
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert_eq!(
        store
            .apply_checked_command(
                admission_for("command-rejected", "input-x", "x", 99),
                clock(999),
            )
            .unwrap(),
        rejected
    );
    assert!(
        store
            .apply_checked_command(
                admission_for("command-rejected", "input-x", "changed", 99),
                clock(102),
            )
            .is_err()
    );
    assert_eq!(store.counts().unwrap(), (1, 0, 0));
}

#[test]
fn second_input_cannot_overtake_entered_first_but_can_prepare_after_settlement() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    store
        .apply_checked_command(
            admission_for("command-2", "input-2", "second", 1),
            clock(102),
        )
        .unwrap();
    let first = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &first.effect,
            id("incarnation-1", RuntimeIncarnation::parse),
            clock(103),
        )
        .unwrap();
    assert!(store.prepare_next_input(&subject).is_err());
    store
        .apply_bound_observation(
            EffectObservation {
                source: id("event-success", EvidenceId::parse),
                effect: first.effect.clone(),
                attempt: entry.attempt().clone(),
                incarnation: entry.incarnation().clone(),
                provider_thread: id("thread-1", ProviderThreadId::parse),
                provider_turn: id("turn-1", ProviderTurnId::parse),
                outcome: ExecutionOutcome::Completed,
                settlement: EffectSettlement::Established(id("settlement-1", EvidenceId::parse)),
            },
            clock(104),
        )
        .unwrap();
    let second = store.prepare_next_input(&subject).unwrap();
    assert_ne!(second.effect, first.effect);
    assert_eq!(store.counts().unwrap(), (2, 2, 1));
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let entry = store
        .claim_prepared_entry(
            &second.effect,
            id("incarnation-2", RuntimeIncarnation::parse),
            clock(105),
        )
        .unwrap();
    match entry.input() {
        EffectInput::ControlledText(input) => assert_eq!(input.text(), "second"),
    }
}

#[test]
fn expiry_and_revocation_block_new_entry_but_not_late_accounting() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    assert!(
        store
            .install_trusted_grant(TrustedGrant {
                id: id("grant-untrusted", GrantId::parse),
                issuer: "caller-supplied".into(),
                principal: "operator:controlled".into(),
                subject: subject.clone(),
                context: id("context-1", ContextGeneration::parse),
                build: id("build-1", BuildId::parse),
                proof_run: id("proof-1", ProofRunId::parse),
                scope: GrantScope::EnqueueInput,
                expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
                revision: Revision::new(1),
                revoked: false,
            })
            .is_err()
    );
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-1", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    store
        .revoke_trusted_grant(&id("grant-1", GrantId::parse), Revision::new(1), clock(103))
        .unwrap();
    assert!(
        store
            .apply_checked_command(
                admission_for("command-2", "input-2", "second", 1),
                clock(104)
            )
            .unwrap()
            .outcome
            == CommandOutcome::Rejected(RejectionCode::InvalidGrant)
    );
    assert!(
        store
            .claim_prepared_entry(
                &plan.effect,
                id("incarnation-2", RuntimeIncarnation::parse),
                clock(104)
            )
            .is_err()
    );
    assert_eq!(
        store
            .apply_bound_observation(
                EffectObservation {
                    source: id("late-result", EvidenceId::parse),
                    effect: plan.effect.clone(),
                    attempt: entry.attempt().clone(),
                    incarnation: entry.incarnation().clone(),
                    provider_thread: id("thread-1", ProviderThreadId::parse),
                    provider_turn: id("turn-1", ProviderTurnId::parse),
                    outcome: ExecutionOutcome::Completed,
                    settlement: EffectSettlement::Established(id(
                        "late-settlement",
                        EvidenceId::parse
                    )),
                },
                clock(1001)
            )
            .unwrap(),
        ObservationApply::Applied
    );
    assert_eq!(
        store.effect_projection(&plan.effect).unwrap().outcome,
        ExecutionOutcome::Completed
    );
}

#[test]
fn selected_successor_accepts_ordered_custody_without_automatic_delivery() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    let first_result = store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let original_delivery = match first_result.outcome {
        CommandOutcome::Enqueued { delivery, .. } => delivery,
        other => panic!("unexpected result {other:?}"),
    };
    store
        .apply_checked_command(
            admission_for("command-2", "input-2", "second", 1),
            clock(102),
        )
        .unwrap();
    for (grant_id, context, scope) in [
        ("grant-replace", "context-1", GrantScope::RequestReplacement),
        ("grant-successor", "context-2", GrantScope::EnqueueInput),
    ] {
        store
            .install_trusted_grant(TrustedGrant {
                id: id(grant_id, GrantId::parse),
                issuer: "test-issuer".into(),
                principal: "operator:controlled".into(),
                subject: subject.clone(),
                context: id(context, ContextGeneration::parse),
                build: id("build-1", BuildId::parse),
                proof_run: id("proof-1", ProofRunId::parse),
                scope,
                expires: AuthorityExpiresAt::new(WallTimeMs::new(1_000)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
    }
    let result = store
        .apply_checked_command(replacement_admission(2), clock(103))
        .unwrap();
    let transition = match result.outcome {
        CommandOutcome::ReplacementRequested { transition } => transition,
        other => panic!("unexpected result {other:?}"),
    };
    assert!(store.prepare_next_input(&subject).is_err());
    let facts = [
        TransitionFact::PredecessorSafe {
            evidence: id("safe", EvidenceId::parse),
        },
        TransitionFact::SourceFrozen {
            evidence: id("source", EvidenceId::parse),
            semantic_revision: Revision::new(1),
        },
        TransitionFact::VerificationEstablished {
            evidence: id("verified", EvidenceId::parse),
            checkpoint: id("checkpoint-1", lifecycle_core::CheckpointId::parse),
        },
        TransitionFact::SwitchAuthorized {
            evidence: id("switch", EvidenceId::parse),
        },
        TransitionFact::SuccessorObserved {
            evidence: id("successor", EvidenceId::parse),
            context: id("context-2", ContextGeneration::parse),
            thread: id("thread-2", ProviderThreadId::parse),
        },
        TransitionFact::RehydrationVerified {
            evidence: id("rehydrated", EvidenceId::parse),
        },
        TransitionFact::CommitSuccessor {
            evidence: id("commit", EvidenceId::parse),
            semantic_revision: Revision::new(1),
        },
    ];
    let mut revision = result.revision.get();
    for (index, fact) in facts.into_iter().enumerate() {
        let successor_grant = (index == 6).then(|| id("grant-successor", GrantId::parse));
        let state = store
            .apply_transition_fact(
                &transition,
                Revision::new(revision),
                fact,
                successor_grant.as_ref(),
                clock(104 + index as i64),
            )
            .unwrap();
        revision += 1;
        if index == 6 {
            assert_eq!(state.stage, TransitionStage::Reconciled);
        }
    }
    assert_eq!(store.counts().unwrap(), (3, 2, 0));
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let inspection = rusqlite::Connection::open_with_flags(
        root.path().join("lifecycle.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (delivery, producer): (String, String) = inspection
        .query_row(
            "SELECT delivery_id,producer_ref FROM inputs WHERE input_id='input-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(delivery, original_delivery.as_str());
    assert_eq!(producer, "fixture:vertical");
    let first = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &first.effect,
            id("incarnation-2", RuntimeIncarnation::parse),
            clock(112),
        )
        .unwrap();
    match entry.input() {
        EffectInput::ControlledText(input) => {
            assert_eq!(input.text(), "hello from owned custody\n")
        }
    }
    assert!(store.prepare_next_input(&subject).is_err());
}

#[test]
fn expired_successor_grant_denial_survives_restart_and_clock_rollback() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    install_grant(&mut store, "grant-replace", GrantScope::RequestReplacement);
    store
        .install_trusted_grant(TrustedGrant {
            id: id("grant-successor-expiring", GrantId::parse),
            issuer: "test-issuer".into(),
            principal: "operator:controlled".into(),
            subject: subject.clone(),
            context: id("context-2", ContextGeneration::parse),
            build: id("build-1", BuildId::parse),
            proof_run: id("proof-1", ProofRunId::parse),
            scope: GrantScope::EnqueueInput,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(1_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    let replacement = store
        .apply_checked_command(replacement_admission(1), clock(102))
        .unwrap();
    let CommandOutcome::ReplacementRequested { transition } = replacement.outcome else {
        panic!("replacement rejected")
    };
    let facts = [
        TransitionFact::PredecessorSafe {
            evidence: id("safe-exp", EvidenceId::parse),
        },
        TransitionFact::SourceFrozen {
            evidence: id("source-exp", EvidenceId::parse),
            semantic_revision: Revision::new(1),
        },
        TransitionFact::VerificationEstablished {
            evidence: id("verified-exp", EvidenceId::parse),
            checkpoint: id("checkpoint-exp", lifecycle_core::CheckpointId::parse),
        },
        TransitionFact::SwitchAuthorized {
            evidence: id("switch-exp", EvidenceId::parse),
        },
        TransitionFact::SuccessorObserved {
            evidence: id("successor-exp", EvidenceId::parse),
            context: id("context-2", ContextGeneration::parse),
            thread: id("thread-exp", ProviderThreadId::parse),
        },
        TransitionFact::RehydrationVerified {
            evidence: id("rehydrated-exp", EvidenceId::parse),
        },
    ];
    let mut revision = replacement.revision.get();
    for (index, fact) in facts.into_iter().enumerate() {
        store
            .apply_transition_fact(
                &transition,
                Revision::new(revision),
                fact,
                None,
                clock(103 + index as i64),
            )
            .unwrap();
        revision += 1;
    }
    let commit = TransitionFact::CommitSuccessor {
        evidence: id("commit-exp", EvidenceId::parse),
        semantic_revision: Revision::new(1),
    };
    let grant = id("grant-successor-expiring", GrantId::parse);
    assert!(matches!(
        store.apply_transition_fact(
            &transition,
            Revision::new(revision),
            commit.clone(),
            Some(&grant),
            clock(1_000)
        ),
        Err(StoreError::Rejected)
    ));
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert!(matches!(
        store.apply_transition_fact(
            &transition,
            Revision::new(revision),
            commit,
            Some(&grant),
            clock(999)
        ),
        Err(StoreError::Rejected)
    ));
    let db = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    let (wall, owner, context, stage, custody): (i64,String,String,String,String) = db.query_row(
        "SELECT s.last_wall_ms,s.owner_kind,s.context_id,t.stage,i.state FROM subjects s JOIN transitions t ON t.subject_id=s.subject_id JOIN inputs i ON i.subject_id=s.subject_id WHERE s.subject_id='subject-1'",
        [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
    ).unwrap();
    assert_eq!(
        (
            wall,
            owner.as_str(),
            context.as_str(),
            stage.as_str(),
            custody.as_str()
        ),
        (
            1_000,
            "transition",
            "context-1",
            "ready_to_commit",
            "queued"
        )
    );
}

#[test]
fn mismatched_attempt_or_incarnation_cannot_fill_an_observation_gap() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-1", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    let observation = EffectObservation {
        source: id("gap-source", EvidenceId::parse),
        effect: plan.effect.clone(),
        attempt: entry.attempt().clone(),
        incarnation: entry.incarnation().clone(),
        provider_thread: id("thread-1", ProviderThreadId::parse),
        provider_turn: id("turn-1", ProviderTurnId::parse),
        outcome: ExecutionOutcome::Unknown,
        settlement: EffectSettlement::Unresolved,
    };
    let mut swapped = observation.clone();
    swapped.attempt = id("attempt-other", lifecycle_core::AttemptId::parse);
    assert!(store.apply_bound_observation(swapped, clock(103)).is_err());
    swapped = observation.clone();
    swapped.incarnation = id("incarnation-other", RuntimeIncarnation::parse);
    assert!(store.apply_bound_observation(swapped, clock(103)).is_err());
    assert_eq!(store.load_recovery().unwrap().unresolved_attempts, 1);
    assert_eq!(
        store
            .apply_bound_observation(observation, clock(104))
            .unwrap(),
        ObservationApply::Applied
    );
    assert_eq!(
        store.effect_projection(&plan.effect).unwrap().outcome,
        ExecutionOutcome::Unknown
    );
    assert!(store.prepare_next_input(&subject).is_err());
}

#[test]
fn sqlite_busy_at_command_commit_leaves_no_partial_result_or_input() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    bootstrap(&mut store);
    let blocker = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(
        store
            .apply_checked_command(admission(), clock(101))
            .is_err()
    );
    blocker.execute_batch("ROLLBACK").unwrap();
    assert_eq!(store.counts().unwrap(), (0, 0, 0));
    assert!(matches!(
        store
            .apply_checked_command(admission(), clock(101))
            .unwrap()
            .outcome,
        CommandOutcome::Enqueued { .. }
    ));
    assert_eq!(store.counts().unwrap(), (1, 1, 0));
}

#[test]
fn durable_identity_swaps_are_stable_rejections_without_custody() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    bootstrap(&mut store);
    for (index, field, replacement) in [
        (1, "context_generation", "context-other"),
        (2, "build_id", "build-other"),
        (3, "proof_run_id", "proof-other"),
    ] {
        let original = admission_for(
            &format!("swapped-command-{index}"),
            &format!("swapped-input-{index}"),
            "bytes",
            0,
        );
        let mut request = original.request().clone();
        let mut basis = original.digest_basis().clone();
        basis[field] = json!(replacement);
        match field {
            "context_generation" => request.context = id(replacement, ContextGeneration::parse),
            "build_id" => request.build = id(replacement, BuildId::parse),
            "proof_run_id" => request.proof_run = id(replacement, ProofRunId::parse),
            _ => unreachable!(),
        }
        request.request_digest = CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap();
        let admission = CommandAdmission::new(
            request,
            "operator:controlled".into(),
            basis,
            original.input().cloned(),
        )
        .unwrap();
        let result = store
            .apply_checked_command(admission.clone(), clock(101))
            .unwrap();
        assert_eq!(
            result.outcome,
            CommandOutcome::Rejected(RejectionCode::WrongIdentity)
        );
        assert_eq!(
            store.apply_checked_command(admission, clock(102)).unwrap(),
            result
        );
    }
    assert_eq!(store.counts().unwrap(), (3, 0, 0));
}

#[test]
fn exact_grant_expiry_and_clock_rollback_prevent_new_entry() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    assert!(
        store
            .claim_prepared_entry(
                &plan.effect,
                id("incarnation-expired", RuntimeIncarnation::parse),
                clock(1000)
            )
            .is_err()
    );
    assert_eq!(store.counts().unwrap(), (1, 1, 0));
    assert!(
        store
            .claim_prepared_entry(
                &plan.effect,
                id("incarnation-rollback", RuntimeIncarnation::parse),
                clock(100)
            )
            .is_err()
    );
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert!(store.prepare_next_input(&subject).is_err());
    assert_eq!(store.counts().unwrap(), (1, 1, 0));
}

#[test]
fn interrupted_settled_result_remains_distinct_from_delivery_retirement() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-interrupted", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    store
        .apply_bound_observation(
            EffectObservation {
                source: id("interrupt-event", EvidenceId::parse),
                effect: plan.effect.clone(),
                attempt: entry.attempt().clone(),
                incarnation: entry.incarnation().clone(),
                provider_thread: id("thread-1", ProviderThreadId::parse),
                provider_turn: id("turn-1", ProviderTurnId::parse),
                outcome: ExecutionOutcome::Interrupted,
                settlement: EffectSettlement::Established(id(
                    "interrupt-settlement",
                    EvidenceId::parse,
                )),
            },
            clock(103),
        )
        .unwrap();
    let projection = store.effect_projection(&plan.effect).unwrap();
    assert_eq!(projection.outcome, ExecutionOutcome::Interrupted);
    assert!(matches!(
        projection.settlement,
        EffectSettlement::Established(_)
    ));
    assert!(store.prepare_next_input(&subject).is_err());
}

#[test]
fn expiry_denial_and_late_observation_preserve_wall_high_water_across_restart() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    assert!(
        store
            .claim_prepared_entry(
                &plan.effect,
                id("incarnation-expired", RuntimeIncarnation::parse),
                clock(1000)
            )
            .is_err()
    );
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert!(
        store
            .claim_prepared_entry(
                &plan.effect,
                id("incarnation-rollback", RuntimeIncarnation::parse),
                clock(999)
            )
            .is_err()
    );
    assert_eq!(store.counts().unwrap().2, 0);

    let separate = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(separate.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-late", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    let observation = |source: &str| EffectObservation {
        source: id(source, EvidenceId::parse),
        effect: plan.effect.clone(),
        attempt: entry.attempt().clone(),
        incarnation: entry.incarnation().clone(),
        provider_thread: id("thread-late", ProviderThreadId::parse),
        provider_turn: id("turn-late", ProviderTurnId::parse),
        outcome: ExecutionOutcome::Pending,
        settlement: EffectSettlement::Unresolved,
    };
    store
        .apply_bound_observation(observation("late-1"), clock(500))
        .unwrap();
    store
        .apply_bound_observation(observation("late-2"), clock(400))
        .unwrap();
    drop(store);
    let mut store = SqliteLifecycleStore::open(separate.path(), "test-issuer".into()).unwrap();
    let result = store
        .apply_checked_command(
            admission_for("command-late", "input-2", "later", 3),
            clock(450),
        )
        .unwrap();
    assert_eq!(
        result.outcome,
        CommandOutcome::Rejected(RejectionCode::ClockRollback)
    );
}

#[test]
fn abort_requeues_prepared_custody_without_resending_an_attempt() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    install_grant(&mut store, "grant-replace", GrantScope::RequestReplacement);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let old = store.prepare_next_input(&subject).unwrap();
    let replacement = store
        .apply_checked_command(replacement_admission(1), clock(102))
        .unwrap();
    let CommandOutcome::ReplacementRequested { transition } = replacement.outcome else {
        panic!("replacement not accepted")
    };
    store
        .apply_transition_fact(
            &transition,
            Revision::new(2),
            TransitionFact::AbortNoEntry {
                evidence: id("abort-evidence", EvidenceId::parse),
            },
            None,
            clock(103),
        )
        .unwrap();
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let next = store.prepare_next_input(&subject).unwrap();
    assert_ne!(old.effect, next.effect);
    assert_eq!(old.input, next.input);
    assert!(
        store
            .claim_prepared_entry(
                &old.effect,
                id("incarnation-old", RuntimeIncarnation::parse),
                clock(104)
            )
            .is_err()
    );
    let entry = store
        .claim_prepared_entry(
            &next.effect,
            id("incarnation-new", RuntimeIncarnation::parse),
            clock(104),
        )
        .unwrap();
    assert_eq!(entry.effect(), &next.effect);
    assert_eq!(store.counts().unwrap().2, 1);
}

#[test]
fn semantic_identity_conflicts_and_unknown_subject_have_stable_command_results() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let conflict = store
        .apply_checked_command(
            admission_for("command-conflict", "input-1", "different", 1),
            clock(102),
        )
        .unwrap();
    assert_eq!(
        conflict.outcome,
        CommandOutcome::Rejected(RejectionCode::Conflict)
    );
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert_eq!(
        store
            .apply_checked_command(
                admission_for("command-conflict", "input-1", "different", 1),
                clock(103)
            )
            .unwrap(),
        conflict
    );
    assert!(matches!(
        store.apply_checked_command(
            admission_for("command-conflict", "input-new", "changed", 1),
            clock(104)
        ),
        Err(StoreError::ClaimConflict)
    ));
    assert_eq!(store.counts().unwrap(), (2, 1, 0));
    let unknown = store
        .apply_checked_command(unknown_subject_admission(), clock(105))
        .unwrap();
    assert_eq!(
        unknown.outcome,
        CommandOutcome::Rejected(RejectionCode::UnknownSubject)
    );
    store
        .register_subject(
            id("subject-unknown", SubjectId::parse),
            id("context-1", ContextGeneration::parse),
            id("build-1", BuildId::parse),
            id("proof-1", ProofRunId::parse),
            clock(106),
        )
        .unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: id("grant-unknown", GrantId::parse),
            issuer: "test-issuer".into(),
            principal: "operator:controlled".into(),
            subject: id("subject-unknown", SubjectId::parse),
            context: id("context-1", ContextGeneration::parse),
            build: id("build-1", BuildId::parse),
            proof_run: id("proof-1", ProofRunId::parse),
            scope: GrantScope::EnqueueInput,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert_eq!(
        store
            .apply_checked_command(unknown_subject_admission(), clock(107))
            .unwrap(),
        unknown
    );
    assert_eq!(store.counts().unwrap(), (3, 1, 0));
}

#[test]
fn same_source_contradiction_is_retained_and_replay_does_not_refence() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-source", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    let original = EffectObservation {
        source: id("source-same", EvidenceId::parse),
        effect: plan.effect.clone(),
        attempt: entry.attempt().clone(),
        incarnation: entry.incarnation().clone(),
        provider_thread: id("thread-source", ProviderThreadId::parse),
        provider_turn: id("turn-source", ProviderTurnId::parse),
        outcome: ExecutionOutcome::Failed,
        settlement: EffectSettlement::Unresolved,
    };
    store
        .apply_bound_observation(original.clone(), clock(103))
        .unwrap();
    let mut contradictory = original.clone();
    contradictory.outcome = ExecutionOutcome::Completed;
    contradictory.settlement =
        EffectSettlement::Established(id("settlement-source", EvidenceId::parse));
    assert_eq!(
        store
            .apply_bound_observation(contradictory.clone(), clock(104))
            .unwrap(),
        ObservationApply::ConflictFenced
    );
    let inspection = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    let revision_after_conflict: i64 = inspection
        .query_row(
            "SELECT revision FROM subjects WHERE subject_id='subject-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(inspection);
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert_eq!(
        store
            .apply_bound_observation(contradictory.clone(), clock(105))
            .unwrap(),
        ObservationApply::ConflictFenced
    );
    let db = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    let revision: i64 = db
        .query_row(
            "SELECT revision FROM subjects WHERE subject_id='subject-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let variants: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM observation_conflicts WHERE source_id='source-same'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(revision, revision_after_conflict);
    assert_eq!(variants, 1);
    let mut wrong = contradictory;
    wrong.incarnation = id("wrong-incarnation", RuntimeIncarnation::parse);
    assert!(matches!(
        store.apply_bound_observation(wrong.clone(), clock(106)),
        Err(StoreError::ObservationMismatch)
    ));
    let revision_after_mismatch: i64 = db
        .query_row(
            "SELECT revision FROM subjects WHERE subject_id='subject-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(matches!(
        store.apply_bound_observation(wrong, clock(107)),
        Err(StoreError::ObservationMismatch)
    ));
    let revision_after_replay: i64 = db
        .query_row(
            "SELECT revision FROM subjects WHERE subject_id='subject-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(revision_after_replay, revision_after_mismatch);
    let variants: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM observation_conflicts WHERE source_id='source-same'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(variants, 2);
}

#[test]
fn mismatched_source_variant_never_changes_exact_attempt_settlement() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-exact", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    let original = EffectObservation {
        source: id("source-mismatch-first", EvidenceId::parse),
        effect: plan.effect.clone(),
        attempt: entry.attempt().clone(),
        incarnation: entry.incarnation().clone(),
        provider_thread: id("thread-exact", ProviderThreadId::parse),
        provider_turn: id("turn-exact", ProviderTurnId::parse),
        outcome: ExecutionOutcome::Pending,
        settlement: EffectSettlement::Unresolved,
    };
    assert_eq!(
        store
            .apply_bound_observation(original.clone(), clock(103))
            .unwrap(),
        ObservationApply::Applied
    );
    let before = store.effect_projection(&plan.effect).unwrap();
    let mut wrong = original;
    wrong.incarnation = id("incarnation-wrong", RuntimeIncarnation::parse);
    wrong.outcome = ExecutionOutcome::Completed;
    wrong.settlement = EffectSettlement::Established(id("settlement-wrong", EvidenceId::parse));
    assert!(matches!(
        store.apply_bound_observation(wrong.clone(), clock(104)),
        Err(StoreError::ObservationMismatch)
    ));
    let after = store.effect_projection(&plan.effect).unwrap();
    assert_eq!(after.outcome, before.outcome);
    assert_eq!(after.settlement, before.settlement);
    assert!(after.admission_fenced);
    let db = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    let revision: i64 = db
        .query_row(
            "SELECT revision FROM subjects WHERE subject_id='subject-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let variant: (i64, String) = db.query_row(
        "SELECT COUNT(*),MAX(attribution_kind) FROM observation_conflicts WHERE source_id='source-mismatch-first'",
        [], |r| Ok((r.get(0)?, r.get(1)?)),
    ).unwrap();
    assert_eq!(variant, (1, "mismatch".into()));
    drop(db);
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert!(matches!(
        store.apply_bound_observation(wrong, clock(105)),
        Err(StoreError::ObservationMismatch)
    ));
    let replay = store.effect_projection(&plan.effect).unwrap();
    assert_eq!(replay.outcome, before.outcome);
    assert_eq!(replay.settlement, before.settlement);
    let db = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    let replay_revision: i64 = db
        .query_row(
            "SELECT revision FROM subjects WHERE subject_id='subject-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(replay_revision, revision);
}

#[test]
fn contradictory_exact_observation_preserves_prior_outcome_and_custody() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-conflict", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    let failed = EffectObservation {
        source: id("source-failed", EvidenceId::parse),
        effect: plan.effect.clone(),
        attempt: entry.attempt().clone(),
        incarnation: entry.incarnation().clone(),
        provider_thread: id("thread-conflict", ProviderThreadId::parse),
        provider_turn: id("turn-conflict", ProviderTurnId::parse),
        outcome: ExecutionOutcome::Failed,
        settlement: EffectSettlement::Unresolved,
    };
    store
        .apply_bound_observation(failed.clone(), clock(103))
        .unwrap();
    let contradicted = EffectObservation {
        source: id("source-completed", EvidenceId::parse),
        outcome: ExecutionOutcome::Completed,
        settlement: EffectSettlement::Established(id("settlement-completed", EvidenceId::parse)),
        ..failed
    };
    assert_eq!(
        store
            .apply_bound_observation(contradicted.clone(), clock(104))
            .unwrap(),
        ObservationApply::ConflictFenced
    );
    let projection = store.effect_projection(&plan.effect).unwrap();
    assert_eq!(projection.outcome, ExecutionOutcome::Failed);
    assert_eq!(
        projection.settlement,
        EffectSettlement::Conflict(contradicted.source)
    );
    assert!(projection.admission_fenced);
    let db = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    let custody: String = db
        .query_row(
            "SELECT state FROM inputs WHERE input_id='input-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(custody, "failed_blocked");
}

#[test]
fn pending_observation_keeps_exact_entered_attempt_interruptible() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    install_grant(
        &mut store,
        "grant-interrupt",
        GrantScope::RequestInterruption,
    );
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-interrupt", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    let pending = EffectObservation {
        source: id("source-pending", EvidenceId::parse),
        effect: plan.effect.clone(),
        attempt: entry.attempt().clone(),
        incarnation: entry.incarnation().clone(),
        provider_thread: id("thread-interrupt", ProviderThreadId::parse),
        provider_turn: id("turn-interrupt", ProviderTurnId::parse),
        outcome: ExecutionOutcome::Pending,
        settlement: EffectSettlement::Unresolved,
    };
    store
        .apply_bound_observation(pending.clone(), clock(103))
        .unwrap();
    let result = store
        .apply_checked_command(
            interruption_admission("command-interrupt", entry.attempt().as_str(), 2),
            clock(104),
        )
        .unwrap();
    assert!(matches!(
        result.outcome,
        CommandOutcome::InterruptionRequested { .. }
    ));
    let terminal = EffectObservation {
        source: id("source-terminal", EvidenceId::parse),
        outcome: ExecutionOutcome::Completed,
        settlement: EffectSettlement::Established(id("settled-interrupt", EvidenceId::parse)),
        ..pending
    };
    store.apply_bound_observation(terminal, clock(105)).unwrap();
    let later = store
        .apply_checked_command(
            interruption_admission("command-too-late", entry.attempt().as_str(), 4),
            clock(106),
        )
        .unwrap();
    assert_eq!(
        later.outcome,
        CommandOutcome::Rejected(RejectionCode::Conflict)
    );
}

#[test]
fn interruption_command_identity_is_scoped_to_principal_across_restart() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    let subject = bootstrap(&mut store);
    install_grant(
        &mut store,
        "grant-interrupt",
        GrantScope::RequestInterruption,
    );
    store
        .install_trusted_grant(TrustedGrant {
            id: id("grant-interrupt-second", GrantId::parse),
            issuer: "test-issuer".into(),
            principal: "operator:second".into(),
            subject: subject.clone(),
            context: id("context-1", ContextGeneration::parse),
            build: id("build-1", BuildId::parse),
            proof_run: id("proof-1", ProofRunId::parse),
            scope: GrantScope::RequestInterruption,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(1_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    store
        .apply_checked_command(admission(), clock(101))
        .unwrap();
    let plan = store.prepare_next_input(&subject).unwrap();
    let entry = store
        .claim_prepared_entry(
            &plan.effect,
            id("incarnation-two-principals", RuntimeIncarnation::parse),
            clock(102),
        )
        .unwrap();
    let first = interruption_admission("same-command", entry.attempt().as_str(), 1);
    let first_result = store
        .apply_checked_command(first.clone(), clock(103))
        .unwrap();
    assert!(matches!(
        first_result.outcome,
        CommandOutcome::InterruptionRequested { .. }
    ));
    let second = interruption_admission_for(
        "operator:second",
        "grant-interrupt-second",
        "same-command",
        entry.attempt().as_str(),
        2,
    );
    let second_result = store
        .apply_checked_command(second.clone(), clock(104))
        .unwrap();
    assert!(matches!(
        second_result.outcome,
        CommandOutcome::InterruptionRequested { .. }
    ));
    drop(store);
    let mut store = SqliteLifecycleStore::open(root.path(), "test-issuer".into()).unwrap();
    assert_eq!(
        store.apply_checked_command(first, clock(105)).unwrap(),
        first_result
    );
    assert_eq!(
        store.apply_checked_command(second, clock(106)).unwrap(),
        second_result
    );
    let db = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    let persisted: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM interruption_requests WHERE command_id='same-command'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(persisted, 2);
    assert_eq!(store.counts().unwrap(), (3, 1, 1));
}
