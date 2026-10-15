use lifecycle_core::{SubjectId, TransitionId};
use lifecycle_semantic::{DependencyObservation, DependencyRevision, ObligationKind, SourceTrust};
use lifecycle_service::{
    CanonicalMaterialPort, MaterialPortError, QuestionOpenPayload as PortQuestion, ServiceError,
    TaskOperationError, TaskOperationPort, ValidatedTaskOperationEntry,
};
use retained_task_workflow::{
    AnswerQuestion, CompletedEffect, FactInput, FactKind, OperationEntry, QuestionOpenPayload,
    ResultPublishPayload, SubjectKind, SubmitTask, TaskError, TaskStore, json_digest, sha256,
};

fn task() -> SubmitTask {
    SubmitTask {
        command_id: "cmd-0".into(),
        task_id: "task-0".into(),
        subject_id: "subject-0".into(),
        kind: SubjectKind::RepositoryAnalysis,
        objective: "Analyze the approved source".into(),
        engineering_question: "What owns the request?".into(),
        snapshot_ref: "snapshot-0".into(),
        facts: Vec::new(),
    }
}

#[test]
fn owner_input_is_immutable_and_currentness_is_scoped() {
    let root = tempfile::tempdir().unwrap();
    let store =
        TaskStore::open(&root.path().join("tasks.sqlite"), "human-operator".into()).unwrap();
    let first = store.submit(&task()).unwrap();
    assert_eq!(store.submit(&task()).unwrap(), first);
    assert_eq!(
        store
            .lookup_command_receipt("submit", "task-0", "cmd-0", &json_digest(&task()).unwrap())
            .unwrap(),
        Some(first.clone())
    );
    assert!(matches!(
        store.lookup_command_receipt("submit", "task-0", "cmd-0", &sha256(b"changed")),
        Err(TaskError::Conflict)
    ));
    assert!(
        store
            .lookup_command_receipt("submit", "task-0", "absent", &json_digest(&task()).unwrap())
            .unwrap()
            .is_none()
    );
    let input = store.input_text("cmd-0").unwrap().unwrap();
    let binding = store.validate_input("input:cmd-0", &input).unwrap();
    assert_eq!(binding.owner_revision, first.owner_revision);
    assert!(store.validate_input("input:cmd-0", "forged").is_err());

    let material = store
        .project(
            &SubjectId::parse("subject-0").unwrap(),
            &TransitionId::parse("transition-0").unwrap(),
            "capture-0",
        )
        .unwrap();
    let objective = material
        .sources
        .iter()
        .find(|s| s.id == "objective")
        .unwrap();
    let expected = lifecycle_semantic::DependencyRevision {
        owner_namespace: objective.owner_namespace.clone(),
        dependency_key: objective.dependency_key.clone(),
        owner_ref: objective.owner_ref.clone(),
        revision: objective.revision,
        sha256: objective.sha256.clone(),
    };
    let followup = SubmitTask {
        command_id: "cmd-1".into(),
        engineering_question: "Which caller?".into(),
        ..task()
    };
    store.submit(&followup).unwrap();
    let readback = store
        .observe_dependencies(std::slice::from_ref(&expected))
        .unwrap();
    assert_eq!(
        readback,
        vec![lifecycle_semantic::DependencyObservation::Current(expected)]
    );
    let later = store.input_text("cmd-1").unwrap().unwrap();
    assert_eq!(
        store
            .validate_input("input:cmd-1", &later)
            .unwrap()
            .cursor_revision,
        1
    );
}

