//! `ContextPressureController`: hysteresis state machine over pressure observations.
//!
//! It returns transition evidence only. It does not own checkpoint validity, transition
//! progress or reset authority, and a disposition never authorizes retirement.

use crate::error::{ErrorKind, PressureError};
use crate::observation::PressureObservation;
use crate::policy::{ContextPressurePolicy, Disposition};
use crate::{fields, js};
use serde::Serialize;
use serde_json::Value;

const SNAPSHOT_SCHEMA_VERSION: u8 = 1;
const TRANSITION_SCHEMA_VERSION: u8 = 1;

/// Why a transition record holds its disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionReason {
    ThresholdCrossed,
    BelowEntryThreshold,
    HysteresisHeld,
    WithinDisposition,
}

/// The pressure evidence record for one accepted observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PressureTransition {
    schema_version: u8,
    policy_revision: String,
    observation: PressureObservation,
    previous_disposition: Disposition,
    disposition: Disposition,
    changed: bool,
    reason: TransitionReason,
}

impl PressureTransition {
    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }

    pub fn observation(&self) -> &PressureObservation {
        &self.observation
    }

    pub fn previous_disposition(&self) -> Disposition {
        self.previous_disposition
    }

    pub fn disposition(&self) -> Disposition {
        self.disposition
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn reason(&self) -> TransitionReason {
        self.reason
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserveStatus {
    Observed,
    Replayed,
}

/// Result of `observe`: the transition record plus whether it is new or a replay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObserveResult {
    pub status: ObserveStatus,
    #[serde(flatten)]
    pub transition: PressureTransition,
}

/// Controller state as the JS `snapshot()` reports it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerSnapshot {
    schema_version: u8,
    policy_revision: String,
    disposition: Disposition,
    minimum_sequence: u64,
    last_observation: Option<PressureObservation>,
    last_transition: Option<PressureTransition>,
}

impl ControllerSnapshot {
    pub fn disposition(&self) -> Disposition {
        self.disposition
    }

    pub fn minimum_sequence(&self) -> u64 {
        self.minimum_sequence
    }

    pub fn last_observation(&self) -> Option<&PressureObservation> {
        self.last_observation.as_ref()
    }

    pub fn last_transition(&self) -> Option<&PressureTransition> {
        self.last_transition.as_ref()
    }
}

#[derive(Clone, Debug)]
pub struct ContextPressureController {
    policy: ContextPressurePolicy,
    disposition: Disposition,
    minimum_sequence: u64,
    last_observation: Option<PressureObservation>,
    last_transition: Option<PressureTransition>,
}

impl ContextPressureController {
    /// The JS constructor over its options object `{ policy, initialDisposition?,
    /// minimumSequence? }`. Checks run in the JS order: disposition, floor, policy.
    pub fn from_options(options: &Value) -> Result<Self, PressureError> {
        let map = fields::record(Some(options), "context pressure controller options")?;
        let initial = match map.get("initialDisposition") {
            None => Disposition::Comfortable,
            Some(value) => value
                .as_str()
                .and_then(Disposition::parse)
                .ok_or_else(unsupported_initial_disposition)?,
        };
        let minimum_sequence = match map.get("minimumSequence") {
            None => 0,
            Some(value) => js::safe_integer(value)
                .filter(|number| *number >= 0)
                .ok_or_else(invalid_minimum_sequence)? as u64,
        };
        let policy = ContextPressurePolicy::from_value(map.get("policy").unwrap_or(&Value::Null))?;
        Ok(Self::assemble(policy, initial, minimum_sequence))
    }

    /// Typed constructor. `minimum_sequence` must be a safe integer, as in the JS.
    pub fn new(
        policy: ContextPressurePolicy,
        initial_disposition: Disposition,
        minimum_sequence: u64,
    ) -> Result<Self, PressureError> {
        if minimum_sequence > js::MAX_SAFE_INTEGER as u64 {
            return Err(invalid_minimum_sequence());
        }
        Ok(Self::assemble(
            policy,
            initial_disposition,
            minimum_sequence,
        ))
    }

    fn assemble(
        policy: ContextPressurePolicy,
        disposition: Disposition,
        minimum_sequence: u64,
    ) -> Self {
        Self {
            policy,
            disposition,
            minimum_sequence,
            last_observation: None,
            last_transition: None,
        }
    }

    pub fn policy(&self) -> &ContextPressurePolicy {
        &self.policy
    }

    pub fn snapshot(&self) -> ControllerSnapshot {
        ControllerSnapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            policy_revision: self.policy.policy_revision().to_owned(),
            disposition: self.disposition,
            minimum_sequence: self.minimum_sequence,
            last_observation: self.last_observation.clone(),
            last_transition: self.last_transition.clone(),
        }
    }

    /// `observe(value)`: validates the observation, rejects reuse and out-of-order sequences,
    /// replays an exact repeat of the last observation, and otherwise applies hysteresis. A
    /// rejected observation leaves the state unchanged.
    pub fn observe(&mut self, value: &Value) -> Result<ObserveResult, PressureError> {
        let observation = PressureObservation::from_value(value)?;
        if let (Some(last), Some(transition)) = (&self.last_observation, &self.last_transition)
            && last.observation_id() == observation.observation_id()
        {
            if *last != observation {
                return Err(PressureError::new(
                    ErrorKind::ObservationIdReused,
                    "context pressure observation id was reused for different evidence",
                ));
            }
            return Ok(ObserveResult {
                status: ObserveStatus::Replayed,
                transition: transition.clone(),
            });
        }
        match &self.last_observation {
            Some(last) if observation.sequence() <= last.sequence() => {
                return Err(PressureError::new(
                    ErrorKind::SequenceNotMonotonic,
                    "context pressure observation sequence must increase monotonically",
                ));
            }
            None if observation.sequence() <= self.minimum_sequence => {
                return Err(PressureError::new(
                    ErrorKind::SequenceBelowFloor,
                    "context pressure observation sequence must exceed the recovered floor",
                ));
            }
            _ => {}
        }
        let previous = self.disposition;
        let pressure = observation.pressure_basis_points();
        let disposition = self.policy.next_disposition(previous, pressure);
        let transition = PressureTransition {
            schema_version: TRANSITION_SCHEMA_VERSION,
            policy_revision: self.policy.policy_revision().to_owned(),
            observation: observation.clone(),
            previous_disposition: previous,
            disposition,
            changed: disposition != previous,
            reason: self.reason(previous, disposition, pressure),
        };
        self.disposition = disposition;
        self.last_observation = Some(observation);
        self.last_transition = Some(transition.clone());
        Ok(ObserveResult {
            status: ObserveStatus::Observed,
            transition,
        })
    }

    fn reason(
        &self,
        previous: Disposition,
        current: Disposition,
        pressure: u16,
    ) -> TransitionReason {
        if previous != current {
            return TransitionReason::ThresholdCrossed;
        }
        match self.policy.entry_threshold(previous) {
            None => TransitionReason::BelowEntryThreshold,
            Some(enter) if pressure < enter => TransitionReason::HysteresisHeld,
            Some(_) => TransitionReason::WithinDisposition,
        }
    }
}

fn unsupported_initial_disposition() -> PressureError {
    PressureError::new(
        ErrorKind::UnsupportedInitialDisposition,
        "context pressure initial disposition is unsupported",
    )
}

fn invalid_minimum_sequence() -> PressureError {
    PressureError::new(
        ErrorKind::InvalidMinimumSequence,
        "context pressure minimum sequence must be a non-negative safe integer",
    )
}
