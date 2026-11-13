//! Sealed, cooperative startup for one disposable productive Rust task host.
//! This does not exclude independently controlled older executables.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

use lifecycle_service::{SelectedLifecycleStore, SelectedRestartRequest};
use retained_task_workflow::TaskStore;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::{HostError, HostStartup, current_uid, validate_startup};

const MAX_SELECTION_BYTES: u64 = 65_536;
const MAX_CONFIG_BYTES: u64 = 131_072;
const MAX_STATE_BYTES: u64 = 16_384;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    version: u32,
    selection_id: String,
    /// A stopped selected host may name its exact predecessor to select a
    /// compatible binary against the same current stores.
    #[serde(default)]
    predecessor_selection_path: Option<PathBuf>,
    /// Version 3 alone names an operator-authored, exact restart declaration.
    #[serde(default)]
    restart_declaration_path: Option<PathBuf>,
    executable_path: PathBuf,
    executable_sha256: String,
    config_path: PathBuf,
    config_sha256: String,
    task_store_path: PathBuf,
    lifecycle_store_root: PathBuf,
    host_socket_path: PathBuf,
    lifecycle_socket_path: PathBuf,
    build_id: String,
    features: Vec<String>,
    provider_identity: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct Intent {
    version: u32,
    selection_sha256: String,
    task_store_path: PathBuf,
    lifecycle_store_root: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct StoreStamp {
    path: PathBuf,
    store_id: String,
    device: u64,
    inode: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct Receipt {
    version: u32,
    selection_sha256: String,
    task: StoreStamp,
    lifecycle: StoreStamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    restart_intent_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    predecessor_receipt_sha256: Option<String>,
}

/// The operator binds one cooperative stop observation to one selected
/// predecessor. Its exact bytes are the store's restart-intent digest.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RestartDeclaration {
    version: u32,
    selection_sha256: String,
    predecessor_selection_sha256: String,
    predecessor_receipt_sha256: String,
    task: StoreStamp,
    lifecycle: StoreStamp,
    subject_id: String,
    context_generation: String,
    task_id: String,
    task_revision: u64,
    task_cursor_revision: u64,
    predecessor_session_id: String,
    predecessor_epoch: u64,
    target_session_id: String,
    expected_subject_revision: u64,
    expected_semantic_revision: u64,
    expected_queue_revision: u64,
    expected_journal_cursor: u64,
    profile_digest: String,
    binary_sha256: String,
    protocol_digest: String,
    snapshot_id: String,
    snapshot_manifest_sha256: String,
    operator_ref: String,
    cooperative_stop_asserted: bool,
    predecessor_host_pid: u32,
    predecessor_host_starttime_ticks: String,
    #[serde(default)]
    predecessor_native_pid: Option<u32>,
    #[serde(default)]
    predecessor_native_starttime_ticks: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectedIdentity {
    pub selection_path: PathBuf,
    pub startup_path: PathBuf,
    pub selection_digest: String,
    pub startup_digest: String,
    pub startup_bytes: Vec<u8>,
    pub selection_bytes: Vec<u8>,
}
pub(crate) struct Prepared {
    pub startup: HostStartup,
    pub identity: SelectedIdentity,
    pub research_admission: Option<crate::research_installation::AuthorizedResearchStartup>,
    pub task_store: TaskStore,
    pub lifecycle_store: SelectedLifecycleStore,
    // A second selected process cannot race this store/receipt handoff.
    pub selection_lock: File,
}

pub(crate) fn prepare(path: &Path) -> Result<Prepared, HostError> {
    prepare_with_operator(path, None)
}
pub(crate) fn prepare_with_operator(
    path: &Path,
    operator_input: Option<&Path>,
) -> Result<Prepared, HostError> {
    let directory = path.parent().ok_or(HostError::Configuration)?;
    if !canonical_exact(directory)? || !canonical_exact(path)? {
        return Err(HostError::Configuration);
    }
    let selection_bytes = read_private(path, MAX_SELECTION_BYTES)?;
    let selection: Selection =
        serde_json::from_slice(&selection_bytes).map_err(|_| HostError::Configuration)?;
    if !matches!(selection.version, 1..=3)
        || (selection.version == 1
            && (selection.predecessor_selection_path.is_some()
                || selection.restart_declaration_path.is_some()))
        || (selection.version == 2
            && (selection.predecessor_selection_path.is_none()
                || selection.restart_declaration_path.is_some()))
        || (selection.version == 3
            && (selection.predecessor_selection_path.is_none()
                || selection.restart_declaration_path.is_none()))
        || selection.selection_id.is_empty()
        || selection.selection_id.len() > 128
        || !valid_sha(&selection.executable_sha256)
        || !valid_sha(&selection.config_sha256)
        || selection.config_path.parent() != Some(directory)
        || selection.task_store_path.parent() != Some(directory)
        || selection.lifecycle_store_root.parent() != Some(directory)
        || selection.host_socket_path.parent() != Some(directory)
        || selection.lifecycle_socket_path.parent() != Some(directory)
        || !canonical_exact(&selection.executable_path)?
        || !canonical_exact(&selection.config_path)?
        || !fresh_or_canonical(&selection.task_store_path)?
        || !fresh_or_canonical(&selection.lifecycle_store_root)?
        || selection.host_socket_path == selection.lifecycle_socket_path
    {
        return Err(HostError::Configuration);
    }
    let executable = std::env::current_exe()?;
    if executable != selection.executable_path
        || sha_file(&executable)? != selection.executable_sha256
    {
        return Err(HostError::Configuration);
    }
    let config_bytes = read_private(&selection.config_path, MAX_CONFIG_BYTES)?;
    if sha_bytes(&config_bytes) != selection.config_sha256 {
        return Err(HostError::Configuration);
    }
    let startup: HostStartup =
        serde_json::from_slice(&config_bytes).map_err(|_| HostError::Configuration)?;
    validate_startup(&startup)?;
    let directory_meta = fs::metadata(directory)?;
    if directory_meta.uid() != current_uid()?
        || directory_meta.permissions().mode() & 0o777 != 0o700
        || startup.executable_sha256 != selection.executable_sha256
        || startup.task_store_path != selection.task_store_path
        || startup.lifecycle.store_root != selection.lifecycle_store_root
        || startup.socket_path != selection.host_socket_path
        || startup.lifecycle.socket_path != selection.lifecycle_socket_path
        || startup.lifecycle.build_id != selection.build_id
        || startup.features != selection.features
        || startup.provider_identity != selection.provider_identity
    {
        return Err(HostError::Configuration);
    }

    let identity = SelectedIdentity {
        selection_path: path.to_path_buf(),
        startup_path: selection.config_path.clone(),
        selection_digest: sha_bytes(&selection_bytes),
        startup_digest: sha_bytes(&config_bytes),
        startup_bytes: config_bytes.clone(),
        selection_bytes: selection_bytes.clone(),
    };
    let research_admission = if let Some(input) = operator_input {
        Some(crate::research_installation::admit_external_operator(
            &startup, &identity, input,
        )?)
    } else {
        crate::research_installation::refuse_unadmitted(&startup)?;
        None
    };
    // Read-only record preflight precedes every startup effect. Locks/CAS still
    // arbitrate races at the owner boundary; they do not excuse malformed input.
    if selection.version == 3 {
        read_restart_declaration(
            path,
            directory,
            &selection,
            &startup,
            &identity.selection_digest,
            &directory.join("startup-receipt.json"),
        )?;
    }
    if selection.version != 3 && exists_no_follow(&directory.join("startup-receipt.tmp"))? {
        return Err(HostError::Configuration);
    }
    let lock_path = directory.join("startup.lock");
    if exists_no_follow(&lock_path)? && !canonical_exact(&lock_path)? {
        return Err(HostError::Configuration);
    }
    let selection_lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    selection_lock.set_permissions(fs::Permissions::from_mode(0o600))?;
    selection_lock
        .try_lock()
        .map_err(|_| HostError::Configuration)?;

    let selection_sha256 = sha_bytes(&selection_bytes);
    let intent_path = directory.join("startup-intent.json");
    let receipt_path = directory.join("startup-receipt.json");
    if selection.version != 3 && exists_no_follow(&directory.join("startup-receipt.tmp"))? {
        return Err(HostError::Configuration);
    }
    if selection.version == 3 {
        return prepare_restart(
            path,
            directory,
            &selection,
            &startup,
            &selection_sha256,
            &intent_path,
            &receipt_path,
            selection_lock,
            identity,
            research_admission,
        );
    }
    let expected_intent = Intent {
        version: 1,
        selection_sha256: selection_sha256.clone(),
        task_store_path: selection.task_store_path.clone(),
        lifecycle_store_root: selection.lifecycle_store_root.clone(),
    };
    let receipt_exists = exists_no_follow(&receipt_path)?;
    if receipt_exists
        && (!exists_no_follow(&selection.task_store_path)?
            || !exists_no_follow(&selection.lifecycle_store_root)?)
    {
        return Err(HostError::Configuration);
    }
    if exists_no_follow(&intent_path)? {
        let recorded: Intent = read_state(&intent_path)?;
        if recorded.version != 1
            || recorded.task_store_path != expected_intent.task_store_path
            || recorded.lifecycle_store_root != expected_intent.lifecycle_store_root
            || (selection.version == 1 && recorded != expected_intent)
        {
            return Err(HostError::Configuration);
        }
    } else if selection.version == 1
        && !receipt_exists
        && !exists_no_follow(&selection.task_store_path)?
        && !exists_no_follow(&selection.lifecycle_store_root)?
    {
        create_synced(&intent_path, &expected_intent)?;
    } else {
        return Err(HostError::Configuration);
    }

    let task_store = TaskStore::open(
        &selection.task_store_path,
        startup.answer_authority_ref.clone(),
    )?;
    #[cfg(feature = "controlled-proof")]
    let task_store =
        task_store.with_controlled_proof_barrier(startup.lifecycle.proof_barrier_dir.clone())?;
    fs::set_permissions(
        &selection.task_store_path,
        fs::Permissions::from_mode(0o600),
    )?;
    let lifecycle_store = SelectedLifecycleStore::open(&startup.lifecycle)?;
    let expected_receipt = Receipt {
        version: 1,
        selection_sha256,
        task: stamp(&selection.task_store_path, task_store.store_id()?)?,
        lifecycle: stamp(
            &selection.lifecycle_store_root.join("lifecycle.sqlite"),
            lifecycle_store.store_id().to_owned(),
        )?,
        restart_intent_sha256: None,
        predecessor_receipt_sha256: None,
    };
    if receipt_exists {
        let recorded: Receipt = read_state(&receipt_path)?;
        if recorded != expected_receipt
            && (selection.version != 2
                || !compatible_reselection(
                    path,
                    &selection,
                    &startup,
                    &recorded,
                    &expected_receipt,
                )?)
        {
            return Err(HostError::Configuration);
        }
        if recorded != expected_receipt {
            publish_receipt(directory, &receipt_path, &expected_receipt, None)?;
        }
    } else {
        if selection.version != 1 || !task_store.is_pristine()? || !lifecycle_store.pristine() {
            return Err(HostError::Configuration);
        }
        publish_receipt(directory, &receipt_path, &expected_receipt, None)?;
    }
    Ok(Prepared {
        startup,
        identity,
        research_admission,
        task_store,
        lifecycle_store,
        selection_lock,
    })
}

#[allow(clippy::too_many_arguments)]
fn prepare_restart(
    path: &Path,
    directory: &Path,
    selection: &Selection,
    startup: &HostStartup,
    selection_sha256: &str,
    intent_path: &Path,
    receipt_path: &Path,
    selection_lock: File,
    identity: SelectedIdentity,
    research_admission: Option<crate::research_installation::AuthorizedResearchStartup>,
) -> Result<Prepared, HostError> {
    let (declaration, intent_sha256, prior_receipt, _, exact_observation) =
        read_restart_declaration(
            path,
            directory,
            selection,
            startup,
            selection_sha256,
            receipt_path,
        )?;
    let recorded_intent: Intent = read_state(intent_path)?;
    if recorded_intent.version != 1
        || recorded_intent.task_store_path != selection.task_store_path
        || recorded_intent.lifecycle_store_root != selection.lifecycle_store_root
        || !exists_no_follow(&selection.task_store_path)?
        || !exists_no_follow(&selection.lifecycle_store_root)?
    {
        return Err(HostError::Configuration);
    }
    let native = startup
        .lifecycle
        .native
        .as_ref()
        .ok_or(HostError::Configuration)?;
    // The store-owned open retains writer.lock across its read-only WAL-aware
    // schema/identity/coherence preflight. This happens before TaskStore's
    // ordinary RW open, and both locks remain held for the authorization CAS.
    let mut lifecycle_store = SelectedLifecycleStore::open_for_restart(
        &startup.lifecycle,
        &declaration.lifecycle.store_id,
    )?;
    let task_store = TaskStore::open(
        &selection.task_store_path,
        startup.answer_authority_ref.clone(),
    )?;
    #[cfg(feature = "controlled-proof")]
    let task_store =
        task_store.with_controlled_proof_barrier(startup.lifecycle.proof_barrier_dir.clone())?;
    let actual_task = stamp(&selection.task_store_path, task_store.store_id()?)?;
    let actual_lifecycle = stamp(
        &selection.lifecycle_store_root.join("lifecycle.sqlite"),
        lifecycle_store.store_id().to_owned(),
    )?;
    if actual_task != declaration.task
        || actual_lifecycle != declaration.lifecycle
        || actual_task != prior_receipt.task
        || actual_lifecycle != prior_receipt.lifecycle
    {
        return Err(HostError::Configuration);
    }
    let owner = task_store
        .observe_subject(&declaration.subject_id)?
        .ok_or(HostError::Configuration)?;
    if owner.task_id != declaration.task_id
        || owner.subject_id != declaration.subject_id
        || (!exact_observation
            && (owner.revision != declaration.task_revision
                || owner.cursor_revision != declaration.task_cursor_revision))
    {
        return Err(HostError::Configuration);
    }
    let request = SelectedRestartRequest {
        subject_id: declaration.subject_id.clone(),
        context_generation: declaration.context_generation.clone(),
        predecessor_session_id: declaration.predecessor_session_id.clone(),
        predecessor_epoch: declaration.predecessor_epoch,
        target_session_id: declaration.target_session_id.clone(),
        expected_store_id: declaration.lifecycle.store_id.clone(),
        expected_subject_revision: declaration.expected_subject_revision,
        expected_semantic_revision: declaration.expected_semantic_revision,
        expected_queue_revision: declaration.expected_queue_revision,
        expected_journal_cursor: declaration.expected_journal_cursor,
        intent_sha256: intent_sha256.clone(),
        selection_sha256: selection_sha256.into(),
        profile_digest: declaration.profile_digest.clone(),
        binary_sha256: declaration.binary_sha256.clone(),
        protocol_digest: declaration.protocol_digest.clone(),
        snapshot_id: declaration.snapshot_id.clone(),
        snapshot_manifest_sha256: declaration.snapshot_manifest_sha256.clone(),
    };
    // Recheck the task owner immediately before lifecycle authorization. The
    // retained owner.lock excludes another cooperative selected process.
    if task_store
        .observe_subject(&declaration.subject_id)?
        .as_ref()
        != Some(&owner)
    {
        return Err(HostError::Configuration);
    }
    // Refuse a stale or occupied target before the epoch authorization can
    // migrate schema 6 or write custody state. A published v3 receipt can be
    // observed again after the first actor has populated this private root.
    if exact_observation {
        ensure_private_native_root(&native.root)?;
    } else {
        validate_fresh_private_native_root(&native.root)?;
    }
    let expected_receipt = Receipt {
        version: 3,
        selection_sha256: selection_sha256.into(),
        task: actual_task,
        lifecycle: actual_lifecycle,
        restart_intent_sha256: Some(intent_sha256),
        predecessor_receipt_sha256: Some(declaration.predecessor_receipt_sha256),
    };
    let stage = directory.join("startup-receipt.tmp");
    let staged = if exists_no_follow(&stage)? {
        let staged: Receipt = read_state(&stage)?;
        if staged != expected_receipt {
            return Err(HostError::Configuration);
        }
        true
    } else {
        false
    };
    let epoch = lifecycle_store.authorize_restart(request)?;
    if epoch.state == "authorized" {
        ensure_empty_private_native_root(&native.root)?;
    } else if epoch.state != "active" {
        return Err(HostError::Configuration);
    }
    if exact_observation {
        if prior_receipt != expected_receipt {
            return Err(HostError::Configuration);
        }
        if staged {
            fs::remove_file(&stage)?;
            sync_directory(directory)?;
        }
    } else if staged {
        // A complete, synced stage can survive a crash after the epoch
        // authorization commits but before rename. Only the exact sealed
        // receipt may finish that publication under startup.lock.
        fs::rename(&stage, receipt_path)?;
        sync_directory(directory)?;
    } else {
        publish_receipt(
            directory,
            receipt_path,
            &expected_receipt,
            startup.lifecycle.proof_barrier_dir.as_deref(),
        )?;
    }
    Ok(Prepared {
        startup: startup.clone(),
        identity,
        research_admission,
        task_store,
        lifecycle_store,
        selection_lock,
    })
}

/// The prior receipt identifies the only predecessor. Both database identity
/// stamps and every state-bearing runtime setting must remain unchanged.
fn compatible_reselection(
    path: &Path,
    selection: &Selection,
    startup: &HostStartup,
    recorded: &Receipt,
    expected: &Receipt,
) -> Result<bool, HostError> {
    if recorded.version != 1
        || recorded.task != expected.task
        || recorded.lifecycle != expected.lifecycle
    {
        return Ok(false);
    }
    let prior_path = selection
        .predecessor_selection_path
        .as_deref()
        .ok_or(HostError::Configuration)?;
    if prior_path == path || prior_path.parent() != path.parent() || !canonical_exact(prior_path)? {
        return Ok(false);
    }
    let prior_bytes = read_private(prior_path, MAX_SELECTION_BYTES)?;
    if sha_bytes(&prior_bytes) != recorded.selection_sha256 {
        return Ok(false);
    }
    let prior: Selection =
        serde_json::from_slice(&prior_bytes).map_err(|_| HostError::Configuration)?;
    if !matches!(prior.version, 1 | 2)
        || prior.config_path.parent() != path.parent()
        || prior.task_store_path != selection.task_store_path
        || prior.lifecycle_store_root != selection.lifecycle_store_root
        || prior.host_socket_path != selection.host_socket_path
        || prior.lifecycle_socket_path != selection.lifecycle_socket_path
        || prior.build_id != selection.build_id
        || prior.features != selection.features
        || prior.provider_identity != selection.provider_identity
    {
        return Ok(false);
    }
    let prior_config = read_private(&prior.config_path, MAX_CONFIG_BYTES)?;
    if sha_bytes(&prior_config) != prior.config_sha256 {
        return Ok(false);
    }
    let old: HostStartup =
        serde_json::from_slice(&prior_config).map_err(|_| HostError::Configuration)?;
    Ok(old.profile == startup.profile
        && old.task_store_path == startup.task_store_path
        && old.socket_path == startup.socket_path
        && old.approved_snapshot_ref == startup.approved_snapshot_ref
        && old.answer_authority_ref == startup.answer_authority_ref
        && old.enqueue_grant_ref == startup.enqueue_grant_ref
        && old.replacement_grant_ref == startup.replacement_grant_ref
        && old.features == startup.features
        && old.provider_identity == startup.provider_identity
        && old.control == startup.control
        && serde_json::to_value(&old.lifecycle)? == serde_json::to_value(&startup.lifecycle)?)
}

/// A v3 selection changes only identities needed for a fresh native process.
/// Comparing the complete normalized typed configs also protects fields added
/// to either configuration later: they cannot silently become restart knobs.
fn compatible_restart(
    path: &Path,
    selection: &Selection,
    startup: &HostStartup,
    recorded: &Receipt,
) -> Result<bool, HostError> {
    let prior_path = selection
        .predecessor_selection_path
        .as_deref()
        .ok_or(HostError::Configuration)?;
    if !matches!(recorded.version, 1 | 3)
        || prior_path == path
        || prior_path.parent() != path.parent()
        || !canonical_exact(prior_path)?
    {
        return Ok(false);
    }
    let prior_bytes = read_private(prior_path, MAX_SELECTION_BYTES)?;
    if sha_bytes(&prior_bytes) != recorded.selection_sha256 {
        return Ok(false);
    }
    let prior: Selection =
        serde_json::from_slice(&prior_bytes).map_err(|_| HostError::Configuration)?;
    if !matches!(prior.version, 1..=3)
        || prior.config_path.parent() != path.parent()
        || prior.executable_path != selection.executable_path
        || prior.executable_sha256 != selection.executable_sha256
        || prior.task_store_path != selection.task_store_path
        || prior.lifecycle_store_root != selection.lifecycle_store_root
        || prior.host_socket_path != selection.host_socket_path
        || prior.lifecycle_socket_path != selection.lifecycle_socket_path
        || prior.build_id != selection.build_id
        || prior.features != selection.features
        || prior.provider_identity != selection.provider_identity
    {
        return Ok(false);
    }
    let prior_config = read_private(&prior.config_path, MAX_CONFIG_BYTES)?;
    if sha_bytes(&prior_config) != prior.config_sha256 {
        return Ok(false);
    }
    let old: HostStartup =
        serde_json::from_slice(&prior_config).map_err(|_| HostError::Configuration)?;
    // Control is never a restart knob: a successor keeps the exact realization.
    if old.control != startup.control {
        return Ok(false);
    }
    let (Some(old_native), Some(new_native)) = (
        old.lifecycle.native.as_ref(),
        startup.lifecycle.native.as_ref(),
    ) else {
        return Ok(false);
    };
    if old_native.session_id == new_native.session_id
        || old_native.root == new_native.root
        || old_native.root.parent() != new_native.root.parent()
        || new_native.root.parent() != path.parent()
        || !fresh_or_canonical(&new_native.root)?
    {
        return Ok(false);
    }
    let mut normalized = startup.clone();
    let normalized_native = normalized
        .lifecycle
        .native
        .as_mut()
        .ok_or(HostError::Configuration)?;
    normalized_native
        .session_id
        .clone_from(&old_native.session_id);
    normalized_native.root.clone_from(&old_native.root);
    normalized_native
        .invocation_id
        .clone_from(&old_native.invocation_id);
    normalized_native
        .attempt_id
        .clone_from(&old_native.attempt_id);
    crate::research_installation::normalize_installed_restart(&old, startup, &mut normalized)?;
    Ok(serde_json::to_value(&old)? == serde_json::to_value(&normalized)?)
}

fn read_restart_declaration(
    path: &Path,
    directory: &Path,
    selection: &Selection,
    startup: &HostStartup,
    selection_sha256: &str,
    receipt_path: &Path,
) -> Result<(RestartDeclaration, String, Receipt, String, bool), HostError> {
    let declaration_path = selection
        .restart_declaration_path
        .as_deref()
        .ok_or(HostError::Configuration)?;
    if declaration_path.parent() != Some(directory) || !canonical_exact(declaration_path)? {
        return Err(HostError::Configuration);
    }
    let declaration_bytes = read_private(declaration_path, MAX_STATE_BYTES)?;
    let declaration: RestartDeclaration =
        serde_json::from_slice(&declaration_bytes).map_err(|_| HostError::Configuration)?;
    let declaration_sha256 = sha_bytes(&declaration_bytes);
    let receipt_bytes = read_private(receipt_path, MAX_STATE_BYTES)?;
    let receipt: Receipt =
        serde_json::from_slice(&receipt_bytes).map_err(|_| HostError::Configuration)?;
    let receipt_sha256 = sha_bytes(&receipt_bytes);
    let exact_observation = receipt.version == 3
        && receipt.selection_sha256 == selection_sha256
        && receipt.restart_intent_sha256.as_deref() == Some(declaration_sha256.as_str())
        && receipt.predecessor_receipt_sha256.as_deref()
            == Some(declaration.predecessor_receipt_sha256.as_str());
    let comparison_receipt = Receipt {
        version: 1,
        selection_sha256: declaration.predecessor_selection_sha256.clone(),
        task: receipt.task.clone(),
        lifecycle: receipt.lifecycle.clone(),
        restart_intent_sha256: None,
        predecessor_receipt_sha256: None,
    };
    if !compatible_restart(path, selection, startup, &comparison_receipt)? {
        return Err(HostError::Configuration);
    }
    let new_native = startup
        .lifecycle
        .native
        .as_ref()
        .ok_or(HostError::Configuration)?;
    let old_config_path = selection
        .predecessor_selection_path
        .as_deref()
        .ok_or(HostError::Configuration)?;
    let prior_bytes = read_private(old_config_path, MAX_SELECTION_BYTES)?;
    let prior: Selection =
        serde_json::from_slice(&prior_bytes).map_err(|_| HostError::Configuration)?;
    let prior_config = read_private(&prior.config_path, MAX_CONFIG_BYTES)?;
    if sha_bytes(&prior_bytes) != declaration.predecessor_selection_sha256
        || sha_bytes(&prior_config) != prior.config_sha256
        || (!exact_observation
            && declaration.predecessor_selection_sha256 != receipt.selection_sha256)
    {
        return Err(HostError::Configuration);
    }
    let old: HostStartup =
        serde_json::from_slice(&prior_config).map_err(|_| HostError::Configuration)?;
    let old_native = old
        .lifecycle
        .native
        .as_ref()
        .ok_or(HostError::Configuration)?;
    if declaration.version != 1
        || declaration.selection_sha256 != selection_sha256
        || !valid_sha(&declaration.predecessor_selection_sha256)
        || (!exact_observation && declaration.predecessor_receipt_sha256 != receipt_sha256)
        || declaration.task != receipt.task
        || declaration.lifecycle != receipt.lifecycle
        || declaration.subject_id != startup.lifecycle.subject_id
        || declaration.context_generation != startup.lifecycle.context_generation
        || declaration.predecessor_session_id != old_native.session_id
        || declaration.target_session_id != new_native.session_id
        || declaration.snapshot_id != new_native.snapshot.id
        || declaration.snapshot_id != startup.approved_snapshot_ref
        || !valid_sha(&declaration.profile_digest)
        || !valid_sha(&declaration.binary_sha256)
        || !valid_sha(&declaration.protocol_digest)
        || !valid_sha(&declaration.snapshot_manifest_sha256)
        || declaration.operator_ref.is_empty()
        || declaration.operator_ref.len() > 256
        || !declaration.cooperative_stop_asserted
        || declaration.predecessor_host_pid == 0
        || declaration.predecessor_host_starttime_ticks.is_empty()
        || declaration.predecessor_native_pid.is_some()
            != declaration.predecessor_native_starttime_ticks.is_some()
    {
        return Err(HostError::Configuration);
    }
    verify_stopped_processes(&declaration, &old_native.root, &old_native.executable)?;
    if exact_observation {
        verify_native_route_absent(&new_native.root, &new_native.executable)?;
    }
    Ok((
        declaration,
        declaration_sha256,
        receipt,
        receipt_sha256,
        exact_observation,
    ))
}

fn process_starttime(pid: u32) -> Result<Option<String>, HostError> {
    let stat = match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let (_, tail) = stat.rsplit_once(')').ok_or(HostError::Configuration)?;
    let ticks = tail
        .split_whitespace()
        .nth(19)
        .ok_or(HostError::Configuration)?;
    Ok(Some(ticks.into()))
}

fn verify_stopped_processes(
    declaration: &RestartDeclaration,
    predecessor_native_root: &Path,
    predecessor_native_executable: &Path,
) -> Result<(), HostError> {
    if process_starttime(declaration.predecessor_host_pid)?.as_deref()
        == Some(declaration.predecessor_host_starttime_ticks.as_str())
    {
        return Err(HostError::Configuration);
    }
    if let (Some(pid), Some(ticks)) = (
        declaration.predecessor_native_pid,
        declaration.predecessor_native_starttime_ticks.as_deref(),
    ) && process_starttime(pid)?.as_deref() == Some(ticks)
    {
        return Err(HostError::Configuration);
    }
    verify_native_route_absent(predecessor_native_root, predecessor_native_executable)
}

fn verify_native_route_absent(
    predecessor_native_root: &Path,
    predecessor_native_executable: &Path,
) -> Result<(), HostError> {
    // The native transport starts from the private root's `work` directory.
    // Observe only process identity and cwd; never inspect process environs,
    // which can contain runtime credentials.
    let expected_work = predecessor_native_root.join("work");
    let executable_name = predecessor_native_executable
        .file_name()
        .ok_or(HostError::Configuration)?
        .as_encoded_bytes();
    let truncated_name = &executable_name[..executable_name.len().min(15)];
    let companion_name = b"codex-code-mode";
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let proc_path = entry.path();
        let meta = match fs::metadata(&proc_path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if meta.uid() != current_uid()? || pid == std::process::id() {
            continue;
        }
        let command_name = match fs::read(proc_path.join("comm")) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let command_name = command_name.strip_suffix(b"\n").unwrap_or(&command_name);
        if command_name != truncated_name && command_name != companion_name {
            continue;
        }
        let cwd = match fs::read_link(proc_path.join("cwd")) {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if cwd == expected_work {
            return Err(HostError::Configuration);
        }
    }
    Ok(())
}

fn stamp(path: &Path, store_id: String) -> Result<StoreStamp, HostError> {
    let metadata = fs::symlink_metadata(path)?;
    if !canonical_exact(path)?
        || !metadata.is_file()
        || metadata.uid() != current_uid()?
        || metadata.nlink() != 1
    {
        return Err(HostError::Configuration);
    }
    Ok(StoreStamp {
        path: path.to_path_buf(),
        store_id,
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

fn ensure_private_native_root(path: &Path) -> Result<(), HostError> {
    let metadata = fs::symlink_metadata(path)?;
    if !canonical_exact(path)?
        || !metadata.is_dir()
        || metadata.uid() != current_uid()?
        || metadata.permissions().mode() & 0o777 != 0o700
    {
        return Err(HostError::Configuration);
    }
    Ok(())
}

fn ensure_empty_private_native_root(path: &Path) -> Result<(), HostError> {
    if !exists_no_follow(path)? {
        fs::DirBuilder::new().mode(0o700).create(path)?;
        sync_directory(path.parent().ok_or(HostError::Configuration)?)?;
    }
    ensure_private_native_root(path)?;
    if fs::read_dir(path)?.next().is_some() {
        return Err(HostError::Configuration);
    }
    Ok(())
}

fn validate_fresh_private_native_root(path: &Path) -> Result<(), HostError> {
    if exists_no_follow(path)? {
        ensure_private_native_root(path)?;
        if fs::read_dir(path)?.next().is_some() {
            return Err(HostError::Configuration);
        }
    }
    Ok(())
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
fn sha_bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
fn sha_file(path: &Path) -> Result<String, HostError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(HostError::Configuration);
    }
    Ok(sha_bytes(&fs::read(path)?))
}
fn canonical_exact(path: &Path) -> Result<bool, HostError> {
    Ok(path.is_absolute() && fs::canonicalize(path)? == path)
}
fn fresh_or_canonical(path: &Path) -> Result<bool, HostError> {
    if !path.is_absolute() || path.file_name().is_none() {
        return Ok(false);
    }
    if exists_no_follow(path)? {
        return canonical_exact(path);
    }
    Ok(path.parent().is_some_and(Path::is_dir))
}
fn exists_no_follow(path: &Path) -> Result<bool, HostError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
fn read_private(path: &Path, limit: u64) -> Result<Vec<u8>, HostError> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.file_type().is_file()
        || meta.len() > limit
        || meta.uid() != current_uid()?
        || meta.permissions().mode() & 0o777 != 0o600
        || meta.nlink() != 1
    {
        return Err(HostError::Configuration);
    }
    Ok(fs::read(path)?)
}
fn read_state<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, HostError> {
    serde_json::from_slice(&read_private(path, MAX_STATE_BYTES)?)
        .map_err(|_| HostError::Configuration)
}
fn sync_directory(directory: &Path) -> Result<(), HostError> {
    File::open(directory)?.sync_all()?;
    Ok(())
}
fn create_synced<T: Serialize>(path: &Path, value: &T) -> Result<(), HostError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    sync_directory(path.parent().ok_or(HostError::Configuration)?)
}
fn publish_receipt(
    directory: &Path,
    path: &Path,
    value: &Receipt,
    proof_barrier_dir: Option<&Path>,
) -> Result<(), HostError> {
    let stage = directory.join("startup-receipt.tmp");
    create_synced(&stage, value)?;
    #[cfg(feature = "controlled-proof")]
    if let Some(root) = proof_barrier_dir {
        let arm = root.join("selected_receipt_staged.arm");
        if arm.exists() {
            let reached = root.join("selected_receipt_staged.reached");
            create_synced(&reached, &"reached")?;
            let release = root.join("selected_receipt_staged.release");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            while !release.exists() {
                if std::time::Instant::now() >= deadline {
                    return Err(HostError::Configuration);
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }
    #[cfg(not(feature = "controlled-proof"))]
    let _ = proof_barrier_dir;
    fs::rename(stage, path)?;
    sync_directory(directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComposedControl, ControlSelection, compose_control, validate_control};
    use serde_json::{Value, json};
    use std::process::Command;

    #[cfg(feature = "controlled-proof")]
    fn startup_inventory(
        root: &Path,
    ) -> std::collections::BTreeMap<PathBuf, (u64, u64, u32, Vec<u8>)> {
        use std::os::unix::fs::MetadataExt;
        let mut out = std::collections::BTreeMap::new();
        fn visit(
            root: &Path,
            path: &Path,
            out: &mut std::collections::BTreeMap<PathBuf, (u64, u64, u32, Vec<u8>)>,
        ) {
            for entry in fs::read_dir(path).unwrap() {
                let p = entry.unwrap().path();
                let m = fs::symlink_metadata(&p).unwrap();
                let bytes = if m.is_file() {
                    fs::read(&p).unwrap()
                } else {
                    Vec::new()
                };
                out.insert(
                    p.strip_prefix(root).unwrap().into(),
                    (m.dev(), m.ino(), m.mode(), bytes),
                );
                if m.is_dir() {
                    visit(root, &p, out);
                }
            }
        }
        visit(root, root, &mut out);
        out
    }
    fn write_private_json(path: &Path, value: &Value) {
        fs::write(
            path,
            [serde_json::to_vec(value).unwrap(), b"\n".to_vec()].concat(),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    struct Fixture {
        _temp: tempfile::TempDir,
        selection: PathBuf,
        config: PathBuf,
        task: PathBuf,
        lifecycle_root: PathBuf,
        host_socket: PathBuf,
        service_socket: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let directory = temp.path().canonicalize().unwrap();
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
            let config = directory.join("host.json");
            let selection = directory.join("selection.json");
            let task = directory.join("tasks.sqlite");
            let lifecycle_root = directory.join("lifecycle-store");
            let host_socket = directory.join("host.sock");
            let service_socket = directory.join("service.sock");
            let binary = fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
            let binary_sha = sha_file(&binary).unwrap();
            fs::write(directory.join("source.txt"), b"approved source\n").unwrap();
            let manifest = directory.join("source-manifest.json");
            fs::write(
                &manifest,
                serde_json::to_vec(
                    &json!([{"path":"source.txt","sha256":sha_bytes(b"approved source\n")}]),
                )
                .unwrap(),
            )
            .unwrap();
            let lock = directory.join("Cargo.lock");
            let toolchain = directory.join("rust-toolchain.toml");
            fs::write(&lock, b"pinned lock\n").unwrap();
            fs::write(&toolchain, b"pinned toolchain\n").unwrap();
            let uid = current_uid().unwrap();
            let grant = |reference: &str, scope: &str| {
                json!({"grant_ref":reference,"context_generation":"context:selected",
                    "scope":scope,"expires_wall_ms":i64::MAX,"revision":1})
            };
            let semantic = json!({
                "capture_id":"capture:selected","compile_session_id":"session:compile:selected",
                "verify_session_id":"session:verify:selected",
                "successor_verify_session_id":"session:successor:selected",
                "compile_invocation_id":"invocation:compile:selected",
                "compile_attempt_id":"attempt:compile:selected",
                "candidate_verify_invocation_id":"invocation:verify:selected",
                "candidate_verify_attempt_id":"attempt:verify:selected",
                "successor_verify_invocation_id":"invocation:successor:selected",
                "successor_verify_attempt_id":"attempt:successor:selected",
                "challenge_id":"challenge:selected","challenge_text":"approved challenge",
                "control_grant_ref":"grant:semantic","control_grant_revision":1,
                "successor_control_grant_ref":"grant:semantic:next",
                "successor_control_grant_revision":1
            });
            let snapshot = json!({"id":"snapshot:selected","source_revision":"source:selected",
                "members":[{"id":"source","sha256":sha_bytes(b"approved source\n"),
                    "bytes":b"approved source\n"}]});
            let native = json!({
                "executable":directory.join("unavailable-native"),
                "root":directory.join("native-root"),
                "simulator_base_url":"http://127.0.0.1:12345/v1",
                "session_id":"session:selected","invocation_id":"invocation:selected",
                "attempt_id":"attempt:selected","prompt":"approved offline prompt",
                "turn_grant_ref":"grant:native","turn_grant_revision":1,
                "snapshot_grant_ref":"grant:snapshot","snapshot_grant_revision":1,
                "snapshot":snapshot,"semantic":semantic
            });
            let lifecycle = json!({
                "profile":"retained-repository-analysis-v1",
                "store_root":lifecycle_root,"socket_path":service_socket,
                "peer_socket_path":directory.join("unused-peer.sock"),
                "verifier_executable":null,"verifier_ledger_path":null,
                "proof_barrier_dir":null,"trusted_issuer":"issuer:selected",
                "principal":{"uid":uid,"principal_ref":"principal:selected","producer_ref":"producer:selected"},
                "subject_id":"subject:selected","context_generation":"context:selected",
                "build_id":"build:selected","proof_run_id":"proof:selected",
                "grants":[grant("grant:enqueue","enqueue_input"),grant("grant:replace","request_replacement")],
                "max_executions":1,"max_wait_ms":1000,"native":native
            });
            let startup = json!({
                "profile":"retained-repository-analysis-v1","task_store_path":task,
                "socket_path":host_socket,"approved_snapshot_ref":"snapshot:selected",
                "answer_authority_ref":"operator:selected","enqueue_grant_ref":"grant:enqueue",
                "replacement_grant_ref":"grant:replace","executable_sha256":binary_sha,
                "source_root":directory,"source_manifest_path":manifest,
                "source_sha256":sha_file(&manifest).unwrap(),"lock_path":lock,
                "lock_sha256":sha_file(&lock).unwrap(),"toolchain_path":toolchain,
                "toolchain_sha256":sha_file(&toolchain).unwrap(),"features":crate::compiled_features(),
                "provider_identity":"native-sim-proof-v1","lifecycle":lifecycle
            });
            write_private_json(&config, &startup);
            let selected = json!({
                "version":1,"selection_id":"selected-disposable-test",
                "executable_path":binary,"executable_sha256":binary_sha,
                "config_path":config,"config_sha256":sha_file(&config).unwrap(),
                "task_store_path":task,"lifecycle_store_root":lifecycle_root,
                "host_socket_path":host_socket,"lifecycle_socket_path":service_socket,
                "build_id":"build:selected","features":crate::compiled_features(),
                "provider_identity":"native-sim-proof-v1"
            });
            write_private_json(&selection, &selected);
            Self {
                _temp: temp,
                selection,
                config,
                task,
                lifecycle_root,
                host_socket,
                service_socket,
            }
        }
    }

    #[cfg(feature = "controlled-proof")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[ignore = "requires explicit pinned native binary inside credential-free no-network qualification root"]
    async fn selected_owned_native_installed_notice_complete_sequence() {
        use crate::research_installation::{
            CONTROLLED_OPERATOR_INPUTS, OBSERVED_NATIVE_PIDS,
            tests::{append_notice_intent, attach_records, authorization},
        };
        struct OwnedHarness(Option<lifecycle_codex::NativeHarness>);
        impl OwnedHarness {
            fn spawn(binary: &Path, root: &Path, transport: &str) -> Self {
                for name in ["work", "home", "codex"] {
                    fs::create_dir_all(root.join(name)).unwrap();
                }
                Self(Some(
                    lifecycle_codex::NativeHarness::spawn_with_auth(
                        binary,
                        &root.join("work"),
                        &root.join("home"),
                        &root.join("codex"),
                        transport.into(),
                        lifecycle_codex::NativeAuthMode::None,
                    )
                    .unwrap(),
                ))
            }
            fn pid(&self) -> u32 {
                self.0.as_ref().unwrap().pid().unwrap()
            }
            async fn reap(&mut self) {
                if let Some(harness) = self.0.as_mut() {
                    let result = harness.terminate_and_reap().await;
                    println!("independent harness cleanup={result:?}");
                    result.expect("independent harness termination/reaping");
                    self.0.take();
                }
            }
        }
        impl Drop for OwnedHarness {
            fn drop(&mut self) {
                if let Some(mut harness) = self.0.take() {
                    // Cleanup is joined synchronously; no detached async Drop.
                    let result = std::thread::spawn(move || {
                        tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(harness.terminate_and_reap())
                    })
                    .join();
                    eprintln!("independent harness unwind cleanup={result:?}");
                    if !std::thread::panicking() {
                        assert!(result.is_ok_and(|r| r.is_ok()));
                    }
                }
            }
        }
        assert_eq!(std::env::var("M4CB_CONFINED").unwrap(), "1");
        assert!(std::env::var_os("OPENAI_API_KEY").is_none());
        assert!(!Path::new("/home/bline/.codex").exists());
        assert!(!Path::new("/home/bline/.claude").exists());
        let routes = fs::read_to_string("/proc/net/route").unwrap();
        assert_eq!(
            routes.lines().count(),
            1,
            "qualification must have no egress route"
        );
        println!(
            "qualification network={} mounts={}",
            fs::read_link("/proc/self/ns/net").unwrap().display(),
            fs::read_to_string("/proc/self/mountinfo").unwrap()
        );
        let f = Fixture::new();
        let directory = f.config.parent().unwrap();
        let native_binary = PathBuf::from(std::env::var_os("M4CB_NATIVE").unwrap());
        let mut startup: HostStartup =
            serde_json::from_slice(&fs::read(&f.config).unwrap()).unwrap();
        let native = startup.lifecycle.native.as_mut().unwrap();
        native.executable = native_binary.clone();
        fs::create_dir(&native.root).unwrap();
        fs::set_permissions(&native.root, fs::Permissions::from_mode(0o700)).unwrap();
        startup.lifecycle.grants.extend([
            lifecycle_service::GrantConfig {
                grant_ref: "grant:native".into(),
                context_generation: startup.lifecycle.context_generation.clone(),
                scope: "native_turn".into(),
                expires_wall_ms: i64::MAX,
                revision: 1,
            },
            lifecycle_service::GrantConfig {
                grant_ref: "grant:snapshot".into(),
                context_generation: startup.lifecycle.context_generation.clone(),
                scope: "snapshot_read".into(),
                expires_wall_ms: i64::MAX,
                revision: 1,
            },
        ]);
        let mode = std::env::var("M4CB_STARTUP_CONTROL").unwrap_or_else(|_| "success".into());
        if matches!(
            mode.as_str(),
            "queued-turn"
                | "queued-successor"
                | "active-successor"
                | "active-turn"
                | "shutdown-buffered"
                | "cut-turn"
                | "cut-successor"
        ) {
            startup.lifecycle.proof_barrier_dir = Some(directory.to_path_buf());
            let queued = match mode.as_str() {
                "active-successor" | "cut-successor" => "queued-successor",
                "active-turn" | "shutdown-buffered" | "cut-turn" => "queued-turn",
                _ => &mode,
            };
            if mode == "shutdown-buffered" {
                fs::write(directory.join("native_test_queued-successor.arm"), b"arm\n").unwrap();
            }
            fs::write(
                directory.join(format!("native_test_{queued}.arm")),
                b"arm\n",
            )
            .unwrap();
        }
        if matches!(
            mode.as_str(),
            "owned-observations"
                | "activation-child-exit"
                | "ack-receiver-loss"
                | "readiness-receiver-loss"
        ) {
            startup.lifecycle.proof_barrier_dir = Some(directory.to_path_buf());
            // Every successful activation reaching the unconditional cut can progress.
            fs::write(
                directory.join("native_before_turn_entry.release"),
                b"release\n",
            )
            .unwrap();
        }
        if matches!(
            mode.as_str(),
            "diagnostic-pid-write" | "diagnostic-outcome-write"
        ) {
            startup.lifecycle.proof_barrier_dir = Some(directory.to_path_buf());
            let filename = if mode == "diagnostic-pid-write" {
                "native_test_owned_pid"
            } else {
                "native_test_runner_outcome.json"
            };
            // The real diagnostic write reaches this exact path and fails with EISDIR.
            fs::create_dir(directory.join(filename)).unwrap();
            fs::write(
                directory.join("native_before_turn_entry.release"),
                b"release\n",
            )
            .unwrap();
        }
        if mode == "owned-observations" {
            fs::write(directory.join("host_before_admit.arm"), b"arm\n").unwrap();
        }
        if mode == "ack-receiver-loss" {
            fs::write(directory.join("native_before_activation_ack.arm"), b"arm\n").unwrap();
        }
        let cut = std::env::var("M4CB_CUT").ok();
        if mode.starts_with("cut-") {
            let cut = cut.as_ref().expect("exact owner-local cut");
            fs::write(directory.join(format!("{cut}.arm")), b"arm\n").unwrap();
            fs::write(
                directory.join("native_before_turn_entry.release"),
                b"release\n",
            )
            .unwrap();
        }
        if matches!(
            mode.as_str(),
            "socket-setup"
                | "socket-budget"
                | "cleanup-failure"
                | "exit-recording-failure"
                | "actor-panic"
        ) {
            startup.lifecycle.proof_barrier_dir = Some(directory.to_path_buf());
            if mode == "socket-budget" {
                fs::write(directory.join("socket_setup_cleanup_budget.arm"), b"arm\n").unwrap();
            }
            if mode == "actor-panic" {
                fs::write(
                    directory.join("actor_after_owned_child_panic.arm"),
                    b"arm\n",
                )
                .unwrap();
            } else {
                fs::write(
                    directory.join("socket_setup_after_owned_child.arm"),
                    b"arm\n",
                )
                .unwrap();
            }
            if mode == "cleanup-failure" {
                fs::write(
                    directory.join("native_cleanup_classification.fail"),
                    b"injected after real reap\n",
                )
                .unwrap();
            }
            if mode == "exit-recording-failure" {
                fs::write(
                    directory.join("native_exit_recording.fail"),
                    b"injected recording failure\n",
                )
                .unwrap();
            }
            fs::write(
                directory.join("native_after_spawn_before_registration.arm"),
                b"arm\n",
            )
            .unwrap();
        }
        let (mut startup, _, payload) = attach_records(directory, startup);
        let second_payload = append_notice_intent(&mut startup, &payload, "notice-2");
        let third_payload = append_notice_intent(&mut startup, &payload, "notice-3");
        if let Some(cut) = mode
            .strip_prefix("readiness-")
            .filter(|_| mode != "readiness-receiver-loss")
        {
            crate::research_installation::READINESS_TEST_FAULTS
                .lock()
                .unwrap()
                .insert(directory.join("protected"), cut.into());
        }
        write_private_json(&f.config, &serde_json::to_value(&startup).unwrap());
        let mut selection: Value =
            serde_json::from_slice(&fs::read(&f.selection).unwrap()).unwrap();
        selection["config_sha256"] = json!(sha_file(&f.config).unwrap());
        write_private_json(&f.selection, &selection);
        let auth = authorization(
            &startup,
            &fs::read(&f.selection).unwrap(),
            &fs::read(&f.config).unwrap(),
        );
        CONTROLLED_OPERATOR_INPUTS
            .lock()
            .unwrap()
            .insert(auth.clone());
        let decoy_root = directory.join("decoy");
        fs::create_dir(&decoy_root).unwrap();
        let mut decoy =
            OwnedHarness::spawn(&native_binary, &decoy_root, "transport:independent-decoy");
        let decoy_pid = decoy.pid();
        let mut earlier = (mode == "owned-observations").then(|| {
            OwnedHarness::spawn(
                &native_binary,
                &directory.join("earlier"),
                "transport:independent-earlier",
            )
        });
        let spec_receiver = if mode == "owned-observations" {
            let (sender, receiver) = std::sync::mpsc::channel();
            crate::STARTUP_TEST_SPECS
                .lock()
                .unwrap()
                .insert(f.selection.clone(), sender);
            Some(receiver)
        } else {
            None
        };
        let mode = std::env::var("M4CB_STARTUP_CONTROL").unwrap_or_else(|_| "success".into());
        if (!mode.starts_with("readiness-") || mode == "readiness-receiver-loss")
            && !matches!(
                mode.as_str(),
                "success"
                    | "socket-setup"
                    | "socket-budget"
                    | "cleanup-failure"
                    | "exit-recording-failure"
                    | "actor-panic"
            )
        {
            crate::STARTUP_TEST_ACTIONS.lock().unwrap().insert(
                f.selection.clone(),
                if mode == "diagnostic-outcome-write" {
                    "readiness-receiver-loss".into()
                } else {
                    mode.clone()
                },
            );
        }
        let selection_path = f.selection.clone();
        let auth_path = auth.clone();
        let mut host = tokio::spawn(async move {
            crate::serve_operator_research_installed(&selection_path, &auth_path).await
        });
        async fn wait_marker(path: &Path) {
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(45);
            while !path.exists() {
                if tokio::time::Instant::now() >= deadline {
                    let root = path.parent().unwrap();
                    let files = fs::read_dir(root)
                        .unwrap()
                        .map(|e| e.unwrap().file_name())
                        .collect::<Vec<_>>();
                    panic!(
                        "missing marker {} files={files:?} runner={:?}",
                        path.display(),
                        fs::read_to_string(root.join("native_test_runner_outcome.json"))
                    );
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
        fn assert_no_effects(directory: &Path, lifecycle_root: &Path) {
            for name in [
                "native_prepare_turn_attempted.reached",
                "native_prepare_turn_handled.reached",
                "native_tool_attempted.reached",
                "native_tool_handled.reached",
            ] {
                assert!(!directory.join(name).exists(), "premature effect {name}");
            }
            assert!(
                !Path::new(&std::env::var_os("M4CB_MODEL_ENTRY_SENTINEL").unwrap()).exists(),
                "intercepted model entry is failure"
            );
            let db = rusqlite::Connection::open_with_flags(
                lifecycle_root.join("lifecycle.sqlite"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            assert_eq!(
                db.query_row("SELECT count(*) FROM native_turns", [], |r| r
                    .get::<_, u64>(0))
                    .unwrap(),
                0
            );
        }
        if mode == "owned-observations" {
            wait_marker(&directory.join("host_before_admit.reached")).await;
            let spec = spec_receiver.unwrap().recv().unwrap();
            let earlier = earlier.as_mut().unwrap();
            let previous = earlier
                .0
                .as_ref()
                .unwrap()
                .observe_protected_surface(&spec, true)
                .unwrap();
            let observed = decoy
                .0
                .as_ref()
                .unwrap()
                .observe_protected_surface(&spec, true)
                .unwrap();
            let actual = *OBSERVED_NATIVE_PIDS.lock().unwrap().last().unwrap();
            assert_ne!(actual, observed.native_pid());
            assert_ne!(actual, previous.native_pid());
            assert_ne!(previous.native_pid(), observed.native_pid());
            for observation in [&previous, &observed] {
                observation.guard().check_current().unwrap();
                assert!(observation.matches_protected_surface(&spec));
                assert_eq!(
                    fs::metadata(format!("/proc/{}/exe", observation.native_pid()))
                        .unwrap()
                        .ino(),
                    fs::metadata(&native_binary).unwrap().ino()
                );
            }
            let copied_json = serde_json::to_vec(observed.evidence()).unwrap();
            assert!(
                serde_json::from_slice::<Value>(&copied_json)
                    .unwrap()
                    .is_object()
            );
            println!(
                "independent previous={} start={} decoy={} start={} genuine={} copied_json_is_data_only=true",
                previous.native_pid(),
                previous.native_starttime(),
                observed.native_pid(),
                observed.native_starttime(),
                actual
            );
            earlier.reap().await;
            assert!(previous.guard().check_current().is_err());
            observed.guard().check_current().unwrap();
            decoy.reap().await;
            assert!(observed.guard().check_current().is_err());
            assert!(Path::new(&format!("/proc/{actual}")).exists());
            assert!(!f.host_socket.exists());
            fs::write(
                directory.join("host_before_admit.release"),
                b"release genuine unchanged\n",
            )
            .unwrap();
        }
        fn owner_rows(path: &Path) -> Vec<(String, Vec<Vec<String>>)> {
            let db = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            let tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap().query_map([], |r|r.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
            tables
                .into_iter()
                .map(|table| {
                    assert!(
                        table
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    );
                    let mut statement = db
                        .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                        .unwrap();
                    let columns = statement.column_count();
                    let rows = statement
                        .query_map([], |row| {
                            Ok((0..columns)
                                .map(|i| format!("{:?}", row.get_ref(i).unwrap()))
                                .collect::<Vec<_>>())
                        })
                        .unwrap()
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();
                    (table, rows)
                })
                .collect()
        }
        fn readiness_files(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
            let mut files = fs::read_dir(root)
                .unwrap()
                .filter_map(|entry| {
                    let path = entry.unwrap().path();
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("native-readiness-v1-")
                        .then(|| (path.clone(), fs::read(path).unwrap()))
                })
                .collect::<Vec<_>>();
            files.sort_by(|a, b| a.0.cmp(&b.0));
            files
        }
        fn assert_owner_collection(
            f: &Fixture,
            directory: &Path,
            unsafe_close: bool,
            child_created: bool,
            exit_recorded: bool,
        ) -> Value {
            let done: Value =
                serde_json::from_slice(&fs::read(directory.join("native_test_done.json")).unwrap())
                    .unwrap();
            assert_eq!(done["exit_recorded"], exit_recorded, "owner Done: {done}");
            assert_eq!(done["close_unsafe"], !child_created, "owner Done: {done}");
            assert_eq!(
                done["record_result"],
                if exit_recorded {
                    "Ok(())"
                } else {
                    "Err(Rejected)"
                }
            );
            let db = rusqlite::Connection::open_with_flags(
                f.lifecycle_root.join("lifecycle.sqlite"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            let close: bool = db
                .query_row(
                    "SELECT unsafe_at_close FROM service_closes ORDER BY rowid DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(close, unsafe_close, "durable owner close differs: {done}");
            let rows: u64 = db
                .query_row("SELECT count(*) FROM native_process_exits", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(rows, u64::from(exit_recorded));
            if child_created {
                let cleanup: Value = serde_json::from_slice(
                    &fs::read(directory.join("native_test_cleanup.json")).unwrap(),
                )
                .unwrap();
                assert_eq!(cleanup["done_sent"], true);
                assert_eq!(
                    cleanup["acknowledgment"],
                    if exit_recorded {
                        "Some(Ok(Ok(())))"
                    } else {
                        "Some(Ok(Err(Rejected)))"
                    }
                );
                assert_eq!(cleanup["collected"]["exit"]["pid"], done["exit"]["pid"]);
                let actual = done["exit"]["pid"].as_u64().unwrap();
                assert!(!Path::new(&format!("/proc/{actual}")).exists());
                if exit_recorded {
                    let (pid, outcome): (u32, String) = db
                        .query_row(
                            "SELECT pid,outcome_kind FROM native_process_exits",
                            [],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .unwrap();
                    assert_eq!(u64::from(pid), actual);
                    assert_eq!(done["outcome_kind"], outcome);
                }
            } else {
                assert!(done["exit"].is_null());
                assert!(!directory.join("native_test_owned_pid").exists());
                assert!(!directory.join("native_test_cleanup.json").exists());
                assert_eq!(
                    fs::read_to_string(directory.join("native_test_nochild_ack")).unwrap(),
                    "Some(Ok(Ok(())))"
                );
            }
            println!(
                "owner classification unsafe_close={close} child_created={child_created} exit_recorded={exit_recorded} done={done}"
            );
            done
        }
        if mode.starts_with("cut-") {
            let cut = cut.as_ref().unwrap();
            wait_marker(&directory.join(format!("{cut}.reached"))).await;
            let before_task = owner_rows(&f.task);
            let before_readiness = readiness_files(&directory.join("protected"));
            assert!(!f.host_socket.exists());
            let socket = f.host_socket.clone();
            let request = json!({"op":"research.invalidate.v1","payload":payload.clone()});
            assert!(
                tokio::task::spawn_blocking(move || crate::exchange(&socket, &request))
                    .await
                    .unwrap()
                    .is_err()
            );
            // Close actual service ownership, then release every reached cut before collection.
            Command::new("/usr/bin/kill")
                .args(["-TERM", &std::process::id().to_string()])
                .status()
                .unwrap();
            wait_marker(&directory.join("shutdown_gate_closed.reached")).await;
            fs::write(directory.join(format!("{cut}.release")), b"release\n").unwrap();
            let result = tokio::time::timeout(std::time::Duration::from_secs(45), &mut host)
                .await
                .unwrap()
                .unwrap();
            assert!(result.is_err(), "canceled startup must refuse: {result:?}");
            let early = matches!(
                cut.as_str(),
                "native_before_spawn" | "native_after_spawn_before_registration"
            );
            assert_owner_collection(&f, directory, early, cut != "native_before_spawn", true);
            println!(
                "cleanup qualification={}",
                if early {
                    "unsafe_unqualified"
                } else {
                    "normal_qualified"
                }
            );
            assert_no_effects(directory, &f.lifecycle_root);
            assert!(
                directory.join(format!("{cut}.released")).exists(),
                "actual cut must consume release"
            );
            assert!(
                !directory.join(format!("{cut}.timed_out")).exists(),
                "timeout is not intended fence"
            );
            assert_eq!(
                owner_rows(&f.task),
                before_task,
                "late failure must preserve truthful nomination/key setup"
            );
            assert_eq!(
                readiness_files(&directory.join("protected")),
                before_readiness,
                "late failure must retain readiness publication"
            );
            if let Ok(pid) = fs::read_to_string(directory.join("native_test_owned_pid")) {
                assert!(
                    !Path::new(&format!("/proc/{}", pid.trim())).exists(),
                    "actual child must be reaped"
                );
            }
            decoy.reap().await;
            println!("exact cut={cut} queued={mode} public_attempt=unavailable result={result:?}");
            CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);
            crate::STARTUP_TEST_ACTIONS
                .lock()
                .unwrap()
                .remove(&f.selection);
            return;
        }
        if matches!(mode.as_str(), "active-turn" | "shutdown-buffered") {
            wait_marker(&directory.join("native_before_turn_entry.reached")).await;
            wait_marker(&f.host_socket).await;
            assert!(
                directory.join("native_test_queue.reached").exists(),
                "actual actor queue must be reached before release"
            );
            if mode == "active-turn" {
                fs::write(&f.selection, b"changed immutable selected startup").unwrap();
            } else {
                Command::new("/usr/bin/kill")
                    .args(["-TERM", &std::process::id().to_string()])
                    .status()
                    .unwrap();
                wait_marker(&directory.join("shutdown_gate_closed.reached")).await;
            }
            fs::write(
                directory.join("native_before_turn_entry.release"),
                b"consume actual buffered work\n",
            )
            .unwrap();
            wait_marker(&directory.join("native_test_consumed.json")).await;
            if mode == "active-turn" {
                wait_marker(&directory.join("native_run_turn_currentness_refused.json")).await;
                let refusal: Value = serde_json::from_slice(
                    &fs::read(directory.join("native_run_turn_currentness_refused.json")).unwrap(),
                )
                .unwrap();
                assert_eq!(refusal["gate_closed"], false);
                assert_eq!(refusal["installed_hold"], true);
                Command::new("/usr/bin/kill")
                    .args(["-TERM", &std::process::id().to_string()])
                    .status()
                    .unwrap();
            } else {
                wait_marker(&directory.join("native_common_gate_refused.reached")).await;
            }
            let result = tokio::time::timeout(std::time::Duration::from_secs(45), host)
                .await
                .unwrap()
                .unwrap();
            assert!(
                result.is_ok(),
                "normal lifetime shutdown must collect: {result:?}"
            );
            let consumed =
                fs::read_to_string(directory.join("native_test_consumed.jsonl")).unwrap();
            let rows = consumed
                .lines()
                .map(|l| serde_json::from_str::<Value>(l).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(rows.len(), 1, "only actual consumed items count");
            assert_eq!(rows[0]["work"], "RunTurn");
            assert_eq!(rows[0]["gate_closed"], mode == "shutdown-buffered");
            assert_no_effects(directory, &f.lifecycle_root);
            let actual = *OBSERVED_NATIVE_PIDS.lock().unwrap().last().unwrap();
            assert!(!Path::new(&format!("/proc/{actual}")).exists());
            assert!(!f.host_socket.exists());
            decoy.reap().await;
            println!("actual buffered refusal mode={mode} consumed={consumed} result={result:?}");
            CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);
            crate::STARTUP_TEST_ACTIONS
                .lock()
                .unwrap()
                .remove(&f.selection);
            return;
        }
        if mode == "active-successor" {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
            // A configured proof directory enables this existing unconditional
            // turn-entry cut. Release it only after observing actual activation;
            // the queued work must then reach the runner's unchanged gates.
            let turn_entry = directory.join("native_before_turn_entry.reached");
            while !turn_entry.exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "active successor must reach the actual turn-entry cut"
                );
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            assert_eq!(
                fs::read_to_string(directory.join("native_test_queue.reached")).unwrap(),
                "queued-successor"
            );
            fs::write(
                directory.join("native_before_turn_entry.release"),
                b"release\n",
            )
            .unwrap();
            let marker = directory.join("native_test_consumed.json");
            while !marker.exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "active successor must be consumed by the actual runner"
                );
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            let consumed: Value = serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
            println!("actual installed successor consumption: {consumed}");
            assert_eq!(consumed["work"], "StartSuccessor");
            assert_eq!(consumed["installed_hold"], true);
            let thread = consumed["thread"].as_str().unwrap().to_owned();
            let actual = *OBSERVED_NATIVE_PIDS.lock().unwrap().last().unwrap();
            while Path::new(&format!("/proc/{actual}")).exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "successor refusal must reap runner child"
                );
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
            if consumed["gate_closed"] != true {
                assert_eq!(
                    fs::read_to_string(
                        directory.join("native_installed_successor_refused.reached")
                    )
                    .unwrap(),
                    thread,
                );
            }
            let db = rusqlite::Connection::open_with_flags(
                f.lifecycle_root.join("lifecycle.sqlite"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            assert_eq!(db.query_row("SELECT thread_id FROM native_sessions WHERE session_id='session:selected' AND role='domain'", [], |r| r.get::<_,String>(0)).unwrap(), thread);
            assert_eq!(
                db.query_row("SELECT count(*) FROM native_turns", [], |r| r
                    .get::<_, u64>(0))
                    .unwrap(),
                0
            );
            drop(db);
            assert!(!Path::new(&std::env::var_os("M4CB_MODEL_ENTRY_SENTINEL").unwrap()).exists());
            decoy.reap().await;
            Command::new("/usr/bin/kill")
                .args(["-TERM", &std::process::id().to_string()])
                .status()
                .unwrap();
            tokio::time::timeout(std::time::Duration::from_secs(35), host)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert!(!f.host_socket.exists());
            assert!(!f.service_socket.exists());
            CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);
            return;
        }
        if mode != "success" && mode != "owned-observations" {
            let result = tokio::time::timeout(std::time::Duration::from_secs(45), &mut host)
                .await
                .unwrap()
                .unwrap();
            decoy.reap().await;
            assert!(result.is_err(), "{mode} must refuse");
            if matches!(
                mode.as_str(),
                "activation-child-exit"
                    | "ack-receiver-loss"
                    | "readiness-receiver-loss"
                    | "diagnostic-pid-write"
                    | "diagnostic-outcome-write"
            ) {
                let diagnostic = mode.starts_with("diagnostic-");
                let done = assert_owner_collection(&f, directory, diagnostic, true, true);
                if diagnostic {
                    assert_eq!(format!("{result:?}"), "Err(Lifecycle(Store(Unavailable)))");
                    let cleanup: Value = serde_json::from_slice(
                        &fs::read(directory.join("native_test_cleanup.json")).unwrap(),
                    )
                    .unwrap();
                    let errors = cleanup["diagnostic_errors"].as_array().unwrap();
                    assert_eq!(errors.len(), 1);
                    let filename = if mode == "diagnostic-pid-write" {
                        "native_test_owned_pid"
                    } else {
                        "native_test_runner_outcome.json"
                    };
                    assert!(
                        errors[0]
                            .as_str()
                            .unwrap()
                            .starts_with(&format!("{filename}: Is a directory"))
                    );
                    assert!(directory.join(filename).is_dir());
                    assert_no_effects(directory, &f.lifecycle_root);
                    println!(
                        "diagnostic failure retained after actual reap and owner acknowledgment: {cleanup}; done={done}"
                    );
                    CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);
                    crate::STARTUP_TEST_ACTIONS
                        .lock()
                        .unwrap()
                        .remove(&f.selection);
                    return;
                }
            }
            if matches!(
                mode.as_str(),
                "activation-child-exit" | "ack-receiver-loss" | "readiness-receiver-loss"
            ) {
                assert_no_effects(directory, &f.lifecycle_root);
                assert!(!directory.join("native_installed_guard.reached").exists());
                if mode == "activation-child-exit" {
                    assert!(f.selection.with_extension("activation-invoked").exists());
                }
                if mode == "ack-receiver-loss" {
                    assert!(
                        directory
                            .join("native_activation_ack_failed.reached")
                            .exists()
                    );
                }
                if mode == "readiness-receiver-loss" {
                    assert!(directory.join("native_publish_attempted.reached").exists());
                    assert!(directory.join("native_publish_failed.reached").exists());
                }
            }
            if let Some(marker) = std::env::var_os("M4CB_MODEL_ENTRY_SENTINEL") {
                assert!(
                    !Path::new(&marker).exists(),
                    "containment intercepted a fence regression"
                );
            }
            if mode.starts_with("wrong-") {
                assert_eq!(
                    fs::read_to_string(f.selection.with_extension("injection-reached")).unwrap(),
                    mode
                );
            }
            if mode.starts_with("queued-") {
                assert_eq!(
                    fs::read_to_string(directory.join("native_test_queue.reached")).unwrap(),
                    mode
                );
                let db = rusqlite::Connection::open_with_flags(
                    f.lifecycle_root.join("lifecycle.sqlite"),
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                )
                .unwrap();
                assert_eq!(
                    db.query_row("SELECT count(*) FROM native_turns", [], |r| r
                        .get::<_, u64>(0))
                        .unwrap(),
                    0
                );
            }
            if matches!(
                mode.as_str(),
                "socket-budget" | "cleanup-failure" | "exit-recording-failure"
            ) {
                assert!(
                    format!("{result:?}").contains("Unavailable"),
                    "budget overrun remains failure after collection"
                );
            }

            assert!(!f.host_socket.exists(), "{mode} exposed productive service");
            if let Some(cut) = mode
                .strip_prefix("readiness-")
                .filter(|_| mode != "readiness-receiver-loss")
            {
                let root = directory.join("protected");
                assert_eq!(
                    fs::read_to_string(root.join("readiness-fault-reached")).unwrap(),
                    cut
                );
                let records = fs::read_dir(&root)
                    .unwrap()
                    .filter_map(|e| {
                        let p = e.unwrap().path();
                        p.file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with("native-readiness-v1-")
                            .then_some(p)
                    })
                    .collect::<Vec<_>>();
                assert_eq!(records.len(), usize::from(cut != "before-create"));
                if let Some(path) = records.first() {
                    let before = fs::read(path).unwrap();
                    if cut == "partial-write" {
                        assert!(serde_json::from_slice::<Value>(&before).is_err());
                    }
                    if cut == "postpublication-tamper" {
                        assert_eq!(before.len(), 1);
                    }
                }
                let db = rusqlite::Connection::open_with_flags(
                    &f.task,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                )
                .unwrap();
                assert_eq!(
                    db.query_row("SELECT count(*) FROM research_tasks", [], |r| r
                        .get::<_, u64>(0))
                        .unwrap(),
                    0
                );
                crate::research_installation::READINESS_TEST_FAULTS
                    .lock()
                    .unwrap()
                    .remove(&root);
            }
            let actual = if matches!(
                mode.as_str(),
                "socket-setup"
                    | "socket-budget"
                    | "cleanup-failure"
                    | "exit-recording-failure"
                    | "actor-panic"
            ) {
                fs::read_to_string(directory.join("native_after_spawn_before_registration.pid"))
                    .unwrap()
                    .trim()
                    .parse::<u32>()
                    .unwrap()
            } else if mode == "readiness-receiver-loss" {
                fs::read_to_string(directory.join("native_test_owned_pid"))
                    .unwrap()
                    .trim()
                    .parse::<u32>()
                    .unwrap()
            } else {
                *OBSERVED_NATIVE_PIDS.lock().unwrap().last().unwrap()
            };
            assert_ne!(actual, decoy_pid);
            assert!(
                !Path::new(&format!("/proc/{actual}")).exists(),
                "{mode} must reap actual runner child"
            );
            if matches!(
                mode.as_str(),
                "actor-panic" | "cleanup-failure" | "exit-recording-failure"
            ) {
                assert!(
                    directory.join("shutdown_gate_closed.reached").exists(),
                    "serving loop must fence admission"
                );
                let db = rusqlite::Connection::open_with_flags(
                    f.lifecycle_root.join("lifecycle.sqlite"),
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                )
                .unwrap();
                assert_eq!(
                    db.query_row("SELECT count(*) FROM native_turns", [], |r| r
                        .get::<_, i64>(0))
                        .unwrap(),
                    0,
                    "fenced startup cannot prepare a model turn"
                );
                if mode != "actor-panic" {
                    assert!(db.query_row("SELECT unsafe_at_close FROM service_closes ORDER BY rowid DESC LIMIT 1", [], |r| r.get::<_, bool>(0)).unwrap(), "injected unknown cleanup/recording must persist unsafe close");
                }
            }
            CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);
            crate::STARTUP_TEST_ACTIONS
                .lock()
                .unwrap()
                .remove(&f.selection);
            println!(
                "controlled failure={mode} runner_owned_pid={actual} decoy_pid={decoy_pid} result={result:?}"
            );
            return;
        }
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(45);
        while !f.host_socket.exists() {
            if host.is_finished() {
                let result = (&mut host).await;
                decoy.reap().await;
                println!("failed fixture retained={}", f._temp.keep().display());
                panic!("startup before socket: {result:?}");
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "selected startup deadline"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let actual = *OBSERVED_NATIVE_PIDS.lock().unwrap().last().unwrap();
        assert_ne!(actual, decoy_pid);
        assert!(Path::new(&format!("/proc/{actual}")).exists());
        println!("runner_owned_pid={actual} decoy_pid={decoy_pid}");
        let socket = f.host_socket.clone();
        let request = json!({"op":"research.invalidate.v1","payload":payload.clone()});
        let retry = request.clone();
        let result = tokio::task::spawn_blocking(move || crate::exchange(&socket, &request))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result["status"], "invalidated", "{result}");
        assert_eq!(result["qualification"], "controlled_only");
        assert_eq!(
            result["receipt"]["admission"]["selection_digest"],
            sha_file(&f.selection).unwrap()
        );
        // Owner writes occurred legitimately; no model turn was prepared. An
        // immutable selection mutation fences fresh use while exact history
        // remains the originally admitted receipt.
        let connection = rusqlite::Connection::open_with_flags(
            f.lifecycle_root.join("lifecycle.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM native_turns", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        drop(connection);
        let first_evidence = result["receipt"]["admission"]["currentness_evidence"].clone();
        let decode_evidence = |evidence: &Value| {
            let bytes: Vec<u8> = serde_json::from_value(evidence["bytes"].clone()).unwrap();
            assert_eq!(sha_bytes(&bytes), evidence["digest"]);
            serde_json::from_slice::<Value>(&bytes).unwrap()
        };
        let first_observation = decode_evidence(&first_evidence);
        let readiness = &first_observation["readiness"];
        let selected_value = serde_json::to_value(
            startup
                .research
                .as_ref()
                .unwrap()
                .installation
                .as_ref()
                .unwrap(),
        )
        .unwrap();
        let installed_root = PathBuf::from(selected_value["root"].as_str().unwrap());
        let readiness_path = installed_root.join(readiness["member"].as_str().unwrap());
        let retained_bytes = fs::read(&readiness_path).unwrap();
        assert_eq!(
            sha_file(&readiness_path).unwrap(),
            readiness["sha256"].as_str().unwrap()
        );
        assert_eq!(
            retained_bytes.len() as u64,
            readiness["length"].as_u64().unwrap()
        );
        let record: Value = serde_json::from_slice(&retained_bytes).unwrap();
        assert_eq!(record["phase"], "held_readiness_observed");
        assert!(record["native_readiness"]["thread_request"].is_object());
        assert_eq!(
            record["native_readiness"]["thread_ack"]["thread"]["id"],
            record["native_readiness"]["thread"]
        );
        assert_eq!(record["initial_observation"]["native_pid"], actual);

        // Change a mutable identity anchor's actual directory metadata. This
        // creates an OS-observable discriminator without changing admission.
        let marker = directory.join("fresh-observation-marker");
        fs::write(&marker, b"owner-created observation discriminator").unwrap();
        let marker_metadata = fs::metadata(directory).unwrap();
        let socket = f.host_socket.clone();
        let second = tokio::task::spawn_blocking(move || {
            crate::exchange(
                &socket,
                &json!({"op":"research.invalidate.v1","payload":second_payload}),
            )
        })
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            second["status"], "invalidated",
            "an exact fresh admitted notice must work before mutation"
        );
        let second_observation =
            decode_evidence(&second["receipt"]["admission"]["currentness_evidence"]);
        assert_ne!(
            first_observation["observation"], second_observation["observation"],
            "actual raw observation must change after mutable anchor metadata changes"
        );
        let observed_anchor = second_observation["observation"]["object_metadata"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == json!(directory))
            .unwrap();
        assert_eq!(observed_anchor["modified_seconds"], marker_metadata.mtime());
        assert_eq!(
            observed_anchor["modified_nanoseconds"],
            marker_metadata.mtime_nsec()
        );
        assert_ne!(
            first_observation["payload_digest"],
            second_observation["payload_digest"]
        );
        assert_eq!(
            second_observation["selection_digest"],
            sha_file(&f.selection).unwrap()
        );
        // Complete retained bytes and response syntax fit the unchanged frame.
        assert!(serde_json::to_vec(&second).unwrap().len() <= crate::MAX_FRAME);
        assert!(
            serde_json::to_vec(&second).unwrap().len()
                - serde_json::to_vec(&second["receipt"]).unwrap().len()
                < 256
        );
        fs::write(&f.selection, b"changed immutable selected startup").unwrap();
        let socket = f.host_socket.clone();
        let historical = tokio::task::spawn_blocking(move || crate::exchange(&socket, &retry))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            historical, result,
            "historical retry must preserve original receipt and admission"
        );
        let fresh = json!({"op":"research.invalidate.v1","payload":third_payload});
        let socket = f.host_socket.clone();
        let refused = tokio::task::spawn_blocking(move || crate::exchange(&socket, &fresh))
            .await
            .unwrap()
            .unwrap();
        assert!(
            refused.get("code").is_some(),
            "fresh mutated admission must refuse"
        );
        decoy.reap().await;
        Command::new("/usr/bin/kill")
            .args(["-TERM", &std::process::id().to_string()])
            .status()
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(35), host)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            !Path::new(&format!("/proc/{actual}")).exists(),
            "runner must reap its child"
        );
        CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);
        assert!(!f.host_socket.exists());
        assert!(!f.service_socket.exists());
        assert_no_effects(directory, &f.lifecycle_root);
    }

    #[cfg(feature = "controlled-proof")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[ignore = "test-only separate selected host process; requires confined qualification caller"]
    async fn installed_selected_subprocess_entry() {
        assert_eq!(std::env::var("M4CB_CONFINED").unwrap(), "1");
        let selection = PathBuf::from(std::env::var_os("M4CB_TEST_SELECTION").unwrap());
        let authorization = PathBuf::from(std::env::var_os("M4CB_TEST_AUTHORIZATION").unwrap());
        crate::research_installation::CONTROLLED_OPERATOR_INPUTS
            .lock()
            .unwrap()
            .insert(authorization.clone());
        crate::serve_operator_research_installed(&selection, &authorization)
            .await
            .unwrap();
    }

    #[cfg(feature = "controlled-proof")]
    #[test]
    #[ignore = "requires pinned native in credential-free no-network qualification root"]
    fn installed_v3_restart_uses_separate_stopped_host_and_retains_history() {
        use crate::research_installation::{
            CONTROLLED_OPERATOR_INPUTS,
            tests::{
                append_notice_intent, attach_records, authorization_named, refreshed_records,
                refreshed_records_named,
            },
        };
        use std::process::{Child, Stdio};
        assert_eq!(std::env::var("M4CB_CONFINED").unwrap(), "1");
        assert_eq!(
            fs::read_to_string("/proc/net/route")
                .unwrap()
                .lines()
                .count(),
            1
        );
        assert!(!Path::new("/home/bline/.codex").exists());
        // Cleanup ownership begins immediately after spawn and survives every
        // readiness assertion and caller unwind. Drop never masks that panic.
        struct OwnedHost {
            child: Child,
            outcomes: std::sync::Arc<std::sync::Mutex<Vec<Value>>>,
            collected: bool,
        }
        impl OwnedHost {
            fn id(&self) -> u32 {
                self.child.id()
            }
            fn collect(&mut self) -> Value {
                let mut errors = Vec::new();
                let mut forced = false;
                let mut status = match self.child.try_wait() {
                    Ok(status) => status,
                    Err(error) => {
                        errors.push(format!("try_wait: {error}"));
                        None
                    }
                };
                let mut signaled = false;
                if status.is_none() {
                    match Command::new("/usr/bin/kill")
                        .args(["-TERM", &self.id().to_string()])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                    {
                        Ok(result) if result.success() => signaled = true,
                        result => errors.push(format!("TERM: {result:?}")),
                    }
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(35);
                    while std::time::Instant::now() < deadline {
                        match self.child.try_wait() {
                            Ok(Some(exit)) => {
                                status = Some(exit);
                                break;
                            }
                            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
                            Err(error) => {
                                errors.push(format!("try_wait: {error}"));
                                break;
                            }
                        }
                    }
                }
                if status.is_none() {
                    forced = true;
                    if let Err(error) = self.child.kill() {
                        errors.push(format!("KILL: {error}"));
                    }
                }
                // Even a failed signal or expired grace period reaches actual
                // wait. The outer custody timeout bounds an unresponsive kernel.
                match self.child.wait() {
                    Ok(exit) => {
                        status = Some(exit);
                        self.collected = true;
                    }
                    Err(error) => errors.push(format!("wait: {error}")),
                }
                let outcome = json!({"host_pid":self.id(),"collected":self.collected,
                    "term_sent":signaled,"forced":forced,"status":status.map(|s|s.to_string()),
                    "normal_shutdown":self.collected && signaled && !forced && errors.is_empty() && status.is_some_and(|s|s.success()),
                    "errors":errors});
                self.outcomes
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(outcome.clone());
                let _ = writeln!(std::io::stderr(), "controlled host cleanup {outcome}");
                if !self.collected {
                    // Returning would release TempDir while a host may remain.
                    // Keep the original panic/unknown outcome in custody logs and
                    // fail closed into the enclosing PID-namespace containment.
                    std::process::abort();
                }
                outcome
            }
        }
        impl Drop for OwnedHost {
            fn drop(&mut self) {
                if !self.collected {
                    self.collect();
                }
            }
        }
        fn start(
            selection: &Path,
            authorization: &Path,
            socket: &Path,
            outcomes: &std::sync::Arc<std::sync::Mutex<Vec<Value>>>,
            fail_after_spawn: bool,
        ) -> OwnedHost {
            let child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "selected_startup::tests::installed_selected_subprocess_entry",
                    "--exact",
                    "--ignored",
                    "--nocapture",
                ])
                .env("M4CB_TEST_SELECTION", selection)
                .env("M4CB_TEST_AUTHORIZATION", authorization)
                .env(
                    "M4CB_TEST_SEED_HISTORY",
                    if selection.file_name().unwrap() == "selection.json" {
                        "1"
                    } else {
                        "0"
                    },
                )
                .stdin(Stdio::null())
                .spawn()
                .unwrap();
            let mut owned = OwnedHost {
                child,
                outcomes: outcomes.clone(),
                collected: false,
            };
            assert!(!fail_after_spawn, "controlled post-spawn host assertion");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
            while !socket.exists() {
                assert!(
                    owned.child.try_wait().unwrap().is_none(),
                    "separate selected host exited before service"
                );
                assert!(
                    std::time::Instant::now() < deadline,
                    "separate selected host deadline"
                );
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            owned
        }
        fn stop(child: &mut OwnedHost) {
            let outcome = child.collect();
            assert_eq!(
                outcome["normal_shutdown"], true,
                "unsafe/unknown host shutdown: {outcome}"
            );
        }
        // Capture only the known private flat record directory and named inputs;
        // never copy live owner databases or follow a symlink. Capture points
        // below run before spawn or after explicit owner stop/wait.
        fn capture(
            stages: &mut Vec<Value>,
            output: &Option<PathBuf>,
            root: &Path,
            stage: &str,
            extra: &[&Path],
        ) {
            if output.is_none() {
                return;
            }
            use std::io::Read;
            use std::os::unix::fs::OpenOptionsExt;
            assert!(stages.len() < 12);
            assert!(canonical_exact(root).unwrap());
            let protected = root.join("protected");
            assert!(canonical_exact(&protected).unwrap());
            let mut paths = std::collections::BTreeSet::new();
            for entry in fs::read_dir(&protected).unwrap() {
                let entry = entry.unwrap();
                assert!(
                    entry.file_type().unwrap().is_file(),
                    "non-file fixture record"
                );
                paths.insert(entry.path());
            }
            assert!(paths.len() <= 128, "bounded flat fixture records");
            for name in [
                "host.json",
                "selection.json",
                "startup-receipt.json",
                "controlled-history-observed.json",
                "source-manifest.json",
                "Cargo.lock",
                "rust-toolchain.toml",
                "source.txt",
            ] {
                let path = root.join(name);
                match fs::symlink_metadata(&path) {
                    Ok(_) => {
                        paths.insert(path);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => panic!("fixture metadata: {error}"),
                }
            }
            paths.extend(extra.iter().map(|p| p.to_path_buf()));
            let mut files = Vec::new();
            let mut total = 0_usize;
            for path in paths {
                let relative = path.strip_prefix(root).unwrap();
                assert!(
                    relative
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_)))
                );
                assert!(canonical_exact(path.parent().unwrap()).unwrap());
                let before = fs::symlink_metadata(&path).unwrap();
                assert!(
                    before.is_file() && before.nlink() == 1 && before.len() <= MAX_CONFIG_BYTES
                );
                // Linux O_NOFOLLOW; this controlled fixture already requires /proc.
                let mut file = OpenOptions::new()
                    .read(true)
                    .custom_flags(0o400000)
                    .open(&path)
                    .unwrap();
                let metadata = file.metadata().unwrap();
                assert_eq!(
                    (metadata.dev(), metadata.ino()),
                    (before.dev(), before.ino())
                );
                let mut bytes = Vec::new();
                std::io::Read::by_ref(&mut file)
                    .take(MAX_CONFIG_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .unwrap();
                assert_eq!(bytes.len() as u64, metadata.len());
                assert!(bytes.len() as u64 <= MAX_CONFIG_BYTES);
                total += bytes.len();
                assert!(total <= 1024 * 1024, "bounded capture stage");
                files.push(json!({"path":relative,"length":bytes.len(),"sha256":sha_bytes(&bytes),
                    "device":metadata.dev(),"inode":metadata.ino(),"mode":metadata.mode(),"bytes":bytes}));
            }
            stages.push(json!({"stage":stage,"files":files}));
        }
        fn publish_capture(output: &Option<PathBuf>, stages: Vec<Value>, outcomes: &Value) {
            let Some(output) = output else {
                return;
            };
            use std::os::unix::fs::OpenOptionsExt;
            assert!(matches!(
                output.to_str(),
                Some("/run/m4cb/restart-evidence.json" | "/run/m4cb/restart-unwind-evidence.json")
            ));
            assert!(canonical_exact(output.parent().unwrap()).unwrap());
            let bytes = serde_json::to_vec(&json!({"qualification":"controlled_only",
                "stages":stages,"observed_outputs":outcomes}))
            .unwrap();
            assert!(bytes.len() <= 32 * 1024 * 1024, "bounded retained capture");
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(0o400000)
                .open(output)
                .unwrap();
            file.write_all(&bytes).unwrap();
            file.sync_all().unwrap();
            println!(
                "restart fixture capture length={} sha256={}",
                bytes.len(),
                sha_bytes(&bytes)
            );
        }
        fn notice(socket: &Path, payload: &Value) -> Value {
            let result = crate::exchange(
                socket,
                &json!({"op":"research.invalidate.v1","payload":payload}),
            )
            .unwrap();
            assert_eq!(result["status"], "invalidated");
            result
        }
        fn observation(result: &Value) -> Value {
            let bytes: Vec<u8> = serde_json::from_value(
                result["receipt"]["admission"]["currentness_evidence"]["bytes"].clone(),
            )
            .unwrap();
            serde_json::from_slice(&bytes).unwrap()
        }
        let f = Fixture::new();
        let directory = f.config.parent().unwrap();
        let capture_output = std::env::var_os("M4CB_RESTART_EVIDENCE").map(PathBuf::from);
        let mut captured_stages = Vec::new();
        let host_outcomes = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));

        let mut startup: HostStartup =
            serde_json::from_slice(&fs::read(&f.config).unwrap()).unwrap();
        let native = startup.lifecycle.native.as_mut().unwrap();
        native.executable = PathBuf::from(std::env::var_os("M4CB_NATIVE").unwrap());
        fs::create_dir(&native.root).unwrap();
        fs::set_permissions(&native.root, fs::Permissions::from_mode(0o700)).unwrap();
        for scope in ["native_turn", "snapshot_read"] {
            startup
                .lifecycle
                .grants
                .push(lifecycle_service::GrantConfig {
                    grant_ref: if scope == "native_turn" {
                        "grant:native"
                    } else {
                        "grant:snapshot"
                    }
                    .into(),
                    context_generation: startup.lifecycle.context_generation.clone(),
                    scope: scope.into(),
                    expires_wall_ms: i64::MAX,
                    revision: 1,
                });
        }
        let (mut startup, _, payload) = attach_records(directory, startup);
        let successor_notice = append_notice_intent(&mut startup, &payload, "notice-successor");
        let continuation_notice =
            append_notice_intent(&mut startup, &payload, "notice-continuation");
        // Seed only through the existing controlled owner APIs in the real
        // predecessor host. Exact old I/C are retained in B; O names the finite
        // prepare/adopt payloads before either host is started.
        let mut basis = crate::research_admission::tests::fixture().bases.remove(0);
        basis.key = serde_json::from_value(payload["key"].clone()).unwrap();
        basis.submit = serde_json::from_value(json!({"command_id":"controlled-history-adoption","task_id":basis.key.task_id,"subject_id":basis.key.subject_id,"kind":"repository_analysis","objective":"retained controlled history","engineering_question":"preserve exact prior input","snapshot_ref":"snapshot:selected","facts":[]})).unwrap();
        basis.snapshot_ref = basis.submit.snapshot_ref.clone();
        basis
            .dependencies
            .insert("research.source".into(), sha_bytes(b"source"));
        basis
            .dependency_artifacts
            .insert("research.source".into(), "source".into());
        let result = basis
            .artifacts
            .iter_mut()
            .find(|a| a.id == "result")
            .unwrap();
        result.bytes = serde_json::to_vec(&json!({"dependencies":basis.dependencies,"text":"controlled historical recommendation; no C1 or production authority"})).unwrap();
        result.digest = sha_bytes(&result.bytes);
        let installed = serde_json::to_value(
            startup
                .research
                .as_ref()
                .unwrap()
                .installation
                .as_ref()
                .unwrap(),
        )
        .unwrap();
        for field in ["installation", "configuration"] {
            let bytes = fs::read(
                directory
                    .join("protected")
                    .join(installed[field]["name"].as_str().unwrap()),
            )
            .unwrap();
            let artifact = basis.artifacts.iter_mut().find(|a| a.id == field).unwrap();
            artifact.bytes = bytes;
            artifact.digest = sha_bytes(&artifact.bytes);
            if field == "installation" {
                basis.installation_digest = artifact.digest.clone();
            } else {
                basis.configuration_digest = artifact.digest.clone();
            }
        }
        crate::research_installation::tests::append_history_intents(&mut startup, &basis);
        write_private_json(&f.config, &serde_json::to_value(&startup).unwrap());
        let mut selection: Value =
            serde_json::from_slice(&fs::read(&f.selection).unwrap()).unwrap();
        selection["config_sha256"] = json!(sha_file(&f.config).unwrap());
        write_private_json(&f.selection, &selection);
        let auth = authorization_named(
            &startup,
            &fs::read(&f.selection).unwrap(),
            &fs::read(&f.config).unwrap(),
            "operator-input-predecessor",
        );
        // Establish selected owner custody, then seed an existing ordinary task
        // before nomination. No lifecycle command is enqueued and no turn runs.
        CONTROLLED_OPERATOR_INPUTS
            .lock()
            .unwrap()
            .insert(auth.clone());
        let prepared = prepare_with_operator(&f.selection, Some(&auth)).unwrap();
        let input: retained_task_workflow::SubmitTask = serde_json::from_value(json!({
            "command_id":"seed-task","task_id":"task-0","subject_id":"subject:selected",
            "kind":"repository_analysis","objective":"retained controlled history",
            "engineering_question":"preserve exact prior input","snapshot_ref":"snapshot:selected","facts":[]
        })).unwrap();
        let commit = prepared.task_store.submit(&input).unwrap();
        drop(prepared);
        CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);
        capture(
            &mut captured_stages,
            &capture_output,
            directory,
            "predecessor-input",
            &[&auth],
        );
        if std::env::var("M4CB_RESTART_GUARD_FAILURE").as_deref() == Ok("post-spawn") {
            let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _owned = start(&f.selection, &auth, &f.host_socket, &host_outcomes, true);
            }))
            .expect_err("post-spawn control must retain the original assertion");
            let message = failure
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| failure.downcast_ref::<String>().map(String::as_str));
            assert_eq!(message, Some("controlled post-spawn host assertion"));
            let outcomes = host_outcomes.lock().unwrap().clone();
            assert_eq!(outcomes.len(), 1);
            assert_eq!(outcomes[0]["collected"], true);
            let pid = outcomes[0]["host_pid"].as_u64().unwrap() as u32;
            assert!(process_starttime(pid).unwrap().is_none());
            assert!(
                directory.exists(),
                "fixture custody outlives actual host wait"
            );
            capture(
                &mut captured_stages,
                &capture_output,
                directory,
                "post-spawn-unwind-collected",
                &[&auth],
            );
            if let Some(marker) = std::env::var_os("M4CB_MODEL_ENTRY_SENTINEL") {
                assert!(
                    !Path::new(&marker).exists(),
                    "post-spawn unwind entered model"
                );
            }
            publish_capture(
                &capture_output,
                captured_stages,
                &json!({"original_panic":"controlled post-spawn host assertion","host_cleanup":outcomes}),
            );
            println!(
                "controlled post-spawn unwind original-panic=retained actual-host={pid} wait=collected fixture=retained"
            );
            return;
        }
        let mut predecessor = start(&f.selection, &auth, &f.host_socket, &host_outcomes, false);
        let predecessor_host = predecessor.id();
        let predecessor_ticks = process_starttime(predecessor_host).unwrap().unwrap();
        let original = notice(&f.host_socket, &payload);
        let history_bytes = fs::read(directory.join("controlled-history-observed.json")).unwrap();
        let history: Value = serde_json::from_slice(&history_bytes).unwrap();
        assert_eq!(history["qualification"], "controlled_only");
        assert_eq!(
            history["basis"]["configuration_digest"],
            installed["configuration"]["sha256"]
        );
        assert_eq!(
            history["basis"]["installation_digest"],
            installed["installation"]["sha256"]
        );
        assert_eq!(history["lifecycle_rejection"]["accepted"], false);
        let original_evidence = observation(&original);
        assert_eq!(original_evidence["host_pid"], predecessor_host);
        let native_pid = original_evidence["observation"]["native_pid"]
            .as_u64()
            .unwrap() as u32;
        let native_ticks = process_starttime(native_pid).unwrap().unwrap();
        assert_ne!(native_pid, std::process::id());
        stop(&mut predecessor);
        assert!(process_starttime(predecessor_host).unwrap().is_none());
        assert!(process_starttime(native_pid).unwrap().is_none());
        let retained = fs::read_dir(directory.join("protected"))
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect::<Vec<_>>();
        let old_startup = fs::read(&f.config).unwrap();
        let old_selection = fs::read(&f.selection).unwrap();
        let receipt_path = directory.join("startup-receipt.json");
        let receipt_bytes = fs::read(&receipt_path).unwrap();
        let receipt: Value = serde_json::from_slice(&receipt_bytes).unwrap();
        let task_store = TaskStore::open(&f.task, "operator:selected".into()).unwrap();
        let owner = task_store
            .observe_subject("subject:selected")
            .unwrap()
            .unwrap();
        assert_eq!(task_store.submit(&input).unwrap(), commit);
        let old_owner = serde_json::to_vec(&owner).unwrap();
        drop(task_store);
        let db = rusqlite::Connection::open_with_flags(
            f.lifecycle_root.join("lifecycle.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let state: (u64,u64,u64) = db.query_row("SELECT revision,semantic_revision,queue_revision FROM subjects WHERE subject_id='subject:selected' AND owner_kind='domain' AND owner_ref IS NULL", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        let cursor: u64 = db
            .query_row("SELECT COALESCE(MAX(sequence),0) FROM journal", [], |r| {
                r.get(0)
            })
            .unwrap();
        let pins: (String,String,String,String,String) = db.query_row("SELECT profile_digest,binary_sha256,protocol_digest,snapshot_id,snapshot_manifest_sha256 FROM native_sessions WHERE session_id='session:selected' AND role='domain'", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM native_turns", [], |r| r
                .get::<_, u64>(0))
                .unwrap(),
            0
        );
        drop(db);
        let predecessor_startup = startup.clone();
        let native = startup.lifecycle.native.as_mut().unwrap();
        native.session_id = "session:installed-successor".into();
        native.invocation_id = "invocation:installed-successor".into();
        native.attempt_id = "attempt:installed-successor".into();
        native.root = directory.join("native-successor");
        fs::create_dir(&native.root).unwrap();
        fs::set_permissions(&native.root, fs::Permissions::from_mode(0o700)).unwrap();
        refreshed_records(&mut startup);
        let config2 = directory.join("host-successor.json");
        write_private_json(&config2, &serde_json::to_value(&startup).unwrap());
        let selection2 = directory.join("selection-successor.json");
        let declaration_path = directory.join("restart-installed.json");
        selection["version"] = json!(3);
        selection["selection_id"] = json!("selected-installed-successor");
        selection["config_path"] = json!(config2);
        selection["config_sha256"] = json!(sha_file(&config2).unwrap());
        selection["predecessor_selection_path"] = json!(f.selection);
        selection["restart_declaration_path"] = json!(declaration_path);
        write_private_json(&selection2, &selection);
        let declaration = json!({"version":1,"selection_sha256":sha_file(&selection2).unwrap(),
            "predecessor_selection_sha256":sha_bytes(&old_selection),"predecessor_receipt_sha256":sha_bytes(&receipt_bytes),
            "task":receipt["task"],"lifecycle":receipt["lifecycle"],"subject_id":"subject:selected","context_generation":"context:selected",
            "task_id":owner.task_id,"task_revision":owner.revision,"task_cursor_revision":owner.cursor_revision,
            "predecessor_session_id":"session:selected","predecessor_epoch":0,"target_session_id":"session:installed-successor",
            "expected_subject_revision":state.0,"expected_semantic_revision":state.1,"expected_queue_revision":state.2,"expected_journal_cursor":cursor,
            "profile_digest":pins.0,"binary_sha256":pins.1,"protocol_digest":pins.2,"snapshot_id":pins.3,"snapshot_manifest_sha256":pins.4,
            "operator_ref":"operator:controlled-restart","cooperative_stop_asserted":true,"predecessor_host_pid":predecessor_host,
            "predecessor_host_starttime_ticks":predecessor_ticks,"predecessor_native_pid":native_pid,"predecessor_native_starttime_ticks":native_ticks});
        write_private_json(&declaration_path, &declaration);
        let auth2 = authorization_named(
            &startup,
            &fs::read(&selection2).unwrap(),
            &fs::read(&config2).unwrap(),
            "operator-input-successor",
        );
        for case in [
            "changed-prompt",
            "stale-current-C",
            "changed-intent",
            "missing-owner-source",
            "old-operation-link",
        ] {
            let mut candidate = startup.clone();
            crate::research_installation::tests::restart_negative(
                &mut candidate,
                &predecessor_startup,
                case,
            );
            // Semantic/intent variants pass the complete static stage: the
            // reached restart comparison, rather than a malformed early input,
            // must reject them. Other variants intentionally test exact links.
            if matches!(case, "changed-prompt" | "changed-intent") {
                crate::research_installation::validate_records(&candidate).unwrap();
            }
            let candidate_config = directory.join(format!("host-{case}.json"));
            write_private_json(
                &candidate_config,
                &serde_json::to_value(&candidate).unwrap(),
            );
            let mut candidate_selection = selection.clone();
            candidate_selection["config_path"] = json!(candidate_config);
            candidate_selection["config_sha256"] = json!(sha_file(&candidate_config).unwrap());
            let candidate_selection_path = directory.join(format!("selection-{case}.json"));
            let candidate_declaration_path = directory.join(format!("restart-{case}.json"));
            candidate_selection["restart_declaration_path"] = json!(candidate_declaration_path);
            write_private_json(&candidate_selection_path, &candidate_selection);
            let mut candidate_declaration = declaration.clone();
            candidate_declaration["selection_sha256"] =
                json!(sha_file(&candidate_selection_path).unwrap());
            write_private_json(&candidate_declaration_path, &candidate_declaration);
            let candidate_auth = authorization_named(
                &candidate,
                &fs::read(&candidate_selection_path).unwrap(),
                &fs::read(&candidate_config).unwrap(),
                &format!("operator-{case}"),
            );
            CONTROLLED_OPERATOR_INPUTS
                .lock()
                .unwrap()
                .insert(candidate_auth.clone());
            let before = startup_inventory(directory);
            let result = prepare_with_operator(&candidate_selection_path, Some(&candidate_auth));
            assert!(
                result.is_err(),
                "accepted refreshed restart negative {case}"
            );
            assert_eq!(
                before,
                startup_inventory(directory),
                "restart effects for {case}"
            );
            CONTROLLED_OPERATOR_INPUTS
                .lock()
                .unwrap()
                .remove(&candidate_auth);
            println!("refreshed restart negative case={case} unchanged-full-inventory");
        }
        // Fully rebound candidates have valid current records and unchanged
        // semantics. Only the native identity distinctness predicate fails.
        for case in ["reused-root", "reused-session"] {
            let mut candidate = startup.clone();
            let old_native = predecessor_startup.lifecycle.native.as_ref().unwrap();
            let native = candidate.lifecycle.native.as_mut().unwrap();
            if case == "reused-root" {
                native.root.clone_from(&old_native.root);
                assert_ne!(native.session_id, old_native.session_id);
            } else {
                native.session_id.clone_from(&old_native.session_id);
                assert_ne!(native.root, old_native.root);
            }
            refreshed_records_named(&mut candidate, case);
            crate::validate_startup(&candidate).unwrap();
            crate::research_installation::validate_records(&candidate).unwrap();
            let mut normalized = candidate.clone();
            let native = normalized.lifecycle.native.as_mut().unwrap();
            native.root.clone_from(&old_native.root);
            native.session_id.clone_from(&old_native.session_id);
            native.invocation_id.clone_from(&old_native.invocation_id);
            native.attempt_id.clone_from(&old_native.attempt_id);
            crate::research_installation::normalize_installed_restart(
                &predecessor_startup,
                &candidate,
                &mut normalized,
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(&normalized).unwrap(),
                serde_json::to_value(&predecessor_startup).unwrap()
            );
            let config = directory.join(format!("host-{case}.json"));
            write_private_json(&config, &serde_json::to_value(&candidate).unwrap());
            let selection_path = directory.join(format!("selection-{case}.json"));
            let declaration_path = directory.join(format!("restart-{case}.json"));
            let mut selected = selection.clone();
            selected["selection_id"] = json!(format!("selected-{case}"));
            selected["config_path"] = json!(config);
            selected["config_sha256"] = json!(sha_file(&config).unwrap());
            selected["restart_declaration_path"] = json!(declaration_path);
            write_private_json(&selection_path, &selected);
            let mut requested = declaration.clone();
            requested["selection_sha256"] = json!(sha_file(&selection_path).unwrap());
            requested["target_session_id"] =
                json!(candidate.lifecycle.native.as_ref().unwrap().session_id);
            write_private_json(&declaration_path, &requested);
            let authorization = authorization_named(
                &candidate,
                &fs::read(&selection_path).unwrap(),
                &fs::read(&config).unwrap(),
                &format!("operator-{case}"),
            );
            CONTROLLED_OPERATOR_INPUTS
                .lock()
                .unwrap()
                .insert(authorization.clone());
            let recorded: Receipt = serde_json::from_value(receipt.clone()).unwrap();
            let selected: Selection = serde_json::from_value(selected).unwrap();
            assert!(
                !compatible_restart(&selection_path, &selected, &candidate, &recorded).unwrap()
            );
            capture(
                &mut captured_stages,
                &capture_output,
                directory,
                case,
                &[&config, &selection_path, &declaration_path, &authorization],
            );
            let before = startup_inventory(directory);
            assert!(
                matches!(
                    prepare_with_operator(&selection_path, Some(&authorization)),
                    Err(HostError::Configuration)
                ),
                "wrong refusal for {case}"
            );
            assert_eq!(
                before,
                startup_inventory(directory),
                "identity refusal effects {case}"
            );
            CONTROLLED_OPERATOR_INPUTS
                .lock()
                .unwrap()
                .remove(&authorization);
            println!(
                "restart ownership case={case} static=valid normalized=exact consumer=compatible_restart refusal=Configuration unchanged-full-inventory"
            );
        }
        capture(
            &mut captured_stages,
            &capture_output,
            directory,
            "first-successor-input",
            &[&config2, &selection2, &declaration_path, &auth2],
        );
        let mut successor = start(&selection2, &auth2, &f.host_socket, &host_outcomes, false);
        let retry_adoption = crate::exchange(
            &f.host_socket,
            &json!({"op":"research.adopt.v1","payload":history["request"]}),
        )
        .unwrap();
        assert_eq!(
            retry_adoption["adoption"], history["adoption"],
            "historical adoption owner receipt must remain exact"
        );
        let command_result = crate::exchange(&f.service_socket,&json!({"op":"command_result","subject_id":"subject:selected","command_id":history["adoption"]["command"]["command_id"],"request_digest":history["adoption"]["command"]["request_digest"]})).unwrap();
        assert_eq!(
            command_result, history["lifecycle_rejection"],
            "successor must read actual terminal predecessor rejection"
        );
        let fresh = notice(&f.host_socket, &successor_notice);
        let new_evidence = observation(&fresh);
        assert_eq!(new_evidence["host_pid"], successor.id());
        assert_ne!(new_evidence["observation"]["native_pid"], native_pid);
        assert_ne!(
            fresh["receipt"]["admission"]["configuration_digest"],
            original["receipt"]["admission"]["configuration_digest"]
        );
        assert_eq!(notice(&f.host_socket, &payload), original);
        let successor_ticks = process_starttime(successor.id()).unwrap().unwrap();
        let successor_native_pid =
            new_evidence["observation"]["native_pid"].as_u64().unwrap() as u32;
        let successor_native_ticks = process_starttime(successor_native_pid).unwrap().unwrap();
        stop(&mut successor);
        assert!(process_starttime(successor.id()).unwrap().is_none());
        assert!(process_starttime(successor_native_pid).unwrap().is_none());

        // Observe the real stopped successor's epoch and all owner revisions.
        // No SQL writes or manufactured owner/epoch seed are used.
        fn owner_state(path: &Path) -> std::collections::BTreeMap<String, Vec<Vec<String>>> {
            let db = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            let tables = db
                .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
                .unwrap()
                .query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            tables
                .into_iter()
                .map(|table| {
                    let mut query = db
                        .prepare(&format!("SELECT * FROM \"{}\"", table.replace('"', "\"\"")))
                        .unwrap();
                    let columns = query.column_count();
                    let mut rows = query
                        .query_map([], |row| {
                            (0..columns)
                                .map(|column| row.get_ref(column).map(|value| format!("{value:?}")))
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .unwrap()
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();
                    rows.sort();
                    (table, rows)
                })
                .collect()
        }
        let receipt2_bytes = fs::read(&receipt_path).unwrap();
        let receipt2: Value = serde_json::from_slice(&receipt2_bytes).unwrap();
        let db = rusqlite::Connection::open_with_flags(
            f.lifecycle_root.join("lifecycle.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let epoch: u64 = db.query_row("SELECT epoch FROM native_execution_epochs WHERE target_session_id='session:installed-successor' AND state='active'", [], |r| r.get(0)).unwrap();
        assert!(
            epoch > 0,
            "stale epoch must be genuinely older than a real owner epoch"
        );
        let state: (u64,u64,u64) = db.query_row("SELECT revision,semantic_revision,queue_revision FROM subjects WHERE subject_id='subject:selected' AND owner_kind='domain' AND owner_ref IS NULL", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        let cursor: u64 = db
            .query_row("SELECT COALESCE(MAX(sequence),0) FROM journal", [], |r| {
                r.get(0)
            })
            .unwrap();
        let pins: (String,String,String,String,String) = db.query_row("SELECT profile_digest,binary_sha256,protocol_digest,snapshot_id,snapshot_manifest_sha256 FROM native_sessions WHERE session_id='session:installed-successor' AND subject_id='subject:selected' AND context_id='context:selected' AND role IN ('domain','successor')", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap();
        drop(db);
        let task_store = TaskStore::open(&f.task, "operator:selected".into()).unwrap();
        let owner2 = task_store
            .observe_subject("subject:selected")
            .unwrap()
            .unwrap();
        drop(task_store);
        let mut continuation = startup.clone();
        let native = continuation.lifecycle.native.as_mut().unwrap();
        native.session_id = "session:installed-continuation".into();
        native.invocation_id = "invocation:installed-continuation".into();
        native.attempt_id = "attempt:installed-continuation".into();
        native.root = directory.join("native-continuation");
        fs::create_dir(&native.root).unwrap();
        fs::set_permissions(&native.root, fs::Permissions::from_mode(0o700)).unwrap();
        refreshed_records_named(&mut continuation, "continuation");
        crate::validate_startup(&continuation).unwrap();
        crate::research_installation::validate_records(&continuation).unwrap();
        let config3 = directory.join("host-continuation.json");
        write_private_json(&config3, &serde_json::to_value(&continuation).unwrap());
        let selection3 = directory.join("selection-continuation.json");
        let declaration3_path = directory.join("restart-continuation.json");
        let mut selected3 = selection.clone();
        selected3["selection_id"] = json!("selected-installed-continuation");
        selected3["config_path"] = json!(config3);
        selected3["config_sha256"] = json!(sha_file(&config3).unwrap());
        selected3["predecessor_selection_path"] = json!(selection2);
        selected3["restart_declaration_path"] = json!(declaration3_path);
        write_private_json(&selection3, &selected3);
        let mut declaration3 = declaration.clone();
        declaration3["selection_sha256"] = json!(sha_file(&selection3).unwrap());
        declaration3["predecessor_selection_sha256"] = json!(sha_file(&selection2).unwrap());
        declaration3["predecessor_receipt_sha256"] = json!(sha_bytes(&receipt2_bytes));
        declaration3["task"] = receipt2["task"].clone();
        declaration3["lifecycle"] = receipt2["lifecycle"].clone();
        declaration3["task_id"] = json!(owner2.task_id);
        declaration3["task_revision"] = json!(owner2.revision);
        declaration3["task_cursor_revision"] = json!(owner2.cursor_revision);
        declaration3["predecessor_session_id"] = json!("session:installed-successor");
        declaration3["predecessor_epoch"] = json!(epoch - 1);
        declaration3["target_session_id"] = json!("session:installed-continuation");
        declaration3["expected_subject_revision"] = json!(state.0);
        declaration3["expected_semantic_revision"] = json!(state.1);
        declaration3["expected_queue_revision"] = json!(state.2);
        declaration3["expected_journal_cursor"] = json!(cursor);
        declaration3["profile_digest"] = json!(pins.0);
        declaration3["binary_sha256"] = json!(pins.1);
        declaration3["protocol_digest"] = json!(pins.2);
        declaration3["snapshot_id"] = json!(pins.3);
        declaration3["snapshot_manifest_sha256"] = json!(pins.4);
        declaration3["predecessor_host_pid"] = json!(successor.id());
        declaration3["predecessor_host_starttime_ticks"] = json!(successor_ticks);
        declaration3["predecessor_native_pid"] = json!(successor_native_pid);
        declaration3["predecessor_native_starttime_ticks"] = json!(successor_native_ticks);
        write_private_json(&declaration3_path, &declaration3);
        let auth3 = authorization_named(
            &continuation,
            &fs::read(&selection3).unwrap(),
            &fs::read(&config3).unwrap(),
            "operator-input-continuation",
        );
        CONTROLLED_OPERATOR_INPUTS
            .lock()
            .unwrap()
            .insert(auth3.clone());
        // These exact read-only preflights eliminate configuration failures at
        // record compatibility and SelectedLifecycleStore::from_store. The full
        // selected entry's nested service Configuration is then the owner basis
        // mismatch. Check every fallible into_intent identity parser explicitly;
        // every remaining service basis field was read from this actual owner.
        lifecycle_core::SubjectId::parse(declaration3["subject_id"].as_str().unwrap()).unwrap();
        lifecycle_core::ContextGeneration::parse(
            declaration3["context_generation"].as_str().unwrap(),
        )
        .unwrap();
        for field in ["predecessor_session_id", "target_session_id"] {
            lifecycle_core::NativeSessionId::parse(declaration3[field].as_str().unwrap()).unwrap();
        }
        lifecycle_core::SnapshotId::parse(declaration3["snapshot_id"].as_str().unwrap()).unwrap();
        let selected3: Selection = serde_json::from_value(selected3).unwrap();
        read_restart_declaration(
            &selection3,
            directory,
            &selected3,
            &continuation,
            &sha_file(&selection3).unwrap(),
            &receipt_path,
        )
        .unwrap();
        drop(
            SelectedLifecycleStore::open_for_restart(
                &continuation.lifecycle,
                receipt2["lifecycle"]["store_id"].as_str().unwrap(),
            )
            .unwrap(),
        );
        capture(
            &mut captured_stages,
            &capture_output,
            directory,
            "older-epoch-input",
            &[
                &config2,
                &selection2,
                &declaration_path,
                &auth2,
                &config3,
                &selection3,
                &declaration3_path,
                &auth3,
            ],
        );
        let before_task = owner_state(&f.task);
        let before_lifecycle = owner_state(&f.lifecycle_root.join("lifecycle.sqlite"));
        let before_inventory = startup_inventory(directory);
        assert!(
            matches!(
                prepare_with_operator(&selection3, Some(&auth3)),
                Err(HostError::Lifecycle(
                    lifecycle_service::ServiceError::Configuration
                ))
            ),
            "stale epoch must reach selected service authorization, not record refusal or store CAS"
        );
        assert_eq!(owner_state(&f.task), before_task);
        assert_eq!(
            owner_state(&f.lifecycle_root.join("lifecycle.sqlite")),
            before_lifecycle
        );
        assert_eq!(fs::read(&receipt_path).unwrap(), receipt2_bytes);
        assert_eq!(startup_inventory(directory), before_inventory);
        assert!(!f.host_socket.exists());
        assert!(!f.service_socket.exists());
        println!(
            "restart ownership case=older-epoch actual={epoch} requested={} consumer=SelectedLifecycleStore::authorize_restart refusal=Lifecycle(Configuration) owner-state=exact transactional-ClaimConflict=unexecuted",
            epoch - 1
        );
        // Only the declaration epoch changes. The failed attempt did not need
        // owner repair; the same selected/config/authorization/records now run.
        let stale_declaration = declaration3.clone();
        declaration3["predecessor_epoch"] = json!(epoch);
        let mut comparison = declaration3.clone();
        comparison["predecessor_epoch"] = stale_declaration["predecessor_epoch"].clone();
        assert_eq!(comparison, stale_declaration);
        write_private_json(&declaration3_path, &declaration3);
        CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth3);
        capture(
            &mut captured_stages,
            &capture_output,
            directory,
            "current-epoch-input",
            &[
                &config2,
                &selection2,
                &declaration_path,
                &auth2,
                &config3,
                &selection3,
                &declaration3_path,
                &auth3,
            ],
        );
        let mut continuation_host =
            start(&selection3, &auth3, &f.host_socket, &host_outcomes, false);
        let final_notice = notice(&f.host_socket, &continuation_notice);
        let final_evidence = observation(&final_notice);
        assert_eq!(final_evidence["host_pid"], continuation_host.id());
        assert_ne!(
            final_evidence["observation"]["native_pid"],
            successor_native_pid
        );
        assert_ne!(
            final_notice["receipt"]["admission"]["configuration_digest"],
            fresh["receipt"]["admission"]["configuration_digest"]
        );
        assert_eq!(notice(&f.host_socket, &payload), original);
        assert_eq!(notice(&f.host_socket, &successor_notice), fresh);
        let retry = crate::exchange(
            &f.host_socket,
            &json!({"op":"research.adopt.v1","payload":history["request"]}),
        )
        .unwrap();
        assert_eq!(retry["adoption"], history["adoption"]);
        assert_eq!(crate::exchange(&f.service_socket,&json!({"op":"command_result","subject_id":"subject:selected","command_id":history["adoption"]["command"]["command_id"],"request_digest":history["adoption"]["command"]["request_digest"]})).unwrap(), history["lifecycle_rejection"]);
        let final_native_pid = final_evidence["observation"]["native_pid"]
            .as_u64()
            .unwrap() as u32;
        stop(&mut continuation_host);
        assert!(process_starttime(continuation_host.id()).unwrap().is_none());
        assert!(process_starttime(final_native_pid).unwrap().is_none());
        let db = rusqlite::Connection::open_with_flags(
            f.lifecycle_root.join("lifecycle.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM native_turns", [], |r| r
                .get::<_, u64>(0))
                .unwrap(),
            0,
            "restart qualification must never enter inference"
        );
        let final_epoch: u64 = db.query_row("SELECT epoch FROM native_execution_epochs WHERE target_session_id='session:installed-continuation' AND state='active'", [], |r|r.get(0)).unwrap();
        assert_eq!(final_epoch, epoch + 1);
        println!(
            "restart continuation observed_epoch={final_epoch} stopped_host={} stopped_native={} history=exact fresh-notice=admitted",
            continuation_host.id(),
            final_native_pid
        );
        drop(db);
        assert_eq!(fs::read(&f.config).unwrap(), old_startup);
        assert_eq!(fs::read(&f.selection).unwrap(), old_selection);
        assert_eq!(
            fs::read(directory.join("controlled-history-observed.json")).unwrap(),
            history_bytes
        );
        for (path, bytes) in retained {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
        let task_store = TaskStore::open(&f.task, "operator:selected".into()).unwrap();
        use retained_task_workflow::research_admission::{
            BasisBundle, ProducerRole, ResearchAdopt, research_digest,
        };
        let old_basis: BasisBundle = serde_json::from_value(history["basis"].clone()).unwrap();
        let old_request: ResearchAdopt =
            serde_json::from_value(history["request"].clone()).unwrap();
        let producer = task_store
            .controlled_research_producer(&old_basis.key, ProducerRole::Preparer)
            .unwrap();
        assert_eq!(
            task_store
                .read_research_basis(
                    &old_basis.key,
                    1,
                    &research_digest("basis", &old_basis).unwrap(),
                    &producer
                )
                .unwrap()
                .unwrap(),
            old_basis
        );
        assert_eq!(
            serde_json::to_value(
                task_store
                    .read_research_adoption(
                        &old_request.submit.command_id,
                        &old_request.payload_digest().unwrap()
                    )
                    .unwrap()
                    .unwrap()
            )
            .unwrap(),
            history["adoption"]
        );
        if let Some(marker) = std::env::var_os("M4CB_MODEL_ENTRY_SENTINEL") {
            assert!(
                !Path::new(&marker).exists(),
                "historical reconciliation attempted model entry"
            );
        }
        assert_eq!(task_store.submit(&input).unwrap(), commit);
        assert_eq!(
            serde_json::to_vec(
                &task_store
                    .observe_subject("subject:selected")
                    .unwrap()
                    .unwrap()
            )
            .unwrap(),
            old_owner
        );
        capture(
            &mut captured_stages,
            &capture_output,
            directory,
            "continuation-stopped-history",
            &[
                &config2,
                &selection2,
                &declaration_path,
                &auth2,
                &config3,
                &selection3,
                &declaration3_path,
                &auth3,
            ],
        );
        let outcomes = host_outcomes.lock().unwrap().clone();
        assert_eq!(outcomes.len(), 3);
        assert!(
            outcomes
                .iter()
                .all(|o| o["collected"] == true && o["normal_shutdown"] == true)
        );
        publish_capture(
            &capture_output,
            captured_stages,
            &json!({"original_notice":original,
            "successor_notice":fresh,"continuation_notice":final_notice,"history":history,
            "subject_input":input,"subject_commit":commit,"observed_predecessor_epoch":epoch,
            "observed_final_epoch":final_epoch,"host_cleanup":outcomes}),
        );
        println!(
            "controlled installed v3 stopped_host={predecessor_host} stopped_native={native_pid} successor_host={} historical_retry=exact",
            successor.id()
        );
    }

    #[test]
    #[ignore = "requires separately frozen default host executable"]
    fn default_selected_executable_refuses_installed_startup_before_effects() {
        use crate::research_installation::tests::attach_records;
        let binary = PathBuf::from(std::env::var_os("M4CB_DEFAULT_BINARY").unwrap());
        fn inventory(root: &Path) -> std::collections::BTreeMap<PathBuf, (u64, u64, u32, Vec<u8>)> {
            fn visit(
                path: &Path,
                out: &mut std::collections::BTreeMap<PathBuf, (u64, u64, u32, Vec<u8>)>,
            ) {
                for e in fs::read_dir(path).unwrap() {
                    let p = e.unwrap().path();
                    let m = fs::symlink_metadata(&p).unwrap();
                    out.insert(
                        p.clone(),
                        (
                            m.dev(),
                            m.ino(),
                            m.mode(),
                            if m.is_file() {
                                fs::read(&p).unwrap()
                            } else {
                                Vec::new()
                            },
                        ),
                    );
                    if m.is_dir() {
                        visit(&p, out);
                    }
                }
            }
            let mut out = std::collections::BTreeMap::new();
            visit(root, &mut out);
            out
        }
        for populated in [false, true] {
            let f = Fixture::new();
            if populated {
                drop(prepare(&f.selection).unwrap());
            }
            let startup: HostStartup =
                serde_json::from_slice(&fs::read(&f.config).unwrap()).unwrap();
            let mut startup = attach_records(f.config.parent().unwrap(), startup).0;
            let native_root = &startup.lifecycle.native.as_ref().unwrap().root;
            fs::create_dir_all(native_root).unwrap();
            fs::set_permissions(native_root, fs::Permissions::from_mode(0o700)).unwrap();
            startup.features = Vec::new();
            startup.lifecycle.native.as_mut().unwrap().provider_route =
                crate::NativeProviderRoute::OpenaiChatgptAccessLeaseV1;
            startup
                .lifecycle
                .native
                .as_mut()
                .unwrap()
                .simulator_base_url
                .clear();
            let dir = f.config.parent().unwrap();
            let source = dir.join("source-home");
            fs::create_dir(&source).unwrap();
            fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
            startup
                .lifecycle
                .native
                .as_mut()
                .unwrap()
                .chatgpt_access_source = Some(lifecycle_service::ChatgptAccessSource {
                codex_home: source,
                account_id: "qualification-only".into(),
                credentials_store: lifecycle_service::ChatgptCredentialsStore::File,
                https_proxy: None,
                // Deserialize the ordinary declaration so optional dependency
                // feature fields default to None under feature unification too.
                isolation: Some(serde_json::from_value(json!({
                    "helper_executable":dir.join("helper"),
                    "helper_sha256":"386bdea885e2af49f534db71fe2ea8c9c235f36ece15f0963fc23bccf0f2033a",
                    "proxy_socket":dir.join("proxy/socket"),
                    "proxy_policy_executable":dir.join("policy.py"),
                    "proxy_policy_sha256":"4fef1370d37ef52b128fe2ab89c778e6b7854c74440d3a84f4ccc6822b1a4dfb"
                })).unwrap()),
            });
            startup.provider_identity = startup
                .lifecycle
                .native
                .as_ref()
                .unwrap()
                .provider_route
                .identity()
                .into();
            startup.executable_sha256 = sha_file(&binary).unwrap();
            startup.lifecycle.native.as_mut().unwrap().executable =
                PathBuf::from(std::env::var_os("M4CB_NATIVE").unwrap());
            let mut installed = startup
                .research
                .as_ref()
                .unwrap()
                .installation
                .clone()
                .unwrap();
            crate::research_installation::tests::rebind_records(&mut startup, &mut installed);
            write_private_json(&f.config, &serde_json::to_value(&startup).unwrap());
            let mut selection: Value =
                serde_json::from_slice(&fs::read(&f.selection).unwrap()).unwrap();
            selection["executable_path"] = json!(binary);
            selection["executable_sha256"] = json!(startup.executable_sha256);
            selection["features"] = json!([]);
            selection["provider_identity"] = json!(startup.provider_identity);
            selection["config_sha256"] = json!(sha_file(&f.config).unwrap());
            write_private_json(&f.selection, &selection);
            let states = [
                f.task.clone(),
                f.lifecycle_root.join("lifecycle.sqlite"),
                f.config.parent().unwrap().join("startup-intent.json"),
                f.config.parent().unwrap().join("startup-receipt.json"),
            ];
            let before = states.iter().map(|p| fs::read(p).ok()).collect::<Vec<_>>();
            let complete_before = inventory(f.config.parent().unwrap());
            let output = Command::new(&binary)
                .args(["serve", "--selected"])
                .arg(&f.selection)
                .output()
                .unwrap();
            assert!(!output.status.success());
            assert_eq!(
                complete_before,
                inventory(f.config.parent().unwrap()),
                "default selected entry changed owner/native/startup artifacts"
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("unavailable"),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                before,
                states.iter().map(|p| fs::read(p).ok()).collect::<Vec<_>>()
            );
            assert!(!f.host_socket.exists());
            assert!(!f.service_socket.exists());
            assert!(
                !f.config
                    .parent()
                    .unwrap()
                    .join("startup-receipt.tmp")
                    .exists()
            );
        }
    }

    #[cfg(feature = "controlled-proof")]
    #[test]
    fn malformed_rehashed_installed_approval_refuses_before_every_startup_effect() {
        use crate::research_installation::{
            CONTROLLED_OPERATOR_INPUTS,
            tests::{attach_records, authorization, malformed_approval_pin, malformed_record},
        };
        fn finalize(f: &Fixture, startup: &HostStartup) -> PathBuf {
            write_private_json(&f.config, &serde_json::to_value(startup).unwrap());
            let mut selection: Value =
                serde_json::from_slice(&fs::read(&f.selection).unwrap()).unwrap();
            selection["config_sha256"] = json!(sha_file(&f.config).unwrap());
            selection["provider_identity"] = json!(startup.provider_identity);
            write_private_json(&f.selection, &selection);
            let auth = authorization(
                startup,
                &fs::read(&f.selection).unwrap(),
                &fs::read(&f.config).unwrap(),
            );
            CONTROLLED_OPERATOR_INPUTS
                .lock()
                .unwrap()
                .insert(auth.clone());
            auth
        }
        fn installed(f: &Fixture) -> HostStartup {
            let startup: HostStartup =
                serde_json::from_slice(&fs::read(&f.config).unwrap()).unwrap();
            let native_root = &startup.lifecycle.native.as_ref().unwrap().root;
            fs::create_dir_all(native_root).unwrap();
            fs::set_permissions(native_root, fs::Permissions::from_mode(0o700)).unwrap();
            attach_records(f.config.parent().unwrap(), startup).0
        }
        fn access_installed(f: &Fixture) -> HostStartup {
            let mut startup = installed(f);
            let dir = f.config.parent().unwrap();
            let source = dir.join("source-home");
            fs::create_dir_all(&source).unwrap();
            let native = startup.lifecycle.native.as_mut().unwrap();
            native.provider_route = crate::NativeProviderRoute::OpenaiChatgptAccessLeaseV1;
            native.simulator_base_url.clear();
            native.chatgpt_access_source = Some(lifecycle_service::ChatgptAccessSource {
                codex_home: source,
                account_id: "qualification-only".into(),
                credentials_store: lifecycle_service::ChatgptCredentialsStore::File,
                https_proxy: None,
                isolation: Some(lifecycle_service::ChatgptAccessIsolation {
                    helper_executable: dir.join("helper"),
                    helper_sha256:
                        "386bdea885e2af49f534db71fe2ea8c9c235f36ece15f0963fc23bccf0f2033a".into(),
                    proxy_socket: dir.join("proxy/socket"),
                    proxy_policy_executable: dir.join("policy.py"),
                    proxy_policy_sha256:
                        "4fef1370d37ef52b128fe2ab89c778e6b7854c74440d3a84f4ccc6822b1a4dfb".into(),
                    synthetic_http_socket: None,
                    synthetic_http_port: None,
                    synthetic_http_fixture_executable: None,
                    synthetic_http_fixture_sha256: None,
                }),
            });
            startup.provider_identity = native.provider_route.identity().into();
            crate::research_installation::tests::rebind_configuration(&mut startup);
            startup
        }
        let mut cases: Vec<(&str, Option<Value>)> = vec![
            ("commit", Some(json!("abc"))),
            ("commit", Some(json!("z".repeat(40)))),
            ("blob", Some(json!("x".repeat(40)))),
            ("path", Some(json!(""))),
            ("path", Some(json!("/approval"))),
            ("path", Some(json!("../approval"))),
            ("path", Some(json!("nul\0path"))),
        ];
        cases.extend(
            [
                "isolation-helper-pin",
                "isolation-policy-pin",
                "snapshot-later-member",
                "snapshot-disclosure",
                "reference-alias",
                "reference-order",
                "unsupported-control",
                "max-wait-zero",
                "max-wait-over",
                "max-executions-zero",
                "native-followup-bound",
                "native-relative-root",
                "native-unexpected-auth",
                "installation-kind",
                "installation-version",
                "duplicate-role",
                "issuer",
                "delegation",
                "purpose",
                "roles-empty",
                "artifact-root",
                "missing-member",
                "member-bound",
                "member-name",
                "selector-family",
                "protected-root-overlap",
                "controlled-mix",
                "approval-json",
                "duplicate-json",
                "source-digest",
                "source-path",
                "authority-kind",
                "intent-role",
                "intent-key",
                "intent-payload",
                "intent-duplicate",
                "intent-bound",
                "operation-kind",
                "operation-admission-kind",
                "configuration-kind",
                "current-C",
                "operation-configuration",
                "authorization-version",
                "authorization-selection",
                "authorization-root",
                "missing-authorization",
            ]
            .into_iter()
            .map(|case| (case, None)),
        );
        // A test-only shard selects cases, never changes the malformed input
        // or the complete-entry/no-effect assertions. Each shard keeps both
        // store states and their independent valid comparison.
        let shard = std::env::var("M4CB_MALFORMED_SHARD").ok().map(|s| {
            let (index, count) = s.split_once('/').expect("shard index/count");
            let index: usize = index.parse().expect("shard index");
            let count: usize = count.parse().expect("shard count");
            assert!(count > 0 && index < count && count <= cases.len());
            (index, count)
        });
        for populated in [false, true] {
            // Every adverse case derives from this same valid fixture builder.
            // One complete-entry positive per store state establishes the
            // common contrast without rehashing the identical host executable
            // for a redundant positive on each independent record mutation.
            let good = Fixture::new();
            let startup = installed(&good);
            let auth = finalize(&good, &startup);
            drop(prepare_with_operator(&good.selection, Some(&auth)).unwrap());
            if populated {
                drop(prepare_with_operator(&good.selection, Some(&auth)).unwrap());
            }
            CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);

            let good = Fixture::new();
            let startup = access_installed(&good);
            let auth = finalize(&good, &startup);
            drop(prepare_with_operator(&good.selection, Some(&auth)).unwrap());
            if populated {
                drop(prepare_with_operator(&good.selection, Some(&auth)).unwrap());
            }
            CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);
            for (case_index, (case, value)) in cases.iter().enumerate() {
                if shard.is_some_and(|(index, count)| case_index % count != index)
                    || std::env::var("M4CB_MALFORMED_CASES")
                        .ok()
                        .is_some_and(|selected| !selected.split(',').any(|name| name == *case))
                {
                    continue;
                }
                println!(
                    "malformed selected-entry index={case_index} case={case} populated={populated}"
                );
                let f = Fixture::new();
                let mut startup = if case.starts_with("isolation-") {
                    access_installed(&f)
                } else {
                    installed(&f)
                };
                if populated {
                    let auth = finalize(&f, &startup);
                    let prepared = prepare_with_operator(&f.selection, Some(&auth)).unwrap();
                    // A populated comparison contains real owner history, not
                    // only freshly-created empty SQLite schemas.
                    use retained_task_workflow::research_admission::{
                        ChangeNotice, ProducerRole, ResearchKey,
                    };
                    let input: retained_task_workflow::SubmitTask = serde_json::from_value(json!({
                        "command_id":"static-history-input","task_id":"task:static-history",
                        "subject_id":"subject:static-history","kind":"repository_analysis",
                        "objective":"controlled populated refusal history","engineering_question":"preserve input",
                        "snapshot_ref":"snapshot:selected","facts":[]
                    })).unwrap();
                    prepared.task_store.submit(&input).unwrap();
                    let key = ResearchKey {
                        task_id: "task:notice-history".into(),
                        subject_id: "subject:notice-history".into(),
                        request_digest: sha_bytes(b"history"),
                    };
                    prepared
                        .task_store
                        .controlled_install_research_key(
                            &key,
                            &std::collections::BTreeSet::from(["research.source".into()]),
                        )
                        .unwrap();
                    let producer = prepared
                        .task_store
                        .controlled_research_producer(&key, ProducerRole::ResearchNotice)
                        .unwrap();
                    let notice = ChangeNotice {
                        selector: "research.source".into(),
                        source_revision: sha_bytes(b"prior-source"),
                        reason: "explicit controlled population".into(),
                    };
                    prepared
                        .task_store
                        .invalidate_research(&key, "notice:history", &notice, &producer)
                        .unwrap();
                    let digest = retained_task_workflow::research_admission::research_digest(
                        "notice",
                        &(&key, "notice:history", &notice),
                    )
                    .unwrap();
                    assert!(
                        prepared
                            .task_store
                            .read_research_notice(&key, "notice:history", &digest)
                            .unwrap()
                            .is_some()
                    );
                    drop(prepared);
                    CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);
                }
                if let Some(value) = value {
                    malformed_approval_pin(&mut startup, case, value.clone());
                } else {
                    match *case {
                        "isolation-helper-pin" | "isolation-policy-pin" => {
                            let selected = startup
                                .lifecycle
                                .native
                                .as_mut()
                                .unwrap()
                                .chatgpt_access_source
                                .as_mut()
                                .unwrap()
                                .isolation
                                .as_mut()
                                .unwrap();
                            if *case == "isolation-helper-pin" {
                                selected.helper_sha256 = "0".repeat(64);
                            } else {
                                selected.proxy_policy_sha256 = "0".repeat(64);
                            }
                        }
                        "snapshot-disclosure" => {
                            let protected = f.config.parent().unwrap().join("protected/source");
                            let bytes = fs::read(protected).unwrap();
                            assert!(!bytes.is_empty());
                            let member =
                                &mut startup.lifecycle.native.as_mut().unwrap().snapshot.members[0];
                            member.sha256 = sha_bytes(&bytes);
                            member.bytes = bytes;
                            // This remains a valid snapshot. Only disclosure of
                            // retained protected source bytes is malformed.
                            lifecycle_service::validate_config(
                                &startup.lifecycle,
                                true,
                                true,
                                false,
                            )
                            .unwrap();
                        }
                        "snapshot-later-member" => {
                            let native = startup.lifecycle.native.as_mut().unwrap();
                            let mut invalid = native.snapshot.members.last().unwrap().clone();
                            invalid.id = "zz-invalid-later".into();
                            invalid.sha256 = "0".repeat(64);
                            native.snapshot.members.push(invalid);
                        }
                        "max-wait-zero" => startup.lifecycle.max_wait_ms = 0,
                        "max-wait-over" => startup.lifecycle.max_wait_ms = 30_001,
                        "max-executions-zero" => startup.lifecycle.max_executions = 0,
                        "native-followup-bound" => {
                            let native = startup.lifecycle.native.as_mut().unwrap();
                            let followup = lifecycle_service::NativeFollowupTurn {
                                invocation_id: "invocation:extra".into(),
                                attempt_id: "attempt:extra".into(),
                                prompt: "extra".into(),
                            };
                            native.followup_turns = vec![followup; 4];
                        }
                        "native-relative-root" => {
                            startup.lifecycle.native.as_mut().unwrap().root =
                                PathBuf::from("relative")
                        }
                        "native-unexpected-auth" => {
                            startup
                                .lifecycle
                                .native
                                .as_mut()
                                .unwrap()
                                .chatgpt_access_source =
                                Some(lifecycle_service::ChatgptAccessSource {
                                    codex_home: PathBuf::from("/nonexistent"),
                                    account_id: "test".into(),
                                    credentials_store:
                                        lifecycle_service::ChatgptCredentialsStore::File,
                                    https_proxy: None,
                                    isolation: None,
                                });
                        }
                        _ if !case.starts_with("authorization-")
                            && *case != "missing-authorization"
                            && *case != "unsupported-control" =>
                        {
                            malformed_record(&mut startup, case)
                        }
                        _ => {}
                    }
                }
                if case.starts_with("isolation-")
                    || case.starts_with("max-")
                    || case.starts_with("native-")
                    || case.starts_with("snapshot-")
                {
                    crate::research_installation::tests::rebind_configuration(&mut startup);
                }
                let auth = finalize(&f, &startup);
                if case.starts_with("authorization-") {
                    let mut input: Value =
                        serde_json::from_slice(&fs::read(&auth).unwrap()).unwrap();
                    match *case {
                        "authorization-version" => input["version"] = json!(1),
                        "authorization-selection" => {
                            input["selection_digest"] = json!("0".repeat(64))
                        }
                        "authorization-root" => {
                            input["protected_root"] = json!(f.config.parent().unwrap())
                        }
                        _ => unreachable!(),
                    }
                    write_private_json(&auth, &input);
                }
                if *case == "unsupported-control" {
                    let mut config: Value =
                        serde_json::from_slice(&fs::read(&f.config).unwrap()).unwrap();
                    config["control"] = json!({"mode":"unsupported-future-control"});
                    write_private_json(&f.config, &config);
                    let mut selection: Value =
                        serde_json::from_slice(&fs::read(&f.selection).unwrap()).unwrap();
                    selection["config_sha256"] = json!(sha_file(&f.config).unwrap());
                    write_private_json(&f.selection, &selection);
                    let mut input: Value =
                        serde_json::from_slice(&fs::read(&auth).unwrap()).unwrap();
                    input["startup_digest"] = json!(sha_file(&f.config).unwrap());
                    input["selection_digest"] = json!(sha_file(&f.selection).unwrap());
                    write_private_json(&auth, &input);
                }
                if *case == "missing-authorization" {
                    fs::remove_file(&auth).unwrap();
                }
                let before = startup_inventory(f.config.parent().unwrap());
                let result = prepare_with_operator(&f.selection, Some(&auth));
                assert!(result.is_err(), "accepted {case} populated={populated}");
                assert_eq!(
                    before,
                    startup_inventory(f.config.parent().unwrap()),
                    "startup effects for {case} populated={populated}"
                );
                CONTROLLED_OPERATOR_INPUTS.lock().unwrap().remove(&auth);
            }
        }
    }

    #[test]
    fn selected_host_seals_two_stores_and_keeps_lifecycle_lock_until_handoff() {
        let fixture = Fixture::new();
        let first = prepare(&fixture.selection).unwrap();
        let task_id = first.task_store.store_id().unwrap();
        let lifecycle_id = first.lifecycle_store.store_id().to_owned();
        assert!(
            fixture
                .selection
                .parent()
                .unwrap()
                .join("startup-intent.json")
                .exists()
        );
        let receipt_path = fixture
            .selection
            .parent()
            .unwrap()
            .join("startup-receipt.json");
        let receipt = fs::read(&receipt_path).unwrap();
        assert!(!fixture.host_socket.exists());
        assert!(!fixture.service_socket.exists());
        assert!(matches!(
            TaskStore::open(&fixture.task, "operator:selected".into()),
            Err(retained_task_workflow::TaskError::Busy)
        ));
        assert!(prepare(&fixture.selection).is_err());
        drop(first);

        let second = prepare(&fixture.selection).unwrap();
        assert_eq!(second.task_store.store_id().unwrap(), task_id);
        assert_eq!(second.lifecycle_store.store_id(), lifecycle_id);
        drop(second);
        assert_eq!(fs::read(&receipt_path).unwrap(), receipt);

        fs::rename(&fixture.task, fixture.task.with_extension("old")).unwrap();
        let replacement = TaskStore::open(&fixture.task, "operator:selected".into()).unwrap();
        drop(replacement);
        assert!(prepare(&fixture.selection).is_err());
        assert!(!fixture.host_socket.exists());
        assert!(!fixture.service_socket.exists());
    }

    #[test]
    fn selected_restart_refuses_missing_owner_operations_without_repair() {
        let fixture = Fixture::new();
        let first = prepare(&fixture.selection).unwrap();
        let task_id = first.task_store.store_id().unwrap();
        drop(first);
        let second = prepare(&fixture.selection).unwrap();
        assert_eq!(second.task_store.store_id().unwrap(), task_id);
        drop(second);
        let receipt_path = fixture
            .selection
            .parent()
            .unwrap()
            .join("startup-receipt.json");
        let receipt = fs::read(&receipt_path).unwrap();

        // Python's standard-library SQLite client supplies the fault fixture;
        // the restart itself still exercises the selected Rust host path.
        let sql = "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('DROP TABLE operations'); c.commit()";
        assert!(
            Command::new("python3")
                .arg("-c")
                .arg(sql)
                .arg(&fixture.task)
                .status()
                .unwrap()
                .success()
        );
        assert!(prepare(&fixture.selection).is_err());
        assert_eq!(fs::read(&receipt_path).unwrap(), receipt);
        assert!(!fixture.host_socket.exists());
        assert!(!fixture.service_socket.exists());
        let check = "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); assert c.execute(\"SELECT count(*) FROM sqlite_master WHERE type='table' AND name='operations'\").fetchone()[0] == 0; assert c.execute('SELECT store_id FROM store_identity').fetchone()[0] == sys.argv[2]";
        assert!(
            Command::new("python3")
                .arg("-c")
                .arg(check)
                .arg(&fixture.task)
                .arg(task_id)
                .status()
                .unwrap()
                .success()
        );
    }

    #[test]
    fn selected_host_refuses_replaced_lifecycle_database() {
        let fixture = Fixture::new();
        drop(prepare(&fixture.selection).unwrap());
        let database = fixture.lifecycle_root.join("lifecycle.sqlite");
        fs::rename(
            &database,
            fixture.lifecycle_root.join("old-lifecycle.sqlite"),
        )
        .unwrap();
        assert!(prepare(&fixture.selection).is_err());
        assert!(!fixture.host_socket.exists());
        assert!(!fixture.service_socket.exists());
    }

    #[test]
    fn selected_host_refuses_config_rebinding_before_store_creation() {
        let fixture = Fixture::new();
        let mut config: Value =
            serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
        config["lifecycle"]["socket_path"] = json!(fixture.host_socket);
        write_private_json(&fixture.config, &config);
        assert!(prepare(&fixture.selection).is_err());
        assert!(!fixture.task.exists());
        assert!(
            !fixture
                .selection
                .parent()
                .unwrap()
                .join("startup-intent.json")
                .exists()
        );
    }

    #[test]
    fn selected_source_and_binary_digest_drift_refuse_before_store_creation() {
        let source = Fixture::new();
        let mut config: Value = serde_json::from_slice(&fs::read(&source.config).unwrap()).unwrap();
        config["source_sha256"] = json!("0".repeat(64));
        write_private_json(&source.config, &config);
        let mut selection: Value =
            serde_json::from_slice(&fs::read(&source.selection).unwrap()).unwrap();
        selection["config_sha256"] = json!(sha_file(&source.config).unwrap());
        write_private_json(&source.selection, &selection);
        assert!(prepare(&source.selection).is_err());
        assert!(!source.task.exists() && !source.lifecycle_root.exists());

        let binary = Fixture::new();
        let mut selection: Value =
            serde_json::from_slice(&fs::read(&binary.selection).unwrap()).unwrap();
        selection["executable_sha256"] = json!("0".repeat(64));
        write_private_json(&binary.selection, &selection);
        assert!(prepare(&binary.selection).is_err());
        assert!(!binary.task.exists() && !binary.lifecycle_root.exists());
    }

    #[test]
    fn compatible_reselection_keeps_both_current_stores_and_rejects_old_selection() {
        let fixture = Fixture::new();
        let first = prepare(&fixture.selection).unwrap();
        let task_id = first.task_store.store_id().unwrap();
        let lifecycle_id = first.lifecycle_store.store_id().to_owned();
        drop(first);
        let old_receipt = fs::read(
            fixture
                .selection
                .parent()
                .unwrap()
                .join("startup-receipt.json"),
        )
        .unwrap();

        let next_config = fixture.selection.parent().unwrap().join("host-next.json");
        fs::copy(&fixture.config, &next_config).unwrap();
        let mut next: Value =
            serde_json::from_slice(&fs::read(&fixture.selection).unwrap()).unwrap();
        next["version"] = json!(2);
        next["selection_id"] = json!("selected-compatible-next");
        next["predecessor_selection_path"] = json!(fixture.selection);
        next["config_path"] = json!(next_config);
        next["config_sha256"] = json!(sha_file(&next_config).unwrap());
        let next_selection = fixture
            .selection
            .parent()
            .unwrap()
            .join("selection-next.json");
        write_private_json(&next_selection, &next);

        let second = prepare(&next_selection).unwrap();
        assert_eq!(second.task_store.store_id().unwrap(), task_id);
        assert_eq!(second.lifecycle_store.store_id(), lifecycle_id);
        drop(second);
        assert_ne!(
            fs::read(
                fixture
                    .selection
                    .parent()
                    .unwrap()
                    .join("startup-receipt.json")
            )
            .unwrap(),
            old_receipt
        );
        assert!(prepare(&fixture.selection).is_err());
        assert!(prepare(&next_selection).is_ok());

        let mut stale = next;
        stale["selection_id"] = json!("selected-stale-predecessor");
        let stale_selection = fixture
            .selection
            .parent()
            .unwrap()
            .join("selection-stale.json");
        write_private_json(&stale_selection, &stale);
        let current_receipt = fs::read(
            fixture
                .selection
                .parent()
                .unwrap()
                .join("startup-receipt.json"),
        )
        .unwrap();
        assert!(prepare(&stale_selection).is_err());
        assert_eq!(
            fs::read(
                fixture
                    .selection
                    .parent()
                    .unwrap()
                    .join("startup-receipt.json")
            )
            .unwrap(),
            current_receipt
        );
    }

    #[test]
    fn reselection_refuses_changed_lifecycle_config_before_publishing() {
        let fixture = Fixture::new();
        drop(prepare(&fixture.selection).unwrap());
        let receipt_path = fixture
            .selection
            .parent()
            .unwrap()
            .join("startup-receipt.json");
        let old_receipt = fs::read(&receipt_path).unwrap();

        let next_config = fixture.selection.parent().unwrap().join("host-next.json");
        let mut config: Value =
            serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
        config["lifecycle"]["build_id"] = json!("build:changed");
        write_private_json(&next_config, &config);
        let mut next: Value =
            serde_json::from_slice(&fs::read(&fixture.selection).unwrap()).unwrap();
        next["version"] = json!(2);
        next["selection_id"] = json!("selected-incompatible-next");
        next["predecessor_selection_path"] = json!(fixture.selection);
        next["config_path"] = json!(next_config);
        next["config_sha256"] = json!(sha_file(&next_config).unwrap());
        next["build_id"] = json!("build:changed");
        let next_selection = fixture
            .selection
            .parent()
            .unwrap()
            .join("selection-next.json");
        write_private_json(&next_selection, &next);
        assert!(prepare(&next_selection).is_err());
        assert_eq!(fs::read(receipt_path).unwrap(), old_receipt);
    }

    #[test]
    fn interrupted_receipt_stage_refuses_before_handoff() {
        let fixture = Fixture::new();
        drop(prepare(&fixture.selection).unwrap());
        let receipt_path = fixture
            .selection
            .parent()
            .unwrap()
            .join("startup-receipt.json");
        let old_receipt = fs::read(&receipt_path).unwrap();
        let stage = fixture
            .selection
            .parent()
            .unwrap()
            .join("startup-receipt.tmp");
        fs::write(&stage, b"interrupted candidate publication\n").unwrap();
        assert!(prepare(&fixture.selection).is_err());
        assert_eq!(fs::read(&receipt_path).unwrap(), old_receipt);
    }

    #[test]
    fn v3_declaration_binds_exact_predecessor_and_rejects_live_process_or_config_drift() {
        let fixture = Fixture::new();
        drop(prepare(&fixture.selection).unwrap());
        let directory = fixture.selection.parent().unwrap();
        let prior_receipt_path = directory.join("startup-receipt.json");
        let prior_receipt: Value =
            serde_json::from_slice(&fs::read(&prior_receipt_path).unwrap()).unwrap();

        let next_config = directory.join("host-restart.json");
        let mut config: Value =
            serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
        config["lifecycle"]["native"]["session_id"] = json!("session:restart");
        config["lifecycle"]["native"]["root"] = json!(directory.join("native-restart"));
        config["lifecycle"]["native"]["invocation_id"] = json!("invocation:restart");
        config["lifecycle"]["native"]["attempt_id"] = json!("attempt:restart");
        write_private_json(&next_config, &config);
        let declaration_path = directory.join("restart-declaration.json");
        let next_selection = directory.join("selection-restart.json");
        let mut selection: Value =
            serde_json::from_slice(&fs::read(&fixture.selection).unwrap()).unwrap();
        selection["version"] = json!(3);
        selection["selection_id"] = json!("selected-restart");
        selection["predecessor_selection_path"] = json!(fixture.selection);
        selection["restart_declaration_path"] = json!(declaration_path);
        selection["config_path"] = json!(next_config);
        selection["config_sha256"] = json!(sha_file(&next_config).unwrap());
        write_private_json(&next_selection, &selection);
        let mut declaration = json!({
            "version":1,
            "selection_sha256":sha_file(&next_selection).unwrap(),
            "predecessor_selection_sha256":sha_file(&fixture.selection).unwrap(),
            "predecessor_receipt_sha256":sha_file(&prior_receipt_path).unwrap(),
            "task":prior_receipt["task"],"lifecycle":prior_receipt["lifecycle"],
            "subject_id":"subject:selected","context_generation":"context:selected",
            "task_id":"task:selected","task_revision":1,"task_cursor_revision":0,
            "predecessor_session_id":"session:selected","predecessor_epoch":0,
            "target_session_id":"session:restart",
            "expected_subject_revision":1,"expected_semantic_revision":0,
            "expected_queue_revision":1,"expected_journal_cursor":1,
            "profile_digest":"a".repeat(64),"binary_sha256":"b".repeat(64),
            "protocol_digest":"c".repeat(64),"snapshot_id":"snapshot:selected",
            "snapshot_manifest_sha256":"d".repeat(64),
            "operator_ref":"operator:test","cooperative_stop_asserted":true,
            "predecessor_host_pid":u32::MAX,"predecessor_host_starttime_ticks":"1"
        });
        write_private_json(&declaration_path, &declaration);
        let selected: Selection =
            serde_json::from_slice(&fs::read(&next_selection).unwrap()).unwrap();
        let startup: HostStartup =
            serde_json::from_slice(&fs::read(&next_config).unwrap()).unwrap();
        let accepted = read_restart_declaration(
            &next_selection,
            directory,
            &selected,
            &startup,
            &sha_file(&next_selection).unwrap(),
            &prior_receipt_path,
        );
        assert!(accepted.is_ok(), "{accepted:?}");

        let work = directory.join("native-root/work");
        fs::create_dir_all(&work).unwrap();
        let mut child = Command::new("python3")
            .arg("-c")
            .arg("import ctypes,time; ctypes.CDLL(None).prctl(15,b'unavailable-nat',0,0,0); time.sleep(5)")
            .current_dir(&work)
            .spawn()
            .unwrap();
        for _ in 0..100 {
            if fs::read_to_string(format!("/proc/{}/comm", child.id()))
                .is_ok_and(|name| name.trim() == "unavailable-nat")
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            read_restart_declaration(
                &next_selection,
                directory,
                &selected,
                &startup,
                &sha_file(&next_selection).unwrap(),
                &prior_receipt_path
            )
            .is_err()
        );
        child.kill().unwrap();
        child.wait().unwrap();

        declaration["predecessor_host_pid"] = json!(std::process::id());
        declaration["predecessor_host_starttime_ticks"] =
            json!(process_starttime(std::process::id()).unwrap().unwrap());
        write_private_json(&declaration_path, &declaration);
        assert!(
            read_restart_declaration(
                &next_selection,
                directory,
                &selected,
                &startup,
                &sha_file(&next_selection).unwrap(),
                &prior_receipt_path
            )
            .is_err()
        );

        declaration["predecessor_host_pid"] = json!(u32::MAX);
        declaration["predecessor_host_starttime_ticks"] = json!("1");
        write_private_json(&declaration_path, &declaration);
        let mut drifted = startup.clone();
        drifted.provider_identity = "changed-provider".into();
        assert!(
            read_restart_declaration(
                &next_selection,
                directory,
                &selected,
                &drifted,
                &sha_file(&next_selection).unwrap(),
                &prior_receipt_path
            )
            .is_err()
        );
    }
    fn direct_control_json(directory: &Path) -> Value {
        let journal = directory.join("direct-journal");
        let release = directory.join("direct-release");
        for dir in [&journal, &release] {
            fs::create_dir_all(dir).unwrap();
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
        }
        // The public route's fixed bounds; any other value is refused.
        json!({"kind":"direct","credential_dir":directory.join("direct-credentials"),
            "credential_host_id":"host_direct","journal_dir":journal,"release_dir":release,
            "model":"model-x",
            "bounds":{"request_bytes":65536,"stream_bytes":1048576,"frame_bytes":131072,
                "output_bytes":16384,"events":4096,"local_deadline_ms":30000}})
    }

    fn startup_from(fixture: &Fixture, edit: impl FnOnce(&mut Value)) -> HostStartup {
        let mut config: Value =
            serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
        edit(&mut config);
        serde_json::from_value(config).unwrap()
    }

    #[test]
    fn control_defaults_to_native_is_not_serialized_and_rejects_unknown_shapes() {
        let fixture = Fixture::new();
        let native = startup_from(&fixture, |_| {});
        assert_eq!(native.control, ControlSelection::Native {});
        assert!(
            serde_json::to_value(&native)
                .unwrap()
                .get("control")
                .is_none()
        );
        assert!(matches!(
            compose_control(&native).unwrap(),
            ComposedControl::Native
        ));
        let directory = fixture.config.parent().unwrap().to_owned();
        let direct = startup_from(&fixture, |c| c["control"] = direct_control_json(&directory));
        assert!(matches!(direct.control, ControlSelection::Direct(_)));
        let round: HostStartup =
            serde_json::from_value(serde_json::to_value(&direct).unwrap()).unwrap();
        assert_eq!(round.control, direct.control);
        for bad in [
            json!({"kind":"fallback"}),
            json!({"kind":"native","evidence":"public_api"}),
            json!("direct"),
        ] {
            let mut config: Value =
                serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
            config["control"] = bad;
            assert!(serde_json::from_value::<HostStartup>(config).is_err());
        }
        let mut with_evidence = direct_control_json(&directory);
        with_evidence["evidence_class"] = json!("public_api");
        let mut config: Value =
            serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
        config["control"] = with_evidence;
        assert!(
            serde_json::from_value::<HostStartup>(config).is_err(),
            "configuration cannot declare an evidence class"
        );
    }

    #[test]
    fn direct_control_validation_binds_profile_semantic_paths_and_bounds() {
        let fixture = Fixture::new();
        let directory = fixture.config.parent().unwrap().to_owned();
        let accepted = startup_from(&fixture, |c| c["control"] = direct_control_json(&directory));
        validate_control(&accepted).unwrap();
        type Edit = Box<dyn Fn(&mut Value)>;
        let cases: Vec<(&str, Edit)> = vec![
            (
                "no semantic capture",
                Box::new(|c| c["lifecycle"]["native"]["semantic"] = Value::Null),
            ),
            (
                "other profile",
                Box::new(|c| c["lifecycle"]["profile"] = json!("read_only")),
            ),
            (
                "relative credentials",
                Box::new(|c| c["control"]["credential_dir"] = json!("creds")),
            ),
            (
                "parent traversal",
                Box::new(|c| c["control"]["journal_dir"] = json!("/tmp/../x")),
            ),
            (
                "empty host id",
                Box::new(|c| c["control"]["credential_host_id"] = json!("")),
            ),
            (
                "blank model",
                Box::new(|c| c["control"]["model"] = json!("  ")),
            ),
            (
                "zero bound",
                Box::new(|c| c["control"]["bounds"]["events"] = json!(0)),
            ),
            (
                "bounds differ from the public route",
                Box::new(|c| c["control"]["bounds"]["output_bytes"] = json!(1024)),
            ),
            (
                "missing release dir",
                Box::new({
                    let d = directory.clone();
                    move |c| c["control"]["release_dir"] = json!(d.join("absent-release"))
                }),
            ),
            (
                "release dir not owner-only",
                Box::new({
                    let d = directory.clone();
                    move |c| {
                        let shared = d.join("shared-release");
                        fs::create_dir_all(&shared).unwrap();
                        fs::set_permissions(&shared, fs::Permissions::from_mode(0o750)).unwrap();
                        c["control"]["release_dir"] = json!(shared);
                    }
                }),
            ),
            (
                "release dir is the journal",
                Box::new(|c| c["control"]["release_dir"] = c["control"]["journal_dir"].clone()),
            ),
            (
                "release dir inside credentials",
                Box::new({
                    let d = directory.clone();
                    move |c| {
                        c["control"]["release_dir"] = json!(d.join("direct-credentials/release"))
                    }
                }),
            ),
            (
                "journal inside credentials",
                Box::new({
                    let d = directory.clone();
                    move |c| {
                        c["control"]["journal_dir"] = json!(d.join("direct-credentials/journal"))
                    }
                }),
            ),
            (
                "same directory",
                Box::new({
                    let d = directory.clone();
                    move |c| c["control"]["journal_dir"] = json!(d.join("direct-credentials"))
                }),
            ),
            (
                "journal not private",
                Box::new({
                    let d = directory.clone();
                    move |c| {
                        let loose = d.join("loose");
                        fs::create_dir_all(&loose).unwrap();
                        fs::set_permissions(&loose, fs::Permissions::from_mode(0o755)).unwrap();
                        c["control"]["journal_dir"] = json!(loose);
                    }
                }),
            ),
        ];
        for (name, edit) in cases {
            let candidate = startup_from(&fixture, |c| {
                c["control"] = direct_control_json(&directory);
                edit(c);
            });
            assert!(validate_control(&candidate).is_err(), "{name}");
        }
        // A Direct selection without a release directory does not parse.
        let mut config: Value =
            serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
        config["control"] = direct_control_json(&directory);
        config["control"]
            .as_object_mut()
            .unwrap()
            .remove("release_dir");
        assert!(
            serde_json::from_value::<HostStartup>(config).is_err(),
            "no release dir"
        );
    }

    #[test]
    fn direct_composition_fails_closed_without_credentials_and_creates_nothing() {
        let fixture = Fixture::new();
        let directory = fixture.config.parent().unwrap().to_owned();
        let direct = startup_from(&fixture, |c| c["control"] = direct_control_json(&directory));
        // Composition is the only route to a Direct port, and it never falls
        // back to native when the credential store is absent.
        assert!(matches!(
            compose_control(&direct),
            Err(HostError::Configuration)
        ));
        assert!(!directory.join("direct-credentials").exists());
        // The same refusal ends selected startup before either store exists.
        let mut config: Value =
            serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
        config["control"] = direct_control_json(&directory);
        write_private_json(&fixture.config, &config);
        let mut selection: Value =
            serde_json::from_slice(&fs::read(&fixture.selection).unwrap()).unwrap();
        selection["config_sha256"] = json!(sha_file(&fixture.config).unwrap());
        write_private_json(&fixture.selection, &selection);
        assert!(prepare(&fixture.selection).is_err());
        assert!(!fixture.task.exists() && !fixture.lifecycle_root.exists());
        assert!(!directory.join("startup-intent.json").exists());
        assert!(!directory.join("direct-credentials").exists());
    }

    #[test]
    fn control_change_is_incompatible_for_reselection_and_restart() {
        let fixture = Fixture::new();
        let directory = fixture.config.parent().unwrap().to_owned();
        drop(prepare(&fixture.selection).unwrap());
        let recorded: Receipt = read_state(&directory.join("startup-receipt.json")).unwrap();
        let next_selection_for = |version: u64, config_path: &Path| -> (PathBuf, Selection) {
            let mut next: Value =
                serde_json::from_slice(&fs::read(&fixture.selection).unwrap()).unwrap();
            next["version"] = json!(version);
            next["selection_id"] = json!(format!("selected-control-v{version}"));
            next["predecessor_selection_path"] = json!(fixture.selection);
            next["config_path"] = json!(config_path);
            next["config_sha256"] = json!(sha_file(config_path).unwrap());
            if version == 3 {
                next["restart_declaration_path"] = json!(directory.join("declaration.json"));
            }
            let path = directory.join(format!("selection-v{version}.json"));
            write_private_json(&path, &next);
            (
                path.clone(),
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap(),
            )
        };

        // Reselection: identical config is compatible; only control differs below.
        let same_config = directory.join("host-same.json");
        fs::copy(&fixture.config, &same_config).unwrap();
        let (path2, selection2) = next_selection_for(2, &same_config);
        let same = startup_from(&fixture, |_| {});
        assert!(compatible_reselection(&path2, &selection2, &same, &recorded, &recorded).unwrap());
        let direct = startup_from(&fixture, |c| c["control"] = direct_control_json(&directory));
        assert!(
            !compatible_reselection(&path2, &selection2, &direct, &recorded, &recorded).unwrap()
        );

        // Restart: a fresh native identity is compatible only with the same control.
        let restarted = |control: Option<Value>| {
            startup_from(&fixture, |c| {
                c["lifecycle"]["native"]["session_id"] = json!("session:restarted");
                c["lifecycle"]["native"]["root"] = json!(directory.join("native-root-next"));
                if let Some(control) = control {
                    c["control"] = control;
                }
            })
        };
        let (path3, selection3) = next_selection_for(3, &same_config);
        assert!(compatible_restart(&path3, &selection3, &restarted(None), &recorded).unwrap());
        assert!(
            !compatible_restart(
                &path3,
                &selection3,
                &restarted(Some(direct_control_json(&directory))),
                &recorded
            )
            .unwrap()
        );

        // And the reverse: a predecessor that was Direct refuses a native successor.
        let direct_config = directory.join("host-direct.json");
        let mut config: Value =
            serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
        config["control"] = direct_control_json(&directory);
        write_private_json(&direct_config, &config);
        let (path3d, mut selection3d) = next_selection_for(3, &same_config);
        let mut prior: Value =
            serde_json::from_slice(&fs::read(&fixture.selection).unwrap()).unwrap();
        prior["config_path"] = json!(direct_config);
        prior["config_sha256"] = json!(sha_file(&direct_config).unwrap());
        let prior_path = directory.join("selection-direct-prior.json");
        write_private_json(&prior_path, &prior);
        selection3d.predecessor_selection_path = Some(prior_path.clone());
        let mut recorded_direct = recorded.clone();
        recorded_direct.selection_sha256 = sha_file(&prior_path).unwrap();
        assert!(
            !compatible_restart(&path3d, &selection3d, &restarted(None), &recorded_direct).unwrap()
        );
    }
}
