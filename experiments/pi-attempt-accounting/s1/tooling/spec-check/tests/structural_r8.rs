//! Supplied-record structural controls; no financial expected-value derivation.
use pi_accounting_spec_check::*;
use serde_json::{Value, json};
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn spec() -> Spec {
    read_spec(&root()).unwrap()
}
fn bundle() -> Value {
    json!({"documents":[
     {"kind":"accounting_input_proof","id":"input","version":"1.7-s1","lineage_id":"L","origins":[{"origin_id":"o","source_origin_id":"s","original_position":1,"raw_bytes_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","source_revision_ref":"raw:revision","initial_contract_ref":"c"}],"initial_contracts":[{"contract_id":"c","reference_namespace":"unestablished","duplicate_rule":"same_origin_only"}],"grants":[],"identity_bindings":[],"exact_links":[],"continuity":{"predecessor_subject_refs":[],"preserved_initial_contract_refs":[],"preserved_fact_refs":[],"exact_origin_map_ref":"raw:origin-map"}},
     {"kind":"accounting_result","id":"result","version":"1.7-s1","input_proof_ref":"input","lineage_id":"L","origins":[{"origin_ref":"o","base_id":"b","evidence_atom_id":"o","active_high_unit_id":"u"}],"bases":[{"base_id":"b","origin_refs":["o"],"atomize":false,"atomize_fact_refs":[],"break_copies":false,"break_copies_fact_refs":[]}],"units":[{"unit_id":"u","state":"active","origin_refs":["o"],"exposure_ref":"computed:exposure"}],"primary_validities":[],"financial_candidates":[],"definite_links":[],"coverage_decisions":[],"low_rollups":[],"continuity_result":"unestablished","failure_refs":[]},
     {"kind":"accounting_expected_assertion","id":"expected","version":"1.7-s1","raw_subject_ref":"raw:revision","policy_refs":["PLAN"],"authored_by":"independent-owner","exact_subject_refs":["input","result"],"assertions":[]}
    ]})
}
#[test]
fn evidence_shapes_and_exact_kinds() {
    let b = bundle();
    validate_accounting_bundle(&b).unwrap();
    for key in ["money", "eligible", "chosen_partition"] {
        let mut bad = b.clone();
        bad["documents"][0][key] = json!(true);
        assert!(validate_accounting_bundle(&bad).is_err());
    }
    let mut bad = b.clone();
    bad["documents"][1]["input_proof_ref"] = json!("expected");
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("interchanged")
    );
    let mut bad = b;
    bad["documents"][1]["input_proof_ref"] = json!("absent");
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("dangling")
    );
}
#[test]
fn exact_ownership_and_refinement_negatives() {
    let b = bundle();
    let mut bad = b.clone();
    bad["documents"][1]["origins"] = json!([]);
    assert!(validate_accounting_bundle(&bad).is_err());
    let mut bad = b.clone();
    bad["documents"][1]["units"][0]["origin_refs"] = json!([]);
    assert!(validate_accounting_bundle(&bad).is_err());
    let mut bad = b.clone();
    bad["documents"][1]["bases"][0]["atomize"] = json!(true);
    assert!(validate_accounting_bundle(&bad).is_err());
    let mut bad = b;
    bad["documents"][1]["bases"][0]["break_copies"] = json!(true);
    assert!(validate_accounting_bundle(&bad).is_err());
}
#[test]
fn s_cannot_construct_or_replace_validity() {
    let mut s = spec();
    let r = s
        .rules
        .iter_mut()
        .find(|r| r.id == "S-unique-one-fail")
        .unwrap();
    r.outputs[0] = Constructor {
        type_id: "SettlementDecision".into(),
        variant: "valid".into(),
        fields: vec![
            "basis".into(),
            "charged".into(),
            "reserved".into(),
            "by".into(),
        ],
        bindings: Default::default(),
    };
    assert!(validate(&s).unwrap_err().contains("C-validity owner violation"));
    let mut s = spec();
    s.rules
        .iter_mut()
        .find(|r| r.id == "S-unique-one-fail")
        .unwrap()
        .outputs[0]
        .bindings
        .insert("validity_ref".into(), json!("other.validity"));
    assert!(validate(&s).unwrap_err().contains("forward immutable"));
}
#[test]
fn standard_schema_distinguishes_stage_boundary() {
    let s = spec();
    validate(&s).unwrap();
    let schema: Value =
        serde_json::from_slice(include_bytes!("../../../spec/ck.schema.json")).unwrap();
    let mut v = serde_json::to_value(&s).unwrap();
    v["boundaries"][0]["id"] = json!("C");
    assert!(validate_schema(&schema, &v).is_err());
    let mut v = serde_json::to_value(&s).unwrap();
    v["stages"][0]["id"] = json!("C-primary");
    assert!(validate_schema(&schema, &v).is_err());
}
#[test]
fn actual_wire_flattening_and_amount_strings() {
    let s = spec();
    let bound =
        json!({"variant":"bound","fields":{"nano":"15","basis":"usage_valuation","evidence":[]}});
    let high =
        serialize_external_constructor(&s, "High", "exact", &json!({"bound":bound})).unwrap();
    assert_eq!(
        high,
        json!({"kind":"exact","nano":"15","basis":"usage_valuation"})
    );
    let exposure = json!({"exposure":{"pool":"pi_api","low":{"nano":"15","basis":"usage_valuation"},"high":high},"valuation":{"state":"valued","low":"15","high":"15","identities":["openai/model"],"complete":true,"derivations":[]},"obligation":{"state":"none","amount_source":"none","cause":"unbudgeted"}});
    validate_wire_document(&s, &exposure).unwrap();
    let mut bad = exposure.clone();
    bad["exposure"]["high"]["nano"] = json!(15);
    assert!(validate_wire_document(&s, &bad).is_err());
    let mut bad = exposure;
    bad["exposure"]["pool"] = json!("unknown");
    assert!(validate_wire_document(&s, &bad).is_err());
}
#[test]
fn semantic_changes_invalidate_actual_consumers() {
    let s = spec();
    let deps: Dependencies =
        serde_json::from_slice(include_bytes!("../../../spec/dependencies.json")).unwrap();
    let before = consumer_digests(&s, &deps).unwrap();
    for node in [
        "USAGE-PRESENCE",
        "RAW-FIELD-PREDICATES",
        "INITIAL-REFERENCE",
        "ATOMIZATION",
        "FINANCIAL-CANDIDATES",
        "DEFINITE-LINK",
        "SUPPORTED-LOW",
        "WHOLE-COVERAGE",
        "CONTINUITY",
    ] {
        let affected = affected_consumers(&s, &deps, &[node.into()]).unwrap();
        let mut changed = s.clone();
        changed
            .invariants
            .iter_mut()
            .find(|n| n.id == node)
            .unwrap()
            .text
            .push_str(" changed predicate");
        let after = consumer_digests(&changed, &deps).unwrap();
        for id in [
            "view-contract",
            "view-schema",
            "consumer-R",
            "python-validator",
            "independent-raw-assertions",
            "corpus-wire",
        ] {
            assert!(
                affected.contains(&id.to_string()),
                "{node} fails to reach {id}"
            );
            assert_ne!(before[id], after[id], "{node} did not invalidate {id}");
        }
    }
    assert!(
        check_acknowledgements(&s, &deps)
            .unwrap()
            .contains(&"consumer-R".into())
    );
}
fn two_origin_bundle() -> Value {
    let mut b = bundle();
    let mut second = b["documents"][0]["origins"][0].clone();
    second["origin_id"] = json!("s");
    second["original_position"] = json!(2);
    second["raw_bytes_sha256"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    b["documents"][0]["origins"]
        .as_array_mut()
        .unwrap()
        .push(second);
    b["documents"][1]["origins"].as_array_mut().unwrap().push(
        json!({"origin_ref":"s","base_id":"b","evidence_atom_id":"s","active_high_unit_id":"u"}),
    );
    b["documents"][1]["bases"][0]["origin_refs"] = json!(["o", "s"]);
    b["documents"][1]["units"][0]["origin_refs"] = json!(["o", "s"]);
    b
}
fn grant_record(id: &str, kind: &str, origins: Value) -> Value {
    let mut grant = grant_record_fields(id, kind, origins);
    if kind == "exact_row_coreference" {
        grant["admitted_payload"] = json!({"from":"o","to":"s"});
    }
    grant
}
fn grant_record_fields(id: &str, kind: &str, origins: Value) -> Value {
    json!({"grant_id":id,"guarantee_kind":kind,"admission":"conditional_fixture","exact_payload_ref":"fixture:exact-payload","covered_origin_refs":origins,"covered_extent_ref":"fixture:extent","authority_ref":"fixture:authority","observer_or_harness_ref":"fixture:harness","assumption_refs":["fixture:assumptions"]})
}
#[test]
fn exact_grant_kinds_and_scopes_are_not_interchangeable() {
    let mut b = two_origin_bundle();
    b["documents"][0]["grants"] = json!([grant_record(
        "g",
        "exact_row_coreference",
        json!(["o", "s"])
    )]);
    b["documents"][0]["exact_links"] =
        json!([{"link_id":"l","from_origin_ref":"o","to_origin_ref":"s","grant_ref":"g"}]);
    validate_accounting_bundle(&b).unwrap();
    let mut bad = b.clone();
    bad["documents"][0]["grants"][0]["guarantee_kind"] = json!("physical_bill_independence");
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("interchanged grant")
    );
    let mut bad = b;
    bad["documents"][0]["grants"][0]["covered_origin_refs"] = json!(["o"]);
    assert!(validate_accounting_bundle(&bad).is_err());
}
#[test]
fn whole_coverage_rejects_partial_ownership_and_invalid_witness() {
    let mut b = two_origin_bundle();
    b["documents"][1]["primary_validities"] = json!([{"obligation_id":"v","distinct_reservation_refs":["o"],"distinct_settlement_refs":["s"],"validity_result":"valid","applicable_check_refs":["computed:checks"]}]);
    b["documents"][1]["primary_memberships"] = json!([{"obligation_id":"v","reservation_classes":[{"origin_refs":["o"],"representative_origin_ref":"o"}],"settlement_classes":[{"origin_refs":["s"],"representative_origin_ref":"s"}],"usage_origin_refs":["s"]}]);
    b["documents"][1]["units"][0]["state"] = json!("inactive");
    b["documents"][1]["units"][0]["successor_ref"] = json!("resolved");
    b["documents"][1]["units"].as_array_mut().unwrap().push(json!({"unit_id":"resolved","state":"active","origin_refs":["o","s"],"exposure_ref":"computed:resolved"}));
    for o in b["documents"][1]["origins"].as_array_mut().unwrap() {
        o["active_high_unit_id"] = json!("resolved");
    }
    b["documents"][1]["coverage_decisions"] = json!([{"coverage_id":"cover","route":"budgeted","obligation_id":"v","status":"accepted","unit_refs":["u"],"monetary_origin_refs":["o","s"],"anchor_origin_refs":[],"validity_ref":"v","valuation_ref":"computed:valuation","definite_link_refs":[],"retained_component_refs":["computed:charge"],"result_unit_ref":"resolved","reason_refs":["computed:eligible"]}]);
    b["documents"][1]["valuation_records"] = json!([{"valuation_id":"computed:valuation","usage_origin_refs":["s"],"complete":true,"wire_ref":"computed:wire"}]);
    b["documents"][1]["retained_components"] = json!([{"component_id":"computed:charge","origin_ref":"s","role":"charge","nano_usd":"5"}]);
    b["documents"][1]["units"][1]["retained_component_refs"] = json!(["computed:charge"]);
    validate_accounting_bundle(&b).unwrap();
    let mut incomplete = b.clone();
    incomplete["documents"][1]["valuation_records"][0]["complete"] = json!(false);
    assert!(
        validate_accounting_bundle(&incomplete)
            .unwrap_err()
            .contains("complete valuation")
    );
    let mut wrong_kind = b.clone();
    wrong_kind["documents"][1]["coverage_decisions"][0]["valuation_ref"] = json!("expected");
    assert!(
        validate_accounting_bundle(&wrong_kind)
            .unwrap_err()
            .contains("non-valuation")
    );
    let mut bad = b.clone();
    bad["documents"][1]["coverage_decisions"][0]["monetary_origin_refs"] = json!(["s"]);
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("whole-unit")
    );
    let mut bad = b;
    bad["documents"][1]["primary_validities"][0]["validity_result"] = json!("rejected");
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("invalid primary")
    );
}
#[test]
fn physical_credit_keeps_every_ambiguous_alternative() {
    let mut b = two_origin_bundle();
    b["documents"][0]["grants"] = json!([
        grant_record(
            "independence",
            "physical_bill_independence",
            json!(["o", "s"])
        ),
        grant_record("binding", "origin_slot_binding", json!(["o"]))
    ]);
    b["documents"][0]["independence_frame"] = json!({"frame_id":"frame","currency":"USD","grant_ref":"independence","slots":[{"slot_id":"P","bill_occurrence_ref":"bill:P","grant_ref":"independence"}],"origin_slots":[{"origin_ref":"o","slot_id":"P","grant_ref":"binding"}],"distinct_origin_slot":[]});
    b["documents"][1]["low_rollups"] = json!([{"scope_ref":"scope","seed_origin_refs":["o"],"alternative_origin_refs":["o","s"],"single_floor":"0","credits":[{"slot_ref":"P","alternative_origin_refs":["o","s"],"low":"0","valuation_refs":["computed:valuation"]}],"independent_floor":"0","selected_low":"0","credit_mode":"independent_slots","basis":"none"}]);
    validate_accounting_bundle(&b).unwrap();
    let mut bad = b;
    bad["documents"][1]["low_rollups"][0]["credits"][0]["alternative_origin_refs"] = json!(["o"]);
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("incomplete physical-credit")
    );
}
#[test]
fn continuity_rejects_rewritten_origin_and_initial_contract() {
    let mut b = bundle();
    let mut prior = b["documents"][0].clone();
    prior["id"] = json!("prior");
    b["documents"].as_array_mut().unwrap().push(prior);
    b["documents"][0]["continuity"]["predecessor_subject_refs"] = json!(["prior"]);
    b["documents"][1]["continuity_result"] = json!("conditional");
    validate_accounting_bundle(&b).unwrap();
    let mut bad = b.clone();
    bad["documents"][0]["origins"][0]["raw_bytes_sha256"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("rewrites origin")
    );
    let mut bad = b;
    bad["documents"][0]["initial_contracts"][0]["reference_namespace"] = json!("admission");
    assert!(validate_accounting_bundle(&bad).is_err());
}
#[test]
fn unbudgeted_completion_uses_computed_applicability_not_expected_authority() {
    let mut b = two_origin_bundle();
    b["documents"][0]["grants"] = json!([grant_record(
        "link",
        "exact_row_coreference",
        json!(["o", "s"])
    )]);
    b["documents"][0]["exact_links"] =
        json!([{"link_id":"l","from_origin_ref":"o","to_origin_ref":"s","grant_ref":"link"}]);
    let r = &mut b["documents"][1];
    r["definite_links"] =
        json!([{"link_id":"d","from_origin_ref":"o","to_origin_ref":"s","admitted_input_link_ref":"l"}]);
    r["unbudgeted_applicabilities"] = json!([{"applicability_id":"app","usage_origin_ref":"o","raw_subject_ref":"raw:usage-scope","classification":"genuine_unbudgeted","dollar_origin_refs":[]}]);
    r["unbudgeted_completions"] = json!([{"completion_id":"done","usage_origin_ref":"o","applicability_ref":"app","valuation_ref":"computed:valuation","complete":true,"usage_origin_refs":["o"]}]);
    r["units"][0]["state"] = json!("inactive");
    r["units"][0]["successor_ref"] = json!("resolved");
    r["units"].as_array_mut().unwrap().push(json!({"unit_id":"resolved","state":"active","origin_refs":["o","s"],"exposure_ref":"computed:exposure"}));
    for o in r["origins"].as_array_mut().unwrap() {
        o["active_high_unit_id"] = json!("resolved");
    }
    r["valuation_records"] = json!([{"valuation_id":"computed:valuation","usage_origin_refs":["o"],"complete":true,"wire_ref":"computed:wire"}]);
    r["retained_components"] = json!([{"component_id":"computed:context","origin_ref":"s","role":"usage","nano_usd":"5"}]);
    r["units"][1]["retained_component_refs"] = json!(["computed:context"]);
    r["coverage_decisions"] = json!([{"coverage_id":"cover","route":"unbudgeted_completion","completion_ref":"done","status":"accepted","unit_refs":["u"],"monetary_origin_refs":["o"],"anchor_origin_refs":["s"],"valuation_ref":"computed:valuation","definite_link_refs":["d"],"retained_component_refs":["computed:context"],"result_unit_ref":"resolved","reason_refs":["computed:complete"]}]);
    validate_accounting_bundle(&b).unwrap();
    let mut bad = b.clone();
    bad["documents"][1]["unbudgeted_completions"][0]["applicability_ref"] = json!("expected");
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("computed unbudgeted applicability")
    );
    let mut bad = b;
    bad["documents"][1]["primary_memberships"] = json!([{"obligation_id":"rejected-dollar","reservation_classes":[],"settlement_classes":[{"origin_refs":["s"],"representative_origin_ref":"s"}],"usage_origin_refs":["o"]}]);
    bad["documents"][1]["primary_validities"] = json!([{"obligation_id":"rejected-dollar","distinct_reservation_refs":[],"distinct_settlement_refs":["s"],"validity_result":"rejected","applicable_check_refs":["NoReservation"]}]);
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("dollar primary evidence")
    );
}
