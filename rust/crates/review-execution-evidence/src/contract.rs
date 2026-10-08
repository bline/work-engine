use claim_evidence::codec::{JsValue, canonical_json, digest, parse_json};
use claim_evidence::{CustodyEvidence, ProductionPathAdmissionBinding};
use sha2::{Digest, Sha256};

pub const REQUEST_MAX_BYTES: usize = 1024 * 1024;
pub const RESULT_MAX_BYTES: usize = 1024 * 1024;
pub const OBSERVATION_MAX_BYTES: usize = 1024 * 1024;
pub const STREAM_MAX_BYTES: usize = 4 * 1024 * 1024;
pub const ARTIFACT_MAX_BYTES: usize = 4 * 1024 * 1024;
pub const TERMINAL_MAX_BYTES: usize = 16 * 1024 * 1024;
pub const ROOT_PAYLOAD_MAX_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_DATABASE_BUSY_MILLIS: u64 = 5_000;

/// Exact locator only. Constructing a locator grants no read or execution authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionEvidenceRef {
    pub schema_version: u32,
    pub owner: String,
    pub root_id: String,
    pub profile: String,
    pub attempt_id: String,
    pub record_id: String,
    pub revision: String,
    pub sha256: String,
}

impl ExecutionEvidenceRef {
    pub fn validate(&self) -> Result<(), EvidenceReadError> {
        if self.schema_version != 1 {
            return Err(EvidenceReadError::Unsupported("evidence reference schema"));
        }
        for value in [
            &self.owner,
            &self.root_id,
            &self.profile,
            &self.attempt_id,
            &self.record_id,
            &self.revision,
        ] {
            require_text(value)?;
        }
        require_sha(&self.sha256)
    }

    pub fn custody_reference(&self) -> String {
        format!("execution-evidence:{}", self.sha256)
    }
}

/// The stable fields that existed when execution was prepared.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionBinding {
    pub campaign_root_id: String,
    pub obligation_id: String,
    pub candidate_digest: String,
    pub selection_digest: String,
    pub profile_digest: String,
    pub prepared_request_sha256: String,
    pub review_episode_id: String,
    pub attempt_id: String,
}

impl ExecutionBinding {
    pub fn validate(&self) -> Result<(), EvidenceReadError> {
        for value in [
            &self.campaign_root_id,
            &self.obligation_id,
            &self.review_episode_id,
            &self.attempt_id,
        ] {
            require_text(value)?;
        }
        for value in [
            &self.candidate_digest,
            &self.selection_digest,
            &self.profile_digest,
            &self.prepared_request_sha256,
        ] {
            require_sha(value)?;
        }
        Ok(())
    }

    #[cfg_attr(not(feature = "test-support"), allow(dead_code))]
    pub(crate) fn matches_admission(&self, binding: &ProductionPathAdmissionBinding) -> bool {
        self.campaign_root_id == binding.campaign_root_id
            && self.obligation_id == binding.obligation_id
            && self.candidate_digest == binding.candidate_digest
            && self.selection_digest == binding.selection_digest
            && self.profile_digest == binding.profile_digest
            && self.prepared_request_sha256 == binding.prepared_request_sha256
            && self.review_episode_id == binding.review_episode_id
            && self.attempt_id == binding.attempt_id
    }
}

/// No future completion operation, episode revision, or CE stage is represented here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionProvenance {
    pub binding: ExecutionBinding,
    pub dispatch_operation_id: String,
    pub dispatch_revision: String,
    pub execution_session_id: Option<String>,
}

impl ExecutionProvenance {
    pub fn validate(&self) -> Result<(), EvidenceReadError> {
        self.binding.validate()?;
        require_text(&self.dispatch_operation_id)?;
        require_text(&self.dispatch_revision)?;
        if let Some(session) = &self.execution_session_id {
            require_text(session)?;
        }
        Ok(())
    }
}

/// Closed classes; a controlled fixture cannot be upgraded by a feature flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    ControlledFixture,
    ControlledProcess,
    QualifiedProvider,
}

