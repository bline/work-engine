//! Admission core tests in the fixture domain. Each test names its oracle class (Amendment 6.1
//! §6): G1 isolated gate input, G2 coupled checks (exact failure set), S observation, P production
//! unconditional refusal. State tests name the state they establish.
#![cfg(feature = "fixture")]

use serde_json::{Value, json};
use work_engine_organization::{
    Continuity, EvidenceCustody, ExecutionConfinement, Integrity, Mutation, ObserverClass,
    OrganizationalRequirements, Profile, ProviderFamily, RealizationClass, derive,
};
use work_engine_organization_authority::fixture::FixtureSource;
use work_engine_organization_authority::records::{
    ActorAnchor, AnchorKind, PrincipalKind, ProblemScopeRef, RealizationProperties, Record,
};
use work_engine_organization_authority::{
    AdmissionRefusal, AdmissionState, Boundary, BoundaryKind, CheckId, CheckOutcome,
    DeclaredRealization, EffectAdmission, ExecutableDigest, Fixture, IssuanceId, OperationRequest,
    Outcomes, Production, RealizationDigest, Reason, ReasonCode, StaleReason, SubjectRef,
    UnknownReason, admit, observe_for, reserve,
};

const ORG: &str = "org-1";
const VANTAGE: &str = "vantage:ob-a";
const NOW: u64 = 1_000;

// --- requirements ----------------------------------------------------------------------------

#[derive(Clone)]
struct Req {
    /// `scope.digest` of `ob-a`; the base subject's exact-subject digest by default (S1 passes).
    scope_digest: String,
    result_version: u64,
    continuity: &'static str,
    state_acceptor_dependency: bool,
    /// `scope.boundaries` of the authority requirement; `None` is the whole problem scope.
    authority_boundaries: Option<&'static [&'static str]>,
}

impl Default for Req {
    fn default() -> Self {
        Req {
            scope_digest: subject().scope_digest(),
            result_version: 1,
            continuity: "fresh_initial",
            state_acceptor_dependency: true,
            authority_boundaries: None,
        }
    }
}

/// Both consumption boundaries of the base candidate's conditions.
const BOTH_BOUNDARIES: &[&str] = &["builder_projection", "campaign_terminalization"];

fn normative(continuity: &str) -> Value {
    json!({"continuity":continuity,"mutation":"none","capabilities":[],"integrity":"verified_artifacts","evidenceCustody":"execution_evidence_custody","observerClass":"independent_execution_evidence"})
}

fn requirements(r: &Req) -> OrganizationalRequirements {
    parse(&requirements_doc(r))
}

fn parse(doc: &Value) -> OrganizationalRequirements {
    OrganizationalRequirements::from_json_bytes(&serde_json::to_vec(doc).unwrap()).unwrap()
}

/// The exact subject every base request names.
fn subject() -> SubjectRef {
    SubjectRef {
        commit: "c0ffee".into(),
        tree: "7ree".into(),
        patch_identity: "patch-1".into(),
    }
}

fn requirements_doc(r: &Req) -> Value {
    let cond = |id: &str, boundary: &str, class: &str, continuity: &str, constraints: Value| {
        let mut c = json!({"id":id,"obligation":"ob-a","consumptionBoundary":boundary,"consumerClass":class,
               "proposition":"p","coveredState":"review-result","normative":normative(continuity),
               "acceptor":"owner-a",
               "unestablishedRoute":"hold-consumption-for-owner-disposition"});
        if constraints != json!([]) {
            c["realizationConstraints"] = constraints;
        }
        c
    };
    let authorities = if r.state_acceptor_dependency {
        let scope = match r.authority_boundaries {
            Some(boundaries) => json!({ "boundaries": boundaries }),
            None => json!({}),
        };
        json!([{"id":"auth-1","principal":"owner-a","permission":"accept_claim","scope":scope}])
    } else {
        json!([])
    };
    json!({
        "problemScope": {"runId": "run-1", "planLineage": "plan-a", "sliceNumber": 1},
        "obligations": [{"id":"ob-a","kind":"candidate_review","subject":"campaign_candidate",
                         "resultContract":{"id":"implementation-review","version":r.result_version},
                         "scope":{"digest":r.scope_digest}}],
        "acceptanceConditions": [
            cond("cond-builder","builder_projection","builder", r.continuity,
                 json!([{"dimension":"provider_family","allowed":["anthropic"]}])),
            cond("cond-campaign","campaign_terminalization","campaign", r.continuity, json!([])),
        ],
        "requiredAuthorities": authorities,
        "organizationPolicy": {"mode": "fixed", "profile": "fixed-slice-review-v1"}
    })
}

fn request_for(r: &Req) -> OperationRequest {
    let requirements = requirements(r);
    let digest = derive(&requirements, Profile::FixedSliceReviewV1).candidates[0].content_digest();
    request_with(requirements, digest)
}

fn request_with(
    requirements: OrganizationalRequirements,
    digest: work_engine_organization::Digest,
) -> OperationRequest {
    OperationRequest::new(ORG, requirements, digest, VANTAGE, subject())
}

// --- the fixture world -----------------------------------------------------------------------

/// A decision scope held by the owner's grant alone (`ds-owner`), so that varying it does not
/// touch the component grants, which use `ds-1`.
struct OwnerDs {
    scope: ProblemScopeRef,
    actions: &'static [&'static str],
    boundaries: Option<&'static [&'static str]>,
}

struct Opts {
    owner_grant_ds: (&'static str, u64),
    owner_ds: Option<OwnerDs>,
    owner_grant_permission: &'static str,
    owner_grant_scope: Option<ProblemScopeRef>,
    owner_grant_expiry: Option<u64>,
    reviewer_anchor: (AnchorKind, &'static str),
    builder_binding: bool,
    campaign_binding: bool,
    /// The realization digest the reviewer's vantage binding states.
    binding_digest: &'static str,
    accept_realization: bool,
    acceptance_profile: &'static str,
    provides_provider: bool,
    provides_continuity: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            owner_grant_ds: ("ds-1", 1),
            owner_ds: None,
            owner_grant_permission: "accept_claim",
            owner_grant_scope: None,
            owner_grant_expiry: None,
            reviewer_anchor: (AnchorKind::ServiceAccount, "svc-reviewer"),
            builder_binding: true,
            campaign_binding: true,
            binding_digest: "real-1",
            accept_realization: true,
            acceptance_profile: "fixed-slice-review-v1",
            provides_provider: true,
            provides_continuity: true,
        }
    }
}

/// The owner decision scope that covers the base requirement; each variant changes one field.
const OWNER_ACTIONS: &[&str] = &["accept_claim"];