#[test]
fn pending_question_survives_and_result_commit_requires_answer_and_fence() {
    let root = tempfile::tempdir().unwrap();
    let store =
        TaskStore::open(&root.path().join("tasks.sqlite"), "human-operator".into()).unwrap();
    store.submit(&task()).unwrap();
    let effect = CompletedEffect {
        effect_id: "effect-0".into(),
        delivery_id: "delivery-0".into(),
        receipt_digest: sha256(b"settled"),
    };
    store
        .record_completed_effect("task-0", effect.clone(), 1)
        .unwrap();
    assert_eq!(
        store.lookup_completed_effect("effect-0").unwrap(),
        Some(effect)
    );

    let question = QuestionOpenPayload {
        question_id: "question-0".into(),
        recipient_ref: "recipient-0".into(),
        scope_ref: "scope-0".into(),
        text: "Approve interpretation?".into(),
    };
    let state = store.observe("task-0").unwrap().unwrap();
    let mut entry = OperationEntry {
        operation_id: "op-question".into(),
        task_id: "task-0".into(),
        subject_id: "subject-0".into(),
        delivery_id: "delivery-0".into(),
        attempt_id: "attempt-0".into(),
        expected_task_revision: state.revision,
        expected_cursor_revision: state.cursor_revision,
        grant_ref: "grant-0".into(),
        grant_revision: 1,
        payload_sha256: json_digest(&question).unwrap(),
    };
    let commit = store
        .open_question(&entry, &question, &mut |write| write())
        .unwrap();
    assert_eq!(
        store.lookup_operation("op-question").unwrap().unwrap(),
        commit
    );
    assert_eq!(
        store
            .open_question(&entry, &question, &mut |_write| {
                panic!("idempotent question readback must not reacquire grant fence")
            })
            .unwrap(),
        commit
    );
    let mut forged_entry = entry.clone();
    forged_entry.delivery_id = "delivery-other".into();
    assert!(matches!(
        store.open_question(&forged_entry, &question, &mut |write| write()),
        Err(TaskError::Conflict)
    ));
    let state = store.observe("task-0").unwrap().unwrap();
    let result = ResultPublishPayload {
        result_id: "result-0".into(),
        body: "The handler owns it.".into(),
        source_refs: vec![
            state
                .sources
                .iter()
                .find(|s| s.id == "objective")
                .unwrap()
                .reference(),
        ],
        attribution: "native-0".into(),
    };
    entry.operation_id = "op-result".into();
    entry.expected_task_revision = state.revision;
    entry.expected_cursor_revision = state.cursor_revision;
    entry.payload_sha256 = json_digest(&result).unwrap();
    assert!(matches!(
        store.publish_result(&entry, &result, &mut |write| write()),
        Err(TaskError::PendingQuestion)
    ));
    store
        .answer(&AnswerQuestion {
            command_id: "answer-0".into(),
            task_id: "task-0".into(),
            question_id: "question-0".into(),
            recipient_ref: "recipient-0".into(),
            scope_ref: "scope-0".into(),
            answer: "Approved".into(),
            expected_revision: state.revision,
            authority_ref: "human-operator".into(),
        })
        .unwrap();
    let state = store.observe("task-0").unwrap().unwrap();
    entry.expected_task_revision = state.revision;
    entry.expected_cursor_revision = state.cursor_revision;
    assert!(matches!(
        store.publish_result(&entry, &result, &mut |_write| Err(TaskError::Stale)),
        Err(TaskError::Stale)
    ));
    assert!(store.lookup_operation("op-result").unwrap().is_none());
    let committed = store
        .publish_result(&entry, &result, &mut |write| write())
        .unwrap();
    assert_eq!(
        store.lookup_operation("op-result").unwrap(),
        Some(committed.clone())
    );
    assert_eq!(
        store
            .publish_result(&entry, &result, &mut |_write| {
                panic!("idempotent result readback must not reacquire grant fence")
            })
            .unwrap(),
        committed
    );
    let changed_result = ResultPublishPayload {
        body: "A different answer".into(),
        ..result
    };
    assert!(matches!(
        store.publish_result(&entry, &changed_result, &mut |_write| {
            panic!("conflicting replay must not reacquire grant fence")
        }),
        Err(TaskError::Conflict)
    ));
}

fn dependency(source: &lifecycle_semantic::SourceMaterial) -> DependencyRevision {
    DependencyRevision {
        owner_namespace: source.owner_namespace.clone(),
        dependency_key: source.dependency_key.clone(),
        owner_ref: source.owner_ref.clone(),
        revision: source.revision,
        sha256: source.sha256.clone(),
    }
}

