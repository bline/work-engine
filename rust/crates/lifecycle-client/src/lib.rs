//! Read-only observation port. No command submission or lifecycle progress API.

use lifecycle_wire::{CursorV1, WaitResultV1};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectRef(String);

impl SubjectRef {
    pub fn parse(value: impl Into<String>) -> Result<Self, ClientError> {
        let value = value.into();
        if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
            return Err(ClientError::InvalidSubject);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotHeaderV1 {
    pub protocol_version: u16,
    pub subject_id: String,
    pub cursor: CursorV1,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("subject reference is invalid")]
    InvalidSubject,
    #[error("read observation is unavailable")]
    ObservationUnavailable,
    #[error("protocol version is unsupported")]
    UnsupportedVersion,
    #[error("cursor is outside the retained history")]
    CursorGap,
}

/// S3 will implement a transport and validate the full snapshot body.
pub trait ObservationTransport {
    fn snapshot(&self, subject: &SubjectRef) -> Result<SnapshotHeaderV1, ClientError>;
    fn wait(
        &self,
        subject: &SubjectRef,
        cursor: &CursorV1,
        wait_budget_ms: u64,
    ) -> WaitResultV1<SnapshotHeaderV1>;
}

pub struct ReadClient<T: ObservationTransport> {
    transport: T,
}

impl<T: ObservationTransport> ReadClient<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn snapshot(&self, subject: &SubjectRef) -> Result<SnapshotHeaderV1, ClientError> {
        self.transport.snapshot(subject)
    }

    pub fn wait(
        &self,
        subject: &SubjectRef,
        cursor: &CursorV1,
        wait_budget_ms: u64,
    ) -> WaitResultV1<SnapshotHeaderV1> {
        self.transport.wait(subject, cursor, wait_budget_ms)
    }
}
