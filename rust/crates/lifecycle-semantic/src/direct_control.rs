//! Direct compile and separate verify values for host-submitted tool-free
//! inference.
//!
//! These values carry operation and attempt identity only. A direct call has no
//! native session or thread, and nothing here invents one. The fixture-scoped
//! `invocation_v2` envelope and the B2 diagnostic assessment are not reused.
//!
//! The host derives exposure from the realized request bytes and the captured
//! response. A caller cannot supply an exposure verdict, and this module never
//! decides production eligibility: the lifecycle owner's policy does.

use crate::{
    Candidate, CheckedSemanticResult, SemanticError, SemanticPolicy, SourceBasis, VerifierReport,
    assessment::derive_report, continuation::candidate_verification_prompt,
    continuation::compile_prompt, continuation::validate_candidate_content, sha256_hex,
    valid_digest, valid_id,
};
use lifecycle_core::{SubjectId, TransitionId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectRole {
    Compile,
    Verify,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectOperationBinding {
    pub subject: SubjectId,
    pub transition: TransitionId,
    pub capture_id: String,
    pub role: DirectRole,
    pub policy_id: String,
    pub operation_id: String,
    pub attempt_id: String,
    pub exposure_id: String,
}

impl DirectOperationBinding {
    pub fn validate(&self, material: &SourceBasis, role: DirectRole) -> Result<(), SemanticError> {
        if self.subject != material.subject
            || self.transition != material.transition
            || self.capture_id != material.capture_id
            || self.policy_id != material.policy_id
            || self.role != role
        {
            return Err(SemanticError::WrongBinding);
        }
        if [&self.operation_id, &self.attempt_id, &self.exposure_id]
            .iter()
            .any(|id| !valid_id(id))
        {
            return Err(SemanticError::InvalidIdentifier);
        }
        Ok(())
    }
}

/// Host-observed provenance of a capture: the transport facts, never a verdict.
///
/// `SyntheticLocal` is a fixture or loopback peer. `PublicApi` is the fixed
/// public provider endpoint reached by the bounded-inference public transport.
/// Neither variant decides production eligibility: that is the owner's
/// admission of a `DirectProductionProfile`, recorded as a store grant, and the
/// class alone can never stand in for it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "class", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum DirectEvidenceClass {
    SyntheticLocal { fixture_ref: String },
    PublicApi { endpoint: String },
}

impl DirectEvidenceClass {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SyntheticLocal { .. } => "synthetic_local",
            Self::PublicApi { .. } => "public_api",
        }
    }

    pub fn reference(&self) -> &str {
        match self {
            Self::SyntheticLocal { fixture_ref } => fixture_ref,
            Self::PublicApi { endpoint } => endpoint,
        }
    }
}

/// The checkpoint profile the lifecycle owner may admit for direct control
/// (`b3-direct-host-separated-control-v1`): host-separated tool-free compile and
/// verify over the fixed public API, under the provider trust the owner accepts.
///
/// This value only states which captured evidence the profile is *about*. It is
/// not an admission: admission is an owner grant of
/// `GrantScope::SemanticDirectCheckpoint`, installed through the store's trusted
/// issuer, and is read by the store at the publication boundary.
pub struct DirectProductionProfile;

impl DirectProductionProfile {
    pub const ID: &'static str = "b3-direct-host-separated-control-v1";
    /// The only endpoint this profile is about.
    pub const PUBLIC_ENDPOINT: &'static str = "https://api.openai.com/v1/responses";

    /// Evidence the profile covers. A synthetic or loopback class is never
    /// covered, whatever reference it carries.
    pub fn covers(evidence: &DirectEvidenceClass) -> bool {
        matches!(evidence, DirectEvidenceClass::PublicApi { endpoint } if endpoint == Self::PUBLIC_ENDPOINT)
    }

    /// Same predicate over a stored class label and reference.
    pub fn covers_stored(label: &str, reference: &str) -> bool {
        label == "public_api" && reference == Self::PUBLIC_ENDPOINT
    }