#[test]
fn owner_material_keeps_governing_and_pending_authority_distinct() {
    let root = tempfile::tempdir().unwrap();
    let store =
        TaskStore::open(&root.path().join("tasks.sqlite"), "human-operator".into()).unwrap();
    let mut first = task();
    first.facts = vec![
        FactInput {
            id: "governing".into(),
            kind: FactKind::Governing,
            text: "Approval is required before mutation".into(),
            current: true,
        },
        FactInput {
            id: "decision".into(),
            kind: FactKind::Decision,
            text: "Use the approved snapshot".into(),
            current: true,
        },
        FactInput {
            id: "commitment".into(),
            kind: FactKind::Commitment,
            text: "Preserve read-only analysis".into(),
            current: true,
        },
        FactInput {
            id: "uncertainty".into(),
            kind: FactKind::Uncertainty,
            text: "Ownership is unresolved".into(),
            current: true,
        },
        FactInput {
            id: "historical".into(),
            kind: FactKind::Decision,
            text: "Old superseded choice".into(),
            current: false,
        },
    ];
    store.submit(&first).unwrap();
    let subject = SubjectId::parse("subject-0").unwrap();
    let transition = TransitionId::parse("transition-0").unwrap();
    let projected = store.project(&subject, &transition, "capture-0").unwrap();
    let governing = projected
        .sources
        .iter()
        .find(|s| s.id == "governing")
        .unwrap();
    assert_eq!(governing.trust, SourceTrust::HostGoverning);
    for (id, kind) in [
        ("governing", ObligationKind::Governing),
        ("decision", ObligationKind::Decision),
        ("commitment", ObligationKind::Commitment),
        ("uncertainty", ObligationKind::Uncertainty),
    ] {
        assert!(
            projected
                .obligations
                .iter()
                .any(|o| o.id == format!("obligation-{id}") && o.kind == kind)
        );
    }
    assert!(
        projected
            .sources
            .iter()
            .any(|s| s.id == "historical" && !s.current)
    );
    assert!(
        !projected
            .obligations
            .iter()
            .any(|o| o.id == "obligation-historical")
    );
    let objective = dependency(
        projected
            .sources
            .iter()
            .find(|s| s.id == "objective")
            .unwrap(),
    );

    // A queued followup is durable work intent; it does not rewrite the active
    // question or stale a captured objective dependency before activation.
    store
        .submit(&SubmitTask {
            command_id: "cmd-1".into(),
            engineering_question: "Which caller?".into(),
            facts: Vec::new(),
            ..first
        })
        .unwrap();
    assert_eq!(
        store
            .observe_dependencies(std::slice::from_ref(&objective))
            .unwrap(),
        vec![DependencyObservation::Current(objective.clone())]
    );
    assert_eq!(
        store
            .observe("task-0")
            .unwrap()
            .unwrap()
            .engineering_question,
        "What owns the request?"
    );

    let state = store.observe("task-0").unwrap().unwrap();
    let question = QuestionOpenPayload {
        question_id: "question-1".into(),
        recipient_ref: "human-operator".into(),
        scope_ref: "scope-1".into(),
        text: "May this task mutate?".into(),
    };
    let entry = OperationEntry {
        operation_id: "operation-1".into(),
        task_id: "task-0".into(),
        subject_id: "subject-0".into(),
        delivery_id: "delivery-0".into(),
        attempt_id: "attempt-0".into(),
        expected_task_revision: state.revision,
        expected_cursor_revision: state.cursor_revision,
        grant_ref: "grant-0".into(),
        grant_revision: 1,
        payload_sha256: json_digest(&question).unwrap(),
    };
    store
        .open_question(&entry, &question, &mut |write| write())
        .unwrap();
    let pending = store.observe("task-0").unwrap().unwrap();
    let question_dependency = dependency(
        pending
            .sources
            .iter()
            .find(|s| s.id == "question-question-1")
            .unwrap(),
    );
    let answer = AnswerQuestion {
        command_id: "answer-1".into(),
        task_id: "task-0".into(),
        question_id: "question-1".into(),
        recipient_ref: "human-operator".into(),
        scope_ref: "scope-1".into(),
        answer: "No mutation authorized".into(),
        expected_revision: pending.revision,
        authority_ref: "human-operator".into(),
    };
    let mut denied = answer.clone();
    denied.authority_ref = "untrusted".into();
    assert!(matches!(
        store.answer(&denied),
        Err(TaskError::Unauthorized)
    ));
    denied = answer.clone();
    denied.scope_ref = "other-scope".into();
    assert!(matches!(store.answer(&denied), Err(TaskError::Conflict)));
    assert_eq!(
        store.observe("task-0").unwrap().unwrap().revision,
        pending.revision
    );
    let answered_commit = store.answer(&answer).unwrap();
    assert_eq!(
        store
            .lookup_command_receipt(
                "answer",
                "task-0",
                "answer-1",
                &json_digest(&answer).unwrap()
            )
            .unwrap(),
        Some(answered_commit)
    );
    assert!(
        matches!(store.observe_dependencies(std::slice::from_ref(&question_dependency)).unwrap().as_slice(), [DependencyObservation::Current(now)] if now.revision == question_dependency.revision + 1)
    );
    assert_eq!(
        store
            .observe_dependencies(std::slice::from_ref(&objective))
            .unwrap(),
        vec![DependencyObservation::Current(objective)]
    );
    let mut stale_committed = false;
    assert!(matches!(
        store.with_current_fence(std::slice::from_ref(&question_dependency), &mut || {
            stale_committed = true;
            Ok(())
        }),
        Err(ServiceError::Material(MaterialPortError::Conflict))
    ));
    assert!(!stale_committed);
    let current_objective = dependency(
        store
            .observe("task-0")
            .unwrap()
            .unwrap()
            .sources
            .iter()
            .find(|s| s.id == "objective")
            .unwrap(),
    );
    store
        .with_current_fence(std::slice::from_ref(&current_objective), &mut || {
            stale_committed = true;
            Ok(())
        })
        .unwrap();
    assert!(stale_committed);
    assert!(matches!(
        store.answer(&AnswerQuestion {
            command_id: "answer-stale".into(),
            ..answer
        }),
        Err(TaskError::Stale)
    ));
    assert!(
        store
            .project(
                &SubjectId::parse("other-subject").unwrap(),
                &transition,
                "capture-1"
            )
            .is_err()
    );
}

