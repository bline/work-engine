//! Rejection fixtures: each rule is violated on its own and must be reported by its
//! stable code. `check` reports it with exit 1; `plan` refuses with exit 2, one JSON
//! object on stderr, and nothing on stdout.

mod support;

use serde_json::{Value, json};
use std::collections::BTreeSet;
use support::*;

/// Apply `edit` to the base profile, then require `code` from `check --kind profile`
/// (exit 1, and no other rule violated unless `also` lists it) and from `plan` (exit 2).
fn profile_rejected(name: &str, code: &str, edit: impl FnOnce(&mut Value)) {
    let mut case = base_case(name);
    edit(&mut case.profile);
    case.write_documents();
    let check = case.check_profile();
    assert_eq!(check.code, 1, "{name}: {}", check.stdout);
    assert!(check.stderr.is_empty());
    let report = check.out_json();
    assert_eq!(report["valid"], false);
    assert!(
        check.codes().iter().any(|c| c == code),
        "{name}: wanted {code}, got {:?}",
        check.codes()
    );
    let plan = case.plan();
    assert_eq!(plan.code, 2, "{name}");
    assert!(plan.stdout.is_empty());
    assert!(
        plan.codes().iter().any(|c| c == code),
        "{name}: plan codes {:?}",
        plan.codes()
    );
}

/// Apply `edit` to the base subject, then require `code` from `check --kind subject` and `plan`.
fn subject_rejected(name: &str, code: &str, edit: impl FnOnce(&mut Case)) {
    let mut case = base_case(name);
    edit(&mut case);
    case.write_documents();
    let check = case.check_subject();
    assert_eq!(check.code, 1, "{name}: {}{}", check.stdout, check.stderr);
    assert!(
        check.codes().iter().any(|c| c == code),
        "{name}: wanted {code}, got {:?}",
        check.codes()
    );
    let plan = case.plan();
    assert_eq!(plan.code, 2, "{name}");
    assert!(plan.stdout.is_empty());
    assert!(
        plan.codes().iter().any(|c| c == code),
        "{name}: plan codes {:?}",
        plan.codes()
    );
}

type Edit = Box<dyn Fn(&mut Value)>;

fn req(p: &mut Value, index: usize) -> &mut Value {
    &mut p["requirements"][index]
}
fn pred(p: &mut Value, requirement: usize, predicate: usize) -> &mut Value {
    &mut p["requirements"][requirement]["predicates"][predicate]
}

// Requirement indices in the base profile.
const R_FILE: usize = 0;
const R_MANIFEST: usize = 1;
const R_JSON: usize = 2;
const R_GREP: usize = 5;
const R_GIT: usize = 6;
const A_NOTE: usize = 7;

#[test]
fn duplicate_requirement_id() {
    profile_rejected("rej-dup-id", "duplicate_requirement_id", |p| {
        req(p, R_MANIFEST)["id"] = json!("R-file");
    });
}

#[test]
fn dotted_requirement_id() {
    profile_rejected("rej-dotted-id", "invalid_requirement_id", |p| {
        req(p, R_FILE)["id"] = json!("R.file");
    });
}

#[test]
fn invalid_profile_id_and_schema_name() {
    profile_rejected("rej-profile-id", "invalid_profile_id", |p| {
        p["profile_id"] = json!(".x")
    });
    profile_rejected("rej-schema-name", "schema_mismatch", |p| {
        p["schema"] = json!("decision-readiness-profile-v2")
    });
}

#[test]
fn predicate_on_an_attributed_requirement() {
    profile_rejected("rej-attr-pred", "attributed_with_predicates", |p| {
        let predicate = json!({"kind": "manifest_match", "manifest": {"param": "source_manifest"}});
        req(p, A_NOTE)["predicates"] = json!([predicate]);
    });
}