fn owner_ds() -> OwnerDs {
    OwnerDs {
        scope: scope(),
        actions: OWNER_ACTIONS,
        boundaries: None,
    }
}

#[allow(dead_code)]
struct World {
    src: FixtureSource,
    owner: IssuanceId,
    reviewer: IssuanceId,
    builder: IssuanceId,
    campaign: IssuanceId,
    episode: IssuanceId,
    ds: IssuanceId,
    owner_grant: IssuanceId,
    reviewer_binding: IssuanceId,
}

fn scope() -> ProblemScopeRef {
    ProblemScopeRef::fixture("run-1", "plan-a", 1)
}

fn principal(
    src: &FixtureSource,
    id: &str,
    kind: PrincipalKind,
    anchor: (AnchorKind, &str),
) -> IssuanceId {
    src.issue(Record::fixture_principal(
        id,
        kind,
        ActorAnchor::fixture(anchor.0, anchor.1),
    ))
}

fn grant(
    src: &FixtureSource,
    id: &str,
    p: IssuanceId,
    perms: &[&str],
    ds: (&str, u64),
    expiry: Option<u64>,
) -> IssuanceId {
    src.issue(Record::fixture_grant(id, p, perms, scope(), ds, expiry))
}

fn world(o: Opts) -> World {
    let src = FixtureSource::new();
    let owner = principal(
        &src,
        "owner-a",
        PrincipalKind::Human,
        (AnchorKind::OwnerAccount, "alice"),
    );
    let reviewer = principal(
        &src,
        "reviewer-1",
        PrincipalKind::Service,
        o.reviewer_anchor,
    );
    let builder = principal(
        &src,
        "builder-1",
        PrincipalKind::Service,
        (AnchorKind::ServiceAccount, "svc-builder"),
    );
    let campaign = principal(
        &src,
        "campaign-svc",
        PrincipalKind::Service,
        (AnchorKind::ServiceAccount, "svc-campaign"),
    );
    let episode = principal(
        &src,
        "episode-svc",
        PrincipalKind::Service,
        (AnchorKind::ServiceAccount, "svc-episode"),
    );
    let ds = src.issue(Record::fixture_decision_scope(
        "ds-1",
        1,
        scope(),
        &[
            "accept_claim",
            "terminalize_campaign",
            "read_episode",
            "write_episode",
        ],
        None,
    ));
    let owner_ds = match &o.owner_ds {
        Some(d) => {
            src.issue(Record::fixture_decision_scope(
                "ds-owner",
                1,
                d.scope.clone(),
                d.actions,
                d.boundaries,
            ));
            ("ds-owner", 1)
        }
        None => o.owner_grant_ds,
    };
    let owner_grant = src.issue(Record::fixture_grant(
        "g-owner",
        owner,
        &[o.owner_grant_permission],
        o.owner_grant_scope.clone().unwrap_or_else(scope),
        owner_ds,
        o.owner_grant_expiry,
    ));
    grant(
        &src,
        "g-campaign",
        campaign,
        &["terminalize_campaign"],
        ("ds-1", 1),
        None,
    );
    grant(
        &src,
        "g-episode",
        episode,
        &["read_episode", "write_episode"],
        ("ds-1", 1),
        None,
    );
    if o.campaign_binding {
        src.issue(Record::fixture_component_binding(
            "component:campaign",
            campaign,
            ExecutableDigest::new("exe-campaign"),
            scope(),
        ));
    }
    src.issue(Record::fixture_component_binding(
        "component:episode_host",
        episode,
        ExecutableDigest::new("exe-episode"),
        scope(),
    ));
    let reviewer_binding = src.issue(Record::fixture_vantage_binding(
        ORG,
        VANTAGE,
        reviewer,
        RealizationDigest::new(o.binding_digest),
        "uid:svc-reviewer",
    ));
    if o.builder_binding {
        src.issue(Record::fixture_vantage_binding(
            ORG,
            "vantage-builder",
            builder,
            RealizationDigest::new("real-builder"),
            "uid:svc-builder",
        ));
    }
    if o.accept_realization {
        let mut classes = vec![RealizationClass::ExecutionConfinement(
            ExecutionConfinement::GuardedSandbox,
        )];
        if o.provides_provider {
            classes.push(RealizationClass::ProviderFamily(ProviderFamily::Anthropic));
        }
        src.issue(Record::fixture_realization_acceptance(
            RealizationDigest::new("real-1"),
            o.acceptance_profile,
            RealizationProperties::fixture(
                &classes,
                o.provides_continuity.then_some(Continuity::FreshInitial),
                Some(Mutation::None),
                Some(Integrity::VerifiedArtifacts),
                Some(EvidenceCustody::ExecutionEvidenceCustody),
                Some(ObserverClass::IndependentExecutionEvidence),
            ),
        ));
    }
    World {
        src,
        owner,
        reviewer,
        builder,
        campaign,
        episode,
        ds,
        owner_grant,
        reviewer_binding,
    }
}

fn realization() -> DeclaredRealization {
    realization_with("uid:svc-reviewer", "exe-campaign")
}

fn realization_with(actor: &str, campaign_exe: &str) -> DeclaredRealization {
    DeclaredRealization::declared(
        RealizationDigest::new("real-1"),
        actor,
        [
            (
                "component:campaign".to_owned(),
                ExecutableDigest::new(campaign_exe),
            ),
            (
                "component:episode_host".to_owned(),
                ExecutableDigest::new("exe-episode"),
            ),
        ],
    )
}

fn boundary(epoch: u64) -> Boundary {
    Boundary::new("op-1", BoundaryKind::Delivery, epoch)
}

fn decide_fixture(
    w: &World,
    req: &OperationRequest,
    real: &DeclaredRealization,
    now: u64,
) -> Result<EffectAdmission, AdmissionRefusal> {
    let reservation = reserve(boundary(1));
    let obs = observe_for(&w.src, &reservation).expect("observation");
    admit::<Fixture>(&reservation, obs, req, real, now)
}

fn refused(result: Result<EffectAdmission, AdmissionRefusal>) -> Outcomes {
    match result {
        Ok(_) => panic!("expected refusal"),
        Err(AdmissionRefusal::Checks(outcomes)) => outcomes,
        Err(other) => panic!("expected check refusal, got {other:?}"),
    }
}

fn r(code: ReasonCode, subject: &str) -> Reason {
    Reason {
        code,
        subject: subject.to_owned(),
    }
}

