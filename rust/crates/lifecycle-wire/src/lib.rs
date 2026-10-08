//! Versioned public shapes and strict wire parsing. DTOs confer no authority.

mod command_v2;
mod json;
mod observation;
mod schema;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use work_engine_types::{CodecContract, IdValue};

pub use command_v2::{
    CommandDtoV2, CommandOperationV2, CommandResultV2, EnqueueInputPayloadV2, parse_command_v2,
};
pub use json::parse_strict_json;
pub use observation::{
    AdmissionV1, DeliveryV1, EffectSettlementV1, ErrorDtoV1, ExecutionOutcomeV1, FieldV1,
    LifecycleSnapshotV1, ObservationProvenanceV1, RuntimeAvailabilityV1, TransitionStageV1,
    TransitionV1, WaitRequestV1, WaitTargetV1,
};
pub use schema::{schema_bundle_v1, schema_bundle_v2};

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
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
    #[serde(flatten)]
    pub operation: CommandOperationV1,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum CommandOperationV1 {
    EnqueueInput(EnqueueInputPayloadV1),
    RequestReplacement(RequestReplacementPayloadV1),
    RequestInterruption(RequestInterruptionPayloadV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnqueueInputPayloadV1 {
    pub input_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestReplacementPayloadV1 {
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestInterruptionPayloadV1 {
    pub attempt_id: String,
}

impl CommandDto {
    /// The trusted service checks this basis before resolving the referenced grant.
    /// Excludes only the digest claim itself.
    pub fn digest_basis(&self) -> Value {
        let mut basis = json!({
            "protocol_version": self.protocol_version,
            "principal_ref": self.principal_ref,
            "command_id": self.command_id,
            "subject_id": self.subject_id,
            "context_generation": self.context_generation,
            "build_id": self.build_id,
            "proof_run_id": self.proof_run_id,
            "grant_ref": self.grant_ref,
            "expected_revision": self.expected_revision,
        });
        let operation = serde_json::to_value(&self.operation).expect("typed operation serializes");
        basis
            .as_object_mut()
            .expect("object")
            .extend(operation.as_object().expect("operation object").clone());
        basis
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WireErrorCode {
    MalformedJson,
    UnsupportedVersion,
    SemanticInvalidity,
    StaleRevision,
    Conflict,
    ObservationUnavailable,
    CursorGap,
    UnsupportedCapability,
    UnsupportedObservationVariant,
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
    #[error("observation variant is unsupported")]
    UnsupportedObservationVariant,
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
            Self::UnsupportedObservationVariant => WireErrorCode::UnsupportedObservationVariant,
        }
    }
}

pub fn parse_command(input: &[u8]) -> Result<CommandDto, WireError> {
    let value = parse_strict_json(input)?;
    let object = value.as_object().ok_or(WireError::SemanticInvalidity)?;
    const FIELDS: &[&str] = &[
        "protocol_version",
        "principal_ref",
        "command_id",
        "subject_id",
        "context_generation",
        "build_id",
        "proof_run_id",
        "grant_ref",
        "expected_revision",
        "request_digest",
        "kind",
        "payload",
    ];
    if object.len() != FIELDS.len() || object.keys().any(|key| !FIELDS.contains(&key.as_str())) {
        return Err(WireError::SemanticInvalidity);
    }
    if object.get("protocol_version").and_then(Value::as_u64) != Some(u64::from(PROTOCOL_VERSION)) {
        return Err(WireError::UnsupportedVersion);
    }
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or(WireError::SemanticInvalidity)?;
    if !matches!(
        kind,
        "enqueue_input" | "request_replacement" | "request_interruption"
    ) {
        return Err(WireError::UnsupportedCapability);
    }
    let dto: CommandDto = serde_json::from_value(value)
        .map_err(|error| WireError::MalformedJson(error.to_string()))?;
    if dto.protocol_version != PROTOCOL_VERSION {
        return Err(WireError::UnsupportedVersion);
    }
    if dto.expected_revision.is_empty()
        || (dto.expected_revision.len() > 1 && dto.expected_revision.starts_with('0'))
        || !dto
            .expected_revision
            .bytes()
            .all(|byte| byte.is_ascii_digit())
        || dto.expected_revision.parse::<u64>().is_err()
        || dto.request_digest.len() != 64
        || !dto
            .request_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(WireError::SemanticInvalidity);
    }
    for value in [
        &dto.principal_ref,
        &dto.command_id,
        &dto.subject_id,
        &dto.context_generation,
        &dto.build_id,
        &dto.proof_run_id,
        &dto.grant_ref,
    ] {
        IdValue::parse(value.clone()).map_err(|_| WireError::SemanticInvalidity)?;
    }
    match &dto.operation {
        CommandOperationV1::EnqueueInput(payload) => {
            IdValue::parse(payload.input_id.clone()).map_err(|_| WireError::SemanticInvalidity)?;
        }
        CommandOperationV1::RequestReplacement(payload) => {
            if payload.reason.len() > 512 || payload.reason.chars().any(char::is_control) {
                return Err(WireError::SemanticInvalidity);
            }
        }
        CommandOperationV1::RequestInterruption(payload) => {
            IdValue::parse(payload.attempt_id.clone())
                .map_err(|_| WireError::SemanticInvalidity)?;
        }
    }
    let digest = CodecContract::LifecycleCommandV1
        .digest_json(&dto.digest_basis())
        .map_err(|_| WireError::SemanticInvalidity)?;
    if dto.request_digest != digest.hex() {
        return Err(WireError::SemanticInvalidity);
    }
    Ok(dto)
}

pub fn negotiate_version(offered: &[u16]) -> Result<u16, WireError> {
    if offered.contains(&PROTOCOL_VERSION) {
        Ok(PROTOCOL_VERSION)
    } else {
        Err(WireError::UnsupportedVersion)
    }
}

pub fn parse_snapshot(
    input: &[u8],
    expected_subject: &str,
) -> Result<LifecycleSnapshotV1, WireError> {
    let value = parse_strict_json(input)?;
    if value.get("protocol_version").and_then(Value::as_u64) != Some(u64::from(PROTOCOL_VERSION)) {
        return Err(WireError::UnsupportedVersion);
    }
    if has_unsupported_observation_variant(&value) {
        return Err(WireError::UnsupportedObservationVariant);
    }
    let snapshot: LifecycleSnapshotV1 =
        serde_json::from_value(value).map_err(|_| WireError::SemanticInvalidity)?;
    snapshot.validate_header(expected_subject)?;
    Ok(snapshot)
}

fn has_unsupported_observation_variant(value: &Value) -> bool {
    fn unsupported(actual: Option<&str>, known: &[&str]) -> bool {
        actual.is_some_and(|actual| !known.contains(&actual))
    }
    if unsupported(
        value
            .pointer("/current_admission/kind")
            .and_then(Value::as_str),
        &["domain", "transition", "fenced", "unknown"],
    ) {
        return true;
    }
    for key in [
        "transition",
        "delivery",
        "runtime_provenance",
        "blocking_reason",
        "measurement",
    ] {
        if unsupported(
            value
                .get(key)
                .and_then(|field| field.get("state"))
                .and_then(Value::as_str),
            &["known", "unknown", "not_applicable", "unavailable"],
        ) {
            return true;
        }
    }
    if value.pointer("/delivery/state").and_then(Value::as_str) == Some("known") {
        if unsupported(
            value
                .pointer("/delivery/value/provenance/state")
                .and_then(Value::as_str),
            &["known", "unknown", "not_applicable", "unavailable"],
        ) {
            return true;
        }
        if unsupported(
            value
                .pointer("/delivery/value/outcome")
                .and_then(Value::as_str),
            &["pending", "completed", "failed", "interrupted", "unknown"],
        ) {
            return true;
        }
        if unsupported(
            value
                .pointer("/delivery/value/settlement/kind")
                .and_then(Value::as_str),
            &["unresolved", "established", "conflict"],
        ) {
            return true;
        }
    }
    if value.pointer("/transition/state").and_then(Value::as_str) == Some("known")
        && unsupported(
            value
                .pointer("/transition/value/stage")
                .and_then(Value::as_str),
            &[
                "quiescing",
                "capturing",
                "verifying",
                "checkpointed",
                "switching",
                "rehydrating",
                "ready_to_commit",
                "reconciled",
                "recovery_required",
                "aborted_before_entry",
            ],
        )
    {
        return true;
    }
    unsupported(
        value.get("runtime_availability").and_then(Value::as_str),
        &["available", "unavailable", "unknown"],
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CursorV1 {
    pub store_id: String,
    pub stream_id: String,
    pub commit_sequence: String,
}

impl CursorV1 {
    pub fn validate(&self) -> Result<(), WireError> {
        if IdValue::parse(self.store_id.clone()).is_err()
            || IdValue::parse(self.stream_id.clone()).is_err()
            || self.commit_sequence.is_empty()
            || (self.commit_sequence.len() > 1 && self.commit_sequence.starts_with('0'))
            || !self
                .commit_sequence
                .bytes()
                .all(|byte| byte.is_ascii_digit())
            || self.commit_sequence.parse::<u64>().is_err()
        {
            return Err(WireError::SemanticInvalidity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum WaitResultV1<T> {
    Observed(T),
    DeadlineExpired { last_seen: Option<T> },
    ObservationUnavailable { code: WireErrorCode },
}
