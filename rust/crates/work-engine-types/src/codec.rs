use serde_json::{Value, json};
use sha2::{Digest as ShaDigest, Sha256};
use thiserror::Error;

/// Trusted, closed registry for new identity-bearing encodings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodecContract {
    LifecycleCommandV1,
    LifecycleSnapshotV1,
    LifecycleTextInputV1,
    BinaryArtifactV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadKind {
    Json,
    Binary,
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CodecError {
    #[error("codec does not support this payload kind")]
    WrongPayloadKind,
    #[error("canonical JSON encoding failed: {0}")]
    CanonicalJson(String),
    #[error("supplied digest does not match the trusted contract")]
    DigestMismatch,
    #[error("declared codec does not match the trusted domain/kind/version")]
    ContractMismatch,
    #[error("JSON number is outside the interoperable numeric range")]
    UnsafeJsonNumber,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Digest {
    contract: CodecContract,
    bytes: [u8; 32],
}

impl CodecContract {
    pub fn verify_declaration(
        self,
        domain: &str,
        kind: &str,
        schema_version: u16,
        codec_version: &str,
    ) -> Result<(), CodecError> {
        if domain != self.domain()
            || kind != self.kind()
            || schema_version != self.schema_version()
            || codec_version != self.codec_version()
        {
            return Err(CodecError::ContractMismatch);
        }
        Ok(())
    }

    pub const fn payload_kind(self) -> PayloadKind {
        match self {
            Self::LifecycleCommandV1 | Self::LifecycleSnapshotV1 => PayloadKind::Json,
            Self::LifecycleTextInputV1 | Self::BinaryArtifactV1 => PayloadKind::Binary,
        }
    }

    pub const fn domain(self) -> &'static str {
        match self {
            Self::LifecycleCommandV1 | Self::LifecycleSnapshotV1 | Self::LifecycleTextInputV1 => {
                "work-engine.lifecycle"
            }
            Self::BinaryArtifactV1 => "work-engine.artifact",
        }
    }

    pub const fn kind(self) -> &'static str {
        match self {
            Self::LifecycleCommandV1 => "command",
            Self::LifecycleSnapshotV1 => "snapshot",
            Self::LifecycleTextInputV1 => "controlled-text-input",
            Self::BinaryArtifactV1 => "bytes",
        }
    }

    pub const fn schema_version(self) -> u16 {
        1
    }

    pub const fn codec_version(self) -> &'static str {
        match self.payload_kind() {
            PayloadKind::Json => "jcs-rfc8785-v1",
            PayloadKind::Binary => "exact-bytes-v1",
        }
    }

    pub fn canonical_json(self, payload: &Value) -> Result<Vec<u8>, CodecError> {
        if self.payload_kind() != PayloadKind::Json {
            return Err(CodecError::WrongPayloadKind);
        }
        validate_ijson_numbers(payload)?;
        let envelope = json!({
            "codec": self.codec_version(),
            "domain": self.domain(),
            "kind": self.kind(),
            "payload": payload,
            "schema": self.schema_version(),
        });
        serde_json_canonicalizer::to_vec(&envelope)
            .map_err(|error| CodecError::CanonicalJson(error.to_string()))
    }

    pub fn digest_json(self, payload: &Value) -> Result<Digest, CodecError> {
        let bytes = self.canonical_json(payload)?;
        Ok(Digest {
            contract: self,
            bytes: Sha256::digest(bytes).into(),
        })
    }

    pub fn digest_binary(self, payload: &[u8]) -> Result<Digest, CodecError> {
        if self.payload_kind() != PayloadKind::Binary {
            return Err(CodecError::WrongPayloadKind);
        }
        let bytes = Sha256::digest(payload).into();
        Ok(Digest {
            contract: self,
            bytes,
        })
    }
}

fn validate_ijson_numbers(value: &Value) -> Result<(), CodecError> {
    const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
    match value {
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                if integer.unsigned_abs() > MAX_SAFE_INTEGER {
                    return Err(CodecError::UnsafeJsonNumber);
                }
            } else if let Some(integer) = number.as_u64() {
                if integer > MAX_SAFE_INTEGER {
                    return Err(CodecError::UnsafeJsonNumber);
                }
            } else if let Some(float) = number.as_f64()
                && (!float.is_finite()
                    || (float.fract() == 0.0 && float.abs() > MAX_SAFE_INTEGER as f64))
            {
                return Err(CodecError::UnsafeJsonNumber);
            }
        }
        Value::Array(items) => {
            for item in items {
                validate_ijson_numbers(item)?;
            }
        }
        Value::Object(items) => {
            for item in items.values() {
                validate_ijson_numbers(item)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::String(_) => {}
    }
    Ok(())
}

impl Digest {
    pub fn contract(&self) -> CodecContract {
        self.contract
    }

    pub fn hex(&self) -> String {
        self.bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub fn verify_json(&self, trusted: CodecContract, payload: &Value) -> Result<(), CodecError> {
        if self.contract != trusted || *self != trusted.digest_json(payload)? {
            return Err(CodecError::DigestMismatch);
        }
        Ok(())
    }

    pub fn verify_binary(&self, trusted: CodecContract, payload: &[u8]) -> Result<(), CodecError> {
        if self.contract != trusted || *self != trusted.digest_binary(payload)? {
            return Err(CodecError::DigestMismatch);
        }
        Ok(())
    }
}
