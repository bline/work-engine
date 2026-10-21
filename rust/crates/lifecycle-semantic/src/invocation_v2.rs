//! Additive, fixture-scoped direct verification. These payloads never decode as
//! legacy native values and have no store or lifecycle admission operation.

use crate::{
    AttributedFinding, Candidate, CheckedSemanticResult, ContinuationPlan, CriticalFinding,
    DependencyRevision, ObservedVerifierExposure, SemanticError, SemanticInvocationBinding,
    SemanticPhase, SemanticPolicy, SourceBasis, VerdictStatus, VerifierResult, VerifyRequest,
    assessment::derive_report, continuation::candidate_verification_prompt,
    continuation::validate_candidate_content, sha256_hex, valid_digest, valid_id,
};
use lifecycle_core::{SubjectId, TransitionId};
use serde::{
    Deserialize, Serialize,
    de::{DeserializeOwned, MapAccess, SeqAccess, Visitor},
};
use std::fmt;

/// The discriminator is mandatory; missing, mixed and unknown tags fail decode.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum NativeOrDirect<N, D> {
    NativeV1(N),
    DirectV1(D),
}

impl<'de, N, D> Deserialize<'de> for NativeOrDirect<N, D>
where
    N: DeserializeOwned + Serialize,
    D: DeserializeOwned + Serialize,
{
    fn deserialize<De: serde::Deserializer<'de>>(deserializer: De) -> Result<Self, De::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct TaggedPayload {
            kind: String,
            payload: NoDuplicateValue,
        }
        let tagged = TaggedPayload::deserialize(deserializer)?;
        let payload = tagged.payload.0;
        match tagged.kind.as_str() {
            "native_v1" => {
                let value: N =
                    serde_json::from_value(payload.clone()).map_err(serde::de::Error::custom)?;
                let roundtrip = serde_json::to_value(&value).map_err(serde::de::Error::custom)?;
                if roundtrip != payload {
                    return Err(serde::de::Error::custom(
                        "mixed or unknown native payload fields",
                    ));
                }
                Ok(Self::NativeV1(value))
            }
            "direct_v1" => {
                let value: D =
                    serde_json::from_value(payload.clone()).map_err(serde::de::Error::custom)?;
                let roundtrip = serde_json::to_value(&value).map_err(serde::de::Error::custom)?;
                if roundtrip != payload {
                    return Err(serde::de::Error::custom(
                        "mixed or unknown direct payload fields",
                    ));
                }
                Ok(Self::DirectV1(value))
            }
            _ => Err(serde::de::Error::custom("unknown invocation kind")),
        }
    }
}

/// Parses JSON without losing contradictory duplicate keys at any depth.
/// This is scoped to the new envelope decoder; legacy V1 DTO codecs remain as-is.
struct NoDuplicateValue(serde_json::Value);

impl<'de> Deserialize<'de> for NoDuplicateValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StrictVisitor;
        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = NoDuplicateValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("JSON with unique object keys")
            }

            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(value.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(value.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(value.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(serde_json::Value::Number)
                    .map(NoDuplicateValue)
                    .ok_or_else(|| E::custom("nonfinite JSON number"))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(value.into()))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(value.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(NoDuplicateValue(serde_json::Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                self.visit_unit()
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = sequence.next_element::<NoDuplicateValue>()? {
                    items.push(item.0);
                }
                Ok(NoDuplicateValue(serde_json::Value::Array(items)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut entries = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, NoDuplicateValue>()? {
                    if entries.insert(key.clone(), value.0).is_some() {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate JSON key: {key}"
                        )));
                    }
                }
                Ok(NoDuplicateValue(serde_json::Value::Object(entries)))
            }
        }
        deserializer.deserialize_any(StrictVisitor)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "N: DeserializeOwned + Serialize, D: DeserializeOwned + Serialize"))]
pub struct VersionedPayload<N, D> {
    #[serde(deserialize_with = "decode_version_2")]
    pub schema_version: u32,
    #[serde(flatten)]
    pub payload: NativeOrDirect<N, D>,
}

fn decode_version_2<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let version = u32::deserialize(deserializer)?;
    if version == 2 {
        Ok(version)
    } else {
        Err(serde::de::Error::custom("unsupported envelope version"))
    }
}

impl<N, D> VersionedPayload<N, D> {
    pub fn validate_version(&self) -> Result<(), SemanticError> {
        (self.schema_version == 2)
            .then_some(())
            .ok_or(SemanticError::WrongBinding)
    }
}

