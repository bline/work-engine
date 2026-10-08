//! HP1 direct Rust library contract; no JSON wire loop between components.
#[allow(dead_code)]
mod support;

use review_episode::admission::FixtureAdmission;
use review_episode::operation::Operation;
use review_episode::protocol::{DirectRequest, Request, RequestSelection};
use review_episode::{AppError, Application, OperationResult, PossibleCommit, WriteStatus};
use review_episode_core::codec::{JsString, JsValue, canonical_json, digest, parse_json};
use review_episode_core::command::{Action, BeginCommand, TransitionCommand};
use review_episode_core::identity::{Authority, Identity, Revision};
use review_episode_core::reducer;
use review_episode_store::{EpisodeStore, StoreError, StoreOptions, init_offline_root};
use std::fs;

fn direct(id: &str, operation: Operation) -> DirectRequest {
    DirectRequest::new(
        RequestSelection::Offline {
            request_id: id.into(),
            grant_id: format!("grant-{id}"),
            selection_revision: Revision("a".repeat(64)),
        },
        operation,
    )
    .unwrap()
}
fn fixture(
    path: &std::path::Path,
    root: &std::path::Path,
    entries: &[(&DirectRequest, Option<&str>)],
) {
    let grants = entries
        .iter()
        .map(|(req, observed)| {
            JsValue::object([
                (
                    "grantId",
                    JsValue::text(&format!("grant-{}", req.request().request_id())),
                ),
                (
                    "requestDigest",
                    JsValue::text(&digest(req.request().envelope())),
                ),
                ("selectionRevision", JsValue::text(&"a".repeat(64))),
                (
                    "observedRevision",
                    observed.map_or(JsValue::Null, JsValue::text),
                ),
            ])
        })
        .collect();
    let registry = JsValue::object([
        (
            "root",
            JsValue::text(root.canonicalize().unwrap().to_str().unwrap()),
        ),
        ("grants", JsValue::Array(grants)),
    ]);
    fs::write(path, canonical_json(&registry)).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}
fn app(
    root: &std::path::Path,
    registry: &std::path::Path,
    limit: usize,
) -> Application<FixtureAdmission> {
    Application::new(
        EpisodeStore::open(root, StoreOptions::default()).unwrap(),
        FixtureAdmission::load(registry, root).unwrap(),
        limit,
    )
}
fn state(
    result: OperationResult,
    expected: WriteStatus,
) -> review_episode_core::state::ReviewEpisodeState {
    match result {
        OperationResult::Write { status, state, .. } => {
            assert_eq!(status, expected);
            state
        }
        _ => panic!("expected write result"),
    }
}

#[test]
fn typed_request_rejects_invalid_identity_revision_and_recovery_fields_before_store() {
    let valid = Authority::parse(support::authority())
        .unwrap()
        .identity()
        .clone();
    let mut empty_identity = valid.clone();
    empty_identity.run_id = JsString::new("   ");
    let mut unsafe_identity = valid.clone();
    unsafe_identity.slice_number = 9_007_199_254_740_992;
    let invalid = [
        Operation::Read {
            identity: valid.clone(),
            revision: Some(Revision("bad".into())),
        },
        Operation::History {
            identity: empty_identity,
        },
        Operation::Read {
            identity: unsafe_identity,
            revision: None,
        },
        Operation::Recover {
            identity: valid.clone(),
            transition: Some((JsString::new(" \t"), Revision("a".repeat(64)))),
        },
        Operation::Recover {
            identity: valid,
            transition: Some((JsString::new("begin"), Revision("wrong".into()))),
        },
    ];
    for (index, operation) in invalid.into_iter().enumerate() {
        let result = DirectRequest::new(
            RequestSelection::Offline {
                request_id: format!("invalid-{index}"),
                grant_id: "grant-invalid".into(),
                selection_revision: Revision("a".repeat(64)),
            },
            operation,
        );
        assert!(
            matches!(result, Err(AppError::Domain(ref error)) if error.kind == review_episode_core::ErrorKind::InvalidCommand),
            "invalid typed request {index} was accepted"
        );
    }
    let native_invalid = DirectRequest::new(
        RequestSelection::Native {
            request_id: "native-invalid".into(),
            grant_id: "grant-invalid".into(),
            selection_revision: Revision("a".repeat(64)),
            root_selection_digest: Revision("b".repeat(64)),
        },
        Operation::Read {
            identity: Identity {
                run_id: JsString::new(""),
                ..Authority::parse(support::authority())
                    .unwrap()
                    .identity()
                    .clone()
            },
            revision: None,
        },
    );
    assert!(matches!(native_invalid, Err(AppError::Domain(_))));
}

