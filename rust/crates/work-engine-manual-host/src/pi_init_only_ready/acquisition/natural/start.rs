//! Natural-exit Start disposition from the controller's original exchanges.
//! This owns local chronology and trusted successful replies, not exclusion
//! of a competing host principal or the stability proof for the Held cut.

use super::super::{Custody, Kind, StartRpc, docker, invalid, refuse};
use super::{Attach, Result, full_hex, json};
use serde::Serialize;
use std::time::Instant;

pub(super) struct RegisteredStart {
    rpc: StartRpc,
    attach: Attach,
    checked_create: u64,
    position: u64,
    armed: Option<(u64, Instant)>,
    issued: Option<(u64, Instant)>,
    accepted: Option<(u64, Instant)>,
}

// The trusted router registers the per-object stdout broadcaster before
// sending the upgrade (Moby daemon/attach.go and the v1.44 attach handler).
// Call this only after the authenticated upgrade and no pending early bytes
// on that original stream, the checked Create, a same-stream version reply
// for Start, and binding the exclusive Start scope.
impl RegisteredStart {
    pub(super) fn register(
        journal: &mut Custody,
        checked_create: u64,
        rpc: &StartRpc,
        attach: &Attach,
    ) -> Result<Self> {
        if !same_exchange_subject(rpc, attach) || checked_create >= journal.next() {
            return refuse("natural Start registration crossed the original exchanges");
        }
        let position = journal.record(
            Kind::NaturalStartRoute,
            json!({"checked_create":checked_create, "start":rpc, "registered_stdout":attach}),
        )?;
        Ok(Self {
            rpc: rpc.clone(),
            attach: attach.clone(),
            checked_create,
            position,
            armed: None,
            issued: None,
            accepted: None,
        })
    }

    pub(super) fn arm(&mut self, position: u64, at: Instant) -> Result<()> {
        if self.armed.is_some() || position <= self.position {
            return refuse("natural Start arm crossed prearm or replayed");
        }
        self.armed = Some((position, at));
        Ok(())
    }

    pub(super) fn written(&mut self, position: u64, at: Instant) -> Result<()> {
        if self.issued.is_some() || !ordered(self.armed, (position, at)) {
            return refuse("natural Start write crossed the actual arm or replayed");
        }
        self.issued = Some((position, at));
        Ok(())
    }

    pub(super) fn accepted(
        &mut self,
        position: u64,
        at: Instant,
        reply: Option<&docker::Response>,
    ) -> Result<()> {
        if self.accepted.is_some()
            || !ordered(self.issued, (position, at))
            || !reply.is_some_and(|r| r.status == 204 && r.body.is_empty())
        {
            return refuse("natural exit requires the exact successful Start response");
        }
        self.accepted = Some((position, at));
        Ok(())
    }

    pub(super) fn require_accepted(&self, rpc: &StartRpc) -> Result<()> {
        if self.accepted.is_none() || self.rpc != *rpc {
            return Err(invalid(
                "stock original selection lacks the exact accepted Start",
            ));
        }
        Ok(())
    }

    pub(super) fn settlement(&self, rpc: &StartRpc) -> Result<Disposition> {
        self.require_accepted(rpc)?;
        let position = |sample: Option<(u64, Instant)>| {
            sample
                .map(|(p, _)| p)
                .ok_or_else(|| invalid("missing original Start disposition"))
        };
        Ok(Disposition {
            checked_create: self.checked_create,
            registration: self.position,
            arm: position(self.armed)?,
            written: position(self.issued)?,
            accepted: position(self.accepted)?,
        })
    }

    pub(super) fn check_stdout(&self, rpc: &StartRpc, attach: &Attach) -> Result<()> {
        self.require_accepted(rpc)?;
        if self.attach != *attach {
            return refuse("original stdout attachment changed after prearm");
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub(super) struct Disposition {
    pub(super) checked_create: u64,
    pub(super) registration: u64,
    pub(super) arm: u64,
    pub(super) written: u64,
    pub(super) accepted: u64,
}

fn ordered(before: Option<(u64, Instant)>, after: (u64, Instant)) -> bool {
    before.is_some_and(|(p, t)| p < after.0 && t <= after.1)
}

fn same_exchange_subject(rpc: &StartRpc, attach: &Attach) -> bool {
    full_hex(&rpc.full_id)
        && attach.serving == rpc.serving
        && attach.resolved_full_id == rpc.full_id
        && attach.connection != 0
        && rpc.exchange != 0
        && attach.connection != rpc.exchange
        && rpc.request_target
            == format!("POST /{}/containers/{}/start", rpc.serving.api, rpc.full_id)
        && attach.http_request == super::super::attach_request(&rpc.serving, &rpc.full_id)
        && attach.registration == super::super::Registration::AuthenticatedPinnedSameObject
}

#[test]
fn natural_start_orders_original_armed_write_and_exact_accepted_reply() {
    let serving = super::super::super::tests::serving();
    let id = "a".repeat(64);
    let links = super::super::super::OperationLinks {
        selection: "b".repeat(64),
        create_request: "c".repeat(64),
        create_mark: "d".repeat(64),
        start_request: "e".repeat(64),
        start_mark: "f".repeat(64),
    };
    let rpc = StartRpc {
        request_target: format!("POST /{}/containers/{id}/start", serving.api),
        serving: serving.clone(),
        full_id: id.clone(),
        links,
        exchange: 17,
    };
    let mut attach = Attach {
        serving: serving.clone(),
        connection: 21,
        resolved_full_id: id.clone(),
        http_request: super::super::attach_request(&serving, &id),
        registration: super::super::Registration::AuthenticatedPinnedSameObject,
    };
    assert!(same_exchange_subject(&rpc, &attach));
    let mut crossed = rpc.clone();
    crossed.exchange = attach.connection;
    assert!(!same_exchange_subject(&crossed, &attach));
    let mut trace = RegisteredStart {
        rpc: rpc.clone(),
        attach: attach.clone(),
        checked_create: 6,
        position: 9,
        armed: None,
        issued: None,
        accepted: None,
    };
    let time = Instant::now();
    let reply = docker::Response {
        status: 204,
        body: vec![],
    };
    assert!(trace.settlement(&rpc).is_err());
    assert!(trace.written(10, time).is_err());
    assert!(trace.arm(9, time).is_err());
    trace.arm(10, time).unwrap();
    assert!(trace.arm(11, time).is_err());
    trace.written(11, time).unwrap();
    assert!(trace.accepted(11, time, Some(&reply)).is_err());
    assert!(trace.accepted(12, time, None).is_err());
    assert!(
        trace
            .accepted(
                12,
                time,
                Some(&docker::Response {
                    status: 304,
                    body: vec![]
                })
            )
            .is_err()
    );
    trace.accepted(12, time, Some(&reply)).unwrap();
    assert!(trace.accepted(13, time, Some(&reply)).is_err());
    trace.check_stdout(&rpc, &attach).unwrap();
    attach.connection = 33;
    assert!(trace.check_stdout(&rpc, &attach).is_err());
    assert!(trace.settlement(&crossed).is_err());
    let cut = trace.settlement(&rpc).unwrap();
    assert_eq!(
        [
            cut.checked_create,
            cut.registration,
            cut.arm,
            cut.written,
            cut.accepted
        ],
        [6, 9, 10, 11, 12],
    );
}