/// Exact expected failure set: every listed check refuses with exactly these reasons, every other
/// check passes. Any extra or missing check fails.
fn assert_exact(outcomes: &Outcomes, expected: &[(CheckId, Vec<Reason>)]) {
    for id in CheckId::ALL {
        let want = expected.iter().find(|(e, _)| *e == id);
        match (want, outcomes.get(id)) {
            (None, CheckOutcome::Passed) => {}
            (Some((_, reasons)), CheckOutcome::Refused(got)) => {
                let mut reasons = reasons.clone();
                reasons.sort();
                assert_eq!(got, &reasons, "reasons for {id:?}");
            }
            (want, got) => panic!("{id:?}: expected {want:?}, got {got:?}"),
        }
    }
}

// --- positive control ------------------------------------------------------------------------

#[test]
fn fixture_admission_binds_principal_actor_subject_boundary_revision_and_realization() {
    let w = world(Opts::default());
    let req = request_for(&Req::default());
    let reservation = reserve(boundary(1));
    let obs = observe_for(&w.src, &reservation).unwrap();
    let observed_revision = obs.revision();
    let admission =
        admit::<Fixture>(&reservation, obs, &req, &realization(), NOW).expect("admitted");
    assert!(admission.outcomes().all_passed());
    assert_eq!(admission.principal(), w.reviewer);
    assert_eq!(
        admission.actor_anchor(),
        &ActorAnchor::fixture(AnchorKind::ServiceAccount, "svc-reviewer")
    );
    assert_eq!(admission.actor_identity(), "uid:svc-reviewer");
    assert_eq!(
        admission.realization_digest(),
        &RealizationDigest::new("real-1")
    );
    assert_eq!(admission.realization_profile(), "fixed-slice-review-v1");
    assert_eq!(admission.boundary(), &boundary(1));
    assert_eq!(admission.revision(), observed_revision);
    assert_eq!(admission.request().subject().commit, "c0ffee");
    assert_eq!(admission.request().vantage(), VANTAGE);
    let pins = admission.pins();
    for id in [w.owner, w.reviewer, w.builder, w.campaign, w.episode] {
        assert!(pins.principals.contains(&id));
    }
    assert!(pins.grants.contains(&w.owner_grant));
    assert!(pins.decision_scopes.contains(&w.ds));
    assert!(pins.vantage_bindings.contains(&w.reviewer_binding));
    assert_eq!(pins.component_bindings.len(), 2);
    assert_eq!(pins.realization_acceptance.len(), 1);
}

// --- P: production unconditional refusal ------------------------------------------------------

#[test]
fn p_production_refusal_set_is_exact_and_fixture_admits_the_same_candidate() {
    let w = world(Opts::default());
    let req = request_for(&Req::default());
    let reservation = reserve(boundary(1));
    let obs = observe_for(&w.src, &reservation).unwrap();
    let outcomes = refused(admit::<Production>(
        &reservation,
        obs,
        &req,
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[
            (
                CheckId::O4,
                vec![
                    r(
                        ReasonCode::BindingContractUnqualified,
                        "component:component:campaign",
                    ),
                    r(
                        ReasonCode::BindingContractUnqualified,
                        "component:component:episode_host",
                    ),
                ],
            ),
            (
                CheckId::O5,
                vec![
                    r(ReasonCode::MechanismNone, "cond-builder/separation"),
                    r(ReasonCode::MechanismNone, "cond-campaign/separation"),
                ],
            ),
            (
                CheckId::O8,
                vec![
                    r(
                        ReasonCode::BindingContractUnqualified,
                        "separation:cond-builder",
                    ),
                    r(
                        ReasonCode::BindingContractUnqualified,
                        "separation:cond-campaign",
                    ),
                ],
            ),
            (
                CheckId::R3,
                vec![r(
                    ReasonCode::BindingContractUnqualified,
                    "actor:vantage:ob-a",
                )],
            ),
        ],
    );
    // The first observation was consumed by the production decision; the fixture decision over
    // the same candidate takes its own.
    let again = observe_for(&w.src, &reservation).unwrap();
    assert!(admit::<Fixture>(&reservation, again, &req, &realization(), NOW).is_ok());
}

// --- reservation seal (6.1 B12b) ---------------------------------------------------------------

#[test]
fn seal_same_observation_cannot_decide_a_later_boundary() {
    let w = world(Opts::default());
    let req = request_for(&Req::default());
    let first = reserve(boundary(1));
    let obs = observe_for(&w.src, &first).unwrap();
    let later = reserve(boundary(2));
    match admit::<Fixture>(&later, obs, &req, &realization(), NOW) {
        Err(AdmissionRefusal::ObservationBoundaryMismatch { sealed, decided }) => {
            assert_eq!(sealed, boundary(1));
            assert_eq!(decided, boundary(2));
        }
        other => panic!("expected boundary mismatch, got {other:?}"),
    }
    // Control: an observation sealed to the boundary being decided is decided.
    let sealed = observe_for(&w.src, &first).unwrap();
    assert!(admit::<Fixture>(&first, sealed, &req, &realization(), NOW).is_ok());
}

#[test]
fn seal_a_later_reserved_boundary_makes_the_admission_stale() {
    let w = world(Opts::default());
    let admission = decide_fixture(&w, &request_for(&Req::default()), &realization(), NOW).unwrap();
    let later = reserve(boundary(2));
    assert_eq!(
        admission.admission_state(&observe_for(&w.src, &later), NOW),
        AdmissionState::Stale(StaleReason::BoundarySuperseded { by: boundary(2) })
    );
    let other = reserve(Boundary::new("op-1", BoundaryKind::Consumption, 1));
    assert!(matches!(
        admission.admission_state(&observe_for(&w.src, &other), NOW),
        AdmissionState::Stale(StaleReason::ObservationBoundaryMismatch { .. })
    ));
}

// --- S: observation ---------------------------------------------------------------------------

#[test]
fn s_unknown_observation_yields_no_observation_and_so_no_outcomes() {
    let w = world(Opts::default());
    let reservation = reserve(boundary(1));
    assert!(observe_for(&w.src, &reservation).is_ok());
    w.src.inject_unknown("write in progress");
    let err = observe_for(&w.src, &reservation).unwrap_err();
    assert_eq!(err.detail, "write in progress");
}

// --- Gate O ------------------------------------------------------------------------------------

#[test]
fn g1_o3_candidate_digest_of_another_candidate_refuses_only_o3() {
    let w = world(Opts::default());
    let other = derive(
        &requirements(&Req {
            scope_digest: "b".repeat(64),
            ..Req::default()
        }),
        Profile::FixedSliceReviewV1,
    )
    .candidates[0]
        .content_digest();
    let req = request_with(requirements(&Req::default()), other);
    let outcomes = refused(decide_fixture(&w, &req, &realization(), NOW));
    let recomputed = request_for(&Req::default()).candidate_digest().to_hex();
    assert_exact(
        &outcomes,
        &[(
            CheckId::O3,
            vec![r(ReasonCode::CandidateDigestMismatch, &recomputed)],
        )],
    );
}

#[test]
fn g2_o3_no_derived_candidate_leaves_candidate_checks_unavailable() {
    let w = world(Opts::default());
    let base = request_for(&Req::default());
    let req = request_with(
        requirements(&Req {
            state_acceptor_dependency: false,
            ..Req::default()
        }),
        base.candidate_digest(),
    );
    let outcomes = refused(decide_fixture(&w, &req, &realization(), NOW));
    let unavailable = vec![r(ReasonCode::CandidateUnavailable, "")];
    assert_exact(
        &outcomes,
        &[
            (
                CheckId::O3,
                vec![r(ReasonCode::CandidateUnderived, "1 unresolved")],
            ),
            (CheckId::O4, unavailable.clone()),
            (CheckId::O5, unavailable.clone()),
            (CheckId::O8, unavailable.clone()),
            (CheckId::O9, unavailable.clone()),
            (CheckId::R1, unavailable.clone()),
            (CheckId::R2, unavailable.clone()),
            (CheckId::S1, unavailable),
        ],
    );
}

#[test]
fn g1_o4_expired_grant_refuses_only_o4() {
    let w = world(Opts {
        owner_grant_expiry: Some(NOW),
        ..Opts::default()
    });
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[(
            CheckId::O4,
            vec![r(ReasonCode::AuthorityExpired, "authority:auth-1")],
        )],
    );
    // Control: one millisecond earlier the same grant covers.
    assert!(decide_fixture(&w, &request_for(&Req::default()), &realization(), NOW - 1).is_ok());
}

