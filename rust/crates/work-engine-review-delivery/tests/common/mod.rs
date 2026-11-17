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
