//! Hosted failover gate: an operator-authored, digest-bound policy applied to
//! route availability state before each send.
//!
//! The state comes from the read-only route availability view
//! (`hosted-route-availability`) in two forms: an admitted observation (the
//! view's output over other journals, bound by the runtime admission) and an
//! in-job observation (the view over this job's own state directory at the
//! decision time). For each pool the admitted observation applies only while
//! this job has no attempt on the pool, is within the policy's age bound,
//! was derived under the same table and pool declarations, and is complete;
//! otherwise the pool is read from the in-job observation alone (the router
//! default: refusal-only, no proactive skip).
//!
//! Fixed semantics: `available`, `reset_elapsed` and `unknown` permit a send;
//! a low route triggers the policy's prepare action; `exhausted` is skipped;
//! the request pauses only when no policy route can be sent to. Another
//! job's exhaustion alone never pauses without one verification send, and a
//! skip to a paying (API-funded) route on it needs that route's policy gate.
//! The gate decides; the router ([`crate::pi_route`]) checks the decision.
use super::hosted::{Endpoint, HostedRoutes};
use super::provider::{AttemptDisposition, Reservation};
use super::*;
use crate::pi_route::{
    Decision, Disposition, FailoverBinding, ResetObservation, RouteDirective, Router,
    RouterSnapshot,
};
use hosted_route_availability::{self as availability, Funding};

pub const POLICY_SCHEMA: &str = "work-engine.hosted-failover-policy.v1";
pub const OBSERVATION_SCHEMA: &str = "work-engine.hosted-failover-observation.v1";

/// What a low route triggers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrepareOnLow {
    /// Switch to the next policy route that can be sent to.
    SwitchEarly,
    /// Keep sending and journal a state preservation request.
    RequestPreservation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRoute {
    pub route: String,
    pub prepare_on_low: PrepareOnLow,
    /// This paying route may be entered by a proactive skip based only on
    /// another job's evidence.
    #[serde(default)]
    pub foreign_evidence_skip: bool,
}
/// The availability configuration the in-job observation is derived under
/// and the admitted observation must match.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Availability {
    pub max_evidence_age_ms: u64,
    pub pools: Vec<availability::Pool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailoverPolicy {
    pub schema: String,
    pub version: String,
    /// Selection order. Admitted routes not listed are never selected while
    /// the policy is in force.
    pub routes: Vec<PolicyRoute>,
    /// An admitted observation older than this at a decision is ignored.
    pub max_observation_age_ms: u64,
    pub availability: Availability,
}

/// Funding of an endpoint kind; loopback routes have none.
pub fn funding(endpoint: &str) -> Option<Funding> {
    match Endpoint::parse(endpoint)? {
        Endpoint::ChatGptCodex | Endpoint::AnthropicOauth => Some(Funding::Subscription),
        Endpoint::OpenAiResponses | Endpoint::AnthropicApi => Some(Funding::Api),
        Endpoint::Loopback(_) => None,
    }
}

impl FailoverPolicy {
    /// Parse and validate against the admitted route table.
    pub fn parse(bytes: &[u8], table: &HostedRoutes) -> Result<Self> {
        let p: Self = serde_json::from_slice(bytes)
            .map_err(|e| Error::Refused(format!("failover policy: {e}")))?;
        if p.schema != POLICY_SCHEMA || p.version.is_empty() || p.version.len() > 128 {
            return refuse("failover policy schema/version");
        }
        let config = serde_json::from_value::<availability::Config>(json!({
            "schema": availability::CONFIG_SCHEMA,
            "now_ms": 0,
            "journal_roots": [],
            "max_evidence_age_ms": p.availability.max_evidence_age_ms,
            "pools": p.availability.pools,
        }))
        .map_err(|e| Error::Refused(format!("failover policy pools: {e}")))?;
        availability::check_config(&config)
            .map_err(|e| Error::Refused(format!("failover policy pools: {e}")))?;
        let mut seen = BTreeSet::new();
        if p.routes.is_empty() {
            return refuse("failover policy names no route");
        }
        for r in &p.routes {
            if !seen.insert(r.route.as_str()) {
                return refuse("failover policy names a route twice");
            }
            let Some(t) = table.routes.iter().find(|t| t.id == r.route) else {
                return refuse(format!("failover policy route {} is not admitted", r.route));
            };
            if p.pool_index(&t.payer, &t.endpoint).is_none() {
                return refuse(format!("failover policy route {} is in no pool", r.route));
            }
            if r.foreign_evidence_skip && funding(&t.endpoint) != Some(Funding::Api) {
                return refuse(format!(
                    "failover policy route {}: foreign_evidence_skip applies to paying routes only",
                    r.route
                ));
            }
        }
        Ok(p)
    }
    fn pool_index(&self, payer: &str, endpoint: &str) -> Option<usize> {
        self.availability.pools.iter().position(|p| {
            p.members
                .iter()
                .any(|m| m.payer == payer && m.endpoint == endpoint)
        })
    }
    /// Each pool as an observation must report it.
    fn declarations(&self) -> Vec<Value> {
        self.availability
            .pools
            .iter()
            .map(|p| {
                let mut d = availability::pool_declaration(p);
                d["id"] = json!(p.id);
                d["funding"] = json!(p.funding.label());
                d
            })
            .collect()
    }
}

/// Why an admitted observation does not match `policy` (admission refuses
/// these), or `Ok`.
fn observation_mismatch(
    obs: &Value,
    policy: &FailoverPolicy,
) -> std::result::Result<(), &'static str> {
    if obs["schema"] != availability::OUTPUT_SCHEMA || !obs["now_ms"].is_u64() {
        return Err("schema");
    }
    if obs["table"]["sha256"] != availability::provisional::table_sha256() {
        return Err("table_mismatch");
    }
    let declared: Option<Vec<Value>> = obs["pools"].as_array().map(|pools| {
        pools
            .iter()
            .map(|p| {
                json!({"id":p["id"],"funding":p["funding"],"members":p["members"],
                "low_thresholds":p["low_thresholds"],"credit_cleared_ms":p["credit_cleared_ms"]})
            })
            .collect()
    });
    if obs["max_evidence_age_ms"] != policy.availability.max_evidence_age_ms
        || declared != Some(policy.declarations())
    {
        return Err("pool_mismatch");
    }
    Ok(())
}
/// Admitted failover inputs: the policy, its digest, and the admitted
/// observation with its digest.
pub type AdmittedFailover = (FailoverPolicy, String, Option<(Value, String)>);
/// Admission check of an admitted observation against the policy. Staleness
/// is not checked here: a stale observation is admitted and ignored.
pub fn check_observation(bytes: &[u8], policy: &FailoverPolicy) -> Result<Value> {
    let obs: Value = serde_json::from_slice(bytes)
        .map_err(|_| Error::Refused("state observation is not JSON".into()))?;
    observation_mismatch(&obs, policy)
        .map_err(|r| Error::Refused(format!("state observation does not match the policy: {r}")))?;
    Ok(obs)
}

/// The ledger's journaled decision on an attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailoverDecision {
    pub policy_version: String,
    pub policy_sha256: String,
    pub table_digest: String,
    pub state_observation_sha256: String,
    pub action: String,
    pub route: String,
    pub decided_ms: u64,
    #[serde(default)]
    pub reset_observation: Option<ResetObservation>,
    #[serde(default)]
    pub preservation_request: Option<Value>,
}
impl FailoverDecision {
    pub fn directive(&self, job_id: &str) -> RouteDirective {
        RouteDirective {
            job_id: job_id.into(),
            table_digest: self.table_digest.clone(),
            policy_version: self.policy_version.clone(),
            state_observation_sha256: self.state_observation_sha256.clone(),
            action: self.action.clone(),
            route: Some(self.route.clone()),
            decided_ms: self.decided_ms,
            reset_observation: self.reset_observation.clone(),
        }
    }
}
/// When a paused request's work can start again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResumeTime {
    At(u64),
    /// Always `"unknown"`: no reset is known; the operator is notified.
    Unknown(String),
}
/// The ledger's journaled pause of a request (no attempt was made).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailoverPause {
    pub policy_version: String,
    pub policy_sha256: String,
    pub table_digest: String,
    pub state_observation_sha256: String,
    pub action: String,
    pub decided_ms: u64,
    pub resume_time: ResumeTime,
    /// `{route, pool, dimension}` the resume time waits for.
    pub waits_for: Option<Value>,
    pub evidence: Option<Value>,
}
pub enum Outcome {
    Attempt(FailoverDecision),
    Pause(FailoverPause),
}

/// Whether the admitted table (and policy) lets subscription exhaustion spill
/// into API spend: an API-funded route after a subscription-funded one in the
/// effective order (policy order, else table order).
pub fn api_exposure(table: &HostedRoutes, policy: Option<&FailoverPolicy>) -> Value {
    let order: Vec<&super::hosted::HostedRoute> = match policy {
        Some(p) => p
            .routes
            .iter()
            .filter_map(|r| table.routes.iter().find(|t| t.id == r.route))
            .collect(),
        None => table.routes.iter().collect(),
    };
    let mut pairs = vec![];
    for (i, a) in order.iter().enumerate() {
        for b in &order[i + 1..] {
            if funding(&a.endpoint) == Some(Funding::Subscription)
                && funding(&b.endpoint) == Some(Funding::Api)
            {
                pairs.push(json!([a.id, b.id]));
            }
        }
    }
    let foreign: Vec<&str> = policy
        .map(|p| {
            p.routes
                .iter()
                .filter(|r| r.foreign_evidence_skip)
                .map(|r| r.route.as_str())
                .collect()
        })
        .unwrap_or_default();
    let unlisted: Vec<&str> = policy
        .map(|p| {
            table
                .routes
                .iter()
                .filter(|t| !p.routes.iter().any(|r| r.route == t.id))
                .map(|t| t.id.as_str())
                .collect()
        })
        .unwrap_or_default();
    json!({"subscription_to_api": !pairs.is_empty(), "pairs": pairs,
        "foreign_evidence_skip_to_api": foreign, "policy": policy.is_some(), "unlisted_routes": unlisted})
}