#[test]
fn mechanical_requirement_rules() {
    profile_rejected("rej-mech-empty", "mechanical_without_predicates", |p| {
        req(p, R_FILE)["predicates"] = json!([]);
    });
    profile_rejected("rej-mech-evidence", "mechanical_with_evidence_from", |p| {
        req(p, R_FILE)["evidence_from"] = json!(["R-manifest"]);
    });
    profile_rejected("rej-evidence-ref", "unknown_evidence_ref", |p| {
        req(p, A_NOTE)["evidence_from"] = json!(["R-file", "no-such-requirement"]);
    });
    profile_rejected("rej-evidence-ref-attr", "unknown_evidence_ref", |p| {
        req(p, A_NOTE)["evidence_from"] = json!(["A-note"]);
    });
}

#[test]
fn unknown_parameter_and_unknown_root() {
    profile_rejected("rej-unknown-param", "unknown_parameter", |p| {
        pred(p, R_MANIFEST, 0)["manifest"] = json!({"param": "nope"});
    });
    profile_rejected("rej-unknown-param-path", "unknown_parameter", |p| {
        pred(p, R_JSON, 0)["file"] = json!({"param": "nope"});
    });
    profile_rejected("rej-unknown-root", "unknown_root", |p| {
        pred(p, R_FILE, 0)["file"] = json!({"root": "zzz", "path": "src/a.txt"});
    });
    profile_rejected("rej-unknown-entries-root", "unknown_root", |p| {
        p["parameters"][0]["entries_root"] = json!("zzz");
    });
}

#[test]
fn parameter_kind_mismatch() {
    profile_rejected("rej-kind-file", "param_kind_mismatch", |p| {
        pred(p, R_JSON, 0)["file"] = json!({"param": "source_manifest"});
    });
    profile_rejected("rej-kind-manifest", "param_kind_mismatch", |p| {
        pred(p, R_MANIFEST, 0)["manifest"] = json!({"param": "suites"});
    });
    profile_rejected("rej-kind-grep", "param_kind_mismatch", |p| {
        pred(p, R_GREP, 0)["files"] = json!({"param": "suites"});
    });
    profile_rejected("rej-kind-each", "param_kind_mismatch", |p| {
        pred(p, 3, 0)["each"] = json!({"param": "head"});
    });
}

#[test]
fn git_predicate_without_a_git_root() {
    profile_rejected("rej-no-git-root", "git_predicate_without_git_root", |p| {
        p["roots"][0] = json!({"name": "dest"});
    });
}

#[test]
fn root_and_parameter_declaration_rules() {
    profile_rejected("rej-dup-root", "duplicate_root", |p| {
        p["roots"][2]["name"] = json!("cand")
    });
    profile_rejected("rej-two-git", "multiple_git_roots", |p| {
        p["roots"][1]["git"] = json!(true)
    });
    profile_rejected("rej-dup-param", "duplicate_parameter", |p| {
        p["parameters"][1]["name"] = json!("source_manifest")
    });
    profile_rejected("rej-entries-required", "entries_root_required", |p| {
        p["parameters"][0]
            .as_object_mut()
            .unwrap()
            .remove("entries_root");
    });
    profile_rejected("rej-entries-unexpected", "entries_root_unexpected", |p| {
        p["parameters"][4]["entries_root"] = json!("cand");
    });
    profile_rejected("rej-bound-kind", "bound_by_unexpected_kind", |p| {
        p["parameters"][4]["bound_by"] = json!("evidence_manifest");
    });
    profile_rejected("rej-bound-target", "bound_by_invalid_target", |p| {
        p["parameters"][2]["bound_by"] = json!("suites");
    });
    profile_rejected("rej-empty-name", "empty_name", |p| {
        p["parameters"][4]["name"] = json!("")
    });
}

#[test]
fn invalid_relative_paths_in_a_profile() {
    for (n, bad) in ["../escape.txt", "a/../b", "/abs/x", "a//b", "a/", ""]
        .iter()
        .enumerate()
    {
        profile_rejected(&format!("rej-rel-{n}"), "invalid_relative_path", |p| {
            pred(p, R_FILE, 0)["file"] = json!({"root": "cand", "path": bad});
        });
    }
}

