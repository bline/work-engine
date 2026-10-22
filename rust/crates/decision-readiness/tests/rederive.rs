//! `check --kind evaluation`: structural rules and `--rederive` (contract 3, 5.3; AC2, AC5 a).

mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::*;

type Mutation = Box<dyn Fn(&mut Value)>;

fn rederive_args(case: &Case, evaluation: &Path) -> Vec<String> {
    vec![
        "check".into(),
        "--kind".into(),
        "evaluation".into(),
        s(evaluation),
        "--rederive".into(),
        "--profile".into(),
        s(&case.profile_path()),
        "--subject".into(),
        s(&case.subject_path()),
    ]
}

fn check_value(case: &Case, tag: &str, value: &Value) -> Run {
    let path = case.dir.join(format!("{tag}.evaluation.json"));
    write_json(&path, value);
    cli(&["check", "--kind", "evaluation", &s(&path)])
}

#[test]
fn evaluate_twice_gives_identical_bytes_and_check_accepts_it() {
    let case = met_case("rederive-twice");
    let observed = observe(&case, "run");
    let first = case.evaluate(&observed, &[]);
    let second = case.evaluate(&observed, &[]);
    assert_eq!((first.code, second.code), (0, 0));
    assert_eq!(first.stdout, second.stdout);
    assert!(!has_key_deep(&first.out_json(), "verdict"));
    let file = evaluation_file(&case, &observed, "e.json");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), first.stdout);
    let run = cli(&["check", "--kind", "evaluation", &s(&file)]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let report = run.out_json();
    assert_eq!(report["valid"], true);
    assert_eq!(report["kind"], "evaluation");
}

#[test]
fn rederive_passes_on_a_fixture_evaluation_and_reports_a_tool_difference_as_a_note() {
    let case = met_case("rederive-pass");
    let observed = observe(&case, "run");
    let file = evaluation_file(&case, &observed, "e.json");
    let run = cli(&rederive_args(&case, &file));
    assert_eq!(run.code, 0, "{}", run.stdout);
    let report = run.out_json();
    assert_eq!(report["valid"], true);
    assert!(
        report.get("notes").is_none(),
        "no notes when the tool matches"
    );
    // A differing tool block is a note, not an error.
    let mut value = read_json(&file);
    value["tool"]["version"] = json!("0.0.0-other");
    let other = case.dir.join("other.json");
    write_json(&other, &value);
    let run = cli(&rederive_args(&case, &other));
    assert_eq!(run.code, 0, "{}", run.stdout);
    let report = run.out_json();
    assert_eq!(report["notes"][0]["code"], "tool_differs");
    assert_eq!(report["errors"], json!([]));
}

#[test]
fn rederive_exits_3_when_any_supplied_or_recorded_file_digest_differs() {
    let case = met_case("rederive-digests");
    let observed = observe(&case, "run");
    let file = evaluation_file(&case, &observed, "e.json");
    let targets: [(&Path, &str); 5] = [
        (&case.profile_path(), "profile_file_digest_mismatch"),
        (&case.subject_path(), "subject_file_digest_mismatch"),
        (&observed.plan, "run_plan_file_digest_mismatch"),
        (&observed.result, "battery_file_digest_mismatch"),
        (&observed.observation, "observation_file_digest_mismatch"),
    ];
    for (path, code) in targets {
        touch_bytes(path);
        let run = cli(&rederive_args(&case, &file));
        assert_eq!(run.code, 3, "{code}: {}", run.stderr);
        assert!(run.stdout.is_empty(), "{code}");
        assert_eq!(run.codes(), vec![code.to_string()]);
        untouch_bytes(path);
    }
    // Restored: passes again.
    assert_eq!(cli(&rederive_args(&case, &file)).code, 0);
    // An unreadable recorded file is an input failure, exit 2.
    std::fs::remove_file(&observed.observation).unwrap();
    let run = cli(&rederive_args(&case, &file));
    assert_eq!(run.code, 2);
    assert!(run.stdout.is_empty());
}