/// Installed reader profile; callers cannot make a controlled record establish
/// provider custody by changing a feature flag or descriptive reference field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceReaderProfile {
    ControlledFixture,
    ControlledProcess,
    QualifiedProvider,
}

impl EvidenceReaderProfile {
    pub fn require_class(self, class: EvidenceClass) -> Result<(), EvidenceReadError> {
        if matches!(
            (self, class),
            (Self::ControlledFixture, EvidenceClass::ControlledFixture)
                | (Self::ControlledProcess, EvidenceClass::ControlledProcess)
                | (Self::QualifiedProvider, EvidenceClass::QualifiedProvider)
        ) {
            Ok(())
        } else {
            Err(EvidenceReadError::Unsupported(
                "evidence class outside installed reader profile",
            ))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CheckedObservation {
    Present(JsValue),
    Absent { reason: String },
}

/// No public constructor or DTO conversion exists.
///
/// ```compile_fail
/// use review_execution_evidence::{CheckedExecutionResult, CheckedObservation, EvidenceClass,
///     ExecutionBinding, ExecutionEvidenceRef, ExecutionProvenance};
/// let reference = ExecutionEvidenceRef {
///     schema_version: 1, owner: "owner".into(), root_id: "root".into(),
///     profile: "profile".into(), attempt_id: "attempt".into(),
///     record_id: "record".into(), revision: "1".into(), sha256: "a".repeat(64),
/// };
/// let provenance = ExecutionProvenance {
///     binding: ExecutionBinding {
///         campaign_root_id: "campaign".into(), obligation_id: "obligation".into(),
///         candidate_digest: "a".repeat(64), selection_digest: "b".repeat(64),
///         profile_digest: "c".repeat(64), prepared_request_sha256: "d".repeat(64),
///         review_episode_id: "episode".into(), attempt_id: "attempt".into(),
///     },
///     dispatch_operation_id: "dispatch".into(), dispatch_revision: "revision".into(),
///     execution_session_id: Some("session".into()),
/// };
/// let _forged = CheckedExecutionResult::from_verified_parts(
///     reference, b"{}", provenance, CheckedObservation::Absent { reason: "none".into() },
///     EvidenceClass::ControlledProcess,
/// );
/// ```
///
/// ```compile_fail
/// let _: review_execution_evidence::CheckedExecutionResult = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct CheckedExecutionResult {
    reference: ExecutionEvidenceRef,
    value: JsValue,
    claim_canonical_bytes: Vec<u8>,
    claim_sha256: String,
    raw_sha256: String,
    provenance: ExecutionProvenance,
    observation: CheckedObservation,
    evidence_class: EvidenceClass,
}

impl CheckedExecutionResult {
    pub fn reference(&self) -> &ExecutionEvidenceRef {
        &self.reference
    }
    pub fn value(&self) -> &JsValue {
        &self.value
    }
    pub fn claim_canonical_bytes(&self) -> &[u8] {
        &self.claim_canonical_bytes
    }
    pub fn claim_sha256(&self) -> &str {
        &self.claim_sha256
    }
    pub fn raw_sha256(&self) -> &str {
        &self.raw_sha256
    }
    pub fn provenance(&self) -> &ExecutionProvenance {
        &self.provenance
    }
    pub fn observation(&self) -> &CheckedObservation {
        &self.observation
    }
    pub fn evidence_class(&self) -> EvidenceClass {
        self.evidence_class
    }

    /// Shared bounded result check. Only crate-owned producer/reader and the
    /// controlled fixture module can call this constructor.
    #[cfg_attr(not(feature = "test-support"), allow(dead_code))]
    pub(crate) fn from_verified_parts(
        reference: ExecutionEvidenceRef,
        raw: &[u8],
        provenance: ExecutionProvenance,
        observation: CheckedObservation,
        evidence_class: EvidenceClass,
    ) -> Result<Self, EvidenceReadError> {
        reference.validate()?;
        provenance.validate()?;
        if reference.attempt_id != provenance.binding.attempt_id {
            return Err(EvidenceReadError::Conflicting("reference attempt mismatch"));
        }
        if raw.len() > RESULT_MAX_BYTES {
            return Err(EvidenceReadError::Capacity("result exceeds limit"));
        }
        let text = std::str::from_utf8(raw)
            .map_err(|_| EvidenceReadError::Conflicting("result is not UTF-8 JSON"))?;
        let value = parse_json(text)
            .map_err(|_| EvidenceReadError::Conflicting("malformed native result"))?;
        let canonical = canonical_json(&value)
            .map_err(|_| EvidenceReadError::Conflicting("result cannot be canonicalized"))?;
        if canonical.len() > RESULT_MAX_BYTES {
            return Err(EvidenceReadError::Capacity(
                "canonical result exceeds limit",
            ));
        }
        let claim_sha256 =
            digest(&value).map_err(|_| EvidenceReadError::Conflicting("result digest failed"))?;
        match &observation {
            CheckedObservation::Present(value) => {
                if canonical_json(value)
                    .map_err(|_| EvidenceReadError::Conflicting("invalid observation"))?
                    .len()
                    > OBSERVATION_MAX_BYTES
                {
                    return Err(EvidenceReadError::Capacity("observation exceeds limit"));
                }
            }
            CheckedObservation::Absent { reason } => {
                require_text(reason)?;
            }
        }
        Ok(Self {
            reference,
            value,
            claim_canonical_bytes: canonical.into_bytes(),
            claim_sha256,
            raw_sha256: sha256(raw),
            provenance,
            observation,
            evidence_class,
        })
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum EvidenceReadError {
    #[error("exact evidence record absent")]
    Absent,
    #[error("conflicting execution evidence: {0}")]
    Conflicting(&'static str),
    #[error("execution custody inaccessible or unresolved: {0}")]
    Unresolved(&'static str),
    #[error("unsupported evidence contract: {0}")]
    Unsupported(&'static str),
    #[error("execution evidence capacity exceeded: {0}")]
    Capacity(&'static str),
}

pub trait ExecutionEvidenceOwner: Send + Sync {
    fn read_result(
        &self,
        reference: &ExecutionEvidenceRef,
    ) -> Result<CheckedExecutionResult, EvidenceReadError>;
    fn read_custody(
        &self,
        reference: &ExecutionEvidenceRef,
        binding: &ProductionPathAdmissionBinding,
        observation: Option<&JsValue>,
    ) -> Result<CustodyEvidence, EvidenceReadError>;
}

pub(crate) fn require_text(value: &str) -> Result<(), EvidenceReadError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().any(char::is_control)
        || value.len() > 512
    {
        return Err(EvidenceReadError::Conflicting("invalid identity text"));
    }
    Ok(())
}

pub(crate) fn require_sha(value: &str) -> Result<(), EvidenceReadError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(EvidenceReadError::Conflicting("invalid lowercase SHA-256"));
    }
    Ok(())
}

#[cfg_attr(not(feature = "test-support"), allow(dead_code))]
pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// New record identity codec, separate from CE and episode JSON codecs. Each
/// field is length prefixed, so separators inside an identity cannot collide.
#[cfg_attr(not(feature = "test-support"), allow(dead_code))]
pub(crate) fn record_digest(fields: &[&[u8]]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"review-execution-evidence-record-v1\0");
    for field in fields {
        hash.update((field.len() as u64).to_be_bytes());
        hash.update(field);
    }
    format!("{:x}", hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::record_digest;

    #[test]
    fn record_codec_v1_golden_vector() {
        assert_eq!(
            record_digest(&[b"a", b"b:c", b""]),
            "c19e1fcf39290e10a2a102998eed6dcaeebed395938b954caacf648b6040e37f"
        );
        assert_ne!(
            record_digest(&[b"a:b", b"c"]),
            record_digest(&[b"a", b"b:c"])
        );
    }
}