#[test]
fn float_or_null_expected() {
    for (n, bad) in [
        json!(1.5),
        json!(null),
        json!([1, null]),
        json!({"a": {"b": 0.25}}),
    ]
    .into_iter()
    .enumerate()
    {
        profile_rejected(&format!("rej-float-{n}"), "float_or_null_expected", |p| {
            pred(p, R_JSON, 0)["expected"] = bad;
        });
    }
}

#[test]
fn forbidden_git_subcommand_and_arguments() {
    profile_rejected("rej-git-push", "git_subcommand_forbidden", |p| {
        pred(p, R_GIT, 0)["args"] = json!(["push", "origin"]);
    });
    profile_rejected("rej-git-output", "git_argument_forbidden", |p| {
        pred(p, R_GIT, 1)["args"] = json!(["status", "--output=x"]);
    });
    profile_rejected("rej-git-c", "git_argument_forbidden", |p| {
        pred(p, R_GIT, 1)["args"] = json!(["status", "-c", "core.x=y"]);
    });
    profile_rejected("rej-git-exec", "git_argument_forbidden", |p| {
        pred(p, R_GIT, 1)["args"] = json!(["status", "--exec-path"]);
    });
    profile_rejected("rej-git-dash-after-sep", "git_argument_forbidden", |p| {
        pred(p, R_GIT, 1)["args"] = json!(["status", "--", "-x"]);
    });
    profile_rejected("rej-git-param-sub", "git_subcommand_not_literal", |p| {
        pred(p, R_GIT, 1)["args"] = json!([{"param": "affected"}]);
    });
    profile_rejected("rej-git-empty", "git_args_empty", |p| {
        pred(p, R_GIT, 1)["args"] = json!([]);
    });
}

#[test]
fn key_and_predicate_shape_rules() {
    profile_rejected("rej-each-missing", "each_placeholder_mismatch", |p| {
        pred(p, 3, 0).as_object_mut().unwrap().remove("each");
    });
    profile_rejected("rej-each-extra", "each_placeholder_mismatch", |p| {
        pred(p, R_JSON, 0)["each"] = json!({"param": "suites"});
    });
    profile_rejected("rej-empty-key", "empty_key", |p| {
        pred(p, R_JSON, 0)["key"] = json!("")
    });
    profile_rejected("rej-empty-files", "empty_files", |p| {
        pred(p, R_GREP, 2)["files"] = json!([])
    });
    profile_rejected("rej-bad-sha-literal", "invalid_sha256", |p| {
        pred(p, R_FILE, 0)["expected"] = json!("ABC");
    });
}

#[test]
fn unknown_fields_are_a_schema_error_from_check_and_plan() {
    // check: exit 1 with code `schema`. plan: exit 2 with the same code in its errors.
    let edits: Vec<(&str, Edit)> = vec![
        ("top-level", Box::new(|p| p["bogus"] = json!(1))),
        (
            "requirement rationale",
            Box::new(|p| req(p, R_FILE)["rationale"] = json!("why")),
        ),
        (
            "predicate",
            Box::new(|p| pred(p, R_FILE, 0)["extra"] = json!(true)),
        ),
        ("root", Box::new(|p| p["roots"][1]["extra"] = json!(1))),
        (
            "authority ref",
            Box::new(|p| p["authority_refs"][0]["extra"] = json!(1)),
        ),
        (
            "lines as number",
            Box::new(|p| p["authority_refs"][0]["lines"] = json!(5)),
        ),
        (
            "unknown predicate kind",
            Box::new(|p| pred(p, R_FILE, 0)["kind"] = json!("file_exists")),
        ),
        (
            "missing field",
            Box::new(|p| {
                p.as_object_mut().unwrap().remove("decision");
            }),
        ),
    ];
    for (n, (label, edit)) in edits.iter().enumerate() {
        let mut case = base_case(&format!("rej-schema-{n}"));
        edit(&mut case.profile);
        case.write_documents();
        let check = case.check_profile();
        assert_eq!(check.code, 1, "{label}: {}", check.stdout);
        assert_eq!(check.codes(), ["schema"], "{label}");
        let plan = case.plan();
        assert_eq!(plan.code, 2, "{label}");
        assert!(plan.codes().iter().any(|c| c == "schema"), "{label}");
    }
    // The same for a subject, and for an observation.
    let mut case = base_case("rej-schema-subject");
    case.subject["extra"] = json!(1);
    case.write_documents();
    let check = case.check_subject();
    assert_eq!((check.code, check.codes()), (1, vec!["schema".to_string()]));
    assert_eq!(case.plan().code, 2);
}

