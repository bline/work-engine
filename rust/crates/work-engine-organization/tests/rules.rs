//! Rule tests 1A-2 to 1A-7, 1A-11 (refusals), 1A-12 and 1A-13. Expected values in these tests are
//! written by hand from the contract and amendment 5.1-A1 rules, independently of the
//! implementation, so they check rules, not stability. (Determinism and goldens are in
//! `stability.rs`.)

mod common;

use common::*;
use serde_json::{Value, json};
use work_engine_organization::{
    EnforceStatus, OrganizationalRequirements, Profile, RelationshipKind, UnresolvedKind,
    ValidationCode, enforceability, view,
};

fn refusal(value: &Value) -> ValidationCode {
    OrganizationalRequirements::from_json_bytes(&bytes(value))
        .expect_err("input must be refused")
        .code
}

fn unresolved_of(value: &Value) -> Vec<(String, String, Vec<String>)> {
    let result = run(value);
    assert!(result.candidates.is_empty(), "no candidate expected");
    result
        .unresolved
        .iter()
        .map(|u| {
            (
                u.kind.as_str().to_owned(),
                u.subject.clone(),
                u.conditions.iter().map(|c| c.as_str().to_owned()).collect(),
            )
        })
        .collect()
}

fn set(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

// ---------------------------------------------------------------------------------------------
// 1A-2 identity (corrected, amendment section 8)

#[test]
fn identity_a_unneeded_authority_entry_changes_digests_but_not_rule_derived_sets() {
    let before = candidate(&base());
    let mut changed = base();
    changed["requiredAuthorities"][1]["permission"] = json!("audit_write");
    let after = candidate(&changed);
    assert_ne!(
        parse(&base()).requirements_digest(),
        parse(&changed).requirements_digest()
    );
    assert_ne!(before.content_digest(), after.content_digest());
    assert_eq!(before.relationships(), after.relationships());
    assert_eq!(
        before.separation_requirements(),
        after.separation_requirements()
    );
    assert_eq!(
        before.component_binding_requirements(),
        after.component_binding_requirements()
    );
    assert_ne!(
        before.authority_requirements(),
        after.authority_requirements()
    );
}

#[test]
fn identity_b_adding_an_authority_entry_changes_both_digests_and_no_relationship() {
    let before = candidate(&base());
    let mut changed = base();
    changed["requiredAuthorities"]
        .as_array_mut()
        .unwrap()
        .push(authority("auth-new", "observer-2", "audit_read", None));
    let after = candidate(&changed);
    assert_ne!(
        parse(&base()).requirements_digest(),
        parse(&changed).requirements_digest()
    );
    assert_ne!(before.content_digest(), after.content_digest());
    assert_eq!(before.relationships(), after.relationships());
    assert_eq!(
        before.component_binding_requirements(),
        after.component_binding_requirements()
    );
}

#[test]
fn identity_c_changing_the_only_satisfying_permission_gives_no_candidate() {
    let mut changed = base();
    changed["requiredAuthorities"][0]["permission"] = json!("accept_claims");
    assert_ne!(
        parse(&base()).requirements_digest(),
        parse(&changed).requirements_digest()
    );
    assert_eq!(
        unresolved_of(&changed),
        vec![(
            "unstated_authority_dependency".to_owned(),
            "principal:owner-a".to_owned(),
            set(&["cond-builder", "cond-campaign"])
        )]
    );
}

#[test]
fn identity_d_a_changed_rule_input_changes_the_content_digest() {
    let before = candidate(&base());
    let mut changed = base();
    changed["acceptanceConditions"]
        .as_array_mut()
        .unwrap()
        .push(condition(
            "cond-extra",
            "ob-a",
            "builder_projection",
            "builder",
            "owner-a",
        ));
    let after = candidate(&changed);
    assert_ne!(before.content_digest(), after.content_digest());
    assert_ne!(before.relationships(), after.relationships());
}

#[test]
fn identity_changes_on_every_listed_input_class() {
    let reference = candidate(&base()).content_digest();
    let mutations: Vec<(&str, &str, Value)> = vec![
        ("obligation id", "/obligations/0/id", json!("ob-z")),
        (
            "scope digest",
            "/obligations/0/scope/digest",
            json!(DIGEST_B),
        ),
        (
            "result contract version",
            "/obligations/0/resultContract/version",
            json!(2),
        ),
        (
            "result contract id",
            "/obligations/0/resultContract/id",
            json!("other-review"),
        ),
        (
            "boundary",
            "/acceptanceConditions/0/consumptionBoundary",
            json!("builder_projection_2"),
        ),
        (
            "proposition",
            "/acceptanceConditions/0/proposition",
            json!("changed"),
        ),
        (
            "coveredState",
            "/acceptanceConditions/0/coveredState",
            json!("other-state"),
        ),
        (
            "unestablishedRoute",
            "/acceptanceConditions/0/unestablishedRoute",
            json!("other-route"),
        ),
        (
            "continuity",
            "/acceptanceConditions/0/normative/continuity",
            json!("retained"),
        ),
        (
            "capabilities",
            "/acceptanceConditions/0/normative/capabilities",
            json!(["read-only"]),
        ),
        (
            "observerClass",
            "/acceptanceConditions/0/normative/observerClass",
            json!("other-class"),
        ),
        (
            "independence",
            "/acceptanceConditions/0/normative/independence",
            json!({"from": ["state.builder_context"]}),
        ),
        (
            "constraint class",
            "/acceptanceConditions/0/realizationConstraints",
            json!([{"dimension": "provider_family", "allowed": ["anthropic"]}]),
        ),
    ];
    for (name, pointer, value) in mutations {
        let mut changed = base();
        // A boundary rename must keep the acceptor dependency stated; auth-1 covers the whole
        // scope, so closure still holds. An obligation rename needs both conditions renamed.
        if pointer == "/obligations/0/id" {
            for index in 0..2 {
                changed["acceptanceConditions"][index]["obligation"] = json!("ob-z");
            }
        }
        // Continuity and observerClass must agree across a vantage's conditions (F4), so the
        // mutated value is set on both conditions of the obligation.
        if pointer.ends_with("/continuity") || pointer.ends_with("/observerClass") {
            let other = pointer.replace("/0/", "/1/");
            *changed.pointer_mut(&other).unwrap() = value.clone();
        }
        match changed.pointer_mut(pointer) {
            Some(slot) => *slot = value,
            None => {
                let (parent, key) = pointer.rsplit_once('/').unwrap();
                changed.pointer_mut(parent).unwrap()[key] = value;
            }
        }
        assert_ne!(
            candidate(&changed).content_digest(),
            reference,
            "{name} must change contentDigest"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 1A-3 conditions (F4)

fn two_condition_vantage() -> Value {
    let mut first = condition("c1", "ob-a", "builder_projection", "builder", "owner-a");
    first["normative"]["capabilities"] = json!(["x"]);
    first["normative"]["independence"] = json!({"from": ["state.builder_context"]});
    first["realizationConstraints"] = json!([
        {"dimension": "provider_family", "allowed": ["anthropic", "other"]},
        {"dimension": "execution_confinement", "allowed": ["guarded_sandbox"]}
    ]);
    let mut second = condition(
        "c2",
        "ob-a",
        "campaign_terminalization",
        "campaign",
        "owner-a",
    );
    second["normative"]["capabilities"] = json!(["y"]);
    second["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": ["anthropic"]}]);
    document(
        vec![obligation("ob-a", DIGEST_A)],
        vec![first, second],
        vec![authority("auth-1", "owner-a", "accept_claim", None)],
    )
}

#[test]
fn heterogeneous_conditions_are_preserved_and_combined_by_the_f4_table() {
    let result = candidate(&two_condition_vantage());
    // Both conditions kept individually, with their own content.
    assert_eq!(result.conditions().len(), 2);
    let c1 = &result.conditions()[0];
    let c2 = &result.conditions()[1];
    assert_eq!(c1.id.as_str(), "c1");
    assert_eq!(c1.normative.capabilities.len(), 1);
    assert!(c1.normative.independence.is_some());
    assert_eq!(c2.id.as_str(), "c2");
    assert!(c2.normative.independence.is_none());
    assert_eq!(c1.consumer, "vantage-builder");
    assert_eq!(c2.consumer, "component:campaign");

    // F4: continuity same -> fresh_initial; capabilities union {x, y}; independence union of
    // `from` (only c1 states it); provider_family: {anthropic, other} n {anthropic} = {anthropic};
    // execution_confinement: only c1 states it, so it is not narrowed by c2.
    assert_eq!(result.vantage_requirements().len(), 1);
    let vantage = &result.vantage_requirements()[0];
    assert_eq!(vantage.vantage, "vantage:ob-a");
    assert_eq!(vantage.effective.continuity.as_str(), "fresh_initial");
    let capabilities: Vec<&str> = vantage
        .effective
        .capabilities
        .iter()
        .map(|c| c.as_str())
        .collect();
    assert_eq!(capabilities, ["x", "y"]);
    assert_eq!(
        vantage.effective.independence.as_ref().unwrap().len(),
        1,
        "independence is the union of from"
    );
    let constraints: Vec<(&str, Vec<&str>)> = vantage
        .effective_constraints
        .iter()
        .map(|c| {
            (
                c.dimension.as_str(),
                c.allowed.iter().map(|a| a.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        constraints,
        [
            ("execution_confinement", vec!["guarded_sandbox"]),
            ("provider_family", vec!["anthropic"])
        ]
    );
    assert_eq!(
        vantage
            .derived_from
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>(),
        ["c1", "c2"]
    );
}

#[test]
fn contradictory_conditions_give_no_candidate_and_a_condition_conflict() {
    let mut continuity = two_condition_vantage();
    continuity["acceptanceConditions"][1]["normative"]["continuity"] = json!("retained");
    assert_eq!(
        unresolved_of(&continuity),
        vec![(
            "condition_conflict".to_owned(),
            "continuity".to_owned(),
            set(&["c1", "c2"])
        )]
    );

    let mut observer = two_condition_vantage();
    observer["acceptanceConditions"][1]["normative"]["observerClass"] = json!("other-class");
    assert_eq!(
        unresolved_of(&observer),
        vec![(
            "condition_conflict".to_owned(),
            "observerClass".to_owned(),
            set(&["c1", "c2"])
        )]
    );

    let mut disjoint = two_condition_vantage();
    disjoint["acceptanceConditions"][1]["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": ["openai"]}]);
    assert_eq!(
        unresolved_of(&disjoint),
        vec![(
            "condition_conflict".to_owned(),
            "realization:provider_family".to_owned(),
            set(&["c1", "c2"])
        )]
    );

    // A conflict at one vantage names only that vantage's conditions.
    let mut two = document(
        vec![obligation("ob-a", DIGEST_A), obligation("ob-b", DIGEST_B)],
        vec![
            condition("a1", "ob-a", "builder_projection", "builder", "owner-a"),
            condition(
                "a2",
                "ob-a",
                "campaign_terminalization",
                "campaign",
                "owner-a",
            ),
            condition("b1", "ob-b", "builder_projection", "builder", "owner-a"),
        ],
        vec![authority("auth-1", "owner-a", "accept_claim", None)],
    );
    two["acceptanceConditions"][1]["normative"]["continuity"] = json!("retained");
    assert_eq!(
        unresolved_of(&two),
        vec![(
            "condition_conflict".to_owned(),
            "continuity".to_owned(),
            set(&["a1", "a2"])
        )]
    );
}

#[test]
fn identical_conditions_under_distinct_ids_are_both_kept() {
    let mut value = base();
    let copy = {
        let mut copy = value["acceptanceConditions"][0].clone();
        copy["id"] = json!("cond-builder-copy");
        copy
    };
    value["acceptanceConditions"]
        .as_array_mut()
        .unwrap()
        .push(copy);
    let result = candidate(&value);
    let ids: Vec<&str> = result.conditions().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["cond-builder", "cond-builder-copy", "cond-campaign"]);
    // Distinct trace entries: each copy has its own condition target rows.
    for id in ["cond-builder", "cond-builder-copy"] {
        assert!(
            result
                .requirement_trace()
                .iter()
                .any(|row| row.target.kind == "condition" && row.target.id == id)
        );
    }
}

#[test]
fn builder_and_campaign_conditions_on_one_obligation_give_the_amended_counts() {
    let result = candidate(&base());
    let count = |kind: RelationshipKind| {
        result
            .relationships()
            .iter()
            .filter(|r| r.kind == kind)
            .count()
    };
    assert_eq!(count(RelationshipKind::ConsumesAt), 2);
    assert_eq!(count(RelationshipKind::AcceptsClaim), 2);
    assert_eq!(count(RelationshipKind::Terminalizes), 1);
}

#[test]
fn an_unknown_consumer_class_is_unresolved() {
    let mut value = base();
    value["acceptanceConditions"][1]["consumerClass"] = json!("operator");
    assert_eq!(
        unresolved_of(&value),
        vec![(
            "unknown_consumer_class".to_owned(),
            "consumerClass".to_owned(),
            set(&["cond-campaign"])
        )]
    );
}

// ---------------------------------------------------------------------------------------------
// 1A-4 enforceability (rows are an exhaustive match in the source; these check the values)

fn status_of(
    report: &work_engine_organization::EnforceabilityReport,
    property: &str,
) -> Vec<EnforceStatus> {
    report
        .rows
        .iter()
        .filter(|row| row.property == property)
        .map(|row| row.status)
        .collect()
}

#[test]
fn enforceability_rows_cover_every_property_and_dimension() {
    let mut value = two_condition_vantage();
    value["acceptanceConditions"][0]["normative"]["capabilities"] = json!([]);
    let c = candidate(&value);
    let report = enforceability(&c, Profile::FixedSliceReviewV1);
    for property in [
        "continuity",
        "mutation",
        "capabilities",
        "integrity",
        "evidenceCustody",
        "observerClass",
    ] {
        assert_eq!(
            status_of(&report, property).len(),
            2,
            "one {property} row per condition"
        );
    }
    // c1 has independence and both constraint dimensions; c2 has one constraint dimension.
    assert_eq!(status_of(&report, "independence").len(), 1);
    assert_eq!(status_of(&report, "realization:provider_family").len(), 2);
    assert_eq!(
        status_of(&report, "realization:execution_confinement").len(),
        1
    );
    // Rows are sorted by (condition, property).
    let keys: Vec<(&str, &str)> = report
        .rows
        .iter()
        .map(|r| (r.scope.as_str(), r.property.as_str()))
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

#[test]
fn enforceability_none_rows_for_unsupported_requirements() {
    let supported_base = enforceability(&candidate(&base()), Profile::FixedSliceReviewV1);
    assert_eq!(
        status_of(&supported_base, "continuity"),
        [EnforceStatus::Supported; 2]
    );
    assert_eq!(
        status_of(&supported_base, "observerClass"),
        [EnforceStatus::SupportedWithRealizationMapping; 2]
    );
    assert_eq!(
        status_of(&supported_base, "capabilities"),
        [EnforceStatus::Supported; 2]
    );

    let cases: Vec<(&str, &str, Value)> = vec![
        (
            "capabilities",
            "/normative/capabilities",
            json!(["read-only"]),
        ),
        ("continuity", "/normative/continuity", json!("retained")),
        (
            "observerClass",
            "/normative/observerClass",
            json!("unmapped-class"),
        ),
        (
            "independence",
            "/normative/independence",
            json!({"from": ["state.builder_context"]}),
        ),
    ];
    for (property, pointer, value) in cases {
        let mut changed = base();
        if property == "continuity" || property == "observerClass" {
            *changed["acceptanceConditions"][1]
                .pointer_mut(pointer)
                .unwrap() = value.clone();
        }
        let slot = changed["acceptanceConditions"][0].pointer_mut(pointer);
        match slot {
            Some(slot) => *slot = value,
            None => {
                let (parent, key) = pointer.rsplit_once('/').unwrap();
                changed["acceptanceConditions"][0]
                    .pointer_mut(parent)
                    .unwrap()[key] = value;
            }
        }
        let report = enforceability(&candidate(&changed), Profile::FixedSliceReviewV1);
        assert!(
            status_of(&report, property).contains(&EnforceStatus::None),
            "{property} must have a none row"
        );
        assert!(!report.executable());
    }

    // execution_confinement: a class other than guarded_sandbox has no mechanism.
    let mut confinement = base();
    confinement["acceptanceConditions"][0]["realizationConstraints"] =
        json!([{"dimension": "execution_confinement", "allowed": ["host_process"]}]);
    let report = enforceability(&candidate(&confinement), Profile::FixedSliceReviewV1);
    assert_eq!(
        status_of(&report, "realization:execution_confinement"),
        [EnforceStatus::None]
    );
}

#[test]
fn separation_rows_are_none_in_v1_so_no_candidate_is_executable() {
    let c = candidate(&base());
    let report = enforceability(&c, Profile::FixedSliceReviewV1);
    let separation = status_of(&report, "separation");
    assert_eq!(separation, [EnforceStatus::None; 2]);
    assert!(!report.executable());
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.property != "separation" || row.status != EnforceStatus::Supported)
    );
}

// ---------------------------------------------------------------------------------------------
// 1A-5 schema refusals and 1A-11 canonical-normalization refusals

#[test]
fn unknown_fields_standing_fields_and_realization_identities_are_refused() {
    let cases: Vec<(&str, &str, Value)> = vec![
        ("top-level unknown", "/extra", json!(1)),
        ("standing field", "/standing", json!("accepted")),
        ("status field", "/status", json!("accepted")),
        ("problemScope unknown", "/problemScope/model", json!("x")),
        ("obligation unknown", "/obligations/0/owner", json!("x")),
        (
            "condition unknown",
            "/acceptanceConditions/0/status",
            json!("satisfied"),
        ),
        (
            "normative realization identity field",
            "/acceptanceConditions/0/normative/observer",
            json!("review-execution-evidence"),
        ),
        (
            "normative model slug",
            "/acceptanceConditions/0/normative/model",
            json!("claude-opus-5-5"),
        ),
        (
            "independence evidence field",
            "/acceptanceConditions/0/normative/independence",
            json!({"from": ["state.builder_context"], "evidence": "review-history"}),
        ),
        (
            "authority grant digest",
            "/requiredAuthorities/0/grantDigest",
            json!(DIGEST_A),
        ),
        (
            "authority decision scope",
            "/requiredAuthorities/0/scope/decisionScope",
            json!("x"),
        ),
        ("policy unknown", "/organizationPolicy/selects", json!("x")),
    ];
    for (name, pointer, value) in cases {
        let mut changed = base();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        let parent_value = if parent.is_empty() {
            &mut changed
        } else {
            changed.pointer_mut(parent).unwrap()
        };
        parent_value[key] = value;
        // For the "independence" case the key is a normal member with a bad child, so it is
        // reported at the child; every other case is an unknown field.
        assert!(
            matches!(refusal(&changed), ValidationCode::UnknownField { .. }),
            "{name}"
        );
    }
}

#[test]
fn concrete_realization_and_observer_identities_are_not_classes() {
    for identity in [
        "claude-cli-json-v1",
        "rp0-guarded-claude-v1",
        "review-execution-evidence",
    ] {
        let mut as_observer = base();
        as_observer["acceptanceConditions"][0]["normative"]["observerClass"] = json!(identity);
        assert!(matches!(
            refusal(&as_observer),
            ValidationCode::RealizationIdentity { .. }
        ));
        let mut as_allowed = base();
        as_allowed["acceptanceConditions"][0]["realizationConstraints"] =
            json!([{"dimension": "provider_family", "allowed": [identity]}]);
        assert!(matches!(
            refusal(&as_allowed),
            ValidationCode::RealizationIdentity { .. }
        ));
    }
}

#[test]
fn literals_dimensions_and_missing_members_are_refused() {
    let mut value = base();
    value["obligations"][0]["kind"] = json!("other_review");
    assert!(matches!(
        refusal(&value),
        ValidationCode::InvalidValue { .. }
    ));
    let mut value = base();
    value["organizationPolicy"]["profile"] = json!("fixed-slice-review-v2");
    assert!(matches!(
        refusal(&value),
        ValidationCode::InvalidValue { .. }
    ));
    let mut value = base();
    value["organizationPolicy"]["mode"] = json!("adaptive");
    assert!(matches!(
        refusal(&value),
        ValidationCode::InvalidValue { .. }
    ));
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] =
        json!([{"dimension": "model_slug", "allowed": ["x"]}]);
    assert!(matches!(
        refusal(&value),
        ValidationCode::InvalidValue { .. }
    ));
    let mut value = base();
    value["acceptanceConditions"][0]["normative"]["independence"] =
        json!({"from": ["state.review_history"]});
    assert!(matches!(
        refusal(&value),
        ValidationCode::InvalidValue { .. }
    ));
    let mut value = base();
    value.as_object_mut().unwrap().remove("requiredAuthorities");
    assert!(matches!(
        refusal(&value),
        ValidationCode::MissingField { .. }
    ));
    let mut value = base();
    value["obligations"][0]["resultContract"]["version"] = json!(0);
    assert!(matches!(
        refusal(&value),
        ValidationCode::InvalidValue { .. }
    ));
    let mut value = base();
    value["problemScope"]["sliceNumber"] = json!(-1);
    assert!(matches!(
        refusal(&value),
        ValidationCode::InvalidValue { .. }
    ));
    let mut value = base();
    value["acceptanceConditions"][0]["proposition"] = json!("");
    assert!(matches!(refusal(&value), ValidationCode::EmptyString));
    let mut value = base();
    value["obligations"][0]["scope"]["digest"] = json!("AAAA");
    assert!(matches!(refusal(&value), ValidationCode::InvalidDigest));
    let mut value = base();
    value["obligations"][0]["scope"]["digest"] = json!(DIGEST_A.to_uppercase());
    assert!(matches!(refusal(&value), ValidationCode::InvalidDigest));
}

