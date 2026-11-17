//! Experimental review-contract delivery (PLAN-R2 vertical). Mints no authority.
//!
//! Three evidence stages stay separately attributable:
//! 1. **constructed**: [`FrozenDelivery`] is the deterministic serialization of one complete
//!    [`ReviewPreparation`] (model context, enforcement rows, provenance, every blocker) inside a
//!    labelled envelope that adds transport identity only (attempt, generation);
//! 2. **SDK-observed**: a [`NativeObservation`] is what the Pi SDK reported at its request/context
//!    boundary and in its own session history. [`BoundAttempt::verify`] compares it to the frozen
//!    bytes; a reported digest alone is never enough, the observed text is compared;
//! 3. **consumer-accepted**: with the non-default `fixture` feature, [`native`] runs the
//!    provider-disabled SDK and hands the exactly-observed result to the existing
//!    `consume_fixture_result`. The episode owner still checks revision, writer, subject and lineage.
//!
//! The joined entrypoint accepts one private [`binding::ExperimentBinding`] only (feature
//! `fixture`): it derives the preparation from the canonical candidate/request, checks the full
//! join with the declaration and episode before any native child exists, and consumes from the
//! same checked binding. [`FrozenDelivery`] is a transport byte type; freezing never establishes
//! canonical derivation.
//!
//! With `fixture`, the delivery gate ([`gate`]) refuses delivery before any child is spawned, and
//! again immediately before consumption, on a known realization incompatibility, a blocker kind
//! without a discharge, or a refused, mismatched or non-current fixture-domain organizational
//! admission (S-A) that the host obtains itself. The host derives the realization description from
//! what it runs. Admitted kinds are discharged by named S-A check ids; the mapping kinds remain
//! labelled fixture **assumptions** (`AssumedDischarge`), never admissions.
//!
//! Nothing here clears a preparation blocker outside the fixture runner, creates a production
//! admission, grants capability or accepts anything. Session/attempt bindings are transport correspondences, never grants. The fixture
//! child is behaviour-constrained fixture code, not an OS sandbox.
//!
//! Without the feature the native runner does not exist:
//!
#![cfg_attr(
    not(feature = "fixture"),
    doc = "```compile_fail,E0432\nuse work_engine_review_delivery::native::run_native;\n```\n"
)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use work_engine_review_preparation::ReviewPreparation;

pub mod attempt;
#[cfg(feature = "fixture")]
pub mod binding;
#[cfg(feature = "fixture")]
pub mod gate;
#[cfg(feature = "fixture")]
pub mod native;
pub mod wire;

pub const EXPERIMENT_LABEL: &str = "experimental_review_delivery_not_authority";

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Transport correspondence chosen by Rust for one attempt. Not a grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transport {
    pub attempt_id: String,
    pub generation: u64,
}

#[derive(Serialize)]
struct EnvelopeRef<'a> {
    label: &'static str,
    transport: &'a Transport,
    preparation: &'a ReviewPreparation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvelopeOwned {
    label: String,
    transport: Transport,
    preparation: ReviewPreparation,
}

/// Why a delivery was refused. Uncertain variants leave a possible send unestablished: they must
/// not advance any state and must not be replayed automatically.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DeliveryRefusal {
    #[error("envelope serialization does not round-trip to the same preparation")]
    Lossy,
    #[error("serialization failed: {0}")]
    Serialize(String),
    #[error("allocation reports active tools {0:?}; none are granted")]
    AllocationTools(Vec<String>),
    #[error("native session `{observed}` is not the session `{expected}` bound before the prompt")]
    SessionMismatch { expected: String, observed: String },
    #[error("attempt `{observed}` is not the bound attempt `{expected}`")]
    AttemptMismatch { expected: String, observed: String },
    #[error("generation {observed} is not the bound generation {expected}")]
    GenerationMismatch { expected: u64, observed: u64 },
    #[error("the SDK refused the prompt: {0}")]
    PromptRefused(String),
    #[error("provider/network entry observed (ordinary stream {ordinary_stream}, fetch {fetch})")]
    ProviderEntry { ordinary_stream: u64, fetch: u64 },
    #[error("expected exactly one request at the context boundary, observed {0}")]
    RequestCount(u64),
    #[error("observed contract bytes differ from the frozen contract: {0}")]
    ContractMismatch(&'static str),
    #[error("context structure: {0}")]
    Structure(String),
    #[error("toolset: {0}")]
    Toolset(String),
    #[error("native history: {0}")]
    History(String),
    #[error("final assistant text differs from the bound synthetic result")]
    ResultMismatch,
    /// Possible delivery without a complete native observation.
    #[error("uncertain: {0}")]
    Uncertain(&'static str),
    /// The native attempt was not a complete, clean, fully collected run. The first matching
    /// observation is only a candidate; success requires the whole attempt. A start frame may
    /// have been delivered, so this is never replayed automatically.
    #[error("incomplete native attempt: {0}")]
    Incomplete(String),
}

impl DeliveryRefusal {
    pub fn is_uncertain(&self) -> bool {
        matches!(
            self,
            DeliveryRefusal::Uncertain(_) | DeliveryRefusal::Incomplete(_)
        )
    }
}

/// The frozen, exact contract bytes for one attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenDelivery {
    transport: Transport,
    preparation: ReviewPreparation,
    contract: String,
    contract_sha256: String,
}

