//! E1-AC2 and E1-AC5: every structural rule has a rejecting test naming its code; unknown
//! keys are rejected at every level; an absent nullable key is rejected; no float is accepted.

mod support;
use serde_json::{Value, json};
use support::*;

/// EXT-005: minimum case counts come from constants in this file, not from the fixture.
const MIN_RULE_CASES: usize = 40;
const MIN_UNKNOWN_KEY_SITES: usize = 14;
const MIN_NULLABLE_KEYS: usize = 7;

fn put(v: &mut Value, pointer: &str, new: Value) {
    *v.pointer_mut(pointer)
        .unwrap_or_else(|| panic!("no {pointer}")) = new;
}

fn remove(v: &mut Value, pointer: &str, key: &str) {
    v.pointer_mut(pointer)
        .unwrap_or_else(|| panic!("no {pointer}"))
        .as_object_mut()
        .unwrap()
        .remove(key)
        .unwrap_or_else(|| panic!("no key {key} at {pointer}"));
}

#[test]
fn baseline_fixture_is_valid() {
    assert_eq!(violations(&dummy_set()), vec![]);
}

type Mutation = fn(&mut Value);

/// (label, mutation, expected code, expected `at`)
fn rule_cases() -> Vec<(&'static str, Mutation, &'static str, &'static str)> {
    vec![
        (
            "schema const",
            |v| put(v, "/schema", json!("other-v1")),
            "schema_mismatch",
            "schema",
        ),
        (
            "schema not a string",
            |v| put(v, "/schema", json!(1)),
            "schema",
            "schema",
        ),
        (
            "set id pattern",
            |v| put(v, "/estimate_set_id", json!("Set 1")),
            "id_pattern",
            "estimate_set_id",
        ),
        (
            "proposal id pattern",
            |v| put(v, "/proposal/proposal_id", json!("single")),
            "id_pattern",
            "proposal.proposal_id",
        ),
        (
            "packet path relative",
            |v| put(v, "/proposal/packet/path", json!("a/b.json")),
            "path_not_normalised",
            "proposal.packet.path",
        ),
        (
            "packet path dotdot",
            |v| put(v, "/proposal/packet/path", json!("/a/../b")),
            "path_not_normalised",
            "proposal.packet.path",
        ),
        (
            "packet sha upper",
            |v| put(v, "/proposal/packet/sha256", json!("A".repeat(64))),
            "sha256_pattern",
            "proposal.packet.sha256",
        ),
        (
            "projection path trailing slash",
            |v| put(v, "/claim_projection/path", json!("/a/b/")),
            "path_not_normalised",
            "claim_projection.path",
        ),
        (
            "projection sha short",
            |v| put(v, "/claim_projection/sha256", json!("abc")),
            "sha256_pattern",
            "claim_projection.sha256",
        ),
        (
            "estimator role empty",
            |v| put(v, "/estimator/role", json!("")),
            "empty_string",
            "estimator.role",
        ),
        (
            "estimator identity empty",
            |v| put(v, "/estimator/identity", json!("")),
            "empty_string",
            "estimator.identity",
        ),
        (
            "issued_at format",
            |v| put(v, "/issued_at", json!("2026-10-22 12:00:00")),
            "time_format",
            "issued_at",
        ),
        (
            "set cutoff format",
            |v| put(v, "/evidence_cutoff", json!("2026-02-30T00:00:00Z")),
            "time_format",
            "evidence_cutoff",
        ),
        (
            "cutoff after issue",
            |v| put(v, "/evidence_cutoff", json!("2026-10-22T12:00:01Z")),
            "cutoff_after_issue",
            "evidence_cutoff",
        ),
        (
            "estimates empty",
            |v| put(v, "/estimates", json!([])),
            "estimates_empty",
            "estimates",
        ),
        (
            "set limitation empty",
            |v| put(v, "/limitations", json!([""])),
            "empty_string",
            "limitations.0",
        ),
        (
            "duplicate measure id",
            |v| put(v, "/estimates/1/measure_id", json!("expected-cost")),
            "duplicate_measure_id",
            "estimates.1.measure_id",
        ),
        (
            "measure id pattern",
            |v| put(v, "/estimates/0/measure_id", json!("Bad Id")),
            "id_pattern",
            "estimates.0.measure_id",
        ),
        (
            "definition empty",
            |v| put(v, "/estimates/0/measure/definition", json!("")),
            "empty_string",
            "estimates.0.measure.definition",
        ),
        (
            "scope empty",
            |v| put(v, "/estimates/0/measure/scope", json!("")),
            "empty_string",
            "estimates.0.measure.scope",
        ),
        (
            "unit empty",
            |v| put(v, "/estimates/0/measure/scale/unit", json!("")),
            "empty_string",
            "estimates.0.measure.scale.unit",
        ),
        (
            "exponent too large",
            |v| put(v, "/estimates/0/measure/scale/exponent", json!(19)),
            "scale_invalid",
            "estimates.0.measure.scale.exponent",
        ),
        (
            "exponent too small",
            |v| put(v, "/estimates/0/measure/scale/exponent", json!(-19)),
            "scale_invalid",
            "estimates.0.measure.scale.exponent",
        ),
        (
            "one level",
            |v| put(v, "/estimates/1/measure/scale/levels", json!(["only"])),
            "scale_invalid",
            "estimates.1.measure.scale.levels",
        ),
        (
            "duplicate levels",
            |v| put(v, "/estimates/1/measure/scale/levels", json!(["a", "a"])),
            "scale_invalid",
            "estimates.1.measure.scale.levels",
        ),
        (
            "empty level label",
            |v| put(v, "/estimates/1/measure/scale/levels", json!(["", "b"])),
            "empty_string",
            "estimates.1.measure.scale.levels.0",
        ),
        (
            "value kind mismatch",
            |v| put(v, "/estimates/0/value", json!({"level": "low"})),
            "value_scale_mismatch",
            "estimates.0.value",
        ),
        (
            "level not in scale",
            |v| {
                put(
                    v,
                    "/estimates/1/competing/0/value",
                    json!({"level": "extreme"}),
                )
            },
            "level_unknown",
            "estimates.1.competing.0.value",
        ),
        (
            "baseline kind mismatch",
            |v| put(v, "/estimates/0/baseline", json!({"level": "low"})),
            "value_scale_mismatch",
            "estimates.0.baseline",
        ),
        (
            "estimate cutoff format",
            |v| put(v, "/estimates/0/evidence_cutoff", json!("yesterday")),
            "time_format",
            "estimates.0.evidence_cutoff",
        ),
        (
            "estimate cutoff after set",
            |v| {
                put(
                    v,
                    "/estimates/0/evidence_cutoff",
                    json!("2026-10-22T10:00:01Z"),
                )
            },
            "estimate_cutoff_after_set_cutoff",
            "estimates.0.evidence_cutoff",
        ),
        (
            "interval low above value",
            |v| put(v, "/estimates/0/interval/low", json!({"quantity": 9})),
            "interval_out_of_order",
            "estimates.0.interval",
        ),
        (
            "interval high below value",
            |v| put(v, "/estimates/0/interval/high", json!({"quantity": 7})),
            "interval_out_of_order",
            "estimates.0.interval",
        ),
        (
            "interval kind mismatch",
            |v| put(v, "/estimates/0/interval/high", json!({"level": "high"})),
            "value_scale_mismatch",
            "estimates.0.interval.high",
        ),
        (
            "estimated without basis",
            |v| put(v, "/estimates/0/basis", json!([])),
            "basis_required",
            "estimates.0.basis",
        ),
        (
            "basis sha pattern",
            |v| {
                put(
                    v,
                    "/estimates/0/basis/0/claim_revision_sha256",
                    json!("xyz"),
                )
            },
            "sha256_pattern",
            "estimates.0.basis.0.claim_revision_sha256",
        ),
        (
            "basis claim id empty",
            |v| put(v, "/estimates/0/basis/0/claim_id", json!("")),
            "empty_string",
            "estimates.0.basis.0.claim_id",
        ),
        (
            "one competing reading",
            |v| {
                let competing = v.pointer("/estimates/1/competing/0").unwrap().clone();
                put(v, "/estimates/1/competing", json!([competing]));
            },
            "competing_insufficient",
            "estimates.1.competing",
        ),
        (
            "competing reading without basis",
            |v| put(v, "/estimates/1/competing/1/basis", json!([])),
            "competing_insufficient",
            "estimates.1.competing.1.basis",
        ),
        (
            "not_estimated reason empty",
            |v| put(v, "/estimates/2/reason", json!("")),
            "empty_string",
            "estimates.2.reason",
        ),
        (
            "estimate limitation empty",
            |v| put(v, "/estimates/2/limitations", json!([""])),
            "empty_string",
            "estimates.2.limitations.0",
        ),
        (
            "projection null with basis",
            |v| put(v, "/claim_projection", Value::Null),
            "claim_projection_presence",
            "claim_projection",
        ),
        (
            "projection present without basis",
            |v| {
                put(v, "/estimates/0/basis", json!([]));
                put(v, "/estimates/1/competing/0/basis", json!([]));
                put(v, "/estimates/1/competing/1/basis", json!([]));
            },
            "claim_projection_presence",
            "claim_projection",
        ),
        (
            "readiness asserted",
            |v| put(v, "/authority/readiness_judgment_issued", json!(true)),
            "authority_asserted",
            "authority.readiness_judgment_issued",
        ),
        (
            "priority asserted",
            |v| put(v, "/authority/priority_authorized", json!(true)),
            "authority_asserted",
            "authority.priority_authorized",
        ),
        (
            "acceptance asserted",
            |v| put(v, "/authority/acceptance_authorized", json!(true)),
            "authority_asserted",
            "authority.acceptance_authorized",
        ),
    ]
}

