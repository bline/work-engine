//! One signed, network-disabled Pi fixture. The registry owns authority and
//! uncertainty; the runtime owns actual SDK/process/tool observations.
use crate::canonical_source::Pin;
use crate::{
    Ref, Row, inspect, request::AdmissionRequest, scoped_pilot as control, service, source, store,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
pub type Result<T> = std::result::Result<T, &'static str>;
pub const PROFILE: &str = "dot.pi-coding-fixture-profile/v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transport {
    pub helper_sha256: String,
    pub principal: String,
    pub subject: String,
    pub handle: String,
    pub revision: String,
    pub idempotency_key: String,
}
impl Transport {
    pub fn sha256(&self) -> String {
        crate::hash(&serde_json::to_string(self).expect("transport serialization"))
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub schema: String,
    pub config: Pin,
    pub profile: Pin,
    pub admission_request: String,
    pub transport: Transport,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: String,
    pub classification: String,
    pub task: Ref,
    pub proposal: Ref,
    pub mode: Ref,
    pub configuration: Ref,
    pub parent: Ref,
    pub actor: String,
    pub objective: String,
    pub job_sha256: String,
    pub native_executable_sha256: String,
    pub transport_sha256: String,
    pub expires_at: String,
}
fn bytes_sha(b: &[u8]) -> String {
    use sha2::Digest;
    format!("{:x}", sha2::Sha256::digest(b))
}
fn sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn clock() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}
fn utc(s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|_| "pi_clock")
}
fn descriptor(pin: &Pin) -> Result<Descriptor> {
    if !source::pinned(pin) {
        return Err("pi_descriptor_pin");
    }
    let d: Descriptor =
        serde_json::from_slice(&service::protected(&pin.path).ok_or("pi_descriptor")?)
            .map_err(|_| "pi_descriptor")?;
    if d.schema != "dot.pi-coding-descriptor/v1"
        || !sha(&d.transport.helper_sha256)
        || [
            &d.transport.principal,
            &d.transport.subject,
            &d.transport.handle,
            &d.transport.revision,
            &d.transport.idempotency_key,
        ]
        .iter()
        .any(|s| s.is_empty() || s.len() > 256)
    {
        return Err("pi_descriptor");
    }
    Ok(d)
}
/// Caller identity is supplied only by the fixed authenticated endpoint. This
/// consistency check is not a JWT verifier or a replacement for HTTP auth.
pub fn check_transport(
    pin: &Pin,
    principal: &str,
    handle: &str,
    revision: &str,
    key: &str,
    helper_sha: &str,
) -> Result<Descriptor> {
    let d = descriptor(pin)?;
    let t = &d.transport;
    if (principal, handle, revision, key, helper_sha)
        != (
            t.principal.as_str(),
            t.handle.as_str(),
            t.revision.as_str(),
            t.idempotency_key.as_str(),
            t.helper_sha256.as_str(),
        )
    {
        return Err("pi_transport_conflict");
    }
    Ok(d)
}
/// No deserialization or public permit constructor. The signed activation is
/// reloaded under the registry write ordering at each native effect boundary.
pub struct PiOwner {
    pin: Pin,
    d: Descriptor,
    p: Profile,
}
impl PiOwner {
    pub fn open(pin: Pin) -> Result<Self> {
        let d = descriptor(&pin)?;
        if !source::pinned(&d.profile) {
            return Err("pi_profile_pin");
        }
        let p: Profile =
            serde_json::from_slice(&service::protected(&d.profile.path).ok_or("pi_profile")?)
                .map_err(|_| "pi_profile")?;
        if p.schema != PROFILE
            || p.classification
                != "fixture-signing-and-identity; fake-provider; network-none; actual-pi-sdk-and-docker"
            || !sha(&p.job_sha256)
            || !sha(&p.native_executable_sha256)
            || p.transport_sha256 != d.transport.sha256()
        {
            return Err("pi_profile");
        }
        if bytes_sha(
            &std::fs::read(std::env::current_exe().map_err(|_| "pi_executor")?)
                .map_err(|_| "pi_executor")?,
        ) != p.native_executable_sha256
        {
            return Err("pi_executor");
        }
        let owner = Self { pin, d, p };
        ingest_control(&owner.pin)?;
        let db = owner.db()?;
        owner.current(&db)?;
        Ok(owner)
    }
    pub fn profile_sha256(&self) -> &str {
        &self.d.profile.sha256
    }
    fn db(&self) -> Result<Connection> {
        control::database(&control::config(&self.d.config)?)
    }
    fn current(&self, db: &Connection) -> Result<(inspect::Evidence, control::Head)> {
        if !source::pinned(&self.pin) || !source::pinned(&self.d.profile) {
            return Err("pi_pin_changed");
        }
        let c = control::config(&self.d.config)?;
        let e = inspect::collect(&c.sources, db, &self.d.admission_request, clock())
            .map_err(|e| e.0)?;
        let h = control::head(&c, &self.d.profile, clock())?;
        control::synchronize(db, &self.d.profile, &h)?;
        let row: Row = serde_json::from_str(&e.original).map_err(|_| "pi_task")?;
        let i = row.data.intent;
        let p = &self.p;
        let mode = row
            .data
            .intent_artifacts
            .iter()
            .find(|a| a.r#ref == i.authority.mode)
            .ok_or("pi_mode")?;
        let mode: crate::ModeView =
            serde_json::from_str(&mode.content_json).map_err(|_| "pi_mode")?;
        if i.schema != "dot.task-intent/pi-coding-v1"
            || i.provider != "pi"
            || i.execution_profile != "disposable-pi-coding-fixture-v1"
            || i.effects.network
            || i.effects.write_paths != ["src/lib.rs"]
            || i.model.id != "fake-pi-model"
            || i.model.effort != "off"
            || i.model.tier != "fixture"
            || e.task != p.task
            || i.proposal != p.proposal
            || i.authority.mode != p.mode
            || i.workspace.configuration != p.configuration
            || !i.prerequisites.contains(&p.parent)
            || i.submitter != p.actor
            || i.budget.objective_id != p.objective
            || mode.mode != "present"
            || mode.pending_transition.is_some()
            || utc(&mode.effective_at)? > clock()
            || h.state != "active"
            || clock() >= h.valid_until
            || clock() >= utc(&p.expires_at)?
            || clock() >= utc(&i.clock.admit_until)?
            || clock() >= utc(&i.clock.close_by)?
            || clock() >= e.verified.expires_at
            || clock() - e.budget.at > time::Duration::seconds(30)
            || e.budget.at > clock()
        {
            return Err("pi_current_authority");
        }
        // A previously observed newer control head can never be rolled back.
        control::synchronize(db, &self.d.profile, &h)?;
        Ok((e, h))
    }
    // Observation and action share the registry lock, but have different rollback
    // semantics: a verified signed head survives denial; action writes do not.
    fn begin_current(&self, db: &Connection) -> Result<()> {
        db.execute_batch("BEGIN IMMEDIATE")
            .map_err(|_| "pi_registry")?;
        if let Err(e) = self.current(db) {
            db.execute_batch("COMMIT").map_err(|_| "pi_registry")?;
            return Err(e);
        }
        Ok(())
    }
    fn action(&self, db: &Connection, run: impl FnOnce() -> Result<()>) -> Result<()> {
        self.begin_current(db)?;
        db.execute_batch("SAVEPOINT pi_action")
            .map_err(|_| "pi_registry")?;
        let result = run();
        if result.is_err() {
            db.execute_batch("ROLLBACK TO pi_action")
                .map_err(|_| "pi_registry")?;
        }
        db.execute_batch("RELEASE pi_action; COMMIT")
            .map_err(|_| "pi_registry")?;
        result
    }
    /// Consume signed approval and capacity using the existing admission owner.
    /// An exact duplicate returns evidence; it does not authorize another run.
    pub fn admit(&self, job_sha: &str, executable_sha: &str) -> Result<Value> {
        if job_sha != self.p.job_sha256 || executable_sha != self.p.native_executable_sha256 {
            return Err("pi_job_binding");
        }
        ingest_control(&self.pin)?;
        let mut db = self.db()?;
        let (e, h) = self.current(&db)?;
        let row: Row = serde_json::from_str(&e.original).map_err(|_| "pi_task")?;
        let i = row.data.intent;
        let request: AdmissionRequest =
            serde_json::from_str(&self.d.admission_request).map_err(|_| "pi_request")?;
        let gate = store::CurrentGate {
            task_id: e.task.id.clone(),
            revision: e.task.revision.clone(),
            request_sha: e.task.sha256.clone(),
            lane_id: i.lane_id,
            proposal: i.proposal,
            decision: e.decision.clone(),
            objective_id: i.budget.objective_id,
            canonical_budget_revision: e.budget_record_sha256,
            observation_ref: e.budget.source_sha,
            exclusive_binding_ref: format!("signed-pi-control:{}", h.sha),
            maximum: i.budget.maximum,
            productive_maximum: i.budget.productive_maximum,
            closure_reserve: i.budget.closure_reserve,
        };
        let a = store::commit(
            &mut db,
            &request.request_id,
            &e.original,
            &e.answer,
            &e.verified,
            &gate,
        )
        .map_err(|_| "pi_admission_conflict")?;
        let result = self.action(&db, || {
            let old: Option<(String, String, String)> = db
                .query_row(
                    "SELECT operation_id,descriptor_sha,job_sha FROM pi_claim WHERE profile_sha=?1",
                    [&self.d.profile.sha256],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
                .map_err(|_| "pi_registry")?;
            if let Some(old) = old {
                if old != (a.operation.clone(), self.pin.sha256.clone(), job_sha.into()) {
                    return Err("pi_claim_conflict");
                }
            } else {
                db.execute("INSERT INTO pi_claim(profile_sha,operation_id,descriptor_sha,job_sha) VALUES(?1,?2,?3,?4)",params![self.d.profile.sha256,a.operation,self.pin.sha256,job_sha]).map_err(|_|"pi_claim_conflict")?;
            }
            Ok(())
        });
        result?;
        read(&self.pin)
    }
    /// Commit unknown BEFORE I/O, then reacquire and retain registry ordering
    /// through dispatch. Dropping an unobserved lease never permits replay.
    pub fn enter(&self, effect: Effect) -> Result<EffectLease> {
        ingest_control(&self.pin)?;
        let db = self.db()?;
        let key = effect.key()?;
        let prepared = self.action(&db, || {
            let (operation,native):(String,Option<String>)=db.query_row("SELECT operation_id,native_id FROM pi_claim WHERE profile_sha=?1 AND descriptor_sha=?2 AND job_sha=?3",params![self.d.profile.sha256,self.pin.sha256,self.p.job_sha256],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|"pi_claim_missing")?;
            store::admitted_heads_live(&db, &operation).map_err(|_| "pi_heads")?;
            if effect != Effect::Allocation && native.is_none() {
                return Err("pi_native_unsealed");
            }
            if effect == Effect::Allocation {
                if native.is_some() {
                    return Err("pi_allocation_replacement");
                }
                if db.execute("UPDATE outbox SET state='unknown' WHERE operation_id=?1 AND state='pending'",[&operation]).map_err(|_|"pi_registry")?!=1 {return Err("pi_allocation_unknown");}
                db.execute(
                    "UPDATE capacity_claim SET state='unknown' WHERE operation_id=?1",
                    [&operation],
                )
                .map_err(|_| "pi_registry")?;
            }
            db.execute(
                "INSERT INTO pi_effect VALUES(?1,?2,'unknown',NULL)",
                params![self.d.profile.sha256, key],
            )
            .map_err(|_| "pi_effect_unknown_no_retry")?;
            Ok(())
        });
        prepared?;
        self.begin_current(&db)?;
        Ok(EffectLease {
            db,
            owner_pin: self.pin.clone(),
            profile_sha: self.d.profile.sha256.clone(),
            key,
            expires: self.until()?,
            closed: std::cell::Cell::new(false),
            failed: std::cell::Cell::new(false),
        })
    }
    fn until(&self) -> Result<OffsetDateTime> {
        utc(&self.p.expires_at)
    }
    pub fn seal(&self, job_sha: &str, native: &str) -> Result<()> {
        if job_sha != self.p.job_sha256 || uuid::Uuid::parse_str(native).is_err() {
            return Err("pi_allocation_binding");
        }
        ingest_control(&self.pin)?;
        let db = self.db()?;
        self.action(&db, || {
            let allocation:i64=db.query_row("SELECT count(*) FROM pi_effect WHERE profile_sha=?1 AND effect_key='allocation' AND state='unknown'",[&self.d.profile.sha256],|r|r.get(0)).map_err(|_|"pi_registry")?;
            if allocation!=1 || db.execute("UPDATE pi_claim SET native_id=?2 WHERE profile_sha=?1 AND native_id IS NULL",params![self.d.profile.sha256,native]).map_err(|_|"pi_registry")?!=1 {return Err("pi_allocation_replacement");}
            db.execute("UPDATE pi_effect SET state='observed',evidence_sha=?2 WHERE profile_sha=?1 AND effect_key='allocation'",params![self.d.profile.sha256,crate::hash(native)]).map_err(|_|"pi_registry")?;
            Ok(())
        })
    }
    pub fn check_native(&self, native: &str) -> Result<()> {
        let db = self.db()?;
        let found: Option<String> = db
            .query_row(
                "SELECT native_id FROM pi_claim WHERE profile_sha=?1",
                [&self.d.profile.sha256],
                |r| r.get(0),
            )
            .map_err(|_| "pi_native_unsealed")?;
        if found.as_deref() != Some(native) {
            return Err("pi_native_mismatch");
        }
        Ok(())
    }
    /// Attributed runtime evidence, never an approval or a remote settlement.
    pub fn record_result(&self, result: &Value) -> Result<Value> {
        let db = self.db()?;
        let body = serde_json::to_string(result).map_err(|_| "pi_result")?;
        let receipt = result.get("receipt").unwrap_or(result);
        if receipt["input_digest"] != self.p.job_sha256 {
            return Err("pi_result_binding");
        }
        if let Some(native) = receipt["session_id"].as_str() {
            self.check_native(native)?;
        }
        let changed = db
            .execute(
                "UPDATE pi_claim SET result_json=?2 WHERE profile_sha=?1 AND result_json IS NULL",
                params![self.d.profile.sha256, body],
            )
            .map_err(|_| "pi_registry")?;
        if changed != 1 {
            return Err("pi_result_already_recorded");
        }
        read(&self.pin)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Allocation,
    Initialization,
    Provider(u64),
    Tool(String),
}
impl Effect {
    fn key(&self) -> Result<String> {
        Ok(match self {
            Self::Allocation => "allocation".into(),
            Self::Initialization => "initialization".into(),
            Self::Provider(n) if *n > 0 => format!("provider:{n}"),
            Self::Tool(id) if !id.is_empty() && id.len() < 128 => format!("tool:{id}"),
            _ => return Err("pi_effect_key"),
        })
    }
}
/// Opaque ordering lease. Only the owner can construct it. No callback claims
/// qualification; the runtime supplies attributable effect evidence afterwards.
pub struct EffectLease {
    db: Connection,
    owner_pin: Pin,
    profile_sha: String,
    key: String,
    expires: OffsetDateTime,
    closed: std::cell::Cell<bool>,
    failed: std::cell::Cell<bool>,
}
impl EffectLease {
    pub fn check_current(&self) -> Result<()> {
        if self.failed.get() || self.closed.get() {
            return Err("pi_lease_closed");
        }
        if clock() >= self.expires {
            self.failed.set(true);
            return Err("pi_lease_expired");
        }
        // Reopen signed proof/key/current board while retaining registry ordering.
        let owner = PiOwner::open_unchecked(self.owner_pin.clone())?;
        if let Err(e) = owner.current(&self.db) {
            // No effect has been acknowledged in this transaction. Retain a
            // newly observed signed revocation/advance instead of forgetting
            // it when the denied dispatch drops its lease. Unknown intent was
            // already committed separately before this transaction.
            self.failed.set(true);
            self.db.execute_batch("COMMIT").map_err(|_| "pi_registry")?;
            self.closed.set(true);
            return Err(e);
        }
        Ok(())
    }
    pub fn observed(self, evidence_sha: &str) -> Result<()> {
        if self.failed.get() || self.closed.get() {
            return Err("pi_lease_closed");
        }
        if !sha(evidence_sha) {
            return Err("pi_evidence");
        }
        self.db.execute("UPDATE pi_effect SET state='observed',evidence_sha=?3 WHERE profile_sha=?1 AND effect_key=?2 AND state='unknown'",params![self.profile_sha,self.key,evidence_sha]).map_err(|_|"pi_registry")?;
        self.db.execute_batch("COMMIT").map_err(|_| "pi_registry")?;
        self.closed.set(true);
        Ok(())
    }
}
impl Drop for EffectLease {
    fn drop(&mut self) {
        if !self.closed.get() {
            let _ = self.db.execute_batch("ROLLBACK");
        }
    }
}
impl PiOwner {
    fn open_unchecked(pin: Pin) -> Result<Self> {
        let d = descriptor(&pin)?;
        if !source::pinned(&d.profile) {
            return Err("pi_profile_pin");
        }
        let p = serde_json::from_slice(&service::protected(&d.profile.path).ok_or("pi_profile")?)
            .map_err(|_| "pi_profile")?;
        Ok(Self { pin, d, p })
    }
}
/// Pure immutable result read: no source collection, ACK, reconciliation or retry.
pub fn read(pin: &Pin) -> Result<Value> {
    let d = descriptor(pin)?;
    let c = control::config(&d.config)?;
    let db = Connection::open_with_flags(&c.database, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| "pi_registry")?;
    let request: AdmissionRequest =
        serde_json::from_str(&d.admission_request).map_err(|_| "pi_request")?;
    let existing = store::retrieve_existing(&db, &request, &c.sources.actor)
        .map_err(|_| "pi_read_conflict")?;
    let Some(a) = existing else {
        return Ok(json!({"state":"not_admitted","retry_permitted":false}));
    };
    type Record = (String, String, Option<String>, Option<String>);
    let row:Option<Record>=db.query_row("SELECT descriptor_sha,job_sha,native_id,result_json FROM pi_claim WHERE operation_id=?1",[&a.admission.operation],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|_|"pi_registry")?;
    let Some((descriptor_sha, job, native, result)) = row else {
        return Ok(
            json!({"state":"admission_without_native_claim","operation_id":a.admission.operation,"retry_permitted":false}),
        );
    };
    if descriptor_sha != pin.sha256 {
        return Err("pi_read_conflict");
    }
    let effects: Vec<Value> = {
        let mut s=db.prepare("SELECT effect_key,state,evidence_sha FROM pi_effect WHERE profile_sha=?1 ORDER BY effect_key").map_err(|_|"pi_registry")?;
        s.query_map([&d.profile.sha256],|r|Ok(json!({"key":r.get::<_,String>(0)?,"state":r.get::<_,String>(1)?,"evidence_sha256":r.get::<_,Option<String>>(2)?}))).map_err(|_|"pi_registry")?.collect::<std::result::Result<_,_>>().map_err(|_|"pi_registry")?
    };
    Ok(
        json!({"schema":"dot.pi-coding-result/v1","classification":"fixture signing/identity/provider; actual Pi SDK/Docker/tool/validator","admission_id":a.admission.id,"operation_id":a.admission.operation,"profile_sha256":d.profile.sha256,"descriptor_sha256":pin.sha256,"job_sha256":job,"native_session_id":native,"state":if result.is_some(){"runtime_evidence_recorded"}else{"unknown_or_pending"},"outbox":a.status,"effects":effects,"runtime_result":result.map(|s|serde_json::from_str::<Value>(&s)).transpose().map_err(|_|"pi_result")?,"owner_acceptance":"not_claimed","retry_permitted":false}),
    )
}
/// Explicit control ingestion uses the same registry ordering as effect entry.
pub fn ingest_control(pin: &Pin) -> Result<()> {
    let d = descriptor(pin)?;
    let c = control::config(&d.config)?;
    let db = control::database(&c)?;
    db.execute_batch("BEGIN IMMEDIATE")
        .map_err(|_| "pi_registry")?;
    let h = control::head(&c, &d.profile, clock())?;
    control::synchronize(&db, &d.profile, &h)?;
    db.execute_batch("COMMIT").map_err(|_| "pi_registry")?;
    Ok(())
}

#[cfg(test)]
#[path = "pi_pilot_tests.rs"]
mod tests;
