use crate::{Bounds, ObservedTerminal, RegistrationRef, StreamObservation};
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
    /// Non-secret credential identity captured before this attempt is created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration: Option<RegistrationRef>,
    /// Protected HTTP evidence. Body text and unsafe header text are not retained.
    /// Public-host content_type is exact, bounded, printable ASCII after secret
    /// screening; local_http retains its existing allow-listed media-type essence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpEvidence {
    pub status: Option<u16>,
    pub request_id: Option<String>,
    pub content_type: Option<String>,
    /// None means this older/other producer did not collect this diagnostic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type_meta: Option<ContentTypeMeta>,
    pub error_shape: Option<HttpErrorShape>,
    pub error_code: Option<String>,
    pub error_param: Option<String>,
    pub body_bytes: u64,
    pub body_sha256: Option<String>,
    /// False when only a bounded prefix was received or a read was interrupted.
    #[serde(default)]
    pub body_complete: bool,
}

pub(crate) const CONTENT_TYPE_VALUE_LIMIT: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentTypeCapture {
    Missing,
    Captured,
    NonText,
    Oversized,
    Redacted,
}

/// Metadata about the selected first Content-Type value, without provider text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentTypeMeta {
    pub capture: ContentTypeCapture,
    /// Omitted for credential-like values; no credential correlation digest.
    pub bytes: Option<usize>,
    /// The unchanged case-sensitive check on the original header value.
    pub event_stream_prefix: bool,
}

impl ContentTypeMeta {
    fn consistent_with(&self, value: Option<&str>) -> bool {
        match self.capture {
            ContentTypeCapture::Captured => value.is_some_and(|v| {
                v.len() <= CONTENT_TYPE_VALUE_LIMIT
                    && v.bytes().all(|b| (0x20..=0x7e).contains(&b))
                    && self.bytes == Some(v.len())
                    && self.event_stream_prefix == v.starts_with("text/event-stream")
            }),
            ContentTypeCapture::Missing => {
                value.is_none() && self.bytes == Some(0) && !self.event_stream_prefix
            }
            ContentTypeCapture::Redacted => value.is_none() && self.bytes.is_none(),
            ContentTypeCapture::NonText => {
                value.is_none() && self.bytes.is_some_and(|n| n <= CONTENT_TYPE_VALUE_LIMIT)
            }
            ContentTypeCapture::Oversized => {
                value.is_none() && self.bytes.is_some_and(|n| n > CONTENT_TYPE_VALUE_LIMIT)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpErrorShape {
    Detail,
    Structured,
    DetailAndStructured,
    OtherJson,
    NonJson,
    TooLarge,
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
            registration: None,
            http: None,
        };
        record.validate()?;
        Ok(record)
    }

    /// Preparation binds an immutable auth revision; a later switch or refresh
    /// cannot change the bearer held by the transport for this attempt.
    pub fn prepared_bound(
        operation_id: impl Into<String>,
        attempt_id: impl Into<String>,
        request_sha256: impl Into<String>,
        profile_revision: impl Into<String>,
        selected_model: impl Into<String>,
        bounds: Bounds,
        registration: RegistrationRef,
    ) -> Result<Self, JournalError> {
        let mut record = Self::prepared(
            operation_id,
            attempt_id,
            request_sha256,
            profile_revision,
            selected_model,
            bounds,
        )?;
        if registration.client_id.is_empty()
            || registration.subject.is_empty()
            || registration.credential_revision == 0
        {
            return Err(JournalError::InvalidRecord);
        }
        record.registration = Some(registration);
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
            || self.registration.as_ref().is_some_and(|r| {
                r.client_id.is_empty() || r.subject.is_empty() || r.credential_revision == 0
            })
            || self.http.as_ref().is_some_and(|h| {
                h.body_sha256
                    .as_ref()
                    .is_some_and(|v| v.len() != 64 || !v.bytes().all(|b| b.is_ascii_hexdigit()))
                    || h.content_type_meta
                        .as_ref()
                        .is_some_and(|meta| !meta.consistent_with(h.content_type.as_deref()))
            })
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

    pub fn mark_http(&mut self, evidence: HttpEvidence) -> Result<(), JournalError> {
        if self.record.state != AttemptState::PossiblyEntered || self.record.http.is_some() {
            return Err(JournalError::InvalidTransition);
        }
        let mut next = self.record.clone();
        next.http = Some(evidence);
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
        #[cfg(feature = "journal-test-faults")]
        let fault_stage = if self.record.state == AttemptState::Prepared {
            "intent"
        } else if self.record.http.is_none() && next.http.is_some() {
            "http"
        } else if next.state == AttemptState::TerminalObserved {
            "terminal"
        } else {
            "unknown"
        };
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
            #[cfg(feature = "journal-test-faults")]
            fault_at(&self.path, fault_stage, "before_write")?;
            file.write_all(&serde_json::to_vec(&next)?)?;
            #[cfg(feature = "journal-test-faults")]
            fault_at(&self.path, fault_stage, "after_write")?;
            file.sync_all()?;
            #[cfg(feature = "journal-test-faults")]
            fault_at(&self.path, fault_stage, "after_file_sync")?;
            fs::rename(&temp, &self.path)?;
            #[cfg(feature = "journal-test-faults")]
            fault_at(&self.path, fault_stage, "after_rename")?;
            sync_parent(&self.path)?;
            #[cfg(feature = "journal-test-faults")]
            fault_at(&self.path, fault_stage, "after_parent_sync")?;
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

/// Available only in an opt-in test build. The child process binds an exact
/// journal path, stage and persistence syscall boundary before this can fire.
#[cfg(feature = "journal-test-faults")]
fn fault_at(path: &Path, stage: &str, point: &str) -> Result<(), JournalError> {
    let Ok(spec) = std::env::var("B2B_JOURNAL_FAULT_SPEC") else {
        return Ok(());
    };
    if std::env::var_os("B2B_JOURNAL_FAULT_PATH").as_deref() != Some(path.as_os_str()) {
        return Ok(());
    }
    let mut parts = spec.split(':');
    let (Some(mode), Some(target_stage), Some(target_point), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Ok(());
    };
    if target_stage != stage || target_point != point {
        return Ok(());
    }
    match mode {
        "io" => Err(JournalError::Io(std::io::Error::other(format!(
            "injected {stage}:{point} failure"
        )))),
        "exit" => std::process::exit(73),
        _ => Ok(()),
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
