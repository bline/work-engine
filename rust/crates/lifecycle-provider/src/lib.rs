//! External execution facts. A provider cannot grant lifecycle admission.

use std::future::Future;

use lifecycle_core::{
    AttemptId, ControlledTextInput, EffectId, EffectSettlement, EvidenceId, ExecutionOutcome,
    ProviderThreadId, ProviderTurnId, RuntimeIncarnation, SubjectId,
};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationProfile {
    ControlledText,
    NativeText,
    NativeSnapshotRead,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextTurnRequest {
    pub subject: SubjectId,
    pub effect: EffectId,
    pub attempt: AttemptId,
    pub incarnation: RuntimeIncarnation,
    /// Preserve the input ID, exact text and trusted digest from the claimed entry.
    pub input: ControlledTextInput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextTurnObservation {
    /// Stable coordinate assigned at ingress; replay must retain it.
    pub source: EvidenceId,
    pub effect: EffectId,
    pub attempt: AttemptId,
    pub incarnation: RuntimeIncarnation,
    pub thread: ProviderThreadId,
    pub turn: ProviderTurnId,
    pub final_text: Option<String>,
    /// Provider-qualified facts, not a conclusion drawn from local task exit.
    pub outcome: ExecutionOutcome,
    pub settlement: EffectSettlement,
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("requested operation profile is unsupported")]
    UnsupportedProfile,
    #[error("provider entry is uncertain")]
    EntryUncertain,
    #[error("provider observation is unavailable")]
    ObservationUnavailable,
}

/// S2 will bind this port to store-claimed work. S0 ships no implementation.
pub trait TextTurnPort {
    type Run: Future<Output = Result<TextTurnObservation, ProviderError>> + Send;

    fn profile(&self) -> OperationProfile;
    fn execute(&self, request: TextTurnRequest) -> Self::Run;
}
