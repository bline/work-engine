//! Selected startup for a fresh, disposable Rust development store.
//! This is a cooperative launch boundary, not a legacy-import witness or an
//! exclusion mechanism for independently controlled executables.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

use lifecycle_store::SqliteLifecycleStore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{ServiceConfig, ServiceError};

const MAX_SELECTION_BYTES: u64 = 65_536;
const MAX_CONFIG_BYTES: u64 = 131_072;
const MAX_STATE_BYTES: u64 = 16_384;

#[derive(Debug, Error)]
pub enum DevelopmentStartupError {
    #[error("selected development startup is invalid or conflicts with existing state")]
    Invalid,
    #[error("selected development startup I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("selected development startup service failed: {0}")]
    Service(#[from] ServiceError),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    version: u32,
    selection_id: String,
    executable_path: PathBuf,
    executable_sha256: String,
    config_path: PathBuf,
    config_sha256: String,
    store_root: PathBuf,
    socket_path: PathBuf,
    build_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct Intent {
    version: u32,
    selection_sha256: String,
    store_root: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct Receipt {
    version: u32,
    selection_sha256: String,
    store_root: PathBuf,
    store_id: String,
}

struct Prepared {
    config: ServiceConfig,
    store: SqliteLifecycleStore,
    // Keep the stable lock file open until the service has closed.
    _selection_lock: File,
}

/// Run the explicitly selected provider-disabled development service. The
/// existing direct-config CLI remains a separate proof entry, not this route.
pub async fn run_selected(path: &Path) -> Result<(), DevelopmentStartupError> {
    let Prepared {
        config,
        store,
        _selection_lock,
    } = prepare(path)?;
    let _guard = _selection_lock;
    crate::run_with_selected_store(config, store).await?;
    Ok(())
}

fn prepare(path: &Path) -> Result<Prepared, DevelopmentStartupError> {
    let directory = path.parent().ok_or(DevelopmentStartupError::Invalid)?;
    if !canonical_exact(directory)? || path.file_name().is_none() || !canonical_exact(path)? {
        return Err(DevelopmentStartupError::Invalid);
    }
    let selection_bytes = read_private(path, MAX_SELECTION_BYTES)?;
    let selection: Selection =
        serde_json::from_slice(&selection_bytes).map_err(|_| DevelopmentStartupError::Invalid)?;
    if selection.version != 1
        || selection.selection_id.is_empty()
        || selection.selection_id.len() > 128
        || !valid_sha(&selection.executable_sha256)
        || !valid_sha(&selection.config_sha256)
        || selection.config_path.parent() != Some(directory)
        || selection.store_root.parent() != Some(directory)
        || selection.socket_path.parent() != Some(directory)
        || !canonical_exact(&selection.config_path)?
        || !canonical_exact(&selection.executable_path)?
        || !fresh_or_canonical(&selection.store_root)?
        || selection.socket_path.file_name().is_none()
    {
        return Err(DevelopmentStartupError::Invalid);
    }
    let current_exe = std::env::current_exe()?;
    if current_exe != selection.executable_path
        || sha_file(&current_exe)? != selection.executable_sha256
    {
        return Err(DevelopmentStartupError::Invalid);
    }
    let config_bytes = read_private(&selection.config_path, MAX_CONFIG_BYTES)?;
    if sha_bytes(&config_bytes) != selection.config_sha256 {
        return Err(DevelopmentStartupError::Invalid);
    }
    let config: ServiceConfig =
        serde_json::from_slice(&config_bytes).map_err(|_| DevelopmentStartupError::Invalid)?;
    let directory_meta = fs::metadata(directory)?;
    if directory_meta.uid() != config.principal.uid
        || directory_meta.permissions().mode() & 0o777 != 0o700
        || config.profile != "read_only"
        || config.native.is_some()
        || !config.grants.is_empty()
        || config.verifier_executable.is_some()
        || config.verifier_ledger_path.is_some()
        || config.proof_barrier_dir.is_some()
        || config.store_root != selection.store_root
        || config.socket_path != selection.socket_path
        || config.build_id != selection.build_id
    {
        return Err(DevelopmentStartupError::Invalid);
    }

    let lock_path = directory.join("startup.lock");
    if exists_no_follow(&lock_path)? && !canonical_exact(&lock_path)? {
        return Err(DevelopmentStartupError::Invalid);
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    lock.set_permissions(fs::Permissions::from_mode(0o600))?;
    lock.try_lock()
        .map_err(|_| DevelopmentStartupError::Invalid)?;
    let selection_sha256 = sha_bytes(&selection_bytes);
    let intent_path = directory.join("startup-intent.json");
    let receipt_path = directory.join("startup-receipt.json");
    if exists_no_follow(&directory.join("startup-receipt.tmp"))? {
        return Err(DevelopmentStartupError::Invalid);
    }
    let expected_intent = Intent {
        version: 1,
        selection_sha256: selection_sha256.clone(),
        store_root: selection.store_root.clone(),
    };
    let receipt_exists = exists_no_follow(&receipt_path)?;
    if exists_no_follow(&intent_path)? {
        let recorded: Intent = read_state(&intent_path)?;
        if recorded != expected_intent {
            return Err(DevelopmentStartupError::Invalid);
        }
    } else if !receipt_exists && !exists_no_follow(&selection.store_root)? {
        create_synced(&intent_path, &expected_intent)?;
    } else {
        // A pre-existing root or receipt without our durable intent is foreign.
        return Err(DevelopmentStartupError::Invalid);
    }

    let store = SqliteLifecycleStore::open(&selection.store_root, config.trusted_issuer.clone())
        .map_err(|_| DevelopmentStartupError::Invalid)?;
    if store.root() != selection.store_root {
        return Err(DevelopmentStartupError::Invalid);
    }
    let owner = store
        .admin_ownership()
        .map_err(|_| DevelopmentStartupError::Invalid)?;
    let recovery = store
        .load_recovery()
        .map_err(|_| DevelopmentStartupError::Invalid)?;
    if owner.status != "active"
        || owner.handoff_id.is_some()
        || owner.capsule_sha256.is_some()
        || owner.peer_store_id.is_some()
        || owner.selection_revision.is_some()
    {
        return Err(DevelopmentStartupError::Invalid);
    }
    let expected_receipt = Receipt {
        version: 1,
        selection_sha256,
        store_root: selection.store_root,
        store_id: owner.store_id,
    };
    if receipt_exists {
        let recorded: Receipt = read_state(&receipt_path)?;
        if recorded != expected_receipt {
            return Err(DevelopmentStartupError::Invalid);
        }
    } else {
        // Before first listener exposure, an unsealed store may contain only
        // migrations and its generated store identity. Same-intent retry may
        // resume a crash between store creation and receipt publication.
        if recovery.subjects != 0
            || recovery.commands != 0
            || recovery.unresolved_attempts != 0
            || recovery.journal_cursor != 0
        {
            return Err(DevelopmentStartupError::Invalid);
        }
        publish_receipt(directory, &receipt_path, &expected_receipt)?;
    }
    Ok(Prepared {
        config,
        store,
        _selection_lock: lock,
    })
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

fn sha_file(path: &Path) -> Result<String, DevelopmentStartupError> {
    Ok(sha_bytes(&fs::read(path)?))
}

fn canonical_exact(path: &Path) -> Result<bool, DevelopmentStartupError> {
    Ok(path.is_absolute() && fs::canonicalize(path)? == path)
}

fn fresh_or_canonical(path: &Path) -> Result<bool, DevelopmentStartupError> {
    if !path.is_absolute() || path.file_name().is_none() {
        return Ok(false);
    }
    if exists_no_follow(path)? {
        return canonical_exact(path);
    }
    Ok(path.parent().is_some_and(|parent| parent.is_dir()))
}

fn exists_no_follow(path: &Path) -> Result<bool, DevelopmentStartupError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn read_private(path: &Path, limit: u64) -> Result<Vec<u8>, DevelopmentStartupError> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.file_type().is_file()
        || meta.len() > limit
        || meta.permissions().mode() & 0o777 != 0o600
    {
        return Err(DevelopmentStartupError::Invalid);
    }
    Ok(fs::read(path)?)
}

fn read_state<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, DevelopmentStartupError> {
    let bytes = read_private(path, MAX_STATE_BYTES)?;
    serde_json::from_slice(&bytes).map_err(|_| DevelopmentStartupError::Invalid)
}

fn sync_directory(directory: &Path) -> Result<(), DevelopmentStartupError> {
    File::open(directory)?.sync_all()?;
    Ok(())
}

fn create_synced<T: Serialize>(path: &Path, value: &T) -> Result<(), DevelopmentStartupError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    serde_json::to_writer(&mut file, value).map_err(|_| DevelopmentStartupError::Invalid)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    sync_directory(path.parent().ok_or(DevelopmentStartupError::Invalid)?)
}

fn publish_receipt(
    directory: &Path,
    receipt_path: &Path,
    value: &Receipt,
) -> Result<(), DevelopmentStartupError> {
    let stage = directory.join("startup-receipt.tmp");
    create_synced(&stage, value)?;
    fs::rename(stage, receipt_path)?;
    sync_directory(directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::time::Duration;

    fn write_private_json(path: &Path, value: &Value) {
        let bytes = serde_json::to_vec(value).unwrap();
        fs::write(path, [&bytes[..], b"\n"].concat()).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    #[tokio::test]
    async fn preopened_store_lock_and_identity_survive_selected_actor_handoff() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().canonicalize().unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let config_path = directory.join("config.json");
        let selection_path = directory.join("selection.json");
        let root = directory.join("store");
        let socket = directory.join("service.sock");
        let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
        let uid = fs::metadata(&directory).unwrap().uid();
        write_private_json(
            &config_path,
            &json!({
                "profile":"read_only", "store_root":root, "socket_path":socket,
                "peer_socket_path":directory.join("unused-peer.sock"),
                "verifier_executable":null, "verifier_ledger_path":null,
                "proof_barrier_dir":null, "trusted_issuer":"development-test-issuer",
                "principal":{"uid":uid,"principal_ref":"development-test-principal","producer_ref":"development-test-producer"},
                "subject_id":"development-test-subject", "context_generation":"development-test-generation",
                "build_id":"development-test-build", "proof_run_id":"development-test-run",
                "grants":[], "max_executions":1, "max_wait_ms":1000,
                "verifier_proof_ms":null, "verifier_cleanup_ms":null, "native":null
            }),
        );
        write_private_json(
            &selection_path,
            &json!({
                "version":1, "selection_id":"handoff-test",
                "executable_path":executable, "executable_sha256":sha_file(&executable).unwrap(),
                "config_path":config_path, "config_sha256":sha_file(&config_path).unwrap(),
                "store_root":root, "socket_path":socket, "build_id":"development-test-build"
            }),
        );
        let Prepared {
            config,
            store,
            _selection_lock,
        } = prepare(&selection_path).unwrap();
        let _guard = _selection_lock;

        // No second ordinary store owner can acquire the original writer lock
        // while the selected actor handoff is pending.
        assert!(SqliteLifecycleStore::open(&root, config.trusted_issuer.clone()).is_err());

        // A same-UID path replacement cannot make the actor initialize or
        // expose the new root: its carried store rejects the changed identity.
        fs::rename(&root, directory.join("old-store")).unwrap();
        let replacement = SqliteLifecycleStore::open(&root, config.trusted_issuer.clone()).unwrap();
        assert_eq!(replacement.load_recovery().unwrap().subjects, 0);
        drop(replacement);
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            crate::run_with_selected_store(config.clone(), store),
        )
        .await
        .expect("selected actor must refuse before listener exposure");
        assert!(result.is_err());
        assert!(!socket.exists());
        let replacement = SqliteLifecycleStore::open(&root, config.trusted_issuer).unwrap();
        assert_eq!(replacement.load_recovery().unwrap().subjects, 0);
    }
}
