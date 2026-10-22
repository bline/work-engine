//! `evaluate` outcomes (contract 3.6, 5.3), one fixture per status rule.
//!
//! Each test builds a fresh fixture tree, derives the plan with the tool, runs the unmodified
//! evidence battery over it, and evaluates the saved result.

mod support;
use support::*;

#[test]
fn base_case_evaluates_with_every_requirement_met_but_the_head_comparison() {
    // R-json predicate 6 compares the string "ok" with the `head` parameter (a commit id):
    // unmet `differs` by construction of the base fixture, which was built for `plan`.
    let case = base_case("eval-base");
    let observed = observe(&case, "base");
    let run = case.evaluate(&observed, &[]);
    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    let evaluation = run.out_json();
    assert!(evaluation.get("verdict").is_none());
    assert_eq!(evaluation["binding"]["git_root"], "verified");
    for requirement in evaluation["requirements"].as_array().unwrap() {
        if requirement["id"] == "R-json" {
            assert_eq!(requirement["status"], "unmet");
            assert_eq!(requirement["predicates"][5]["reason"], "differs");
            assert_eq!(requirement["predicates"][4]["status"], "met");
        } else if requirement["class"] == "mechanical" {
            assert_eq!(requirement["status"], "met", "{requirement}");
        } else {
            assert_eq!(requirement["status"], "indeterminate");
            assert_eq!(requirement["reason"], "attributed_judgment_required");
        }
    }
    assert_eq!(evaluation["counts"]["met"], 6);
    assert_eq!(evaluation["counts"]["unmet"], 1);
    assert_eq!(evaluation["counts"]["indeterminate"], 1);
}

// ---------------------------------------------------------------------------------------------
// Single-requirement fixtures for the status rules of contract 3.6.
// ---------------------------------------------------------------------------------------------

use decision_readiness::sha256_hex;
use serde_json::{Value, json};
use std::path::Path;

const REQ: &str = "R";

/// `base_case` with its requirements replaced by `requirements` (parameters are kept).
fn with_requirements(name: &str, requirements: Vec<Value>) -> Case {
    let mut case = base_case(name);
    case.profile["requirements"] = Value::Array(requirements);
    case.write_documents();
    case
}

/// `base_case` with one mechanical requirement `R` holding `predicates`.
fn single(name: &str, predicates: Vec<Value>) -> Case {
    with_requirements(
        name,
        vec![json!({
            "id": REQ, "class": "mechanical", "statement": "single requirement",
            "predicates": predicates
        })],
    )
}

fn file(root: &str, path: &str) -> Value {
    json!({"root": root, "path": path})
}

fn no_verdict_anywhere(value: &Value) {
    match value {
        Value::Object(map) => {
            assert!(!map.contains_key("verdict"), "verdict key in {value}");
            map.values().for_each(no_verdict_anywhere);
        }
        Value::Array(items) => items.iter().for_each(no_verdict_anywhere),
        _ => {}
    }
}

/// Derive the plan, run `prepare` (a change to the tree after planning), run the battery, and
/// evaluate. Asserts a successful evaluation with no `verdict` key anywhere.
fn evaluated(case: &Case, tag: &str, prepare: impl FnOnce()) -> (Observed, Value) {
    evaluated_with(case, tag, prepare, || {})
}

/// As [`evaluated`], with `after_battery` run between the battery and `evaluate`.
fn evaluated_with(
    case: &Case,
    tag: &str,
    prepare: impl FnOnce(),
    after_battery: impl FnOnce(),
) -> (Observed, Value) {
    let planned = case.plan();
    assert_eq!(planned.code, 0, "plan failed: {}", planned.stderr);
    let plan = case.dir.join(format!("{tag}.plan.json"));
    write(&plan, &planned.stdout);
    prepare();
    let observed = observe_plan(case, tag, &plan);
    after_battery();
    let run = case.evaluate(&observed, &[]);
    assert_eq!(run.code, 0, "evaluate failed: {}", run.stderr);
    let evaluation = run.out_json();
    no_verdict_anywhere(&evaluation);
    (observed, evaluation)
}

fn predicates(evaluation: &Value) -> Vec<Value> {
    evaluation["requirements"][0]["predicates"]
        .as_array()
        .unwrap()
        .clone()
}

