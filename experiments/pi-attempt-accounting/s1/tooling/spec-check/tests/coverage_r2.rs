//! R8-COVERAGE-01 spec-coherence controls on a self-contained two-request unbudgeted fixture
//! (o,s) and (p,q). Each negative names the predicate it must hit and is derived from the same
//! valid fixture, so a blanket rejection cannot satisfy it. Financial truth stays with the oracle.
use pi_accounting_spec_check::*;
use serde_json::{Value, json};

fn origin(id: &str, position: u64) -> Value {
    json!({"origin_id":id,"source_origin_id":"S","original_position":position,
        "raw_bytes_sha256":format!("{position:064x}"),"source_revision_ref":"raw:revision","initial_contract_ref":"c"})
}
fn grant(id: &str, from: &str, to: &str) -> Value {
    json!({"grant_id":id,"guarantee_kind":"exact_row_coreference","admission":"conditional_fixture",
        "exact_payload_ref":"fixture:payload","covered_origin_refs":[from,to],"covered_extent_ref":"fixture:extent",
        "authority_ref":"fixture:authority","observer_or_harness_ref":"fixture:harness",
        "assumption_refs":["fixture:assumptions"],"admitted_payload":{"from":from,"to":to}})
}
fn fixture() -> Value {
    let result_origin = |id: &str, unit: &str| json!({"origin_ref":id,"base_id":format!("base:{id}"),"evidence_atom_id":format!("atom:{id}"),"active_high_unit_id":unit});
    let base = |id: &str| json!({"base_id":format!("base:{id}"),"origin_refs":[id],"atomize":false,"atomize_fact_refs":[],"break_copies":false,"break_copies_fact_refs":[]});
    let unit = |id: &str, origins: Value, state: &str, successor: Option<&str>| {
        let mut u = json!({"unit_id":id,"state":state,"origin_refs":origins,"exposure_ref":format!("computed:{id}")});
        if let Some(s) = successor {
            u["successor_ref"] = json!(s);
        }
        u
    };
    let coverage = |n: &str, usage: &str, anchor: &str| json!({"coverage_id":format!("cover:{n}"),"route":"unbudgeted_completion","completion_ref":format!("done:{n}"),"status":"accepted",
        "unit_refs":[format!("u:{usage}"),format!("u:{anchor}")],"monetary_origin_refs":[usage],"anchor_origin_refs":[anchor],"definite_link_refs":[format!("d:{n}")],
        "valuation_ref":format!("val:{n}"),"result_unit_ref":format!("R:{n}"),"retained_component_refs":[],"reason_refs":[]});
    let applicability = |n: &str, usage: &str| json!({"applicability_id":format!("app:{n}"),"usage_origin_ref":usage,"raw_subject_ref":"raw:scope","classification":"genuine_unbudgeted","dollar_origin_refs":[]});
    let completion = |n: &str, usage: &str| json!({"completion_id":format!("done:{n}"),"usage_origin_ref":usage,"applicability_ref":format!("app:{n}"),"valuation_ref":format!("val:{n}"),"complete":true,"usage_origin_refs":[usage]});
    let valuation = |n: &str, usage: &str| json!({"valuation_id":format!("val:{n}"),"usage_origin_refs":[usage],"complete":true,"wire_ref":"computed:wire"});
    let link = |n: &str, from: &str, to: &str| json!({"link_id":format!("d:{n}"),"from_origin_ref":from,"to_origin_ref":to,"admitted_input_link_ref":format!("l:{n}")});
    json!({"documents":[
      {"kind":"accounting_input_proof","id":"input","version":"1.7-s1","lineage_id":"L",
       "origins":[origin("o",1),origin("s",2),origin("p",3),origin("q",4)],
       "initial_contracts":[{"contract_id":"c","reference_namespace":"unestablished","duplicate_rule":"same_origin_only"}],
       "grants":[grant("g1","o","s"),grant("g2","p","q")],"identity_bindings":[],
       "exact_links":[{"link_id":"l:1","from_origin_ref":"o","to_origin_ref":"s","grant_ref":"g1"},{"link_id":"l:2","from_origin_ref":"p","to_origin_ref":"q","grant_ref":"g2"}],
       "continuity":{"predecessor_subject_refs":[],"preserved_initial_contract_refs":[],"preserved_fact_refs":[],"exact_origin_map_ref":"raw:origin-map"}},
      {"kind":"accounting_result","id":"result","version":"1.7-s1","input_proof_ref":"input","lineage_id":"L",
       "origins":[result_origin("o","R:1"),result_origin("s","R:1"),result_origin("p","R:2"),result_origin("q","R:2")],
       "bases":[base("o"),base("s"),base("p"),base("q")],
       "units":[unit("u:o",json!(["o"]),"inactive",Some("R:1")),unit("u:s",json!(["s"]),"inactive",Some("R:1")),unit("R:1",json!(["o","s"]),"active",None),
                unit("u:p",json!(["p"]),"inactive",Some("R:2")),unit("u:q",json!(["q"]),"inactive",Some("R:2")),unit("R:2",json!(["p","q"]),"active",None)],
       "primary_validities":[],"financial_candidates":[],"low_rollups":[],"continuity_result":"unestablished","failure_refs":[],
       "definite_links":[link("1","o","s"),link("2","p","q")],
       "unbudgeted_applicabilities":[applicability("1","o"),applicability("2","p")],
       "unbudgeted_completions":[completion("1","o"),completion("2","p")],
       "valuation_records":[valuation("1","o"),valuation("2","p")],
       "coverage_decisions":[coverage("1","o","s"),coverage("2","p","q")]}]})
}
fn result(b: &mut Value) -> &mut Value {
    &mut b["documents"][1]
}
fn rejected(mutate: impl FnOnce(&mut Value), predicate: &str) {
    let mut b = fixture();
    mutate(&mut b);
    let error = validate_accounting_bundle(&b).unwrap_err();
    assert!(error.contains(predicate), "wanted `{predicate}`, got: {error}");
}

