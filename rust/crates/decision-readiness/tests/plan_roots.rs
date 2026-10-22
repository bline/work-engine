//! Plan-time rules for roots (S1-T1-001, contract 3.4 rules 2 and 5, section 4) and for check ids
//! and grep bounds (errata E1, E2).

mod support;
use serde_json::{Value, json};
use std::path::PathBuf;
use support::*;

/// One profile (`id`, version 1) declaring `roots` (name, is_git) in order, with a requirement
/// whose single predicate is `predicate`; and its subject, mapping each root name to `paths`.
fn pair(
    case: &Case,
    id: &str,
    roots: &[(&str, bool)],
    paths: &[(&str, &PathBuf)],
    requirement_id: &str,
    predicate: Value,
) -> (String, String) {
    let declared: Vec<Value> = roots
        .iter()
        .map(|(name, git)| {
            if *git {
                json!({"name": name, "git": true})
            } else {
                json!({"name": name})
            }
        })
        .collect();
    let profile = json!({
        "schema": "decision-readiness-profile-v1",
        "profile_id": id, "version": 1,
        "decision": {"id": "d", "owner_role": "coordinator"},
        "profile_owner": "coordinator", "assessor_roles": ["reviewer"],
        "authority_refs": [{"path": "n.md", "lines": "1"}],
        "roots": declared,
        "parameters": [],
        "requirements": [{"id": requirement_id, "class": "mechanical", "statement": "s",
            "predicates": [predicate]}]
    });
    let profile_path = case.dir.join(format!("{id}-profile.json"));
    let sha = write_json(&profile_path, &profile);
    let mapped: serde_json::Map<String, Value> = paths
        .iter()
        .map(|(name, path)| (name.to_string(), json!(s(path))))
        .collect();
    let subject = json!({
        "schema": "decision-readiness-subject-v1",
        "profile": {"id": id, "version": 1, "sha256": sha},
        "subject_id": "s", "author": "tests", "roots": mapped, "values": {}
    });
    let subject_path = case.dir.join(format!("{id}-subject.json"));
    write_json(&subject_path, &subject);
    (s(&profile_path), s(&subject_path))
}

fn a_file(case: &Case, root: &str) -> Value {
    json!({"kind": "file_sha256", "file": {"root": root, "path": "src/a.txt"},
           "expected": file_sha(&case.cand.join("src/a.txt"))})
}

fn plan_of(pairs: &[&(String, String)]) -> Run {
    let mut args = vec!["plan".to_string()];
    for (profile, subject) in pairs {
        args.extend(["--profile".into(), profile.clone()]);
        args.extend(["--subject".into(), subject.clone()]);
    }
    cli(&args)
}

