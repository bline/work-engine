//! Campaign-owned installation and one-shot Applied initial entry. Descriptive
//! startup, locator, and E1 profile data cannot construct these types.
use crate::codec::raw_sha256;
use crate::contract::{CampaignIdentity, NativeReviewRequestRef};
use crate::{CampaignError, Result, require_sha, require_text};
use serde::Deserialize;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

const DEPLOYMENT_REGISTRY: &str = "/etc/work-engine/review-host/deployment-pin.json";
const PIN_LIMIT: usize = 64 * 1024;
const RESOURCE_POLICY_V1: &str = "strict_aggregate_session_64mib_v1";
const RESOURCE_POLICY_V2: &str = "session64mib_combined1g_zero_swap_v2";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeploymentRecord {
    schema_version: u32,
    selected_by: String,
    acceptance_path: PathBuf,
    acceptance_sha256: String,
}
impl DeploymentRecord {
    fn parse(raw: &[u8]) -> Result<Self> {
        let record: Self = serde_json::from_slice(raw)
            .map_err(|e| CampaignError::Unsupported(format!("deployment pin invalid: {e}")))?;
        if record.schema_version != 1 || record.selected_by != "review-host-deployment-owner" {
            return Err(CampaignError::Unsupported(
                "deployment pin not owner-selected".into(),
            ));
        }
        require_sha(&record.acceptance_sha256, "acceptance digest")?;
        Ok(record)
    }
}

/// Opened only from the protected permanent-host deployment registry. A
/// startup document and a protocol command have no constructor for this pin.
pub struct HostDeploymentPin {
    acceptance_path: PathBuf,
    acceptance_sha256: String,
}
impl HostDeploymentPin {
    pub fn open_installed_host() -> Result<Self> {
        let raw = read_protected(Path::new(DEPLOYMENT_REGISTRY))?;
        let record = DeploymentRecord::parse(&raw)?;
        Ok(Self {
            acceptance_path: record.acceptance_path,
            acceptance_sha256: record.acceptance_sha256,
        })
    }
}

