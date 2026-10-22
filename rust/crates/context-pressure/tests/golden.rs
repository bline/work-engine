//! Parity with the JS reference, proven against JS-generated golden vectors.
//!
//! `tests/golden/vectors.json` is produced by `tests/golden/generate.mjs`, which runs the JS
//! modules over a fixed corpus and records, for every case, either the JS output or the failure
//! (`{kind, message}`, the kind mapped from the JS message by a regex table). Each test replays
//! the corpus through the Rust API and requires `serde_json::Value` equality of outputs and
//! equality of error kind and message; revision strings are compared by `==`. The fixture header
//! records the Node version and the sha256 of both JS files, and `fixture_matches_js_sources`
//! fails when either file has changed since generation.
//!
//! Declared differences (not parity claims):
//! * `deviationCases`: the JS hashes and interpolates a missing or non-string `threadId` and a
//!   missing `schemaVersion`; Rust rejects them (`invalid_snapshot_field`).
//! * Timestamps tagged `legacy`: V8 `Date.parse` accepts non-ISO legacy forms; Rust accepts only
//!   the ECMAScript date-time grammar and must never accept what V8 rejects. Tagged `es_iso`
//!   strings must have identical verdicts.
//! * Not covered: lone-surrogate strings (not representable in `serde_json` input), and numbers
//!   outside f64 range (JS `JSON.parse` yields Infinity; `serde_json` rejects the text).

use context_pressure::{
    ContextPressureController, ContextPressurePolicy, PressureError, PressureObservation,
    TokenUsagePressureProfile, TokenUsagePressureProjector,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

const VECTORS: &str = include_str!("golden/vectors.json");

fn fixture() -> Value {
    serde_json::from_str(VECTORS).expect("vectors.json parses")
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|error| panic!("fixture input {text:?}: {error}"))
}

fn outcome<T: Serialize>(result: Result<T, PressureError>) -> Value {
    match result {
        Ok(value) => json!({ "ok": serde_json::to_value(value).expect("serializes") }),
        Err(error) => {
            json!({ "err": { "kind": error.kind().as_str(), "message": error.message() } })
        }
    }
}

fn cases<'a>(fixture: &'a Value, section: &str) -> &'a Vec<Value> {
    fixture[section]
        .as_array()
        .unwrap_or_else(|| panic!("section {section}"))
}

fn text<'a>(case: &'a Value, field: &str) -> &'a str {
    case[field]
        .as_str()
        .unwrap_or_else(|| panic!("field {field} in {case}"))
}

#[derive(Default)]
struct Mismatches(Vec<String>);

impl Mismatches {
    fn check(&mut self, label: &str, expected: &Value, actual: &Value) {
        if expected != actual {
            self.0
                .push(format!("{label}\n  js:   {expected}\n  rust: {actual}"));
        }
    }

    fn finish(self, total: usize) {
        if !self.0.is_empty() {
            let shown: Vec<&str> = self.0.iter().take(25).map(String::as_str).collect();
            panic!(
                "{} of {total} golden cases differ from the JS reference:\n{}",
                self.0.len(),
                shown.join("\n")
            );
        }
    }
}

#[test]
fn fixture_header_is_consistent() {
    let fixture = fixture();
    let header = &fixture["header"];
    let sections = &header["sections"];
    let count = |section: &str| cases(&fixture, section).len() as u64;
    let steps: u64 = cases(&fixture, "controllerCases")
        .iter()
        .map(|case| case["steps"].as_array().expect("steps").len() as u64)
        .sum();
    let expected = [
        ("policy", count("policyCases")),
        ("profile", count("profileCases")),
        ("projector", count("projectorCases")),
        ("projectorDeviations", count("deviationCases")),
        ("pressureSweepRows", count("sweepRows")),
        ("controllerScenarios", count("controllerCases")),
        ("controllerSteps", steps),
        ("pipelines", count("pipelineCases")),
        ("timestamps", count("timestampCases")),
    ];
    let mut total = 0;
    for (name, actual) in expected {
        assert_eq!(sections[name].as_u64(), Some(actual), "section {name}");
        total += actual;
    }
    assert_eq!(header["caseCount"].as_u64(), Some(total));
    assert!(
        header["nodeVersion"]
            .as_str()
            .is_some_and(|v| v.starts_with('v'))
    );
}

