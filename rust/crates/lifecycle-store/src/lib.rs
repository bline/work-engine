//! Transaction boundary for future S1 implementation. Only this crate can mint
//! a claimed entry after a durable authority/revision transaction.

use lifecycle_core::{
    AttemptId, ClockSample, CommandRequest, EffectId, EffectInput, EffectObservation, EffectPlan,
    RuntimeIncarnation, SubjectId,
};
use thiserror::Error;

#[must_use = "an accepted entry must be transferred to its executor or durably accounted"]
#[derive(Debug)]
pub struct AuthorizedEntry {
    effect: EffectId,
    attempt: AttemptId,
    subject: SubjectId,
    incarnation: RuntimeIncarnation,
    input: EffectInput,
}

impl AuthorizedEntry {
    pub fn effect(&self) -> &EffectId {
        &self.effect
    }

    pub fn attempt(&self) -> &AttemptId {
        &self.attempt
    }

    pub fn subject(&self) -> &SubjectId {
        &self.subject
    }

    pub fn incarnation(&self) -> &RuntimeIncarnation {
        &self.incarnation
    }

    /// The exact committed operation input; the executor must derive its request from this.
    pub fn input(&self) -> &EffectInput {
        &self.input
    }
}

#[must_use = "capture authorization must be consumed or recorded"]
#[derive(Debug)]
pub struct CapturePermit {
    subject: SubjectId,
    incarnation: RuntimeIncarnation,
}

impl CapturePermit {
    pub fn subject(&self) -> &SubjectId {
        &self.subject
    }

    pub fn incarnation(&self) -> &RuntimeIncarnation {
        &self.incarnation
    }
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("authority or revision rejected the command")]
    Rejected,
    #[error("entry claim conflicted with durable state")]
    ClaimConflict,
    #[error("observation cannot be applied to the claimed attempt")]
    ObservationMismatch,
    #[error("store is unavailable")]
    Unavailable,
}

/// Transactional signatures only. S1 owns all implementations and SQLite policy.
pub trait LifecycleStore {
    type CommandResult;
    type ApplyResult;

    fn apply_command(
        &mut self,
        request: CommandRequest,
        clock: ClockSample,
    ) -> Result<Self::CommandResult, StoreError>;

    fn claim_entry(
        &mut self,
        plan: EffectPlan,
        incarnation: RuntimeIncarnation,
        clock: ClockSample,
    ) -> Result<AuthorizedEntry, StoreError>;

    fn apply_observation(
        &mut self,
        observation: EffectObservation,
        clock: ClockSample,
    ) -> Result<Self::ApplyResult, StoreError>;
}
