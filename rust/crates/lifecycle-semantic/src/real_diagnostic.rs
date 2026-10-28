//! A separate, direct API diagnostic. These values cannot be decoded as the
//! fixture-scoped DirectV1 envelope and never qualify production admission.
use crate::invocation_v2::validate_direct_candidate;
use crate::{
    CandidateOrigin, DirectCandidate, SemanticError, SemanticPhase, SemanticPolicy, SourceBasis,
    VerdictStatus, candidate_verification_prompt, derive_report, json_digest, sha256_hex,
    valid_digest, valid_id,
};
use lifecycle_core::{SubjectId, TransitionId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealDiagnosticBinding {
    pub subject: SubjectId,
    pub transition: TransitionId,
    pub capture_id: String,
    pub policy_id: String,
    pub operation_id: String,
    pub attempt_id: String,
    pub exposure_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealDiagnosticRequest {
    pub binding: RealDiagnosticBinding,
    pub source_basis_sha256: String,
    pub candidate_sha256: String,
    pub candidate_origin: CandidateOrigin,
    pub semantic_prompt: String,
    pub semantic_prompt_sha256: String,
    pub max_output_bytes: usize,
}

pub fn real_diagnostic_request(
    material: &SourceBasis,
    candidate: &DirectCandidate,
    binding: RealDiagnosticBinding,
    policy: &SemanticPolicy,
) -> Result<RealDiagnosticRequest, SemanticError> {
    material.validate(policy)?;
    if !material.is_eligible() || !policy.within_frozen_limits() {
        return Err(SemanticError::IneligibleMaterial);
    }
    validate_direct_candidate(material, candidate, policy)?;
    if binding.subject != material.subject
        || binding.transition != material.transition
        || binding.capture_id != material.capture_id
        || binding.policy_id != material.policy_id
        || [
            &binding.operation_id,
            &binding.attempt_id,
            &binding.exposure_id,
        ]
        .iter()
        .any(|id| !valid_id(id))
    {
        return Err(SemanticError::WrongBinding);
    }
    let (semantic_prompt, semantic_prompt_sha256) =
        candidate_verification_prompt(material, &candidate.plan, &candidate.sha256, policy)?;
    Ok(RealDiagnosticRequest {
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
pub struct RealDiagnosticPrepared {
    pub request: RealDiagnosticRequest,
    pub instructions: String,
    pub wire_bytes: Vec<u8>,
    pub wire_sha256: String,
    pub profile_revision: String,
    pub requested_provider: String,
    pub requested_model: String,
    pub binding_sha256: String,
}

impl RealDiagnosticPrepared {
    pub fn bind(mut self) -> Result<Self, SemanticError> {
        self.binding_sha256 = self.digest()?;
        self.validate()?;
        Ok(self)
    }

    fn digest(&self) -> Result<String, SemanticError> {
        json_digest(&(
            &self.request,
            &self.instructions,
            &self.wire_sha256,
            &self.profile_revision,
            &self.requested_provider,
            &self.requested_model,
        ))
    }

    pub fn validate(&self) -> Result<(), SemanticError> {
        if self.request.semantic_prompt_sha256
            != sha256_hex(self.request.semantic_prompt.as_bytes())
            || self.wire_bytes.is_empty()
            || self.wire_sha256 != sha256_hex(&self.wire_bytes)
            || !valid_id(&self.profile_revision)
            || !valid_id(&self.requested_provider)
            || !valid_id(&self.requested_model)
            || self.binding_sha256 != self.digest()?
        {
            return Err(SemanticError::WrongBinding);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealDiagnosticResult {
    pub operation_id: String,
    pub attempt_id: String,
    pub prepared_wire_sha256: String,
    pub prepared_binding_sha256: String,
    pub capture_ref: String,
    pub captured_stream_sha256: String,
    pub response_id: Option<String>,
    pub observed_model: Option<String>,
    pub report_bytes: Vec<u8>,
    pub report_sha256: String,
}

/// Host-side evidence references are deliberately separate from report content.
/// Missing local observation makes diagnostic eligibility unknown even when a
/// valid report yields a raw host verdict.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealDiagnosticEvidence {
    pub operation_id: String,
    pub attempt_id: String,
    pub exposure_id: String,
    pub prepared_binding_sha256: String,
    pub source_basis_sha256: String,
    pub candidate_sha256: String,
    pub profile_revision: String,
    pub selected_model: String,
    pub request_admission_ref: Option<String>,
    pub submitted_bytes_ref: Option<String>,
    pub capture_complete_ref: Option<String>,
    pub documented_contract_ref: Option<String>,
    pub tool_free_request_ref: Option<String>,
    pub oracle_separation_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealDiagnosticAssessment {
    pub binding: RealDiagnosticBinding,
    pub raw_host_verdict: VerdictStatus,
    pub diagnostic_eligible: bool,
    pub production_qualified: bool,
    pub eligibility_reasons: Vec<String>,
    pub source_basis_sha256: String,
    pub candidate_sha256: String,
    pub prepared_wire_sha256: String,
    pub prepared_binding_sha256: String,
    pub report_sha256: String,
    pub findings: Vec<crate::AttributedFinding>,
    pub critical_errors: Vec<crate::CriticalFinding>,
    pub host_reasons: Vec<String>,
}

pub fn assess_real_diagnostic(
    material: &SourceBasis,
    candidate: &DirectCandidate,
    prepared: &RealDiagnosticPrepared,
    result: RealDiagnosticResult,
    evidence: RealDiagnosticEvidence,
    policy: &SemanticPolicy,
) -> Result<RealDiagnosticAssessment, SemanticError> {
    let expected = real_diagnostic_request(
        material,
        candidate,
        prepared.request.binding.clone(),
        policy,
    )?;
    if prepared.request != expected {
        return Err(SemanticError::WrongBinding);
    }
    prepared.validate()?;
    let binding = &prepared.request.binding;
    if result.operation_id != binding.operation_id
        || result.attempt_id != binding.attempt_id
        || result.prepared_wire_sha256 != prepared.wire_sha256
        || result.prepared_binding_sha256 != prepared.binding_sha256
        || !valid_id(&result.capture_ref)
        || !valid_digest(&result.captured_stream_sha256)
        || result.report_sha256 != sha256_hex(&result.report_bytes)
        || result
            .response_id
            .as_deref()
            .is_some_and(|id| !valid_id(id))
        || evidence.operation_id != binding.operation_id
        || evidence.attempt_id != binding.attempt_id
        || evidence.exposure_id != binding.exposure_id
        || evidence.prepared_binding_sha256 != prepared.binding_sha256
        || evidence.source_basis_sha256 != material.sha256
        || evidence.candidate_sha256 != candidate.sha256
        || evidence.profile_revision != prepared.profile_revision
        || evidence.selected_model != prepared.requested_model
    {
        return Err(SemanticError::WrongBinding);
    }
    let mut eligibility_reasons = Vec::new();
    if result.observed_model.as_deref() != Some(&prepared.requested_model) {
        eligibility_reasons.push("observed model missing or mismatched".into());
    }
    for (name, reference) in [
        ("request admission", &evidence.request_admission_ref),
        ("submitted bytes", &evidence.submitted_bytes_ref),
        ("complete capture", &evidence.capture_complete_ref),
        ("documented contract", &evidence.documented_contract_ref),
        ("tool-free request", &evidence.tool_free_request_ref),
        ("oracle separation", &evidence.oracle_separation_ref),
    ] {
        if !reference.as_deref().is_some_and(valid_id) {
            eligibility_reasons.push(format!("{name} evidence unavailable"));
        }
    }
    let checked = derive_report(
        material,
        SemanticPhase::CandidateVerification,
        &candidate.sha256,
        None,
        None,
        result.operation_id,
        result.report_sha256.clone(),
        &result.report_bytes,
        eligibility_reasons.is_empty(),
        policy,
    )?;
    // Report quality is the observed outcome, not a gap in local evidence.
    let diagnostic_eligible = eligibility_reasons.is_empty();
    Ok(RealDiagnosticAssessment {
        binding: binding.clone(),
        raw_host_verdict: checked.status,
        diagnostic_eligible,
        production_qualified: false,
        eligibility_reasons,
        source_basis_sha256: material.sha256.clone(),
        candidate_sha256: candidate.sha256.clone(),
        prepared_wire_sha256: prepared.wire_sha256.clone(),
        prepared_binding_sha256: prepared.binding_sha256.clone(),
        report_sha256: result.report_sha256,
        findings: checked.findings,
        critical_errors: checked.critical_errors,
        host_reasons: checked.reasons,
    })
}
