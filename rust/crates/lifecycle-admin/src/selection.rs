//! A component launch selector for disposable offline transfer proofs.
//!
//! The permanent host must supply its own selector at the S9 deployment join.
//! The lock is deliberately never removed: unlinking it can let a paused holder
//! and a new process both believe they own the launch fence.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use lifecycle_store::SelectionEvidence;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionRecord {
    pub generation: u64,
    pub store_id: String,
    pub root: PathBuf,
    pub handoff_id: Option<String>,
    pub capsule_digest: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionCas {
    pub before: SelectionRecord,
    pub after: SelectionRecord,
}

#[derive(Debug, Error)]
pub enum SelectionError {
    #[error("launch selection is unavailable")]
    Unavailable,
    #[error("launch selection conflicts with the expected generation or identity")]
    Conflict,
    #[error("launch selection path or record is invalid")]
    Invalid,
}

/// The guard must remain alive from source fencing through destination commit.
pub trait LaunchSelectionGuard {
    fn read(&self) -> Result<SelectionRecord, SelectionError>;
    fn compare_and_swap(
        &mut self,
        expected: &SelectionRecord,
        successor: SelectionRecord,
    ) -> Result<SelectionCas, SelectionError>;

    fn evidence(&self, cas: &SelectionCas) -> Result<SelectionEvidence, SelectionError>;
}

pub trait LaunchSelectionPort {
    type Guard<'a>: LaunchSelectionGuard
    where
        Self: 'a;

    fn lock(&mut self) -> Result<Self::Guard<'_>, SelectionError>;
}

/// A local, process-shared selector used by disposable component tests.
pub struct FileLaunchSelectionPort {
    directory: PathBuf,
}

pub struct FileLaunchSelectionGuard {
    directory: PathBuf,
    _lock: File,
    lock_identity: String,
}

impl FileLaunchSelectionPort {
    pub fn open(directory: &Path) -> Result<Self, SelectionError> {
        let metadata = fs::symlink_metadata(directory).map_err(|_| SelectionError::Unavailable)?;
        if !metadata.file_type().is_dir() {
            return Err(SelectionError::Invalid);
        }
        let canonical = fs::canonicalize(directory).map_err(|_| SelectionError::Unavailable)?;
        if canonical != directory {
            return Err(SelectionError::Invalid);
        }
        Ok(Self {
            directory: canonical,
        })
    }

    /// Create the component fixture selection once; it cannot replace a prior selection.
    pub fn initialize(directory: &Path, initial: SelectionRecord) -> Result<Self, SelectionError> {
        if initial.generation != 0
            || initial.handoff_id.is_some()
            || initial.capsule_digest.is_some()
        {
            return Err(SelectionError::Invalid);
        }
        let port = Self::open(directory)?;
        validate_record(&initial)?;
        let path = port.directory.join("selection.json");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| SelectionError::Conflict)?;
        let bytes = serde_json::to_vec(&initial).map_err(|_| SelectionError::Invalid)?;
        file.write_all(&bytes)
            .map_err(|_| SelectionError::Unavailable)?;
        file.sync_all().map_err(|_| SelectionError::Unavailable)?;
        sync_dir(&port.directory)?;
        Ok(port)
    }
}

impl LaunchSelectionPort for FileLaunchSelectionPort {
    type Guard<'a> = FileLaunchSelectionGuard;

    fn lock(&mut self) -> Result<Self::Guard<'_>, SelectionError> {
        let path = self.directory.join("selection.lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|_| SelectionError::Unavailable)?;
        lock.lock().map_err(|_| SelectionError::Unavailable)?;
        let lock_path = self.directory.join("selection.lock");
        let link_metadata =
            fs::symlink_metadata(&lock_path).map_err(|_| SelectionError::Unavailable)?;
        if !link_metadata.file_type().is_file() || hard_linked(&link_metadata) {
            return Err(SelectionError::Invalid);
        }
        let metadata = lock.metadata().map_err(|_| SelectionError::Unavailable)?;
        if !same_file(&metadata, &link_metadata) {
            return Err(SelectionError::Invalid);
        }
        Ok(FileLaunchSelectionGuard {
            directory: self.directory.clone(),
            lock_identity: lock_identity(&self.directory, &metadata),
            _lock: lock,
        })
    }
}

impl LaunchSelectionGuard for FileLaunchSelectionGuard {
    fn read(&self) -> Result<SelectionRecord, SelectionError> {
        self.check_lock()?;
        let path = self.directory.join("selection.json");
        let metadata = fs::symlink_metadata(&path).map_err(|_| SelectionError::Unavailable)?;
        if !metadata.file_type().is_file() || hard_linked(&metadata) {
            return Err(SelectionError::Invalid);
        }
        let bytes = fs::read(path).map_err(|_| SelectionError::Unavailable)?;
        let record: SelectionRecord =
            serde_json::from_slice(&bytes).map_err(|_| SelectionError::Invalid)?;
        validate_record(&record)?;
        Ok(record)
    }