#[test]
fn fixture_matches_js_sources() {
    let fixture = fixture();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let hashes = fixture["header"]["jsSha256"].as_object().expect("jsSha256");
    assert_eq!(hashes.len(), 2);
    for (relative, expected) in hashes {
        let bytes = std::fs::read(root.join(relative))
            .unwrap_or_else(|error| panic!("read {relative}: {error}"));
        let actual = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(
            expected.as_str(),
            Some(actual.as_str()),
            "stale fixture: {relative} changed"
        );
    }
}

#[test]
fn policy_vectors_match_js() {
    let fixture = fixture();
    let mut mismatches = Mismatches::default();
    let all = cases(&fixture, "policyCases");
    for case in all {
        let actual = outcome(ContextPressurePolicy::from_value(&parse(text(
            case,
            "inputJson",
        ))));
        mismatches.check(
            &format!("policy: {}", text(case, "name")),
            &case["expect"],
            &actual,
        );
    }
    mismatches.finish(all.len());
}

#[test]
fn profile_vectors_match_js() {
    let fixture = fixture();
    let mut mismatches = Mismatches::default();
    let all = cases(&fixture, "profileCases");
    for case in all {
        let actual = outcome(TokenUsagePressureProfile::from_value(&parse(text(
            case,
            "inputJson",
        ))));
        mismatches.check(
            &format!("profile: {}", text(case, "name")),
            &case["expect"],
            &actual,
        );
    }
    mismatches.finish(all.len());
}

fn project(profile_json: &str, snapshot_json: &str) -> Value {
    match TokenUsagePressureProjector::new(&parse(profile_json)) {
        Err(error) => outcome::<()>(Err(error)),
        Ok(projector) => outcome(projector.project(&parse(snapshot_json))),
    }
}

#[test]
fn projector_vectors_match_js() {
    let fixture = fixture();
    let mut mismatches = Mismatches::default();
    let all = cases(&fixture, "projectorCases");
    for case in all {
        let actual = project(text(case, "profileJson"), text(case, "snapshotJson"));
        mismatches.check(
            &format!("projector: {}", text(case, "name")),
            &case["expect"],
            &actual,
        );
    }
    mismatches.finish(all.len());
}

#[test]
fn declared_projector_deviations_are_rejections() {
    let fixture = fixture();
    let all = cases(&fixture, "deviationCases");
    assert_eq!(all.len(), 5);
    for case in all {
        let name = text(case, "name");
        assert!(
            case["jsExpect"]["ok"].is_object(),
            "{name}: the JS accepts this input"
        );
        let actual = project(text(case, "profileJson"), text(case, "snapshotJson"));
        assert_eq!(
            actual["err"]["kind"].as_str(),
            case["rustExpectKind"].as_str(),
            "{name}: {actual}"
        );
    }
}

#[test]
fn pressure_sweep_matches_js() {
    let fixture = fixture();
    let template = text(&fixture, "sweepTemplate");
    let projector = TokenUsagePressureProjector::new(&json!({
        "schemaVersion": 1,
        "usageField": "last.totalTokens",
        "windowField": "modelContextWindow",
        "rounding": "floor",
        "saturation": "clamp_10000",
    }))
    .expect("default profile");
    let mut mismatches = Mismatches::default();
    let rows = cases(&fixture, "sweepRows");
    for row in rows {
        let (measured, window) = (row[0].as_u64().unwrap(), row[1].as_u64().unwrap());
        let snapshot = template
            .replace("__M__", &measured.to_string())
            .replace("__W__", &window.to_string());
        let projected =
            serde_json::to_value(projector.project(&parse(&snapshot)).expect("projects"))
                .expect("serializes");
        let actual = json!([
            projected["observation"]["pressureBasisPoints"],
            projected["observation"]["sourceRevision"],
        ]);
        mismatches.check(
            &format!("sweep m={measured} w={window}"),
            &json!([row[2], row[3]]),
            &actual,
        );
    }
    mismatches.finish(rows.len());
}

#[test]
fn controller_vectors_match_js() {
    let fixture = fixture();
    let mut mismatches = Mismatches::default();
    let all = cases(&fixture, "controllerCases");
    for case in all {
        let name = text(case, "name");
        let built = ContextPressureController::from_options(&parse(text(case, "optionsJson")));
        let mut controller = match built {
            Err(error) => {
                mismatches.check(
                    &format!("controller construct: {name}"),
                    &case["construct"],
                    &outcome::<()>(Err(error)),
                );
                continue;
            }
            Ok(controller) => controller,
        };
        mismatches.check(
            &format!("controller construct: {name}"),
            &case["construct"],
            &outcome(Ok::<_, PressureError>(controller.snapshot())),
        );
        for (index, step) in case["steps"].as_array().expect("steps").iter().enumerate() {
            let result = outcome(controller.observe(&parse(text(step, "inputJson"))));
            mismatches.check(
                &format!("controller {name} step {index}"),
                &step["expect"],
                &result,
            );
            let snapshot = serde_json::to_value(controller.snapshot()).expect("serializes");
            mismatches.check(
                &format!("controller {name} step {index} snapshot"),
                &step["snapshot"],
                &snapshot,
            );
        }
    }
    mismatches.finish(all.len());
}

