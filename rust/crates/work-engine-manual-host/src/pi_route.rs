//! Bounded route selection and authorized funding failover for admitted
//! inference requests.
//!
//! Mechanical admission only: the requested model is fixed (no substitution),
//! candidate routes and their order come from a pinned policy, and a switch
//! happens only after a definitive refusal that produced no output. Anything
//! that may have entered the provider or reached the consumer is uncertain and
//! is returned to the continuation owner instead of being retried. Reservations
//! are conservative: only definitive refusals release them. Leaving a route
//! requires its definitive refusal; limits and revocation stop admission.
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
}
/// HTTP statuses treated as definitive funding/authorization refusals when
/// observed at the response head, before anything is relayed.
pub fn refusal_status(status: u16) -> bool {
    matches!(status, 401 | 402 | 403 | 429)
}

#[derive(Debug, Clone, Serialize)]
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
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Attempt { route: usize, attempt: u64 },
    Unavailable(&'static str),
}

pub struct Router {
    policy: Policy,
    digest: String,
    reserved: u64,
    attempts: Vec<Attempt>,
    /// Per route: attempts used and the refusal that made it unavailable for
    /// the rest of this job (stable routing; no needless switching back).
    state: Vec<(u32, Option<String>)>,
}
impl Router {
    pub fn new(policy: Policy) -> Self {
        let n = policy.routes.len();
        Self {
            digest: policy.digest(),
            policy,
            reserved: 0,
            attempts: vec![],
            state: vec![(0, None); n],
        }
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
    /// Admit the next attempt for `request` or explain why none is permitted.
    pub fn next(&mut self, request: u64, model: &str, now_ms: u64) -> Decision {
        if now_ms >= self.policy.expires_ms {
            return Decision::Unavailable("policy_stale");
        }
        if model != self.policy.model {
            return Decision::Unavailable("model_incompatible");
        }
        let prior: Vec<&Attempt> = self
            .attempts
            .iter()
            .filter(|a| a.request == request)
            .collect();
        let predecessor = prior.last().map(|a| (a.attempt, a.disposition.clone()));
        let reason = match &predecessor {
            None => "preferred".to_owned(),
            Some((_, None)) => return Decision::Unavailable("attempt_unsettled"),
            Some((_, Some(Disposition::Completed))) => {
                return Decision::Unavailable("request_completed");
            }
            Some((_, Some(Disposition::Uncertain { .. }))) => {
                return Decision::Unavailable("uncertain_predecessor");
            }
            Some((_, Some(Disposition::Refused { status, code }))) => {
                format!("failover-after-{status}-{code}")
            }
        };
        if prior.len() as u32 >= self.policy.max_attempts_per_request {
            return Decision::Unavailable("attempt_limit");
        }
        // The active route is the first statically eligible route (authorized,
        // same model) without a recorded definitive refusal. Moving past a
        // route requires such a refusal; revocation or an exhausted per-route
        // limit stops admission instead of switching. Changing that needs a
        // new policy (a separate, authorized policy transition).
        let Some(route) = self.policy.routes.iter().enumerate().position(|(i, r)| {
            r.authorized && r.model == self.policy.model && self.state[i].1.is_none()
        }) else {
            return Decision::Unavailable("no_permitted_route");
        };
        let candidate = &self.policy.routes[route];
        if candidate.revoked {
            return Decision::Unavailable("route_revoked");
        }
        if self.state[route].0 >= candidate.max_attempts {
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
        self.state[route].0 += 1;
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
            self.state[i].1 = Some(format!("{status}-{code}"));
        }
        a.disposition = Some(disposition);
        a.usage = usage;
        Ok(())
    }
    pub fn evidence(&self) -> Value {
        json!({
            "policy": self.policy,
            "policyDigest": self.digest,
            "reservedUnits": self.reserved,
            "attempts": self.attempts,
            "routeState": self.policy.routes.iter().zip(&self.state).map(|(r, s)| json!({"route":r.id,"attempts":s.0,"unavailable":s.1})).collect::<Vec<_>>(),
        })
    }
}
