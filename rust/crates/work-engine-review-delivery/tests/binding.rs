//! RA002 regressions (feature `fixture`, no native child): the canonical binding refuses a
//! mismatched join before any child or prompt exists. Each counterexample keeps the identity
//! strings the consumer could compare and changes real semantics, so only full canonical equality
//! (not repeated digest strings) can refuse it. Revision/writer/lineage stay with the consumer
//! and are exercised in native_vertical.rs.
mod common;

use common::world::{World, world};
use common::*;
use review_episode_core::codec::{digest, parse_json};
use review_episode_core::command::BeginCommand;
use review_episode_core::identity::Authority;
use review_episode_core::reducer::begin;
use review_episode_core::state::ReviewEpisodeState;
use serde_json::{Value, json};
use work_engine_review_delivery::Transport;
use work_engine_review_delivery::binding::{BindingRefusal, EpisodeInputs, ExperimentBinding};
use work_engine_review_preparation::fixture::{ConsumeRefusal, FixtureDeclaration};
use work_engine_review_preparation::{
    PreparationRefusal, PreparationRequest, ReviewPreparation, Subject, prepare,
};

fn transport() -> Transport {
    Transport {
        attempt_id: "attempt-1".into(),
        generation: 1,
    }
}

fn episode(state: &ReviewEpisodeState, w: &World) -> EpisodeInputs {
    EpisodeInputs {
        state: state.clone(),
        authority: authority(),
        expected_revision: state.revision().clone(),
        transition_id: "result-1".into(),
        unresolved_questions: vec![],
        admissions_json: serde_json::to_string(&w.admissions).unwrap(),
    }
}

fn subject_a() -> Subject {
    Subject {
        commit: "1".repeat(40),
        tree: "2".repeat(40),
        patch_identity: "3".repeat(64),
    }
}

/// The frozen authority with its initial subject digest replaced by `subject`'s canonical digest.
fn state_for_subject(subject: &Subject) -> ReviewEpisodeState {
    let mut value: Value = serde_json::from_str(AUTHORITY).unwrap();
    value["initialSubject"]["sha256"] = json!(digest(&subject.to_js_value()));
    let authority = Authority::parse(parse_json(&value.to_string()).unwrap()).unwrap();
    begin(
        None,
        &BeginCommand::new(authority, "begin-1", vec![]).unwrap(),
    )
    .unwrap()
    .into_state()
}

fn declaration_with(w: &World, edit: impl FnOnce(&mut Value)) -> FixtureDeclaration {
    let mut value = serde_json::to_value(&w.declaration).unwrap();
    edit(&mut value);
    serde_json::from_value(value).unwrap()
}

fn bind(
    w: &World,
    request: &PreparationRequest,
    declaration: &FixtureDeclaration,
    state: &ReviewEpisodeState,
) -> Result<ExperimentBinding, BindingRefusal> {
    ExperimentBinding::new(
        &w.candidate,
        request,
        declaration,
        episode(state, w),
        &result_json(|_| {}),
        transport(),
    )
}

fn supplied(
    w: &World,
    request: &PreparationRequest,
    state: &ReviewEpisodeState,
    prep: &ReviewPreparation,
) -> Result<ExperimentBinding, BindingRefusal> {
    ExperimentBinding::from_supplied_preparation(
        &w.candidate,
        request,
        &w.declaration,
        episode(state, w),
        &result_json(|_| {}),
        transport(),
        prep,
    )
}

#[test]
fn the_canonical_world_binds_and_delivers_the_recomputed_preparation() {
    let w = world();
    let state = begun();
    let binding = bind(&w, &request(None), &w.declaration, &state).expect("canonical join");
    let canonical = prepare(&w.candidate, &request(None)).unwrap();
    assert_eq!(binding.preparation(), &canonical);
    let envelope: Value = serde_json::from_str(binding.frozen().contract()).unwrap();
    assert_eq!(
        envelope["preparation"],
        serde_json::to_value(&canonical).unwrap()
    );
    // A supplied preparation equal in full is accepted; the recomputation is what is delivered.
    let again =
        supplied(&w, &request(None), &state, &canonical).expect("equal supplied preparation");
    assert_eq!(
        again.frozen().contract_sha256(),
        binding.frozen().contract_sha256()
    );
}

#[test]
fn preparation_for_subject_a_is_refused_when_the_consumer_world_is_b() {
    let w = world();
    let state_b = begun();
    let for_a = prepare(
        &w.candidate,
        &PreparationRequest {
            subject: subject_a(),
            ..request(None)
        },
    )
    .unwrap();
    // Valid on its own: A's preparation is canonical for A.
    assert_ne!(for_a, prepare(&w.candidate, &request(None)).unwrap());
    // Delivered as B's preparation with B's request, declaration and episode: refused in full.
    let e = supplied(&w, &request(None), &state_b, &for_a).unwrap_err();
    assert!(
        matches!(
            e,
            BindingRefusal::PreparationMismatch {
                part: "model_context.obligation"
            }
        ),
        "{e:?}"
    );
}

