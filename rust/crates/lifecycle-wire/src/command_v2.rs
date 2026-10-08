//! Separate v2 command contract. Published v1 command bytes remain historical.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use work_engine_types::{CodecContract, IdValue};

use crate::{
    RequestInterruptionPayloadV1, RequestReplacementPayloadV1, WireError, parse_strict_json,
};

pub const COMMAND_VERSION_V2: u16 = 2;
pub const MAX_TEXT_BYTES: usize = 65_536;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandDtoV2 {
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
    pub operation: CommandOperationV2,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum CommandOperationV2 {
    EnqueueInput(EnqueueInputPayloadV2),
    RequestReplacement(RequestReplacementPayloadV1),
    RequestInterruption(RequestInterruptionPayloadV1),
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnqueueInputPayloadV2 {
    pub input_id: String,
    pub producer_ref: String,
    pub text: String,
    pub text_digest: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandResultV2 {
    pub protocol_version: u16,
    pub command_id: String,
    pub accepted: bool,
    pub outcome_kind: String,
    pub outcome_ref: Option<String>,
    pub rejection_code: Option<String>,
    pub resulting_revision: String,
}

impl CommandDtoV2 {
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

pub fn parse_command_v2(input: &[u8]) -> Result<CommandDtoV2, WireError> {
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
    if object.get("protocol_version").and_then(Value::as_u64) != Some(2) {
        return Err(WireError::UnsupportedVersion);
    }
    let dto: CommandDtoV2 = serde_json::from_value(value)
        .map_err(|error| WireError::MalformedJson(error.to_string()))?;
    if dto.protocol_version != COMMAND_VERSION_V2 {
        return Err(WireError::UnsupportedVersion);
    }
    if dto.expected_revision.is_empty()
        || (dto.expected_revision.len() > 1 && dto.expected_revision.starts_with('0'))
        || !dto.expected_revision.bytes().all(|b| b.is_ascii_digit())
        || dto.expected_revision.parse::<u64>().is_err()
        || dto.request_digest.len() != 64
        || !dto.request_digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(WireError::SemanticInvalidity);
    }
    for id in [
        &dto.principal_ref,
        &dto.command_id,
        &dto.subject_id,
        &dto.context_generation,
        &dto.build_id,
        &dto.proof_run_id,
        &dto.grant_ref,
    ] {
        IdValue::parse(id.clone()).map_err(|_| WireError::SemanticInvalidity)?;
    }
    match &dto.operation {
        CommandOperationV2::EnqueueInput(payload) => {
            IdValue::parse(payload.input_id.clone()).map_err(|_| WireError::SemanticInvalidity)?;
            IdValue::parse(payload.producer_ref.clone())
                .map_err(|_| WireError::SemanticInvalidity)?;
            if payload.text.len() > MAX_TEXT_BYTES
                || payload.text_digest.len() != 64
                || !payload.text_digest.bytes().all(|b| b.is_ascii_hexdigit())
                || CodecContract::LifecycleTextInputV1
                    .digest_binary(payload.text.as_bytes())
                    .map_err(|_| WireError::SemanticInvalidity)?
                    .hex()
                    != payload.text_digest
            {
                return Err(WireError::SemanticInvalidity);
            }
        }
        CommandOperationV2::RequestReplacement(payload) => {
            if payload.reason.len() > 512 || payload.reason.chars().any(char::is_control) {
                return Err(WireError::SemanticInvalidity);
            }
        }
        CommandOperationV2::RequestInterruption(payload) => {
            IdValue::parse(payload.attempt_id.clone())
                .map_err(|_| WireError::SemanticInvalidity)?;
        }
    }
    if CodecContract::LifecycleCommandV2
        .digest_json(&dto.digest_basis())
        .map_err(|_| WireError::SemanticInvalidity)?
        .hex()
        != dto.request_digest
    {
        return Err(WireError::SemanticInvalidity);
    }
    Ok(dto)
}