    /// The class the public transport reports. A value with any other endpoint
    /// is simply not covered.
    pub fn public_api() -> DirectEvidenceClass {
        DirectEvidenceClass::PublicApi {
            endpoint: Self::PUBLIC_ENDPOINT.into(),
        }
    }
}

/// Why a submission produced no capture. `NotSent` is the adapter's claim that
/// no byte reached the endpoint; every other failure leaves the outcome unknown.
/// `Withheld` is the stronger claim that the adapter refused before consuming
/// any release, custody or action identity, so nothing was or can be sent for
/// it: the lifecycle owner may withdraw the intent and offer the call again.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectSubmitError {
    NotSent,
    OutcomeUnknown,
    Withheld,
}

/// Whether the release owner's inputs for one exact prepared call are present
/// and consistent now. `Pending` is not an outcome: nothing was recorded or
/// consumed, and the lifecycle owner may offer the same call again later.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectReleaseState {
    Ready,
    Pending { reason: String },
}

/// Endpoint seam owned by the lifecycle consumer and implemented beside the
/// transport. The adapter assembles the wire and fills a capture only from
/// bytes it actually sent and received; it assigns no semantic verdict.
/// `submit` is invoked at most once per operation: the store commits send
/// intent first and never re-offers a possibly sent call.
pub trait DirectInferencePort: Send + Sync {
    /// Evidence class this port's captures belong to. The port owns it; a
    /// service configuration cannot declare one for a port.
    fn evidence(&self) -> DirectEvidenceClass;
    fn wire(&self, role: DirectRole, input: &str) -> Result<DirectWire, DirectSubmitError>;
    /// Expose the exact prepared call to its release owner and report whether
    /// that owner's release is present, before any send intent exists. It
    /// consumes nothing and is not an admission: `submit` re-checks every
    /// input at the send boundary. Ports without an external release owner
    /// are always `Ready`.
    fn offer_release(&self, call: &DirectPreparedCall) -> DirectReleaseState {
        let _ = call;
        DirectReleaseState::Ready
    }
    fn submit(&self, call: &DirectPreparedCall) -> Result<DirectCapture, DirectSubmitError>;
}

/// Instruction and wire bytes assembled by the trusted endpoint adapter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectWire {
    pub instructions: String,
    pub wire_bytes: Vec<u8>,
    pub profile_revision: String,
    pub requested_provider: String,
    pub requested_model: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectPreparedCall {
    pub binding: DirectOperationBinding,
    pub source_basis_sha256: String,
    /// Present for verification only: the candidate the verifier is given.
    pub candidate_sha256: Option<String>,
    /// Present for verification only: the prepared compile call that produced it.
    pub compile_binding_sha256: Option<String>,
    pub input: String,
    pub input_sha256: String,
    pub wire: DirectWire,
    pub wire_sha256: String,
    pub max_output_bytes: usize,
    /// Digest of every field above, fixed after the wire bytes are final.
    pub binding_sha256: String,
}

const FORBIDDEN_WIRE_KEYS: &[&str] = &[
    "tools",
    "tool_choice",
    "parallel_tool_calls",
    "previous_response_id",
    "conversation",
];

impl DirectPreparedCall {
    fn compute_binding(&self) -> Result<String, SemanticError> {
        crate::json_digest(&(
            &self.binding,
            &self.source_basis_sha256,
            &self.candidate_sha256,
            &self.compile_binding_sha256,
            &self.input_sha256,
            &self.wire,
            &self.wire_sha256,
            self.max_output_bytes,
        ))
    }