pub type InvocationEnvelope = VersionedPayload<SemanticInvocationBinding, DirectInvocationBinding>;
pub type CandidateEnvelope = VersionedPayload<Candidate, DirectCandidate>;
pub type VerifyRequestEnvelope = VersionedPayload<VerifyRequest, DirectVerifyRequest>;
pub type VerifierResultEnvelope = VersionedPayload<VerifierResult, DirectVerifierResult>;
pub type ExposureEnvelope = VersionedPayload<ObservedVerifierExposure, DirectExposure>;
pub type AssessedResultEnvelope = VersionedPayload<CheckedSemanticResult, AssessedDirectResult>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureScope {
    pub owner: String,
    pub reference: String,
    pub sha256: String,
}

impl FixtureScope {
    fn validate(&self) -> Result<(), SemanticError> {
        if valid_id(&self.owner) && valid_id(&self.reference) && valid_digest(&self.sha256) {
            Ok(())
        } else {
            Err(SemanticError::InvalidIdentifier)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectInvocationBinding {
    pub subject: SubjectId,
    pub transition: TransitionId,
    pub capture_id: String,
    pub phase: SemanticPhase,
    pub policy_id: String,
    pub operation_id: String,
    pub attempt_id: String,
    pub exposure_id: String,
    pub scope: FixtureScope,
}

impl DirectInvocationBinding {
    pub fn validate(&self, material: &SourceBasis) -> Result<(), SemanticError> {
        self.scope.validate()?;
        if self.subject != material.subject
            || self.transition != material.transition
            || self.capture_id != material.capture_id
            || self.phase != SemanticPhase::CandidateVerification
            || self.policy_id != material.policy_id
            || [&self.operation_id, &self.attempt_id, &self.exposure_id]
                .iter()
                .any(|id| !valid_id(id))
        {
            return Err(SemanticError::WrongBinding);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateOrigin {
    SyntheticFixture {
        owner: String,
        reference: String,
        sha256: String,
    },
    DirectCompilation {
        operation_id: String,
        attempt_id: String,
        prepared_request_sha256: String,
    },
}

impl CandidateOrigin {
    fn validate(&self) -> Result<(), SemanticError> {
        match self {
            Self::SyntheticFixture {
                owner,
                reference,
                sha256,
            } => FixtureScope {
                owner: owner.clone(),
                reference: reference.clone(),
                sha256: sha256.clone(),
            }
            .validate(),
            Self::DirectCompilation {
                operation_id,
                attempt_id,
                prepared_request_sha256,
            } => {
                if valid_id(operation_id)
                    && valid_id(attempt_id)
                    && valid_digest(prepared_request_sha256)
                {
                    Ok(())
                } else {
                    Err(SemanticError::InvalidIdentifier)
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectCandidate {
    pub source_basis_sha256: String,
    pub origin: CandidateOrigin,
    pub raw: Vec<u8>,
    pub sha256: String,
    pub plan: ContinuationPlan,
}

/// Authored replay candidates have an explicit fixture origin, never a fake
/// compiler invocation or session.
pub fn synthetic_candidate(
    material: &SourceBasis,
    raw: Vec<u8>,
    fixture: FixtureScope,
    policy: &SemanticPolicy,
) -> Result<DirectCandidate, SemanticError> {
    fixture.validate()?;
    let plan = validate_candidate_content(material, &raw, policy)?;
    Ok(DirectCandidate {
        source_basis_sha256: material.sha256.clone(),
        origin: CandidateOrigin::SyntheticFixture {
            owner: fixture.owner,
            reference: fixture.reference,
            sha256: fixture.sha256,
        },
        sha256: sha256_hex(&raw),
        raw,
        plan,
    })
}

fn validate_direct_candidate(
    material: &SourceBasis,
    candidate: &DirectCandidate,
    policy: &SemanticPolicy,
) -> Result<(), SemanticError> {
    candidate.origin.validate()?;
    // B1b has no direct compiler producer. Its coordinates may be represented,
    // but cannot be promoted from authored bytes into compiler provenance.
    if matches!(candidate.origin, CandidateOrigin::DirectCompilation { .. }) {
        return Err(SemanticError::WrongBinding);
    }
    if candidate.source_basis_sha256 != material.sha256
        || candidate.sha256 != sha256_hex(&candidate.raw)
        || candidate.plan != validate_candidate_content(material, &candidate.raw, policy)?
    {
        return Err(SemanticError::WrongBinding);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectVerifyRequest {
    pub binding: DirectInvocationBinding,
    pub source_basis_sha256: String,
    pub candidate_sha256: String,
    pub candidate_origin: CandidateOrigin,
    pub semantic_prompt: String,
    pub semantic_prompt_sha256: String,
    pub max_output_bytes: usize,
}

pub fn direct_verify_candidate(
    material: &SourceBasis,
    candidate: &DirectCandidate,
    binding: DirectInvocationBinding,
    policy: &SemanticPolicy,
) -> Result<DirectVerifyRequest, SemanticError> {
    material.validate(policy)?;
    if !material.is_eligible() || !policy.within_frozen_limits() {
        return Err(SemanticError::IneligibleMaterial);
    }
    binding.validate(material)?;
    validate_direct_candidate(material, candidate, policy)?;
    if let CandidateOrigin::SyntheticFixture {
        owner,
        reference,
        sha256,
    } = &candidate.origin
        && (owner != &binding.scope.owner
            || reference != &binding.scope.reference
            || sha256 != &binding.scope.sha256)
    {
        return Err(SemanticError::WrongBinding);
    }
    let (semantic_prompt, semantic_prompt_sha256) =
        candidate_verification_prompt(material, &candidate.plan, &candidate.sha256, policy)?;
    Ok(DirectVerifyRequest {
        binding,
        source_basis_sha256: material.sha256.clone(),
        candidate_sha256: candidate.sha256.clone(),
        candidate_origin: candidate.origin.clone(),
        semantic_prompt,
        semantic_prompt_sha256,
        max_output_bytes: policy.max_control_output_bytes,
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectPreparedRequest {
    pub request: DirectVerifyRequest,
    pub instructions: String,
    pub input: String,
    pub wire_bytes: Vec<u8>,
    pub wire_sha256: String,
    pub profile_revision: String,
    pub requested_provider: String,
    pub requested_model: String,
    /// Deterministic digest of the complete prepared envelope, excluding this field.
    pub binding_sha256: String,
}

impl DirectPreparedRequest {
    pub fn complete_binding_sha256(&self) -> Result<String, SemanticError> {
        crate::json_digest(&(
            &self.request,
            &self.instructions,
            &self.input,
            &self.wire_sha256,
            &self.profile_revision,
            &self.requested_provider,
            &self.requested_model,
        ))
    }

    /// Call only after the wire request has been assembled and its bytes fixed.
    pub fn bind(mut self) -> Result<Self, SemanticError> {
        self.binding_sha256 = self.complete_binding_sha256()?;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), SemanticError> {
        if self.input != self.request.semantic_prompt
            || self.request.semantic_prompt_sha256 != sha256_hex(self.input.as_bytes())
            || self.wire_sha256 != sha256_hex(&self.wire_bytes)
            || self.wire_bytes.is_empty()
            || !valid_id(&self.profile_revision)
            || !valid_id(&self.requested_provider)
            || !valid_id(&self.requested_model)
            || self.binding_sha256 != self.complete_binding_sha256()?
        {
            return Err(SemanticError::WrongBinding);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectVerifierResult {
    pub operation_id: String,
    pub attempt_id: String,
    pub prepared_wire_sha256: String,
    pub prepared_binding_sha256: String,
    pub capture_ref: String,
    pub captured_stream_sha256: String,
    pub semantic_prompt_sha256: String,
    pub response_id: Option<String>,
    pub observed_model: Option<String>,
    pub raw: Vec<u8>,
    pub sha256: String,
}

/// An observation is established only with a concrete, attributable reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Established<T> {
    Unknown,
    Known { value: T, evidence_ref: String },
}

impl<T: PartialEq> Established<T> {
    fn equals(&self, expected: &T) -> bool {
        matches!(self, Self::Known { value, evidence_ref } if value == expected && valid_id(evidence_ref))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectExposure {
    pub operation_id: String,
    pub attempt_id: String,
    pub exposure_id: String,
    pub prepared_wire_sha256: String,
    pub prepared_binding_sha256: String,
    pub candidate_origin: CandidateOrigin,
    pub source_basis_sha256: String,
    pub source_custody_ref: String,
    pub capability_profile_revision: String,
    pub scope: FixtureScope,
    pub compiler_reasoning_exposed: Established<bool>,
    pub hidden_answer_exposed: Established<bool>,
    pub dynamic_tools_enabled: Established<bool>,
    pub human_side_effects_enabled: Established<bool>,
    pub observed_provider: Established<String>,
    pub observed_model: Established<String>,
}

impl DirectExposure {
    fn established_for(&self, prepared: &DirectPreparedRequest) -> Result<bool, SemanticError> {
        let binding = &prepared.request.binding;
        if self.operation_id != binding.operation_id
            || self.attempt_id != binding.attempt_id
            || self.exposure_id != binding.exposure_id
            || self.prepared_wire_sha256 != prepared.wire_sha256
            || self.prepared_binding_sha256 != prepared.binding_sha256
            || self.candidate_origin != prepared.request.candidate_origin
            || self.source_basis_sha256 != prepared.request.source_basis_sha256
            || !valid_id(&self.source_custody_ref)
            || self.capability_profile_revision != prepared.profile_revision
            || self.scope != binding.scope
        {
            return Err(SemanticError::WrongBinding);
        }
        Ok(self.compiler_reasoning_exposed.equals(&false)
            && self.hidden_answer_exposed.equals(&false)
            && self.dynamic_tools_enabled.equals(&false)
            && self.human_side_effects_enabled.equals(&false)
            && self.observed_provider.equals(&prepared.requested_provider)
            && self.observed_model.equals(&prepared.requested_model))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssessedDirectResult {
    pub scope: FixtureScope,
    pub binding: DirectInvocationBinding,
    pub candidate_origin: CandidateOrigin,
    pub prepared_wire_sha256: String,
    pub prepared_binding_sha256: String,
    pub response_id: Option<String>,
    pub capture_ref: String,
    pub captured_stream_sha256: String,
    pub status: VerdictStatus,
    pub source_basis_sha256: String,
    pub candidate_sha256: String,
    pub verifier_result_sha256: String,
    pub findings: Vec<AttributedFinding>,
    pub critical_errors: Vec<CriticalFinding>,
    pub reasons: Vec<String>,
    pub assessed_dependencies: Vec<DependencyRevision>,
}

pub fn assess_direct_candidate(
    material: &SourceBasis,
    candidate: &DirectCandidate,
    prepared: &DirectPreparedRequest,
    result: DirectVerifierResult,
    exposure: DirectExposure,
    policy: &SemanticPolicy,
) -> Result<AssessedResultEnvelope, SemanticError> {
    let expected = direct_verify_candidate(
        material,
        candidate,
        prepared.request.binding.clone(),
        policy,
    )?;
    if prepared.request != expected {
        return Err(SemanticError::WrongBinding);
    }
    prepared.validate()?;
    if result.operation_id != prepared.request.binding.operation_id
        || result.attempt_id != prepared.request.binding.attempt_id
        || result.prepared_wire_sha256 != prepared.wire_sha256
        || result.prepared_binding_sha256 != prepared.binding_sha256
        || !valid_id(&result.capture_ref)
        || !valid_digest(&result.captured_stream_sha256)
        || result.semantic_prompt_sha256 != prepared.request.semantic_prompt_sha256
        || result.sha256 != sha256_hex(&result.raw)
        || result
            .response_id
            .as_deref()
            .is_some_and(|id| !valid_id(id))
    {
        return Err(SemanticError::WrongBinding);
    }
    let exposure_established = exposure.established_for(prepared)?
        && result
            .observed_model
            .as_ref()
            .is_some_and(|model| model == &prepared.requested_model);
    let checked = derive_report(
        material,
        SemanticPhase::CandidateVerification,
        &candidate.sha256,
        None,
        None,
        result.operation_id.clone(),
        result.sha256.clone(),
        &result.raw,
        exposure_established,
        policy,
    )?;
    Ok(AssessedResultEnvelope {
        schema_version: 2,
        payload: NativeOrDirect::DirectV1(AssessedDirectResult {
            scope: prepared.request.binding.scope.clone(),
            binding: prepared.request.binding.clone(),
            candidate_origin: candidate.origin.clone(),
            prepared_wire_sha256: prepared.wire_sha256.clone(),
            prepared_binding_sha256: prepared.binding_sha256.clone(),
            response_id: result.response_id,
            capture_ref: result.capture_ref,
            captured_stream_sha256: result.captured_stream_sha256,
            status: checked.status,
            source_basis_sha256: checked.source_basis_sha256,
            candidate_sha256: checked.candidate_sha256,
            verifier_result_sha256: checked.verifier_result_sha256,
            findings: checked.findings,
            critical_errors: checked.critical_errors,
            reasons: checked.reasons,
            assessed_dependencies: checked.assessed_dependencies,
        }),
    })
}
