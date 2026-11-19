#![allow(dead_code)]
//! Shared test support. The compiler and the episode core are the real owners; the requirements,
//! selection and episode authority are the frozen fixture bytes, whose digests are asserted
//! against `provenance.json`'s record. Frozen provenance is a source of fixture values, not a
//! production binding.
use review_episode_core::codec::{JsValue, canonical_json, parse_json};
use review_episode_core::command::BeginCommand;
use review_episode_core::identity::Authority;
use review_episode_core::reducer::begin;
use review_episode_core::state::ReviewEpisodeState;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use work_engine_organization::{
    OrganizationCandidate, OrganizationalRequirements, Profile, derive,
};
use work_engine_review_preparation::{
    MutationClaim, PreparationRequest, RealizationClaim, Subject,
};

pub const REQ: &[u8] =
    include_bytes!("../../../work-engine-organization/tests/fixtures/frozen/requirements.json");
pub const SELECTION: &[u8] =
    include_bytes!("../../../work-engine-organization/tests/fixtures/frozen/selection.json");
pub const AUTHORITY: &str =
    include_str!("../../../work-engine-organization/tests/fixtures/frozen/episode-authority.json");
pub const ACCEPTABLE: &str = include_str!(
    "../../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json"
);

pub const OBLIGATION: &str = "rp3-initial-implementation-review";
pub const COND_BUILDER: &str = "cond-builder-projection";
pub const COND_CAMPAIGN: &str = "cond-campaign-terminalization";

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn candidate_from(bytes: &[u8]) -> OrganizationCandidate {
    let requirements = OrganizationalRequirements::from_json_bytes(bytes).expect("owner accepts");
    let mut derived = derive(&requirements, Profile::FixedSliceReviewV1);
    assert!(derived.unresolved.is_empty(), "{:?}", derived.unresolved);
    assert_eq!(derived.candidates.len(), 1);
    derived.candidates.remove(0)
}

pub fn candidate() -> OrganizationCandidate {
    candidate_from(REQ)
}

/// The frozen requirements with `scope.digest` set to `subject`'s exact-subject digest (A1): the
/// episode codec's digest of `{commit, tree, patchIdentity}`. S-A check S1 refuses the frozen
/// file's `patchIdentity` stand-in, so every S-A admission in these tests is decided over a
/// scoped world. Labelled synthetic fixture; the frozen file is unchanged.
pub fn scoped_req_for(subject: &Subject) -> Vec<u8> {
    scoped_edited_for(subject, |_| {})
}

/// [`scoped_req_for`] of the frozen subject.
pub fn scoped_req() -> Vec<u8> {
    scoped_req_for(&subject())
}

/// [`scoped_req`] with `edit` applied, as authored JSON bytes.
pub fn scoped_edited(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    scoped_edited_for(&subject(), edit)
}

fn scoped_edited_for(subject: &Subject, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    edited(|value| {
        value["obligations"][0]["scope"]["digest"] =
            json!(review_episode_core::codec::digest(&subject.to_js_value()));
        edit(value);
    })
}

/// The frozen requirements with `edit` applied, as authored JSON bytes.
pub fn edited(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value: Value = serde_json::from_slice(REQ).unwrap();
    edit(&mut value);
    serde_json::to_vec(&value).unwrap()
}

pub fn for_each_condition(value: &mut Value, mut edit: impl FnMut(&mut Value)) {
    for condition in value["acceptanceConditions"].as_array_mut().unwrap() {
        edit(condition);
    }
}

/// The exact subject of the frozen vertical (selection.json and episode-authority.json).
pub fn subject() -> Subject {
    Subject {
        commit: "96808de3d565a6c4b0211e8dc263cb7343514d74".to_owned(),
        tree: "9c5d94ba059ea5b4ff497a0540f6b8a7476e4474".to_owned(),
        patch_identity: "5212141d54e001b1fa85f7660fbf9605a957cf11a7636636dc03d11dded58c51"
            .to_owned(),
    }
}

pub fn request(realization: Option<RealizationClaim>) -> PreparationRequest {
    PreparationRequest {
        obligation_id: OBLIGATION.to_owned(),
        subject: subject(),
        realization,
    }
}

/// A descriptor whose every stated value matches the candidate's requirements. Still a claim.
pub fn matching_claim() -> RealizationClaim {
    RealizationClaim {
        provider_family: Some("anthropic".to_owned()),
        execution_confinement: Some("guarded_sandbox".to_owned()),
        mutation: MutationClaim::ReadOnlyClaimed,
        granted_tools: vec![],
        continuity: Some("fresh_initial".to_owned()),
    }
}

/// The frozen episode authority, begun through the real reducer entry point.
pub fn authority() -> Authority {
    Authority::parse(parse_json(AUTHORITY).unwrap()).unwrap()
}