/// Assert predicate `index` (0-based) has `status` and `reason` (`None`: no reason).
fn expect(list: &[Value], index: usize, status: &str, reason: Option<&str>) {
    let p = &list[index];
    assert_eq!(p["status"], status, "predicate {index}: {p}");
    match reason {
        Some(r) => assert_eq!(p["reason"], r, "predicate {index}: {p}"),
        None => assert!(p.get("reason").is_none(), "predicate {index}: {p}"),
    }
}

fn detail(list: &[Value], index: usize) -> String {
    list[index]["detail"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn symlink(target: &Path, link: &Path) {
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[test]
fn rule1_an_errored_existence_entry_is_indeterminate_existence_check_error() {
    // `cand/escape` is a symlink to a directory outside every root: the battery's sha256 check
    // errors (B:47-48), so the `.exists` entry of each predicate carries `error`.
    let mut case = single(
        "eval-rule1",
        vec![
            json!({"kind": "file_sha256", "file": file("cand", "escape/f.txt"), "expected": "0".repeat(64)}),
            json!({"kind": "json_equals", "file": file("cand", "escape/f.json"), "key": "a", "expected": "x"}),
            json!({"kind": "grep_count", "pattern": "x", "files": [file("cand", "escape/f.txt")], "max": 0}),
        ],
    );
    let outside = case.dir.join("outside");
    write(&outside.join("f.txt"), "x\n");
    write(&outside.join("f.json"), "{\"a\":\"x\"}\n");
    case.write_documents();
    let (_, evaluation) = evaluated(&case, "rule1", || {
        symlink(&outside, &case.cand.join("escape"));
    });
    let list = predicates(&evaluation);
    for i in 0..3 {
        expect(&list, i, "indeterminate", Some("existence_check_error"));
    }
    assert_eq!(evaluation["requirements"][0]["status"], "indeterminate");
}

#[test]
fn rule2_a_path_that_is_not_a_regular_file_is_unmet_before_any_content_rule() {
    let zero = "0".repeat(64);
    let mut case = single(
        "eval-rule2",
        vec![
            json!({"kind": "file_sha256", "file": file("cand", "gone.txt"), "expected": zero}),
            json!({"kind": "file_sha256", "file": file("cand", "d"), "expected": zero}),
            json!({"kind": "file_sha256", "file": file("cand", "link.txt"), "expected": sha256_hex(b"alpha\n")}),
            json!({"kind": "manifest_match", "manifest": {"param": "m2"}}),
            json!({"kind": "json_equals", "file": file("evid", "missing.json"), "key": "status", "expected": "ok"}),
            json!({"kind": "json_compare", "file": file("evid", "dirjson"), "key": "count", "op": "ge", "value": 1}),
            json!({"kind": "json_present", "file": file("evid", "link.json"), "key": "status"}),
            json!({"kind": "grep_count", "pattern": "x", "files": [file("cand", "gone.txt")], "max": 0}),
            json!({"kind": "grep_count", "pattern": "x", "files": [file("cand", "d")], "max": 0}),
        ],
    );
    case.profile["parameters"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "m2", "kind": "manifest", "entries_root": "cand"}));
    // The manifest names `gone.txt`, which exists while the manifest is written.
    write(&case.cand.join("gone.txt"), "soon gone\n");
    case.subject["values"]["m2"] = manifest(
        &case.evid,
        "m2.json",
        "evid",
        &case.cand,
        &["src/a.txt", "gone.txt"],
    );
    case.write_documents();
    let (observed, evaluation) = evaluated(&case, "rule2", || {
        std::fs::remove_file(case.cand.join("gone.txt")).unwrap();
        std::fs::create_dir_all(case.cand.join("d")).unwrap();
        symlink(&case.cand.join("src/a.txt"), &case.cand.join("link.txt"));
        std::fs::create_dir_all(case.evid.join("dirjson")).unwrap();
        symlink(
            &case.evid.join("validation.json"),
            &case.evid.join("link.json"),
        );
    });
    // The missing JSON file makes the battery exit 1 and its `json` check carry `error`;
    // the saved result is still valid and the evaluation succeeds.
    assert_eq!(observed.exit, 1);
    assert_eq!(evaluation["observation"]["battery_exit"], 1);
    let failed = evaluation["battery"]["failed"].as_array().unwrap();
    assert!(
        failed.iter().any(|id| id == "fixture.base.v1.R.5"),
        "{failed:?}"
    );
    let list = predicates(&evaluation);
    let want = [
        "absent",
        "directory",
        "symlink",
        "absent",
        "absent",
        "directory",
        "symlink",
        "absent",
        "directory",
    ];
    for (i, reason) in want.iter().enumerate() {
        expect(&list, i, "unmet", Some(reason));
    }
    // The detail carries the count and the first paths, relative.
    assert!(
        detail(&list, 3).contains("gone.txt"),
        "{}",
        detail(&list, 3)
    );
    assert!(detail(&list, 3).starts_with('1'), "{}", detail(&list, 3));
    assert!(detail(&list, 1).contains('d'), "{}", detail(&list, 1));
    assert_eq!(evaluation["requirements"][0]["status"], "unmet");
}

#[test]
fn rule3_a_content_check_that_errored_on_a_present_file_is_indeterminate_check_error() {
    let mut case = single(
        "eval-rule3",
        vec![
            json!({"kind": "json_equals", "file": file("evid", "bad.json"), "key": "a", "expected": "x"}),
            json!({"kind": "grep_count", "pattern": "(", "files": [file("evid", "logs/core.log")], "min": 1}),
            json!({"kind": "git_output_equals", "args": ["rev-parse", "--verify", "no-such-ref"], "expected": "x"}),
            json!({"kind": "git_output_empty", "args": ["rev-parse", "--verify", "no-such-ref"]}),
            json!({"kind": "git_output_equals", "args": ["show", "HEAD:big.txt"], "expected": "x"}),
        ],
    );
    case.write_documents();
    write(&case.evid.join("bad.json"), "{ not json");
    write(&case.dest.join("big.txt"), &"0123456789".repeat(500));
    git(&case.dest, &["add", "big.txt"]);
    git(&case.dest, &["commit", "--quiet", "-m", "big"]);
    let (_, evaluation) = evaluated(&case, "rule3", || {});
    let list = predicates(&evaluation);
    expect(&list, 0, "indeterminate", Some("check_error"));
    expect(&list, 1, "indeterminate", Some("check_error"));
    expect(&list, 2, "indeterminate", Some("git_nonzero"));
    expect(&list, 3, "indeterminate", Some("git_nonzero"));
    expect(&list, 4, "indeterminate", Some("output_truncated"));
    // The error text is led by the check id and cut to 200 characters.
    assert!(
        detail(&list, 0).starts_with("fixture.base.v1.R.1: "),
        "{}",
        detail(&list, 0)
    );
    assert!(detail(&list, 0).len() <= "fixture.base.v1.R.1: ".len() + 200 + 3);
    assert_eq!(evaluation["requirements"][0]["status"], "indeterminate");
}

#[test]
fn rule5_predicate_outcomes_by_kind() {
    let zero = "0".repeat(64);
    let validation = || file("evid", "validation.json");
    let logs = || json!({"param": "logs"});
    let case = single(
        "eval-rule5",
        vec![
            /* 0 */
            json!({"kind": "file_sha256", "file": file("cand", "src/a.txt"), "expected": sha256_hex(b"alpha\n")}),
            /* 1 */
            json!({"kind": "manifest_match", "manifest": {"param": "source_manifest"}}),
            /* 2 */
            json!({"kind": "json_equals", "file": validation(), "key": "status", "expected": "bad"}),
            /* 3 */
            json!({"kind": "json_equals", "file": validation(), "key": "count", "expected": "3"}),
            /* 4 */
            json!({"kind": "json_equals", "file": validation(), "key": "nokey", "expected": "x"}),
            /* 5 */
            json!({"kind": "json_compare", "file": validation(), "key": "status", "op": "eq", "value": 1}),
            /* 6 */
            json!({"kind": "json_compare", "file": validation(), "key": "ratio", "op": "ge", "value": 0.9}),
            /* 7 */
            json!({"kind": "json_compare", "file": validation(), "key": "count", "op": "eq", "value": 4}),
            /* 8 */ json!({"kind": "json_present", "file": validation(), "key": "nokey"}),
            /* 9 */
            json!({"kind": "grep_count", "pattern": "ok", "files": logs(), "max": 0}),
            /* 10 */
            json!({"kind": "grep_count", "pattern": "3 passed", "files": logs(), "min": 1, "per_path": true}),
            /* 11 */
            json!({"kind": "grep_count", "pattern": "passed", "files": logs(), "min": 1, "per_path": true}),
            /* 12 */
            json!({"kind": "git_output_equals", "args": ["rev-parse", "HEAD"], "expected": zero}),
            /* 13 */ json!({"kind": "git_output_empty", "args": ["status", "--porcelain"]}),
            /* 14 */
            json!({"kind": "json_compare", "file": validation(), "key": "ratio", "op": "le", "value": 0.5}),
            /* 15 */
            json!({"kind": "json_equals", "file": validation(), "key": "count", "expected": 3}),
            /* 16 */
            json!({"kind": "json_compare", "file": validation(), "key": "suites.{each}.counts.failed", "each": {"param": "suites"}, "op": "eq", "value": 1}),
            /* 17 */
            json!({"kind": "grep_count", "pattern": "passed", "files": logs(), "min": 2, "max": 2}),
            /* 18 */
            json!({"kind": "grep_count", "pattern": "ok", "files": logs(), "min": 3}),
        ],
    );
    let (_, evaluation) = evaluated(&case, "rule5", || {
        write(&case.cand.join("src/a.txt"), "changed\n");
        write(&case.cand.join("src/b.txt"), "changed too\n");
        write(&case.dest.join("new.txt"), "untracked\n");
    });
    let list = predicates(&evaluation);
    expect(&list, 0, "unmet", Some("digest_differs"));
    expect(&list, 1, "unmet", Some("digest_differs"));
    assert!(
        detail(&list, 1).contains("src/a.txt"),
        "{}",
        detail(&list, 1)
    );
    assert!(
        detail(&list, 1).contains("src/b.txt"),
        "{}",
        detail(&list, 1)
    );
    expect(&list, 2, "unmet", Some("differs"));
    expect(&list, 3, "unmet", Some("type_mismatch"));
    expect(&list, 4, "unmet", Some("null_or_missing"));
    expect(&list, 5, "unmet", Some("type_mismatch"));
    expect(&list, 6, "unmet", Some("out_of_bound"));
    expect(&list, 7, "unmet", Some("differs"));
    expect(&list, 8, "unmet", Some("null_or_missing"));
    expect(&list, 9, "unmet", Some("count_out_of_bound"));
    expect(&list, 10, "unmet", Some("count_out_of_bound"));
    assert!(
        detail(&list, 10).contains("logs/ui.log"),
        "{}",
        detail(&list, 10)
    );
    expect(&list, 11, "met", None);
    expect(&list, 12, "unmet", Some("differs"));
    expect(&list, 13, "unmet", Some("not_empty"));
    expect(&list, 14, "met", None);
    expect(&list, 15, "met", None);
    expect(&list, 16, "unmet", Some("differs"));
    // Whole-set bounds: two files each count 1 `passed` line, so the set count is 2.
    expect(&list, 17, "met", None);
    expect(&list, 18, "unmet", Some("count_out_of_bound"));
    assert_eq!(evaluation["requirements"][0]["status"], "unmet");
    let c = &evaluation["counts"];
    assert_eq!(
        (
            c["met"].as_u64(),
            c["unmet"].as_u64(),
            c["indeterminate"].as_u64()
        ),
        (Some(0), Some(1), Some(0))
    );
}

#[test]
fn rule5_a_manifest_file_that_changed_during_the_battery_is_indeterminate_manifest_changed() {
    let case = single(
        "eval-manifest-changed",
        vec![json!({"kind": "manifest_match", "manifest": {"param": "source_manifest"}})],
    );
    let manifest_file = case.evid.join("source-manifest.json");
    let original = std::fs::read(&manifest_file).unwrap();
    // Entries also differ, to show `manifest_changed` is decided before `digest_differs`.
    let (_, evaluation) = evaluated_with(
        &case,
        "mchanged",
        || {
            write(&case.cand.join("src/a.txt"), "changed\n");
            // The battery sees a changed manifest file; it is restored before `evaluate`.
            let mut changed = original.clone();
            changed.extend_from_slice(b" \n");
            std::fs::write(&manifest_file, changed).unwrap();
        },
        || std::fs::write(&manifest_file, &original).unwrap(),
    );
    let list = predicates(&evaluation);
    expect(&list, 0, "indeterminate", Some("manifest_changed"));
}

#[test]
fn rule4_a_json_digest_that_differs_from_its_exists_entry_is_observation_inconsistent() {
    let case = single(
        "eval-rule4",
        vec![
            json!({"kind": "json_equals", "file": file("evid", "validation.json"), "key": "status", "expected": "ok"}),
            json!({"kind": "json_present", "file": file("evid", "validation.json"), "key": "status"}),
        ],
    );
    let observed = observe(&case, "rule4");
    // Simulate a file that changed between the `json` and `.exists` checks: edit the saved result
    // and re-record its digest in the observation.
    let mut result: Value =
        serde_json::from_slice(&std::fs::read(&observed.result).unwrap()).unwrap();
    result["checks"]["fixture.base.v1.R.1"]["sha256"] = json!("f".repeat(64));
    std::fs::remove_file(&observed.result).unwrap();
    write_json(&observed.result, &result);
    write_observation_again(&observed);
    let run = case.evaluate(&observed, &[]);
    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    let evaluation = run.out_json();
    no_verdict_anywhere(&evaluation);
    let list = predicates(&evaluation);
    expect(&list, 0, "indeterminate", Some("observation_inconsistent"));
    // Only the first check was altered.
    expect(&list, 1, "met", None);
}

/// Re-record the plan and result digests of `observed` in its observation file.
fn write_observation_again(observed: &Observed) {
    std::fs::remove_file(&observed.observation).unwrap();
    write_observation(
        &observed.observation,
        &observed.plan,
        &observed.result,
        observed.exit,
        &observed.stdout_battery_sha256,
    );
}

fn git_and_file_requirements() -> Vec<Value> {
    vec![
        json!({"id": "R-file", "class": "mechanical", "statement": "a file",
               "predicates": [{"kind": "file_sha256", "file": file("cand", "src/a.txt"), "expected": sha256_hex(b"alpha\n")}]}),
        json!({"id": "R-git", "class": "mechanical", "statement": "git",
               "predicates": [
                   {"kind": "git_output_equals", "args": ["rev-parse", "HEAD"], "expected": {"param": "head"}},
                   {"kind": "git_output_empty", "args": ["status", "--porcelain"]}]}),
    ]
}

fn requirement<'a>(evaluation: &'a Value, id: &str) -> &'a Value {
    evaluation["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("no requirement {id}"))
}