#[test]
fn task_port_refuses_wrong_contract_and_cursor_without_owner_mutation() {
    let root = tempfile::tempdir().unwrap();
    let store =
        TaskStore::open(&root.path().join("tasks.sqlite"), "human-operator".into()).unwrap();
    store.submit(&task()).unwrap();
    let state = store.observe("task-0").unwrap().unwrap();
    let payload = PortQuestion {
        question_id: "question-0".into(),
        recipient_ref: "human-operator".into(),
        scope_ref: "scope-0".into(),
        text: "Approve?".into(),
    };
    let mut entry = ValidatedTaskOperationEntry {
        operation_id: "operation-0".into(),
        task_id: "task-0".into(),
        subject_id: "subject-0".into(),
        logical_delivery_id: "delivery-0".into(),
        attempt_id: "attempt-0".into(),
        cursor_ref: "task-0".into(),
        cursor_revision: state.cursor_revision,
        expected_task_revision: state.revision,
        grant_ref: "grant-0".into(),
        grant_revision: 1,
        payload_sha256: json_digest(&payload).unwrap(),
        contract_id: "wrong.contract.v1".into(),
    };
    let mut fence = |write: &mut dyn FnMut() -> Result<(), TaskOperationError>| write();
    assert_eq!(
        TaskOperationPort::open_question(&store, &entry, payload.clone(), &mut fence),
        Err(TaskOperationError::Invalid)
    );
    entry.contract_id = "task.question.open.v1".into();
    entry.cursor_ref = "forged-task".into();
    assert_eq!(
        TaskOperationPort::open_question(&store, &entry, payload.clone(), &mut fence),
        Err(TaskOperationError::Invalid)
    );
    entry.cursor_ref = "task-0".into();
    entry.payload_sha256 = sha256(b"forged payload");
    assert_eq!(
        TaskOperationPort::open_question(&store, &entry, payload, &mut fence),
        Err(TaskOperationError::Invalid)
    );
    assert!(
        store
            .observe("task-0")
            .unwrap()
            .unwrap()
            .questions
            .is_empty()
    );
    assert!(store.lookup_operation("operation-0").unwrap().is_none());
}