#[test]
fn g1_o4_unresolved_decision_scope_covers_nothing() {
    let w = world(Opts {
        owner_grant_ds: ("ds-1", 2),
        ..Opts::default()
    });
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[(
            CheckId::O4,
            vec![r(ReasonCode::AuthorityScopeUnresolved, "authority:auth-1")],
        )],
    );
}

#[test]
fn g1_o4_a_principal_named_like_a_component_binds_nothing() {
    let w = world(Opts {
        campaign_binding: false,
        ..Opts::default()
    });
    let lookalike = principal(
        &w.src,
        "component:campaign",
        PrincipalKind::Service,
        (AnchorKind::ServiceAccount, "svc-x"),
    );
    grant(
        &w.src,
        "g-lookalike",
        lookalike,
        &["terminalize_campaign"],
        ("ds-1", 1),
        None,
    );
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[(
            CheckId::O4,
            vec![r(
                ReasonCode::ComponentUnbound,
                "component:component:campaign",
            )],
        )],
    );
}

#[test]
fn g1_o4_revoke_disables_exactly_its_issuance_not_a_reissue_with_identical_terms() {
    let w = world(Opts::default());
    let reissue = grant(
        &w.src,
        "g-owner",
        w.owner,
        &["accept_claim"],
        ("ds-1", 1),
        None,
    );
    assert_ne!(reissue, w.owner_grant);
    let reservation = reserve(boundary(1));
    let obs = observe_for(&w.src, &reservation).unwrap();
    assert_eq!(
        obs.state().body_digest(&reissue),
        obs.state().body_digest(&w.owner_grant)
    );
    w.src.revoke(&w.owner_grant);
    let admission = decide_fixture(&w, &request_for(&Req::default()), &realization(), NOW)
        .expect("reissue covers");
    assert!(admission.pins().grants.contains(&reissue));
    assert!(!admission.pins().grants.contains(&w.owner_grant));
}

#[test]
fn g1_o5_a_none_row_other_than_separation_refuses_only_o5() {
    let w = world(Opts::default());
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req {
            result_version: 2,
            ..Req::default()
        }),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[(
            CheckId::O5,
            vec![r(ReasonCode::MechanismNone, "ob-a/resultContract")],
        )],
    );
}

#[test]
fn g2_o5_retained_continuity_has_no_mechanism_and_the_realization_does_not_provide_it() {
    let w = world(Opts::default());
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req {
            continuity: "retained",
            ..Req::default()
        }),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[
            (
                CheckId::O5,
                vec![
                    r(ReasonCode::MechanismNone, "cond-builder/continuity"),
                    r(ReasonCode::MechanismNone, "cond-campaign/continuity"),
                ],
            ),
            (
                CheckId::R2,
                vec![r(
                    ReasonCode::RealizationIncompatible,
                    "vantage:ob-a/continuity",
                )],
            ),
        ],
    );
}

#[test]
fn g1_o8_distinct_principals_with_the_same_actor_anchor_are_not_separate() {
    let w = world(Opts {
        reviewer_anchor: (AnchorKind::OwnerAccount, "alice"),
        ..Opts::default()
    });
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[(
            CheckId::O8,
            vec![
                r(ReasonCode::SameActorAnchor, "cond-builder/vantage:ob-a"),
                r(ReasonCode::SameActorAnchor, "cond-campaign/vantage:ob-a"),
            ],
        )],
    );
}

#[test]
fn g1_o8_an_unbound_dependent_vantage_is_unresolved() {
    let w = world(Opts {
        builder_binding: false,
        ..Opts::default()
    });
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[(
            CheckId::O8,
            vec![
                r(
                    ReasonCode::SeparationUnresolved,
                    "cond-builder/vantage-builder",
                ),
                r(
                    ReasonCode::SeparationUnresolved,
                    "cond-campaign/vantage-builder",
                ),
            ],
        )],
    );
}

#[test]
fn g2_revoking_the_acceptor_principal_fails_o4_and_o8_together() {
    let w = world(Opts::default());
    w.src.revoke(&w.owner);
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[
            (
                CheckId::O4,
                vec![r(
                    ReasonCode::PrincipalNotCurrent,
                    "authority:auth-1: revoked",
                )],
            ),
            (
                CheckId::O8,
                vec![
                    r(
                        ReasonCode::SeparationUnresolved,
                        "cond-builder/principal:owner-a",
                    ),
                    r(
                        ReasonCode::SeparationUnresolved,
                        "cond-campaign/principal:owner-a",
                    ),
                ],
            ),
        ],
    );
}

// --- Gate R ------------------------------------------------------------------------------------