#[test]
fn a_git_root_the_battery_does_not_see_as_the_subjects_makes_git_predicates_git_root_mismatch() {
    // The control: the declared git root is the repository's top level.
    let case = with_requirements("eval-gitroot-ok", git_and_file_requirements());
    let (_, ok) = evaluated(&case, "ok", || {});
    assert_eq!(ok["binding"]["git_root"], "verified");
    assert_eq!(requirement(&ok, "R-git")["status"], "met");

    // (a) The subject's git root is a subdirectory: `git rev-parse --show-toplevel` prints the
    // parent, which differs from the subject's path.
    let mut case = with_requirements("eval-gitroot-sub", git_and_file_requirements());
    let sub = case.dest.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    case.subject["roots"]["dest"] = json!(s(&sub));
    case.write_documents();
    let (_, evaluation) = evaluated(&case, "sub", || {});
    assert_eq!(evaluation["binding"]["git_root"], "mismatch");
    let git = requirement(&evaluation, "R-git");
    assert_eq!(git["status"], "indeterminate");
    for p in git["predicates"].as_array().unwrap() {
        assert_eq!(p["status"], "indeterminate", "{p}");
        assert_eq!(p["reason"], "git_root_mismatch", "{p}");
        let d = p["detail"].as_str().unwrap();
        assert!(d.contains(&s(&case.dest)) && d.contains(&s(&sub)), "{d}");
    }
    // A requirement without a git predicate is unaffected.
    assert_eq!(requirement(&evaluation, "R-file")["status"], "met");

    // (b) The subject's git root is a directory that is not in a git work tree (the fixtures
    // live outside any repository): the probe exits nonzero.
    let mut case = with_requirements("eval-gitroot-plain", git_and_file_requirements());
    let plain = case.dir.join("plain");
    std::fs::create_dir_all(&plain).unwrap();
    case.subject["roots"]["dest"] = json!(s(&plain));
    case.write_documents();
    let (_, evaluation) = evaluated(&case, "plain", || {});
    assert_eq!(evaluation["binding"]["git_root"], "mismatch");
    for p in requirement(&evaluation, "R-git")["predicates"]
        .as_array()
        .unwrap()
    {
        assert_eq!(p["reason"], "git_root_mismatch", "{p}");
    }
    assert_eq!(requirement(&evaluation, "R-file")["status"], "met");
}

