//! Durable, private SQLite owner for review episodes. No authority decision is made here.
mod import;
mod integrity;
mod sqlite;

pub use import::{CopyManifest, ImportReport, import_closed_copy, reconcile_closed_copy};
pub use integrity::{HistoryEntry, Snapshot, validate_history};
pub use sqlite::{EpisodeStore, StoreOptions, WriteDisposition, init_offline_root};

use review_episode_core::EpisodeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("unsupported or unsafe offline storage path: {0}")]
    Path(String),
    #[error("incompatible review-episode SQLite schema: {0}")]
    Schema(String),
    #[error("review-episode history integrity failure: {0}")]
    Integrity(String),
    #[error("review-episode revision conflict")]
    RevisionConflict,
    #[error("review-episode database is busy")]
    Busy,
    #[error("response exceeds configured byte limit")]
    ResponseTooLarge,
    #[error("review-episode operation outcome is unknown after SQLite I/O failure")]
    OutcomeUnknown,
    #[error("review-episode I/O failure: {0}")]
    Io(String),
    #[error("{0}")]
    Domain(#[from] EpisodeError),
}

pub type StoreResult<T> = Result<T, StoreError>;

#[cfg(feature = "test-faults")]
pub fn test_checkpoint(name: &str) {
    use std::path::Path;
    use std::time::Duration;
    if std::env::var("REVIEW_EPISODE_FAULT_CUT").ok().as_deref() != Some(name) {
        return;
    }
    let Ok(directory) = std::env::var("REVIEW_EPISODE_FAULT_DIR") else {
        return;
    };
    let directory = Path::new(&directory);
    let ready = directory.join(format!("{name}.ready"));
    let release = directory.join(format!("{name}.release"));
    std::fs::write(&ready, b"ready\n").expect("fault checkpoint ready signal");
    while !release.exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(not(feature = "test-faults"))]
#[inline]
pub fn test_checkpoint(_: &str) {}
