//! Identity rules owned by slice-campaign, published so that consumers outside the campaign can
//! apply the campaign's own rules instead of restating them.
//!
//! Owner: slice-campaign. This crate holds the rules slice-campaign mints identities and
//! consumption consumers with; slice-campaign re-exports them and calls them itself, so one rule
//! exists, not a copy. Owner sign-off for extracting them: operator decision O-2, 2026-11-18.
//!
//! It is light by design: its dependencies provide the episode codec, JSON serialization and
//! error types; no store, I/O or authority is involved.

use review_episode_core::identity::Identity;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// A refusal of a campaign identity rule; the text is the campaign's contract message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct IdentityError(pub String);

/// The identity of one campaign attempt: the run, the slice, the attempt and the plan version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CampaignIdentity {
    pub run_id: String,
    pub slice_number: u64,
    pub attempt_id: String,
    pub plan_version: String,
}

impl CampaignIdentity {
    pub fn validate(&self) -> Result<(), IdentityError> {
        for (name, value) in [
            ("runId", &self.run_id),
            ("attemptId", &self.attempt_id),
            ("planVersion", &self.plan_version),
        ] {
            if value.trim().is_empty() {
                return Err(IdentityError(format!("{name} must be nonempty")));
            }
            if value.contains(':') {
                return Err(IdentityError(format!("{name} contains identity separator")));
            }
        }
        if self.slice_number == 0 || self.slice_number > 9_007_199_254_740_991 {
            return Err(IdentityError(
                "sliceNumber must be a positive JavaScript safe integer".into(),
            ));
        }
        Ok(())
    }

    pub fn key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.run_id, self.slice_number, self.attempt_id, self.plan_version
        )
    }
}

/// The episode codec's digest (`review_episode_core::codec::digest`, sha256 of canonical JSON,
/// lowercase hex) of a JSON value.
pub fn episode_digest(value: &Value) -> Result<String, IdentityError> {
    let raw = serde_json::to_string(value).map_err(|e| IdentityError(e.to_string()))?;
    let episode =
        review_episode_core::codec::parse_json(&raw).map_err(|e| IdentityError(e.to_string()))?;
    Ok(review_episode_core::codec::digest(&episode))
}

/// The review episode id the campaign mints for one obligation of one campaign identity.
///
/// Owner definition (slice-campaign): `reviewEpisodeId := episode_digest({"identity":
/// <CampaignIdentity as camelCase JSON>, "obligationId": <obligation id>})[..32]`, the first 32
/// lowercase hex characters of the episode codec digest.
pub fn episode_id(
    identity: &CampaignIdentity,
    obligation_id: &str,
) -> Result<String, IdentityError> {
    let digest = episode_digest(&json!({"identity": identity, "obligationId": obligation_id}))?;
    Ok(digest[..32].to_owned())
}

/// The problem scope an episode must lie within: the terms of an organization candidate's
/// `problemScope`, borrowed so this crate needs no organization types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProblemScopeTerms<'a> {
    pub run_id: &'a str,
    pub plan_lineage: &'a str,
    pub slice_number: u64,
}

/// The term of [`episode_within_scope`] that refused, in evaluation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScopeTerm {
    /// The episode's run id differs from the scope's.
    RunId,
    /// The episode's slice number differs from the scope's.
    SliceNumber,
    /// The episode's plan version differs from the scope's plan lineage (D3 (i)).
    PlanLineage,
    /// The episode's review obligation differs from the obligation being consumed.
    ObligationId,
    /// The episode's run, slice, attempt and plan version are not a valid campaign identity.
    CampaignIdentity,
    /// The episode's review episode id is not the one the campaign mints for its identity.
    EpisodeId,
}

/// An episode outside the problem scope: the first refusing term and what differed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("episode outside the problem scope ({term:?}): {detail}")]
pub struct OutsideScope {
    pub term: ScopeTerm,
    pub detail: String,
}