// ---------------------------------------------------------------------------
// Subject
// ---------------------------------------------------------------------------

#[test]
fn manifest_digest_mismatch() {
    subject_rejected("rej-manifest-digest", "manifest_digest_mismatch", |c| {
        c.subject["values"]["source_manifest"]["sha256"] = json!("1".repeat(64));
    });
    // A changed manifest file is the same fault from the other side.
    subject_rejected("rej-manifest-changed", "manifest_digest_mismatch", |c| {
        let path = c.evid.join("source-manifest.json");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push('\n');
        write(&path, &text);
    });
}

#[test]
fn empty_manifest() {
    subject_rejected("rej-empty-manifest", "empty_manifest", |c| {
        let sha = write_json(&c.evid.join("source-manifest.json"), &json!([]));
        c.subject["values"]["source_manifest"]["sha256"] = json!(sha);
    });
}

#[test]
fn invalid_manifest_contents() {
    let bad: Vec<(&str, Value)> = vec![
        ("not-an-array", json!({"path": "src/a.txt"})),
        ("missing-sha", json!([{"path": "src/a.txt"}])),
        ("bad-sha", json!([{"path": "src/a.txt", "sha256": "xyz"}])),
        (
            "dup-path",
            json!([{"path": "src/a.txt", "sha256": "0".repeat(64)}, {"path": "src/a.txt", "sha256": "0".repeat(64)}]),
        ),
        (
            "dotdot",
            json!([{"path": "../a.txt", "sha256": "0".repeat(64)}]),
        ),
    ];
    for (label, content) in bad {
        subject_rejected(&format!("rej-manifest-{label}"), "manifest_invalid", |c| {
            let sha = write_json(&c.evid.join("source-manifest.json"), &content);
            c.subject["values"]["source_manifest"]["sha256"] = json!(sha);
        });
    }
}

#[test]
fn manifest_entries_with_extra_keys_are_accepted() {
    // Contract 3.3: other keys in a manifest entry are ignored.
    let mut case = base_case("manifest-extra-keys");
    let sha = file_sha(&case.cand.join("src/a.txt"));
    let manifest_sha = write_json(
        &case.evid.join("source-manifest.json"),
        &json!([{"path": "src/a.txt", "sha256": sha, "bytes": 6, "note": null}]),
    );
    case.subject["values"]["source_manifest"]["sha256"] = json!(manifest_sha);
    case.write_documents();
    assert_eq!(case.check_subject().code, 0);
    assert_eq!(case.plan().code, 0);
}

#[test]
fn bound_by_path_absent_from_its_manifest() {
    subject_rejected("rej-bound-absent", "bound_by_path_absent", |c| {
        c.subject["values"]["validation_record"]["path"] = json!("not-in-manifest.json");
    });
    subject_rejected("rej-bound-absent-list", "bound_by_path_absent", |c| {
        c.subject["values"]["logs"][1]["path"] = json!("logs/other.log");
    });
}

#[test]
fn bound_by_root_differs_from_the_manifests_entries_root() {
    // The path is an entry of the manifest, but named under another root.
    subject_rejected("rej-bound-root", "bound_by_root_mismatch", |c| {
        c.subject["values"]["validation_record"]["root"] = json!("cand");
    });
    subject_rejected("rej-bound-root-list", "bound_by_root_mismatch", |c| {
        c.subject["values"]["logs"][0]["root"] = json!("bins");
    });
}

