//! Synthetic-only answer consumption. The canonical board is the only decision
//! journal. This module cannot construct admission, outbox or native permits.
pub use crate::source::{Pin, Program};
use crate::{Ref, Row, hash, proof, source, store};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Digest;
use std::{path::PathBuf, time::Duration};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Unavailable,
    Invalid,
    Conflict,
    Held,
    Unknown,
}
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self::Unavailable
    }
}

/// Backend construction only: never deserialize this from consume/project input.
/// The clock is one synthetic epoch advanced by monotonic elapsed time.
#[derive(Clone)]
pub struct SyntheticConfig {
    pub database: PathBuf,
    pub repository: PathBuf,
    pub board: Program,
    pub policy: Pin,
    pub credential_id: String,
    pub key_fingerprint: String,
    pub owner: String,
    pub actor: String,
    pub session_id: String,
    pub rp_id: String,
    pub origin: String,
    pub epoch: OffsetDateTime,
    /// Existing scoped-control owner configuration/profile, when applicable.
    pub scoped_control: Option<(Pin, Pin)>,
}

/// No production/live constructor exists. This instance uses immutable synthetic
/// key policy, not a live passkey identity or mutable key revocation service.
pub struct SyntheticOwner {
    config: SyntheticConfig,
    db: Connection,
    key: crate::key_policy::PinnedKey,
    clock: source::AcquisitionTime,
    policy_expires: OffsetDateTime,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub task: Ref,
    pub proposal: Ref,
    pub task_generation: i64,
    pub proposal_generation: i64,
    pub request_sequence: u64,
    pub board_revision: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    Current,
    Held,
    Replaced,
    Revoked,
    Expired,
    Unavailable,
}
#[derive(Debug, Serialize)]
pub struct Projection {
    pub schema: &'static str,
    pub synthetic: bool,
    pub selection: Selection,
    pub request_json: String,
    pub request_sha256: String,
    pub source_sha256: String,
    pub policy_sha256: String,
    pub standing: Standing,
    pub execution_admission: Option<()>,
}
/// All five signed strings retain their original bytes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SignedAnswer {
    pub answer_json: String,
    pub credential_id: String,
    pub authenticator_data: String,
    pub client_data_json: String,
    pub signature: String,
}
impl SignedAnswer {
    fn assertion(&self) -> proof::Assertion<'_> {
        proof::Assertion {
            credential_id: &self.credential_id,
            authenticator_data: &self.authenticator_data,
            client_data_json: &self.client_data_json,
            signature: &self.signature,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consume {
    pub selection: Selection,
    pub signed: SignedAnswer,
}
#[derive(Debug, Serialize)]
pub struct Decision {
    pub decision: Ref,
    pub selection: Selection,
    pub request_json: String,
    pub signed: SignedAnswer,
    pub board_revision: String,
    pub current_standing: Standing,
    /// History does not re-grant current authority.
    pub historical: bool,
    pub execution_admission: Option<()>,
}
struct Snapshot {
    revision: String,
    rows: Vec<String>,
}

fn utc(s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|_| Error::Invalid)
}
fn parsed(raw: &str) -> Result<Value> {
    serde_json::from_str(raw).map_err(|_| Error::Invalid)
}
fn monotonic() -> f64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // Same OS CLOCK_MONOTONIC domain used by Python time.monotonic().
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) } != 0 {
        return f64::NEG_INFINITY;
    }
    ts.tv_sec as f64 + ts.tv_nsec as f64 / 1e9
}
fn compact(v: &Value) -> Result<String> {
    serde_json::to_string(v).map_err(|_| Error::Invalid)
}