/// A pool's state at a decision, from one view output.
#[derive(Clone, Debug, Default)]
struct PoolState {
    /// `available`, `unknown`, `reset_elapsed` or `exhausted`.
    status: &'static str,
    /// Exhausted: the reset after which every exhausted dimension is usable
    /// again (`None`: unknown), with that dimension and its evidence.
    resume: Option<(u64, String, Value)>,
    /// Exhausted only on other jobs' evidence: every admitted reading, or
    /// in-job evidence naming another job.
    foreign: bool,
    /// Reset elapsed: the dimension, evidence and reset time.
    reset: Option<(String, Value, u64)>,
    /// A low dimension: name, utilization and reading evidence.
    low: Option<(String, Value, Value)>,
    /// The low reading is another job's (always, when admitted).
    low_foreign: bool,
}
fn evidence_job(e: &Value) -> Option<&str> {
    e["job_id"].as_str()
}
/// A pool's state from one view output. `admitted`: the output is the
/// admitted observation, so none of its evidence is this job's own.
fn pool_state(pool: Option<&Value>, decided_ms: u64, job: &str, admitted: bool) -> PoolState {
    let foreign = |e: &Value| admitted || evidence_job(e).is_some_and(|j| j != job);
    let mut st = PoolState {
        status: "unknown",
        ..PoolState::default()
    };
    let Some(dims) = pool.and_then(|p| p["dimensions"].as_array()) else {
        return st;
    };
    let mut exhausted = vec![];
    let mut available = false;
    for d in dims {
        let name = d["dimension"].as_str().unwrap_or_default().to_owned();
        let at = d["available_again_at"].as_u64();
        let evidence = d["evidence"].clone();
        match d["status"].as_str() {
            Some("exhausted") if at.is_none_or(|t| t > decided_ms) => {
                exhausted.push((name.clone(), at, evidence))
            }
            Some("exhausted") | Some("reset_elapsed") => {
                if st.reset.is_none()
                    && let Some(t) = at
                {
                    st.reset = Some((name.clone(), evidence, t));
                }
            }
            Some("available") => available = true,
            _ => {}
        }
        if d["low"] == true && st.low.is_none() {
            st.low_foreign = foreign(&d["utilization_evidence"]);
            st.low = Some((
                name,
                d["utilization"].clone(),
                d["utilization_evidence"].clone(),
            ));
        }
    }
    if !exhausted.is_empty() {
        st.status = "exhausted";
        st.foreign = exhausted.iter().all(|(_, _, e)| foreign(e));
        st.resume = if exhausted.iter().all(|(_, at, _)| at.is_some()) {
            exhausted
                .iter()
                .max_by_key(|(_, at, _)| *at)
                .map(|(d, at, e)| (at.unwrap(), d.clone(), e.clone()))
        } else {
            None
        };
        st.reset = None;
        st.low = None;
        st.low_foreign = false;
    } else if st.reset.is_some() {
        st.status = "reset_elapsed";
    } else if available {
        st.status = "available";
    }
    st
}

#[derive(Clone, Debug, PartialEq)]
enum Class {
    Send(Option<ResetObservation>),
    ExhaustedOwn,
    ExhaustedForeign,
    Excluded,
}
impl Class {
    fn label(&self) -> &'static str {
        match self {
            Self::Send(None) => "send_permitted",
            Self::Send(Some(_)) => "fail_back_eligible",
            Self::ExhaustedOwn => "exhausted",
            Self::ExhaustedForeign => "exhausted_foreign",
            Self::Excluded => "refused",
        }
    }
}
/// The gate's result for one decision.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    /// `send`, `skip`, `switch_early`, `request_preservation`, `fail_back`,
    /// `verify_foreign` or `pause`.
    pub action: String,
    pub route: Option<String>,
    pub reset_observation: Option<ResetObservation>,
    pub preservation_request: Option<Value>,
    pub resume_time: Option<ResumeTime>,
    pub waits_for: Option<Value>,
    pub evidence: Option<Value>,
    pub admitted_ignored: Option<&'static str>,
    /// Per policy route, as evaluated (for reading).
    pub routes: Vec<Value>,
}

