//! Production preparation (default features): real compile -> view/projection -> serialize/read.
//! The fixture consumer is not compiled here.
mod common;

use std::collections::BTreeSet;

use common::*;
use serde_json::{Value, json};
use work_engine_organization::{
    EnforceStatus, OrganizationalRequirements, Profile, enforceability, view,
};
use work_engine_review_preparation::{
    BlockerClass, Disposition, MutationClaim, PreparationRefusal, PreparationStatus,
    RealizationClaim, ReviewPreparation, RowStatus, Subject, prepare,
};

fn blocker_ids(prep: &ReviewPreparation) -> BTreeSet<String> {
    prep.blockers.iter().map(|b| b.id.clone()).collect()
}

fn unconditional() -> Vec<String> {
    let mut ids = vec![
        format!("scope-digest-to-subject:{OBLIGATION}"),
        "problem-scope-to-episode-identity".to_owned(),
    ];
    for cond in [COND_BUILDER, COND_CAMPAIGN] {
        ids.push(format!("separation:{cond}"));
        ids.push(format!("consumption-evidence-mapping:{cond}"));
        ids.push(format!("observer-identity:{cond}"));
        ids.push(format!("realization-admission:{cond}"));
    }
    ids
}

#[test]
fn the_fixture_inputs_are_the_frozen_bytes() {
    // provenance.json records these digests.
    assert_eq!(
        sha256_hex(REQ),
        "7085318950aceb6f7071d29dec83e13da610b20d110f435e3399c9de57d3b292"
    );
    assert_eq!(
        sha256_hex(SELECTION),
        "5d73611756fdac10ca0edab663e27ec13f1205d3eea387e7a738a5c6ee0a6fb5"
    );
    assert_eq!(
        sha256_hex(AUTHORITY.as_bytes()),
        "85c16c492c152e768cab038464ff385fcc1ed9f10c79423ced42a92b428084e5"
    );
}

#[test]
fn projection_carries_the_owned_meaning_of_the_selected_obligation() {
    let candidate = candidate();
    let prep = prepare(&candidate, &request(None)).expect("prepares");
    assert_eq!(prep.status, PreparationStatus::InertNotAnAdmissionToken);

    // Identity is the candidate's own.
    let identity = &prep.model_context.identity;
    assert_eq!(
        identity.candidate_content_digest,
        candidate.content_digest().to_hex()
    );
    assert_eq!(
        identity.requirements_digest,
        candidate.requirements_digest().to_hex()
    );
    assert_eq!(identity.profile, "fixed-slice-review-v1");
    assert_eq!(identity.run_id, "production-qualification-20261107");
    assert_eq!(identity.slice_number, 1);

    // Obligation, result contract and exact subject.
    let obligation = &prep.model_context.obligation;
    assert_eq!(obligation.id, OBLIGATION);
    assert_eq!(obligation.kind, "candidate_review");
    assert_eq!(obligation.subject_class, "campaign_candidate");
    assert_eq!(obligation.result_contract_id, "implementation-review");
    assert_eq!(obligation.result_contract_version, 1);
    assert_eq!(obligation.subject, subject());

    // Conditions equal the view's rows field for field, in the owner's order.
    let owner_view = view(&candidate).unwrap();
    assert_eq!(
        prep.model_context.conditions.len(),
        owner_view.conditions.len()
    );
    for (mine, theirs) in prep
        .model_context
        .conditions
        .iter()
        .zip(&owner_view.conditions)
    {
        assert_eq!(mine.id, theirs.id.as_str());
        assert_eq!(mine.boundary, theirs.boundary.as_str());
        assert_eq!(mine.consumer, theirs.consumer);
        assert_eq!(mine.proposition, theirs.proposition);
        assert_eq!(mine.covered_state, theirs.covered_state.as_str());
        assert_eq!(mine.acceptor, theirs.acceptor.as_str());
        assert_eq!(
            mine.unestablished_route,
            theirs.unestablished_route.as_str()
        );
    }
    let served = prep.model_context.vantage.id.clone();
    assert_eq!(served, format!("vantage:{OBLIGATION}"));
    let owner_vantage = owner_view.vantages.iter().find(|v| v.id == served).unwrap();
    assert_eq!(
        prep.model_context
            .vantage
            .effective
            .as_ref()
            .unwrap()
            .continuity,
        owner_vantage
            .effective
            .as_ref()
            .unwrap()
            .continuity
            .as_str()
    );
    assert_eq!(
        prep.model_context.vantage.effective_constraints.len(),
        owner_vantage.effective_constraints.len()
    );

    // Enforcement rows are the owner's rows for this obligation, verbatim.
    let report = enforceability(&candidate, Profile::FixedSliceReviewV1);
    assert_eq!(prep.host_enforcement.rows.len(), report.rows.len());
    for (mine, theirs) in prep.host_enforcement.rows.iter().zip(&report.rows) {
        assert_eq!(
            (&mine.property, &mine.scope, &mine.requirement),
            (&theirs.property, &theirs.scope, &theirs.requirement)
        );
        assert_eq!(mine.note, theirs.note);
    }
    assert_eq!(prep.host_enforcement.separation.len(), 2);
    assert!(
        prep.host_enforcement
            .rows
            .iter()
            .filter(|r| r.property == "separation")
            .all(|r| r.status == RowStatus::None)
    );
    assert_eq!(
        prep.host_enforcement.component_binding.len(),
        candidate.component_binding_requirements().len()
    );
    assert!(!prep.provenance.is_empty(), "owner trace rows are carried");
    assert!(prep.provenance.iter().all(|p| {
        p.target.id == OBLIGATION || p.target.id == COND_BUILDER || p.target.id == COND_CAMPAIGN
    }));
}