fn read_protected(path: &Path) -> Result<Vec<u8>> {
    if !path.is_absolute() || fs::canonicalize(path).ok().as_deref() != Some(path) {
        return Err(CampaignError::Root(
            "protected pin path not canonical".into(),
        ));
    }
    // The permanent host is unprivileged. Owner-selected deployment metadata
    // is readable, but no directory in its path may be replaceable by that
    // host or another unprivileged actor.
    let mut parent = path.parent();
    while let Some(directory) = parent {
        let meta =
            fs::symlink_metadata(directory).map_err(|e| CampaignError::Root(e.to_string()))?;
        if !meta.file_type().is_dir() || meta.uid() != 0 || meta.permissions().mode() & 0o022 != 0 {
            return Err(CampaignError::Root(
                "protected pin ancestry is writable or unowned".into(),
            ));
        }
        parent = directory.parent();
    }
    let before = fs::symlink_metadata(path).map_err(|e| CampaignError::Root(e.to_string()))?;
    if !before.file_type().is_file()
        || before.uid() != 0
        || !matches!(before.permissions().mode() & 0o777, 0o644 | 0o444)
        || before.len() > PIN_LIMIT as u64
    {
        return Err(CampaignError::Root(
            "protected pin owner, mode, type or size differs".into(),
        ));
    }
    let opened = File::open(path).map_err(|e| CampaignError::Root(e.to_string()))?;
    let meta = opened
        .metadata()
        .map_err(|e| CampaignError::Root(e.to_string()))?;
    if (meta.dev(), meta.ino()) != (before.dev(), before.ino()) {
        return Err(CampaignError::Root(
            "protected pin changed while opening".into(),
        ));
    }
    let mut raw = Vec::new();
    opened
        .take((PIN_LIMIT + 1) as u64)
        .read_to_end(&mut raw)
        .map_err(|e| CampaignError::Root(e.to_string()))?;
    if raw.len() > PIN_LIMIT
        || fs::symlink_metadata(path)
            .ok()
            .map(|m| (m.dev(), m.ino(), m.len()))
            != Some((before.dev(), before.ino(), before.len()))
    {
        return Err(CampaignError::Root(
            "protected pin changed while reading".into(),
        ));
    }
    Ok(raw)
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AcceptanceRecord {
    pub(crate) schema_version: u32,
    pub(crate) decision: String,
    pub(crate) reference: String,
    pub(crate) revision: String,
    pub(crate) identity: CampaignIdentity,
    pub(crate) obligation_id: String,
    pub(crate) candidate_tree: String,
    pub(crate) builder_claim_id: String,
    pub(crate) campaign_claim_id: String,
    pub(crate) selection_digest: String,
    pub(crate) physical_profile_digest: String,
    pub(crate) claude_profile_digest: String,
    pub(crate) program_sha256: String,
    pub(crate) guard_sha256: String,
    pub(crate) cbm_store_sha256: String,
    pub(crate) catalog_sha256: String,
    pub(crate) requested_model: String,
    pub(crate) credential_source_metadata_sha256: String,
    pub(crate) resource_policy: String,
    pub(crate) session_id: String,
    pub(crate) attempt_id: String,
    pub(crate) accepted_boundary_sha256: String,
    pub(crate) prompt_template_sha256: String,
    pub(crate) attempt_horizon_millis: u64,
}
impl AcceptanceRecord {
    fn parse(raw: &[u8]) -> Result<Self> {
        if raw.len() > PIN_LIMIT {
            return Err(CampaignError::Contract("acceptance exceeds limit".into()));
        }
        let record: Self = serde_json::from_slice(raw)
            .map_err(|e| CampaignError::Contract(format!("acceptance record invalid: {e}")))?;
        if record.schema_version != 1
            || record.decision != "accepted"
            || !matches!(
                record.resource_policy.as_str(),
                RESOURCE_POLICY_V1 | RESOURCE_POLICY_V2
            )
        {
            return Err(CampaignError::Unsupported(
                "acceptance decision or resource policy differs".into(),
            ));
        }
        record.identity.validate()?;
        for (name, value) in [
            ("acceptance reference", &record.reference),
            ("acceptance revision", &record.revision),
            ("obligation", &record.obligation_id),
            ("builder claim", &record.builder_claim_id),
            ("campaign claim", &record.campaign_claim_id),
            ("model", &record.requested_model),
            ("session", &record.session_id),
            ("attempt", &record.attempt_id),
        ] {
            require_text(value, name)?;
        }
        if !matches!(record.candidate_tree.len(), 40 | 64)
            || !record
                .candidate_tree
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(CampaignError::Contract(
                "candidate tree must be a lowercase Git OID".into(),
            ));
        }
        for (name, value) in [
            ("selection", &record.selection_digest),
            ("physical profile", &record.physical_profile_digest),
            ("Claude profile", &record.claude_profile_digest),
            ("program", &record.program_sha256),
            ("guard", &record.guard_sha256),
            ("CBM", &record.cbm_store_sha256),
            ("catalog", &record.catalog_sha256),
            (
                "credential metadata",
                &record.credential_source_metadata_sha256,
            ),
            ("accepted boundary", &record.accepted_boundary_sha256),
            ("prompt template", &record.prompt_template_sha256),
        ] {
            require_sha(value, name)?;
        }
        if record.attempt_horizon_millis == 0 || record.attempt_horizon_millis > 600_000 {
            return Err(CampaignError::Contract(
                "accepted attempt horizon invalid".into(),
            ));
        }
        if record.builder_claim_id == record.campaign_claim_id {
            return Err(CampaignError::Contract(
                "accepted claim pair is not distinct".into(),
            ));
        }
        Ok(record)
    }
}