/// One job's fixed failover inputs.
#[derive(Clone, Debug)]
pub struct Context {
    pub policy: FailoverPolicy,
    pub policy_sha256: String,
    pub table: HostedRoutes,
    pub job_id: String,
    /// The admitted observation and its digest.
    pub admitted: Option<(Value, String)>,
}
impl Context {
    pub fn table_digest(&self) -> String {
        self.table.digest()
    }
    pub fn binding(&self) -> FailoverBinding {
        FailoverBinding {
            job_id: self.job_id.clone(),
            table_digest: self.table_digest(),
            policy_version: self.policy.version.clone(),
        }
    }
    fn admitted_usable(&self, decided_ms: u64) -> std::result::Result<&Value, &'static str> {
        let (obs, _) = self.admitted.as_ref().ok_or("absent")?;
        observation_mismatch(obs, &self.policy)?;
        if obs["complete"] != true {
            return Err("incomplete");
        }
        let observed = obs["now_ms"].as_u64().ok_or("schema")?;
        if observed > decided_ms {
            return Err("future_dated");
        }
        if decided_ms - observed > self.policy.max_observation_age_ms {
            return Err("stale_age");
        }
        Ok(obs)
    }
    /// Apply the policy to the state for one send of `request`.
    pub fn evaluate(
        &self,
        decided_ms: u64,
        in_job: &Value,
        snapshot: &RouterSnapshot,
        preserved: &[Value],
    ) -> Evaluation {
        let admitted = self.admitted_usable(decided_ms);
        let job = self.job_id.as_str();
        let index = |id: &str| self.table.routes.iter().position(|t| t.id == id);
        let pool_of = |i: usize| {
            let t = &self.table.routes[i];
            self.policy.pool_index(&t.payer, &t.endpoint)
        };
        let view_pool = |view: &Value, pool: usize| -> Option<Value> {
            let id = &self.policy.availability.pools[pool].id;
            view["pools"]
                .as_array()?
                .iter()
                .find(|p| p["id"] == json!(id))
                .cloned()
        };
        struct Row {
            idx: usize,
            pool: usize,
            source: &'static str,
            state: PoolState,
            class: Class,
        }
        let mut rows = vec![];
        for r in &self.policy.routes {
            let (Some(idx), Some(pool)) = (index(&r.route), index(&r.route).and_then(pool_of))
            else {
                continue;
            };
            let attempted = (0..self.table.routes.len())
                .any(|j| snapshot.attempted.get(j) == Some(&true) && pool_of(j) == Some(pool));
            let (source, state) = match (&admitted, attempted) {
                (_, true) => (
                    "in_job",
                    pool_state(view_pool(in_job, pool).as_ref(), decided_ms, job, false),
                ),
                (Ok(obs), false) => (
                    "admitted",
                    pool_state(view_pool(obs, pool).as_ref(), decided_ms, job, true),
                ),
                (Err(_), false) => ("none", pool_state(None, decided_ms, job, false)),
            };
            let class = match snapshot.refused.get(idx).copied().flatten() {
                Some(refused_at) => {
                    let reset = state.reset.as_ref().and_then(|(_, e, at)| {
                        Some(ResetObservation {
                            job_id: evidence_job(e)?.to_owned(),
                            request: e["request"].as_u64()?,
                            attempt: e["attempt"].as_u64()?,
                            available_again_at: *at,
                        })
                    });
                    match reset.filter(|o| {
                        o.job_id == job
                            && (o.request, o.attempt) >= refused_at
                            && o.available_again_at <= decided_ms
                            && !snapshot
                                .verified
                                .iter()
                                .any(|(id, v)| id == &r.route && v == o)
                    }) {
                        Some(o) => Class::Send(Some(o)),
                        None => Class::Excluded,
                    }
                }
                None if state.status == "exhausted" && state.foreign => Class::ExhaustedForeign,
                None if state.status == "exhausted" => Class::ExhaustedOwn,
                None => Class::Send(None),
            };
            rows.push(Row {
                idx,
                pool,
                source,
                state,
                class,
            });
        }
        let policy_route = |row: &Row| {
            self.policy
                .routes
                .iter()
                .find(|r| r.route == self.table.routes[row.idx].id)
                .expect("policy route")
        };
        let paying_gated = |row: &Row| {
            funding(&self.table.routes[row.idx].endpoint) == Some(Funding::Api)
                && !policy_route(row).foreign_evidence_skip
        };
        let mut foreign_skipped = vec![];
        let mut target = None;
        for (k, row) in rows.iter().enumerate() {
            match row.class {
                Class::Send(_) => {
                    target = Some(k);
                    break;
                }
                Class::ExhaustedForeign => foreign_skipped.push(k),
                _ => {}
            }
        }
        let mut verify = None;
        if let Some(t) = target
            && !foreign_skipped.is_empty()
            && paying_gated(&rows[t])
        {
            target = None;
        }
        if target.is_none() && !foreign_skipped.is_empty() {
            verify = Some(foreign_skipped[0]);
        }
        let mut eval = Evaluation {
            action: String::new(),
            route: None,
            reset_observation: None,
            preservation_request: None,
            resume_time: None,
            waits_for: None,
            evidence: None,
            admitted_ignored: admitted.err(),
            routes: rows
                .iter()
                .map(|row| {
                    json!({"route": self.table.routes[row.idx].id,
                        "pool": self.policy.availability.pools[row.pool].id,
                        "source": row.source, "status": row.state.status,
                        "low": row.state.low.is_some(), "foreign": row.state.foreign,
                        "available_again_at": row.state.resume.as_ref().map(|r| r.0),
                        "class": row.class.label()})
                })
                .collect(),
        };
        let route_id = |row: &Row| self.table.routes[row.idx].id.clone();
        if let Some(t) = target {
            let row = &rows[t];
            let Class::Send(reset) = &row.class else {
                unreachable!()
            };
            eval.route = Some(route_id(row));
            eval.reset_observation = reset.clone();
            eval.action = if reset.is_some() {
                "fail_back"
            } else if rows[..t].iter().all(|r| r.class == Class::Excluded) {
                // Only refused routes precede it: the refusal path.
                "send"
            } else {
                // A policy route that is not refused was passed over.
                "skip"
            }
            .into();
            if let Some((dimension, utilization, evidence)) = &row.state.low {
                match policy_route(row).prepare_on_low {
                    PrepareOnLow::SwitchEarly => {
                        // Every route passed over on the way to the switch
                        // target: a paying target needs its gate when any of
                        // them was passed only on another job's evidence (the
                        // low route itself, or an exhausted one).
                        let next = rows[t + 1..]
                            .iter()
                            .position(|n| matches!(n.class, Class::Send(_)))
                            .map(|k| t + 1 + k);
                        let foreign_passed = |j: usize| {
                            row.state.low_foreign
                                || rows[..j].iter().any(|r| r.class == Class::ExhaustedForeign)
                        };
                        if let Some(next) = next
                            .filter(|j| !(foreign_passed(*j) && paying_gated(&rows[*j])))
                            .map(|j| &rows[j])
                        {
                            let Class::Send(reset) = &next.class else {
                                unreachable!()
                            };
                            eval.route = Some(route_id(next));
                            eval.reset_observation = reset.clone();
                            eval.action = "switch_early".into();
                        }
                    }
                    PrepareOnLow::RequestPreservation => {
                        if !preserved.contains(evidence) {
                            eval.action = "request_preservation".into();
                            eval.preservation_request = Some(json!({
                                "route": route_id(row),
                                "pool": self.policy.availability.pools[row.pool].id,
                                "dimension": dimension, "utilization": utilization,
                                "evidence": evidence}));
                        }
                    }
                }
            }
        } else if let Some(v) = verify {
            eval.action = "verify_foreign".into();
            eval.route = Some(route_id(&rows[v]));
        } else {
            eval.action = "pause".into();
            // Only resets still ahead count: `resume` holds only exhausted
            // dimensions whose reset is after `decided_ms` (`pool_state`). A
            // passed reset (also on a route refused again after it) says
            // nothing about when work can resume.
            let earliest = rows
                .iter()
                .filter_map(|row| {
                    let (at, dimension, evidence) = row.state.resume.clone()?;
                    Some((at, row, dimension, evidence))
                })
                .min_by_key(|(at, row, ..)| (*at, row.idx));
            match earliest {
                Some((at, row, dimension, evidence)) => {
                    eval.resume_time = Some(ResumeTime::At(at));
                    eval.waits_for = Some(json!({"route": route_id(row),
                        "pool": self.policy.availability.pools[row.pool].id, "dimension": dimension}));
                    eval.evidence = Some(evidence);
                }
                None => eval.resume_time = Some(ResumeTime::Unknown("unknown".into())),
            }
        }
        eval
    }
    /// The retained decision observation: the gate's inputs (admitted digest,
    /// in-job view) and its evaluated routes. Its SHA-256 is the decision's
    /// `state_observation_sha256`.
    pub fn document(
        &self,
        request: u64,
        decided_ms: u64,
        in_job: &Value,
        eval: &Evaluation,
    ) -> Vec<u8> {
        let admitted = self.admitted.as_ref();
        serde_json::to_vec(&json!({
            "schema": OBSERVATION_SCHEMA,
            "job_id": self.job_id,
            "request": request,
            "decided_ms": decided_ms,
            "admitted": {"sha256": admitted.map(|a| &a.1),
                "observed_ms": admitted.map(|a| &a.0["now_ms"]),
                "ignored": eval.admitted_ignored},
            "in_job": in_job,
            "routes": eval.routes,
            "exposure": api_exposure(&self.table, Some(&self.policy)),
        }))
        .expect("observation json")
    }
    /// Evaluate and produce the journaled outcome with its observation
    /// document.
    pub fn decide(
        &self,
        request: u64,
        decided_ms: u64,
        in_job: &Value,
        snapshot: &RouterSnapshot,
        preserved: &[Value],
    ) -> (Outcome, Vec<u8>) {
        let eval = self.evaluate(decided_ms, in_job, snapshot, preserved);
        let document = self.document(request, decided_ms, in_job, &eval);
        let sha = digest(&document);
        let outcome = match eval.route {
            Some(route) => Outcome::Attempt(FailoverDecision {
                policy_version: self.policy.version.clone(),
                policy_sha256: self.policy_sha256.clone(),
                table_digest: self.table_digest(),
                state_observation_sha256: sha,
                action: eval.action,
                route,
                decided_ms,
                reset_observation: eval.reset_observation,
                preservation_request: eval.preservation_request,
            }),
            None => Outcome::Pause(FailoverPause {
                policy_version: self.policy.version.clone(),
                policy_sha256: self.policy_sha256.clone(),
                table_digest: self.table_digest(),
                state_observation_sha256: sha,
                action: eval.action,
                decided_ms,
                resume_time: eval
                    .resume_time
                    .unwrap_or(ResumeTime::Unknown("unknown".into())),
                waits_for: eval.waits_for,
                evidence: eval.evidence,
            }),
        };
        (outcome, document)
    }
}

/// Attempts of the ledger in order, up to `request` and, within it, before
/// attempt `before` (all of its attempts when `None`).
fn ledger_prefix(
    requests: &BTreeMap<u64, Reservation>,
    request: u64,
    before: Option<u64>,
) -> impl Iterator<Item = (u64, &super::provider::AttemptRecord)> {
    requests.range(..=request).flat_map(move |(&rid, r)| {
        r.attempts
            .iter()
            .take_while(move |a| rid != request || before.is_none_or(|b| a.attempt < b))
            .map(move |a| (rid, a))
    })
}
/// What the gate needs from routing history, derived from the ledger's
/// attempt records (the journal replay reads) rather than from router-only
/// attempts that never reached the ledger. A refused route stays refused
/// until an attempt on it (a verification) follows; `verified` lists the
/// reset observations of this request's verification attempts.
pub fn ledger_snapshot(
    table: &HostedRoutes,
    requests: &BTreeMap<u64, Reservation>,
    request: u64,
    before: Option<u64>,
) -> RouterSnapshot {
    let n = table.routes.len();
    let mut refused = vec![None; n];
    let mut attempted = vec![false; n];
    let mut verified = vec![];
    for (rid, a) in ledger_prefix(requests, request, before) {
        let Some(i) = table.routes.iter().position(|t| t.id == a.route) else {
            continue;
        };
        attempted[i] = true;
        if refused[i].take().is_some()
            && rid == request
            && let Some(o) = a
                .failover
                .as_ref()
                .and_then(|d| d.reset_observation.clone())
        {
            verified.push((a.route.clone(), o));
        }
        if a.disposition == AttemptDisposition::Refused {
            refused[i] = Some((rid, a.attempt));
        }
    }
    RouterSnapshot {
        refused,
        attempted,
        verified,
    }
}
/// Evidence identities of low readings already sent as preservation
/// requests in this job, before the given point.
pub fn preserved(
    requests: &BTreeMap<u64, Reservation>,
    request: u64,
    before: Option<u64>,
) -> Vec<Value> {
    ledger_prefix(requests, request, before)
        .filter_map(|(_, a)| a.failover.as_ref()?.preservation_request.as_ref())
        .map(|p| p["evidence"].clone())
        .collect()
}
impl Context {
    /// The view over this job's own state directory at `decided_ms`, under
    /// the policy's pools.
    pub fn in_job_observation(
        &self,
        state_dir: &std::path::Path,
        decided_ms: u64,
    ) -> Result<Value> {
        let config = json!({
            "schema": availability::CONFIG_SCHEMA,
            "now_ms": decided_ms,
            "journal_roots": [state_dir],
            "max_evidence_age_ms": self.policy.availability.max_evidence_age_ms,
            "pools": self.policy.availability.pools,
        });
        let out = availability::view(&serde_json::to_vec(&config)?)
            .map_err(|e| Error::Refused(format!("in-job observation: {e}")))?;
        Ok(serde_json::from_slice(&out)?)
    }
}

/// Retained decision observation names.
pub fn observation_name(request: u64, attempt: Option<u64>) -> String {
    match attempt {
        Some(a) => format!("coding-hosted-{request}-{a}.observation.json"),
        None => format!("coding-hosted-{request}-pause.observation.json"),
    }
}