#[test]
fn wire_and_typed_paths_reject_the_same_malformed_read_revision() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("episode");
    init_offline_root(&root).unwrap();
    let registry = temp.path().join("grants.json");
    fixture(&registry, &root, &[]);
    let identity = Authority::parse(support::authority())
        .unwrap()
        .identity()
        .clone();
    let typed = DirectRequest::new(
        RequestSelection::Offline {
            request_id: "invalid-wire-parity".into(),
            grant_id: "grant-invalid-wire-parity".into(),
            selection_revision: Revision("a".repeat(64)),
        },
        Operation::Read {
            identity: identity.clone(),
            revision: Some(Revision("bad".into())),
        },
    );
    assert!(matches!(typed, Err(AppError::Domain(_))));
    let wire = JsValue::object([
        ("version", JsValue::Number(1.0)),
        ("domain", JsValue::text("review-episode")),
        ("requestId", JsValue::text("invalid-wire-parity")),
        ("operation", JsValue::text("read")),
        ("grantId", JsValue::text("grant-invalid-wire-parity")),
        ("selectionRevision", JsValue::text(&"a".repeat(64))),
        (
            "args",
            JsValue::object([
                ("identity", identity.value()),
                ("revision", JsValue::text("bad")),
            ]),
        ),
    ]);
    let request = Request::parse(canonical_json(&wire).as_bytes()).unwrap();
    let mut owner = app(&root, &registry, 32 * 1024 * 1024);
    assert_eq!(
        owner.execute_typed(&request).unwrap_err().kind(),
        "InvalidCommand"
    );
}