#[test]
fn the_current_candidate_is_blocked_by_every_unestablished_mapping() {
    let prep = prepare(&candidate(), &request(None)).unwrap();
    let ids = blocker_ids(&prep);
    for id in unconditional() {
        assert!(ids.contains(&id), "missing blocker {id}");
    }
    assert!(ids.iter().any(|id| id.starts_with("component-binding:")));
    assert!(
        ids.iter()
            .any(|id| id.starts_with("authority-requirement:"))
    );
    // With no realization descriptor nothing is known incompatible.
    assert!(
        prep.blockers
            .iter()
            .all(|b| b.class == BlockerClass::Unestablished)
    );
    assert!(prep.realization_evaluation.is_none());
}

#[test]
fn preparation_serializes_and_reads_back_without_loss() {
    let prep = prepare(&candidate(), &request(Some(matching_claim()))).unwrap();
    let text = serde_json::to_string(&prep).unwrap();
    let read: ReviewPreparation = serde_json::from_str(&text).unwrap();
    assert_eq!(read, prep);
    // The rendering of the model context is a serialization of derived data.
    let context = prep.model_context.to_json();
    assert_eq!(
        context["obligation"]["result_contract_id"],
        "implementation-review"
    );
    assert_eq!(context["obligation"]["result_contract_version"], 1);
    assert_eq!(context["conditions"][0]["boundary"], "builder_projection");
    assert!(
        context.get("blockers").is_none(),
        "derivation and blockers stay separate"
    );
}

#[test]
fn changing_an_owned_proposition_or_route_changes_the_constructed_context() {
    let base = prepare(&candidate(), &request(None)).unwrap();
    let changed_bytes = edited(|v| {
        for_each_condition(v, |c| {
            c["proposition"] = json!("A different proposition for the same boundary.");
            c["unestablishedRoute"] = json!("route-after-owner-change");
        });
    });
    let changed = prepare(&candidate_from(&changed_bytes), &request(None)).unwrap();
    assert_ne!(
        changed.model_context.identity.candidate_content_digest,
        base.model_context.identity.candidate_content_digest
    );
    for condition in &changed.model_context.conditions {
        assert_eq!(
            condition.proposition,
            "A different proposition for the same boundary."
        );
        assert_eq!(condition.unestablished_route, "route-after-owner-change");
    }
    assert_ne!(
        changed.model_context.conditions,
        base.model_context.conditions
    );
}

#[test]
fn removing_the_realization_constraints_removes_their_enforcement_rows() {
    let base = prepare(&candidate(), &request(None)).unwrap();
    assert!(
        base.host_enforcement
            .rows
            .iter()
            .any(|r| r.property == "realization:provider_family")
    );
    let bytes = edited(|v| {
        for_each_condition(v, |c| {
            c.as_object_mut().unwrap().remove("realizationConstraints");
        })
    });
    let changed = prepare(&candidate_from(&bytes), &request(Some(matching_claim()))).unwrap();
    assert!(
        !changed
            .host_enforcement
            .rows
            .iter()
            .any(|r| r.property.starts_with("realization:"))
    );
    assert!(
        changed
            .model_context
            .conditions
            .iter()
            .all(|c| c.constraints.is_empty())
    );
    assert_ne!(changed.host_enforcement.rows, base.host_enforcement.rows);
    // With no constraint there is nothing for the descriptor's provider to be checked against.
    for condition in &changed.realization_evaluation.as_ref().unwrap().conditions {
        assert!(
            !condition
                .checks
                .iter()
                .any(|c| c.property.starts_with("realization:"))
        );
    }
}

#[test]
fn a_changed_requirement_without_a_mechanism_is_refused_by_preparation_not_by_the_owner() {
    // Retained continuity: the owner accepts it and derives a candidate; preparation refuses.
    let retained =
        edited(|v| for_each_condition(v, |c| c["normative"]["continuity"] = json!("retained")));
    let candidate = candidate_from(&retained);
    match prepare(&candidate, &request(None)) {
        Err(PreparationRefusal::UnsupportedMandatorySemantics { rows }) => {
            assert_eq!(rows.len(), 2, "{rows:?}");
            assert!(
                rows.iter()
                    .all(|r| r.property == "continuity" && r.requirement == "retained")
            );
        }
        other => panic!("expected an unsupported-semantics refusal, got {other:?}"),
    }

    // A stated independence requirement is non-executable in v1.
    let independent = edited(|v| {
        for_each_condition(v, |c| {
            c["normative"]["independence"] = json!({"from": ["state.builder_context"]});
        })
    });
    match prepare(&candidate_from(&independent), &request(None)) {
        Err(PreparationRefusal::UnsupportedMandatorySemantics { rows }) => {
            assert!(
                rows.iter().all(|r| r.property == "independence"),
                "{rows:?}"
            );
            assert_eq!(rows.len(), 2);
        }
        other => panic!("expected an unsupported-semantics refusal, got {other:?}"),
    }
}