    /// Mechanical request facts. Missing or contradicted facts leave exposure
    /// unestablished rather than assuming a clean request.
    fn wire_is_tool_free_and_carries_input(&self) -> bool {
        let Ok(serde_json::Value::Object(map)) =
            serde_json::from_slice::<serde_json::Value>(&self.wire.wire_bytes)
        else {
            return false;
        };
        if FORBIDDEN_WIRE_KEYS.iter().any(|key| map.contains_key(*key)) {
            return false;
        }
        let Ok(input_json) = serde_json::to_string(&self.input) else {
            return false;
        };
        let Ok(instructions_json) = serde_json::to_string(&self.wire.instructions) else {
            return false;
        };
        let wire = String::from_utf8_lossy(&self.wire.wire_bytes);
        wire.contains(&input_json)
            && wire.contains(&instructions_json)
            && map.get("model").and_then(|v| v.as_str()) == Some(self.wire.requested_model.as_str())
    }

    pub fn validate(&self) -> Result<(), SemanticError> {
        if self.input_sha256 != sha256_hex(self.input.as_bytes())
            || self.wire.wire_bytes.is_empty()
            || self.wire_sha256 != sha256_hex(&self.wire.wire_bytes)
            || !valid_id(&self.wire.profile_revision)
            || !valid_id(&self.wire.requested_provider)
            || !valid_id(&self.wire.requested_model)
            || self.binding_sha256 != self.compute_binding()?
            || !self.wire_is_tool_free_and_carries_input()
        {
            return Err(SemanticError::WrongBinding);
        }
        Ok(())
    }

    fn sealed(mut self) -> Result<Self, SemanticError> {
        self.binding_sha256 = self.compute_binding()?;
        self.validate()?;
        Ok(self)
    }
}

/// The exact text the endpoint must wrap for a compile call.
pub fn direct_compile_input(
    material: &SourceBasis,
    binding: &DirectOperationBinding,
    policy: &SemanticPolicy,
) -> Result<(String, String), SemanticError> {
    material.validate(policy)?;
    if !material.is_eligible() || !policy.within_frozen_limits() || policy.id != material.policy_id
    {
        return Err(SemanticError::IneligibleMaterial);
    }
    binding.validate(material, DirectRole::Compile)?;
    compile_prompt(material, policy)
}

pub fn prepare_direct_compile(
    material: &SourceBasis,
    binding: DirectOperationBinding,
    policy: &SemanticPolicy,
    wire: DirectWire,
) -> Result<DirectPreparedCall, SemanticError> {
    let (input, input_sha256) = direct_compile_input(material, &binding, policy)?;
    seal_call(material, binding, None, None, input, input_sha256, wire, policy)
}

#[allow(clippy::too_many_arguments)]
fn seal_call(
    material: &SourceBasis,
    binding: DirectOperationBinding,
    candidate_sha256: Option<String>,
    compile_binding_sha256: Option<String>,
    input: String,
    input_sha256: String,
    wire: DirectWire,
    policy: &SemanticPolicy,
) -> Result<DirectPreparedCall, SemanticError> {
    let wire_sha256 = sha256_hex(&wire.wire_bytes);
    DirectPreparedCall {
        binding,
        source_basis_sha256: material.sha256.clone(),
        candidate_sha256,
        compile_binding_sha256,
        input,
        input_sha256,
        wire,
        wire_sha256,
        max_output_bytes: policy.max_control_output_bytes,
        binding_sha256: String::new(),
    }
    .sealed()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectTerminal {
    Complete,
    Incomplete,
    Failed,
}

/// What the host observed for one submitted call. The adapter fills it from the
/// bytes it actually sent and received; a model's own output cannot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectCapture {
    pub operation_id: String,
    pub attempt_id: String,
    pub prepared_wire_sha256: String,
    pub prepared_binding_sha256: String,
    pub capture_ref: String,
    pub captured_stream_sha256: String,
    pub terminal: DirectTerminal,
    pub response_id: Option<String>,
    pub observed_provider: Option<String>,
    pub observed_model: Option<String>,
    pub output: Vec<u8>,
    pub output_sha256: String,
    pub evidence: DirectEvidenceClass,
}

