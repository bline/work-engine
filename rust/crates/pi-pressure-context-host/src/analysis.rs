//! Lifecycle gate for the hosted analysis profile
//! (`work-engine.pi-analysis-hosted.v1`): one domain turn on a fresh native
//! session. The lifecycle store owns the turn (claim, send intent, current
//! grant through every IPC write and provider attempt, settlement against the
//! analysis grammar). The Host owns every effect: provider attempts and the
//! read-only snapshot deliveries, which this gate records as trusted ingress
//! before their replies are dispatched. No shell, workspace effect, checkpoint
//! or reset exists here.
use crate::coding::HostedProviderPolicy;
use lifecycle_core::*;
use lifecycle_store::{
    NativeSessionEntry, NativeTurnEntry, NativeTurnRequest, PiSettlementProof, SqliteLifecycleStore,
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use work_engine_manual_host::{
    Artifact, Error, ProcessRecord, Result,
    pi_docker::coding::{CodingEffectGate, CodingToolIntent, ProviderAttemptIntent, analysis},
};

fn refused(message: impl ToString) -> Error {
    Error::Refused(message.to_string())
}
const INVOCATION: &str = "analysis";
const TRANSPORT: &str = "analysis-wire";
const MAX_PROMPT_BYTES: usize = 32_768;

/// Pinned outside controls of one analysis job.
#[derive(Clone)]
pub struct AnalysisConfig {
    /// Exact task bytes admitted by the Host plan.
    pub task: String,
    pub expires_ms: i64,
    pub max_requests: u64,
    /// The failover policy the turn grant covers (bound into its id).
    pub hosted: HostedProviderPolicy,
}
impl AnalysisConfig {
    pub fn grant_id(&self) -> String {
        self.hosted.grant_id("grant:analysis")
    }
    fn prompt(&self) -> String {
        format!(
            "{}\n\nBounded read-only analysis job. The snapshot tools are the only material available: there is no shell, network or file editing. Read what you need, then give the complete answer as your final message.",
            self.task
        )
    }
}
struct State {
    store: SqliteLifecycleStore,
    session: NativeSessionEntry,
    parent: Option<NativeTurnEntry>,
    config: AnalysisConfig,
    clock: fn() -> ClockSample,
    sequence: u64,
    pending: Option<Value>,
    provider_calls: u64,
    hosted_attempts: (u64, u64),
    hosted_log: Vec<Value>,
    /// Host deliveries recorded as ingress; each reply is sent once.
    deliveries: BTreeMap<String, (Value, bool)>,
    closing: bool,
    report: Option<Value>,
}
#[derive(Clone)]
pub struct AnalysisSession(Rc<RefCell<State>>);
impl AnalysisSession {
    pub fn new(
        store: SqliteLifecycleStore,
        session: NativeSessionEntry,
        config: AnalysisConfig,
        clock: fn() -> ClockSample,
    ) -> Result<Self> {
        if config.task.is_empty()
            || config.prompt().len() > MAX_PROMPT_BYTES
            || !(1..=u64::from(analysis::MAX_REQUESTS)).contains(&config.max_requests)
            || config.hosted.validate().is_err()
            || config.expires_ms <= clock().wall.get()
            || session.profile_digest() != lifecycle_store::pi_analysis_profile_digest()
        {
            return Err(refused("analysis control refused"));
        }
        Ok(Self(Rc::new(RefCell::new(State {
            store,
            session,
            parent: None,
            config,
            clock,
            sequence: 0,
            pending: None,
            provider_calls: 0,
            hosted_attempts: (0, 0),
            hosted_log: vec![],
            deliveries: BTreeMap::new(),
            closing: false,
            report: None,
        }))))
    }
    /// Hosted attempts admitted under the current grant (every exit).
    pub fn hosted_attempts(&self) -> Vec<Value> {
        self.0.borrow().hosted_log.clone()
    }
    pub fn report(&self) -> Result<Value> {
        self.0
            .borrow()
            .report
            .clone()
            .ok_or_else(|| refused("analysis unfinished"))
    }
}
impl State {
    fn check(&self) -> Result<()> {
        if (self.clock)().wall.get() >= self.config.expires_ms {
            return Err(refused("analysis deadline passed"));
        }
        Ok(())
    }
    fn parent(&self) -> Result<&NativeTurnEntry> {
        self.parent
            .as_ref()
            .ok_or_else(|| refused("analysis turn absent"))
    }
    fn record(&mut self, method: &str, value: &Value) -> Result<()> {
        let State {
            store,
            session,
            parent,
            sequence,
            clock,
            ..
        } = self;
        let parent = parent
            .as_ref()
            .ok_or_else(|| refused("analysis turn absent"))?;
        *sequence += 1;
        store
            .record_native_ingress(
                session.id(),
                TRANSPORT,
                *sequence,
                &serde_json::to_vec(value)?,
                "qualified",
                Some(method),
                Some(parent.thread().as_str()),
                parent.turn().map(|t| t.as_str()),
                None,
                clock(),
            )
            .map_err(refused)?;
        Ok(())
    }
    fn queue(&mut self, command: Value) -> Result<Vec<Value>> {
        if self.pending.is_some() {
            return Err(refused("analysis command already pending"));
        }
        self.pending = Some(command.clone());
        Ok(vec![command])
    }
    fn correlated(&self, frame: &Value) -> Result<()> {
        let parent = self.parent()?;
        if frame["sessionId"] != parent.thread().as_str()
            || frame["invocationId"] != parent.invocation_id()
            || frame["turnCorrelation"] != parent.turn().map(|t| t.as_str()).unwrap_or("")
        {
            return Err(refused("analysis owner correlation conflict"));
        }
        Ok(())
    }
    fn ready(&mut self, frame: &Value) -> Result<Vec<Value>> {
        let thread = frame["sessionId"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| refused("native session absent"))?;
        if self.parent.is_some()
            || frame["snapshot"]["idle"] != true
            || frame["snapshot"]["sessionId"] != thread
        {
            return Err(refused("analysis initial observation conflict"));
        }
        self.store
            .bind_native_thread(
                &mut self.session,
                ProviderThreadId::parse(thread).map_err(refused)?,
            )
            .map_err(refused)?;
        let prompt = self.config.prompt();
        let mut turn = self
            .store
            .claim_native_turn(
                &self.session,
                NativeTurnRequest {
                    invocation_id: INVOCATION.into(),
                    attempt: AttemptId::parse(format!("attempt:{INVOCATION}")).map_err(refused)?,
                    grant: GrantId::parse(self.config.grant_id()).map_err(refused)?,
                    grant_revision: 1,
                    purpose: NativeInvocationPurpose::DomainWork,
                    prompt: prompt.as_bytes().to_vec(),
                },
                "operator:analysis",
                (self.clock)(),
            )
            .map_err(refused)?;
        self.store.mark_native_send_intent(&turn).map_err(refused)?;
        self.store
            .bind_native_turn(
                &mut turn,
                ProviderTurnId::parse(format!("turn:{INVOCATION}")).map_err(refused)?,
            )
            .map_err(refused)?;
        self.parent = Some(turn);
        self.queue(json!({"type":"bridge_prompt","sessionId":thread,"invocationId":INVOCATION,"turnCorrelation":format!("turn:{INVOCATION}"),"prompt":prompt}))
    }
    fn provider(&mut self, frame: &Value) -> Result<Vec<Value>> {
        self.correlated(frame)?;
        if self.closing
            || self.provider_calls >= self.config.max_requests
            || frame["id"] != self.provider_calls + 1
        {
            return Err(refused(
                "analysis provider request outside its admitted count",
            ));
        }
        self.provider_calls += 1;
        let mut request = frame.clone();
        request["type"] = json!("pi.provider_request");
        if let Some(m) = request.as_object_mut() {
            m.remove("schema");
            m.remove("host_workspace");
        }
        self.record("pi.provider_request", &request)?;
        let session = frame["sessionId"].clone();
        let id = self.provider_calls;
        self.queue(json!({"type":"bridge_provider","sessionId":session,"id":id,"mode":"live"}))
    }
    fn native(&mut self, frame: &Value) -> Result<Vec<Value>> {
        if frame["sessionId"] != self.parent()?.thread().as_str() || self.closing {
            return Err(refused("analysis native event outside the turn"));
        }
        let kind = frame["event"]["type"]
            .as_str()
            .ok_or_else(|| refused("native event kind"))?
            .to_string();
        self.record(&kind, &frame["event"])?;
        Ok(vec![])
    }
    fn history(&mut self, frame: &Value) -> Result<Vec<Value>> {
        let parent = self.parent()?;
        if frame["sessionId"] != parent.thread().as_str()
            || frame["invocationId"] != parent.invocation_id()
            || frame["snapshot"]["idle"] != true
            || self.closing
        {
            return Err(refused("analysis history mismatch"));
        }
        let mut history = frame.clone();
        history["type"] = json!("pi.session_history");
        if let Some(m) = history.as_object_mut() {
            m.remove("schema");
            m.remove("host_workspace");
        }
        self.record("pi.session_history", &history)?;
        let proof = PiSettlementProof {
            transport: TRANSPORT.into(),
            first_sequence: 1,
            history_sequence: self.sequence,
        };
        let clock = (self.clock)();
        let State { store, parent, .. } = self;
        let parent = parent
            .as_ref()
            .ok_or_else(|| refused("analysis turn absent"))?;
        store
            .settle_pi_coding(parent, &proof, clock)
            .map_err(refused)?;
        let class = self
            .store
            .native_recovery(INVOCATION)
            .map_err(refused)?
            .ok_or_else(|| refused("analysis turn absent after settlement"))?
            .class;
        // The answer the grammar validated: the final assistant message.
        let answer = frame["messages"]
            .as_array()
            .and_then(|m| m.last())
            .filter(|m| m["role"] == "assistant")
            .and_then(|m| m["content"].as_array())
            .map(|c| {
                c.iter()
                    .filter_map(|b| b["text"].as_str())
                    .collect::<String>()
            })
            .ok_or_else(|| refused("analysis answer absent"))?;
        self.report = Some(json!({
            "schema":"work-engine.pi-analysis-result.v1",
            "standing":"live-provider hosted analysis; real inference; no research standing, owner acceptance or domain effect",
            "settlement":class,
            "answer":answer,
            "answer_sha256":work_engine_manual_host::digest(answer.as_bytes()),
            "provider_requests":self.provider_calls,
            "snapshot_deliveries":self.deliveries.len(),
            "hosted_attempts":self.hosted_log,
        }));
        self.closing = true;
        let session = frame["sessionId"].clone();
        self.queue(json!({"type":"bridge_close","sessionId":session}))
    }
}
impl CodingEffectGate for AnalysisSession {
    fn execute(
        &mut self,
        _intent: &CodingToolIntent,
        _actuator: &mut dyn FnMut(i64) -> Result<(bool, Vec<u8>)>,
    ) -> Result<(bool, Vec<u8>)> {
        Err(refused("the analysis profile has no workspace effect"))
    }
    fn observed(
        &mut self,
        _intent: &CodingToolIntent,
        _result: &Artifact,
        _process: &ProcessRecord,
        _container: &str,
    ) -> Result<()> {
        Err(refused("the analysis profile has no workspace effect"))
    }
    fn frame(&mut self, frame: &Value) -> Result<Vec<Value>> {
        let mut s = self.0.borrow_mut();
        s.check()?;
        match frame["type"].as_str() {
            Some("ready") => s.ready(frame),
            Some("native_event") => s.native(frame),
            Some("coding_provider_request") => s.provider(frame),
            Some("coding_history") => s.history(frame),
            _ => Err(refused("unsupported analysis frame")),
        }
    }
    fn snapshot_delivered(
        &mut self,
        id: &str,
        tool: &str,
        arguments: &Value,
        result: &Value,
        artifact: &Artifact,
    ) -> Result<()> {
        let mut s = self.0.borrow_mut();
        s.check()?;
        let bytes = artifact.verify()?;
        let artifact_bytes = String::from_utf8(bytes).map_err(refused)?;
        if s.closing
            || s.deliveries.contains_key(id)
            || serde_json::from_str::<Value>(&artifact_bytes)? != *result
            || result["id"] != id
        {
            return Err(refused("analysis delivery conflict"));
        }
        let invocation = s.parent()?.invocation_id().to_string();
        let read = json!({"type":"pi.analysis_read","invocationId":invocation,"call_id":id,"tool":tool,
            "arguments":arguments,"result":result,"artifact":artifact,"artifact_bytes":artifact_bytes});
        s.record("pi.analysis_read", &read)?;
        s.deliveries.insert(id.into(), (result.clone(), false));
        Ok(())
    }
    fn provider_attempt(
        &mut self,
        intent: &ProviderAttemptIntent,
        dispatch: &mut dyn FnMut(i64) -> Result<()>,
    ) -> Result<()> {
        let mut s = self.0.borrow_mut();
        s.check()?;
        let policy = s.config.hosted.clone();
        if s.pending.is_some() || intent.request == 0 || intent.request != s.provider_calls {
            return Err(refused("provider attempt outside an admitted request"));
        }
        let previous = if s.hosted_attempts.0 == intent.request {
            s.hosted_attempts.1
        } else {
            0
        };
        if intent.attempt != previous + 1
            || intent.attempt > u64::from(policy.max_attempts_per_request)
        {
            return Err(refused("provider attempt order/limit"));
        }
        if !policy
            .routes
            .iter()
            .any(|(r, p)| *r == intent.route && *p == intent.payer)
        {
            return Err(refused("provider attempt route/payer not granted"));
        }
        // Consumed before entry: a refused or failed attempt is never replayed.
        s.hosted_attempts = (intent.request, intent.attempt);
        let clock = (s.clock)();
        let configured = s.config.expires_ms;
        let State {
            store,
            hosted_log,
            parent,
            ..
        } = &mut *s;
        let parent = parent
            .as_ref()
            .ok_or_else(|| refused("analysis turn absent"))?;
        let entry =
            json!({"invocationId":parent.invocation_id(),"intent":intent,"policy":policy.digest()});
        store
            .with_pi_coding_turn_send(parent, clock, |granted| {
                let deadline = granted.min(configured);
                let mut entry = entry;
                entry["deadlineMs"] = json!(deadline);
                hosted_log.push(entry);
                dispatch(deadline)
            })
            .map_err(refused)?
    }
    fn dispatch(&mut self, command: &Value, send: &mut dyn FnMut(i64) -> Result<()>) -> Result<()> {
        let mut s = self.0.borrow_mut();
        s.check()?;
        let clock = (s.clock)();
        // A snapshot reply is exactly the recorded delivery, sent once under
        // the current turn grant.
        if command["type"] == "snapshot_result" {
            let id = command["id"].as_str().unwrap_or("").to_string();
            match s.deliveries.get_mut(&id) {
                Some((delivered, sent)) if !*sent && delivered == command => *sent = true,
                _ => return Err(refused("snapshot reply differs from the recorded delivery")),
            }
            let State { store, parent, .. } = &mut *s;
            let parent = parent
                .as_ref()
                .ok_or_else(|| refused("analysis turn absent"))?;
            return store
                .with_pi_coding_turn_send(parent, clock, send)
                .map_err(refused)?;
        }
        let expected = s
            .pending
            .take()
            .ok_or_else(|| refused("analysis command consumed"))?;
        if *command != expected {
            return Err(refused("analysis command differs from admitted bytes"));
        }
        if command["type"] == "bridge_close" {
            if !s.closing {
                return Err(refused("close before settlement"));
            }
            return send(s.config.expires_ms);
        }
        let State { store, parent, .. } = &mut *s;
        let parent = parent
            .as_ref()
            .ok_or_else(|| refused("analysis turn absent"))?;
        store
            .with_pi_coding_turn_send(parent, clock, send)
            .map_err(refused)?
    }
}
