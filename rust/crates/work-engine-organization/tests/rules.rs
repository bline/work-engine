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
        // `capabilities` and `observerClass` have no second recognized value in v1, so no
        // authored change of either exists; their identity coverage is the closed vocabulary.
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

#[test]
fn heterogeneous_conditions_are_preserved_and_combined_by_the_f4_table() {
    let result = candidate(&two_condition_vantage());
    // Both conditions kept individually, with their own content.
    assert_eq!(result.conditions().len(), 2);
    let c1 = &result.conditions()[0];
    let c2 = &result.conditions()[1];
    assert_eq!(c1.id.as_str(), "c1");
    assert!(c1.normative.capabilities.is_empty());
    assert!(c1.normative.independence.is_some());
    assert_eq!(c2.id.as_str(), "c2");
    assert!(c2.normative.independence.is_none());
    assert_eq!(c1.consumer, "vantage-builder");
    assert_eq!(c2.consumer, "component:campaign");

    // F4: continuity same -> fresh_initial; capabilities empty (the v1 vocabulary is empty; the
    // union is tested by injection in derive.rs); independence union of `from` (only c1 states
    // it); provider_family: {anthropic} n {anthropic} = {anthropic} (a narrowing intersection is
    // tested by injection); execution_confinement: only c1 states it, so c2 does not narrow it.
    assert_eq!(result.vantage_requirements().len(), 1);
    let vantage = &result.vantage_requirements()[0];
    assert_eq!(vantage.vantage, "vantage:ob-a");
    assert_eq!(vantage.effective.continuity.as_str(), "fresh_initial");
    assert!(vantage.effective.capabilities.is_empty());
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

    // observerClass and disjoint provider_family conflicts need a second recognized class, which
    // the v1 vocabulary does not have; the disjoint branch is exercised by injection in derive.rs
    // and the observerClass comparison is unreachable until a class is added.

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

    // `capabilities` and an unmapped `observerClass` are refused by validation (the v1 vocabularies
    // admit none), so they have no authored path to a `none` row; see the refusal tests and the
    // crate-private injection test in enforce.rs.
    let cases: Vec<(&str, &str, Value)> = vec![
        ("continuity", "/normative/continuity", json!("retained")),
        (
            "independence",
            "/normative/independence",
            json!({"from": ["state.builder_context"]}),
        ),
    ];
    for (property, pointer, value) in cases {
        let mut changed = base();
        if property == "continuity" {
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

    // A class other than guarded_sandbox is not recognized, so it cannot reach a row.
    let mut confinement = base();
    confinement["acceptanceConditions"][0]["realizationConstraints"] =
        json!([{"dimension": "execution_confinement", "allowed": ["host_process"]}]);
    assert_eq!(
        unrecognized(&confinement),
        (
            "execution_confinement".to_owned(),
            "host_process".to_owned()
        )
    );
}

/// The exact evidence class of every supported mechanism row, from revision 4 section 4.3
/// (C constraint, R recording, O independent observation). A row's class is asserted, not only
/// its status.
#[test]
fn every_supported_mechanism_row_has_its_exact_revision_4_evidence_class() {
    use work_engine_organization::EvidenceClass::{
        Constraint as C, ConstraintAndRecording as CR, Observation as O, Recording as R,
    };
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] = json!([
        {"dimension": "provider_family", "allowed": ["anthropic"]},
        {"dimension": "execution_confinement", "allowed": ["guarded_sandbox"]}
    ]);
    let report = enforceability(&candidate(&value), Profile::FixedSliceReviewV1);
    let expected: [(&str, EnforceStatus, work_engine_organization::EvidenceClass); 9] = [
        ("continuity", EnforceStatus::Supported, CR),
        ("mutation", EnforceStatus::Supported, CR),
        ("capabilities", EnforceStatus::Supported, R),
        ("integrity", EnforceStatus::Supported, R),
        ("evidenceCustody", EnforceStatus::Supported, O),
        (
            "observerClass",
            EnforceStatus::SupportedWithRealizationMapping,
            R,
        ),
        ("realization:provider_family", EnforceStatus::Supported, R),
        (
            "realization:execution_confinement",
            EnforceStatus::Supported,
            CR,
        ),
        ("resultContract", EnforceStatus::Supported, C),
    ];
    for (property, status, class) in expected {
        let rows: Vec<_> = report
            .rows
            .iter()
            .filter(|row| row.property == property)
            .collect();
        assert!(!rows.is_empty(), "{property} has rows");
        for row in rows {
            assert_eq!(row.status, status, "{property} status");
            assert_eq!(row.evidence_class, class, "{property} evidence class");
        }
    }
    // Every row is accounted for: nothing is Supported with an unasserted class.
    let asserted: Vec<&str> = expected.iter().map(|(property, _, _)| *property).collect();
    for row in &report.rows {
        if row.status != EnforceStatus::None {
            assert!(
                asserted.contains(&row.property.as_str()),
                "{}",
                row.property
            );
        }
    }
}

/// Recognized-but-unsupported cases yield `none` with no evidence class; unsupported result
/// contracts are unchanged (G32).
#[test]
fn recognized_but_unsupported_cases_and_unsupported_result_contracts_are_none() {
    use work_engine_organization::EvidenceClass::NotApplicable;
    let mut retained = base();
    retained["acceptanceConditions"][0]["normative"]["continuity"] = json!("retained");
    retained["acceptanceConditions"][1]["normative"]["continuity"] = json!("retained");
    let mut independent = base();
    independent["acceptanceConditions"][0]["normative"]["independence"] =
        json!({"from": ["state.builder_context"]});
    let mut contract_id = base();
    contract_id["obligations"][0]["resultContract"]["id"] = json!("other-review");
    let mut contract_version = base();
    contract_version["obligations"][0]["resultContract"]["version"] = json!(2);
    for (property, value) in [
        ("continuity", retained),
        ("independence", independent),
        ("resultContract", contract_id),
        ("resultContract", contract_version),
    ] {
        let report = enforceability(&candidate(&value), Profile::FixedSliceReviewV1);
        let rows: Vec<_> = report
            .rows
            .iter()
            .filter(|row| row.property == property && row.status == EnforceStatus::None)
            .collect();
        assert!(!rows.is_empty(), "{property} has a none row");
        assert!(rows.iter().all(|row| row.evidence_class == NotApplicable));
        assert!(!report.executable());
    }
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

/// Strings that are not classes of any governed field: concrete identities and arbitrary text
/// (decision N2). `2.1.322` and the 64-hex digest have valid ID syntax; `Claude Opus` does not.
const NOT_CLASSES: [&str; 9] = [
    "claude-opus-5-5",
    "2.1.322",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "claude-cli-json-v1",
    "rp0-guarded-claude-v1",
    "review-execution-evidence",
    "some-other-class",
    "Claude Opus",
    "host_process",
];

fn unrecognized(value: &Value) -> (String, String) {
    match refusal(value) {
        ValidationCode::UnrecognizedClass { field, value } => (field.to_owned(), value),
        other => panic!("expected UnrecognizedClass, got {other:?}"),
    }
}

#[test]
fn concrete_identities_and_arbitrary_strings_are_not_classes_in_any_governed_field() {
    for identity in NOT_CLASSES {
        let mut as_observer = base();
        as_observer["acceptanceConditions"][0]["normative"]["observerClass"] = json!(identity);
        assert_eq!(
            unrecognized(&as_observer),
            ("observerClass".to_owned(), identity.to_owned())
        );
        for dimension in ["provider_family", "execution_confinement"] {
            let mut as_allowed = base();
            as_allowed["acceptanceConditions"][0]["realizationConstraints"] =
                json!([{"dimension": dimension, "allowed": [identity]}]);
            assert_eq!(
                unrecognized(&as_allowed),
                (dimension.to_owned(), identity.to_owned()),
                "{dimension}"
            );
            // Mixed with a recognized class the unrecognized member is still refused.
            let recognized = if dimension == "provider_family" {
                "anthropic"
            } else {
                "guarded_sandbox"
            };
            let mut mixed = base();
            mixed["acceptanceConditions"][0]["realizationConstraints"] =
                json!([{"dimension": dimension, "allowed": [recognized, identity]}]);
            assert_eq!(
                unrecognized(&mixed),
                (dimension.to_owned(), identity.to_owned()),
                "{dimension} mixed"
            );
        }
        let mut as_capability = base();
        as_capability["acceptanceConditions"][0]["normative"]["capabilities"] = json!([identity]);
        assert_eq!(
            unrecognized(&as_capability),
            ("capabilities".to_owned(), identity.to_owned())
        );
    }
}

#[test]
fn a_class_of_one_dimension_is_not_a_class_of_another_or_of_the_observer_field() {
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": ["guarded_sandbox"]}]);
    assert_eq!(
        unrecognized(&value),
        ("provider_family".to_owned(), "guarded_sandbox".to_owned())
    );
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] =
        json!([{"dimension": "execution_confinement", "allowed": ["anthropic"]}]);
    assert_eq!(
        unrecognized(&value),
        ("execution_confinement".to_owned(), "anthropic".to_owned())
    );
    let mut value = base();
    value["acceptanceConditions"][0]["normative"]["observerClass"] = json!("anthropic");
    assert_eq!(
        unrecognized(&value),
        ("observerClass".to_owned(), "anthropic".to_owned())
    );
}

#[test]
fn a_non_string_in_a_governed_field_is_a_wrong_type_not_a_class() {
    let mut value = base();
    value["acceptanceConditions"][0]["normative"]["observerClass"] = json!(7);
    assert!(matches!(refusal(&value), ValidationCode::WrongType { .. }));
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": [7]}]);
    assert!(matches!(refusal(&value), ValidationCode::WrongType { .. }));
    let mut value = base();
    value["acceptanceConditions"][0]["normative"]["capabilities"] = json!([7]);
    assert!(matches!(refusal(&value), ValidationCode::WrongType { .. }));
}

#[test]
fn every_recognized_class_is_accepted_and_the_vocabulary_is_the_profile_table() {
    let vocabulary = Profile::FixedSliceReviewV1.vocabulary();
    assert_eq!(vocabulary.provider_family, ["anthropic"]);
    assert_eq!(vocabulary.execution_confinement, ["guarded_sandbox"]);
    assert_eq!(
        vocabulary.observer_class,
        ["independent_execution_evidence"]
    );
    assert!(vocabulary.capabilities.is_empty());
    assert_eq!(
        Profile::FixedSliceReviewV1.vocabulary_version(),
        "fixed-slice-review-v1/classes-1"
    );
    for observer in &vocabulary.observer_class {
        for provider in &vocabulary.provider_family {
            for confinement in &vocabulary.execution_confinement {
                let mut value = base();
                value["acceptanceConditions"][0]["normative"]["observerClass"] = json!(observer);
                value["acceptanceConditions"][0]["realizationConstraints"] = json!([
                    {"dimension": "provider_family", "allowed": [provider]},
                    {"dimension": "execution_confinement", "allowed": [confinement]}
                ]);
                let report = enforceability(&candidate(&value), Profile::FixedSliceReviewV1);
                assert_eq!(
                    status_of(&report, "realization:provider_family"),
                    [EnforceStatus::Supported]
                );
            }
        }
    }
    // An empty capability list is the only recognized capability value.
    let mut value = base();
    value["acceptanceConditions"][0]["normative"]["capabilities"] = json!([]);
    parse(&value);
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
    // Duplicate capabilities are refused earlier: no capability is recognized in v1.
    let mut changed = base();
    changed["acceptanceConditions"][0]["normative"]["independence"] =
        json!({"from": ["state.builder_context", "state.builder_context"]});
    assert!(matches!(
        refusal(&changed),
        ValidationCode::DuplicateMember { .. }
    ));
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": ["anthropic", "anthropic"]}]);
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
        {"dimension": "provider_family", "allowed": ["anthropic"]},
        {"dimension": "provider_family", "allowed": ["anthropic"]}
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
    let v = view(&c).expect("a derived candidate projects");
    assert_eq!(v.obligations.len(), 1);
    assert_eq!(v.obligations[0].id.as_str(), "ob-a");
    assert_eq!(v.obligations[0].served_by.as_deref(), Some("vantage:ob-a"));
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
    assert_eq!(
        v,
        view(&c).expect("a derived candidate projects"),
        "a projection is deterministic"
    );
}

// ---------------------------------------------------------------------------------------------
// G32 completeness: every requirements leaf that carries meaning reaches the candidate, and the
// view and report project what the matrix (builder/p3-completeness-matrix.md) says they project.
//
// The leaves are found by walking the JSON, not from a list. The pattern table below is the
// source-to-projection matrix in executable form: a leaf with no row fails the test, and a row no
// leaf exercises fails it too.

/// What a view or report is expected to do when one leaf changes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Proj {
    /// Every successful single-leaf instance changes the projection.
    Yes,
    /// No single-leaf instance changes the projection (the leaf has no field there).
    No,
    /// Depends on the other leaves (for example, whether a builder vantage exists).
    Depends,
}
use Proj::{Depends, No, Yes};

