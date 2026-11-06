use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const PROFILE: &str = "canonical-publication-local-v1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordRef {
    pub kind: String,
    pub schema_version: u32,
    pub root_id: String,
    pub record_id: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitOid {
    pub algorithm: String,
    pub hex: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlobSide {
    pub mode: u32,
    pub blob: GitOid,
    pub raw_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestEntry {
    pub path: String,
    pub before: Option<BlobSide>,
    pub after: Option<BlobSide>,
    pub task_attribution: String,
    pub attribution_evidence: RecordRef,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttributedManifest {
    pub entries: Vec<ManifestEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationCandidate {
    pub campaign: RecordRef,
    pub candidate: RecordRef,
    pub plan_revision: String,
    pub scope_revision: String,
    pub checkpoint_commit: GitOid,
    pub checkpoint_tree: GitOid,
    pub checkpoint_parent: GitOid,
    pub review_outcomes: Vec<RecordRef>,
    pub gate_receipts: Vec<RecordRef>,
    pub manifest: AttributedManifest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Eligibility {
    pub candidate_digest: String,
    pub campaign_ref: RecordRef,
    pub acceptance_ref: RecordRef,
    pub scope: String,
    pub active: bool,
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidationReceipt {
    pub schema_version: u32,
    pub root_id: String,
    pub record_id: String,
    pub tree: GitOid,
    pub policy_digest: String,
    pub argv_digest: String,
    pub exit_code: i32,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedPublication {
    pub operation_id: String,
    pub preparation_digest: String,
    pub eligibility_ref: RecordRef,
    pub repository_id: String,
    pub canonical_ref: String,
    pub expected_parent: GitOid,
    pub manifest_digest: String,
    pub integrated_tree: GitOid,
    pub prepared_commit: GitOid,
    pub proposed_message: String,
    pub message: String,
    pub validation_policy_digest: String,
    pub validation_ref: RecordRef,
    pub artifact_identity: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationRequest {
    pub offer_ref: RecordRef,
    pub eligibility_ref: RecordRef,
    pub prepared_ref: RecordRef,
    pub operation_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum OperationState {
    Prepared,
    Admitted,
    AttemptStarted,
    ClosedNoEffect,
    PublishedObserved,
    PublishedUnconfirmed,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceOperation {
    pub operation_id: String,
    pub prepared_ref: Option<RecordRef>,
    pub state: OperationState,
    pub revision: u64,
    pub generation: Option<u64>,
    pub admission_ref: Option<RecordRef>,
    pub attempt_ref: Option<RecordRef>,
    pub result_ref: Option<RecordRef>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationReceipt {
    pub operation_id: String,
    pub request_digest: String,
    pub preparation_digest: String,
    pub admission_digest: Option<String>,
    pub resources: Vec<String>,
    pub generation: Option<u64>,
    pub admission_ref: Option<RecordRef>,
    pub closure_ref: Option<RecordRef>,
    pub commit: GitOid,
    pub parent: GitOid,
    pub tree: GitOid,
    pub message: String,
    pub observed_target: Option<GitOid>,
    pub state: OperationState,
    pub limitation: String,
    pub recovery_locator: String,
    pub authority_mode: String,
}

/// Installed by a trusted launcher. Requests cannot deserialize or replace it.
#[derive(Clone, Debug)]
pub struct WorkspaceConfig {
    pub root: PathBuf,
    pub repository: PathBuf,
    pub canonical_refs: Vec<String>,
    pub validation_program: Vec<String>,
    pub validation_policy_digest: String,
    pub authority_mode: AuthorityMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorityMode {
    FixtureOperator,
    /// Actual managed boundary exercised under explicit test-owner selection.
    FixtureManagedCustody,
    Production,
}
impl AuthorityMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::FixtureOperator => "fixture_operator",
            Self::FixtureManagedCustody => "fixture_managed_custody",
            Self::Production => "production",
        }
    }
}

/// The campaign owner implements this readback; request JSON cannot implement it.
pub trait AuthorityReadback {
    fn check_eligibility(&self, eligibility: &RecordRef) -> crate::Result<Eligibility>;
    fn read_candidate(&self, candidate_digest: &str) -> crate::Result<PublicationCandidate>;
    fn check_create_consumption(
        &self,
        request: &PublicationRequest,
    ) -> crate::Result<CheckedCreate>;
}

#[derive(Clone, Debug)]
pub struct CheckedCreate {
    pub decision_ref: RecordRef,
    pub consumption_ref: RecordRef,
}
