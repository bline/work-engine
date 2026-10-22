//! `plan`: determinism (AC2), positive fixture trees run under the unmodified evidence
//! battery (one plan per predicate kind, a two-profile plan, the frozen C1 profile), and the
//! binary's exit codes.

mod support;

use serde_json::{Value, json};
use support::*;

fn check_ids(plan: &Value) -> Vec<String> {
    plan["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap().to_string())
        .collect()
}

fn check_of<'a>(plan: &'a Value, id: &str) -> &'a Value {
    plan["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("no check {id}"))
}

#[test]
fn single_profile_plan_is_byte_stable_and_has_the_pinned_shape() {
    let case = base_case("plan-single");
    let first = case.plan();
    let second = case.plan();
    assert_eq!((first.code, second.code), (0, 0), "{}", first.stderr);
    assert_eq!(first.stdout, second.stdout, "two runs differ");
    assert!(first.stderr.is_empty());
    let text = &first.stdout;
    assert!(
        text.ends_with("}\n") && !text.ends_with("\n\n"),
        "one trailing newline"
    );
    assert!(!text.starts_with('\u{feff}'), "no BOM");
    assert!(text.contains("\n  \"root\":"), "2-space indent");
    // Output holds exactly root, extra_roots, checks, in that order.
    let root_at = text.find("\"root\"").unwrap();
    let extra_at = text.find("\"extra_roots\"").unwrap();
    let checks_at = text.find("\"checks\"").unwrap();
    assert!(root_at < extra_at && extra_at < checks_at);
    let plan = first.out_json();
    assert_eq!(plan.as_object().unwrap().len(), 3);
    // The plan's root is the git root; extra roots follow profile root order.
    assert_eq!(plan["root"], s(&case.dest));
    assert_eq!(
        plan["extra_roots"],
        json!([s(&case.cand), s(&case.evid), s(&case.bins)])
    );
    // A plan holds strings, arrays and objects only, never a number (contract 2.3).
    fn no_numbers(v: &Value) {
        match v {
            Value::Number(n) => panic!("number {n} in a plan"),
            Value::Array(a) => a.iter().for_each(no_numbers),
            Value::Object(o) => o.values().for_each(no_numbers),
            _ => {}
        }
    }
    no_numbers(&plan);
}

#[test]
fn plan_derives_the_contract_check_ids_and_shapes() {
    let case = base_case("plan-ids");
    let plan = case.plan().out_json();
    let ids = check_ids(&plan);
    let p = "fixture.base.v1";
    assert_eq!(ids[0], format!("{p}._git_root"));
    assert_eq!(
        check_of(&plan, &ids[0])["git"],
        json!(["rev-parse", "--show-toplevel"])
    );
    // file_sha256: one sha256 over the absolute path.
    assert_eq!(
        check_of(&plan, &format!("{p}.R-file.1"))["sha256"],
        json!([format!("{}/src/a.txt", s(&case.cand))])
    );
    // manifest_match: every entry in manifest order, then .manifest over the manifest file.
    assert_eq!(
        check_of(&plan, &format!("{p}.R-manifest.1"))["sha256"],
        json!([
            format!("{}/src/a.txt", s(&case.cand)),
            format!("{}/src/b.txt", s(&case.cand))
        ])
    );
    assert_eq!(
        check_of(&plan, &format!("{p}.R-manifest.1.manifest"))["sha256"],
        json!([format!("{}/source-manifest.json", s(&case.evid))])
    );
    // json_*: select one key, or one per `each` element; .exists beside it.
    assert_eq!(
        check_of(&plan, &format!("{p}.R-compare.1"))["json"],
        json!({"path": format!("{}/validation.json", s(&case.evid)),
               "select": ["suites.core.exit_code", "suites.ui.exit_code"]})
    );
    assert_eq!(
        check_of(&plan, &format!("{p}.R-compare.1.exists"))["sha256"],
        json!([format!("{}/validation.json", s(&case.evid))])
    );
    // grep_count: whole set is one check; per_path gives <i>.<j>; each has .exists.
    let whole = check_of(&plan, &format!("{p}.R-grep.1"));
    assert_eq!(whole["grep"]["paths"].as_array().unwrap().len(), 2);
    assert_eq!(
        check_of(&plan, &format!("{p}.R-grep.2.1"))["grep"]["paths"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        check_of(&plan, &format!("{p}.R-grep.2.2"))["grep"]["paths"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(ids.contains(&format!("{p}.R-grep.2.exists")));
    // git: list parameters expand in place.
    assert_eq!(
        check_of(&plan, &format!("{p}.R-git.2"))["git"],
        json!(["status", "--porcelain", "--", "tracked.txt", "other.txt"])
    );
    // Attributed requirements derive no check.
    assert!(!ids.iter().any(|id| id.contains("A-note")));
    // Ids are unique and match the battery's identity pattern.
    let unique: std::collections::BTreeSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len());
    for id in &ids {
        assert!(
            id.chars().next().unwrap().is_ascii_alphanumeric()
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)),
            "{id}"
        );
    }
}