/// `(leaf pattern, view, report)`. A closed single-value vocabulary has no valid second value; for
/// those the test requires that the changed value is refused.
const LEAVES: &[(&str, Proj, Proj)] = &[
    ("/problemScope/runId", Yes, No),
    ("/problemScope/planLineage", Yes, No),
    ("/problemScope/sliceNumber", Yes, No),
    ("/obligations/*/id", Yes, Yes),
    ("/obligations/*/resultContract/id", Yes, Yes),
    ("/obligations/*/resultContract/version", Yes, Yes),
    ("/obligations/*/scope/digest", Yes, No),
    ("/acceptanceConditions/*/id", Yes, Yes),
    ("/acceptanceConditions/*/obligation", Yes, Yes),
    ("/acceptanceConditions/*/consumptionBoundary", Yes, No),
    ("/acceptanceConditions/*/consumerClass", Yes, Depends),
    ("/acceptanceConditions/*/proposition", Yes, No),
    ("/acceptanceConditions/*/coveredState", Yes, No),
    ("/acceptanceConditions/*/normative/continuity", Yes, Yes),
    ("/acceptanceConditions/*/acceptor", Yes, Yes),
    ("/acceptanceConditions/*/unestablishedRoute", Yes, No),
    ("/requiredAuthorities/*/id", Yes, No),
    ("/requiredAuthorities/*/principal", Yes, Depends),
    ("/requiredAuthorities/*/permission", Yes, No),
    ("/requiredAuthorities/*/scope/boundaries/*", Yes, Depends),
];

