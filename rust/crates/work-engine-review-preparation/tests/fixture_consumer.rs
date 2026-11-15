//! Fixture consumer (feature `fixture`): synthetic declarations, the real episode reducer.
//!
//! Every declaration here is a test assumption built from the frozen selection and episode
//! authority; none is establishment evidence. The negative tests assert two things for each
//! refusal: the refusal is the specific one, and the borrowed input state is byte-identical
//! afterwards.
mod common;

use common::*;
use review_episode_core::ErrorKind;
use review_episode_core::codec::{JsValue, canonical_json, parse_json};
use review_episode_core::command::{Action, TransitionCommand};
use review_episode_core::identity::{Authority, Revision};
use review_episode_core::implementation_review_v1::ReviewResult;
use review_episode_core::reducer::transition;
use review_episode_core::state::{Phase, ReviewEpisodeState};
use serde_json::{Value, json};
use work_engine_organization::OrganizationCandidate;
use work_engine_review_preparation::fixture::{
    Acceptance, ConsumeRefusal, EvidenceStanding, FIXTURE_LABEL, FixtureConsumption,
    FixtureDeclaration, ProductionConsequence, consume_fixture_result, fixture_coverage,
};
use work_engine_review_preparation::{PreparationRefusal, prepare};

struct World {
    candidate: OrganizationCandidate,
    declaration: FixtureDeclaration,
    admissions: Vec<Value>,
}

fn identity_of(authority: &Value, key: &str) -> Value {
    authority["identity"][key].clone()
}

/// A synthetic world: the declaration and the admissions that agree with it.
fn world() -> World {
    let candidate = candidate();
    let selection: Value = serde_json::from_slice(SELECTION).unwrap();
    let authority: Value = serde_json::from_str(AUTHORITY).unwrap();
    let mut bindings = Vec::new();
    let mut admissions = Vec::new();
    for claim in selection["specialists"][0]["requiredClaims"]
        .as_array()
        .unwrap()
    {
        let boundary = claim["consumptionBoundary"].as_str().unwrap();
        let consumer = claim["consumer"].as_str().unwrap();
        let condition = match boundary {
            "builder_projection" => COND_BUILDER,
            "campaign_terminalization" => COND_CAMPAIGN,
            other => panic!("unexpected boundary {other}"),
        };
        let claim_ref = reference(
            "fixture-claims",
            claim["claimId"].as_str().unwrap(),
            claim["revision"].as_str().unwrap(),
        );
        let establishment = reference(
            "fixture-establisher",
            &format!("establishment:{boundary}"),
            "est-r1",
        );
        let observation = reference(
            "fixture-observer",
            &format!("observation:{boundary}"),
            "obs-r1",
        );
        let consumption = reference(
            "fixture-consumption",
            &format!("consumption:{boundary}"),
            "cons-r1",
        );
        bindings.push(json!({
            "condition_id": condition,
            "boundary": boundary,
            "consumer_participant": if boundary == "builder_projection" { "vantage-builder" } else { "component:campaign" },
            "admission_consumer": consumer,
            "expected": {
                "claim_revision": claim_ref,
                "establishment": establishment,
                "observation": observation,
                "consumption": consumption,
            },
        }));
        admissions.push(json!({
            "claimRevisionRef": claim_ref,
            "establishmentRef": establishment,
            "observationRef": observation,
            "consumptionRef": consumption,
            "status": "established",
            "boundary": boundary,
            "consumer": consumer,
        }));
    }
    let declaration: FixtureDeclaration = serde_json::from_value(json!({
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
            "run_id": identity_of(&authority, "runId"),
            "slice_number": identity_of(&authority, "sliceNumber"),
            "attempt_id": identity_of(&authority, "attemptId"),
            "plan_version": identity_of(&authority, "planVersion"),
            "review_obligation_id": identity_of(&authority, "reviewObligationId"),
            "review_episode_id": identity_of(&authority, "reviewEpisodeId"),
        },
        "bindings": bindings,
    }))
    .unwrap();
    World {
        candidate,
        declaration,
        admissions,
    }
}

fn text(admissions: &[Value]) -> String {
    serde_json::to_string(admissions).unwrap()
}

fn try_consume(
    w: &World,
    declaration: &FixtureDeclaration,
    state: &ReviewEpisodeState,
    authority: &Authority,
    revision: Revision,
    result: &str,
    admissions: &[Value],
) -> Result<FixtureConsumption, ConsumeRefusal> {
    consume_fixture_result(
        &w.candidate,
        declaration,
        state,
        authority,
        revision,
        "result-1",
        result,
        &[],
        &text(admissions),
    )
}

