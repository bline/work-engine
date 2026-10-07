use review_episode_core::ErrorKind;
use review_episode_core::codec::{JsValue, digest, field, parse_json};
use review_episode_core::command::{Action, BeginCommand, TransitionCommand};
use review_episode_core::identity::{Authority, Identity, Revision};
use review_episode_core::reducer::{begin, resume_initial, transition, validate_result};
use review_episode_core::state::{Phase, ReviewEpisodeState, Status};
use sha2::{Digest, Sha256};

const COMMANDS: &str =
    include_str!("../../../../app-server/tests/fixtures/review-episode-rust/commands.json");
const STATES: &str =
    include_str!("../../../../app-server/tests/fixtures/review-episode-rust/states-v1-v2.json");
const REMEDIATION: &str = include_str!(
    "../../../../app-server/tests/fixtures/implementation-review/remediation-required.json"
);
const ACCEPTABLE: &str = include_str!(
    "../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json"
);

fn raw_sha(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn reference(owner: &str, name: &str, revision: &str, raw: &str) -> JsValue {
    JsValue::object([
        ("owner", JsValue::text(owner)),
        ("reference", JsValue::text(name)),
        ("revision", JsValue::text(revision)),
        ("sha256", JsValue::text(&raw_sha(raw))),
        ("freshness", JsValue::text("exact_revision")),
    ])
}

fn authority(
    subject: &JsValue,
    identity: &JsValue,
    generation: u64,
    predecessor: Option<&Revision>,
) -> Authority {
    let mut initial_subject = reference(
        "checkpoint",
        "candidate",
        "candidate-commit",
        "candidate-commit",
    );
    if let JsValue::Object(map) = &mut initial_subject {
        map.insert("revision".into(), field(subject, "commit").unwrap().clone());
        map.insert("sha256".into(), JsValue::text(&digest(subject)));
    }
    let source = reference("human", "accepted-plan", "plan-v1", "accepted-plan");
    let session = format!("session-{generation}");
    let gen_revision = format!("generation-{generation}");
    Authority::parse(JsValue::object([
        ("schemaVersion", JsValue::Number(1.0)),
        ("grantId", JsValue::text(&format!("grant-{generation}"))),
        ("identity", identity.clone()),
        ("source", source),
        (
            "writer",
            JsValue::object([
                ("actorId", JsValue::text("reviewer")),
                ("provider", JsValue::text("fixture")),
                ("generation", JsValue::Number(generation as f64)),
                (
                    "runtimeSession",
                    reference("runtime", &session, &gen_revision, &session),
                ),
            ]),
        ),
        (
            "readers",
            JsValue::Array(vec![
                JsValue::text("reviewer"),
                JsValue::text("builder"),
                JsValue::text("supervisor"),
            ]),
        ),
        ("initialSubject", initial_subject),
        (
            "predecessorRevision",
            predecessor.map_or(JsValue::Null, Revision::value),
        ),
    ]))
    .unwrap()
}

fn apply(
    current: &ReviewEpisodeState,
    grant: &Authority,
    id: &str,
    action: Action,
    payload: JsValue,
) -> ReviewEpisodeState {
    transition(
        current,
        &TransitionCommand::new(
            grant.clone(),
            current.revision().clone(),
            id,
            action,
            payload,
        )
        .unwrap(),
    )
    .unwrap()
    .into_state()
}

#[test]
fn frozen_begin_result_and_replacement_revisions_match_legacy() {
    let fixture = parse_json(COMMANDS).unwrap();
    let identity = field(&fixture, "identity").unwrap();
    let result = parse_json(REMEDIATION).unwrap();
    let grant = authority(field(&result, "subject").unwrap(), identity, 1, None);
    let first = begin(
        None,
        &BeginCommand::new(grant.clone(), "begin", vec![]).unwrap(),
    )
    .unwrap()
    .into_state();
    assert_eq!(
        first.revision().value(),
        *field(field(&fixture, "revisions").unwrap(), "begin").unwrap()
    );
    assert_eq!(
        resume_initial(&first, &grant).unwrap().revision(),
        first.revision()
    );
    let outcome = transition(
        &first,
        &TransitionCommand::new(
            grant.clone(),
            first.revision().clone(),
            "initial-result",
            Action::RecordResult,
            JsValue::object([
                ("result", result.clone()),
                ("unresolvedQuestions", JsValue::Array(vec![])),
            ]),
        )
        .unwrap(),
    )
    .unwrap();
    let result_state = outcome.into_state();
    assert_eq!(
        result_state.revision().value(),
        *field(field(&fixture, "revisions").unwrap(), "initialResult").unwrap()
    );
    assert_eq!(result_state.phase(), Phase::Remediation);
    let uncertain = apply(
        &result_state,
        &grant,
        "uncertain",
        Action::MarkUncertain,
        JsValue::object([
            ("reason", JsValue::text("session lost")),
            ("reconciliationAction", JsValue::text("replace")),
        ]),
    );
    let successor = authority(
        field(&result, "subject").unwrap(),
        identity,
        2,
        Some(uncertain.revision()),
    );
    let replaced = apply(
        &uncertain,
        &successor,
        "replace",
        Action::ReplaceWriter,
        JsValue::object([
            ("reason", JsValue::text("session unavailable")),
            ("pendingAction", JsValue::text("reconcile exact state")),
        ]),
    );
    assert_eq!(
        replaced.revision().value(),
        *field(field(&fixture, "revisions").unwrap(), "replaced").unwrap()
    );
    assert_eq!(replaced.writer().generation, 2);
    let revisions: Vec<JsValue> = [first, result_state, uncertain, replaced.clone()]
        .into_iter()
        .map(|state| state.revision().value())
        .collect();
    assert_eq!(
        JsValue::Array(revisions),
        *field(&fixture, "historyRevisions").unwrap()
    );
    let retirement = apply(
        &replaced,
        &successor,
        "retire",
        Action::Retire,
        JsValue::object([
            ("outcome", JsValue::text("closed")),
            ("reason", JsValue::text("review complete")),
            (
                "protectedReferences",
                JsValue::Array(vec![grant.initial_subject().clone()]),
            ),
        ]),
    );
    assert_eq!(retirement.status(), Status::Retired);
    assert!(
        transition(
            &retirement,
            &TransitionCommand::new(
                successor,
                retirement.revision().clone(),
                "late",
                Action::MarkUncertain,
                JsValue::object([
                    ("reason", JsValue::text("late")),
                    ("reconciliationAction", JsValue::text("stop"))
                ])
            )
            .unwrap()
        )
        .is_err()
    );
}

#[test]
fn current_writer_replay_is_idempotent_but_replaced_writer_replay_is_rejected() {
    let fixture = parse_json(COMMANDS).unwrap();
    let identity = field(&fixture, "identity").unwrap();
    let result = parse_json(ACCEPTABLE).unwrap();
    let grant = authority(field(&result, "subject").unwrap(), identity, 1, None);
    let first = begin(
        None,
        &BeginCommand::new(grant.clone(), "begin", vec![]).unwrap(),
    )
    .unwrap()
    .into_state();
    let payload = JsValue::object([
        ("result", result),
        ("unresolvedQuestions", JsValue::Array(vec![])),
    ]);
    let command = TransitionCommand::new(
        grant.clone(),
        first.revision().clone(),
        "result",
        Action::RecordResult,
        payload.clone(),
    )
    .unwrap();
    let reported = transition(&first, &command).unwrap().into_state();
    assert!(transition(&reported, &command).unwrap().is_replay());
    assert_eq!(
        begin(
            Some(&reported),
            &BeginCommand::new(grant.clone(), "begin", vec![]).unwrap()
        )
        .unwrap()
        .state()
        .revision(),
        reported.revision()
    );
    let uncertain = apply(
        &reported,
        &grant,
        "uncertain",
        Action::MarkUncertain,
        JsValue::object([
            ("reason", JsValue::text("lost")),
            ("reconciliationAction", JsValue::text("replace")),
        ]),
    );
    let successor = authority(
        field(&payload, "result").unwrap().get("subject").unwrap(),
        identity,
        2,
        Some(uncertain.revision()),
    );
    let replaced = apply(
        &uncertain,
        &successor,
        "replace",
        Action::ReplaceWriter,
        JsValue::object([
            ("reason", JsValue::text("lost")),
            ("pendingAction", JsValue::text("reconcile")),
        ]),
    );
    assert!(transition(&replaced, &command).is_err());
    assert!(
        begin(
            Some(&replaced),
            &BeginCommand::new(grant, "begin", vec![]).unwrap()
        )
        .is_err()
    );
    assert!(
        transition(
            &replaced,
            &TransitionCommand::new(
                successor,
                replaced.revision().clone(),
                "conflict",
                Action::MarkUncertain,
                JsValue::object([
                    ("reason", JsValue::text("x")),
                    ("reconciliationAction", JsValue::text("y"))
                ])
            )
            .unwrap()
        )
        .is_ok()
    );
}

#[test]
fn evidence_succession_retains_questions_and_old_admissions_cannot_establish_new_result() {
    let fixture = parse_json(STATES).unwrap();
    let identity = field(&fixture, "identity").unwrap();
    let accepted = parse_json(ACCEPTABLE).unwrap();
    let grant = authority(field(&accepted, "subject").unwrap(), identity, 1, None);
    let states = field(&fixture, "states").unwrap();
    let blocked_source = field(field(states, "v2_blocked").unwrap(), "canonical")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let blocked = ReviewEpisodeState::from_stored_json(&blocked_source).unwrap();
    let all = field(
        ReviewEpisodeState::from_stored_json(
            &field(field(states, "v2_succeeded").unwrap(), "canonical")
                .unwrap()
                .as_text()
                .unwrap()
                .to_string_checked()
                .unwrap(),
        )
        .unwrap()
        .value(),
        "evidenceAdmissions",
    )
    .unwrap()
    .as_array()
    .unwrap()
    .to_vec();
    let old = blocked
        .get("evidenceAdmissions")
        .as_array()
        .unwrap()
        .to_vec();
    let successor = all[2..].to_vec();
    let succeeded = apply(
        &blocked,
        &grant,
        "succession",
        Action::SucceedEvidence,
        JsValue::object([
            ("predecessorAdmissions", JsValue::Array(old)),
            ("successorAdmissions", JsValue::Array(successor)),
        ]),
    );
    assert_eq!(succeeded.phase(), Phase::Remediation); // selected R0 delta
    assert_eq!(
        succeeded.get("pendingAction"),
        &JsValue::text("await_remediation")
    );
    assert_eq!(
        succeeded.get("unresolvedQuestions"),
        blocked.get("unresolvedQuestions")
    );
    assert_eq!(
        digest(succeeded.get("currentResult")),
        digest(blocked.get("currentResult"))
    );
    let new_subject = JsValue::object([
        ("commit", JsValue::text("candidate-2")),
        ("tree", JsValue::text("tree-2")),
        ("patchIdentity", JsValue::text("patch-2")),
    ]);
    let mut subject_reference =
        reference("checkpoint", "candidate-2", "candidate-2", "candidate-2");
    if let JsValue::Object(map) = &mut subject_reference {
        map.insert("sha256".into(), JsValue::text(&digest(&new_subject)));
    }
    let re_evaluation = apply(
        &succeeded,
        &grant,
        "new-subject",
        Action::RecordRemediationSubject,
        JsValue::object([("subject", subject_reference)]),
    );
    let mut new_result = accepted;
    if let JsValue::Object(map) = &mut new_result {
        map.insert("subject".into(), new_subject);
    }
    let blocked_again = apply(
        &re_evaluation,
        &grant,
        "new-result",
        Action::RecordResult,
        JsValue::object([
            ("result", new_result),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    assert_eq!(blocked_again.phase(), Phase::EvidenceUnestablished);
    assert_eq!(
        blocked_again.get("evidenceAdmissions"),
        succeeded.get("evidenceAdmissions")
    );
    assert!(
        validate_result(
            &blocked_again,
            &grant,
            blocked_again.revision(),
            parse_json(ACCEPTABLE).unwrap()
        )
        .is_err()
    );
}

#[test]
fn null_admissions_follow_js_omission_but_do_not_reuse_stale_v2_claims() {
    let fixture = parse_json(STATES).unwrap();
    let identity = field(&fixture, "identity").unwrap();
    let accepted = parse_json(ACCEPTABLE).unwrap();
    let grant = authority(field(&accepted, "subject").unwrap(), identity, 1, None);
    let initial = begin(
        None,
        &BeginCommand::new(grant.clone(), "begin", vec![]).unwrap(),
    )
    .unwrap()
    .into_state();
    let plain = apply(
        &initial,
        &grant,
        "result",
        Action::RecordResult,
        JsValue::object([
            ("result", accepted.clone()),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    let explicit_null = apply(
        &initial,
        &grant,
        "result",
        Action::RecordResult,
        JsValue::object([
            ("result", accepted.clone()),
            ("unresolvedQuestions", JsValue::Array(vec![])),
            ("evidenceAdmissions", JsValue::Null),
        ]),
    );
    assert_eq!(plain.schema_version(), 1);
    assert_eq!(explicit_null.schema_version(), 1);
    assert_eq!(plain.phase(), explicit_null.phase());
    assert_eq!(
        plain.get("currentResult"),
        explicit_null.get("currentResult")
    );
    assert_eq!(
        plain.get("pendingAction"),
        explicit_null.get("pendingAction")
    );
    assert!(plain.value().get("evidenceAdmissions").is_none());
    assert!(explicit_null.value().get("evidenceAdmissions").is_none());
    assert_ne!(
        plain.get("handledTransitions").get("result"),
        explicit_null.get("handledTransitions").get("result")
    );
    assert_ne!(plain.revision(), explicit_null.revision()); // same ID, different raw payload

    let states = field(&fixture, "states").unwrap();
    let reported_json = field(field(states, "v2_succeeded").unwrap(), "canonical")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let reported = ReviewEpisodeState::from_stored_json(&reported_json).unwrap();
    let subject = JsValue::object([
        ("commit", JsValue::text("candidate-2")),
        ("tree", JsValue::text("tree-2")),
        ("patchIdentity", JsValue::text("patch-2")),
    ]);
    let mut subject_ref = reference("checkpoint", "candidate-2", "candidate-2", "candidate-2");
    if let JsValue::Object(map) = &mut subject_ref {
        map.insert("sha256".into(), JsValue::text(&digest(&subject)));
    }
    let reevaluation = apply(
        &reported,
        &grant,
        "new-subject",
        Action::RecordRemediationSubject,
        JsValue::object([("subject", subject_ref)]),
    );
    let mut new_result = accepted;
    if let JsValue::Object(map) = &mut new_result {
        map.insert("subject".into(), subject);
    }
    let blocked = apply(
        &reevaluation,
        &grant,
        "new-result-null",
        Action::RecordResult,
        JsValue::object([
            ("result", new_result),
            ("unresolvedQuestions", JsValue::Array(vec![])),
            ("evidenceAdmissions", JsValue::Null),
        ]),
    );
    assert_eq!(blocked.phase(), Phase::EvidenceUnestablished);
    assert_eq!(
        blocked.get("evidenceAdmissions"),
        reported.get("evidenceAdmissions")
    );
}

#[test]
fn error_kinds_separate_result_contract_authority_cas_and_store_integrity() {
    let fixture = parse_json(COMMANDS).unwrap();
    let identity = field(&fixture, "identity").unwrap();
    let accepted = parse_json(ACCEPTABLE).unwrap();
    let grant = authority(field(&accepted, "subject").unwrap(), identity, 1, None);
    let first = begin(
        None,
        &BeginCommand::new(grant.clone(), "begin", vec![]).unwrap(),
    )
    .unwrap()
    .into_state();
    let wrong_result = JsValue::object([("schemaVersion", JsValue::Number(1.0))]);
    assert_eq!(
        validate_result(&first, &grant, first.revision(), wrong_result)
            .unwrap_err()
            .kind,
        ErrorKind::ResultContract
    );
    let wrong_authority = authority(
        field(&accepted, "subject").unwrap(),
        identity,
        2,
        Some(first.revision()),
    );
    assert_eq!(
        resume_initial(&first, &wrong_authority).unwrap_err().kind,
        ErrorKind::Authority
    );
    assert_eq!(
        validate_result(&first, &grant, &Revision("0".repeat(64)), accepted)
            .unwrap_err()
            .kind,
        ErrorKind::RevisionConflict
    );
    let mut malformed = grant.value().clone();
    if let JsValue::Object(map) = &mut malformed {
        map.insert("grantId".into(), JsValue::Null);
    }
    assert_eq!(
        Authority::parse(malformed).unwrap_err().kind,
        ErrorKind::Authority
    );
    let stored = review_episode_core::codec::canonical_json(first.value());
    assert_eq!(
        ReviewEpisodeState::from_stored_json(&(stored + "\n"))
            .unwrap_err()
            .kind,
        ErrorKind::StateIntegrity
    );
    assert_eq!(
        Action::parse("unknown").unwrap_err().kind,
        ErrorKind::InvalidCommand
    );
}

#[test]
fn remediation_preserves_finding_attribution_and_valid_status_evolution() {
    let fixture = parse_json(COMMANDS).unwrap();
    let identity = field(&fixture, "identity").unwrap();
    let first_result = parse_json(REMEDIATION).unwrap();
    let grant = authority(field(&first_result, "subject").unwrap(), identity, 1, None);
    let first = begin(
        None,
        &BeginCommand::new(grant.clone(), "begin", vec![]).unwrap(),
    )
    .unwrap()
    .into_state();
    let remediation = apply(
        &first,
        &grant,
        "first-result",
        Action::RecordResult,
        JsValue::object([
            ("result", first_result.clone()),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    assert_eq!(remediation.phase(), Phase::Remediation);
    let new_subject = JsValue::object([
        ("commit", JsValue::text("candidate-2")),
        ("tree", JsValue::text("tree-2")),
        ("patchIdentity", JsValue::text("patch-2")),
    ]);
    let mut subject_ref = reference("checkpoint", "candidate-2", "candidate-2", "candidate-2");
    if let JsValue::Object(map) = &mut subject_ref {
        map.insert("sha256".into(), JsValue::text(&digest(&new_subject)));
    }
    let reevaluation = apply(
        &remediation,
        &grant,
        "new-subject",
        Action::RecordRemediationSubject,
        JsValue::object([("subject", subject_ref)]),
    );
    assert_eq!(reevaluation.phase(), Phase::ReEvaluation);
    let mut changed = first_result.clone();
    if let JsValue::Object(map) = &mut changed {
        map.insert("subject".into(), new_subject.clone());
        let findings = map.get_mut(&"findings".into()).unwrap();
        if let JsValue::Array(items) = findings
            && let JsValue::Object(finding) = &mut items[0]
        {
            finding.insert("title".into(), JsValue::text("rewritten attribution"));
        }
    }
    let changed_command = TransitionCommand::new(
        grant.clone(),
        reevaluation.revision().clone(),
        "changed-finding",
        Action::RecordResult,
        JsValue::object([
            ("result", changed),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    )
    .unwrap();
    assert_eq!(
        transition(&reevaluation, &changed_command)
            .unwrap_err()
            .kind,
        ErrorKind::ResultContract
    );
    let mut resolved = first_result;
    if let JsValue::Object(map) = &mut resolved {
        map.insert("subject".into(), new_subject);
        map.insert("verdict".into(), JsValue::text("acceptable_as_is"));
        let evidence = field(
            &map.get(&"findings".into()).unwrap().as_array().unwrap()[0],
            "evidence",
        )
        .unwrap()
        .clone();
        map.insert("decisiveEvidence".into(), evidence.clone());
        if let JsValue::Array(items) = map.get_mut(&"findings".into()).unwrap()
            && let JsValue::Object(finding) = &mut items[0]
        {
            finding.insert("status".into(), JsValue::text("verified_resolved"));
            finding.insert("remediationEvidence".into(), evidence);
        }
    }
    let reported = apply(
        &reevaluation,
        &grant,
        "resolved",
        Action::RecordResult,
        JsValue::object([
            ("result", resolved),
            ("unresolvedQuestions", JsValue::Array(vec![])),
        ]),
    );
    assert_eq!(reported.phase(), Phase::Reported);
    assert_eq!(
        field(
            &remediation
                .get("currentResult")
                .get("findings")
                .unwrap()
                .as_array()
                .unwrap()[0],
            "status"
        )
        .unwrap(),
        &JsValue::text("open")
    );
    assert_eq!(
        field(
            &reported
                .get("currentResult")
                .get("findings")
                .unwrap()
                .as_array()
                .unwrap()[0],
            "status"
        )
        .unwrap(),
        &JsValue::text("verified_resolved")
    );
    assert!(
        transition(
            &reported,
            &TransitionCommand::new(
                grant,
                reported.revision().clone(),
                "late-result",
                Action::RecordResult,
                JsValue::object([
                    ("result", parse_json(ACCEPTABLE).unwrap()),
                    ("unresolvedQuestions", JsValue::Array(vec![]))
                ])
            )
            .unwrap()
        )
        .is_err()
    );
}

#[test]
fn generated_action_sequences_keep_replay_cas_phase_and_retirement_fences() {
    let accepted = parse_json(ACCEPTABLE).unwrap();
    let remediation_result = parse_json(REMEDIATION).unwrap();
    for ordinal in 1..=24 {
        let mut identity =
            Identity::parse(field(&parse_json(COMMANDS).unwrap(), "identity").unwrap())
                .unwrap()
                .value();
        if let JsValue::Object(map) = &mut identity {
            map.insert(
                "reviewEpisodeId".into(),
                JsValue::text(&format!("generated-{ordinal}")),
            );
        }
        let grant = authority(field(&accepted, "subject").unwrap(), &identity, 1, None);
        let first = begin(
            None,
            &BeginCommand::new(grant.clone(), "begin", vec![]).unwrap(),
        )
        .unwrap()
        .into_state();
        let (current, active_grant) = match ordinal % 3 {
            0 => {
                let uncertain = apply(
                    &first,
                    &grant,
                    "uncertain",
                    Action::MarkUncertain,
                    JsValue::object([
                        ("reason", JsValue::text("lost")),
                        ("reconciliationAction", JsValue::text("replace")),
                    ]),
                );
                let successor = authority(
                    field(&accepted, "subject").unwrap(),
                    &identity,
                    2,
                    Some(uncertain.revision()),
                );
                let replaced = apply(
                    &uncertain,
                    &successor,
                    "replace",
                    Action::ReplaceWriter,
                    JsValue::object([
                        ("reason", JsValue::text("session lost")),
                        ("pendingAction", JsValue::text("resume initial")),
                    ]),
                );
                assert_eq!(replaced.writer().generation, 2);
                assert!(resume_initial(&replaced, &grant).is_err());
                let reported = apply(
                    &replaced,
                    &successor,
                    "result",
                    Action::RecordResult,
                    JsValue::object([
                        ("result", accepted.clone()),
                        ("unresolvedQuestions", JsValue::Array(vec![])),
                    ]),
                );
                (reported, successor)
            }
            1 => {
                let payload = JsValue::object([
                    ("result", accepted.clone()),
                    ("unresolvedQuestions", JsValue::Array(vec![])),
                ]);
                let command = TransitionCommand::new(
                    grant.clone(),
                    first.revision().clone(),
                    "result",
                    Action::RecordResult,
                    payload,
                )
                .unwrap();
                let reported = transition(&first, &command).unwrap().into_state();
                assert!(transition(&reported, &command).unwrap().is_replay());
                assert_eq!(
                    transition(
                        &reported,
                        &TransitionCommand::new(
                            grant.clone(),
                            first.revision().clone(),
                            "stale",
                            Action::MarkUncertain,
                            JsValue::object([
                                ("reason", JsValue::text("stale")),
                                ("reconciliationAction", JsValue::text("read"))
                            ])
                        )
                        .unwrap()
                    )
                    .unwrap_err()
                    .kind,
                    ErrorKind::RevisionConflict
                );
                (reported, grant.clone())
            }
            _ => {
                let remediation = apply(
                    &first,
                    &grant,
                    "result",
                    Action::RecordResult,
                    JsValue::object([
                        ("result", remediation_result.clone()),
                        ("unresolvedQuestions", JsValue::Array(vec![])),
                    ]),
                );
                assert_eq!(remediation.phase(), Phase::Remediation);
                let new_subject = JsValue::object([
                    ("commit", JsValue::text("candidate-2")),
                    ("tree", JsValue::text("tree-2")),
                    ("patchIdentity", JsValue::text("patch-2")),
                ]);
                let mut subject_ref =
                    reference("checkpoint", "candidate-2", "candidate-2", "candidate-2");
                if let JsValue::Object(map) = &mut subject_ref {
                    map.insert("sha256".into(), JsValue::text(&digest(&new_subject)));
                }
                let reevaluation = apply(
                    &remediation,
                    &grant,
                    "subject",
                    Action::RecordRemediationSubject,
                    JsValue::object([("subject", subject_ref)]),
                );
                assert_eq!(reevaluation.phase(), Phase::ReEvaluation);
                let mut next = remediation_result.clone();
                if let JsValue::Object(map) = &mut next {
                    map.insert("subject".into(), new_subject);
                }
                let second = apply(
                    &reevaluation,
                    &grant,
                    "result-2",
                    Action::RecordResult,
                    JsValue::object([
                        ("result", next),
                        ("unresolvedQuestions", JsValue::Array(vec![])),
                    ]),
                );
                assert_eq!(second.phase(), Phase::Remediation);
                (second, grant.clone())
            }
        };
        let retired = apply(
            &current,
            &active_grant,
            "retire",
            Action::Retire,
            JsValue::object([
                ("outcome", JsValue::text("closed")),
                ("reason", JsValue::text("done")),
                (
                    "protectedReferences",
                    JsValue::Array(vec![active_grant.initial_subject().clone()]),
                ),
            ]),
        );
        assert!(
            transition(
                &retired,
                &TransitionCommand::new(
                    active_grant,
                    retired.revision().clone(),
                    "after-retire",
                    Action::MarkUncertain,
                    JsValue::object([
                        ("reason", JsValue::text("late")),
                        ("reconciliationAction", JsValue::text("stop")),
                    ])
                )
                .unwrap()
            )
            .is_err()
        );
    }
}
