//! Public, bounded read projection for one admitted historical S5 investigation.
use serde::{Deserialize, Serialize};

pub const MAX_FRAME: usize = 131_072;
pub const OPERATION: &str = "historical_s5.inspect.v1";
pub const RUN_ID: &str = "skills-migration-app-server-20260828";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub operation: String,
    pub candidate_attempt: Option<u8>,
    pub review_key: Option<String>,
}
impl Request {
    pub fn all() -> Self {
        Self {
            operation: OPERATION.into(),
            candidate_attempt: None,
            review_key: None,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.operation != OPERATION
            || self
                .candidate_attempt
                .is_some_and(|n| !(1..=5).contains(&n))
            || self.review_key.as_ref().is_some_and(|key| {
                !key.starts_with("independent-review-episode-v1/") || key.len() > 128
            })
        {
            return Err("request is outside the fixed historical operation");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRead {
    pub owner: String,
    pub identity: String,
    pub integrity: String,
    pub completeness: String,
    pub confidentiality: String,
    pub limit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateCheck {
    pub identity: String,
    pub outcome: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    pub source_kind: String,
    pub receipt_sha256: String,
    pub capture_sha256: String,
    pub corroborating_output_sha256: Option<String>,
    pub status: String,
    pub check_count: usize,
    pub passed_count: usize,
    pub checks: Vec<GateCheck>,
    pub limit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub kind: String,
    pub attempt: u8,
    pub checkpoint_ref: String,
    pub commit_oid: String,
    pub tree_oid: String,
    pub parent_oid: String,
    pub gate_digest: String,
    pub gate: Option<Gate>,
    pub integrity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub owner: String,
    pub reference: String,
    pub revision: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub status: String,
    pub severity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewRevision {
    pub revision: String,
    pub predecessor_revision: Option<String>,
    pub payload_sha256: String,
    pub status: String,
    pub phase: String,
    pub subjects: Vec<Subject>,
    pub findings: Vec<Finding>,
    pub authority_reference: String,
    pub authority_manifest_sha256: String,
    pub historical_grant_id: String,
    pub writer_generation: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewEpisode {
    pub label: String,
    pub key: String,
    pub head: String,
    pub revisions: Vec<ReviewRevision>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplacementLink {
    pub replacement_key: String,
    pub replacement_revision: String,
    pub original_key: String,
    pub original_reported_revision: String,
    pub authority_reference: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub protocol_version: u8,
    pub projection_version: u8,
    pub operation: String,
    pub run_id: String,
    pub slice_number: u8,
    pub grant_id: String,
    pub cutoff: String,
    pub read_at: String,
    pub source_identity_sha256: String,
    pub candidates: Vec<Candidate>,
    pub reviews: Vec<ReviewEpisode>,
    pub replacement_links: Vec<ReplacementLink>,
    pub source_reads: Vec<SourceRead>,
    pub gaps: Vec<String>,
}
impl Observation {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.protocol_version != 1
            || self.projection_version != 1
            || self.operation != OPERATION
            || self.run_id != RUN_ID
            || self.slice_number != 6
            || self.candidates.len() != 6
            || self.reviews.len() != 3
            || self.grant_id.is_empty()
            || self.source_identity_sha256.len() != 64
        {
            return Err("observation identity or version is invalid");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reply {
    Observed { observation: Observation },
    Refused { reason: String },
    Unavailable { reason: String },
}
