//! The implementation-review v1 cross-field contract after the owner's rule
//! change of 2026-11-13. `acceptable_as_is` needs decisive evidence. It may
//! carry limitations, and it may carry pending findings (`open`,
//! `remediation_presented`, `unresolved`) of severity `info` only.
//! `remediation_required` and `incomplete` are unchanged.

use review_episode_core::ErrorKind;
use review_episode_core::codec::{JsValue, digest, field};
use review_episode_core::command::{Action, BeginCommand, TransitionCommand};
use review_episode_core::identity::Authority;
use review_episode_core::implementation_review_v1::{ReviewResult, Verdict};
use review_episode_core::reducer::{begin, transition};
use review_episode_core::state::{Phase, ReviewEpisodeState};
use sha2::{Digest, Sha256};

const PENDING: [&str; 3] = ["open", "remediation_presented", "unresolved"];
const SETTLED: [&str; 2] = ["verified_resolved", "withdrawn"];
const ABOVE_INFO: [&str; 4] = ["low", "medium", "high", "blocker"];
const NO_DECISIVE: &str = "acceptable_as_is requires decisive evidence";
const ABOVE_INFO_PENDING: &str =
    "acceptable_as_is permits an unresolved finding only of severity info";

fn subject() -> JsValue {
    JsValue::object([
        ("commit", JsValue::text("candidate-commit")),
        ("tree", JsValue::text("candidate-tree")),
        ("patchIdentity", JsValue::text("candidate-patch")),
    ])
}

fn evidence(path: &str) -> JsValue {
    JsValue::object([
        ("path", JsValue::text(path)),
        ("startLine", JsValue::Number(1.0)),
        ("endLine", JsValue::Number(3.0)),
        ("sha256", JsValue::text(&"a".repeat(64))),
    ])
}

fn finding(id: &str, severity: &str, status: &str) -> JsValue {
    let remediation = if matches!(status, "remediation_presented" | "verified_resolved") {
        vec![evidence("src/fix.rs")]
    } else {
        vec![]
    };
    JsValue::object([
        ("id", JsValue::text(id)),
        ("severity", JsValue::text(severity)),
        ("title", JsValue::text("synthetic finding")),
        ("evidence", JsValue::Array(vec![evidence("src/lib.rs")])),
        ("observed", JsValue::text("synthetic observation")),
        (
            "violatedExpectation",
            JsValue::text("synthetic expectation"),
        ),
        ("consequence", JsValue::text("synthetic consequence")),
        ("basis", JsValue::text("inferred")),
        ("confidence", JsValue::text("medium")),
        (
            "recommendedRemediation",
            JsValue::text("synthetic remediation"),
        ),
        ("status", JsValue::text(status)),
        ("remediationEvidence", JsValue::Array(remediation)),
    ])
}

fn result(verdict: &str, findings: Vec<JsValue>, decisive: usize, limitations: usize) -> JsValue {
    JsValue::object([
        ("schemaVersion", JsValue::Number(1.0)),
        ("subject", subject()),
        ("verdict", JsValue::text(verdict)),
        ("findings", JsValue::Array(findings)),
        (
            "decisiveEvidence",
            JsValue::Array(
                (0..decisive)
                    .map(|index| evidence(&format!("src/decisive_{index}.rs")))
                    .collect(),
            ),
        ),
        (
            "limitations",
            JsValue::Array(
                (0..limitations)
                    .map(|index| JsValue::text(&format!("synthetic limitation {index}")))
                    .collect(),
            ),
        ),
    ])
}

fn admitted(value: JsValue, verdict: Verdict, case: &str) {
    match ReviewResult::parse(value) {
        Ok(parsed) => assert_eq!(parsed.verdict, verdict, "{case}"),
        Err(error) => panic!("{case}: refused: {}", error.message),
    }
}

fn refused(value: JsValue, message: &str, case: &str) {
    let error = ReviewResult::parse(value).expect_err(case);
    assert_eq!(error.kind, ErrorKind::ResultContract, "{case}");
    assert_eq!(error.message, message, "{case}");
}

#[test]
fn acceptable_as_is_may_carry_limitations() {
    for limitations in [1, 6] {
        admitted(
            result("acceptable_as_is", vec![], 1, limitations),
            Verdict::AcceptableAsIs,
            &format!("{limitations} limitations"),
        );
    }
}

#[test]
fn acceptable_as_is_may_carry_an_open_info_finding() {
    for limitations in [0, 2] {
        admitted(
            result(
                "acceptable_as_is",
                vec![finding("F1", "info", "open")],
                1,
                limitations,
            ),
            Verdict::AcceptableAsIs,
            &format!("open info, {limitations} limitations"),
        );
    }
}

#[test]
fn acceptable_as_is_refuses_an_open_finding_above_info() {
    for severity in ABOVE_INFO {
        for limitations in [0, 2] {
            refused(
                result(
                    "acceptable_as_is",
                    vec![finding("F1", severity, "open")],
                    1,
                    limitations,
                ),
                ABOVE_INFO_PENDING,
                &format!("open {severity}, {limitations} limitations"),
            );
        }
        // One above-info pending finding is refused even beside an info one.
        refused(
            result(
                "acceptable_as_is",
                vec![
                    finding("F1", "info", "open"),
                    finding("F2", severity, "open"),
                ],
                1,
                0,
            ),
            ABOVE_INFO_PENDING,
            &format!("open info and open {severity}"),
        );
    }
}