#[test]
fn referential_integrity_and_non_empty_collections_are_enforced() {
    let mut value = base();
    value["acceptanceConditions"][0]["obligation"] = json!("ob-missing");
    assert!(matches!(
        refusal(&value),
        ValidationCode::UnknownObligation { .. }
    ));
    let mut value = base();
    value["obligations"]
        .as_array_mut()
        .unwrap()
        .push(obligation("ob-lonely", DIGEST_B));
    assert!(matches!(
        refusal(&value),
        ValidationCode::ObligationWithoutCondition { .. }
    ));
    let mut value = base();
    value["obligations"] = json!([]);
    assert!(matches!(
        refusal(&value),
        ValidationCode::EmptyCollection { .. }
    ));
    let mut value = base();
    value["acceptanceConditions"] = json!([]);
    assert!(matches!(
        refusal(&value),
        ValidationCode::EmptyCollection { .. }
    ));
}

#[test]
fn canonical_normalization_refusals_are_typed() {
    // Duplicate ids.
    let mut value = base();
    value["obligations"]
        .as_array_mut()
        .unwrap()
        .push(obligation("ob-a", DIGEST_B));
    assert!(matches!(
        refusal(&value),
        ValidationCode::DuplicateId { .. }
    ));
    let mut value = base();
    let copy = value["acceptanceConditions"][0].clone();
    value["acceptanceConditions"]
        .as_array_mut()
        .unwrap()
        .push(copy);
    assert!(matches!(
        refusal(&value),
        ValidationCode::DuplicateId { .. }
    ));
    let mut value = base();
    let copy = value["requiredAuthorities"][0].clone();
    value["requiredAuthorities"]
        .as_array_mut()
        .unwrap()
        .push(copy);
    assert!(matches!(
        refusal(&value),
        ValidationCode::DuplicateId { .. }
    ));
    // Duplicate set members.
    for (pointer, value) in [
        ("normative/capabilities", json!(["a", "a"])),
        (
            "normative/independence",
            json!({"from": ["state.builder_context", "state.builder_context"]}),
        ),
    ] {
        let mut changed = base();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        changed["acceptanceConditions"][0][parent][key] = value;
        assert!(matches!(
            refusal(&changed),
            ValidationCode::DuplicateMember { .. }
        ));
    }
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": ["a", "a"]}]);
    assert!(matches!(
        refusal(&value),
        ValidationCode::DuplicateMember { .. }
    ));
    let mut value = base();
    value["requiredAuthorities"][0]["scope"] = json!({"boundaries": ["b", "b"]});
    assert!(matches!(
        refusal(&value),
        ValidationCode::DuplicateMember { .. }
    ));
    // Duplicate constraint dimension.
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] = json!([
        {"dimension": "provider_family", "allowed": ["a"]},
        {"dimension": "provider_family", "allowed": ["b"]}
    ]);
    assert!(matches!(
        refusal(&value),
        ValidationCode::DuplicateDimension { .. }
    ));
    // Present-but-empty sets where empty would mean absent.
    let mut value = base();
    value["acceptanceConditions"][0]["normative"]["independence"] = json!({"from": []});
    assert!(matches!(refusal(&value), ValidationCode::EmptySet));
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] = json!([]);
    assert!(matches!(refusal(&value), ValidationCode::EmptySet));
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": []}]);
    assert!(matches!(refusal(&value), ValidationCode::EmptySet));
    let mut value = base();
    value["requiredAuthorities"][0]["scope"] = json!({"boundaries": []});
    assert!(matches!(refusal(&value), ValidationCode::EmptySet));
    // IDs outside the syntax.
    for bad in ["Ob-a", "-ob", "ob a", "", &"a".repeat(129), "ob/a"] {
        let mut value = base();
        value["problemScope"]["runId"] = json!(bad);
        assert!(
            matches!(refusal(&value), ValidationCode::InvalidId(_)),
            "{bad:?}"
        );
    }
    // A 128-byte id is accepted.
    let mut value = base();
    value["problemScope"]["runId"] = json!("a".repeat(128));
    parse(&value);
}