#[test]
fn exact_unbudgeted_positive_survives() {
    validate_accounting_bundle(&fixture()).unwrap();
}

#[test]
fn anchor_release_needs_the_covered_request_link() {
    rejected(|b| result(b)["coverage_decisions"][0]["definite_link_refs"] = json!([]), "released anchor lacks cited exact link");
    // another request's link is the only one cited
    rejected(|b| result(b)["coverage_decisions"][0]["definite_link_refs"] = json!(["d:2"]), "cited definite link d:2 does not join a covered anchor to the covered request");
    // the covered link plus another request's link: the foreign link is not this coverage's authority
    rejected(
        |b| result(b)["coverage_decisions"][0]["definite_link_refs"] = json!(["d:1", "d:2"]),
        "cited definite link d:2 does not join a covered anchor to the covered request",
    );
    rejected(|b| result(b)["coverage_decisions"][0]["definite_link_refs"] = json!(["d:1", "d:missing"]), "dangling coverage definite-link identity");
}

#[test]
fn self_and_rewritten_links_are_rejected() {
    rejected(
        |b| {
            b["documents"][0]["exact_links"][0]["to_origin_ref"] = json!("o");
            result(b)["definite_links"][0]["to_origin_ref"] = json!("o");
        },
        "self-link cannot establish co-reference",
    );
    // the result may not redirect an admitted link to a different endpoint
    rejected(|b| result(b)["definite_links"][0]["to_origin_ref"] = json!("q"), "result link endpoints differ from exact admitted link");
    rejected(|b| result(b)["definite_links"][0]["admitted_input_link_ref"] = json!("l:2"), "result link endpoints differ from exact admitted link");
}

#[test]
fn an_anchor_cannot_be_smuggled_in_as_a_monetary_origin() {
    // retiring s as `monetary` instead of anchor skips the link requirement entirely
    rejected(
        |b| {
            let c = &mut result(b)["coverage_decisions"][0];
            c["monetary_origin_refs"] = json!(["o", "s"]);
            c["anchor_origin_refs"] = json!([]);
            c["definite_link_refs"] = json!([]);
        },
        "coverage monetary origins differ from the selected primary/usage inventory",
    );
}

#[test]
fn valuation_usage_and_completion_must_name_the_covered_request() {
    rejected(|b| result(b)["valuation_records"][0]["usage_origin_refs"] = json!(["s"]), "coverage valuation usage differs");
    rejected(|b| result(b)["valuation_records"][0]["usage_origin_refs"] = json!(["p"]), "coverage valuation usage differs");
    rejected(|b| result(b)["coverage_decisions"][0]["valuation_ref"] = json!("val:2"), "unbudgeted applicability/completion origin or valuation mismatch");
    rejected(|b| result(b)["valuation_records"][0]["complete"] = json!(false), "coverage requires complete valuation witness");
}

#[test]
fn partial_and_overlapping_coverage_are_rejected() {
    rejected(|b| result(b)["coverage_decisions"][0]["unit_refs"] = json!(["u:o"]), "coverage is not whole-unit complete");
    rejected(
        |b| {
            let c = &mut result(b)["coverage_decisions"][1];
            c["monetary_origin_refs"] = json!(["o"]);
            c["completion_ref"] = json!("done:1");
            c["valuation_ref"] = json!("val:1");
            c["unit_refs"] = json!(["u:o", "u:s"]);
            c["anchor_origin_refs"] = json!(["s"]);
            c["definite_link_refs"] = json!(["d:1"]);
            c["result_unit_ref"] = json!("R:1");
        },
        "overlapping accepted coverage",
    );
}