#[test]
fn each_pending_status_is_admitted_at_info_and_refused_at_low() {
    for status in PENDING {
        admitted(
            result(
                "acceptable_as_is",
                vec![finding("F1", "info", status)],
                1,
                1,
            ),
            Verdict::AcceptableAsIs,
            &format!("{status} info"),
        );
        refused(
            result("acceptable_as_is", vec![finding("F1", "low", status)], 1, 1),
            ABOVE_INFO_PENDING,
            &format!("{status} low"),
        );
    }
}

#[test]
fn settled_findings_of_any_severity_remain_admissible() {
    for status in SETTLED {
        for severity in ABOVE_INFO {
            admitted(
                result(
                    "acceptable_as_is",
                    vec![finding("F1", severity, status)],
                    1,
                    0,
                ),
                Verdict::AcceptableAsIs,
                &format!("{status} {severity}"),
            );
        }
    }
}

#[test]
fn acceptable_as_is_still_requires_decisive_evidence() {
    refused(
        result("acceptable_as_is", vec![], 0, 0),
        NO_DECISIVE,
        "nothing",
    );
    refused(
        result("acceptable_as_is", vec![], 0, 2),
        NO_DECISIVE,
        "limitations only",
    );
    refused(
        result(
            "acceptable_as_is",
            vec![finding("F1", "info", "open")],
            0,
            2,
        ),
        NO_DECISIVE,
        "open info and limitations",
    );
}

#[test]
fn remediation_required_and_incomplete_are_unchanged() {
    // remediation_required still needs a pending finding, of any severity.
    for severity in ["info", "low", "blocker"] {
        for status in PENDING {
            admitted(
                result(
                    "remediation_required",
                    vec![finding("F1", severity, status)],
                    0,
                    0,
                ),
                Verdict::RemediationRequired,
                &format!("remediation_required {status} {severity}"),
            );
        }
    }
    for findings in [vec![], vec![finding("F1", "high", "verified_resolved")]] {
        refused(
            result("remediation_required", findings, 1, 1),
            "remediation_required requires an unresolved evidence-bearing finding",
            "remediation_required without a pending finding",
        );
    }
    // incomplete still needs limitations.
    refused(
        result("incomplete", vec![finding("F1", "info", "open")], 1, 0),
        "incomplete requires explicit limitations",
        "incomplete without limitations",
    );
    admitted(
        result("incomplete", vec![finding("F1", "high", "open")], 0, 1),
        Verdict::Incomplete,
        "incomplete with a limitation",
    );
}

fn raw_reference(owner: &str, name: &str, revision: &str) -> JsValue {
    JsValue::object([
        ("owner", JsValue::text(owner)),
        ("reference", JsValue::text(name)),
        ("revision", JsValue::text(revision)),
        (
            "sha256",
            JsValue::text(&format!("{:x}", Sha256::digest(name.as_bytes()))),
        ),
        ("freshness", JsValue::text("exact_revision")),
    ])
}

fn authority() -> Authority {
    let identity = JsValue::object([
        ("runId", JsValue::text("contract-run")),
        ("sliceNumber", JsValue::Number(1.0)),
        ("attemptId", JsValue::text("attempt-1")),
        ("planVersion", JsValue::text("plan-v1")),
        ("reviewObligationId", JsValue::text("generic")),
        ("reviewEpisodeId", JsValue::text("episode-result-contract")),
    ]);
    let mut initial_subject = raw_reference("checkpoint", "candidate", "candidate-commit");
    if let JsValue::Object(map) = &mut initial_subject {
        map.insert("sha256".into(), JsValue::text(&digest(&subject())));
    }
    Authority::parse(JsValue::object([
        ("schemaVersion", JsValue::Number(1.0)),
        ("grantId", JsValue::text("grant-1")),
        ("identity", identity),
        ("source", raw_reference("human", "accepted-plan", "plan-v1")),
        (
            "writer",
            JsValue::object([
                ("actorId", JsValue::text("reviewer")),
                ("provider", JsValue::text("fixture")),
                ("generation", JsValue::Number(1.0)),
                (
                    "runtimeSession",
                    raw_reference("runtime", "session-1", "generation-1"),
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
        ("predecessorRevision", JsValue::Null),
    ]))
    .unwrap()
}

fn record(result: JsValue) -> Result<ReviewEpisodeState, review_episode_core::EpisodeError> {
    let grant = authority();
    let first = begin(
        None,
        &BeginCommand::new(grant.clone(), "begin", vec![]).unwrap(),
    )
    .unwrap()
    .into_state();
    assert_eq!(first.phase(), Phase::InitialReview);
    transition(
        &first,
        &TransitionCommand::new(
            grant,
            first.revision().clone(),
            "result",
            Action::RecordResult,
            JsValue::object([
                ("result", result),
                ("unresolvedQuestions", JsValue::Array(vec![])),
            ]),
        )
        .unwrap(),
    )
    .map(|outcome| outcome.into_state())
}

#[test]
fn acceptable_as_is_with_limitations_and_info_finding_reaches_reported() {
    let state = record(result(
        "acceptable_as_is",
        vec![finding("F1", "info", "open")],
        4,
        6,
    ))
    .unwrap();
    assert_eq!(state.phase(), Phase::Reported);
    assert_eq!(
        state.get("pendingAction"),
        &JsValue::text("return_review_result_to_builder")
    );
    assert_eq!(
        field(state.get("currentResult"), "verdict").unwrap(),
        &JsValue::text("acceptable_as_is")
    );
    for severity in ABOVE_INFO {
        let error = record(result(
            "acceptable_as_is",
            vec![finding("F1", severity, "open")],
            4,
            6,
        ))
        .expect_err(severity);
        assert_eq!(error.kind, ErrorKind::ResultContract, "{severity}");
        assert_eq!(error.message, ABOVE_INFO_PENDING, "{severity}");
    }
}