impl DirectCapture {
    /// A complete, attributable response for exactly this prepared call.
    fn attributes_to(&self, call: &DirectPreparedCall) -> bool {
        self.operation_id == call.binding.operation_id
            && self.attempt_id == call.binding.attempt_id
            && self.prepared_wire_sha256 == call.wire_sha256
            && self.prepared_binding_sha256 == call.binding_sha256
            && self.output_sha256 == sha256_hex(&self.output)
            && self.output.len() <= call.max_output_bytes
            && valid_id(&self.capture_ref)
            && valid_digest(&self.captured_stream_sha256)
            && self.terminal == DirectTerminal::Complete
            && self.response_id.as_deref().is_some_and(valid_id)
            && self.observed_provider.as_deref() == Some(call.wire.requested_provider.as_str())
            && self.observed_model.as_deref() == Some(call.wire.requested_model.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectCompileProvenance {
    pub operation_id: String,
    pub attempt_id: String,
    pub prepared_binding_sha256: String,
    pub prepared_wire_sha256: String,
    pub capture_ref: String,
    pub captured_stream_sha256: String,
    pub response_id: String,
    pub observed_model: String,
}

/// A direct compile yields the shared `Candidate`, so every downstream consumer
/// reads one value. Its origin is the attributable operation, not a session.
pub fn direct_candidate_from_capture(
    material: &SourceBasis,
    call: &DirectPreparedCall,
    capture: &DirectCapture,
    policy: &SemanticPolicy,
) -> Result<Candidate, SemanticError> {
    material.validate(policy)?;
    call.binding.validate(material, DirectRole::Compile)?;
    call.validate()?;
    if call.source_basis_sha256 != material.sha256 || call.candidate_sha256.is_some() {
        return Err(SemanticError::WrongBinding);
    }
    if !capture.attributes_to(call) {
        return Err(SemanticError::WrongBinding);
    }
    let plan = validate_candidate_content(material, &capture.output, policy)?;
    Ok(Candidate {
        source_basis_sha256: material.sha256.clone(),
        compile_invocation_id: call.binding.operation_id.clone(),
        compile_attempt_id: call.binding.attempt_id.clone(),
        compile_session_id: None,
        direct: Some(DirectCompileProvenance {
            operation_id: call.binding.operation_id.clone(),
            attempt_id: call.binding.attempt_id.clone(),
            prepared_binding_sha256: call.binding_sha256.clone(),
            prepared_wire_sha256: call.wire_sha256.clone(),
            capture_ref: capture.capture_ref.clone(),
            captured_stream_sha256: capture.captured_stream_sha256.clone(),
            response_id: capture.response_id.clone().unwrap_or_default(),
            observed_model: capture.observed_model.clone().unwrap_or_default(),
        }),
        compile_prompt_sha256: call.input_sha256.clone(),
        sha256: sha256_hex(&capture.output),
        raw: capture.output.clone(),
        plan,
    })
}

pub fn direct_verify_input(
    material: &SourceBasis,
    candidate: &Candidate,
    binding: &DirectOperationBinding,
    compile_call: &DirectPreparedCall,
    policy: &SemanticPolicy,
) -> Result<(String, String), SemanticError> {
    material.validate(policy)?;
    if !material.is_eligible() || !policy.within_frozen_limits() || policy.id != material.policy_id
    {
        return Err(SemanticError::IneligibleMaterial);
    }
    binding.validate(material, DirectRole::Verify)?;
    if candidate.source_basis_sha256 != material.sha256
        || candidate.sha256 != sha256_hex(&candidate.raw)
        || candidate.plan != validate_candidate_content(material, &candidate.raw, policy)?
        || !candidate.origin_is_coherent()
        || candidate.direct.as_ref().map(|d| d.prepared_binding_sha256.as_str())
            != Some(compile_call.binding_sha256.as_str())
        || candidate.compile_invocation_id != compile_call.binding.operation_id
        || binding.operation_id == candidate.compile_invocation_id
        || binding.attempt_id == candidate.compile_attempt_id
        || binding.exposure_id == compile_call.binding.exposure_id
    {
        return Err(SemanticError::WrongBinding);
    }
    candidate_verification_prompt(material, &candidate.plan, &candidate.sha256, policy)
}

pub fn prepare_direct_verify(
    material: &SourceBasis,
    candidate: &Candidate,
    compile_call: &DirectPreparedCall,
    binding: DirectOperationBinding,
    policy: &SemanticPolicy,
    wire: DirectWire,
) -> Result<DirectPreparedCall, SemanticError> {
    let (input, input_sha256) =
        direct_verify_input(material, candidate, &binding, compile_call, policy)?;
    // A verifier request that equals the compile request would be a resend, not
    // a separate judgment.
    if sha256_hex(&wire.wire_bytes) == compile_call.wire_sha256 {
        return Err(SemanticError::WrongBinding);
    }
    seal_call(
        material,
        binding,
        Some(candidate.sha256.clone()),
        Some(compile_call.binding_sha256.clone()),
        input,
        input_sha256,
        wire,
        policy,
    )
}

/// Host-derived verdict plus the identities that bind it to both calls.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectCheckedResult {
    pub checked: CheckedSemanticResult,
    pub compile_binding_sha256: String,
    pub verify_binding_sha256: String,
    pub verifier_capture_ref: String,
    pub evidence: DirectEvidenceClass,
    /// Whether the host established the verifier exposure and model identity.
    /// The verdict above already includes the consequence of `false`.
    pub exposure_established: bool,
}

pub fn assess_direct_control_candidate(
    material: &SourceBasis,
    candidate: &Candidate,
    compile_call: &DirectPreparedCall,
    verify_call: &DirectPreparedCall,
    capture: &DirectCapture,
    policy: &SemanticPolicy,
) -> Result<DirectCheckedResult, SemanticError> {
    material.validate(policy)?;
    if !policy.within_frozen_limits() || policy.id != material.policy_id {
        return Err(SemanticError::WrongBinding);
    }
    verify_call.validate()?;
    let (input, input_sha256) = direct_verify_input(
        material,
        candidate,
        &verify_call.binding,
        compile_call,
        policy,
    )?;
    if verify_call.input != input
        || verify_call.input_sha256 != input_sha256
        || verify_call.candidate_sha256.as_deref() != Some(candidate.sha256.as_str())
        || verify_call.compile_binding_sha256.as_deref() != Some(compile_call.binding_sha256.as_str())
        || verify_call.source_basis_sha256 != material.sha256
        || verify_call.wire_sha256 == compile_call.wire_sha256
    {
        return Err(SemanticError::WrongBinding);
    }
    if capture.operation_id != verify_call.binding.operation_id
        || capture.attempt_id != verify_call.binding.attempt_id
        || capture.prepared_wire_sha256 != verify_call.wire_sha256
        || capture.prepared_binding_sha256 != verify_call.binding_sha256
        || capture.output_sha256 != sha256_hex(&capture.output)
    {
        return Err(SemanticError::WrongBinding);
    }
    let exposure_established = capture.attributes_to(verify_call);
    let checked = derive_report(
        material,
        crate::SemanticPhase::CandidateVerification,
        &candidate.sha256,
        None,
        None,
        verify_call.binding.operation_id.clone(),
        capture.output_sha256.clone(),
        &capture.output,
        exposure_established,
        policy,
    )?;
    Ok(DirectCheckedResult {
        checked,
        compile_binding_sha256: compile_call.binding_sha256.clone(),
        verify_binding_sha256: verify_call.binding_sha256.clone(),
        verifier_capture_ref: capture.capture_ref.clone(),
        evidence: capture.evidence.clone(),
        exposure_established,
    })
}

/// Shape-only parse, exposed so a service can classify a verifier output as
/// malformed without deriving a verdict.
pub fn parses_as_verifier_report(raw: &[u8]) -> bool {
    serde_json::from_slice::<VerifierReport>(raw).is_ok()
}
