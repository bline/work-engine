//! Offline fixture host. A future production installer must establish distinct
//! operator UID and repository custody before activating this same application route.
use crate::Application;
use claim_evidence::{OpenFiles, RootIdentity};
use review_episode::Application as EpisodeApplication;
use review_episode::in_process_admission::{EpisodeReadTarget, TrustedEpisodeReadScope};
use review_episode_core::{
    codec::parse_json,
    identity::{Authority, Revision},
};
use review_episode_store::{NativeRootSelection, StoreOptions};
use review_execution_custody::test_support::{ControlledFixtureOwner, FixtureRecord};
use review_execution_custody::{
    CheckedObservation, EvidenceReader, EvidenceReaderProfile, ExecutionBinding,
    ExecutionEvidenceOwner, ExecutionProvenance, ReaderPin,
};
use serde::Deserialize;
use serde_json::Value;
use slice_campaign::publication::CampaignPublication;
use slice_campaign::{
    AcceptedBoundary, Baseline, Campaign, CampaignIdentity, CompletionSelection,
    InitialCompletionOwners, InitialJoinSelection, TrustedConfig,
};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::Arc;
use workspace_publication::{AuthorityMode, Workspace, WorkspaceConfig};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FilePin {
    path: PathBuf,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Installed {
    host_build_sha256: String,
    authority_mode: String,
    socket: PathBuf,
    campaign_root: PathBuf,
    repository: PathBuf,
    workspace: PathBuf,
    identity: CampaignIdentity,
    boundary: AcceptedBoundary,
    baseline: Baseline,
    writer_identity: String,
    gate_capture: FilePin,
    gate_manifest: FilePin,
    gate_receipt: FilePin,
    completion_root: PathBuf,
    workspace_root: PathBuf,
    operator_uid: u32,
    authority_source_digest: String,
    canonical_refs: Vec<String>,
    validation_program: Vec<String>,
    validation_policy_digest: String,
    claims_root: PathBuf,
    claims_config: FilePin,
    claims_source: FilePin,
    claims_root_id: String,
    claims_profile: String,
    production_grant_id: String,
    production_grant_sha256: String,
    episode_root: PathBuf,
    episode_executable_sha256: String,
    episode_selection_digest: String,
    episode_authority: FilePin,
    episode_read_revisions: Vec<String>,
    episode_read_transition: Option<ReadTransition>,
    evidence_root: PathBuf,
    evidence_root_id: String,
    evidence_configuration_sha256: String,
    evidence_executable_sha256: String,
    evidence_source_sha256: String,
    evidence_profile: String,
    evidence_owner: String,
    evidence_record_profile: String,
    fixture_evidence: Option<FilePin>,
    completion_selection: InstalledCompletionSelection,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstalledCompletionSelection {
    finding_grant_id: String,
    finding_grant_sha256: String,
    reliance_grant_id: String,
    reliance_grant_sha256: String,
    consumer: String,
    consumer_tree: String,
    decision_scope: String,
    authority_sources: Vec<Value>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadTransition {
    transition_id: String,
    content_digest: String,
}
fn checked_file(pin: &FilePin) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(&pin.path).map_err(|e| e.to_string())?;
    if workspace_publication::codec::raw_sha256(&bytes) != pin.sha256 {
        return Err(format!(
            "installed file digest differs: {}",
            pin.path.display()
        ));
    }
    Ok(bytes)
}
fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("fixture evidence missing {key}"))
}
fn fixture_owner(pin: &FilePin) -> Result<Arc<dyn ExecutionEvidenceOwner>, String> {
    let value: Value = serde_json::from_slice(&checked_file(pin)?).map_err(|e| e.to_string())?;
    let p = &value["provenance"];
    let b = &p["binding"];
    let provenance = ExecutionProvenance {
        binding: ExecutionBinding {
            campaign_root_id: field(b, "campaignRootId")?.into(),
            obligation_id: field(b, "obligationId")?.into(),
            candidate_digest: field(b, "candidateDigest")?.into(),
            selection_digest: field(b, "selectionDigest")?.into(),
            profile_digest: field(b, "profileDigest")?.into(),
            execution_profile_digest: b["executionProfileDigest"].as_str().map(str::to_string),
            prepared_request_sha256: field(b, "preparedRequestSha256")?.into(),
            review_episode_id: field(b, "reviewEpisodeId")?.into(),
            attempt_id: field(b, "attemptId")?.into(),
        },
        dispatch_operation_id: field(p, "dispatchOperationId")?.into(),
        dispatch_revision: field(p, "dispatchRevision")?.into(),
        execution_session_id: p["executionSessionId"].as_str().map(str::to_string),
    };
    let observation = CheckedObservation::Present(
        claim_evidence::codec::parse_json(field(&value, "observationCanonicalJson")?)
            .map_err(|e| e.to_string())?,
    );
    let artifacts = value["artifactReferences"]
        .as_array()
        .ok_or("fixture artifact references missing")?
        .iter()
        .map(|x| {
            x.as_str()
                .map(str::to_string)
                .ok_or("fixture artifact reference malformed".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let record = FixtureRecord::controlled(
        field(&value, "rootId")?,
        field(&value, "recordId")?,
        field(&value, "revision")?,
        field(&value, "rawResult")?.as_bytes().to_vec(),
        provenance,
        observation,
        artifacts,
    )
    .map_err(|e| e.to_string())?;
    if record.reference.sha256 != field(&value, "expectedReferenceSha256")? {
        return Err("fixture evidence reference differs".into());
    }
    Ok(Arc::new(
        ControlledFixtureOwner::new(vec![record]).map_err(|e| e.to_string())?,
    ))
}
pub fn run_fixture(config_path: PathBuf) -> Result<(), String> {
    if !config_path.is_absolute() {
        return Err("installed config path must be absolute".into());
    }
    let installed: Installed =
        serde_json::from_slice(&std::fs::read(&config_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if installed.authority_mode != "fixture_operator" {
        return Err("production authority/custody installation is unavailable".into());
    }
    let binary = std::env::current_exe().map_err(|e| e.to_string())?;
    if workspace_publication::codec::raw_sha256(&std::fs::read(binary).map_err(|e| e.to_string())?)
        != installed.host_build_sha256
    {
        return Err("host build pin differs".into());
    }
    if installed.operator_uid != rustix::process::getuid().as_raw() {
        return Err("fixture operator UID differs from host UID".into());
    }
    let trusted = TrustedConfig::remediation_v4(
        installed.campaign_root.clone(),
        installed.repository.clone(),
        installed.workspace.clone(),
        installed.identity.clone(),
        installed.boundary.clone(),
        installed.baseline.clone(),
        installed.writer_identity.clone(),
        checked_file(&installed.gate_capture)?,
        checked_file(&installed.gate_manifest)?,
        checked_file(&installed.gate_receipt)?,
    )
    .map_err(|e| e.to_string())?;
    let mut campaign = Campaign::open(trusted).map_err(|e| e.to_string())?;
    let authority_text = String::from_utf8(checked_file(&installed.episode_authority)?)
        .map_err(|e| e.to_string())?;
    let authority = Authority::parse(parse_json(&authority_text).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let native = NativeRootSelection {
        root: installed.episode_root.to_string_lossy().into(),
        executable_sha256: installed.episode_executable_sha256.clone(),
        selection_digest: installed.episode_selection_digest.clone(),
    };
    let mut read_targets = installed
        .episode_read_revisions
        .iter()
        .map(|s| {
            let v = review_episode_core::codec::JsValue::text(s);
            Revision::parse(&v, "installed read revision")
                .map(EpisodeReadTarget::Revision)
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(t) = &installed.episode_read_transition {
        let digest = Revision::parse(
            &review_episode_core::codec::JsValue::text(&t.content_digest),
            "installed transition digest",
        )
        .map_err(|e| e.to_string())?;
        read_targets.push(EpisodeReadTarget::Transition {
            transition_id: review_episode_core::codec::JsString::new(&t.transition_id),
            content_digest: digest,
        });
    }
    let scope =
        TrustedEpisodeReadScope::new(authority.clone(), read_targets).map_err(|e| e.to_string())?;
    let mut episode = EpisodeApplication::open_read_only(
        &installed.episode_root,
        native.clone(),
        scope,
        StoreOptions::default(),
        1_048_576,
    )
    .map_err(|e| e.to_string())?;
    let evidence_profile = match installed.evidence_profile.as_str() {
        "controlled_fixture" => EvidenceReaderProfile::ControlledFixture,
        "controlled_process" => EvidenceReaderProfile::ControlledProcess,
        "qualified_provider" => EvidenceReaderProfile::QualifiedProvider,
        _ => return Err("evidence profile unsupported".into()),
    };
    let evidence: Arc<dyn ExecutionEvidenceOwner> = if evidence_profile
        == EvidenceReaderProfile::ControlledFixture
    {
        fixture_owner(
            installed
                .fixture_evidence
                .as_ref()
                .ok_or("fixture evidence pin absent")?,
        )?
    } else {
        let pin = ReaderPin {
            root_id: installed.evidence_root_id.clone(),
            configuration_sha256: installed.evidence_configuration_sha256.clone(),
            executable_sha256: installed.evidence_executable_sha256.clone(),
            source_sha256: installed.evidence_source_sha256.clone(),
            profile: installed.evidence_record_profile.clone(),
        };
        Arc::new(EvidenceReader::open(&installed.evidence_root, pin).map_err(|e| e.to_string())?)
    };
    let selection = InitialJoinSelection {
        claims_root: RootIdentity {
            absolute_path: installed.claims_root.clone(),
            root_id: installed.claims_root_id.clone(),
            profile: installed.claims_profile.clone(),
        },
        production_grant_id: installed.production_grant_id.clone(),
        production_grant_sha256: installed.production_grant_sha256.clone(),
        episode_selection: native,
        episode_authority: authority,
        evidence_profile,
        evidence_owner: installed.evidence_owner.clone(),
        evidence_root_id: installed.evidence_root_id.clone(),
        evidence_record_profile: installed.evidence_record_profile.clone(),
    };
    let claims_files = OpenFiles {
        root: installed.claims_root.clone(),
        config: installed.claims_config.path.clone(),
        source: installed.claims_source.path.clone(),
        expected_config_sha256: installed.claims_config.sha256.clone(),
        expected_source_sha256: installed.claims_source.sha256.clone(),
    };
    let mut claims = campaign
        .open_initial_claims(claims_files, selection, evidence)
        .map_err(|e| e.to_string())?;
    let selected = &installed.completion_selection;
    campaign
        .install_completion_selection(
            CompletionSelection::new(
                claims.claims().root().clone(),
                selected.finding_grant_id.clone(),
                selected.finding_grant_sha256.clone(),
                selected.reliance_grant_id.clone(),
                selected.reliance_grant_sha256.clone(),
                selected.consumer.clone(),
                selected.consumer_tree.clone(),
                selected.decision_scope.clone(),
                selected.authority_sources.clone(),
            )
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    let mut owners = InitialCompletionOwners::new(&mut claims, Some(&mut episode));
    let completion = if installed.completion_root.exists() {
        CampaignPublication::open(
            &mut campaign,
            &installed.completion_root,
            installed.operator_uid,
            installed.authority_source_digest.clone(),
            true,
        )
    } else {
        CampaignPublication::initialize(
            &mut campaign,
            &installed.completion_root,
            installed.operator_uid,
            installed.authority_source_digest.clone(),
            true,
        )
    }
    .map_err(|e| e.to_string())?;
    let workspace_config = WorkspaceConfig {
        root: installed.workspace_root,
        repository: installed.repository,
        canonical_refs: installed.canonical_refs,
        validation_program: installed.validation_program,
        validation_policy_digest: installed.validation_policy_digest,
        authority_mode: AuthorityMode::FixtureOperator,
    };
    let workspace = if workspace_config.root.exists() {
        Workspace::open(workspace_config)
    } else {
        Workspace::initialize(workspace_config)
    }
    .map_err(|e| e.to_string())?;
    let mut app = Application {
        campaign: completion,
        owners: &mut owners,
        workspace,
    };
    if installed.socket.exists() {
        return Err("socket path already exists".into());
    }
    let listener = UnixListener::bind(&installed.socket).map_err(|e| e.to_string())?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&installed.socket, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| e.to_string())?;
    for peer in listener.incoming() {
        let peer = peer.map_err(|e| e.to_string())?;
        app.serve_connection(peer).map_err(|e| e.to_string())?;
    }
    Ok(())
}
