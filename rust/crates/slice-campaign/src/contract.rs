use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeSet;

use crate::codec::campaign_digest;
use crate::completion::CampaignProgress;
use crate::{CampaignError, Result, require_sha, require_text};

pub const PROFILE: &str = "campaign-native-initial-v3";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CampaignIdentity {
    pub run_id: String,
    pub slice_number: u64,
    pub attempt_id: String,
    pub plan_version: String,
}

impl CampaignIdentity {
    pub fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("runId", &self.run_id),
            ("attemptId", &self.attempt_id),
            ("planVersion", &self.plan_version),
        ] {
            require_text(value, name)?;
            if value.contains(':') {
                return Err(CampaignError::Contract(format!(
                    "{name} contains identity separator"
                )));
            }
        }
        if self.slice_number == 0 || self.slice_number > 9_007_199_254_740_991 {
            return Err(CampaignError::Contract(
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptedBoundary {
    pub reference: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Baseline {
    pub accepted_commit: String,
    pub accepted_tree: String,
    pub inter_slice_commit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignRevision(String);

impl CampaignRevision {
    pub fn new(value: String) -> Result<Self> {
        require_sha(&value, "campaign revision")?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CampaignRef {
    pub root_id: String,
    pub identity: CampaignIdentity,
    pub revision: CampaignRevision,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CandidateRef {
    pub commit: String,
    pub tree: String,
    pub patch_identity: String,
    pub receipt_sha256: String,
    pub physical_profile_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionRef {
    pub selection_id: String,
    pub campaign_digest: String,
    pub episode_authority_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeReviewRequestRef {
    pub root_id: String,
    pub identity: CampaignIdentity,
    pub obligation_id: String,
    pub operation_id: String,
    pub kind: String,
    pub prepared_revision: CampaignRevision,
    pub request_digest: String,
    pub selection: SelectionRef,
    pub candidate: CandidateRef,
    pub profile_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvancePhase {
    Implementing,
    GateReady,
    ReviewReady,
}

impl AdvancePhase {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Implementing => "implementing",
            Self::GateReady => "gate_ready",
            Self::ReviewReady => "review_ready",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Consequence {
    pub owner: String,
    pub reference: String,
    pub sha256: String,
}

impl Consequence {
    pub fn validate(&self) -> Result<()> {
        require_text(&self.owner, "consequence owner")?;
        require_text(&self.reference, "consequence reference")?;
        require_sha(&self.sha256, "consequence digest")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub identity: CampaignIdentity,
    pub workspace: String,
    pub repository: String,
    pub accepted_boundary: AcceptedBoundary,
    pub baseline: Baseline,
    pub expected_impact: Option<Consequence>,
    pub phase: String,
    pub latest_consequence: Option<Consequence>,
    pub candidate: Option<CandidateRef>,
    pub candidate_receipt: Option<Value>,
    pub physical_profile: Option<Value>,
    pub gate_capture_digest: Option<String>,
    pub review_selection: Option<Value>,
    pub selection_ref: Option<SelectionRef>,
    pub obligations: Vec<Obligation>,
    #[serde(default)]
    pub progress: CampaignProgress,
    pub revision: CampaignRevision,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Obligation {
    pub obligation_id: String,
    pub skill: String,
    pub status: String,
    pub request: Option<NativeReviewRequestRef>,
}

fn exact(value: &Value, fields: &[&str], name: &str) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| CampaignError::Contract(format!("{name} must be object")))?;
    let keys: BTreeSet<_> = object.keys().map(String::as_str).collect();
    if keys != fields.iter().copied().collect() {
        return Err(CampaignError::Contract(format!("{name} fields differ")));
    }
    Ok(())
}
fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| CampaignError::Contract(format!("{field} must be nonempty text")))
}
fn claim_digest(value: &Value) -> Result<String> {
    let raw = serde_json::to_string(value).map_err(|e| CampaignError::Contract(e.to_string()))?;
    let claim = claim_evidence::codec::parse_json(&raw)
        .map_err(|e| CampaignError::Contract(e.to_string()))?;
    claim_evidence::codec::digest(&claim).map_err(|e| CampaignError::Contract(e.to_string()))
}
pub fn episode_digest(value: &Value) -> Result<String> {
    let raw = serde_json::to_string(value).map_err(|e| CampaignError::Contract(e.to_string()))?;
    let episode = review_episode_core::codec::parse_json(&raw)
        .map_err(|e| CampaignError::Contract(e.to_string()))?;
    Ok(review_episode_core::codec::digest(&episode))
}

/// Campaign-owned selection validation; the historical claim-evidence codec
/// supplies identity bytes, while the selected obligation owns the documents.
pub fn validate_selection(
    selection: &Value,
    identity: &CampaignIdentity,
    candidate: &CandidateRef,
) -> Result<SelectionRef> {
    exact(
        selection,
        &[
            "schemaVersion",
            "owner",
            "selectionId",
            "subject",
            "specialists",
        ],
        "selection",
    )?;
    if selection["schemaVersion"] != 2 || selection["owner"] != "slice-supervisor" {
        return Err(CampaignError::Contract(
            "selection schema or owner unsupported".into(),
        ));
    }
    let selection_id = text(selection, "selectionId")?;
    let subject = json!({"commit":candidate.commit,"tree":candidate.tree,"patchIdentity":candidate.patch_identity});
    if campaign_digest(&selection["subject"])? != campaign_digest(&subject)? {
        return Err(CampaignError::Contract("selection subject differs".into()));
    }
    let specialists = selection["specialists"]
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| CampaignError::Contract("specialists absent".into()))?;
    let mut obligations = BTreeSet::new();
    let mut claim_ids = BTreeSet::new();
    let identity_key = identity.key();
    for specialist in specialists {
        exact(
            specialist,
            &["obligationId", "skill", "selection", "requiredClaims"],
            "specialist",
        )?;
        let obligation = text(specialist, "obligationId")?;
        if !obligations.insert(obligation.to_owned()) {
            return Err(CampaignError::Contract("duplicate obligation".into()));
        }
        let skill = text(specialist, "skill")?;
        if !["implementation-review", "claude-recon-implementation"].contains(&skill) {
            return Err(CampaignError::Unsupported(
                "specialist skill outside initial profile".into(),
            ));
        }
        let disposition = text(specialist, "selection")?;
        let claims = specialist["requiredClaims"]
            .as_array()
            .ok_or_else(|| CampaignError::Contract("required claims absent".into()))?;
        match disposition {
            "omitted" if claims.is_empty() => {}
            "selected" if claims.len() == 2 => {
                let mut boundaries = BTreeSet::new();
                for claim in claims {
                    validate_selected_claim(claim, &subject, identity, obligation)?;
                    let boundary = text(claim, "consumptionBoundary")?;
                    boundaries.insert(boundary.to_owned());
                    if !claim_ids.insert(text(claim, "claimId")?.to_owned()) {
                        return Err(CampaignError::Contract(
                            "duplicate required claim ID".into(),
                        ));
                    }
                }
                if boundaries
                    != BTreeSet::from([
                        "builder_projection".to_owned(),
                        "campaign_terminalization".to_owned(),
                    ])
                {
                    return Err(CampaignError::Contract(
                        "required claim boundaries incomplete".into(),
                    ));
                }
            }
            _ => {
                return Err(CampaignError::Contract(
                    "selection disposition/claim pair invalid".into(),
                ));
            }
        }
    }
    if !specialists.iter().any(|s| s["selection"] == "selected") {
        return Err(CampaignError::Contract("no selected obligation".into()));
    }
    let _ = identity_key;
    Ok(SelectionRef {
        selection_id: selection_id.into(),
        campaign_digest: campaign_digest(selection)?,
        episode_authority_digest: episode_digest(selection)?,
    })
}

fn validate_selected_claim(
    claim: &Value,
    expected_candidate: &Value,
    identity: &CampaignIdentity,
    obligation_id: &str,
) -> Result<()> {
    exact(
        claim,
        &[
            "schemaVersion",
            "claimId",
            "revision",
            "proposition",
            "subject",
            "coveredState",
            "consumptionBoundary",
            "consumer",
            "acceptance",
            "profile",
        ],
        "required claim",
    )?;
    if claim["schemaVersion"] != 1 {
        return Err(CampaignError::Contract("claim schema unsupported".into()));
    }
    for field in [
        "claimId",
        "revision",
        "proposition",
        "coveredState",
        "consumptionBoundary",
        "consumer",
    ] {
        text(claim, field)?;
    }
    exact(
        &claim["subject"],
        &["candidate", "reviewEpisodeId"],
        "claim subject",
    )?;
    exact(
        &claim["subject"]["candidate"],
        &["commit", "tree", "patchIdentity"],
        "claim candidate",
    )?;
    if campaign_digest(&claim["subject"]["candidate"])? != campaign_digest(expected_candidate)? {
        return Err(CampaignError::Contract("claim candidate differs".into()));
    }
    let episode_id = episode_digest(&json!({"identity":identity,"obligationId":obligation_id}))?;
    if claim["subject"]["reviewEpisodeId"] != episode_id[..32] {
        return Err(CampaignError::Contract(
            "claim episode identity differs".into(),
        ));
    }
    let boundary = text(claim, "consumptionBoundary")?;
    let expected_consumer = match boundary {
        "builder_projection" => format!("slice-builder:{}", identity.key()),
        "campaign_terminalization" => format!("slice-campaign:{}", identity.key()),
        _ => return Err(CampaignError::Contract("claim boundary unsupported".into())),
    };
    if claim["consumer"] != expected_consumer {
        return Err(CampaignError::Contract("claim consumer differs".into()));
    }
    exact(
        &claim["acceptance"],
        &["owner", "source", "unestablishedRoute"],
        "claim acceptance",
    )?;
    let owner = text(&claim["acceptance"], "owner")?;
    if ["reviewer", "builder", "adapter", "terminalizer"].contains(&owner) {
        return Err(CampaignError::Contract("claim self-authorized".into()));
    }
    text(&claim["acceptance"], "source")?;
    text(&claim["acceptance"], "unestablishedRoute")?;
    exact(
        &claim["profile"],
        &[
            "id",
            "revision",
            "allowedMechanisms",
            "admissibleObservers",
            "integrityRequired",
            "requiredRealization",
            "requiredCapabilities",
            "continuity",
        ],
        "claim profile",
    )?;
    if claim["profile"]["id"] != "production-path-v1"
        || claim["profile"]["revision"] != "production-path-profile-v1"
        || !claim["profile"]["integrityRequired"].is_boolean()
        || !["fresh_initial", "retained"]
            .iter()
            .any(|s| claim["profile"]["continuity"] == *s)
    {
        return Err(CampaignError::Contract("claim profile unsupported".into()));
    }
    for field in [
        "allowedMechanisms",
        "admissibleObservers",
        "requiredCapabilities",
    ] {
        let entries = claim["profile"][field]
            .as_array()
            .ok_or_else(|| CampaignError::Contract(format!("{field} invalid")))?;
        if field != "requiredCapabilities" && entries.is_empty() {
            return Err(CampaignError::Contract(format!("{field} empty")));
        }
        let mut unique = BTreeSet::new();
        for entry in entries {
            let value = entry
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| CampaignError::Contract(format!("{field} text invalid")))?;
            if !unique.insert(value) {
                return Err(CampaignError::Contract(format!("{field} duplicate")));
            }
        }
    }
    text(&claim["profile"], "requiredRealization")?;
    let claim_id_body = json!({"proposition":claim["proposition"],"subject":claim["subject"],"coveredState":claim["coveredState"],"consumptionBoundary":claim["consumptionBoundary"],"consumer":claim["consumer"]});
    let expected_id = format!("production-path-claim-v1@{}", claim_digest(&claim_id_body)?);
    let mut revision_body = claim.clone();
    revision_body.as_object_mut().unwrap().remove("revision");
    let expected_revision = format!(
        "production-path-claim-revision-v1@{}",
        claim_digest(&revision_body)?
    );
    if claim["claimId"] != expected_id || claim["revision"] != expected_revision {
        return Err(CampaignError::Contract("claim identity differs".into()));
    }
    Ok(())
}