#[test]
fn every_structural_rule_rejects_with_its_code() {
    let cases = rule_cases();
    assert!(cases.len() >= MIN_RULE_CASES, "only {} cases", cases.len());
    for (label, mutate, code, at) in cases {
        let mut doc = dummy_set();
        mutate(&mut doc);
        let found = violations(&doc);
        assert!(
            has(&found, code, at),
            "{label}: wanted ({code}, {at}) in {found:?}"
        );
    }
}

#[test]
fn unknown_keys_rejected_at_every_level() {
    let sites = [
        "",
        "/proposal",
        "/proposal/packet",
        "/estimator",
        "/authority",
        "/claim_projection",
        "/estimates/0",
        "/estimates/1",
        "/estimates/2",
        "/estimates/0/measure",
        "/estimates/0/measure/scale",
        "/estimates/1/measure/scale",
        "/estimates/0/basis/0",
        "/estimates/0/interval",
        "/estimates/1/competing/0",
        "/estimates/1/competing/0/basis/0",
    ];
    assert!(sites.len() >= MIN_UNKNOWN_KEY_SITES);
    for site in sites {
        let mut doc = dummy_set();
        let target = if site.is_empty() {
            &mut doc
        } else {
            doc.pointer_mut(site).unwrap()
        };
        target
            .as_object_mut()
            .unwrap()
            .insert("surprise".into(), json!(1));
        let found = violations(&doc);
        assert!(
            found.iter().any(|(c, _)| c == "schema"),
            "unknown key at {site:?} not rejected: {found:?}"
        );
    }
    // Unknown key inside a value object and an unknown enum variant.
    let mut doc = dummy_set();
    put(
        &mut doc,
        "/estimates/0/value",
        json!({"quantity": 8, "extra": 1}),
    );
    assert!(violations(&doc).iter().any(|(c, _)| c == "schema"));
    let mut doc = dummy_set();
    put(&mut doc, "/estimates/0/status", json!("maybe"));
    assert!(violations(&doc).iter().any(|(c, _)| c == "schema"));
    let mut doc = dummy_set();
    put(&mut doc, "/estimates/0/confidence", json!("certain"));
    assert!(violations(&doc).iter().any(|(c, _)| c == "schema"));
}

