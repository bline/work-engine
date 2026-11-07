//! Installed completion composition. Deployment data selects existing owners;
//! it cannot issue review, claims, acceptance, or publication authority.
use crate::{Result, Workflow, refuse};
use claim_evidence::sha256_bytes;
use review_episode::{
    Application as EpisodeApplication,
    in_process_admission::{EpisodeReadTarget, TrustedEpisodeReadScope},
    protocol::DEFAULT_RESPONSE_LIMIT,
};
use review_episode_core::{codec::JsString, identity::Revision};
use review_episode_store::StoreOptions;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use slice_campaign::{InitialCompletionOwners, StageKind, publication::CampaignPublication};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        net::UnixListener,
    },
    path::{Path, PathBuf},
};
use workspace_publication::{Workspace, custody::ManagedCustody};

pub const INSTALLED_SELECTION: &str = "/etc/work-engine/review-host/completion.json";
const LIMIT: usize = 64 * 1024;

/// An inert proposal. Only the fixed protected loader below confers deployment
/// standing, and the workflow still requires all underlying owner authority.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletionSelection {
    pub schema_version: u32,
    pub startup_digest: String,
    pub runtime_sha256: String,
    pub completion_root: PathBuf,
    pub workspace_root: PathBuf,
    pub repository: PathBuf,
    pub custody_sha256: String,
    pub socket: PathBuf,
    pub operator_uid: u32,
    pub authority_source_digest: String,
}

impl CompletionSelection {
    /// Mechanical proposal validation supplies no installed standing.
    pub fn validate_proposal(&self) -> Result<()> {
        let sha = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if self.schema_version != 1
            || ![
                &self.startup_digest,
                &self.runtime_sha256,
                &self.authority_source_digest,
                &self.custody_sha256,
            ]
            .into_iter()
            .all(|s| sha(s))
            || ![
                &self.completion_root,
                &self.workspace_root,
                &self.repository,
                &self.socket,
            ]
            .into_iter()
            .all(|p| p.is_absolute())
        {
            return Err(refuse(
                "completion_installation",
                "unsupported completion selection",
            ));
        }
        Ok(())
    }
}

struct InstalledSelection {
    value: CompletionSelection,
    bytes: Vec<u8>,
}

fn protected(path: &Path) -> Result<()> {
    if !path.is_absolute() || fs::canonicalize(path)? != path {
        return Err(refuse(
            "completion_installation",
            "noncanonical installed path",
        ));
    }
    for ancestor in path.ancestors() {
        let meta = fs::symlink_metadata(ancestor)?;
        if meta.uid() != 0 || meta.mode() & 0o022 != 0 {
            return Err(refuse(
                "completion_installation",
                "unprotected installation ancestry",
            ));
        }
    }
    Ok(())
}

fn read_selection(path: &Path) -> Result<Vec<u8>> {
    protected(path)?;
    let before = fs::symlink_metadata(path)?;
    if !before.is_file() || before.len() > LIMIT as u64 {
        return Err(refuse(
            "completion_installation",
            "invalid selection type or size",
        ));
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC).bits() as i32)
        .open(path)?;
    let opened = file.metadata()?;
    if (before.dev(), before.ino(), before.uid(), before.mode())
        != (opened.dev(), opened.ino(), opened.uid(), opened.mode())
    {
        return Err(refuse(
            "completion_installation",
            "selection changed while opening",
        ));
    }
    let mut bytes = Vec::new();
    file.take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    let after = fs::symlink_metadata(path)?;
    if bytes.len() > LIMIT
        || (
            before.dev(),
            before.ino(),
            before.len(),
            before.mtime(),
            before.mtime_nsec(),
        ) != (
            after.dev(),
            after.ino(),
            after.len(),
            after.mtime(),
            after.mtime_nsec(),
        )
    {
        return Err(refuse(
            "completion_installation",
            "selection changed while reading",
        ));
    }
    Ok(bytes)
}

