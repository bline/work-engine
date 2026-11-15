//! Synthetic supplied-record applicability controls. No financial expected results.
use pi_accounting_spec_check::validate_accounting_bundle;
use serde_json::{Value, json};
fn bundle() -> Value {
    let ids = [
        "anchor",
        "reserve",
        "settle",
        "usage",
        "other-reserve",
        "other-settle",
        "other-usage",
    ];
    let mut input = json!({"kind":"accounting_input_proof","id":"input","version":"1.7-s1","lineage_id":"L","origins":[],"initial_contracts":[{"contract_id":"c","reference_namespace":"unestablished","duplicate_rule":"same_origin_only"}],"grants":[],"identity_bindings":[],"exact_links":[],"continuity":{"predecessor_subject_refs":[],"preserved_initial_contract_refs":[],"preserved_fact_refs":[],"exact_origin_map_ref":"raw:origin-map"}});
    input["origins"] = json!(ids.iter().enumerate().map(|(i,id)| json!({"origin_id":id,"source_origin_id":"source","original_position":i,"raw_bytes_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","source_revision_ref":"raw:revision","initial_contract_ref":"c"})).collect::<Vec<_>>());
    input["grants"] = json!([{"grant_id":"grant","guarantee_kind":"exact_row_coreference","admission":"conditional_fixture","exact_payload_ref":"raw:exact-pairs","covered_origin_refs":ids,"covered_extent_ref":"raw:extent","authority_ref":"fixture","observer_or_harness_ref":"synthetic","assumption_refs":["conditional"],"admitted_payload":{"pairs":[{"from":"anchor","to":"usage"},{"from":"other-reserve","to":"other-usage"}]}}]);
    input["exact_links"] = json!([{"link_id":"input-link","from_origin_ref":"anchor","to_origin_ref":"usage","grant_ref":"grant"}]);
    let covered = ["anchor", "reserve", "settle", "usage"];
    let others = ["other-reserve", "other-settle", "other-usage"];
    let mut result = json!({"kind":"accounting_result","id":"result","version":"1.7-s1","input_proof_ref":"input","lineage_id":"L","origins":[],"bases":[{"base_id":"base","origin_refs":ids,"atomize":false,"atomize_fact_refs":[],"break_copies":false,"break_copies_fact_refs":[]}],"units":[{"unit_id":"hold","state":"inactive","origin_refs":covered,"exposure_ref":"computed:hold","successor_ref":"replacement"},{"unit_id":"replacement","state":"active","origin_refs":covered,"exposure_ref":"computed:replacement"},{"unit_id":"others","state":"active","origin_refs":others,"exposure_ref":"computed:others"}],"primary_validities":[],"primary_memberships":[],"financial_candidates":[],"definite_links":[{"link_id":"result-link","admitted_input_link_ref":"input-link","from_origin_ref":"anchor","to_origin_ref":"usage"}],"coverage_decisions":[{"coverage_id":"coverage","status":"accepted","unit_refs":["hold"],"monetary_origin_refs":["reserve","settle","usage"],"anchor_origin_refs":["anchor"],"definite_link_refs":["result-link"],"retained_component_refs":[],"reason_refs":[],"route":"budgeted","obligation_id":"obligation","validity_ref":"obligation","valuation_ref":"valuation","result_unit_ref":"replacement"}],"valuation_records":[{"valuation_id":"valuation","usage_origin_refs":["usage"],"complete":true,"wire_ref":"opaque:wire"}],"low_rollups":[],"continuity_result":"unestablished","failure_refs":[]});
    result["origins"] = json!(ids.iter().map(|id|json!({"origin_ref":id,"base_id":"base","evidence_atom_id":id,"active_high_unit_id":if covered.contains(id){"replacement"}else{"others"}})).collect::<Vec<_>>());
    for (obligation, r, s, u) in [
        ("obligation", "reserve", "settle", "usage"),
        ("other", "other-reserve", "other-settle", "other-usage"),
    ] {
        result["primary_validities"].as_array_mut().unwrap().push(json!({"obligation_id":obligation,"distinct_reservation_refs":[r],"distinct_settlement_refs":[s],"validity_result":"valid","applicable_check_refs":["computed:check"]}));
        result["primary_memberships"].as_array_mut().unwrap().push(json!({"obligation_id":obligation,"reservation_classes":[{"origin_refs":[r],"representative_origin_ref":r}],"settlement_classes":[{"origin_refs":[s],"representative_origin_ref":s}],"usage_origin_refs":[u]}));
    }
    json!({"documents":[input,result]})
}
fn rejects(mutator: impl FnOnce(&mut Value), reason: &str) {
    let mut b = bundle();
    mutator(&mut b);
    let error = validate_accounting_bundle(&b).unwrap_err();
    assert!(error.contains(reason), "expected {reason}, got {error}");
}
#[test]
fn exact_positive_and_witness_application() {
    validate_accounting_bundle(&bundle()).unwrap();
    rejects(
        |b| b["documents"][1]["coverage_decisions"][0]["validity_ref"] = json!("other"),
        "obligation/validity",
    );
    rejects(
        |b| {
            b["documents"][1]["coverage_decisions"][0]["validity_ref"] = json!("other");
            b["documents"][1]["coverage_decisions"][0]["obligation_id"] = json!("other");
        },
        "missing selected primary",
    );
    rejects(
        |b| {
            b["documents"][1]
                .as_object_mut()
                .unwrap()
                .remove("primary_memberships");
        },
        "Draft2020-12",
    );
    rejects(
        |b| {
            b["documents"][1]["primary_validities"][0]["distinct_reservation_refs"] =
                json!(["other-reserve"])
        },
        "representatives differ",
    );
    rejects(
        |b| b["documents"][1]["valuation_records"][0]["usage_origin_refs"] = json!(["other-usage"]),
        "usage differs",
    );
}
#[test]
fn exact_links_and_shared_grant_pair_application() {
    rejects(
        |b| b["documents"][1]["coverage_decisions"][0]["definite_link_refs"] = json!([]),
        "lacks cited exact link",
    );
    rejects(
        |b| {
            b["documents"][0]["exact_links"][0]["to_origin_ref"] = json!("anchor");
            b["documents"][1]["definite_links"][0]["to_origin_ref"] = json!("anchor");
        },
        "self-link",
    );
    // All endpoints are in the grant extent; this crossed pair is not admitted.
    rejects(
        |b| {
            b["documents"][0]["exact_links"][0]["to_origin_ref"] = json!("other-usage");
            b["documents"][1]["definite_links"][0]["to_origin_ref"] = json!("other-usage");
        },
        "pair not admitted",
    );
    rejects(
        |b| b["documents"][1]["definite_links"][0]["to_origin_ref"] = json!("other-usage"),
        "endpoints differ",
    );
    rejects(
        |b| b["documents"][0]["grants"][0]["admitted_payload"] = json!({"anything":"extent"}),
        "Draft2020-12",
    );
    // A separately admitted other-request pair still cannot release this anchor.
    rejects(
        |b| {
            b["documents"][0]["grants"][0]["admitted_payload"] =
                json!({"from":"anchor","to":"other-usage"});
            b["documents"][0]["exact_links"][0]["to_origin_ref"] = json!("other-usage");
            b["documents"][1]["definite_links"][0]["to_origin_ref"] = json!("other-usage");
        },
        "lacks cited exact link",
    );
}
#[test]
fn unlinked_anchor_and_genuine_unbudgeted_remain_distinct() {
    let mut b = bundle();
    let r = &mut b["documents"][1];
    r["coverage_decisions"] = json!([]);
    r["definite_links"] = json!([]);
    r["units"][0]["state"] = json!("active");
    r["units"][1]["state"] = json!("inactive");
    for o in r["origins"].as_array_mut().unwrap() {
        if o["active_high_unit_id"] == "replacement" {
            o["active_high_unit_id"] = json!("hold");
        }
    }
    b["documents"][0]["exact_links"] = json!([]);
    validate_accounting_bundle(&b).unwrap();
    let mut b = bundle();
    let r = &mut b["documents"][1];
    r["primary_validities"] = json!([]);
    r["primary_memberships"] = json!([]);
    r["unbudgeted_applicabilities"] = json!([{"applicability_id":"app","usage_origin_ref":"usage","raw_subject_ref":"opaque:raw","classification":"genuine_unbudgeted","dollar_origin_refs":[]}]);
    r["unbudgeted_completions"] = json!([{"completion_id":"completion","usage_origin_ref":"usage","usage_origin_refs":["usage"],"applicability_ref":"app","valuation_ref":"valuation","complete":true}]);
    let c = &mut r["coverage_decisions"][0];
    c["route"] = json!("unbudgeted_completion");
    c["completion_ref"] = json!("completion");
    c.as_object_mut().unwrap().remove("obligation_id");
    c.as_object_mut().unwrap().remove("validity_ref");
    validate_accounting_bundle(&b).unwrap();
}

