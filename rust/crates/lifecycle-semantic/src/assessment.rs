use crate::{
    Candidate, Challenge, DependencyChange, DependencyRevision, SemanticError, SemanticPhase,
    SemanticPolicy, SourceBasis, SourceReference, SuccessorResponse, VerifyRequest,
    compare_dependencies, sha256_hex, valid_id, verify_candidate, verify_successor,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Satisfied,
    Violated,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    InventedOrWidenedApproval,
    StaleGoverningAuthority,
    RevivedSupersededAction,
    OmittedMaterialCommitment,
    FabricatedSource,
    CompletedEffectReplay,
    OtherCriticalContradiction,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AttributedFinding {
    pub obligation_id: String,
    pub status: FindingStatus,
    pub sources: Vec<SourceReference>,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CriticalFinding {
    pub kind: ErrorKind,
    pub sources: Vec<SourceReference>,
    pub explanation: String,
}

/// Verifier-produced analysis only. The host derives a verdict after checking
/// all exact bindings and citations. This report grants no authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerifierReport {
    pub whole_response_assessed: bool,
    pub findings: Vec<AttributedFinding>,
    pub critical_errors: Vec<CriticalFinding>,
}

/// Native ingress attaches this trusted envelope to exact received bytes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerifierResult {
    pub invocation_id: String,
    pub attempt_id: String,
    pub session_id: String,
    pub input_sha256: String,
    pub raw: Vec<u8>,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedVerifierExposure {
    /// Host-observed control session must differ from compiler and successor.
    pub verifier_session_id: String,
    pub compiler_session_id: String,
    pub successor_session_id: Option<String>,
    pub input_sha256: String,
    pub saw_compiler_reasoning: bool,
    pub saw_hidden_oracle: bool,
    pub dynamic_tools_enabled: bool,
    pub human_side_effects_enabled: bool,
    pub requested_provider: String,
    pub requested_model: String,
    pub observed_provider: String,
    pub observed_model: String,
    pub exposure_id: String,
    /// Independent host observation/reference of the realized exposure.
    pub host_evidence_ref: String,
}

impl ObservedVerifierExposure {
    fn valid_for(&self, request: &VerifyRequest, policy: &SemanticPolicy) -> bool {
        self.verifier_session_id == request.binding.session_id
            && self.compiler_session_id == request.compiler_session_id
            && self.input_sha256 == request.prompt_sha256
            && self.exposure_id == request.binding.exposure_id
            && valid_id(&self.host_evidence_ref)
            && valid_id(&self.requested_provider)
            && valid_id(&self.requested_model)
            && self.requested_provider == self.observed_provider
            && self.requested_model == self.observed_model
            && !self.saw_compiler_reasoning
            && !self.saw_hidden_oracle
            && !self.dynamic_tools_enabled
            && !self.human_side_effects_enabled
            && (!policy.require_isolated_verifier
                || self.verifier_session_id != self.compiler_session_id)
            && self
                .successor_session_id
                .as_deref()
                .is_none_or(|id| id != self.verifier_session_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictStatus {
    Accepted,
    Rejected,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CheckedSemanticResult {
    pub phase: SemanticPhase,
    pub status: VerdictStatus,
    pub source_basis_sha256: String,
    pub candidate_sha256: String,
    pub challenge_sha256: Option<String>,
    pub successor_response_sha256: Option<String>,
    pub verifier_invocation_id: String,
    pub verifier_result_sha256: String,
    pub policy_id: String,
    pub findings: Vec<AttributedFinding>,
    pub critical_errors: Vec<CriticalFinding>,
    pub reasons: Vec<String>,
    /// Currentness is separate: this is only the dependency vector assessed.
    pub assessed_dependencies: Vec<DependencyRevision>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BasisEligibility {
    Current,
    Stale(Vec<DependencyChange>),
    Unavailable,
}

pub fn assess_currentness(
    result: &CheckedSemanticResult,
    current_dependencies: Option<&[DependencyRevision]>,
) -> BasisEligibility {
    match current_dependencies {
        None => BasisEligibility::Unavailable,
        Some(current) => {
            let changes = compare_dependencies(&result.assessed_dependencies, current);
            if changes.is_empty() {
                BasisEligibility::Current
            } else {
                BasisEligibility::Stale(changes)
            }
        }
    }
}

fn validate_result_binding(
    request: &VerifyRequest,
    result: &VerifierResult,
    policy: &SemanticPolicy,
) -> Result<(), SemanticError> {
    if result.invocation_id != request.binding.invocation_id
        || result.attempt_id != request.binding.attempt_id
        || result.session_id != request.binding.session_id
        || result.input_sha256 != request.prompt_sha256
        || result.sha256 != sha256_hex(&result.raw)
        || request.prompt_sha256 != sha256_hex(request.prompt.as_bytes())
    {
        return Err(SemanticError::WrongBinding);
    }
    if result.raw.len() > policy.max_control_output_bytes {
        return Err(SemanticError::OutputTooLarge);
    }
    Ok(())
}

fn derive(
    material: &SourceBasis,
    request: &VerifyRequest,
    result: VerifierResult,
    exposure: ObservedVerifierExposure,
    policy: &SemanticPolicy,
) -> Result<CheckedSemanticResult, SemanticError> {
    material.validate(policy)?;
    if !policy.within_frozen_limits()
        || request.binding.policy_id != policy.id
        || request.source_basis_sha256 != material.sha256
        || request.candidate_sha256.is_empty()
    {
        return Err(SemanticError::WrongBinding);
    }
    validate_result_binding(request, &result, policy)?;
    let report: VerifierReport =
        serde_json::from_slice(&result.raw).map_err(|_| SemanticError::InvalidOutput)?;
    let mut reasons = Vec::new();
    if !material.is_eligible() {
        reasons.push("source basis incomplete or over bound".into());
    }
    if !exposure.valid_for(request, policy) {
        reasons.push("verifier exposure or observed model identity unestablished".into());
    }
    if !report.whole_response_assessed {
        reasons.push("whole response not assessed".into());
    }
    if material.obligations.is_empty() {
        reasons.push("no required obligations were declared".into());
    }

    let obligations: BTreeMap<_, _> = material
        .obligations
        .iter()
        .map(|obligation| (&obligation.id, obligation))
        .collect();
    let mut seen = BTreeSet::new();
    let mut concrete_violation = false;
    for finding in &report.findings {
        if !seen.insert(finding.obligation_id.clone()) {
            reasons.push(format!("duplicate finding: {}", finding.obligation_id));
            continue;
        }
        let Some(obligation) = obligations.get(&finding.obligation_id) else {
            reasons.push(format!("unknown obligation: {}", finding.obligation_id));
            continue;
        };
        let attributed = !finding.explanation.trim().is_empty()
            && !finding.sources.is_empty()
            && finding
                .sources
                .iter()
                .all(|source| material.resolve(source).is_some())
            && finding
                .sources
                .iter()
                .any(|source| obligation.sources.contains(source));
        if !attributed {
            reasons.push(format!(
                "finding lacks valid attribution: {}",
                finding.obligation_id
            ));
        }
        match finding.status {
            FindingStatus::Violated => concrete_violation |= attributed,
            FindingStatus::Indeterminate => {
                reasons.push(format!(
                    "indeterminate obligation: {}",
                    finding.obligation_id
                ));
            }
            FindingStatus::Satisfied => {}
        }
    }
    for obligation in &material.obligations {
        if !seen.contains(&obligation.id) {
            reasons.push(format!("missing finding: {}", obligation.id));
        }
    }
    for critical in &report.critical_errors {
        if critical.explanation.trim().is_empty()
            || critical.sources.is_empty()
            || critical
                .sources
                .iter()
                .any(|source| material.resolve(source).is_none())
        {
            reasons.push("critical finding attribution invalid".into());
        } else {
            concrete_violation = true;
        }
    }
    let status = if concrete_violation {
        VerdictStatus::Rejected
    } else if !reasons.is_empty() {
        VerdictStatus::Unresolved
    } else {
        VerdictStatus::Accepted
    };
    Ok(CheckedSemanticResult {
        phase: request.binding.phase,
        status,
        source_basis_sha256: material.sha256.clone(),
        candidate_sha256: request.candidate_sha256.clone(),
        challenge_sha256: request.challenge_sha256.clone(),
        successor_response_sha256: request.successor_response_sha256.clone(),
        verifier_invocation_id: result.invocation_id,
        verifier_result_sha256: result.sha256,
        policy_id: policy.id.clone(),
        findings: report.findings,
        critical_errors: report.critical_errors,
        reasons,
        assessed_dependencies: material.dependencies.clone(),
    })
}

pub fn assess_candidate(
    material: &SourceBasis,
    candidate: &Candidate,
    request: &VerifyRequest,
    result: VerifierResult,
    exposure: ObservedVerifierExposure,
    policy: &SemanticPolicy,
) -> Result<CheckedSemanticResult, SemanticError> {
    let expected = verify_candidate(material, candidate, request.binding.clone(), policy)?;
    if *request != expected {
        return Err(SemanticError::WrongBinding);
    }
    request
        .binding
        .validate(material, SemanticPhase::CandidateVerification)?;
    if request.candidate_sha256 != candidate.sha256
        || candidate.sha256 != sha256_hex(&candidate.raw)
        || candidate.source_basis_sha256 != material.sha256
        || request.challenge_sha256.is_some()
        || request.successor_response_sha256.is_some()
    {
        return Err(SemanticError::WrongBinding);
    }
    derive(material, request, result, exposure, policy)
}

/// All four immutable semantic subjects are supplied separately so the caller
/// cannot accidentally assess a response against a different capture lineage.
#[allow(clippy::too_many_arguments)]
pub fn assess_successor(
    material: &SourceBasis,
    candidate: &Candidate,
    challenge: &Challenge,
    successor: &SuccessorResponse,
    request: &VerifyRequest,
    result: VerifierResult,
    exposure: ObservedVerifierExposure,
    policy: &SemanticPolicy,
) -> Result<CheckedSemanticResult, SemanticError> {
    let expected = verify_successor(
        material,
        candidate,
        challenge,
        successor,
        request.binding.clone(),
        policy,
    )?;
    if *request != expected {
        return Err(SemanticError::WrongBinding);
    }
    request
        .binding
        .validate(material, SemanticPhase::SuccessorAdequacy)?;
    if request.candidate_sha256 != candidate.sha256
        || request.challenge_sha256.as_deref() != Some(challenge.sha256.as_str())
        || request.successor_response_sha256.as_deref() != Some(successor.sha256.as_str())
        || successor.challenge_sha256 != challenge.sha256
        || successor.sha256 != sha256_hex(&successor.raw)
        || challenge.source_basis_sha256 != material.sha256
    {
        return Err(SemanticError::WrongBinding);
    }
    derive(material, request, result, exposure, policy)
}
