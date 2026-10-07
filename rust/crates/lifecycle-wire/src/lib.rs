//! Versioned public shapes and strict wire parsing. DTOs confer no authority.

mod json;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

pub use json::parse_strict_json;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandDto {
    pub protocol_version: u16,
    pub principal_ref: String,
    pub command_id: String,
    pub subject_id: String,
    pub context_generation: String,
    pub build_id: String,
    pub proof_run_id: String,
    pub grant_ref: String,
    pub expected_revision: String,
    pub request_digest: String,
    pub kind: String,
    pub payload: Value,
}

impl CommandDto {
    /// The trusted service checks this basis before resolving the referenced grant.
    /// Excludes only the digest claim itself.
    pub fn digest_basis(&self) -> Value {
        json!({
            "protocol_version": self.protocol_version,
            "principal_ref": self.principal_ref,
            "command_id": self.command_id,
            "subject_id": self.subject_id,
            "context_generation": self.context_generation,
            "build_id": self.build_id,
            "proof_run_id": self.proof_run_id,
            "grant_ref": self.grant_ref,
            "expected_revision": self.expected_revision,
            "kind": self.kind,
            "payload": self.payload,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireErrorCode {
    MalformedJson,
    UnsupportedVersion,
    SemanticInvalidity,
    StaleRevision,
    Conflict,
    ObservationUnavailable,
    CursorGap,
    UnsupportedCapability,
}

#[derive(Debug, Error)]
pub enum WireError {
    #[error("malformed JSON: {0}")]
    MalformedJson(String),
    #[error("protocol version is unsupported")]
    UnsupportedVersion,
    #[error("request is semantically invalid")]
    SemanticInvalidity,
    #[error("expected revision is stale")]
    StaleRevision,
    #[error("command conflicts with committed history")]
    Conflict,
    #[error("observation is unavailable")]
    ObservationUnavailable,
    #[error("cursor gap requires a new snapshot")]
    CursorGap,
    #[error("capability is unsupported")]
    UnsupportedCapability,
}

impl WireError {
    pub const fn code(&self) -> WireErrorCode {
        match self {
            Self::MalformedJson(_) => WireErrorCode::MalformedJson,
            Self::UnsupportedVersion => WireErrorCode::UnsupportedVersion,
            Self::SemanticInvalidity => WireErrorCode::SemanticInvalidity,
            Self::StaleRevision => WireErrorCode::StaleRevision,
            Self::Conflict => WireErrorCode::Conflict,
            Self::ObservationUnavailable => WireErrorCode::ObservationUnavailable,
            Self::CursorGap => WireErrorCode::CursorGap,
            Self::UnsupportedCapability => WireErrorCode::UnsupportedCapability,
        }
    }
}

pub fn parse_command(input: &[u8]) -> Result<CommandDto, WireError> {
    let value = parse_strict_json(input)?;
    let dto: CommandDto = serde_json::from_value(value)
        .map_err(|error| WireError::MalformedJson(error.to_string()))?;
    if dto.protocol_version != PROTOCOL_VERSION {
        return Err(WireError::UnsupportedVersion);
    }
    Ok(dto)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CursorV1 {
    pub store_id: String,
    pub stream_id: String,
    pub commit_sequence: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WaitResultV1<T> {
    Observed(T),
    DeadlineExpired { last_seen: Option<T> },
    ObservationUnavailable { code: WireErrorCode },
}
