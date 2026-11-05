//! Authority for the separately versioned one-session disposable pilot.
//! These observations do not attest native custody and cannot write an effect.
//! Only the bridge's private actual-native conjunction consumes them.
use crate::{
    Ref, Row,
    canonical_source::{CanonicalSource, Pin},
    inspect, proof, service, source,
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
type Result<T> = std::result::Result<T, &'static str>;
fn selected_final_text<'a>(events: &'a [Value], session: &str, turn: &str) -> Option<&'a str> {
    let terminals: Vec<_> = events
        .iter()
        .filter(|e| {
            e["method"] == "turn/completed"
                && e.pointer("/params/threadId").and_then(Value::as_str) == Some(session)
                && e.pointer("/params/turn/id").and_then(Value::as_str) == Some(turn)
        })
        .collect();
    if terminals.len() != 1
        || terminals[0].pointer("/params/turn/status") != Some(&json!("completed"))
        || !terminals[0]
            .pointer("/params/turn/error")
            .is_none_or(Value::is_null)
    {
        return None;
    }
    let packets: Vec<_> = events
        .iter()
        .filter(|e| {
            e["method"] == "item/completed"
                && e.pointer("/params/threadId").and_then(Value::as_str) == Some(session)
                && e.pointer("/params/turnId").and_then(Value::as_str) == Some(turn)
                && e.pointer("/params/item/type") == Some(&json!("agentMessage"))
        })
        .collect();
    if packets.len() != 1 {
        return None;
    }
    packets[0].pointer("/params/item/text")?.as_str()
}
pub const PROFILE: &str = "dot.present.one-owned-session-disposable/v1";
pub const PROFILE_FINAL: &str = "dot.present.one-owned-session-disposable/v2-two-requests";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalStageAllowance {
    pub attempt: String,
    pub requests: u8,
    pub effects: u8,
    pub initial_maximum: String,
    pub initial_reserve: String,
    pub maximum: String,
    pub reserve: String,
    pub initial_output_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub path: String,
    pub before_sha256: String,
    pub after: String,
    pub after_sha256: String,
}
/// Signed data, not a permit. The activation signs this exact file digest.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_stage: Option<FinalStageAllowance>,
    pub schema: String,
    pub id: String,
    pub task: Ref,
    pub proposal: Ref,
    pub mode: Ref,
    pub configuration: Ref,
    pub actor: String,
    pub objective: String,
    pub session: String,
    pub turn_rule: String,
    pub entry_request_sha256: String,
    pub native_executable_sha256: String,
    pub target_generation: String,
    pub bridge_store_id: String,
    pub transport_sha256: String,
    pub root: PathBuf,
    pub publication: String,
    pub effect: Effect,
    pub parent: Ref,
    pub maximum: String,
    pub reserve: String,
    pub expires_at: String,
    pub trust: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    schema: String,
    profile_sha256: String,
    generation: i64,
    predecessor: Option<String>,
    state: String,
}
pub(crate) struct Head {
    pub(crate) valid_until: OffsetDateTime,
    pub(crate) generation: i64,
    pub(crate) sha: String,
    pub(crate) state: String,
    pub(crate) raw: String,
}
/// No Deserialize, qualification boolean, callback, or filesystem executor.
pub struct ScopedOwner {
    config: Pin,
    request: String,
    profile_pin: Pin,
    profile: Profile,
    db: Connection,
    #[cfg(test)]
    before_effect_lease: Option<Box<dyn FnOnce()>>,
    #[cfg(test)]
    acquisition_delay: Option<time::Duration>,
}
fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn utc(s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|_| "scoped_clock")
}
pub(crate) fn config(pin: &Pin) -> Result<service::Config> {
    if !source::pinned(pin) {
        return Err("scoped_config_pin");
    }
    let c: service::Config =
        serde_json::from_slice(&service::protected(&pin.path).ok_or("scoped_config")?)
            .map_err(|_| "scoped_config")?;
    if c.schema != "dot.admission-config/v2-owned-source" || !Path::new(&c.database).is_absolute() {
        return Err("scoped_config");
    }
    Ok(c)
}
pub(crate) fn database(c: &service::Config) -> Result<Connection> {
    use std::os::unix::fs::MetadataExt;
    let m = std::fs::symlink_metadata(&c.database).map_err(|_| "scoped_registry")?;
    if !m.is_file()
        || m.file_type().is_symlink()
        || m.mode() & 0o077 != 0
        || m.uid() != unsafe { libc::geteuid() }
    {
        return Err("scoped_registry");
    }
    let db = Connection::open_with_flags(&c.database, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(|_| "scoped_registry")?;
    db.busy_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| "scoped_registry")?;
    db.execute_batch("PRAGMA synchronous=FULL")
        .map_err(|_| "scoped_registry")?;
    crate::store::initialize(&db).map_err(|_| "scoped_registry")?;
    Ok(db)
}
pub(crate) fn head(c: &service::Config, pin: &Pin, now: OffsetDateTime) -> Result<Head> {
    let clock = source::AcquisitionTime::new(now);
    let board =
        source::board(&c.sources.board, &c.sources.board_repository).ok_or("scoped_board")?;
    let mut previous: Option<Head> = None;
    for raw in &board.rows {
        let row: Value = serde_json::from_str(raw).map_err(|_| "scoped_board")?;
        let Some(value) = row.pointer("/data/scoped_control") else {
            continue;
        };
        if value.get("profile_sha256").and_then(Value::as_str) != Some(&pin.sha256) {
            continue;
        }
        let control: Control =
            serde_json::from_value(value.clone()).map_err(|_| "scoped_control")?;
        if control.profile_sha256 != pin.sha256
            || control.schema != "dot.scoped-control/v1"
            || !["active", "revoked"].contains(&control.state.as_str())
            || control.generation != previous.as_ref().map_or(1, |p| p.generation + 1)
            || control.predecessor.as_deref() != previous.as_ref().map(|p| p.sha.as_str())
            || previous.as_ref().is_some_and(|p| p.state == "revoked")
        {
            return Err("scoped_transition");
        }
        let seq = row["sequence"].as_u64().ok_or("scoped_control")?;
        let reply = board
            .rows
            .iter()
            .find(|r| {
                serde_json::from_str::<Value>(r)
                    .ok()
                    .is_some_and(|v| v["in_reply_to"] == seq && v["kind"] == "reply")
            })
            .ok_or("scoped_signature")?;
        let reply: Value = serde_json::from_str(reply).map_err(|_| "scoped_signature")?;
        let signed = &reply["data"]["signed_answer"];
        let field = |k: &str| signed[k].as_str().ok_or("scoped_signature");
        if field("format")? != "work-engine.owner-answer/v1" {
            return Err("scoped_signature");
        }
        let answer = field("answer_json")?;
        let a = proof::Assertion {
            credential_id: field("credential_id")?,
            authenticator_data: field("authenticator_data")?,
            client_data_json: field("client_data_json")?,
            signature: field("signature")?,
        };
        let key = crate::key_policy::load(&c.sources.key_policy, a.credential_id)
            .map_err(|_| "scoped_key")?;
        let policy = proof::KeyPolicy {
            owner: &c.sources.owner,
            rp_id: &c.sources.rp_id,
            origin: &c.sources.origin,
            credential_id: a.credential_id,
            spki_der: &key.spki_der,
            fingerprint: &key.fingerprint,
        };
        let verified =
            proof::verify(raw, answer, &a, &policy, clock.now()).map_err(|_| "scoped_signature")?;
        crate::board::own_close(&board, raw, answer, &a, &verified)
            .ok_or("scoped_closing_reply")?;
        let valid_until = previous.as_ref().map_or(verified.expires_at, |p| {
            p.valid_until.min(verified.expires_at)
        });
        previous = Some(Head {
            valid_until,
            generation: control.generation,
            sha: crate::hash(raw),
            state: control.state,
            raw: raw.clone(),
        });
    }
    let h = previous.ok_or("scoped_activation_missing")?;
    if clock.now() >= h.valid_until {
        return Err("scoped_signature");
    }
    Ok(h)
}
/// Apply the signed current head under the existing registry's write ordering.
/// Possession of the startup config/DB does not replace the signed board proof.
pub(crate) fn synchronize(db: &Connection, pin: &Pin, h: &Head) -> Result<()> {
    let fence = crate::registry_fence::Fence::acquire(db).map_err(|_| "scoped_registry")?;
    synchronize_fenced(db, pin, h, &fence)
}
// Callers already owning the registry fence must forward it, never reacquire.
pub(crate) fn synchronize_fenced(
    db: &Connection,
    pin: &Pin,
    h: &Head,
    _fence: &crate::registry_fence::Fence,
) -> Result<()> {
    let old: Option<(i64,String)> = db.query_row("SELECT generation,request_sha FROM scoped_control WHERE profile_sha=?1 ORDER BY generation DESC LIMIT 1", [&pin.sha256], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(|_| "scoped_registry")?;
    if let Some((generation, sha)) = old.as_ref() {
        if *generation > h.generation || (*generation == h.generation && sha != &h.sha) {
            return Err("scoped_head_rollback");
        }
        if *generation == h.generation {
            return Ok(());
        }
    }
    // The complete signed predecessor chain was verified above. Persist the
    // selected head; its predecessor stays in signed request_json for audit.
    let v: Value = serde_json::from_str(&h.raw).map_err(|_| "scoped_control")?;
    db.execute("INSERT INTO scoped_control(profile_sha,generation,request_sha,predecessor,state,request_json) VALUES(?1,?2,?3,?4,?5,?6)",params![pin.sha256,h.generation,h.sha,v.pointer("/data/scoped_control/predecessor").and_then(Value::as_str),h.state,h.raw]).map_err(|_| "scoped_registry")?;
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn observe_current(
    config_pin: &Pin,
    profile_pin: &Pin,
    profile: &Profile,
    db: &Connection,
    request: &str,
    budget: Option<&CanonicalSource>,
    now: OffsetDateTime,
) -> Result<Head> {
    let clock = source::AcquisitionTime::new(now);
    if !source::pinned(profile_pin) {
        return Err("scoped_profile_pin");
    }
    let c = config(config_pin)?;
    let e = match budget {
        Some(b) => inspect::collect_current(&c.sources, db, request, clock.now(), b),
        None => inspect::collect(&c.sources, db, request, clock.now()),
    }
    .map_err(|h| h.0)?;
    let mut h = head(&c, profile_pin, clock.now())?;
    if h.state != "active" || clock.now() >= h.valid_until {
        return Err("scoped_revoked");
    }
    let now = clock.now();
    let row: Row = serde_json::from_str(&e.original).map_err(|_| "scoped_task")?;
    let i = row.data.intent;
    let p = profile;
    let mode = row
        .data
        .intent_artifacts
        .iter()
        .find(|a| a.r#ref == i.authority.mode)
        .ok_or("scoped_mode")?;
    let mode: crate::ModeView =
        serde_json::from_str(&mode.content_json).map_err(|_| "scoped_mode")?;
    #[cfg(test)]
    if now >= utc(&p.expires_at)?
        || now >= utc(&i.clock.admit_until)?
        || now >= utc(&i.clock.close_by)?
        || now - e.budget.at > time::Duration::seconds(30)
    {
        eprintln!(
            "scoped_time_rejection: now={now} budget_at={} profile_until={} admit_until={} close_by={}",
            e.budget.at, p.expires_at, i.clock.admit_until, i.clock.close_by
        );
    }
    if e.task != p.task
        || i.proposal != p.proposal
        || i.authority.mode != p.mode
        || i.workspace.configuration != p.configuration
        || i.workspace.profile_id != p.id
        || i.submitter != p.actor
        || c.sources.actor != p.actor
        || i.budget.objective_id != p.objective
        || !i.effects.write_paths.contains(&p.effect.path)
        || i.effects.max_child_tasks != 0
        || i.effects.max_concurrent_tasks != 1
        || !i.prerequisites.contains(&p.parent)
        || !crate::budget_within(&i.budget.maximum, &p.maximum, "0")
        || !crate::budget_within(&i.budget.closure_reserve, &p.reserve, "0")
        || e.verified.expires_at <= now
        || e.budget.at > now
        || now - e.budget.at > time::Duration::seconds(30)
        || now >= utc(&p.expires_at)?
        || now >= utc(&i.clock.admit_until)?
        || now >= utc(&i.clock.close_by)?
        || mode.mode != "present"
        || mode.pending_transition.is_some()
        || utc(&mode.effective_at)? > now
    {
        return Err("scoped_task_binding");
    }
    h.valid_until = h
        .valid_until
        .min(e.verified.expires_at)
        .min(e.budget.at + time::Duration::seconds(30))
        .min(utc(&p.expires_at)?)
        .min(utc(&i.clock.admit_until)?)
        .min(utc(&i.clock.close_by)?);
    Ok(h)
}
impl ScopedOwner {
    /// Production constructor: independently loads signed activation and task
    /// proofs. Success establishes scoped authority, never native qualification.
    pub fn new(
        config_pin: Pin,
        request: String,
        profile_pin: Pin,
        now: OffsetDateTime,
    ) -> Result<Self> {
        let clock = source::AcquisitionTime::new(now);
        let c = config(&config_pin)?;
        if !source::pinned(&profile_pin) {
            return Err("scoped_profile_pin");
        }
        let profile: Profile =
            serde_json::from_slice(&service::protected(&profile_pin.path).ok_or("scoped_profile")?)
                .map_err(|_| "scoped_profile")?;
        if !((profile.schema == PROFILE && profile.final_stage.is_none())
            || (profile.schema == PROFILE_FINAL
                && profile.final_stage.as_ref().is_some_and(|s| {
                    s.requests == 2
                        && s.effects == 1
                        && (256..=4096).contains(&s.initial_output_tokens)
                        && (256..=4096).contains(&s.output_tokens)
                        && digest(&s.attempt)
                        && crate::budget_within(&profile.maximum, &s.initial_maximum, &s.maximum)
                        && crate::budget_within(&profile.reserve, &s.initial_reserve, &s.reserve)
                        && crate::budget_within(&s.initial_maximum, "0", &s.initial_reserve)
                        && crate::budget_within(&s.maximum, "0", &s.reserve)
                })))
            || profile.turn_rule != "one-first-owned-turn"
            || profile.trust
                != "configured-signed-board-and-key; same-uid-source-selection-trusted; sampled-native-checks; no-global-isolation"
            || !digest(&profile.transport_sha256)
            || uuid::Uuid::parse_str(&profile.target_generation).is_err()
            || uuid::Uuid::parse_str(&profile.bridge_store_id).is_err()
            || !digest(&profile.native_executable_sha256)
            || uuid::Uuid::parse_str(&profile.session).is_err()
            || !digest(&profile.entry_request_sha256)
            || !profile.root.is_absolute()
            || std::fs::canonicalize(&profile.root).ok().as_ref() != Some(&profile.root)
            || !crate::relative_write_path(&profile.publication)
            || Path::new(&profile.publication).components().count() != 1
            || profile.publication == profile.effect.path
            || !crate::relative_write_path(&profile.effect.path)
            || Path::new(&profile.effect.path).components().count() != 1
            || profile.effect.after.len() > 16384
            || !digest(&profile.effect.before_sha256)
            || crate::hash(&profile.effect.after) != profile.effect.after_sha256
        {
            return Err("scoped_profile");
        }
        let db = database(&c)?;
        let mut owner = Self {
            config: config_pin,
            request,
            profile_pin,
            profile,
            db,
            #[cfg(test)]
            before_effect_lease: None,
            #[cfg(test)]
            acquisition_delay: None,
        };
        owner.observe(None, clock.now())?;
        let tx = owner
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "scoped_registry")?;
        let h = observe_current(
            &owner.config,
            &owner.profile_pin,
            &owner.profile,
            &tx,
            &owner.request,
            None,
            clock.now(),
        )?;
        synchronize(&tx, &owner.profile_pin, &h)?;
        if h.state != "active" || clock.now() >= h.valid_until {
            return Err("scoped_revoked");
        }
        tx.commit().map_err(|_| "scoped_registry")?;
        Ok(owner)
    }
    pub fn admission_id(&self) -> Result<String> {
        let request: crate::request::AdmissionRequest =
            serde_json::from_str(&self.request).map_err(|_| "invalid_request")?;
        Ok(crate::hash(&format!(
            "dot-admission/v1\0{}\0{}",
            request.decision.id, self.profile.task.sha256
        )))
    }
    pub fn operation_id(&self) -> Result<String> {
        Ok(crate::hash(&format!(
            "dot-operation/v1\0{}",
            self.admission_id()?
        )))
    }
    pub fn check_settings(&self, model: &str, effort: &str, tier: &str) -> Result<()> {
        let raw: String = self
            .db
            .query_row(
                "SELECT request_json FROM task_revision WHERE task_id=?1 AND revision=?2",
                params![self.profile.task.id, self.profile.task.revision],
                |r| r.get(0),
            )
            .map_err(|_| "scoped_task")?;
        if crate::hash(&raw) != self.profile.task.sha256 {
            return Err("scoped_task");
        }
        let row: Row = serde_json::from_str(&raw).map_err(|_| "scoped_task")?;
        let m = row.data.intent.model;
        if model != m.id || effort != m.effort || tier != m.tier {
            return Err("scoped_native_settings");
        }
        Ok(())
    }
    /// Attributed observations from the concrete bridge, never a native permit.
    /// Missing worker/result/usage evidence stays unresolved in the projection.
    pub fn publish_observation(
        &mut self,
        path: &Path,
        native: &Value,
        receipts: &[Value],
        closure: &Value,
        source: &CanonicalSource,
        now: OffsetDateTime,
    ) -> Result<Value> {
        if path != self.profile.root.join(&self.profile.publication) {
            return Err("scoped_publication_path");
        }
        // Publication records evidence; it grants no current funding authority.
        let source_error = source.check_current().err();
        let (session, turn, state, effect): (String, String, String, Option<String>) = self
            .db
            .query_row(
                "SELECT session,turn,state,effect_sha FROM scoped_claim WHERE profile_sha=?1",
                [&self.profile_pin.sha256],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|_| "scoped_claim_missing")?;
        let mut normalized = native.clone();
        if normalized["error"].is_string()
            && normalized.get("session").is_none()
            && normalized.get("turn").is_none()
        {
            normalized["session"] = session.clone().into();
            normalized["turn"] = turn.clone().into();
            normalized["identity_source"] = "durable_scoped_claim".into();
        }
        if normalized.get("events").is_none() {
            normalized["events"] = json!([]);
        }
        normalized["journal_effect_state"] = state.clone().into();
        normalized["journal_effect_sha256"] =
            effect.clone().map(Value::String).unwrap_or(Value::Null);
        if normalized["error"].is_string() {
            normalized["worker_result"] = "missing_or_unverified".into();
        }
        let native = &normalized;
        if native["session"] != session || native["turn"] != turn {
            return Err("scoped_result_identity");
        }
        let admission = self.admission_id()?;
        let raw: String = self
            .db
            .query_row(
                "SELECT request_json FROM task_revision WHERE task_id=?1 AND revision=?2",
                params![self.profile.task.id, self.profile.task.revision],
                |r| r.get(0),
            )
            .map_err(|_| "scoped_task")?;
        let row: Row = serde_json::from_str(&raw).map_err(|_| "scoped_task")?;
        let at = now.format(&Rfc3339).map_err(|_| "scoped_clock")?;
        let profile_json =
            std::fs::read_to_string(&self.profile_pin.path).map_err(|_| "scoped_profile_pin")?;
        if crate::hash(&profile_json) != self.profile_pin.sha256 {
            return Err("scoped_profile_pin");
        }
        let body=json!({"schema":"dot.scoped-observation/v1","profile_json":profile_json,"profile_sha256":self.profile_pin.sha256,"task":self.profile.task,"session":session,"turn":turn,"native":native,"canonical_receipts":receipts,"closure":closure,"effect_state":state,"effect_sha256":effect,"canonical_source_sha256":source.current_sha256(),"canonical_source_identity":"last_acknowledged","canonical_source_error":source_error}).to_string();
        if body.len() > 1024 * 1024 {
            return Err("scoped_result_oversize");
        }
        let revision = crate::hash(&body);
        let provenance = Ref {
            id: "scoped-native-observation".into(),
            revision: "1".into(),
            sha256: revision.clone(),
        };
        let events = native["events"].as_array().ok_or("scoped_result_events")?;
        let final_stage_released = if let Some(stage) = &self.profile.final_stage {
            native.pointer("/provider/final/state")==Some(&json!("sdk_delivery_unknown"))
                && !native.pointer("/provider/error").is_some_and(Value::is_string)
                && self.db.query_row("SELECT count(*) FROM scoped_final_claim WHERE profile_sha=?1 AND attempt=?2",params![self.profile_pin.sha256,stage.attempt],|r|r.get::<_,i64>(0)).map_err(|_|"scoped_registry")?==1
        } else {
            true
        };
        let packet = if source_error.is_none()
            && native["terminal_observed"] == true
            && !native["error"].is_string()
            && final_stage_released
        {
            selected_final_text(events, &session, &turn)
                .and_then(|text| crate::result::packet(text, &self.profile.task, &admission))
        } else {
            None
        };
        let clocks = || crate::result::Clock {
            first_funded_at: None,
            admitted_at: None,
            observed_start_at: None,
            close_by: &row.data.intent.clock.close_by,
        };
        let usage = native
            .pointer("/provider/provider_reported_usage")
            .filter(|v| !v.is_null())
            .map(|v| crate::hash(&v.to_string()));
        let accounting = || crate::result::Accounting {
            units: "speed_adjusted_api_usd",
            limit: &self.profile.maximum,
            reservation: Some(&revision),
            observed_usage: usage.as_deref(),
            coverage: "all_objective_actors",
            unsettled: Some(
                "canonical reservation/effect settlement is not inferred from terminal or local closure",
            ),
        };
        let closed = closure["pid"].is_i64()
            && closure["wait_status"].is_string()
            && closure["owned_group_signalled"] == true;
        let custody = || crate::result::Custody {
            native: crate::result::Disposition {
                id: Some(&session),
                disposition: if closed {
                    "owned_process_wait_observed"
                } else {
                    "unknown"
                },
            },
            children: vec![],
            processes: vec![],
        };
        let view = match packet.as_ref() {
            Some(p) => crate::result::observed(
                &self.profile.task,
                &admission,
                &session,
                &turn,
                p,
                provenance.clone(),
                clocks(),
                accounting(),
                custody(),
                &at,
                &revision,
            )
            .ok_or("scoped_result_packet")?,
            None => crate::result::unresolved(
                &self.profile.task,
                &admission,
                Some(&session),
                Some(&turn),
                false,
                clocks(),
                accounting(),
                custody(),
                &at,
                &revision,
            ),
        };
        let mut view = serde_json::to_value(view).map_err(|_| "scoped_result_packet")?;
        let exceptions = view["exceptions"]
            .as_array_mut()
            .ok_or("scoped_result_packet")?;
        exceptions.push("canonical_accounting_unsettled".into());
        if source_error.is_some() {
            exceptions.push("canonical_source_uncertain".into());
        }
        if state != "effect_observed" {
            exceptions.push("disposable_effect_unknown_or_missing".into());
        }
        if !closed {
            exceptions.push("native_closure_unknown".into());
        }
        let worker_status = view["operational_status"].clone();
        let worker_outcome = view["outcome"].clone();
        let items = vec![crate::projection::TaskProjection {
            task: self.profile.task.clone(),
            request_json: raw,
            admission: None,
            result: Some(view),
        }];
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "scoped_registry")?;
        let old: Option<String> = tx
            .query_row(
                "SELECT revision FROM scoped_publication WHERE profile_sha=?1",
                [&self.profile_pin.sha256],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| "scoped_registry")?;
        if old.as_ref().is_some_and(|s| s != &revision) {
            return Err("scoped_result_conflict");
        }
        tx.execute(
            "INSERT OR IGNORE INTO scoped_publication(profile_sha,revision,body) VALUES(?1,?2,?3)",
            params![self.profile_pin.sha256, revision, body],
        )
        .map_err(|_| "scoped_registry")?;
        tx.commit().map_err(|_| "scoped_registry")?;
        crate::projection::publish(path, &at, &revision, &items)
            .map_err(|_| "scoped_publication_unknown")?;
        Ok(
            json!({"path":path,"revision":revision,"state":"published_attributed_observation","usage_settlement":"unknown","session":session,"turn":turn,"effect_state":state,"effect_sha256":effect,"worker_result_status":worker_status,"worker_outcome":worker_outcome}),
        )
    }
    pub fn profile(&self) -> &Profile {
        &self.profile
    }
    pub fn profile_sha256(&self) -> &str {
        &self.profile_pin.sha256
    }
    pub fn budget_source(&self) -> Result<CanonicalSource> {
        let c = config(&self.config)?;
        let s = c
            .sources
            .canonical_budget
            .as_ref()
            .ok_or("budget_source_contract_missing")?;
        if s.contract != "canonical-envelope-source/v1" {
            return Err("budget_source_contract_mismatch");
        }
        CanonicalSource::new(
            s.scope.clone(),
            c.sources.ledger.clone(),
            c.sources.budget_record.clone(),
        )
    }
    fn observe(&self, budget: Option<&CanonicalSource>, now: OffsetDateTime) -> Result<Head> {
        observe_current(
            &self.config,
            &self.profile_pin,
            &self.profile,
            &self.db,
            &self.request,
            budget,
            now,
        )
    }
    /// Claim data is taken from the bridge's actual ACK. This method records
    /// authority-side identity; it does not attest that a caller owns a runtime.
    #[allow(clippy::too_many_arguments)]
    pub fn claim(
        &mut self,
        session: &str,
        turn: &str,
        entry: &str,
        attempt: &str,
        request: &str,
        budget: &CanonicalSource,
        now: OffsetDateTime,
    ) -> Result<()> {
        let clock = source::AcquisitionTime::new(now);
        let h = self.observe(Some(budget), clock.now())?;
        if session != self.profile.session
            || entry != self.profile.entry_request_sha256
            || uuid::Uuid::parse_str(turn).is_err()
            || !digest(attempt)
            || !digest(request)
        {
            return Err("scoped_native_binding");
        }
        let c = config(&self.config)?;
        let e = inspect::collect_current(&c.sources, &self.db, &self.request, clock.now(), budget)
            .map_err(|e| e.0)?;
        let row: Row = serde_json::from_str(&e.original).map_err(|_| "scoped_task")?;
        let i = row.data.intent;
        let parsed: crate::request::AdmissionRequest =
            serde_json::from_str(&self.request).map_err(|_| "invalid_request")?;
        let gate = crate::store::CurrentGate {
            task_id: e.task.id.clone(),
            revision: e.task.revision.clone(),
            request_sha: e.task.sha256.clone(),
            lane_id: i.lane_id,
            proposal: i.proposal,
            decision: e.decision.clone(),
            objective_id: i.budget.objective_id,
            canonical_budget_revision: e.budget_record_sha256.clone(),
            observation_ref: e.budget.source_sha.clone(),
            exclusive_binding_ref: format!("scoped-session-authority:{}", h.sha),
            maximum: i.budget.maximum,
            productive_maximum: i.budget.productive_maximum,
            closure_reserve: i.budget.closure_reserve,
        };
        let admission = crate::store::commit(
            &mut self.db,
            &parsed.request_id,
            &e.original,
            &e.answer,
            &e.verified,
            &gate,
        )
        .map_err(|_| "scoped_admission_conflict")?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "scoped_registry")?;
        let current = observe_current(
            &self.config,
            &self.profile_pin,
            &self.profile,
            &tx,
            &self.request,
            Some(budget),
            clock.now(),
        )?;
        if current.sha != h.sha || current.state != "active" || clock.now() >= current.valid_until {
            return Err("scoped_revoked");
        }
        synchronize(&tx, &self.profile_pin, &h)?;
        let old: Option<(String,String,String,String,String,String)> = tx.query_row("SELECT activation_sha,session,turn,entry_sha,attempt,request_sha FROM scoped_claim WHERE profile_sha=?1",[&self.profile_pin.sha256],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(|_|"scoped_registry")?;
        if let Some(old) = old {
            if old
                != (
                    h.sha,
                    session.into(),
                    turn.into(),
                    entry.into(),
                    attempt.into(),
                    request.into(),
                )
            {
                return Err("scoped_turn_already_claimed");
            }
        } else {
            tx.execute("INSERT INTO scoped_claim(profile_sha,operation_id,activation_sha,session,turn,entry_sha,attempt,request_sha,state) VALUES(?1,?8,?2,?3,?4,?5,?6,?7,'claimed')",params![self.profile_pin.sha256,h.sha,session,turn,entry,attempt,request,admission.operation]).map_err(|_|"scoped_turn_already_claimed")?;
        }
        tx.commit().map_err(|_| "scoped_registry")
    }
    pub fn recheck(
        &self,
        session: &str,
        turn: &str,
        request: &str,
        budget: &CanonicalSource,
        now: OffsetDateTime,
    ) -> Result<()> {
        let clock = source::AcquisitionTime::new(now);
        let h = self.observe(Some(budget), clock.now())?;
        let count: i64 = self.db.query_row("SELECT count(*) FROM scoped_claim WHERE profile_sha=?1 AND activation_sha=?2 AND session=?3 AND turn=?4 AND request_sha=?5",params![self.profile_pin.sha256,h.sha,session,turn,request],|r|r.get(0)).map_err(|_|"scoped_registry")?;
        if count != 1 {
            return Err("scoped_claim_mismatch");
        }
        Ok(())
    }
    /// Admit one distinct final request without modifying the original claim.
    /// Native custody is established by the bridge; this owner independently
    /// checks the current signed allowance, prior effect and callback binding.
    #[allow(clippy::too_many_arguments)]
    pub fn admit_final(
        &mut self,
        session: &str,
        turn: &str,
        first_attempt: &str,
        first_request: &str,
        attempt: &str,
        request: &str,
        callback: &Value,
        budget: &CanonicalSource,
        now: OffsetDateTime,
    ) -> Result<()> {
        let clock = source::AcquisitionTime::new(now);
        let allowance = self
            .profile
            .final_stage
            .as_ref()
            .ok_or("scoped_final_not_admitted")?;
        if self.profile.schema != PROFILE_FINAL
            || attempt != allowance.attempt
            || attempt == first_attempt
            || request == first_request
            || !digest(request)
        {
            return Err("scoped_final_binding");
        }
        self.recheck(session, turn, first_request, budget, clock.now())?;
        let original:(String,String,String,String)=self.db.query_row(
            "SELECT attempt,request_sha,state,operation_id FROM scoped_claim WHERE profile_sha=?1",
            [&self.profile_pin.sha256],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_|"scoped_registry")?;
        if original.0 != first_attempt
            || original.1 != first_request
            || original.2 != "effect_observed"
        {
            return Err("scoped_final_effect_unknown");
        }
        let params = &callback["params"];
        let item = &params["item"];
        let content = item["contentItems"]
            .as_array()
            .ok_or("scoped_final_callback")?;
        if callback["method"] != "item/completed"
            || params["threadId"] != session
            || params["turnId"] != turn
            || item["type"] != "dynamicToolCall"
            || item["tool"] != "exact_disposable_edit"
            || item["status"] != "completed"
            || item["success"] != true
            || item["arguments"] != json!({"operation_id":original.3})
            || content.len() != 1
            || content[0]["type"] != "inputText"
        {
            return Err("scoped_final_callback");
        }
        let output: Value =
            serde_json::from_str(content[0]["text"].as_str().ok_or("scoped_final_callback")?)
                .map_err(|_| "scoped_final_callback")?;
        if output["state"] != "effect_observed"
            || output["operation_id"] != original.3
            || output["session"] != session
            || output["turn"] != turn
            || output["attempt"] != first_attempt
            || output["request_sha256"] != first_request
            || output["after_sha256"] != self.profile.effect.after_sha256
        {
            return Err("scoped_final_callback");
        }
        let call_id = item["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("scoped_final_callback")?;
        let raw = callback.to_string();
        let callback_sha = crate::hash(&raw);
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "scoped_registry")?;
        let h = observe_current(
            &self.config,
            &self.profile_pin,
            &self.profile,
            &tx,
            &self.request,
            Some(budget),
            clock.now(),
        )?;
        synchronize(&tx, &self.profile_pin, &h)?;
        if h.state != "active" || clock.now() >= h.valid_until {
            return Err("scoped_revoked");
        }
        tx.execute(
            "INSERT INTO scoped_final_claim VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                self.profile_pin.sha256,
                first_attempt,
                first_request,
                attempt,
                request,
                callback_sha,
                call_id,
                raw
            ],
        )
        .map_err(|_| "scoped_final_already_claimed")?;
        tx.commit().map_err(|_| "scoped_registry")
    }
    pub fn recheck_final(
        &self,
        session: &str,
        turn: &str,
        attempt: &str,
        request: &str,
        budget: &CanonicalSource,
        now: OffsetDateTime,
    ) -> Result<()> {
        let clock = source::AcquisitionTime::new(now);
        let first:String=self.db.query_row("SELECT first_request FROM scoped_final_claim WHERE profile_sha=?1 AND attempt=?2 AND request_sha=?3",params![self.profile_pin.sha256,attempt,request],|r|r.get(0)).map_err(|_|"scoped_final_binding")?;
        self.recheck(session, turn, &first, budget, clock.now())
    }
    /// Durable unknown fence precedes external I/O. The returned lease holds
    /// the registry's revocation ordering lock, but has no I/O capability.
    pub fn fence<'a>(
        &'a mut self,
        session: &str,
        turn: &str,
        request: &str,
        budget: &CanonicalSource,
        now: OffsetDateTime,
    ) -> Result<EffectLease<'a>> {
        #[allow(unused_mut)]
        let mut clock = source::AcquisitionTime::new(now);
        self.recheck(session, turn, request, budget, clock.now())?;
        let operation: String = self
            .db
            .query_row(
                "SELECT operation_id FROM scoped_claim WHERE profile_sha=?1",
                [&self.profile_pin.sha256],
                |r| r.get(0),
            )
            .map_err(|_| "scoped_claim_mismatch")?;
        let state: String = self
            .db
            .query_row(
                "SELECT state FROM outbox WHERE operation_id=?1",
                [&operation],
                |r| r.get(0),
            )
            .map_err(|_| "scoped_registry")?;
        if state == "pending" {
            crate::store::fence_delivery(&self.db, &operation)
                .map_err(|_| "scoped_effect_unknown_no_retry")?;
        }
        let changed = self.db.execute("UPDATE scoped_claim SET state='effect_unknown' WHERE profile_sha=?1 AND state='claimed'",[&self.profile_pin.sha256]).map_err(|_|"scoped_registry")?;
        if changed != 1 {
            return Err("scoped_effect_unknown_no_retry");
        }
        #[cfg(test)]
        if let Some(barrier) = self.before_effect_lease.take() {
            barrier();
        }
        #[cfg(test)]
        if let Some(delay) = self.acquisition_delay.take() {
            clock.advance(delay);
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "scoped_registry")?;
        // Fence durability and lease authority have different lifetimes. Recheck
        // the admitted heads under the write lock retained through file I/O.
        crate::store::admitted_heads_live(&tx, &operation)
            .map_err(|_| "scoped_effect_unknown_no_retry")?;
        let h = observe_current(
            &self.config,
            &self.profile_pin,
            &self.profile,
            &tx,
            &self.request,
            Some(budget),
            clock.now(),
        )?;
        synchronize(&tx, &self.profile_pin, &h)?;
        if h.state != "active" || clock.now() >= h.valid_until {
            return Err("scoped_revoked");
        }
        Ok(EffectLease {
            valid_until: h.valid_until,
            clock,
            tx,
            profile_sha: self.profile_pin.sha256.clone(),
            effect: self.profile.effect.clone(),
        })
    }
}
/// No public filesystem executor exists in admission. This is a durable effect
/// journal lease, not evidence of native custody or an executable permission.
pub struct EffectLease<'a> {
    valid_until: OffsetDateTime,
    clock: source::AcquisitionTime,
    tx: Transaction<'a>,
    profile_sha: String,
    effect: Effect,
}
impl EffectLease<'_> {
    /// Revalidate elapsed authority immediately before the consumer's first
    /// mutation, after its blocking preparation. The transaction still orders
    /// revocations; expiry leaves the committed unknown fence intact.
    pub fn check_current(&self) -> Result<()> {
        if self.clock.now() >= self.valid_until {
            return Err("scoped_effect_expired_no_retry");
        }
        Ok(())
    }

    pub fn effect(&self) -> &Effect {
        &self.effect
    }
    pub fn observed(self, sha: &str) -> Result<()> {
        if sha != self.effect.after_sha256 {
            return Err("scoped_effect_digest");
        }
        self.tx.execute("UPDATE scoped_claim SET state='effect_observed',effect_sha=?2 WHERE profile_sha=?1 AND state='effect_unknown'",params![self.profile_sha,sha]).map_err(|_|"scoped_registry")?;
        self.tx.commit().map_err(|_| "scoped_registry")
    }
}
/// Authenticated control producer usable by the coordinator without granting
/// it signing authority. The complete board and current key remain authoritative.
pub fn ingest_control(config_pin: Pin, profile_pin: Pin, now: OffsetDateTime) -> Result<Value> {
    let clock = source::AcquisitionTime::new(now);
    if !source::pinned(&profile_pin) {
        return Err("scoped_profile_pin");
    }
    let c = config(&config_pin)?;
    let mut db = database(&c)?;
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| "scoped_registry")?;
    let h = head(&c, &profile_pin, clock.now())?;
    synchronize(&tx, &profile_pin, &h)?;
    tx.commit().map_err(|_| "scoped_registry")?;
    Ok(
        json!({"profile_sha256":profile_pin.sha256,"generation":h.generation,"request_sha256":h.sha,"state":h.state}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
    use p256::{
        ecdsa::{Signature, SigningKey, signature::Signer},
        pkcs8::EncodePublicKey,
    };
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    use time::macros::datetime;
    const NOW: OffsetDateTime = datetime!(2026-11-02 01:00:10 UTC);
    struct Fixture {
        dir: PathBuf,
        config: Pin,
        profile: Pin,
        request: String,
        rows: Vec<String>,
        key: SigningKey,
        now: OffsetDateTime,
    }
    fn put(path: &Path, bytes: impl AsRef<[u8]>) {
        std::fs::write(path, bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn pin(path: PathBuf) -> Pin {
        Pin {
            sha256: format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
            path,
        }
    }
    impl Fixture {
        fn new() -> Self {
            Self::new_at(None, NOW)
        }
        fn new_at(export: Option<PathBuf>, now: OffsetDateTime) -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let dir = export.unwrap_or_else(|| {
                std::env::temp_dir().join(format!(
                    "scoped-pilot-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                ))
            });
            std::fs::create_dir(&dir).unwrap();
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
            let fixture: Value =
                serde_json::from_str(include_str!("../tests/projection-fixture.json")).unwrap();
            let mut task: Value =
                serde_json::from_str(fixture["tasks"][0]["request_json"].as_str().unwrap())
                    .unwrap();
            task["sequence"] = 1.into();
            task["data"]["intent"]["workspace"]["profile_id"] = PROFILE.into();
            task["data"]["intent"]["effects"]["write_paths"] = json!(["edit.txt"]);
            task["data"]["intent"]["clock"]["admit_until"] = (now + time::Duration::minutes(10))
                .format(&Rfc3339)
                .unwrap()
                .into();
            task["data"]["intent"]["clock"]["close_by"] = (now + time::Duration::minutes(20))
                .format(&Rfc3339)
                .unwrap()
                .into();
            let prior = task["data"]["intent"]["prerequisites"][0]["id"].clone();
            task["data"]["intent"]["prerequisites"][0]["id"] = "d".repeat(64).into();
            for a in task["data"]["intent_artifacts"].as_array_mut().unwrap() {
                if a["ref"]["id"] == prior {
                    a["ref"]["id"] = "d".repeat(64).into();
                }
            }
            let raw = task.to_string();
            let task_ref = crate::validate_review(&raw).unwrap();
            let i = &task["data"]["intent"];
            let key = SigningKey::from_slice(&[7; 32]).unwrap();
            let spki = key.verifying_key().to_public_key_der().unwrap();
            put(&dir.join("keys"),json!({"keys":{"key":{"spki_b64url":B64.encode(spki.as_bytes()),"fingerprint":format!("{:x}",Sha256::digest(spki.as_bytes())),"cose_alg":-7}}}).to_string());
            put(&dir.join("record"), b"fixture");
            let python = pin(std::fs::canonicalize("/usr/bin/python3").unwrap());
            let program = |name: &str, body: String| {
                let path = dir.join(name);
                put(&path, body);
                source::Program {
                    python: python.clone(),
                    script: pin(path),
                    dependencies: vec![],
                }
            };
            let board = program(
                "board.py",
                format!(
                    "from pathlib import Path\nprint(Path({:?}).read_text())",
                    dir.join("board.json").to_str().unwrap()
                ),
            );
            let mut cost = json!({"schema":"objective-budget-control-v1","topic":"control:budget:fixture-objective","closed_at":null,"snapshot":{"snapshot_at":"2026-11-02T01:00:00Z","status":"complete","totals":{"codex":{"quota_usd_low":1,"quota_usd_high":1,"unpriced_calls":0},"claude_all_models":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0},"claude_fable":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0}},"cost_controls":[{"pool":"codex","node_id":null,"max_usd":5,"basis":"speed_adjusted_api_usd","remaining_usd":4,"observed":{"quota_usd_low":1,"quota_usd_high":1,"unpriced_calls":0,"cost_basis":"speed_adjusted_api_usd"}}]}});
            cost["snapshot"]["snapshot_at"] = now.format(&Rfc3339).unwrap().into();
            put(&dir.join("cost.json"), cost.to_string());
            let ledger = program(
                "ledger.py",
                format!(
                    "from pathlib import Path\nprint(Path({:?}).read_text())",
                    dir.join("cost.json").to_str().unwrap()
                ),
            );
            let record = pin(dir.join("record"));
            let export = json!({"schema":"canonical-envelope-source/v1","path":record.path,"before_sha256":record.sha256,"after_sha256":record.sha256,"authority_sha256":"a".repeat(64)});
            let mut scope = program("scope.py", format!("print({:?})", export.to_string()));
            scope.dependencies.push(ledger.script.clone());
            let sources = inspect::Sources {
                board,
                board_repository: dir.clone(),
                key_policy: dir.join("keys"),
                owner: "owner".into(),
                rp_id: "auth.example".into(),
                origin: "https://auth.example".into(),
                actor: "fixture-actor".into(),
                ledger,
                budget_record: record,
                canonical_budget: Some(inspect::CanonicalBudgetConfig {
                    contract: "canonical-envelope-source/v1".into(),
                    scope,
                }),
            };
            let db_path = dir.join("registry");
            let mut db = Connection::open(&db_path).unwrap();
            std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600)).unwrap();
            crate::store::initialize(&db).unwrap();
            let row: Row = serde_json::from_str(&raw).unwrap();
            crate::store::register_proposal(&mut db, &row.data.intent.proposal, "live", None)
                .unwrap();
            crate::store::register_task(&mut db, &raw, "live", None).unwrap();
            put(&dir.join("config"),json!({"schema":"dot.admission-config/v2-owned-source","database":db_path,"sources":sources}).to_string());
            let mut profile = json!({"schema":PROFILE,"id":PROFILE,"task":task_ref,"proposal":i["proposal"],"mode":i["authority"]["mode"],"configuration":i["workspace"]["configuration"],"actor":"fixture-actor","objective":"fixture-objective","session":"00000000-0000-4000-8000-000000000001","turn_rule":"one-first-owned-turn","entry_request_sha256":"e".repeat(64),"native_executable_sha256":"f".repeat(64),"target_generation":"00000000-0000-4000-8000-000000000004","bridge_store_id":"00000000-0000-4000-8000-000000000005","transport_sha256":"0".repeat(64),"root":dir,"publication":"published.json","effect":{"path":"edit.txt","before_sha256":crate::hash("before"),"after":"after","after_sha256":crate::hash("after")},"parent":i["prerequisites"][0],"maximum":"0.1","reserve":"0.1","expires_at":"2026-11-02T01:02:00Z","trust":"configured-signed-board-and-key; same-uid-source-selection-trusted; sampled-native-checks; no-global-isolation"});
            profile["expires_at"] = (now + time::Duration::minutes(2))
                .format(&Rfc3339)
                .unwrap()
                .into();
            put(&dir.join("profile"), profile.to_string());
            put(&dir.join("edit.txt"), "before");
            let mut f = Self {
                config: pin(dir.join("config")),
                profile: pin(dir.join("profile")),
                dir,
                request: String::new(),
                rows: vec![raw],
                key,
                now,
            };
            f.sign_last();
            let reply: Value = serde_json::from_str(&f.rows[1]).unwrap();
            f.request=json!({"schema":"dot.admission-request/v1-proposal","request_id":"request","task":task_ref,"decision":{"id":reply["message_id"],"revision":"2","sha256":crate::hash(&f.rows[1])},"expected_task_revision":"r1"}).to_string();
            f.save();
            f
        }
        fn sign_last(&mut self) {
            let raw = self.rows.last().unwrap();
            let row: Value = serde_json::from_str(raw).unwrap();
            let seq = row["sequence"].as_u64().unwrap();
            let id = format!("00000000-0000-4000-8000-{seq:012}");
            let answer=json!({"v":1,"type":"work-engine.owner-answer","answer_id":id,"board_seq":seq,"request_digest":crate::hash(raw),"decision":"approve","comment":"","owner":"owner","created_at":(self.now-time::Duration::minutes(1)).format(&Rfc3339).unwrap(),"expires_at":(self.now+time::Duration::days(1)).format(&Rfc3339).unwrap(),"rp_id":"auth.example"}).to_string();
            let client=json!({"type":"webauthn.get","challenge":B64.encode(Sha256::digest(answer.as_bytes())),"origin":"https://auth.example","crossOrigin":false}).to_string();
            let mut auth = Sha256::digest(b"auth.example").to_vec();
            auth.extend_from_slice(&[5, 0, 0, 0, 0]);
            let mut message = auth.clone();
            message.extend_from_slice(&Sha256::digest(client.as_bytes()));
            let sig: Signature = self.key.sign(&message);
            self.rows.push(json!({"kind":"reply","sequence":seq+1,"in_reply_to":seq,"topic":row["topic"],"to":[format!("session:{}",row["session_id"].as_str().unwrap())],"message_id":id,"data":{"disposition":"answered","decision":"approve","comment":"","answer_id":id,"signed_answer":{"format":"work-engine.owner-answer/v1","answer_json":answer,"credential_id":"key","authenticator_data":B64.encode(auth),"client_data_json":B64.encode(client),"signature":B64.encode(sig.to_der().as_bytes())}}}).to_string());
        }
        fn control(&mut self, state: &str) {
            let seq = self.rows.len() + 1;
            let predecessor = if seq > 3 {
                Some(crate::hash(&self.rows[self.rows.len() - 2]))
            } else {
                None
            };
            self.rows.push(json!({"kind":"request","sequence":seq,"topic":"scoped:test","session_id":"fixture-session","data":{"scoped_control":{"schema":"dot.scoped-control/v1","profile_sha256":self.profile.sha256,"generation":(seq-1)/2,"predecessor":predecessor,"state":state}}}).to_string());
            self.sign_last();
            self.save();
        }
        fn save(&self) {
            put(&self.dir.join("board.json"),json!({"revision":"a".repeat(40),"next_sequence":self.rows.len()+1,"rows":self.rows}).to_string());
        }
        fn owner(&self) -> Result<ScopedOwner> {
            ScopedOwner::new(
                self.config.clone(),
                self.request.clone(),
                self.profile.clone(),
                NOW,
            )
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
    #[test]
    fn effect_expiry_after_lease_return_keeps_unknown_and_rejects_retry() {
        let mut f = Fixture::new();
        f.control("active");
        let mut owner = f.owner().unwrap();
        let source = owner.budget_source().unwrap();
        let session = owner.profile.session.clone();
        let entry = owner.profile.entry_request_sha256.clone();
        let turn = "00000000-0000-4000-8000-000000000002";
        let request = "2".repeat(64);
        owner
            .claim(
                &session,
                turn,
                &entry,
                &"1".repeat(64),
                &request,
                &source,
                NOW,
            )
            .unwrap();
        let target = owner.profile.root.join(&owner.profile.effect.path);
        let before = std::fs::read(&target).unwrap();
        let record = std::fs::read(source.current().path.clone()).unwrap();
        let mut lease = owner.fence(&session, turn, &request, &source, NOW).unwrap();
        assert_eq!(lease.check_current(), Ok(()));
        lease.clock.advance(time::Duration::seconds(60));
        assert_eq!(lease.check_current(), Err("scoped_effect_expired_no_retry"));
        drop(lease);
        assert_eq!(std::fs::read(&target).unwrap(), before);
        assert_eq!(
            std::fs::read(source.current().path.clone()).unwrap(),
            record
        );
        let states: (String,String,String) = owner.db.query_row("SELECT s.state,o.state,c.state FROM scoped_claim s JOIN outbox o ON o.operation_id=s.operation_id JOIN capacity_claim c ON c.operation_id=s.operation_id",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_eq!(
            states,
            ("effect_unknown".into(), "unknown".into(), "unknown".into())
        );
        assert!(owner.fence(&session, turn, &request, &source, NOW).is_err());
    }
    #[test]
    fn canonical_collect_current_accounts_for_precollection_source_check() {
        for (offset, expired) in [(0, false), (60, false), (-60, false), (0, true)] {
            let at = OffsetDateTime::now_utc();
            let f = Fixture::new_at(None, at);
            let mut c = config(&f.config).unwrap();
            let path = c.sources.ledger.script.path.clone();
            put(
                &path,
                format!(
                    "import json,datetime\nfrom pathlib import Path\nd=json.loads(Path({:?}).read_text())\nd['snapshot']['snapshot_at']=(datetime.datetime.now(datetime.timezone.utc)+datetime.timedelta(seconds={})).isoformat()\nprint(json.dumps(d))\n",
                    f.dir.join("cost.json").to_str().unwrap(),
                    if expired { 172800 } else { offset }
                ),
            );
            c.sources.ledger.script = pin(path);
            let contract = c.sources.canonical_budget.as_mut().unwrap();
            contract.scope.dependencies = vec![c.sources.ledger.script.clone()];
            let source = CanonicalSource::new(
                contract.scope.clone(),
                c.sources.ledger.clone(),
                c.sources.budget_record.clone(),
            )
            .unwrap();
            source
                .before_check_delay
                .set(Some(std::time::Duration::from_millis(400)));
            if expired {
                source.check_elapsed.set(Some(time::Duration::days(2)));
            }
            let db = database(&c).unwrap();
            let start = OffsetDateTime::now_utc();
            let result = inspect::collect_current(&c.sources, &db, &f.request, start, &source);
            if expired {
                assert!(
                    start < at + time::Duration::days(1),
                    "proof must be current before acquisition"
                );
                assert!(matches!(result, Err(inspect::Held("proof_invalid"))));
            } else if offset == 0 {
                let evidence = result
                    .unwrap_or_else(|held| panic!("fresh canonical snapshot rejected: {}", held.0));
                assert!(evidence.budget.at > start);
            } else {
                assert!(
                    matches!(result, Err(inspect::Held("budget_unavailable"))),
                    "offset {offset}"
                );
            }
            assert!(source.before_check_delay.get().is_none());
        }
    }
    #[test]
    fn delayed_effect_lease_rejects_expired_profile_and_task_deadlines() {
        for (seconds, profile_seconds) in [(120, 120), (600, 1800)] {
            let mut f = Fixture::new();
            let mut profile: Value =
                serde_json::from_slice(&std::fs::read(&f.profile.path).unwrap()).unwrap();
            profile["expires_at"] = (NOW + time::Duration::seconds(profile_seconds))
                .format(&Rfc3339)
                .unwrap()
                .into();
            put(&f.profile.path, profile.to_string());
            f.profile = pin(f.profile.path.clone());
            f.rows.truncate(2);
            f.control("active");
            let mut owner = f.owner().unwrap();
            let source = owner.budget_source().unwrap();
            let turn = "00000000-0000-4000-8000-000000000002";
            let attempt = "1".repeat(64);
            let request = "2".repeat(64);
            owner
                .claim(
                    &owner.profile.session.clone(),
                    turn,
                    &owner.profile.entry_request_sha256.clone(),
                    &attempt,
                    &request,
                    &source,
                    NOW,
                )
                .unwrap();
            let at = NOW + time::Duration::seconds(seconds) - time::Duration::seconds(30);
            let mut cost: Value =
                serde_json::from_slice(&std::fs::read(f.dir.join("cost.json")).unwrap()).unwrap();
            cost["snapshot"]["snapshot_at"] = at.format(&Rfc3339).unwrap().into();
            put(&f.dir.join("cost.json"), cost.to_string());
            let reached = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let signal = reached.clone();
            let cost_path = f.dir.join("cost.json");
            cost["snapshot"]["snapshot_at"] = (NOW + time::Duration::seconds(seconds + 30))
                .format(&Rfc3339)
                .unwrap()
                .into();
            owner.acquisition_delay = Some(time::Duration::seconds(60));
            owner.before_effect_lease = Some(Box::new(move || {
                signal.store(true, std::sync::atomic::Ordering::SeqCst);
                put(&cost_path, cost.to_string());
            }));
            let session = owner.profile.session.clone();
            match owner.fence(&session, turn, &request, &source, at) {
                Err(error) => assert_eq!(error, "scoped_task_binding"),
                Ok(_) => panic!("expired authority granted a lease"),
            }
            assert!(reached.load(std::sync::atomic::Ordering::SeqCst));
            let state: String = owner
                .db
                .query_row("SELECT state FROM scoped_claim", [], |r| r.get(0))
                .unwrap();
            assert_eq!(state, "effect_unknown");
            assert!(owner.fence(&session, turn, &request, &source, at).is_err());
        }
    }
    #[test]
    fn export_scoped_native_fixture() {
        let Some(path) = std::env::var_os("SCOPED_PILOT_INPUTS") else {
            return;
        };
        let path = PathBuf::from(path);
        let until = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while !path.exists() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let inputs: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let dir = PathBuf::from(inputs["fixture_root"].as_str().unwrap());
        let existing = dir.join("admission-request.json").exists();
        let mut f = if existing {
            let board: Value =
                serde_json::from_slice(&std::fs::read(dir.join("board.json")).unwrap()).unwrap();
            Fixture {
                config: pin(dir.join("config")),
                profile: pin(dir.join("profile")),
                request: std::fs::read_to_string(dir.join("admission-request.json")).unwrap(),
                rows: serde_json::from_value(board["rows"].clone()).unwrap(),
                key: SigningKey::from_slice(&[7; 32]).unwrap(),
                now: OffsetDateTime::now_utc(),
                dir,
            }
        } else {
            Fixture::new_at(Some(dir), OffsetDateTime::now_utc())
        };
        if !existing {
            let mut raw: Value = serde_json::from_str(&f.rows[0]).unwrap();
            raw["data"]["intent"]["model"] = inputs["model"].clone();
            f.rows = vec![raw.to_string()];
            f.sign_last();
            let c = config(&f.config).unwrap();
            let mut db = database(&c).unwrap();
            db.execute_batch("DELETE FROM task_head; DELETE FROM task_revision")
                .unwrap();
            crate::store::register_task(&mut db, &f.rows[0], "live", None).unwrap();
            let task = crate::validate_review(&f.rows[0]).unwrap();
            let reply: Value = serde_json::from_str(&f.rows[1]).unwrap();
            f.request=json!({"schema":"dot.admission-request/v1-proposal","request_id":"request","task":task,"decision":{"id":reply["message_id"],"revision":"2","sha256":crate::hash(&f.rows[1])},"expected_task_revision":"r1"}).to_string();
        }
        let task = crate::validate_review(&f.rows[0]).unwrap();
        let c = config(&f.config).unwrap();
        let mut profile: Value =
            serde_json::from_slice(&std::fs::read(&f.profile.path).unwrap()).unwrap();
        profile["task"] = serde_json::to_value(&task).unwrap();
        if inputs.get("session").is_some() {
            profile["session"] = inputs["session"].clone();
        }
        if inputs.get("wire_sha256").is_some() {
            profile["entry_request_sha256"] = inputs["wire_sha256"].clone();
        }
        if inputs.get("native_sha256").is_some() {
            profile["native_executable_sha256"] = inputs["native_sha256"].clone();
        }
        if inputs["two_stage"] == true {
            profile["schema"] = PROFILE_FINAL.into();
            profile["final_stage"] = json!({"attempt":"f".repeat(64),"requests":2,"effects":1,"initial_maximum":"0.05","initial_reserve":"0.05","maximum":"0.05","reserve":"0.05","initial_output_tokens":1024,"output_tokens":2048});
        }
        for key in ["target_generation", "bridge_store_id", "transport_sha256"] {
            if inputs.get(key).is_some() {
                profile[key] = inputs[key].clone();
            }
        }
        put(&f.profile.path, profile.to_string());
        f.profile = pin(f.profile.path.clone());
        // Simulated canonical protocol, not a duplicate production ledger.
        let script = f.dir.join("scope.py");
        let export = json!({"schema":"canonical-envelope-source/v1","path":c.sources.budget_record.path,"before_sha256":c.sources.budget_record.sha256,"after_sha256":c.sources.budget_record.sha256,"authority_sha256":"a".repeat(64)});
        let code = format!(
            "import sys,json\na=dict(zip(sys.argv[2::2],sys.argv[3::2]))\ns={}\nb={{k[2:].replace('-','_'):v for k,v in a.items()}}\nb['state']='effect_unknown' if a.get('--action')=='fence' else 'reserved'\nx={{'source':s,'binding':b,'envelope':b,'exposure':{{'objective_id':'fixture-objective'}},'readonly':True,'effect_state':'effect_unknown','authority_status':'not_established'}}\nprint(json.dumps(s if sys.argv[1]=='envelope-source' else x))\n",
            export
        );
        let ledger = f.dir.join("ledger.py");
        put(
            &ledger,
            format!(
                "import json,datetime\nfrom pathlib import Path\nd=json.loads(Path({:?}).read_text())\nd['snapshot']['snapshot_at']=(datetime.datetime.now(datetime.timezone.utc)).isoformat()\nprint(json.dumps(d))\n",
                f.dir.join("cost.json").to_str().unwrap()
            ),
        );
        put(&script, code);
        let mut cfg: Value =
            serde_json::from_slice(&std::fs::read(&f.config.path).unwrap()).unwrap();
        let ledger_pin = pin(ledger);
        cfg["sources"]["ledger"]["script"] = serde_json::to_value(&ledger_pin).unwrap();
        cfg["sources"]["canonical_budget"]["scope"]["dependencies"] =
            serde_json::to_value(vec![ledger_pin]).unwrap();
        cfg["sources"]["canonical_budget"]["scope"]["script"] =
            serde_json::to_value(pin(script)).unwrap();
        put(&f.config.path, cfg.to_string());
        f.config = pin(f.config.path.clone());
        let prepare = inputs["phase"] == "prepare";
        if prepare {
            f.save();
        } else {
            f.control("active");
        }
        put(&f.dir.join("admission-request.json"), &f.request);
        let ready = json!({"config":f.config,"profile":f.profile,"admission_request":pin(f.dir.join("admission-request.json"))});
        put(
            &f.dir.join(if prepare {
                "sources-ready.json"
            } else {
                "ready.json"
            }),
            ready.to_string(),
        );
        if let Some(packet) = inputs["packet_path"].as_str() {
            put(Path::new(packet), json!({"profile":f.profile}).to_string());
        }
        std::mem::forget(f); // explicit disposable export; caller owns cleanup
    }
    #[test]
    fn lost_canonical_ack_keeps_claim_effect_and_failure_publication() {
        for action in ["reserve", "fence"] {
            let mut f = Fixture::new();
            let mut c: Value =
                serde_json::from_slice(&std::fs::read(&f.config.path).unwrap()).unwrap();
            let record = pin(f.dir.join("record"));
            let export = json!({"schema":"canonical-envelope-source/v1","path":record.path,"before_sha256":record.sha256,"after_sha256":record.sha256,"authority_sha256":"a".repeat(64)});
            let script = f.dir.join("scope.py");
            put(
                &script,
                format!(
                    "import sys\nfrom pathlib import Path\nif sys.argv[1]=='envelope-source': print({:?})\nelse:\n Path({:?}).write_text('durable reservation or fence')\n print('lost ack')\n",
                    export.to_string(),
                    record.path.to_str().unwrap()
                ),
            );
            c["sources"]["canonical_budget"]["scope"]["script"] =
                serde_json::to_value(pin(script)).unwrap();
            put(&f.config.path, c.to_string());
            f.config = pin(f.config.path.clone());
            f.control("active");
            let mut owner = f.owner().unwrap();
            let mut source = owner.budget_source().unwrap();
            let old = source.current_sha256().to_owned();
            let session = owner.profile.session.clone();
            let turn = "00000000-0000-4000-8000-000000000002";
            let request = "b".repeat(64);
            owner
                .claim(
                    &session,
                    turn,
                    &"e".repeat(64),
                    &"a".repeat(64),
                    &request,
                    &source,
                    NOW,
                )
                .unwrap();
            owner
                .fence(&session, turn, &request, &source, NOW)
                .unwrap()
                .observed(&crate::hash("after"))
                .unwrap();
            let envelope = crate::canonical_source::Envelope {
                operation: "op",
                actor: "actor",
                task: "task",
                parent: "parent",
                parent_sha256: "pin",
                maximum: "0.1",
                reserve: "0.1",
                request: "request",
                proposal: "proposal",
                mode: "mode",
                profile: "profile",
                native: "native",
            };
            assert!(source.transition(action, &envelope).is_err());
            assert_eq!(source.current_sha256(), old);
            assert!(source.check_current().is_err());
            assert!(
                owner
                    .recheck(&session, turn, &request, &source, NOW)
                    .is_err()
            );
            let native = json!({"session":session,"turn":turn,"error":"original_final_funding_ack_unknown","terminal_observed":true,"events":[{"known":"event"}]});
            let receipts = vec![json!({"first":"known receipt"})];
            let closure = json!({"pid":123,"wait_status":"exit:0","owned_group_signalled":true});
            let result = owner
                .publish_observation(
                    &f.dir.join("published.json"),
                    &native,
                    &receipts,
                    &closure,
                    &source,
                    NOW,
                )
                .unwrap();
            assert_eq!(result["effect_state"], "effect_observed");
            let raw: String = owner
                .db
                .query_row("SELECT body FROM scoped_publication", [], |r| r.get(0))
                .unwrap();
            let body: Value = serde_json::from_str(&raw).unwrap();
            assert_eq!(body["native"]["error"], native["error"]);
            assert_eq!(body["native"]["events"], native["events"]);
            assert_eq!(body["canonical_receipts"], json!(receipts));
            assert_eq!(body["closure"], closure);
            assert_eq!(body["canonical_source_sha256"], old);
            assert_eq!(body["canonical_source_error"], "canonical_source_pin");
            let projection = std::fs::read_to_string(f.dir.join("published.json")).unwrap();
            assert!(projection.contains("canonical_source_uncertain"));
            assert_ne!(result["worker_outcome"], "completed");
        }
    }
    #[test]
    fn production_constructor_requires_distinct_signed_activation_and_current_key() {
        let mut f = Fixture::new();
        assert!(matches!(f.owner(), Err("scoped_activation_missing")));
        f.control("active");
        assert!(f.owner().is_ok());
        let saved = f.rows.clone();
        let mut forged: Value = serde_json::from_str(&f.rows[3]).unwrap();
        forged["data"]["signed_answer"]["signature"] = "forged".into();
        f.rows[3] = forged.to_string();
        f.save();
        assert!(matches!(f.owner(), Err("scoped_signature")));
        f.rows = saved;
        f.save();
        put(&f.dir.join("keys"), "{\"keys\":{}}");
        assert!(f.owner().is_err());
    }
    #[test]
    fn signed_but_changed_scope_actor_cost_prerequisite_or_revision_is_held() {
        for field in [
            "actor",
            "maximum",
            "reserve",
            "path",
            "task",
            "proposal",
            "configuration",
            "parent",
            "expires",
            "profile",
        ] {
            let mut f = Fixture::new();
            let mut p: Value =
                serde_json::from_slice(&std::fs::read(&f.profile.path).unwrap()).unwrap();
            match field {
                "actor" => p["actor"] = "foreign".into(),
                "maximum" => p["maximum"] = "100".into(),
                "reserve" => p["reserve"] = "100".into(),
                "path" => p["effect"]["path"] = "other.txt".into(),
                "task" => p["task"]["revision"] = "r2".into(),
                "proposal" => p["proposal"]["sha256"] = "0".repeat(64).into(),
                "configuration" => p["configuration"]["revision"] = "changed".into(),
                "parent" => p["parent"]["sha256"] = "0".repeat(64).into(),
                "expires" => p["expires_at"] = "2026-11-02T00:00:00Z".into(),
                "profile" => p["id"] = "legacy-global-profile".into(),
                _ => unreachable!(),
            }
            put(&f.profile.path, p.to_string());
            f.profile = pin(f.profile.path.clone());
            f.control("active");
            assert!(matches!(f.owner(), Err("scoped_task_binding")), "{field}");
        }
    }
    fn head_transition_in_fence_lease_gap(proposal: bool, state: &'static str) {
        let mut f = Fixture::new();
        f.control("active");
        let mut owner = f.owner().unwrap();
        let source = owner.budget_source().unwrap();
        let session = owner.profile.session.clone();
        let turn = "00000000-0000-4000-8000-000000000002";
        let request = "b".repeat(64);
        owner
            .claim(
                &session,
                turn,
                &"e".repeat(64),
                &"a".repeat(64),
                &request,
                &source,
                NOW,
            )
            .unwrap();
        let operation: String = owner
            .db
            .query_row("SELECT operation_id FROM scoped_claim", [], |r| r.get(0))
            .unwrap();
        let raw = f.rows[0].clone();
        let row: Row = serde_json::from_str(&raw).unwrap();
        let reference = row.data.intent.proposal;
        let path = f.dir.join("registry");
        let target = f.dir.join("edit.txt");
        let record_before = std::fs::read(f.dir.join("record")).unwrap();
        let (fenced, at_gap) = std::sync::mpsc::channel();
        let (committed, resumed) = std::sync::mpsc::channel();
        let op = operation.clone();
        let writer = std::thread::spawn(move || {
            let mut db = Connection::open(path).unwrap();
            at_gap.recv().unwrap();
            // A distinct connection observes the committed fence before changing
            // the authority heads through their production transition APIs.
            let exposure: (String, String) = db.query_row(
                "SELECT o.state,c.state FROM outbox o JOIN capacity_claim c USING(operation_id) WHERE operation_id=?1",
                [&op], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
            assert_eq!(exposure, ("unknown".into(), "unknown".into()));
            let generation = if state == "replacement" {
                if proposal {
                    let mut next = reference;
                    next.revision = "r2".into();
                    next.sha256 = crate::hash("replacement proposal");
                    crate::store::register_proposal(&mut db, &next, "live", Some(1))
                } else {
                    let mut next: Value = serde_json::from_str(&raw).unwrap();
                    next["data"]["intent"]["revision"] = "r2".into();
                    crate::store::register_task(&mut db, &next.to_string(), "live", Some(1))
                }
            } else if proposal {
                crate::store::register_proposal(&mut db, &reference, state, Some(1))
            } else {
                crate::store::register_task(&mut db, &raw, state, Some(1))
            }
            .unwrap();
            assert_eq!(generation, 2);
            committed.send(()).unwrap();
        });
        owner.before_effect_lease = Some(Box::new(move || {
            fenced.send(()).unwrap();
            resumed.recv().unwrap();
        }));
        let rejected = match owner.fence(&session, turn, &request, &source, NOW) {
            Ok(lease) => {
                // Mirror the bridge's mutation boundary: no write before lease.
                std::fs::write(&target, &lease.effect().after).unwrap();
                lease.observed(&crate::hash("after")).unwrap();
                false
            }
            Err(error) => {
                assert_eq!(error, "scoped_effect_unknown_no_retry");
                true
            }
        };
        writer.join().unwrap();
        assert!(
            rejected,
            "{proposal}/{state}: stale admission granted a lease"
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"before");
        assert_eq!(std::fs::read(f.dir.join("record")).unwrap(), record_before);
        let retained: (String, String, String, i64) = owner.db.query_row(
            "SELECT o.state,c.state,s.state,(SELECT count(*) FROM outbox) FROM outbox o JOIN capacity_claim c USING(operation_id) JOIN scoped_claim s USING(operation_id) WHERE operation_id=?1",
            [&operation], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        assert_eq!(
            retained,
            (
                "unknown".into(),
                "unknown".into(),
                "effect_unknown".into(),
                1
            )
        );
        assert!(owner.fence(&session, turn, &request, &source, NOW).is_err());
        assert_eq!(
            crate::store::fence_delivery(&owner.db, &operation),
            Err(crate::store::Error::Held)
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"before");
    }
    #[test]
    fn task_withdrawal_in_fence_lease_gap() {
        head_transition_in_fence_lease_gap(false, "withdrawn");
    }
    #[test]
    fn task_supersession_in_fence_lease_gap() {
        head_transition_in_fence_lease_gap(false, "superseded");
    }
    #[test]
    fn proposal_withdrawal_in_fence_lease_gap() {
        head_transition_in_fence_lease_gap(true, "withdrawn");
    }
    #[test]
    fn proposal_supersession_in_fence_lease_gap() {
        head_transition_in_fence_lease_gap(true, "superseded");
    }
    #[test]
    fn task_replacement_in_fence_lease_gap() {
        head_transition_in_fence_lease_gap(false, "replacement");
    }
    #[test]
    fn proposal_replacement_in_fence_lease_gap() {
        head_transition_in_fence_lease_gap(true, "replacement");
    }
    #[test]
    fn concurrent_revocation_orders_after_inflight_lease_and_blocks_next_effect() {
        let mut f = Fixture::new();
        f.control("active");
        let mut owner = f.owner().unwrap();
        let source = owner.budget_source().unwrap();
        let session = owner.profile.session.clone();
        let turn = "00000000-0000-4000-8000-000000000002";
        let request = "b".repeat(64);
        owner
            .claim(
                &session,
                turn,
                &"e".repeat(64),
                &"a".repeat(64),
                &request,
                &source,
                NOW,
            )
            .unwrap();
        let lease = owner.fence(&session, turn, &request, &source, NOW).unwrap();
        f.control("revoked");
        let config = f.config.clone();
        let profile = f.profile.clone();
        let (started, ready) = std::sync::mpsc::channel();
        let (done, finished) = std::sync::mpsc::channel();
        let producer = std::thread::spawn(move || {
            started.send(()).unwrap();
            let result = ingest_control(config, profile, NOW);
            done.send(result).unwrap();
        });
        ready.recv().unwrap();
        assert!(
            finished
                .recv_timeout(std::time::Duration::from_millis(100))
                .is_err()
        );
        lease.observed(&crate::hash("after")).unwrap();
        let revoked = finished
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert_eq!(revoked["state"], "revoked");
        producer.join().unwrap();
        assert!(matches!(
            owner.recheck(&session, turn, &request, &source, NOW),
            Err("scoped_revoked")
        ));
        assert!(owner.fence(&session, turn, &request, &source, NOW).is_err());
    }
    #[test]
    fn captured_native_final_uses_existing_packet_contract_and_normal_status() {
        let Some(root) = std::env::var_os("SCOPED_CAPTURE_ROOT") else {
            return;
        };
        let raw = std::fs::read(PathBuf::from(root).join("observation.json")).unwrap();
        let doc: Value = serde_json::from_slice(&raw).unwrap();
        let observed = &doc["observation"]["Ok"];
        let session = observed["session"].as_str().unwrap();
        let turn = observed["turn"].as_str().unwrap();
        let mut events = observed["events"].as_array().unwrap().clone();
        let text = selected_final_text(&events, session, turn).unwrap();
        let task = Ref {
            id: "fixture-task".into(),
            revision: "1".into(),
            sha256: "b".repeat(64),
        };
        let packet = crate::result::packet(text, &task, "fixture-admission").unwrap();
        assert_eq!(packet.outcome, "completed");
        assert!(crate::result::packet(text, &task, "foreign-admission").is_none());
        let terminal = events
            .iter_mut()
            .find(|e| e["method"] == "turn/completed")
            .unwrap();
        terminal["params"]["turn"]["status"] = "interrupted".into();
        assert!(selected_final_text(&events, session, turn).is_none());
        events.retain(|e| e["method"] != "turn/completed");
        assert!(selected_final_text(&events, session, turn).is_none());
    }
    #[test]
    fn two_stage_signed_allowance_preserves_first_claim_and_fences_restart() {
        let mut f = Fixture::new();
        let mut profile: Value =
            serde_json::from_slice(&std::fs::read(&f.profile.path).unwrap()).unwrap();
        profile["schema"] = PROFILE_FINAL.into();
        profile["final_stage"] = json!({"attempt":"c".repeat(64),"requests":2,"effects":1,"initial_maximum":"0.05","initial_reserve":"0.05","maximum":"0.05","reserve":"0.05","initial_output_tokens":1024,"output_tokens":2048});
        put(&f.profile.path, profile.to_string());
        f.profile = pin(f.profile.path.clone());
        f.control("active");
        let mut owner = f.owner().unwrap();
        let source = owner.budget_source().unwrap();
        let session = owner.profile.session.clone();
        let turn = "00000000-0000-4000-8000-000000000002";
        let first_attempt = "a".repeat(64);
        let first_request = "b".repeat(64);
        let attempt = "c".repeat(64);
        let request = "d".repeat(64);
        owner
            .claim(
                &session,
                turn,
                &"e".repeat(64),
                &first_attempt,
                &first_request,
                &source,
                NOW,
            )
            .unwrap();
        let operation = owner.operation_id().unwrap();
        let output = json!({"state":"effect_observed","operation_id":operation,"session":session,"turn":turn,"attempt":first_attempt,"request_sha256":first_request,"after_sha256":crate::hash("after")});
        let ack = json!({"method":"item/completed","params":{"threadId":session,"turnId":turn,"item":{"type":"dynamicToolCall","tool":"exact_disposable_edit","id":"call_fixture","status":"completed","success":true,"arguments":{"operation_id":operation},"contentItems":[{"type":"inputText","text":output.to_string()}]}}});
        assert_eq!(
            owner.admit_final(
                &session,
                turn,
                &first_attempt,
                &first_request,
                &attempt,
                &request,
                &ack,
                &source,
                NOW
            ),
            Err("scoped_final_effect_unknown")
        );
        owner
            .fence(&session, turn, &first_request, &source, NOW)
            .unwrap()
            .observed(&crate::hash("after"))
            .unwrap();
        let mut bad = ack.clone();
        bad["params"]["item"]["success"] = Value::Null;
        assert_eq!(
            owner.admit_final(
                &session,
                turn,
                &first_attempt,
                &first_request,
                &attempt,
                &request,
                &bad,
                &source,
                NOW
            ),
            Err("scoped_final_callback")
        );
        owner
            .admit_final(
                &session,
                turn,
                &first_attempt,
                &first_request,
                &attempt,
                &request,
                &ack,
                &source,
                NOW,
            )
            .unwrap();
        assert_eq!(
            owner.admit_final(
                &session,
                turn,
                &first_attempt,
                &first_request,
                &attempt,
                &request,
                &ack,
                &source,
                NOW
            ),
            Err("scoped_final_already_claimed")
        );
        drop(owner);
        let mut owner = f.owner().unwrap();
        owner
            .recheck_final(&session, turn, &attempt, &request, &source, NOW)
            .unwrap();
        assert_eq!(
            owner.admit_final(
                &session,
                turn,
                &first_attempt,
                &first_request,
                &attempt,
                &request,
                &ack,
                &source,
                NOW
            ),
            Err("scoped_final_already_claimed")
        );
        owner
            .claim(
                &session,
                turn,
                &"e".repeat(64),
                &first_attempt,
                &first_request,
                &source,
                NOW,
            )
            .unwrap();
        assert_eq!(
            owner.claim(
                &session,
                turn,
                &"e".repeat(64),
                &attempt,
                &request,
                &source,
                NOW
            ),
            Err("scoped_turn_already_claimed")
        );
        f.control("revoked");
        ingest_control(f.config.clone(), f.profile.clone(), NOW).unwrap();
        assert_eq!(
            owner.recheck_final(&session, turn, &attempt, &request, &source, NOW),
            Err("scoped_revoked")
        );
    }
    #[test]
    fn old_profile_cannot_gain_final_stage_allowance() {
        let mut f = Fixture::new();
        f.control("active");
        let mut owner = f.owner().unwrap();
        let source = owner.budget_source().unwrap();
        assert_eq!(
            owner.admit_final(
                "wrong",
                "wrong",
                &"a".repeat(64),
                &"b".repeat(64),
                &"c".repeat(64),
                &"d".repeat(64),
                &json!({}),
                &source,
                NOW
            ),
            Err("scoped_final_not_admitted")
        );
        let mut profile: Value =
            serde_json::from_slice(&std::fs::read(&f.profile.path).unwrap()).unwrap();
        profile["final_stage"] = json!({"attempt":"c".repeat(64),"requests":2,"effects":1,"initial_maximum":"0.05","initial_reserve":"0.05","maximum":"0.05","reserve":"0.05","initial_output_tokens":1024,"output_tokens":2048});
        put(&f.profile.path, profile.to_string());
        f.profile = pin(f.profile.path.clone());
        assert!(f.owner().is_err());
    }
    #[test]
    fn exact_claim_revocation_expiry_and_restart_never_replace_a_turn() {
        let mut f = Fixture::new();
        f.control("active");
        let mut owner = f.owner().unwrap();
        let source = owner.budget_source().unwrap();
        let session = owner.profile.session.clone();
        let turn = "00000000-0000-4000-8000-000000000002";
        let entry = "e".repeat(64);
        let attempt = "a".repeat(64);
        let request = "b".repeat(64);
        assert!(
            owner
                .claim("wrong", turn, &entry, &attempt, &request, &source, NOW)
                .is_err()
        );
        owner
            .claim(&session, turn, &entry, &attempt, &request, &source, NOW)
            .unwrap();
        drop(owner);
        let mut owner = f.owner().unwrap();
        owner
            .claim(&session, turn, &entry, &attempt, &request, &source, NOW)
            .unwrap();
        assert!(
            owner
                .claim(
                    &session,
                    "00000000-0000-4000-8000-000000000003",
                    &entry,
                    &attempt,
                    &request,
                    &source,
                    NOW
                )
                .is_err()
        );
        {
            let _lease = owner.fence(&session, turn, &request, &source, NOW).unwrap();
        } // lost effect ACK: persisted unknown
        assert!(matches!(
            owner.fence(&session, turn, &request, &source, NOW),
            Err("scoped_effect_unknown_no_retry")
        ));
        assert!(
            owner
                .recheck(
                    &session,
                    turn,
                    &request,
                    &source,
                    NOW + time::Duration::minutes(3)
                )
                .is_err()
        );
        f.control("revoked");
        ingest_control(f.config.clone(), f.profile.clone(), NOW).unwrap();
        assert!(matches!(
            owner.recheck(&session, turn, &request, &source, NOW),
            Err("scoped_revoked")
        ));
        drop(owner);
        assert!(f.owner().is_err());
    }
}