impl FrozenDelivery {
    /// Freezes the complete preparation. All source identity comes from the preparation; the
    /// envelope adds `transport` only. Refuses a serialization that does not round-trip.
    pub fn freeze(
        preparation: ReviewPreparation,
        transport: Transport,
    ) -> Result<Self, DeliveryRefusal> {
        let contract = serde_json::to_string(&EnvelopeRef {
            label: EXPERIMENT_LABEL,
            transport: &transport,
            preparation: &preparation,
        })
        .map_err(|e| DeliveryRefusal::Serialize(e.to_string()))?;
        let back: EnvelopeOwned = serde_json::from_str(&contract)
            .map_err(|e| DeliveryRefusal::Serialize(e.to_string()))?;
        if back.label != EXPERIMENT_LABEL
            || back.transport != transport
            || back.preparation != preparation
        {
            return Err(DeliveryRefusal::Lossy);
        }
        let contract_sha256 = sha256_hex(contract.as_bytes());
        Ok(Self {
            transport,
            preparation,
            contract,
            contract_sha256,
        })
    }
    pub fn contract(&self) -> &str {
        &self.contract
    }
    pub fn contract_sha256(&self) -> &str {
        &self.contract_sha256
    }
    pub fn transport(&self) -> &Transport {
        &self.transport
    }
    pub fn preparation(&self) -> &ReviewPreparation {
        &self.preparation
    }
}

/// What the SDK manager actually allocated before any prompt. The adapter emits exactly these
/// members (plus `type`, removed before decoding); like duplicate members (ORG-PV-WR7-001), an
/// unknown member means the frame is not the pinned producer's and the frame is invalid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allocation {
    pub session_id: String,
    pub sdk_version: String,
    pub node_version: String,
    pub model: Value,
    pub system_prompt: String,
    pub active_tools: Vec<String>,
    pub loader: Value,
    pub preamble: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Counters {
    pub ordinary_stream: u64,
    pub fetch: u64,
}

/// The SDK's own report, after the single prompt. Exactly the adapter's members; an unknown
/// member makes the frame invalid (see [`Allocation`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeObservation {
    pub session_id: String,
    pub attempt_id: String,
    pub generation: u64,
    pub request_count: u64,
    /// `{messages, tools}` (message 0 is the SDK system context) as seen by the stream function at the request boundary.
    pub context: Option<Value>,
    /// The SDK session's own message history.
    pub history: Vec<Value>,
    pub active_tools: Vec<String>,
    pub terminal_idle: bool,
    pub counters: Counters,
    pub prompt_error: Option<String>,
    pub system_prompt: String,
    pub loader_calls: Value,
}

/// What a verified, fully observed delivery established. Constructed from observation only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedDelivery {
    pub session_id: String,
    pub attempt_id: String,
    pub generation: u64,
    pub contract_sha256: String,
    /// The final assistant text read from the native history.
    pub result_text: String,
    pub effective_system_context_sha256: String,
}

/// A frozen contract bound to the one session the SDK allocated, before the prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundAttempt {
    frozen: FrozenDelivery,
    allocation: Allocation,
}

fn text_of(message: &Value, role: &str) -> Result<String, DeliveryRefusal> {
    let bad = |m: String| DeliveryRefusal::Structure(m);
    if message.get("role").and_then(Value::as_str) != Some(role) {
        return Err(bad(format!("expected role `{role}`")));
    }
    match message.get("content") {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Array(parts)) if parts.len() == 1 => {
            let part = &parts[0];
            if part.get("type").and_then(Value::as_str) != Some("text") {
                return Err(bad("content part is not text".into()));
            }
            part.get("text")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| bad("text part has no text".into()))
        }
        _ => Err(bad(format!("`{role}` content is not one text part"))),
    }
}