fn second_profile_and_subject(case: &Case) -> (std::path::PathBuf, std::path::PathBuf) {
    let profile = json!({
        "schema": "decision-readiness-profile-v1",
        "profile_id": "fixture.second",
        "version": 2,
        "decision": {"id": "second-decision", "owner_role": "coordinator"},
        "profile_owner": "coordinator",
        "assessor_roles": ["reviewer"],
        "authority_refs": [{"path": "notes/second.md", "lines": "3"}],
        "roots": [{"name": "dest", "git": true}, {"name": "cand"}],
        "parameters": [{"name": "head", "kind": "string"}],
        "requirements": [
            {"id": "S-file", "class": "mechanical", "statement": "b.txt",
             "predicates": [{"kind": "file_sha256", "file": {"root": "cand", "path": "src/b.txt"},
                             "expected": file_sha(&case.cand.join("src/b.txt"))}]},
            {"id": "S-head", "class": "mechanical", "statement": "head",
             "predicates": [{"kind": "git_output_equals", "args": ["rev-parse", "HEAD"], "expected": {"param": "head"}}]}
        ]
    });
    let profile_path = case.dir.join("second-profile.json");
    let sha = write_json(&profile_path, &profile);
    let subject = json!({
        "schema": "decision-readiness-subject-v1",
        "profile": {"id": "fixture.second", "version": 2, "sha256": sha},
        "subject_id": "second-subject", "author": "tests",
        "roots": {"dest": s(&case.dest), "cand": s(&case.cand)},
        "values": {"head": case.subject["values"]["head"].clone()}
    });
    let subject_path = case.dir.join("second-subject.json");
    write_json(&subject_path, &subject);
    (profile_path, subject_path)
}

fn two_profile_plan(case: &Case) -> Run {
    let (p2, s2) = second_profile_and_subject(case);
    cli(&[
        "plan",
        "--profile",
        &s(&case.profile_path()),
        "--subject",
        &s(&case.subject_path()),
        "--profile",
        &s(&p2),
        "--subject",
        &s(&s2),
    ])
}