/// Descriptive execution selection checked against the same accepted record
/// in production and offline wrappers. It grants no campaign or E1 entry.
pub struct AcceptedExecutionSelection<'a> {
    pub claude_profile_digest: &'a str,
    pub program_sha256: &'a str,
    pub guard_sha256: &'a str,
    pub cbm_store_sha256: &'a str,
    pub catalog_sha256: &'a str,
    pub requested_model: &'a str,
    pub credential_source_metadata_sha256: &'a str,
    pub session_id: &'a str,
    pub attempt_id: &'a str,
    pub resource_policy: &'a str,
    pub prompt_template_sha256: &'a str,
    pub attempt_horizon_millis: u64,
}
fn matches_execution(
    record: &AcceptanceRecord,
    selected: &AcceptedExecutionSelection<'_>,
) -> Result<()> {
    if record.claude_profile_digest != selected.claude_profile_digest
        || record.program_sha256 != selected.program_sha256
        || record.guard_sha256 != selected.guard_sha256
        || record.cbm_store_sha256 != selected.cbm_store_sha256
        || record.catalog_sha256 != selected.catalog_sha256
        || record.requested_model != selected.requested_model
        || record.credential_source_metadata_sha256 != selected.credential_source_metadata_sha256
        || record.session_id != selected.session_id
        || record.attempt_id != selected.attempt_id
        || record.resource_policy != selected.resource_policy
        || record.prompt_template_sha256 != selected.prompt_template_sha256
        || record.attempt_horizon_millis != selected.attempt_horizon_millis
    {
        return Err(CampaignError::Conflict(
            "accepted execution selection differs".into(),
        ));
    }
    Ok(())
}

