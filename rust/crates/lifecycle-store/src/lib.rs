//! Lifecycle transaction owner. Only this crate mints an entry after a durable claim.

mod artifact;
mod recovery;
mod sqlite;

pub use artifact::{PublishedArtifact, StagedArtifact};
pub use recovery::{RecoveryDisposition, RecoveryEntry, RecoverySnapshot};
pub use sqlite::SqliteLifecycleStore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectProjection {
    pub store_id: String,
    pub subject_id: String,
    pub context_id: String,
    pub revision: u64,
    pub semantic_revision: u64,
    pub owner_kind: String,
    pub owner_ref: Option<String>,
    pub journal_cursor: u64,
    pub transition: Option<TransitionProjection>,
    pub delivery: Option<DeliveryProjection>,
    pub unresolved_attempts: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionProjection {
    pub transition_id: String,
    pub stage: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryProjection {
    pub delivery_id: String,
    pub effect_id: Option<String>,
    pub attempt_id: Option<String>,
    pub outcome: String,
    pub settlement_kind: String,
    pub settlement_evidence: Option<String>,
    pub source_ref: Option<String>,
    pub observed_wall_ms: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedTextResult {
    pub source_id: String,
    pub attempt_id: String,
    pub provider_thread_id: String,
    pub final_text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierReport {
    pub source_id: String,
    pub activation: String,
    pub disposition: String,
    pub primary_error: String,
    pub exit_code: Option<i32>,
    pub exit_success: bool,
    pub unsafe_at_close: bool,
    pub late_at_close: bool,
}

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