/// Closed single-value vocabularies: no second valid value exists. Since decision N2 this includes
/// `observerClass`, `allowed[]` and, because a dimension is bound to its own classes, `dimension` (one recognized class per field); `capabilities[]` has no
/// leaf at all because the v1 vocabulary is empty. Their destinations are checked by
/// `single_value_class_leaves_reach_their_destinations`.
const LITERAL_LEAVES: &[&str] = &[
    "/acceptanceConditions/*/normative/observerClass",
    "/acceptanceConditions/*/realizationConstraints/*/dimension",
    "/acceptanceConditions/*/realizationConstraints/*/allowed/*",
    "/obligations/*/kind",
    "/obligations/*/subject",
    "/acceptanceConditions/*/normative/mutation",
    "/acceptanceConditions/*/normative/integrity",
    "/acceptanceConditions/*/normative/evidenceCustody",
    "/acceptanceConditions/*/normative/independence/from/*",
    "/organizationPolicy/mode",
    "/organizationPolicy/profile",
];

/// Members that may be absent (G8, G23, amendment 3A): absence is meaningful and is not refused.
const OPTIONAL_MEMBERS: &[&str] = &[
    "/acceptanceConditions/*/realizationConstraints",
    "/acceptanceConditions/*/normative/independence",
    "/requiredAuthorities/*/scope/boundaries",
];

const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

