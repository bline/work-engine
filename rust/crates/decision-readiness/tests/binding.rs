//! `evaluate` binding refusals (contract 5.2): exit 3, nothing on stdout, no `--out` file.
//!
//! Each test derives a plan, runs the unmodified battery, then damages one input (and, unless the
//! test is about the digests, re-records the observation so the damage is the only violation).

mod support;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;
use support::*;

fn setup(name: &str) -> (Case, Observed) {
    let case = base_case(name);
    let observed = observe(&case, "run");
    (case, observed)
}

fn edit(path: &Path, f: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    f(&mut value);
    std::fs::remove_file(path).unwrap();
    write_json(path, &value);
}

/// Record the current plan and result digests in the observation.
fn rerecord(observed: &Observed) {
    std::fs::remove_file(&observed.observation).unwrap();
    write_observation(
        &observed.observation,
        &observed.plan,
        &observed.result,
        observed.exit,
        &observed.stdout_battery_sha256,
    );
}

fn plan_check<'a>(plan: &'a mut Value, id: &str) -> &'a mut Value {
    plan["checks"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("no plan check {id}"))
}

const FILE1: &str = "fixture.base.v1.R-file.1";
const JSON1: &str = "fixture.base.v1.R-json.1";
const GREP21: &str = "fixture.base.v1.R-grep.2.1";

/// Evaluate and require a binding refusal whose codes are exactly `codes`.
fn refused(case: &Case, observed: &Observed, codes: &[&str]) -> Run {
    let out = case.dir.join("refused-evaluation.json");
    let _ = std::fs::remove_file(&out);
    let run = case.evaluate(observed, &["--out", &s(&out)]);
    assert_eq!(run.code, 3, "stdout: {} stderr: {}", run.stdout, run.stderr);
    assert!(run.stdout.is_empty(), "stdout: {}", run.stdout);
    assert!(!out.exists(), "a refusal created --out");
    let got: BTreeSet<String> = run.codes().into_iter().collect();
    let want: BTreeSet<String> = codes.iter().map(|c| c.to_string()).collect();
    assert_eq!(got, want, "stderr: {}", run.stderr);
    run
}

#[test]
fn the_undamaged_run_evaluates() {
    let (case, observed) = setup("bind-control");
    let run = case.evaluate(&observed, &[]);
    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
}

#[test]
fn a_derived_check_missing_from_the_run_plan_is_refused() {
    let (case, observed) = setup("bind-missing");
    edit(&observed.plan, |plan| {
        plan["checks"]
            .as_array_mut()
            .unwrap()
            .retain(|c| c["id"] != FILE1);
    });
    rerecord(&observed);
    let run = refused(&case, &observed, &["derived_check_missing"]);
    assert!(run.stderr.contains(FILE1));
}

#[test]
fn an_altered_derived_check_is_refused() {
    let (case, observed) = setup("bind-altered");
    edit(&observed.plan, |plan| {
        plan_check(plan, GREP21)["grep"]["pattern"] = json!("a different pattern");
    });
    rerecord(&observed);
    refused(&case, &observed, &["derived_check_altered"]);
}

#[test]
fn a_run_plan_whose_root_is_not_the_git_root_is_refused() {
    let (case, observed) = setup("bind-root");
    let git_root = s(&case.dest);
    edit(&observed.plan, |plan| {
        let cand = s(&case.cand);
        // The git root moves to `extra_roots`, so the root *paths* are all still present.
        let extras = plan["extra_roots"].as_array_mut().unwrap();
        extras.retain(|r| r != &json!(cand));
        extras.push(json!(git_root));
        plan["root"] = json!(cand);
    });
    rerecord(&observed);
    refused(&case, &observed, &["run_plan_root_mismatch"]);
}

#[test]
fn a_derived_root_absent_from_the_run_plan_is_refused() {
    let (case, observed) = setup("bind-rootmissing");
    edit(&observed.plan, |plan| {
        plan["extra_roots"].as_array_mut().unwrap().remove(0);
    });
    rerecord(&observed);
    refused(&case, &observed, &["run_plan_root_missing"]);
}