/// Checked acceptance from the independent protected deployment pin.
pub struct InstalledAcceptancePin {
    pub(crate) digest: String,
    pub(crate) record: AcceptanceRecord,
}
impl InstalledAcceptancePin {
    pub fn open_production(deployment: &HostDeploymentPin) -> Result<Self> {
        let raw = read_protected(&deployment.acceptance_path)?;
        Self::from_protected_bytes(deployment, &raw)
    }
    fn from_protected_bytes(deployment: &HostDeploymentPin, raw: &[u8]) -> Result<Self> {
        let digest = raw_sha256(raw);
        if digest != deployment.acceptance_sha256 {
            return Err(CampaignError::Root(
                "acceptance source digest changed".into(),
            ));
        }
        Ok(Self {
            digest,
            record: AcceptanceRecord::parse(raw)?,
        })
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn reference(&self) -> &str {
        &self.record.reference
    }
    pub fn validate_execution(&self, selected: &AcceptedExecutionSelection<'_>) -> Result<()> {
        matches_execution(&self.record, selected)
    }
}

#[cfg(feature = "test-support")]
pub struct OfflineAcceptancePin {
    pub(crate) digest: String,
    pub(crate) record: AcceptanceRecord,
}
#[cfg(feature = "test-support")]
impl OfflineAcceptancePin {
    pub fn test_owned(raw: &[u8]) -> Result<Self> {
        let record = AcceptanceRecord::parse(raw)?;
        Ok(Self {
            digest: raw_sha256(raw),
            record,
        })
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn validate_execution(&self, selected: &AcceptedExecutionSelection<'_>) -> Result<()> {
        matches_execution(&self.record, selected)
    }
}

pub struct InstalledProfileAuthority {
    record: AcceptanceRecord,
    digest: String,
}
#[cfg(feature = "test-support")]
pub struct OfflineProfileAuthority {
    record: AcceptanceRecord,
    digest: String,
}

pub struct FreshInitialEntry {
    binding: InitialEntryBinding,
    _authority: InstalledProfileAuthority,
}
#[cfg(feature = "test-support")]
/// The offline entry type cannot be converted into installed authority.
///
/// ```
/// use slice_campaign::{FreshInitialEntry, OfflineInitialEntry};
/// fn import_check(_installed: Option<FreshInitialEntry>, _offline: Option<OfflineInitialEntry>) {}
/// import_check(None, None);
/// ```
///
/// ```compile_fail
/// use slice_campaign::{FreshInitialEntry, OfflineInitialEntry};
/// fn installed(_: FreshInitialEntry) {}
/// fn try_upgrade(offline: OfflineInitialEntry) { installed(offline); }
/// ```
pub struct OfflineInitialEntry {
    binding: InitialEntryBinding,
    _authority: OfflineProfileAuthority,
}

/// Read-only exact bindings; constructing these values does not grant entry.
pub struct InitialEntryBinding {
    pub(crate) campaign_root_id: String,
    pub(crate) obligation_id: String,
    pub(crate) request_digest: String,
    pub(crate) dispatch_operation_id: String,
    pub(crate) dispatch_revision: String,
    pub(crate) dispatch_receipt_digest: String,
    pub(crate) attempt_id: String,
    pub(crate) session_id: String,
    pub(crate) candidate_tree: String,
    pub(crate) selection_digest: String,
    pub(crate) physical_profile_digest: String,
    pub(crate) claude_profile_digest: String,
    pub(crate) program_sha256: String,
    pub(crate) guard_sha256: String,
    pub(crate) cbm_store_sha256: String,
    pub(crate) catalog_sha256: String,
    pub(crate) requested_model: String,
    pub(crate) credential_source_metadata_sha256: String,
    pub(crate) acceptance_digest: String,
    pub(crate) acceptance_reference: String,
    pub(crate) resource_policy: String,
    pub(crate) dispatched_request: NativeReviewRequestRef,
    pub(crate) review_episode_id: String,
    pub(crate) prompt_template_sha256: String,
    pub(crate) attempt_horizon_millis: u64,
}
impl InitialEntryBinding {
    pub fn campaign_root_id(&self) -> &str {
        &self.campaign_root_id
    }
    pub fn obligation_id(&self) -> &str {
        &self.obligation_id
    }
    pub fn request_digest(&self) -> &str {
        &self.request_digest
    }
    pub fn dispatch_operation_id(&self) -> &str {
        &self.dispatch_operation_id
    }
    pub fn dispatch_revision(&self) -> &str {
        &self.dispatch_revision
    }
    pub fn dispatch_receipt_digest(&self) -> &str {
        &self.dispatch_receipt_digest
    }
    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn candidate_tree(&self) -> &str {
        &self.candidate_tree
    }
    pub fn selection_digest(&self) -> &str {
        &self.selection_digest
    }
    pub fn physical_profile_digest(&self) -> &str {
        &self.physical_profile_digest
    }
    pub fn claude_profile_digest(&self) -> &str {
        &self.claude_profile_digest
    }
    pub fn program_sha256(&self) -> &str {
        &self.program_sha256
    }
    pub fn guard_sha256(&self) -> &str {
        &self.guard_sha256
    }
    pub fn cbm_store_sha256(&self) -> &str {
        &self.cbm_store_sha256
    }
    pub fn catalog_sha256(&self) -> &str {
        &self.catalog_sha256
    }
    pub fn requested_model(&self) -> &str {
        &self.requested_model
    }
    pub fn credential_source_metadata_sha256(&self) -> &str {
        &self.credential_source_metadata_sha256
    }
    pub fn acceptance_digest(&self) -> &str {
        &self.acceptance_digest
    }
    pub fn acceptance_reference(&self) -> &str {
        &self.acceptance_reference
    }
    pub fn resource_policy(&self) -> &str {
        &self.resource_policy
    }
    pub fn dispatched_request(&self) -> &NativeReviewRequestRef {
        &self.dispatched_request
    }
    pub fn review_episode_id(&self) -> &str {
        &self.review_episode_id
    }
    pub fn prompt_template_sha256(&self) -> &str {
        &self.prompt_template_sha256
    }
    pub fn attempt_horizon_millis(&self) -> u64 {
        self.attempt_horizon_millis
    }
}
impl FreshInitialEntry {
    pub fn binding(&self) -> &InitialEntryBinding {
        &self.binding
    }
}
#[cfg(feature = "test-support")]
impl OfflineInitialEntry {
    pub fn binding(&self) -> &InitialEntryBinding {
        &self.binding
    }
}

impl InstalledProfileAuthority {
    pub(crate) fn from_pin(pin: InstalledAcceptancePin) -> Self {
        Self {
            record: pin.record,
            digest: pin.digest,
        }
    }
    pub(crate) fn record(&self) -> &AcceptanceRecord {
        &self.record
    }
    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }
    pub(crate) fn issue(self, binding: InitialEntryBinding) -> FreshInitialEntry {
        FreshInitialEntry {
            binding,
            _authority: self,
        }
    }
}
#[cfg(feature = "test-support")]
impl OfflineProfileAuthority {
    pub(crate) fn from_pin(pin: OfflineAcceptancePin) -> Self {
        Self {
            record: pin.record,
            digest: pin.digest,
        }
    }
    pub(crate) fn record(&self) -> &AcceptanceRecord {
        &self.record
    }
    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }
    pub(crate) fn issue(self, binding: InitialEntryBinding) -> OfflineInitialEntry {
        OfflineInitialEntry {
            binding,
            _authority: self,
        }
    }
}