impl BoundAttempt {
    pub fn bind(frozen: FrozenDelivery, allocation: Allocation) -> Result<Self, DeliveryRefusal> {
        if !allocation.active_tools.is_empty() {
            return Err(DeliveryRefusal::AllocationTools(allocation.active_tools));
        }
        Ok(Self { frozen, allocation })
    }
    pub fn session_id(&self) -> &str {
        &self.allocation.session_id
    }
    pub fn frozen(&self) -> &FrozenDelivery {
        &self.frozen
    }
    pub fn allocation(&self) -> &Allocation {
        &self.allocation
    }

    /// Compares the SDK observation with the frozen contract and the bound identities.
    /// `expected_result` is the synthetic text Rust supplied for this attempt; the returned
    /// `result_text` is the text actually found in the native history.
    pub fn verify(
        &self,
        obs: &NativeObservation,
        expected_result: &str,
    ) -> Result<VerifiedDelivery, DeliveryRefusal> {
        if obs.session_id != self.allocation.session_id {
            return Err(DeliveryRefusal::SessionMismatch {
                expected: self.allocation.session_id.clone(),
                observed: obs.session_id.clone(),
            });
        }
        let t = self.frozen.transport();
        if obs.attempt_id != t.attempt_id {
            return Err(DeliveryRefusal::AttemptMismatch {
                expected: t.attempt_id.clone(),
                observed: obs.attempt_id.clone(),
            });
        }
        if obs.generation != t.generation {
            return Err(DeliveryRefusal::GenerationMismatch {
                expected: t.generation,
                observed: obs.generation,
            });
        }
        if let Some(error) = &obs.prompt_error {
            // A prompt the SDK refused before any request is a refusal, not a delivery.
            if obs.request_count == 0 {
                return Err(DeliveryRefusal::PromptRefused(error.clone()));
            }
            return Err(DeliveryRefusal::Uncertain(
                "the prompt reported an error after a request was made",
            ));
        }
        if obs.counters.ordinary_stream != 0 || obs.counters.fetch != 0 {
            return Err(DeliveryRefusal::ProviderEntry {
                ordinary_stream: obs.counters.ordinary_stream,
                fetch: obs.counters.fetch,
            });
        }
        if !obs.terminal_idle {
            return Err(DeliveryRefusal::Uncertain("the session is not idle"));
        }
        if obs.request_count != 1 {
            return Err(DeliveryRefusal::RequestCount(obs.request_count));
        }
        if !obs.active_tools.is_empty() {
            return Err(DeliveryRefusal::Toolset(format!(
                "active tools {:?}",
                obs.active_tools
            )));
        }
        let context = obs.context.as_ref().ok_or(DeliveryRefusal::Uncertain(
            "no request context was observed",
        ))?;
        match context.get("tools") {
            None | Some(Value::Null) => {}
            Some(Value::Array(tools)) if tools.is_empty() => {}
            Some(other) => {
                return Err(DeliveryRefusal::Toolset(format!(
                    "advertised tools {other}"
                )));
            }
        }
        // The pinned SDK presents its system context as message 0 with role `system` and named
        // sections; the contract is message 1. Allowed structure: exactly these two, in order.
        let messages = context
            .get("messages")
            .and_then(Value::as_array)
            .ok_or_else(|| DeliveryRefusal::Structure("no messages".into()))?;
        if messages.len() != 2 {
            return Err(DeliveryRefusal::Structure(format!(
                "{} messages at the request boundary, expected system then user",
                messages.len()
            )));
        }
        let system = &messages[0];
        if system.get("role").and_then(Value::as_str) != Some("system") {
            return Err(DeliveryRefusal::Structure(
                "message 0 is not the system context".into(),
            ));
        }
        if system.pointer("/sections/preamble").and_then(Value::as_str)
            != Some(self.allocation.preamble.as_str())
        {
            return Err(DeliveryRefusal::Structure(
                "system context does not carry the fixed fixture preamble".into(),
            ));
        }
        let delivered = text_of(&messages[1], "user")?;
        if delivered != self.frozen.contract() {
            return Err(DeliveryRefusal::ContractMismatch("request-boundary text"));
        }
        if sha256_hex(delivered.as_bytes()) != self.frozen.contract_sha256() {
            return Err(DeliveryRefusal::ContractMismatch("digest"));
        }
        if obs.history.len() != 3 {
            return Err(DeliveryRefusal::History(format!(
                "{} history entries, expected system, user, assistant",
                obs.history.len()
            )));
        }
        // The SDK's own history holds the same system context it sent, then the contract and the
        // assistant reply; none may differ from what was observed at the request boundary.
        let (h_role, h_sections) = (obs.history[0].get("role"), obs.history[0].get("sections"));
        if h_role != system.get("role") || h_sections != system.get("sections") {
            return Err(DeliveryRefusal::History(
                "history system context differs from the request boundary's".into(),
            ));
        }
        if text_of(&obs.history[1], "user")? != self.frozen.contract() {
            return Err(DeliveryRefusal::ContractMismatch("history user text"));
        }
        let assistant = &obs.history[2];
        if assistant.get("stopReason").and_then(Value::as_str) != Some("stop") {
            return Err(DeliveryRefusal::History(
                "assistant message did not stop normally".into(),
            ));
        }
        let result_text = text_of(assistant, "assistant")?;
        if result_text != expected_result {
            return Err(DeliveryRefusal::ResultMismatch);
        }
        Ok(VerifiedDelivery {
            session_id: obs.session_id.clone(),
            attempt_id: obs.attempt_id.clone(),
            generation: obs.generation,
            contract_sha256: self.frozen.contract_sha256().to_owned(),
            result_text,
            effective_system_context_sha256: sha256_hex(system.to_string().as_bytes()),
        })
    }
}