#[test]
fn a_mutated_evaluation_field_fails_rederive_with_exit_1_naming_the_field() {
    let case = met_case("rederive-mutated");
    let observed = observe(&case, "run");
    let file = evaluation_file(&case, &observed, "e.json");
    let original = read_json(&file);
    let mutations: [(&str, Mutation); 3] = [
        (
            "binding",
            Box::new(|v| v["binding"]["git_root"] = json!("mismatch")),
        ),
        (
            "derived_check_ids",
            Box::new(|v| {
                let ids = v["derived_check_ids"].as_array_mut().unwrap();
                let n = ids.len();
                ids.swap(0, n - 1);
            }),
        ),
        (
            "requirements",
            Box::new(|v| v["requirements"][0]["evidence"][0]["digest"] = json!("f".repeat(64))),
        ),
    ];
    for (field, mutate) in mutations {
        let mut value = original.clone();
        mutate(&mut value);
        let path = case.dir.join(format!("mut-{field}.json"));
        write_json(&path, &value);
        let run = cli(&rederive_args(&case, &path));
        assert_eq!(run.code, 1, "{field}: {}", run.stdout);
        let report = run.out_json();
        let hits: Vec<_> = report["errors"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["code"] == "rederive_mismatch")
            .map(|e| e["at"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(hits, vec![field.to_string()]);
    }
}

#[test]
fn rederive_refused_by_evaluate_is_the_result_exit_3() {
    let case = met_case("rederive-refused");
    let observed = observe(&case, "run");
    let file = evaluation_file(&case, &observed, "e.json");
    // Change the run plan and record its new digest in the evaluation: the digests agree, so
    // evaluate itself refuses (a derived check was removed).
    let mut plan = read_json(&observed.plan);
    plan["checks"].as_array_mut().unwrap().remove(0);
    write_json(&observed.plan, &plan);
    let mut value = read_json(&file);
    value["run_plan"]["sha256"] = json!(file_sha(&observed.plan));
    let changed = case.dir.join("changed.json");
    write_json(&changed, &value);
    let run = cli(&rederive_args(&case, &changed));
    assert_eq!(run.code, 3, "{}", run.stderr);
    assert!(run.stdout.is_empty());
    assert!(run.codes().contains(&"derived_check_missing".to_string()));
}

#[test]
fn structural_rules_each_report_their_code() {
    let case = met_case("rederive-structural");
    let observed = observe(&case, "run");
    let file = evaluation_file(&case, &observed, "e.json");
    let original = read_json(&file);
    let last = original["requirements"].as_array().unwrap().len() - 1;
    assert_eq!(original["requirements"][last]["class"], "attributed");
    let cases: Vec<(&str, Mutation)> = vec![
        (
            "counts_inconsistent",
            Box::new(|v| v["counts"]["met"] = json!(99)),
        ),
        (
            "status_inconsistent",
            Box::new(|v| v["requirements"][0]["status"] = json!("unmet")),
        ),
        (
            "attributed_shape",
            Box::new(move |v| v["requirements"][last]["reason"] = json!("something else")),
        ),
        (
            "reason_invalid",
            Box::new(|v| v["requirements"][0]["predicates"][0]["reason"] = json!("differs")),
        ),
        (
            "reason_invalid",
            Box::new(|v| {
                v["requirements"][0]["predicates"][0]["status"] = json!("unmet");
                v["requirements"][0]["predicates"][0]["reason"] = json!("bogus");
            }),
        ),
        (
            "detail_too_long",
            Box::new(|v| v["requirements"][0]["predicates"][0]["detail"] = json!("x".repeat(401))),
        ),
        (
            "evidence_invalid",
            Box::new(|v| v["requirements"][0]["evidence"][0]["digest_field"] = json!("sha256")),
        ),
        (
            "invalid_sha256",
            Box::new(|v| v["profile"]["sha256"] = json!("nothex")),
        ),
        (
            "schema_mismatch",
            Box::new(|v| v["schema"] = json!("other-v1")),
        ),
        (
            "invalid_timestamp",
            Box::new(|v| v["evaluated_at"] = json!("2026-13-01T00:00:00Z")),
        ),
        (
            "invalid_absolute_path",
            Box::new(|v| v["run_plan"]["path"] = json!("relative.json")),
        ),
        ("schema", Box::new(|v| v["verdict"] = json!("ready"))),
    ];
    for (i, (code, mutate)) in cases.iter().enumerate() {
        let mut value = original.clone();
        mutate(&mut value);
        let run = check_value(&case, &format!("s{i}"), &value);
        assert_eq!(run.code, 1, "{code}: {}", run.stdout);
        assert!(
            run.codes().contains(&code.to_string()),
            "{code}: {:?}",
            run.codes()
        );
    }
}

/// S2-T1-002. A mechanical status can never be attributed: an attributed requirement is
/// indeterminate with reason `attributed_judgment_required` and no predicates. Each variant keeps
/// the counts consistent, so `attributed_shape` is the only code that can reject it.
#[test]
fn an_attributed_requirement_upgraded_to_met_or_unmet_is_an_attributed_shape_error() {
    let case = met_case("rederive-attributed-met");
    let observed = observe(&case, "run");
    let file = evaluation_file(&case, &observed, "e.json");
    let original = read_json(&file);
    let last = original["requirements"].as_array().unwrap().len() - 1;
    assert_eq!(original["requirements"][last]["class"], "attributed");
    assert_eq!(original["requirements"][last]["status"], "indeterminate");
    assert_eq!(
        check_value(&case, "genuine", &original).code,
        0,
        "the unmodified evaluation is accepted"
    );
    let reason = original["requirements"][last]["reason"].clone();
    let cases: Vec<(&str, &str, Mutation)> = vec![
        ("met, reason kept", "met", Box::new(|_| {})),
        (
            "met, reason dropped",
            "met",
            Box::new(move |v| {
                v["requirements"][last]
                    .as_object_mut()
                    .unwrap()
                    .remove("reason");
            }),
        ),
        ("unmet, reason kept", "unmet", Box::new(|_| {})),
    ];
    for (i, (label, status, mutate)) in cases.iter().enumerate() {
        let mut value = original.clone();
        value["requirements"][last]["status"] = json!(status);
        value["requirements"][last]["reason"] = reason.clone();
        mutate(&mut value);
        let counts = &mut value["counts"];
        counts["indeterminate"] = json!(counts["indeterminate"].as_u64().unwrap() - 1);
        counts[status] = json!(counts[status].as_u64().unwrap() + 1);
        let run = check_value(&case, &format!("attr{i}"), &value);
        assert_eq!(run.code, 1, "{label}: {}", run.stdout);
        assert_eq!(run.codes(), ["attributed_shape"], "{label}");
    }
}
