use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use lifecycle_core::{ClockSample, SubjectId};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use work_engine_types::CodecContract;

use crate::{SqliteLifecycleStore, StoreError};

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[must_use = "a staged artifact is not committed evidence until published"]
#[derive(Debug)]
pub struct StagedArtifact {
    path: PathBuf,
    digest_hex: String,
    byte_length: usize,
    subject: SubjectId,
}

impl StagedArtifact {
    pub fn digest_hex(&self) -> &str {
        &self.digest_hex
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedArtifact {
    pub digest_hex: String,
    pub byte_length: usize,
}

impl SqliteLifecycleStore {
    /// Write and sync owned staging bytes. No database reference exists yet.
    pub fn stage_artifact(
        &self,
        subject: SubjectId,
        bytes: &[u8],
    ) -> Result<StagedArtifact, StoreError> {
        self.verify_identity()?;
        let digest_hex = CodecContract::BinaryArtifactV1
            .digest_binary(bytes)
            .map_err(|_| StoreError::Unavailable)?
            .hex();
        let artifacts = self.root.join("artifacts");
        let staging = artifacts.join("staging");
        ensure_durable_child(&self.root, &artifacts)?;
        ensure_durable_child(&artifacts, &staging)?;
        for _ in 0..16 {
            let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = staging.join(format!(
                "{digest_hex}.{}.{}.stage",
                std::process::id(),
                sequence
            ));
            let file = OpenOptions::new().write(true).create_new(true).open(&path);
            let Ok(mut file) = file else { continue };
            file.write_all(bytes).map_err(|_| StoreError::Unavailable)?;
            file.sync_all().map_err(|_| StoreError::Unavailable)?;
            sync_directory(&staging)?;
            return Ok(StagedArtifact {
                path,
                digest_hex,
                byte_length: bytes.len(),
                subject,
            });
        }
        Err(StoreError::Unavailable)
    }

    /// Publish complete immutable bytes without overwrite, then commit their reference.
    pub fn publish_artifact(
        &mut self,
        staged: StagedArtifact,
        clock: ClockSample,
    ) -> Result<PublishedArtifact, StoreError> {
        self.verify_identity()?;
        let staging = self.root.join("artifacts/staging");
        if staged.path.parent() != Some(staging.as_path()) {
            return Err(StoreError::Rejected);
        }
        let bytes = fs::read(&staged.path).map_err(|_| StoreError::Unavailable)?;
        if bytes.len() != staged.byte_length || artifact_digest(&bytes)? != staged.digest_hex {
            return Err(StoreError::Rejected);
        }
        let artifacts = self.root.join("artifacts");
        let blobs = artifacts.join("blobs");
        ensure_durable_child(&self.root, &artifacts)?;
        ensure_durable_child(&artifacts, &blobs)?;
        let final_path = blobs.join(&staged.digest_hex);
        match fs::hard_link(&staged.path, &final_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata =
                    fs::symlink_metadata(&final_path).map_err(|_| StoreError::Unavailable)?;
                if !metadata.file_type().is_file() {
                    return Err(StoreError::Rejected);
                }
                let prior = fs::read(&final_path).map_err(|_| StoreError::Unavailable)?;
                if prior != bytes {
                    return Err(StoreError::ClaimConflict);
                }
            }
            Err(_) => return Err(StoreError::Unavailable),
        }
        sync_directory(&blobs)?;
        fs::remove_file(&staged.path).map_err(|_| StoreError::Unavailable)?;
        sync_directory(&staging)?;
        let relative_path = format!("artifacts/blobs/{}", staged.digest_hex);
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let prior: Option<(String, i64, String)> = tx
            .query_row(
                "SELECT subject_id,byte_length,relative_path FROM artifacts WHERE digest_hex=?1",
                [&staged.digest_hex],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        if let Some((subject, length, path)) = prior {
            if subject != staged.subject.as_str()
                || length != staged.byte_length as i64
                || path != relative_path
            {
                return Err(StoreError::ClaimConflict);
            }
        } else {
            tx.execute(
                "INSERT INTO artifacts(digest_hex,subject_id,byte_length,relative_path,committed_wall_ms) VALUES (?1,?2,?3,?4,?5)",
                params![staged.digest_hex,staged.subject.as_str(),i64::try_from(staged.byte_length).map_err(|_| StoreError::Rejected)?,relative_path,clock.wall.get()],
            ).map_err(|_| StoreError::Unavailable)?;
            tx.execute(
                "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'artifact_published',?2,?3)",
                params![staged.subject.as_str(),staged.digest_hex,clock.wall.get()],
            ).map_err(|_| StoreError::Unavailable)?;
        }
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(PublishedArtifact {
            digest_hex: staged.digest_hex,
            byte_length: staged.byte_length,
        })
    }

    pub fn read_committed_artifact(&self, digest_hex: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let row: Option<(String, i64)> = self
            .connection
            .query_row(
                "SELECT relative_path,byte_length FROM artifacts WHERE digest_hex=?1",
                [digest_hex],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        let Some((path, length)) = row else {
            return Ok(None);
        };
        if path != format!("artifacts/blobs/{digest_hex}") {
            return Err(StoreError::Unavailable);
        }
        let bytes = fs::read(self.root.join(path)).map_err(|_| StoreError::Unavailable)?;
        if bytes.len() != usize::try_from(length).map_err(|_| StoreError::Unavailable)?
            || artifact_digest(&bytes)? != digest_hex
        {
            return Err(StoreError::Unavailable);
        }
        Ok(Some(bytes))
    }
}

fn artifact_digest(bytes: &[u8]) -> Result<String, StoreError> {
    Ok(CodecContract::BinaryArtifactV1
        .digest_binary(bytes)
        .map_err(|_| StoreError::Unavailable)?
        .hex())
}

fn sync_directory(path: &std::path::Path) -> Result<(), StoreError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| StoreError::Unavailable)
}

fn ensure_durable_child(
    parent: &std::path::Path,
    child: &std::path::Path,
) -> Result<(), StoreError> {
    ensure_durable_child_with(parent, child, sync_directory)
}

fn ensure_durable_child_with(
    parent: &std::path::Path,
    child: &std::path::Path,
    sync: impl Fn(&std::path::Path) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    match fs::create_dir(child) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(child).map_err(|_| StoreError::Unavailable)?;
            if !metadata.file_type().is_dir() {
                return Err(StoreError::Unavailable);
            }
        }
        Err(_) => return Err(StoreError::Unavailable),
    }
    // An existing directory may be the remnant of an earlier failed sync.
    // Re-establish both durability edges before any database reference.
    sync(child)?;
    sync(parent)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn retry_after_child_sync_failure_reestablishes_parent_directory() {
        let root = tempfile::tempdir().unwrap();
        let child = root.path().join("artifacts");
        let calls = Cell::new(0);
        assert!(
            ensure_durable_child_with(root.path(), &child, |_| {
                calls.set(calls.get() + 1);
                Err(StoreError::Unavailable)
            })
            .is_err()
        );
        assert!(child.is_dir());
        assert_eq!(calls.get(), 1);

        let retried = RefCell::new(Vec::new());
        ensure_durable_child_with(root.path(), &child, |path| {
            retried.borrow_mut().push(path.to_path_buf());
            sync_directory(path)
        })
        .unwrap();
        assert_eq!(&*retried.borrow(), &[child, root.path().to_path_buf()]);
    }
}