impl InstalledSelection {
    fn load() -> Result<Self> {
        // Even feature-unified binaries cannot activate an offline/fault route.
        if cfg!(feature = "test-support") || cfg!(feature = "test-faults") {
            return Err(refuse(
                "completion_installation",
                "test build cannot serve production completion",
            ));
        }
        let bytes = read_selection(Path::new(INSTALLED_SELECTION))?;
        let value: CompletionSelection = serde_json::from_slice(&bytes)?;
        value.validate_proposal()?;
        Ok(Self { value, bytes })
    }
    fn custody(&self) -> Result<ManagedCustody> {
        let path = Path::new(workspace_publication::custody::INSTALLED_SELECTION);
        let bytes = read_selection(path)?;
        if sha256_bytes(&bytes) != self.value.custody_sha256 {
            return Err(refuse(
                "completion_installation",
                "installed custody selection differs",
            ));
        }
        let selected: workspace_publication::custody::CustodySelection =
            serde_json::from_slice(&bytes)?;
        if selected.repository != self.value.repository
            || selected.root != self.value.workspace_root
        {
            return Err(refuse(
                "completion_installation",
                "installed custody owner paths differ",
            ));
        }
        let custody =
            ManagedCustody::load_installed().map_err(|e| refuse("workspace", e.to_string()))?;
        if read_selection(path)? != bytes {
            return Err(refuse(
                "completion_installation",
                "custody changed while opening",
            ));
        }
        Ok(custody)
    }
    fn revalidate(&self, workflow: &Workflow) -> Result<()> {
        if read_selection(Path::new(INSTALLED_SELECTION))? != self.bytes {
            return Err(refuse(
                "completion_installation",
                "installed completion selection changed",
            ));
        }
        self.check_workflow(workflow)
    }
    fn check_workflow(&self, workflow: &Workflow) -> Result<()> {
        let d = &workflow.startup.document;
        if !workflow
            .startup
            .qualified
            .as_ref()
            .is_some_and(|r| r.installed())
            || self.value.startup_digest != workflow.startup.digest()
            || self.value.runtime_sha256 != d.host.executable.sha256
            || self.value.authority_source_digest != d.claims.source.sha256
        {
            return Err(refuse(
                "completion_installation",
                "completion startup or authority differs",
            ));
        }
        let executable = std::env::current_exe()?;
        protected(&executable)?;
        if sha256_bytes(&fs::read(executable)?) != self.value.runtime_sha256 {
            return Err(refuse(
                "completion_installation",
                "installed completion executable differs",
            ));
        }
        let root = &self.value.completion_root;
        if root.parent() != Some(d.roots.campaign.as_path())
            || root.file_name().and_then(|v| v.to_str()) != Some("publication")
        {
            return Err(refuse(
                "completion_installation",
                "completion root must be campaign/publication",
            ));
        }
        check_socket_parent(&self.value.socket)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_proposal_is_strict_data_and_not_fixture_authority() {
        let value = json!({"schemaVersion":1,"startupDigest":"a".repeat(64),
            "runtimeSha256":"b".repeat(64),"authoritySourceDigest":"c".repeat(64),
            "custodySha256":"d".repeat(64),"completionRoot":"/campaign/publication",
            "workspaceRoot":"/workspace","repository":"/canonical.git","socket":"/operator/completion.sock","operatorUid":1000});
        let proposal: CompletionSelection = serde_json::from_value(value.clone()).unwrap();
        proposal.validate_proposal().unwrap();
        for key in [
            "fixtureOperator",
            "authorityMode",
            "approve",
            "installedAcceptancePin",
        ] {
            let mut forged = value.clone();
            forged[key] = json!(true);
            assert!(serde_json::from_value::<CompletionSelection>(forged).is_err());
        }
        let mut stale = proposal.clone();
        stale.schema_version = 2;
        assert!(stale.validate_proposal().is_err());
        stale = proposal.clone();
        stale.startup_digest = "caller-selected".into();
        assert!(stale.validate_proposal().is_err());
        stale = proposal;
        stale.socket = "relative/socket".into();
        assert!(stale.validate_proposal().is_err());
    }

    #[test]
    fn selection_loader_refuses_unprotected_or_symlinked_installation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("completion.json");
        fs::write(&path, b"{}").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        assert!(read_selection(&path).is_err());
        let link = temp.path().join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(read_selection(&link).is_err());
        assert!(read_selection(&temp.path().join("missing")).is_err());
    }
}