#[test]
fn g2_r1_r2_unstated_provider_class_is_unestablished_for_condition_and_vantage() {
    let w = world(Opts {
        provides_provider: false,
        ..Opts::default()
    });
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[
            (
                CheckId::R1,
                vec![r(
                    ReasonCode::RealizationUnestablished,
                    "cond-builder/provider_family",
                )],
            ),
            (
                CheckId::R2,
                vec![r(
                    ReasonCode::RealizationUnestablished,
                    "vantage:ob-a/provider_family",
                )],
            ),
        ],
    );
}

#[test]
fn g1_r2_unstated_effective_property_refuses_only_r2() {
    let w = world(Opts {
        provides_continuity: false,
        ..Opts::default()
    });
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[(
            CheckId::R2,
            vec![r(
                ReasonCode::RealizationUnestablished,
                "vantage:ob-a/continuity",
            )],
        )],
    );
}

#[test]
fn g1_r3_realized_actor_differs_from_bound_actor() {
    let w = world(Opts::default());
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization_with("uid:someone-else", "exe-campaign"),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[(CheckId::R3, vec![r(ReasonCode::ActorMismatch, VANTAGE)])],
    );
}

#[test]
fn g1_r3_installed_component_digest_differs_from_binding() {
    let w = world(Opts::default());
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization_with("uid:svc-reviewer", "exe-other"),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[(
            CheckId::R3,
            vec![r(
                ReasonCode::ComponentExecutableMismatch,
                "component:campaign",
            )],
        )],
    );
}

#[test]
fn g2_r4_unaccepted_realization_leaves_r1_r2_without_accepted_values() {
    let w = world(Opts {
        accept_realization: false,
        ..Opts::default()
    });
    // Another realization is accepted; this one is not.
    w.src.issue(Record::fixture_realization_acceptance(
        RealizationDigest::new("real-2"),
        "fixed-slice-review-v1",
        RealizationProperties::fixture(&[], None, None, None, None, None),
    ));
    let outcomes = refused(decide_fixture(
        &w,
        &request_for(&Req::default()),
        &realization(),
        NOW,
    ));
    let none = vec![r(
        ReasonCode::RealizationUnestablished,
        "no accepted realization",
    )];
    assert_exact(
        &outcomes,
        &[
            (CheckId::R1, none.clone()),
            (CheckId::R2, none),
            (
                CheckId::R4,
                vec![r(ReasonCode::RealizationNotAccepted, "real-1")],
            ),
        ],
    );
}

// --- admission states --------------------------------------------------------------------------

fn admitted(w: &World) -> EffectAdmission {
    decide_fixture(w, &request_for(&Req::default()), &realization(), NOW).expect("admitted")
}

fn state_now(w: &World, admission: &EffectAdmission, now: u64) -> AdmissionState {
    admission.admission_state(&observe_for(&w.src, &reserve(boundary(1))), now)
}

#[test]
fn state_current_at_the_same_revision_and_after_an_unrelated_commit() {
    let w = world(Opts::default());
    let admission = admitted(&w);
    assert_eq!(
        state_now(&w, &admission, NOW),
        AdmissionState::Current {
            revision: admission.revision()
        }
    );
    principal(
        &w.src,
        "unrelated",
        PrincipalKind::Human,
        (AnchorKind::OwnerAccount, "zed"),
    );
    match state_now(&w, &admission, NOW) {
        AdmissionState::Current { revision } => {
            assert!(revision.sequence() > admission.revision().sequence())
        }
        other => panic!("expected current after regate, got {other:?}"),
    }
}

#[test]
fn state_revoked_when_a_pinned_grant_is_revoked() {
    let w = world(Opts::default());
    let admission = admitted(&w);
    w.src.revoke(&w.owner_grant);
    assert_eq!(
        state_now(&w, &admission, NOW),
        AdmissionState::Revoked(vec![w.owner_grant])
    );
    assert!(!state_now(&w, &admission, NOW).permits_effect());
}

#[test]
fn state_revoked_when_the_acting_principal_is_revoked() {
    let w = world(Opts::default());
    let admission = admitted(&w);
    w.src.revoke(&w.reviewer);
    assert_eq!(
        state_now(&w, &admission, NOW),
        AdmissionState::Revoked(vec![w.reviewer])
    );
}

#[test]
fn state_stale_when_a_pinned_issuance_is_superseded() {
    let w = world(Opts::default());
    let admission = admitted(&w);
    w.src.supersede(
        &w.ds,
        Record::fixture_decision_scope("ds-1", 2, scope(), &["accept_claim"], None),
    );
    assert_eq!(
        state_now(&w, &admission, NOW),
        AdmissionState::Stale(StaleReason::IssuanceSuperseded(vec![w.ds]))
    );
}

#[test]
fn state_unknown_without_an_observation() {
    let w = world(Opts::default());
    let admission = admitted(&w);
    w.src.inject_unknown("read error");
    assert_eq!(
        state_now(&w, &admission, NOW),
        AdmissionState::Unknown(UnknownReason::NoObservation("read error".into()))
    );
    w.src.clear_unknown();
    assert!(state_now(&w, &admission, NOW).permits_effect());
}

#[test]
fn state_unknown_when_the_source_breaks_its_ordering_obligations() {
    let w = world(Opts::default());
    let admission = admitted(&w);
    let reservation = reserve(boundary(1));
    // A source at a lower revision: regressed.
    let smaller = FixtureSource::new();
    smaller.issue(Record::fixture_principal(
        "x",
        PrincipalKind::Human,
        ActorAnchor::fixture(AnchorKind::OwnerAccount, "x"),
    ));
    assert_eq!(
        admission.admission_state(&observe_for(&smaller, &reservation), NOW),
        AdmissionState::Unknown(UnknownReason::RevisionRegressed)
    );
    // Another state at the same revision: conflict.
    let same_sequence = world(Opts {
        reviewer_anchor: (AnchorKind::ServiceAccount, "svc-other"),
        ..Opts::default()
    });
    assert_eq!(
        admission.admission_state(&observe_for(&same_sequence.src, &reservation), NOW),
        AdmissionState::Unknown(UnknownReason::RevisionConflict)
    );
    // A later revision that lost pinned issuances: retracted.
    let later = FixtureSource::new();
    for n in 0..=admission.revision().sequence() {
        principal(
            &later,
            &format!("filler-{n}"),
            PrincipalKind::Human,
            (AnchorKind::OwnerAccount, "f"),
        );
    }
    match admission.admission_state(&observe_for(&later, &reservation), NOW) {
        AdmissionState::Unknown(UnknownReason::IssuanceRetracted(ids)) => assert!(!ids.is_empty()),
        other => panic!("expected retracted, got {other:?}"),
    }
}