#[test]
fn unsupported_result_contracts_are_refused_with_the_contract_named() {
    for (id, version) in [("implementation-review", 2), ("another-review", 1)] {
        let bytes = edited(|v| {
            v["obligations"][0]["resultContract"] = json!({"id": id, "version": version});
        });
        match prepare(&candidate_from(&bytes), &request(None)) {
            Err(PreparationRefusal::UnsupportedResultContract {
                obligation,
                requirement,
            }) => {
                assert_eq!(obligation, OBLIGATION);
                assert_eq!(requirement, format!("{id}@{version}"));
            }
            other => panic!("expected a result-contract refusal for {id}@{version}, got {other:?}"),
        }
    }
}

#[test]
fn values_the_schema_owner_rejects_never_reach_preparation() {
    // `mutation` has the single value `none`; capabilities must be empty. The owner refuses the
    // bytes, so there is no candidate for preparation to refuse.
    let writes = edited(|v| for_each_condition(v, |c| c["normative"]["mutation"] = json!("write")));
    assert!(OrganizationalRequirements::from_json_bytes(&writes).is_err());
    let capable = edited(|v| {
        for_each_condition(v, |c| {
            c["normative"]["capabilities"] = json!(["repository_write"]);
        })
    });
    assert!(OrganizationalRequirements::from_json_bytes(&capable).is_err());
}

#[test]
fn unknown_obligations_and_invalid_subjects_are_refused() {
    let candidate = candidate();
    let mut unknown = request(None);
    unknown.obligation_id = "no-such-obligation".to_owned();
    assert_eq!(
        prepare(&candidate, &unknown),
        Err(PreparationRefusal::UnknownObligation(
            "no-such-obligation".to_owned()
        ))
    );
    let mut bad = request(None);
    bad.subject = Subject {
        commit: String::new(),
        ..subject()
    };
    assert!(matches!(
        prepare(&candidate, &bad),
        Err(PreparationRefusal::InvalidSubject(_))
    ));
}

#[test]
fn a_matching_descriptor_is_a_claim_and_clears_no_blocker() {
    let base = prepare(&candidate(), &request(None)).unwrap();
    let matching = prepare(&candidate(), &request(Some(matching_claim()))).unwrap();
    let evaluation = matching.realization_evaluation.as_ref().unwrap();
    assert_eq!(evaluation.standing, "supplied_claim_not_admission_evidence");
    for condition in &evaluation.conditions {
        assert!(
            condition
                .checks
                .iter()
                .all(|c| c.disposition == Disposition::ClaimedCompatible),
            "{condition:?}"
        );
    }
    // Every unconditional blocker of the descriptor-less preparation is still present, and the
    // matching descriptor added no known incompatibility.
    assert_eq!(blocker_ids(&matching), blocker_ids(&base));
    assert!(
        matching
            .blockers
            .iter()
            .all(|b| b.class == BlockerClass::Unestablished)
    );
}

#[test]
fn incompatible_descriptors_add_known_incompatibilities() {
    let base = blocker_ids(&prepare(&candidate(), &request(None)).unwrap());

    let wrong_provider = RealizationClaim {
        provider_family: Some("openai".to_owned()),
        ..matching_claim()
    };
    let prep = prepare(&candidate(), &request(Some(wrong_provider))).unwrap();
    let incompatible: Vec<_> = prep
        .blockers
        .iter()
        .filter(|b| b.class == BlockerClass::KnownIncompatibility)
        .map(|b| b.id.as_str())
        .collect();
    assert_eq!(
        incompatible,
        vec![
            format!("realization:{COND_BUILDER}:realization:provider_family"),
            format!("realization:{COND_CAMPAIGN}:realization:provider_family"),
        ]
    );
    assert!(base.is_subset(&blocker_ids(&prep)));

    // Silence about confinement is unestablished, not incompatible.
    let silent = RealizationClaim {
        execution_confinement: None,
        ..matching_claim()
    };
    let prep = prepare(&candidate(), &request(Some(silent))).unwrap();
    assert!(
        prep.blockers
            .iter()
            .all(|b| b.class == BlockerClass::Unestablished)
    );
    assert!(blocker_ids(&prep).contains(&format!(
        "realization:{COND_BUILDER}:realization:execution_confinement"
    )));

    // A claimed read-only realization that names a tool cannot be checked: no owner classifies tools.
    let tooled = RealizationClaim {
        granted_tools: vec!["workspace_exec".to_owned()],
        ..matching_claim()
    };
    let prep = prepare(&candidate(), &request(Some(tooled))).unwrap();
    assert!(
        prep.blockers
            .iter()
            .all(|b| b.class == BlockerClass::Unestablished)
    );
    let mutation = prep.realization_evaluation.as_ref().unwrap().conditions[0]
        .checks
        .iter()
        .find(|c| c.property == "mutation")
        .unwrap();
    assert_eq!(mutation.disposition, Disposition::Unestablished);
}