/// Two obligations and four conditions, every optional member present at least once, every set
/// with more than one member where F4 allows it.
fn rich() -> Value {
    let mut a_build = condition(
        "a-build",
        "ob-a",
        "builder_projection",
        "builder",
        "owner-a",
    );
    a_build["normative"]["independence"] = json!({"from": ["state.builder_context"]});
    a_build["realizationConstraints"] = json!([
        {"dimension": "provider_family", "allowed": ["anthropic"]},
        {"dimension": "execution_confinement", "allowed": ["guarded_sandbox"]}
    ]);
    let mut a_camp = condition(
        "a-camp",
        "ob-a",
        "campaign_terminalization",
        "campaign",
        "owner-a",
    );
    a_camp["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": ["anthropic"]}]);
    let b_camp = condition(
        "b-camp",
        "ob-b",
        "campaign_terminalization",
        "campaign",
        "owner-b",
    );
    let b_build = condition(
        "b-build",
        "ob-b",
        "builder_projection",
        "builder",
        "owner-b",
    );
    document(
        vec![obligation("ob-a", DIGEST_A), obligation("ob-b", DIGEST_B)],
        vec![a_build, a_camp, b_camp, b_build],
        vec![
            authority(
                "auth-a",
                "owner-a",
                "accept_claim",
                Some(&["builder_projection", "campaign_terminalization"]),
            ),
            authority("auth-b", "owner-b", "accept_claim", None),
            authority("auth-x", "observer-1", "audit_read", Some(&["audit-b"])),
        ],
    )
}

/// `/a/0/b` to `/a/*/b`.
fn pattern_of(pointer: &str) -> String {
    pointer
        .split('/')
        .map(|part| {
            if !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()) {
                "*"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Every scalar leaf of the document as a JSON pointer.
fn leaves(value: &Value, here: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                leaves(child, &format!("{here}/{key}"), out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                leaves(child, &format!("{here}/{index}"), out);
            }
        }
        _ => out.push(here.to_owned()),
    }
}

/// Every object member (not array element) of the document as a JSON pointer.
fn members(value: &Value, here: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = format!("{here}/{key}");
                out.push(path.clone());
                members(child, &path, out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                members(child, &format!("{here}/{index}"), out);
            }
        }
        _ => {}
    }
}