#[test]
fn state_refused_when_regating_the_same_request_refuses() {
    let w = world(Opts {
        owner_grant_expiry: Some(NOW + 10),
        ..Opts::default()
    });
    let admission = admitted(&w);
    assert!(state_now(&w, &admission, NOW + 9).permits_effect());
    match state_now(&w, &admission, NOW + 10) {
        AdmissionState::Refused(outcomes) => {
            assert_exact(
                &outcomes,
                &[(
                    CheckId::O4,
                    vec![r(ReasonCode::AuthorityExpired, "authority:auth-1")],
                )],
            );
        }
        other => panic!("expected refused, got {other:?}"),
    }
}

#[test]
fn state_refused_when_a_new_commit_makes_the_decision_scope_ambiguous() {
    let w = world(Opts::default());
    let admission = admitted(&w);
    w.src.issue(Record::fixture_decision_scope(
        "ds-1",
        1,
        scope(),
        &["accept_claim"],
        None,
    ));
    match state_now(&w, &admission, NOW) {
        AdmissionState::Refused(outcomes) => {
            // `ds-1` is shared by the owner grant and by both component bindings' grants, so every
            // one of them loses its decision scope (exact reasons, not check ids alone).
            assert_exact(
                &outcomes,
                &[(
                    CheckId::O4,
                    vec![
                        r(ReasonCode::AuthorityScopeUnresolved, "authority:auth-1"),
                        r(
                            ReasonCode::AuthorityScopeUnresolved,
                            "component:component:campaign/terminalize_campaign",
                        ),
                        r(
                            ReasonCode::AuthorityScopeUnresolved,
                            "component:component:episode_host/read_episode",
                        ),
                        r(
                            ReasonCode::AuthorityScopeUnresolved,
                            "component:component:episode_host/write_episode",
                        ),
                    ],
                )],
            );
        }
        other => panic!("expected refused, got {other:?}"),
    }
}

// --- Gate O: decision-scope containment (Amendment 6.1 F4) and authority resolution -----------------

fn owner_grant_world(owner: OwnerDs) -> World {
    world(Opts {
        owner_ds: Some(owner),
        ..Opts::default()
    })
}

fn decide_base(w: &World) -> Result<EffectAdmission, AdmissionRefusal> {
    decide_fixture(w, &request_for(&Req::default()), &realization(), NOW)
}

fn only_o4(reason: ReasonCode) -> [(CheckId, Vec<Reason>); 1] {
    [(CheckId::O4, vec![r(reason, "authority:auth-1")])]
}

#[test]
fn g1_o4_grant_for_another_problem_scope_refuses_only_o4() {
    let other = ProblemScopeRef::fixture("run-2", "plan-a", 1);
    let w = world(Opts {
        owner_grant_scope: Some(other),
        ..Opts::default()
    });
    assert_exact(
        &refused(decide_base(&w)),
        &only_o4(ReasonCode::AuthorityInsufficientProblemScope),
    );
    // Control: the same grant for the candidate's problem scope covers.
    assert!(decide_base(&world(Opts::default())).is_ok());
}

#[test]
fn g1_o4_decision_scope_of_another_problem_scope_refuses_only_o4() {
    let w = owner_grant_world(OwnerDs {
        scope: ProblemScopeRef::fixture("run-1", "plan-a", 2),
        ..owner_ds()
    });
    assert_exact(
        &refused(decide_base(&w)),
        &only_o4(ReasonCode::AuthorityInsufficientProblemScope),
    );
    // Control: the same decision scope for the candidate's problem scope covers.
    assert!(decide_base(&owner_grant_world(owner_ds())).is_ok());
}

#[test]
fn g1_o4_decision_scope_without_the_required_action_refuses_only_o4() {
    let w = owner_grant_world(OwnerDs {
        actions: &["read_episode"],
        ..owner_ds()
    });
    assert_exact(
        &refused(decide_base(&w)),
        &only_o4(ReasonCode::AuthorityInsufficientActions),
    );
    // Control: a decision scope listing the action among others covers.
    let w = owner_grant_world(OwnerDs {
        actions: &["read_episode", "accept_claim"],
        ..owner_ds()
    });
    assert!(decide_base(&w).is_ok());
}

#[test]
fn g1_o4_decision_scope_boundaries_must_contain_the_requirements_boundaries() {
    let req = request_for(&Req {
        authority_boundaries: Some(BOTH_BOUNDARIES),
        ..Req::default()
    });
    let decide = |boundaries: Option<&'static [&'static str]>| {
        let w = owner_grant_world(OwnerDs {
            boundaries,
            ..owner_ds()
        });
        decide_fixture(&w, &req, &realization(), NOW)
    };
    // Non-covering: the scope names one of the two required boundaries.
    assert_exact(
        &refused(decide(Some(&["builder_projection"]))),
        &only_o4(ReasonCode::AuthorityInsufficientBoundaries),
    );
    // Non-covering: the scope names an unrelated boundary only.
    assert_exact(
        &refused(decide(Some(&["unrelated_boundary"]))),
        &only_o4(ReasonCode::AuthorityInsufficientBoundaries),
    );
    // Controls: exactly the required boundaries, a superset, and no boundary restriction.
    assert!(decide(Some(BOTH_BOUNDARIES)).is_ok());
    assert!(
        decide(Some(&[
            "builder_projection",
            "campaign_terminalization",
            "extra_boundary"
        ]))
        .is_ok()
    );
    assert!(decide(None).is_ok());
}

#[test]
fn g1_o4_no_grant_for_the_permission_is_authority_missing() {
    let w = world(Opts {
        owner_grant_permission: "audit_log",
        ..Opts::default()
    });
    assert_exact(
        &refused(decide_base(&w)),
        &only_o4(ReasonCode::AuthorityMissing),
    );
}

#[test]
fn g1_o4_a_superseded_grant_does_not_cover_unless_its_replacement_does() {
    let w = world(Opts::default());
    let replacement = w.src.supersede(
        &w.owner_grant,
        Record::fixture_grant(
            "g-owner-2",
            w.owner,
            &["audit_log"],
            scope(),
            ("ds-1", 1),
            None,
        ),
    );
    assert_ne!(replacement, w.owner_grant);
    assert_exact(
        &refused(decide_base(&w)),
        &only_o4(ReasonCode::AuthoritySuperseded),
    );
    // Control: superseding by a grant with the same terms covers, and pins the replacement only.
    let w = world(Opts::default());
    let replacement = w.src.supersede(
        &w.owner_grant,
        Record::fixture_grant(
            "g-owner",
            w.owner,
            &["accept_claim"],
            scope(),
            ("ds-1", 1),
            None,
        ),
    );
    let admission = decide_base(&w).expect("replacement covers");
    assert!(admission.pins().grants.contains(&replacement));
    assert!(!admission.pins().grants.contains(&w.owner_grant));
}