#[test]
fn a_malformed_run_plan_is_refused() {
    for (n, damage) in [
        "not json",
        "missing root",
        "extra_roots not array",
        "check without id",
    ]
    .iter()
    .enumerate()
    {
        let (case, observed) = setup(&format!("bind-malformed-{n}"));
        match *damage {
            "not json" => {
                std::fs::remove_file(&observed.plan).unwrap();
                write(&observed.plan, "{ nope");
            }
            "missing root" => edit(&observed.plan, |p| {
                p.as_object_mut().unwrap().remove("root");
            }),
            "extra_roots not array" => edit(&observed.plan, |p| p["extra_roots"] = json!("x")),
            _ => edit(&observed.plan, |p| {
                p["checks"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"sha256": []}));
            }),
        }
        rerecord(&observed);
        let run = case.evaluate(&observed, &[]);
        assert_eq!(run.code, 3, "{damage}: {}", run.stderr);
        assert!(run.stdout.is_empty());
        assert!(
            run.codes().contains(&"run_plan_malformed".to_string()),
            "{damage}: {}",
            run.stderr
        );
    }
}

#[test]
fn a_duplicate_check_id_in_the_run_plan_is_refused() {
    let (case, observed) = setup("bind-dup");
    edit(&observed.plan, |plan| {
        let copy = plan_check(plan, FILE1).clone();
        plan["checks"].as_array_mut().unwrap().push(copy);
    });
    rerecord(&observed);
    refused(&case, &observed, &["run_plan_duplicate_id"]);
}

#[test]
fn a_result_without_an_entry_for_a_derived_id_is_refused() {
    let (case, observed) = setup("bind-entry");
    edit(&observed.result, |r| {
        r["checks"].as_object_mut().unwrap().remove(JSON1);
    });
    rerecord(&observed);
    refused(&case, &observed, &["result_entry_missing"]);
}

#[test]
fn a_result_entry_of_the_wrong_kind_is_refused() {
    let (case, observed) = setup("bind-kind");
    edit(&observed.result, |r| {
        r["checks"][JSON1]["kind"] = json!("grep")
    });
    rerecord(&observed);
    refused(&case, &observed, &["result_kind_mismatch"]);
}

#[test]
fn a_sha256_entry_that_does_not_echo_the_spec_is_refused() {
    let (case, observed) = setup("bind-echo-sha");
    edit(&observed.result, |r| {
        r["checks"][FILE1]["entries"][0]["path"] = json!("/somewhere/else.txt");
    });
    rerecord(&observed);
    refused(&case, &observed, &["echo_mismatch"]);

    let (case, observed) = setup("bind-echo-count");
    edit(&observed.result, |r| {
        r["checks"][FILE1]["path_count"] = json!(2)
    });
    rerecord(&observed);
    refused(&case, &observed, &["echo_mismatch"]);
}

#[test]
fn a_json_entry_whose_selected_keys_differ_from_the_spec_is_refused() {
    let (case, observed) = setup("bind-echo-json");
    edit(&observed.result, |r| {
        r["checks"][JSON1]["selected"] = json!({"other": "ok"});
    });
    rerecord(&observed);
    refused(&case, &observed, &["echo_mismatch"]);
}

#[test]
fn a_grep_match_outside_the_spec_paths_is_refused() {
    let (case, observed) = setup("bind-echo-grep");
    edit(&observed.result, |r| {
        r["checks"][GREP21]["matches"][0]["path"] = json!("/not/in/the/spec.log");
    });
    rerecord(&observed);
    refused(&case, &observed, &["echo_mismatch"]);

    // match_count below the number of matches is also an echo failure.
    let (case, observed) = setup("bind-echo-grep-count");
    edit(&observed.result, |r| {
        r["checks"][GREP21]["match_count"] = json!(0)
    });
    rerecord(&observed);
    refused(&case, &observed, &["echo_mismatch"]);
}

