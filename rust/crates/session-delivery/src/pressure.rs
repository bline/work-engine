//! Service-owned existing manual-context-control reader; no pressure policy here.
use super::*;
use std::{
    fs,
    io::Read,
    os::fd::AsRawFd,
    os::unix::fs::MetadataExt,
    process::{Command, Stdio},
    time::Instant,
};

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PressureConfig {
    pub reader: PathBuf,
    pub reader_sha256: String,
    /// Explicit owner-selected incarnation; changing it requires a new store.
    pub incarnation: String,
    pub session: String,
    pub host: String,
    pub agent: Option<String>,
    pub rollout: PathBuf,
    /// Exact existing reader binding levels, never a second policy.
    pub levels: Value,
    pub step_percent: f64,
    pub lead_calls: u32,
    pub recipients: Vec<String>,
}
impl PressureConfig {
    pub fn validate(&self, targets: &[Binding]) -> Result<()> {
        let levels = &self.levels;
        let (a, b) = match levels["window_percent"].as_object() {
            Some(v) if self.host == "codex" && v.len() == 2 => (
                v.get("trigger").and_then(Value::as_f64),
                v.get("band_floor").and_then(Value::as_f64),
            ),
            _ => match levels["tokens"].as_object() {
                Some(v) if v.len() == 2 => (
                    v.get("trigger").and_then(Value::as_u64).map(|v| v as f64),
                    v.get("band_floor")
                        .and_then(Value::as_u64)
                        .map(|v| v as f64),
                ),
                _ => (None, None),
            },
        };
        if !self.reader.is_absolute()
            || !self.rollout.is_absolute()
            || !text(&self.session)
            || !text(&self.incarnation)
            || !["codex", "claude"].contains(&self.host.as_str())
            || self.reader_sha256.len() != 64
            || !self.reader_sha256.bytes().all(|v| v.is_ascii_hexdigit())
            || (self.host == "codex" && self.agent.is_some())
            || self.levels.as_object().is_none_or(|v| v.len() != 1)
            || !matches!((a,b),(Some(a),Some(b)) if a>0.0 && b>a && (levels.get("window_percent").is_none() || b<=100.0))
            || !self.step_percent.is_finite()
            || self.step_percent <= 0.0
            || self.step_percent > 100.0
            || self.lead_calls > 1000
            || self.recipients.is_empty()
            || self.recipients.len() > 16
            || self.recipients.iter().any(|r| !text(r))
            || (!targets.is_empty()
                && self
                    .recipients
                    .iter()
                    .any(|r| !targets.iter().any(|b| &b.recipient == r)))
        {
            return fail("invalid_pressure_binding");
        }
        Ok(())
    }
    pub fn sha(&self) -> String {
        digest(&serde_json::to_vec(self).expect("pressure config"))
    }
    fn binding(&self) -> Value {
        // The reader parses percent CLI values as f64. JSON integer spelling in
        // trusted config must resolve to the same native binding as 58.0.
        let levels = if let Some(v) = self.levels.get("window_percent") {
            json!({"window_percent":{"trigger":v["trigger"].as_f64(),"band_floor":v["band_floor"].as_f64()}})
        } else {
            self.levels.clone()
        };
        json!({"host":self.host,"session":self.session,"agent":self.agent,"rollout":self.rollout,
            "levels":levels,"step_percent":self.step_percent,"lead_calls":self.lead_calls})
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PressureState {
    pub binding_sha256: String,
    pub cursor: Option<String>,
    pub history_len: usize,
    pub history_sha256: String,
    pub last_output: Option<String>,
    pub hold: Option<String>,
    pub recipients: Vec<String>,
}
impl Store {
    pub fn pressure_hold(&mut self, reason: &str) -> Result<()> {
        self.change(|s| {
            if let Some(p) = s.pressure.as_mut() {
                p.hold = Some(reason.chars().take(200).collect());
            }
            Ok(())
        })
    }
    /// Read-only producer followed by one CAS transaction for cursor, inbox and deliveries.
    pub fn observe_pressure(&mut self) -> Result<()> {
        let Some(cfg) = self.config.pressure.clone() else {
            return Ok(());
        };
        let (saved, revision) = self.pressure_snapshot()?;
        let observation = read(&cfg, &saved);
        match observation
            .and_then(|(raw, history)| self.ingest_pressure(&cfg, &saved, revision, raw, &history))
        {
            Ok(()) => Ok(()),
            Err(e) => {
                self.pressure_hold(&e.to_string())?;
                Err(e)
            }
        }
    }
    // A separate pressure-owned sequence avoids changing the persisted public DTO.
    // A single SELECT binds the reader snapshot to the sequence under SQLite's
    // snapshot rules. The table and state update commit in the same transaction.
    fn pressure_snapshot(&self) -> Result<(PressureState, i64)> {
        self.connection.execute_batch("CREATE TABLE IF NOT EXISTS pressure_revision (id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL CHECK(revision>=0)); INSERT OR IGNORE INTO pressure_revision VALUES(1,0);")?;
        let (body, revision): (String, i64) = self.connection.query_row(
            "SELECT body, (SELECT revision FROM pressure_revision WHERE id=1) FROM delivery_state WHERE id=1", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
        let state: State = serde_json::from_str(&body)?;
        Ok((
            state
                .pressure
                .ok_or_else(|| Error::Contract("pressure_not_bound".into()))?,
            revision,
        ))
    }
    fn ingest_pressure(
        &mut self,
        cfg: &PressureConfig,
        previous: &PressureState,
        revision: i64,
        raw: String,
        history: &[u8],
    ) -> Result<()> {
        let output: Value = serde_json::from_str(&raw)?;
        let cursor = output["next_control_cursor"]
            .as_str()
            .ok_or_else(|| Error::Contract("missing_pressure_cursor".into()))?
            .to_owned();
        let parsed: Value = serde_json::from_str(&cursor)?;
        if output["schema"] != "manual-context-control-v1"
            || parsed["version"] != 2
            || parsed["binding"] != cfg.binding()
        {
            return fail("pressure_reader_binding_mismatch");
        }
        let source_only = self.config.targets.is_empty();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (body, current): (String, i64) = tx.query_row(
            "SELECT body, (SELECT revision FROM pressure_revision WHERE id=1) FROM delivery_state WHERE id=1", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
        if current != revision {
            return fail("pressure_cursor_conflict");
        }
        let next = revision
            .checked_add(1)
            .ok_or_else(|| Error::Contract("pressure_revision_exhausted".into()))?;
        let mut state: State = serde_json::from_str(&body)?;
        let s = &mut state;
        if source_only && !s.bindings.is_empty() {
            return fail("source_only_existing_bindings");
        }
        {
            let p = s
                .pressure
                .as_ref()
                .ok_or_else(|| Error::Contract("pressure_not_bound".into()))?;
            if p.binding_sha256 != cfg.sha()
                || p.cursor != previous.cursor
                || p.history_len != previous.history_len
                || p.history_sha256 != previous.history_sha256
            {
                return fail("pressure_cursor_conflict");
            }
            let event = &output["event"];
            if !event.is_null() {
                if event["kind"] != "context_pressure"
                    || event["authority"] != "observation_only"
                    || event["session_id"] != cfg.session
                    || event["host"] != cfg.host
                    || event["agent_id"] != json!(cfg.agent)
                {
                    return fail("pressure_event_binding_mismatch");
                }
                // A transactional admission sequence distinguishes recurring cursor states.
                // It is scoped to this store and never derives from a board row.
                let identity = digest(&serde_json::to_vec(&(
                    &cfg.incarnation,
                    cfg.sha(),
                    &cursor,
                    next,
                ))?);
                if let Some(old) = s.events.get(&identity) {
                    if old.raw_payload != raw {
                        return fail("pressure_event_identity_conflict");
                    }
                } else {
                    if s.events.len() >= MAX_EVENTS {
                        return fail("inbox_backlog_hold");
                    }
                    let e = Event {
                        event_id: identity.clone(),
                        source_id: cfg.incarnation.clone(),
                        repository: s.repository.clone(),
                        source_key: "manual-context-control-v1".into(),
                        revision: digest(history),
                        epoch: serde_json::to_string(&event["epoch"])?,
                        message_id: identity.clone(),
                        // Zero is only the legacy projection sentinel; nullable native position
                        // and timestamps remain exact in raw_payload, never lifecycle order.
                        sequence: event["source_line"].as_u64().unwrap_or(0),
                        observed_at: event["observed_at"].as_str().unwrap_or("").into(),
                        kind: "context_pressure".into(),
                        schema: "manual-context-control-v1".into(),
                        payload_codec: "reader-output-utf8-v1".into(),
                        payload_sha256: digest(raw.as_bytes()),
                        raw_payload: raw.clone(),
                    };
                    for b in s.bindings.clone().values() {
                        enqueue(s, &e, b)?;
                    }
                    s.event_order.push(identity.clone());
                    s.events.insert(identity, e);
                }
            }
            crash("after_pressure_inbox_before_cursor");
            let p = s.pressure.as_mut().expect("bound");
            p.cursor = Some(cursor);
            p.history_len = history.len();
            p.history_sha256 = digest(history);
            p.last_output = Some(raw);
            p.hold = None;
        }
        tx.execute(
            "UPDATE delivery_state SET body=?1 WHERE id=1",
            params![serde_json::to_string(&state)?],
        )?;
        tx.execute(
            "UPDATE pressure_revision SET revision=?1 WHERE id=1",
            params![next],
        )?;
        crash("before_commit");
        tx.commit()?;
        crash("after_commit");
        Ok(())
    }
}
fn history(cfg: &PressureConfig) -> Result<Vec<u8>> {
    let f = fs::File::open(&cfg.rollout)?;
    if !f.metadata()?.is_file() || f.metadata()?.len() > 256 * 1024 * 1024 {
        return fail("pressure_history_invalid_or_oversized");
    }
    let mut bytes = Vec::new();
    f.take(256 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 256 * 1024 * 1024 {
        return fail("pressure_history_oversized");
    }
    Ok(bytes)
}
fn read(cfg: &PressureConfig, saved: &PressureState) -> Result<(String, Vec<u8>)> {
    let history = history(cfg)?;
    if history.len() < saved.history_len
        || digest(&history[..saved.history_len]) != saved.history_sha256
    {
        return fail("pressure_history_reset_explicit_baseline_required");
    }
    let mut executable = fs::File::open(&cfg.reader)?;
    let meta = executable.metadata()?;
    if !meta.is_file() || meta.mode() & 0o022 != 0 || meta.len() > 64 * 1024 * 1024 {
        return fail("pressure_reader_unsafe");
    }
    let mut bytes = Vec::new();
    executable.read_to_end(&mut bytes)?;
    if digest(&bytes) != cfg.reader_sha256 {
        return fail("pressure_reader_digest_mismatch");
    }
    // Execute the same opened file that was hashed, avoiding pathname replacement.
    let mut command = Command::new(format!("/proc/self/fd/{}", executable.as_raw_fd()));
    command
        .env_clear()
        .args([
            "control",
            "--host",
            &cfg.host,
            "--session",
            &cfg.session,
            "--rollout",
        ])
        .arg(&cfg.rollout);
    if let Some(agent) = &cfg.agent {
        command.args(["--agent", agent]);
    }
    if let Some(levels) = cfg.levels.get("window_percent") {
        command.args([
            "--trigger-percent",
            &levels["trigger"].to_string(),
            "--band-floor-percent",
            &levels["band_floor"].to_string(),
        ]);
    } else {
        let levels = &cfg.levels["tokens"];
        command.args([
            "--trigger-tokens",
            &levels["trigger"].to_string(),
            "--band-floor-tokens",
            &levels["band_floor"].to_string(),
        ]);
    }
    command.args([
        "--step-percent",
        &cfg.step_percent.to_string(),
        "--lead-calls",
        &cfg.lead_calls.to_string(),
    ]);
    if let Some(cursor) = &saved.cursor {
        command.args(["--cursor", cursor]);
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child.stdout.take().expect("stdout");
    let reader = std::thread::spawn(move || {
        let mut b = Vec::new();
        stdout.take(128 * 1024 + 1).read_to_end(&mut b).map(|_| b)
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            let _ = reader.join();
            return fail("pressure_reader_timeout");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let bytes = reader
        .join()
        .map_err(|_| Error::Contract("pressure_reader_thread".into()))??;
    if !status.success() || bytes.len() > 128 * 1024 {
        return fail("pressure_reader_failed_or_oversized");
    }
    if self::history(cfg)? != history {
        return fail("pressure_history_changed_during_observation_retry_required");
    }
    let raw =
        String::from_utf8(bytes).map_err(|_| Error::Contract("pressure_reader_utf8".into()))?;
    Ok((raw, history))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Store, PressureConfig) {
        let t = tempfile::tempdir().unwrap();
        let c:Config=serde_json::from_value(json!({"schema":"session-delivery-fixture-config-v1","fixture_only":true,
            "database":t.path().join("state.sqlite"),"repository":t.path(),"repository_identity":"fixture","source_id":"board",
            "board_reader":"/unused/board","source_adapter":"/unused/adapter","targets":[{
                "recipient":"one","principal":"p","job":"j","lane":"l","session":"receiver","workspace":t.path(),
                "generation":1,"address":"role:one","fixture_mode":"observe","expires_at":4102444800u64,"active":true}],
            "pressure":{"reader":"/unused/reader","reader_sha256":"0".repeat(64),"incarnation":"fixture-pressure",
                "host":"codex","session":"source","agent":null,"rollout":"/unused/rollout",
                "levels":{"tokens":{"trigger":100,"band_floor":200}},"step_percent":5.0,"lead_calls":3,"recipients":["one"]}})).unwrap();
        let cfg = c.pressure.clone().unwrap();
        let mut s = Store::open(c).unwrap();
        s.subscribe("one", "p").unwrap();
        (t, s, cfg)
    }
    fn output(c: &PressureConfig) -> String {
        json!({"schema":"manual-context-control-v1","next_control_cursor":json!({"version":2,"binding":c.binding(),"epoch":1,"line":4}).to_string(),
            "event":{"kind":"context_pressure","authority":"observation_only","session_id":c.session,"host":c.host,"agent_id":c.agent,"epoch":1,"source_line":4,"observed_at":null,"input_tokens":null,"level":"unavailable"}}).to_string()
    }
    #[test]
    fn stale_snapshot_rejected_even_after_cursor_cycles() {
        let (_t, mut s, c) = fixture();
        let (initial, rev) = s.pressure_snapshot().unwrap();
        let raw = output(&c);
        s.ingest_pressure(&c, &initial, rev, raw.clone(), b"source")
            .unwrap();
        let (a, a_rev) = s.pressure_snapshot().unwrap();
        let mut b: Value = serde_json::from_str(&raw).unwrap();
        let mut cursor: Value =
            serde_json::from_str(b["next_control_cursor"].as_str().unwrap()).unwrap();
        cursor["level"] = json!("other");
        b["next_control_cursor"] = json!(cursor.to_string());
        s.ingest_pressure(&c, &a, a_rev, b.to_string(), b"source")
            .unwrap();
        let (mid, mid_rev) = s.pressure_snapshot().unwrap();
        s.ingest_pressure(&c, &mid, mid_rev, raw.clone(), b"source")
            .unwrap();
        let (again, again_rev) = s.pressure_snapshot().unwrap();
        assert_eq!(a.cursor, again.cursor);
        assert_eq!(a.history_sha256, again.history_sha256);
        assert!(
            s.ingest_pressure(&c, &a, a_rev, raw.clone(), b"source")
                .is_err()
        );
        let mut quiet: Value = serde_json::from_str(&raw).unwrap();
        quiet["event"] = Value::Null;
        s.ingest_pressure(&c, &again, again_rev, quiet.to_string(), b"source")
            .unwrap();
        assert_eq!(s.state().unwrap().events.len(), 3);
        let (saved, saved_rev) = s.pressure_snapshot().unwrap();
        let mut wrong: Value = serde_json::from_str(&raw).unwrap();
        let mut cursor: Value =
            serde_json::from_str(wrong["next_control_cursor"].as_str().unwrap()).unwrap();
        cursor["binding"]["session"] = json!("wrong");
        wrong["next_control_cursor"] = json!(cursor.to_string());
        assert!(
            s.ingest_pressure(&c, &saved, saved_rev, wrong.to_string(), b"source")
                .is_err()
        );
    }
    #[test]
    fn percent_integer_spelling_resolves_to_native_float_binding() {
        let (_t, _s, mut c) = fixture();
        c.levels = json!({"window_percent":{"trigger":58,"band_floor":66}});
        assert_eq!(
            c.binding()["levels"],
            json!({"window_percent":{"trigger":58.0,"band_floor":66.0}})
        );
    }
    #[test]
    fn mixed_source_dispatch_uses_ingest_order_not_native_counters() {
        let (_t, mut s, c) = fixture();
        let board = |seq, previous| {
            Observation { previous_revision:previous,schema:"board-observation-v1".into(),repository:"fixture".into(),source_key:"work-engine/codex-chatboard/v1".into(),revision:"r".into(),epoch:"git-ref-v1:work-engine/codex-chatboard/v1".into(),since:seq-1,next_since:seq,has_more:false,rows:vec![json!({"message_id":format!("board-{seq}"),"sequence":seq,"observed_at":"synthetic","topic":"fixture","session_id":"publisher","body":"observation","to":["role:one"]}).to_string()] }
        };
        s.ingest(board(1, None)).unwrap();
        let (saved, saved_rev) = s.pressure_snapshot().unwrap();
        s.ingest_pressure(&c, &saved, saved_rev, output(&c), b"source")
            .unwrap();
        s.ingest(board(2, Some("r".into()))).unwrap();
        let first = s.pending_ids().unwrap()[0].clone();
        assert_eq!(s.envelope(&first).unwrap().event.sequence, 1);
        s.change(|s| {
            s.deliveries.get_mut(&first).unwrap().target_observed = true;
            Ok(())
        })
        .unwrap();
        let next = s.pending_ids().unwrap()[0].clone();
        assert_eq!(s.envelope(&next).unwrap().event.kind, "context_pressure");
        assert_eq!(s.envelope(&next).unwrap().event.sequence, 4);
    }
    #[test]
    fn quiet_baseline_and_late_subscription_share_pressure_routing() {
        let (_t, mut s, c) = fixture();
        let (initial, initial_rev) = s.pressure_snapshot().unwrap();
        let raw = output(&c);
        let mut quiet: Value = serde_json::from_str(&raw).unwrap();
        quiet["event"] = Value::Null;
        s.ingest_pressure(&c, &initial, initial_rev, quiet.to_string(), b"source")
            .unwrap();
        assert!(s.state().unwrap().events.is_empty());
        let (saved, saved_rev) = s.pressure_snapshot().unwrap();
        s.change(|s| {
            s.bindings.clear();
            Ok(())
        })
        .unwrap();
        s.ingest_pressure(&c, &saved, saved_rev, raw, b"source")
            .unwrap();
        assert!(s.state().unwrap().deliveries.is_empty());
        s.subscribe("one", "p").unwrap();
        assert_eq!(s.state().unwrap().deliveries.len(), 1);
    }
}