/// Consumes with the world's own declaration and `admissions`, from the begun episode.
fn consume_with(w: &World, admissions: &[Value]) -> Result<FixtureConsumption, ConsumeRefusal> {
    let state = begun();
    try_consume(
        w,
        &w.declaration,
        &state,
        &authority(),
        state.revision().clone(),
        &result_json(|_| {}),
        admissions,
    )
}

/// The refusal for `admissions`, asserting that the input state is unchanged.
fn refused_with(w: &World, admissions: &[Value]) -> ConsumeRefusal {
    let state = begun();
    let before = canonical(&state);
    let error = try_consume(
        w,
        &w.declaration,
        &state,
        &authority(),
        state.revision().clone(),
        &result_json(|_| {}),
        admissions,
    )
    .expect_err("must be refused");
    assert_eq!(
        canonical(&state),
        before,
        "a refusal leaves the input state unchanged"
    );
    error
}

fn refused_declaration(w: &World, edit: impl FnOnce(&mut Value)) -> ConsumeRefusal {
    let mut value = serde_json::to_value(&w.declaration).unwrap();
    edit(&mut value);
    let declaration: FixtureDeclaration = serde_json::from_value(value).unwrap();
    let state = begun();
    let before = canonical(&state);
    let error = try_consume(
        w,
        &declaration,
        &state,
        &authority(),
        state.revision().clone(),
        &result_json(|_| {}),
        &w.admissions,
    )
    .expect_err("must be refused");
    assert_eq!(canonical(&state), before);
    error
}

#[test]
fn a_complete_consistent_admission_set_reaches_the_real_reducer_and_is_only_a_fixture_observation()
{
    let w = world();
    let state = begun();
    let before = canonical(&state);
    assert_eq!(state.phase(), Phase::InitialReview);

    let consumed = consume_with(&w, &w.admissions).expect("consumed");
    assert_eq!(canonical(&state), before, "the input state is not modified");
    assert!(!consumed.outcome().is_replay());
    assert_eq!(consumed.state().phase(), Phase::Reported);
    assert_ne!(consumed.state().revision(), state.revision());

    // Fixture observation only: no acceptance, no production consequence, references checked
    // for consistency with the fixture's own assumptions.
    assert_eq!(
        consumed.production_consequence,
        ProductionConsequence::Unestablished
    );
    assert_eq!(
        consumed.evidence,
        EvidenceStanding::ConsistentWithFixtureAssumptionsOnly
    );
    assert_eq!(consumed.conditions.len(), 2);
    for condition in &consumed.conditions {
        assert_eq!(condition.acceptance, Acceptance::NotEstablished);
        assert_eq!(
            condition.acceptor,
            "operator-production-qualification-20261107"
        );
        assert_eq!(
            condition.unestablished_route,
            "hold-consumption-for-owner-disposition"
        );
    }

    // The resulting episode state survives the owner's own serialization round trip.
    let stored = canonical_json(consumed.state().value());
    let read = ReviewEpisodeState::from_stored_json(&stored).expect("stored form reads");
    assert_eq!(read.revision(), consumed.state().revision());
    assert_eq!(read.phase(), Phase::Reported);
    assert_eq!(
        consumed.state().value().get("schemaVersion"),
        Some(&JsValue::Number(2.0))
    );
}

#[test]
fn the_episode_core_alone_reaches_reported_without_admissions_so_the_gate_is_what_stops_it() {
    // Control. The reducer does not know the organization's evidence requirement.
    let state = begun();
    let authority = authority();
    let payload = JsValue::object([
        ("result", parse_json(&result_json(|_| {})).unwrap()),
        ("unresolvedQuestions", JsValue::Array(vec![])),
    ]);
    let command = TransitionCommand::new(
        authority,
        state.revision().clone(),
        "result-1",
        Action::RecordResult,
        payload,
    )
    .unwrap();
    let direct = transition(&state, &command).unwrap().into_state();
    assert_eq!(direct.phase(), Phase::Reported);

    // The composed entry point refuses the same result with the same state.
    let w = world();
    assert!(matches!(
        refused_with(&w, &[]),
        ConsumeRefusal::AdmissionMissing { .. }
    ));
}

#[test]
fn coverage_is_derived_from_the_obligation_and_the_declared_mapping() {
    let w = world();
    let coverage = fixture_coverage(&w.candidate, &w.declaration).unwrap();
    assert_eq!(coverage.label, FIXTURE_LABEL);
    let ids: Vec<_> = coverage
        .conditions
        .iter()
        .map(|c| c.condition.as_str())
        .collect();
    assert_eq!(ids, vec![COND_BUILDER, COND_CAMPAIGN]);
    // `evidenceCustody` is an Observation-class row, so each requires an observation reference.
    assert!(coverage.conditions.iter().all(|c| c.requires_observation));
}

