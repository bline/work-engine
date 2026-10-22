//! `TokenUsagePressureProjector`: turns a lifecycle evidence snapshot's latest token-usage
//! observation into a pressure observation, or into `unavailable` evidence.

use crate::error::{ErrorKind, PressureError};
use crate::observation::PressureObservation;
use crate::profile::{Rounding, Saturation, TokenUsagePressureProfile, UsageField, WindowField};
use crate::{fields, js, timestamp};
use serde::Serialize;
use serde_json::{Value, json};

const OBSERVATION_SOURCE: &str = "codex_app_server:last.totalTokens/modelContextWindow";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    TokenUsageMissing,
    ContextWindowMissing,
}

/// Projected pressure with the measurements it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedPressure {
    measured_tokens: u64,
    reported_input_tokens: u64,
    context_window: u64,
    profile_revision: String,
    observation: PressureObservation,
}

impl ProjectedPressure {
    pub fn measured_tokens(&self) -> u64 {
        self.measured_tokens
    }

    pub fn reported_input_tokens(&self) -> u64 {
        self.reported_input_tokens
    }

    pub fn context_window(&self) -> u64 {
        self.context_window
    }

    pub fn profile_revision(&self) -> &str {
        &self.profile_revision
    }

    pub fn observation(&self) -> &PressureObservation {
        &self.observation
    }
}

/// Projector output. The wire form is tagged by `status`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PressureProjection {
    Unavailable { reason: UnavailableReason },
    Projected(Box<ProjectedPressure>),
}

struct UsageReadings<'a> {
    measured: Option<&'a Value>,
    reported_input: Option<&'a Value>,
    window: Option<&'a Value>,
}

#[derive(Clone, Debug)]
pub struct TokenUsagePressureProjector {
    profile: TokenUsagePressureProfile,
}

impl TokenUsagePressureProjector {
    /// Validates the profile as `validateTokenUsagePressureProfile` does.
    pub fn new(profile: &Value) -> Result<Self, PressureError> {
        Ok(Self {
            profile: TokenUsagePressureProfile::from_value(profile)?,
        })
    }

    pub fn profile(&self) -> &TokenUsagePressureProfile {
        &self.profile
    }

    /// `project(lifecycleSnapshot)`.
    pub fn project(&self, snapshot: &Value) -> Result<PressureProjection, PressureError> {
        let snapshot_map = fields::record(Some(snapshot), "lifecycle evidence snapshot")?;
        let usage_value = snapshot_map.get("latestTokenUsage");
        if matches!(usage_value, Some(Value::Null)) {
            return Ok(PressureProjection::Unavailable {
                reason: UnavailableReason::TokenUsageMissing,
            });
        }
        let usage = fields::record(usage_value, "latest token usage observation")?;
        let readings = self.read_usage(usage);
        let non_negative =
            |value: Option<&Value>, message: &str| match value.and_then(js::safe_integer) {
                Some(number) if number >= 0 => Ok(number as u64),
                _ => Err(PressureError::new(ErrorKind::InvalidTokenCount, message)),
            };
        let measured = non_negative(
            readings.measured,
            "latest token usage last.totalTokens must be a non-negative safe integer",
        )?;
        let reported = non_negative(
            readings.reported_input,
            "latest token usage last.inputTokens must be a non-negative safe integer",
        )?;
        let window = match readings.window.and_then(js::safe_integer) {
            Some(number) if number >= 1 => number as u64,
            _ => {
                return Ok(PressureProjection::Unavailable {
                    reason: UnavailableReason::ContextWindowMissing,
                });
            }
        };
        let sequence = match usage.get("sequence").and_then(js::safe_integer) {
            Some(number) if number >= 1 => number as u64,
            _ => {
                return Err(PressureError::new(
                    ErrorKind::InvalidUsageSequence,
                    "latest token usage sequence must be a positive safe integer",
                ));
            }
        };
        let observed_at = match usage.get("observedAt") {
            Some(Value::String(text)) if timestamp::is_es_iso_timestamp(text) => text.clone(),
            _ => {
                return Err(PressureError::new(
                    ErrorKind::InvalidUsageTimestamp,
                    "latest token usage observedAt must be an ISO timestamp",
                ));
            }
        };
        // Rust-only strictness: the JS hashes and interpolates whatever these hold.
        let thread_id = match snapshot_map.get("threadId") {
            Some(Value::String(text)) => text.as_str(),
            _ => {
                return Err(PressureError::new(
                    ErrorKind::InvalidSnapshotField,
                    "lifecycle evidence snapshot threadId must be a string",
                ));
            }
        };
        let Some(lifecycle_schema_version) = snapshot_map.get("schemaVersion") else {
            return Err(PressureError::new(
                ErrorKind::InvalidSnapshotField,
                "lifecycle evidence snapshot schemaVersion must be present",
            ));
        };
        let source_revision = js::revision(&json!({
            "profileRevision": self.profile.profile_revision(),
            "lifecycleSchemaVersion": lifecycle_schema_version,
            "threadId": thread_id,
            "observation": usage_value,
        }));
        let observation = PressureObservation::new(
            format!("token-usage:{thread_id}:{sequence}"),
            sequence,
            observed_at,
            self.pressure(measured, window),
            OBSERVATION_SOURCE.to_owned(),
            source_revision,
        );
        Ok(PressureProjection::Projected(Box::new(ProjectedPressure {
            measured_tokens: measured,
            reported_input_tokens: reported,
            context_window: window,
            profile_revision: self.profile.profile_revision().to_owned(),
            observation,
        })))
    }

    /// Reads the profile's usage and window fields: `usage.details?.last?.totalTokens`,
    /// `usage.details?.last?.inputTokens` and `usage.details?.modelContextWindow`.
    fn read_usage<'a>(&self, usage: &'a serde_json::Map<String, Value>) -> UsageReadings<'a> {
        let details = usage.get("details").and_then(Value::as_object);
        let last = details
            .and_then(|map| map.get("last"))
            .and_then(Value::as_object);
        UsageReadings {
            measured: match self.profile.usage_field() {
                UsageField::LastTotalTokens => last.and_then(|map| map.get("totalTokens")),
            },
            reported_input: last.and_then(|map| map.get("inputTokens")),
            window: match self.profile.window_field() {
                WindowField::ModelContextWindow => {
                    details.and_then(|map| map.get("modelContextWindow"))
                }
            },
        }
    }

    /// `min(10000, floor((measured / window) * 10000))` in IEEE double arithmetic, as the JS
    /// computes it; integer arithmetic would differ at some exact boundaries.
    fn pressure(&self, measured: u64, window: u64) -> u16 {
        let ratio = (measured as f64 / window as f64) * 10_000.0;
        let rounded = match self.profile.rounding() {
            Rounding::Floor => ratio.floor(),
        };
        match self.profile.saturation() {
            Saturation::Clamp10000 => rounded.min(10_000.0) as u16,
        }
    }
}