#[test]
fn export_cli_subjects_when_requested() {
    let Ok(dest) = std::env::var("R9_SPEC_FIXTURE_OUT") else {
        return;
    };
    let dest = std::path::Path::new(&dest);
    std::fs::create_dir_all(dest).unwrap();
    let b = bundle();
    std::fs::write(
        dest.join("positive.json"),
        serde_json::to_vec_pretty(&b).unwrap(),
    )
    .unwrap();
    let mutations: Vec<(&str, Box<dyn Fn(&mut Value)>)> = vec![
        (
            "witness-swap",
            Box::new(|b| {
                b["documents"][1]["coverage_decisions"][0]["validity_ref"] = json!("other")
            }),
        ),
        (
            "redirected-validity",
            Box::new(|b| {
                b["documents"][1]["coverage_decisions"][0]["validity_ref"] = json!("other");
                b["documents"][1]["coverage_decisions"][0]["obligation_id"] = json!("other");
            }),
        ),
        (
            "uncited-link",
            Box::new(|b| {
                b["documents"][1]["coverage_decisions"][0]["definite_link_refs"] = json!([])
            }),
        ),
        (
            "self-link",
            Box::new(|b| {
                b["documents"][0]["exact_links"][0]["to_origin_ref"] = json!("anchor");
                b["documents"][1]["definite_links"][0]["to_origin_ref"] = json!("anchor");
            }),
        ),
        (
            "shared-grant-pair",
            Box::new(|b| {
                b["documents"][0]["exact_links"][0]["to_origin_ref"] = json!("other-usage");
                b["documents"][1]["definite_links"][0]["to_origin_ref"] = json!("other-usage");
            }),
        ),
        (
            "wrongrequest",
            Box::new(|b| {
                b["documents"][0]["grants"][0]["admitted_payload"] =
                    json!({"from":"anchor","to":"other-usage"});
                b["documents"][0]["exact_links"][0]["to_origin_ref"] = json!("other-usage");
                b["documents"][1]["definite_links"][0]["to_origin_ref"] = json!("other-usage");
            }),
        ),
    ];
    for (name, mutate) in mutations {
        let mut wrong = b.clone();
        mutate(&mut wrong);
        for d in wrong["documents"].as_array().unwrap() {
            pi_accounting_spec_check::validate_accounting_document(d).unwrap();
        }
        assert!(validate_accounting_bundle(&wrong).is_err());
        std::fs::write(
            dest.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&wrong).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn primary_exact_copy_is_admitted_evidence_not_value_equality() {
    let mut b = bundle();
    let mut copy = b["documents"][0]["origins"][1].clone();
    copy["origin_id"] = json!("reserve-copy");
    copy["original_position"] = json!(10);
    b["documents"][0]["origins"]
        .as_array_mut()
        .unwrap()
        .push(copy);
    let mut owner = b["documents"][1]["origins"][1].clone();
    owner["origin_ref"] = json!("reserve-copy");
    owner["evidence_atom_id"] = json!("reserve-copy");
    b["documents"][1]["origins"]
        .as_array_mut()
        .unwrap()
        .push(owner);
    for ptr in [
        "/documents/1/bases/0/origin_refs",
        "/documents/1/units/0/origin_refs",
        "/documents/1/units/1/origin_refs",
        "/documents/1/coverage_decisions/0/monetary_origin_refs",
    ] {
        b.pointer_mut(ptr)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(json!("reserve-copy"));
    }
    b["documents"][1]["primary_memberships"][0]["reservation_classes"][0]["origin_refs"] =
        json!(["reserve", "reserve-copy"]);
    assert!(
        validate_accounting_bundle(&b)
            .unwrap_err()
            .contains("exact-copy equivalence")
    );
    b["documents"][0]["initial_contracts"].as_array_mut().unwrap().push(json!({"contract_id":"copy-contract","reference_namespace":"dollar_ticket","duplicate_rule":"scope_exact_line","scope_id":"scope","grant_ref":"scope-grant"}));
    for o in b["documents"][0]["origins"].as_array_mut().unwrap() {
        if o["origin_id"] == "reserve" || o["origin_id"] == "reserve-copy" {
            o["initial_contract_ref"] = json!("copy-contract");
        }
    }
    for (id, kind) in [
        ("scope-grant", "initial_reference_scope"),
        ("copy-grant", "exact_copy_equivalence"),
    ] {
        b["documents"][0]["grants"].as_array_mut().unwrap().push(json!({"grant_id":id,"guarantee_kind":kind,"admission":"conditional_fixture","exact_payload_ref":"opaque:payload","covered_origin_refs":["reserve","reserve-copy"],"covered_extent_ref":"opaque:extent","authority_ref":"conditional","observer_or_harness_ref":"synthetic","assumption_refs":["copy"]}));
    }
    validate_accounting_bundle(&b).unwrap();
    b["documents"][0]["origins"][7]["raw_bytes_sha256"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    assert!(
        validate_accounting_bundle(&b)
            .unwrap_err()
            .contains("exact-copy equivalence")
    );
}
#[test]
fn changed_applicability_clauses_invalidate_actual_registered_consumers() {
    use pi_accounting_spec_check::{Dependencies, affected_consumers, consumer_digests, read_spec};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original = read_spec(&root).unwrap();
    let deps: Dependencies =
        serde_json::from_slice(&std::fs::read(root.join("spec/dependencies.json")).unwrap())
            .unwrap();
    let before = consumer_digests(&original, &deps).unwrap();
    for id in [
        "LOGICAL-PRIMARY",
        "DEFINITE-LINK",
        "WHOLE-COVERAGE",
        "F-PROJECTION",
        "SCHEMA-ACCOUNTING",
    ] {
        let mut changed = original.clone();
        changed
            .invariants
            .iter_mut()
            .find(|n| n.id == id)
            .unwrap()
            .text
            .push_str(" synthetic semantic alteration");
        let after = consumer_digests(&changed, &deps).unwrap();
        let affected = affected_consumers(&changed, &deps, &[id.to_owned()]).unwrap();
        for consumer in [
            "python-validator",
            "python-corpus-reference-consumer",
            "rust-corpus-reference-consumer",
            "independent-raw-assertions",
            "view-contract",
            "view-schema",
        ] {
            assert!(
                affected.iter().any(|v| v == consumer),
                "{id} omitted {consumer}"
            );
            assert_ne!(
                before[consumer], after[consumer],
                "{id} failed to invalidate {consumer}"
            );
        }
    }
}
#[test]
fn typed_retained_components_have_exact_active_owners() {
    let mut b = bundle();
    b["documents"][1]["retained_components"] =
        json!([{"component_id":"component","origin_ref":"settle","role":"charge","nano_usd":"0"}]);
    b["documents"][1]["units"][1]["retained_component_refs"] = json!(["component"]);
    b["documents"][1]["coverage_decisions"][0]["retained_component_refs"] = json!(["component"]);
    validate_accounting_bundle(&b).unwrap();
    let mut bad = b.clone();
    bad["documents"][1]["retained_components"]
        .as_array_mut()
        .unwrap()
        .push(b["documents"][1]["retained_components"][0].clone());
    assert!(validate_accounting_bundle(&bad).is_err());
    let mut bad = b.clone();
    bad["documents"][1]["retained_components"][0]["origin_ref"] = json!("missing");
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("dangling retained component origin")
    );
    let mut bad = b.clone();
    bad["documents"][1]["units"][1]["retained_component_refs"] = json!([]);
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("omitted by active origin owner")
    );
    let mut bad = b.clone();
    bad["documents"][1]["units"][2]["retained_component_refs"] = json!(["component"]);
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("wrong unit owner")
    );
    let mut bad = b.clone();
    bad["documents"][1]["coverage_decisions"][0]["retained_component_refs"] = json!(["missing"]);
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("dangling coverage retained component")
    );
    let mut bad = b.clone();
    bad["documents"][1]["retained_components"][0]["nano_usd"] = json!("00");
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("Draft2020-12")
    );
    let mut bad = b;
    bad["documents"][1]["retained_components"][0]["origin_ref"] = json!("other-settle");
    bad["documents"][1]["units"][1]["retained_component_refs"] = json!([]);
    bad["documents"][1]["units"][2]["retained_component_refs"] = json!(["component"]);
    assert!(
        validate_accounting_bundle(&bad)
            .unwrap_err()
            .contains("wrong coverage owner")
    );
}