/// Reproduce every journaled decision of a job from its ledger and retained
/// decision observations (`artifact` returns digest-verified bytes by name):
/// each is re-evaluated from its recorded inputs and must give the same
/// document and outcome, and the router (fresh, bound to `ctx`) must admit
/// each attempt as journaled. The resulting attempt log must also pass
/// [`Router::recover`].
pub fn replay(
    ctx: &Context,
    mut router: Router,
    requests: &BTreeMap<u64, Reservation>,
    artifact: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> std::result::Result<(), String> {
    let model = router.policy().model.clone();
    let redecide = |request: u64,
                    attempt: Option<u64>,
                    decided_ms: u64,
                    sha: &str|
     -> std::result::Result<Outcome, String> {
        let name = observation_name(request, attempt);
        let bytes = artifact(&name).ok_or(format!("{name}: not retained"))?;
        if digest(&bytes) != sha {
            return Err(format!("{name}: digest differs from the decision"));
        }
        let doc: Value = serde_json::from_slice(&bytes).map_err(|_| format!("{name}: not JSON"))?;
        // The outcome carries the digest of the rebuilt document, so an
        // equal outcome means the same document from the same inputs.
        let (outcome, _) = ctx.decide(
            request,
            decided_ms,
            &doc["in_job"],
            &ledger_snapshot(&ctx.table, requests, request, attempt),
            &preserved(requests, request, attempt),
        );
        Ok(outcome)
    };
    for (&request, r) in requests {
        for a in &r.attempts {
            let decision = match &a.failover {
                Some(d) => {
                    match redecide(
                        request,
                        Some(a.attempt),
                        d.decided_ms,
                        &d.state_observation_sha256,
                    )? {
                        Outcome::Attempt(again) if again == *d => {}
                        _ => {
                            return Err(format!(
                                "request {request} attempt {}: decision not reproduced",
                                a.attempt
                            ));
                        }
                    }
                    router.next_with(request, &model, d.decided_ms, &d.directive(&ctx.job_id))
                }
                None => {
                    return Err(format!(
                        "request {request} attempt {}: no failover decision journaled",
                        a.attempt
                    ));
                }
            };
            let Decision::Attempt { route, attempt } = decision else {
                return Err(format!(
                    "request {request} attempt {}: router refused: {decision:?}",
                    a.attempt
                ));
            };
            if router.route(route).id != a.route {
                return Err(format!(
                    "request {request} attempt {}: router chose another route",
                    a.attempt
                ));
            }
            let disposition = match a.disposition {
                AttemptDisposition::Pending => continue,
                AttemptDisposition::Refused => Disposition::Refused {
                    status: a.http_status.unwrap_or(0),
                    code: "journal".into(),
                },
                AttemptDisposition::NotDispatched => Disposition::NotDispatched,
                AttemptDisposition::Completed => Disposition::Completed,
                AttemptDisposition::Uncertain => Disposition::Uncertain {
                    reason: a.reason.clone().unwrap_or_default(),
                },
            };
            router.settle(attempt, disposition, Value::Null)?;
        }
        if let Some(p) = &r.failover_pause {
            match redecide(request, None, p.decided_ms, &p.state_observation_sha256)? {
                Outcome::Pause(again) if again == *p => {}
                _ => return Err(format!("request {request}: pause not reproduced")),
            }
        }
    }
    let intents: Vec<_> = router
        .attempts()
        .iter()
        .map(|a| {
            let mut a = a.clone();
            a.disposition = None;
            a.usage = Value::Null;
            a
        })
        .collect();
    let settles: Vec<_> = router
        .attempts()
        .iter()
        .filter(|a| a.disposition.is_some())
        .cloned()
        .collect();
    Router::recover(
        router.policy().clone(),
        Some(ctx.binding()),
        &intents,
        &settles,
    )?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::hosted::HostedRoute;
    use super::*;

    const JOB: &str = "job-1";
    const NOW: u64 = 1_790_000_000_000;
    const H: u64 = 3_600_000;

    fn table() -> HostedRoutes {
        HostedRoutes {
            routes: vec![
                HostedRoute {
                    id: "sub".into(),
                    payer: "claude-sub".into(),
                    endpoint: "https:anthropic-oauth".into(),
                },
                HostedRoute {
                    id: "api".into(),
                    payer: "anthropic-api".into(),
                    endpoint: "https:anthropic-api".into(),
                },
            ],
            max_attempts_per_request: 3,
        }
    }
    fn policy_json() -> Value {
        json!({
            "schema": POLICY_SCHEMA,
            "version": "ops-1",
            "routes": [
                {"route": "sub", "prepare_on_low": "request_preservation"},
                {"route": "api", "prepare_on_low": "switch_early"}
            ],
            "max_observation_age_ms": H,
            "availability": {
                "max_evidence_age_ms": 7 * 24 * H,
                "pools": [
                    {"id": "claude-sub", "funding": "subscription",
                     "members": [{"payer": "claude-sub", "endpoint": "https:anthropic-oauth"}],
                     "low_thresholds": {"subscription_5h": 0.9}},
                    {"id": "anthropic-api", "funding": "api",
                     "members": [{"payer": "anthropic-api", "endpoint": "https:anthropic-api"}]}
                ]
            }
        })
    }
    fn policy_with(edit: impl FnOnce(&mut Value)) -> FailoverPolicy {
        let mut v = policy_json();
        edit(&mut v);
        FailoverPolicy::parse(&serde_json::to_vec(&v).unwrap(), &table()).unwrap()
    }
    /// A dimension state: (pool, dimension, status, available_again_at, low,
    /// evidence job, request, attempt).
    pub(crate) type Dim<'a> = (
        &'a str,
        &'a str,
        &'a str,
        Option<u64>,
        bool,
        &'a str,
        u64,
        u64,
    );
    /// A view output under `policy` at `now` with the given dimension states;
    /// every other dimension is `unknown`.
    pub(crate) fn view(policy: &FailoverPolicy, now: u64, dims: &[Dim]) -> Value {
        let table = availability::provisional::table();
        let pools: Vec<Value> = policy
            .availability
            .pools
            .iter()
            .map(|p| {
                let dimensions: Vec<Value> = table
                    .dimensions(p.funding.label())
                    .iter()
                    .map(|d| {
                        match dims.iter().find(|x| x.0 == p.id && x.1 == d) {
                            Some(&(_, _, status, at, low, job, request, attempt)) => {
                                let e = json!({"job_id":job,"request":request,"attempt":attempt,"observed_ms":now - 1});
                                json!({"dimension":d,"status":status,"available_again_at":at,
                                    "evidence":e,"low":if low {json!(true)} else {Value::Null},
                                    "utilization":if low {json!(0.95)} else {Value::Null},
                                    "utilization_evidence":if low {e.clone()} else {Value::Null}})
                            }
                            None => json!({"dimension":d,"status":"unknown","reason":"no_evidence",
                                "available_again_at":null,"evidence":null,"low":null}),
                        }
                    })
                    .collect();
                let mut v = availability::pool_declaration(p);
                v["id"] = json!(p.id);
                v["funding"] = json!(p.funding.label());
                v["dimensions"] = json!(dimensions);
                v
            })
            .collect();
        json!({"schema": availability::OUTPUT_SCHEMA, "now_ms": now,
            "max_evidence_age_ms": policy.availability.max_evidence_age_ms,
            "table": {"sha256": availability::provisional::table_sha256()},
            "pools": pools, "complete": true})
    }
    fn ctx(policy: FailoverPolicy, admitted: Option<Value>) -> Context {
        Context {
            policy_sha256: "policy-sha".into(),
            policy,
            table: table(),
            job_id: JOB.into(),
            admitted: admitted.map(|v| {
                let sha = digest(&serde_json::to_vec(&v).unwrap());
                (v, sha)
            }),
        }
    }
    fn snap(refused: [Option<(u64, u64)>; 2], attempted: [bool; 2]) -> RouterSnapshot {
        RouterSnapshot {
            refused: refused.to_vec(),
            attempted: attempted.to_vec(),
            verified: vec![],
        }
    }
    fn fresh() -> RouterSnapshot {
        snap([None, None], [false, false])
    }
    fn empty(p: &FailoverPolicy) -> Value {
        view(p, NOW, &[])
    }
    fn act(e: &Evaluation) -> (&str, Option<&str>) {
        (e.action.as_str(), e.route.as_deref())
    }

    #[test]
    fn unknown_and_reset_elapsed_are_send_permitted() {
        let p = policy_with(|_| {});
        // Cold start: no evidence anywhere.
        let c = ctx(p.clone(), None);
        let e = c.evaluate(NOW, &empty(&p), &fresh(), &[]);
        assert_eq!(act(&e), ("send", Some("sub")));
        assert_eq!(e.admitted_ignored, Some("absent"));
        // An admitted exhaustion whose reset passed by the decision.
        let obs = view(
            &p,
            NOW - 1000,
            &[(
                "claude-sub",
                "subscription_5h",
                "exhausted",
                Some(NOW),
                false,
                "job-0",
                1,
                1,
            )],
        );
        let c = ctx(p.clone(), Some(obs));
        let e = c.evaluate(NOW, &empty(&p), &fresh(), &[]);
        assert_eq!(act(&e), ("send", Some("sub")));
        assert_eq!(e.routes[0]["status"], "reset_elapsed");
        // One ms earlier it is still exhausted (another job's evidence, paying
        // target not gated: verification send on the subscription).
        let e = c.evaluate(NOW - 1, &empty(&p), &fresh(), &[]);
        assert_eq!(act(&e), ("verify_foreign", Some("sub")));
    }

    #[test]
    fn own_subscription_exhaustion_skips_to_api() {
        let p = policy_with(|_| {});
        let c = ctx(p.clone(), None);
        let in_job = view(
            &p,
            NOW,
            &[(
                "claude-sub",
                "subscription_5h",
                "exhausted",
                Some(NOW + H),
                false,
                JOB,
                1,
                1,
            )],
        );
        let e = c.evaluate(NOW, &in_job, &snap([None, None], [true, false]), &[]);
        assert_eq!(act(&e), ("skip", Some("api")));
        // The same exhaustion from the router's refusal: the router default is
        // already the API route.
        let e = c.evaluate(
            NOW,
            &in_job,
            &snap([Some((1, 1)), None], [true, false]),
            &[],
        );
        assert_eq!(act(&e), ("send", Some("api")));
    }

    #[test]
    fn low_quota_triggers_the_policy_prepare_action() {
        // Subscription: request preservation, once per low reading.
        let p = policy_with(|_| {});
        let c = ctx(p.clone(), None);
        let in_job = view(
            &p,
            NOW,
            &[(
                "claude-sub",
                "subscription_5h",
                "available",
                None,
                true,
                JOB,
                1,
                1,
            )],
        );
        let s = snap([None, None], [true, false]);
        let e = c.evaluate(NOW, &in_job, &s, &[]);
        assert_eq!(act(&e), ("request_preservation", Some("sub")));
        let request = e.preservation_request.clone().unwrap();
        assert_eq!(request["dimension"], "subscription_5h");
        assert_eq!(request["evidence"]["request"], 1);
        let e = c.evaluate(NOW, &in_job, &s, &[request["evidence"].clone()]);
        assert_eq!(act(&e), ("send", Some("sub")));
        assert!(e.preservation_request.is_none());
        // Switch early: the next allowed route.
        let p = policy_with(|v| v["routes"][0]["prepare_on_low"] = json!("switch_early"));
        let c = ctx(p.clone(), None);
        let e = c.evaluate(NOW, &in_job, &s, &[]);
        assert_eq!(act(&e), ("switch_early", Some("api")));
        // Nothing to switch to: keep sending on the low route.
        let in_job = view(
            &p,
            NOW,
            &[
                (
                    "claude-sub",
                    "subscription_5h",
                    "available",
                    None,
                    true,
                    JOB,
                    1,
                    1,
                ),
                (
                    "anthropic-api",
                    "api_credit",
                    "exhausted",
                    None,
                    false,
                    JOB,
                    1,
                    2,
                ),
            ],
        );
        let e = c.evaluate(NOW, &in_job, &snap([None, None], [true, true]), &[]);
        assert_eq!(act(&e), ("send", Some("sub")));
    }

    #[test]
    fn both_exhausted_pauses_until_the_subscription_reset() {
        let p = policy_with(|_| {});
        let c = ctx(p.clone(), None);
        let reset = NOW + 2 * H;
        let in_job = view(
            &p,
            NOW,
            &[
                (
                    "claude-sub",
                    "subscription_5h",
                    "exhausted",
                    Some(reset),
                    false,
                    JOB,
                    1,
                    1,
                ),
                (
                    "anthropic-api",
                    "api_credit",
                    "exhausted",
                    None,
                    false,
                    JOB,
                    1,
                    2,
                ),
            ],
        );
        let e = c.evaluate(
            NOW,
            &in_job,
            &snap([Some((1, 1)), Some((1, 2))], [true, true]),
            &[],
        );
        assert_eq!(act(&e), ("pause", None));
        assert_eq!(e.resume_time, Some(ResumeTime::At(reset)));
        assert_eq!(
            e.waits_for,
            Some(json!({"route":"sub","pool":"claude-sub","dimension":"subscription_5h"}))
        );
        assert_eq!(e.evidence.unwrap()["attempt"], 1);
        // Two windows exhausted: usable again only after the later reset.
        let in_job = view(
            &p,
            NOW,
            &[
                (
                    "claude-sub",
                    "subscription_5h",
                    "exhausted",
                    Some(reset),
                    false,
                    JOB,
                    1,
                    1,
                ),
                (
                    "claude-sub",
                    "subscription_weekly",
                    "exhausted",
                    Some(reset + H),
                    false,
                    JOB,
                    1,
                    1,
                ),
                (
                    "anthropic-api",
                    "api_credit",
                    "exhausted",
                    None,
                    false,
                    JOB,
                    1,
                    2,
                ),
            ],
        );
        let e = c.evaluate(
            NOW,
            &in_job,
            &snap([Some((1, 1)), Some((1, 2))], [true, true]),
            &[],
        );
        assert_eq!(e.resume_time, Some(ResumeTime::At(reset + H)));
    }

    #[test]
    fn no_usable_route_pauses_with_an_unknown_resume_time() {
        // Both refused, no reset known (a Codex-like refusal yields unknown).
        let p = policy_with(|_| {});
        let c = ctx(p.clone(), None);
        let e = c.evaluate(
            NOW,
            &empty(&p),
            &snap([Some((1, 1)), Some((1, 2))], [true, true]),
            &[],
        );
        assert_eq!(act(&e), ("pause", None));
        assert_eq!(e.resume_time, Some(ResumeTime::Unknown("unknown".into())));
        assert_eq!(e.waits_for, None);
    }

    #[test]
    fn a_refused_route_fails_back_only_on_its_own_reset_after_the_refusal() {
        let p = policy_with(|_| {});
        let c = ctx(p.clone(), None);
        let refused = snap([Some((1, 1)), None], [true, true]);
        // Reset passed, evidence is the refusal itself: fail back.
        let in_job = view(
            &p,
            NOW,
            &[(
                "claude-sub",
                "subscription_5h",
                "reset_elapsed",
                Some(NOW - 1),
                false,
                JOB,
                1,
                1,
            )],
        );
        let e = c.evaluate(NOW, &in_job, &refused, &[]);
        assert_eq!(act(&e), ("fail_back", Some("sub")));
        let o = e.reset_observation.clone().unwrap();
        assert_eq!(
            (o.request, o.attempt, o.available_again_at),
            (1, 1, NOW - 1)
        );
        // Already used for a verification in this request: not again.
        let mut spent = refused.clone();
        spent.verified = vec![("sub".into(), o)];
        assert_eq!(
            act(&c.evaluate(NOW, &in_job, &spent, &[])),
            ("send", Some("api"))
        );
        // Evidence older than the refusal, another job's, or a future reset:
        // not eligible.
        for in_job in [
            view(
                &p,
                NOW,
                &[(
                    "claude-sub",
                    "subscription_5h",
                    "reset_elapsed",
                    Some(NOW - 1),
                    false,
                    JOB,
                    0,
                    9,
                )],
            ),
            view(
                &p,
                NOW,
                &[(
                    "claude-sub",
                    "subscription_5h",
                    "reset_elapsed",
                    Some(NOW - 1),
                    false,
                    "job-0",
                    1,
                    1,
                )],
            ),
            view(
                &p,
                NOW,
                &[(
                    "claude-sub",
                    "subscription_5h",
                    "exhausted",
                    Some(NOW + 1),
                    false,
                    JOB,
                    1,
                    1,
                )],
            ),
            empty(&p),
        ] {
            assert_eq!(
                act(&c.evaluate(NOW, &in_job, &refused, &[])),
                ("send", Some("api"))
            );
        }
    }

    #[test]
    fn a_stale_or_mismatched_admitted_observation_is_ignored() {
        let p = policy_with(|_| {});
        let foreign = [(
            "claude-sub",
            "subscription_5h",
            "exhausted",
            Some(NOW + H),
            false,
            "job-0",
            1,
            1,
        )];
        let gated = policy_with(|v| v["routes"][1]["foreign_evidence_skip"] = json!(true));
        let fresh_obs = view(&gated, NOW - H, &foreign);
        // Within the bound: the gated skip happens.
        let c = ctx(gated.clone(), Some(fresh_obs.clone()));
        assert_eq!(
            act(&c.evaluate(NOW, &empty(&gated), &fresh(), &[])),
            ("skip", Some("api"))
        );
        // One ms past the bound: ignored, no proactive skip.
        let e = c.evaluate(NOW + 1, &empty(&gated), &fresh(), &[]);
        assert_eq!(act(&e), ("send", Some("sub")));
        assert_eq!(e.admitted_ignored, Some("stale_age"));
        // Older than this job's own attempt on the pool: in-job state only.
        let e = c.evaluate(NOW, &empty(&gated), &snap([None, None], [true, false]), &[]);
        assert_eq!(act(&e), ("send", Some("sub")));
        assert_eq!(e.routes[0]["source"], "in_job");
        // Incomplete, another table, other pools: ignored.
        for (edit, reason) in [
            (
                Box::new(|o: &mut Value| o["complete"] = json!(false)) as Box<dyn Fn(&mut Value)>,
                "incomplete",
            ),
            (
                Box::new(|o: &mut Value| o["table"]["sha256"] = json!("other")),
                "table_mismatch",
            ),
            (
                Box::new(|o: &mut Value| o["pools"][0]["low_thresholds"] = json!({})),
                "pool_mismatch",
            ),
            (
                Box::new(|o: &mut Value| o["max_evidence_age_ms"] = json!(1)),
                "pool_mismatch",
            ),
        ] {
            let mut obs = fresh_obs.clone();
            edit(&mut obs);
            let e = ctx(gated.clone(), Some(obs)).evaluate(NOW, &empty(&gated), &fresh(), &[]);
            assert_eq!(
                (act(&e), e.admitted_ignored),
                (("send", Some("sub")), Some(reason))
            );
        }
        let _ = p;
    }

    #[test]
    fn another_jobs_evidence_skips_to_a_paying_route_only_through_its_gate() {
        let foreign = [(
            "claude-sub",
            "subscription_5h",
            "exhausted",
            Some(NOW + H),
            false,
            "job-0",
            1,
            1,
        )];
        // Not gated: one verification send on the subscription.
        let p = policy_with(|_| {});
        let c = ctx(p.clone(), Some(view(&p, NOW, &foreign)));
        assert_eq!(
            act(&c.evaluate(NOW, &empty(&p), &fresh(), &[])),
            ("verify_foreign", Some("sub"))
        );
        // Gated: skip to the API route.
        let g = policy_with(|v| v["routes"][1]["foreign_evidence_skip"] = json!(true));
        let c = ctx(g.clone(), Some(view(&g, NOW, &foreign)));
        assert_eq!(
            act(&c.evaluate(NOW, &empty(&g), &fresh(), &[])),
            ("skip", Some("api"))
        );
        // Every policy route exhausted on foreign evidence only: never a
        // pause, one verification send first.
        let both = [
            foreign[0],
            (
                "anthropic-api",
                "api_credit",
                "exhausted",
                None,
                false,
                "job-0",
                2,
                1,
            ),
        ];
        let c = ctx(g.clone(), Some(view(&g, NOW, &both)));
        assert_eq!(
            act(&c.evaluate(NOW, &empty(&g), &fresh(), &[])),
            ("verify_foreign", Some("sub"))
        );
        // A foreign low reading: switch early to a paying route only through
        // its gate.
        let low = [(
            "claude-sub",
            "subscription_5h",
            "available",
            None,
            true,
            "job-0",
            1,
            1,
        )];
        let p = policy_with(|v| v["routes"][0]["prepare_on_low"] = json!("switch_early"));
        let c = ctx(p.clone(), Some(view(&p, NOW, &low)));
        assert_eq!(
            act(&c.evaluate(NOW, &empty(&p), &fresh(), &[])),
            ("send", Some("sub"))
        );
        let g = policy_with(|v| {
            v["routes"][0]["prepare_on_low"] = json!("switch_early");
            v["routes"][1]["foreign_evidence_skip"] = json!(true);
        });
        let c = ctx(g.clone(), Some(view(&g, NOW, &low)));
        assert_eq!(
            act(&c.evaluate(NOW, &empty(&g), &fresh(), &[])),
            ("switch_early", Some("api"))
        );
    }

    #[test]
    fn exposure_reports_subscription_to_api_spill_and_foreign_skip() {
        let t = table();
        let none = api_exposure(&t, None);
        assert_eq!(none["subscription_to_api"], true);
        assert_eq!(none["pairs"], json!([["sub", "api"]]));
        assert_eq!(none["policy"], false);
        let p = policy_with(|v| v["routes"][1]["foreign_evidence_skip"] = json!(true));
        let e = api_exposure(&t, Some(&p));
        assert_eq!(e["foreign_evidence_skip_to_api"], json!(["api"]));
        // API first, subscription after: no spill; the unlisted route is
        // reported.
        let p =
            policy_with(|v| v["routes"] = json!([{"route":"api","prepare_on_low":"switch_early"}]));
        let e = api_exposure(&t, Some(&p));
        assert_eq!(e["subscription_to_api"], false);
        assert_eq!(e["unlisted_routes"], json!(["sub"]));
        let p = policy_with(
            |v| v["routes"] = json!([{"route":"api","prepare_on_low":"switch_early"},{"route":"sub","prepare_on_low":"switch_early"}]),
        );
        assert_eq!(api_exposure(&t, Some(&p))["subscription_to_api"], false);
    }

    #[test]
    fn an_unlisted_route_is_never_selected() {
        let p =
            policy_with(|v| v["routes"] = json!([{"route":"sub","prepare_on_low":"switch_early"}]));
        let c = ctx(p.clone(), None);
        let e = c.evaluate(
            NOW,
            &empty(&p),
            &snap([Some((1, 1)), None], [true, false]),
            &[],
        );
        assert_eq!(act(&e), ("pause", None));
    }

    #[test]
    fn an_invalid_policy_is_refused() {
        let t = table();
        let bad = |edit: &dyn Fn(&mut Value)| {
            let mut v = policy_json();
            edit(&mut v);
            FailoverPolicy::parse(&serde_json::to_vec(&v).unwrap(), &t).is_err()
        };
        assert!(bad(&|v| v["schema"] = json!("other")));
        assert!(bad(&|v| v["version"] = json!("")));
        assert!(bad(&|v| v["routes"] = json!([])));
        assert!(bad(&|v| v["routes"][1]["route"] = json!("sub")));
        assert!(bad(&|v| v["routes"][1]["route"] = json!("fabricated")));
        assert!(bad(
            &|v| v["routes"][0]["foreign_evidence_skip"] = json!(true)
        ));
        assert!(bad(&|v| v["availability"]["pools"]
            .as_array_mut()
            .unwrap()
            .truncate(1)));
        assert!(bad(
            &|v| v["availability"]["pools"][0]["funding"] = json!("api")
        ));
        assert!(bad(&|v| v["routes"][0]["prepare_on_low"] = json!("pause")));
        assert!(bad(&|v| v["extra"] = json!(1)));
        assert!(!bad(&|_| {}));
    }

    #[test]
    fn an_admitted_observation_must_match_the_policy() {
        let p = policy_with(|_| {});
        let obs = view(&p, NOW, &[]);
        assert!(check_observation(&serde_json::to_vec(&obs).unwrap(), &p).is_ok());
        // Stale is admitted (ignored later), a mismatch is refused.
        let old = view(&p, 0, &[]);
        assert!(check_observation(&serde_json::to_vec(&old).unwrap(), &p).is_ok());
        for edit in [
            |o: &mut Value| o["schema"] = json!("work-engine.hosted-route-availability.v1"),
            |o: &mut Value| o["table"]["sha256"] = json!("x"),
            |o: &mut Value| o["pools"][1]["members"] = json!([]),
        ] {
            let mut o = obs.clone();
            edit(&mut o);
            assert!(check_observation(&serde_json::to_vec(&o).unwrap(), &p).is_err());
        }
        assert!(check_observation(b"not json", &p).is_err());
    }

    /// Run decisions as the Host will, with a ledger and retained documents,
    /// then replay them.
    #[test]
    fn replay_reproduces_every_decision_and_detects_tampering() {
        use super::super::provider::{AttemptRecord, reserve_attempt};
        let p = policy_with(|_| {});
        let c = ctx(p.clone(), None);
        let t = table();
        let fresh_router = || {
            Router::with_failover(
                t.router("gpt-6-luna", 4, u64::MAX).policy().clone(),
                c.binding(),
            )
        };
        let mut router = fresh_router();
        let mut ledger: BTreeMap<u64, Reservation> = BTreeMap::new();
        let mut store: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        let mut preserved: Vec<Value> = vec![];
        // (request, in-job view at the decision, disposition if attempted)
        let reset = NOW + H;
        let steps: Vec<(u64, u64, Value, AttemptDisposition)> = vec![
            // Request 1: send on the subscription, refused (5h reset known),
            // then the API route, completed.
            (1, NOW, empty(&p), AttemptDisposition::Refused),
            (
                1,
                NOW,
                view(
                    &p,
                    NOW,
                    &[(
                        "claude-sub",
                        "subscription_5h",
                        "exhausted",
                        Some(reset),
                        false,
                        JOB,
                        1,
                        1,
                    )],
                ),
                AttemptDisposition::Completed,
            ),
            // Request 2 after the reset: fail back to the subscription.
            (
                2,
                reset,
                view(
                    &p,
                    reset,
                    &[(
                        "claude-sub",
                        "subscription_5h",
                        "reset_elapsed",
                        Some(reset),
                        false,
                        JOB,
                        1,
                        1,
                    )],
                ),
                AttemptDisposition::Completed,
            ),
            // Request 3: low subscription, preservation requested.
            (
                3,
                reset,
                view(
                    &p,
                    reset,
                    &[(
                        "claude-sub",
                        "subscription_5h",
                        "available",
                        None,
                        true,
                        JOB,
                        2,
                        1,
                    )],
                ),
                AttemptDisposition::Completed,
            ),
            // Request 4: the same low reading: no second request.
            (
                4,
                reset,
                view(
                    &p,
                    reset,
                    &[(
                        "claude-sub",
                        "subscription_5h",
                        "available",
                        None,
                        true,
                        JOB,
                        2,
                        1,
                    )],
                ),
                AttemptDisposition::Completed,
            ),
        ];
        for (request, at, in_job, disposition) in steps {
            ledger.entry(request).or_default();
            let (outcome, doc) =
                c.decide(request, at, &in_job, &router.snapshot(request), &preserved);
            let Outcome::Attempt(d) = outcome else {
                panic!("pause")
            };
            let Decision::Attempt {
                attempt: routed, ..
            } = router.next_with(request, "gpt-6-luna", at, &d.directive(JOB))
            else {
                panic!("router refused {d:?}")
            };
            let a = reserve_attempt(
                &mut ledger,
                request,
                &d.route,
                if d.route == "sub" {
                    "claude-sub"
                } else {
                    "anthropic-api"
                },
                1,
                100,
                3,
            )
            .unwrap();
            store.insert(observation_name(request, Some(a)), doc);
            if let Some(p) = &d.preservation_request {
                preserved.push(p["evidence"].clone());
            }
            let rec: &mut AttemptRecord = ledger
                .get_mut(&request)
                .unwrap()
                .attempts
                .last_mut()
                .unwrap();
            rec.failover = Some(d);
            rec.disposition = disposition;
            rec.http_status = Some(if disposition == AttemptDisposition::Refused {
                429
            } else {
                200
            });
            if disposition == AttemptDisposition::Completed {
                rec.settled_microusd = Some(1);
            }
            router
                .settle(
                    routed,
                    match disposition {
                        AttemptDisposition::Refused => Disposition::Refused {
                            status: 429,
                            code: "x".into(),
                        },
                        _ => Disposition::Completed,
                    },
                    Value::Null,
                )
                .unwrap();
        }
        let actions: Vec<_> = ledger
            .values()
            .flat_map(|r| {
                r.attempts
                    .iter()
                    .map(|a| a.failover.as_ref().unwrap().action.clone())
            })
            .collect();
        assert_eq!(
            actions,
            ["send", "send", "fail_back", "request_preservation", "send"]
        );
        // Request 5: everything exhausted -> pause, journaled on the request.
        ledger.entry(5).or_default();
        let in_job = view(
            &p,
            reset,
            &[
                (
                    "claude-sub",
                    "subscription_weekly",
                    "exhausted",
                    Some(reset + 5 * H),
                    false,
                    JOB,
                    3,
                    1,
                ),
                (
                    "anthropic-api",
                    "api_credit",
                    "exhausted",
                    None,
                    false,
                    JOB,
                    1,
                    2,
                ),
            ],
        );
        let (outcome, doc) = c.decide(5, reset, &in_job, &router.snapshot(5), &preserved);
        let Outcome::Pause(pause) = outcome else {
            panic!("not a pause")
        };
        assert_eq!(pause.resume_time, ResumeTime::At(reset + 5 * H));
        store.insert(observation_name(5, None), doc);
        ledger
            .get_mut(&5)
            .unwrap()
            .record_failover_pause(pause)
            .unwrap();

        let read = |s: &BTreeMap<String, Vec<u8>>| {
            let s = s.clone();
            move |name: &str| s.get(name).cloned()
        };
        replay(&c, fresh_router(), &ledger, &read(&store)).unwrap();

        // A changed document, a changed decision, a missing document, or a
        // router bound elsewhere: not reproduced.
        let mut tampered = store.clone();
        let name = observation_name(2, Some(1));
        let mut doc: Value = serde_json::from_slice(&tampered[&name]).unwrap();
        doc["in_job"] = empty(&p);
        tampered.insert(name.clone(), serde_json::to_vec(&doc).unwrap());
        assert!(replay(&c, fresh_router(), &ledger, &read(&tampered)).is_err());
        let mut other = ledger.clone();
        other.get_mut(&1).unwrap().attempts[1]
            .failover
            .as_mut()
            .unwrap()
            .action = "skip".into();
        assert!(replay(&c, fresh_router(), &other, &read(&store)).is_err());
        let mut undecided = ledger.clone();
        undecided.get_mut(&1).unwrap().attempts[0].failover = None;
        assert!(replay(&c, fresh_router(), &undecided, &read(&store)).is_err());
        let mut missing = store.clone();
        missing.remove(&observation_name(5, None));
        assert!(replay(&c, fresh_router(), &ledger, &read(&missing)).is_err());
        let mut other = ledger.clone();
        other
            .get_mut(&5)
            .unwrap()
            .failover_pause
            .as_mut()
            .unwrap()
            .resume_time = ResumeTime::Unknown("unknown".into());
        assert!(replay(&c, fresh_router(), &other, &read(&store)).is_err());
        let unbound = Router::new(t.router("gpt-6-luna", 4, u64::MAX).policy().clone());
        assert!(replay(&c, unbound, &ledger, &read(&store)).is_err());
    }

    #[test]
    fn admission_binds_both_failover_digests_or_neither() {
        let (dir, h) = super::super::provider::ledger_tests::fixture();
        let mut plan = h
            .journal
            .job
            .coding
            .as_ref()
            .unwrap()
            .provider
            .clone()
            .unwrap();
        plan.hosted = table();
        // Absent inputs: today's behaviour, and no digest may be admitted.
        assert!(plan.failover_admitted(None, None).unwrap().is_none());
        assert!(plan.failover_admitted(Some("x"), None).is_err());
        assert!(plan.failover_admitted(None, Some("x")).is_err());
        let file = |name: &str, bytes: &[u8]| {
            let path = dir.path().join(name);
            fs::write(&path, bytes).unwrap();
            Artifact {
                path,
                sha256: digest(bytes),
            }
        };
        let policy = file("policy.json", &serde_json::to_vec(&policy_json()).unwrap());
        let p = policy_with(|_| {});
        let obs = file(
            "observation.json",
            &serde_json::to_vec(&view(&p, NOW, &[])).unwrap(),
        );
        plan.failover = Some(super::super::provider::FailoverInputs {
            policy: policy.clone(),
            observation: Some(obs.clone()),
        });
        let (got, sha, observed) = plan
            .failover_admitted(Some(&policy.sha256), Some(&obs.sha256))
            .unwrap()
            .unwrap();
        assert_eq!(
            (got.version.as_str(), sha.as_str()),
            ("ops-1", policy.sha256.as_str())
        );
        assert_eq!(observed.unwrap().1, obs.sha256);
        // One side missing or another digest: refused.
        for (ps, os) in [
            (None, Some(obs.sha256.as_str())),
            (Some(policy.sha256.as_str()), None),
            (Some("other"), Some(obs.sha256.as_str())),
            (Some(policy.sha256.as_str()), Some("other")),
        ] {
            assert!(plan.failover_admitted(ps, os).is_err());
        }
        // An observation without a policy is never admitted.
        plan.failover = None;
        assert!(plan.failover_admitted(None, Some(&obs.sha256)).is_err());
        // A policy naming a route outside the table, or an observation for
        // other pools, is refused at admission.
        let mut bad = policy_json();
        bad["routes"][0]["route"] = json!("other");
        let bad = file("bad.json", &serde_json::to_vec(&bad).unwrap());
        plan.failover = Some(super::super::provider::FailoverInputs {
            policy: bad.clone(),
            observation: None,
        });
        assert!(plan.failover_admitted(Some(&bad.sha256), None).is_err());
        let mut other = view(&p, NOW, &[]);
        other["pools"][0]["members"] = json!([]);
        let other = file("other.json", &serde_json::to_vec(&other).unwrap());
        plan.failover = Some(super::super::provider::FailoverInputs {
            policy: policy.clone(),
            observation: Some(other.clone()),
        });
        assert!(
            plan.failover_admitted(Some(&policy.sha256), Some(&other.sha256))
                .is_err()
        );
        // The plan without failover inputs serializes as before.
        plan.failover = None;
        assert!(
            serde_json::to_value(&plan)
                .unwrap()
                .get("failover")
                .is_none()
        );
    }

    #[test]
    fn a_pause_ends_the_request_without_an_attempt() {
        use super::super::provider::reserve_attempt;
        let pause = FailoverPause {
            policy_version: "ops-1".into(),
            policy_sha256: "p".into(),
            table_digest: "t".into(),
            state_observation_sha256: "o".into(),
            action: "pause".into(),
            decided_ms: NOW,
            resume_time: ResumeTime::Unknown("unknown".into()),
            waits_for: None,
            evidence: None,
        };
        let mut ledger: BTreeMap<u64, Reservation> = BTreeMap::new();
        ledger.insert(1, Reservation::default());
        let r = ledger.get_mut(&1).unwrap();
        r.record_failover_pause(pause.clone()).unwrap();
        assert_eq!(r.settled_microusd, Some(0));
        assert!(!r.unsettled());
        assert!(r.record_failover_pause(pause.clone()).is_err());
        assert!(reserve_attempt(&mut ledger, 1, "sub", "claude-sub", 1, 100, 3).is_err());
        let json = serde_json::to_value(&ledger[&1]).unwrap();
        assert_eq!(json["failover_pause"]["resume_time"], "unknown");
        // After a refusal the pause ends the request; after any other
        // outcome it is refused.
        ledger.insert(2, Reservation::default());
        reserve_attempt(&mut ledger, 2, "sub", "claude-sub", 1, 100, 3).unwrap();
        let r = ledger.get_mut(&2).unwrap();
        assert!(
            r.record_failover_pause(pause.clone()).is_err(),
            "pending attempt"
        );
        r.attempts[0].disposition = AttemptDisposition::Refused;
        r.record_failover_pause(pause.clone()).unwrap();
        assert_eq!(r.settled_microusd, None);
        assert!(!r.unsettled());
        assert!(reserve_attempt(&mut ledger, 2, "api", "anthropic-api", 1, 100, 3).is_err());
        let mut uncertain = Reservation {
            attempts: ledger[&2].attempts.clone(),
            ..Reservation::default()
        };
        uncertain.attempts[0].disposition = AttemptDisposition::Uncertain;
        assert!(uncertain.record_failover_pause(pause.clone()).is_err());
        let mut wrong = Reservation::default();
        assert!(
            wrong
                .record_failover_pause(FailoverPause {
                    action: "send".into(),
                    ..pause
                })
                .is_err()
        );
    }

    #[test]
    fn a_pause_waits_for_the_earliest_reset_among_policy_routes() {
        let p = policy_with(|_| {});
        let c = ctx(p.clone(), None);
        let all = snap([None, None], [true, true]);
        let in_job = view(
            &p,
            NOW,
            &[
                (
                    "claude-sub",
                    "subscription_5h",
                    "exhausted",
                    Some(NOW + 2 * H),
                    false,
                    JOB,
                    1,
                    1,
                ),
                (
                    "anthropic-api",
                    "api_rate",
                    "exhausted",
                    Some(NOW + H),
                    false,
                    JOB,
                    1,
                    2,
                ),
            ],
        );
        let e = c.evaluate(NOW, &in_job, &all, &[]);
        assert_eq!(e.resume_time, Some(ResumeTime::At(NOW + H)));
        assert_eq!(e.waits_for.unwrap()["route"], "api");
    }

    #[test]
    fn the_ledger_snapshot_mirrors_refusals_verifications_and_attempts() {
        use super::super::provider::reserve_attempt;
        let t = table();
        let mut ledger: BTreeMap<u64, Reservation> = BTreeMap::new();
        ledger.insert(1, Reservation::default());
        reserve_attempt(&mut ledger, 1, "sub", "claude-sub", 1, 100, 3).unwrap();
        ledger.get_mut(&1).unwrap().attempts[0].disposition = AttemptDisposition::Refused;
        let s = ledger_snapshot(&t, &ledger, 1, None);
        assert_eq!(
            (s.refused.clone(), s.attempted.clone()),
            (vec![Some((1, 1)), None], vec![true, false])
        );
        // A verification attempt on the refused route clears the refusal and
        // records its reset observation for this request only.
        let o = ResetObservation {
            job_id: JOB.into(),
            request: 1,
            attempt: 1,
            available_again_at: NOW,
        };
        reserve_attempt(&mut ledger, 1, "sub", "claude-sub", 1, 100, 3).unwrap();
        let a = &mut ledger.get_mut(&1).unwrap().attempts[1];
        a.disposition = AttemptDisposition::Refused;
        a.failover = Some(FailoverDecision {
            policy_version: "ops-1".into(),
            policy_sha256: "p".into(),
            table_digest: "t".into(),
            state_observation_sha256: "o".into(),
            action: "fail_back".into(),
            route: "sub".into(),
            decided_ms: NOW,
            reset_observation: Some(o.clone()),
            preservation_request: None,
        });
        let s = ledger_snapshot(&t, &ledger, 1, None);
        assert_eq!(s.refused[0], Some((1, 2)));
        assert_eq!(s.verified, vec![("sub".to_string(), o)]);
        // Before attempt 2 the snapshot is the earlier one.
        assert_eq!(
            ledger_snapshot(&t, &ledger, 1, Some(2)).refused[0],
            Some((1, 1))
        );
        assert!(ledger_snapshot(&t, &ledger, 1, Some(2)).verified.is_empty());
        ledger.insert(2, Reservation::default());
        assert!(ledger_snapshot(&t, &ledger, 2, None).verified.is_empty());
    }

    fn table3() -> HostedRoutes {
        let mut t = table();
        t.routes.insert(
            1,
            HostedRoute {
                id: "sub2".into(),
                payer: "claude-sub2".into(),
                endpoint: "https:anthropic-oauth".into(),
            },
        );
        t
    }
    /// Three routes `sub` (low on this job's reading), `sub2` (exhausted on
    /// another job's admitted evidence) and `api`, in `order`.
    fn three_routes(order: &[&str], api_gate: bool, sub2_exhausted: bool) -> Evaluation {
        let mut v = policy_json();
        v["routes"] = json!(
            order
                .iter()
                .map(|r| json!({"route": r, "prepare_on_low": "switch_early",
                "foreign_evidence_skip": *r == "api" && api_gate}))
                .collect::<Vec<_>>()
        );
        v["availability"]["pools"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": "claude-sub2", "funding": "subscription",
                "members": [{"payer": "claude-sub2", "endpoint": "https:anthropic-oauth"}]}));
        let t = table3();
        let p = FailoverPolicy::parse(&serde_json::to_vec(&v).unwrap(), &t).unwrap();
        let foreign: Vec<Dim> = if sub2_exhausted {
            vec![(
                "claude-sub2",
                "subscription_5h",
                "exhausted",
                Some(NOW + H),
                false,
                "job-0",
                1,
                1,
            )]
        } else {
            vec![]
        };
        let c = Context {
            policy_sha256: "p".into(),
            table: t,
            job_id: JOB.into(),
            admitted: Some((view(&p, NOW, &foreign), "obs".into())),
            policy: p.clone(),
        };
        let in_job = view(
            &p,
            NOW,
            &[(
                "claude-sub",
                "subscription_5h",
                "available",
                None,
                true,
                JOB,
                1,
                1,
            )],
        );
        let s = RouterSnapshot {
            refused: vec![None; 3],
            attempted: vec![true, false, false],
            verified: vec![],
        };
        c.evaluate(NOW, &in_job, &s, &[])
    }

    #[test]
    fn an_early_switch_past_another_jobs_exhaustion_to_a_paying_route_needs_its_gate() {
        // The switch would pass sub2 only on another job's evidence.
        assert_eq!(
            act(&three_routes(&["sub", "sub2", "api"], false, true)),
            ("send", Some("sub"))
        );
        assert_eq!(
            act(&three_routes(&["sub", "sub2", "api"], true, true)),
            ("switch_early", Some("api"))
        );
        // Without that evidence the switch goes to sub2, no API spend.
        assert_eq!(
            act(&three_routes(&["sub", "sub2", "api"], false, false)),
            ("switch_early", Some("sub2"))
        );
        // sub2 first: skipped on foreign evidence before the low route.
        assert_eq!(
            act(&three_routes(&["sub2", "sub", "api"], false, true)),
            ("skip", Some("sub"))
        );
        assert_eq!(
            act(&three_routes(&["sub2", "sub", "api"], true, true)),
            ("switch_early", Some("api"))
        );
    }

    /// The API route failed back on its rate-limit reset and was refused
    /// again (an authentication failure adds no observation, so its passed
    /// reset stays the newest); the subscription is exhausted until T1.
    #[test]
    fn a_pause_never_waits_for_a_reset_that_has_passed() {
        let p = policy_with(|v| {
            v["routes"] = json!([{"route":"api","prepare_on_low":"switch_early"},
                {"route":"sub","prepare_on_low":"switch_early"}])
        });
        let c = ctx(p.clone(), None);
        let refused = snap([Some((2, 2)), Some((2, 1))], [true, true]);
        let t1 = NOW + H;
        let with_sub = |at: Option<u64>| {
            view(
                &p,
                NOW,
                &[
                    (
                        "anthropic-api",
                        "api_rate",
                        "reset_elapsed",
                        Some(NOW - 1),
                        false,
                        JOB,
                        1,
                        1,
                    ),
                    (
                        "claude-sub",
                        "subscription_5h",
                        "exhausted",
                        at,
                        false,
                        JOB,
                        2,
                        2,
                    ),
                ],
            )
        };
        let e = c.evaluate(NOW, &with_sub(Some(t1)), &refused, &[]);
        assert_eq!(act(&e), ("pause", None));
        assert_eq!(e.resume_time, Some(ResumeTime::At(t1)));
        assert_eq!(e.waits_for.unwrap()["route"], "sub");
        let e = c.evaluate(NOW, &with_sub(None), &refused, &[]);
        assert_eq!(e.resume_time, Some(ResumeTime::Unknown("unknown".into())));
        assert_eq!(e.waits_for, None);
    }

    #[test]
    fn admitted_evidence_is_never_this_jobs_own() {
        // Admitted evidence naming this job's id, or no job: still foreign.
        let p = policy_with(|_| {});
        for job in [JOB, ""] {
            let mut obs = view(
                &p,
                NOW,
                &[(
                    "claude-sub",
                    "subscription_5h",
                    "exhausted",
                    Some(NOW + H),
                    false,
                    job,
                    1,
                    1,
                )],
            );
            if job.is_empty() {
                obs["pools"][0]["dimensions"][0]["evidence"]
                    .as_object_mut()
                    .unwrap()
                    .remove("job_id");
            }
            let e = ctx(p.clone(), Some(obs)).evaluate(NOW, &empty(&p), &fresh(), &[]);
            assert_eq!(act(&e), ("verify_foreign", Some("sub")), "job {job:?}");
        }
        // An admitted low reading switches early to a paying route only
        // through its gate.
        let p = policy_with(|v| v["routes"][0]["prepare_on_low"] = json!("switch_early"));
        let obs = view(
            &p,
            NOW,
            &[(
                "claude-sub",
                "subscription_5h",
                "available",
                None,
                true,
                JOB,
                1,
                1,
            )],
        );
        let e = ctx(p.clone(), Some(obs)).evaluate(NOW, &empty(&p), &fresh(), &[]);
        assert_eq!(act(&e), ("send", Some("sub")));
    }

    #[test]
    fn a_future_dated_admitted_observation_is_ignored() {
        let g = policy_with(|v| v["routes"][1]["foreign_evidence_skip"] = json!(true));
        let foreign = [(
            "claude-sub",
            "subscription_5h",
            "exhausted",
            Some(NOW + H),
            false,
            "job-0",
            1,
            1,
        )];
        let c = ctx(g.clone(), Some(view(&g, NOW + 1, &foreign)));
        let e = c.evaluate(NOW, &empty(&g), &fresh(), &[]);
        assert_eq!(act(&e), ("send", Some("sub")));
        assert_eq!(e.admitted_ignored, Some("future_dated"));
        // At its own time it applies.
        assert_eq!(
            act(&c.evaluate(NOW + 1, &empty(&g), &fresh(), &[])),
            ("skip", Some("api"))
        );
    }
}