fn refusal_of_text(text: &str) -> ValidationCode {
    OrganizationalRequirements::from_json_bytes(text.as_bytes())
        .expect_err("input must be refused")
        .code
}

#[test]
fn authored_bytes_with_null_floats_and_ambiguity_are_refused() {
    let template = |scalar: &str| {
        serde_json::to_string(&base())
            .unwrap()
            .replace("\"sliceNumber\":1", &format!("\"sliceNumber\":{scalar}"))
    };
    use work_engine_organization::CodecError;
    for (scalar, name) in [("1.0", "float"), ("1e2", "exponent"), ("0.5", "float")] {
        assert!(
            matches!(
                refusal_of_text(&template(scalar)),
                ValidationCode::Codec(CodecError::NonIntegerNumber { .. })
            ),
            "{name}"
        );
    }
    assert!(matches!(
        refusal_of_text(&template("9007199254740992")),
        ValidationCode::Codec(CodecError::IntegerOutOfRange { .. })
    ));
    assert!(matches!(
        refusal_of_text(&template("-0")),
        ValidationCode::Codec(CodecError::NegativeZero { .. })
    ));
    assert!(matches!(
        refusal_of_text(&template("null")),
        ValidationCode::Codec(CodecError::ExplicitNull { .. })
    ));
    // A duplicate object key is refused; serde_json::Value would have kept the last one.
    let duplicate =
        serde_json::to_string(&base())
            .unwrap()
            .replacen("{", "{\"problemScope\":{},", 1);
    assert!(matches!(
        refusal_of_text(&duplicate),
        ValidationCode::Codec(CodecError::DuplicateKey { .. })
    ));
    // An explicit null in an optional field is refused, not treated as absent.
    let optional_null = serde_json::to_string(&base()).unwrap().replacen(
        "\"scope\":{}",
        "\"scope\":{\"boundaries\":null}",
        1,
    );
    assert!(matches!(
        refusal_of_text(&optional_null),
        ValidationCode::Codec(CodecError::ExplicitNull { .. })
    ));
    // Not JSON at all.
    assert!(matches!(
        refusal_of_text("{"),
        ValidationCode::Codec(CodecError::Syntax { .. })
    ));
}

