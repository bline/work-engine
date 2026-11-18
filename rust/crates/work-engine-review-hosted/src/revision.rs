//! The requirements revisions, the closed set of provider profiles with their realization claims,
//! and preparation with refusal.

use serde::{Deserialize, Serialize};
use work_engine_organization::{
    OrganizationCandidate, OrganizationalRequirements, Profile as CompilerProfile, derive,
};
use work_engine_review_delivery::{FrozenDelivery, Transport};
use work_engine_review_preparation::{
    BlockerClass, MutationClaim, PreparationRequest, RealizationClaim, ReviewPreparation, Subject,
    prepare,
};

use crate::{Refusal, sha256_hex};

/// Revision r1 (operator decision D2/OQ-1 (D), 2026-11-17): both realization-class constraints
/// removed. Used by the `openai-codex` profile (see its `DERIVATION.md`).
pub const REVISION_ID: &str = "hosted-route-qualification-r1";
/// Revision r1's requirements bytes.
pub const REVISION_REQUIREMENTS: &[u8] =
    include_bytes!("../fixtures/hosted-route-qualification-r1/requirements.json");
/// Revision r2 (operator decision D, 2026-11-18): keeps `provider_family: anthropic`, removes only
/// `execution_confinement`. Used by the `claude-sdk` profile (see its `DERIVATION.md`).
pub const REVISION_R2_ID: &str = "hosted-route-qualification-r2";
/// Revision r2's requirements bytes.
pub const REVISION_R2_REQUIREMENTS: &[u8] =
    include_bytes!("../fixtures/hosted-route-qualification-r2/requirements.json");
/// The frozen requirements, unchanged: the reported negative world on this route.
pub const FROZEN_REQUIREMENTS: &[u8] =
    include_bytes!("../../work-engine-organization/tests/fixtures/frozen/requirements.json");
/// The one obligation every world carries.
pub const OBLIGATION: &str = "rp3-initial-implementation-review";

/// The `openai-codex` profile's provider (kept for the r1 callers): the catalog entry must name it
/// and every provider request frame must carry the plan's provider
/// (`work-engine-manual-host/src/pi_coding_provider.rs:466-487`, `:857`).
pub const PROVIDER: &str = "openai-codex";
/// The `openai-codex` profile's only model (`pi_coding_provider.rs:545`). Requested, not observed.
pub const REQUESTED_MODEL: &str = "gpt-6-luna";
/// The profile's tools, in the receiver's registration order
/// (`work-engine-manual-host/src/pi_coding_analysis.rs:9`).
pub const HOSTED_TOOLS: [&str; 2] = ["snapshot_list", "snapshot_read"];
/// The `claude-sdk` profile's only model (`pi_coding_provider.rs:12`, admitted at `:542-543`).
pub const CLAUDE_MODEL: &str = "claude-haiku-5-5";
/// The tools a `claude-sdk` request declares, sorted: the profile's tools as `mcp__pi__<name>`
/// plus Claude Code's `DeferredToolPlaceholder`, exactly the set the Host pins
/// (`pi_coding_analysis.rs:119-121`, `:137-146`).
pub const CLAUDE_TOOLS: [&str; 3] = [
    "DeferredToolPlaceholder",
    "mcp__pi__snapshot_list",
    "mcp__pi__snapshot_read",
];

/// One requirements revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revision {
    pub id: &'static str,
    pub requirements: &'static [u8],
}

/// The request wire family of a profile (`pi_coding_provider.rs:452-457`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    Responses,
    Messages,
}

/// The closed set of provider profiles. No other provider or model is accepted; the profile is
/// chosen explicitly and recorded in the job-inputs record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    /// Pi's `openai-codex` provider, `gpt-6-luna`, Responses family, revision r1.
    OpenaiCodex,
    /// Pi's `claude-sdk` provider (Claude Code in the container), `claude-haiku-5-5`, Anthropic
    /// Messages family, revision r2.
    ClaudeSdk,
}

impl Profile {
    pub const ALL: [Profile; 2] = [Profile::OpenaiCodex, Profile::ClaudeSdk];

