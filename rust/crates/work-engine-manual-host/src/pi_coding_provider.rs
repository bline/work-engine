//! The hosted provider lane: Rust owns admission, reservations and the
//! attempt ledger. Provider access never enters the container.
use super::*;
/// Host-relayed response stream (`openResponse`) imported by the hosted receiver.
pub const STREAM_ADAPTER: &[u8] = include_bytes!("../receiver/pi-native-tools.mjs");
const CATALOG: &str = "node_modules/@earendil-works/pi-ai/dist/providers/data/openai-codex.json";
/// Pi's Anthropic catalog: the prices and limits of the Claude models, which
/// the `claude-sdk` extension takes from it too.
const ANTHROPIC_CATALOG: &str =
    "node_modules/@earendil-works/pi-ai/dist/providers/data/anthropic.json";
/// Claude models admitted on the Messages routes (operator, 2026-11-18).
pub const CLAUDE_MODELS: [&str; 1] = ["claude-haiku-5-5"];
/// The Claude Code binary a `claude-sdk` runtime must carry (C1(vii)): its
/// manifest entry and digest.
pub const CLAUDE_CODE_ENTRY: &str = "node_modules/@anthropic-ai/claude-agent-sdk-linux-x64/claude";
pub const CLAUDE_CODE_SHA256: &str =
    "0dc4f4a9a165fc60746d7ae8489712e5fc352f1e7ad4eb22e7141bd0812eb0cf";