fn key_of(pointer: &str) -> &str {
    pointer
        .rsplit('/')
        .find(|part| !part.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or("")
}

/// Another value this leaf could take; the closed vocabularies with two values are listed.
fn alternative(key: &str, value: &str) -> Option<String> {
    match (key, value) {
        ("continuity", "fresh_initial") => Some("retained".to_owned()),
        ("continuity", "retained") => Some("fresh_initial".to_owned()),
        ("dimension", "provider_family") => Some("execution_confinement".to_owned()),
        ("dimension", "execution_confinement") => Some("provider_family".to_owned()),
        ("digest", digest) if digest.len() == 64 => Some(DIGEST_C.to_owned()),
        ("digest", _) => None,
        _ => Some(format!("{value}-m")),
    }
}

#[derive(Debug)]
enum Edit {
    /// One leaf changed.
    Single,
    /// Every leaf with the same member name and value changed together.
    SameMember,
    /// Every leaf with the same value changed together (a cross-reference rename).
    SameValue,
}

/// The candidate edits to try for one leaf, narrowest first.
fn edits_for(doc: &Value, pointer: &str) -> Vec<(Edit, Vec<(String, Value)>)> {
    let current = doc.pointer(pointer).unwrap();
    let mut all = Vec::new();
    leaves(doc, "", &mut all);
    let Some(text) = current.as_str() else {
        // Integers: one more.
        let next = current.as_u64().unwrap() + 1;
        return vec![(Edit::Single, vec![(pointer.to_owned(), json!(next))])];
    };
    let mut values: Vec<String> = Vec::new();
    // A peer: the value of another leaf with the same member name.
    for other in &all {
        if key_of(other) == key_of(pointer)
            && let Some(peer) = doc.pointer(other).and_then(Value::as_str)
            && peer != text
            && !values.iter().any(|v| v == peer)
        {
            values.push(peer.to_owned());
        }
    }
    if let Some(alt) = alternative(key_of(pointer), text) {
        values.push(alt);
    }
    let same_member: Vec<&String> = all
        .iter()
        .filter(|o| {
            key_of(o) == key_of(pointer) && doc.pointer(o).and_then(Value::as_str) == Some(text)
        })
        .collect();
    let same_value: Vec<&String> = all
        .iter()
        .filter(|o| doc.pointer(o).and_then(Value::as_str) == Some(text))
        .collect();
    let mut edits = Vec::new();
    for value in &values {
        edits.push((Edit::Single, vec![(pointer.to_owned(), json!(value))]));
    }
    for (kind, group) in [(0, &same_member), (1, &same_value)] {
        for value in &values {
            let changes: Vec<(String, Value)> =
                group.iter().map(|p| ((*p).clone(), json!(value))).collect();
            edits.push((
                if kind == 0 {
                    Edit::SameMember
                } else {
                    Edit::SameValue
                },
                changes,
            ));
        }
    }
    edits
}

fn apply(doc: &Value, changes: &[(String, Value)]) -> Value {
    let mut changed = doc.clone();
    for (pointer, value) in changes {
        *changed.pointer_mut(pointer).unwrap() = value.clone();
    }
    changed
}

/// The candidate's canonical JSON without its `requirementsDigest` member: what the candidate
/// itself says. Without this a leaf that never reached the candidate would still change
/// `contentDigest`, because `requirementsDigest` is a member of it.
fn candidate_content(c: &work_engine_organization::OrganizationCandidate) -> Value {
    let mut value: Value = serde_json::from_slice(c.canonical_bytes()).unwrap();
    value.as_object_mut().unwrap().remove("requirementsDigest");
    value
}

fn try_candidate(value: &Value) -> Result<work_engine_organization::OrganizationCandidate, String> {
    match OrganizationalRequirements::from_json_bytes(&bytes(value)) {
        Err(error) => Err(format!("refused: {}", error.code)),
        Ok(requirements) => {
            let mut result =
                work_engine_organization::derive(&requirements, Profile::FixedSliceReviewV1);
            if result.candidates.is_empty() {
                Err(format!(
                    "unresolved: {:?}",
                    result
                        .unresolved
                        .iter()
                        .map(|u| u.kind.as_str())
                        .collect::<Vec<_>>()
                ))
            } else {
                Ok(result.candidates.remove(0))
            }
        }
    }
}

#[test]
fn every_requirements_leaf_reaches_the_candidate_and_its_documented_projections() {
    let doc = rich();
    let reference = candidate(&doc);
    let reference_content = candidate_content(&reference);
    let reference_view = view(&reference).expect("a derived candidate projects");
    let reference_report = enforceability(&reference, Profile::FixedSliceReviewV1);

    let mut all = Vec::new();
    leaves(&doc, "", &mut all);
    // The count is the matrix's leaf count for this input; a changed input must update both.
    assert_eq!(all.len(), 88, "leaves found by the walk");

    let mut exercised: Vec<String> = Vec::new();
    let mut reached: Vec<String> = Vec::new();
    for pointer in &all {
        let pattern = pattern_of(pointer);
        let known = LEAVES.iter().find(|(p, _, _)| *p == pattern);
        let literal = LITERAL_LEAVES.contains(&pattern.as_str());
        assert!(
            known.is_some() || literal,
            "leaf {pointer} ({pattern}) has no row in the matrix table"
        );
        exercised.push(pattern.clone());

        if literal {
            // A closed vocabulary: a different value is refused, never carried.
            let outcome = try_candidate(&apply(&doc, &[(pointer.clone(), json!("other-value"))]));
            assert!(
                matches!(&outcome, Err(reason) if reason.starts_with("refused")),
                "{pointer}: a second value must be refused, got {:?}",
                outcome.map(|_| "a candidate")
            );
            continue;
        }

        let (_, view_expectation, report_expectation) = *known.unwrap();
        let mut winner = None;
        let mut failures = Vec::new();
        for (edit, changes) in edits_for(&doc, pointer) {
            let changed = apply(&doc, &changes);
            match try_candidate(&changed) {
                Ok(c) if c.requirements_digest() != reference.requirements_digest() => {
                    winner = Some((edit, c));
                    break;
                }
                Ok(_) => failures.push(format!("{edit:?}: same requirementsDigest")),
                Err(reason) => failures.push(format!("{edit:?}: {reason}")),
            }
        }
        let Some((edit, changed)) = winner else {
            // The leaf is an input to a rule (for example the accept_claim permission): any change
            // is refused or unresolved, never a silently different candidate.
            assert!(!failures.is_empty(), "{pointer}: no edit was tried");
            continue;
        };
        reached.push(pattern.clone());

        // The identities change, and the candidate itself says something different.
        assert_ne!(
            changed.requirements_digest(),
            reference.requirements_digest(),
            "{pointer}"
        );
        assert_ne!(
            changed.content_digest(),
            reference.content_digest(),
            "{pointer}"
        );
        assert_ne!(
            candidate_content(&changed),
            reference_content,
            "{pointer} ({edit:?}) reached the digests only through requirementsDigest, not the candidate's own content"
        );

        let view_changed = view(&changed).expect("a derived candidate projects") != reference_view;
        let report_changed =
            enforceability(&changed, Profile::FixedSliceReviewV1) != reference_report;
        let single = matches!(edit, Edit::Single);
        for (what, expectation, changed_now) in [
            ("view", view_expectation, view_changed),
            ("report", report_expectation, report_changed),
        ] {
            match expectation {
                Yes => assert!(changed_now, "{pointer} ({edit:?}): the {what} must change"),
                No if single => assert!(
                    !changed_now,
                    "{pointer}: the {what} has no field for this leaf but changed"
                ),
                No | Depends => {}
            }
        }
    }

    // Every table row was exercised, and every row's pattern produced at least one candidate.
    for (pattern, _, _) in LEAVES {
        assert!(
            exercised.iter().any(|p| p == pattern),
            "unused row {pattern}"
        );
        assert!(
            reached.iter().any(|p| p == pattern),
            "no instance of {pattern} produced a candidate"
        );
    }
    for pattern in LITERAL_LEAVES {
        assert!(
            exercised.iter().any(|p| p == pattern),
            "unused literal row {pattern}"
        );
    }
}

/// The single recognized class of each governed field reaches the candidate, the view and the
/// report. A changed class cannot be authored (validation refuses it), so the destination is
/// checked directly rather than by edit.
#[test]
fn single_value_class_leaves_reach_their_destinations() {
    let mut value = base();
    value["acceptanceConditions"][0]["realizationConstraints"] = json!([
        {"dimension": "provider_family", "allowed": ["anthropic"]},
        {"dimension": "execution_confinement", "allowed": ["guarded_sandbox"]}
    ]);
    let c = candidate(&value);
    let content = candidate_content(&c).to_string();
    for class in [
        "independent_execution_evidence",
        "anthropic",
        "guarded_sandbox",
    ] {
        assert!(content.contains(&format!("\"{class}\"")), "{class}");
    }
    let projected = view(&c).expect("a derived candidate projects");
    let vantage = projected
        .vantages
        .iter()
        .find(|v| v.effective.is_some())
        .expect("a reviewer vantage");
    assert_eq!(
        vantage.effective.as_ref().unwrap().observer_class.as_str(),
        "independent_execution_evidence"
    );
    let allowed: Vec<(&str, Vec<&str>)> = vantage
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
        allowed,
        [
            ("execution_confinement", vec!["guarded_sandbox"]),
            ("provider_family", vec!["anthropic"])
        ]
    );
    let report = enforceability(&c, Profile::FixedSliceReviewV1);
    let requirements: Vec<&str> = report
        .rows
        .iter()
        .filter(|row| row.scope == "cond-builder" && row.property.starts_with("realization:"))
        .map(|row| row.requirement.as_str())
        .collect();
    assert_eq!(requirements, ["guarded_sandbox", "anthropic"]);
}

