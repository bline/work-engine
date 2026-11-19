//! Bounded route selection and authorized funding failover for admitted
//! inference requests.
//!
//! Mechanical admission only: the requested model is fixed (no substitution),
//! candidate routes and their order come from a pinned policy, and a switch
//! happens only after a definitive refusal that produced no output. Anything
//! that may have entered the provider or reached the consumer is uncertain and
//! is returned to the continuation owner instead of being retried. Reservations
//! are conservative: only definitive refusals release them. Leaving a route
//! requires its definitive refusal or a bound failover decision
//! ([`RouteDirective`]); limits and revocation stop admission.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub id: String,
    /// Nonsecret connection identity (not a credential).
    pub connection: String,
    pub model: String,
    pub payer: String,
    /// Listed but not permitted routes are never selected.
    pub authorized: bool,
    pub revoked: bool,
    pub max_attempts: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub model: String,
    /// Preference order.
    pub routes: Vec<Route>,
    pub max_attempts_per_request: u32,
    pub allowance_units: u64,
    pub reserve_units: u64,
    pub expires_ms: u64,
}
impl Policy {
    pub fn digest(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("policy json"))
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Disposition {
    Completed,
    /// Definitive refusal before any output reached the consumer.
    Refused {
        status: u16,
        code: String,
    },
    /// Provider possibly entered or output partly adopted.
    Uncertain {
        reason: String,
    },
    /// The consumer cancelled after admission but before any upstream byte
    /// was written: known not entered. Releases the reservation; the request
    /// is over (no switch).
    NotDispatched,
}
/// HTTP statuses treated as definitive funding/authorization refusals when
/// observed at the response head, before anything is relayed.
pub fn refusal_status(status: u16) -> bool {
    matches!(status, 401 | 402 | 403 | 429)
}

/// The job, admitted route table and failover policy a router's decision
/// inputs must name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailoverBinding {
    pub job_id: String,
    /// `hosted_routes_sha256` of the admitted table.
    pub table_digest: String,
    pub policy_version: String,
}
/// A bound observation that a route's reset has passed: the evidence attempt
/// (job, request, per-request attempt ordinal) and the reset time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResetObservation {
    pub job_id: String,
    pub request: u64,
    pub attempt: u64,
    pub available_again_at: u64,
}
/// Deterministic decision input for one send, produced by the failover gate.
/// It can select only an admitted route; a route this router saw refuse needs
/// a [`ResetObservation`] of this job at or after that refusal whose reset
/// has passed, usable once per request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteDirective {
    pub job_id: String,
    pub table_digest: String,
    pub policy_version: String,
    pub state_observation_sha256: String,
    /// `send`, `skip`, `switch_early`, `request_preservation`, `fail_back`,
    /// `verify_foreign`, or `pause` (no attempt).
    pub action: String,
    pub route: Option<String>,
    pub decided_ms: u64,
    #[serde(default)]
    pub reset_observation: Option<ResetObservation>,
}
/// What the failover gate needs from the router for one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterSnapshot {
    /// Per route (policy order): the refusal, as `(request, ordinal)`, that
    /// makes it unavailable.
    pub refused: Vec<Option<(u64, u64)>>,
    /// Per route: attempted at least once in this job.
    pub attempted: Vec<bool>,
    /// Reset observations already used for a verification attempt in this
    /// request, with their route.
    pub verified: Vec<(String, ResetObservation)>,
    /// The route the router selects without a decision input, if any.
    pub default_route: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub attempt: u64,
    pub request: u64,
    pub predecessor: Option<u64>,
    pub reason: String,
    pub route: String,
    pub connection: String,
    pub model: String,
    pub payer: String,
    pub policy: String,
    pub reserved_units: u64,
    pub disposition: Option<Disposition>,
    pub usage: Value,
    /// The bound failover decision this attempt was admitted under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<RouteDirective>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Attempt { route: usize, attempt: u64 },
    Unavailable(&'static str),
}