#[test]
fn a_profile_without_a_git_root_records_not_applicable() {
    let mut case = single(
        "eval-no-git",
        vec![
            json!({"kind": "file_sha256", "file": file("cand", "src/a.txt"), "expected": sha256_hex(b"alpha\n")}),
        ],
    );
    for root in case.profile["roots"].as_array_mut().unwrap() {
        root.as_object_mut().unwrap().remove("git");
    }
    case.write_documents();
    let (_, evaluation) = evaluated(&case, "nogit", || {});
    assert_eq!(evaluation["binding"]["git_root"], "not_applicable");
    assert_eq!(evaluation["requirements"][0]["status"], "met");
}

#[test]
fn an_attributed_requirement_is_always_indeterminate_with_deduplicated_evidence() {
    let mut requirements = git_and_file_requirements();
    requirements[0]["predicates"][0]["expected"] = json!("0".repeat(64)); // R-file is unmet
    requirements.push(
        json!({"id": "A-one", "class": "attributed", "statement": "j1",
        "evidence_from": ["R-file", "R-git"]}),
    );
    requirements.push(
        json!({"id": "A-two", "class": "attributed", "statement": "j2",
        "evidence_from": ["R-git", "R-file"]}),
    );
    let case = with_requirements("eval-attributed", requirements);
    let (_, evaluation) = evaluated(&case, "attr", || {});
    assert_eq!(requirement(&evaluation, "R-file")["status"], "unmet");
    for id in ["A-one", "A-two"] {
        let a = requirement(&evaluation, id);
        assert_eq!(a["status"], "indeterminate");
        assert_eq!(a["reason"], "attributed_judgment_required");
        assert!(a.get("predicates").is_none(), "{a}");
        let ids: Vec<&str> = a["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["check_id"].as_str().unwrap())
            .collect();
        let unique: std::collections::BTreeSet<&&str> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "duplicate evidence in {ids:?}");
        assert!(ids.contains(&"fixture.base.v1.R-file.1"), "{ids:?}");
        assert!(ids.contains(&"fixture.base.v1.R-git.1"), "{ids:?}");
        assert!(ids.contains(&"fixture.base.v1.R-git.2"), "{ids:?}");
    }
    assert_eq!(evaluation["counts"]["indeterminate"], 2);
}

