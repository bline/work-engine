//! E2 unit 1: the comparison-contract schema and `check --kind contract` (contract 3.1, 3.4,
//! S4; E2-AC6).

mod support;
use serde_json::{Value, json};
use support::*;

const MIN_RULE_ROWS: usize = 24;
const MIN_UNKNOWN_KEY_SITES: usize = 9;

/// Apply `edit` to a valid contract.
fn edited(edit: impl FnOnce(&mut Value)) -> Value {
    let mut c = valid_contract();
    edit(&mut c);
    c
}

#[test]
fn a_valid_contract_has_no_violations() {
    assert_eq!(contract_violations(&valid_contract()), vec![]);
}

#[test]
fn the_drop_measure_treatment_is_also_valid() {
    let c = edited(|c| {
        c["treatment"] =
            json!({"missing": "drop_measure", "contested": "drop_measure", "stale": "drop_measure"})
    });
    assert_eq!(contract_violations(&c), vec![]);
}

#[test]
fn every_rule_has_its_one_code_at_its_place() {
    type Row = (&'static str, &'static str, &'static str, fn(&mut Value));
    let rows: Vec<Row> = vec![
        ("schema_mismatch", "schema", "wrong schema", |c| {
            c["schema"] = json!("other-v1")
        }),
        ("schema", "schema", "schema not a string", |c| {
            c["schema"] = json!(3)
        }),
        ("id_pattern", "contract_id", "bad contract id", |c| {
            c["contract_id"] = json!("Bad_Id")
        }),
        ("version_invalid", "version", "version 0", |c| {
            c["version"] = json!(0)
        }),
        ("schema", "", "negative version", |c| {
            c["version"] = json!(-1)
        }),
        ("schema", "", "float version", |c| c["version"] = json!(1.5)),
        (
            "empty_string",
            "selecting_authority.decision_owner",
            "owner",
            |c| c["selecting_authority"]["decision_owner"] = json!(""),
        ),
        (
            "empty_string",
            "selecting_authority.exercised_by",
            "exercised",
            |c| c["selecting_authority"]["exercised_by"] = json!(""),
        ),
        (
            "empty_string",
            "selecting_authority.evidence",
            "evidence",
            |c| c["selecting_authority"]["evidence"] = json!(""),
        ),
        ("empty_string", "decision_scope", "scope", |c| {
            c["decision_scope"] = json!("")
        }),
        (
            "contract_proposals_insufficient",
            "proposals",
            "one proposal",
            |c| {
                c["proposals"].as_array_mut().unwrap().pop();
            },
        ),
        (
            "duplicate_proposal_id",
            "proposals.1.proposal_id",
            "duplicate id",
            |c| c["proposals"][1]["proposal_id"] = json!("alpha.one"),
        ),
        (
            "id_pattern",
            "proposals.0.proposal_id",
            "no separator",
            |c| c["proposals"][0]["proposal_id"] = json!("alpha"),
        ),
        (
            "sha256_pattern",
            "proposals.0.packet_sha256",
            "short digest",
            |c| c["proposals"][0]["packet_sha256"] = json!("abc"),
        ),
        ("measures_empty", "measures", "no measures", |c| {
            c["measures"] = json!([])
        }),
        (
            "duplicate_measure_id",
            "measures.1.measure_id",
            "duplicate measure",
            |c| c["measures"][1]["measure_id"] = json!("cost"),
        ),
        (
            "id_pattern",
            "measures.0.measure_id",
            "bad measure id",
            |c| c["measures"][0]["measure_id"] = json!("Cost"),
        ),
        ("empty_string", "measures.0.definition", "definition", |c| {
            c["measures"][0]["definition"] = json!("")
        }),
        ("empty_string", "measures.0.scope", "measure scope", |c| {
            c["measures"][0]["scope"] = json!("")
        }),
        (
            "scale_invalid",
            "measures.1.scale.levels",
            "one level",
            |c| c["measures"][1]["scale"]["levels"] = json!(["only"]),
        ),
        (
            "scale_invalid",
            "measures.1.scale.levels",
            "repeated level",
            |c| c["measures"][1]["scale"]["levels"] = json!(["a", "a"]),
        ),
        (
            "scale_invalid",
            "measures.0.scale.exponent",
            "exponent range",
            |c| c["measures"][0]["scale"]["exponent"] = json!(19),
        ),
        ("empty_string", "measures.0.scale.unit", "unit", |c| {
            c["measures"][0]["scale"]["unit"] = json!("")
        }),
        ("schema", "", "minimum_confidence outside UD3 set", |c| {
            c["measures"][0]["minimum_confidence"] = json!("certain")
        }),
        (
            "empty_string",
            "excluded_concerns.0.concern",
            "concern",
            |c| c["excluded_concerns"][0]["concern"] = json!(""),
        ),
        (
            "empty_string",
            "excluded_concerns.0.reason",
            "reason",
            |c| c["excluded_concerns"][0]["reason"] = json!(""),
        ),
        (
            "schema",
            "",
            "treatment value outside block|drop_measure",
            |c| c["treatment"]["stale"] = json!("ignore"),
        ),
        ("schema", "", "treatment member absent", |c| {
            c["treatment"].as_object_mut().unwrap().remove("stale");
        }),
        ("schema", "", "uncertainty outside the closed set", |c| {
            c["uncertainty"] = json!("weighted")
        }),
        ("time_format", "evidence_cutoff", "cutoff format", |c| {
            c["evidence_cutoff"] = json!("2026-10-22")
        }),
        ("time_format", "stale_before", "stale format", |c| {
            c["stale_before"] = json!("yesterday")
        }),
        (
            "stale_before_after_cutoff",
            "stale_before",
            "stale after cutoff",
            |c| c["stale_before"] = json!("2026-10-23T00:00:00Z"),
        ),
        (
            "authority_asserted",
            "authority.priority_authorized",
            "priority",
            |c| c["authority"]["priority_authorized"] = json!(true),
        ),
        (
            "authority_asserted",
            "authority.acceptance_authorized",
            "acceptance",
            |c| c["authority"]["acceptance_authorized"] = json!(true),
        ),
    ];
    assert!(rows.len() >= MIN_RULE_ROWS, "{} rows", rows.len());
    for (code, at, label, edit) in rows {
        let found = contract_violations(&edited(edit));
        assert!(
            has(&found, code, at),
            "{label}: expected ({code}, {at}) in {found:?}"
        );
    }
}

#[test]
fn a_violated_rule_reports_exactly_its_own_code() {
    // Every rule has exactly one code: a single-rule edit yields one distinct code.
    let found = contract_violations(&edited(|c| c["version"] = json!(0)));
    assert_eq!(
        found,
        vec![("version_invalid".to_string(), "version".to_string())]
    );
}

#[test]
fn every_violation_of_a_parseable_contract_is_listed() {
    let c = edited(|c| {
        c["version"] = json!(0);
        c["decision_scope"] = json!("");
        c["authority"]["priority_authorized"] = json!(true);
    });
    let found = contract_violations(&c);
    assert!(has(&found, "version_invalid", "version"));
    assert!(has(&found, "empty_string", "decision_scope"));
    assert!(has(
        &found,
        "authority_asserted",
        "authority.priority_authorized"
    ));
}

#[test]
fn an_unknown_key_is_rejected_at_every_nested_record() {
    // E2-AC6: no weights field can be expressed; the same closure holds at every site.
    type Site = (&'static str, fn(&mut Value));
    let sites: Vec<Site> = vec![
        ("top level weights", |c| c["weights"] = json!({"cost": 1})),
        ("top level other", |c| c["extra"] = json!(1)),
        ("selecting_authority", |c| {
            c["selecting_authority"]["x"] = json!(1)
        }),
        ("proposal", |c| c["proposals"][0]["x"] = json!(1)),
        ("measure", |c| c["measures"][0]["x"] = json!(1)),
        ("measure weight", |c| c["measures"][0]["weight"] = json!(3)),
        ("scale", |c| c["measures"][0]["scale"]["x"] = json!(1)),
        ("excluded concern", |c| {
            c["excluded_concerns"][0]["x"] = json!(1)
        }),
        ("treatment", |c| c["treatment"]["x"] = json!("block")),
        ("authority", |c| c["authority"]["x"] = json!(false)),
    ];
    assert!(sites.len() >= MIN_UNKNOWN_KEY_SITES);
    for (label, edit) in sites {
        let found = contract_violations(&edited(edit));
        assert!(
            found.iter().any(|(code, _)| code == "schema"),
            "{label}: {found:?}"
        );
    }
}

#[test]
fn a_nullable_or_required_key_may_not_be_absent() {
    for key in [
        "treatment",
        "uncertainty",
        "excluded_concerns",
        "stale_before",
    ] {
        let c = edited(|c| {
            c.as_object_mut().unwrap().remove(key);
        });
        let found = contract_violations(&c);
        assert!(
            found.iter().any(|(code, _)| code == "schema"),
            "{key}: {found:?}"
        );
    }
}

#[test]
fn a_repeated_key_is_a_schema_error() {
    let text = serde_json::to_string(&valid_contract()).unwrap();
    let doubled = text.replacen('{', "{\"version\":2,", 1);
    let report = proposal_evaluation::check_contract_bytes(doubled.as_bytes()).unwrap();
    assert!(!report.valid);
    assert!(report.errors.iter().all(|e| e.code == "schema"));
}

#[test]
fn the_contract_cli_prints_a_pretty_report_and_exits_by_validity() {
    let dir = fresh_dir("contract-cli");
    let good = dir.join("good.json");
    write_json(&good, &valid_contract());
    let run = cli(&["check", "--kind", "contract", &s(&good)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.ends_with("}\n") && !run.stdout.ends_with("\n\n"));
    assert_eq!(run.out_json()["kind"], json!("contract"));
    assert_eq!(run.out_json()["valid"], json!(true));

    let bad = dir.join("bad.json");
    write_json(&bad, &edited(|c| c["version"] = json!(0)));
    let run = cli(&["check", "--kind", "contract", &s(&bad)]);
    assert_eq!(run.code, 1);
    assert!(has(&run.pairs(), "version_invalid", "version"));

    let junk = dir.join("junk.json");
    write(&junk, "not json");
    let run = cli(&["check", "--kind", "contract", &s(&junk)]);
    assert_eq!(run.code, 2);
    assert_eq!(run.stdout, "");
}