/// The Pi provider a plan runs when it names none.
pub const DEFAULT_PROVIDER: &str = "openai-codex";
fn default_provider() -> String {
    DEFAULT_PROVIDER.into()
}
/// Analysis budget ceiling for the Messages family: full catalog reservations
/// of a 1M-context Claude model (1 320 001 µUSD each, one-hour cache writes at
/// the upper tier) for the profile's 32-request maximum.
const MESSAGES_ANALYSIS_BUDGET: u64 = 43_000_000;
/// Public Pi configuration staged read-only for the container. Provider
/// access is not a lane input: the Host's transport holds it.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaneInputs {
    pub pi_config: Tree,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderPlan {
    pub lane: LaneInputs,
    pub task: Artifact,
    pub catalog: Artifact,
    pub stream_adapter: Artifact,
    /// Route table; its digest is bound by the runtime admission.
    pub hosted: super::hosted::HostedRoutes,
    /// The Pi provider the receiver runs: `openai-codex` (Responses family)
    /// or `claude-sdk` (Claude Code in the container; Messages family, only
    /// on the analysis or the lifecycle relay profile). A `claude-sdk` plan may route to the
    /// subscription and to the API-key Messages endpoint: both receive the
    /// same request body.
    #[serde(default = "default_provider")]
    pub provider: String,
    pub model: String,
    pub max_requests: u32,
    pub budget_microusd: u64,
    pub max_context_bytes: u64,
    /// Failover policy and admitted state observation; digests bound by the
    /// runtime admission. Absent: refusal-only failover.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failover: Option<FailoverInputs>,
    /// Digest of the classification table the Host classifies non-success
    /// heads with (`coding::classification`); bound by the runtime
    /// admission. Absent: no classification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification_table_sha256: Option<String>,
}
/// Operator failover inputs of a hosted plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailoverInputs {
    /// `work-engine.hosted-failover-policy.v1`.
    pub policy: Artifact,
    /// The route availability view's output (v2) the job starts from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<Artifact>,
}
impl std::fmt::Debug for ProviderPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderPlan")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("max_requests", &self.max_requests)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reservation {
    pub reserved_microusd: u64,
    pub settled_microusd: Option<u64>,
    pub backend_model: Option<String>,
    pub http_status: Option<u64>,
    pub usage: Option<Value>,
    #[serde(default)]
    pub raw_terminal: Option<Value>,
    /// Upstream attempts under this logical request, in order. Empty for
    /// single-attempt profiles, whose request-level fields stay authoritative.
    /// When present, these records own reservation state; the request-level
    /// `settled_microusd` mirrors a settled completion for existing checks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attempts: Vec<AttemptRecord>,
    /// Single-attempt profiles: raw-usage cost of a completion that exceeded
    /// the admitted bound (saturating at the catalog cost cap). Held as
    /// `max(reservation, raw)`; the request stays unsettled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_microusd: Option<u64>,
    /// Hosted: why the Host refused to dispatch this request before any
    /// attempt was reserved (no byte was written; settled at 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_dispatched: Option<String>,
    /// Hosted: the failover gate paused this request; no attempt followed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failover_pause: Option<super::failover::FailoverPause>,
}
/// Financial disposition of one attempt, applied only by this ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptDisposition {
    /// Reserved and persisted; no outcome recorded yet.
    Pending,
    /// Definitive head refusal, nothing relayed, body drained and screened.
    /// Reservation released.
    Refused,
    /// Head-phase cancel confirmed before any upstream byte was written.
    /// Reservation released.
    NotDispatched,
    /// Relayed to its framed end; full reservation kept until raw-usage
    /// settlement.
    Completed,
    /// Anything else. Full reservation kept; blocks the request.
    Uncertain,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptRecord {
    /// Per-request ordinal, matching the admission key
    /// `provider:{request}:{attempt}`.
    pub attempt: u64,
    pub route: String,
    pub payer: String,
    pub reserved_microusd: u64,
    pub disposition: AttemptDisposition,
    #[serde(default)]
    pub http_status: Option<u16>,
    /// Digest of the screened bytes forwarded to the consumer.
    #[serde(default)]
    pub relay_sha256: Option<String>,
    #[serde(default)]
    pub refusal_body_sha256: Option<String>,
    #[serde(default)]
    pub settled_microusd: Option<u64>,
    #[serde(default)]
    pub reason: Option<String>,
    /// Hosted: the lifecycle gate admitted this attempt under the current
    /// grant; persisted before any upstream byte is written.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub gate_entered: bool,
    /// Hosted: the admitted dispatch deadline (ms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<i64>,
    /// Hosted: digest of the admitted route table the gate policy mirrors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routes_sha256: Option<String>,
    /// Hosted relays: whether the provider's framed end was observed. A
    /// completion settled on the terminal event alone records `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framed_end_observed: Option<bool>,
    /// Hosted: SHA-256 of the exact request body this attempt dispatched,
    /// retained create-only (`coding-hosted-{request}.body.json`) before the
    /// request's first attempt and persisted here before any upstream byte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_body_sha256: Option<String>,
    /// Raw-usage cost of a completion that exceeded the admitted bound
    /// (saturating at the catalog cost cap). Held as `max(reservation, raw)`;
    /// the attempt stays unsettled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_microusd: Option<u64>,
    /// Hosted: the bound failover decision this attempt was sent under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failover: Option<super::failover::FailoverDecision>,
    /// Hosted: the classification table this attempt's non-success heads
    /// were classified under (`coding::classification`); set before any
    /// upstream byte while classification is active for the route.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification_table_sha256: Option<String>,
    /// Hosted: what the head hold did with this attempt's head (held,
    /// refused, relayed, or a refusal's class as evidence).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_hold: Option<crate::pi_stream::HoldRecord>,
}
/// Observed facts about one attempt, from which the ledger derives the
/// disposition. Callers report evidence; they never choose a release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttemptEvidence {
    /// The response head was not relayed.
    HeadRefusal {
        status: u16,
        relayed_bytes: u64,
        drained_and_screened: bool,
        body_sha256: String,
        /// A held head refused on its class under the classification table
        /// (a status outside the definitive set); `None` for a refusal by
        /// status.
        classified: Option<TableRefusal>,
    },
    /// A head-phase cancel arrived before dispatch.
    NotDispatched { confirmed_before_write: bool },
    /// The response was relayed (fully or partly) to the consumer.
    Relayed {
        status: u16,
        upstream_end: bool,
        interrupted: bool,
        relay_sha256: String,
        /// Provider failure event (`response.failed`/`error`) in the relayed
        /// bytes; such a stream is never a completion.
        provider_failure: Option<String>,
        /// A screened `response.completed` was relayed, nothing but SSE
        /// comments or blank lines followed it (`pi_stream::terminal_is_last`),
        /// and every provider byte after it was read and kept: the framed end
        /// was observed or the release drain ran, and no held-back byte was
        /// dropped.
        terminal_last: bool,
        /// Read or framing error that ended the release drain.
        drain_error: Option<String>,
    },
    /// No trustworthy observation.
    Unknown { reason: String },
}
/// The classification a held head was refused on: the table's digest and
/// the class name it gave the head and its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRefusal {
    pub table_sha256: String,
    pub class: String,
}
impl TableRefusal {
    /// Releases only under the linked table and for a failover class.
    fn definitive(&self) -> bool {
        self.table_sha256 == hosted_route_availability::provisional::table_sha256()
            && super::classification::failover_class_name(&self.class)
    }
}
impl AttemptEvidence {
    pub fn from_relay(relay: &crate::pi_stream::Relay) -> Self {
        Self::from_relay_for(crate::pi_stream::Wire::Responses, relay)
    }
    pub fn from_relay_for(wire: crate::pi_stream::Wire, relay: &crate::pi_stream::Relay) -> Self {
        use crate::pi_stream::Outcome;
        let usage = crate::pi_stream::usage_for(wire, &relay.forwarded);
        Self::Relayed {
            status: relay.status,
            upstream_end: matches!(
                relay.outcome,
                Outcome::Completed | Outcome::Cancelled { upstream_end: true }
            ),
            interrupted: matches!(relay.outcome, Outcome::Interrupted(_)),
            relay_sha256: relay.sha256(),
            provider_failure: usage["failed"].as_str().map(String::from),
            terminal_last: usage["completed"] == true
                && crate::pi_stream::terminal_is_last_for(wire, &relay.forwarded, &relay.unforwarded)
                && relay.held_dropped == 0
                // A drain that ended on a read or framing error may have
                // dropped provider bytes (M4): never clean.
                && relay.drain_error.is_none()
                && (relay.drained
                    || matches!(
                        relay.outcome,
                        Outcome::Completed | Outcome::Cancelled { upstream_end: true }
                    )),
            drain_error: relay.drain_error.map(String::from),
        }
    }
}
impl Reservation {
    /// Money this request currently holds against the budget.
    pub fn charged_microusd(&self) -> u64 {
        if self.attempts.is_empty() {
            return self
                .settled_microusd
                .unwrap_or(self.reserved_microusd.max(self.raw_microusd.unwrap_or(0)));
        }
        self.attempts
            .iter()
            .map(|a| match a.disposition {
                AttemptDisposition::Refused | AttemptDisposition::NotDispatched => 0,
                AttemptDisposition::Completed => a
                    .settled_microusd
                    .unwrap_or(a.reserved_microusd.max(a.raw_microusd.unwrap_or(0))),
                AttemptDisposition::Pending | AttemptDisposition::Uncertain => a.reserved_microusd,
            })
            .fold(0u64, u64::saturating_add)
    }
    /// Whether this request still blocks new grants.
    pub fn unsettled(&self) -> bool {
        if self.attempts.is_empty() {
            return self.settled_microusd.is_none();
        }
        self.attempts.iter().any(|a| match a.disposition {
            AttemptDisposition::Refused | AttemptDisposition::NotDispatched => false,
            AttemptDisposition::Completed => a.settled_microusd.is_none(),
            AttemptDisposition::Pending | AttemptDisposition::Uncertain => true,
        })
    }
    /// Apply the release rule to the current pending attempt.
    pub fn settle_attempt(
        &mut self,
        attempt: u64,
        evidence: AttemptEvidence,
    ) -> Result<AttemptDisposition> {
        let a = match self.attempts.last_mut() {
            Some(a) if a.attempt == attempt && a.disposition == AttemptDisposition::Pending => a,
            _ => return refuse("attempt is not the pending attempt"),
        };
        let (disposition, reason) = match evidence {
            AttemptEvidence::HeadRefusal {
                status,
                relayed_bytes,
                drained_and_screened,
                body_sha256,
                classified,
            } => {
                a.http_status = Some(status);
                a.refusal_body_sha256 = Some(body_sha256);
                // Definitive by status, or by the linked table's failover
                // class; either way nothing relayed, drained and screened.
                if (crate::pi_route::refusal_status(status)
                    || classified.as_ref().is_some_and(TableRefusal::definitive))
                    && relayed_bytes == 0
                    && drained_and_screened
                {
                    (AttemptDisposition::Refused, None)
                } else {
                    (
                        AttemptDisposition::Uncertain,
                        Some("refusal not definitive".to_string()),
                    )
                }
            }
            AttemptEvidence::NotDispatched {
                confirmed_before_write,
            } => {
                if confirmed_before_write {
                    (AttemptDisposition::NotDispatched, None)
                } else {
                    (
                        AttemptDisposition::Uncertain,
                        Some("cancel not confirmed before dispatch".to_string()),
                    )
                }
            }
            AttemptEvidence::Relayed {
                status,
                upstream_end,
                interrupted,
                relay_sha256,
                provider_failure,
                terminal_last,
                drain_error,
            } => {
                // Status 0: no response head was read.
                a.http_status = (status != 0).then_some(status);
                a.relay_sha256 = Some(relay_sha256);
                a.framed_end_observed = Some(upstream_end);
                if let Some(code) = provider_failure {
                    (
                        AttemptDisposition::Uncertain,
                        Some(format!("provider failure event in relayed stream: {code}")),
                    )
                } else if status == 200 && !interrupted && terminal_last && upstream_end {
                    (AttemptDisposition::Completed, None)
                } else if status == 200 && !interrupted && terminal_last {
                    // K-a: settle on the provider's terminal event; the
                    // missing framed end is an evidence limitation only.
                    (
                        AttemptDisposition::Completed,
                        Some(match drain_error {
                            Some(code) => format!(
                                "completed on terminal event; framed end not observed (drain ended: {code})"
                            ),
                            None => "completed on terminal event; framed end not observed".into(),
                        }),
                    )
                } else if status == 200 && !interrupted {
                    // Q-a: the tail rule applies with or without the framed end.
                    (
                        AttemptDisposition::Uncertain,
                        Some(
                            "no clean terminal event: no response.completed, provider bytes after it, or bytes after it not all read and kept"
                                .to_string(),
                        ),
                    )
                } else {
                    (
                        AttemptDisposition::Uncertain,
                        Some("relay incomplete or not successful".to_string()),
                    )
                }
            }
            AttemptEvidence::Unknown { reason } => (AttemptDisposition::Uncertain, Some(reason)),
        };
        a.disposition = disposition;
        a.reason = reason;
        Ok(disposition)
    }
    /// Journal a failover pause: the request ends without a further send.
    /// Allowed on a new request or after a refused last attempt; a request
    /// without attempts settles at 0.
    pub fn record_failover_pause(&mut self, pause: super::failover::FailoverPause) -> Result<()> {
        if self.failover_pause.is_some()
            || self.not_dispatched.is_some()
            || self.settled_microusd.is_some()
            || pause.action != "pause"
            || self
                .attempts
                .last()
                .is_some_and(|a| a.disposition != AttemptDisposition::Refused)
        {
            return refuse("failover pause outside an open request");
        }
        if self.attempts.is_empty() {
            self.settled_microusd = Some(0);
        }
        self.failover_pause = Some(pause);
        Ok(())
    }
    /// Record raw-usage settlement of the completed attempt.
    pub fn settle_completed(&mut self, attempt: u64, actual_microusd: u64) -> Result<()> {
        let a = match self.attempts.last_mut() {
            Some(a)
                if a.attempt == attempt
                    && a.disposition == AttemptDisposition::Completed
                    && a.settled_microusd.is_none() =>
            {
                a
            }
            _ => return refuse("attempt is not an unsettled completion"),
        };
        if actual_microusd > a.reserved_microusd {
            return refuse("attempt usage exceeds its reservation");
        }
        a.settled_microusd = Some(actual_microusd);
        self.settled_microusd = Some(self.charged_microusd());
        Ok(())
    }
}
/// Reserve the next attempt of request `id`. Attempt 1 needs a new request
/// with no attempts; attempt k>1 needs attempt k-1 settled as `Refused`.
/// Every other request must be settled, and the reservation must fit the
/// budget. The caller persists the ledger before dispatching the attempt.
pub fn reserve_attempt(
    requests: &mut BTreeMap<u64, Reservation>,
    id: u64,
    route: &str,
    payer: &str,
    reserve_microusd: u64,
    budget_microusd: u64,
    max_attempts: u32,
) -> Result<u64> {
    if route.is_empty() || payer.is_empty() {
        return refuse("attempt route/payer missing");
    }
    if requests.iter().any(|(k, r)| *k != id && r.unsettled()) {
        return refuse("another provider request is unsettled");
    }
    let r = requests
        .get(&id)
        .ok_or_else(|| Error::Refused("attempt before provider request".into()))?;
    if r.settled_microusd.is_some() {
        return refuse("attempt after request settlement");
    }
    if r.failover_pause.is_some() {
        return refuse("attempt after a failover pause");
    }
    let next = match r.attempts.last() {
        None => 1,
        Some(a) if a.disposition == AttemptDisposition::Refused => a.attempt + 1,
        Some(_) => return refuse("previous attempt not refused"),
    };
    if next > u64::from(max_attempts) {
        return refuse("attempt limit reached");
    }
    // The target request is held only through its attempt records, so its
    // request-level reservation is not counted alongside attempt 1.
    let held: u64 = requests
        .iter()
        .map(|(k, r)| {
            if *k == id && r.attempts.is_empty() {
                0
            } else {
                r.charged_microusd()
            }
        })
        .sum();
    if held
        .checked_add(reserve_microusd)
        .is_none_or(|v| v > budget_microusd)
    {
        return refuse("provider budget exhausted");
    }
    let r = requests.get_mut(&id).expect("checked above");
    r.attempts.push(AttemptRecord {
        attempt: next,
        route: route.into(),
        payer: payer.into(),
        reserved_microusd: reserve_microusd,
        disposition: AttemptDisposition::Pending,
        http_status: None,
        relay_sha256: None,
        refusal_body_sha256: None,
        settled_microusd: None,
        reason: None,
        gate_entered: false,
        deadline_ms: None,
        routes_sha256: None,
        framed_end_observed: None,
        request_body_sha256: None,
        raw_microusd: None,
        failover: None,
        classification_table_sha256: None,
        head_hold: None,
    });
    Ok(next)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Admission {
    schema: String,
    job_id: String,
    binary_sha256: String,
    model: String,
    pi_config_manifest_sha256: String,
    task_sha256: String,
    catalog_sha256: String,
    max_requests: u32,
    budget_microusd: u64,
    allow_provider: bool,
    /// Digest of the hosted route table.
    hosted_routes_sha256: String,
    /// Digest of the failover policy, when the plan carries one.
    #[serde(default)]
    failover_policy_sha256: Option<String>,
    /// Digest of the admitted state observation, when the plan carries one.
    #[serde(default)]
    state_observation_sha256: Option<String>,
    /// Digest of the classification table, when the plan names one.
    #[serde(default)]
    classification_table_sha256: Option<String>,
    #[serde(default = "default_provider")]
    provider: String,
}
impl ProviderPlan {
    /// Wire family of the plan's provider; `None` for an unknown provider.
    pub fn wire(&self) -> Option<crate::pi_stream::Wire> {
        match self.provider.as_str() {
            "openai-codex" => Some(crate::pi_stream::Wire::Responses),
            "claude-sdk" => Some(crate::pi_stream::Wire::Messages),
            _ => None,
        }
    }
    pub(super) fn wire_checked(&self) -> Result<crate::pi_stream::Wire> {
        self.wire()
            .ok_or_else(|| Error::Refused("provider family unknown".into()))
    }
    /// The pinned catalog entry of the plan's model.
    pub(super) fn selected(&self) -> Result<Value> {
        let data: Value = serde_json::from_slice(&self.catalog.verify()?)?;
        let (key, provider, api, base) = match self.wire_checked()? {
            crate::pi_stream::Wire::Responses => (
                "openai-codex-responses",
                "openai-codex",
                "openai-codex-responses",
                "https://chatgpt.com/backend-api",
            ),
            crate::pi_stream::Wire::Messages => (
                "anthropic-messages",
                "anthropic",
                "anthropic-messages",
                "https://api.anthropic.com",
            ),
        };
        let selected = data[key][format!("chat:{}", self.model)].clone();
        if selected["id"] != self.model
            || selected["provider"] != provider
            || selected["api"] != api
            || selected["baseUrl"] != base
        {
            return refuse("exact catalog model absent");
        }
        Ok(selected)
    }
    /// Full catalog reservation of one request of the plan's model.
    pub(super) fn request_reservation(&self) -> Result<u64> {
        let model = self.selected()?;
        match self.wire_checked()? {
            crate::pi_stream::Wire::Responses => full_reservation(&model),
            crate::pi_stream::Wire::Messages => messages_full_reservation(&model),
        }
    }
    /// Whether the receiver's native model is the pinned catalog model: same
    /// id, limits and prices; provider and API as the plan's Pi provider
    /// registers them.
    pub(super) fn native_model_matches(&self, native: &Value) -> Result<bool> {
        let selected = self.selected()?;
        let (provider, api) = match self.provider.as_str() {
            "claude-sdk" => (json!("claude-sdk"), json!("claude-agent-sdk")),
            _ => (selected["provider"].clone(), selected["api"].clone()),
        };
        Ok(native["provider"] == provider
            && native["api"] == api
            && ["id", "contextWindow", "maxTokens", "cost"]
                .iter()
                .all(|k| native[*k] == selected[*k]))
    }
    /// Pure preparation check over the pinned public catalog; grants no entry
    /// and neither inspects nor opens native credential references.
    pub fn reservation_microusd(&self) -> Result<u64> {
        let reserve = self.request_reservation()?;
        let total = reserve
            .checked_mul(self.max_requests.into())
            .ok_or_else(|| Error::Refused("catalog reservation overflow".into()))?;
        if total > self.budget_microusd {
            return refuse("full catalog request reservations exceed runtime proxy budget");
        }
        Ok(total)
    }
    pub(super) fn validate(&self, job: &Job) -> Result<()> {
        let canonical = |a: &Artifact| -> Result<bool> {
            Ok(a.path.is_absolute()
                && fs::canonicalize(&a.path)? == a.path
                && !a.path.to_string_lossy().contains([',', ':', '\n', '\r']))
        };
        let analysis = job
            .coding
            .as_ref()
            .is_some_and(|p| super::is_analysis(&p.schema));
        let relay = job
            .coding
            .as_ref()
            .is_some_and(|p| super::is_relay(&p.schema));
        let wire = self.wire_checked()?;
        let messages = wire == crate::pi_stream::Wire::Messages;
        if job
            .coding
            .as_ref()
            .is_none_or(|p| !super::is_hosted(&p.schema))
            || (messages && !analysis)
            || !if messages {
                CLAUDE_MODELS.contains(&self.model.as_str())
            } else {
                self.model == "gpt-6-luna"
            }
            || !(1..=if analysis { super::analysis::MAX_REQUESTS } else { 6 })
                .contains(&self.max_requests)
            || !(1..=match (analysis, messages) {
                (true, true) => MESSAGES_ANALYSIS_BUDGET,
                (true, false) => 10_000_000,
                _ => 1_000_000,
            })
                .contains(&self.budget_microusd)
            // K-d: a turn admitted by context bytes must fit the hosted request
            // body bound (instructions and tools need the remaining room).
            || !(1024..=context_bound(analysis, relay))
                .contains(&self.max_context_bytes)
            || self.catalog.path
                != job
                    .runtime
                    .root
                    .join(if messages { ANTHROPIC_CATALOG } else { CATALOG })
            || self.stream_adapter.verify()? != STREAM_ADAPTER
            || !canonical(&self.stream_adapter)?
        {
            return refuse("provider lane/catalog/limits conflict");
        }
        if self.provider == "claude-sdk" {
            claude_code_pinned(&job.runtime.manifest)?;
        }
        self.hosted.validate()?;
        // Every route speaks the plan's wire family.
        if self
            .hosted
            .routes
            .iter()
            .any(|r| super::hosted::Endpoint::parse(&r.endpoint).is_none_or(|e| !e.serves(wire)))
        {
            return refuse("route endpoint does not serve the provider's wire family");
        }
        self.lane.pi_config.verify()?;
        canonical_directory(&self.lane.pi_config.root)?;
        if self.lane.pi_config.root.starts_with(&job.state_dir)
            || job.state_dir.starts_with(&self.lane.pi_config.root)
        {
            return refuse("provider private/public mount references overlap");
        }
        // This bounded lane admits public Pi settings, not custom providers or
        // executable resources. Never silently load a caller-supplied endpoint/key.
        for (path, entry) in inventory(&self.lane.pi_config.root)? {
            if !matches!(entry, Entry::File { .. })
                || !matches!(path.as_str(), "settings.json" | "models.json")
            {
                return refuse("unsupported lane configuration resource");
            }
            if fs::metadata(self.lane.pi_config.root.join(&path))?.len() > 65536 {
                return refuse("public config file exceeded bound");
            }
            let value: Value =
                serde_json::from_slice(&fs::read(self.lane.pi_config.root.join(&path))?)?;
            if !value.is_object() || (path == "models.json" && value != json!({})) {
                return refuse(
                    "custom model/provider configuration unsupported in subscription lane",
                );
            }
        }
        if self.task.verify()?.len() > 32768 {
            return refuse("provider task input exceeded bound");
        }
        self.reservation_microusd()?;
        let admission: Admission = serde_json::from_slice(&job.authority.verify()?)
            .map_err(|_| Error::Refused("exact provider runtime admission required".into()))?;
        if admission.schema != "work-engine.pi-coding-runtime-admission.v1"
            || admission.job_id != job.job_id
            || admission.binary_sha256 != file_digest(&std::env::current_exe()?, None)?
            || admission.model != self.model
            || admission.provider != self.provider
            || admission.pi_config_manifest_sha256 != self.lane.pi_config.manifest.sha256
            || admission.task_sha256 != self.task.sha256
            || admission.catalog_sha256 != self.catalog.sha256
            || admission.max_requests != self.max_requests
            || admission.budget_microusd != self.budget_microusd
            || !admission.allow_provider
            || admission.hosted_routes_sha256 != self.hosted.digest()
        {
            return refuse("provider admission does not bind this exact realization");
        }
        self.failover_admitted(
            admission.failover_policy_sha256.as_deref(),
            admission.state_observation_sha256.as_deref(),
        )?;
        self.classification_admitted(admission.classification_table_sha256.as_deref())?;
        Ok(())
    }
    /// The plan's classification table digest is the admission's: both
    /// absent or equal. A digest other than the linked table's is admitted
    /// and classifies nothing (`classification::table_binding`).
    pub fn classification_admitted(&self, admitted_sha256: Option<&str>) -> Result<()> {
        if self.classification_table_sha256.as_deref() != admitted_sha256 {
            return refuse("classification table not bound by the admission");
        }
        Ok(())
    }
    /// The plan's failover inputs, checked against the admission's digests:
    /// both absent, or each present on both sides with equal digests. The
    /// policy must be valid for the route table and the observation must
    /// match it. Returns the policy, its digest and the observation.
    pub fn failover_admitted(
        &self,
        policy_sha256: Option<&str>,
        observation_sha256: Option<&str>,
    ) -> Result<Option<super::failover::AdmittedFailover>> {
        let Some(f) = &self.failover else {
            if policy_sha256.is_some() || observation_sha256.is_some() {
                return refuse("failover digests admitted without failover inputs");
            }
            return Ok(None);
        };
        if policy_sha256 != Some(f.policy.sha256.as_str())
            || observation_sha256 != f.observation.as_ref().map(|o| o.sha256.as_str())
        {
            return refuse("failover inputs not bound by the admission");
        }
        let policy = super::failover::FailoverPolicy::parse(&f.policy.verify()?, &self.hosted)?;
        let observation = match &f.observation {
            None => None,
            Some(o) => Some((
                super::failover::check_observation(&o.verify()?, &policy)?,
                o.sha256.clone(),
            )),
        };
        Ok(Some((policy, f.policy.sha256.clone(), observation)))
    }
}
/// The upper bound of a plan's `max_context_bytes` on its profile.
fn context_bound(analysis: bool, relay: bool) -> u64 {
    if relay {
        super::analysis::RELAY_MAX_CONTEXT_BYTES
    } else if analysis {
        super::analysis::MAX_CONTEXT_BYTES
    } else {
        32768
    }
}
/// The runtime manifest carries the pinned Claude Code binary as a file
/// (admission verifies the runtime tree against that manifest).
fn claude_code_pinned(manifest: &Artifact) -> Result<()> {
    let entries: BTreeMap<String, Entry> = serde_json::from_slice(&manifest.verify()?)?;
    if entries.get(CLAUDE_CODE_ENTRY)
        != Some(&Entry::File {
            sha256: CLAUDE_CODE_SHA256.into(),
        })
    {
        return refuse("runtime does not carry the pinned Claude Code binary");
    }
    Ok(())
}
fn rates(model: &Value) -> Result<(f64, f64)> {
    let mut input = 0.0_f64;
    let mut output = 0.0_f64;
    let mut entries = vec![&model["cost"]];
    if let Some(tiers) = model["cost"]["tiers"].as_array() {
        entries.extend(tiers);
    }
    for cost in entries {
        for key in ["input", "cacheRead", "cacheWrite"] {
            let value = cost[key]
                .as_f64()
                .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 1000.0)
                .ok_or_else(|| Error::Refused("catalog input rate absent/invalid".into()))?;
            input = input.max(value);
        }
        output = output.max(
            cost["output"]
                .as_f64()
                .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 1000.0)
                .ok_or_else(|| Error::Refused("catalog output rate absent/invalid".into()))?,
        );
    }
    Ok((input, output))
}
fn cost(model: &Value, input: u64, output: u64) -> Result<u64> {
    let (input_rate, output_rate) = rates(model)?;
    // USD/million * tokens == microUSD. One additional microUSD rounds upward
    // conservatively even if floating catalog representation lands just below.
    let value = (input as f64 * input_rate + output as f64 * output_rate).ceil();
    if !value.is_finite() || !(0.0..=1_000_000_000.0).contains(&value) {
        return refuse("catalog cost overflow");
    }
    Ok(value as u64 + 1)
}
/// Raw-usage cost for the ledger record: `cost`, or the cap plus one when the
/// catalog cost overflows its bound, so an overrun is never recorded as less.
const RAW_COST_SATURATED: u64 = 1_000_000_001;
fn raw_cost(model: &Value, input: u64, output: u64) -> u64 {
    cost(model, input, output).unwrap_or(RAW_COST_SATURATED)
}
/// Token counts of one Messages response, by price class.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MessagesTokens {
    pub input: u64,
    /// Cache writes with the five-minute lifetime.
    pub write_5m: u64,
    /// Cache writes with the one-hour lifetime (Claude Code's default).
    pub write_1h: u64,
    pub read: u64,
    pub output: u64,
}
fn catalog_rate(cost: &Value, key: &str) -> Result<f64> {
    cost[key]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 1000.0)
        .ok_or_else(|| Error::Refused("catalog rate absent/invalid".into()))
}
/// Anthropic prices one-hour cache writes at twice the input rate; Pi's
/// catalog `cacheWrite` is the five-minute rate.
const ONE_HOUR_WRITE_FACTOR: f64 = 2.0;
/// Messages usage priced per token class at the catalog rates of the tier
/// the request falls in. The tier is chosen by all input tokens (uncached,
/// cache writes and cache reads), so a request is never priced below its
/// tier. Rounded up, plus one microUSD, like [`cost`].
fn messages_cost(model: &Value, t: MessagesTokens) -> Result<u64> {
    let all_input = t
        .input
        .saturating_add(t.write_5m)
        .saturating_add(t.write_1h)
        .saturating_add(t.read);
    let mut tier = &model["cost"];
    let mut above = 0u64;
    if let Some(tiers) = model["cost"]["tiers"].as_array() {
        for candidate in tiers {
            let threshold = candidate["inputTokensAbove"]
                .as_u64()
                .ok_or_else(|| Error::Refused("catalog tier threshold absent".into()))?;
            if all_input > threshold && threshold >= above {
                tier = candidate;
                above = threshold;
            }
        }
    }
    let input = catalog_rate(tier, "input")?;
    let value = (t.input as f64 * input
        + t.write_5m as f64 * catalog_rate(tier, "cacheWrite")?
        + t.write_1h as f64 * input * ONE_HOUR_WRITE_FACTOR
        + t.read as f64 * catalog_rate(tier, "cacheRead")?
        + t.output as f64 * catalog_rate(tier, "output")?)
    .ceil();
    if !value.is_finite() || !(0.0..=1_000_000_000.0).contains(&value) {
        return refuse("catalog cost overflow");
    }
    Ok(value as u64 + 1)
}
#[cfg(test)]
pub(super) fn messages_cost_for_tests(model: &Value, t: MessagesTokens) -> u64 {
    messages_cost(model, t).unwrap()
}
#[cfg(test)]
pub(super) fn messages_full_reservation_for_tests(model: &Value) -> u64 {
    messages_full_reservation(model).unwrap()
}
/// Worst-case Messages request: the whole context window at the highest
/// input-side rate of any tier (including one-hour cache writes) and the
/// output bound at the highest output rate.
fn messages_full_reservation(model: &Value) -> Result<u64> {
    let (window, out) = catalog_bounds(model)?;
    let mut input = 0.0_f64;
    let mut output = 0.0_f64;
    let mut entries = vec![&model["cost"]];
    if let Some(tiers) = model["cost"]["tiers"].as_array() {
        entries.extend(tiers);
    }
    for cost in entries {
        let base = catalog_rate(cost, "input")?;
        for v in [
            base,
            base * ONE_HOUR_WRITE_FACTOR,
            catalog_rate(cost, "cacheRead")?,
            catalog_rate(cost, "cacheWrite")?,
        ] {
            input = input.max(v);
        }
        output = output.max(catalog_rate(cost, "output")?);
    }
    let value = (window as f64 * input + out as f64 * output).ceil();
    if !value.is_finite() || !(0.0..=1_000_000_000.0).contains(&value) {
        return refuse("catalog cost overflow");
    }
    Ok(value as u64 + 1)
}
fn catalog_bounds(model: &Value) -> Result<(u64, u64)> {
    let input = model["contextWindow"]
        .as_u64()
        .filter(|v| *v > 0 && *v <= 2_000_000)
        .ok_or_else(|| Error::Refused("catalog context bound absent".into()))?;
    let output = model["maxTokens"]
        .as_u64()
        .filter(|v| *v > 0 && *v <= input)
        .ok_or_else(|| Error::Refused("catalog output bound absent".into()))?;
    Ok((input, output))
}
pub(super) fn full_reservation(model: &Value) -> Result<u64> {
    let (input, output) = catalog_bounds(model)?;
    cost(model, input, output)
}
impl Host {
    pub(super) fn coding_prepare_provider(&mut self) -> Result<()> {
        let profile = self.journal.job.coding.as_ref().unwrap();
        let Some(plan) = &profile.provider else {
            return Ok(());
        };
        plan.validate(&self.journal.job)?;
        let private = self.journal.job.state_dir.join("private-access");
        fs::create_dir(&private)?;
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700))?;
        // Provider access stays on the Host and is applied only at dispatch;
        // `private-access` stays empty.
        let config = self.journal.job.state_dir.join("coding-public-config");
        fs::create_dir(&config)?;
        for (path, entry) in inventory(&plan.lane.pi_config.root)? {
            let Entry::File { sha256 } = entry else {
                return refuse("public config symlink refused");
            };
            let bytes = fs::read(plan.lane.pi_config.root.join(&path))?;
            if digest(&bytes) != sha256 {
                return refuse("public config changed while staging");
            }
            fs::write(config.join(path), bytes)?;
        }
        let public = self.journal.job.state_dir.join("coding-public");
        fs::create_dir(&public)?;
        let task = String::from_utf8(plan.task.verify()?)
            .map_err(|_| Error::Refused("task is not UTF8".into()))?;
        let mut job = json!({"schema":super::HOSTED_BRIDGE_PROFILE,"provider":plan.provider,"model":plan.model,"selected":plan.selected()?,"task":task,"deadline":self.journal.job.deadline_unix_seconds,"maxRequests":plan.max_requests,"maxContextBytes":plan.max_context_bytes});
        if super::is_analysis(&profile.schema) {
            // The receiver's own bounds for this profile; the Host checks
            // every one of them again where it reads.
            job["schema"] = json!(super::ANALYSIS_PROFILE);
            job["maxBodyBytes"] = json!(super::analysis::MAX_REQUEST_BODY);
            job["frameBytes"] = json!(super::analysis::FRAME_BYTES);
            job["maxReads"] = json!(profile.max_tool_calls);
            job["pageBytes"] = json!(super::analysis::PAGE_BYTES);
            if super::is_relay(&profile.schema) {
                // Relayed lifecycle tools: the pinned definitions, executed by
                // the controller; the receiver returns each answer unchanged.
                job["schema"] = json!(super::LIFECYCLE_PROFILE);
                job["tools"] = native_realization::pi_host::tools::receiver_tools();
            }
        }
        atomic(&public.join("job.json"), &job)?;
        Ok(())
    }
    pub(super) fn coding_provider_frame(&mut self, frame: &Value) -> Result<Option<Value>> {
        if self.signed_pi() {
            return self.coding_fake_provider_frame(frame);
        }
        let plan = self
            .journal
            .job
            .coding
            .as_ref()
            .unwrap()
            .provider
            .as_ref()
            .ok_or_else(|| Error::Refused("provider frame in disabled fixture".into()))?
            .clone();
        let model = plan.selected()?;
        let id = frame["id"]
            .as_u64()
            .filter(|v| *v >= 1 && *v <= u64::from(plan.max_requests))
            .ok_or_else(|| Error::Refused("provider request id invalid".into()))?;
        match frame["type"].as_str().unwrap_or("") {
            "provider_request" => {
                if self.expired() || self.cancelled() {
                    return refuse("provider request expired/cancelled");
                }
                let state = self.journal.coding.as_mut().unwrap();
                if state.provider_failed
                    || state.provider_requests.contains_key(&id)
                    || id != state.provider_requests.len() as u64 + 1
                    || state.provider_requests.values().any(Reservation::unsettled)
                    || frame["model"] != plan.model
                    || frame["provider"] != plan.provider
                    || frame["contextBytes"]
                        .as_u64()
                        .is_none_or(|v| v > plan.max_context_bytes)
                {
                    return refuse("provider identity/context/unknown reservation conflict");
                }
                let reserve = plan.request_reservation()?;
                let spent: u64 = state
                    .provider_requests
                    .values()
                    .map(Reservation::charged_microusd)
                    .sum();
                if spent
                    .checked_add(reserve)
                    .is_none_or(|v| v > plan.budget_microusd)
                {
                    return refuse("provider budget exhausted");
                }
                state.provider_requests.insert(
                    id,
                    Reservation {
                        reserved_microusd: reserve,
                        ..Reservation::default()
                    },
                );
                self.save()?; // Durable unknown reservation BEFORE granting possible network entry.
                let consumption_deadline =
                    self.journal.job.deadline_unix_seconds.saturating_mul(1000);
                Ok(Some(
                    json!({"type":"provider_grant","id":id,"reservedMicrousd":reserve,"consumptionDeadlineMs":consumption_deadline}),
                ))
            }
            "provider_observation" => {
                let state = self.journal.coding.as_mut().unwrap();
                let r = state
                    .provider_requests
                    .get_mut(&id)
                    .ok_or_else(|| Error::Refused("provider observation before intent".into()))?;
                if r.settled_microusd.is_some() {
                    return refuse("provider observation after settlement");
                }
                if let Some(status) = frame["httpStatus"].as_u64()
                    && r.http_status.replace(status).is_some()
                {
                    return refuse("duplicate provider HTTP response");
                }
                if let Some(name) = frame["backendModel"].as_str() {
                    if name != plan.model
                        || r.backend_model.as_ref().is_some_and(|prior| prior != name)
                    {
                        return refuse("backend model identity conflict");
                    }
                    r.backend_model = Some(name.into());
                }
                if let Some(raw) = frame.get("rawTerminal")
                    && r.raw_terminal.replace(raw.clone()).is_some()
                {
                    return refuse("duplicate raw terminal evidence");
                }
                self.save()?;
                Ok(None)
            }
            "provider_complete" => {
                let state = self.journal.coding.as_mut().unwrap();
                let r = state
                    .provider_requests
                    .get_mut(&id)
                    .ok_or_else(|| Error::Refused("provider completion before intent".into()))?;
                if r.settled_microusd.is_some() {
                    return refuse("duplicate provider completion");
                }
                let usage = &frame["usage"];
                // Hosted: settlement uses the Host's relayed raw terminal; the
                // container's own usage is kept only as labelled context.
                r.usage = Some(if r.attempts.is_empty() {
                    usage.clone()
                } else {
                    json!({"source":"container-reported","evidence":false,"value":usage})
                });
                // Hosted: the container's completion settles only an attempt the
                // Host itself classified as a completed relay.
                if let Some(a) = r
                    .attempts
                    .last()
                    .filter(|a| a.disposition != AttemptDisposition::Completed)
                {
                    let detail = format!(
                        "hosted request {id} attempt {} is {:?} ({}); container completion cannot settle it; {}",
                        a.attempt,
                        a.disposition,
                        a.reason.as_deref().unwrap_or("no reason recorded"),
                        match a.disposition {
                            AttemptDisposition::Refused | AttemptDisposition::NotDispatched =>
                                "that attempt's reservation was released and no later attempt was made",
                            _ => "reservation retained",
                        }
                    );
                    state.provider_failed = true;
                    self.save()?;
                    return refuse(detail);
                }
                if !matches!(frame["stopReason"].as_str(), Some("stop" | "toolUse"))
                    || r.http_status != Some(200)
                    || r.backend_model.as_deref() != Some(&plan.model)
                {
                    state.provider_failed = true;
                    self.save()?;
                    return refuse("provider result unknown/error; reservation retained");
                }
                // Raw-usage cost and all input tokens, per wire family.
                let measured = match plan.wire_checked()? {
                    crate::pi_stream::Wire::Responses => {
                        let raw = r.raw_terminal.as_ref().filter(|raw| {
                            raw["eventType"] == "response.completed" && raw["status"] == "completed"
                        });
                        let measured = raw.and_then(|raw| {
                            Some((
                                raw["usage"]["input_tokens"].as_u64()?,
                                raw["usage"]["output_tokens"].as_u64()?,
                                raw["usage"]["total_tokens"].as_u64()?,
                            ))
                        });
                        match measured {
                            None => Err("raw completed status/usage unavailable"),
                            Some((input, output, total))
                                if input.checked_add(output) != Some(total) =>
                            {
                                Err("raw usage inconsistent")
                            }
                            Some((input, output, _)) => {
                                Ok((raw_cost(&model, input, output), input, output))
                            }
                        }
                    }
                    crate::pi_stream::Wire::Messages => {
                        // The raw stop reason must be the one the container
                        // reports: `stop` is an ended turn, `toolUse` a tool call.
                        let raw = r.raw_terminal.as_ref().filter(|raw| {
                            raw["eventType"] == "message_stop"
                                && match frame["stopReason"].as_str() {
                                    Some("stop") => matches!(
                                        raw["stopReason"].as_str(),
                                        Some("end_turn" | "stop_sequence")
                                    ),
                                    Some("toolUse") => raw["stopReason"] == "tool_use",
                                    _ => false,
                                }
                        });
                        // Cache writes split by lifetime must add up to the total.
                        let measured = raw.and_then(|raw| {
                            let u = &raw["usage"];
                            let t = MessagesTokens {
                                input: u["input_tokens"].as_u64()?,
                                write_5m: u["cache_creation_5m_input_tokens"].as_u64()?,
                                write_1h: u["cache_creation_1h_input_tokens"].as_u64()?,
                                read: u["cache_read_input_tokens"].as_u64()?,
                                output: u["output_tokens"].as_u64()?,
                            };
                            (t.write_5m.checked_add(t.write_1h)?
                                == u["cache_creation_input_tokens"].as_u64()?)
                            .then_some(t)
                        });
                        match measured {
                            None => Err("raw completed stop reason/usage unavailable"),
                            Some(t) => Ok((
                                messages_cost(&model, t).unwrap_or(RAW_COST_SATURATED),
                                t.input
                                    .saturating_add(t.write_5m)
                                    .saturating_add(t.write_1h)
                                    .saturating_add(t.read),
                                t.output,
                            )),
                        }
                    }
                };
                let (actual, input, output) = match measured {
                    Ok(m) => m,
                    Err(reason) => {
                        state.provider_failed = true;
                        self.save()?;
                        return refuse(format!("{reason}; full reservation retained"));
                    }
                };
                // An overrun (or catalog cost overflow) still fails the job,
                // but its raw-usage cost is recorded and held (G1).
                if input > model["contextWindow"].as_u64().unwrap_or(0)
                    || output > model["maxTokens"].as_u64().unwrap_or(0)
                    || actual > r.reserved_microusd
                {
                    if actual > r.reserved_microusd {
                        if r.attempts.is_empty() {
                            r.raw_microusd = Some(actual);
                        } else if let Some(a) = r
                            .attempts
                            .last_mut()
                            .filter(|a| a.disposition == AttemptDisposition::Completed)
                        {
                            a.raw_microusd = Some(actual);
                        }
                    }
                    state.provider_failed = true;
                    self.save()?;
                    return refuse("provider usage exceeds admitted catalog bound; raw cost held");
                }
                if r.attempts.is_empty() {
                    r.settled_microusd = Some(actual);
                } else {
                    // Hosted: the completed attempt settles; refused attempts
                    // already released theirs.
                    let attempt = r.attempts.last().map(|a| a.attempt).unwrap_or(0);
                    if let Err(e) = r.settle_completed(attempt, actual) {
                        state.provider_failed = true;
                        self.save()?;
                        return Err(e);
                    }
                }
                self.save()?;
                Ok(None)
            }
            _ => refuse("unknown provider protocol frame"),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_catalog_reservation_covers_cache_and_higher_tier_rates() {
        let model = json!({"contextWindow":272000,"maxTokens":128000,"cost":{"input":0.1,"output":0.5,"cacheRead":0.01,"cacheWrite":0.125,"tiers":[{"input":0.2,"output":0.75,"cacheRead":0.02,"cacheWrite":0.25}]}});
        assert_eq!(full_reservation(&model).unwrap(), 164001);
        assert_eq!(full_reservation(&model).unwrap() * 6, 984006);
        assert!(full_reservation(&json!({"contextWindow":272000,"maxTokens":128000})).is_err());
    }
}

#[cfg(test)]
pub(super) mod ledger_tests {
    use super::*;
    pub(crate) fn fixture() -> (tempfile::TempDir, Host) {
        let (dir, mut h) = super::super::tests::fixture();
        let path = dir.path().join("catalog.json");
        let bytes=serde_json::to_vec(&json!({"openai-codex-responses":{"chat:gpt-6-luna":{"id":"gpt-6-luna","provider":"openai-codex","api":"openai-codex-responses","baseUrl":"https://chatgpt.com/backend-api","contextWindow":272000,"maxTokens":128000,"cost":{"input":0.1,"output":0.5,"cacheRead":0.01,"cacheWrite":0.125,"tiers":[{"input":0.2,"output":0.75,"cacheRead":0.02,"cacheWrite":0.25}]}}}})).unwrap();
        fs::write(&path, &bytes).unwrap();
        let pin = Artifact {
            path,
            sha256: digest(&bytes),
        };
        let tree = Tree {
            root: dir.path().join("unused"),
            manifest: pin.clone(),
        };
        h.journal.job.coding.as_mut().unwrap().provider = Some(ProviderPlan {
            lane: LaneInputs { pi_config: tree },
            task: pin.clone(),
            catalog: pin.clone(),
            stream_adapter: pin,
            hosted: super::super::hosted::HostedRoutes {
                routes: vec![super::super::hosted::HostedRoute {
                    id: "primary".into(),
                    payer: "payer".into(),
                    endpoint: "loopback:9".into(),
                }],
                max_attempts_per_request: 1,
            },
            provider: DEFAULT_PROVIDER.into(),
            model: "gpt-6-luna".into(),
            max_requests: 6,
            budget_microusd: 1_000_000,
            max_context_bytes: 32768,
            failover: None,
            classification_table_sha256: None,
        });
        (dir, h)
    }
    fn request(id: u64) -> Value {
        json!({"type":"provider_request","id":id,"provider":"openai-codex","model":"gpt-6-luna","contextBytes":1000})
    }
    #[test]
    fn reservation_is_durable_before_grant_and_unknown_cannot_replay() {
        let (dir, mut h) = fixture();
        let grant = h.coding_provider_frame(&request(1)).unwrap().unwrap();
        assert_eq!(grant["reservedMicrousd"], 164001);
        assert_eq!(
            grant["consumptionDeadlineMs"],
            h.journal.job.deadline_unix_seconds * 1000
        );
        let record: Value =
            serde_json::from_slice(&fs::read(dir.path().join("pi-state.json")).unwrap()).unwrap();
        assert_eq!(
            record["coding"]["provider_requests"]["1"]["reserved_microusd"],
            164001
        );
        assert!(record["coding"]["provider_requests"]["1"]["settled_microusd"].is_null());
        assert!(h.coding_provider_frame(&request(1)).is_err());
        assert!(h.coding_provider_frame(&request(2)).is_err());
        assert!(h.journal.processes.is_empty());
    }
    #[test]
    fn source_bound_usage_settles_and_admits_next_finite_request() {
        let (_dir, mut h) = fixture();
        h.coding_provider_frame(&request(1)).unwrap();
        h.coding_provider_frame(&json!({"type":"provider_observation","id":1,"httpStatus":200,"backendModel":"gpt-6-luna","rawTerminal":{"eventType":"response.completed","status":"completed","usage":{"input_tokens":1000,"output_tokens":100,"total_tokens":1100}}})).unwrap();
        h.coding_provider_frame(&json!({"type":"provider_complete","id":1,"stopReason":"toolUse","usage":{"input":1000,"output":100,"cacheRead":0,"cacheWrite":0}})).unwrap();
        assert_eq!(
            h.journal.coding.as_ref().unwrap().provider_requests[&1].settled_microusd,
            Some(326)
        );
        assert!(h.coding_provider_frame(&request(2)).unwrap().is_some());
        assert!(h.coding_provider_frame(&request(7)).is_err());
    }
    /// G1: a completion whose raw-usage cost exceeds the reservation still
    /// fails the job, but the raw cost is recorded and held, never less.
    #[test]
    fn overrun_completion_holds_its_raw_cost() {
        for (output, expect_saturated) in [(230_000u64, false), (4_000_000_000_000u64, true)] {
            let (_dir, mut h) = fixture();
            h.coding_provider_frame(&request(1)).unwrap();
            let total = 20_000 + output;
            h.coding_provider_frame(&json!({"type":"provider_observation","id":1,"httpStatus":200,"backendModel":"gpt-6-luna","rawTerminal":{"eventType":"response.completed","status":"completed","usage":{"input_tokens":20000,"output_tokens":output,"total_tokens":total}}})).unwrap();
            let refused = h.coding_provider_frame(&json!({"type":"provider_complete","id":1,"stopReason":"stop","usage":{"input":20000,"output":output,"cacheRead":0,"cacheWrite":0}}));
            assert!(refused.is_err());
            let state = h.journal.coding.as_ref().unwrap();
            assert!(state.provider_failed);
            let r = &state.provider_requests[&1];
            let raw = r.raw_microusd.expect("raw cost recorded");
            assert!(raw > r.reserved_microusd, "{raw}");
            assert_eq!(raw == RAW_COST_SATURATED, expect_saturated);
            assert_eq!((r.charged_microusd(), r.unsettled()), (raw, true));
            assert!(h.coding_provider_frame(&request(2)).is_err());
        }
    }
    #[test]
    fn hosted_overrun_attempt_holds_max_of_reservation_and_raw() {
        let mut r = Reservation {
            reserved_microusd: 164001,
            ..Reservation::default()
        };
        r.attempts.push(AttemptRecord {
            attempt: 1,
            route: "primary".into(),
            payer: "payer".into(),
            reserved_microusd: 164001,
            disposition: AttemptDisposition::Completed,
            http_status: Some(200),
            relay_sha256: None,
            refusal_body_sha256: None,
            settled_microusd: None,
            reason: None,
            gate_entered: true,
            deadline_ms: None,
            routes_sha256: None,
            framed_end_observed: Some(true),
            request_body_sha256: None,
            raw_microusd: None,
            failover: None,
            classification_table_sha256: None,
            head_hold: None,
        });
        assert_eq!((r.charged_microusd(), r.unsettled()), (164001, true));
        r.attempts[0].raw_microusd = Some(177501);
        assert_eq!((r.charged_microusd(), r.unsettled()), (177501, true));
        // A raw cost at or below the reservation never lowers the hold.
        r.attempts[0].raw_microusd = Some(1);
        assert_eq!(r.charged_microusd(), 164001);
        // The record serializes exactly as before when no raw cost is set.
        r.attempts[0].raw_microusd = None;
        assert!(!serde_json::to_string(&r).unwrap().contains("raw_microusd"));
    }
    #[test]
    fn error_unknown_identity_and_over_budget_never_regrant() {
        let (_dir, mut h) = fixture();
        h.coding_provider_frame(&request(1)).unwrap();
        assert!(
            h.coding_provider_frame(
                &json!({"type":"provider_complete","id":1,"stopReason":"error","usage":{}})
            )
            .is_err()
        );
        assert!(
            h.journal.coding.as_ref().unwrap().provider_requests[&1]
                .settled_microusd
                .is_none()
        );
        assert!(h.coding_provider_frame(&request(2)).is_err());
        let (_dir, mut h) = fixture();
        h.journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap()
            .budget_microusd = 100;
        assert!(h.coding_provider_frame(&request(1)).is_err());
        assert!(
            h.journal
                .coding
                .as_ref()
                .unwrap()
                .provider_requests
                .is_empty()
        );
        let (_dir, mut h) = fixture();
        let mut wrong = request(1);
        wrong["model"] = json!("an-alias");
        assert!(h.coding_provider_frame(&wrong).is_err());
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    #[test]
    fn missing_runtime_admission_refuses_before_staging() {
        let (dir, mut h) = super::super::tests::fixture();
        let root = dir.path().canonicalize().unwrap();
        let runtime = root.join("runtime");
        let catalog_path = runtime.join(CATALOG);
        fs::create_dir_all(catalog_path.parent().unwrap()).unwrap();
        let catalog=serde_json::to_vec(&json!({"openai-codex-responses":{"chat:gpt-6-luna":{"id":"gpt-6-luna","provider":"openai-codex","api":"openai-codex-responses","baseUrl":"https://chatgpt.com/backend-api","contextWindow":272000,"maxTokens":128000,"cost":{"input":0.1,"output":0.5,"cacheRead":0.01,"cacheWrite":0.125,"tiers":[{"input":0.2,"output":0.75,"cacheRead":0.02,"cacheWrite":0.25}]}}}})).unwrap();
        fs::write(&catalog_path, &catalog).unwrap();
        let config = root.join("config");
        fs::create_dir(&config).unwrap();
        let manifest = root.join("config.json");
        let bytes = serde_json::to_vec(&inventory(&config).unwrap()).unwrap();
        fs::write(&manifest, &bytes).unwrap();
        let adapter = root.join("adapter.mjs");
        fs::write(&adapter, STREAM_ADAPTER).unwrap();
        let task = root.join("task");
        fs::write(&task, b"fixture task").unwrap();
        let authority = root.join("authority");
        fs::write(&authority, b"no provider admission").unwrap();
        h.journal.job.runtime.root = runtime;
        h.journal.job.state_dir = root.join("state");
        fs::create_dir(&h.journal.job.state_dir).unwrap();
        h.journal.job.authority = Artifact {
            path: authority,
            sha256: digest(b"no provider admission"),
        };
        let profile = h.journal.job.coding.as_mut().unwrap();
        profile.schema = super::super::HOSTED_BRIDGE_PROFILE.into();
        profile.provider = Some(ProviderPlan {
            lane: LaneInputs {
                pi_config: Tree {
                    root: config,
                    manifest: Artifact {
                        path: manifest,
                        sha256: digest(&bytes),
                    },
                },
            },
            task: Artifact {
                path: task,
                sha256: digest(b"fixture task"),
            },
            catalog: Artifact {
                path: catalog_path,
                sha256: digest(&catalog),
            },
            stream_adapter: Artifact {
                path: adapter,
                sha256: digest(STREAM_ADAPTER),
            },
            hosted: super::super::hosted::HostedRoutes {
                routes: vec![super::super::hosted::HostedRoute {
                    id: "primary".into(),
                    payer: "payer".into(),
                    endpoint: "loopback:9".into(),
                }],
                max_attempts_per_request: 1,
            },
            provider: DEFAULT_PROVIDER.into(),
            model: "gpt-6-luna".into(),
            max_requests: 6,
            budget_microusd: 1000000,
            max_context_bytes: 32768,
            failover: None,
            classification_table_sha256: None,
        });
        let error = h.coding_prepare_provider().unwrap_err();
        assert!(error.to_string().contains("runtime admission"), "{error}");
        assert!(!h.journal.job.state_dir.join("private-access").exists());
        assert!(h.journal.processes.is_empty());
        // Every retired credential-mounted schema is refused by name, with or
        // without a provider plan (F7).
        for retired in super::super::RETIRED_LIVE_PROFILES {
            for with_provider in [true, false] {
                let mut job = h.journal.job.clone();
                let profile = job.coding.as_mut().unwrap();
                profile.schema = retired.into();
                if !with_provider {
                    profile.provider = None;
                }
                let error = job.coding.as_ref().unwrap().validate(&job).unwrap_err();
                assert!(error.to_string().contains("retired"), "{retired}: {error}");
            }
        }
    }
}

#[cfg(test)]
mod raw_evidence_tests {
    use super::*;
    #[test]
    fn context_bound_is_per_profile() {
        assert_eq!(context_bound(true, true), 500_000, "lifecycle relay");
        assert_eq!(context_bound(true, false), 400_000, "analysis");
        assert_eq!(context_bound(false, false), 32768, "hosted coding");
        // The relay bound keeps a request body and a frame carrying the
        // context twice within their bounds.
        assert!(
            (super::super::analysis::RELAY_MAX_CONTEXT_BYTES as usize)
                < super::super::analysis::MAX_REQUEST_BODY
        );
        assert!(
            2 * super::super::analysis::RELAY_MAX_CONTEXT_BYTES
                < super::super::analysis::FRAME_BYTES as u64
        );
    }
    #[test]
    fn claude_code_binary_entry_is_the_pinned_file() {
        let dir = tempfile::tempdir().unwrap();
        let pinned = |entry: Value| {
            let bytes = serde_json::to_vec(
                &json!({"node_modules/x":{"kind":"file","sha256":"0".repeat(64)},
                CLAUDE_CODE_ENTRY:entry}),
            )
            .unwrap();
            let path = dir.path().join("runtime.json");
            fs::write(&path, &bytes).unwrap();
            claude_code_pinned(&Artifact {
                path,
                sha256: digest(&bytes),
            })
        };
        pinned(json!({"kind":"file","sha256":CLAUDE_CODE_SHA256})).unwrap();
        for bad in [
            json!({"kind":"file","sha256":"1".repeat(64)}),
            json!({"kind":"symlink","target":"claude-other"}),
        ] {
            assert!(pinned(bad).is_err());
        }
        let bytes = br#"{"node_modules/x":{"kind":"file","sha256":"0"}}"#;
        let path = dir.path().join("absent.json");
        fs::write(&path, bytes).unwrap();
        let absent = Artifact {
            path,
            sha256: digest(bytes),
        };
        assert!(claude_code_pinned(&absent).is_err(), "entry absent");
    }
    #[test]
    fn sdk_zero_defaults_cannot_settle_missing_incomplete_or_nonterminal_raw_evidence() {
        for raw in [
            Value::Null,
            json!({"eventType":"response.completed","status":"completed"}),
            json!({"eventType":"response.completed","usage":{"input_tokens":0,"output_tokens":0,"total_tokens":0}}),
            json!({"eventType":"response.completed","status":"in_progress","usage":{"input_tokens":0,"output_tokens":0,"total_tokens":0}}),
            json!({"eventType":"response.completed","status":"completed","usage":{"input_tokens":0,"total_tokens":0}}),
            json!({"eventType":"response.completed","status":"completed","usage":{"input_tokens":"0","output_tokens":0,"total_tokens":0}}),
        ] {
            let (_dir, mut h) = super::ledger_tests::fixture();
            h.coding_provider_frame(&json!({"type":"provider_request","id":1,"provider":"openai-codex","model":"gpt-6-luna","contextBytes":1000})).unwrap();
            h.coding_provider_frame(&json!({"type":"provider_observation","id":1,"httpStatus":200,"backendModel":"gpt-6-luna","rawTerminal":raw})).unwrap();
            assert!(h.coding_provider_frame(&json!({"type":"provider_complete","id":1,"stopReason":"stop","usage":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0}})).is_err());
            assert_eq!(
                h.journal.coding.as_ref().unwrap().provider_requests[&1].reserved_microusd,
                164001
            );
            assert!(
                h.journal.coding.as_ref().unwrap().provider_requests[&1]
                    .settled_microusd
                    .is_none()
            );
            assert!(h.coding_provider_frame(&json!({"type":"provider_request","id":2,"provider":"openai-codex","model":"gpt-6-luna","contextBytes":1000})).is_err());
        }
    }
}

impl Host {
    fn coding_fake_provider_frame(&mut self, frame: &Value) -> Result<Option<Value>> {
        let id = frame["id"]
            .as_u64()
            .filter(|id| (1..=2).contains(id))
            .ok_or_else(|| Error::Refused("fake provider ID".into()))?;
        match frame["type"].as_str().unwrap_or("") {
            "provider_request" => {
                if self.expired() || self.cancelled() {
                    return refuse("fake provider deadline/cancel");
                }
                let state = self.journal.coding.as_mut().unwrap();
                if state.provider_failed
                    || id != state.provider_requests.len() as u64 + 1
                    || state.provider_requests.values().any(Reservation::unsettled)
                    || frame["provider"] != "fake_fixture"
                    || frame["model"] != "fake-pi-model"
                    || frame["contextBytes"].as_u64().is_none_or(|n| n > 65536)
                {
                    return refuse("fake provider identity/context/unknown conflict");
                }
                state.provider_requests.insert(
                    id,
                    Reservation {
                        reserved_microusd: 0,
                        ..Reservation::default()
                    },
                );
                self.save()?;
                let owner = self
                    .pi_owner
                    .as_ref()
                    .ok_or_else(|| Error::Refused("fake provider owner absent".into()))?;
                let lease = owner
                    .enter(session_task_admission::pi_pilot::Effect::Provider(id))
                    .map_err(|e| Error::Refused(e.into()))?;
                lease
                    .check_current()
                    .map_err(|e| Error::Refused(e.into()))?;
                let consumption_deadline = lease
                    .consumption_deadline_millis(self.journal.job.deadline_unix_seconds)
                    .map_err(|e| Error::Refused(e.into()))?;
                self.reply_leases.push(lease);
                Ok(Some(
                    json!({"type":"provider_grant","id":id,"classification":"fake_fixture","reservedMicrousd":0,"consumptionDeadlineMs":consumption_deadline}),
                ))
            }
            "provider_observation" => {
                let r = self
                    .journal
                    .coding
                    .as_mut()
                    .unwrap()
                    .provider_requests
                    .get_mut(&id)
                    .ok_or_else(|| Error::Refused("fake observation before intent".into()))?;
                if r.raw_terminal.is_some()
                    || r.settled_microusd.is_some()
                    || frame["backendModel"] != "fake-pi-model"
                    || frame["httpStatus"] != 200
                    || frame["rawTerminal"]
                        != json!({"classification":"fake_fixture","eventType":"response.completed","status":"completed","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}})
                {
                    return refuse("fake raw terminal conflict");
                }
                r.backend_model = Some("fake-pi-model".into());
                r.http_status = Some(200);
                r.raw_terminal = Some(frame["rawTerminal"].clone());
                self.save()?;
                Ok(None)
            }
            "provider_complete" => {
                let r = self
                    .journal
                    .coding
                    .as_mut()
                    .unwrap()
                    .provider_requests
                    .get_mut(&id)
                    .ok_or_else(|| Error::Refused("fake completion before intent".into()))?;
                if r.settled_microusd.is_some()
                    || r.raw_terminal.is_none()
                    || frame["usage"]
                        != json!({"input":1,"output":1,"cacheRead":0,"cacheWrite":0,"totalTokens":2,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}})
                    || frame["stopReason"] != if id == 1 { "toolUse" } else { "stop" }
                {
                    return refuse("fake usage/completion conflict");
                }
                r.usage = Some(frame["usage"].clone());
                r.settled_microusd = Some(0);
                self.save()?;
                Ok(None)
            }
            _ => refuse("unknown fake provider frame"),
        }
    }
}

#[cfg(test)]
mod signed_fake_tests {
    use super::*;
    #[test]
    fn fake_provider_unknown_entry_and_missing_raw_usage_never_replay() {
        let (_dir, mut h) = super::super::tests::fixture();
        h.journal.job.coding.as_mut().unwrap().schema = super::super::SIGNED_PROFILE.into();
        h.journal.job.deadline_unix_seconds = now() + 120;
        let request = json!({"type":"provider_request","id":1,"provider":"fake_fixture","model":"fake-pi-model","contextBytes":1});
        // The journal reserves first, but an absent authority owner cannot
        // release a provider grant. This durable unknown cannot be replayed.
        assert!(h.coding_provider_frame(&request).is_err());
        assert!(h.coding_provider_frame(&request).is_err());
        assert_eq!(
            h.journal.coding.as_ref().unwrap().provider_requests.len(),
            1
        );
        assert!(
            h.coding_provider_frame(
                &json!({"type":"provider_complete","id":1,"stopReason":"toolUse","usage":{}})
            )
            .is_err()
        );
        assert!(h.coding_provider_frame(&json!({"type":"provider_observation","id":1,"httpStatus":200,"backendModel":"fake-pi-model","rawTerminal":{"classification":"fake_fixture","status":"completed"}})).is_err());
        assert!(
            h.journal.coding.as_ref().unwrap().provider_requests[&1]
                .settled_microusd
                .is_none()
        );
        assert!(h.journal.processes.is_empty());
    }
}

#[cfg(test)]
mod attempt_tests {
    use super::*;
    const BUDGET: u64 = 1_000_000;
    fn requests(ids: &[u64]) -> BTreeMap<u64, Reservation> {
        ids.iter()
            .map(|id| {
                (
                    *id,
                    Reservation {
                        reserved_microusd: 164001,
                        ..Reservation::default()
                    },
                )
            })
            .collect()
    }
    fn reserve(rs: &mut BTreeMap<u64, Reservation>, id: u64, route: &str) -> Result<u64> {
        reserve_attempt(rs, id, route, "payer-a", 164001, BUDGET, 3)
    }
    fn refusal(status: u16, relayed: u64, drained: bool) -> AttemptEvidence {
        AttemptEvidence::HeadRefusal {
            status,
            relayed_bytes: relayed,
            drained_and_screened: drained,
            body_sha256: "c".repeat(64),
            classified: None,
        }
    }
    /// A relay whose terminal event was clean (`terminal_last`) when the
    /// framed end was observed.
    fn relayed(status: u16, upstream_end: bool, interrupted: bool) -> AttemptEvidence {
        AttemptEvidence::Relayed {
            status,
            upstream_end,
            interrupted,
            relay_sha256: "d".repeat(64),
            provider_failure: None,
            terminal_last: upstream_end,
            drain_error: None,
        }
    }
    #[test]
    fn single_attempt_reservations_keep_their_bytes_and_rules() {
        let legacy = Reservation {
            reserved_microusd: 7,
            ..Reservation::default()
        };
        let json = serde_json::to_value(&legacy).unwrap();
        assert!(json.get("attempts").is_none());
        let back: Reservation = serde_json::from_value(json).unwrap();
        assert!(back.attempts.is_empty());
        assert_eq!((legacy.charged_microusd(), legacy.unsettled()), (7, true));
        let settled = Reservation {
            settled_microusd: Some(3),
            ..legacy
        };
        assert_eq!(
            (settled.charged_microusd(), settled.unsettled()),
            (3, false)
        );
    }
    #[test]
    fn definitive_refusal_releases_and_admits_exactly_the_next_attempt() {
        let mut rs = requests(&[1]);
        assert_eq!(reserve(&mut rs, 1, "primary").unwrap(), 1);
        // Attempt 1 is not counted twice with the request-level reservation.
        assert_eq!(rs[&1].charged_microusd(), 164001);
        assert_eq!(
            reserve(&mut rs, 1, "secondary").unwrap_err().to_string(),
            Error::Refused("previous attempt not refused".into()).to_string()
        );
        let r = rs.get_mut(&1).unwrap();
        assert_eq!(
            r.settle_attempt(1, refusal(429, 0, true)).unwrap(),
            AttemptDisposition::Refused
        );
        assert_eq!((r.charged_microusd(), r.unsettled()), (0, false));
        assert!(r.settle_attempt(1, refusal(429, 0, true)).is_err());
        assert_eq!(reserve(&mut rs, 1, "secondary").unwrap(), 2);
        let a = &rs[&1].attempts;
        assert_eq!(
            (a[1].route.as_str(), a[1].disposition, a[0].http_status),
            ("secondary", AttemptDisposition::Pending, Some(429))
        );
        assert_eq!(rs[&1].charged_microusd(), 164001);
    }
    fn classified(
        status: u16,
        relayed: u64,
        drained: bool,
        table: &str,
        class: &str,
    ) -> AttemptEvidence {
        AttemptEvidence::HeadRefusal {
            status,
            relayed_bytes: relayed,
            drained_and_screened: drained,
            body_sha256: "c".repeat(64),
            classified: Some(TableRefusal {
                table_sha256: table.into(),
                class: class.into(),
            }),
        }
    }
    #[test]
    fn classified_head_refusal_releases_only_with_the_linked_table_and_a_failover_class() {
        let linked = hosted_route_availability::provisional::table_sha256();
        // A held 529 refused on a failover class under the linked table is
        // released like a definitive refusal, and the next attempt admitted.
        for class in [
            "overload",
            "api_credit_exhausted",
            "subscription_quota_exhausted",
            "rate_limited",
        ] {
            let mut rs = requests(&[1]);
            reserve(&mut rs, 1, "primary").unwrap();
            let r = rs.get_mut(&1).unwrap();
            assert_eq!(
                r.settle_attempt(1, classified(529, 0, true, &linked, class))
                    .unwrap(),
                AttemptDisposition::Refused,
                "{class}"
            );
            assert_eq!(r.attempts[0].http_status, Some(529));
            assert_eq!((r.charged_microusd(), r.unsettled()), (0, false));
            assert_eq!(reserve(&mut rs, 1, "secondary").unwrap(), 2);
        }
        // Anything else keeps the reservation: another table, a class that is
        // not a failover class, bytes relayed, or a body not drained.
        for evidence in [
            classified(529, 0, true, &"0".repeat(64), "overload"),
            classified(529, 0, true, &linked, "authorization_failure"),
            classified(529, 0, true, &linked, "unclassified"),
            classified(529, 0, true, &linked, "credit"),
            classified(529, 1, true, &linked, "overload"),
            classified(529, 0, false, &linked, "overload"),
        ] {
            let mut rs = requests(&[1]);
            reserve(&mut rs, 1, "primary").unwrap();
            let r = rs.get_mut(&1).unwrap();
            assert_eq!(
                r.settle_attempt(1, evidence.clone()).unwrap(),
                AttemptDisposition::Uncertain,
                "{evidence:?}"
            );
            assert_eq!(
                r.attempts[0].reason.as_deref(),
                Some("refusal not definitive")
            );
            assert_eq!((r.charged_microusd(), r.unsettled()), (164001, true));
        }
    }
    #[test]
    fn classification_digest_must_match_between_plan_and_admission() {
        let (_dir, h) = super::ledger_tests::fixture();
        let mut plan = h
            .journal
            .job
            .coding
            .as_ref()
            .unwrap()
            .provider
            .clone()
            .unwrap();
        let linked = hosted_route_availability::provisional::table_sha256();
        // Absent on both sides: today's plan bytes (no key) and admission.
        assert!(
            serde_json::to_value(&plan)
                .unwrap()
                .get("classification_table_sha256")
                .is_none()
        );
        assert!(plan.classification_admitted(None).is_ok());
        assert!(plan.classification_admitted(Some(&linked)).is_err());
        // Named by the plan: the admission must bind the same digest. Another
        // table's digest is admitted (it classifies nothing).
        for sha in [linked.clone(), "0".repeat(64)] {
            plan.classification_table_sha256 = Some(sha.clone());
            assert!(plan.classification_admitted(Some(&sha)).is_ok());
            assert!(plan.classification_admitted(None).is_err());
            assert!(plan.classification_admitted(Some("x")).is_err());
        }
        // The admission accepts the key (it rejects unknown fields).
        let admission = json!({"schema":"s","job_id":"j","binary_sha256":"b","model":"m",
            "pi_config_manifest_sha256":"p","task_sha256":"t","catalog_sha256":"c","max_requests":1,
            "budget_microusd":1,"allow_provider":true,"hosted_routes_sha256":"r",
            "classification_table_sha256":linked});
        let parsed: Admission = serde_json::from_value(admission).unwrap();
        assert_eq!(parsed.classification_table_sha256, Some(linked));
    }
    #[test]
    fn non_definitive_refusals_retain_and_block() {
        for evidence in [
            refusal(500, 0, true),
            refusal(503, 0, true),
            refusal(429, 1, true),
            refusal(401, 0, false),
            AttemptEvidence::NotDispatched {
                confirmed_before_write: false,
            },
            AttemptEvidence::Unknown {
                reason: "stream_interleaved_frame".into(),
            },
        ] {
            let mut rs = requests(&[1]);
            reserve(&mut rs, 1, "primary").unwrap();
            let r = rs.get_mut(&1).unwrap();
            assert_eq!(
                r.settle_attempt(1, evidence.clone()).unwrap(),
                AttemptDisposition::Uncertain,
                "{evidence:?}"
            );
            assert!(r.attempts[0].reason.is_some());
            assert_eq!((r.charged_microusd(), r.unsettled()), (164001, true));
            assert!(reserve(&mut rs, 1, "secondary").is_err());
            rs.extend(requests(&[2]));
            assert_eq!(
                reserve(&mut rs, 2, "primary").unwrap_err().to_string(),
                Error::Refused("another provider request is unsettled".into()).to_string()
            );
        }
    }
    #[test]
    fn confirmed_not_dispatched_releases_and_ends_the_request() {
        let mut rs = requests(&[1]);
        reserve(&mut rs, 1, "primary").unwrap();
        let r = rs.get_mut(&1).unwrap();
        assert_eq!(
            r.settle_attempt(
                1,
                AttemptEvidence::NotDispatched {
                    confirmed_before_write: true
                }
            )
            .unwrap(),
            AttemptDisposition::NotDispatched
        );
        assert_eq!((r.charged_microusd(), r.unsettled()), (0, false));
        assert!(r.settled_microusd.is_none());
        assert!(reserve(&mut rs, 1, "secondary").is_err());
        rs.extend(requests(&[2]));
        assert_eq!(reserve(&mut rs, 2, "primary").unwrap(), 1);
    }
    #[test]
    fn relayed_attempts_keep_the_reservation_until_raw_settlement() {
        for (evidence, expected) in [
            (relayed(200, true, false), AttemptDisposition::Completed),
            (relayed(200, false, false), AttemptDisposition::Uncertain),
            (relayed(200, false, true), AttemptDisposition::Uncertain),
            (relayed(500, true, false), AttemptDisposition::Uncertain),
            // Q-a: the framed end does not excuse bytes after the terminal.
            (
                AttemptEvidence::Relayed {
                    status: 200,
                    upstream_end: true,
                    interrupted: false,
                    relay_sha256: "d".repeat(64),
                    provider_failure: None,
                    terminal_last: false,
                    drain_error: None,
                },
                AttemptDisposition::Uncertain,
            ),
            (
                AttemptEvidence::Relayed {
                    status: 200,
                    upstream_end: true,
                    interrupted: false,
                    relay_sha256: "d".repeat(64),
                    provider_failure: Some("insufficient_quota".into()),
                    terminal_last: true,
                    drain_error: None,
                },
                AttemptDisposition::Uncertain,
            ),
            (
                AttemptEvidence::Relayed {
                    status: 200,
                    upstream_end: false,
                    interrupted: false,
                    relay_sha256: "d".repeat(64),
                    provider_failure: None,
                    terminal_last: true,
                    drain_error: None,
                },
                AttemptDisposition::Completed,
            ),
            (
                AttemptEvidence::Relayed {
                    status: 200,
                    upstream_end: false,
                    interrupted: true,
                    relay_sha256: "d".repeat(64),
                    provider_failure: None,
                    terminal_last: true,
                    drain_error: None,
                },
                AttemptDisposition::Uncertain,
            ),
            (
                AttemptEvidence::Relayed {
                    status: 500,
                    upstream_end: false,
                    interrupted: false,
                    relay_sha256: "d".repeat(64),
                    provider_failure: None,
                    terminal_last: true,
                    drain_error: None,
                },
                AttemptDisposition::Uncertain,
            ),
        ] {
            let mut rs = requests(&[1]);
            reserve(&mut rs, 1, "primary").unwrap();
            let r = rs.get_mut(&1).unwrap();
            assert_eq!(r.settle_attempt(1, evidence).unwrap(), expected);
            assert_eq!((r.charged_microusd(), r.unsettled()), (164001, true));
            assert_eq!(
                r.attempts[0].relay_sha256.as_deref(),
                Some(&*"d".repeat(64))
            );
            if expected == AttemptDisposition::Uncertain {
                assert!(r.settle_completed(1, 10).is_err());
            }
        }
        // Refused, then completed on the secondary route and settled.
        let mut rs = requests(&[1]);
        reserve(&mut rs, 1, "primary").unwrap();
        rs.get_mut(&1)
            .unwrap()
            .settle_attempt(1, refusal(402, 0, true))
            .unwrap();
        reserve(&mut rs, 1, "secondary").unwrap();
        let r = rs.get_mut(&1).unwrap();
        r.settle_attempt(2, relayed(200, true, false)).unwrap();
        assert!(r.settle_completed(1, 10).is_err());
        assert!(r.settle_completed(2, 164002).is_err());
        r.settle_completed(2, 1234).unwrap();
        assert!(r.settle_completed(2, 1234).is_err());
        assert_eq!(
            (r.charged_microusd(), r.unsettled(), r.settled_microusd),
            (1234, false, Some(1234))
        );
        assert_eq!(
            reserve(&mut rs, 1, "tertiary").unwrap_err().to_string(),
            Error::Refused("attempt after request settlement".into()).to_string()
        );
        // The mirrored settlement keeps request-level completion checks valid.
        assert!(!rs.values().any(|r| r.settled_microusd.is_none()));
    }
    #[test]
    fn attempts_respect_order_limit_budget_and_identity() {
        let mut rs = requests(&[1]);
        assert!(reserve(&mut rs, 2, "primary").is_err());
        assert!(reserve_attempt(&mut rs, 1, "", "payer-a", 1, BUDGET, 3).is_err());
        assert!(reserve_attempt(&mut rs, 1, "primary", "", 1, BUDGET, 3).is_err());
        assert!(
            rs.get_mut(&1)
                .unwrap()
                .settle_attempt(1, refusal(429, 0, true))
                .is_err()
        );
        for attempt in 1..=3 {
            assert_eq!(reserve(&mut rs, 1, "route").unwrap(), attempt);
            rs.get_mut(&1)
                .unwrap()
                .settle_attempt(attempt, refusal(403, 0, true))
                .unwrap();
        }
        assert_eq!(
            reserve(&mut rs, 1, "route").unwrap_err().to_string(),
            Error::Refused("attempt limit reached".into()).to_string()
        );
        // Settled spend on other requests counts against the budget.
        let mut rs = requests(&[1, 2]);
        rs.get_mut(&1).unwrap().settled_microusd = Some(900_000);
        assert!(reserve_attempt(&mut rs, 2, "primary", "payer-a", 100_001, BUDGET, 3).is_err());
        assert_eq!(
            reserve_attempt(&mut rs, 2, "primary", "payer-a", 100_000, BUDGET, 3).unwrap(),
            1
        );
        // Records round-trip through the journal encoding.
        let back: Reservation =
            serde_json::from_value(serde_json::to_value(&rs[&2]).unwrap()).unwrap();
        assert_eq!(back.attempts, rs[&2].attempts);
    }
    #[test]
    fn relay_evidence_is_clean_only_when_every_byte_after_the_terminal_was_kept() {
        use crate::pi_stream::{Outcome, Relay};
        let done = "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r\"}}\n\n";
        let relay = |outcome, unforwarded: &str, drained, held_dropped| Relay {
            status: 200,
            outcome,
            forwarded: done.as_bytes().to_vec(),
            chunks: vec![],
            upstream_end_ms: None,
            opened_ms: 0,
            unforwarded_after_release: unforwarded.len(),
            unforwarded: unforwarded.as_bytes().to_vec(),
            drained,
            held_dropped,
            drain_error: None,
            cancel_wait_ms: None,
            closing: None,
            head_headers: vec![],
            head_ms: None,
            hold: None,
        };
        let clean = |r: &Relay| match AttemptEvidence::from_relay(r) {
            AttemptEvidence::Relayed { terminal_last, .. } => terminal_last,
            _ => unreachable!(),
        };
        let released = Outcome::Cancelled {
            upstream_end: false,
        };
        assert!(clean(&relay(released.clone(), ": c\n\n", true, 0)));
        assert!(clean(&relay(Outcome::Completed, "", false, 0)));
        // N2: released without a drain, or with held-back bytes dropped.
        assert!(!clean(&relay(released.clone(), "", false, 0)));
        assert!(!clean(&relay(released.clone(), ": c\n\n", true, 1)));
        // Q-a: bytes after the terminal fail with or without the framed end.
        assert!(!clean(&relay(Outcome::Completed, "data: {}\n\n", false, 0)));
        assert!(!clean(&relay(released.clone(), done, true, 0)));
        // M4: a drain that ended on an error is never clean.
        let mut errored = relay(released, ": c\n\n", true, 0);
        errored.drain_error = Some("upstream_chunk_line");
        assert!(!clean(&errored));
    }
}