#[test]
fn a_result_entry_with_a_missing_field_is_result_malformed_not_echo_mismatch() {
    let (case, observed) = setup("bind-resmal");
    edit(&observed.result, |r| {
        r["checks"][FILE1]
            .as_object_mut()
            .unwrap()
            .remove("entries");
    });
    rerecord(&observed);
    refused(&case, &observed, &["result_malformed"]);

    let (case, observed) = setup("bind-resmal-sha");
    edit(&observed.result, |r| {
        r.as_object_mut().unwrap().remove("battery_sha256");
    });
    rerecord(&observed);
    let run = case.evaluate(&observed, &[]);
    assert_eq!(run.code, 3, "{}", run.stderr);
    assert!(run.codes().contains(&"result_malformed".to_string()));
}

#[test]
fn observation_digests_that_do_not_match_the_files_passed_are_refused() {
    let (case, observed) = setup("bind-obs-plan");
    edit(&observed.plan, |p| {
        p["note"] = json!("edited after observation")
    });
    refused(&case, &observed, &["observation_plan_digest_mismatch"]);

    let (case, observed) = setup("bind-obs-result");
    edit(&observed.result, |r| {
        r["note"] = json!("edited after observation")
    });
    refused(&case, &observed, &["observation_result_digest_mismatch"]);

    // Both at once: both are reported.
    let (case, observed) = setup("bind-obs-both");
    edit(&observed.plan, |p| p["note"] = json!("edited"));
    edit(&observed.result, |r| r["note"] = json!("edited"));
    refused(
        &case,
        &observed,
        &[
            "observation_plan_digest_mismatch",
            "observation_result_digest_mismatch",
        ],
    );
}

#[test]
fn an_observation_that_fails_its_schema_or_is_not_json_is_refused() {
    let (case, observed) = setup("bind-obs-invalid");
    edit(&observed.observation, |o| o["battery_exit"] = json!(2));
    refused(&case, &observed, &["observation_invalid"]);

    let (case, observed) = setup("bind-obs-notjson");
    std::fs::remove_file(&observed.observation).unwrap();
    write(&observed.observation, "{ nope");
    refused(&case, &observed, &["observation_invalid"]);

    let (case, observed) = setup("bind-obs-schema");
    edit(&observed.observation, |o| {
        o["schema"] = json!("something-else")
    });
    refused(&case, &observed, &["observation_invalid"]);
}

#[test]
fn a_battery_exit_that_disagrees_with_the_failed_list_is_refused() {
    // The result's failed list is empty; an exit of 1 contradicts it.
    let (case, observed) = setup("bind-exit");
    edit(&observed.observation, |o| o["battery_exit"] = json!(1));
    refused(&case, &observed, &["battery_exit_inconsistent"]);

    // The converse: a non-empty failed list with an exit of 0.
    let (case, observed) = setup("bind-exit-conv");
    edit(&observed.result, |r| r["failed"] = json!([JSON1]));
    rerecord(&observed);
    refused(&case, &observed, &["battery_exit_inconsistent"]);
}

#[test]
fn a_stdout_battery_sha256_that_differs_from_the_result_is_refused() {
    let (case, observed) = setup("bind-sha");
    edit(&observed.observation, |o| {
        o["stdout_battery_sha256"] = json!("e".repeat(64));
    });
    refused(&case, &observed, &["battery_sha256_mismatch"]);
}

#[test]
fn every_violation_is_listed_not_only_the_first() {
    let (case, observed) = setup("bind-many");
    edit(&observed.plan, |plan| {
        plan["checks"]
            .as_array_mut()
            .unwrap()
            .retain(|c| c["id"] != FILE1);
        plan_check(plan, GREP21)["grep"]["pattern"] = json!("changed");
    });
    edit(&observed.result, |r| {
        r["checks"].as_object_mut().unwrap().remove(JSON1);
    });
    edit(&observed.observation, |o| {
        o["stdout_battery_sha256"] = json!("e".repeat(64));
    });
    // The plan and result were edited after the observation recorded them as well.
    let run = refused(
        &case,
        &observed,
        &[
            "derived_check_missing",
            "derived_check_altered",
            "result_entry_missing",
            "battery_sha256_mismatch",
            "observation_plan_digest_mismatch",
            "observation_result_digest_mismatch",
        ],
    );
    assert!(run.err_json()["errors"].as_array().unwrap().len() >= 6);
}