fn check_socket_parent(socket: &Path) -> Result<()> {
    let parent = socket
        .parent()
        .ok_or_else(|| refuse("completion_installation", "socket parent absent"))?;
    if !socket.is_absolute() || socket.file_name().is_none() || fs::canonicalize(parent)? != parent
    {
        return Err(refuse(
            "completion_installation",
            "socket parent is not canonical",
        ));
    }
    // A private directory protects the socket name as well as its contents.
    // Runtime UID is checked independently by installed workspace custody.
    let owner = fs::metadata(parent)?;
    if owner.mode() & 0o077 != 0 || owner.uid() != rustix::process::geteuid().as_raw() {
        return Err(refuse(
            "completion_installation",
            "operator socket directory is not private",
        ));
    }
    for ancestor in parent.ancestors() {
        let meta = fs::symlink_metadata(ancestor)?;
        if meta.mode() & 0o022 != 0 || (meta.uid() != 0 && meta.uid() != owner.uid()) {
            return Err(refuse(
                "completion_installation",
                "operator socket ancestry is writable or foreign",
            ));
        }
    }
    Ok(())
}

impl Workflow {
    /// Allocate only publication-owner state under an initialize-phase custody
    /// selection. Activation requires a separately sealed operator selection.
    pub fn initialize_completion(mut self) -> Result<Value> {
        let installation = InstalledSelection::load()?;
        self.startup.revalidate()?;
        installation.revalidate(&self)?;
        let workspace = Workspace::initialize_managed(installation.custody()?)
            .map_err(|e| refuse("workspace", e.to_string()))?;
        let value = &installation.value;
        let _publication = CampaignPublication::initialize(
            &mut self.campaign,
            &value.completion_root,
            value.operator_uid,
            value.authority_source_digest.clone(),
            false,
        )?;
        Ok(
            json!({"status":"completion_initialized","workspaceRootId":workspace.root_id(),
            "repositoryId":workspace.repository_id(),"completionRoot":value.completion_root,
            "startupDigest":self.startup.digest(),"active":false}),
        )
    }

    /// Reopen durable owner readbacks once in a freshly consumed workflow. No
    /// review launch, result transition write, or J1/J2 synthesis occurs here.
    pub fn serve_completion(mut self) -> Result<()> {
        let installation = InstalledSelection::load()?;
        self.startup.revalidate()?;
        installation.revalidate(&self)?;
        let (mut claims, mut episode) = self.reopen_completion_owners()?;
        let workspace = Workspace::open_managed(installation.custody()?)
            .map_err(|e| refuse("workspace", e.to_string()))?;
        let value = &installation.value;
        let mut owners = InitialCompletionOwners::new(&mut claims, Some(&mut episode));
        let publication = CampaignPublication::open(
            &mut self.campaign,
            &value.completion_root,
            value.operator_uid,
            value.authority_source_digest.clone(),
            false,
        )?;
        let mut application = work_engine_completion_host::Application {
            campaign: publication,
            owners: &mut owners,
            workspace,
        };
        // Never unlink a pre-existing name: it might be another live host or a
        // recovery signal. An operator inspects/removes stale sockets explicitly.
        let listener = UnixListener::bind(&value.socket)?;
        fs::set_permissions(&value.socket, fs::Permissions::from_mode(0o600))?;
        for peer in listener.incoming() {
            let peer = peer?;
            let startup = &self.startup;
            application
                .serve_connection_checked(peer, || {
                    let check = || -> Result<()> {
                        startup.revalidate()?;
                        if read_selection(Path::new(INSTALLED_SELECTION))? != installation.bytes {
                            return Err(refuse(
                                "completion_installation",
                                "installed completion selection changed",
                            ));
                        }
                        check_socket_parent(&value.socket)?;
                        Ok(())
                    };
                    check().map_err(|e| work_engine_completion_host::Error::Wire(e.to_string()))
                })
                .map_err(|e| refuse("completion", e.to_string()))?;
        }
        Ok(())
    }