#[test]
fn seven_typed_operations_share_one_admitted_owner_and_preserve_exact_results() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("episode");
    init_offline_root(&root).unwrap();
    let registry = temp.path().join("grants.json");
    let authority = Authority::parse(support::authority()).unwrap();
    let identity = authority.identity().clone();
    let result_value = parse_json(support::ACCEPTABLE).unwrap();
    let first = reducer::begin(
        None,
        &BeginCommand::new(authority.clone(), "begin", vec![]).unwrap(),
    )
    .unwrap()
    .into_state();
    let payload = JsValue::object([
        ("result", result_value.clone()),
        ("unresolvedQuestions", JsValue::Array(vec![])),
    ]);
    let second = reducer::transition(
        &first,
        &TransitionCommand::new(
            authority.clone(),
            first.revision().clone(),
            "result",
            Action::RecordResult,
            payload.clone(),
        )
        .unwrap(),
    )
    .unwrap()
    .into_state();
    let begin = direct(
        "begin",
        Operation::Begin {
            authority: authority.clone(),
            transition_id: JsString::new("begin"),
            unresolved_questions: vec![],
        },
    );
    let resume = direct(
        "resume",
        Operation::ResumeInitial {
            authority: authority.clone(),
        },
    );
    let validate = direct(
        "validate",
        Operation::ValidateResult {
            authority: authority.clone(),
            expected_revision: first.revision().clone(),
            result: result_value,
        },
    );
    let transition = direct(
        "transition",
        Operation::Transition {
            authority: authority.clone(),
            expected_revision: first.revision().clone(),
            transition_id: JsString::new("result"),
            action: Action::RecordResult,
            payload,
        },
    );
    let read = direct(
        "read",
        Operation::Read {
            identity: identity.clone(),
            revision: Some(first.revision().clone()),
        },
    );
    let history = direct(
        "history",
        Operation::History {
            identity: identity.clone(),
        },
    );
    let content = JsValue::object([
        ("action", JsValue::text("begin")),
        ("unresolvedQuestions", JsValue::Array(vec![])),
    ]);
    let recover = direct(
        "recover",
        Operation::Recover {
            identity: identity.clone(),
            transition: Some((JsString::new("begin"), Revision(digest(&content)))),
        },
    );
    fixture(
        &registry,
        &root,
        &[
            (&begin, None),
            (&resume, Some(&first.revision().0)),
            (&validate, Some(&first.revision().0)),
            (&transition, Some(&first.revision().0)),
            (&read, Some(&second.revision().0)),
            (&history, Some(&second.revision().0)),
            (&recover, Some(&second.revision().0)),
        ],
    );
    let mut owner = app(&root, &registry, 32 * 1024 * 1024);
    let applied = owner.execute_direct(&begin).unwrap();
    let bound = applied.transition_context().unwrap();
    assert_eq!(bound.root, root);
    assert_eq!(bound.operation, "begin");
    assert_eq!(bound.transition_id, JsString::new("begin"));
    assert_eq!(bound.content_digest.0, digest(&content));
    assert_eq!(
        state(applied, WriteStatus::Applied).revision(),
        first.revision()
    );
    match owner.execute_direct(&resume).unwrap() {
        OperationResult::ObservedState { state: Some(s), .. } => {
            assert_eq!(s.revision(), first.revision())
        }
        _ => panic!("resume"),
    }
    match owner.execute_direct(&validate).unwrap() {
        OperationResult::ValidatedResult {
            observed_revision, ..
        } => assert_eq!(observed_revision, *first.revision()),
        _ => panic!("validate"),
    }
    assert_eq!(
        state(
            owner.execute_direct(&transition).unwrap(),
            WriteStatus::Applied
        )
        .revision(),
        second.revision()
    );
    match owner.execute_direct(&read).unwrap() {
        OperationResult::ObservedState {
            state: Some(s),
            observed_revision: Some(r),
        } => {
            assert_eq!(s.revision(), first.revision());
            assert_eq!(r, *second.revision());
        }
        _ => panic!("read"),
    }
    match owner.execute_direct(&history).unwrap() {
        OperationResult::History {
            history,
            observed_revision: Some(r),
        } => {
            assert_eq!(history.len(), 2);
            assert_eq!(r, *second.revision());
        }
        _ => panic!("history"),
    }
    match owner.execute_direct(&recover).unwrap() {
        OperationResult::Recovery {
            committed_revision: Some(r),
            ..
        } => assert_eq!(r, *first.revision()),
        _ => panic!("recover"),
    }
    assert_eq!(owner.root(), root);
}