#[test]
fn out_writes_a_file_once_and_a_refusal_writes_nothing() {
    let case = base_case("eval-out");
    let observed = observe(&case, "out");
    let out = case.dir.join("evaluation.json");

    let first = case.evaluate(&observed, &["--out", &s(&out)]);
    assert_eq!(first.code, 0, "stderr: {}", first.stderr);
    assert!(first.stdout.is_empty(), "stdout: {}", first.stdout);
    let written: Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert_eq!(written["schema"], "decision-readiness-evaluation-v1");
    no_verdict_anywhere(&written);
    // The same evaluation, byte for byte, on stdout.
    let again = case.evaluate(&observed, &[]);
    assert_eq!(again.stdout, std::fs::read_to_string(&out).unwrap());

    // An existing `--out` is never overwritten.
    let before = std::fs::read(&out).unwrap();
    let second = case.evaluate(&observed, &["--out", &s(&out)]);
    assert_ne!(second.code, 0);
    assert!(second.stdout.is_empty());
    assert_eq!(std::fs::read(&out).unwrap(), before);

    // A refusal creates no `--out` and writes nothing to stdout.
    let refused_out = case.dir.join("refused.json");
    write(&case.subject_path(), "{}");
    let refused = case.evaluate(&observed, &["--out", &s(&refused_out)]);
    assert_ne!(refused.code, 0);
    assert!(refused.stdout.is_empty());
    assert!(!refused_out.exists());
}

#[test]
fn a_malformed_evaluated_at_and_an_invalid_profile_exit_2() {
    let case = base_case("eval-exit2");
    let observed = observe(&case, "e2");
    let mut args = vec![
        "evaluate".to_string(),
        "--profile".into(),
        s(&case.profile_path()),
        "--subject".into(),
        s(&case.subject_path()),
        "--run-plan".into(),
        s(&observed.plan),
        "--battery".into(),
        s(&observed.result),
        "--observation".into(),
        s(&observed.observation),
        "--evaluated-at".into(),
        "yesterday".into(),
    ];
    let run = cli(&args);
    assert_eq!(run.code, 2, "stderr: {}", run.stderr);
    assert!(run.stdout.is_empty());

    // An invalid profile (its schema string is wrong) is a validation refusal, exit 2.
    args[12] = EVALUATED_AT.to_string();
    let mut broken = case.profile.clone();
    broken["schema"] = json!("not-a-profile");
    let broken_path = case.dir.join("broken-profile.json");
    write_json(&broken_path, &broken);
    args[2] = s(&broken_path);
    let run = cli(&args);
    assert_eq!(run.code, 2, "stderr: {}", run.stderr);
    assert!(run.stdout.is_empty());
}
