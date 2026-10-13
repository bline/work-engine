//! Startup data is selected by the trusted launcher, never by a wire command.
use crate::{Result, refuse};
use claim_evidence::sha256_bytes;
use review_episode_core::{
    codec::{digest, parse_json},
    identity::Authority,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use slice_campaign::{AcceptedBoundary, Baseline, CampaignIdentity, TrustedConfig};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

pub const PROFILE: &str = "review-host-controlled-initial-v1";
pub const STARTUP_LIMIT: usize = 1024 * 1024;
const INPUT_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilePin {
    pub path: PathBuf,
    pub sha256: String,
    pub mode: u32,
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
        let mut bytes = Vec::new();
        File::open(path)?
            .take((STARTUP_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > STARTUP_LIMIT {
            return Err(refuse("capacity", "startup exceeds limit"));
        }
        let document: StartupDocument = serde_json::from_slice(&bytes)?;
        if document.schema_version != 1
            || document.profile != PROFILE
            || document.evidence_profile != "controlled-process-v1"
        {
            return Err(refuse(
                "unsupported",
                "only the controlled SC0 initial profile is supported",
            ));
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
        let subject = slice_campaign::verify_existing_subject(
            &document.repository,
            &document.identity,
            &document.baseline,
            &candidate,
            &physical_profile,
            &gate[0],
            &gate[1],
            &gate[2],
        )?;
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
        if claims_config["root_id"] != document.claims.root_id
            || custody["owner"] != "review-execution-evidence"
            || custody["verifier"] != "review-execution-evidence-process-v1"
            || custody["profile"] != "controlled-process-v1"
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
        let expected_custody = json!({"owner":"review-execution-evidence","verifier":"review-execution-evidence-process-v1","profile":"controlled-process-v1","artifact_schemes":["execution-artifact:"],"reference_schemes":["execution-evidence:"]});
        let custody_hash = claim_evidence::codec::digest(&claim_evidence::codec::parse_json(
            &expected_custody.to_string(),
        )?)?;
        let expected_source = json!({"schema_version":2,"owner":text("owner")?,"reference":text("reference")?,"revision":text("revision")?,"freshness":text("freshness")?,"profile":"native-review-claims-v1","actors":["campaign"],"decision_scopes":[decision_scope],"grant_ids":[document.claims.production_grant_id],"custody_config_sha256":custody_hash});
        let expected_grant = json!({"schema_version":1,"grant_id":document.claims.production_grant_id,"actor":"campaign","profile":"production-path-v1","permissions":["record_observation","establish_claim","read_admission"],"decision_scope":decision_scope,"authority_reference":{"owner":text("owner")?,"reference":text("reference")?,"revision":text("revision")?,"freshness":text("freshness")?,"integrity_sha256":document.claims.source.sha256,"status":"verified"}});
        let expected_config = json!({"schema_version":2,"profile":"native-review-claims-v1","root_id":document.claims.root_id,"trusted_custody":expected_custody,"grants":[expected_grant]});
        if claims_source != expected_source || claims_config != expected_config {
            return Err(refuse(
                "unsupported",
                "claims startup differs from H1 single production-grant profile",
            ));
        }
        let request_input: Value = serde_json::from_slice(&input)?;
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
        };
        value.campaign_config()?;
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
        Ok(TrustedConfig::controlled_sc0(
            d.roots.campaign.clone(),
            d.repository.clone(),
            d.workspace.clone(),
            d.identity.clone(),
            d.accepted_boundary.clone(),
            d.baseline.clone(),
            format!("{PROFILE}:{}", self.digest),
            self.gate[0].clone(),
            self.gate[1].clone(),
            self.gate[2].clone(),
        )?)
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
            reviewer_profile: PROFILE.into(),
            begin_transition_id: self.id("begin"),
            result_transition_id: self.id("result"),
        }
    }
    pub(crate) fn request_bytes(
        &self,
        request: &slice_campaign::NativeReviewRequestRef,
    ) -> Result<Vec<u8>> {
        let input = json!({"schemaVersion":1,"profile":PROFILE,"preparedRequest":request,"dispatch":self.dispatch_command(),"compilerMode":self.document.compiler.mode,"compilerProjection":self.projection,"selectedRole":self.document.compiler.role,"input":self.request_input});
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