#[test]
fn a_request_for_subject_a_is_refused_against_a_declaration_or_episode_for_b() {
    let w = world();
    let req_a = PreparationRequest {
        subject: subject_a(),
        ..request(None)
    };
    // Declaration B (the fixture's) against request A.
    let e = bind(&w, &req_a, &w.declaration, &begun()).unwrap_err();
    assert!(
        matches!(
            e,
            BindingRefusal::Declaration {
                field: "subject",
                ..
            }
        ),
        "{e:?}"
    );
    // Declaration A, episode B: the episode's exact subject differs.
    let decl_a = declaration_with(&w, |v| {
        v["subject"] = json!({"commit": subject_a().commit, "tree": subject_a().tree, "patchIdentity": subject_a().patch_identity});
    });
    let e = bind(&w, &req_a, &decl_a, &begun()).unwrap_err();
    assert!(
        matches!(
            e,
            BindingRefusal::Episode {
                field: "subject.sha256",
                ..
            }
        ),
        "{e:?}"
    );
    // All three agree on A: the join holds (the episode owner still owns every later check).
    assert!(bind(&w, &req_a, &decl_a, &state_for_subject(&subject_a())).is_ok());
}

#[test]
fn a_different_candidate_or_obligation_is_refused_before_any_child() {
    let w = world();
    let state = begun();
    // Candidate B: same obligation id, same declared scope strings, different problem scope content.
    let other = candidate_from(&edited(|v| v["problemScope"]["sliceNumber"] = json!(2)));
    assert_ne!(
        other.content_digest().to_hex(),
        w.candidate.content_digest().to_hex()
    );
    let other_prep = prepare(&other, &request(None)).unwrap();
    let e = supplied(&w, &request(None), &state, &other_prep).unwrap_err();
    assert!(
        matches!(
            e,
            BindingRefusal::PreparationMismatch {
                part: "model_context.identity"
            }
        ),
        "{e:?}"
    );
    // Candidate B consumed with candidate A's declaration: the declaration's digest names A.
    let w_b = World {
        candidate: other,
        declaration: w.declaration.clone(),
        admissions: w.admissions.clone(),
    };
    let e = bind(&w_b, &request(None), &w.declaration, &state).unwrap_err();
    assert!(
        matches!(
            e,
            BindingRefusal::Declaration {
                field: "candidate_content_digest",
                ..
            }
        ),
        "{e:?}"
    );
    // A declaration for another obligation, and a request for an unknown obligation.
    let decl = declaration_with(&w, |v| v["obligation_id"] = json!("some-other-obligation"));
    let e = bind(&w, &request(None), &decl, &state).unwrap_err();
    assert!(
        matches!(
            e,
            BindingRefusal::Declaration {
                field: "obligation_id",
                ..
            }
        ),
        "{e:?}"
    );
    let unknown = PreparationRequest {
        obligation_id: "some-other-obligation".into(),
        ..request(None)
    };
    let e = bind(&w, &unknown, &w.declaration, &state).unwrap_err();
    assert!(
        matches!(
            e,
            BindingRefusal::Preparation(PreparationRefusal::UnknownObligation { .. })
        ),
        "{e:?}"
    );
}

#[test]
fn altered_conditions_blockers_or_provenance_with_unchanged_identity_are_refused() {
    let w = world();
    let state = begun();
    let canonical = prepare(&w.candidate, &request(None)).unwrap();
    let identity_before = (
        canonical.model_context.identity.clone(),
        canonical.model_context.obligation.id.clone(),
        canonical.model_context.obligation.subject.clone(),
    );
    let mut cases: Vec<(&str, ReviewPreparation)> = Vec::new();

    let mut conditions = canonical.clone();
    conditions.model_context.conditions[0]
        .proposition
        .push_str(" (altered)");
    cases.push(("model_context.conditions", conditions));

    let mut constraints = canonical.clone();
    constraints.model_context.conditions[0].constraints.clear();
    cases.push(("model_context.conditions", constraints));

    let mut blockers = canonical.clone();
    let dropped = blockers
        .blockers
        .pop()
        .expect("preparation carries blockers");
    assert!(!dropped.id.is_empty());
    cases.push(("blockers", blockers));

    let mut blocker_text = canonical.clone();
    blocker_text.blockers[0].basis.push_str(" (altered)");
    cases.push(("blockers", blocker_text));

    let mut provenance = canonical.clone();
    provenance
        .provenance
        .pop()
        .expect("preparation carries provenance");
    cases.push(("provenance", provenance));

    let mut enforcement = canonical.clone();
    enforcement
        .host_enforcement
        .rows
        .pop()
        .expect("enforcement rows");
    cases.push(("host_enforcement", enforcement));

    for (expected_part, altered) in cases {
        assert_eq!(
            (
                altered.model_context.identity.clone(),
                altered.model_context.obligation.id.clone(),
                altered.model_context.obligation.subject.clone()
            ),
            identity_before,
            "identity strings are unchanged in the altered preparation"
        );
        assert_ne!(altered, canonical);
        let e = supplied(&w, &request(None), &state, &altered).unwrap_err();
        match e {
            BindingRefusal::PreparationMismatch { part } => {
                assert_eq!(part, expected_part, "{expected_part}")
            }
            other => panic!("{expected_part}: expected a mismatch, got {other:?}"),
        }
    }
}

#[test]
fn the_declaration_owner_still_refuses_unmapped_conditions_before_any_child() {
    let w = world();
    let mut value = serde_json::to_value(&w.declaration).unwrap();
    value["bindings"].as_array_mut().unwrap().pop();
    let decl: FixtureDeclaration = serde_json::from_value(value).unwrap();
    let e = bind(&w, &request(None), &decl, &begun()).unwrap_err();
    assert!(
        matches!(
            e,
            BindingRefusal::Coverage(ConsumeRefusal::RequiredConditionUnmapped { .. })
        ),
        "{e:?}"
    );
}