#[test]
fn changing_only_the_provider_of_a_writing_coding_fixture_removes_no_other_blocker() {
    // Hand-built to mirror the P1-F coding realization (provider `fake_fixture`, tool
    // `workspace_exec`); it is not derived from the manual host's profile type.
    let mirror = RealizationClaim {
        provider_family: Some("fake_fixture".to_owned()),
        execution_confinement: Some("guarded_sandbox".to_owned()),
        mutation: MutationClaim::WritesPossible,
        granted_tools: vec!["workspace_exec".to_owned()],
        continuity: Some("fresh_initial".to_owned()),
    };
    let as_is = prepare(&candidate(), &request(Some(mirror.clone()))).unwrap();
    let provider_swapped = RealizationClaim {
        provider_family: Some("anthropic".to_owned()),
        ..mirror
    };
    let swapped = prepare(&candidate(), &request(Some(provider_swapped))).unwrap();

    let before = blocker_ids(&as_is);
    let after = blocker_ids(&swapped);
    let removed: BTreeSet<_> = before.difference(&after).cloned().collect();
    let expected: BTreeSet<_> = [COND_BUILDER, COND_CAMPAIGN]
        .iter()
        .map(|c| format!("realization:{c}:realization:provider_family"))
        .collect();
    assert_eq!(
        removed, expected,
        "only the provider blockers may disappear"
    );
    assert!(after.is_superset(&unconditional().into_iter().collect()));
    for cond in [COND_BUILDER, COND_CAMPAIGN] {
        let mutation = swapped
            .blockers
            .iter()
            .find(|b| b.id == format!("realization:{cond}:mutation"))
            .expect("mutation blocker remains");
        assert_eq!(mutation.class, BlockerClass::KnownIncompatibility);
    }
}

#[test]
fn separation_has_no_mechanism_in_the_report_and_stays_a_blocker() {
    let candidate = candidate();
    let report = enforceability(&candidate, Profile::FixedSliceReviewV1);
    assert!(!report.executable());
    let none_rows: Vec<_> = report
        .rows
        .iter()
        .filter(|r| r.status == EnforceStatus::None)
        .collect();
    assert!(none_rows.iter().all(|r| r.property == "separation"));
    let prep = prepare(&candidate, &request(Some(matching_claim()))).unwrap();
    for cond in [COND_BUILDER, COND_CAMPAIGN] {
        assert!(blocker_ids(&prep).contains(&format!("separation:{cond}")));
    }
}

// ---------------------------------------------------------------------------------------------
// ORG-C1-001: obligation and condition ids are separate namespaces in the compiler, so the row
// and provenance selection must follow ownership, not a shared string. Every variant below is the
// frozen requirements bytes edited in memory; the compiler (not this crate) accepts them, which
// each test asserts before relying on the overlap.
// ---------------------------------------------------------------------------------------------

const OTHER_DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SECOND_CONDITION: &str = "cond-second-campaign";

/// The frozen requirements plus a second obligation `id` (result contract
/// `implementation-review@contract_version`) with its own campaign-boundary condition
/// `condition_id`. `edit` adjusts only that new condition.
fn with_second_obligation(
    id: &str,
    contract_version: u64,
    condition_id: &str,
    edit: impl FnOnce(&mut Value),
) -> Vec<u8> {
    edited(|v| {
        v["obligations"].as_array_mut().unwrap().push(json!({
            "id": id,
            "kind": "candidate_review",
            "subject": "campaign_candidate",
            "resultContract": {"id": "implementation-review", "version": contract_version},
            "scope": {"digest": OTHER_DIGEST},
        }));
        let mut condition = v["acceptanceConditions"][1].clone();
        assert_eq!(condition["id"], COND_CAMPAIGN);
        condition["id"] = json!(condition_id);
        condition["obligation"] = json!(id);
        edit(&mut condition);
        v["acceptanceConditions"]
            .as_array_mut()
            .unwrap()
            .push(condition);
    })
}

fn request_for(obligation_id: &str) -> work_engine_review_preparation::PreparationRequest {
    work_engine_review_preparation::PreparationRequest {
        obligation_id: obligation_id.to_owned(),
        ..request(None)
    }
}

/// `(property, scope)` of every enforcement row, in the row order the preparation carries.
fn row_keys(prep: &ReviewPreparation) -> Vec<(String, String)> {
    prep.host_enforcement
        .rows
        .iter()
        .map(|r| (r.property.clone(), r.scope.clone()))
        .collect()
}

