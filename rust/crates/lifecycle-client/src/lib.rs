//! Passive, reusable lifecycle observation. The service owns projection and authority.

use std::time::{Duration, Instant};

use lifecycle_wire::{CursorV1, LifecycleSnapshotV1, WaitResultV1, WaitTargetV1, WireError};
use thiserror::Error;

mod socket;
pub use socket::UnixSocketTransport;

pub use lifecycle_wire::LifecycleSnapshotV1 as SnapshotV1;

const MAX_HINTS_PER_WAIT: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectRef(String);

impl SubjectRef {
    pub fn parse(value: impl Into<String>) -> Result<Self, ClientError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
        {
            return Err(ClientError::InvalidSubject);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ClientError {
    #[error("subject reference is invalid")]
    InvalidSubject,
    #[error("read observation is unavailable")]
    ObservationUnavailable,
    #[error("observation deadline expired")]
    DeadlineExpired,
    #[error("protocol version is unsupported")]
    UnsupportedVersion,
    #[error("observation variant is unsupported")]
    UnsupportedObservationVariant,
    #[error("observation belongs to another subject or store")]
    IdentityMismatch,
    #[error("observation is semantically invalid")]
    InvalidObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaitHintV1 {
    Changed,
    NoChange,
    /// The server explicitly reports that a cursor is outside retained history.
    CursorGap,
}

/// A passive read port. The S4 transport must honor every supplied I/O budget.
/// A targeted snapshot must make the named durable delivery or transition
/// observable even when a later record is current. Admission is always current.
/// Sequence-number jumps alone are not cursor gaps: journal numbers are global.
pub trait ObservationTransport {
    fn snapshot(
        &self,
        subject: &SubjectRef,
        target: Option<&WaitTargetV1>,
        budget: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError>;
    fn wait_hint(
        &self,
        subject: &SubjectRef,
        target: &WaitTargetV1,
        cursor: &CursorV1,
        budget: Duration,
    ) -> Result<WaitHintV1, ClientError>;
}

pub struct ReadClient<T: ObservationTransport> {
    transport: T,
}

impl<T: ObservationTransport> ReadClient<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn snapshot(
        &self,
        subject: &SubjectRef,
        budget: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        let started = Instant::now();
        let snapshot = self.read_snapshot(subject, None, budget)?;
        if started.elapsed() > budget {
            return Err(ClientError::DeadlineExpired);
        }
        Ok(snapshot)
    }

    fn read_snapshot(
        &self,
        subject: &SubjectRef,
        target: Option<&WaitTargetV1>,
        budget: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        if budget.is_zero() {
            return Err(ClientError::DeadlineExpired);
        }
        let snapshot = self.transport.snapshot(subject, target, budget)?;
        snapshot
            .validate_header(subject.as_str())
            .map_err(ClientError::from)?;
        Ok(snapshot)
    }

    /// Re-read after a lost connection. A different store identity is not silently adopted.
    pub fn reconnect(
        &self,
        subject: &SubjectRef,
        previous: &CursorV1,
        budget: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        previous.validate().map_err(ClientError::from)?;
        let snapshot = self.snapshot(subject, budget)?;
        if snapshot.cursor.store_id != previous.store_id
            || snapshot.cursor.stream_id != previous.stream_id
        {
            return Err(ClientError::IdentityMismatch);
        }
        if sequence(&snapshot.cursor) < sequence(previous) {
            return Err(ClientError::InvalidObservation);
        }
        Ok(snapshot)
    }

    /// Hints always cause a new read; they are never lifecycle events or progress commands.
    /// `NoChange` means the transport exhausted its supplied wait budget.
    pub fn wait(
        &self,
        subject: &SubjectRef,
        target: &WaitTargetV1,
        budget: Duration,
    ) -> WaitResultV1<LifecycleSnapshotV1> {
        let started = Instant::now();
        let mut last_seen = match self.read_snapshot(subject, Some(target), budget) {
            Ok(snapshot) => snapshot,
            Err(ClientError::DeadlineExpired) => {
                return WaitResultV1::DeadlineExpired { last_seen: None };
            }
            Err(_) if started.elapsed() > budget => {
                return WaitResultV1::DeadlineExpired { last_seen: None };
            }
            Err(error) => return unavailable(error),
        };
        if started.elapsed() > budget {
            return WaitResultV1::DeadlineExpired {
                last_seen: Some(last_seen),
            };
        }
        if target.is_observed(&last_seen) {
            return WaitResultV1::Observed(last_seen);
        }
        for _ in 0..MAX_HINTS_PER_WAIT {
            let remaining = budget.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return WaitResultV1::DeadlineExpired {
                    last_seen: Some(last_seen),
                };
            }
            match self
                .transport
                .wait_hint(subject, target, &last_seen.cursor, remaining)
            {
                Ok(WaitHintV1::NoChange) => {
                    return WaitResultV1::DeadlineExpired {
                        last_seen: Some(last_seen),
                    };
                }
                Ok(WaitHintV1::Changed | WaitHintV1::CursorGap) => match self.read_snapshot(
                    subject,
                    Some(target),
                    budget.saturating_sub(started.elapsed()),
                ) {
                    Ok(snapshot) => {
                        if snapshot.cursor.store_id != last_seen.cursor.store_id
                            || snapshot.cursor.stream_id != last_seen.cursor.stream_id
                        {
                            return unavailable(ClientError::IdentityMismatch);
                        }
                        if sequence(&snapshot.cursor) < sequence(&last_seen.cursor) {
                            return unavailable(ClientError::InvalidObservation);
                        }
                        last_seen = snapshot;
                        if started.elapsed() > budget {
                            return WaitResultV1::DeadlineExpired {
                                last_seen: Some(last_seen),
                            };
                        }
                        if target.is_observed(&last_seen) {
                            return WaitResultV1::Observed(last_seen);
                        }
                    }
                    Err(ClientError::DeadlineExpired) => {
                        return WaitResultV1::DeadlineExpired {
                            last_seen: Some(last_seen),
                        };
                    }
                    Err(_) if started.elapsed() > budget => {
                        return WaitResultV1::DeadlineExpired {
                            last_seen: Some(last_seen),
                        };
                    }
                    Err(error) => return unavailable(error),
                },
                Err(ClientError::DeadlineExpired) => {
                    return WaitResultV1::DeadlineExpired {
                        last_seen: Some(last_seen),
                    };
                }
                Err(_) if started.elapsed() > budget => {
                    return WaitResultV1::DeadlineExpired {
                        last_seen: Some(last_seen),
                    };
                }
                Err(error) => return unavailable(error),
            }
        }
        unavailable(ClientError::ObservationUnavailable)
    }
}

fn sequence(cursor: &CursorV1) -> u64 {
    cursor
        .commit_sequence
        .parse::<u64>()
        .expect("validated snapshot cursor")
}

impl From<WireError> for ClientError {
    fn from(error: WireError) -> Self {
        match error {
            WireError::UnsupportedVersion => Self::UnsupportedVersion,
            WireError::UnsupportedObservationVariant => Self::UnsupportedObservationVariant,
            _ => Self::InvalidObservation,
        }
    }
}

fn unavailable<T>(error: ClientError) -> WaitResultV1<T> {
    let code = match error {
        ClientError::UnsupportedVersion => lifecycle_wire::WireErrorCode::UnsupportedVersion,
        ClientError::UnsupportedObservationVariant => {
            lifecycle_wire::WireErrorCode::UnsupportedObservationVariant
        }
        _ => lifecycle_wire::WireErrorCode::ObservationUnavailable,
    };
    WaitResultV1::ObservationUnavailable { code }
}