    /// The profile named exactly `name`.
    pub fn parse(name: &str) -> Option<Profile> {
        Self::ALL.into_iter().find(|p| p.provider() == name)
    }
    /// The Pi provider id the Host requires in the plan, the admission and every provider request
    /// frame (`pi_coding_provider.rs:38-45`, `:619`, `:857`). Not the serving endpoint.
    pub fn provider(self) -> &'static str {
        match self {
            Profile::OpenaiCodex => PROVIDER,
            Profile::ClaudeSdk => "claude-sdk",
        }
    }
    /// The one model the Host admits for the profile (`pi_coding_provider.rs:542-546`).
    pub fn model(self) -> &'static str {
        match self {
            Profile::OpenaiCodex => REQUESTED_MODEL,
            Profile::ClaudeSdk => CLAUDE_MODEL,
        }
    }
    pub fn wire(self) -> Wire {
        match self {
            Profile::OpenaiCodex => Wire::Responses,
            Profile::ClaudeSdk => Wire::Messages,
        }
    }
    /// The revision the profile runs on: r1 for `openai-codex` (unchanged), r2 for `claude-sdk`.
    pub fn revision(self) -> Revision {
        match self {
            Profile::OpenaiCodex => Revision {
                id: REVISION_ID,
                requirements: REVISION_REQUIREMENTS,
            },
            Profile::ClaudeSdk => Revision {
                id: REVISION_R2_ID,
                requirements: REVISION_R2_REQUIREMENTS,
            },
        }
    }
    /// Tool names exactly as a request body declares them, sorted.
    pub fn request_tools(self) -> Vec<String> {
        let mut names: Vec<String> = match self {
            Profile::OpenaiCodex => HOSTED_TOOLS.iter(),
            Profile::ClaudeSdk => CLAUDE_TOOLS.iter(),
        }
        .map(|t| (*t).to_owned())
        .collect();
        names.sort();
        names
    }
    /// The profile's realization claim: what the Host enforces, stated as a claim (P1-2(b)).
    ///
    /// Confinement is unstated: no owner maps the hosted container to a confinement class.
    /// Read-only is claimed and tools are named, so `prepare` leaves mutation and capabilities
    /// unestablished (`work-engine-review-preparation/src/realization.rs:134-138`, `:166-176`).
    /// Continuity is a fresh native session per job.
    ///
    /// * `openai-codex`: provider family `openai-codex` (the catalog entry's provider,
    ///   `pi_coding_provider.rs:466-471`); the two Host-served tools.
    /// * `claude-sdk`: provider family `anthropic`: the Host requires the catalog entry of
    ///   `anthropic-messages` with provider `anthropic`, API `anthropic-messages` and base URL
    ///   `https://api.anthropic.com` (`pi_coding_provider.rs:473-487`), and its hosted routes serve
    ///   the Messages wire only from Anthropic origins (`pi_coding_hosted.rs:184-189`, `:76-89`).
    ///   The granted tools are the names the Host pins in the request, verbatim.
    pub fn claim(self) -> RealizationClaim {
        let (family, tools): (&str, Vec<String>) = match self {
            Profile::OpenaiCodex => (
                PROVIDER,
                HOSTED_TOOLS.iter().map(|t| (*t).to_owned()).collect(),
            ),
            Profile::ClaudeSdk => ("anthropic", self.request_tools()),
        };
        RealizationClaim {
            provider_family: Some(family.to_owned()),
            execution_confinement: None,
            mutation: MutationClaim::ReadOnlyClaimed,
            granted_tools: tools,
            continuity: Some("fresh_initial".to_owned()),
        }
    }
}

/// The `openai-codex` profile's realization claim (the r1 route).
pub fn hosted_claim() -> RealizationClaim {
    Profile::OpenaiCodex.claim()
}

/// The one candidate the unchanged v1 compiler derives from `requirements`.
pub fn candidate(requirements: &[u8]) -> Result<OrganizationCandidate, Refusal> {
    let requirements = OrganizationalRequirements::from_json_bytes(requirements)
        .map_err(|e| Refusal::Requirements(format!("{e:?}")))?;
    let mut derived = derive(&requirements, CompilerProfile::FixedSliceReviewV1);
    if !derived.unresolved.is_empty() || derived.candidates.len() != 1 {
        return Err(Refusal::Requirements(format!(
            "derivation gave {} candidates and {} unresolved",
            derived.candidates.len(),
            derived.unresolved.len()
        )));
    }
    Ok(derived.candidates.remove(0))
}

/// The preparation request: the obligation, the exact subject and the profile's claim.
pub fn request(profile: Profile, subject: &Subject) -> PreparationRequest {
    PreparationRequest {
        obligation_id: OBLIGATION.to_owned(),
        subject: subject.clone(),
        realization: Some(profile.claim()),
    }
}