#[test]
fn removing_a_required_admission_cannot_advance_the_episode() {
    let w = world();
    assert!(matches!(
        refused_with(&w, &w.admissions[..1]),
        ConsumeRefusal::AdmissionMissing { condition } if condition == COND_CAMPAIGN
    ));
    assert!(matches!(
        refused_with(&w, &w.admissions[1..]),
        ConsumeRefusal::AdmissionMissing { condition } if condition == COND_BUILDER
    ));
    assert!(matches!(
        refused_with(&w, &[]),
        ConsumeRefusal::AdmissionMissing { .. }
    ));
}

#[test]
fn a_substituted_binding_or_reference_cannot_advance_the_episode() {
    let w = world();

    // Second admission carries the first condition's boundary and consumer: its own binding is
    // then missing and this key is declared for another condition.
    let mut swapped = w.admissions.clone();
    swapped[1]["boundary"] = swapped[0]["boundary"].clone();
    assert!(matches!(
        refused_with(&w, &swapped),
        ConsumeRefusal::AdmissionUnbound { index: 1, .. }
    ));

    // Same boundary, a consumer string nothing declares.
    let mut other_consumer = w.admissions.clone();
    other_consumer[0]["consumer"] = json!("slice-builder:someone-else");
    assert!(matches!(
        refused_with(&w, &other_consumer),
        ConsumeRefusal::AdmissionUnbound { index: 0, .. }
    ));

    // Two admissions for one binding.
    let doubled = vec![w.admissions[0].clone(), w.admissions[0].clone()];
    assert!(matches!(
        refused_with(&w, &doubled),
        ConsumeRefusal::AdmissionDuplicate { condition } if condition == COND_BUILDER
    ));

    // Each reference swapped with the other condition's, one field at a time.
    for (key, name) in [
        ("claimRevisionRef", "claimRevisionRef"),
        ("establishmentRef", "establishmentRef"),
        ("consumptionRef", "consumptionRef"),
        ("observationRef", "observationRef"),
    ] {
        let mut substituted = w.admissions.clone();
        substituted[1][key] = w.admissions[0][key].clone();
        match refused_with(&w, &substituted) {
            ConsumeRefusal::AdmissionReferenceMismatch {
                condition,
                reference,
            } => {
                assert_eq!(condition, COND_CAMPAIGN);
                assert_eq!(reference, name);
            }
            other => panic!("{key}: expected a reference mismatch, got {other:?}"),
        }
    }

    // A reference changed in place (wrong revision) is also wrong.
    let mut wrong = w.admissions.clone();
    wrong[0]["claimRevisionRef"]["revision"] = json!("not-the-declared-revision");
    assert!(matches!(
        refused_with(&w, &wrong),
        ConsumeRefusal::AdmissionReferenceMismatch {
            reference: "claimRevisionRef",
            ..
        }
    ));
}

#[test]
fn non_established_evidence_cannot_advance_the_episode_but_the_reducer_alone_would_record_it() {
    let w = world();
    for status in ["unestablished", "false"] {
        let mut admissions = w.admissions.clone();
        admissions[1]["status"] = json!(status);
        assert!(matches!(
            refused_with(&w, &admissions),
            ConsumeRefusal::EvidenceNotEstablished { condition, status: s } if condition == COND_CAMPAIGN && s == status
        ));
    }
    let mut no_observation = w.admissions.clone();
    no_observation[0]["observationRef"] = Value::Null;
    assert!(matches!(
        refused_with(&w, &no_observation),
        ConsumeRefusal::ObservationMissing { condition } if condition == COND_BUILDER
    ));
}

#[test]
fn a_boundary_and_consumer_that_merely_match_the_candidate_are_not_a_mapping() {
    // The admissions carry the candidate's own boundary strings and participant ids as their
    // boundary and consumer; the declaration binds nothing. String equality maps nothing.
    let w = world();
    let candidate_view = work_engine_organization::view(&w.candidate).unwrap();
    let matching_strings: Vec<Value> = candidate_view
        .conditions
        .iter()
        .map(|c| {
            let mut admission = w.admissions[0].clone();
            admission["boundary"] = json!(c.boundary.as_str());
            admission["consumer"] = json!(c.consumer);
            admission
        })
        .collect();
    let error = refused_declaration(&w, |v| v["bindings"] = json!([]));
    assert!(
        matches!(error, ConsumeRefusal::RequiredConditionUnmapped { .. }),
        "{error:?}"
    );
    // And with the full declaration, those admissions match no declared binding.
    assert!(matches!(
        refused_with(&w, &matching_strings),
        ConsumeRefusal::AdmissionUnbound { .. }
    ));
}