#[test]
fn oversized_or_reserved_owner_input_never_partially_publishes() {
    let root = tempfile::tempdir().unwrap();
    let store =
        TaskStore::open(&root.path().join("tasks.sqlite"), "human-operator".into()).unwrap();
    let oversized = SubmitTask {
        objective: "x".repeat(8193),
        ..task()
    };
    assert!(matches!(store.submit(&oversized), Err(TaskError::Invalid)));
    assert!(store.observe("task-0").unwrap().is_none());

    let reserved = SubmitTask {
        facts: vec![FactInput {
            id: "objective".into(),
            kind: FactKind::Governing,
            text: "forged authority".into(),
            current: true,
        }],
        ..task()
    };
    assert!(matches!(store.submit(&reserved), Err(TaskError::Invalid)));
    assert!(store.observe("task-0").unwrap().is_none());

    let near_bound = SubmitTask {
        objective: "x".repeat(8192),
        ..task()
    };
    let committed = store.submit(&near_bound).unwrap();
    assert_eq!(committed.owner_revision, 1);
    assert_eq!(
        store.observe("task-0").unwrap().unwrap().objective.len(),
        8192
    );

    let changed_payload = SubmitTask {
        objective: "different".into(),
        ..task()
    };
    assert!(matches!(
        store.submit(&changed_payload),
        Err(TaskError::Conflict)
    ));
    assert_eq!(store.observe("task-0").unwrap().unwrap().revision, 1);
}

#[cfg(feature = "controlled-proof")]
#[test]
fn controlled_result_barrier_holds_owner_lock_before_grant_fence() {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        thread,
        time::{Duration, Instant},
    };

    let root = tempfile::tempdir().unwrap();
    let barrier = root.path().join("barrier");
    std::fs::create_dir(&barrier).unwrap();
    std::fs::write(barrier.join("task_result_after_owner_begin.arm"), b"arm\n").unwrap();
    let store = Arc::new(
        TaskStore::open(&root.path().join("tasks.sqlite"), "human-operator".into())
            .unwrap()
            .with_controlled_proof_barrier(Some(barrier.clone()))
            .unwrap(),
    );
    store.submit(&task()).unwrap();
    let state = store.observe("task-0").unwrap().unwrap();
    let payload = ResultPublishPayload {
        result_id: "result-0".into(),
        body: "Attribution".into(),
        source_refs: vec![
            state
                .sources
                .iter()
                .find(|s| s.id == "objective")
                .unwrap()
                .reference(),
        ],
        attribution: "source-0".into(),
    };
    let entry = OperationEntry {
        operation_id: "result-operation-0".into(),
        task_id: "task-0".into(),
        subject_id: "subject-0".into(),
        delivery_id: "delivery-0".into(),
        attempt_id: "attempt-0".into(),
        expected_task_revision: state.revision,
        expected_cursor_revision: state.cursor_revision,
        grant_ref: "grant-0".into(),
        grant_revision: 1,
        payload_sha256: json_digest(&payload).unwrap(),
    };
    let fence_entered = Arc::new(AtomicBool::new(false));
    let worker_store = store.clone();
    let worker_fence = fence_entered.clone();
    let publish = thread::spawn(move || {
        worker_store.publish_result(&entry, &payload, &mut |write| {
            worker_fence.store(true, Ordering::SeqCst);
            write()
        })
    });
    let reached = barrier.join("task_result_after_owner_begin.reached");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !reached.exists() {
        assert!(Instant::now() < deadline, "owner barrier was not reached");
        thread::sleep(Duration::from_millis(5));
    }
    assert!(!fence_entered.load(Ordering::SeqCst));
    let (tx, rx) = mpsc::channel();
    let observer = store.clone();
    let read = thread::spawn(move || tx.send(observer.observe("task-0")).unwrap());
    assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
    std::fs::write(
        barrier.join("task_result_after_owner_begin.release"),
        b"release\n",
    )
    .unwrap();
    assert_eq!(
        publish.join().unwrap().unwrap().kind,
        "task.result.publish.v1"
    );
    assert!(fence_entered.load(Ordering::SeqCst));
    assert!(
        rx.recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap()
            .unwrap()
            .result
            .is_some()
    );
    read.join().unwrap();
}