#[test]
fn direct_write_exact_capacity_refuses_before_commit_and_replay_never_appends() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("episode");
    init_offline_root(&root).unwrap();
    let registry = temp.path().join("grants.json");
    let authority = Authority::parse(support::authority()).unwrap();
    let key = authority.identity().key().0;
    let begin = direct(
        "begin",
        Operation::Begin {
            authority: authority.clone(),
            transition_id: JsString::new("begin"),
            unresolved_questions: vec![],
        },
    );
    let expected = reducer::begin(
        None,
        &BeginCommand::new(authority, "begin", vec![]).unwrap(),
    )
    .unwrap()
    .into_state();
    let context = PossibleCommit {
        root: root.clone(),
        operation: "begin",
        identity: expected.identity().clone(),
        transition_id: JsString::new("begin"),
        content_digest: Revision(digest(&JsValue::object([
            ("action", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]))),
    };
    let exact = OperationResult::Write {
        status: WriteStatus::Applied,
        state: expected.clone(),
        context: context.clone(),
    }
    .encoded_len(begin.request());
    fixture(&registry, &root, &[(&begin, None)]);
    let mut owner = app(&root, &registry, exact - 1);
    assert!(matches!(
        owner.execute_direct(&begin),
        Err(AppError::Store(StoreError::ResponseTooLarge))
    ));
    drop(owner);
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let snapshot = store.read(&key).unwrap();
    assert!(snapshot.current.is_none() && snapshot.history.is_empty());
    drop(store);
    let mut owner = app(&root, &registry, exact);
    assert_eq!(
        state(owner.execute_direct(&begin).unwrap(), WriteStatus::Applied).revision(),
        expected.revision()
    );
    drop(owner);
    fixture(&registry, &root, &[(&begin, Some(&expected.revision().0))]);
    let replay_exact = OperationResult::Write {
        status: WriteStatus::Replay,
        state: expected.clone(),
        context,
    }
    .encoded_len(begin.request());
    let mut too_small = app(&root, &registry, replay_exact - 1);
    assert!(matches!(
        too_small.execute_direct(&begin),
        Err(AppError::Store(StoreError::ResponseTooLarge))
    ));
    drop(too_small);
    let mut owner = app(&root, &registry, replay_exact);
    assert_eq!(
        state(owner.execute_direct(&begin).unwrap(), WriteStatus::Replay).revision(),
        expected.revision()
    );
    drop(owner);
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    assert_eq!(store.read(&key).unwrap().history.len(), 1);
}

#[test]
fn direct_admission_rejects_mutated_scope_and_stale_revision() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("episode");
    init_offline_root(&root).unwrap();
    let registry = temp.path().join("grants.json");
    let authority = Authority::parse(support::authority()).unwrap();
    let begin = direct(
        "begin",
        Operation::Begin {
            authority: authority.clone(),
            transition_id: JsString::new("begin"),
            unresolved_questions: vec![],
        },
    );
    fixture(&registry, &root, &[(&begin, None)]);
    let mut owner = app(&root, &registry, 32 * 1024 * 1024);
    let changed = direct(
        "begin",
        Operation::Begin {
            authority,
            transition_id: JsString::new("other"),
            unresolved_questions: vec![],
        },
    );
    assert!(matches!(
        owner.execute_direct(&changed),
        Err(AppError::Admission(_))
    ));
    let native_under_fixture = DirectRequest::new(
        RequestSelection::Native {
            request_id: "begin".into(),
            grant_id: "grant-begin".into(),
            selection_revision: Revision("a".repeat(64)),
            root_selection_digest: Revision("b".repeat(64)),
        },
        Operation::History {
            identity: Authority::parse(support::authority())
                .unwrap()
                .identity()
                .clone(),
        },
    )
    .unwrap();
    assert!(matches!(
        owner.execute_direct(&native_under_fixture),
        Err(AppError::Admission(_))
    ));
    let first = state(owner.execute_direct(&begin).unwrap(), WriteStatus::Applied);
    drop(owner);
    let stale = direct(
        "stale",
        Operation::Transition {
            authority: Authority::parse(support::authority()).unwrap(),
            expected_revision: Revision("0".repeat(64)),
            transition_id: JsString::new("stale"),
            action: Action::MarkUncertain,
            payload: JsValue::object([
                ("reason", JsValue::text("stale")),
                ("reconciliationAction", JsValue::text("read")),
            ]),
        },
    );
    fixture(&registry, &root, &[(&stale, Some(&first.revision().0))]);
    let mut owner = app(&root, &registry, 32 * 1024 * 1024);
    assert_eq!(
        owner.execute_direct(&stale).unwrap_err().kind(),
        "RevisionConflict"
    );
}

