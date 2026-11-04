//! Offline delivery owner. Events are observations, never execution authority.
use rusqlite::{Connection, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};

mod pressure;
pub use pressure::{PressureConfig, PressureState};

pub const MAX_ROW: usize = 64 * 1024;
pub const MAX_EVENTS: usize = 1000;
pub const MAX_DELIVERIES: usize = 2000;
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Contract(String),
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
pub fn fail<T>(why: &str) -> Result<T> {
    Err(Error::Contract(why.into()))
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn text(v: &str) -> bool {
    !v.is_empty() && v.len() <= 512 && !v.contains('\0')
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub recipient: String,
    pub principal: String,
    pub job: String,
    pub lane: String,
    pub session: String,
    pub workspace: String,
    pub generation: u64,
    pub address: String,
    pub fixture_mode: String,
    pub expires_at: u64,
    pub active: bool,
}
impl Binding {
    pub fn validate(&self) -> Result<()> {
        if ![
            &self.recipient,
            &self.principal,
            &self.job,
            &self.lane,
            &self.session,
            &self.workspace,
            &self.address,
        ]
        .iter()
        .all(|v| text(v))
            || self.generation == 0
            || !["observe", "busy", "lost_ack", "offline"].contains(&self.fixture_mode.as_str())
            || !Path::new(&self.workspace).is_absolute()
        {
            return fail("invalid_binding");
        }
        Ok(())
    }
    pub fn sha(&self) -> String {
        digest(&serde_json::to_vec(self).expect("serializable binding"))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub fixture_only: bool,
    pub database: PathBuf,
    pub repository: PathBuf,
    pub repository_identity: String,
    pub source_id: String,
    pub board_reader: PathBuf,
    pub source_adapter: PathBuf,
    pub targets: Vec<Binding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pressure: Option<PressureConfig>,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.schema != "session-delivery-fixture-config-v1"
            || !self.fixture_only
            || !text(&self.source_id)
            || !text(&self.repository_identity)
            || ![
                &self.database,
                &self.repository,
                &self.board_reader,
                &self.source_adapter,
            ]
            .iter()
            .all(|p| p.is_absolute())
            || (self.targets.is_empty() && self.pressure.is_none())
            || self.targets.len() > 16
        {
            return fail("fixture_configuration_required");
        }
        if let Some(p) = &self.pressure {
            p.validate(&self.targets)?;
        }
        let mut names = std::collections::BTreeSet::new();
        for b in &self.targets {
            b.validate()?;
            if !names.insert(&b.recipient) {
                return fail("duplicate_target");
            }
        }
        Ok(())
    }
    pub fn target(&self, name: &str, principal: &str) -> Result<&Binding> {
        self.targets
            .iter()
            .find(|b| b.recipient == name && b.principal == principal)
            .ok_or_else(|| Error::Contract("unauthorized_target".into()))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub previous_revision: Option<String>,
    pub schema: String,
    pub repository: String,
    pub source_key: String,
    pub revision: String,
    pub epoch: String,
    pub since: u64,
    pub next_since: u64,
    pub has_more: bool,
    pub rows: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
pub struct Event {
    pub event_id: String,
    pub source_id: String,
    pub repository: String,
    pub source_key: String,
    pub revision: String,
    pub epoch: String,
    pub message_id: String,
    pub sequence: u64,
    pub observed_at: String,
    pub kind: String,
    pub schema: String,
    pub payload_codec: String,
    pub payload_sha256: String,
    pub raw_payload: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Persisted,
    AttemptPrepared,
    Unknown,
    TransportAccepted,
    TargetObserved,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Attempt {
    pub attempt_id: String,
    pub stage: Stage,
    pub hold: Option<String>,
    pub transport_accepted: bool,
    pub target_observed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Delivery {
    pub delivery_id: String,
    pub event_id: String,
    pub binding: Binding,
    pub binding_sha256: String,
    pub envelope_sha256: String,
    pub stage: Stage,
    pub attempts: u32,
    pub hold: Option<String>,
    pub attempt_id: String,
    pub attempt_journal: Vec<Attempt>,
    pub transport_accepted: bool,
    pub target_observed: bool,
    // Application/result is outside this contract and is never inferred from ACK.
    pub application_result: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pressure: Option<PressureState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_order: Vec<String>,
    pub source: String,
    pub repository: String,
    pub cursor: u64,
    pub source_revision: Option<String>,
    pub source_hold: Option<String>,
    pub events: BTreeMap<String, Event>,
    pub deliveries: BTreeMap<String, Delivery>,
    pub bindings: BTreeMap<String, Binding>,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub schema: String,
    pub fixture_only: bool,
    pub authority: String,
    pub delivery_id: String,
    pub attempt_id: String,
    pub subscription_id: String,
    pub binding_sha256: String,
    #[serde(rename = "recipient")]
    pub binding: Binding,
    pub event: Event,
}
impl Envelope {
    pub fn sha(&self) -> String {
        digest(&serde_json::to_vec(self).expect("serializable envelope"))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ack {
    pub schema: String,
    pub attempt_id: String,
    pub delivery_id: String,
    pub event_id: String,
    pub binding_sha256: String,
    pub envelope_sha256: String,
    pub payload_sha256: String,
    pub generation: u64,
    pub stage: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema: String,
    pub fixture_only: bool,
    pub transport_accepted: bool,
    pub ack: Ack,
}

pub struct Store {
    connection: Connection,
    config: Config,
}
impl Store {
    pub fn open(config: Config) -> Result<Self> {
        config.validate()?;
        let connection = Connection::open(&config.database)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS delivery_state (id INTEGER PRIMARY KEY CHECK(id=1), body TEXT NOT NULL);")?;
        let state = State {
            source: config.source_id.clone(),
            repository: config.repository_identity.clone(),
            ..State::default()
        };
        connection.execute(
            "INSERT OR IGNORE INTO delivery_state VALUES(1,?1)",
            [serde_json::to_string(&state)?],
        )?;
        let mut store = Self { connection, config };
        let state = store.state()?;
        if state.source != store.config.source_id
            || state.repository != store.config.repository_identity
        {
            return fail("source_identity_conflict");
        }
        let pressure = store.config.pressure.clone();
        if state.pressure.as_ref().map(|p| p.binding_sha256.clone())
            != pressure.as_ref().map(PressureConfig::sha)
        {
            if state.pressure.is_some() || pressure.is_none() {
                return fail("pressure_binding_changed_explicit_baseline_required");
            }
            let cfg = pressure.expect("present");
            store.change(|s| {
                if let Some(p) = &s.pressure {
                    return if p.binding_sha256 == cfg.sha() {
                        Ok(())
                    } else {
                        fail("pressure_binding_conflict")
                    };
                }
                s.pressure = Some(PressureState {
                    binding_sha256: cfg.sha(),
                    cursor: None,
                    history_len: 0,
                    history_sha256: digest(b""),
                    last_output: None,
                    hold: None,
                    recipients: cfg.recipients.clone(),
                });
                Ok(())
            })?;
        }
        // Migrate legacy board state once, retaining source ordering solely as
        // an initial local dispatch order. Future mixed sources use insertion order.
        if state.event_order.len() != state.events.len() {
            store.change(|s| {
                if s.event_order.len() == s.events.len() {
                    return Ok(());
                }
                let mut events: Vec<_> = s.events.values().collect();
                events.sort_by_key(|e| e.sequence);
                s.event_order = events.iter().map(|e| e.event_id.clone()).collect();
                Ok(())
            })?;
        }
        Ok(store)
    }
    pub fn refresh_config(&mut self, config: Config) -> Result<()> {
        config.validate()?;
        if config.database != self.config.database
            || config.source_id != self.config.source_id
            || config.repository_identity != self.config.repository_identity
            || config.repository != self.config.repository
            || config.board_reader != self.config.board_reader
            || config.source_adapter != self.config.source_adapter
            || config.pressure != self.config.pressure
        {
            return fail("running_source_configuration_changed");
        }
        self.config = config;
        Ok(())
    }
    pub fn unknown_hold(&mut self, id: &str, reason: &str) -> Result<()> {
        self.change(|s| {
            let d = s
                .deliveries
                .get_mut(id)
                .ok_or_else(|| Error::Contract("unknown_delivery".into()))?;
            if d.stage != Stage::Unknown {
                return fail("not_unknown");
            }
            d.hold = Some(reason.into());
            if let Some(a) = d.attempt_journal.last_mut() {
                a.hold = d.hold.clone();
            }
            Ok(())
        })
    }
    pub fn state(&self) -> Result<State> {
        let body: String =
            self.connection
                .query_row("SELECT body FROM delivery_state WHERE id=1", [], |r| {
                    r.get(0)
                })?;
        Ok(serde_json::from_str(&body)?)
    }
    fn change<T>(&mut self, op: impl FnOnce(&mut State) -> Result<T>) -> Result<T> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let body: String = tx.query_row("SELECT body FROM delivery_state WHERE id=1", [], |r| {
            r.get(0)
        })?;
        let mut state: State = serde_json::from_str(&body)?;
        let result = op(&mut state)?;
        tx.execute(
            "UPDATE delivery_state SET body=?1 WHERE id=1",
            params![serde_json::to_string(&state)?],
        )?;
        crash("before_commit");
        tx.commit()?;
        crash("after_commit");
        Ok(result)
    }
    pub fn source_hold(&mut self, reason: &str) -> Result<()> {
        self.change(|s| {
            s.source_hold = Some(reason.chars().take(200).collect());
            Ok(())
        })
    }
    pub fn subscribe(&mut self, recipient: &str, principal: &str) -> Result<Value> {
        let b = self.config.target(recipient, principal)?.clone();
        self.change(|s| {
            if let Some(old) = s.bindings.get(recipient) && old != &b { return fail("binding_conflict_explicit_successor_required"); }
            s.bindings.insert(recipient.into(), b.clone());
            for event in s.events.clone().values() { enqueue(s, event, &b)?; }
            Ok(json!({"schema":"session-delivery-status-v1","recipient":recipient,"binding_sha256":b.sha(),"generation":b.generation,"fixture_only":true}))
        })
    }
    pub fn ingest(&mut self, observation: Observation) -> Result<()> {
        let cfg = self.config.clone();
        self.change(|s| {
            if observation.schema != "board-observation-v1"
                || observation.repository != s.repository
                || observation.source_key != "work-engine/codex-chatboard/v1"
                || observation.epoch != "git-ref-v1:work-engine/codex-chatboard/v1"
                || observation.revision.is_empty()
                || observation.since != s.cursor
                || observation.previous_revision != s.source_revision
                || observation.next_since < s.cursor
                || observation.rows.len() > 500
            {
                return fail("invalid_source_cursor_or_identity");
            }
            let mut previous = 0;
            for raw in &observation.rows {
                if raw.len() > MAX_ROW {
                    return fail("oversized_event");
                }
                let row: Value = serde_json::from_str(raw)?;
                let id = row["message_id"]
                    .as_str()
                    .filter(|v| text(v))
                    .ok_or_else(|| Error::Contract("invalid_message_id".into()))?;
                let seq = row["sequence"]
                    .as_u64()
                    .ok_or_else(|| Error::Contract("invalid_sequence".into()))?;
                let at = row["observed_at"]
                    .as_str()
                    .filter(|v| text(v))
                    .ok_or_else(|| Error::Contract("invalid_observed_at".into()))?;
                if !row["topic"].is_string()
                    || !row["session_id"].is_string()
                    || !row["body"].is_string()
                    || seq == 0
                    || seq <= previous
                    || seq > observation.next_since
                {
                    return fail("invalid_event");
                }
                previous = seq;
                let event_id = digest(&serde_json::to_vec(&(&cfg.source_id, id))?);
                if let Some(old) = s.events.get(&event_id) {
                    if old.raw_payload != *raw || old.sequence != seq {
                        return fail("event_identity_conflict");
                    }
                    continue;
                }
                if seq <= s.cursor
                    || s.events
                        .values()
                        .any(|e| e.schema == "codex-chatboard-row-v1" && e.sequence == seq)
                {
                    return fail("sequence_conflict");
                }
                if s.events.len() >= MAX_EVENTS {
                    return fail("inbox_backlog_hold");
                }
                let event = Event {
                    event_id: event_id.clone(),
                    source_id: cfg.source_id.clone(),
                    repository: s.repository.clone(),
                    source_key: observation.source_key.clone(),
                    revision: observation.revision.clone(),
                    epoch: observation.epoch.clone(),
                    message_id: id.into(),
                    sequence: seq,
                    observed_at: at.into(),
                    kind: row["kind"].as_str().unwrap_or("board_message").into(),
                    schema: "codex-chatboard-row-v1".into(),
                    payload_codec: "json-object-slice-utf8-v1".into(),
                    payload_sha256: digest(raw.as_bytes()),
                    raw_payload: raw.clone(),
                };
                for b in s.bindings.clone().values() {
                    enqueue(s, &event, b)?;
                }
                s.event_order.push(event_id.clone());
                s.events.insert(event_id, event);
            }
            crash("after_inbox_before_cursor");
            s.cursor = observation.next_since;
            s.source_revision = Some(observation.revision);
            s.source_hold = None;
            Ok(())
        })
    }
    pub fn envelope(&self, id: &str) -> Result<Envelope> {
        let s = self.state()?;
        let d = s
            .deliveries
            .get(id)
            .ok_or_else(|| Error::Contract("unknown_delivery".into()))?;
        Ok(envelope(d, &s.events[&d.event_id]))
    }
    /// Final admission compares the full configured fixture binding. No provider adapter exists.
    pub fn prepare(&mut self, id: &str, now: u64) -> Result<Option<Envelope>> {
        let cfg = self.config.clone();
        self.change(|s| {
            let d = s
                .deliveries
                .get_mut(id)
                .ok_or_else(|| Error::Contract("unknown_delivery".into()))?;
            if !matches!(d.stage, Stage::Persisted) {
                return Ok(None);
            }
            let current = cfg
                .targets
                .iter()
                .find(|b| b.recipient == d.binding.recipient);
            let reason = if current != Some(&d.binding) {
                Some("current_binding_mismatch")
            } else if !d.binding.active {
                Some("revoked")
            } else if now >= d.binding.expires_at {
                Some("expired")
            } else if d.attempts >= 3 {
                Some("pre_send_retry_limit")
            } else {
                None
            };
            d.hold = reason.map(str::to_owned);
            if reason.is_some() {
                return Ok(None);
            }
            d.stage = Stage::AttemptPrepared;
            d.attempts += 1;
            d.hold = Some("prepared_no_send_evidence".into());
            d.attempt_id = digest(&serde_json::to_vec(&(&d.delivery_id, d.attempts))?);
            d.attempt_journal.push(Attempt {
                attempt_id: d.attempt_id.clone(),
                stage: d.stage.clone(),
                hold: d.hold.clone(),
                transport_accepted: false,
                target_observed: false,
            });
            d.envelope_sha256 = envelope(d, &s.events[&d.event_id]).sha();
            Ok(Some(envelope(d, &s.events[&d.event_id])))
        })
    }
    pub fn pre_send_failure(&mut self, id: &str, reason: &str) -> Result<()> {
        self.change(|s| {
            let d = s
                .deliveries
                .get_mut(id)
                .ok_or_else(|| Error::Contract("unknown_delivery".into()))?;
            if d.stage != Stage::AttemptPrepared {
                return fail("not_pre_send");
            }
            d.stage = Stage::Persisted;
            d.hold = Some(reason.into());
            if let Some(a) = d.attempt_journal.last_mut() {
                a.hold = d.hold.clone();
                a.stage = Stage::Persisted;
            }
            Ok(())
        })
    }
    pub fn possible_send(&mut self, id: &str) -> Result<()> {
        self.change(|s| {
            let d = s
                .deliveries
                .get_mut(id)
                .ok_or_else(|| Error::Contract("unknown_delivery".into()))?;
            if d.stage != Stage::AttemptPrepared {
                return fail("not_prepared");
            }
            d.stage = Stage::Unknown;
            d.hold = Some("possible_send_no_ack".into());
            if let Some(a) = d.attempt_journal.last_mut() {
                a.hold = d.hold.clone();
                a.stage = d.stage.clone();
            }
            Ok(())
        })
    }
    pub fn ack(&mut self, recipient: &str, principal: &str, ack: Ack, now: u64) -> Result<Value> {
        let b = self.config.target(recipient, principal)?.clone();
        self.change(|s| {
            let d = s
                .deliveries
                .get_mut(&ack.delivery_id)
                .ok_or_else(|| Error::Contract("unknown_delivery".into()))?;
            let e = &s.events[&d.event_id];
            if ack.schema != "work-engine.delivery-observation/v1"
                || ack.attempt_id != d.attempt_id
                || d.attempt_id.is_empty()
                || d.binding != b
                || ack.binding_sha256 != d.binding_sha256
                || ack.envelope_sha256 != d.envelope_sha256
                || ack.event_id != d.event_id
                || ack.payload_sha256 != e.payload_sha256
                || ack.generation != b.generation
            {
                return fail("ack_identity_mismatch");
            }
            if !matches!(ack.stage.as_str(), "transport_accepted" | "target_observed") {
                return fail("unsupported_ack_stage");
            }
            // Exact duplicates survive expiry; new evidence on a stale binding does not.
            if (ack.stage == "target_observed" && d.target_observed)
                || (ack.stage == "transport_accepted" && d.transport_accepted)
            {
                return Ok(
                    json!({"schema":"session-delivery-status-v1","duplicate":true,"stage":d.stage}),
                );
            }
            if now >= b.expires_at || !b.active {
                return fail("stale_ack");
            }
            if !matches!(
                d.stage,
                Stage::Unknown | Stage::TransportAccepted | Stage::TargetObserved
            ) {
                return fail("ack_without_attempt");
            }
            if ack.stage == "transport_accepted" {
                d.transport_accepted = true;
                if !d.target_observed {
                    d.stage = Stage::TransportAccepted;
                }
            } else {
                d.target_observed = true;
                d.stage = Stage::TargetObserved;
            }
            d.hold = None;
            if let Some(a) = d.attempt_journal.last_mut() {
                a.stage = d.stage.clone();
                a.hold = None;
                a.transport_accepted = d.transport_accepted;
                a.target_observed = d.target_observed;
            }
            crash("before_ack_commit");
            Ok(json!({"schema":"session-delivery-status-v1","duplicate":false,"stage":d.stage}))
        })
    }
    pub fn status(&self, recipient: &str, principal: &str) -> Result<Value> {
        self.status_after(recipient, principal, None)
    }
    pub fn status_after(
        &self,
        recipient: &str,
        principal: &str,
        after: Option<&str>,
    ) -> Result<Value> {
        if after.is_some_and(|v| {
            v.len() != 64
                || !v
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        }) {
            return fail("invalid_status_cursor");
        }
        self.config.target(recipient, principal)?;
        let s = self.state()?;
        let rows:Vec<Value>=s.deliveries.values().filter(|d|d.binding.recipient==recipient && d.binding.principal==principal && after.is_none_or(|v|d.delivery_id.as_str()>v)).take(50).map(|d| {
            let e=&s.events[&d.event_id];
            json!({"delivery_id":d.delivery_id,"attempt_id":d.attempt_id,"event_id":d.event_id,"binding_sha256":d.binding_sha256,
                "envelope_sha256":d.envelope_sha256,"payload_sha256":e.payload_sha256,"generation":d.binding.generation,
                "stage":d.stage,"hold":d.hold,"attempts":d.attempts,"transport_accepted":d.transport_accepted,
                "target_observed":d.target_observed,"application_result":d.application_result})
        }).collect();
        let count = s
            .deliveries
            .values()
            .filter(|d| d.binding.recipient == recipient && d.binding.principal == principal)
            .count();
        let remaining = s
            .deliveries
            .values()
            .filter(|d| {
                d.binding.recipient == recipient
                    && d.binding.principal == principal
                    && after.is_none_or(|v| d.delivery_id.as_str() > v)
            })
            .count();
        let next_after = if remaining > rows.len() {
            rows.last()
                .and_then(|r| r["delivery_id"].as_str())
                .map(str::to_owned)
        } else {
            None
        };
        Ok(
            json!({"schema":"session-delivery-status-v1","fixture_only":true,"recipient":recipient,"cursor":s.cursor,
            "source_hold":s.source_hold,"pressure":s.pressure.as_ref().map(|p|json!({"binding_sha256":p.binding_sha256,"cursor_committed":p.cursor.is_some(),"hold":p.hold})),"count":count,"truncated":remaining>rows.len(),"next_after":next_after,"deliveries":rows}),
        )
    }
    pub fn pending_ids(&self) -> Result<Vec<String>> {
        let s = self.state()?;
        let mut rows: Vec<_> = s.deliveries.values().collect();
        rows.sort_by_key(|d| {
            (
                s.event_order
                    .iter()
                    .position(|e| e == &d.event_id)
                    .unwrap_or(usize::MAX),
                d.binding.recipient.clone(),
            )
        });
        let mut blocked = std::collections::BTreeSet::new();
        let mut pending = Vec::new();
        for d in rows {
            if d.target_observed {
                continue;
            }
            if !blocked.insert(&d.binding.recipient) {
                continue;
            }
            if d.stage == Stage::Persisted {
                pending.push(d.delivery_id.clone());
            }
        }
        pending.truncate(50);
        Ok(pending)
    }
}
fn envelope(d: &Delivery, e: &Event) -> Envelope {
    Envelope {
        schema: "work-engine.delivery-envelope/v1".into(),
        fixture_only: true,
        authority: "observation_only".into(),
        delivery_id: d.delivery_id.clone(),
        attempt_id: d.attempt_id.clone(),
        subscription_id: d.binding.recipient.clone(),
        binding_sha256: d.binding_sha256.clone(),
        binding: d.binding.clone(),
        event: e.clone(),
    }
}
fn enqueue(s: &mut State, e: &Event, b: &Binding) -> Result<()> {
    let row: Value = serde_json::from_str(&e.raw_payload)?;
    // Exact addressing only. Broadcast/topic opt-in is deliberately not implicit.
    let pressure_match = e.schema == "manual-context-control-v1"
        && s.pressure
            .as_ref()
            .is_some_and(|p| p.recipients.contains(&b.recipient));
    if !pressure_match
        && !row["to"]
            .as_array()
            .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(&b.address)))
    {
        return Ok(());
    }
    let id = digest(&serde_json::to_vec(&(
        &e.event_id,
        &b.recipient,
        b.generation,
        b.sha(),
    ))?);
    if s.deliveries.contains_key(&id) {
        return Ok(());
    }
    if s.deliveries.len() >= MAX_DELIVERIES {
        return fail("delivery_backlog_hold");
    }
    let mut d = Delivery {
        delivery_id: id.clone(),
        event_id: e.event_id.clone(),
        binding: b.clone(),
        binding_sha256: b.sha(),
        envelope_sha256: String::new(),
        stage: Stage::Persisted,
        attempts: 0,
        hold: None,
        attempt_id: String::new(),
        attempt_journal: Vec::new(),
        transport_accepted: false,
        target_observed: false,
        application_result: None,
    };
    d.envelope_sha256 = envelope(&d, e).sha();
    s.deliveries.insert(id, d);
    Ok(())
}
/// Deliberate fixture-only process crash points. No live route is supported.
pub fn crash(point: &str) {
    if std::env::var("SESSION_DELIVERY_FIXTURE_CRASH")
        .ok()
        .as_deref()
        == Some(point)
    {
        std::process::exit(86);
    }
}