pub fn begun() -> ReviewEpisodeState {
    let command = BeginCommand::new(authority(), "begin-1", vec![]).unwrap();
    begin(None, &command).unwrap().into_state()
}

pub fn canonical(state: &ReviewEpisodeState) -> String {
    canonical_json(state.value())
}

pub fn json_value(js: &JsValue) -> Value {
    serde_json::from_str(&canonical_json(js)).unwrap()
}

/// The acceptable result sample with its subject replaced by the frozen subject.
pub fn result_json(edit: impl FnOnce(&mut Value)) -> String {
    let mut value: Value = serde_json::from_str(ACCEPTABLE).unwrap();
    value["subject"] = json!({
        "commit": subject().commit,
        "tree": subject().tree,
        "patchIdentity": subject().patch_identity,
    });
    edit(&mut value);
    serde_json::to_string(&value).unwrap()
}

/// A synthetic S-A authority world (fixture records) that admits the frozen candidate for the
/// frozen subject and the obligation's reviewer vantage, and the consumption proof built from it.
/// A labelled test assumption: the records, the source and the time are synthetic, like the
/// declaration.
pub mod gate {
    use super::*;
    use work_engine_organization::{
        Continuity, EvidenceCustody, ExecutionConfinement, Integrity, Mutation, ObserverClass,
        ProviderFamily, RealizationClass,
    };
    use work_engine_organization_authority::fixture::FixtureSource;
    use work_engine_organization_authority::records::{
        ActorAnchor, AnchorKind, PrincipalKind, ProblemScopeRef, RealizationProperties, Record,
    };
    use work_engine_organization_authority::{
        Boundary, BoundaryKind, DeclaredRealization, EffectAdmission, ExecutableDigest, Fixture,
        IssuanceId, OperationRequest, RealizationDigest, Reservation, SealedObservation,
        SubjectRef, Unobserved, admit, observe_for, reserve,
    };

    pub const ORG: &str = "org-fixture-consumer";
    /// The reviewer vantage the v1 compiler derives for the obligation.
    pub const VANTAGE: &str = "vantage:rp3-initial-implementation-review";
    const ACCEPTOR: &str = "operator-production-qualification-20261107";
    const ACTOR: &str = "uid:fixture-reviewer";
    pub const OPERATION: &str = "fixture-consumer-op-1";
    pub const NOW: u64 = 1_000;
    const DS: (&str, u64) = ("ds-1", 1);

    fn digest() -> RealizationDigest {
        RealizationDigest::new("fixture-consumer-realization")
    }

