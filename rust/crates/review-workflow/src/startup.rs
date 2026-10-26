//! Startup data is selected by the trusted launcher, never by a wire command.
use crate::{Result, refuse};
use claim_evidence::sha256_bytes;
use review_episode_core::{
    codec::{digest, parse_json},
    identity::Authority,
};
use review_execution_evidence::claude::{
    CandidateBoundary, CredentialSource, PinnedProgram, TrustedClaudeProfile, TrustedRp0Profile,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[cfg(feature = "test-support")]
use slice_campaign::OfflineAcceptancePin;
use slice_campaign::{
    AcceptedBoundary, AcceptedExecutionSelection, Baseline, Campaign, CampaignIdentity,
    HostDeploymentPin, InstalledAcceptancePin, TrustedConfig,
};
use std::sync::Arc;
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

pub const PROFILE: &str = "review-host-controlled-initial-v1";
pub const QUALIFIED_PROFILE: &str = "review-host-qualified-initial-v1";
#[cfg(feature = "test-support")]
pub const OFFLINE_PROFILE: &str = "review-host-qualified-offline-v1";
pub const STARTUP_LIMIT: usize = 1024 * 1024;
const INPUT_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilePin {
    pub path: PathBuf,
    pub sha256: String,
    pub mode: u32,
    #[serde(default)]
    pub owner_uid: Option<u32>,
}
impl FilePin {
    pub fn read(&self, limit: usize) -> Result<Vec<u8>> {
        if !self.path.is_absolute() || fs::canonicalize(&self.path)? != self.path {
            return Err(refuse("startup", "pinned file path is not canonical"));
        }
        let before = fs::symlink_metadata(&self.path)?;
        if !before.is_file()
            || before.file_type().is_symlink()
            || before.mode() & 0o7777 != self.mode
            || self.owner_uid.is_some_and(|uid| uid != before.uid())
            || before.len() > limit as u64
        {
            return Err(refuse("startup", "pinned file type, mode or size differs"));
        }
        let file = File::open(&self.path)?;
        let opened = file.metadata()?;
        if (opened.dev(), opened.ino()) != (before.dev(), before.ino()) {
            return Err(refuse("startup", "pinned file changed while opening"));
        }
        let mut bytes = Vec::new();
        file.take((limit + 1) as u64).read_to_end(&mut bytes)?;
        if bytes.len() > limit || sha256_bytes(&bytes) != self.sha256 {
            return Err(refuse("startup", "pinned file bytes differ"));
        }
        let after = fs::symlink_metadata(&self.path)?;
        if (
            after.dev(),
            after.ino(),
            after.len(),
            after.mode(),
            after.uid(),
        ) != (
            before.dev(),
            before.ino(),
            before.len(),
            before.mode(),
            before.uid(),
        ) {
            return Err(refuse("startup", "pinned file changed while reading"));
        }
        Ok(bytes)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Roots {
    pub campaign: PathBuf,
    pub episode: PathBuf,
    pub claims: PathBuf,
    pub evidence: PathBuf,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleasePins {
    pub executable: FilePin,
    pub source_archive: FilePin,
    pub toolchain: FilePin,
    pub lockfile: FilePin,
    pub target: String,
    pub features: Vec<String>,
    pub runtime_dependencies: Vec<FilePin>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompilerInputs {
    pub mode: String,
    pub document: FilePin,
    pub options: FilePin,
    pub role: String,
    pub requirements: Option<FilePin>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Peer {
    pub executable: FilePin,
    pub source: FilePin,
    pub arguments: Vec<String>,
    pub session: String,
    pub evidence_root_id: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Claims {
    pub root_id: String,
    pub source: FilePin,
    pub configuration: FilePin,
    pub production_grant_id: String,
    pub production_grant_sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartupDocument {
    pub schema_version: u32,
    pub profile: String,
    pub evidence_profile: String,
    pub identity: CampaignIdentity,
    pub repository: PathBuf,
    pub workspace: PathBuf,
    pub accepted_boundary: AcceptedBoundary,
    pub baseline: Baseline,
    pub roots: Roots,
    pub host: ReleasePins,
    pub gate_capture: FilePin,
    pub gate_manifest: FilePin,
    pub gate_receipt: FilePin,
    pub candidate: FilePin,
    pub physical_profile: FilePin,
    pub selection: FilePin,
    pub episode_authority: FilePin,
    pub compiler: CompilerInputs,
    pub request_input: FilePin,
    pub peer: Peer,
    pub claims: Claims,
    pub obligation_id: String,
    pub operation_id: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualifiedStartupDocument {
    pub schema_version: u32,
    pub profile: String,
    pub base: StartupDocument,
    pub qualified: QualifiedInputs,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualifiedInputs {
    pub bwrap: FilePin,
    pub cbm: FilePin,
    pub cbm_launcher: FilePin,
    pub cbm_client: FilePin,
    pub guard: FilePin,
    pub catalog: FilePin,
    pub boundary_root: PathBuf,
    pub boundary_manifest_sha256: String,
    pub requested_model: String,
    pub requested_effort: String,
    pub accepted_observed_models: Vec<String>,
    pub credential_source: PathBuf,
    pub credential_source_metadata_sha256: String,
    pub resource_policy: String,
    pub attempt_horizon_millis: u64,
    pub completion_grant_id: String,
    pub completion_grant_sha256: String,
    pub completion_consumer: String,
    pub completion_decision_scope: String,
    pub completion_authority_source: Value,
    #[cfg(feature = "test-support")]
    pub offline_acceptance: Option<FilePin>,
    #[cfg(feature = "test-support")]
    pub offline_witness: Option<PathBuf>,
}

pub(crate) enum QualifiedPin {
    Installed(InstalledAcceptancePin),
    #[cfg(feature = "test-support")]
    Offline(OfflineAcceptancePin),
}
pub(crate) struct QualifiedRuntime {
    pub(crate) profile: TrustedClaudeProfile,
    pub(crate) boundary: Arc<CandidateBoundary>,
    pub(crate) credential: CredentialSource,
    pub(crate) prompt_template: Vec<u8>,
    pub(crate) horizon_millis: u64,
    pub(crate) completion_grant_id: String,
    pub(crate) completion_grant_sha256: String,
    pub(crate) completion_consumer: String,
    pub(crate) completion_decision_scope: String,
    pub(crate) completion_authority_source: Value,
    #[cfg(feature = "test-support")]
    pub(crate) offline_witness: Option<PathBuf>,
    pub(crate) pin: Option<QualifiedPin>,
}

fn qualified_prompt_template_from_parts(
    document: &StartupDocument,
    projection: &Value,
    request_input: &Value,
) -> Vec<u8> {
    work_engine_compiler::canonical_json(&json!({
        "schemaVersion":1,"profile":document.profile,"compilerMode":document.compiler.mode,
        "compilerProjection":projection,"selectedRole":document.compiler.role,
        "input":request_input,
    }))
    .into_bytes()
}

/// The selected compiler role owns this text. Startup checks its effective
/// projection before a schema-2 boundary may qualify; RP0 never injects it
/// after compilation or grants an additional reviewer tool.
pub const RP0_COVERAGE_GUIDANCE_V2: &str = "RP0 coverage ledger: read `/cbm/coverage.json` for the selected candidate before relying on graph results. For each material graph claim, record project and generation, bounded query and pagination, exact symbol/path, and coverage for each relied-on path. If the ledger or query reports unverified, partial, skipped, excluded, stale or unknown coverage, read the corresponding exact source at `/repo/candidate/<path>` for every reported gap range, or the whole file where no usable range exists; cite the direct source and record the graph limitation. A ledger disposition only identifies work required. It does not prove that you inspected source or that graph coverage is complete. If required evidence cannot be obtained, report the result incomplete.";

fn selected_role_has_coverage_guidance(template: &[u8], selected_role: &str) -> Result<bool> {
    let prompt: Value = serde_json::from_slice(template)?;
    Ok(prompt
        .get("compilerProjection")
        .and_then(|projection| projection.get("roles"))
        .and_then(|roles| roles.get(selected_role))
        .and_then(|role| role.get("developerInstructions"))
        .and_then(Value::as_str)
        .is_some_and(|instructions| instructions.contains(RP0_COVERAGE_GUIDANCE_V2)))
}

/// Descriptive deterministic bytes; constructing them cannot install an
/// acceptance pin or issue an Applied entry.
pub fn qualified_prompt_template_for_document(document: &StartupDocument) -> Result<Vec<u8>> {
    let compiler_document: Value =
        serde_json::from_slice(&document.compiler.document.read(INPUT_LIMIT)?)?;
    let compiler_options: Value =
        serde_json::from_slice(&document.compiler.options.read(INPUT_LIMIT)?)?;
    let request_input: Value = serde_json::from_slice(&document.request_input.read(INPUT_LIMIT)?)?;
    let projection =
        work_engine_compiler::project_runtime_manifest(&compiler_document, &compiler_options)
            .map_err(|e| refuse("compiler", format!("{}: {}", e.code, e.message)))?;
    Ok(qualified_prompt_template_from_parts(
        document,
        &projection,
        &request_input,
    ))
}
impl QualifiedRuntime {
    fn load(
        d: &StartupDocument,
        inputs: QualifiedInputs,
        offline: bool,
        prompt_template: Vec<u8>,
    ) -> Result<Self> {
        if !matches!(
            inputs.resource_policy.as_str(),
            "strict_aggregate_session_64mib_v1" | "session64mib_combined1g_zero_swap_v2"
        ) || inputs.attempt_horizon_millis == 0
            || inputs.attempt_horizon_millis > 600_000
            || inputs.accepted_observed_models.is_empty()
        {
            return Err(refuse(
                "startup",
                "qualified resource/model policy unsupported",
            ));
        }
        let mut all_pins = vec![
            &inputs.bwrap,
            &inputs.cbm,
            &inputs.cbm_launcher,
            &inputs.cbm_client,
            &inputs.guard,
            &inputs.catalog,
            &d.peer.executable,
            &d.host.executable,
            &d.host.source_archive,
            &d.host.toolchain,
            &d.host.lockfile,
            &d.gate_capture,
            &d.gate_manifest,
            &d.gate_receipt,
            &d.candidate,
            &d.physical_profile,
            &d.selection,
            &d.episode_authority,
            &d.compiler.document,
            &d.compiler.options,
            &d.request_input,
            &d.peer.source,
            &d.claims.source,
            &d.claims.configuration,
        ];
        all_pins.extend(d.host.runtime_dependencies.iter());
        all_pins.extend(d.compiler.requirements.iter());
        #[cfg(feature = "test-support")]
        all_pins.extend(inputs.offline_acceptance.iter());
        for pin in all_pins {
            if pin.owner_uid.is_none() {
                return Err(refuse("startup", "qualified pin owner absent"));
            }
        }
        let program = |pin: &FilePin| -> Result<PinnedProgram> {
            pin.read(128 * INPUT_LIMIT)?;
            PinnedProgram::load(&pin.path, &pin.sha256)
                .map_err(|e| refuse("startup", e.to_string()))
        };
        let rp0 = TrustedRp0Profile::load(
            program(&inputs.bwrap)?,
            program(&inputs.cbm)?,
            program(&inputs.cbm_launcher)?,
            program(&inputs.cbm_client)?,
        )
        .map_err(|e| refuse("startup", e.to_string()))?;
        let boundary = CandidateBoundary::open_frozen(
            rp0,
            &d.repository,
            &inputs.boundary_root,
            &inputs.boundary_manifest_sha256,
        )
        .and_then(|boundary| boundary.with_resource_policy(&inputs.resource_policy))
        .map_err(|e| refuse("startup", e.to_string()))?;
        if boundary.manifest_schema() == 2
            && !selected_role_has_coverage_guidance(&prompt_template, &d.compiler.role)?
        {
            return Err(refuse("startup", "schema-2 coverage guidance absent"));
        }
        let catalog = inputs.catalog.read(64 * 1024)?;
        let claude = program(&d.peer.executable)?;
        let guard = program(&inputs.guard)?;
        let profile = if offline {
            #[cfg(feature = "test-support")]
            {
                TrustedClaudeProfile::load_offline_guarded_peer(
                    claude,
                    guard,
                    &inputs.requested_model,
                    &inputs.requested_effort,
                    inputs.accepted_observed_models.clone(),
                    &catalog,
                )
            }
            #[cfg(not(feature = "test-support"))]
            {
                return Err(refuse("unsupported", "offline profile unavailable"));
            }
        } else {
            TrustedClaudeProfile::load_installed(
                claude,
                guard,
                &inputs.requested_model,
                &inputs.requested_effort,
                inputs.accepted_observed_models.clone(),
                &catalog,
            )
        }
        .map_err(|e| refuse("startup", e.to_string()))?;
        let credential = CredentialSource::new(&inputs.credential_source)
            .map_err(|e| refuse("startup", e.to_string()))?;
        if credential.metadata_digest() != inputs.credential_source_metadata_sha256 {
            return Err(refuse("startup", "credential source metadata differs"));
        }
        let pin = if offline {
            #[cfg(feature = "test-support")]
            {
                let selected = inputs
                    .offline_acceptance
                    .as_ref()
                    .ok_or_else(|| refuse("startup", "offline acceptance absent"))?;
                QualifiedPin::Offline(OfflineAcceptancePin::test_owned(
                    &selected.read(64 * 1024)?,
                )?)
            }
            #[cfg(not(feature = "test-support"))]
            {
                return Err(refuse("unsupported", "offline acceptance unavailable"));
            }
        } else {
            let deployment = HostDeploymentPin::open_installed_host()?;
            QualifiedPin::Installed(InstalledAcceptancePin::open_production(&deployment)?)
        };
        let selected = AcceptedExecutionSelection {
            claude_profile_digest: profile.profile_digest(),
            program_sha256: d.peer.executable.sha256.as_str(),
            guard_sha256: inputs.guard.sha256.as_str(),
            cbm_store_sha256: boundary.index().store_sha256(),
            catalog_sha256: profile.catalog_sha256(),
            requested_model: profile.requested_model(),
            credential_source_metadata_sha256: &inputs.credential_source_metadata_sha256,
            session_id: &d.peer.session,
            attempt_id: &d.identity.attempt_id,
            resource_policy: &inputs.resource_policy,
            prompt_template_sha256: &sha256_bytes(&prompt_template),
            attempt_horizon_millis: inputs.attempt_horizon_millis,
        };
        match &pin {
            QualifiedPin::Installed(value) => value.validate_execution(&selected)?,
            #[cfg(feature = "test-support")]
            QualifiedPin::Offline(value) => value.validate_execution(&selected)?,
        }
        if prompt_template.is_empty()
            || prompt_template.len() > review_execution_evidence::REQUEST_MAX_BYTES
        {
            return Err(refuse("capacity", "qualified prompt capacity"));
        }
        Ok(Self {
            profile,
            boundary: Arc::new(boundary),
            credential,
            prompt_template,
            horizon_millis: inputs.attempt_horizon_millis,
            completion_grant_id: inputs.completion_grant_id,
            completion_grant_sha256: inputs.completion_grant_sha256,
            completion_consumer: inputs.completion_consumer,
            completion_decision_scope: inputs.completion_decision_scope,
            completion_authority_source: inputs.completion_authority_source,
            #[cfg(feature = "test-support")]
            offline_witness: inputs.offline_witness,
            pin: Some(pin),
        })
    }
    pub(crate) fn install(&mut self, campaign: &mut Campaign) -> Result<()> {
        match self
            .pin
            .take()
            .ok_or_else(|| refuse("startup", "qualified acceptance already consumed"))?
        {
            QualifiedPin::Installed(pin) => campaign.install_qualified_initial_authority(pin)?,
            #[cfg(feature = "test-support")]
            QualifiedPin::Offline(pin) => campaign.install_offline_initial_authority(pin)?,
        }
        Ok(())
    }
    pub(crate) fn installed(&self) -> bool {
        self.profile.installed()
    }
}

/// Only loading and validating the installed startup can construct this scope.
/// Descriptor bytes, roots and executables never come from protocol requests.
/// ```compile_fail
/// let _ = review_workflow::startup::TrustedStartup {};
/// ```
/// ```compile_fail
/// let _: review_workflow::startup::TrustedStartup = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// fn serialize(scope: &review_workflow::startup::TrustedStartup) {
///     serde_json::to_string(scope).unwrap();
/// }
/// ```
pub struct TrustedStartup {
    pub(crate) document: StartupDocument,
    path: PathBuf,
    pub(crate) digest: String,
    pub(crate) authority: Authority,
    pub(crate) projection: Value,
    pub(crate) request_input: Value,
    pub(crate) candidate: Vec<u8>,
    pub(crate) physical_profile: Vec<u8>,
    pub(crate) selection: Value,
    gate: [Vec<u8>; 3],
    pub(crate) qualified: Option<QualifiedRuntime>,
}
impl TrustedStartup {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.is_absolute() || fs::canonicalize(path)? != path {
            return Err(refuse(
                "startup",
                "startup path must be canonical and absolute",
            ));
        }
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_file()
            || metadata.mode() & 0o222 != 0
            || metadata.len() > STARTUP_LIMIT as u64
        {
            return Err(refuse(
                "startup",
                "startup must be an installed read-only file of at most 1 MiB",
            ));
        }
        let file = File::open(path)?;
        let opened = file.metadata()?;
        if (opened.dev(), opened.ino(), opened.uid(), opened.mode())
            != (
                metadata.dev(),
                metadata.ino(),
                metadata.uid(),
                metadata.mode(),
            )
        {
            return Err(refuse("startup", "startup changed while opening"));
        }
        let mut bytes = Vec::new();
        file.take((STARTUP_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > STARTUP_LIMIT {
            return Err(refuse("capacity", "startup exceeds limit"));
        }
        let after = fs::symlink_metadata(path)?;
        if (
            after.dev(),
            after.ino(),
            after.uid(),
            after.mode(),
            after.len(),
        ) != (
            metadata.dev(),
            metadata.ino(),
            metadata.uid(),
            metadata.mode(),
            metadata.len(),
        ) {
            return Err(refuse("startup", "startup changed while reading"));
        }
        let header: Value = serde_json::from_slice(&bytes)?;
        if header["profile"] == QUALIFIED_PROFILE {
            if metadata.uid() != 0 || !matches!(metadata.mode() & 0o777, 0o644 | 0o444) {
                return Err(refuse(
                    "startup",
                    "installed startup is not deployment-owned",
                ));
            }
            let mut parent = path.parent();
            while let Some(directory) = parent {
                let meta = fs::symlink_metadata(directory)?;
                if !meta.is_dir() || meta.uid() != 0 || meta.mode() & 0o022 != 0 {
                    return Err(refuse(
                        "startup",
                        "installed startup ancestry is writable or unowned",
                    ));
                }
                parent = directory.parent();
            }
        }
        let (document, qualified_inputs, offline) = if header["profile"] == QUALIFIED_PROFILE {
            let envelope: QualifiedStartupDocument = serde_json::from_slice(&bytes)?;
            if envelope.schema_version != 2
                || envelope.base.profile != QUALIFIED_PROFILE
                || envelope.base.evidence_profile != "claude-qualified-v1"
            {
                return Err(refuse("unsupported", "qualified startup profile differs"));
            }
            (envelope.base, Some(envelope.qualified), false)
        } else {
            #[cfg(feature = "test-support")]
            if header["profile"] == OFFLINE_PROFILE {
                let envelope: QualifiedStartupDocument = serde_json::from_slice(&bytes)?;
                if envelope.schema_version != 2
                    || envelope.base.profile != OFFLINE_PROFILE
                    || envelope.base.evidence_profile != "claude-offline-peer-v1"
                {
                    return Err(refuse("unsupported", "offline startup profile differs"));
                }
                (envelope.base, Some(envelope.qualified), true)
            } else {
                (serde_json::from_slice(&bytes)?, None, false)
            }
            #[cfg(not(feature = "test-support"))]
            {
                (serde_json::from_slice(&bytes)?, None, false)
            }
        };
        if document.schema_version != 1
            || (qualified_inputs.is_none()
                && (document.profile != PROFILE
                    || document.evidence_profile != "controlled-process-v1"))
        {
            return Err(refuse("unsupported", "startup profile unsupported"));
        }
        document.identity.validate()?;
        for id in [
            &document.obligation_id,
            &document.operation_id,
            &document.peer.session,
            &document.peer.evidence_root_id,
            &document.claims.root_id,
        ] {
            if id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_: .".contains(&b))
            {
                return Err(refuse("startup", "invalid bounded startup identity"));
            }
        }
        if document.peer.arguments.len() > 32
            || document
                .peer
                .arguments
                .iter()
                .any(|s| s.is_empty() || s.len() > 4096)
        {
            return Err(refuse("capacity", "peer argument limit"));
        }
        if document.host.runtime_dependencies.len() > 32
            || document.host.target.is_empty()
            || document.host.features.len() > 16
        {
            return Err(refuse("startup", "release identity limits"));
        }
        let host_bytes = document.host.executable.read(128 * INPUT_LIMIT)?;
        let actual = fs::read(std::env::current_exe()?)?;
        if sha256_bytes(&actual) != sha256_bytes(&host_bytes) {
            return Err(refuse("startup", "actual host executable differs"));
        }
        for pin in [
            &document.host.source_archive,
            &document.host.toolchain,
            &document.host.lockfile,
        ] {
            pin.read(64 * INPUT_LIMIT)?;
        }
        for pin in &document.host.runtime_dependencies {
            pin.read(64 * INPUT_LIMIT)?;
        }
        document.peer.executable.read(128 * INPUT_LIMIT)?;
        document.peer.source.read(INPUT_LIMIT)?;
        if document.peer.executable.mode & 0o111 == 0 {
            return Err(refuse("startup", "peer is not executable"));
        }
        let mut roots = BTreeSet::new();
        for root in [
            &document.roots.campaign,
            &document.roots.episode,
            &document.roots.claims,
            &document.roots.evidence,
        ] {
            let parent = root
                .parent()
                .ok_or_else(|| refuse("startup", "root parent missing"))?;
            if !root.is_absolute()
                || fs::canonicalize(parent)? != parent
                || !roots.insert(root.clone())
                || root.file_name().is_none()
            {
                return Err(refuse(
                    "startup",
                    "roots must be distinct anchored children",
                ));
            }
            if root.exists() && fs::canonicalize(root)? != *root {
                return Err(refuse("startup", "root path changed"));
            }
        }
        if roots
            .iter()
            .any(|a| roots.iter().any(|b| a != b && a.starts_with(b)))
        {
            return Err(refuse("startup", "owner roots overlap"));
        }
        let gate = [
            document.gate_capture.read(STARTUP_LIMIT)?,
            document.gate_manifest.read(STARTUP_LIMIT)?,
            document.gate_receipt.read(STARTUP_LIMIT)?,
        ];
        let candidate = document.candidate.read(STARTUP_LIMIT)?;
        let physical_profile = document.physical_profile.read(STARTUP_LIMIT)?;
        let subject = if qualified_inputs.is_some() {
            slice_campaign::verify_existing_subject_dynamic(
                &document.repository,
                &document.identity,
                &document.baseline,
                &candidate,
                &physical_profile,
                &gate[0],
                &gate[1],
                &gate[2],
            )?
        } else {
            slice_campaign::verify_existing_subject(
                &document.repository,
                &document.identity,
                &document.baseline,
                &candidate,
                &physical_profile,
                &gate[0],
                &gate[1],
                &gate[2],
            )?
        };
        if qualified_inputs.is_some() {
            slice_campaign::verify_candidate_snapshot(
                &document.repository,
                &subject.reference.tree,
            )?;
        }
        let selection: Value = serde_json::from_slice(&document.selection.read(STARTUP_LIMIT)?)?;
        let selection_reference = slice_campaign::contract::validate_selection(
            &selection,
            &document.identity,
            &subject.reference,
        )?;
        let selected = selection["specialists"]
            .as_array()
            .ok_or_else(|| refuse("startup", "selection missing"))?;
        if selected.len() != 1
            || selected[0]["obligationId"] != document.obligation_id
            || selected[0]["selection"] != "selected"
        {
            return Err(refuse(
                "unsupported",
                "exactly one selected obligation is supported",
            ));
        }
        let raw_authority = document.episode_authority.read(STARTUP_LIMIT)?;
        let authority = Authority::parse(parse_json(
            std::str::from_utf8(&raw_authority)
                .map_err(|_| refuse("startup", "authority is not UTF-8"))?,
        )?)?;
        let expected_episode = slice_campaign::contract::episode_digest(
            &json!({"identity":document.identity,"obligationId":document.obligation_id}),
        )?;
        let expected_identity = json!({"runId":document.identity.run_id,"sliceNumber":document.identity.slice_number,"attemptId":document.identity.attempt_id,"planVersion":document.identity.plan_version,"reviewObligationId":document.obligation_id,"reviewEpisodeId":&expected_episode[..32]});
        let expected_subject = parse_json(&json!({"commit":subject.reference.commit,"tree":subject.reference.tree,"patchIdentity":subject.reference.patch_identity}).to_string())?;
        let av = authority.value();
        if authority.identity().value() != parse_json(&expected_identity.to_string())?
            || authority.writer().generation != 1
            || authority.predecessor_revision().is_some()
            || av.get("writer").and_then(|v| v.get("actorId"))
                != Some(&review_episode_core::codec::JsValue::text(
                    "native-review-host",
                ))
            || av
                .get("writer")
                .and_then(|v| v.get("runtimeSession"))
                .and_then(|v| v.get("reference"))
                != Some(&review_episode_core::codec::JsValue::text(
                    &document.peer.session,
                ))
            || authority.initial_subject().get("revision")
                != Some(&review_episode_core::codec::JsValue::text(
                    &subject.reference.commit,
                ))
            || !av
                .get("readers")
                .and_then(|v| v.as_array().ok())
                .is_some_and(|readers| {
                    readers.contains(&review_episode_core::codec::JsValue::text(
                        "native-review-host",
                    ))
                })
            || authority.initial_subject().get("sha256")
                != Some(&review_episode_core::codec::JsValue::text(&digest(
                    &expected_subject,
                )))
        {
            return Err(refuse(
                "startup",
                "episode authority identity, writer or subject differs",
            ));
        }
        let compiler_document = document.compiler.document.read(INPUT_LIMIT)?;
        let compiler_options = document.compiler.options.read(INPUT_LIMIT)?;
        let input = document.request_input.read(INPUT_LIMIT)?;
        if compiler_document.len() + compiler_options.len() + input.len() > INPUT_LIMIT {
            return Err(refuse(
                "capacity",
                "captured compiler/request input exceeds 4 MiB",
            ));
        }
        if document.compiler.mode != "manifest-only-unverified" {
            return Err(refuse(
                "unsupported",
                "compiler verification mode unavailable",
            ));
        }
        let projection = work_engine_compiler::project_runtime_manifest(
            &serde_json::from_slice(&compiler_document)?,
            &serde_json::from_slice(&compiler_options)?,
        )
        .map_err(|e| refuse("compiler", format!("{}: {}", e.code, e.message)))?;
        if projection["roles"].get(&document.compiler.role).is_none() {
            return Err(refuse("compiler", "selected role absent"));
        }
        if let Some(pin) = &document.compiler.requirements {
            let requirements: Value = serde_json::from_slice(&pin.read(INPUT_LIMIT)?)?;
            let diagnostic = work_engine_compiler::satisfy_runtime_requirements(
                &projection,
                &document.compiler.role,
                &requirements,
                None,
            );
            return Err(refuse(
                "unsupported",
                format!(
                    "source-verification/AEG requirements unavailable in unverified controlled profile: {diagnostic:?}"
                ),
            ));
        }
        if projection["roles"].as_object().unwrap().values().any(|r| {
            !r["runtimeRequirements"].is_null()
                || r["skills"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|s| s.get("runtimeRequirements").is_some_and(|v| !v.is_null()))
        }) {
            return Err(refuse(
                "unsupported",
                "embedded runtime requirements need unavailable source verification",
            ));
        }
        let claims_config: Value =
            serde_json::from_slice(&document.claims.configuration.read(STARTUP_LIMIT)?)?;
        let claims_source: Value =
            serde_json::from_slice(&document.claims.source.read(STARTUP_LIMIT)?)?;
        let custody = &claims_config["trusted_custody"];
        #[cfg(feature = "test-support")]
        let synthetic_fixture_join = qualified_inputs.is_none()
            && std::env::var_os("RP2_TEST_SYNTHETIC_FIXTURE_JOIN").is_some();
        #[cfg(not(feature = "test-support"))]
        let synthetic_fixture_join = false;
        let (expected_verifier, expected_profile) = if qualified_inputs.is_some() {
            if offline {
                (
                    "review-execution-evidence-claude-offline-v1",
                    "claude-offline-peer-v1",
                )
            } else {
                ("review-execution-evidence-claude-v1", "claude-qualified-v1")
            }
        } else if synthetic_fixture_join {
            (
                "review-execution-evidence-fixture-v1",
                "controlled-fixture-v1",
            )
        } else {
            (
                "review-execution-evidence-process-v1",
                "controlled-process-v1",
            )
        };
        if claims_config["root_id"] != document.claims.root_id
            || custody["owner"] != "review-execution-evidence"
            || custody["verifier"] != expected_verifier
            || custody["profile"] != expected_profile
        {
            return Err(refuse(
                "startup",
                "claims custody is not selected controlled E1",
            ));
        }
        let grant = claims_config["grants"]
            .as_array()
            .and_then(|g| {
                g.iter()
                    .find(|g| g["grant_id"] == document.claims.production_grant_id)
            })
            .ok_or_else(|| refuse("startup", "production grant absent"))?;
        if claim_evidence::codec::digest(&claim_evidence::codec::parse_json(&grant.to_string())?)?
            != document.claims.production_grant_sha256
        {
            return Err(refuse("startup", "production grant digest differs"));
        }
        // H1 selects one production grant. Domain bootstrap remains authoritative;
        // this exact subset refuses incompatible startup inputs before any root effect.
        let text = |name: &str| -> Result<String> {
            claims_source[name]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| refuse("startup", format!("claims source {name} absent")))
        };
        let decision_scope = grant["decision_scope"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| refuse("startup", "claim decision scope absent"))?;
        let expected_custody = json!({"owner":"review-execution-evidence","verifier":expected_verifier,"profile":expected_profile,"artifact_schemes":["execution-artifact:"],"reference_schemes":["execution-evidence:"]});
        let custody_hash = claim_evidence::codec::digest(&claim_evidence::codec::parse_json(
            &expected_custody.to_string(),
        )?)?;
        let expected_source = json!({"schema_version":2,"owner":text("owner")?,"reference":text("reference")?,"revision":text("revision")?,"freshness":text("freshness")?,"profile":"native-review-claims-v1","actors":["campaign"],"decision_scopes":[decision_scope],"grant_ids":[document.claims.production_grant_id],"custody_config_sha256":custody_hash});
        let expected_grant = json!({"schema_version":1,"grant_id":document.claims.production_grant_id,"actor":"campaign","profile":"production-path-v1","permissions":["record_observation","establish_claim","read_admission"],"decision_scope":decision_scope,"authority_reference":{"owner":text("owner")?,"reference":text("reference")?,"revision":text("revision")?,"freshness":text("freshness")?,"integrity_sha256":document.claims.source.sha256,"status":"verified"}});
        let expected_config = json!({"schema_version":2,"profile":"native-review-claims-v1","root_id":document.claims.root_id,"trusted_custody":expected_custody,"grants":[expected_grant]});
        if let Some(q) = &qualified_inputs {
            if q.completion_grant_id.is_empty()
                || q.completion_grant_id == document.claims.production_grant_id
                || q.completion_decision_scope != decision_scope
                || q.completion_consumer.is_empty()
                || q.completion_authority_source != expected_grant["authority_reference"]
            {
                return Err(refuse("startup", "qualified J2 selection/source differs"));
            }
            let completion_grant = json!({
                "schema_version":1,"grant_id":q.completion_grant_id,"actor":"campaign",
                "profile":"revision-bound-review-finding-v1",
                "permissions":["create_claim","publish_revision","record_reliance"],
                "decision_scope":q.completion_decision_scope,
                "authority_reference":q.completion_authority_source,
            });
            if claim_evidence::codec::digest(&claim_evidence::codec::parse_json(
                &completion_grant.to_string(),
            )?)? != q.completion_grant_sha256
            {
                return Err(refuse("startup", "qualified J2 grant digest differs"));
            }
            let qualified_source = json!({"schema_version":2,"owner":text("owner")?,"reference":text("reference")?,
                "revision":text("revision")?,"freshness":text("freshness")?,
                "profile":"native-review-claims-v1","actors":["campaign"],
                "decision_scopes":[decision_scope],
                "grant_ids":[document.claims.production_grant_id,q.completion_grant_id],
                "custody_config_sha256":custody_hash});
            let qualified_config = json!({"schema_version":2,"profile":"native-review-claims-v1",
                "root_id":document.claims.root_id,"trusted_custody":expected_custody,
                "grants":[expected_grant,completion_grant]});
            if claims_source != qualified_source || claims_config != qualified_config {
                return Err(refuse(
                    "unsupported",
                    "qualified claims source/config differs",
                ));
            }
        } else if synthetic_fixture_join {
            let completion_grant = json!({"schema_version":1,"grant_id":"h1-completion",
                "actor":"campaign","profile":"revision-bound-review-finding-v1",
                "permissions":["create_claim","publish_revision","record_reliance"],
                "decision_scope":decision_scope,
                "authority_reference":expected_grant["authority_reference"]});
            let synthetic_source = json!({"schema_version":2,"owner":text("owner")?,
                "reference":text("reference")?,"revision":text("revision")?,
                "freshness":text("freshness")?,"profile":"native-review-claims-v1",
                "actors":["campaign"],"decision_scopes":[decision_scope],
                "grant_ids":[document.claims.production_grant_id,"h1-completion"],
                "custody_config_sha256":custody_hash});
            let synthetic_config = json!({"schema_version":2,"profile":"native-review-claims-v1",
                "root_id":document.claims.root_id,"trusted_custody":expected_custody,
                "grants":[expected_grant,completion_grant]});
            if claims_source != synthetic_source || claims_config != synthetic_config {
                return Err(refuse("unsupported", "synthetic claims root differs"));
            }
        } else if claims_source != expected_source || claims_config != expected_config {
            return Err(refuse(
                "unsupported",
                "claims startup differs from H1 single production-grant profile",
            ));
        }
        let request_input: Value = serde_json::from_slice(&input)?;
        let prompt_template =
            qualified_prompt_template_from_parts(&document, &projection, &request_input);
        let qualified = qualified_inputs
            .map(|inputs| QualifiedRuntime::load(&document, inputs, offline, prompt_template))
            .transpose()?;
        let value = Self {
            document,
            path: path.into(),
            digest: sha256_bytes(&bytes),
            authority,
            projection,
            request_input,
            candidate,
            physical_profile,
            selection,
            gate,
            qualified,
        };
        value.campaign_config()?;
        value.validate_native_identifiers()?;
        // SC0 root IDs are 16 random bytes encoded as 32 hex characters. All
        // other unknown owner digests/revisions are fixed-width hex; this
        // descriptive sizing value grants nothing and is never dispatched.
        let sizing = slice_campaign::NativeReviewRequestRef {
            root_id: "0".repeat(32),
            identity: value.document.identity.clone(),
            obligation_id: value.document.obligation_id.clone(),
            operation_id: value.id("prepare"),
            kind: "initial".into(),
            prepared_revision: slice_campaign::CampaignRevision::new("0".repeat(64))?,
            request_digest: "0".repeat(64),
            selection: selection_reference,
            candidate: subject.reference,
            profile_digest: subject.profile_digest,
        };
        value.request_bytes(&sizing)?;
        Ok(value)
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn document(&self) -> &StartupDocument {
        &self.document
    }
    pub(crate) fn revalidate(&self) -> Result<()> {
        let actual = Self::load(&self.path)?;
        if actual.digest != self.digest {
            return Err(refuse("startup", "installed startup changed"));
        }
        Ok(())
    }
    pub(crate) fn campaign_config(&self) -> Result<TrustedConfig> {
        let d = &self.document;
        let writer = format!("{}:{}", d.profile, self.digest);
        if let Some(qualified) = &self.qualified {
            match qualified
                .pin
                .as_ref()
                .ok_or_else(|| refuse("startup", "qualified pin consumed"))?
            {
                QualifiedPin::Installed(pin) => Ok(TrustedConfig::qualified_initial_v4(
                    d.roots.campaign.clone(),
                    d.repository.clone(),
                    d.workspace.clone(),
                    d.identity.clone(),
                    d.accepted_boundary.clone(),
                    d.baseline.clone(),
                    writer,
                    self.gate[0].clone(),
                    self.gate[1].clone(),
                    self.gate[2].clone(),
                    pin,
                )?),
                #[cfg(feature = "test-support")]
                QualifiedPin::Offline(pin) => Ok(TrustedConfig::qualified_initial_offline_v4(
                    d.roots.campaign.clone(),
                    d.repository.clone(),
                    d.workspace.clone(),
                    d.identity.clone(),
                    d.accepted_boundary.clone(),
                    d.baseline.clone(),
                    writer,
                    self.gate[0].clone(),
                    self.gate[1].clone(),
                    self.gate[2].clone(),
                    pin,
                )?),
            }
        } else {
            Ok(TrustedConfig::controlled_sc0(
                d.roots.campaign.clone(),
                d.repository.clone(),
                d.workspace.clone(),
                d.identity.clone(),
                d.accepted_boundary.clone(),
                d.baseline.clone(),
                writer,
                self.gate[0].clone(),
                self.gate[1].clone(),
                self.gate[2].clone(),
            )?)
        }
    }
    pub(crate) fn dispatch_command(&self) -> slice_campaign::DispatchCommand {
        slice_campaign::DispatchCommand {
            episode_id: self
                .authority
                .identity()
                .review_episode_id
                .to_string_checked()
                .expect("validated startup episode"),
            writer_actor: "native-review-host".into(),
            runtime_session: self.document.peer.session.clone(),
            reviewer_profile: self.document.profile.clone(),
            begin_transition_id: self.id("begin"),
            result_transition_id: self.id("result"),
        }
    }
    fn validate_native_identifiers(&self) -> Result<()> {
        let dispatch = self.dispatch_command();
        // These exact transition IDs become both requestId and grantId at the
        // native episode boundary. Check the owner's envelope contract before
        // initialization or dispatch can create durable state. The root digest
        // is only a sizing/validation placeholder; no root or grant is created.
        for (operation, id) in [
            ("begin", dispatch.begin_transition_id),
            ("transition", dispatch.result_transition_id),
        ] {
            let envelope = json!({
                "version": 2,
                "domain": "review-episode",
                "profile": "native-host-v1",
                "requestId": id,
                "operation": operation,
                "grantId": id,
                "selectionRevision": self.digest,
                "rootSelectionDigest": "0".repeat(64),
                "args": {},
            });
            review_episode::protocol::Request::parse(&serde_json::to_vec(&envelope)?).map_err(
                |error| {
                    refuse(
                        "startup",
                        format!("derived native {operation} identifiers: {error}"),
                    )
                },
            )?;
        }
        Ok(())
    }
    pub(crate) fn request_bytes(
        &self,
        request: &slice_campaign::NativeReviewRequestRef,
    ) -> Result<Vec<u8>> {
        let input = json!({"schemaVersion":1,"profile":self.document.profile,"preparedRequest":request,"dispatch":self.dispatch_command(),"compilerMode":self.document.compiler.mode,"compilerProjection":self.projection,"selectedRole":self.document.compiler.role,"input":self.request_input});
        let bytes = work_engine_compiler::canonical_json(&input).into_bytes();
        if bytes.len() > review_execution_evidence::REQUEST_MAX_BYTES {
            return Err(refuse(
                "capacity",
                "composed request exceeds E1 1 MiB limit",
            ));
        }
        Ok(bytes)
    }
    pub(crate) fn id(&self, suffix: &str) -> String {
        format!("{}:{suffix}", self.document.operation_id)
    }
}