#[test]
fn two_profile_plan_is_byte_stable_and_concatenates_in_argument_order() {
    let case = base_case("plan-two");
    let first = two_profile_plan(&case);
    let second = two_profile_plan(&case);
    assert_eq!((first.code, second.code), (0, 0), "{}", first.stderr);
    assert_eq!(first.stdout, second.stdout);
    let plan = first.out_json();
    let ids = check_ids(&plan);
    let single = check_ids(&case.plan().out_json());
    assert_eq!(
        &ids[..single.len()],
        &single[..],
        "first pair's checks come first"
    );
    assert_eq!(ids[single.len()], "fixture.second.v2._git_root");
    assert!(ids.contains(&"fixture.second.v2.S-file.1".to_string()));
    assert_eq!(plan["root"], s(&case.dest));
    // Distinct non-git root paths in order of first appearance, with no repeats.
    assert_eq!(
        plan["extra_roots"],
        json!([s(&case.cand), s(&case.evid), s(&case.bins)])
    );
    // Reversed argument order changes the order of checks, not the set.
    let (p2, s2) = second_profile_and_subject(&case);
    let reversed = cli(&[
        "plan",
        "--profile",
        &s(&p2),
        "--subject",
        &s(&s2),
        "--profile",
        &s(&case.profile_path()),
        "--subject",
        &s(&case.subject_path()),
    ]);
    assert_eq!(reversed.code, 0);
    let mut a = ids.clone();
    let mut b = check_ids(&reversed.out_json());
    assert_ne!(a, b);
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn permuted_key_profile_has_a_different_digest_and_is_still_valid() {
    let mut case = base_case("plan-permuted");
    let original_sha = file_sha(&case.profile_path());
    let original_plan = case.plan().stdout;
    // Same content, keys in reverse order, written to a second file.
    let permuted_path = case.dir.join("profile-permuted.json");
    write(&permuted_path, &permuted_text(&case.profile));
    let permuted_sha = file_sha(&permuted_path);
    assert_ne!(original_sha, permuted_sha, "key order is part of identity");
    let report = cli(&["check", "--kind", "profile", &s(&permuted_path)]);
    assert_eq!(report.code, 0, "{}", report.stdout);
    assert_eq!(report.out_json()["valid"], true);
    // A subject that records the permuted digest is valid and derives the same plan bytes.
    case.subject["profile"]["sha256"] = json!(permuted_sha);
    let subject_path = case.dir.join("subject-permuted.json");
    write_json(&subject_path, &case.subject);
    let plan = cli(&[
        "plan",
        "--profile",
        &s(&permuted_path),
        "--subject",
        &s(&subject_path),
    ]);
    assert_eq!(plan.code, 0, "{}", plan.stderr);
    assert_eq!(
        plan.stdout, original_plan,
        "plan does not depend on key order"
    );
    // The subject bound to the original digest does not bind the permuted profile.
    let mismatch = cli(&[
        "plan",
        "--profile",
        &s(&permuted_path),
        "--subject",
        &s(&case.subject_path()),
    ]);
    assert_eq!(mismatch.code, 2);
    assert!(
        mismatch
            .codes()
            .iter()
            .any(|c| c == "profile_digest_mismatch")
    );
}

/// Run `plan` output under the battery and require `failed` empty and every path entry a file.
fn battery_clean(case: &Case, plan_text: &str, tag: &str) -> Value {
    let plan_path = case.dir.join(format!("{tag}-plan.json"));
    write(&plan_path, plan_text);
    let result_path = case.dir.join(format!("{tag}-result.json"));
    let (code, result) = run_battery(&plan_path, &result_path);
    assert_eq!(
        result["failed"],
        json!([]),
        "failed checks in {tag}: {result}"
    );
    assert_eq!(code, 0);
    let expected_count = serde_json::from_str::<Value>(plan_text).unwrap()["checks"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(
        result["check_count"].as_u64().unwrap() as usize,
        expected_count
    );
    for (id, check) in result["checks"].as_object().unwrap() {
        if let Some(entries) = check.get("entries").and_then(Value::as_array) {
            for entry in entries {
                assert_eq!(entry["type"], "file", "{id}: {entry}");
            }
        }
    }
    result
}

#[test]
fn every_predicate_kind_plan_runs_under_the_unmodified_battery() {
    let case = base_case("battery-base");
    let run = case.plan();
    assert_eq!(run.code, 0, "{}", run.stderr);
    // The profile covers all eight predicate kinds.
    let kinds: std::collections::BTreeSet<String> = case.profile["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["predicates"].as_array().cloned().unwrap_or_default())
        .map(|p| p["kind"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(kinds.len(), 8, "{kinds:?}");
    let result = battery_clean(&case, &run.stdout, "base");
    let p = "fixture.base.v1";
    let checks = &result["checks"];
    // The git probe observes the subject's git-root path exactly.
    assert_eq!(
        checks[format!("{p}._git_root")]["text"]
            .as_str()
            .unwrap()
            .trim(),
        s(&case.dest)
    );
    assert_eq!(checks[format!("{p}._git_root")]["returncode"], 0);
    // The battery observed what the plan asked about: manifest entries equal the digests the
    // manifest records, grep counts and the selected JSON values are as the fixture wrote them.
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(case.evid.join("source-manifest.json")).unwrap())
            .unwrap();
    let entries = checks[format!("{p}.R-manifest.1")]["entries"]
        .as_array()
        .unwrap();
    assert_eq!(entries.len(), 2);
    for (entry, recorded) in entries.iter().zip(manifest.as_array().unwrap()) {
        assert_eq!(entry["sha256"], recorded["sha256"]);
    }
    assert_eq!(checks[format!("{p}.R-grep.1")]["match_count"], 0);
    assert_eq!(checks[format!("{p}.R-grep.2.1")]["match_count"], 1);
    assert_eq!(checks[format!("{p}.R-grep.2.2")]["match_count"], 1);
    let selected = &checks[format!("{p}.R-compare.1")]["selected"];
    assert_eq!(selected["suites.core.exit_code"], 0);
    assert_eq!(selected["suites.ui.exit_code"], 0);
    assert_eq!(
        checks[format!("{p}.R-git.1")]["text"]
            .as_str()
            .unwrap()
            .trim(),
        case.subject["values"]["head"].as_str().unwrap()
    );
    assert_eq!(checks[format!("{p}.R-git.2")]["byte_count"], 0);
}

#[test]
fn file_sha256_with_a_param_expected_runs_under_the_battery() {
    // A custom profile that includes `file_sha256` with a `{param}` expected, run under the battery.
    let mut case = base_case("battery-file-sha");
    case.profile["requirements"][0]["predicates"][0]["expected"] = json!({"param": "head"});
    case.write_documents();
    let run = case.plan();
    assert_eq!(run.code, 0, "{}", run.stderr);
    let result = battery_clean(&case, &run.stdout, "param");
    let digest = &result["checks"]["fixture.base.v1.R-file.1"]["entries"][0]["sha256"];
    assert_eq!(
        digest.as_str().unwrap(),
        file_sha(&case.cand.join("src/a.txt"))
    );
}

#[test]
fn two_profile_plan_runs_under_the_unmodified_battery() {
    let case = base_case("battery-two");
    let run = two_profile_plan(&case);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let result = battery_clean(&case, &run.stdout, "two");
    assert!(result["checks"].get("fixture.second.v2.S-file.1").is_some());
    assert!(
        result["checks"]
            .get("fixture.second.v2._git_root")
            .is_some()
    );
}

fn c1_case(name: &str) -> Case {
    // The frozen C1 profile's own fixture copy, with a subject and tree shaped as C1 describes.
    let dir = fresh_dir(name);
    let (dest, cand, evid, bins) = (
        dir.join("dest"),
        dir.join("cand"),
        dir.join("evid"),
        dir.join("bins"),
    );
    for d in [&dest, &cand, &evid, &bins] {
        std::fs::create_dir_all(d).unwrap();
    }
    git(&dest, &["init", "--quiet"]);
    write(&dest.join("affected-a.txt"), "a\n");
    write(&dest.join("affected-b.txt"), "b\n");
    git(&dest, &["add", "."]);
    git(&dest, &["commit", "--quiet", "-m", "baseline"]);
    let head = git(&dest, &["rev-parse", "HEAD"]);
    write(&cand.join("crate/src/lib.rs"), "pub fn f() {}\n");
    write(&bins.join("bin/work-engine-readiness"), "binary\n");
    write(&evid.join("authority/authority.md"), "authority\n");
    write_json(
        &evid.join("validation-v1.json"),
        &json!({"supervisor_final_tests": {"decision_readiness": {"exit_code": 0, "counts": {"failed": 0}}}}),
    );
    write_json(
        &evid.join("review-receipt.json"),
        &json!({"review_conclusion": "accept", "integration_decision": "not_made_root_owned"}),
    );
    write(
        &evid.join("logs/cargo-test-decision_readiness.log"),
        "test result: ok. 4 passed; 0 failed\n",
    );
    let authority_manifest = manifest(
        &evid,
        "authority-manifest.json",
        "evidence",
        &evid,
        &["authority/authority.md"],
    );
    let source_manifest = manifest(
        &evid,
        "source-manifest.json",
        "evidence",
        &cand,
        &["crate/src/lib.rs"],
    );
    let binary_manifest = manifest(
        &evid,
        "binary-manifest.json",
        "evidence",
        &bins,
        &["bin/work-engine-readiness"],
    );
    let evidence_manifest = manifest(
        &evid,
        "evidence-manifest.json",
        "evidence",
        &evid,
        &[
            "validation-v1.json",
            "review-receipt.json",
            "logs/cargo-test-decision_readiness.log",
        ],
    );
    let profile: Value =
        serde_json::from_slice(&std::fs::read(c1_profile_path()).unwrap()).unwrap();
    let subject = json!({
        "schema": "decision-readiness-subject-v1",
        "profile": {"id": "manual-orchestration.integration-readiness", "version": 1, "sha256": "0".repeat(64)},
        "subject_id": "c1-fixture", "author": "tests",
        "roots": {"destination": s(&dest), "candidate": s(&cand), "evidence": s(&evid), "binaries": s(&bins)},
        "values": {
            "authority_manifest": authority_manifest,
            "source_manifest": source_manifest,
            "binary_manifest": binary_manifest,
            "evidence_manifest": evidence_manifest,
            "validation_record": {"root": "evidence", "path": "validation-v1.json"},
            "review_receipt": {"root": "evidence", "path": "review-receipt.json"},
            "suite_logs": [{"root": "evidence", "path": "logs/cargo-test-decision_readiness.log"}],
            "suites": ["decision_readiness"],
            "destination_head": head,
            "affected_paths": ["affected-a.txt", "affected-b.txt"]
        }
    });
    Case {
        dir,
        dest,
        cand,
        evid,
        bins,
        profile,
        subject,
    }
}

#[test]
fn the_frozen_c1_profile_checks_plans_and_runs_under_the_battery() {
    let mut case = c1_case("battery-c1");
    // Byte-identical to the frozen digest (contract freeze record).
    assert_eq!(
        file_sha(&c1_profile_path()),
        "3ea2eabf3a5f14add09f24005b1a3bf579a41d757bc2bb197e3b7c576c5cdd85"
    );
    // Write the profile bytes unchanged (the Value round trip is for the subject only).
    std::fs::copy(c1_profile_path(), case.profile_path()).unwrap();
    case.subject["profile"]["sha256"] = json!(file_sha(&case.profile_path()));
    write_json(&case.subject_path(), &case.subject);
    let profile_report = case.check_profile();
    assert_eq!(profile_report.code, 0, "{}", profile_report.stdout);
    assert_eq!(profile_report.out_json()["valid"], true);
    let subject_report = case.check_subject();
    assert_eq!(subject_report.code, 0, "{}", subject_report.stdout);
    let run = case.plan();
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout, case.plan().stdout);
    let result = battery_clean(&case, &run.stdout, "c1");
    let id = |suffix: &str| format!("manual-orchestration.integration-readiness.v1.{suffix}");
    // 4 manifest_match, 2 + 0 json_compare over suites, 2 review checks, 2 grep, 2 git.
    assert_eq!(
        result["checks"][id("_git_root")]["text"]
            .as_str()
            .unwrap()
            .trim(),
        s(&case.dest)
    );
    assert_eq!(result["checks"][id("M-logs.2.1")]["match_count"], 1);
    assert_eq!(result["checks"][id("M-logs.1")]["match_count"], 0);
}

#[test]
fn check_subject_accepts_the_base_fixture() {
    let case = base_case("check-subject-ok");
    for run in [case.check_profile(), case.check_subject()] {
        assert_eq!(run.code, 0, "{}", run.stdout);
        let report = run.out_json();
        assert_eq!(report["valid"], true);
        assert_eq!(report["errors"], json!([]));
    }
    assert_eq!(case.check_profile().out_json()["kind"], "profile");
    assert_eq!(case.check_subject().out_json()["kind"], "subject");
}

#[test]
fn out_writes_the_same_bytes_and_refuses_to_overwrite() {
    let case = base_case("plan-out");
    let target = case.dir.join("plan-out.json");
    let args = |t: &std::path::Path| {
        vec![
            "plan".to_string(),
            "--profile".to_string(),
            s(&case.profile_path()),
            "--subject".to_string(),
            s(&case.subject_path()),
            "--out".to_string(),
            s(t),
        ]
    };
    let first = cli(&args(&target));
    assert_eq!(first.code, 0, "{}", first.stderr);
    assert!(first.stdout.is_empty(), "with --out nothing goes to stdout");
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        case.plan().stdout
    );
    let before = std::fs::read(&target).unwrap();
    let second = cli(&args(&target));
    assert_eq!(second.code, 2);
    assert!(second.stdout.is_empty());
    assert!(
        second.err_json()["error"]
            .as_str()
            .unwrap()
            .contains("cannot create")
    );
    assert_eq!(
        std::fs::read(&target).unwrap(),
        before,
        "existing file untouched"
    );
}

#[test]
fn a_refused_plan_writes_nothing_to_stdout_or_out() {
    let mut case = base_case("plan-refused-out");
    case.subject["values"]["suites"] = json!([]);
    case.write_documents();
    let target = case.dir.join("never.json");
    let run = cli(&[
        "plan",
        "--profile",
        &s(&case.profile_path()),
        "--subject",
        &s(&case.subject_path()),
        "--out",
        &s(&target),
    ]);
    assert_eq!(run.code, 2);
    assert!(run.stdout.is_empty());
    assert!(!target.exists());
}

#[test]
fn exit_codes_zero_one_and_two() {
    let case = base_case("exit-codes");
    // 0: valid document.
    assert_eq!(case.check_profile().code, 0);
    // 1: invalid document, with the report on stdout and nothing on stderr.
    let mut bad = case.profile.clone();
    bad["profile_id"] = json!(".bad");
    let bad_path = case.dir.join("bad-profile.json");
    write_json(&bad_path, &bad);
    let invalid = cli(&["check", "--kind", "profile", &s(&bad_path)]);
    assert_eq!(invalid.code, 1);
    assert!(invalid.stderr.is_empty());
    assert_eq!(invalid.out_json()["valid"], false);
    // 2: usage, IO and parse errors, each with one JSON object on stderr and empty stdout.
    let malformed = case.dir.join("malformed.json");
    write(&malformed, "{not json");
    let missing = s(&case.dir.join("does-not-exist.json"));
    let cases: Vec<Vec<String>> = vec![
        vec![],
        vec!["frobnicate".into()],
        vec!["check".into(), "--kind".into(), "profile".into()],
        vec![
            "check".into(),
            "--kind".into(),
            "profile".into(),
            missing.clone(),
        ],
        vec![
            "check".into(),
            "--kind".into(),
            "profile".into(),
            s(&malformed),
        ],
        vec![
            "check".into(),
            "--kind".into(),
            "nonsense".into(),
            s(&case.profile_path()),
        ],
        vec![
            "check".into(),
            "--kind".into(),
            "subject".into(),
            s(&case.subject_path()),
        ],
        vec![
            "check".into(),
            "--kind".into(),
            "evaluation".into(),
            s(&case.profile_path()),
        ],
        vec![
            "check".into(),
            "--kind".into(),
            "assessment".into(),
            s(&case.profile_path()),
        ],
        vec!["plan".into()],
        vec!["plan".into(), "--profile".into(), s(&case.profile_path())],
        vec![
            "plan".into(),
            "--profile".into(),
            s(&case.profile_path()),
            "--subject".into(),
            missing,
        ],
        vec![
            "plan".into(),
            "--profile".into(),
            s(&malformed),
            "--subject".into(),
            s(&case.subject_path()),
        ],
        vec!["plan".into(), "--bogus".into(), "x".into()],
    ];
    for args in cases {
        let run = cli(&args);
        assert_eq!(run.code, 2, "{args:?}: {}{}", run.stdout, run.stderr);
        assert!(run.stdout.is_empty(), "{args:?}");
        let body = run.err_json();
        assert!(body["error"].is_string(), "{args:?}");
    }
}
