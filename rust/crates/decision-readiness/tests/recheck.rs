//! `recheck` (contract 5.4): which derived checks observed something different in a second
//! battery result. A refusal exits 3 with nothing on stdout.

mod support;

use serde_json::json;
use std::path::{Path, PathBuf};
use support::*;

struct Setup {
    case: Case,
    first: Observed,
    evaluation: PathBuf,
}

fn setup(name: &str) -> Setup {
    let case = met_case(name);
    let first = observe(&case, "run");
    let evaluation = evaluation_file(&case, &first, "e.json");
    // AC5 (a): the evaluation every recheck test starts from has no verdict key anywhere.
    assert!(!has_key_deep(&read_json(&evaluation), "verdict"));
    Setup {
        case,
        first,
        evaluation,
    }
}

fn recheck(setup: &Setup, second: &Observed, extra: &[&str]) -> Run {
    let mut args = vec![
        "recheck".to_string(),
        "--evaluation".into(),
        s(&setup.evaluation),
        "--battery".into(),
        s(&second.result),
        "--observation".into(),
        s(&second.observation),
    ];
    args.extend(extra.iter().map(|a| a.to_string()));
    cli(&args)
}

fn refused(run: &Run, code: &str) {
    assert_eq!(run.code, 3, "{code}: {}", run.stderr);
    assert!(run.stdout.is_empty(), "{code}");
    assert_eq!(run.codes(), vec![code.to_string()]);
}

#[test]
fn an_unchanged_second_observation_reports_unchanged() {
    let st = setup("recheck-same");
    let second = observe(&st.case, "run2");
    let run = recheck(&st, &second, &[]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        run.out_json(),
        json!({"status": "unchanged", "changed_check_ids": []})
    );
}

#[test]
fn a_changed_file_is_reported_by_derived_check_id_in_derived_order() {
    let st = setup("recheck-changed");
    write(&st.case.cand.join("src/a.txt"), "alpha changed\n");
    let second = observe(&st.case, "run2");
    let run = recheck(&st, &second, &[]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let report = run.out_json();
    assert_eq!(report["status"], "observation_changed");
    let changed: Vec<String> = report["changed_check_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let evaluation = read_json(&st.evaluation);
    let derived: Vec<String> = evaluation["derived_check_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    // The `.manifest` check reads the manifest file, which did not change.
    let expected: Vec<String> = derived
        .iter()
        .filter(|id| {
            (id.starts_with("fixture.base.v1.R-file.")
                || id.starts_with("fixture.base.v1.R-manifest."))
                && !id.ends_with(".manifest")
        })
        .cloned()
        .collect();
    assert!(!changed.is_empty());
    assert_eq!(
        changed, expected,
        "only the checks over the changed file differ"
    );
    let positions: Vec<usize> = changed
        .iter()
        .map(|id| derived.iter().position(|d| d == id).unwrap())
        .collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn refusals_exit_3_with_empty_stdout_and_write_no_out_file() {
    let st = setup("recheck-refusals");
    let second = observe(&st.case, "run2");
    let out = st.case.dir.join("recheck.out.json");
    // The observation names a different plan.
    let other_plan = st.case.dir.join("other.plan.json");
    write(
        &other_plan,
        &(std::fs::read_to_string(&st.first.plan).unwrap() + " "),
    );
    let wrong = st.case.dir.join("wrong.observation.json");
    write_observation(
        &wrong,
        &other_plan,
        &second.result,
        second.exit,
        &second.stdout_battery_sha256,
    );
    let wrong_obs = Observed {
        observation: wrong,
        ..clone_observed(&second)
    };
    let run = recheck(&st, &wrong_obs, &["--out", &s(&out)]);
    refused(&run, "observation_plan_digest_mismatch");
    assert!(!out.exists());
    // The observation's result digest does not match the second result.
    let tampered = Observed {
        result: st.case.dir.join("t.result.json"),
        ..clone_observed(&second)
    };
    std::fs::copy(&second.result, &tampered.result).unwrap();
    touch_bytes(&tampered.result);
    refused(
        &recheck(&st, &tampered, &[]),
        "observation_result_digest_mismatch",
    );
    // The second result's exit disagrees with its failed list.
    let bad_exit = st.case.dir.join("badexit.observation.json");
    write_observation(
        &bad_exit,
        &second.plan,
        &second.result,
        1,
        &second.stdout_battery_sha256,
    );
    let run = recheck(
        &st,
        &Observed {
            observation: bad_exit,
            ..clone_observed(&second)
        },
        &[],
    );
    refused(&run, "battery_exit_inconsistent");
    // The stdout digest disagrees with the second result's battery_sha256.
    let bad_sha = st.case.dir.join("badsha.observation.json");
    write_observation(
        &bad_sha,
        &second.plan,
        &second.result,
        second.exit,
        &"0".repeat(64),
    );
    let run = recheck(
        &st,
        &Observed {
            observation: bad_sha,
            ..clone_observed(&second)
        },
        &[],
    );
    refused(&run, "battery_sha256_mismatch");
    // The original result was altered after the evaluation recorded its digest.
    touch_bytes(&st.first.result);
    refused(&recheck(&st, &second, &[]), "battery_file_digest_mismatch");
    untouch_bytes(&st.first.result);
    // An evaluation that fails its structural rules is refused.
    let mut evaluation = read_json(&st.evaluation);
    evaluation["counts"]["met"] = json!(99);
    let bad = st.case.dir.join("bad.evaluation.json");
    write_json(&bad, &evaluation);
    let run = cli(&[
        "recheck",
        "--evaluation",
        &s(&bad),
        "--battery",
        &s(&second.result),
        "--observation",
        &s(&second.observation),
    ]);
    refused(&run, "counts_inconsistent");
}

#[test]
fn out_writes_once_and_refuses_to_overwrite() {
    let st = setup("recheck-out");
    let second = observe(&st.case, "run2");
    let out = st.case.dir.join("recheck.json");
    let run = recheck(&st, &second, &["--out", &s(&out)]);
    assert_eq!(run.code, 0);
    assert!(run.stdout.is_empty());
    assert_eq!(read_json(&out)["status"], "unchanged");
    let again = recheck(&st, &second, &["--out", &s(&out)]);
    assert_eq!(again.code, 2);
    assert!(again.stdout.is_empty());
    assert_eq!(read_json(&out)["status"], "unchanged");
    // An unreadable original result is an input failure, exit 2.
    std::fs::remove_file(&st.first.result).unwrap();
    assert_eq!(recheck(&st, &second, &[]).code, 2);
    let _: &Path = &st.first.plan;
}

fn clone_observed(o: &Observed) -> Observed {
    Observed {
        plan: o.plan.clone(),
        result: o.result.clone(),
        observation: o.observation.clone(),
        exit: o.exit,
        stdout_battery_sha256: o.stdout_battery_sha256.clone(),
    }
}