#[test]
fn every_required_source_member_is_refused_when_absent_and_never_filled() {
    let doc = rich();
    let mut all = Vec::new();
    members(&doc, "", &mut all);
    assert!(all.len() > 60, "the walk found {} members", all.len());
    let mut optional_seen = Vec::new();
    for pointer in &all {
        let pattern = pattern_of(pointer);
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        let mut changed = doc.clone();
        let removed = if parent.is_empty() {
            changed.as_object_mut().unwrap().remove(key)
        } else {
            changed
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(key)
        };
        assert!(removed.is_some());
        let outcome = OrganizationalRequirements::from_json_bytes(&bytes(&changed));
        if OPTIONAL_MEMBERS.contains(&pattern.as_str()) {
            optional_seen.push(pattern.clone());
            let parsed = outcome.unwrap_or_else(|e| panic!("{pointer} is optional: {e}"));
            // Absent means absent: the candidate does not carry a filled value.
            assert_ne!(
                parsed.requirements_digest(),
                parse(&doc).requirements_digest(),
                "{pointer}: dropping it changes the requirements"
            );
        } else {
            let error = outcome.expect_err(&format!("{pointer} is required"));
            assert_eq!(
                error.code,
                ValidationCode::MissingField {
                    name: key.to_owned()
                },
                "{pointer}"
            );
            assert_eq!(
                error.path, parent,
                "{pointer}: the error names the containing object"
            );
        }
    }
    for pattern in OPTIONAL_MEMBERS {
        assert!(
            optional_seen.iter().any(|p| p == pattern),
            "unused {pattern}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// G32 identity, report coverage, masking and heterogeneous obligations

#[test]
fn problem_scope_and_obligations_are_copied_unchanged_into_the_candidate() {
    let doc = rich();
    let requirements = parse(&doc);
    let c = candidate(&doc);
    let from_requirements: Value = serde_json::from_slice(requirements.canonical_bytes()).unwrap();
    let from_candidate: Value = serde_json::from_slice(c.canonical_bytes()).unwrap();
    assert_eq!(
        from_candidate["problemScope"],
        from_requirements["problemScope"]
    );
    assert_eq!(
        from_candidate["obligations"],
        from_requirements["obligations"]
    );
    // The typed accessors agree with the bytes.
    assert_eq!(c.problem_scope().run_id.as_str(), "run-1");
    assert_eq!(c.problem_scope().plan_lineage.as_str(), "plan-a");
    assert_eq!(c.problem_scope().slice_number, 1);
    let ids: Vec<&str> = c.obligations().iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, ["ob-a", "ob-b"]);
    assert_eq!(c.obligations()[0].kind(), "candidate_review");
    assert_eq!(c.obligations()[0].subject(), "campaign_candidate");
    assert_eq!(c.obligations()[1].scope_digest.to_hex(), DIGEST_B);
}

#[test]
fn identity_covers_problem_scope_and_every_obligation_member() {
    let reference = candidate(&rich());
    let cases: Vec<(&str, &str, Value)> = vec![
        ("runId", "/problemScope/runId", json!("run-2")),
        ("planLineage", "/problemScope/planLineage", json!("plan-b")),
        ("sliceNumber", "/problemScope/sliceNumber", json!(2)),
        (
            "contract id",
            "/obligations/1/resultContract/id",
            json!("other-review"),
        ),
        (
            "contract version",
            "/obligations/1/resultContract/version",
            json!(2),
        ),
        (
            "scope digest",
            "/obligations/1/scope/digest",
            json!(DIGEST_C),
        ),
    ];
    for (name, pointer, value) in cases {
        let mut changed = rich();
        *changed.pointer_mut(pointer).unwrap() = value;
        let changed = candidate(&changed);
        assert_ne!(
            changed.content_digest(),
            reference.content_digest(),
            "{name}"
        );
        assert_ne!(
            changed.canonical_bytes(),
            reference.canonical_bytes(),
            "{name}"
        );
        assert_ne!(
            candidate_content(&changed),
            candidate_content(&reference),
            "{name} must be in the candidate's own content"
        );
    }
}

/// The rows the input requires, counted from the input and not from the output: `(scope,
/// property)` pairs.
fn required_rows(doc: &Value) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for obligation in doc["obligations"].as_array().unwrap() {
        rows.push((
            obligation["id"].as_str().unwrap().to_owned(),
            "resultContract".to_owned(),
        ));
    }
    for condition in doc["acceptanceConditions"].as_array().unwrap() {
        let id = condition["id"].as_str().unwrap();
        for property in [
            "continuity",
            "mutation",
            "capabilities",
            "integrity",
            "evidenceCustody",
            "observerClass",
            "separation",
        ] {
            rows.push((id.to_owned(), property.to_owned()));
        }
        if condition["normative"].get("independence").is_some() {
            rows.push((id.to_owned(), "independence".to_owned()));
        }
        for constraint in condition
            .get("realizationConstraints")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            rows.push((
                id.to_owned(),
                format!("realization:{}", constraint["dimension"].as_str().unwrap()),
            ));
        }
    }
    rows.sort();
    rows
}