#[test]
fn declarations_that_disagree_with_the_candidate_the_subject_or_the_episode_are_refused() {
    let w = world();
    assert!(matches!(
        refused_declaration(&w, |v| v["assumption_label"] = json!("evidence")),
        ConsumeRefusal::UnlabelledAssumption
    ));
    assert!(matches!(
        refused_declaration(&w, |v| v["candidate_content_digest"] =
            json!("0".repeat(64))),
        ConsumeRefusal::DeclarationConflict {
            field: "candidate_content_digest",
            ..
        }
    ));
    assert!(matches!(
        refused_declaration(&w, |v| v["problem_scope"]["slice_number"] = json!(2)),
        ConsumeRefusal::DeclarationConflict {
            field: "problem_scope.slice_number",
            ..
        }
    ));
    // A different subject: the declaration no longer matches the episode's exact subject.
    assert!(matches!(
        refused_declaration(&w, |v| v["subject"]["tree"] = json!("0".repeat(40))),
        ConsumeRefusal::DeclarationConflict {
            field: "subject.sha256",
            ..
        }
    ));
    assert!(matches!(
        refused_declaration(&w, |v| v["episode"]["review_episode_id"] =
            json!("another-episode")),
        ConsumeRefusal::DeclarationConflict {
            field: "episode.review_episode_id",
            ..
        }
    ));
    assert!(matches!(
        refused_declaration(&w, |v| v["episode"]["attempt_id"] = json!("review-006")),
        ConsumeRefusal::DeclarationConflict {
            field: "episode.attempt_id",
            ..
        }
    ));
    assert!(matches!(
        refused_declaration(&w, |v| v["obligation_id"] = json!("no-such-obligation")),
        ConsumeRefusal::Selection(PreparationRefusal::UnknownObligation(_))
    ));

    // Binding set must equal the derived required set.
    assert!(matches!(
        refused_declaration(&w, |v| { v["bindings"].as_array_mut().unwrap().pop(); }),
        ConsumeRefusal::RequiredConditionUnmapped { condition } if condition == COND_CAMPAIGN
    ));
    assert!(matches!(
        refused_declaration(&w, |v| v["bindings"][1]["condition_id"] =
            json!("cond-not-required")),
        ConsumeRefusal::BindingNotRequired { .. }
    ));
    assert!(matches!(
        refused_declaration(&w, |v| v["bindings"][1]["condition_id"] =
            json!(COND_BUILDER)),
        ConsumeRefusal::DuplicateBinding { .. }
    ));
    assert!(matches!(
        refused_declaration(&w, |v| {
            let first = v["bindings"][0].clone();
            v["bindings"][1]["boundary"] = first["boundary"].clone();
            v["bindings"][1]["admission_consumer"] = first["admission_consumer"].clone();
        }),
        ConsumeRefusal::DeclarationConflict {
            field: "binding.boundary",
            ..
        } | ConsumeRefusal::AmbiguousBindingKey { .. }
    ));
    assert!(matches!(
        refused_declaration(&w, |v| v["bindings"][0]["boundary"] =
            json!("campaign_terminalization")),
        ConsumeRefusal::DeclarationConflict {
            field: "binding.boundary",
            ..
        }
    ));
    assert!(matches!(
        refused_declaration(&w, |v| v["bindings"][0]["consumer_participant"] =
            json!("component:campaign")),
        ConsumeRefusal::DeclarationConflict {
            field: "binding.consumer_participant",
            ..
        }
    ));
    assert!(matches!(
        refused_declaration(&w, |v| v["bindings"][0]["expected"]["observation"] = Value::Null),
        ConsumeRefusal::ObservationNotDeclared { condition } if condition == COND_BUILDER
    ));
}