/// Everything preparation says about the frozen obligation must be what it says about the same
/// obligation in the frozen candidate alone: another obligation in the candidate is not part of
/// this obligation's context, whatever its ids are.
fn assert_first_obligation_is_unchanged_by_the_second(
    prep: &ReviewPreparation,
    baseline: &ReviewPreparation,
) {
    assert_eq!(
        prep.model_context.obligation,
        baseline.model_context.obligation
    );
    assert_eq!(prep.model_context.vantage, baseline.model_context.vantage);
    assert_eq!(
        prep.model_context.conditions,
        baseline.model_context.conditions
    );
    assert_eq!(prep.host_enforcement.rows, baseline.host_enforcement.rows);
    assert_eq!(
        prep.host_enforcement.separation,
        baseline.host_enforcement.separation
    );
    assert_eq!(prep.provenance, baseline.provenance);
    assert_eq!(blocker_ids(prep), blocker_ids(baseline));
}

#[test]
fn an_obligation_named_like_another_obligations_condition_does_not_leak_its_result_contract_row() {
    let baseline = prepare(&candidate(), &request(None)).unwrap();
    // `implementation-review@2` has no mechanism; `@1` has one. The first must not make the
    // frozen obligation refuse, and neither may leave a row or trace of the second in it.
    for version in [2, 1] {
        let bytes = with_second_obligation(COND_BUILDER, version, SECOND_CONDITION, |_| {});
        let candidate = candidate_from(&bytes);

        // The owner really does carry the overlap: an obligation and a different obligation's
        // condition share one id, and the report holds rows scoped by that id for both.
        assert!(
            candidate
                .obligations()
                .iter()
                .any(|o| o.id.as_str() == COND_BUILDER)
        );
        assert_eq!(
            candidate
                .conditions()
                .iter()
                .filter(|c| c.id.as_str() == COND_BUILDER)
                .count(),
            1
        );
        let report = enforceability(&candidate, Profile::FixedSliceReviewV1);
        for property in ["resultContract", "continuity"] {
            assert!(
                report
                    .rows
                    .iter()
                    .any(|r| r.scope == COND_BUILDER && r.property == property),
                "the owner's report has a `{property}` row scoped `{COND_BUILDER}`"
            );
        }

        let prep = prepare(&candidate, &request(None)).unwrap_or_else(|refusal| {
            panic!(
                "the frozen obligation refused over another obligation (@{version}): {refusal:?}"
            )
        });
        assert_first_obligation_is_unchanged_by_the_second(&prep, &baseline);
        assert_eq!(
            prep.host_enforcement
                .rows
                .iter()
                .filter(|r| r.property == "resultContract")
                .map(|r| (r.scope.as_str(), r.requirement.as_str()))
                .collect::<Vec<_>>(),
            vec![(OBLIGATION, "implementation-review@1")],
            "exactly the frozen obligation's own contract row, never the second obligation's"
        );
        assert!(
            prep.provenance
                .iter()
                .all(|p| !(p.target.kind == "obligation" && p.target.id == COND_BUILDER)),
            "no trace row of the second obligation"
        );
    }
}

#[test]
fn the_second_obligation_selects_only_its_own_contract_and_condition() {
    // Supported contract: the colliding id must select the second obligation's own contract row
    // and its own condition, and none of the frozen obligation's condition of that id.
    let candidate = candidate_from(&with_second_obligation(
        COND_BUILDER,
        1,
        SECOND_CONDITION,
        |_| {},
    ));
    let prep = prepare(&candidate, &request_for(COND_BUILDER)).expect("prepares");
    assert_eq!(prep.model_context.obligation.id, COND_BUILDER);
    let condition_ids: Vec<_> = prep
        .model_context
        .conditions
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(condition_ids, vec![SECOND_CONDITION]);
    let keys = row_keys(&prep);
    assert_eq!(
        keys.iter()
            .filter(|(_, scope)| scope == COND_BUILDER)
            .collect::<Vec<_>>(),
        vec![&("resultContract".to_owned(), COND_BUILDER.to_owned())],
        "the obligation id selects its contract row only; the frozen condition of that id is not its own"
    );
    assert!(
        keys.iter()
            .filter(|(property, _)| property != "resultContract")
            .all(|(_, scope)| scope == SECOND_CONDITION),
        "{keys:?}"
    );
    assert!(
        keys.iter()
            .any(|(property, scope)| property == "continuity" && scope == SECOND_CONDITION)
    );
    for p in &prep.provenance {
        match p.target.kind.as_str() {
            "obligation" => assert_eq!(p.target.id, COND_BUILDER),
            // A separation requirement is keyed by its owning condition.
            "condition" | "separationRequirement" => assert_eq!(p.target.id, SECOND_CONDITION),
            other => panic!("trace row of kind `{other}` is not owned by the obligation: {p:?}"),
        }
    }
    assert!(
        prep.provenance
            .iter()
            .any(|p| p.target.kind == "obligation")
    );
    assert!(prep.provenance.iter().any(|p| p.target.kind == "condition"));

    // Unsupported contract: refused, and the refusal names the second obligation's own contract.
    let unsupported = candidate_from(&with_second_obligation(
        COND_BUILDER,
        2,
        SECOND_CONDITION,
        |_| {},
    ));
    match prepare(&unsupported, &request_for(COND_BUILDER)) {
        Err(PreparationRefusal::UnsupportedResultContract {
            obligation,
            requirement,
        }) => {
            assert_eq!(obligation, COND_BUILDER);
            assert_eq!(requirement, "implementation-review@2");
        }
        other => panic!("expected the second obligation's own contract refusal, got {other:?}"),
    }
}