#[test]
fn the_report_has_exactly_the_rows_the_input_requires() {
    let inputs = vec![
        rich(),
        base(),
        two_condition_vantage(),
        heterogeneous_obligations(),
    ];
    for doc in inputs {
        let report = enforceability(&candidate(&doc), Profile::FixedSliceReviewV1);
        let mut actual: Vec<(String, String)> = report
            .rows
            .iter()
            .map(|row| (row.scope.clone(), row.property.clone()))
            .collect();
        actual.sort();
        assert_eq!(
            actual,
            required_rows(&doc),
            "one row per obligation and per condition property"
        );
        // And exactly one resultContract row per obligation, whatever its value.
        for obligation in doc["obligations"].as_array().unwrap() {
            let id = obligation["id"].as_str().unwrap();
            let rows: Vec<_> = report
                .rows
                .iter()
                .filter(|r| r.property == "resultContract" && r.scope == id)
                .collect();
            assert_eq!(rows.len(), 1, "obligation {id}");
            assert_eq!(
                rows[0].requirement,
                format!(
                    "{}@{}",
                    obligation["resultContract"]["id"].as_str().unwrap(),
                    obligation["resultContract"]["version"]
                )
            );
        }
    }
}

fn contract_status(
    report: &work_engine_organization::EnforceabilityReport,
    obligation: &str,
) -> EnforceStatus {
    let rows: Vec<_> = report
        .rows
        .iter()
        .filter(|row| row.property == "resultContract" && row.scope == obligation)
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "exactly one resultContract row for {obligation}"
    );
    rows[0].status
}

#[test]
fn an_unsupported_result_contract_is_not_masked_by_other_none_rows() {
    // The independence requirement already gives a none row; the contract row must still be there.
    let mut unsupported_id = base();
    unsupported_id["acceptanceConditions"][0]["normative"]["independence"] =
        json!({"from": ["state.builder_context"]});
    unsupported_id["obligations"][0]["resultContract"]["id"] = json!("other-review");
    let report = enforceability(&candidate(&unsupported_id), Profile::FixedSliceReviewV1);
    assert!(status_of(&report, "independence").contains(&EnforceStatus::None));
    assert_eq!(contract_status(&report, "ob-a"), EnforceStatus::None);

    // A supported id at an unsupported version, again behind the independence none row.
    let mut unsupported_version = base();
    unsupported_version["acceptanceConditions"][0]["normative"]["independence"] =
        json!({"from": ["state.builder_context"]});
    unsupported_version["obligations"][0]["resultContract"]["version"] = json!(2);
    let report = enforceability(
        &candidate(&unsupported_version),
        Profile::FixedSliceReviewV1,
    );
    assert!(status_of(&report, "independence").contains(&EnforceStatus::None));
    assert_eq!(contract_status(&report, "ob-a"), EnforceStatus::None);

    // The supported contract is the only one that gets `supported`, with the independence row
    // still none beside it: the rows do not hide each other in either direction.
    let mut supported = base();
    supported["acceptanceConditions"][0]["normative"]["independence"] =
        json!({"from": ["state.builder_context"]});
    let report = enforceability(&candidate(&supported), Profile::FixedSliceReviewV1);
    assert!(status_of(&report, "independence").contains(&EnforceStatus::None));
    assert_eq!(contract_status(&report, "ob-a"), EnforceStatus::Supported);
    assert!(!report.executable());
}

fn heterogeneous_obligations() -> Value {
    let mut unknown = obligation("ob-b", DIGEST_B);
    unknown["resultContract"]["id"] = json!("unknown-review");
    let mut other_version = obligation("ob-c", DIGEST_C);
    other_version["resultContract"]["version"] = json!(2);
    document(
        vec![obligation("ob-a", DIGEST_A), unknown, other_version],
        vec![
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
                "owner-a",
            ),
            condition(
                "c-camp",
                "ob-c",
                "campaign_terminalization",
                "campaign",
                "owner-a",
            ),
        ],
        vec![authority("auth-1", "owner-a", "accept_claim", None)],
    )
}

#[test]
fn heterogeneous_result_contracts_stay_distinct_in_candidate_view_and_report() {
    let doc = heterogeneous_obligations();
    let c = candidate(&doc);
    let expected = [
        ("ob-a", "implementation-review", 1, EnforceStatus::Supported),
        ("ob-b", "unknown-review", 1, EnforceStatus::None),
        ("ob-c", "implementation-review", 2, EnforceStatus::None),
    ];
    assert_eq!(c.obligations().len(), 3);
    let projected = view(&c).expect("a derived candidate projects");
    assert_eq!(projected.obligations.len(), 3);
    let report = enforceability(&c, Profile::FixedSliceReviewV1);
    for (index, (id, contract, version, status)) in expected.into_iter().enumerate() {
        let from_candidate = &c.obligations()[index];
        assert_eq!(from_candidate.id.as_str(), id);
        assert_eq!(from_candidate.result_contract.id.as_str(), contract);
        assert_eq!(from_candidate.result_contract.version, version);
        let from_view = &projected.obligations[index];
        assert_eq!(from_view.id.as_str(), id);
        assert_eq!(from_view.result_contract.id.as_str(), contract);
        assert_eq!(from_view.result_contract.version, version);
        assert_eq!(
            from_view.served_by.as_deref(),
            Some(format!("vantage:{id}").as_str())
        );
        assert_eq!(contract_status(&report, id), status, "{id}");
    }
    assert!(!report.executable());
}