#[test]
fn a_hold_is_released_only_by_an_accepted_coverage_into_its_successor() {
    rejected(|b| result(b)["coverage_decisions"] = json!([result(b)["coverage_decisions"][1].clone()]), "inactive unit u:o is not retired into its successor");
    rejected(|b| result(b)["coverage_decisions"][0]["status"] = json!("rejected"), "inactive unit u:o is not retired into its successor");
    rejected(|b| result(b)["units"][0]["successor_ref"] = json!("R:2"), "inactive unit u:o is not retired into its successor");
    rejected(|b| { result(b)["units"][0].as_object_mut().unwrap().remove("successor_ref"); }, "inactive unit u:o is not retired into its successor");
}

// BASE §3 OriginBase: without an admitted scope each origin is its own base; a multi-origin base
// needs an established (scope-granted) contract or an admitted exact link / identity binding
// joining its members.
fn group(b: &mut Value, members: &[&str]) {
    let r = result(b);
    r["bases"].as_array_mut().unwrap().retain(|base| !members.iter().any(|m| base["origin_refs"][0] == *m));
    r["bases"].as_array_mut().unwrap().push(json!({"base_id":"base:group","origin_refs":members,"atomize":false,"atomize_fact_refs":[],"break_copies":false,"break_copies_fact_refs":[]}));
    for o in r["origins"].as_array_mut().unwrap() {
        if members.iter().any(|m| o["origin_ref"] == *m) {
            o["base_id"] = json!("base:group");
        }
    }
}

#[test]
fn a_multi_origin_base_needs_a_scope_a_link_or_an_identity_binding() {
    let groups_error = "groups 2 origins under an unestablished contract (or without an admitted scope) with no admitted exact link or identity binding joining them";
    // unestablished contract, members not joined by any admitted link or binding
    rejected(|b| group(b, &["o", "p"]), groups_error);
    rejected(|b| group(b, &["s", "q"]), groups_error);
    // positives from the same grouping step: joined by the admitted exact link o->s
    let mut linked = fixture();
    group(&mut linked, &["o", "s"]);
    validate_accounting_bundle(&linked).unwrap();
    // joined by shared producer identity binding (with its atomization facts)
    let mut bound = fixture();
    bound["documents"][0]["grants"].as_array_mut().unwrap().extend([
        json!({"grant_id":"bind-o","guarantee_kind":"producer_attempt_binding","admission":"conditional_fixture","exact_payload_ref":"fixture:payload","covered_origin_refs":["o"],"covered_extent_ref":"fixture:extent","authority_ref":"fixture:authority","observer_or_harness_ref":"fixture:harness","assumption_refs":["fixture:assumptions"]}),
        json!({"grant_id":"bind-p","guarantee_kind":"producer_attempt_binding","admission":"conditional_fixture","exact_payload_ref":"fixture:payload","covered_origin_refs":["p"],"covered_extent_ref":"fixture:extent","authority_ref":"fixture:authority","observer_or_harness_ref":"fixture:harness","assumption_refs":["fixture:assumptions"]}),
    ]);
    bound["documents"][0]["identity_bindings"] = json!([
        {"origin_ref":"o","producer_attempt_id":"attempt","grant_ref":"bind-o"},
        {"origin_ref":"p","producer_attempt_id":"attempt","grant_ref":"bind-p"}]);
    group(&mut bound, &["o", "p"]);
    {
        let base = result(&mut bound)["bases"].as_array_mut().unwrap().last_mut().unwrap();
        base["atomize"] = json!(true);
        base["atomize_fact_refs"] = json!(["bind-o", "bind-p"]);
    }
    validate_accounting_bundle(&bound).unwrap();
    // different attempt ids do not join
    let mut apart = bound.clone();
    apart["documents"][0]["identity_bindings"][1]["producer_attempt_id"] = json!("other");
    let error = validate_accounting_bundle(&apart).unwrap_err();
    assert!(error.contains(groups_error), "{error}");
    // an admitted scope (established contract with its grant) permits the grouping itself
    let mut scoped = fixture();
    scoped["documents"][0]["initial_contracts"][0] = json!({"contract_id":"c","reference_namespace":"dollar_ticket","scope_id":"scope","grant_ref":"scope-grant","duplicate_rule":"same_origin_only"});
    scoped["documents"][0]["grants"].as_array_mut().unwrap().push(json!({"grant_id":"scope-grant","guarantee_kind":"initial_reference_scope","admission":"conditional_fixture","exact_payload_ref":"fixture:payload","covered_origin_refs":["o","s","p","q"],"covered_extent_ref":"fixture:extent","authority_ref":"fixture:authority","observer_or_harness_ref":"fixture:harness","assumption_refs":["fixture:assumptions"]}));
    group(&mut scoped, &["o", "p"]);
    validate_accounting_bundle(&scoped).unwrap();
}