#[test]
fn a_condition_named_like_another_obligation_does_not_leak_its_rows_into_that_obligation() {
    let baseline = prepare(&candidate(), &request(None)).unwrap();
    // The second obligation's only condition carries the frozen obligation's id.
    let candidate = candidate_from(&with_second_obligation(
        "second-review",
        1,
        OBLIGATION,
        |_| {},
    ));
    let report = enforceability(&candidate, Profile::FixedSliceReviewV1);
    for property in ["resultContract", "continuity"] {
        assert!(
            report
                .rows
                .iter()
                .any(|r| r.scope == OBLIGATION && r.property == property),
            "the owner's report has a `{property}` row scoped `{OBLIGATION}`"
        );
    }
    let prep = prepare(&candidate, &request(None)).expect("prepares");
    assert_first_obligation_is_unchanged_by_the_second(&prep, &baseline);

    // And the other way: the second obligation does not take the frozen obligation's contract row.
    let second = prepare(&candidate, &request_for("second-review")).expect("prepares");
    assert_eq!(
        row_keys(&second)
            .iter()
            .filter(|(property, _)| property == "resultContract")
            .collect::<Vec<_>>(),
        vec![&("resultContract".to_owned(), "second-review".to_owned())]
    );
    let owned: Vec<_> = row_keys(&second)
        .into_iter()
        .filter(|(_, scope)| scope == OBLIGATION)
        .collect();
    assert!(!owned.is_empty());
    assert!(
        owned
            .iter()
            .all(|(property, _)| property != "resultContract"),
        "{owned:?}"
    );
    assert!(
        second
            .provenance
            .iter()
            .all(|p| !(p.target.kind == "obligation" && p.target.id == OBLIGATION)),
        "the frozen obligation's own trace rows are not the second obligation's"
    );
}

#[test]
fn another_obligations_unsupported_condition_does_not_refuse_an_obligation_it_does_not_own() {
    let baseline = prepare(&candidate(), &request(None)).unwrap();
    // The second obligation's condition (id = the frozen obligation's id) requires retained
    // continuity, which has no mechanism. That is the second obligation's refusal alone.
    let bytes = with_second_obligation("second-review", 1, OBLIGATION, |c| {
        c["normative"]["continuity"] = json!("retained");
    });
    let candidate = candidate_from(&bytes);
    let prep = prepare(&candidate, &request(None)).unwrap_or_else(|refusal| {
        panic!("the frozen obligation refused over another obligation's condition: {refusal:?}")
    });
    assert_first_obligation_is_unchanged_by_the_second(&prep, &baseline);

    match prepare(&candidate, &request_for("second-review")) {
        Err(PreparationRefusal::UnsupportedMandatorySemantics { rows }) => {
            assert_eq!(rows.len(), 1, "{rows:?}");
            assert_eq!(rows[0].property, "continuity");
            assert_eq!(rows[0].scope, OBLIGATION);
            assert_eq!(rows[0].requirement, "retained");
        }
        other => panic!("expected the second obligation's own refusal, got {other:?}"),
    }
}