#[test]
fn g1_o4_two_committed_principals_with_the_named_id_resolve_to_none() {
    // F-5: O4 resolves a named principal exactly as O8 does (exactly one committed principal).
    let w = world(Opts::default());
    principal(
        &w.src,
        "owner-a",
        PrincipalKind::Human,
        (AnchorKind::OwnerAccount, "bob"),
    );
    let outcomes = refused(decide_base(&w));
    assert_exact(
        &outcomes,
        &[
            (
                CheckId::O4,
                vec![r(ReasonCode::PrincipalAmbiguous, "authority:auth-1")],
            ),
            (
                CheckId::O8,
                vec![
                    r(
                        ReasonCode::SeparationUnresolved,
                        "cond-builder/principal:owner-a",
                    ),
                    r(
                        ReasonCode::SeparationUnresolved,
                        "cond-campaign/principal:owner-a",
                    ),
                ],
            ),
        ],
    );
}

#[test]
fn g1_o4_two_committed_bindings_of_one_component_are_ambiguous() {
    let w = world(Opts::default());
    w.src.issue(Record::fixture_component_binding(
        "component:campaign",
        w.campaign,
        ExecutableDigest::new("exe-campaign-2"),
        scope(),
    ));
    assert_exact(
        &refused(decide_base(&w)),
        &[(
            CheckId::O4,
            vec![r(
                ReasonCode::ComponentBindingAmbiguous,
                "component:component:campaign",
            )],
        )],
    );
}

// --- Gate R: remaining refusal branches ----------------------------------------------------------

fn request_at(vantage: &str) -> OperationRequest {
    let requirements = requirements(&Req::default());
    let digest = derive(&requirements, Profile::FixedSliceReviewV1).candidates[0].content_digest();
    OperationRequest::new(ORG, requirements, digest, vantage, subject())
}

#[test]
fn g1_r3_bound_realization_digest_differs_from_the_realization() {
    let w = world(Opts {
        binding_digest: "real-bound",
        ..Opts::default()
    });
    assert_exact(
        &refused(decide_base(&w)),
        &[(
            CheckId::R3,
            vec![r(ReasonCode::RealizationDigestMismatch, VANTAGE)],
        )],
    );
}

#[test]
fn g2_a_vantage_the_candidate_does_not_have_and_no_binding_refuse_r2_and_r3() {
    let w = world(Opts::default());
    let outcomes = refused(decide_fixture(
        &w,
        &request_at("vantage:nope"),
        &realization(),
        NOW,
    ));
    assert_exact(
        &outcomes,
        &[
            (
                CheckId::R2,
                vec![r(ReasonCode::VantageUnknown, "vantage:nope")],
            ),
            (
                CheckId::R3,
                vec![r(ReasonCode::VantageUnbound, "vantage:nope")],
            ),
            (
                CheckId::S1,
                vec![r(ReasonCode::VantageUnknown, "vantage:nope")],
            ),
        ],
    );
}

#[test]
fn g2_two_committed_bindings_of_the_vantage_are_ambiguous_for_o8_and_r3() {
    let w = world(Opts::default());
    w.src.issue(Record::fixture_vantage_binding(
        ORG,
        VANTAGE,
        w.builder,
        RealizationDigest::new("real-1"),
        "uid:svc-builder",
    ));
    assert_exact(
        &refused(decide_base(&w)),
        &[
            (
                CheckId::O8,
                vec![
                    r(
                        ReasonCode::SeparationUnresolved,
                        "cond-builder/vantage:ob-a",
                    ),
                    r(
                        ReasonCode::SeparationUnresolved,
                        "cond-campaign/vantage:ob-a",
                    ),
                ],
            ),
            (
                CheckId::R3,
                vec![r(ReasonCode::VantageBindingAmbiguous, VANTAGE)],
            ),
        ],
    );
}

#[test]
fn g2_two_committed_acceptances_of_one_digest_are_ambiguous_and_state_nothing() {
    let w = world(Opts::default());
    w.src.issue(Record::fixture_realization_acceptance(
        RealizationDigest::new("real-1"),
        "fixed-slice-review-v1",
        RealizationProperties::fixture(&[], None, None, None, None, None),
    ));
    let none = vec![r(
        ReasonCode::RealizationUnestablished,
        "no accepted realization",
    )];
    assert_exact(
        &refused(decide_base(&w)),
        &[
            (CheckId::R1, none.clone()),
            (CheckId::R2, none),
            (
                CheckId::R4,
                vec![r(ReasonCode::RealizationAcceptanceAmbiguous, "real-1")],
            ),
        ],
    );
}

#[test]
fn g2_r4_acceptance_for_another_profile_is_not_an_acceptance_for_the_candidate() {
    // F-4: the accepted record's profile must equal the candidate's profile.
    let w = world(Opts {
        acceptance_profile: "other-profile-v1",
        ..Opts::default()
    });
    let none = vec![r(
        ReasonCode::RealizationUnestablished,
        "no accepted realization",
    )];
    assert_exact(
        &refused(decide_base(&w)),
        &[
            (CheckId::R1, none.clone()),
            (CheckId::R2, none),
            (
                CheckId::R4,
                vec![r(ReasonCode::RealizationProfileMismatch, "real-1")],
            ),
        ],
    );
}

// --- F-2: the point check needs its own observation -------------------------------------------------

#[test]
fn state_a_revocation_after_admission_is_seen_only_by_a_new_observation() {
    let w = world(Opts::default());
    let reservation = reserve(boundary(1));
    let obs = observe_for(&w.src, &reservation).unwrap();
    let admission = admit::<Fixture>(
        &reservation,
        obs,
        &request_for(&Req::default()),
        &realization(),
        NOW,
    )
    .expect("admitted");
    w.src.revoke(&w.owner_grant);
    // `admit` consumed the admission-time observation (see the crate's compile-fail doctest), so
    // the only way to a point check is another observation, and it shows the revocation.
    assert_eq!(
        admission.admission_state(&observe_for(&w.src, &reservation), NOW),
        AdmissionState::Revoked(vec![w.owner_grant])
    );
}

// --- S1: scope.digest is the exact-subject digest ---------------------------------------------

/// The exact-subject digest is the episode codec's digest of the subject object (the owner
/// definition at `SubjectRef::scope_digest`), pinned for the base subject.
#[test]
fn s1_scope_digest_is_the_episode_codec_digest_of_the_subject() {
    let canonical = r#"{"commit":"c0ffee","patchIdentity":"patch-1","tree":"7ree"}"#;
    let expected = format!("{:x}", sha2_digest(canonical.as_bytes()));
    assert_eq!(subject().scope_digest(), expected);
    assert_eq!(expected.len(), 64);
}