pub(crate) fn acceptance_matches(
    record: &AcceptanceRecord,
    identity: &CampaignIdentity,
    obligation: &str,
    candidate_tree: &str,
    selection_digest: &str,
    physical_profile_digest: &str,
    boundary_digest: &str,
) -> bool {
    record.identity == *identity
        && record.obligation_id == obligation
        && record.candidate_tree == candidate_tree
        && record.selection_digest == selection_digest
        && record.physical_profile_digest == physical_profile_digest
        && record.accepted_boundary_sha256 == boundary_digest
}
#[allow(clippy::too_many_arguments)] // Independent acceptance, owner origin and exact dispatch identities.
pub(crate) fn entry_binding(
    record: &AcceptanceRecord,
    acceptance_digest: &str,
    campaign_root_id: &str,
    dispatched_request: &NativeReviewRequestRef,
    review_episode_id: &str,
    dispatch_operation_id: &str,
    dispatch_revision: &str,
    dispatch_receipt_digest: &str,
) -> InitialEntryBinding {
    InitialEntryBinding {
        campaign_root_id: campaign_root_id.into(),
        obligation_id: record.obligation_id.clone(),
        request_digest: dispatched_request.request_digest.clone(),
        dispatch_operation_id: dispatch_operation_id.into(),
        dispatch_revision: dispatch_revision.into(),
        dispatch_receipt_digest: dispatch_receipt_digest.into(),
        attempt_id: record.attempt_id.clone(),
        session_id: record.session_id.clone(),
        candidate_tree: record.candidate_tree.clone(),
        selection_digest: record.selection_digest.clone(),
        physical_profile_digest: record.physical_profile_digest.clone(),
        claude_profile_digest: record.claude_profile_digest.clone(),
        program_sha256: record.program_sha256.clone(),
        guard_sha256: record.guard_sha256.clone(),
        cbm_store_sha256: record.cbm_store_sha256.clone(),
        catalog_sha256: record.catalog_sha256.clone(),
        requested_model: record.requested_model.clone(),
        credential_source_metadata_sha256: record.credential_source_metadata_sha256.clone(),
        acceptance_digest: acceptance_digest.into(),
        acceptance_reference: record.reference.clone(),
        resource_policy: record.resource_policy.clone(),
        dispatched_request: dispatched_request.clone(),
        review_episode_id: review_episode_id.into(),
        prompt_template_sha256: record.prompt_template_sha256.clone(),
        attempt_horizon_millis: record.attempt_horizon_millis,
    }
}

#[cfg(test)]
mod protected_loader_tests {
    use super::*;

    #[test]
    fn installed_loader_refuses_wrong_owner_wrong_selection_and_changed_acceptance() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("deployment.json");
        fs::write(&source, b"{}").unwrap();
        assert!(
            read_protected(&source).is_err(),
            "user-owned source cannot be installed"
        );
        let wrong = serde_json::json!({"schemaVersion":1,"selectedBy":"startup-document",
            "acceptancePath":"/etc/work-engine/review-host/accepted.json",
            "acceptanceSha256":"a".repeat(64)});
        assert!(DeploymentRecord::parse(&serde_json::to_vec(&wrong).unwrap()).is_err());
        let pin = HostDeploymentPin {
            acceptance_path: PathBuf::from("/etc/work-engine/review-host/accepted.json"),
            acceptance_sha256: "b".repeat(64),
        };
        assert!(InstalledAcceptancePin::from_protected_bytes(&pin, b"changed").is_err());
    }
}