// ---------------------------------------------------------------------------------------------
// 1A-6 traceability

#[test]
fn every_candidate_member_is_traced_to_an_existing_input_item() {
    let value = two_condition_vantage_with_two_obligations();
    let requirements = parse(&value);
    let c = candidate(&value);
    let targets = |kind: &str, id: &str| {
        c.requirement_trace()
            .iter()
            .any(|row| row.target.kind == kind && row.target.id == id)
    };
    for p in c.participants() {
        assert!(targets("participant", &p.id), "participant {}", p.id);
    }
    for cond in c.conditions() {
        assert!(targets("condition", cond.id.as_str()));
    }
    for v in c.vantage_requirements() {
        assert!(targets("vantageRequirement", &v.vantage));
    }
    for key in relationship_keys(&c) {
        assert!(targets("relationship", &key), "relationship {key}");
    }
    for a in c.authority_requirements() {
        assert!(targets("authorityRequirement", a.id.as_str()));
    }
    for s in c.separation_requirements() {
        assert!(targets("separationRequirement", s.condition.as_str()));
    }
    for b in c.component_binding_requirements() {
        assert!(targets("componentBindingRequirement", &b.component));
    }
    // Each row's source names an item that exists in the input, and no field is a JSON path.
    for row in c.requirement_trace() {
        let exists = match row.source.kind {
            "obligation" => requirements
                .obligations()
                .iter()
                .any(|o| o.id.as_str() == row.source.id),
            "acceptanceCondition" => requirements
                .conditions()
                .iter()
                .any(|cond| cond.id.as_str() == row.source.id),
            "requiredAuthority" => requirements
                .required_authorities()
                .iter()
                .any(|a| a.id.as_str() == row.source.id),
            "problemScope" => row.source.id == "problemScope",
            "organizationPolicy" => row.source.id == "organizationPolicy",
            other => panic!("unexpected source kind {other}"),
        };
        assert!(exists, "{:?}", row.source);
        for field in [&row.source.field, &row.target.field] {
            assert!(!field.contains('[') && !field.contains('/'), "{field}");
        }
    }
    // The rows are a set: sorted and without duplicates.
    let rows = c.requirement_trace();
    assert!(rows.windows(2).all(|pair| pair[0] < pair[1]));
}

