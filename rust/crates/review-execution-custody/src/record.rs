use crate::{ExecutionBinding, ExecutionProvenance};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredRequest {
    pub schema_version: u32,
    pub root_id: String,
    pub campaign_root_id: String,
    pub obligation_id: String,
    pub candidate_digest: String,
    pub selection_digest: String,
    pub profile_digest: String,
    pub prepared_request_sha256: String,
    pub review_episode_id: String,
    pub attempt_id: String,
    pub dispatch_operation_id: String,
    pub dispatch_revision: String,
    pub execution_session_id: String,
    pub selection_id: String,
    pub candidate_commit: String,
    pub candidate_tree: String,
    pub candidate_patch_identity: String,
    pub request_raw_sha256: String,
    pub request_claim_sha256: String,
    pub executable_sha256: String,
    pub executable: String,
    pub arguments: Vec<String>,
    pub workspace: String,
    pub source_sha256: String,
}

impl StoredRequest {
    pub fn provenance(&self) -> ExecutionProvenance {
        ExecutionProvenance {
            binding: ExecutionBinding {
                campaign_root_id: self.campaign_root_id.clone(),
                obligation_id: self.obligation_id.clone(),
                candidate_digest: self.candidate_digest.clone(),
                selection_digest: self.selection_digest.clone(),
                profile_digest: self.profile_digest.clone(),
                execution_profile_digest: None,
                prepared_request_sha256: self.prepared_request_sha256.clone(),
                review_episode_id: self.review_episode_id.clone(),
                attempt_id: self.attempt_id.clone(),
            },
            dispatch_operation_id: self.dispatch_operation_id.clone(),
            dispatch_revision: self.dispatch_revision.clone(),
            execution_session_id: Some(self.execution_session_id.clone()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedRecord {
    pub pid: u32,
    pub exit_code: i32,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalRecord {
    pub schema_version: u32,
    pub raw_result_sha256: String,
    pub claim_result_sha256: String,
    pub observation_sha256: String,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub transport_sha256: String,
    pub session_id: String,
    pub pid: u32,
    pub exit_code: i32,
    pub artifact_references: Vec<String>,
}
