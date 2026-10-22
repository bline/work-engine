//! Error type. Every JS rejection is a `TypeError` whose message names the failed rule; this
//! crate keeps that message verbatim and adds a closed [`ErrorKind`] so callers and tests
//! compare the failure class rather than prose.

use thiserror::Error;

/// Failure class. One variant per distinct JS rejection template.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    NotObject,
    UnsupportedFields,
    UnsupportedSchemaVersion,
    UnsupportedProfile,
    UnsupportedUnit,
    NotNonEmptyString,
    NotBasisPoints,
    NotPositiveInteger,
    NotShaRevision,
    NotIsoTimestamp,
    BandExitNotBelowEnter,
    ThresholdsNotOrdered,
    UnsupportedInitialDisposition,
    InvalidMinimumSequence,
    ObservationIdReused,
    SequenceNotMonotonic,
    SequenceBelowFloor,
    InvalidTokenCount,
    InvalidUsageSequence,
    InvalidUsageTimestamp,
    /// Rust-only: the JS coerces a missing or non-string `threadId` into the hashed evidence
    /// and the observation id; this crate rejects it instead.
    InvalidSnapshotField,
}

impl ErrorKind {
    /// Stable snake_case name, used by the golden fixtures.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotObject => "not_object",
            Self::UnsupportedFields => "unsupported_fields",
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnsupportedProfile => "unsupported_profile",
            Self::UnsupportedUnit => "unsupported_unit",
            Self::NotNonEmptyString => "not_non_empty_string",
            Self::NotBasisPoints => "not_basis_points",
            Self::NotPositiveInteger => "not_positive_integer",
            Self::NotShaRevision => "not_sha_revision",
            Self::NotIsoTimestamp => "not_iso_timestamp",
            Self::BandExitNotBelowEnter => "band_exit_not_below_enter",
            Self::ThresholdsNotOrdered => "thresholds_not_ordered",
            Self::UnsupportedInitialDisposition => "unsupported_initial_disposition",
            Self::InvalidMinimumSequence => "invalid_minimum_sequence",
            Self::ObservationIdReused => "observation_id_reused",
            Self::SequenceNotMonotonic => "sequence_not_monotonic",
            Self::SequenceBelowFloor => "sequence_below_floor",
            Self::InvalidTokenCount => "invalid_token_count",
            Self::InvalidUsageSequence => "invalid_usage_sequence",
            Self::InvalidUsageTimestamp => "invalid_usage_timestamp",
            Self::InvalidSnapshotField => "invalid_snapshot_field",
        }
    }
}

/// A rejected input. `Display` is the JS message text.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct PressureError {
    kind: ErrorKind,
    message: String,
}

impl PressureError {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
