mod support;
use review_episode_core::codec::{JsValue, field, parse_json};
use support::*;

#[test]
fn actual_binary_begin_result_exit_reopen_exact_read_and_recover() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let fixture = temp.path().join("registry.json");
    init(&root);
    let authority = authority();
    let identity = field(&authority, "identity").unwrap().clone();
    let begin = request(
        "begin",
        "begin-grant",
        "begin-1",
        JsValue::object([
            ("authority", authority.clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&fixture, &root, &begin, None);
    let first = invoke(&root, &fixture, &begin);
    assert_eq!(status(&first), "applied", "{first:?}");
    let first_revision = revision(&first);
    let result = parse_json(ACCEPTABLE).unwrap();
    let payload = JsValue::object([
        ("result", result),
        ("unresolvedQuestions", JsValue::Array(vec![])),
    ]);
    let transition = request(
        "transition",
        "result-grant",
        "result-1",
        JsValue::object([
            ("authority", authority),
            ("expectedRevision", JsValue::text(&first_revision)),
            ("transitionId", JsValue::text("initial-result")),
            ("action", JsValue::text("record_result")),
            ("payload", payload),
        ]),
    );
    write_fixture(&fixture, &root, &transition, Some(&first_revision));
    let second = invoke(&root, &fixture, &transition);
    assert_eq!(status(&second), "applied", "{second:?}");
    let second_revision = revision(&second);
    let read = request(
        "read",
        "read-grant",
        "read-1",
        JsValue::object([
            ("identity", identity.clone()),
            ("revision", JsValue::text(&first_revision)),
        ]),
    );
    write_fixture(&fixture, &root, &read, Some(&second_revision));
    let exact = invoke(&root, &fixture, &read);
    assert_eq!(status(&exact), "observed");
    assert_eq!(
        field(field(&exact, "state").unwrap(), "revision").unwrap(),
        &JsValue::text(&first_revision)
    );
    let latest = request(
        "read",
        "latest-grant",
        "read-2",
        JsValue::object([("identity", identity.clone()), ("revision", JsValue::Null)]),
    );
    write_fixture(&fixture, &root, &latest, Some(&second_revision));
    let now = invoke(&root, &fixture, &latest);
    assert_eq!(
        field(&now, "state").unwrap(),
        field(&second, "state").unwrap()
    );
    let history = request(
        "history",
        "history-grant",
        "history-1",
        JsValue::object([("identity", identity)]),
    );
    write_fixture(&fixture, &root, &history, Some(&second_revision));
    let all = invoke(&root, &fixture, &history);
    assert_eq!(field(&all, "history").unwrap().as_array().unwrap().len(), 2);
}

#[test]
fn large_valid_reply_is_drained_before_child_exit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let question = "Q".repeat(128 * 1024);
    let begin = request(
        "begin",
        "large-reply-grant",
        "large-reply",
        JsValue::object([
            ("authority", authority()),
            ("transitionId", JsValue::text("begin")),
            (
                "unresolvedQuestions",
                JsValue::Array(vec![JsValue::text(&question)]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let response = invoke(&root, &registry, &begin);
    assert_eq!(status(&response), "applied");
    let questions = field(field(&response, "state").unwrap(), "unresolvedQuestions")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(questions, &[JsValue::text(&question)]);
}

#[test]
fn direct_library_execution_uses_only_the_admitted_parsed_envelope() {
    use review_episode::Application;
    use review_episode::admission::FixtureAdmission;
    use review_episode::protocol::Request;
    use review_episode_store::{EpisodeStore, StoreOptions};

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let begin = request(
        "begin",
        "direct-grant",
        "direct-1",
        JsValue::object([
            ("authority", authority()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let port = FixtureAdmission::load(&registry, &root).unwrap();
    let store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let mut app = Application {
        store,
        port,
        response_limit: 32 * 1024 * 1024,
    };
    let mut changed = begin.clone();
    if let JsValue::Object(envelope) = &mut changed
        && let Some(JsValue::Object(args)) = envelope.get_mut(&"args".into())
    {
        args.insert("transitionId".into(), JsValue::text("invented"));
    }
    let forged =
        Request::parse(review_episode_core::codec::canonical_json(&changed).as_bytes()).unwrap();
    assert!(matches!(
        app.execute(&forged),
        Err(review_episode::AppError::Admission(_))
    ));
    let original =
        Request::parse(review_episode_core::codec::canonical_json(&begin).as_bytes()).unwrap();
    let response = parse_json(&app.execute(&original).unwrap()).unwrap();
    assert_eq!(status(&response), "applied");
    let key = review_episode_core::identity::Authority::parse(authority())
        .unwrap()
        .identity()
        .key()
        .0;
    assert_eq!(app.store.read(&key).unwrap().history.len(), 1);
}

fn raw_request(
    root: &std::path::Path,
    registry: &std::path::Path,
    body: &[u8],
    extra: &[u8],
    options: &[&str],
) -> JsValue {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(BIN)
        .arg("offline")
        .arg("--root")
        .arg(root)
        .arg("--fixture")
        .arg(registry)
        .args(options)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut input = child.stdin.take().unwrap();
        input.write_all(body).unwrap();
        input.write_all(extra).unwrap();
    }
    let output = bounded_output(child);
    assert!(
        output.stdout.len() >= 4,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let len = u32::from_be_bytes(output.stdout[..4].try_into().unwrap()) as usize;
    assert_eq!(len + 4, output.stdout.len());
    parse_json(std::str::from_utf8(&output.stdout[4..]).unwrap()).unwrap()
}
fn framed(body: &str) -> Vec<u8> {
    let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
    bytes.extend(body.as_bytes());
    bytes
}

#[test]
fn framing_limits_and_offline_grant_binding_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let authority = authority();
    let key = review_episode_core::identity::Authority::parse(authority.clone())
        .unwrap()
        .identity()
        .key()
        .0;
    let begin = request(
        "begin",
        "begin-grant",
        "bounded",
        JsValue::object([
            ("authority", authority.clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let canonical = review_episode_core::codec::canonical_json(&begin);
    for raw in [
        {
            let mut b = (canonical.len() as u32 + 1).to_be_bytes().to_vec();
            b.extend(canonical.as_bytes());
            b
        },
        (8 * 1024 * 1024u32 + 1).to_be_bytes().to_vec(),
    ] {
        let response = raw_request(&root, &registry, &raw, &[], &[]);
        assert_eq!(field(&response, "kind").unwrap(), &JsValue::text("Framing"));
    }
    let response = raw_request(&root, &registry, &framed(&canonical), b"X", &[]);
    assert_eq!(field(&response, "kind").unwrap(), &JsValue::text("Framing"));
    let extra = canonical.replacen("\"version\":1", "\"unexpected\":null,\"version\":1", 1);
    let response = raw_request(&root, &registry, &framed(&extra), &[], &[]);
    assert_eq!(
        field(&response, "kind").unwrap(),
        &JsValue::text("InvalidCommand")
    );
    let bounded = raw_request(
        &root,
        &registry,
        &framed(&canonical),
        &[],
        &["--max-response-bytes", "512"],
    );
    assert_eq!(
        field(&bounded, "kind").unwrap(),
        &JsValue::text("ResponseTooLarge")
    );
    let mut changed = begin.clone();
    if let JsValue::Object(map) = &mut changed
        && let JsValue::Object(args) = map.get_mut(&"args".into()).unwrap()
    {
        args.insert("transitionId".into(), JsValue::text("invented"));
    }
    let response = invoke(&root, &registry, &changed);
    assert_eq!(
        field(&response, "kind").unwrap(),
        &JsValue::text("Admission")
    );
    let mut missing_grant = begin.clone();
    if let JsValue::Object(map) = &mut missing_grant {
        map.insert("grantId".into(), JsValue::text("caller-invented-grant"));
    }
    let response = invoke(&root, &registry, &missing_grant);
    assert_eq!(
        field(&response, "kind").unwrap(),
        &JsValue::text("Admission")
    );
    let invalid_root = JsValue::object([
        ("root", JsValue::text("/wrong-offline-root")),
        ("grants", JsValue::Array(vec![])),
    ]);
    std::fs::write(
        &registry,
        review_episode_core::codec::canonical_json(&invalid_root),
    )
    .unwrap();
    let response = invoke(&root, &registry, &begin);
    assert_eq!(
        field(&response, "kind").unwrap(),
        &JsValue::text("Admission")
    );
    let mut store = review_episode_store::EpisodeStore::open(
        &root,
        review_episode_store::StoreOptions::default(),
    )
    .unwrap();
    assert!(store.read(&key).unwrap().current.is_none());
}

#[test]
fn utf16_surrogates_survive_wire_store_restart_and_transition_digest() {
    use review_episode_core::codec::{JsString, canonical_json, digest};
    use review_episode_core::command::{Action, BeginCommand, TransitionCommand};
    use review_episode_core::identity::Authority;
    use review_episode_core::reducer::{begin as reduce_begin, transition as reduce_transition};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let authority_value = authority();
    let authority = Authority::parse(authority_value.clone()).unwrap();
    let identity = authority.identity().value();
    let key = authority.identity().key().0;
    let begin_id = JsString(vec![0xd800]);
    let question = JsValue::String(JsString(vec![0xd801]));
    let expected_begin = reduce_begin(
        None,
        &BeginCommand::new_utf16(authority.clone(), begin_id.clone(), vec![question.clone()])
            .unwrap(),
    )
    .unwrap()
    .into_state();
    let begin_request = request(
        "begin",
        "utf16-begin",
        "utf16-1",
        JsValue::object([
            ("authority", authority_value.clone()),
            ("transitionId", JsValue::String(begin_id.clone())),
            ("unresolvedQuestions", JsValue::Array(vec![question])),
        ]),
    );
    write_fixture(&registry, &root, &begin_request, None);
    let first = invoke(&root, &registry, &begin_request);
    assert_eq!(status(&first), "applied");
    assert_eq!(field(&first, "state").unwrap(), expected_begin.value());
    let first_revision = revision(&first);
    let payload = JsValue::object([
        ("reason", JsValue::String(JsString(vec![0xd800]))),
        ("reconciliationAction", JsValue::text("reconcile")),
    ]);
    let transition_id = JsString(vec![0xd803]);
    let expected = reduce_transition(
        &expected_begin,
        &TransitionCommand::new_utf16(
            authority.clone(),
            expected_begin.revision().clone(),
            transition_id.clone(),
            Action::MarkUncertain,
            payload.clone(),
        )
        .unwrap(),
    )
    .unwrap()
    .into_state();
    let transition_request = request(
        "transition",
        "utf16-transition",
        "utf16-2",
        JsValue::object([
            ("authority", authority_value),
            ("expectedRevision", JsValue::text(&first_revision)),
            ("transitionId", JsValue::String(transition_id.clone())),
            ("action", JsValue::text("mark_uncertain")),
            ("payload", payload.clone()),
        ]),
    );
    write_fixture(&registry, &root, &transition_request, Some(&first_revision));
    let second = invoke(&root, &registry, &transition_request);
    assert_eq!(status(&second), "applied");
    assert_eq!(field(&second, "state").unwrap(), expected.value());
    let current = revision(&second);
    let mut store = review_episode_store::EpisodeStore::open(
        &root,
        review_episode_store::StoreOptions::default(),
    )
    .unwrap();
    let snapshot = store.read(&key).unwrap();
    assert_eq!(snapshot.history.len(), 2);
    assert_eq!(
        snapshot.history[1].state_json,
        canonical_json(expected.value())
    );
    assert!(snapshot.history[1].state_json.contains("\\ud801"));
    drop(store);
    let content = digest(&JsValue::object([
        ("action", JsValue::text("mark_uncertain")),
        ("payload", payload),
    ]));
    let recovery = request(
        "recover",
        "utf16-recover",
        "utf16-3",
        JsValue::object([
            ("identity", identity),
            ("transitionId", JsValue::String(transition_id)),
            ("contentDigest", JsValue::text(&content)),
        ]),
    );
    write_fixture(&registry, &root, &recovery, Some(&current));
    let readback = invoke(&root, &registry, &recovery);
    assert_eq!(status(&readback), "committed");
    assert_eq!(
        field(&readback, "committedRevision").unwrap(),
        &JsValue::text(&current)
    );
}

#[test]
fn replacement_fences_old_writer_replay_but_independent_reader_observes_history() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let old = authority();
    let identity = field(&old, "identity").unwrap().clone();
    let begin = request(
        "begin",
        "grant-begin",
        "b",
        JsValue::object([
            ("authority", old.clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let first = invoke(&root, &registry, &begin);
    let first_revision = revision(&first);
    let uncertain = request(
        "transition",
        "grant-old",
        "u",
        JsValue::object([
            ("authority", old.clone()),
            ("expectedRevision", JsValue::text(&first_revision)),
            ("transitionId", JsValue::text("uncertain")),
            ("action", JsValue::text("mark_uncertain")),
            (
                "payload",
                JsValue::object([
                    ("reason", JsValue::text("lost")),
                    ("reconciliationAction", JsValue::text("replace")),
                ]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &uncertain, Some(&first_revision));
    let uncertain_response = invoke(&root, &registry, &uncertain);
    let uncertain_revision = revision(&uncertain_response);
    let mut successor = old.clone();
    if let JsValue::Object(map) = &mut successor {
        map.insert("grantId".into(), JsValue::text("grant-2"));
        map.insert(
            "predecessorRevision".into(),
            JsValue::text(&uncertain_revision),
        );
        if let JsValue::Object(writer) = map.get_mut(&"writer".into()).unwrap() {
            writer.insert("generation".into(), JsValue::Number(2.0));
            writer.insert(
                "runtimeSession".into(),
                reference("runtime", "session-2", "generation-2", "session-2"),
            );
        }
    }
    let replace = request(
        "transition",
        "grant-successor",
        "r",
        JsValue::object([
            ("authority", successor),
            ("expectedRevision", JsValue::text(&uncertain_revision)),
            ("transitionId", JsValue::text("replace")),
            ("action", JsValue::text("replace_writer")),
            (
                "payload",
                JsValue::object([
                    ("reason", JsValue::text("lost")),
                    ("pendingAction", JsValue::text("reconcile")),
                ]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &replace, Some(&uncertain_revision));
    let replaced = invoke(&root, &registry, &replace);
    assert_eq!(status(&replaced), "applied");
    let replaced_revision = revision(&replaced);
    write_fixture(&registry, &root, &uncertain, Some(&replaced_revision));
    let denied = invoke(&root, &registry, &uncertain);
    assert_eq!(field(&denied, "kind").unwrap(), &JsValue::text("Authority"));
    let read = request(
        "history",
        "read-independent",
        "h",
        JsValue::object([("identity", identity)]),
    );
    write_fixture(&registry, &root, &read, Some(&replaced_revision));
    let history = invoke(&root, &registry, &read);
    assert_eq!(status(&history), "observed");
    assert_eq!(
        field(&history, "history")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn fixture_evidence_statuses_and_result_contract_remain_typed() {
    use review_episode_core::codec::JsString;
    const STATES: &str =
        include_str!("../../../../app-server/tests/fixtures/review-episode-rust/states-v1-v2.json");
    let fixture_states = parse_json(STATES).unwrap();
    let blocked = field(field(&fixture_states, "states").unwrap(), "v2_blocked").unwrap();
    let blocked_json = field(blocked, "canonical")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let blocked_state = parse_json(&blocked_json).unwrap();
    let base_admissions = field(&blocked_state, "evidenceAdmissions")
        .unwrap()
        .as_array()
        .unwrap()
        .to_vec();
    for evidence_status in ["established", "false", "unestablished"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("offline");
        let registry = temp.path().join("registry.json");
        init(&root);
        let authority = authority();
        let identity = field(&authority, "identity").unwrap().clone();
        let begin = request(
            "begin",
            "begin-grant",
            "b",
            JsValue::object([
                ("authority", authority.clone()),
                ("transitionId", JsValue::text("begin")),
                ("unresolvedQuestions", JsValue::Array(vec![])),
            ]),
        );
        write_fixture(&registry, &root, &begin, None);
        let first = invoke(&root, &registry, &begin);
        let first_revision = revision(&first);
        let mut admissions = base_admissions.clone();
        for admission in &mut admissions {
            if let JsValue::Object(map) = admission {
                map.insert(JsString::new("status"), JsValue::text(evidence_status));
            }
        }
        let result = parse_json(ACCEPTABLE).unwrap();
        let invalid_result = JsValue::object([
            ("result", JsValue::Null),
            ("unresolvedQuestions", JsValue::Array(vec![])),
            ("evidenceAdmissions", JsValue::Array(admissions.clone())),
        ]);
        let invalid = request(
            "transition",
            "invalid-grant",
            "bad",
            JsValue::object([
                ("authority", authority.clone()),
                ("expectedRevision", JsValue::text(&first_revision)),
                ("transitionId", JsValue::text("invalid")),
                ("action", JsValue::text("record_result")),
                ("payload", invalid_result),
            ]),
        );
        write_fixture(&registry, &root, &invalid, Some(&first_revision));
        let rejected = invoke(&root, &registry, &invalid);
        assert_eq!(
            field(&rejected, "kind").unwrap(),
            &JsValue::text("ResultContract")
        );
        let payload = JsValue::object([
            ("result", result.clone()),
            ("unresolvedQuestions", JsValue::Array(vec![])),
            ("evidenceAdmissions", JsValue::Array(admissions)),
        ]);
        let command = request(
            "transition",
            "result-grant",
            "result",
            JsValue::object([
                ("authority", authority.clone()),
                ("expectedRevision", JsValue::text(&first_revision)),
                ("transitionId", JsValue::text("result")),
                ("action", JsValue::text("record_result")),
                ("payload", payload),
            ]),
        );
        write_fixture(&registry, &root, &command, Some(&first_revision));
        let second = invoke(&root, &registry, &command);
        assert_eq!(status(&second), "applied", "{second:?}");
        let expected_phase = if evidence_status == "established" {
            "reported"
        } else {
            "evidence_unestablished"
        };
        assert_eq!(
            field(field(&second, "state").unwrap(), "phase").unwrap(),
            &JsValue::text(expected_phase)
        );
        let mut store = review_episode_store::EpisodeStore::open(
            &root,
            review_episode_store::StoreOptions::default(),
        )
        .unwrap();
        assert_eq!(
            store
                .read(
                    &review_episode_core::identity::Identity::parse(&identity)
                        .unwrap()
                        .key()
                        .0
                )
                .unwrap()
                .history
                .len(),
            2
        );
    }
}

#[test]
fn later_result_without_new_evidence_is_blocked_in_offline_vertical() {
    const STATES: &str =
        include_str!("../../../../app-server/tests/fixtures/review-episode-rust/states-v1-v2.json");
    let fixture_states = parse_json(STATES).unwrap();
    let blocked = field(field(&fixture_states, "states").unwrap(), "v2_blocked").unwrap();
    let source = field(blocked, "canonical")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let base = parse_json(&source).unwrap();
    let mut admissions = field(&base, "evidenceAdmissions")
        .unwrap()
        .as_array()
        .unwrap()
        .to_vec();
    for admission in &mut admissions {
        if let JsValue::Object(map) = admission {
            map.insert("status".into(), JsValue::text("established"));
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let authority = authority();
    let subject = field(&authority, "initialSubject").unwrap().clone();
    let begin = request(
        "begin",
        "begin-grant",
        "b",
        JsValue::object([
            ("authority", authority.clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let first = invoke(&root, &registry, &begin);
    let first_revision = revision(&first);
    let result = parse_json(ACCEPTABLE).unwrap();
    let initial = request(
        "transition",
        "first-result",
        "r1",
        JsValue::object([
            ("authority", authority.clone()),
            ("expectedRevision", JsValue::text(&first_revision)),
            ("transitionId", JsValue::text("result-1")),
            ("action", JsValue::text("record_result")),
            (
                "payload",
                JsValue::object([
                    ("result", result.clone()),
                    ("unresolvedQuestions", JsValue::Array(vec![])),
                    ("evidenceAdmissions", JsValue::Array(admissions)),
                ]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &initial, Some(&first_revision));
    let first_result = invoke(&root, &registry, &initial);
    assert_eq!(status(&first_result), "applied");
    let first_result_revision = revision(&first_result);
    let remediation = request(
        "transition",
        "remediation",
        "subject",
        JsValue::object([
            ("authority", authority.clone()),
            ("expectedRevision", JsValue::text(&first_result_revision)),
            ("transitionId", JsValue::text("subject")),
            ("action", JsValue::text("record_remediation_subject")),
            ("payload", JsValue::object([("subject", subject)])),
        ]),
    );
    write_fixture(&registry, &root, &remediation, Some(&first_result_revision));
    let resumed = invoke(&root, &registry, &remediation);
    assert_eq!(status(&resumed), "applied");
    let resumed_revision = revision(&resumed);
    let later = request(
        "transition",
        "later-result",
        "r2",
        JsValue::object([
            ("authority", authority),
            ("expectedRevision", JsValue::text(&resumed_revision)),
            ("transitionId", JsValue::text("result-2")),
            ("action", JsValue::text("record_result")),
            (
                "payload",
                JsValue::object([
                    ("result", result),
                    ("unresolvedQuestions", JsValue::Array(vec![])),
                ]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &later, Some(&resumed_revision));
    let blocked = invoke(&root, &registry, &later);
    assert_eq!(status(&blocked), "applied");
    assert_eq!(
        field(field(&blocked, "state").unwrap(), "phase").unwrap(),
        &JsValue::text("evidence_unestablished")
    );
}

#[test]
fn offline_commands_cover_observation_succession_questions_and_retirement() {
    const STATES: &str =
        include_str!("../../../../app-server/tests/fixtures/review-episode-rust/states-v1-v2.json");
    let fixture_states = parse_json(STATES).unwrap();
    let states = field(&fixture_states, "states").unwrap();
    let source = field(field(states, "v2_blocked").unwrap(), "canonical")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let successor_source = field(field(states, "v2_succeeded").unwrap(), "canonical")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let old_state = parse_json(&source).unwrap();
    let successor_state = parse_json(&successor_source).unwrap();
    let predecessor = field(&old_state, "evidenceAdmissions")
        .unwrap()
        .as_array()
        .unwrap()
        .to_vec();
    let successor = field(&successor_state, "evidenceAdmissions")
        .unwrap()
        .as_array()
        .unwrap()[2..]
        .to_vec();
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let authority = authority();
    let identity = field(&authority, "identity").unwrap().clone();
    let question = JsValue::text("unresolved review question");
    let begin = request(
        "begin",
        "begin-grant",
        "b",
        JsValue::object([
            ("authority", authority.clone()),
            ("transitionId", JsValue::text("begin")),
            (
                "unresolvedQuestions",
                JsValue::Array(vec![question.clone()]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let first = invoke(&root, &registry, &begin);
    let first_revision = revision(&first);
    let resume = request(
        "resumeInitial",
        "resume-grant",
        "resume",
        JsValue::object([("authority", authority.clone())]),
    );
    write_fixture(&registry, &root, &resume, Some(&first_revision));
    assert_eq!(status(&invoke(&root, &registry, &resume)), "observed");
    let result = parse_json(ACCEPTABLE).unwrap();
    let validate = request(
        "validateResult",
        "validate-grant",
        "validate",
        JsValue::object([
            ("authority", authority.clone()),
            ("expectedRevision", JsValue::text(&first_revision)),
            ("result", result.clone()),
        ]),
    );
    write_fixture(&registry, &root, &validate, Some(&first_revision));
    assert_eq!(status(&invoke(&root, &registry, &validate)), "observed");
    let record = request(
        "transition",
        "record-grant",
        "record",
        JsValue::object([
            ("authority", authority.clone()),
            ("expectedRevision", JsValue::text(&first_revision)),
            ("transitionId", JsValue::text("result")),
            ("action", JsValue::text("record_result")),
            (
                "payload",
                JsValue::object([
                    ("result", result),
                    (
                        "unresolvedQuestions",
                        JsValue::Array(vec![question.clone()]),
                    ),
                    ("evidenceAdmissions", JsValue::Array(predecessor.clone())),
                ]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &record, Some(&first_revision));
    let blocked = invoke(&root, &registry, &record);
    assert_eq!(status(&blocked), "applied");
    let blocked_revision = revision(&blocked);
    assert_eq!(
        field(field(&blocked, "state").unwrap(), "phase").unwrap(),
        &JsValue::text("evidence_unestablished")
    );
    let succeed = request(
        "transition",
        "successor-grant",
        "successor",
        JsValue::object([
            ("authority", authority.clone()),
            ("expectedRevision", JsValue::text(&blocked_revision)),
            ("transitionId", JsValue::text("succession")),
            ("action", JsValue::text("succeed_evidence")),
            (
                "payload",
                JsValue::object([
                    ("predecessorAdmissions", JsValue::Array(predecessor)),
                    ("successorAdmissions", JsValue::Array(successor)),
                ]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &succeed, Some(&blocked_revision));
    let advanced = invoke(&root, &registry, &succeed);
    assert_eq!(status(&advanced), "applied", "{advanced:?}");
    let advanced_revision = revision(&advanced);
    assert_eq!(
        field(field(&advanced, "state").unwrap(), "phase").unwrap(),
        &JsValue::text("remediation")
    );
    assert_eq!(
        field(field(&advanced, "state").unwrap(), "unresolvedQuestions").unwrap(),
        &JsValue::Array(vec![question])
    );
    let retire = request(
        "transition",
        "retire-grant",
        "retire",
        JsValue::object([
            ("authority", authority.clone()),
            ("expectedRevision", JsValue::text(&advanced_revision)),
            ("transitionId", JsValue::text("retire")),
            ("action", JsValue::text("retire")),
            (
                "payload",
                JsValue::object([
                    ("outcome", JsValue::text("closed")),
                    ("reason", JsValue::text("review complete")),
                    (
                        "protectedReferences",
                        JsValue::Array(vec![field(&authority, "initialSubject").unwrap().clone()]),
                    ),
                ]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &retire, Some(&advanced_revision));
    let retired = invoke(&root, &registry, &retire);
    assert_eq!(status(&retired), "applied", "{retired:?}");
    let current = revision(&retired);
    let history = request(
        "history",
        "history-grant",
        "history",
        JsValue::object([("identity", identity)]),
    );
    write_fixture(&registry, &root, &history, Some(&current));
    assert_eq!(
        field(&invoke(&root, &registry, &history), "history")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        4
    );
}

#[cfg(not(feature = "test-faults"))]
#[test]
fn default_executable_rejects_fault_environment() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    init(&root);
    let output = std::process::Command::new(BIN)
        .arg("offline")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg(temp.path().join("registry.json"))
        .env("REVIEW_EPISODE_FAULT_CUT", "after_commit")
        .env("REVIEW_EPISODE_FAULT_DIR", temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn broken_reply_pipe_does_not_erase_committed_begin() {
    use review_episode_core::codec::{canonical_json, digest};
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let authority = authority();
    let identity = field(&authority, "identity").unwrap().clone();
    let key = review_episode_core::identity::Identity::parse(&identity)
        .unwrap()
        .key()
        .0;
    let begin = request(
        "begin",
        "begin-grant",
        "broken-pipe",
        JsValue::object([
            ("authority", authority),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let mut child = Command::new(BIN)
        .arg("offline")
        .arg("--root")
        .arg(&root)
        .arg("--fixture")
        .arg(&registry)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let body = canonical_json(&begin);
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(&(body.len() as u32).to_be_bytes()).unwrap();
        stdin.write_all(body.as_bytes()).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("broken-pipe child hung");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!child.wait().unwrap().success());
    let mut store = review_episode_store::EpisodeStore::open(
        &root,
        review_episode_store::StoreOptions::default(),
    )
    .unwrap();
    let snapshot = store.read(&key).unwrap();
    assert_eq!(snapshot.history.len(), 1);
    let committed = snapshot.current.unwrap().revision().0.clone();
    drop(store);
    let content = digest(&JsValue::object([
        ("action", JsValue::text("begin")),
        ("unresolvedQuestions", JsValue::Array(vec![])),
    ]));
    let recovery = request(
        "recover",
        "read-grant",
        "recover",
        JsValue::object([
            ("identity", identity),
            ("transitionId", JsValue::text("begin")),
            ("contentDigest", JsValue::text(&content)),
        ]),
    );
    write_fixture(&registry, &root, &recovery, Some(&committed));
    assert_eq!(status(&invoke(&root, &registry, &recovery)), "committed");
}

#[test]
fn current_writer_replay_after_further_transition_returns_latest_without_append() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let authority = authority();
    let key = review_episode_core::identity::Authority::parse(authority.clone())
        .unwrap()
        .identity()
        .key()
        .0;
    let begin = request(
        "begin",
        "begin-grant",
        "begin",
        JsValue::object([
            ("authority", authority.clone()),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &begin, None);
    let first = invoke(&root, &registry, &begin);
    let first_revision = revision(&first);
    let make = |grant: &str, id: &str, expected: &str| {
        request(
            "transition",
            grant,
            id,
            JsValue::object([
                ("authority", authority.clone()),
                ("expectedRevision", JsValue::text(expected)),
                ("transitionId", JsValue::text(id)),
                ("action", JsValue::text("mark_uncertain")),
                (
                    "payload",
                    JsValue::object([
                        ("reason", JsValue::text(id)),
                        ("reconciliationAction", JsValue::text("reconcile")),
                    ]),
                ),
            ]),
        )
    };
    let one = make("grant-one", "one", &first_revision);
    write_fixture(&registry, &root, &one, Some(&first_revision));
    let one_result = invoke(&root, &registry, &one);
    let one_revision = revision(&one_result);
    let two = make("grant-two", "two", &one_revision);
    write_fixture(&registry, &root, &two, Some(&one_revision));
    let latest = invoke(&root, &registry, &two);
    let latest_revision = revision(&latest);
    write_fixture(&registry, &root, &one, Some(&latest_revision));
    let replay = invoke(&root, &registry, &one);
    assert_eq!(status(&replay), "replay");
    assert_eq!(
        field(&replay, "state").unwrap(),
        field(&latest, "state").unwrap()
    );
    let mut store = review_episode_store::EpisodeStore::open(
        &root,
        review_episode_store::StoreOptions::default(),
    )
    .unwrap();
    assert_eq!(store.read(&key).unwrap().history.len(), 3);
}

#[test]
fn safe_number_boundary_and_excess_nesting_fail_without_normalization() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let registry = temp.path().join("registry.json");
    init(&root);
    let mut at_limit = authority();
    if let JsValue::Object(map) = &mut at_limit
        && let JsValue::Object(identity) = map.get_mut(&"identity".into()).unwrap()
    {
        identity.insert(
            "sliceNumber".into(),
            JsValue::Number(9_007_199_254_740_991.0),
        );
    }
    let valid = request(
        "begin",
        "max-safe",
        "max-safe",
        JsValue::object([
            ("authority", at_limit.clone()),
            ("transitionId", JsValue::text("begin")),
            (
                "unresolvedQuestions",
                JsValue::Array(vec![JsValue::text("😀")]),
            ),
        ]),
    );
    write_fixture(&registry, &root, &valid, None);
    let accepted = invoke(&root, &registry, &valid);
    assert_eq!(status(&accepted), "applied");
    let mut above = at_limit;
    if let JsValue::Object(map) = &mut above
        && let JsValue::Object(identity) = map.get_mut(&"identity".into()).unwrap()
    {
        identity.insert(
            "sliceNumber".into(),
            JsValue::Number(9_007_199_254_740_992.0),
        );
    }
    let invalid = request(
        "begin",
        "too-large",
        "too-large",
        JsValue::object([
            ("authority", above),
            ("transitionId", JsValue::text("begin")),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    write_fixture(&registry, &root, &invalid, None);
    let denied = invoke(&root, &registry, &invalid);
    assert_eq!(field(&denied, "kind").unwrap(), &JsValue::text("Authority"));
    let deeply_nested = format!("{}null{}", "[".repeat(258), "]".repeat(258));
    let response = raw_request(&root, &registry, &framed(&deeply_nested), &[], &[]);
    assert_eq!(
        field(&response, "kind").unwrap(),
        &JsValue::text("InvalidCommand")
    );
    let valid_key = review_episode_core::identity::Authority::parse(
        field(&valid, "args")
            .unwrap()
            .get("authority")
            .unwrap()
            .clone(),
    )
    .unwrap()
    .identity()
    .key()
    .0;
    let mut store = review_episode_store::EpisodeStore::open(
        &root,
        review_episode_store::StoreOptions::default(),
    )
    .unwrap();
    assert_eq!(store.read(&valid_key).unwrap().history.len(), 1);
}