#[test]
fn projector_to_controller_pipelines_match_js() {
    let fixture = fixture();
    let projector = TokenUsagePressureProjector::new(&json!({
        "schemaVersion": 1,
        "usageField": "last.totalTokens",
        "windowField": "modelContextWindow",
        "rounding": "floor",
        "saturation": "clamp_10000",
    }))
    .expect("default profile");
    let mut mismatches = Mismatches::default();
    let all = cases(&fixture, "pipelineCases");
    for case in all {
        let name = text(case, "name");
        let mut controller = ContextPressureController::from_options(&json!({
            "policy": {
                "schemaVersion": 1,
                "unit": "basis_points",
                "approaching": { "enter": 6500, "exit": 5500 },
                "replacementCandidate": { "enter": 7800, "exit": 6800 },
                "critical": { "enter": 9000, "exit": 8000 },
            }
        }))
        .expect("default policy");
        for (index, step) in case["steps"].as_array().expect("steps").iter().enumerate() {
            let projected = serde_json::to_value(
                projector
                    .project(&parse(text(step, "snapshotJson")))
                    .expect("projects"),
            )
            .expect("serializes");
            mismatches.check(
                &format!("pipeline {name} step {index} projected"),
                &step["projected"],
                &projected,
            );
            let observed = outcome(controller.observe(&projected["observation"]));
            mismatches.check(
                &format!("pipeline {name} step {index} observed"),
                &step["observed"],
                &observed,
            );
            let snapshot = serde_json::to_value(controller.snapshot()).expect("serializes");
            mismatches.check(
                &format!("pipeline {name} step {index} snapshot"),
                &step["snapshot"],
                &snapshot,
            );
        }
    }
    mismatches.finish(all.len());
}

fn rust_accepts_timestamp(candidate: &str) -> bool {
    PressureObservation::from_value(&json!({
        "schemaVersion": 1,
        "observationId": "t",
        "sequence": 1,
        "observedAt": candidate,
        "pressureBasisPoints": 1,
        "source": "s",
        "sourceRevision": format!("sha256:{}", "a".repeat(64)),
    }))
    .is_ok()
}

#[test]
fn timestamp_corpus_never_accepts_what_v8_rejects() {
    let fixture = fixture();
    let all = cases(&fixture, "timestampCases");
    let (mut es_total, mut es_differs) = (0, Vec::new());
    let (mut legacy_total, mut legacy_v8_only, mut legacy_both) = (0, Vec::new(), 0);
    let mut looser = Vec::new();
    for case in all {
        let candidate = text(case, "text");
        let v8 = case["v8"].as_bool().expect("v8 verdict");
        let rust = rust_accepts_timestamp(candidate);
        if rust && !v8 {
            looser.push(format!("{candidate:?}"));
        }
        match text(case, "tag") {
            "es_iso" => {
                es_total += 1;
                if rust != v8 {
                    es_differs.push(format!("{candidate:?} v8={v8} rust={rust}"));
                }
            }
            "legacy" => {
                legacy_total += 1;
                match (v8, rust) {
                    (true, false) => legacy_v8_only.push(format!("{candidate:?}")),
                    (true, true) => legacy_both += 1,
                    _ => {}
                }
            }
            other => panic!("unknown tag {other}"),
        }
    }
    println!(
        "timestamps: es_iso {es_total} cases, {} verdict differences; legacy {legacy_total} cases, \
         V8-accepts-Rust-rejects {} (accepted by both {legacy_both}); Rust-accepts-V8-rejects {}",
        es_differs.len(),
        legacy_v8_only.len(),
        looser.len()
    );
    println!(
        "legacy forms V8 accepts and Rust rejects: {}",
        legacy_v8_only.join(", ")
    );
    assert!(
        looser.is_empty(),
        "Rust accepts what V8 rejects: {}",
        looser.join(", ")
    );
    assert!(
        es_differs.is_empty(),
        "{} es_iso verdict differences:\n{}",
        es_differs.len(),
        es_differs
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
