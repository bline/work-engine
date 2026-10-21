use crate::{Bounds, ObservedTerminal, StreamObservation};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// `PossiblyEntered` is durable send intent, written before handing bytes to a transport.
/// On recovery its remote outcome is unknown even if no local response was seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptState {
    Prepared,
    PossiblyEntered,
    TerminalObserved,
    OutcomeUnknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub operation_id: String,
    pub attempt_id: String,
    pub request_sha256: String,
    pub profile_revision: String,
    pub selected_model: String,
    pub bounds: Bounds,
    pub state: AttemptState,
    pub observation: Option<StreamObservation>,
}

impl AttemptRecord {
    pub fn prepared(
        operation_id: impl Into<String>,
        attempt_id: impl Into<String>,
        request_sha256: impl Into<String>,
        profile_revision: impl Into<String>,
        selected_model: impl Into<String>,
        bounds: Bounds,
    ) -> Result<Self, JournalError> {
        let record = Self {
            operation_id: operation_id.into(),
            attempt_id: attempt_id.into(),
            request_sha256: request_sha256.into(),
            profile_revision: profile_revision.into(),
            selected_model: selected_model.into(),
            bounds,
            state: AttemptState::Prepared,
            observation: None,
        };
        record.validate()?;
        Ok(record)
    }

    fn validate(&self) -> Result<(), JournalError> {
        if self.operation_id.is_empty()
            || self.attempt_id.is_empty()
            || self.profile_revision.is_empty()
            || self.selected_model.is_empty()
            || self.request_sha256.len() != 64
            || !self.request_sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || self.bounds.validate().is_err()
            || matches!(
                self.state,
                AttemptState::TerminalObserved | AttemptState::OutcomeUnknown
            ) != self.observation.is_some()
            || self
                .observation
                .as_ref()
                .is_some_and(|seen| match self.state {
                    AttemptState::TerminalObserved => {
                        seen.local_failure.is_some()
                            || seen.response_id.as_deref().is_none_or(str::is_empty)
                            || match seen.terminal {
                                Some(ObservedTerminal::Completed) => seen.output.is_none(),
                                Some(
                                    ObservedTerminal::Failed { .. }
                                    | ObservedTerminal::Incomplete { .. },
                                ) => seen.output.is_some(),
                                None => true,
                            }
                    }
                    AttemptState::OutcomeUnknown => {
                        seen.local_failure.is_none() || seen.output.is_some()
                    }
                    _ => true,
                })
        {
            return Err(JournalError::InvalidRecord);
        }
        Ok(())
    }

    pub fn remote_outcome_unknown(&self) -> bool {
        matches!(
            self.state,
            AttemptState::PossiblyEntered | AttemptState::OutcomeUnknown
        )
    }
}

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("journal I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("journal writer lock unavailable: {0}")]
    Lock(#[from] std::fs::TryLockError),
    #[error("journal serialization: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid journal record")]
    InvalidRecord,
    #[error("invalid attempt transition")]
    InvalidTransition,
}

/// One file per attempt. A caller must create a distinct attempt for any retry.
/// Opening the file is observation-only and cannot send a request.
pub struct AttemptJournal {
    path: PathBuf,
    record: AttemptRecord,
}

impl AttemptJournal {
    pub fn create(path: impl AsRef<Path>, record: AttemptRecord) -> Result<Self, JournalError> {
        record.validate()?;
        if record.state != AttemptState::Prepared {
            return Err(JournalError::InvalidTransition);
        }
        let path = path.as_ref().to_path_buf();
        let bytes = serde_json::to_vec(&record)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        sync_parent(&path)?;
        Ok(Self { path, record })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, JournalError> {
        let path = path.as_ref().to_path_buf();
        let mut bytes = Vec::new();
        File::open(&path)?.read_to_end(&mut bytes)?;
        let record: AttemptRecord = serde_json::from_slice(&bytes)?;
        record.validate()?;
        Ok(Self { path, record })
    }

    pub fn record(&self) -> &AttemptRecord {
        &self.record
    }

    /// Commit intent before invoking a transport. A failed commit forbids sending.
    pub fn mark_send_intent(&mut self) -> Result<(), JournalError> {
        if self.record.state != AttemptState::Prepared {
            return Err(JournalError::InvalidTransition);
        }
        let mut next = self.record.clone();
        next.state = AttemptState::PossiblyEntered;
        self.persist(next)
    }

    /// Persist the complete attributed observation, including failed/incomplete terminals.
    pub fn mark_terminal(&mut self, observation: StreamObservation) -> Result<(), JournalError> {
        if self.record.state != AttemptState::PossiblyEntered {
            return Err(JournalError::InvalidTransition);
        }
        if observation.terminal.is_none() || observation.local_failure.is_some() {
            return Err(JournalError::InvalidRecord);
        }
        let mut next = self.record.clone();
        next.state = AttemptState::TerminalObserved;
        next.observation = Some(observation);
        self.persist(next)
    }

    pub fn mark_unknown(&mut self, observation: StreamObservation) -> Result<(), JournalError> {
        if self.record.state != AttemptState::PossiblyEntered
            || observation.local_failure.is_none()
            || observation.output.is_some()
        {
            return Err(JournalError::InvalidTransition);
        }
        let mut next = self.record.clone();
        next.state = AttemptState::OutcomeUnknown;
        next.observation = Some(observation);
        self.persist(next)
    }

    fn persist(&mut self, next: AttemptRecord) -> Result<(), JournalError> {
        next.validate()?;
        let _lock = WriterLock::acquire(self.path.with_extension("writer-lock"))?;
        if Self::open(&self.path)?.record != self.record {
            return Err(JournalError::InvalidTransition);
        }
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = self
            .path
            .with_extension(format!("tmp-{}-{sequence}", std::process::id()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(&serde_json::to_vec(&next)?)?;
            file.sync_all()?;
            fs::rename(&temp, &self.path)?;
            sync_parent(&self.path)?;
            Ok::<(), JournalError>(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result?;
        self.record = next;
        Ok(())
    }
}

/// Sidecar inode is stable across process death; only the OS lock grants ownership.
struct WriterLock {
    _file: File,
}

impl WriterLock {
    fn acquire(path: PathBuf) -> Result<Self, JournalError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        file.try_lock()?;
        sync_parent(&path)?;
        Ok(Self { _file: file })
    }
}

fn sync_parent(path: &Path) -> Result<(), JournalError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent)?.sync_all()?;
    Ok(())
}