    fn scope() -> ProblemScopeRef {
        ProblemScopeRef::fixture(
            "production-qualification-20261107",
            "timestamp-projection-v1",
            1,
        )
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

    /// Issues the world: the organization admission of the request's candidate first, then the
    /// records whose vantage bindings belong to it. Returns the owner's grant (the one a test
    /// revokes).
    fn issue(
        src: &FixtureSource,
        request: &OperationRequest,
        extra_vantages: &[String],
    ) -> IssuanceId {
        let organization = src.issue(Record::fixture_organization_admission(
            ORG,
            1,
            None,
            scope(),
            request.requirements_digest(),
            request.candidate_digest(),
        ));
        let owner = principal(
            src,
            ACCEPTOR,
            PrincipalKind::Human,
            (AnchorKind::OwnerAccount, "operator"),
        );
        let reviewer = principal(
            src,
            "reviewer-fixture",
            PrincipalKind::Service,
            (AnchorKind::ServiceAccount, "svc-reviewer"),
        );
        let builder = principal(
            src,
            "builder-1",
            PrincipalKind::Service,
            (AnchorKind::ServiceAccount, "svc-builder"),
        );
        let campaign = principal(
            src,
            "campaign-svc",
            PrincipalKind::Service,
            (AnchorKind::ServiceAccount, "svc-campaign"),
        );
        let episode = principal(
            src,
            "episode-svc",
            PrincipalKind::Service,
            (AnchorKind::ServiceAccount, "svc-episode"),
        );
        src.issue(Record::fixture_decision_scope(
            DS.0,
            DS.1,
            scope(),
            &[
                "accept_claim",
                "terminalize_campaign",
                "read_episode",
                "write_episode",
            ],
            None,
        ));
        src.issue(Record::fixture_grant(
            "g-campaign",
            campaign,
            &["terminalize_campaign"],
            scope(),
            DS,
            None,
        ));
        src.issue(Record::fixture_grant(
            "g-episode",
            episode,
            &["read_episode", "write_episode"],
            scope(),
            DS,
            None,
        ));
        src.issue(Record::fixture_component_binding(
            "component:campaign",
            campaign,
            ExecutableDigest::new("exe-campaign"),
            scope(),
        ));
        src.issue(Record::fixture_component_binding(
            "component:episode_host",
            episode,
            ExecutableDigest::new("exe-episode"),
            scope(),
        ));
        src.issue(Record::fixture_vantage_binding(
            organization,
            VANTAGE,
            reviewer,
            digest(),
            ACTOR,
        ));
        // Further reviewer vantages (a second obligation's), bound like the first: same principal,
        // realization digest and actor, so the same acceptance covers them.
        for vantage in extra_vantages {
            src.issue(Record::fixture_vantage_binding(
                organization,
                vantage,
                reviewer,
                digest(),
                ACTOR,
            ));
        }
        src.issue(Record::fixture_vantage_binding(
            organization,
            "vantage-builder",
            builder,
            RealizationDigest::new("real-builder"),
            "uid:svc-builder",
        ));
        src.issue(Record::fixture_realization_acceptance(
            digest(),
            "fixed-slice-review-v1",
            RealizationProperties::fixture(
                &[
                    RealizationClass::ProviderFamily(ProviderFamily::Anthropic),
                    RealizationClass::ExecutionConfinement(ExecutionConfinement::GuardedSandbox),
                ],
                Some(Continuity::FreshInitial),
                Some(Mutation::None),
                Some(Integrity::VerifiedArtifacts),
                Some(EvidenceCustody::ExecutionEvidenceCustody),
                Some(ObserverClass::IndependentExecutionEvidence),
            ),
        ));
        src.issue(Record::fixture_grant(
            "g-owner",
            owner,
            &["accept_claim"],
            scope(),
            DS,
            None,
        ))
    }

    /// What an admission is requested for: the scoped frozen requirements ([`scoped_req`]),
    /// candidate, subject and reviewer vantage by default, and a consumption boundary.
    pub struct Ask {
        pub requirements: Vec<u8>,
        pub subject: Subject,
        pub vantage: String,
        pub kind: BoundaryKind,
        /// Vantages the world binds and accepts besides [`VANTAGE`]; empty by default.
        pub bound_vantages: Vec<String>,
    }

    impl Default for Ask {
        fn default() -> Self {
            Ask {
                requirements: scoped_req(),
                subject: subject(),
                vantage: VANTAGE.to_owned(),
                kind: BoundaryKind::Consumption,
                bound_vantages: Vec::new(),
            }
        }
    }

    /// One admitted world: the source, the owner grant, the admission and a fresh regate
    /// observation for the admission's reservation.
    pub struct Admitted {
        pub src: FixtureSource,
        pub owner_grant: IssuanceId,
        reservation: Reservation,
        pub admission: EffectAdmission,
        pub observation: Result<SealedObservation, Unobserved>,
        pub now: u64,
    }

    impl Admitted {
        /// A fresh observation of the source for the admission's reservation.
        pub fn reobserve(&mut self) {
            self.observation = observe_for(&self.src, &self.reservation);
        }

        #[cfg(feature = "fixture")]
        pub fn proof(&self) -> work_engine_review_preparation::fixture::ConsumptionProof<'_> {
            work_engine_review_preparation::fixture::ConsumptionProof {
                admission: &self.admission,
                regate_observation: &self.observation,
                regate_now_millis: self.now,
            }
        }
    }

    pub fn admitted(ask: Ask) -> Admitted {
        let src = FixtureSource::new();
        let requirements = OrganizationalRequirements::from_json_bytes(&ask.requirements).unwrap();
        let candidate_digest =
            derive(&requirements, Profile::FixedSliceReviewV1).candidates[0].content_digest();
        let request = OperationRequest::new(
            requirements,
            candidate_digest,
            ask.vantage,
            SubjectRef {
                commit: ask.subject.commit,
                tree: ask.subject.tree,
                patch_identity: ask.subject.patch_identity,
            },
        );
        let owner_grant = issue(&src, &request, &ask.bound_vantages);
        let declared = DeclaredRealization::declared(
            digest(),
            ACTOR,
            [
                (
                    "component:campaign".to_owned(),
                    ExecutableDigest::new("exe-campaign"),
                ),
                (
                    "component:episode_host".to_owned(),
                    ExecutableDigest::new("exe-episode"),
                ),
            ],
        );
        let reservation = reserve(Boundary::new(OPERATION, ask.kind, 1));
        let first = observe_for(&src, &reservation).expect("observable");
        let admission = admit::<Fixture>(&reservation, first, &request, &declared, NOW)
            .unwrap_or_else(|refusal| panic!("the fixture world admits: {refusal:?}"));
        let observation = observe_for(&src, &reservation);
        Admitted {
            src,
            owner_grant,
            reservation,
            admission,
            observation,
            now: NOW,
        }
    }
}

pub fn reference(owner: &str, name: &str, revision: &str) -> Value {
    json!({
        "owner": owner,
        "reference": name,
        "revision": revision,
        "sha256": sha256_hex(name.as_bytes()),
        "freshness": "exact_revision",
    })
}