#[test]
fn a_git_root_declared_second_in_one_profile_is_the_plan_root() {
    let case = base_case("roots-second");
    let p = pair(
        &case,
        "r.one",
        &[("cand", false), ("dest", true)],
        &[("cand", &case.cand), ("dest", &case.dest)],
        "R",
        a_file(&case, "cand"),
    );
    let run = plan_of(&[&p]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let plan = run.out_json();
    assert_eq!(plan["root"], s(&case.dest));
    assert_eq!(plan["extra_roots"], json!([s(&case.cand)]));
}

#[test]
fn a_non_git_first_pair_does_not_decide_the_root_when_a_later_pair_has_a_git_root() {
    let case = base_case("roots-nongit-first");
    let first = pair(
        &case,
        "r.first",
        &[("cand", false), ("evid", false)],
        &[("cand", &case.cand), ("evid", &case.evid)],
        "R",
        a_file(&case, "cand"),
    );
    let second = pair(
        &case,
        "r.second",
        &[("dest", true), ("cand", false)],
        &[("dest", &case.dest), ("cand", &case.cand)],
        "R",
        a_file(&case, "cand"),
    );
    let run = plan_of(&[&first, &second]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let plan = run.out_json();
    assert_eq!(plan["root"], s(&case.dest));
    // First-appearance order across the pairs, the root itself excluded.
    assert_eq!(plan["extra_roots"], json!([s(&case.cand), s(&case.evid)]));
    // The git root appears in the probe check only when some profile declares one.
    let ids: Vec<&str> = plan["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"r.second.v1._git_root"), "{ids:?}");
    assert!(!ids.contains(&"r.first.v1._git_root"), "{ids:?}");
}

#[test]
fn with_no_git_root_anywhere_the_root_is_the_first_declared_root_of_the_first_pair() {
    let case = base_case("roots-nogit");
    let first = pair(
        &case,
        "r.first",
        &[("evid", false), ("cand", false)],
        &[("evid", &case.evid), ("cand", &case.cand)],
        "R",
        a_file(&case, "cand"),
    );
    let second = pair(
        &case,
        "r.second",
        &[("bins", false), ("cand", false)],
        &[("bins", &case.bins), ("cand", &case.cand)],
        "R",
        a_file(&case, "cand"),
    );
    let run = plan_of(&[&first, &second]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let plan = run.out_json();
    assert_eq!(plan["root"], s(&case.evid));
    assert_eq!(plan["extra_roots"], json!([s(&case.cand), s(&case.bins)]));
}

#[test]
fn git_roots_under_different_names_and_paths_are_multiple_git_root_paths_only() {
    let case = base_case("roots-multigit");
    let first = pair(
        &case,
        "r.first",
        &[("dest", true), ("cand", false)],
        &[("dest", &case.dest), ("cand", &case.cand)],
        "R",
        a_file(&case, "cand"),
    );
    // A different name (`repo2`) at a different path: no root name maps to two paths, so the only
    // fault is the second git-root path.
    let second = pair(
        &case,
        "r.second",
        &[("repo2", true), ("cand", false)],
        &[("repo2", &case.evid), ("cand", &case.cand)],
        "R",
        a_file(&case, "cand"),
    );
    let run = plan_of(&[&first, &second]);
    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(run.stdout.is_empty());
    assert_eq!(run.codes(), vec!["multiple_git_root_paths".to_string()]);

    // A non-git name at two paths is a name conflict, and only that.
    let renamed = pair(
        &case,
        "r.third",
        &[("dest", true), ("cand", false)],
        &[("dest", &case.dest), ("cand", &case.evid)],
        "R",
        a_file(&case, "cand"),
    );
    let run = plan_of(&[&first, &renamed]);
    assert_eq!(run.code, 2);
    assert_eq!(run.codes(), vec!["root_path_conflict".to_string()]);

    // The git-root name at two paths is both faults: the name conflicts and two distinct
    // git-root paths appear (contract section 4: at most one distinct git-root path).
    let both = pair(
        &case,
        "r.fifth",
        &[("dest", true), ("cand", false)],
        &[("dest", &case.evid), ("cand", &case.cand)],
        "R",
        a_file(&case, "cand"),
    );
    let run = plan_of(&[&first, &both]);
    assert_eq!(run.code, 2);
    let mut codes = run.codes();
    codes.sort();
    assert_eq!(codes, ["multiple_git_root_paths", "root_path_conflict"]);

    // Two names for the same git-root path are accepted.
    let alias = pair(
        &case,
        "r.fourth",
        &[("repo2", true), ("cand", false)],
        &[("repo2", &case.dest), ("cand", &case.cand)],
        "R",
        a_file(&case, "cand"),
    );
    let run = plan_of(&[&first, &alias]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.out_json()["root"], s(&case.dest));
}

#[test]
fn a_check_id_produced_twice_is_duplicate_check_id() {
    // E1, the review's example: profile `x` requirement `v1` with a per-path grep makes
    // `x.v1.v1.1.1`; profile `x.v1` requirement `1` with predicate 1 makes the same id.
    let case = base_case("ids-duplicate");
    let grep = json!({"kind": "grep_count", "pattern": "alpha",
        "files": [{"root": "cand", "path": "src/a.txt"}], "min": 1, "per_path": true});
    let first = pair(
        &case,
        "x",
        &[("cand", false)],
        &[("cand", &case.cand)],
        "v1",
        grep,
    );
    let second = pair(
        &case,
        "x.v1",
        &[("cand", false)],
        &[("cand", &case.cand)],
        "1",
        a_file(&case, "cand"),
    );
    let run = plan_of(&[&first, &second]);
    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(run.stdout.is_empty());
    assert_eq!(run.codes(), vec!["duplicate_check_id".to_string()]);
    assert!(run.stderr.contains("x.v1.v1.1.1"), "{}", run.stderr);

    // Each profile alone plans, so the fault is the collision and nothing else.
    assert_eq!(plan_of(&[&first]).code, 0);
    assert_eq!(plan_of(&[&second]).code, 0);
}

fn grep_profile(name: &str, bounds: Value) -> Case {
    let mut case = base_case(name);
    let mut predicate = json!({"kind": "grep_count", "pattern": "alpha",
        "files": [{"root": "cand", "path": "src/a.txt"}]});
    for (key, value) in bounds.as_object().unwrap() {
        predicate[key] = value.clone();
    }
    case.profile["requirements"] = json!([{"id": "R", "class": "mechanical", "statement": "s",
        "predicates": [predicate]}]);
    case.write_documents();
    case
}

#[test]
fn a_grep_count_without_a_bound_or_with_min_above_max_is_refused() {
    // E2. `check --kind profile` reports (exit 1), `plan` refuses (exit 2).
    for (bounds, code) in [
        (json!({}), "grep_count_unbounded"),
        (json!({"per_path": true}), "grep_count_unbounded"),
        (json!({"min": 3, "max": 2}), "grep_count_min_gt_max"),
    ] {
        let case = grep_profile("grep-bounds-bad", bounds.clone());
        let checked = case.check_profile();
        assert_eq!(checked.code, 1, "{bounds}: {}", checked.stdout);
        assert_eq!(checked.codes(), vec![code.to_string()], "{bounds}");
        let at = checked.out_json()["errors"][0]["at"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(at.contains("predicates"), "at: {at}");
        let planned = case.plan();
        assert_eq!(planned.code, 2, "{bounds}");
        assert!(planned.stdout.is_empty());
        assert_eq!(planned.codes(), vec![code.to_string()], "{bounds}");
    }
}

#[test]
fn grep_count_bounds_that_are_satisfiable_are_accepted() {
    for bounds in [
        json!({"min": 1}),
        json!({"max": 0}),
        json!({"min": 2, "max": 2}),
        json!({"min": 0, "max": 5, "per_path": true}),
    ] {
        let case = grep_profile("grep-bounds-ok", bounds.clone());
        assert_eq!(case.check_profile().code, 0, "{bounds}");
        let planned = case.plan();
        assert_eq!(planned.code, 0, "{bounds}: {}", planned.stderr);
    }
}
