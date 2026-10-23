//! `check --kind comparison` (contract 3.4) and E2-AC5: one row per rule code over a finding
//! derived from real fixture files, then the four `--rederive` cases.

mod support;
use proposal_evaluation::check_comparison_bytes;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use support::*;

/// Minimum mutation rows (E2 test constant; EXT-005).
const MIN_RULE_ROWS: usize = 24;

fn derive_to(w: &World, name: &str) -> PathBuf {
    let out = w.dir.join(name);
    let run = cli(&[
        "compare".to_string(),
        "--contract".to_string(),
        s(&w.contract),
        "--estimates".to_string(),
        s(&w.sets[0]),
        "--estimates".to_string(),
        s(&w.sets[1]),
        "--derived-at".to_string(),
        "2026-10-23T00:00:00Z".to_string(),
        "--out".to_string(),
        s(&out),
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    out
}

fn read(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn check(path: &Path, rederive: bool) -> Run {
    let mut args = vec!["check".to_string(), "--kind".into(), "comparison".into()];
    if rederive {
        args.push("--rederive".into());
    }
    args.push(s(path));
    cli(&args)
}

/// Write the edited finding next to the valid one and check it without `--rederive`.
fn mutant(w: &World, base: &Value, edit: impl Fn(&mut Value)) -> Run {
    let mut doc = base.clone();
    edit(&mut doc);
    let path = w.dir.join("mutant.json");
    write_json(&path, &doc);
    check(&path, false)
}

#[test]
fn a_derived_finding_passes_both_modes_and_through_the_library() {
    let w = world("cmp-valid");
    let path = derive_to(&w, "finding.json");
    for rederive in [false, true] {
        let run = check(&path, rederive);
        assert_eq!(run.code, 0, "{}", run.stdout);
        let report = run.out_json();
        assert_eq!(report["valid"], json!(true));
        assert_eq!(report["kind"], json!("comparison"));
        assert_eq!(report["errors"], json!([]));
    }
    let bytes = std::fs::read(&path).unwrap();
    let report = check_comparison_bytes(&bytes, true).unwrap();
    assert!(report.valid, "{:?}", report.errors);
    // The test executable is not the CLI binary, so the library call sees a different tool.
    assert_eq!(report.notes, vec!["tool_differs".to_string()]);
    assert!(
        !check_comparison_bytes(&bytes, false)
            .unwrap()
            .notes
            .is_empty()
    );
}

#[test]
fn every_rule_code_is_reported_at_its_place() {
    let w = world("cmp-rules");
    let path = derive_to(&w, "finding.json");
    let base = read(&path);
    let other = "c".repeat(64);
    let set_path = w.sets[0].clone();
    let set_sha = sha256_of(&set_path);
    type Edit = Box<dyn Fn(&mut Value)>;
    let rows: Vec<(&str, Edit, &str, &str)> = vec![
        (
            "unknown key",
            Box::new(|d| d["weights"] = json!(1)),
            "schema",
            "weights",
        ),
        (
            "schema const",
            Box::new(|d| d["schema"] = json!("x")),
            "schema_mismatch",
            "schema",
        ),
        (
            "derived_at",
            Box::new(|d| d["derived_at"] = json!("yesterday")),
            "time_format",
            "derived_at",
        ),
        (
            "tool name",
            Box::new(|d| d["tool"]["name"] = json!("")),
            "empty_string",
            "tool.name",
        ),
        (
            "tool sha",
            Box::new(|d| d["tool"]["binary_sha256"] = json!("zz")),
            "sha256_pattern",
            "tool.binary_sha256",
        ),
        (
            "contract path",
            Box::new(|d| d["contract"]["path"] = json!("relative.json")),
            "path_not_normalised",
            "contract.path",
        ),
        (
            "contract id",
            Box::new(|d| d["contract"]["contract_id"] = json!("Bad Id")),
            "id_pattern",
            "contract.contract_id",
        ),
        (
            "contract version",
            Box::new(|d| d["contract"]["version"] = json!(0)),
            "version_invalid",
            "contract.version",
        ),
        (
            "pair id",
            Box::new(|d| d["pairs"][0]["a"] = json!("BAD")),
            "id_pattern",
            "pairs.0.a",
        ),
        (
            "authority",
            Box::new(|d| d["authority"]["priority_authorized"] = json!(true)),
            "authority_asserted",
            "authority.priority_authorized",
        ),
        (
            "empty limitation",
            Box::new(|d| d["limitations"][0] = json!("")),
            "empty_string",
            "limitations.0",
        ),
        (
            "pair removed",
            Box::new(|d| d["pairs"] = json!([])),
            "pair_coverage",
            "pairs",
        ),
        (
            "pair a>=b",
            Box::new(|d| {
                d["pairs"][0]["a"] = json!("alpha.two");
                d["pairs"][0]["b"] = json!("alpha.one");
            }),
            "pair_coverage",
            "pairs.0",
        ),
        (
            "pair repeated",
            Box::new(|d| {
                let p = d["pairs"][0].clone();
                d["pairs"].as_array_mut().unwrap().push(p);
            }),
            "pair_coverage",
            "pairs",
        ),
        (
            "sets unsorted",
            Box::new(|d| d["estimate_sets"].as_array_mut().unwrap().reverse()),
            "pair_coverage",
            "estimate_sets",
        ),
        (
            "sets differ from contract",
            Box::new(|d| d["estimate_sets"][0]["packet_sha256"] = json!("d".repeat(64))),
            "pair_coverage",
            "estimate_sets",
        ),
        (
            "measure dropped from both lists",
            Box::new(|d| d["pairs"][0]["dropped_measures"] = json!([])),
            "measure_coverage",
            "pairs.0",
        ),
        (
            "measure listed twice",
            Box::new(|d| {
                let m = d["pairs"][0]["measures"][0].clone();
                d["pairs"][0]["measures"].as_array_mut().unwrap().push(m);
            }),
            "measure_coverage",
            "pairs.0",
        ),
        (
            "outcome",
            Box::new(|d| d["pairs"][0]["outcome"] = json!("incomparable")),
            "outcome_inconsistent",
            "pairs.0.outcome",
        ),
        (
            "blocked reasons",
            Box::new(|d| d["pairs"][0]["blocked_reasons"] = json!(["missing"])),
            "outcome_inconsistent",
            "pairs.0.blocked_reasons",
        ),
        (
            "relation without reasons",
            Box::new(|d| d["pairs"][0]["measures"][0]["relation"] = json!("blocked")),
            "outcome_inconsistent",
            "pairs.0.measures.0.reasons",
        ),
        (
            "dropped without reasons",
            Box::new(|d| d["pairs"][0]["dropped_measures"][0]["reasons"] = json!([])),
            "outcome_inconsistent",
            "pairs.0.dropped_measures.0.reasons",
        ),
        (
            "frontier",
            Box::new(|d| d["frontier"]["dominated"] = json!([])),
            "frontier_inconsistent",
            "frontier",
        ),
        (
            "limitation 1",
            Box::new(|d| {
                d["limitations"].as_array_mut().unwrap().remove(0);
            }),
            "limitation_missing",
            "limitations",
        ),
        (
            "limitation 2",
            Box::new(|d| {
                d["limitations"].as_array_mut().unwrap().remove(1);
            }),
            "limitation_missing",
            "limitations",
        ),
        (
            "limitation dropped",
            Box::new(|d| {
                d["limitations"].as_array_mut().unwrap().remove(2);
            }),
            "limitation_missing",
            "limitations",
        ),
        (
            "limitation 5",
            Box::new(|d| {
                d["limitations"].as_array_mut().unwrap().pop();
            }),
            "limitation_missing",
            "limitations",
        ),
        (
            "conditional_on version",
            Box::new(|d| d["conditional_on"]["contract_version"] = json!(2)),
            "conditional_on_mismatch",
            "conditional_on.contract_version",
        ),
        (
            "conditional_on cutoff",
            Box::new(|d| d["conditional_on"]["evidence_cutoff"] = json!("2020-01-01T00:00:00Z")),
            "conditional_on_mismatch",
            "conditional_on.evidence_cutoff",
        ),
        (
            "contract digest",
            {
                let o = other.clone();
                Box::new(move |d| {
                    d["contract"]["sha256"] = json!(o);
                    d["conditional_on"]["contract_sha256"] = json!(o);
                })
            },
            "digest_mismatch",
            "contract",
        ),
        (
            "contract unresolved",
            Box::new(|d| d["contract"]["path"] = json!("/nonexistent/contract.json")),
            "pointer_unresolved",
            "contract",
        ),
        (
            "contract is another class",
            {
                let (p, h) = (s(&set_path), set_sha.clone());
                Box::new(move |d| {
                    d["contract"]["path"] = json!(p);
                    d["contract"]["sha256"] = json!(h);
                    d["conditional_on"]["contract_sha256"] = json!(h);
                })
            },
            "upstream_class_mismatch",
            "contract",
        ),
    ];
    assert!(rows.len() >= MIN_RULE_ROWS, "{} rows", rows.len());
    for (name, edit, code, at) in &rows {
        let run = mutant(&w, &base, |d| edit(d));
        assert_eq!(run.code, 1, "{name}: {}", run.stdout);
        assert!(
            has(&run.pairs(), code, at),
            "{name}: wanted ({code}, {at:?}) in {:?}",
            run.pairs()
        );
    }
}

fn sha256_of(path: &Path) -> String {
    proposal_evaluation::sha256_hex(&std::fs::read(path).unwrap())
}

#[test]
fn the_interval_limitation_is_required_when_the_contract_is_interval_robust() {
    let w = world("cmp-interval");
    rewrite_contract(&w, |c| c["uncertainty"] = json!("interval_robust"));
    let path = derive_to(&w, "finding.json");
    let base = read(&path);
    assert_eq!(check(&path, true).code, 0);
    let run = mutant(&w, &base, |d| {
        let l = d["limitations"].as_array_mut().unwrap();
        l.retain(|x| {
            !x.as_str()
                .unwrap()
                .starts_with("point_as_degenerate_interval:")
        });
    });
    assert_eq!(run.code, 1);
    assert!(has(&run.pairs(), "limitation_missing", "limitations"));
}

#[test]
fn a_finding_that_is_not_json_or_has_the_wrong_class_is_refused_before_any_rule() {
    let w = world("cmp-class");
    let path = w.dir.join("junk.json");
    write(&path, "not json");
    assert_eq!(check(&path, false).code, 2);
    let estimate_set = check(&w.sets[0], false);
    assert_eq!(estimate_set.code, 1);
    assert!(has(&estimate_set.pairs(), "schema_mismatch", "schema"));
    // --rederive is for comparison findings only.
    let run = cli(&["check", "--kind", "contract", "--rederive", &s(&w.contract)]);
    assert_eq!(run.code, 2);
}

fn edit_json(path: &Path, edit: impl Fn(&mut Value)) {
    let mut doc = read(path);
    edit(&mut doc);
    write_json(path, &doc);
}

#[test]
fn ac5a_an_estimate_set_edited_after_derivation_is_a_digest_refusal() {
    let w = world("cmp-ac5a");
    let path = derive_to(&w, "finding.json");
    edit_json(&w.sets[1], |d| {
        d["estimates"][0]["value"] = json!({"quantity": 3})
    });
    // Contract S6: the plain check hashes the stored estimate-set paths too, so the edit shows.
    let plain = check(&path, false);
    assert_eq!(plain.code, 1, "{}", plain.stdout);
    assert!(has(&plain.pairs(), "digest_mismatch", "estimate_sets.1"));
    let run = check(&path, true);
    assert_eq!(run.code, 3, "{}", run.stderr);
    assert!(has(&run.pairs(), "digest_mismatch", "estimate_sets.1"));
    assert_eq!(run.stdout, "");
    // The library makes the same call P1 will: a binding failure, not a report.
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        check_comparison_bytes(&bytes, true)
            .unwrap_err()
            .exit_code(),
        3
    );
}

#[test]
fn ac5a_a_contract_edited_after_derivation_is_a_digest_refusal() {
    let w = world("cmp-ac5a-contract");
    let path = derive_to(&w, "finding.json");
    rewrite_contract(&w, |c| c["decision_scope"] = json!("a different scope"));
    let run = check(&path, true);
    assert_eq!(run.code, 3, "{}", run.stderr);
    assert!(has(&run.pairs(), "digest_mismatch", "contract"));
    let plain = check(&path, false);
    assert_eq!(plain.code, 1);
    assert!(has(&plain.pairs(), "digest_mismatch", "contract"));
}

#[test]
fn ac5b_an_edited_outcome_is_a_rederive_mismatch() {
    let w = world("cmp-ac5b");
    let path = derive_to(&w, "finding.json");
    edit_json(&path, |d| d["pairs"][0]["outcome"] = json!("incomparable"));
    let run = check(&path, true);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(has(&run.pairs(), "rederive_mismatch", "pairs"));
}

#[test]
fn ac5c_a_consistently_edited_relation_passes_the_plain_check_and_fails_rederive() {
    let w = world("cmp-ac5c");
    let path = derive_to(&w, "finding.json");
    edit_json(&path, |d| {
        d["pairs"][0]["measures"][0]["relation"] = json!("worse");
        d["pairs"][0]["outcome"] = json!("b_dominates_a");
        d["frontier"] =
            json!({"non_dominated": ["alpha.two"], "dominated": ["alpha.one"], "undetermined": []});
    });
    let plain = check(&path, false);
    assert_eq!(plain.code, 0, "{}", plain.stdout);
    let run = check(&path, true);
    assert_eq!(run.code, 1, "{}", run.stdout);
    assert!(has(&run.pairs(), "rederive_mismatch", "frontier"));
}

#[test]
fn ac5d_a_different_tool_alone_is_a_note() {
    let w = world("cmp-ac5d");
    let path = derive_to(&w, "finding.json");
    edit_json(&path, |d| {
        d["tool"]["binary_sha256"] = json!("e".repeat(64))
    });
    let run = check(&path, true);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let report = run.out_json();
    assert_eq!(report["valid"], json!(true));
    assert_eq!(report["notes"], json!(["tool_differs"]));
    let bytes = std::fs::read(&path).unwrap();
    let lib = check_comparison_bytes(&bytes, true).unwrap();
    assert!(lib.valid);
    assert_eq!(lib.notes, vec!["tool_differs".to_string()]);
}