#[test]
fn empty_list_and_path_list_values() {
    subject_rejected("rej-empty-list", "empty_list_value", |c| {
        c.subject["values"]["suites"] = json!([])
    });
    subject_rejected("rej-empty-list-2", "empty_list_value", |c| {
        c.subject["values"]["affected"] = json!([])
    });
    subject_rejected("rej-empty-path-list", "empty_list_value", |c| {
        c.subject["values"]["logs"] = json!([])
    });
}

#[test]
fn non_normalised_root_paths() {
    for (n, bad) in [
        "relative/dir",
        "/a/../b",
        "/a/./b",
        "/trailing/",
        "/a//b",
        "/",
        "",
    ]
    .iter()
    .enumerate()
    {
        subject_rejected(&format!("rej-root-{n}"), "root_not_normalised", |c| {
            c.subject["roots"]["cand"] = json!(bad);
        });
    }
}

#[test]
fn paths_with_dot_dot_in_subject_values() {
    subject_rejected("rej-value-dotdot", "invalid_relative_path", |c| {
        c.subject["values"]["validation_record"]["path"] = json!("../validation.json");
    });
    subject_rejected("rej-value-abs", "invalid_relative_path", |c| {
        c.subject["values"]["logs"][0]["path"] = json!("/etc/passwd");
    });
    subject_rejected("rej-value-empty-comp", "invalid_relative_path", |c| {
        c.subject["values"]["validation_record"]["path"] = json!("logs//core.log");
    });
}

#[test]
fn value_and_root_binding_rules() {
    subject_rejected("rej-missing-value", "missing_value", |c| {
        c.subject["values"].as_object_mut().unwrap().remove("head");
    });
    subject_rejected("rej-unknown-value", "unknown_value", |c| {
        c.subject["values"]["extra"] = json!("x")
    });
    subject_rejected("rej-type-string", "value_type_mismatch", |c| {
        c.subject["values"]["head"] = json!(7)
    });
    subject_rejected("rej-type-list", "value_type_mismatch", |c| {
        c.subject["values"]["suites"] = json!("core")
    });
    subject_rejected("rej-type-path", "value_type_mismatch", |c| {
        c.subject["values"]["validation_record"] = json!("validation.json")
    });
    subject_rejected("rej-type-manifest", "value_type_mismatch", |c| {
        c.subject["values"]["source_manifest"] =
            json!({"root": "evid", "path": "source-manifest.json"});
    });
    subject_rejected("rej-missing-root", "missing_root", |c| {
        c.subject["roots"].as_object_mut().unwrap().remove("bins");
    });
    subject_rejected("rej-unknown-root-name", "unknown_root", |c| {
        c.subject["roots"]["extra"] = json!("/x")
    });
    subject_rejected("rej-value-unknown-root", "unknown_root", |c| {
        c.subject["values"]["validation_record"]["root"] = json!("zzz");
    });
    subject_rejected("rej-bad-manifest-sha", "invalid_sha256", |c| {
        c.subject["values"]["source_manifest"]["sha256"] = json!("NOT-HEX");
    });
}

#[test]
fn profile_binding_rules() {
    subject_rejected("rej-bind-id", "profile_id_mismatch", |c| {
        c.subject["profile"]["id"] = json!("other.id")
    });
    subject_rejected("rej-bind-version", "profile_version_mismatch", |c| {
        c.subject["profile"]["version"] = json!(2)
    });
    subject_rejected("rej-bind-schema", "schema_mismatch", |c| {
        c.subject["schema"] = json!("decision-readiness-subject-v0")
    });
    // A subject recording another digest: write_documents would overwrite it, so write by hand.
    let case = base_case("rej-bind-digest");
    let mut subject = case.subject.clone();
    subject["profile"]["sha256"] = json!("f".repeat(64));
    write_json(&case.subject_path(), &subject);
    let check = case.check_subject();
    assert_eq!(check.code, 1);
    assert!(
        check
            .codes()
            .contains(&"profile_digest_mismatch".to_string())
    );
    let plan = case.plan();
    assert_eq!(plan.code, 2);
    assert!(
        plan.codes()
            .contains(&"profile_digest_mismatch".to_string())
    );
}