/// The episode-within-scope predicate.
///
/// Owner definition (slice-campaign): a review episode with identity `episode` lies within
/// problem scope `scope` for obligation `obligation_id` exactly when
/// 1. `episode.runId == scope.runId`;
/// 2. `episode.sliceNumber == scope.sliceNumber`;
/// 3. `episode.planVersion == scope.planLineage` (D3 (i): plan lineage is compared by equality
///    with the plan version);
/// 4. `episode.reviewObligationId == obligation_id`;
/// 5. `{runId, sliceNumber, attemptId, planVersion}` of the episode is a valid
///    [`CampaignIdentity`]; and
/// 6. `episode.reviewEpisodeId == episode_id(that identity, obligation_id)`.
///
/// The terms are evaluated in that order and the first refusing one is returned. On success the
/// campaign identity the episode was minted for is returned.
pub fn episode_within_scope(
    episode: &Identity,
    obligation_id: &str,
    scope: &ProblemScopeTerms<'_>,
) -> Result<CampaignIdentity, OutsideScope> {
    let text = |term: ScopeTerm, value: &review_episode_core::codec::JsString| {
        value.to_string_checked().map_err(|e| OutsideScope {
            term,
            detail: e.to_string(),
        })
    };
    let differs = |term: ScopeTerm, episode: &str, scope: &str| {
        Err(OutsideScope {
            term,
            detail: format!("episode `{episode}`, expected `{scope}`"),
        })
    };
    let run_id = text(ScopeTerm::RunId, &episode.run_id)?;
    if run_id != scope.run_id {
        return differs(ScopeTerm::RunId, &run_id, scope.run_id);
    }
    if episode.slice_number != scope.slice_number {
        return differs(
            ScopeTerm::SliceNumber,
            &episode.slice_number.to_string(),
            &scope.slice_number.to_string(),
        );
    }
    let plan_version = text(ScopeTerm::PlanLineage, &episode.plan_version)?;
    if plan_version != scope.plan_lineage {
        return differs(ScopeTerm::PlanLineage, &plan_version, scope.plan_lineage);
    }
    let review_obligation_id = text(ScopeTerm::ObligationId, &episode.review_obligation_id)?;
    if review_obligation_id != obligation_id {
        return differs(
            ScopeTerm::ObligationId,
            &review_obligation_id,
            obligation_id,
        );
    }
    let identity = CampaignIdentity {
        run_id,
        slice_number: episode.slice_number,
        attempt_id: text(ScopeTerm::CampaignIdentity, &episode.attempt_id)?,
        plan_version,
    };
    identity.validate().map_err(|e| OutsideScope {
        term: ScopeTerm::CampaignIdentity,
        detail: e.0,
    })?;
    let expected = episode_id(&identity, obligation_id).map_err(|e| OutsideScope {
        term: ScopeTerm::EpisodeId,
        detail: e.0,
    })?;
    let review_episode_id = text(ScopeTerm::EpisodeId, &episode.review_episode_id)?;
    if review_episode_id != expected {
        return differs(ScopeTerm::EpisodeId, &review_episode_id, &expected);
    }
    Ok(identity)
}

/// The consumption boundaries the campaign supports, each with the consumer class that consumes
/// at it and the prefix of its consumer string. One table: [`expected_consumer`] and
/// [`expected_admission`] both read it.
const CONSUMPTION_BOUNDARIES: [(&str, &str, &str); 2] = [
    ("builder_projection", "builder", "slice-builder"),
    ("campaign_terminalization", "campaign", "slice-campaign"),
];

/// Why no consumer can be derived for a boundary.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConsumptionMappingError {
    #[error("consumption boundary `{0}` is unsupported")]
    UnsupportedBoundary(String),
    #[error("consumer class `{class}` does not consume at boundary `{boundary}`")]
    ClassCrossesBoundary { boundary: String, class: String },
}

/// The consumer string evidence for `boundary` must name for campaign `identity`.
///
/// Owner definition (slice-campaign): `builder_projection` → `slice-builder:<identity.key()>`,
/// `campaign_terminalization` → `slice-campaign:<identity.key()>`; every other boundary is
/// unsupported.
pub fn expected_consumer(
    boundary: &str,
    identity: &CampaignIdentity,
) -> Result<String, ConsumptionMappingError> {
    let (_, _, prefix) = CONSUMPTION_BOUNDARIES
        .iter()
        .find(|(supported, _, _)| *supported == boundary)
        .ok_or_else(|| ConsumptionMappingError::UnsupportedBoundary(boundary.to_owned()))?;
    Ok(format!("{prefix}:{}", identity.key()))
}

/// The `(boundary, consumer)` an episode evidence admission must carry for one condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedAdmission {
    pub boundary: &'static str,
    pub consumer: String,
}

/// The consumption-evidence mapping.
///
/// Owner definition (slice-campaign): a condition with consumption boundary `boundary` and
/// consumer class `consumer_class` is satisfied, for campaign `identity`, only by an evidence
/// admission whose boundary is `boundary` and whose consumer is
/// [`expected_consumer`]`(boundary, identity)`, where `builder_projection` is consumed by class
/// `builder` and `campaign_terminalization` by class `campaign`. A class that does not consume
/// at the boundary, or an unsupported boundary, has no admission.
pub fn expected_admission(
    boundary: &str,
    consumer_class: &str,
    identity: &CampaignIdentity,
) -> Result<ExpectedAdmission, ConsumptionMappingError> {
    let (supported, class, _) = CONSUMPTION_BOUNDARIES
        .iter()
        .find(|(supported, _, _)| *supported == boundary)
        .ok_or_else(|| ConsumptionMappingError::UnsupportedBoundary(boundary.to_owned()))?;
    if *class != consumer_class {
        return Err(ConsumptionMappingError::ClassCrossesBoundary {
            boundary: boundary.to_owned(),
            class: consumer_class.to_owned(),
        });
    }
    Ok(ExpectedAdmission {
        boundary: supported,
        consumer: expected_consumer(boundary, identity)?,
    })
}
