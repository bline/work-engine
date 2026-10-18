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

pub(crate) struct Prepared {
    pub startup: HostStartup,
    pub task_store: TaskStore,
    pub lifecycle_store: SelectedLifecycleStore,
    // A second selected process cannot race this store/receipt handoff.
    pub selection_lock: File,
}

pub(crate) fn prepare(path: &Path) -> Result<Prepared, HostError> {
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
    use serde_json::{Value, json};
    use std::process::Command;

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
}