fn sha2_digest(bytes: &[u8]) -> impl std::fmt::LowerHex {
    use sha2::Digest as _;
    sha2::Sha256::digest(bytes)
}

/// Positive: the base world admits, S1 among the passed checks, and the admission's outcome list
/// is exactly the ten check ids (S1 included).
#[test]
fn s1_the_base_admission_passes_s1_and_lists_every_check() {
    let w = world(Opts::default());
    let admission = decide_base(&w).expect("admits");
    let ids: Vec<CheckId> = admission.outcomes().iter().map(|(id, _)| id).collect();
    use CheckId::*;
    assert_eq!(ids, [O3, O4, O5, O8, O9, R1, R2, R3, R4, S1]);
    assert_eq!(admission.outcomes().get(S1), &CheckOutcome::Passed);
}

/// Negatives: a subject differing in exactly one of `commit`, `tree` and `patchIdentity` refuses
/// S1 alone, naming the served obligation and the scope digest it carries.
#[test]
fn s1_a_subject_differing_in_each_field_refuses_only_s1() {
    let w = world(Opts::default());
    let base = request_for(&Req::default());
    let scope = subject().scope_digest();
    let variants = [
        (
            "commit",
            SubjectRef {
                commit: "c0ffef".into(),
                ..subject()
            },
        ),
        (
            "tree",
            SubjectRef {
                tree: "7ref".into(),
                ..subject()
            },
        ),
        (
            "patchIdentity",
            SubjectRef {
                patch_identity: "patch-2".into(),
                ..subject()
            },
        ),
    ];
    for (field, other) in variants {
        assert_ne!(other.scope_digest(), scope, "{field}");
        let req = OperationRequest::new(
            ORG,
            requirements(&Req::default()),
            base.candidate_digest(),
            VANTAGE,
            other,
        );
        assert_exact(
            &refused(decide_fixture(&w, &req, &realization(), NOW)),
            &[(
                CheckId::S1,
                vec![r(
                    ReasonCode::ScopeDigestMismatch,
                    &format!("ob-a: {scope}"),
                )],
            )],
        );
    }
}

/// Negative control: requirements whose `scope.digest` is the stand-in `patchIdentity` (the shape
/// of the frozen world) refuse S1 alone for the subject that carries that patch identity.
#[test]
fn s1_a_patch_identity_stand_in_scope_digest_refuses_only_s1() {
    let w = world(Opts::default());
    let stand_in = "5212141d54e001b1fa85f7660fbf9605a957cf11a7636636dc03d11dded58c51";
    let requirements = requirements(&Req {
        scope_digest: stand_in.to_owned(),
        ..Req::default()
    });
    let digest = derive(&requirements, Profile::FixedSliceReviewV1).candidates[0].content_digest();
    let req = OperationRequest::new(
        ORG,
        requirements,
        digest,
        VANTAGE,
        SubjectRef {
            patch_identity: stand_in.into(),
            ..subject()
        },
    );
    assert_exact(
        &refused(decide_fixture(&w, &req, &realization(), NOW)),
        &[(
            CheckId::S1,
            vec![r(
                ReasonCode::ScopeDigestMismatch,
                &format!("ob-a: {stand_in}"),
            )],
        )],
    );
}

/// S1 compares the obligations the request's vantage serves, not the candidate's first or every
/// obligation: in a two-obligation candidate where only `ob-b` carries the subject's digest,
/// `vantage:ob-b` passes S1 and `vantage:ob-a` refuses it. (Derivation gives each reviewer vantage
/// exactly one served obligation, so a vantage serving several obligations is not constructible.)
#[test]
fn s1_compares_the_obligation_the_vantage_serves() {
    let w = world(Opts::default());
    let mut doc = requirements_doc(&Req {
        scope_digest: "b".repeat(64),
        ..Req::default()
    });
    let mut second = doc["obligations"][0].clone();
    second["id"] = json!("ob-b");
    second["scope"]["digest"] = json!(subject().scope_digest());
    doc["obligations"].as_array_mut().unwrap().push(second);
    let mut conditions = doc["acceptanceConditions"].as_array().unwrap().clone();
    for c in &mut conditions {
        c["id"] = json!(format!("{}-b", c["id"].as_str().unwrap()));
        c["obligation"] = json!("ob-b");
    }
    doc["acceptanceConditions"]
        .as_array_mut()
        .unwrap()
        .extend(conditions);
    let requirements = parse(&doc);
    let digest = derive(&requirements, Profile::FixedSliceReviewV1).candidates[0].content_digest();
    let outcome_at = |vantage: &str| {
        let req = OperationRequest::new(ORG, requirements.clone(), digest, vantage, subject());
        match decide_fixture(&w, &req, &realization(), NOW) {
            Ok(admission) => admission.outcomes().get(CheckId::S1).clone(),
            Err(AdmissionRefusal::Checks(outcomes)) => outcomes.get(CheckId::S1).clone(),
            Err(other) => panic!("expected checks, got {other:?}"),
        }
    };
    assert_eq!(outcome_at("vantage:ob-b"), CheckOutcome::Passed);
    assert_eq!(
        outcome_at("vantage:ob-a"),
        CheckOutcome::Refused(vec![r(
            ReasonCode::ScopeDigestMismatch,
            &format!("ob-a: {}", "b".repeat(64)),
        )])
    );
}

/// A vantage that is a participant but serves no obligation (the builder's) has no scope to
/// relate to the subject: S1 refuses with `ScopeDigestUnserved`.
#[test]
fn s1_a_vantage_that_serves_no_obligation_refuses_s1() {
    let w = world(Opts::default());
    let outcomes = refused(decide_fixture(
        &w,
        &request_at("vantage-builder"),
        &realization(),
        NOW,
    ));
    assert_eq!(
        outcomes.get(CheckId::S1),
        &CheckOutcome::Refused(vec![r(ReasonCode::ScopeDigestUnserved, "vantage-builder")])
    );
}

/// Regression: a pin revoked after an S1-passing admission still reads as revoked.
#[test]
fn s1_a_pin_revoked_after_admission_is_still_revoked() {
    let w = world(Opts::default());
    let admission = decide_base(&w).expect("admits");
    assert!(admission.outcomes().passed(CheckId::S1));
    w.src.revoke(&w.owner_grant);
    assert!(matches!(
        state_now(&w, &admission, NOW),
        AdmissionState::Revoked(_)
    ));
}