#[test]
fn trace_rows_of_other_kinds_with_a_coincident_id_are_not_provenance() {
    // `vantage-builder` is the id of the builder vantage participant. An obligation may carry it
    // too; the participant's trace rows are the compiler's own, not this obligation's.
    let candidate = candidate_from(&with_second_obligation(
        "vantage-builder",
        1,
        SECOND_CONDITION,
        |_| {},
    ));
    assert!(
        candidate
            .requirement_trace()
            .iter()
            .any(|r| r.target.kind == "participant" && r.target.id == "vantage-builder"),
        "the owner traces the builder vantage participant under the coincident id"
    );
    let prep = prepare(&candidate, &request_for("vantage-builder")).expect("prepares");
    assert!(!prep.provenance.is_empty());
    for p in &prep.provenance {
        match p.target.kind.as_str() {
            "obligation" => assert_eq!(p.target.id, "vantage-builder"),
            "condition" | "separationRequirement" => assert_eq!(p.target.id, SECOND_CONDITION),
            other => panic!("trace row of kind `{other}` is not owned by the obligation: {p:?}"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// ORG-C1-002: the compiler keys a separation requirement's trace rows by its owning condition and
// sources them from that condition's `acceptor` and `obligation` and from every builder-class
// condition's `consumerClass`, which may belong to another obligation. Preparation emits the
// separation requirement and its OC2 blocker, so those rows are its provenance.
// ---------------------------------------------------------------------------------------------

type Endpoint = (String, String, String);

/// The owner's trace rows for one condition's separation requirement, read from the canonical
/// candidate by the owner's own key (target kind and owning condition id).
fn canonical_separation_trace(
    candidate: &work_engine_organization::OrganizationCandidate,
    condition: &str,
) -> Vec<(Endpoint, Endpoint)> {
    candidate
        .requirement_trace()
        .iter()
        .filter(|row| row.target.kind == "separationRequirement" && row.target.id == condition)
        .map(|row| {
            (
                (
                    row.target.kind.to_owned(),
                    row.target.id.clone(),
                    row.target.field.clone(),
                ),
                (
                    row.source.kind.to_owned(),
                    row.source.id.clone(),
                    row.source.field.clone(),
                ),
            )
        })
        .collect()
}

fn emitted_separation_trace(
    prep: &ReviewPreparation,
    condition: &str,
) -> Vec<(Endpoint, Endpoint)> {
    prep.provenance
        .iter()
        .filter(|p| p.target.kind == "separationRequirement" && p.target.id == condition)
        .map(|p| {
            (
                (
                    p.target.kind.clone(),
                    p.target.id.clone(),
                    p.target.field.clone(),
                ),
                (
                    p.source.kind.clone(),
                    p.source.id.clone(),
                    p.source.field.clone(),
                ),
            )
        })
        .collect()
}

fn source(kind: &str, id: &str, field: &str) -> Endpoint {
    (kind.to_owned(), id.to_owned(), field.to_owned())
}

/// Every separation requirement preparation emits carries exactly the owner's trace rows for it,
/// and no separation row of a condition the obligation does not own. Returns the number of
/// requirements checked so a caller can reject a vacuous run.
fn assert_separation_provenance_is_canonical(
    candidate: &work_engine_organization::OrganizationCandidate,
    prep: &ReviewPreparation,
) -> usize {
    let emitted_ids: BTreeSet<&str> = prep
        .host_enforcement
        .separation
        .iter()
        .map(|s| s.condition.as_str())
        .collect();
    assert_eq!(
        emitted_ids,
        prep.model_context
            .conditions
            .iter()
            .map(|c| c.id.as_str())
            .collect::<BTreeSet<_>>(),
        "one separation requirement per selected condition"
    );
    for separation in &prep.host_enforcement.separation {
        let expected = canonical_separation_trace(candidate, &separation.condition);
        assert!(
            !expected.is_empty(),
            "the owner traces the separation requirement of `{}`",
            separation.condition
        );
        assert_eq!(
            emitted_separation_trace(prep, &separation.condition),
            expected,
            "separation requirement `{}` keeps the owner's trace rows, source endpoints unchanged",
            separation.condition
        );
    }
    // Nothing else of that kind: every separation row belongs to a selected condition.
    for p in prep
        .provenance
        .iter()
        .filter(|p| p.target.kind == "separationRequirement")
    {
        assert!(emitted_ids.contains(p.target.id.as_str()), "{p:?}");
    }
    emitted_ids.len()
}

#[test]
fn separation_requirements_keep_the_owners_trace_rows() {
    let candidate = candidate();
    let prep = prepare(&candidate, &request(None)).expect("prepares");
    assert_eq!(
        assert_separation_provenance_is_canonical(&candidate, &prep),
        2
    );
    // The expected rows are the compiler's: acceptor and obligation of the owning condition, and
    // the builder condition's consumer class.
    for condition in [COND_BUILDER, COND_CAMPAIGN] {
        let rows = emitted_separation_trace(&prep, condition);
        let sources: BTreeSet<Endpoint> = rows.into_iter().map(|(_, source)| source).collect();
        assert_eq!(
            sources,
            BTreeSet::from([
                source("acceptanceCondition", condition, "acceptor"),
                source("acceptanceCondition", condition, "obligation"),
                source("acceptanceCondition", COND_BUILDER, "consumerClass"),
            ])
        );
    }
}

#[test]
fn a_builder_condition_of_another_obligation_still_contributes_to_the_selected_separation() {
    // The second obligation's campaign-boundary condition is separated from the frozen
    // obligation's builder condition: the contributing source belongs to another obligation and
    // is exactly the row the owner traces.
    let candidate = candidate_from(&with_second_obligation(
        "second-review",
        1,
        SECOND_CONDITION,
        |_| {},
    ));
    let prep = prepare(&candidate, &request_for("second-review")).expect("prepares");
    assert_eq!(
        assert_separation_provenance_is_canonical(&candidate, &prep),
        1
    );
    let owner = candidate
        .conditions()
        .iter()
        .find(|c| c.id.as_str() == COND_BUILDER)
        .expect("the frozen builder condition");
    assert_eq!(owner.obligation.as_str(), OBLIGATION);
    assert_ne!(owner.obligation.as_str(), "second-review");
    let sources: BTreeSet<Endpoint> = emitted_separation_trace(&prep, SECOND_CONDITION)
        .into_iter()
        .map(|(_, source)| source)
        .collect();
    assert_eq!(
        sources,
        BTreeSet::from([
            source("acceptanceCondition", SECOND_CONDITION, "acceptor"),
            source("acceptanceCondition", SECOND_CONDITION, "obligation"),
            source("acceptanceCondition", COND_BUILDER, "consumerClass"),
        ])
    );
    // The frozen obligation's own separation rows are not the second obligation's.
    assert!(
        prep.provenance
            .iter()
            .filter(|p| p.target.kind == "separationRequirement")
            .all(|p| p.target.id == SECOND_CONDITION)
    );
}

#[test]
fn a_builder_condition_added_by_another_obligation_extends_the_selected_separation_rows() {
    // The other direction: a builder-class condition under a second obligation contributes its
    // `consumerClass` to the frozen conditions' separation requirements, and preparation of the
    // frozen obligation carries it unchanged.
    let bytes = with_second_obligation("second-review", 1, SECOND_CONDITION, |c| {
        c["consumerClass"] = json!("builder");
        c["consumptionBoundary"] = json!("builder_projection");
    });
    let candidate = candidate_from(&bytes);
    let prep = prepare(&candidate, &request(None)).expect("prepares");
    assert_eq!(
        assert_separation_provenance_is_canonical(&candidate, &prep),
        2
    );
    for condition in [COND_BUILDER, COND_CAMPAIGN] {
        let sources: BTreeSet<Endpoint> = emitted_separation_trace(&prep, condition)
            .into_iter()
            .map(|(_, source)| source)
            .collect();
        assert!(
            sources.contains(&source(
                "acceptanceCondition",
                SECOND_CONDITION,
                "consumerClass"
            )),
            "`{condition}` carries the contribution of the other obligation's builder condition: {sources:?}"
        );
    }
    assert!(
        prep.provenance
            .iter()
            .filter(|p| p.target.kind == "separationRequirement")
            .all(|p| p.target.id != SECOND_CONDITION),
        "the other obligation's own separation requirement is not this obligation's"
    );
}

/// The fixture route selects through the same code, so the same overlap must not change what it
/// covers for the frozen obligation. Not in `tests/fixture_consumer.rs`, which this change does
/// not own; it needs only the declaration, never an episode.
#[cfg(feature = "fixture")]
mod fixture_route {
    use super::*;
    use work_engine_review_preparation::fixture::{
        FIXTURE_LABEL, FixtureDeclaration, fixture_coverage,
    };

    fn declaration_for(
        candidate: &work_engine_organization::OrganizationCandidate,
    ) -> FixtureDeclaration {
        let selection: Value = serde_json::from_slice(SELECTION).unwrap();
        let mut bindings = Vec::new();
        for claim in selection["specialists"][0]["requiredClaims"]
            .as_array()
            .unwrap()
        {
            let boundary = claim["consumptionBoundary"].as_str().unwrap();
            let (condition, participant) = match boundary {
                "builder_projection" => (COND_BUILDER, "vantage-builder"),
                "campaign_terminalization" => (COND_CAMPAIGN, "component:campaign"),
                other => panic!("unexpected boundary {other}"),
            };
            bindings.push(json!({
                "condition_id": condition,
                "boundary": boundary,
                "consumer_participant": participant,
                "admission_consumer": claim["consumer"],
                "expected": {
                    "claim_revision": reference("fixture-claims", "claim", "r1"),
                    "establishment": reference("fixture-establisher", "establishment", "r1"),
                    "observation": reference("fixture-observer", "observation", "r1"),
                    "consumption": reference("fixture-consumption", "consumption", "r1"),
                },
            }));
        }
        serde_json::from_value(json!({
            "assumption_label": FIXTURE_LABEL,
            "candidate_content_digest": candidate.content_digest().to_hex(),
            "obligation_id": OBLIGATION,
            "problem_scope": {
                "run_id": "production-qualification-20261107",
                "plan_lineage": "timestamp-projection-v1",
                "slice_number": 1,
            },
            "subject": {
                "commit": subject().commit,
                "tree": subject().tree,
                "patchIdentity": subject().patch_identity,
            },
            "episode": {
                "run_id": "run", "slice_number": 1, "attempt_id": "attempt",
                "plan_version": "plan", "review_obligation_id": OBLIGATION,
                "review_episode_id": "episode",
            },
            "bindings": bindings,
        }))
        .unwrap()
    }

    #[test]
    fn another_obligations_coincident_id_does_not_change_fixture_coverage() {
        let baseline_candidate = candidate();
        let baseline = fixture_coverage(&baseline_candidate, &declaration_for(&baseline_candidate))
            .expect("the frozen candidate is covered");
        for version in [2, 1] {
            let candidate = candidate_from(&with_second_obligation(
                COND_BUILDER,
                version,
                SECOND_CONDITION,
                |_| {},
            ));
            let coverage = fixture_coverage(&candidate, &declaration_for(&candidate))
                .unwrap_or_else(|refusal| {
                    panic!(
                        "fixture coverage refused over another obligation (@{version}): {refusal:?}"
                    )
                });
            assert_eq!(coverage, baseline);
        }
    }
}
