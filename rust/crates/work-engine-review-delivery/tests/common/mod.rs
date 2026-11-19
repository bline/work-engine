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

/// The R2-S assumption world: the frozen requirements without `realizationConstraints` (see
/// `tests/fixtures/assumption-world/DERIVATION.md`). A labelled synthetic fixture.
pub const ASSUMPTION_REQ: &[u8] = include_bytes!("../fixtures/assumption-world/requirements.json");

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

/// The assumption world's candidate, compiled by the unchanged v1 compiler.
pub fn assumption_candidate() -> OrganizationCandidate {
    candidate_from(ASSUMPTION_REQ)
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

pub fn reference(owner: &str, name: &str, revision: &str) -> Value {
    json!({
        "owner": owner,
        "reference": name,
        "revision": revision,
        "sha256": sha256_hex(name.as_bytes()),
        "freshness": "exact_revision",
    })
}

#[cfg(feature = "fixture")]
pub mod world {
    use super::*;
    use work_engine_review_preparation::fixture::{FIXTURE_LABEL, FixtureDeclaration};
    pub struct World {
        pub candidate: OrganizationCandidate,
        pub declaration: FixtureDeclaration,
        pub admissions: Vec<Value>,
    }

    fn identity_of(authority: &Value, key: &str) -> Value {
        authority["identity"][key].clone()
    }

    /// The request the host prepares with: the frozen obligation and subject with the host's own
    /// realization description (what the binding derives internally).
    pub fn host_request() -> PreparationRequest {
        let cfg = work_engine_review_delivery::native::NativeConfig {
            node: "node".into(),
            adapter: work_engine_review_delivery::native::NativeConfig::default_adapter(),
            sdk_root: "unused".into(),
            fixture_root: "unused".into(),
            timeout: std::time::Duration::from_secs(1),
            cleanup: std::time::Duration::from_secs(1),
            control: work_engine_review_delivery::native::Control::Positive,
            limits: Default::default(),
        };
        request(Some(
            work_engine_review_delivery::native::realization_description(&cfg),
        ))
    }

    /// The R2-S synthetic world the native suite runs on: the assumption world (no realization
    /// constraints), its declaration and the admissions that agree with it.
    pub fn world() -> World {
        world_from(assumption_candidate())
    }

    /// The frozen world: the R2-S negative (known realization incompatibility before spawn).
    pub fn frozen_world() -> World {
        world_from(candidate())
    }

    /// A synthetic world over `candidate`: the declaration and the admissions that agree with it.
    pub fn world_from(candidate: OrganizationCandidate) -> World {
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
}

/// The S-B fixture authority world (labelled synthetic): owner-attested records that admit the
/// R2-S assumption world's request under the S-A fixture policy. Principals, vantage-actor
/// bindings, component bindings, grants, one decision scope and the realization acceptance of the
/// host's own realization digest. Nothing here is a production record or evidence.
#[cfg(feature = "fixture")]
pub mod authority {
    use super::*;
    use std::cell::Cell;
    use work_engine_organization::{
        Continuity, EvidenceCustody, Integrity, Mutation, ObserverClass,
    };
    use work_engine_organization_authority::fixture::FixtureSource;
    use work_engine_organization_authority::records::{
        ActorAnchor, AnchorKind, PrincipalKind, ProblemScopeRef, RealizationProperties, Record,
    };
    use work_engine_organization_authority::{
        AuthorityObservation, AuthorityState, Boundary, BoundaryKind, DeclaredRealization,
        ExecutableDigest, IssuanceId, OperationRequest, RealizationDigest, SubjectRef, Unobserved,
    };
    use work_engine_review_delivery::gate::TimeSource;
    use work_engine_review_delivery::native::{FixtureAuthority, realization_digest};

    /// A synthetic clock that always reads the same time.
    pub struct FixedTime(pub u64);
    impl TimeSource for FixedTime {
        fn now_millis(&self) -> u64 {
            self.0
        }
    }
    pub static FIXED: FixedTime = FixedTime(NOW);

    /// A synthetic clock that reads `times[n]` at its n-th sample (the last value afterwards) and
    /// counts its samples, so a test sees exactly when the host sampled time.
    pub struct ScriptedClock {
        times: Vec<u64>,
        samples: Cell<usize>,
    }
    impl ScriptedClock {
        pub fn new(times: &[u64]) -> Self {
            Self {
                times: times.to_vec(),
                samples: Cell::new(0),
            }
        }
        pub fn samples(&self) -> usize {
            self.samples.get()
        }
    }
    impl TimeSource for ScriptedClock {
        fn now_millis(&self) -> u64 {
            let n = self.samples.get();
            self.samples.set(n + 1);
            self.times[n.min(self.times.len() - 1)]
        }
    }

    pub const ORG: &str = "org-fixture-review-delivery";
    /// The reviewer vantage the v1 compiler derives for the obligation.
    pub const VANTAGE: &str = "vantage:rp3-initial-implementation-review";
    /// The acceptor both authority requirements name.
    pub const ACCEPTOR: &str = "operator-production-qualification-20261107";
    /// The declared actor identity of the native runner (a fixture value R3 compares).
    pub const ACTOR: &str = "uid:fixture-native-runner";
    pub const OPERATION: &str = "fixture-review-delivery-op-1";
    pub const NOW: u64 = 1_000;
    const DS: (&str, u64) = ("ds-1", 1);

    pub fn problem_scope() -> ProblemScopeRef {
        ProblemScopeRef::fixture(
            "production-qualification-20261107",
            "timestamp-projection-v1",
            1,
        )
    }

    /// The host's realization digest of its own description (what the binding prepares with).
    pub fn host_digest() -> RealizationDigest {
        realization_digest(world::host_request().realization.as_ref().unwrap())
    }

    pub struct Ids {
        /// The `OrganizationAdmission` of the assumption world's candidate; every vantage binding
        /// belongs to it.
        pub organization: IssuanceId,
        pub owner: IssuanceId,
        pub reviewer: IssuanceId,
        pub ds: IssuanceId,
        pub reviewer_binding: IssuanceId,
        pub campaign_binding: IssuanceId,
        pub acceptance: IssuanceId,
        pub owner_grant: Option<IssuanceId>,
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

    /// Variations of the authority world.
    #[derive(Clone, Copy)]
    pub struct Shape {
        pub owner_grant: bool,
        /// Expiry of the owner's grant (`g-owner`).
        pub owner_expiry: Option<u64>,
        /// Bind the preparation's vantage to a different actor than the declared one and bind the
        /// builder vantage (a second vantage, V') to the declared actor and the host digest.
        pub swap_actors: bool,
    }

    pub const OTHER_ACTOR: &str = "uid:some-other-actor";

    /// The organization admission of `request`'s candidate (its content digest, requirements
    /// digest and problem scope), revision 1.
    pub fn organization_admission(organization_id: &str, request: &OperationRequest) -> Record {
        Record::fixture_organization_admission(
            organization_id,
            1,
            None,
            problem_scope(),
            request.requirements_digest(),
            request.candidate_digest(),
        )
    }

    /// Issue the world into `src`. The organization admission is the first issuance and the
    /// owner's grant the last, so a world issued without the grant shares every other issuance
    /// identity with one issued with it.
    pub fn issue(src: &FixtureSource, owner_grant: bool) -> Ids {
        issue_with(
            src,
            Shape {
                owner_grant,
                owner_expiry: None,
                swap_actors: false,
            },
        )
    }

    pub fn issue_with(src: &FixtureSource, shape: Shape) -> Ids {
        issue_admitting(src, shape, &operation_request())
    }

    /// The world with the organization admission of `request`'s candidate instead of the
    /// assumption world's.
    pub fn issue_admitting(src: &FixtureSource, shape: Shape, request: &OperationRequest) -> Ids {
        let (owner_grant, owner_expiry) = (shape.owner_grant, shape.owner_expiry);
        let organization = src.issue(organization_admission(ORG, request));
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
        let ds = src.issue(Record::fixture_decision_scope(
            DS.0,
            DS.1,
            problem_scope(),
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
            problem_scope(),
            DS,
            None,
        ));
        src.issue(Record::fixture_grant(
            "g-episode",
            episode,
            &["read_episode", "write_episode"],
            problem_scope(),
            DS,
            None,
        ));
        let campaign_binding = src.issue(Record::fixture_component_binding(
            "component:campaign",
            campaign,
            ExecutableDigest::new("exe-campaign"),
            problem_scope(),
        ));
        src.issue(Record::fixture_component_binding(
            "component:episode_host",
            episode,
            ExecutableDigest::new("exe-episode"),
            problem_scope(),
        ));
        let (reviewer_actor, builder_realization, builder_actor) = if shape.swap_actors {
            (OTHER_ACTOR, host_digest(), ACTOR)
        } else {
            (
                ACTOR,
                RealizationDigest::new("real-builder"),
                "uid:svc-builder",
            )
        };
        let reviewer_binding = src.issue(Record::fixture_vantage_binding(
            organization,
            VANTAGE,
            reviewer,
            host_digest(),
            reviewer_actor,
        ));
        src.issue(Record::fixture_vantage_binding(
            organization,
            "vantage-builder",
            builder,
            builder_realization,
            builder_actor,
        ));
        let acceptance = src.issue(Record::fixture_realization_acceptance(
            host_digest(),
            "fixed-slice-review-v1",
            RealizationProperties::fixture(
                &[],
                Some(Continuity::FreshInitial),
                Some(Mutation::None),
                Some(Integrity::VerifiedArtifacts),
                Some(EvidenceCustody::ExecutionEvidenceCustody),
                Some(ObserverClass::IndependentExecutionEvidence),
            ),
        ));
        let owner_grant = owner_grant.then(|| {
            src.issue(Record::fixture_grant(
                "g-owner",
                owner,
                &["accept_claim"],
                problem_scope(),
                DS,
                owner_expiry,
            ))
        });
        Ids {
            organization,
            owner,
            reviewer,
            ds,
            reviewer_binding,
            campaign_binding,
            acceptance,
            owner_grant,
        }
    }

    pub struct AuthorityWorld {
        pub src: FixtureSource,
        pub ids: Ids,
    }

    pub fn authority_world() -> AuthorityWorld {
        let src = FixtureSource::new();
        let ids = issue(&src, true);
        AuthorityWorld { src, ids }
    }

    /// The authority world whose organization admission admits `request`'s candidate.
    pub fn authority_world_admitting(request: &OperationRequest) -> AuthorityWorld {
        let src = FixtureSource::new();
        let ids = issue_admitting(
            &src,
            Shape {
                owner_grant: true,
                owner_expiry: None,
                swap_actors: false,
            },
            request,
        );
        AuthorityWorld { src, ids }
    }

    pub fn authority_world_shaped(shape: Shape) -> AuthorityWorld {
        let src = FixtureSource::new();
        let ids = issue_with(&src, shape);
        AuthorityWorld { src, ids }
    }

    pub fn installed() -> Vec<(String, ExecutableDigest)> {
        vec![
            (
                "component:campaign".to_owned(),
                ExecutableDigest::new("exe-campaign"),
            ),
            (
                "component:episode_host".to_owned(),
                ExecutableDigest::new("exe-episode"),
            ),
        ]
    }

    /// The host's declared realization (what `deliver_and_consume` builds), for direct S-A tests.
    pub fn declared() -> DeclaredRealization {
        DeclaredRealization::declared(host_digest(), ACTOR, installed())
    }

    pub fn operation_request_from(requirements: &[u8], subject: Subject) -> OperationRequest {
        let requirements = OrganizationalRequirements::from_json_bytes(requirements).unwrap();
        let digest =
            derive(&requirements, Profile::FixedSliceReviewV1).candidates[0].content_digest();
        OperationRequest::new(
            requirements,
            digest,
            VANTAGE,
            SubjectRef {
                commit: subject.commit,
                tree: subject.tree,
                patch_identity: subject.patch_identity,
            },
        )
    }

    /// The assumption world's request for the frozen subject. For direct S-A / `HeldAdmission`
    /// tests only: `FixtureAuthority` has no request (the host constructs it).
    pub fn operation_request() -> OperationRequest {
        operation_request_from(ASSUMPTION_REQ, super::subject())
    }

    /// The requirements preimage of the assumption world.
    pub fn requirements_from(bytes: &[u8]) -> OrganizationalRequirements {
        OrganizationalRequirements::from_json_bytes(bytes).unwrap()
    }

    pub fn fixture_authority(source: &dyn AuthorityObservation) -> FixtureAuthority<'_> {
        fixture_authority_with(source, &FIXED, requirements_from(ASSUMPTION_REQ))
    }

    pub fn fixture_authority_with<'a>(
        source: &'a dyn AuthorityObservation,
        clock: &'a dyn TimeSource,
        requirements: OrganizationalRequirements,
    ) -> FixtureAuthority<'a> {
        FixtureAuthority {
            source,
            clock,
            requirements,
            actor_identity: ACTOR.to_owned(),
            installed: installed(),
        }
    }

    pub fn boundary(kind: BoundaryKind) -> Boundary {
        Boundary::new(OPERATION, kind, 1)
    }

    /// A source whose observations come from `a` until `switched`, then from `b`. With `b` issued
    /// as `a` without the owner's grant and padded past `a`'s revision, a pinned grant is missing
    /// at a later revision.
    pub struct Switch {
        pub a: AuthorityWorld,
        pub b: FixtureSource,
        pub switched: Cell<bool>,
    }

    impl Switch {
        pub fn missing_owner_grant() -> Self {
            let a = authority_world();
            let b = FixtureSource::new();
            issue(&b, false);
            principal(
                &b,
                "padding-1",
                PrincipalKind::Human,
                (AnchorKind::OwnerAccount, "pad-1"),
            );
            principal(
                &b,
                "padding-2",
                PrincipalKind::Human,
                (AnchorKind::OwnerAccount, "pad-2"),
            );
            Switch {
                a,
                b,
                switched: Cell::new(false),
            }
        }
    }

    impl AuthorityObservation for Switch {
        fn observe(&self) -> Result<AuthorityState, Unobserved> {
            if self.switched.get() {
                self.b.observe()
            } else {
                self.a.src.observe()
            }
        }
    }
}

