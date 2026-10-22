//! AC5: tooling never emits a verdict, shown by type and structure.
//!
//! (a) An evaluation with a `verdict` key fails to parse, and no serialised evaluation here
//! carries the key. (b) `Assessment` derives `Deserialize` and not `Serialize`. (c) The CLI has
//! no subcommand that writes an assessment. Fixtures and error strings may contain "ready".

mod support;

use decision_readiness::model::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use support::*;

fn evaluation() -> Evaluation {
    let sha = "a".repeat(64);
    Evaluation {
        schema: EVALUATION_SCHEMA.to_string(),
        profile: ProfileRef {
            id: "p".into(),
            version: 1,
            sha256: sha.clone(),
        },
        subject: SubjectRef {
            subject_id: "s".into(),
            sha256: sha.clone(),
        },
        run_plan: FileRef {
            path: "/abs/plan.json".into(),
            sha256: sha.clone(),
        },
        battery: BatteryRecord {
            path: "/abs/result.json".into(),
            file_sha256: sha.clone(),
            battery_sha256: sha.clone(),
            failed: vec![],
        },
        observation: ObservationRecord {
            path: "/abs/observation.json".into(),
            sha256: sha.clone(),
            started_at: "2026-10-21T12:00:00Z".into(),
            finished_at: "2026-10-21T12:00:03Z".into(),
            battery_exit: 0,
            battery_script_sha256: sha.clone(),
        },
        tool: ToolRecord {
            name: "work-engine-readiness".into(),
            version: "0.1.0".into(),
            binary_sha256: sha,
        },
        evaluated_at: "2026-10-21T12:00:05Z".into(),
        derived_check_ids: vec!["p.v1.R.1".into()],
        binding: Binding {
            git_root: GitRootBinding::NotApplicable,
            checks: BTreeMap::from([("p.v1.R.1".to_string(), EchoBinding::EchoVerified)]),
        },
        requirements: vec![EvaluatedRequirement {
            id: "R".into(),
            class: EvaluatedClass::Mechanical,
            status: Status::Met,
            reason: None,
            predicates: vec![PredicateResult {
                index: 1,
                kind: "file_sha256".into(),
                status: Status::Met,
                reason: None,
                detail: None,
            }],
            evidence: vec![],
        }],
        counts: Counts {
            met: 1,
            unmet: 0,
            indeterminate: 0,
        },
    }
}

fn contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => map.contains_key(key) || map.values().any(|v| contains_key(v, key)),
        Value::Array(items) => items.iter().any(|v| contains_key(v, key)),
        _ => false,
    }
}

#[test]
fn an_evaluation_round_trips_and_carries_no_verdict_key() {
    let original = evaluation();
    let value = serde_json::to_value(&original).unwrap();
    assert!(!contains_key(&value, "verdict"));
    let back: Evaluation = serde_json::from_value(value).unwrap();
    assert_eq!(back, original);
}

#[test]
fn an_evaluation_with_a_verdict_key_fails_to_parse() {
    let mut top = serde_json::to_value(evaluation()).unwrap();
    top["verdict"] = json!("ready");
    let error = serde_json::from_value::<Evaluation>(top)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown field `verdict`"), "{error}");
    // Also nested: a requirement or a counts object cannot carry one.
    let mut nested = serde_json::to_value(evaluation()).unwrap();
    nested["requirements"][0]["verdict"] = json!("ready");
    assert!(serde_json::from_value::<Evaluation>(nested).is_err());
    let mut counts = serde_json::to_value(evaluation()).unwrap();
    counts["counts"]["verdict"] = json!("blocked");
    assert!(serde_json::from_value::<Evaluation>(counts).is_err());
}

// Autoref specialisation: `Witness::<T>::IS_SERIALIZE` resolves to the inherent constant when
// `T: Serialize` holds, and to the trait default otherwise. No new crate is involved.
struct Witness<T>(PhantomData<T>);
trait Fallback {
    const IS_SERIALIZE: bool = false;
}
impl<T> Fallback for Witness<T> {}
impl<T: Serialize> Witness<T> {
    const IS_SERIALIZE: bool = true;
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn assessment_has_no_way_to_be_serialised() {
    assert!(
        !Witness::<Assessment>::IS_SERIALIZE,
        "Assessment must not implement Serialize"
    );
    // The same types an assessment holds are also deserialise-only.
    assert!(!Witness::<AssessedRequirement>::IS_SERIALIZE);
    assert!(!Witness::<Assessor>::IS_SERIALIZE);
    assert!(!Witness::<FreshnessBasis>::IS_SERIALIZE);
    // The witness distinguishes: an evaluation does serialise.
    assert!(Witness::<Evaluation>::IS_SERIALIZE);
    assert!(Witness::<Observation>::IS_SERIALIZE);
}

#[test]
fn the_cli_has_no_subcommand_that_writes_an_assessment() {
    let case = base_case("no-verdict-cli");
    for subcommand in [
        "assess",
        "assessment",
        "verdict",
        "decide",
        "issue",
        "judge",
    ] {
        let run = cli(&[subcommand, "--profile", &s(&case.profile_path())]);
        assert_eq!(run.code, 2, "{subcommand}");
        assert!(run.stdout.is_empty());
    }
    // `check --kind assessment` without its inputs is a usage error and writes nothing; it has
    // no `--out`, so it cannot write a document even with them.
    let run = cli(&["check", "--kind", "assessment", &s(&case.profile_path())]);
    assert_eq!(run.code, 2);
    assert!(run.stdout.is_empty());
    let out = case.dir.join("assessment-out.json");
    let run = cli(&[
        "check",
        "--kind",
        "assessment",
        &s(&case.profile_path()),
        "--evaluation",
        &s(&case.profile_path()),
        "--profile",
        &s(&case.profile_path()),
        "--out",
        &s(&out),
    ]);
    assert_eq!(run.code, 2);
    assert!(!out.exists());
    // The usage text lists exactly the subcommands this build provides (AC5 c): none of them
    // writes an assessment.
    let usage = cli::<&str>(&[]).err_json()["usage"]
        .as_str()
        .unwrap()
        .to_string();
    let mut subcommands: Vec<&str> = usage
        .lines()
        .filter_map(|line| line.trim().strip_prefix("work-engine-readiness "))
        .filter_map(|rest| rest.split_whitespace().next())
        .collect();
    subcommands.sort_unstable();
    subcommands.dedup();
    assert_eq!(subcommands, ["check", "evaluate", "plan", "recheck"]);
}

#[test]
fn check_and_plan_output_never_carry_a_verdict_key() {
    let case = base_case("no-verdict-output");
    assert!(!contains_key(&case.plan().out_json(), "verdict"));
    assert!(!contains_key(&case.check_profile().out_json(), "verdict"));
    assert!(!contains_key(&case.check_subject().out_json(), "verdict"));
}
