//! Bounded Pi SDK context estimates as pressure evidence, with no lifecycle authority.
use context_pressure::{ContextPressureController, ContextPressurePolicy, Disposition};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Caller-supplied descriptive binding, never a task or replacement-session grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub source: String,
    pub session: String,
    pub generation: String,
    pub model: String,
    pub window: u64,
}

/// `tokens` is the SDK estimate, not exact provider input. `None` is unavailable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub identity: Identity,
    pub observation_id: String,
    pub sequence: u64,
    pub observed_at: String,
    pub observed_ms: u64,
    pub tokens: Option<u64>,
    pub provenance: String,
}

/// A single identity's observation cursor and existing pressure controller.
/// The caller supplies a current binding and a clock; neither is inferred from a reading.
pub struct Observer {
    identity: Identity,
    max_age_ms: u64,
    controller: ContextPressureController,
    last: Option<(Observation, Value)>,
}

impl Observer {
    pub fn new(
        identity: Identity,
        max_age_ms: u64,
        policy: &Value,
        initial: Disposition,
    ) -> Result<Self, String> {
        if [
            &identity.source,
            &identity.session,
            &identity.generation,
            &identity.model,
        ]
        .iter()
        .any(|v| v.trim().is_empty())
            || identity.window == 0
        {
            return Err("missing identity or window".into());
        }
        let policy = ContextPressurePolicy::from_value(policy).map_err(|e| e.to_string())?;
        let controller =
            ContextPressureController::new(policy, initial, 0).map_err(|e| e.to_string())?;
        Ok(Self {
            identity,
            max_age_ms,
            controller,
            last: None,
        })
    }

    /// Rejects mismatched bindings, stale/future data and out-of-order source observations.
    /// Unavailable evidence advances source ordering but never changes the controller.
    pub fn observe(&mut self, input: Observation, now_ms: u64) -> Result<Value, String> {
        if input.identity != self.identity {
            return Err("identity mismatch".into());
        }
        if input.observation_id.trim().is_empty()
            || input.provenance.trim().is_empty()
            || input.sequence == 0
            || input.sequence > 9_007_199_254_740_991
        {
            return Err("invalid observation identity or sequence".into());
        }
        let timestamp = time::OffsetDateTime::parse(
            &input.observed_at,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|_| "invalid observation timestamp")?;
        if timestamp.unix_timestamp_nanos() != i128::from(input.observed_ms) * 1_000_000 {
            return Err("observation timestamp mismatch".into());
        }
        if input.observed_ms > now_ms || now_ms - input.observed_ms > self.max_age_ms {
            return Err("stale or future observation".into());
        }
        if let Some((last, result)) = &self.last {
            if input.observation_id == last.observation_id {
                return if &input == last {
                    Ok(result.clone())
                } else {
                    Err("observation id reused".into())
                };
            }
            if input.sequence <= last.sequence {
                return Err("source sequence not monotonic".into());
            }
        }
        let result = if let Some(tokens) = input.tokens {
            if tokens > 9_007_199_254_740_991 || input.identity.window > 9_007_199_254_740_991 {
                return Err("unsafe numeric estimate or window".into());
            }
            let basis_points = ((u128::from(tokens) * 10_000 / u128::from(input.identity.window))
                .min(10_000)) as u16;
            let encoded = serde_json::to_vec(&input).map_err(|e| e.to_string())?;
            let revision = format!("sha256:{:x}", Sha256::digest(encoded));
            let pressure = json!({"schemaVersion":1,"observationId":input.observation_id,"sequence":input.sequence,"observedAt":input.observed_at,"pressureBasisPoints":basis_points,"source":input.identity.source,"sourceRevision":revision});
            let transition = self
                .controller
                .observe(&pressure)
                .map_err(|e| e.to_string())?;
            json!({"availability":"available","metric":"pi_sdk_context_estimate","input":input,"pressure":transition})
        } else {
            json!({"availability":"unavailable","metric":"pi_sdk_context_estimate","input":input})
        };
        self.last = Some((input, result.clone()));
        Ok(result)
    }
}