#[test]
fn absent_nullable_keys_are_rejected() {
    let keys: [(&str, &str); 7] = [
        ("", "claim_projection"),
        ("/estimates/0", "baseline"),
        ("/estimates/0", "evidence_cutoff"),
        ("/estimates/0", "interval"),
        ("/estimates/1", "baseline"),
        ("/estimates/1", "evidence_cutoff"),
        ("/estimates/2", "evidence_cutoff"),
    ];
    assert!(keys.len() >= MIN_NULLABLE_KEYS);
    for (site, key) in keys {
        let mut doc = dummy_set();
        if site.is_empty() {
            doc.as_object_mut().unwrap().remove(key).unwrap();
        } else {
            remove(&mut doc, site, key);
        }
        let found = violations(&doc);
        assert!(
            found.iter().any(|(c, _)| c == "schema"),
            "absent {key} at {site:?} not rejected: {found:?}"
        );
    }
}

#[test]
fn no_float_is_accepted_anywhere() {
    // `1.0` and `8.0` must be rejected where an integer is required (E1-AC5); the text is
    // spliced into the serialised bytes because `json!` would keep the float as a float too.
    let base = serde_json::to_string_pretty(&dummy_set()).unwrap();
    let swaps = [
        ("\"quantity\": 8\n", "\"quantity\": 8.0\n"),
        ("\"exponent\": 0", "\"exponent\": 1.0"),
        ("\"quantity\": 10", "\"quantity\": 1.0e1"),
        ("\"quantity\": 5", "\"quantity\": 5.5"),
    ];
    for (from, to) in swaps {
        assert!(base.contains(from), "fixture lacks {from:?}");
        let bytes = base.replacen(from, to, 1).into_bytes();
        let parsed = proposal_evaluation::validate::parse_estimate_set(&bytes).unwrap();
        match parsed {
            proposal_evaluation::validate::Parsed::Invalid(errors) => {
                assert_eq!(errors[0].code, "schema", "{to}")
            }
            other => panic!("float {to:?} accepted: {other:?}"),
        }
    }
}

#[test]
fn several_violations_are_all_reported() {
    let mut doc = dummy_set();
    put(&mut doc, "/estimate_set_id", json!("Bad Id"));
    put(&mut doc, "/estimator/role", json!(""));
    put(&mut doc, "/estimates/0/value", json!({"level": "low"}));
    put(&mut doc, "/authority/priority_authorized", json!(true));
    assert!(violations(&doc).len() >= 4);
}