    fn compare_and_swap(
        &mut self,
        expected: &SelectionRecord,
        successor: SelectionRecord,
    ) -> Result<SelectionCas, SelectionError> {
        let observed = self.read()?;
        if observed == successor && successor.generation == expected.generation.saturating_add(1) {
            return Ok(SelectionCas {
                before: expected.clone(),
                after: successor,
            });
        }
        if observed != *expected {
            return Err(SelectionError::Conflict);
        }
        if successor.generation
            != expected
                .generation
                .checked_add(1)
                .ok_or(SelectionError::Invalid)?
            || successor.handoff_id.is_none()
            || successor.capsule_digest.is_none()
        {
            return Err(SelectionError::Invalid);
        }
        validate_record(&successor)?;
        let bytes = serde_json::to_vec(&successor).map_err(|_| SelectionError::Invalid)?;
        let (temporary, mut file) = (0..16)
            .find_map(|_| {
                let mut nonce = [0u8; 16];
                getrandom::fill(&mut nonce).ok()?;
                let suffix = nonce
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                let path = self.directory.join(format!("selection.{suffix}.next"));
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .ok()
                    .map(|file| (path, file))
            })
            .ok_or(SelectionError::Unavailable)?;
        if file
            .write_all(&bytes)
            .and_then(|_| file.sync_all())
            .is_err()
        {
            let _ = fs::remove_file(&temporary);
            return Err(SelectionError::Unavailable);
        }
        fs::rename(&temporary, self.directory.join("selection.json"))
            .map_err(|_| SelectionError::Unavailable)?;
        sync_dir(&self.directory)?;
        Ok(SelectionCas {
            before: observed,
            after: successor,
        })
    }

    fn evidence(&self, cas: &SelectionCas) -> Result<SelectionEvidence, SelectionError> {
        self.check_lock()?;
        let selected = self.read()?;
        if selected != cas.after
            || cas.before.generation.checked_add(1) != Some(cas.after.generation)
        {
            return Err(SelectionError::Conflict);
        }
        let handoff_id = cas
            .after
            .handoff_id
            .clone()
            .ok_or(SelectionError::Invalid)?;
        let capsule_sha256 = cas
            .after
            .capsule_digest
            .clone()
            .ok_or(SelectionError::Invalid)?;
        let bytes = serde_json::to_vec(&selected).map_err(|_| SelectionError::Invalid)?;
        let current_selection_digest = format!("{:x}", Sha256::digest(bytes));
        Ok(SelectionEvidence {
            handoff_id,
            source_store_id: cas.before.store_id.clone(),
            destination_store_id: cas.after.store_id.clone(),
            capsule_sha256,
            prior_selection_revision: cas.before.generation,
            current_selection_revision: cas.after.generation,
            current_selection_digest,
            launch_guard_id: self.lock_identity.clone(),
        })
    }
}

impl FileLaunchSelectionGuard {
    fn check_lock(&self) -> Result<(), SelectionError> {
        let link_metadata = fs::symlink_metadata(self.directory.join("selection.lock"))
            .map_err(|_| SelectionError::Unavailable)?;
        let held_metadata = self
            ._lock
            .metadata()
            .map_err(|_| SelectionError::Unavailable)?;
        if !link_metadata.file_type().is_file()
            || hard_linked(&link_metadata)
            || !same_file(&link_metadata, &held_metadata)
            || lock_identity(&self.directory, &held_metadata) != self.lock_identity
        {
            return Err(SelectionError::Invalid);
        }
        Ok(())
    }
}

fn validate_record(record: &SelectionRecord) -> Result<(), SelectionError> {
    if record.store_id.is_empty() || !record.root.is_absolute() {
        return Err(SelectionError::Invalid);
    }
    let canonical = fs::canonicalize(&record.root).map_err(|_| SelectionError::Unavailable)?;
    if canonical != record.root {
        return Err(SelectionError::Invalid);
    }
    Ok(())
}

fn sync_dir(path: &Path) -> Result<(), SelectionError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| SelectionError::Unavailable)
}

#[cfg(unix)]
fn hard_linked(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() != 1
}

#[cfg(not(unix))]
fn hard_linked(_: &fs::Metadata) -> bool {
    false
}

#[cfg(unix)]
fn lock_identity(directory: &Path, metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!(
        "{}:{}:{}",
        directory.display(),
        metadata.dev(),
        metadata.ino()
    )
}

#[cfg(not(unix))]
fn lock_identity(directory: &Path, _: &fs::Metadata) -> String {
    directory.display().to_string()
}

#[cfg(unix)]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file(_: &fs::Metadata, _: &fs::Metadata) -> bool {
    true
}
