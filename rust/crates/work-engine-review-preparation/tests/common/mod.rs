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