/// Per route: attempts used, and the refusal that made it unavailable (its
/// code and `(request, ordinal)`). Without a failover decision input a
/// refused route stays unavailable for the rest of the job (stable routing;
/// no needless switching back).
#[derive(Debug, Clone, Default)]
struct RouteState {
    attempts: u32,
    refusal: Option<(String, (u64, u64))>,
}
pub struct Router {
    policy: Policy,
    digest: String,
    reserved: u64,
    attempts: Vec<Attempt>,
    state: Vec<RouteState>,
    failover: Option<FailoverBinding>,
    /// `(request, route, observation)` used for verification attempts.
    verified: Vec<(u64, String, ResetObservation)>,
}
impl Router {
    pub fn new(policy: Policy) -> Self {
        let n = policy.routes.len();
        Self {
            digest: policy.digest(),
            policy,
            reserved: 0,
            attempts: vec![],
            state: vec![RouteState::default(); n],
            failover: None,
            verified: vec![],
        }
    }
    /// A router that also accepts decision inputs bound to `binding`.
    pub fn with_failover(policy: Policy, binding: FailoverBinding) -> Self {
        let mut r = Self::new(policy);
        r.failover = Some(binding);
        r
    }
    pub fn failover(&self) -> Option<&FailoverBinding> {
        self.failover.as_ref()
    }
    pub fn policy(&self) -> &Policy {
        &self.policy
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn reserved_units(&self) -> u64 {
        self.reserved
    }
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }
    pub fn route(&self, i: usize) -> &Route {
        &self.policy.routes[i]
    }
    pub fn last(&self) -> Option<&Attempt> {
        self.attempts.last()
    }
    fn eligible(&self, i: usize) -> bool {
        let r = &self.policy.routes[i];
        r.authorized && r.model == self.policy.model
    }
    /// The first statically eligible route without a recorded refusal.
    fn default_route(&self) -> Option<usize> {
        (0..self.policy.routes.len())
            .find(|i| self.eligible(*i) && self.state[*i].refusal.is_none())
    }
    pub fn snapshot(&self, request: u64) -> RouterSnapshot {
        RouterSnapshot {
            refused: self
                .state
                .iter()
                .map(|s| s.refusal.as_ref().map(|r| r.1))
                .collect(),
            attempted: self.state.iter().map(|s| s.attempts > 0).collect(),
            verified: self
                .verified
                .iter()
                .filter(|v| v.0 == request)
                .map(|v| (v.1.clone(), v.2.clone()))
                .collect(),
            default_route: self.default_route(),
        }
    }
    /// Admit the next attempt for `request` or explain why none is permitted.
    pub fn next(&mut self, request: u64, model: &str, now_ms: u64) -> Decision {
        self.admit(request, model, now_ms, None, true)
    }
    /// Admit the next attempt for `request` under a bound failover decision.
    /// A `pause` decision admits nothing.
    pub fn next_with(
        &mut self,
        request: u64,
        model: &str,
        now_ms: u64,
        directive: &RouteDirective,
    ) -> Decision {
        self.admit(request, model, now_ms, Some(directive), true)
    }
    fn admit(
        &mut self,
        request: u64,
        model: &str,
        now_ms: u64,
        directive: Option<&RouteDirective>,
        check_expiry: bool,
    ) -> Decision {
        if check_expiry && now_ms >= self.policy.expires_ms {
            return Decision::Unavailable("policy_stale");
        }
        if model != self.policy.model {
            return Decision::Unavailable("model_incompatible");
        }
        if let Some(d) = directive {
            let Some(b) = &self.failover else {
                return Decision::Unavailable("decision_without_binding");
            };
            if (&d.job_id, &d.table_digest, &d.policy_version)
                != (&b.job_id, &b.table_digest, &b.policy_version)
            {
                return Decision::Unavailable("decision_binding_mismatch");
            }
        }
        let prior: Vec<&Attempt> = self
            .attempts
            .iter()
            .filter(|a| a.request == request)
            .collect();
        let predecessor = prior.last().map(|a| (a.attempt, a.disposition.clone()));
        let refused_reason = match &predecessor {
            None => None,
            Some((_, None)) => return Decision::Unavailable("attempt_unsettled"),
            Some((_, Some(Disposition::Completed))) => {
                return Decision::Unavailable("request_completed");
            }
            Some((_, Some(Disposition::Uncertain { .. }))) => {
                return Decision::Unavailable("uncertain_predecessor");
            }
            Some((_, Some(Disposition::NotDispatched))) => {
                return Decision::Unavailable("request_cancelled");
            }
            Some((_, Some(Disposition::Refused { status, code }))) => {
                Some(format!("failover-after-{status}-{code}"))
            }
        };
        if directive.is_some_and(|d| d.action == "pause") {
            return Decision::Unavailable("failover_pause");
        }
        if prior.len() as u32 >= self.policy.max_attempts_per_request {
            return Decision::Unavailable("attempt_limit");
        }
        // Without a decision input the active route is the first statically
        // eligible route (authorized, same model) without a recorded
        // definitive refusal. Moving past a route requires such a refusal or
        // a bound decision; revocation or an exhausted per-route limit stops
        // admission instead of switching.
        let mut verification = None;
        let route = match directive {
            None => match self.default_route() {
                Some(i) => i,
                None => return Decision::Unavailable("no_permitted_route"),
            },
            Some(d) => {
                let Some(i) = d
                    .route
                    .as_ref()
                    .and_then(|id| self.policy.routes.iter().position(|r| &r.id == id))
                    .filter(|i| self.eligible(*i))
                else {
                    return Decision::Unavailable("decision_route_not_admitted");
                };
                if let Some((_, refused_at)) = &self.state[i].refusal {
                    let job = &self.failover.as_ref().expect("bound above").job_id;
                    let Some(o) = d.reset_observation.as_ref().filter(|o| {
                        &o.job_id == job
                            && (o.request, o.attempt) >= *refused_at
                            && o.available_again_at <= now_ms
                    }) else {
                        return Decision::Unavailable("refused_route_without_reset_observation");
                    };
                    let id = &self.policy.routes[i].id;
                    if self
                        .verified
                        .iter()
                        .any(|v| v.0 == request && &v.1 == id && v.2 == *o)
                    {
                        return Decision::Unavailable("verification_spent");
                    }
                    verification = Some((request, id.clone(), o.clone()));
                }
                i
            }
        };
        let candidate = &self.policy.routes[route];
        if candidate.revoked {
            return Decision::Unavailable("route_revoked");
        }
        if self.state[route].attempts >= candidate.max_attempts {
            return Decision::Unavailable("route_limit");
        }
        let Some(reserved) = self
            .reserved
            .checked_add(self.policy.reserve_units)
            .filter(|r| *r <= self.policy.allowance_units)
        else {
            return Decision::Unavailable("allowance_exhausted");
        };
        self.reserved = reserved;
        self.state[route].attempts += 1;
        if let Some(v) = verification {
            // The verification attempt re-enables the route; a new refusal
            // marks it again with a newer identity.
            self.state[route].refusal = None;
            self.verified.push(v);
        }
        let reason = match (directive, refused_reason) {
            (Some(d), _) => format!("failover-{}", d.action),
            (None, Some(r)) => r,
            (None, None) => "preferred".to_owned(),
        };
        let r = &self.policy.routes[route];
        let attempt = self.attempts.len() as u64 + 1;
        self.attempts.push(Attempt {
            attempt,
            request,
            predecessor: predecessor.map(|p| p.0),
            reason,
            route: r.id.clone(),
            connection: r.connection.clone(),
            model: r.model.clone(),
            payer: r.payer.clone(),
            policy: self.digest.clone(),
            reserved_units: self.policy.reserve_units,
            disposition: None,
            usage: Value::Null,
            decision: directive.cloned(),
        });
        Decision::Attempt { route, attempt }
    }
    pub fn settle(
        &mut self,
        attempt: u64,
        disposition: Disposition,
        usage: Value,
    ) -> Result<(), &'static str> {
        let a = self
            .attempts
            .iter_mut()
            .find(|a| a.attempt == attempt)
            .ok_or("route_attempt_unknown")?;
        if a.disposition.is_some() {
            return Err("route_attempt_settled");
        }
        if disposition == Disposition::NotDispatched {
            self.reserved = self
                .reserved
                .checked_sub(a.reserved_units)
                .ok_or("route_reservation_accounting")?;
            a.reserved_units = 0;
        }
        if let Disposition::Refused { status, code } = &disposition {
            self.reserved = self
                .reserved
                .checked_sub(a.reserved_units)
                .ok_or("route_reservation_accounting")?;
            a.reserved_units = 0;
            let i = self
                .policy
                .routes
                .iter()
                .position(|r| r.id == a.route)
                .ok_or("route_unknown")?;
            let (request, number) = (a.request, a.attempt);
            let ordinal = self
                .attempts
                .iter()
                .filter(|x| x.request == request && x.attempt <= number)
                .count() as u64;
            self.state[i].refusal = Some((format!("{status}-{code}"), (request, ordinal)));
        }
        let a = self
            .attempts
            .iter_mut()
            .find(|a| a.attempt == attempt)
            .expect("found above");
        a.disposition = Some(disposition);
        a.usage = usage;
        Ok(())
    }
    /// Rebuild routing state in a fresh process from durable attempt intents
    /// and settlements of the same pinned policy by replaying them in order:
    /// each intent must be what this router admits at that point, under its
    /// journaled decision input (checked against `binding`) or, without one,
    /// by default selection, so any route change not preceded by a refusal
    /// needs a decision. An intent without a settlement becomes
    /// `Uncertain(restart-unsettled)`: its reservation is kept and the request
    /// cannot be retried or switched by this router.
    pub fn recover(
        policy: Policy,
        binding: Option<FailoverBinding>,
        intents: &[Attempt],
        settles: &[Attempt],
    ) -> Result<Self, &'static str> {
        let mut r = Self::new(policy);
        r.failover = binding;
        let mut intents = intents.to_vec();
        intents.sort_by_key(|a| a.attempt);
        for (i, a) in intents.iter().enumerate() {
            if a.attempt != i as u64 + 1 || a.disposition.is_some() {
                return Err("route_intent_sequence");
            }
            if a.policy != r.digest {
                return Err("route_policy_mismatch");
            }
        }
        if settles
            .iter()
            .any(|s| !intents.iter().any(|a| a.attempt == s.attempt))
        {
            return Err("route_settle_without_intent");
        }
        let mut settles = settles.to_vec();
        settles.sort_by_key(|s| s.attempt);
        let mut pending = settles.iter().peekable();
        for a in &intents {
            while let Some(s) = pending.next_if(|s| s.attempt < a.attempt) {
                r.recover_settle(s)?;
            }
            if a.decision.is_some() && r.failover.is_none() {
                return Err("route_decision_without_binding");
            }
            if !r.policy.routes.iter().any(|x| x.id == a.route) {
                return Err("route_unknown");
            }
            let model = r.policy.model.clone();
            let now = a.decision.as_ref().map_or(0, |d| d.decided_ms);
            match r.admit(a.request, &model, now, a.decision.as_ref(), false) {
                Decision::Attempt { attempt, .. } => {
                    let got = r.attempts.last().expect("admitted");
                    if attempt != a.attempt
                        || (got.request, &got.route, &got.payer, got.predecessor)
                            != (a.request, &a.route, &a.payer, a.predecessor)
                    {
                        return Err(if a.decision.is_none() {
                            "route_change_without_decision"
                        } else {
                            "route_decision_not_reproduced"
                        });
                    }
                    let reserved = got.reserved_units;
                    r.reserved = r
                        .reserved
                        .checked_sub(reserved)
                        .and_then(|v| v.checked_add(a.reserved_units))
                        .ok_or("route_reservation_accounting")?;
                    *r.attempts.last_mut().expect("admitted") = a.clone();
                }
                Decision::Unavailable(_) => {
                    return Err(if a.decision.is_none() {
                        "route_change_without_decision"
                    } else {
                        "route_decision_not_reproduced"
                    });
                }
            }
        }
        for s in pending {
            r.recover_settle(s)?;
        }
        let unsettled: Vec<u64> = r
            .attempts
            .iter()
            .filter(|a| a.disposition.is_none())
            .map(|a| a.attempt)
            .collect();
        for a in unsettled {
            r.settle(
                a,
                Disposition::Uncertain {
                    reason: "restart-unsettled".into(),
                },
                Value::Null,
            )?;
        }
        Ok(r)
    }
    fn recover_settle(&mut self, s: &Attempt) -> Result<(), &'static str> {
        let intent = self
            .attempts
            .iter()
            .find(|a| a.attempt == s.attempt)
            .ok_or("route_settle_without_intent")?;
        if (intent.request, &intent.route, &intent.payer) != (s.request, &s.route, &s.payer) {
            return Err("route_settle_mismatch");
        }
        let d = s.disposition.clone().ok_or("route_settle_empty")?;
        self.settle(s.attempt, d, s.usage.clone())
    }
    pub fn evidence(&self) -> Value {
        json!({
            "policy": self.policy,
            "policyDigest": self.digest,
            "reservedUnits": self.reserved,
            "attempts": self.attempts,
            "failover": self.failover,
            "routeState": self.policy.routes.iter().zip(&self.state).map(|(r, s)| json!({"route":r.id,"attempts":s.attempts,"unavailable":s.refusal.as_ref().map(|x| &x.0)})).collect::<Vec<_>>(),
        })
    }
}