#[test]
fn direct_utf16_identity_survives_store_and_exact_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("episode");
    init_offline_root(&root).unwrap();
    let registry = temp.path().join("grants.json");
    let authority = Authority::parse(support::authority()).unwrap();
    let identity = authority.identity().clone();
    let id = JsString(vec![0xd801]);
    let question = JsValue::String(JsString(vec![0xd800, b'Q' as u16]));
    let questions = vec![question];
    let content = JsValue::object([
        ("action", JsValue::text("begin")),
        ("unresolvedQuestions", JsValue::Array(questions.clone())),
    ]);
    let begin = direct(
        "unicode",
        Operation::Begin {
            authority,
            transition_id: id.clone(),
            unresolved_questions: questions,
        },
    );
    fixture(&registry, &root, &[(&begin, None)]);
    let mut owner = app(&root, &registry, 32 * 1024 * 1024);
    let response = owner.execute_direct(&begin).unwrap();
    assert!(response.wire_json(begin.request()).contains("\\ud801"));
    let committed = state(response, WriteStatus::Applied);
    drop(owner);
    let recover = direct(
        "unicode-recover",
        Operation::Recover {
            identity: identity.clone(),
            transition: Some((id, Revision(digest(&content)))),
        },
    );
    fixture(
        &registry,
        &root,
        &[(&recover, Some(&committed.revision().0))],
    );
    let mut reopened = app(&root, &registry, 32 * 1024 * 1024);
    match reopened.execute_direct(&recover).unwrap() {
        OperationResult::Recovery {
            committed_revision: Some(r),
            ..
        } => assert_eq!(r, *committed.revision()),
        _ => panic!("Unicode transition not recovered"),
    }
}

#[cfg(feature = "test-faults")]
#[test]
fn direct_possible_commit_exposes_exact_context_and_reconciles_once() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("episode");
    init_offline_root(&root).unwrap();
    let registry = temp.path().join("grants.json");
    let authority = Authority::parse(support::authority()).unwrap();
    let identity = authority.identity().clone();
    let begin = direct(
        "begin",
        Operation::Begin {
            authority,
            transition_id: JsString::new("begin"),
            unresolved_questions: vec![],
        },
    );
    fixture(&registry, &root, &[(&begin, None)]);
    let mut owner = app(&root, &registry, 32 * 1024 * 1024);
    unsafe {
        std::env::set_var("REVIEW_EPISODE_FAULT_CUT", "after_commit_unknown");
    }
    let outcome = owner.execute_direct(&begin);
    unsafe {
        std::env::remove_var("REVIEW_EPISODE_FAULT_CUT");
    }
    let error = outcome.unwrap_err();
    let context = error.possible_commit().expect("possible commit context");
    assert_eq!(context.root, root);
    assert_eq!(context.operation, "begin");
    assert_eq!(context.identity, identity);
    assert_eq!(context.transition_id, JsString::new("begin"));
    let digest = context.content_digest.clone();
    drop(owner);
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let first = store.read(&identity.key().0).unwrap().current.unwrap();
    drop(store);
    let recover = direct(
        "recover",
        Operation::Recover {
            identity: identity.clone(),
            transition: Some((JsString::new("begin"), digest)),
        },
    );
    fixture(
        &registry,
        &root,
        &[
            (&recover, Some(&first.revision().0)),
            (&begin, Some(&first.revision().0)),
        ],
    );
    let mut owner = app(&root, &registry, 32 * 1024 * 1024);
    match owner.execute_direct(&recover).unwrap() {
        OperationResult::Recovery {
            committed_revision: Some(r),
            ..
        } => assert_eq!(r, *first.revision()),
        _ => panic!("not committed"),
    }
    assert_eq!(
        state(owner.execute_direct(&begin).unwrap(), WriteStatus::Replay).revision(),
        first.revision()
    );
    drop(owner);
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    assert_eq!(store.read(&identity.key().0).unwrap().history.len(), 1);
}
