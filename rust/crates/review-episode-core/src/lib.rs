//! Pure review-episode state transitions. Host authorization, claims establishment,
//! durable CAS, and provider execution belong to their separate owners.

pub mod codec;
pub mod command;
pub mod identity;
pub mod implementation_review_v1;
pub mod reducer;
pub mod state;

use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    InvalidCommand,
    ResultContract,
    Authority,
    RevisionConflict,
    StateIntegrity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeError {
    pub kind: ErrorKind,
    pub message: String,
}

impl EpisodeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidCommand,
            message: message.into(),
        }
    }

    pub fn with_kind(mut self, kind: ErrorKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn authority(message: impl Into<String>) -> Self {
        Self::new(message).with_kind(ErrorKind::Authority)
    }

    pub fn revision_conflict(message: impl Into<String>) -> Self {
        Self::new(message).with_kind(ErrorKind::RevisionConflict)
    }

    pub fn result_contract(message: impl Into<String>) -> Self {
        Self::new(message).with_kind(ErrorKind::ResultContract)
    }
}

impl fmt::Display for EpisodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for EpisodeError {}

pub type EpisodeResult<T> = Result<T, EpisodeError>;
