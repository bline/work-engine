//! Offline verifier replay only. There is no transport, credential, store,
//! compiler, successor or publication capability in this package.

use bounded_inference::{ObservedTerminal, StreamObservation, StreamObserver, ToolFreeProfile};
use lifecycle_semantic::{
    AssessedResultEnvelope, DirectCandidate, DirectExposure, DirectPreparedRequest,
    DirectVerifierResult, FixtureScope, SemanticPolicy, SourceBasis, assess_direct_candidate,
    sha256_hex,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OfflineReplayInput {
    pub material: SourceBasis,
    pub candidate: DirectCandidate,
    pub prepared: DirectPreparedRequest,
    pub exposure: DirectExposure,
    pub profile: ToolFreeProfile,
    pub capture: CapturedFixtureStream,
}

/// Synthetic capture coordinates are fixed with the stream bytes, before a
/// request is selected for assessment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedFixtureStream {
    pub capture_ref: String,
    pub scope: FixtureScope,
    pub operation_id: String,
    pub attempt_id: String,
    pub prepared_binding_sha256: String,
    pub stream_bytes: Vec<u8>,
    pub stream_sha256: String,
}

impl CapturedFixtureStream {
    pub fn new(
        capture_ref: String,
        prepared: &DirectPreparedRequest,
        stream_bytes: Vec<u8>,
    ) -> Result<Self, ReplayError> {
        prepared.validate()?;
        if capture_ref.is_empty() {
            return Err(ReplayError::CaptureMismatch);
        }
        Ok(Self {
            capture_ref,
            scope: prepared.request.binding.scope.clone(),
            operation_id: prepared.request.binding.operation_id.clone(),
            attempt_id: prepared.request.binding.attempt_id.clone(),
            prepared_binding_sha256: prepared.binding_sha256.clone(),
            stream_sha256: sha256_hex(&stream_bytes),
            stream_bytes,
        })
    }

    fn validate_for(&self, prepared: &DirectPreparedRequest) -> Result<(), ReplayError> {
        if self.capture_ref.is_empty()
            || self.scope != prepared.request.binding.scope
            || self.operation_id != prepared.request.binding.operation_id
            || self.attempt_id != prepared.request.binding.attempt_id
            || self.prepared_binding_sha256 != prepared.binding_sha256
            || self.stream_sha256 != sha256_hex(&self.stream_bytes)
        {
            return Err(ReplayError::CaptureMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ReplayError {
    PreparedRequestMismatch,
    CaptureMismatch,
    TransportOutcome,
    Stream(bounded_inference::StreamError),
    Request(bounded_inference::RequestError),
    Semantic(lifecycle_semantic::SemanticError),
}

impl From<bounded_inference::StreamError> for ReplayError {
    fn from(value: bounded_inference::StreamError) -> Self {
        Self::Stream(value)
    }
}
impl From<bounded_inference::RequestError> for ReplayError {
    fn from(value: bounded_inference::RequestError) -> Self {
        Self::Request(value)
    }
}
impl From<lifecycle_semantic::SemanticError> for ReplayError {
    fn from(value: lifecycle_semantic::SemanticError) -> Self {
        Self::Semantic(value)
    }
}

pub fn replay_fixture(
    input: &OfflineReplayInput,
    policy: &SemanticPolicy,
) -> Result<AssessedResultEnvelope, ReplayError> {
    input.prepared.validate()?;
    input.capture.validate_for(&input.prepared)?;
    let assembled = bounded_inference::assemble_request(
        &input.prepared.instructions,
        &input.prepared.input,
        &input.profile,
    )?;
    if assembled.bytes != input.prepared.wire_bytes
        || assembled.sha256 != input.prepared.wire_sha256
        || assembled.profile_revision != input.prepared.profile_revision
        || assembled.selected_model != input.prepared.requested_model
        || input.prepared.request.binding.scope.owner.is_empty()
    {
        return Err(ReplayError::PreparedRequestMismatch);
    }
    let mut observer = StreamObserver::new(input.profile.bounds.clone())?;
    observer.push(&input.capture.stream_bytes)?;
    let observation = observer.finish_evidence();
    assess_completed_observation(input, observation, policy)
}

fn assess_completed_observation(
    input: &OfflineReplayInput,
    observation: StreamObservation,
    policy: &SemanticPolicy,
) -> Result<AssessedResultEnvelope, ReplayError> {
    if observation.local_failure.is_some()
        || observation.terminal != Some(ObservedTerminal::Completed)
    {
        return Err(ReplayError::TransportOutcome);
    }
    let output = observation.output.ok_or(ReplayError::TransportOutcome)?;
    let raw = output.into_bytes();
    let result = DirectVerifierResult {
        operation_id: input.capture.operation_id.clone(),
        attempt_id: input.capture.attempt_id.clone(),
        prepared_wire_sha256: input.prepared.wire_sha256.clone(),
        prepared_binding_sha256: input.capture.prepared_binding_sha256.clone(),
        capture_ref: input.capture.capture_ref.clone(),
        captured_stream_sha256: input.capture.stream_sha256.clone(),
        semantic_prompt_sha256: input.prepared.request.semantic_prompt_sha256.clone(),
        response_id: observation.response_id,
        observed_model: observation.observed_model,
        sha256: sha256_hex(&raw),
        raw,
    };
    assess_direct_candidate(
        &input.material,
        &input.candidate,
        &input.prepared,
        result,
        input.exposure.clone(),
        policy,
    )
    .map_err(Into::into)
}
