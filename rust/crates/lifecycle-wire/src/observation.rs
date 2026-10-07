//! Public observation values. They describe evidence; they confer no authority.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{CursorV1, PROTOCOL_VERSION, WireError};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum FieldV1<T> {
    Known(T),
    Unknown,
    NotApplicable,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOutcomeV1 {
    Pending,
    Completed,
    Failed,
    Interrupted,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EffectSettlementV1 {
    Unresolved,
    Established { evidence_ref: String },
    Conflict { evidence_ref: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AdmissionV1 {
    Domain { context_generation: String },
    Transition { transition_id: String },
    Fenced { reason: String },
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TransitionV1 {
    pub transition_id: String,
    pub stage: TransitionStageV1,
    pub completed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TransitionStageV1 {
    Quiescing,
    Capturing,
    Verifying,
    Checkpointed,
    Switching,
    Rehydrating,
    ReadyToCommit,
    Reconciled,
    RecoveryRequired,
    AbortedBeforeEntry,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DeliveryV1 {
    pub delivery_id: String,
    pub effect_id: String,
    pub attempt_id: String,
    pub outcome: ExecutionOutcomeV1,
    pub settlement: EffectSettlementV1,
    pub provenance: FieldV1<ObservationProvenanceV1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ObservationProvenanceV1 {
    pub source_ref: String,
    /// Validated decimal Unix milliseconds supplied by the observation owner.
    pub observed_wall_ms: String,
}

impl ObservationProvenanceV1 {
    fn validate(&self) -> Result<(), WireError> {
        if self.source_ref.is_empty()
            || self
                .observed_wall_ms
                .parse::<i64>()
                .ok()
                .map(|value| value.to_string())
                != Some(self.observed_wall_ms.clone())
        {
            return Err(WireError::SemanticInvalidity);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAvailabilityV1 {
    Available,
    Unavailable,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LifecycleSnapshotV1 {
    pub protocol_version: u16,
    pub subject_id: String,
    pub context_generation: String,
    pub cursor: CursorV1,
    /// For a targeted read, the named durable transition when established.
    /// For an untargeted read, the service-selected relevant transition.
    pub transition: FieldV1<TransitionV1>,
    pub current_admission: AdmissionV1,
    /// For a targeted read, the named durable delivery when established.
    /// Current admission and the cursor still describe the current read.
    pub delivery: FieldV1<DeliveryV1>,
    pub runtime_availability: RuntimeAvailabilityV1,
    pub runtime_provenance: FieldV1<ObservationProvenanceV1>,
    pub blocking_reason: FieldV1<String>,
    pub authority_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub measurement: FieldV1<String>,
}

impl LifecycleSnapshotV1 {
    pub fn validate_header(&self, expected_subject: &str) -> Result<(), WireError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(WireError::UnsupportedVersion);
        }
        if self.subject_id != expected_subject || self.context_generation.is_empty() {
            return Err(WireError::SemanticInvalidity);
        }
        self.cursor.validate()?;
        if let FieldV1::Known(provenance) = &self.runtime_provenance {
            provenance.validate()?;
        }
        if let FieldV1::Known(delivery) = &self.delivery
            && let FieldV1::Known(provenance) = &delivery.provenance
        {
            provenance.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WaitRequestV1 {
    pub protocol_version: u16,
    pub subject_id: String,
    pub cursor: CursorV1,
    pub target: WaitTargetV1,
    pub max_wait_ms: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorDtoV1 {
    pub protocol_version: u16,
    pub code: crate::WireErrorCode,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WaitTargetV1 {
    DeliveryOutcome {
        delivery_id: String,
        outcome: ExecutionOutcomeV1,
    },
    TransitionStage {
        transition_id: String,
        stage: TransitionStageV1,
    },
}

impl WaitTargetV1 {
    pub fn is_observed(&self, snapshot: &LifecycleSnapshotV1) -> bool {
        match self {
            Self::DeliveryOutcome {
                delivery_id,
                outcome,
            } => matches!(
                &snapshot.delivery,
                FieldV1::Known(delivery) if &delivery.delivery_id == delivery_id && &delivery.outcome == outcome
            ),
            Self::TransitionStage {
                transition_id,
                stage,
            } => matches!(
                &snapshot.transition,
                FieldV1::Known(transition) if &transition.transition_id == transition_id && &transition.stage == stage
            ),
        }
    }
}