impl SyntheticOwner {
    pub fn open_synthetic(config: SyntheticConfig) -> Result<Self> {
        // Pin the actual owner implementation and its imported semantics, not an
        // arbitrary helper selected by a consume caller.
        let script_hash = format!(
            "{:x}",
            sha2::Sha256::digest(include_bytes!(
                "../../../../skills/durable-state/scripts/codex_chatboard.py"
            ))
        );
        let script_dir = config.board.script.path.parent().ok_or(Error::Invalid)?;
        let dependencies = [
            (
                script_dir.join("durable_state.py"),
                include_bytes!("../../../../skills/durable-state/scripts/durable_state.py")
                    .as_slice(),
            ),
            (
                script_dir.join("../contracts/codex-chatboard-kinds.v1.json"),
                include_bytes!(
                    "../../../../skills/durable-state/contracts/codex-chatboard-kinds.v1.json"
                )
                .as_slice(),
            ),
        ];
        if config.board.script.sha256 != script_hash
            || !source::pinned(&config.board.script)
            || dependencies.iter().any(|(p, b)| {
                !config.board.dependencies.iter().any(|pin| {
                    pin.path == *p
                        && pin.sha256 == format!("{:x}", sha2::Sha256::digest(b))
                        && source::pinned(pin)
                })
            })
            || !source::pinned(&config.policy)
            || !config.database.is_absolute()
            || !config.repository.is_absolute()
            || config.owner.is_empty()
            || config.actor.is_empty()
            || uuid::Uuid::parse_str(&config.session_id).is_err()
        {
            return Err(Error::Invalid);
        }
        let key = crate::key_policy::load(&config.policy.path, &config.credential_id)
            .map_err(|_| Error::Invalid)?;
        if !source::pinned(&config.policy) {
            return Err(Error::Invalid);
        }
        let policy_doc: Value = serde_json::from_slice(
            &std::fs::read(&config.policy.path).map_err(|_| Error::Invalid)?,
        )
        .map_err(|_| Error::Invalid)?;
        let policy_expires = utc(policy_doc["expires_at"].as_str().ok_or(Error::Invalid)?)?;
        if policy_doc["synthetic"] != true
            || policy_doc["owner"] != config.owner
            || policy_doc["rp_id"] != config.rp_id
            || policy_doc["origin"] != config.origin
            || key.fingerprint != config.key_fingerprint
            || config.epoch >= policy_expires
            || !source::pinned(&config.policy)
        {
            return Err(Error::Invalid);
        }
        let db = Connection::open(&config.database)?;
        db.busy_timeout(Duration::from_secs(30))?;
        db.execute_batch("PRAGMA synchronous=FULL")?;
        store::initialize(&db).map_err(|_| Error::Unavailable)?;
        let clock = source::AcquisitionTime::new(config.epoch);
        Ok(Self {
            config,
            db,
            key,
            clock,
            policy_expires,
        })
    }
    /// Synthetic producer helper using the existing registry owner semantics.
    pub fn register_proposal_fixture(
        &mut self,
        r: &Ref,
        state: &str,
        expected: Option<i64>,
    ) -> Result<i64> {
        store::register_proposal(&mut self.db, r, state, expected).map_err(map_store)
    }
    /// Exact board codec bytes must be registered; snapshots never register heads.
    pub fn register_task_fixture(
        &mut self,
        raw: &str,
        state: &str,
        expected: Option<i64>,
    ) -> Result<i64> {
        store::register_task(&mut self.db, raw, state, expected).map_err(map_store)
    }
    fn call(config: &SyntheticConfig, command: Value) -> Result<Value> {
        let cmd = compact(&command)?;
        if cmd.len() > 100_000 {
            return Err(Error::Invalid);
        }
        let bytes = source::run_bounded(
            &config.board,
            &[
                "--repository".as_ref(),
                config.repository.as_os_str(),
                "owner-boundary-io".as_ref(),
                "--command-json".as_ref(),
                cmd.as_ref(),
            ],
            Duration::from_secs(10),
        )
        .ok_or(Error::Unavailable)?;
        serde_json::from_slice(&bytes).map_err(|_| Error::Unavailable)
    }
    fn snapshot(config: &SyntheticConfig) -> Result<Snapshot> {
        let v = Self::call(config, json!({"operation":"snapshot"}))?;
        let revision = v["revision"].as_str().ok_or(Error::Unavailable)?.to_owned();
        let rows = v["rows"]
            .as_array()
            .ok_or(Error::Unavailable)?
            .iter()
            .map(|v| v.as_str().map(str::to_owned).ok_or(Error::Unavailable))
            .collect::<Result<Vec<_>>>()?;
        if v["next_sequence"].as_u64() != Some(rows.len() as u64 + 1) {
            return Err(Error::Unavailable);
        }
        // Validate complete sequence coverage. Board owner exports the full rows.
        for (i, raw) in rows.iter().enumerate() {
            if parsed(raw)?["sequence"].as_u64() != Some(i as u64 + 1) {
                return Err(Error::Unavailable);
            }
        }
        Ok(Snapshot { revision, rows })
    }
    fn raw_task(db: &Connection, task: &Ref) -> Result<String> {
        let raw: Option<String> = db.query_row("SELECT request_json FROM task_revision WHERE task_id=?1 AND revision=?2 AND request_sha=?3",
            params![task.id,task.revision,task.sha256],|r|r.get(0)).optional()?;
        let raw = raw.ok_or(Error::Held)?;
        if crate::validate_review(&raw).map_err(|_| Error::Invalid)? != *task {
            return Err(Error::Conflict);
        }
        Ok(raw)
    }
    fn registry(db: &Connection, s: &Selection, raw: &str, actor: &str) -> Result<Standing> {
        let row: Row = serde_json::from_str(raw).map_err(|_| Error::Invalid)?;
        if row.data.intent.submitter != actor || row.data.intent.proposal != s.proposal {
            return Err(Error::Conflict);
        }
        let head: Option<(String, String, String, i64)> = db
            .query_row(
                "SELECT revision,sha256,state,generation FROM task_head WHERE id=?1",
                [&s.task.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let prop: Option<(String, String, String, i64)> = db
            .query_row(
                "SELECT revision,sha256,state,generation FROM proposal_head WHERE id=?1",
                [&s.proposal.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let (Some(t), Some(p)) = (head, prop) else {
            return Ok(Standing::Unavailable);
        };
        if t.2 == "withdrawn" || p.2 == "withdrawn" {
            return Ok(Standing::Revoked);
        }
        if t.0 != s.task.revision
            || t.1 != s.task.sha256
            || t.3 != s.task_generation
            || p.0 != s.proposal.revision
            || p.1 != s.proposal.sha256
            || p.3 != s.proposal_generation
            || t.2 != "live"
            || p.2 != "live"
        {
            return Ok(Standing::Replaced);
        }
        Ok(Standing::Current)
    }
    fn board_standing(snapshot: &Snapshot, s: &Selection, raw: &str) -> Result<Standing> {
        let mut found = false;
        let mut closed = false;
        for r in &snapshot.rows {
            let v = parsed(r)?;
            if v["sequence"] == s.request_sequence {
                if v["kind"] != "request" || r != raw {
                    return Err(Error::Conflict);
                }
                found = true;
            }
            if v["supersedes"] == s.request_sequence {
                return Ok(Standing::Replaced);
            }
            if v["kind"] == "reply"
                && v["in_reply_to"] == s.request_sequence
                && matches!(
                    v.pointer("/data/disposition").and_then(Value::as_str),
                    Some("answered" | "declined")
                )
            {
                closed = true;
            }
        }
        Ok(if closed {
            Standing::Held
        } else if found {
            Standing::Current
        } else {
            Standing::Unavailable
        })
    }
    fn standing(
        db: &Connection,
        snapshot: &Snapshot,
        s: &Selection,
        raw: &str,
        config: &SyntheticConfig,
        now: OffsetDateTime,
    ) -> Result<Standing> {
        let current = Self::registry(db, s, raw, &config.actor)?;
        if current != Standing::Current {
            return Ok(current);
        }
        let row: Row = serde_json::from_str(raw).map_err(|_| Error::Invalid)?;
        if now >= utc(&row.data.intent.clock.admit_until)? {
            return Ok(Standing::Expired);
        }
        Self::board_standing(snapshot, s, raw)
    }
    fn control_deadline(
        db: &Connection,
        snapshot: &Snapshot,
        selection: &Selection,
        config: &SyntheticConfig,
        now: OffsetDateTime,
    ) -> Result<OffsetDateTime> {
        if let Some((config_pin, profile_pin)) = &config.scoped_control {
            let c = crate::scoped_pilot::config(config_pin).map_err(|_| Error::Held)?;
            if c.sources.board_repository != config.repository
                || c.database != config.database.to_string_lossy()
                || !source::pinned(profile_pin)
                || !source::pinned(&config.policy)
                || c.sources.key_policy != config.policy.path
            {
                return Err(Error::Held);
            }
            let profile: crate::scoped_pilot::Profile = serde_json::from_slice(
                &std::fs::read(&profile_pin.path).map_err(|_| Error::Unavailable)?,
            )
            .map_err(|_| Error::Invalid)?;
            if profile.task != selection.task
                || profile.proposal != selection.proposal
                || profile.actor != config.actor
            {
                return Err(Error::Held);
            }
            let registered:Option<String>=db.query_row("SELECT state FROM scoped_control WHERE profile_sha=?1 ORDER BY generation DESC LIMIT 1",[&profile_pin.sha256],|r|r.get(0)).optional()?;
            if registered.as_deref() == Some("revoked") {
                return Err(Error::Held);
            }
            let head =
                crate::scoped_pilot::head(&c, profile_pin, now).map_err(|_| Error::Unavailable)?;
            crate::scoped_pilot::synchronize(db, profile_pin, &head).map_err(|_| Error::Held)?;
            if head.state != "active" {
                return Err(Error::Held);
            }
            let deadline = head.valid_until.min(utc(&profile.expires_at)?);
            if Self::snapshot(config)?.revision != snapshot.revision {
                return Err(Error::Conflict);
            }
            return Ok(deadline);
        }
        let raw = Self::raw_task(db, &selection.task)?;
        let row: Row = serde_json::from_str(&raw).map_err(|_| Error::Invalid)?;
        if row.data.intent.execution_profile != "disposable-codex-write-v1" {
            return Err(Error::Invalid);
        }
        Ok(time::macros::datetime!(9999-12-31 23:59:59 UTC))
    }
    /// Read actual registered current heads and the complete canonical board.
    pub fn project(&mut self, task_id: &str) -> Result<Projection> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let s=tx.query_row("SELECT t.revision,t.sha256,t.proposal_id,t.proposal_revision,t.proposal_sha,t.generation,p.generation FROM task_head t JOIN proposal_head p ON p.id=t.proposal_id WHERE t.id=?1",[task_id],|r|Ok(Selection{task:Ref{id:task_id.into(),revision:r.get(0)?,sha256:r.get(1)?},proposal:Ref{id:r.get(2)?,revision:r.get(3)?,sha256:r.get(4)?},task_generation:r.get(5)?,proposal_generation:r.get(6)?,request_sequence:0,board_revision:String::new()}))?;
        let raw = Self::raw_task(&tx, &s.task)?;
        let v = parsed(&raw)?;
        let snapshot = Self::snapshot(&self.config)?;
        let s = Selection {
            request_sequence: v["sequence"].as_u64().ok_or(Error::Invalid)?,
            board_revision: snapshot.revision.clone(),
            ..s
        };
        let mut standing =
            Self::standing(&tx, &snapshot, &s, &raw, &self.config, self.clock.now())?;
        if self.clock.now() >= self.policy_expires {
            standing = Standing::Expired;
        }
        if standing == Standing::Current {
            match Self::control_deadline(&tx, &snapshot, &s, &self.config, self.clock.now()) {
                Ok(d) if self.clock.now() < d => {}
                Ok(_) => standing = Standing::Expired,
                Err(Error::Held) => standing = Standing::Revoked,
                Err(Error::Invalid) => standing = Standing::Held,
                Err(_) => standing = Standing::Unavailable,
            };
        }
        let row: Row = serde_json::from_str(&raw).map_err(|_| Error::Invalid)?;
        if self.clock.now() >= self.policy_expires
            || self.clock.now() >= utc(&row.data.intent.clock.admit_until)?
        {
            standing = Standing::Expired;
        }
        Ok(Projection {
            schema: "dot.owner-task/v1-synthetic",
            synthetic: true,
            request_sha256: hash(&raw),
            request_json: raw,
            selection: s,
            source_sha256: self.config.board.script.sha256.clone(),
            policy_sha256: self.config.policy.sha256.clone(),
            standing,
            execution_admission: None,
        })
    }
    fn exact(
        snapshot: &Snapshot,
        request: &Consume,
        raw: &str,
        config: &SyntheticConfig,
        key: &crate::key_policy::PinnedKey,
    ) -> Result<Option<Decision>> {
        let answer = parsed(&request.signed.answer_json)?;
        let id = answer["answer_id"].as_str().ok_or(Error::Invalid)?;
        for r in &snapshot.rows {
            let row = parsed(r)?;
            if row["message_id"] != id {
                continue;
            }
            let data = &row["data"];
            let signed = &data["signed_answer"];
            if row["kind"] != "reply"
                || row["in_reply_to"] != request.selection.request_sequence
                || signed["format"] != "work-engine.owner-answer/v1"
                || signed["answer_json"] != request.signed.answer_json
                || signed["credential_id"] != request.signed.credential_id
                || signed["authenticator_data"] != request.signed.authenticator_data
                || signed["client_data_json"] != request.signed.client_data_json
                || signed["signature"] != request.signed.signature
                || data["owner_boundary"]["task"] != json!(request.selection.task)
                || data["owner_boundary"]["proposal"] != json!(request.selection.proposal)
                || data["owner_boundary"]["task_generation"] != request.selection.task_generation
                || data["owner_boundary"]["proposal_generation"]
                    != request.selection.proposal_generation
                || data["owner_boundary"]["request_sha256"] != hash(raw)
                || answer["request_digest"] != hash(raw)
            {
                return Err(Error::Conflict);
            }
            let policy = proof::KeyPolicy {
                owner: &config.owner,
                rp_id: &config.rp_id,
                origin: &config.origin,
                credential_id: &config.credential_id,
                spki_der: &key.spki_der,
                fingerprint: &key.fingerprint,
            };
            let verified = proof::verify_answer(
                raw,
                &request.signed.answer_json,
                &request.signed.assertion(),
                &policy,
                utc(answer["created_at"].as_str().ok_or(Error::Invalid)?)?,
            )
            .map_err(|_| Error::Conflict)?;
            let disposition = match verified.decision.as_str() {
                "approve" => "answered",
                "reject" => "declined",
                "defer" => "deferred",
                "ask" => "partial",
                _ => return Err(Error::Conflict),
            };
            let original = parsed(raw)?;
            if data["decision"] != verified.decision
                || data["comment"] != verified.comment
                || data["answer_id"] != id
                || data["disposition"] != disposition
                || row["topic"] != original["topic"]
                || row["to"]
                    != json!([format!(
                        "session:{}",
                        original["session_id"].as_str().ok_or(Error::Invalid)?
                    )])
                || data["owner_boundary"]["policy_sha256"] != config.policy.sha256
                || data["owner_boundary"]["source_sha256"] != config.board.script.sha256
            {
                return Err(Error::Conflict);
            }
            return Ok(Some(Decision {
                decision: Ref {
                    id: id.into(),
                    revision: row["sequence"].as_u64().ok_or(Error::Invalid)?.to_string(),
                    sha256: hash(r),
                },
                selection: request.selection.clone(),
                request_json: raw.into(),
                signed: request.signed.clone(),
                board_revision: snapshot.revision.clone(),
                current_standing: Standing::Held,
                historical: true,
                execution_admission: None,
            }));
        }
        Ok(None)
    }
    /// Immutable exact reconciliation. Absence is unknown, never no-effect proof.
    pub fn read_exact(&mut self, request: &Consume) -> Result<Decision> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let raw = Self::raw_task(&tx, &request.selection.task)?;
        let snap = Self::snapshot(&self.config)?;
        let mut d =
            Self::exact(&snap, request, &raw, &self.config, &self.key)?.ok_or(Error::Unknown)?;
        d.current_standing = Self::historical_standing(
            &tx,
            &snap,
            request,
            &raw,
            &self.config,
            self.clock.now(),
            self.policy_expires,
        )?;
        Ok(d)
    }
    fn historical_standing(
        db: &Connection,
        snapshot: &Snapshot,
        request: &Consume,
        raw: &str,
        config: &SyntheticConfig,
        now: OffsetDateTime,
        policy_expires: OffsetDateTime,
    ) -> Result<Standing> {
        let clock = source::AcquisitionTime::new(now);
        let mut standing = Self::standing(db, snapshot, &request.selection, raw, config, now)?;
        if standing == Standing::Revoked || standing == Standing::Replaced {
            return Ok(standing);
        }
        let answer = parsed(&request.signed.answer_json)?;
        if now >= policy_expires
            || now >= utc(answer["expires_at"].as_str().ok_or(Error::Invalid)?)?
        {
            return Ok(Standing::Expired);
        }
        match Self::control_deadline(db, snapshot, &request.selection, config, now) {
            Ok(d) if clock.now() < d => {}
            Ok(_) => standing = Standing::Expired,
            Err(Error::Held) => standing = Standing::Revoked,
            Err(Error::Invalid) => standing = Standing::Held,
            Err(_) => standing = Standing::Unavailable,
        };
        let row: Row = serde_json::from_str(raw).map_err(|_| Error::Invalid)?;
        if clock.now() >= utc(&row.data.intent.clock.admit_until)?
            || clock.now() >= policy_expires
            || clock.now() >= utc(answer["expires_at"].as_str().ok_or(Error::Invalid)?)?
        {
            standing = Standing::Expired;
        }
        Ok(standing)
    }
    pub fn consume(&mut self, request: &Consume) -> Result<Decision> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let raw = Self::raw_task(&tx, &request.selection.task)?;
        let snapshot = Self::snapshot(&self.config)?;
        if let Some(mut d) = Self::exact(&snapshot, request, &raw, &self.config, &self.key)? {
            d.current_standing = Self::historical_standing(
                &tx,
                &snapshot,
                request,
                &raw,
                &self.config,
                self.clock.now(),
                self.policy_expires,
            )?;
            return Ok(d);
        }
        if snapshot.revision != request.selection.board_revision {
            return Err(Error::Conflict);
        }
        if Self::standing(
            &tx,
            &snapshot,
            &request.selection,
            &raw,
            &self.config,
            self.clock.now(),
        )? != Standing::Current
        {
            return Err(Error::Held);
        }
        let row: Row = serde_json::from_str(&raw).map_err(|_| Error::Invalid)?;
        let mut deadline = utc(&row.data.intent.clock.admit_until)?.min(self.policy_expires);
        deadline = deadline.min(Self::control_deadline(
            &tx,
            &snapshot,
            &request.selection,
            &self.config,
            self.clock.now(),
        )?);
        let policy = proof::KeyPolicy {
            owner: &self.config.owner,
            rp_id: &self.config.rp_id,
            origin: &self.config.origin,
            credential_id: &self.config.credential_id,
            spki_der: &self.key.spki_der,
            fingerprint: &self.key.fingerprint,
        };
        let verified = proof::verify_answer(
            &raw,
            &request.signed.answer_json,
            &request.signed.assertion(),
            &policy,
            self.clock.now(),
        )
        .map_err(|_| Error::Invalid)?;
        deadline = deadline.min(verified.expires_at);
        let remaining = (deadline - self.clock.now()).as_seconds_f64().min(9.0);
        if remaining <= 0.0 {
            return Err(Error::Held);
        }
        let original = parsed(&raw)?;
        let disposition = match verified.decision.as_str() {
            "approve" => "answered",
            "reject" => "declined",
            "defer" => "deferred",
            "ask" => "partial",
            _ => return Err(Error::Invalid),
        };
        // Signed creation time fixes candidate bytes across process restart/retry.
        let answer = parsed(&request.signed.answer_json)?;
        let candidate = json!({"author":self.config.owner,"session_id":self.config.session_id,"topic":original["topic"],"body":"Synthetic signed owner answer; execution admission unavailable","references":[],"message_id":verified.answer_id,"observed_at":answer["created_at"],"kind":"reply","to":[format!("session:{}",original["session_id"].as_str().ok_or(Error::Invalid)?)],"in_reply_to":request.selection.request_sequence,"data":{"disposition":disposition,"decision":verified.decision,"comment":verified.comment,"answer_id":verified.answer_id,"signed_answer":{"format":"work-engine.owner-answer/v1","answer_json":request.signed.answer_json,"credential_id":request.signed.credential_id,"authenticator_data":request.signed.authenticator_data,"client_data_json":request.signed.client_data_json,"signature":request.signed.signature},"owner_boundary":{"task":request.selection.task,"proposal":request.selection.proposal,"task_generation":request.selection.task_generation,"proposal_generation":request.selection.proposal_generation,"request_sha256":hash(&raw),"policy_sha256":self.config.policy.sha256,"source_sha256":self.config.board.script.sha256}}});
        // Any failure after entering publication is unknown. Do not create a new
        // ID, retry automatically, or let SQLite rollback negate board history.
        let published=Self::call(&self.config,json!({"operation":"conditional_reply","expected_revision":snapshot.revision,"max_elapsed_seconds":remaining,"deadline_monotonic":monotonic()+remaining,"candidate":candidate})).map_err(|_|Error::Unknown)?;
        if published["status"] == "rejected" {
            return Err(if published["code"] == "conflict" {
                Error::Conflict
            } else {
                Error::Held
            });
        }
        let snapshot = Self::snapshot(&self.config).map_err(|_| Error::Unknown)?;
        let mut d = Self::exact(&snapshot, request, &raw, &self.config, &self.key)?
            .ok_or(Error::Unknown)?;
        d.historical = false;
        d.current_standing = Self::standing(
            &tx,
            &snapshot,
            &request.selection,
            &raw,
            &self.config,
            self.clock.now(),
        )?;
        // No local decision row/receipt, admission or effect is written.
        Ok(d)
    }
}
fn map_store(e: store::Error) -> Error {
    match e {
        store::Error::Conflict => Error::Conflict,
        store::Error::Held => Error::Held,
        store::Error::Store => Error::Unavailable,
    }
}