/// Ids of every blocker of class `KnownIncompatibility`.
pub fn known_incompatibilities(preparation: &ReviewPreparation) -> Vec<String> {
    preparation
        .blockers
        .iter()
        .filter(|b| b.class == BlockerClass::KnownIncompatibility)
        .map(|b| b.id.clone())
        .collect()
}

/// Candidate → public `prepare` with the profile's claim.
pub fn preparation(
    profile: Profile,
    requirements: &[u8],
    subject: &Subject,
) -> Result<ReviewPreparation, Refusal> {
    prepare(&candidate(requirements)?, &request(profile, subject))
        .map_err(|e| Refusal::Preparation(format!("{e:?}")))
}

/// Candidate → `prepare` → refusal on known incompatibility → `FrozenDelivery::freeze` with
/// transport `{attempt_id: job_id, generation: 1}`.
pub fn prepare_world_for(
    profile: Profile,
    requirements: &[u8],
    subject: &Subject,
    job_id: &str,
) -> Result<FrozenDelivery, Refusal> {
    let preparation = preparation(profile, requirements, subject)?;
    let known = known_incompatibilities(&preparation);
    if !known.is_empty() {
        return Err(Refusal::KnownIncompatibility(known));
    }
    FrozenDelivery::freeze(
        preparation,
        Transport {
            attempt_id: job_id.to_owned(),
            generation: 1,
        },
    )
    .map_err(|e| Refusal::Freeze(e.to_string()))
}

/// [`prepare_world_for`] on the `openai-codex` profile.
pub fn prepare_world(
    requirements: &[u8],
    subject: &Subject,
    job_id: &str,
) -> Result<FrozenDelivery, Refusal> {
    prepare_world_for(Profile::OpenaiCodex, requirements, subject, job_id)
}

/// The frozen world's outcome on this route, retained as an artifact. It never yields job inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenWorld {
    pub requirements_sha256: String,
    /// `refused` (a known realization incompatibility) or `prepared_conditions_unmet` (no known
    /// incompatibility; execution confinement unestablished, so the conditions stay unmet).
    pub outcome: String,
    pub known_incompatibilities: Vec<String>,
    /// Ids of the `realization:*:realization:execution_confinement` blockers of class
    /// `Unestablished` (only when `outcome` is `prepared_conditions_unmet`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unestablished_confinement: Vec<String>,
}

const CONFINEMENT_SUFFIX: &str = ":realization:execution_confinement";

/// Prepares (never freezes) the frozen requirements with the profile's claim.
///
/// * `openai-codex`: the provider family is known incompatible; the outcome is `refused`.
/// * `claude-sdk`: the claimed family `anthropic` is allowed, so nothing is known incompatible;
///   the confinement constraint is unestablished (the claim states none), so every condition stays
///   unmet and the outcome is `prepared_conditions_unmet`.
///
/// A frozen world with neither a known incompatibility nor an unestablished confinement would
/// contradict the route's premise and is itself refused.
pub fn frozen_world_for(profile: Profile, subject: &Subject) -> Result<FrozenWorld, Refusal> {
    let preparation = preparation(profile, FROZEN_REQUIREMENTS, subject)?;
    let known = known_incompatibilities(&preparation);
    let requirements_sha256 = sha256_hex(FROZEN_REQUIREMENTS);
    if !known.is_empty() {
        return Ok(FrozenWorld {
            requirements_sha256,
            outcome: "refused".to_owned(),
            known_incompatibilities: known,
            unestablished_confinement: Vec::new(),
        });
    }
    let confinement: Vec<String> = preparation
        .blockers
        .iter()
        .filter(|b| b.class == BlockerClass::Unestablished && b.id.ends_with(CONFINEMENT_SUFFIX))
        .map(|b| b.id.clone())
        .collect();
    if confinement.is_empty() {
        return Err(Refusal::Requirements(
            "the frozen world prepared with no known incompatibility and no unestablished confinement"
                .to_owned(),
        ));
    }
    Ok(FrozenWorld {
        requirements_sha256,
        outcome: "prepared_conditions_unmet".to_owned(),
        known_incompatibilities: Vec::new(),
        unestablished_confinement: confinement,
    })
}

/// [`frozen_world_for`] on the `openai-codex` profile (`job_id` is unused: the frozen world is
/// prepared, never frozen).
pub fn frozen_world(subject: &Subject, _job_id: &str) -> Result<FrozenWorld, Refusal> {
    frozen_world_for(Profile::OpenaiCodex, subject)
}