fn two_condition_vantage_with_two_obligations() -> Value {
    document(
        vec![obligation("ob-a", DIGEST_A), obligation("ob-b", DIGEST_B)],
        vec![
            condition(
                "a-build",
                "ob-a",
                "builder_projection",
                "builder",
                "owner-a",
            ),
            condition(
                "a-camp",
                "ob-a",
                "campaign_terminalization",
                "campaign",
                "owner-a",
            ),
            condition(
                "b-camp",
                "ob-b",
                "campaign_terminalization",
                "campaign",
                "owner-b",
            ),
        ],
        vec![
            authority(
                "auth-a",
                "owner-a",
                "accept_claim",
                Some(&["builder_projection", "campaign_terminalization"]),
            ),
            authority("auth-b", "owner-b", "accept_claim", None),
        ],
    )
}

#[test]
fn trace_rows_cite_the_sources_the_amendment_names() {
    let c = candidate(&base());
    let has = |target: (&str, &str, &str), source: (&str, &str, &str)| {
        c.requirement_trace().iter().any(|row| {
            (
                row.target.kind,
                row.target.id.as_str(),
                row.target.field.as_str(),
            ) == target
                && (
                    row.source.kind,
                    row.source.id.as_str(),
                    row.source.field.as_str(),
                ) == source
        })
    };
    // terminalizes cites the profile and the problem scope (amendment 3).
    let terminalizes = "terminalizes|component:campaign||";
    assert!(has(
        ("relationship", terminalizes, ""),
        ("organizationPolicy", "organizationPolicy", "profile")
    ));
    assert!(has(
        ("relationship", terminalizes, ""),
        ("problemScope", "problemScope", "")
    ));
    // accepts_claim cites the condition's acceptor.
    assert!(has(
        (
            "relationship",
            "accepts_claim|principal:owner-a|vantage:ob-a|cond-builder",
            ""
        ),
        ("acceptanceCondition", "cond-builder", "acceptor")
    ));
    // consumes_at cites consumerClass and consumptionBoundary.
    let consumes = "consumes_at|vantage-builder|vantage:ob-a|cond-builder";
    assert!(has(
        ("relationship", consumes, ""),
        ("acceptanceCondition", "cond-builder", "consumerClass")
    ));
    assert!(has(
        ("relationship", consumes, ""),
        ("acceptanceCondition", "cond-builder", "consumptionBoundary")
    ));
    // The builder vantage is traced to the conditions whose consumerClass is builder.
    assert!(has(
        ("participant", "vantage-builder", ""),
        ("acceptanceCondition", "cond-builder", "consumerClass")
    ));
    // The component binding requirement cites the profile (amendment 3A.3).
    assert!(has(
        ("componentBindingRequirement", "component:episode_host", ""),
        ("organizationPolicy", "organizationPolicy", "profile")
    ));
    // A condition field cites its own input field (normative property, 1A-6).
    assert!(has(
        ("condition", "cond-builder", "normative.continuity"),
        (
            "acceptanceCondition",
            "cond-builder",
            "normative.continuity"
        )
    ));
    // The principal participant cites its acceptor and its required-authority sources.
    assert!(has(
        ("participant", "principal:owner-a", ""),
        ("requiredAuthority", "auth-1", "principal")
    ));
}