#[test]
fn expanded_git_arguments_are_checked_like_literal_ones() {
    subject_rejected("rej-expanded-output", "git_argument_forbidden", |c| {
        c.subject["values"]["affected"] = json!(["--output=x"]);
    });
    subject_rejected("rej-expanded-dash", "git_argument_forbidden", |c| {
        c.subject["values"]["affected"] = json!(["tracked.txt", "-x"]);
    });
}

#[test]
fn every_violated_rule_is_reported_not_only_the_first() {
    let mut case = base_case("rej-many");
    case.subject["values"]["suites"] = json!([]);
    case.subject["roots"]["cand"] = json!("relative");
    case.subject["values"]["validation_record"]["path"] = json!("../x");
    case.write_documents();
    let codes: BTreeSet<String> = case.check_subject().codes().into_iter().collect();
    for want in [
        "empty_list_value",
        "root_not_normalised",
        "invalid_relative_path",
    ] {
        assert!(codes.contains(want), "{want} not in {codes:?}");
    }
}

#[test]
fn unreadable_or_malformed_inputs_are_usage_errors_not_findings() {
    // A manifest file that cannot be read: exit 2 from check and plan, no code finding.
    let case = base_case("rej-manifest-unreadable");
    std::fs::remove_file(case.evid.join("source-manifest.json")).unwrap();
    for run in [case.check_subject(), case.plan()] {
        assert_eq!(run.code, 2, "{}", run.stdout);
        assert!(run.stdout.is_empty());
        assert!(
            run.err_json()["error"]
                .as_str()
                .unwrap()
                .contains("cannot read")
        );
    }
    // Malformed JSON for each document.
    let case = base_case("rej-malformed");
    write(&case.subject_path(), "{\"schema\": ");
    for run in [case.check_subject(), case.plan()] {
        assert_eq!(run.code, 2);
        assert!(run.stdout.is_empty());
    }
    // `check --kind subject` against an invalid profile refuses rather than guessing.
    let mut case = base_case("rej-subject-invalid-profile");
    case.profile["profile_id"] = json!(".bad");
    case.write_documents();
    let run = case.check_subject();
    assert_eq!(run.code, 2);
    assert!(
        run.err_json()["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["code"] == "invalid_profile_id")
    );
}

// ---------------------------------------------------------------------------
// Multi-profile plan
// ---------------------------------------------------------------------------

/// A second profile that shares root names with the base fixture.
fn add_pair(case: &Case, id: &str, version: u64, roots: Value, git: bool) -> (String, String) {
    let profile = json!({
        "schema": "decision-readiness-profile-v1",
        "profile_id": id, "version": version,
        "decision": {"id": "d", "owner_role": "coordinator"},
        "profile_owner": "coordinator", "assessor_roles": ["reviewer"],
        "authority_refs": [{"path": "n.md", "lines": "1"}],
        "roots": [{"name": "dest", "git": git}, {"name": "cand"}],
        "parameters": [],
        "requirements": [{"id": "S-file", "class": "mechanical", "statement": "s",
            "predicates": [{"kind": "file_sha256", "file": {"root": "cand", "path": "src/a.txt"},
                            "expected": file_sha(&case.cand.join("src/a.txt"))}]}]
    });
    let profile_path = case.dir.join(format!("{id}-{version}-profile.json"));
    let sha = write_json(&profile_path, &profile);
    let subject = json!({
        "schema": "decision-readiness-subject-v1",
        "profile": {"id": id, "version": version, "sha256": sha},
        "subject_id": "s", "author": "tests", "roots": roots, "values": {}
    });
    let subject_path = case.dir.join(format!("{id}-{version}-subject.json"));
    write_json(&subject_path, &subject);
    (s(&profile_path), s(&subject_path))
}

fn multi(case: &Case, extra: &(String, String)) -> Run {
    cli(&[
        "plan",
        "--profile",
        &s(&case.profile_path()),
        "--subject",
        &s(&case.subject_path()),
        "--profile",
        &extra.0,
        "--subject",
        &extra.1,
    ])
}

#[test]
fn multi_profile_refusals() {
    let case = base_case("rej-multi");
    // The same (profile_id, version) twice.
    let again = (s(&case.profile_path()), s(&case.subject_path()));
    let run = multi(&case, &again);
    assert_eq!(run.code, 2);
    assert!(
        run.codes().contains(&"duplicate_profile".to_string()),
        "{:?}",
        run.codes()
    );
    // A root name mapped to two paths.
    let other_dir = case.dir.join("elsewhere");
    std::fs::create_dir_all(&other_dir).unwrap();
    let conflicting = add_pair(
        &case,
        "fixture.conflict",
        1,
        json!({"dest": s(&case.dest), "cand": s(&other_dir)}),
        true,
    );
    let run = multi(&case, &conflicting);
    assert_eq!(run.code, 2);
    assert!(
        run.codes().contains(&"root_path_conflict".to_string()),
        "{:?}",
        run.codes()
    );
    // Two distinct git-root paths: dest for the first pair, `elsewhere` for the second.
    let second_git = add_pair(
        &case,
        "fixture.gitroots",
        1,
        json!({"dest": s(&other_dir), "cand": s(&case.cand)}),
        true,
    );
    let run = multi(&case, &second_git);
    assert_eq!(run.code, 2);
    let codes = run.codes();
    assert!(
        codes.contains(&"root_path_conflict".to_string())
            || codes.contains(&"multiple_git_root_paths".to_string()),
        "{codes:?}"
    );
    // The same names mapped to the same paths are accepted.
    let fine = add_pair(
        &case,
        "fixture.fine",
        1,
        json!({"dest": s(&case.dest), "cand": s(&case.cand)}),
        true,
    );
    assert_eq!(multi(&case, &fine).code, 0);
    // A different version of the same profile id is a different pair.
    let versioned = add_pair(
        &case,
        "fixture.base",
        2,
        json!({"dest": s(&case.dest), "cand": s(&case.cand)}),
        true,
    );
    assert_eq!(multi(&case, &versioned).code, 0);
}

#[test]
fn multi_profile_git_root_rules() {
    // A git root in only one profile: the plan's root is that path, whichever pair comes first.
    let case = base_case("multi-git-only-second");
    let no_git = add_pair(
        &case,
        "fixture.nogit",
        1,
        json!({"dest": s(&case.dest), "cand": s(&case.cand)}),
        false,
    );
    // no_git first, then the base (git) pair.
    let run = cli(&[
        "plan",
        "--profile",
        &no_git.0,
        "--subject",
        &no_git.1,
        "--profile",
        &s(&case.profile_path()),
        "--subject",
        &s(&case.subject_path()),
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.out_json()["root"], s(&case.dest));
    // Only non-git profiles: the root is the first declared root of the first pair.
    let other = add_pair(
        &case,
        "fixture.nogit2",
        1,
        json!({"dest": s(&case.dest), "cand": s(&case.cand)}),
        false,
    );
    let run = cli(&[
        "plan",
        "--profile",
        &no_git.0,
        "--subject",
        &no_git.1,
        "--profile",
        &other.0,
        "--subject",
        &other.1,
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let plan = run.out_json();
    assert_eq!(plan["root"], s(&case.dest));
    assert_eq!(plan["extra_roots"], json!([s(&case.cand)]));
    assert!(
        plan["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| !c["id"].as_str().unwrap().ends_with("_git_root"))
    );
}

// ---------------------------------------------------------------------------
// Observation
// ---------------------------------------------------------------------------

fn observation() -> Value {
    json!({
        "schema": "decision-readiness-observation-v1",
        "plan": {"path": "/abs/plan.json", "sha256": "a".repeat(64)},
        "result": {"path": "/abs/result.json", "sha256": "b".repeat(64)},
        "battery_script": {"path": "/abs/evidence_battery.py", "sha256": "c".repeat(64)},
        "battery_exit": 0,
        "stdout_battery_sha256": "d".repeat(64),
        "started_at": "2026-10-21T12:00:00Z",
        "finished_at": "2026-10-21T12:00:03Z",
        "invoker": {"script": "/abs/integration_readiness.py", "sha256": "e".repeat(64)}
    })
}

fn check_observation(name: &str, doc: &Value) -> Run {
    let dir = fresh_dir(name);
    let path = dir.join("observation.json");
    write_json(&path, doc);
    cli(&["check", "--kind", "observation", &s(&path)])
}

#[test]
fn a_valid_observation_is_accepted_including_battery_exit_one() {
    for exit in [0, 1] {
        let mut doc = observation();
        doc["battery_exit"] = json!(exit);
        let run = check_observation(&format!("obs-ok-{exit}"), &doc);
        assert_eq!(run.code, 0, "{}", run.stdout);
        assert_eq!(run.out_json()["kind"], "observation");
        assert_eq!(run.out_json()["errors"], json!([]));
    }
    // Equal start and finish is not a time-order violation.
    let mut doc = observation();
    doc["finished_at"] = doc["started_at"].clone();
    assert_eq!(check_observation("obs-same-time", &doc).code, 0);
}

#[test]
fn observation_rejections_by_code() {
    let cases: Vec<(&str, &str, Edit)> = vec![
        (
            "exit2",
            "battery_exit_invalid",
            Box::new(|d| d["battery_exit"] = json!(2)),
        ),
        (
            "exit-neg",
            "battery_exit_invalid",
            Box::new(|d| d["battery_exit"] = json!(-1)),
        ),
        (
            "ts-format",
            "invalid_timestamp",
            Box::new(|d| d["started_at"] = json!("2026-10-21 12:00:00")),
        ),
        (
            "ts-offset",
            "invalid_timestamp",
            Box::new(|d| d["finished_at"] = json!("2026-10-21T12:00:03+00:00")),
        ),
        (
            "ts-date",
            "invalid_timestamp",
            Box::new(|d| d["started_at"] = json!("2026-02-30T00:00:00Z")),
        ),
        (
            "order",
            "time_order",
            Box::new(|d| d["finished_at"] = json!("2026-10-21T11:59:59Z")),
        ),
        (
            "rel-plan",
            "invalid_absolute_path",
            Box::new(|d| d["plan"]["path"] = json!("plan.json")),
        ),
        (
            "dotdot-result",
            "invalid_absolute_path",
            Box::new(|d| d["result"]["path"] = json!("/a/../b")),
        ),
        (
            "rel-script",
            "invalid_absolute_path",
            Box::new(|d| d["invoker"]["script"] = json!("script.py")),
        ),
        (
            "bad-sha",
            "invalid_sha256",
            Box::new(|d| d["plan"]["sha256"] = json!("ABC")),
        ),
        (
            "bad-stdout-sha",
            "invalid_sha256",
            Box::new(|d| d["stdout_battery_sha256"] = json!("abc")),
        ),
        (
            "bad-invoker-sha",
            "invalid_sha256",
            Box::new(|d| d["invoker"]["sha256"] = json!("g".repeat(64))),
        ),
        (
            "schema-name",
            "schema_mismatch",
            Box::new(|d| d["schema"] = json!("decision-readiness-observation-v2")),
        ),
        (
            "unknown-field",
            "schema",
            Box::new(|d| d["clock"] = json!("now")),
        ),
        (
            "missing-field",
            "schema",
            Box::new(|d| {
                d.as_object_mut().unwrap().remove("started_at");
            }),
        ),
    ];
    for (label, code, edit) in cases {
        let mut doc = observation();
        edit(&mut doc);
        let run = check_observation(&format!("obs-{label}"), &doc);
        assert_eq!(run.code, 1, "{label}: {}", run.stdout);
        assert!(
            run.codes().iter().any(|c| c == code),
            "{label}: wanted {code}, got {:?}",
            run.codes()
        );
    }
    // --profile does not apply to an observation.
    let dir = fresh_dir("obs-profile-flag");
    let path = dir.join("o.json");
    write_json(&path, &observation());
    let run = cli(&[
        "check",
        "--kind",
        "observation",
        "--profile",
        &s(&path),
        &s(&path),
    ]);
    assert_eq!(run.code, 2);
}
