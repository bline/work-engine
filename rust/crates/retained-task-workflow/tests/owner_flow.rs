use lifecycle_core::{SubjectId, TransitionId};
use lifecycle_semantic::{DependencyObservation, DependencyRevision, ObligationKind, SourceTrust};
use lifecycle_service::{
    CanonicalMaterialPort, MaterialPortError, QuestionOpenPayload as PortQuestion, ServiceError,
    TaskOperationError, TaskOperationPort, ValidatedTaskOperationEntry,
};
use retained_task_workflow::{
    AdmissionCommand, AnswerQuestion, CompletedEffect, FactInput, FactKind, OperationEntry,
    QuestionOpenPayload, ResultPublishPayload, SubjectKind, SubmitTask, TaskError, TaskStore,
    json_digest, sha256,
};
use rusqlite::Connection;

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
fn submit_and_admission_are_one_owner_commit_with_exact_duplicate_readback() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tasks.sqlite");
    let store = TaskStore::open(&path, "human-operator".into()).unwrap();
    let input = task();
    let mut interrupted = |_text: &str| Err(TaskError::Unavailable);
    assert!(
        store
            .submit_with_admission(&input, &mut interrupted)
            .is_err()
    );
    assert!(store.observe("task-0").unwrap().is_none());
    assert!(store.lookup_admission_command("cmd-0").unwrap().is_none());
    assert!(
        store
            .lookup_command_receipt("submit", "task-0", "cmd-0", &json_digest(&input).unwrap())
            .unwrap()
            .is_none()
    );

    let digest = "a".repeat(64);
    let mut build = |text: &str| {
        Ok(AdmissionCommand {
            command_id: "cmd-0".into(),
            request_digest: digest.clone(),
            command_json: serde_json::json!({
                "protocol_version":2,"principal_ref":"principal-0","command_id":"cmd-0",
                "subject_id":"subject-0","context_generation":"context-0","build_id":"build-0",
                "proof_run_id":"proof-0","grant_ref":"grant-0","expected_revision":"1",
                "request_digest":digest,"kind":"enqueue_input",
                "payload":{"input_id":"input:cmd-0","producer_ref":"producer-0","text":text,"text_digest":"b".repeat(64)}
            }).to_string(),
        })
    };
    let (first, published) = store.submit_with_admission(&input, &mut build).unwrap();
    assert!(published);
    assert_eq!(first.owner_revision, 1);
    assert_eq!(store.admission_commands().unwrap().len(), 1);
    let saved = store.lookup_admission_command("cmd-0").unwrap().unwrap();
    assert_eq!(saved.request_digest, "a".repeat(64));
    assert_eq!(
        store.input_text("cmd-0").unwrap().unwrap(),
        serde_json::from_str::<serde_json::Value>(&saved.command_json).unwrap()["payload"]["text"]
            .as_str()
            .unwrap()
    );
    drop(store);

    let reopened = TaskStore::open(&path, "human-operator".into()).unwrap();
    let mut never_rebuild = |_text: &str| -> Result<AdmissionCommand, TaskError> {
        panic!("duplicate must preserve recorded command and owner receipt")
    };
    let (same, published) = reopened
        .submit_with_admission(&input, &mut never_rebuild)
        .unwrap();
    assert!(!published);
    assert_eq!(same, first);
    assert_eq!(
        reopened.lookup_admission_command("cmd-0").unwrap().unwrap(),
        saved
    );
    let changed = SubmitTask {
        objective: "different".into(),
        ..input
    };
    assert!(matches!(
        reopened.submit_with_admission(&changed, &mut never_rebuild),
        Err(TaskError::Conflict)
    ));
    assert_eq!(reopened.observe("task-0").unwrap().unwrap().revision, 1);
}

#[test]
fn existing_owner_schema_is_checked_before_repair_or_wal_change() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tasks.sqlite");
    let store = TaskStore::open(&path, "human-operator".into()).unwrap();
    let store_id = store.store_id().unwrap();
    let submit = store.submit(&task()).unwrap();
    drop(store);

    let reopened = TaskStore::open(&path, "human-operator".into()).unwrap();
    assert_eq!(reopened.store_id().unwrap(), store_id);
    assert_eq!(
        reopened.observe("task-0").unwrap().unwrap().revision,
        submit.owner_revision
    );
    drop(reopened);

    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("ALTER TABLE operations RENAME TO operations_missing;")
        .unwrap();
    let before: Vec<(String, String)> = connection
        .prepare("SELECT name, sql FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let before_task: Vec<u8> = connection
        .query_row(
            "SELECT record_json FROM tasks WHERE task_id='task-0'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);
    assert!(matches!(
        TaskStore::open(&path, "human-operator".into()),
        Err(TaskError::Invalid)
    ));
    let connection = Connection::open(&path).unwrap();
    let after: Vec<(String, String)> = connection
        .prepare("SELECT name, sql FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let after_task: Vec<u8> = connection
        .query_row(
            "SELECT record_json FROM tasks WHERE task_id='task-0'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(after, before);
    assert_eq!(after_task, before_task);
    assert!(!after.iter().any(|(name, _)| name == "operations"));
}

#[test]
fn altered_owner_table_is_not_accepted_as_current_schema() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("tasks.sqlite");
    drop(TaskStore::open(&path, "human-operator".into()).unwrap());
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "DROP TABLE operations;
         CREATE TABLE operations (
           operation_id TEXT PRIMARY KEY, kind TEXT NOT NULL,
           payload_digest TEXT NOT NULL, entry_digest TEXT,
           commit_json BLOB NOT NULL
         );",
        )
        .unwrap();
    let before: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='operations'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);
    assert!(matches!(
        TaskStore::open(&path, "human-operator".into()),
        Err(TaskError::Invalid)
    ));
    let connection = Connection::open(&path).unwrap();
    let after: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='operations'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(after, before);
}

#[test]
fn only_recognized_legacy_owner_layout_is_migrated() {
    let root = tempfile::tempdir().unwrap();
    let unknown = root.path().join("unknown.sqlite");
    Connection::open(&unknown).unwrap();
    assert!(matches!(
        TaskStore::open(&unknown, "human-operator".into()),
        Err(TaskError::Invalid)
    ));

    let path = root.path().join("legacy.sqlite");
    let store = TaskStore::open(&path, "human-operator".into()).unwrap();
    store.submit(&task()).unwrap();
    drop(store);
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TABLE store_identity; PRAGMA user_version=0;")
        .unwrap();
    drop(connection);
    let migrated = TaskStore::open(&path, "human-operator".into()).unwrap();
    assert_eq!(migrated.observe("task-0").unwrap().unwrap().revision, 1);
    assert_eq!(migrated.store_id().unwrap().len(), 32);
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