// ---------------------------------------------------------------------------------------------
// 1A-7 self-acceptance

fn with_acceptor(acceptor: &str) -> Value {
    document(
        vec![obligation("ob-a", DIGEST_A)],
        vec![
            condition(
                "cond-builder",
                "ob-a",
                "builder_projection",
                "builder",
                acceptor,
            ),
            condition(
                "cond-campaign",
                "ob-a",
                "campaign_terminalization",
                "campaign",
                acceptor,
            ),
        ],
        vec![authority("auth-1", acceptor, "accept_claim", None)],
    )
}

#[test]
fn an_acceptor_that_names_a_dependent_vantage_is_refused() {
    // reviewer vantage, with and without the `vantage:` prefix; builder vantage by its ID.
    for acceptor in ["vantage:ob-a", "ob-a", "vantage-builder"] {
        assert_eq!(
            unresolved_of(&with_acceptor(acceptor)),
            vec![(
                "self_acceptance".to_owned(),
                format!("principal:{acceptor}"),
                set(&["cond-builder", "cond-campaign"])
            )],
            "{acceptor}"
        );
    }
}

#[test]
fn the_builder_vantage_is_dependent_for_every_condition_of_a_candidate_that_has_one() {
    // Amendment 4: dependents are the condition's reviewer vantage and the builder vantage when
    // the candidate has one, so a campaign-only condition is also refused if its acceptor names
    // the builder vantage.
    let mut value = with_acceptor("owner-a");
    value["acceptanceConditions"][1]["acceptor"] = json!("vantage-builder");
    value["requiredAuthorities"]
        .as_array_mut()
        .unwrap()
        .push(authority("auth-2", "vantage-builder", "accept_claim", None));
    assert_eq!(
        unresolved_of(&value),
        vec![(
            "self_acceptance".to_owned(),
            "principal:vantage-builder".to_owned(),
            set(&["cond-campaign"])
        )]
    );
    // With no builder vantage in the candidate, the name is not a dependent vantage.
    let campaign_only = document(
        vec![obligation("ob-a", DIGEST_A)],
        vec![condition(
            "c",
            "ob-a",
            "campaign_terminalization",
            "campaign",
            "vantage-builder",
        )],
        vec![authority("auth-1", "vantage-builder", "accept_claim", None)],
    );
    assert!(candidate(&campaign_only).separation_requirements().len() == 1);
}

