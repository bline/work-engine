//! Lifecycle transaction owner. Only this crate mints an entry after a durable claim.

mod artifact;
mod recovery;
mod sqlite;

pub use artifact::{PublishedArtifact, StagedArtifact};
pub use recovery::RecoverySnapshot;
pub use sqlite::SqliteLifecycleStore;

use lifecycle_core::{
    AttemptId, ClockSample, CommandAdmission, EffectId, EffectInput, EffectObservation, EffectPlan,
    EffectSettlement, ExecutionOutcome, RuntimeIncarnation, SubjectId,
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
    fn new(
        effect: EffectId,
        attempt: AttemptId,
        subject: SubjectId,
        incarnation: RuntimeIncarnation,
        input: EffectInput,
    ) -> Self {
        Self {
            effect,
            attempt,
            subject,
            incarnation,
            input,
        }
    }

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationApply {
    Applied,
    Duplicate,
    ConflictFenced,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectProjection {
    pub effect: EffectId,
    pub attempt: AttemptId,
    pub outcome: ExecutionOutcome,
    pub settlement: EffectSettlement,
    pub admission_fenced: bool,
}

/// Domain-owned transactional port. The service supplies a checked admission envelope.
pub trait LifecycleStore {
    type CommandResult;
    type ApplyResult;

    fn apply_command(
        &mut self,
        admission: CommandAdmission,
        clock: ClockSample,
    ) -> Result<Self::CommandResult, StoreError>;

    fn prepare_next_input(&mut self, subject: &SubjectId) -> Result<EffectPlan, StoreError>;

    fn claim_entry(
        &mut self,
        effect: &EffectId,
        incarnation: RuntimeIncarnation,
        clock: ClockSample,
    ) -> Result<AuthorizedEntry, StoreError>;

    fn apply_observation(
        &mut self,
        observation: EffectObservation,
        clock: ClockSample,
    ) -> Result<Self::ApplyResult, StoreError>;
}