/// Delivery-owned interpretation of the complete raw stdout of one native attempt. Decoding never
/// replaces the raw bytes: this reads them, it does not own them. A frame is one `\n`-terminated
/// line; bytes after the last newline are an incomplete trailing frame and a defect.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamInterpretation {
    /// One entry per line in order: `allocated`, `observation`, `error`, `other:<type>`, `invalid`.
    pub kinds: Vec<String>,
    pub allocation: Option<Allocation>,
    pub observation: Option<NativeObservation>,
    pub error_message: Option<String>,
    /// Every invalid UTF-8/JSON/typed/empty/incomplete frame, in order.
    pub defects: Vec<String>,
}

impl StreamInterpretation {
    /// The only permitted successful protocol: exactly allocation then one observation, every
    /// frame complete and valid, nothing before, between or after.
    pub fn permitted_success(&self) -> bool {
        self.defects.is_empty()
            && self.kinds == ["allocated", "observation"]
            && self.allocation.is_some()
            && self.observation.is_some()
    }
}

pub fn interpret_stream(raw: &[u8]) -> StreamInterpretation {
    let mut out = StreamInterpretation {
        kinds: vec![],
        allocation: None,
        observation: None,
        error_message: None,
        defects: vec![],
    };
    let mut rest = raw;
    let mut index = 0usize;
    while !rest.is_empty() {
        let (line, complete) = match rest.iter().position(|b| *b == b'\n') {
            Some(i) => {
                let line = &rest[..i];
                rest = &rest[i + 1..];
                (line, true)
            }
            None => {
                let line = rest;
                rest = &[];
                (line, false)
            }
        };
        if !complete {
            out.defects.push(format!(
                "frame {index}: incomplete trailing frame ({} bytes, no newline)",
                line.len()
            ));
        }
        index += 1;
        let invalid = |out: &mut StreamInterpretation, why: String| {
            out.kinds.push("invalid".into());
            out.defects
                .push(format!("frame {}: {why}", out.kinds.len() - 1));
        };
        let Ok(text) = std::str::from_utf8(line) else {
            invalid(&mut out, "invalid UTF-8".into());
            continue;
        };
        if text.trim().is_empty() {
            invalid(&mut out, "empty frame".into());
            continue;
        }
        let mut value = match wire::parse(line) {
            Ok(v) => v,
            Err(e) => {
                invalid(&mut out, format!("not JSON: {e}"));
                continue;
            }
        };
        let kind = value.get("type").and_then(Value::as_str).map(str::to_owned);
        match kind.as_deref() {
            Some("allocated") => {
                value.as_object_mut().map(|o| o.remove("type"));
                match serde_json::from_value::<Allocation>(value) {
                    Ok(a) => {
                        out.kinds.push("allocated".into());
                        if out.allocation.is_none() {
                            out.allocation = Some(a);
                        }
                    }
                    Err(e) => invalid(&mut out, format!("allocation malformed: {e}")),
                }
            }
            Some("observation") => {
                value.as_object_mut().map(|o| o.remove("type"));
                match serde_json::from_value::<NativeObservation>(value) {
                    Ok(o) => {
                        out.kinds.push("observation".into());
                        if out.observation.is_none() {
                            out.observation = Some(o);
                        }
                    }
                    Err(e) => invalid(&mut out, format!("observation malformed: {e}")),
                }
            }
            Some("error") => {
                out.kinds.push("error".into());
                if out.error_message.is_none() {
                    out.error_message = Some(text.to_owned());
                }
            }
            Some(other) => out.kinds.push(format!("other:{other}")),
            None => invalid(&mut out, "no frame type".into()),
        }
    }
    out
}