/// Explicit external inputs of the native vertical, and per-control scratch roots.
#[cfg(feature = "fixture")]
pub mod native_inputs {
    use super::sha256_hex;
    use std::path::{Path, PathBuf};

    /// SHA-256 of the handoff's `pi.manifest.json` for the selected read-only Pi SDK 1.5.0.
    pub const SDK_MANIFEST_SHA256: &str =
        "cf9d096e61d8ffd2c35df238ff74e1f8d7dacfb9bc5813333beef6cc73010985";
    /// SHA-256 of the SDK entry module, as recorded in that manifest.
    pub const SDK_ENTRY_SHA256: &str =
        "4569157e6038be9840fa514db4b00a27fd0be9e3622cc7b49a281de6c027d7ff";
    pub const SDK_ENTRY: &str = "node_modules/@earendil-works/pi-coding-agent/dist/index.js";

    /// The selected Pi SDK root, given explicitly by `WE_PI_SDK_ROOT` together with its handoff
    /// manifest `WE_PI_SDK_MANIFEST`. Checked on every call: the manifest bytes have the pinned
    /// digest; the manifest records the pinned entry digest; the entry module under the root has
    /// those bytes. The pin covers exactly that: the manifest's identity and the entry module's
    /// bytes. The rest of the SDK tree is NOT re-verified against the manifest here (no tree
    /// walk), so a changed file elsewhere under the root would not be detected by this check.
    /// A missing or mismatched input refuses (panics with the reason); these tests never skip.
    pub fn sdk_root() -> PathBuf {
        let var = |name: &str| {
            std::env::var_os(name).map(PathBuf::from).unwrap_or_else(|| {
                panic!(
                    "{name} is required: the native vertical runs against an explicitly selected \
                     read-only Pi SDK (WE_PI_SDK_ROOT=<handoff>/pi, \
                     WE_PI_SDK_MANIFEST=<handoff>/pi.manifest.json); this test does not skip"
                )
            })
        };
        let root = var("WE_PI_SDK_ROOT");
        let manifest_path = var("WE_PI_SDK_MANIFEST");
        let manifest = std::fs::read(&manifest_path)
            .unwrap_or_else(|e| panic!("WE_PI_SDK_MANIFEST {}: {e}", manifest_path.display()));
        assert_eq!(
            sha256_hex(&manifest),
            SDK_MANIFEST_SHA256,
            "WE_PI_SDK_MANIFEST {} is not the selected handoff manifest",
            manifest_path.display()
        );
        let recorded: serde_json::Value =
            serde_json::from_slice(&manifest).expect("the handoff manifest is JSON");
        assert_eq!(
            recorded[SDK_ENTRY]["sha256"], SDK_ENTRY_SHA256,
            "the handoff manifest records the pinned entry digest"
        );
        let entry = root.join(SDK_ENTRY);
        let bytes = std::fs::read(&entry).unwrap_or_else(|e| {
            panic!(
                "WE_PI_SDK_ROOT {}: SDK entry {}: {e}",
                root.display(),
                entry.display()
            )
        });
        assert_eq!(
            sha256_hex(&bytes),
            SDK_ENTRY_SHA256,
            "WE_PI_SDK_ROOT {} does not hold the selected SDK entry bytes",
            root.display()
        );
        root
    }

    /// A fresh scratch root for one native control under Cargo's per-target test temp directory.
    /// Removed on drop; kept, with its path printed, when the test is unwinding (an ordinary
    /// diagnostic, not retained evidence).
    pub struct Scratch {
        pub path: PathBuf,
    }

    impl Scratch {
        pub fn new(name: &str) -> Self {
            let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
                .join("native-vertical")
                .join(format!("{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("scratch root is creatable");
            Self { path }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            if std::thread::panicking() {
                eprintln!("scratch retained for diagnosis: {}", self.path.display());
            } else {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }
}