#[test]
fn the_episode_owner_keeps_revision_writer_subject_and_phase_checks() {
    let w = world();
    let state = begun();
    let before = canonical(&state);
    let good = result_json(|_| {});

    // Stale expected revision.
    let stale = Revision("f".repeat(64));
    let error = try_consume(
        &w,
        &w.declaration,
        &state,
        &authority(),
        stale,
        &good,
        &w.admissions,
    )
    .expect_err("stale revision");
    assert!(
        matches!(&error, ConsumeRefusal::Episode(e) if e.kind == ErrorKind::RevisionConflict),
        "{error:?}"
    );

    // A writer that is not the current writer generation.
    let mut other: Value = serde_json::from_str(AUTHORITY).unwrap();
    other["writer"]["actorId"] = json!("a-different-writer");
    let other = Authority::parse(parse_json(&other.to_string()).unwrap()).unwrap();
    let error = try_consume(
        &w,
        &w.declaration,
        &state,
        &other,
        state.revision().clone(),
        &good,
        &w.admissions,
    )
    .expect_err("unauthorized writer");
    assert!(
        matches!(&error, ConsumeRefusal::Episode(e) if e.kind == ErrorKind::Authority),
        "{error:?}"
    );

    // A result for another subject: the shape parses, the episode owner refuses it.
    let wrong_subject = result_json(|v| v["subject"]["tree"] = json!("0".repeat(40)));
    assert!(
        ReviewResult::parse(parse_json(&wrong_subject).unwrap()).is_ok(),
        "parse is shape only"
    );
    let error = try_consume(
        &w,
        &w.declaration,
        &state,
        &authority(),
        state.revision().clone(),
        &wrong_subject,
        &w.admissions,
    )
    .expect_err("wrong subject");
    assert!(
        matches!(&error, ConsumeRefusal::Episode(e) if e.kind == ErrorKind::ResultContract),
        "{error:?}"
    );

    // A malformed result is the episode owner's ResultContract error too.
    let malformed = result_json(|v| v["verdict"] = json!("maybe"));
    let error = try_consume(
        &w,
        &w.declaration,
        &state,
        &authority(),
        state.revision().clone(),
        &malformed,
        &w.admissions,
    )
    .expect_err("malformed result");
    assert!(
        matches!(&error, ConsumeRefusal::Episode(e) if e.kind == ErrorKind::ResultContract),
        "{error:?}"
    );

    assert_eq!(
        canonical(&state),
        before,
        "no refusal changed the input state"
    );

    // After a successful consumption the episode is no longer awaiting a result: the old
    // revision is stale and the phase no longer accepts one.
    let consumed = consume_with(&w, &w.admissions).unwrap();
    let after = consumed.state().clone();
    let again = consume_fixture_result(
        &w.candidate,
        &w.declaration,
        &after,
        &authority(),
        state.revision().clone(),
        "result-2",
        &good,
        &[],
        &text(&w.admissions),
    )
    .expect_err("second result");
    assert!(
        matches!(&again, ConsumeRefusal::Episode(e) if e.kind == ErrorKind::RevisionConflict),
        "{again:?}"
    );
    let phase_check = consume_fixture_result(
        &w.candidate,
        &w.declaration,
        &after,
        &authority(),
        after.revision().clone(),
        "result-3",
        &good,
        &[],
        &text(&w.admissions),
    )
    .expect_err("reported episode");
    assert!(
        matches!(&phase_check, ConsumeRefusal::Episode(_)),
        "{phase_check:?}"
    );
    assert_eq!(canonical(&after), canonical_json(consumed.state().value()));
}

#[test]
fn an_unsupported_result_contract_is_refused_before_the_gate_or_the_transition() {
    let bytes = edited(|v| {
        v["obligations"][0]["resultContract"] =
            json!({"id": "implementation-review", "version": 2});
    });
    let mut w = world();
    w.candidate = candidate_from(&bytes);
    w.declaration.candidate_content_digest = w.candidate.content_digest().to_hex();
    let error = refused_with(&w, &w.admissions);
    assert!(
        matches!(
            error,
            ConsumeRefusal::Selection(PreparationRefusal::UnsupportedResultContract { .. })
        ),
        "{error:?}"
    );
}

#[test]
fn a_declaration_cannot_change_the_production_preparation() {
    let w = world();
    let before = serde_json::to_string(&prepare(&w.candidate, &request(None)).unwrap()).unwrap();
    // Exercise the whole fixture path, including a successful consumption.
    fixture_coverage(&w.candidate, &w.declaration).unwrap();
    consume_with(&w, &w.admissions).unwrap();
    let after = prepare(&w.candidate, &request(None)).unwrap();
    assert_eq!(serde_json::to_string(&after).unwrap(), before);
    for id in [
        "problem-scope-to-episode-identity".to_owned(),
        format!("consumption-evidence-mapping:{COND_BUILDER}"),
        format!("separation:{COND_CAMPAIGN}"),
        format!("observer-identity:{COND_BUILDER}"),
    ] {
        assert!(
            after.blockers.iter().any(|b| b.id == id),
            "{id} must remain a blocker"
        );
    }
}