    #[cfg(feature = "test-support")]
    pub fn inspect_completion_fixture(mut self) -> Result<Value> {
        if self
            .startup
            .qualified
            .as_ref()
            .is_some_and(|r| r.installed())
        {
            return Err(refuse(
                "unsupported",
                "fixture inspection refuses installed startup",
            ));
        }
        self.startup.revalidate()?;
        let (mut claims, mut episode) = self.reopen_completion_owners()?;
        let d = &self.startup.document;
        let locator = self
            .campaign
            .locate_initial_outcome(&d.identity, &d.obligation_id, &self.startup.id("outcome"))?
            .ok_or_else(|| refuse("unresolved", "original initial outcome absent"))?;
        let outcome = self.campaign.recover_initial_outcome(
            &locator,
            &mut InitialCompletionOwners::new(&mut claims, Some(&mut episode)),
        )?;
        let slice_campaign::InitialOutcomeState::Committed { receipt, .. } = outcome.state else {
            return Err(refuse("unresolved", "initial outcome is not committed"));
        };
        Ok(json!({"standing":"fixture_owner_readback_only","receipt":receipt,"locator":locator}))
    }

    fn reopen_completion_owners(
        &mut self,
    ) -> Result<(
        slice_campaign::InitialClaimsOwner,
        EpisodeApplication<review_episode::in_process_admission::InProcessReadAdmission>,
    )> {
        let d = self.startup.document.clone();
        let snapshot = self.snapshot()?;
        let prepared = snapshot
            .obligations
            .iter()
            .find(|o| o.obligation_id == d.obligation_id)
            .and_then(|o| o.request.as_ref())
            .ok_or_else(|| refuse("unresolved", "original prepared request absent"))?;
        let intent = snapshot
            .progress
            .attempts
            .iter()
            .find(|a| a.attempt_id == d.identity.attempt_id)
            .and_then(|a| a.completion_intent.as_ref())
            .ok_or_else(|| refuse("unresolved", "original completion intent absent"))?;
        if intent.outcome.is_none() {
            return Err(refuse(
                "unresolved",
                "initial outcome must be recovered by the review owner first",
            ));
        }
        let stage = intent
            .stages
            .iter()
            .find(|s| s.kind == StageKind::EpisodeResultRead)
            .ok_or_else(|| refuse("unresolved", "registered episode read absent"))?;
        let transition = stage.owner_locator["transitionId"]
            .as_str()
            .ok_or_else(|| refuse("unresolved", "registered result transition absent"))?;
        let content_digest = stage.owner_locator["contentDigest"]
            .as_str()
            .ok_or_else(|| refuse("unresolved", "registered result digest absent"))?;
        let dispatch = self.dispatch_receipt(prepared)?;
        let (reader, provenance, evidence_profile, record_profile) =
            self.initial_reader(prepared, dispatch.result_revision.as_str())?;
        let r = &intent.execution_evidence_ref;
        let reference = review_execution_evidence::ExecutionEvidenceRef {
            schema_version: r.schema_version,
            owner: r.owner.clone(),
            root_id: r.root_id.clone(),
            profile: r.profile.clone(),
            attempt_id: r.attempt_id.clone(),
            record_id: r.record_id.clone(),
            revision: r.revision.clone(),
            sha256: r.sha256.clone(),
        };
        let checked = reader.read_result(&reference)?;
        evidence_profile.require_class(checked.evidence_class())?;
        if checked.provenance() != &provenance {
            return Err(refuse(
                "conflict",
                "completion execution provenance differs",
            ));
        }
        let (claims, native) =
            self.initial_claims(prepared, reader, evidence_profile, record_profile, false)?;
        let scope = TrustedEpisodeReadScope::new(
            self.startup.authority.clone(),
            vec![EpisodeReadTarget::Transition {
                transition_id: JsString::new(transition),
                content_digest: Revision(content_digest.into()),
            }],
        )?;
        let episode = EpisodeApplication::open_read_only(
            Path::new(&native.root),
            native.clone(),
            scope,
            StoreOptions::default(),
            DEFAULT_RESPONSE_LIMIT,
        )?;
        Ok((claims, episode))
    }
}