// ---------------------------------------------------------------------------------------------
// G32 matrix: the candidate's member list

/// Every member path of the candidate's canonical form (containers included; `*` is an array
/// element). This is the candidate half of builder/p3-completeness-matrix.md: a member added to or
/// removed from the candidate fails the test until the matrix is updated.
const CANDIDATE_MEMBERS: &[&str] = &[
    "/authorityRequirements",
    "/authorityRequirements/*/id",
    "/authorityRequirements/*/permission",
    "/authorityRequirements/*/principal",
    "/authorityRequirements/*/scope",
    "/authorityRequirements/*/scope/boundaries",
    "/componentBindingRequirements",
    "/componentBindingRequirements/*/component",
    "/componentBindingRequirements/*/permissions",
    "/conditions",
    "/conditions/*/acceptor",
    "/conditions/*/boundary",
    "/conditions/*/consumer",
    "/conditions/*/coveredState",
    "/conditions/*/id",
    "/conditions/*/normative",
    "/conditions/*/normative/capabilities",
    "/conditions/*/normative/continuity",
    "/conditions/*/normative/evidenceCustody",
    "/conditions/*/normative/independence",
    "/conditions/*/normative/independence/from",
    "/conditions/*/normative/integrity",
    "/conditions/*/normative/mutation",
    "/conditions/*/normative/observerClass",
    "/conditions/*/obligation",
    "/conditions/*/proposition",
    "/conditions/*/realizationConstraints",
    "/conditions/*/realizationConstraints/*/allowed",
    "/conditions/*/realizationConstraints/*/dimension",
    "/conditions/*/unestablishedRoute",
    "/conditions/*/vantage",
    "/obligations",
    "/obligations/*/id",
    "/obligations/*/kind",
    "/obligations/*/resultContract",
    "/obligations/*/resultContract/id",
    "/obligations/*/resultContract/version",
    "/obligations/*/scope",
    "/obligations/*/scope/digest",
    "/obligations/*/subject",
    "/participants",
    "/participants/*/id",
    "/participants/*/kind",
    "/participants/*/serves",
    "/problemScope",
    "/problemScope/planLineage",
    "/problemScope/runId",
    "/problemScope/sliceNumber",
    "/profile",
    "/relationships",
    "/relationships/*/condition",
    "/relationships/*/from",
    "/relationships/*/kind",
    "/relationships/*/to",
    "/requirementTrace",
    "/requirementTrace/*/source",
    "/requirementTrace/*/source/field",
    "/requirementTrace/*/source/id",
    "/requirementTrace/*/source/kind",
    "/requirementTrace/*/target",
    "/requirementTrace/*/target/field",
    "/requirementTrace/*/target/id",
    "/requirementTrace/*/target/kind",
    "/requirementsDigest",
    "/separationRequirements",
    "/separationRequirements/*/acceptor",
    "/separationRequirements/*/condition",
    "/separationRequirements/*/vantages",
    "/vantageRequirements",
    "/vantageRequirements/*/derivedFrom",
    "/vantageRequirements/*/effective",
    "/vantageRequirements/*/effective/capabilities",
    "/vantageRequirements/*/effective/continuity",
    "/vantageRequirements/*/effective/evidenceCustody",
    "/vantageRequirements/*/effective/independence",
    "/vantageRequirements/*/effective/independence/from",
    "/vantageRequirements/*/effective/integrity",
    "/vantageRequirements/*/effective/mutation",
    "/vantageRequirements/*/effective/observerClass",
    "/vantageRequirements/*/effectiveConstraints",
    "/vantageRequirements/*/effectiveConstraints/*/allowed",
    "/vantageRequirements/*/effectiveConstraints/*/dimension",
    "/vantageRequirements/*/vantage",
];

fn member_paths(value: &Value, here: &str, out: &mut std::collections::BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = format!("{here}/{key}");
                out.insert(path.clone());
                member_paths(child, &path, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                member_paths(child, &format!("{here}/*"), out);
            }
        }
        _ => {}
    }
}

#[test]
fn the_candidate_has_exactly_the_members_the_matrix_lists() {
    // `rich` has every optional member; the other inputs add nothing but are walked for safety.
    let mut found = std::collections::BTreeSet::new();
    for doc in [
        rich(),
        base(),
        two_condition_vantage(),
        heterogeneous_obligations(),
    ] {
        let c = candidate(&doc);
        member_paths(
            &serde_json::from_slice(c.canonical_bytes()).unwrap(),
            "",
            &mut found,
        );
    }
    let listed: std::collections::BTreeSet<String> =
        CANDIDATE_MEMBERS.iter().map(|p| (*p).to_owned()).collect();
    assert_eq!(CANDIDATE_MEMBERS.len(), listed.len(), "no duplicate row");
    assert_eq!(found, listed);
}