#[test]
fn distinct_identifiers_give_a_separation_requirement_that_nothing_can_mark_satisfied() {
    let c = candidate(&base());
    let separations: Vec<(&str, &str, Vec<&str>)> = c
        .separation_requirements()
        .iter()
        .map(|s| {
            (
                s.condition.as_str(),
                s.acceptor.as_str(),
                s.vantages.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        separations,
        [
            (
                "cond-builder",
                "principal:owner-a",
                vec!["vantage-builder", "vantage:ob-a"]
            ),
            (
                "cond-campaign",
                "principal:owner-a",
                vec!["vantage-builder", "vantage:ob-a"]
            ),
        ]
    );
    let report = enforceability(&c, Profile::FixedSliceReviewV1);
    assert!(
        report
            .rows
            .iter()
            .filter(|row| row.property == "separation")
            .all(|row| row.status == EnforceStatus::None)
    );
    // A different obligation's vantage is not a dependent vantage of this condition (the rule
    // compares only declared identifiers of the dependent vantages).
    let two = document(
        vec![obligation("ob-a", DIGEST_A), obligation("ob-b", DIGEST_B)],
        vec![
            condition("a", "ob-a", "campaign_terminalization", "campaign", "ob-b"),
            condition(
                "b",
                "ob-b",
                "campaign_terminalization",
                "campaign",
                "owner-b",
            ),
        ],
        vec![
            authority("auth-1", "ob-b", "accept_claim", None),
            authority("auth-2", "owner-b", "accept_claim", None),
        ],
    );
    candidate(&two);
}

// ---------------------------------------------------------------------------------------------
// 1A-12 relationships (rule check)

fn row(kind: &str, from: &str, to: &str, condition: &str) -> String {
    format!("{kind}|{from}|{to}|{condition}")
}

#[test]
fn the_relationship_set_equals_the_set_written_from_the_amendment_table() {
    // Amendment 3, applied by hand to `base()`: one obligation ob-a, builder condition cond-builder
    // and campaign condition cond-campaign, both accepted by owner-a.
    let mut expected = vec![
        // serves: once per obligation.
        row("serves", "vantage:ob-a", "ob-a", ""),
        // hosts_execution: campaign -> every vantage, the builder vantage included.
        row("hosts_execution", "component:campaign", "vantage:ob-a", ""),
        row(
            "hosts_execution",
            "component:campaign",
            "vantage-builder",
            "",
        ),
        // consumes_at: consumer -> reviewer vantage, one per condition.
        row(
            "consumes_at",
            "vantage-builder",
            "vantage:ob-a",
            "cond-builder",
        ),
        row(
            "consumes_at",
            "component:campaign",
            "vantage:ob-a",
            "cond-campaign",
        ),
        // accepts_claim: principal -> reviewer vantage, one per condition.
        row(
            "accepts_claim",
            "principal:owner-a",
            "vantage:ob-a",
            "cond-builder",
        ),
        row(
            "accepts_claim",
            "principal:owner-a",
            "vantage:ob-a",
            "cond-campaign",
        ),
        // writes/reads_episode: episode host -> each reviewer vantage.
        row(
            "writes_episode",
            "component:episode_host",
            "vantage:ob-a",
            "",
        ),
        row(
            "reads_episode",
            "component:episode_host",
            "vantage:ob-a",
            "",
        ),
        // terminalizes: campaign, no `to`, no condition, exactly once.
        row("terminalizes", "component:campaign", "", ""),
    ];
    expected.sort();
    let mut actual = relationship_keys(&candidate(&base()));
    actual.sort();
    assert_eq!(actual, expected);

    // Adding any required authority adds no relationship.
    let mut more = base();
    for entry in [
        authority(
            "extra-1",
            "owner-a",
            "accept_claim",
            Some(&["builder_projection"]),
        ),
        authority("extra-2", "episode_host", "write_episode", None),
        authority("extra-3", "campaign", "terminalize_campaign", None),
        authority("extra-4", "new-principal", "admit_organization", None),
    ] {
        more["requiredAuthorities"]
            .as_array_mut()
            .unwrap()
            .push(entry);
    }
    let mut after = relationship_keys(&candidate(&more));
    after.sort();
    assert_eq!(after, expected);
}

#[test]
fn two_obligations_give_one_reviewer_vantage_each_and_one_terminalizes() {
    let c = candidate(&two_condition_vantage_with_two_obligations());
    let keys = relationship_keys(&c);
    let count = |kind: &str| {
        keys.iter()
            .filter(|k| k.starts_with(&format!("{kind}|")))
            .count()
    };
    assert_eq!(count("serves"), 2);
    assert_eq!(count("writes_episode"), 2);
    assert_eq!(count("reads_episode"), 2);
    assert_eq!(
        count("hosts_execution"),
        3,
        "two reviewer vantages and the builder vantage"
    );
    assert_eq!(count("consumes_at"), 3);
    assert_eq!(count("accepts_claim"), 3);
    assert_eq!(count("terminalizes"), 1);
}

// ---------------------------------------------------------------------------------------------
// 1A-13 authority closure (rule check)

fn closure_case(authorities: Vec<Value>) -> Value {
    document(
        vec![obligation("ob-a", DIGEST_A)],
        vec![condition(
            "cond-builder",
            "ob-a",
            "builder_projection",
            "builder",
            "owner-a",
        )],
        authorities,
    )
}

#[test]
fn closure_failures_give_no_candidate_and_name_the_uncovered_conditions() {
    let expected = vec![(
        "unstated_authority_dependency".to_owned(),
        "principal:owner-a".to_owned(),
        set(&["cond-builder"]),
    )];
    let cases: Vec<(&str, Vec<Value>)> = vec![
        ("no accept_claim entry", vec![]),
        (
            "an entry for a different principal",
            vec![authority("a", "owner-b", "accept_claim", None)],
        ),
        (
            "a different permission",
            vec![authority("a", "owner-a", "read_episode", None)],
        ),
        (
            "boundaries present without the condition's boundary",
            vec![authority(
                "a",
                "owner-a",
                "accept_claim",
                Some(&["campaign_terminalization"]),
            )],
        ),
    ];
    for (name, authorities) in cases {
        assert_eq!(
            unresolved_of(&closure_case(authorities)),
            expected,
            "{name}"
        );
    }
    // Only the uncovered conditions are named.
    let mut partial = base();
    partial["requiredAuthorities"][0]["scope"] = json!({"boundaries": ["builder_projection"]});
    assert_eq!(
        unresolved_of(&partial),
        vec![(
            "unstated_authority_dependency".to_owned(),
            "principal:owner-a".to_owned(),
            set(&["cond-campaign"])
        )]
    );
    // The kind avoids the `authority_` prefix reserved for Gate O refusals.
    assert!(
        !UnresolvedKind::UnstatedAuthorityDependency
            .as_str()
            .starts_with("authority_")
    );
}

#[test]
fn closure_successes_give_a_candidate() {
    for authorities in [
        vec![authority("a", "owner-a", "accept_claim", None)],
        vec![authority(
            "a",
            "owner-a",
            "accept_claim",
            Some(&["builder_projection"]),
        )],
        vec![authority(
            "a",
            "owner-a",
            "accept_claim",
            Some(&["other", "builder_projection"]),
        )],
        // Coverage may come from different entries for different conditions.
        vec![
            authority(
                "a",
                "owner-a",
                "accept_claim",
                Some(&["builder_projection"]),
            ),
            authority("b", "owner-b", "audit_read", None),
        ],
    ] {
        candidate(&closure_case(authorities));
    }
}

#[test]
fn every_candidate_has_exactly_the_two_component_binding_requirements() {
    for value in [
        base(),
        two_condition_vantage(),
        two_condition_vantage_with_two_obligations(),
        with_acceptor("vantage-builder-not"),
    ] {
        let c = candidate(&value);
        let bindings: Vec<(&str, Vec<&str>)> = c
            .component_binding_requirements()
            .iter()
            .map(|b| {
                (
                    b.component.as_str(),
                    b.permissions.iter().map(String::as_str).collect(),
                )
            })
            .collect();
        assert_eq!(
            bindings,
            [
                ("component:campaign", vec!["terminalize_campaign"]),
                (
                    "component:episode_host",
                    vec!["read_episode", "write_episode"]
                ),
            ]
        );
    }
}

#[test]
fn component_named_principals_bind_nothing() {
    let before = candidate(&base());
    let mut changed = base();
    for entry in [
        authority("x-1", "episode_host", "write_episode", None),
        authority("x-2", "campaign", "terminalize_campaign", None),
        authority("x-3", "component:episode_host", "read_episode", None),
    ] {
        changed["requiredAuthorities"]
            .as_array_mut()
            .unwrap()
            .push(entry);
    }
    let after = candidate(&changed);
    assert_eq!(before.relationships(), after.relationships());
    assert_eq!(
        before.component_binding_requirements(),
        after.component_binding_requirements()
    );
    // They appear only as ordinary authority requirements, and as principal participants.
    for id in ["x-1", "x-2", "x-3"] {
        assert!(
            after
                .authority_requirements()
                .iter()
                .any(|a| a.id.as_str() == id)
        );
    }
    assert!(after.relationships().iter().all(|r| {
        r.kind != RelationshipKind::WritesEpisode || r.from == "component:episode_host"
    }));
}

// ---------------------------------------------------------------------------------------------
// OrganizationView (amendment section 7)

#[test]
fn the_view_projects_hosting_episode_access_and_terminalization() {
    let c = candidate(&base());
    let v = view(&c);
    assert_eq!(v.obligations.len(), 1);
    assert_eq!(v.obligations[0].id.as_str(), "ob-a");
    assert_eq!(v.obligations[0].served_by, "vantage:ob-a");
    let vantage_ids: Vec<&str> = v.vantages.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(vantage_ids, ["vantage-builder", "vantage:ob-a"]);
    assert!(v.vantages[0].effective.is_none());
    assert_eq!(v.vantages[1].conditions.len(), 2);
    let hosting: Vec<(&str, &str)> = v
        .hosting
        .iter()
        .map(|h| (h.component.as_str(), h.vantage.as_str()))
        .collect();
    assert_eq!(
        hosting,
        [
            ("component:campaign", "vantage-builder"),
            ("component:campaign", "vantage:ob-a")
        ]
    );
    assert_eq!(v.episode_access.len(), 1);
    assert_eq!(v.episode_access[0].writer, "component:episode_host");
    assert_eq!(v.episode_access[0].readers, ["component:episode_host"]);
    assert_eq!(
        v.terminalization.as_ref().unwrap().component,
        "component:campaign"
    );
    assert_eq!(v.separation_requirements.len(), 2);
    assert_eq!(v.component_binding_requirements.len(), 2);
    assert_eq!(v.authority_requirements.len(), 2);
    assert_eq!(v.conditions.len(), 2);
    assert_eq!(v, view(&c), "a projection is deterministic");
}
