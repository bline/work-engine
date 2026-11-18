//! Pi Host controller mode: the lifecycle protocol in `native-realization`
//! `PI_HOST_PROTOCOL.md` on stdin/stdout. The controller is
//! `lifecycle-service`; it owns turn claims, tool execution, input retention
//! and settlement. This gate owns no lifecycle store: it maps controller
//! turns onto one Host job (one container, one Pi session), relays Pi's tool
//! calls, fences provider attempts to the active turn and writes each turn's
//! capture before reporting the turn.
use crate::pi_docker::coding::{
    CodingEffectGate, CodingToolIntent, LIFECYCLE_PROFILE, ProviderAttemptIntent,
};
use crate::pi_docker::{Host, Job};
use crate::{Artifact, Error, ProcessRecord, Result, digest};
use native_realization::pi_host;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const SERVE_SCHEMA: &str = "work-engine.pi-host-serve.v1";
/// The serve record (status, run receipt or error, attempt log) in the state
/// directory.
pub const SERVE_RECORD: &str = "pi-host-serve-record.json";
/// Controller frames are bounded like the Host's own input frames.
pub const MAX_CONTROLLER_LINE: usize = 1 << 20;
const MAX_TURNS: usize = 16;

fn refused<T>(message: impl Into<String>) -> Result<T> {
    Err(Error::Refused(message.into()))
}

/// Written by trusted composition before the controller spawns the Host.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServeConfig {
    pub schema: String,
    /// The admitted Host job the thread runs under (lifecycle relay profile).
    pub job: Artifact,
    /// Upper bound of one turn, within the job deadline.
    pub turn_deadline_secs: u64,
}

impl ServeConfig {
    pub fn load(path: &Path) -> Result<(Self, Job)> {
        let config: ServeConfig = serde_json::from_slice(&fs::read(path)?)?;
        if config.schema != SERVE_SCHEMA || !(1..=3600).contains(&config.turn_deadline_secs) {
            return refused("serve config schema/turn deadline");
        }
        let job: Job = serde_json::from_slice(&config.job.verify()?)?;
        let profile = job
            .coding
            .as_ref()
            .ok_or_else(|| Error::Refused("serve job has no coding profile".into()))?;
        if profile.schema != LIFECYCLE_PROFILE || profile.provider.is_none() {
            return refused("serve job must be the lifecycle relay profile with a provider plan");
        }
        Ok((config, job))
    }
}

/// The JSON-RPC writer (the Host process's stdout in production).
#[derive(Clone)]
pub struct Out(Arc<Mutex<Box<dyn Write + Send>>>);

impl Out {
    pub fn new(w: Box<dyn Write + Send>) -> Self {
        Self(Arc::new(Mutex::new(w)))
    }
    fn send(&self, v: &Value) -> Result<()> {
        let mut o = self
            .0
            .lock()
            .map_err(|_| Error::Refused("controller writer lock".into()))?;
        writeln!(o, "{v}")?;
        o.flush()?;
        Ok(())
    }
    fn reply(&self, id: &Value, result: Value) -> Result<()> {
        self.send(&json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
    fn error(&self, id: &Value, code: i64, message: &str) -> Result<()> {
        self.send(&json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}}))
    }
    fn notify(&self, method: &str, params: Value) -> Result<()> {
        self.send(&json!({"jsonrpc":"2.0","method":method,"params":params}))
    }
}

/// Reads controller lines into a channel (`None` at end of input or on an
/// over-long line, which ends the controller channel).
pub fn spawn_reader(input: impl BufRead + Send + 'static) -> Receiver<Option<Value>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut input = input;
        loop {
            let mut line = String::new();
            match std::io::Read::take(&mut input, MAX_CONTROLLER_LINE as u64 + 1)
                .read_line(&mut line)
            {
                Ok(0) | Err(_) => break,
                Ok(n) if n > MAX_CONTROLLER_LINE => break,
                Ok(_) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let parsed = serde_json::from_str::<Value>(&line).unwrap_or(Value::Null);
                    if tx.send(Some(parsed)).is_err() {
                        return;
                    }
                }
            }
        }
        let _ = tx.send(None);
    });
    rx
}

/// Identities the Host runs with, fixed at launch.
#[derive(Clone, Debug)]
pub struct Identity {
    pub host_sha256: String,
    pub job_sha256: String,
    pub provider: String,
    pub model: String,
    pub state_dir: PathBuf,
    pub routes: Vec<(String, String)>,
    /// The job's nonsecret route table (id, payer, endpoint), as admitted.
    pub route_table: Value,
    pub max_attempts_per_request: u64,
    pub job_deadline_ms: i64,
    pub turn_deadline: Duration,
    /// Protected-surface observation (H5).
    pub observe: fn(&Path) -> Result<Value>,
}

impl Identity {
    pub fn wire(&self) -> &'static str {
        if self.provider == "claude-sdk" {
            "messages"
        } else {
            "responses"
        }
    }
    fn thread_id(&self) -> String {
        format!("pi-host-thread-{}", &self.job_sha256[..16])
    }
}

struct TurnRecord {
    id: String,
    status: &'static str,
    items: Vec<Value>,
    capture: Option<(PathBuf, String)>,
}

#[derive(Clone)]
struct Active {
    number: usize,
    id: String,
    invocation: String,
    first_request: u64,
    deadline: Instant,
    deadline_ms: i64,
    /// Relayed calls in order, their `dynamicToolCall` items and answers.
    order: Vec<String>,
    items: BTreeMap<String, Value>,
    answers: BTreeMap<String, (bool, String)>,
    sent: BTreeSet<String>,
    /// Pi's completed assistant messages, in stream order.
    messages: Vec<Value>,
    sends: u64,
    prompt_sha256: String,
}

struct State {
    out: Out,
    inbox: Receiver<Option<Value>>,
    /// Controller messages read early (while checking for end of input
    /// during a turn), answered in order later.
    queued: std::collections::VecDeque<Option<Value>>,
    id: Identity,
    /// The `thread/start` request awaiting the container's readiness.
    thread_start: Option<Value>,
    thread: Option<String>,
    session: Option<String>,
    turns: Vec<TurnRecord>,
    active: Option<Active>,
    pending: Option<Value>,
    provider_calls: u64,
    last_attempt: (u64, u64),
    attempt_log: Vec<Value>,
    next_rpc: u64,
    closing: bool,
    thread_failed: bool,
    /// `claude-sdk`: the receiver's latest count of requests its forwarder
    /// answered locally (reported with each provider request and turn end).
    forwarder_refused: u64,
}

/// The controller-mode gate. Cloned handles share one state: the Host holds
/// one as its gate, the serve loop keeps one for the requests it answers
/// after the Host run ends.
#[derive(Clone)]
pub struct Controller(Rc<RefCell<State>>);

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(i64::MAX)
}

impl Controller {
    pub fn new(out: Out, inbox: Receiver<Option<Value>>, id: Identity) -> Self {
        Self(Rc::new(RefCell::new(State {
            out,
            inbox,
            queued: Default::default(),
            id,
            thread_start: None,
            thread: None,
            session: None,
            turns: vec![],
            active: None,
            pending: None,
            provider_calls: 0,
            last_attempt: (0, 0),
            attempt_log: vec![],
            next_rpc: 1_000_000,
            closing: false,
            thread_failed: false,
            forwarder_refused: 0,
        })))
    }

    /// The handshake before the Host runs: `pi-host/initialize` then
    /// `thread/start`. Returns false if the controller ended first or sent
    /// a refused `thread/start`.
    pub fn handshake(&self) -> Result<bool> {
        let mut s = self.0.borrow_mut();
        let mut initialized = false;
        loop {
            let Ok(Some(msg)) = s.inbox.recv() else {
                return Ok(false);
            };
            let Some(rid) = msg.get("id").cloned() else {
                continue;
            };
            let method = msg["method"].as_str().unwrap_or("").to_owned();
            if method.is_empty() {
                continue;
            }
            match (method.as_str(), initialized) {
                ("pi-host/initialize", false) => {
                    initialized = true;
                    let id = &s.id;
                    let result = json!({"realization":pi_host::REALIZATION,
                        "protocol_digest":pi_host::PROTOCOL_DIGEST,
                        "evidence_class":pi_host::EVIDENCE_CLASS,"host_sha256":id.host_sha256,
                        "job_sha256":id.job_sha256,"provider":id.provider,"model":id.model,
                        "wire":id.wire()});
                    s.out.reply(&rid, result)?;
                }
                ("pi-host/initialize", true) => s.out.error(&rid, -32600, "already initialized")?,
                (_, false) => s.out.error(&rid, -32002, "not initialized")?,
                ("thread/start", true) => {
                    if msg["params"]["model"] != s.id.model.as_str()
                        || msg["params"]["dynamicTools"] != pi_host::tools::dynamic_tools()
                    {
                        s.out.error(
                            &rid,
                            -32602,
                            "thread/start refused: the job model and the pinned tools",
                        )?;
                        return Ok(false);
                    }
                    s.thread_start = Some(rid);
                    return Ok(true);
                }
                _ => s
                    .out
                    .error(&rid, -32601, "method not provided by the Pi Host")?,
            }
        }
    }

    /// After the Host run returns: a turn still active is reported failed
    /// (with its capture), a pending `thread/start` is refused, and the
    /// thread's read-only requests are answered until the controller ends.
    pub fn finish(&self, run: &Result<Value>) -> Result<()> {
        {
            let mut s = self.0.borrow_mut();
            let reason = match run {
                Ok(_) => {
                    // The Host's first recorded failure names why the job ended.
                    let first = fs::read(s.id.state_dir.join("pi-state.json"))
                        .ok()
                        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                        .and_then(|j| j["failures"][0].as_str().map(str::to_owned));
                    match first {
                        Some(f) => format!("the Host job ended during the turn: {f}"),
                        None => "the Host job ended during the turn".to_string(),
                    }
                }
                Err(e) => format!("Host job failed: {e}"),
            };
            if let Some(rid) = s.thread_start.take() {
                s.out.error(&rid, -32000, &reason)?;
                return Ok(());
            }
            if s.active.is_some() {
                s.finish_turn(None, Some(reason))?;
            }
            s.thread_failed = true;
        }
        let mut s = self.0.borrow_mut();
        // Read-only requests until the controller ends; a turn cannot start
        // on an ended thread (wait_controller refuses it).
        while !s.closing {
            match s.wait_controller()? {
                Some(c) if c["type"] == "bridge_close" => s.pending = None,
                Some(_) => return refused("turn started after the Host job ended"),
                None => break,
            }
        }
        Ok(())
    }

    /// Provider attempts admitted under this thread (for the run record).
    pub fn attempts(&self) -> Vec<Value> {
        self.0.borrow().attempt_log.clone()
    }
}

impl State {
    /// The next controller message, queued ones first.
    fn next_message(
        &mut self,
        timeout: Option<Duration>,
    ) -> std::result::Result<Option<Value>, RecvTimeoutError> {
        if let Some(m) = self.queued.pop_front() {
            return Ok(m);
        }
        match timeout {
            Some(t) => self.inbox.recv_timeout(t),
            None => self
                .inbox
                .recv()
                .map_err(|_| RecvTimeoutError::Disconnected),
        }
    }
    /// Whether the controller ended its input; messages read on the way are
    /// queued. A turn cannot finish once its controller is gone, so the Host
    /// ends the job (closing the container) instead of running on.
    fn controller_ended(&mut self) -> bool {
        while let Ok(m) = self.inbox.try_recv() {
            self.queued.push_back(m);
        }
        self.queued.iter().any(Option::is_none)
    }
    fn view(&self) -> Vec<Value> {
        self.turns
            .iter()
            .map(|r| json!({"id":r.id,"status":r.status,"itemsView":"full","items":r.items}))
            .collect()
    }

    /// Answers controller requests until a turn starts (its `bridge_prompt`)
    /// or the controller ends (`bridge_close`, `None` once closing).
    fn wait_controller(&mut self) -> Result<Option<Value>> {
        loop {
            let msg = match self.next_message(None) {
                Ok(Some(msg)) => msg,
                Ok(None) | Err(_) => {
                    if self.closing {
                        return Ok(None);
                    }
                    self.closing = true;
                    let Some(session) = self.session.clone() else {
                        return Ok(None);
                    };
                    let command = json!({"type":"bridge_close","sessionId":session});
                    self.pending = Some(command.clone());
                    return Ok(Some(command));
                }
            };
            let Some(rid) = msg.get("id").cloned() else {
                continue;
            };
            let method = msg["method"].as_str().unwrap_or("").to_owned();
            if method.is_empty() {
                continue;
            }
            let params = &msg["params"];
            let thread_ok = self
                .thread
                .as_deref()
                .is_some_and(|t| params["threadId"] == t);
            match method.as_str() {
                "thread/read" if thread_ok => {
                    let result = json!({"thread":{"id":self.thread,"historyMode":"legacy","turns":self.view()}});
                    self.out.reply(&rid, result)?;
                }
                "pi-host/capture" if thread_ok => {
                    match self
                        .turns
                        .iter()
                        .find(|r| params["turnId"] == r.id.as_str())
                        .and_then(|r| r.capture.clone())
                    {
                        Some((path, sha)) => self
                            .out
                            .reply(&rid, json!({"path":path,"capture_sha256":sha}))?,
                        None => self.out.error(&rid, -32602, "no capture for that turn")?,
                    }
                }
                "turn/start" if thread_ok => {
                    let prompt = match params["input"].as_array() {
                        Some(a) if a.len() == 1 && a[0]["type"] == "text" => {
                            a[0]["text"].as_str().filter(|t| !t.is_empty())
                        }
                        _ => None,
                    };
                    if self.thread_failed || self.session.is_none() {
                        self.out.error(&rid, -32000, "the thread has ended")?;
                    } else if self.turns.len() >= MAX_TURNS {
                        self.out.error(&rid, -32000, "turn bound reached")?;
                    } else if let Some(prompt) = prompt.map(str::to_owned) {
                        return self.start_turn(&rid, prompt).map(Some);
                    } else {
                        self.out
                            .error(&rid, -32602, "turn/start needs exactly one text input")?;
                    }
                }
                "pi-host/observe" if thread_ok => match (self.id.observe)(&self.id.state_dir) {
                    Ok(v) => self.out.reply(&rid, v)?,
                    Err(e) => self
                        .out
                        .error(&rid, -32000, &format!("observation refused: {e}"))?,
                },
                "thread/read" | "pi-host/capture" | "turn/start" | "pi-host/observe" => {
                    self.out.error(&rid, -32602, "unknown thread")?
                }
                "pi-host/initialize" | "thread/start" => {
                    self.out
                        .error(&rid, -32600, "one handshake and one thread")?
                }
                _ => self
                    .out
                    .error(&rid, -32601, "method not provided by the Pi Host")?,
            }
        }
    }

    fn start_turn(&mut self, rid: &Value, prompt: String) -> Result<Value> {
        let number = self.turns.len() + 1;
        let id = format!("pi-host-turn-{number}");
        let thread = self.thread.clone().unwrap_or_default();
        let session = self.session.clone().unwrap_or_default();
        let deadline_ms = now_ms()
            .saturating_add(self.id.turn_deadline.as_millis() as i64)
            .min(self.id.job_deadline_ms);
        self.out.reply(rid, json!({"turn":{"id":id}}))?;
        self.out
            .notify("turn/started", json!({"threadId":thread,"turn":{"id":id}}))?;
        let invocation = format!("turn-{number}");
        self.active = Some(Active {
            number,
            id: id.clone(),
            invocation: invocation.clone(),
            first_request: self.provider_calls + 1,
            deadline: Instant::now() + self.id.turn_deadline,
            deadline_ms,
            order: vec![],
            items: BTreeMap::new(),
            answers: BTreeMap::new(),
            sent: BTreeSet::new(),
            messages: vec![],
            sends: 0,
            prompt_sha256: digest(prompt.as_bytes()),
        });
        let command = json!({"type":"bridge_prompt","sessionId":session,"invocationId":invocation,
            "turnCorrelation":id,"prompt":prompt});
        self.pending = Some(command.clone());
        Ok(command)
    }

    fn active(&mut self) -> Result<&mut Active> {
        match self.active.as_mut() {
            Some(a) if Instant::now() < a.deadline => Ok(a),
            Some(_) => refused("turn deadline passed"),
            None => refused("no active turn"),
        }
    }

    fn correlated(&mut self, frame: &Value) -> Result<()> {
        let session = self.session.clone();
        let a = self.active()?;
        if frame["sessionId"].as_str() != session.as_deref()
            || frame["invocationId"] != a.invocation.as_str()
            || frame["turnCorrelation"] != a.id.as_str()
        {
            return refused("frame outside the active turn");
        }
        Ok(())
    }

    fn ready(&mut self, frame: &Value) -> Result<Vec<Value>> {
        let session = frame["sessionId"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Refused("native session absent".into()))?;
        if self.session.is_some()
            || frame["snapshot"]["idle"] != true
            || frame["snapshot"]["sessionId"] != session
        {
            return refused("initial observation conflict");
        }
        let rid = self
            .thread_start
            .take()
            .ok_or_else(|| Error::Refused("ready without a thread/start".into()))?;
        self.session = Some(session.into());
        let thread = self.id.thread_id();
        self.thread = Some(thread.clone());
        self.out
            .reply(&rid, json!({"thread":{"id":thread,"ephemeral":false}}))?;
        Ok(self.wait_controller()?.into_iter().collect())
    }

    /// The forwarder count a `claude-sdk` frame carries: present, never
    /// decreasing; absent on any other provider.
    fn forwarder_count(&mut self, frame: &Value) -> Result<()> {
        let reported = frame.get("forwarderRefused");
        if self.id.provider != "claude-sdk" {
            return match reported {
                None => Ok(()),
                Some(_) => refused("forwarder count outside claude-sdk"),
            };
        }
        match reported.and_then(Value::as_u64) {
            Some(n) if n >= self.forwarder_refused => {
                self.forwarder_refused = n;
                Ok(())
            }
            _ => refused("forwarder count absent or decreased"),
        }
    }

    fn provider(&mut self, frame: &Value) -> Result<Vec<Value>> {
        self.correlated(frame)?;
        if self.pending.is_some() || frame["id"] != self.provider_calls + 1 {
            return refused("provider request outside its admitted order");
        }
        self.forwarder_count(frame)?;
        self.provider_calls += 1;
        let command = json!({"type":"bridge_provider","sessionId":self.session,
            "id":self.provider_calls,"mode":"live"});
        self.pending = Some(command.clone());
        Ok(vec![command])
    }

    fn native(&mut self, frame: &Value) -> Result<Vec<Value>> {
        if frame["sessionId"].as_str() != self.session.as_deref() {
            return refused("native event outside the session");
        }
        let event = &frame["event"];
        if event["type"] == "message_end" && event["message"]["role"] == "assistant" {
            self.active()?.messages.push(event["message"].clone());
        }
        Ok(vec![])
    }

    fn history(&mut self, frame: &Value) -> Result<Vec<Value>> {
        let session = self.session.clone();
        let a = self.active()?;
        if frame["sessionId"].as_str() != session.as_deref()
            || frame["invocationId"] != a.invocation.as_str()
            || frame["snapshot"]["idle"] != true
        {
            return refused("history outside the active turn");
        }
        self.forwarder_count(frame)?;
        self.finish_turn(Some(frame), None)?;
        Ok(self.wait_controller()?.into_iter().collect())
    }

    /// Ends the active turn: retain Pi's history, assemble the ledger items,
    /// write the capture, then the notifications and `turn/completed`.
    fn finish_turn(&mut self, history: Option<&Value>, failure: Option<String>) -> Result<()> {
        let a = self
            .active
            .take()
            .ok_or_else(|| Error::Refused("no active turn".into()))?;
        let mut failure = failure;
        let history = match history {
            Some(frame) => {
                let path = self
                    .id
                    .state_dir
                    .join(format!("pi-host-{}.history.json", a.id));
                Some(create_only(&path, &serde_json::to_vec(frame)?)?)
            }
            None => None,
        };
        let items = match ledger_items(&a) {
            Ok(items) => items,
            Err(e) => {
                failure.get_or_insert(e);
                vec![]
            }
        };
        if failure.is_none() {
            let last = a.messages.last();
            let final_text = items.last().filter(|i| i["type"] == "agentMessage");
            if last.is_none_or(|m| m["stopReason"] != "stop") || final_text.is_none() {
                failure = Some("the turn ended without a final answer".into());
            } else if final_text
                .and_then(|i| i["text"].as_str())
                .is_some_and(|t| t.len() > pi_host::MAX_ANSWER_BYTES)
            {
                failure = Some("final answer exceeds its bound".into());
            }
        }
        let mut capture = build_capture(
            &self.id,
            &a,
            self.provider_calls,
            self.forwarder_refused,
            history.as_ref(),
            &items,
        );
        // H5: the protected surface as observed when the turn ended.
        if let (Ok(v), Some(_)) = (capture.as_mut(), &history) {
            match (self.id.observe)(&self.id.state_dir) {
                Ok(o) => v["observation"] = o,
                Err(e) => {
                    v["observation"] = Value::Null;
                    if let Some(r) = v["eligibility"]["reasons"].as_array_mut() {
                        r.push(json!(format!("protected-surface observation: {e}")));
                    }
                }
            }
        }
        let (capture_value, reasons) = match capture {
            Ok(v) => {
                let reasons: Vec<String> = v["eligibility"]["reasons"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|r| r.as_str().map(str::to_owned))
                    .collect();
                (Some(v), reasons)
            }
            Err(e) => (None, vec![format!("capture: {e}")]),
        };
        if let Some(r) = reasons.first() {
            failure.get_or_insert(r.clone());
        }
        let thread = self.thread.clone().unwrap_or_default();
        let mut view = self.view();
        view.push(json!({"id":a.id,"status":"completed","itemsView":"full","items":items}));
        let frame = json!({"jsonrpc":"2.0","id":u64::MAX,"result":{"thread":{"id":thread,"historyMode":"legacy","turns":view}}});
        if serde_json::to_vec(&frame)?.len() > pi_host::HISTORY_FRAME_LIMIT {
            failure.get_or_insert("Host-ledger history exceeds the lifecycle frame bound".into());
        }
        let capture = match capture_value {
            Some(mut v) => {
                v["turn"]["status"] = json!(if failure.is_some() {
                    "failed"
                } else {
                    "completed"
                });
                v["turn"]["failure"] = json!(failure);
                v["eligibility"]["eligible"] = json!(failure.is_none());
                let path = self
                    .id
                    .state_dir
                    .join(format!("pi-host-{}.capture.json", a.id));
                let artifact = create_only(&path, &serde_json::to_vec_pretty(&v)?)?;
                Some((artifact.path, artifact.sha256))
            }
            None => None,
        };
        for item in items.iter().filter(|i| i["type"] == "agentMessage") {
            let mut started = item.clone();
            started["text"] = json!("");
            self.out.notify(
                "item/started",
                json!({"threadId":thread,"turnId":a.id,"item":started}),
            )?;
            self.out.notify(
                "item/completed",
                json!({"threadId":thread,"turnId":a.id,"item":item}),
            )?;
        }
        let (status, error) = match &failure {
            None => ("completed", Value::Null),
            Some(reason) => ("failed", json!({"message":reason})),
        };
        self.thread_failed |= failure.is_some();
        self.turns.push(TurnRecord {
            id: a.id.clone(),
            status,
            items,
            capture,
        });
        self.out.notify(
            "turn/completed",
            json!({"threadId":thread,"turn":{"id":a.id,"status":status,"error":error}}),
        )
    }

    fn relay(
        &mut self,
        id: &str,
        tool: &str,
        arguments: &Value,
        until: Instant,
    ) -> Result<(bool, String)> {
        let namespace = pi_host::tools::namespace(tool)
            .ok_or_else(|| Error::Refused("tool has no lifecycle identity".into()))?;
        let thread = self.thread.clone().unwrap_or_default();
        self.next_rpc += 1;
        let rpc = self.next_rpc;
        let a = self.active()?;
        if a.items.contains_key(id) {
            return refused("tool call relayed twice");
        }
        let turn = a.id.clone();
        let until = until.min(a.deadline);
        self.out.send(
            &json!({"jsonrpc":"2.0","id":rpc,"method":"item/tool/call","params":{
            "threadId":thread,"turnId":turn,"callId":id,"namespace":namespace,"tool":tool,
            "arguments":arguments}}),
        )?;
        let answer = loop {
            let left = until.saturating_duration_since(Instant::now());
            match self.next_message(Some(left.max(Duration::from_millis(1)))) {
                Ok(Some(msg)) if msg.get("method").is_none() && msg["id"] == rpc => break msg,
                Ok(Some(msg)) if msg.get("method").is_some() => {
                    if let Some(rid) = msg.get("id") {
                        self.out.error(rid, -32000, "a turn is active")?;
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) => {
                    self.closing = true;
                    return refused("controller ended during a relayed call");
                }
                Err(RecvTimeoutError::Timeout) => return refused("controller answer missing"),
                Err(RecvTimeoutError::Disconnected) => return refused("controller channel closed"),
            }
        };
        if answer.get("error").is_some() {
            return refused(format!("controller refused tool call {id}"));
        }
        let result = &answer["result"];
        let content = result["contentItems"].clone();
        let text = match content.as_array() {
            Some(c)
                if c.len() == 1
                    && c[0]["type"] == "inputText"
                    && c[0].as_object().is_some_and(|o| o.len() == 2) =>
            {
                c[0]["text"].as_str().map(str::to_owned)
            }
            _ => None,
        };
        let (Some(text), Some(success)) = (text, result["success"].as_bool()) else {
            return refused(format!("controller answer for {id} is not one inputText"));
        };
        let item = json!({"type":"dynamicToolCall","id":id,"namespace":namespace,"tool":tool,
            "arguments":arguments,"status":"completed","success":success,"contentItems":content});
        self.out.notify(
            "item/completed",
            json!({"threadId":thread,"turnId":turn,"item":item}),
        )?;
        let a = self.active()?;
        a.order.push(id.into());
        a.items.insert(id.into(), item);
        a.answers.insert(id.into(), (success, text.clone()));
        Ok((success, text))
    }
}

/// Items of the turn in stream order: assistant text blocks become
/// `agentMessage` items, tool calls their relayed `dynamicToolCall` items.
/// Every tool call must have been relayed and every relayed call must
/// appear.
fn ledger_items(a: &Active) -> std::result::Result<Vec<Value>, String> {
    let mut items = vec![];
    let mut seen = BTreeSet::new();
    let mut texts = 0;
    for message in &a.messages {
        for block in message["content"].as_array().into_iter().flatten() {
            match block["type"].as_str() {
                Some("text") => {
                    let t = block["text"].as_str().unwrap_or("");
                    if !t.is_empty() {
                        texts += 1;
                        items.push(json!({"type":"agentMessage","id":format!("{}:message:{texts}",a.id),"text":t}));
                    }
                }
                Some("toolCall") => {
                    let id = block["id"].as_str().unwrap_or("");
                    let item = a
                        .items
                        .get(id)
                        .ok_or(format!("tool call {id} was not relayed"))?;
                    if !seen.insert(id.to_owned()) {
                        return Err(format!("tool call {id} appears twice"));
                    }
                    items.push(item.clone());
                }
                // Pi's thinking block as observed on claude-sdk (board 5886):
                // exactly {type, thinking, thinkingSignature}, all strings.
                // Anything else (no signature, an extra or redacted marker)
                // is not a recognized shape.
                Some("thinking") => {
                    let keys: BTreeSet<&str> = block
                        .as_object()
                        .map(|o| o.keys().map(String::as_str).collect())
                        .unwrap_or_default();
                    if keys != BTreeSet::from(["type", "thinking", "thinkingSignature"])
                        || !block["thinking"].is_string()
                        || !block["thinkingSignature"].is_string()
                    {
                        return Err("thinking block outside the observed shape".into());
                    }
                }
                other => return Err(format!("unknown assistant block {other:?}")),
            }
        }
    }
    if seen.len() != a.items.len() {
        return Err("a relayed call is absent from Pi's messages".into());
    }
    Ok(items)
}

fn create_only(path: &Path, bytes: &[u8]) -> Result<Artifact> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(Artifact {
        path: path.to_owned(),
        sha256: digest(bytes),
    })
}

/// SSE `data:` payloads of a retained stream.
fn sse_events(bytes: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .filter_map(|d| serde_json::from_str(d.trim()).ok())
        .collect()
}

/// The response id and model tool call ids of one retained response stream.
fn response_calls(wire: &str, events: &[Value]) -> (Option<String>, BTreeSet<String>) {
    let mut response = None;
    let mut calls = BTreeSet::new();
    for e in events {
        if wire == "messages" {
            if e["type"] == "message_start" {
                response = e["message"]["id"].as_str().map(str::to_owned);
            }
            if e["type"] == "content_block_start"
                && e["content_block"]["type"] == "tool_use"
                && let Some(id) = e["content_block"]["id"].as_str()
            {
                calls.insert(id.to_owned());
            }
        } else {
            if matches!(
                e["type"].as_str(),
                Some("response.created" | "response.completed")
            ) {
                response = e["response"]["id"].as_str().map(str::to_owned).or(response);
            }
            if e["item"]["type"] == "function_call"
                && let Some(id) = e["item"]["call_id"].as_str()
            {
                calls.insert(id.to_owned());
            }
        }
    }
    (response, calls)
}

/// The wire call id of a Pi tool call id (`call|item` on Responses).
fn wire_call_id<'a>(wire: &str, id: &'a str) -> &'a str {
    if wire == "responses" {
        id.split('|').next().unwrap_or(id)
    } else {
        id
    }
}

/// The turn's capture (`work-engine.pi-host-capture.v1`) from the Host's
/// durable journal and retained artifacts. Eligibility reasons are listed;
/// the caller adds the turn outcome.
fn build_capture(
    id: &Identity,
    a: &Active,
    last_request: u64,
    forwarder_refused: u64,
    history: Option<&Artifact>,
    items: &[Value],
) -> Result<Value> {
    let journal: Value = serde_json::from_slice(&fs::read(id.state_dir.join("pi-state.json"))?)?;
    let requests = &journal["coding"]["provider_requests"];
    let wire = id.wire();
    let mut reasons: Vec<String> = vec![];
    let mut payloads = vec![];
    let mut responses = vec![];
    let mut attempt_records = vec![];
    let mut total = 0u64;
    // `contains_account_identifier` is false only when the shared absence
    // check verified every request body of the turn (Messages wire only).
    let mut account = wire != "messages";
    // (wire call id) -> [(request, attempt, response id)]
    let mut produced: BTreeMap<String, Vec<(u64, u64, Option<String>)>> = BTreeMap::new();
    if last_request < a.first_request {
        reasons.push("the turn made no provider request".into());
    }
    if forwarder_refused > 0 {
        reasons.push(format!(
            "the receiver's forwarder answered {forwarder_refused} request(s) locally"
        ));
    }
    for n in a.first_request..=last_request {
        let r = &requests[n.to_string()];
        if r.is_null() {
            reasons.push(format!("request {n} has no ledger record"));
            account = true;
            continue;
        }
        if r["settled_microusd"].is_null() {
            reasons.push(format!("request {n} is not settled"));
        }
        let body_path = id.state_dir.join(format!("coding-hosted-{n}.body.json"));
        let body = fs::read(&body_path).ok();
        let body_sha = body.as_deref().map(digest);
        if wire == "messages" {
            let verified = body
                .as_deref()
                .and_then(|b| serde_json::from_slice::<Value>(b).ok())
                .is_some_and(|v| pi_host::messages_account_identifier_absent(&v));
            account |= !verified;
        }
        let attempts = r["attempts"].as_array().cloned().unwrap_or_default();
        if attempts.is_empty() {
            reasons.push(format!("request {n} has no attempt"));
        }
        for (i, at) in attempts.iter().enumerate() {
            let k = at["attempt"].as_u64().unwrap_or(0);
            let disposition = at["disposition"].as_str().unwrap_or("");
            attempt_records.push(json!({"request":n,"attempt":k,"disposition":disposition,
                "route":at["route"],"payer":at["payer"]}));
            if matches!(disposition, "pending" | "uncertain") || disposition.is_empty() {
                reasons.push(format!("request {n} attempt {k} is {disposition}"));
            }
            if i + 1 == attempts.len() && disposition != "completed" {
                reasons.push(format!("request {n} did not complete"));
            }
            if disposition == "not_dispatched" {
                continue;
            }
            if at["request_body_sha256"].as_str() != body_sha.as_deref() || body_sha.is_none() {
                reasons.push(format!(
                    "request {n} attempt {k} body is not the retained body"
                ));
            }
            total += body.as_ref().map_or(0, |b| b.len() as u64);
            payloads.push(json!({"inference_call_id":format!("request-{n}"),"physical_send":k,
                "representation":"wire","original_codec":"identity","path":body_path,
                "sha256":body_sha,"route":at["route"],"payer":at["payer"],"disposition":disposition}));
            let stream_path = id
                .state_dir
                .join(format!("coding-hosted-{n}-{k}.stream.sse"));
            let stream = fs::read(&stream_path);
            if stream.is_err() && disposition != "refused" {
                reasons.push(format!("request {n} attempt {k} response stream absent"));
            }
            if let Ok(stream) = stream {
                responses.push(
                    json!({"inference_call_id":format!("request-{n}"),"physical_send":k,
                    "path":stream_path,"sha256":digest(&stream)}),
                );
                if disposition == "completed" {
                    let (response, calls) = response_calls(wire, &sse_events(&stream));
                    for c in calls {
                        produced
                            .entry(c)
                            .or_default()
                            .push((n, k, response.clone()));
                    }
                }
            }
        }
    }
    if payloads.len() as u64 > pi_host::MAX_TURN_SENDS {
        reasons.push("physical sends exceed the turn bound".into());
    }
    if total > pi_host::MAX_TURN_BODY_BYTES {
        reasons.push("retained request bodies exceed the turn bound".into());
    }
    let mut calls = vec![];
    let mut results = vec![];
    for call in &a.order {
        let item = &a.items[call];
        let (ok, text) = &a.answers[call];
        results.push(json!({"call_id":call,"success":ok,"text_sha256":digest(text.as_bytes())}));
        if !ok {
            // The lifecycle store settles only turns whose calls all succeeded.
            reasons.push(format!(
                "call {call}: the controller answered success false"
            ));
        }
        match produced.get(wire_call_id(wire, call)).map(Vec::as_slice) {
            Some([(n, k, Some(response))]) => calls.push(json!({"call_id":call,
                "inference_call_id":format!("request-{n}"),"physical_send":k,"response_id":response,
                "namespace":item["namespace"],"tool":item["tool"],"arguments":item["arguments"]})),
            Some([_]) => reasons.push(format!("call {call}: response id absent")),
            _ => reasons.push(format!(
                "call {call} is not joined to exactly one model tool call"
            )),
        }
    }
    if history.is_none() {
        reasons.push("Pi's history for the turn is absent".into());
    }
    Ok(json!({
        "schema":pi_host::CAPTURE_SCHEMA,
        "realization":pi_host::REALIZATION,
        "protocol_digest":pi_host::PROTOCOL_DIGEST,
        "evidence_class":pi_host::EVIDENCE_CLASS,
        "identities":{"host_sha256":id.host_sha256,"job_sha256":id.job_sha256,
            "provider":id.provider,"model":id.model,"routes":id.route_table},
        "wire":wire,
        "turn":{"id":a.id,"number":a.number,"invocation":a.invocation,"prompt_sha256":a.prompt_sha256,
            "requests":[a.first_request,last_request],"items":items.len()},
        "contains_account_identifier":account,
        "payloads":payloads,
        "attempts":attempt_records,
        "responses":responses,
        "calls":calls,
        "results":results,
        "history":history,
        "eligibility":{"reasons":reasons},
    }))
}

impl CodingEffectGate for Controller {
    fn execute(
        &mut self,
        _intent: &CodingToolIntent,
        _actuator: &mut dyn FnMut(i64) -> Result<(bool, Vec<u8>)>,
    ) -> Result<(bool, Vec<u8>)> {
        refused("the lifecycle relay profile has no workspace effect")
    }
    fn observed(
        &mut self,
        _intent: &CodingToolIntent,
        _result: &Artifact,
        _process: &ProcessRecord,
        _container: &str,
    ) -> Result<()> {
        refused("the lifecycle relay profile has no workspace effect")
    }
    fn frame(&mut self, frame: &Value) -> Result<Vec<Value>> {
        let mut s = self.0.borrow_mut();
        if now_ms() >= s.id.job_deadline_ms {
            return refused("job deadline passed");
        }
        // The history frame ends the turn, so a turn already finished is
        // still reported; any other frame of a turn whose controller is gone
        // ends the job.
        if s.active.is_some() && frame["type"] != "coding_history" && s.controller_ended() {
            return refused("controller ended during a turn");
        }
        match frame["type"].as_str() {
            Some("ready") => s.ready(frame),
            Some("native_event") => s.native(frame),
            Some("coding_provider_request") => s.provider(frame),
            Some("coding_history") => s.history(frame),
            _ => refused("unsupported relay frame"),
        }
    }
    fn relay(
        &mut self,
        id: &str,
        tool: &str,
        arguments: &Value,
        until: Instant,
    ) -> Result<(bool, String)> {
        self.0.borrow_mut().relay(id, tool, arguments, until)
    }
    fn provider_attempt(
        &mut self,
        intent: &ProviderAttemptIntent,
        dispatch: &mut dyn FnMut(i64) -> Result<()>,
    ) -> Result<()> {
        let mut s = self.0.borrow_mut();
        if s.controller_ended() {
            return refused("controller ended during a turn");
        }
        if s.pending.is_some() || intent.request == 0 || intent.request != s.provider_calls {
            return refused("provider attempt outside an admitted request");
        }
        let previous = if s.last_attempt.0 == intent.request {
            s.last_attempt.1
        } else {
            0
        };
        if intent.attempt != previous + 1 || intent.attempt > s.id.max_attempts_per_request {
            return refused("provider attempt order/limit");
        }
        if !s
            .id
            .routes
            .iter()
            .any(|(r, p)| *r == intent.route && *p == intent.payer)
        {
            return refused("provider attempt route/payer not granted");
        }
        // Consumed before entry: a refused or failed attempt is never replayed.
        s.last_attempt = (intent.request, intent.attempt);
        let job_deadline = s.id.job_deadline_ms;
        let a = s.active()?;
        a.sends += 1;
        if a.sends > pi_host::MAX_TURN_SENDS {
            return refused("physical sends exceed the turn bound");
        }
        let deadline = a.deadline_ms.min(job_deadline);
        let turn = a.id.clone();
        s.attempt_log
            .push(json!({"turn":turn,"intent":intent,"deadlineMs":deadline}));
        drop(s);
        dispatch(deadline)
    }
    fn dispatch(&mut self, command: &Value, send: &mut dyn FnMut(i64) -> Result<()>) -> Result<()> {
        let mut s = self.0.borrow_mut();
        let job_deadline = s.id.job_deadline_ms;
        if command["type"] == "snapshot_result" {
            let id = command["id"].as_str().unwrap_or("").to_owned();
            let a = s.active()?;
            let expected = a
                .answers
                .get(&id)
                .map(|(ok, text)| json!({"type":"snapshot_result","id":&id,"ok":ok,"text":text}));
            if expected.as_ref() != Some(command) || !a.sent.insert(id) {
                return refused("relayed reply differs from the controller's answer");
            }
            let deadline = a.deadline_ms.min(job_deadline);
            drop(s);
            return send(deadline);
        }
        let expected = s
            .pending
            .take()
            .ok_or_else(|| Error::Refused("relay command consumed".into()))?;
        if *command != expected {
            return refused("relay command differs from admitted bytes");
        }
        let deadline = match &s.active {
            Some(a) => a.deadline_ms.min(job_deadline),
            None => job_deadline,
        };
        drop(s);
        send(deadline)
    }
}

/// Runs controller mode: the handshake, then the Host job with this gate,
/// then the thread's remaining read-only requests. `transport` binds the
/// Host's provider transport.
pub fn serve(
    config_path: &Path,
    out: Out,
    inbox: Receiver<Option<Value>>,
    bind_transport: impl FnOnce(&mut Host) -> Result<()>,
) -> Result<Value> {
    let (config, job) = ServeConfig::load(config_path)?;
    let plan = job.coding.as_ref().unwrap().provider.clone().unwrap();
    let identity = Identity {
        host_sha256: digest(&fs::read(std::env::current_exe()?)?),
        job_sha256: config.job.sha256.clone(),
        provider: plan.provider.clone(),
        model: plan.model.clone(),
        state_dir: job.state_dir.clone(),
        routes: plan.hosted.pairs(),
        route_table: serde_json::to_value(&plan.hosted.routes)?,
        max_attempts_per_request: u64::from(plan.hosted.max_attempts_per_request),
        job_deadline_ms: i64::try_from(job.deadline_unix_seconds.saturating_mul(1000))
            .unwrap_or(i64::MAX),
        turn_deadline: Duration::from_secs(config.turn_deadline_secs),
        observe: crate::pi_docker::coding::observe_relay_container,
    };
    let state_dir = identity.state_dir.clone();
    let controller = Controller::new(out, inbox, identity);
    if !controller.handshake()? {
        return Ok(json!({"status":"controller_ended_before_thread"}));
    }
    let run = (|| {
        let mut host = Host::admit(job)?;
        host.bind_coding_effect_gate(Box::new(controller.clone()))?;
        bind_transport(&mut host)?;
        let result = host.run();
        if let Ok(r) = &result
            && r["receipt"]["status"] == "component_passed"
            && let Some(id) = r["receipt_id"].as_str()
        {
            host.acknowledge(id)?;
        }
        result
    })();
    let finished = controller.finish(&run);
    let record = serve_record(&state_dir, &run, controller.attempts(), &finished)?;
    finished?;
    Ok(record)
}

/// The serve record, also retained beside the Host's journal since the
/// embedder may discard stderr. Admission can fail before the state
/// directory exists; then it is only returned.
fn serve_record(
    state_dir: &Path,
    run: &Result<Value>,
    attempts: Vec<Value>,
    finished: &Result<()>,
) -> Result<Value> {
    let mut record = json!({"status":"served","run":match run {Ok(v) => v.clone(), Err(e) => json!({"error":e.to_string()})},
        "attempts":attempts});
    if let Err(e) = finished {
        record["status"] = json!("refused");
        record["error"] = json!(e.to_string());
    }
    if state_dir.is_dir() {
        create_only(
            &state_dir.join(SERVE_RECORD),
            &serde_json::to_vec_pretty(&record)?,
        )?;
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[derive(Clone, Default)]
    struct Buf(Arc<Mutex<Vec<u8>>>);
    impl Write for Buf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Buf {
        fn frames(&self) -> Vec<Value> {
            String::from_utf8(self.0.lock().unwrap().clone())
                .unwrap()
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect()
        }
    }

    fn identity(dir: &Path) -> Identity {
        Identity {
            host_sha256: "h".repeat(64),
            job_sha256: "a".repeat(64),
            provider: "claude-sdk".into(),
            model: "claude-haiku-5-5".into(),
            state_dir: dir.into(),
            routes: vec![("primary".into(), "synthetic".into())],
            route_table: json!([{"id":"primary","payer":"synthetic","endpoint":"loopback:1"}]),
            max_attempts_per_request: 2,
            job_deadline_ms: now_ms() + 600_000,
            turn_deadline: Duration::from_secs(120),
            observe: |_| Ok(json!({"containment_verified":true})),
        }
    }

    fn sse(events: &[Value]) -> Vec<u8> {
        events
            .iter()
            .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap()))
            .collect::<String>()
            .into_bytes()
    }

    /// The Host's durable artifacts for `n` completed single-attempt requests.
    fn journal(dir: &Path, responses: &[Vec<Value>]) {
        let mut requests = serde_json::Map::new();
        for (i, events) in responses.iter().enumerate() {
            let n = i + 1;
            let body =
                json!({"model":"claude-haiku-5-5","messages":[{"role":"user","content":"q"}],
                "metadata":{"user_id":"{\"account_uuid\":\"\"}"}})
                .to_string();
            fs::write(dir.join(format!("coding-hosted-{n}.body.json")), &body).unwrap();
            fs::write(
                dir.join(format!("coding-hosted-{n}-1.stream.sse")),
                sse(events),
            )
            .unwrap();
            requests.insert(
                n.to_string(),
                json!({"settled_microusd":10,"attempts":[{"attempt":1,
                "route":"primary","payer":"synthetic","disposition":"completed",
                "request_body_sha256":digest(body.as_bytes())}]}),
            );
        }
        fs::write(
            dir.join("pi-state.json"),
            json!({"coding":{"provider_requests":requests}}).to_string(),
        )
        .unwrap();
    }

    struct Rig {
        gate: Controller,
        out: Buf,
        tx: mpsc::Sender<Option<Value>>,
        _dir: tempfile::TempDir,
    }

    fn rig() -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let out = Buf::default();
        let (tx, rx) = mpsc::channel();
        let gate = Controller::new(Out::new(Box::new(out.clone())), rx, identity(dir.path()));
        Rig {
            gate,
            out,
            tx,
            _dir: dir,
        }
    }

    fn rpc(id: u64, method: &str, params: Value) -> Option<Value> {
        Some(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
    }

    #[test]
    fn handshake_requires_initialize_and_the_pinned_tools() {
        let r = rig();
        r.tx.send(rpc(1, "initialize", json!({}))).unwrap();
        r.tx.send(rpc(2, "pi-host/initialize", json!({}))).unwrap();
        r.tx.send(rpc(
            3,
            "thread/start",
            json!({"model":"claude-haiku-5-5","dynamicTools":[]}),
        ))
        .unwrap();
        assert!(!r.gate.handshake().unwrap());
        let f = r.out.frames();
        assert_eq!(f[0]["error"]["code"], -32002);
        assert_eq!(f[1]["result"]["protocol_digest"], pi_host::PROTOCOL_DIGEST);
        assert_eq!(f[1]["result"]["wire"], "messages");
        assert_eq!(f[2]["error"]["code"], -32602);
    }

    /// A full turn through the gate: thread/start answered at readiness, the
    /// prompt, one relayed call, the final answer, capture and thread/read;
    /// then the controller ends and the gate closes the bridge.
    #[test]
    fn one_turn_relays_settles_and_captures() {
        let (f, capture) = turn(0);
        let completed = f.iter().find(|v| v["method"] == "turn/completed").unwrap();
        assert_eq!(completed["params"]["turn"]["status"], "completed", "{f:?}");
        assert!(completed["params"]["turn"]["error"].is_null());
        let read = f.iter().find(|v| v["id"] == 4).unwrap();
        let items = &read["result"]["thread"]["turns"][0]["items"];
        assert_eq!(items[0]["type"], "dynamicToolCall");
        assert_eq!(items[1]["text"], "answer");
        assert_eq!(capture["eligibility"]["eligible"], true, "{capture}");
        assert_eq!(capture["calls"][0]["response_id"], "msg_1");
        assert_eq!(capture["calls"][0]["inference_call_id"], "request-1");
        assert_eq!(capture["payloads"].as_array().unwrap().len(), 2);
        assert_eq!(capture["contains_account_identifier"], false);
        assert_eq!(
            capture["attempts"],
            json!([{"request":1,"attempt":1,"disposition":"completed","route":"primary","payer":"synthetic"},
                {"request":2,"attempt":1,"disposition":"completed","route":"primary","payer":"synthetic"}])
        );
    }

    /// M1: a request the receiver's forwarder answered locally during the
    /// turn (count reported at turn end) makes the turn ineligible, before
    /// any settlement.
    #[test]
    fn forwarder_refusal_in_the_turn_fails_it() {
        let (f, capture) = turn(1);
        let completed = f.iter().find(|v| v["method"] == "turn/completed").unwrap();
        let reason = "the receiver's forwarder answered 1 request(s) locally";
        assert_eq!(completed["params"]["turn"]["status"], "failed");
        assert_eq!(completed["params"]["turn"]["error"]["message"], reason);
        assert_eq!(capture["eligibility"]["eligible"], false);
        assert_eq!(capture["eligibility"]["reasons"], json!([reason]));
    }

    /// The frames and the capture of one relayed turn whose history reports
    /// `refused` forwarder refusals.
    fn turn(refused: u64) -> (Vec<Value>, Value) {
        let mut r = rig();
        let dir = r.gate.0.borrow().id.state_dir.clone();
        r.tx.send(rpc(1, "pi-host/initialize", json!({}))).unwrap();
        r.tx.send(rpc(
            2,
            "thread/start",
            json!({"model":"claude-haiku-5-5","dynamicTools":pi_host::tools::dynamic_tools()}),
        ))
        .unwrap();
        assert!(r.gate.handshake().unwrap());
        let thread = format!("pi-host-thread-{}", &"a".repeat(16));
        r.tx.send(rpc(
            3,
            "turn/start",
            json!({"threadId":thread,"input":[{"type":"text","text":"do it"}]}),
        ))
        .unwrap();
        let cmds = r
            .gate
            .frame(
                &json!({"type":"ready","sessionId":"s1","snapshot":{"idle":true,"sessionId":"s1"}}),
            )
            .unwrap();
        assert_eq!(cmds[0]["type"], "bridge_prompt");
        assert_eq!(cmds[0]["prompt"], "do it");
        let mut sent = vec![];
        r.gate
            .dispatch(&cmds[0], &mut |d| {
                sent.push(d);
                Ok(())
            })
            .unwrap();
        let corr =
            json!({"sessionId":"s1","invocationId":"turn-1","turnCorrelation":"pi-host-turn-1"});
        let mut req = corr.clone();
        req["type"] = json!("coding_provider_request");
        req["id"] = json!(1);
        req["forwarderRefused"] = json!(0);
        let grant = r.gate.frame(&req).unwrap();
        assert_eq!(grant[0]["type"], "bridge_provider");
        r.gate.dispatch(&grant[0], &mut |_| Ok(())).unwrap();
        let intent = ProviderAttemptIntent {
            request: 1,
            attempt: 1,
            route: "primary".into(),
            payer: "synthetic".into(),
        };
        r.gate.provider_attempt(&intent, &mut |_| Ok(())).unwrap();
        // The controller answers the relayed call.
        r.tx.send(Some(
            json!({"jsonrpc":"2.0","id":1_000_001,"result":{"success":true,
            "contentItems":[{"type":"inputText","text":"{\"bytes\":\"x\"}"}]}}),
        ))
        .unwrap();
        let args = json!({"snapshot_id":"s","member_id":"m","offset":0,"length":10});
        let (ok, text) = r
            .gate
            .relay(
                "toolu_1",
                "snapshot_read",
                &args,
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
        assert!(ok);
        assert_eq!(text, "{\"bytes\":\"x\"}");
        let reply = json!({"type":"snapshot_result","id":"toolu_1","ok":true,"text":text});
        r.gate.dispatch(&reply, &mut |_| Ok(())).unwrap();
        assert!(
            r.gate.dispatch(&reply, &mut |_| Ok(())).is_err(),
            "sent once"
        );
        req["id"] = json!(2);
        let grant = r.gate.frame(&req).unwrap();
        r.gate.dispatch(&grant[0], &mut |_| Ok(())).unwrap();
        let intent = ProviderAttemptIntent {
            request: 2,
            ..intent
        };
        r.gate.provider_attempt(&intent, &mut |_| Ok(())).unwrap();
        for m in [
            json!({"role":"assistant","stopReason":"toolUse","content":[{"type":"toolCall","id":"toolu_1","name":"snapshot_read"}]}),
            json!({"role":"assistant","stopReason":"stop","content":[{"type":"thinking","thinking":"","thinkingSignature":"sig"},{"type":"text","text":"answer"}]}),
        ] {
            r.gate
                .frame(&json!({"type":"native_event","sessionId":"s1","event":{"type":"message_end","message":m}}))
                .unwrap();
        }
        journal(
            &dir,
            &[
                vec![
                    json!({"type":"message_start","message":{"id":"msg_1"}}),
                    json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"mcp__pi__snapshot_read"}}),
                ],
                vec![json!({"type":"message_start","message":{"id":"msg_2"}})],
            ],
        );
        r.tx.send(rpc(
            4,
            "thread/read",
            json!({"threadId":thread,"includeTurns":true}),
        ))
        .unwrap();
        r.tx.send(rpc(
            5,
            "pi-host/capture",
            json!({"threadId":thread,"turnId":"pi-host-turn-1"}),
        ))
        .unwrap();
        r.tx.send(None).unwrap();
        let mut hist = corr.clone();
        hist["type"] = json!("coding_history");
        hist["snapshot"] = json!({"idle":true});
        hist["forwarderRefused"] = json!(refused);
        let close = r.gate.frame(&hist).unwrap();
        assert_eq!(close[0]["type"], "bridge_close");
        let f = r.out.frames();
        let methods: Vec<&str> = f.iter().filter_map(|v| v["method"].as_str()).collect();
        assert_eq!(
            methods,
            [
                "turn/started",
                "item/tool/call",
                "item/completed",
                "item/started",
                "item/completed",
                "turn/completed"
            ]
        );
        let cap = f.iter().find(|v| v["id"] == 5).unwrap();
        let path = cap["result"]["path"].as_str().unwrap();
        let capture: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let _ = &mut r;
        (f, capture)
    }

    /// The forwarder count: required on `claude-sdk`, never decreasing,
    /// absent on any other provider.
    #[test]
    fn forwarder_count_is_required_and_monotonic_on_claude_sdk() {
        let r = rig();
        let mut s = r.gate.0.borrow_mut();
        assert!(s.forwarder_count(&json!({})).is_err(), "absent");
        assert!(s.forwarder_count(&json!({"forwarderRefused":"0"})).is_err());
        s.forwarder_count(&json!({"forwarderRefused":2})).unwrap();
        assert_eq!(s.forwarder_refused, 2);
        assert!(
            s.forwarder_count(&json!({"forwarderRefused":1})).is_err(),
            "decreased"
        );
        s.id.provider = "openai-codex".into();
        s.forwarder_count(&json!({})).unwrap();
        assert!(s.forwarder_count(&json!({"forwarderRefused":0})).is_err());
    }

    /// R2: every attempt of the turn's requests is listed, including one
    /// never dispatched; a forwarder count above zero is a reason.
    #[test]
    fn capture_lists_every_attempt_and_forwarder_refusals() {
        let dir = tempfile::tempdir().unwrap();
        let id = identity(dir.path());
        journal(dir.path(), &[vec![]]);
        let path = dir.path().join("pi-state.json");
        let mut state: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let attempts = state["coding"]["provider_requests"]["1"]["attempts"]
            .as_array_mut()
            .unwrap();
        attempts[0]["attempt"] = json!(2);
        attempts[0]["route"] = json!("secondary");
        attempts.insert(
            0,
            json!({"attempt":1,"route":"primary","payer":"synthetic","disposition":"not_dispatched"}),
        );
        fs::write(&path, state.to_string()).unwrap();
        fs::rename(
            dir.path().join("coding-hosted-1-1.stream.sse"),
            dir.path().join("coding-hosted-1-2.stream.sse"),
        )
        .unwrap();
        let capture = build_capture(&id, &active(), 1, 3, None, &[]).unwrap();
        assert_eq!(
            capture["attempts"],
            json!([{"request":1,"attempt":1,"disposition":"not_dispatched","route":"primary","payer":"synthetic"},
                {"request":1,"attempt":2,"disposition":"completed","route":"secondary","payer":"synthetic"}])
        );
        assert_eq!(capture["payloads"].as_array().unwrap().len(), 1);
        assert_eq!(
            capture["eligibility"]["reasons"],
            json!([
                "the receiver's forwarder answered 3 request(s) locally",
                "Pi's history for the turn is absent"
            ])
        );
    }

    /// R1: the serve record is retained in an existing state directory,
    /// including a refused finish; without the directory it is only returned.
    #[test]
    fn serve_record_is_retained_in_the_state_directory() {
        let dir = tempfile::tempdir().unwrap();
        let run: Result<Value> = Err(Error::Refused("admission".into()));
        let attempts = vec![json!({"turn":"pi-host-turn-1"})];
        let record = serve_record(dir.path(), &run, attempts.clone(), &Ok(())).unwrap();
        assert_eq!(record["status"], "served");
        let retained: Value =
            serde_json::from_slice(&fs::read(dir.path().join(SERVE_RECORD)).unwrap()).unwrap();
        assert_eq!(retained, record);
        assert!(
            retained["run"]["error"]
                .as_str()
                .unwrap()
                .contains("admission")
        );
        assert_eq!(retained["attempts"], json!(attempts));
        let other = dir.path().join("other");
        fs::create_dir(&other).unwrap();
        let refused = Err(Error::Refused("finish".into()));
        let record = serve_record(&other, &Ok(json!({})), vec![], &refused).unwrap();
        assert_eq!(record["status"], "refused");
        assert!(record["error"].as_str().unwrap().contains("finish"));
        assert!(other.join(SERVE_RECORD).is_file());
        let absent = dir.path().join("absent");
        serve_record(&absent, &Ok(json!({})), vec![], &Ok(())).unwrap();
        assert!(!absent.exists());
    }

    fn active() -> Active {
        Active {
            number: 1,
            id: "t".into(),
            invocation: "turn-1".into(),
            first_request: 1,
            deadline: Instant::now(),
            deadline_ms: 0,
            order: vec![],
            items: BTreeMap::new(),
            answers: BTreeMap::new(),
            sent: BTreeSet::new(),
            messages: vec![],
            sends: 0,
            prompt_sha256: String::new(),
        }
    }

    /// End of controller input during a turn ends the Host job (the next
    /// frame or attempt is refused) instead of letting the turn run on.
    #[test]
    fn controller_end_during_a_turn_is_noticed_at_the_next_frame() {
        let r = rig();
        r.tx.send(rpc(1, "pi-host/initialize", json!({}))).unwrap();
        r.tx.send(rpc(
            2,
            "thread/start",
            json!({"model":"claude-haiku-5-5","dynamicTools":pi_host::tools::dynamic_tools()}),
        ))
        .unwrap();
        assert!(r.gate.handshake().unwrap());
        let thread = format!("pi-host-thread-{}", &"a".repeat(16));
        r.tx.send(rpc(
            3,
            "turn/start",
            json!({"threadId":thread,"input":[{"type":"text","text":"x"}]}),
        ))
        .unwrap();
        let mut gate = r.gate.clone();
        let cmds = gate
            .frame(
                &json!({"type":"ready","sessionId":"s1","snapshot":{"idle":true,"sessionId":"s1"}}),
            )
            .unwrap();
        gate.dispatch(&cmds[0], &mut |_| Ok(())).unwrap();
        r.tx.send(rpc(4, "thread/read", json!({"threadId":thread})))
            .unwrap();
        r.tx.send(None).unwrap();
        let request = json!({"type":"coding_provider_request","id":1,"sessionId":"s1",
            "invocationId":"turn-1","turnCorrelation":"pi-host-turn-1"});
        match gate.frame(&request) {
            Err(Error::Refused(m)) => assert_eq!(m, "controller ended during a turn"),
            other => panic!("{other:?}"),
        }
        // The early-read request stays queued in order.
        assert_eq!(r.gate.0.borrow().queued.len(), 2);
    }

    #[test]
    fn unrelayed_tool_call_and_missing_final_answer_fail_the_items() {
        let mut a = Active {
            number: 1,
            id: "t".into(),
            invocation: "turn-1".into(),
            first_request: 1,
            deadline: Instant::now(),
            deadline_ms: 0,
            order: vec![],
            items: BTreeMap::new(),
            answers: BTreeMap::new(),
            sent: BTreeSet::new(),
            messages: vec![json!({"role":"assistant","content":[{"type":"toolCall","id":"x"}]})],
            sends: 0,
            prompt_sha256: String::new(),
        };
        assert!(ledger_items(&a).unwrap_err().contains("not relayed"));
        // Thinking blocks: exactly the observed shape.
        let thinking = |block: Value| {
            let mut t = Active {
                messages: vec![
                    json!({"role":"assistant","content":[block,{"type":"text","text":"x"}]}),
                ],
                ..a.clone()
            };
            t.items.clear();
            ledger_items(&t)
        };
        assert!(thinking(json!({"type":"thinking","thinking":"","thinkingSignature":"s"})).is_ok());
        for bad in [
            json!({"type":"thinking","thinking":""}),
            json!({"type":"thinking","thinking":"","thinkingSignature":"s","redacted":true}),
            json!({"type":"thinking","thinking":"","thinkingSignature":1}),
            json!({"type":"redacted_thinking","data":"x"}),
        ] {
            assert!(thinking(bad.clone()).is_err(), "{bad}");
        }
        a.messages.clear();
        a.items.insert("y".into(), json!({}));
        assert!(ledger_items(&a).unwrap_err().contains("absent"));
    }

    /// `contains_account_identifier` is the shared absence check over every
    /// body of the turn: false only when each was read and verified (L1).
    #[test]
    fn capture_account_flag_is_the_verified_absence_check() {
        let dir = tempfile::tempdir().unwrap();
        let id = identity(dir.path());
        let a = active();
        let flag = |n: u64| {
            build_capture(&id, &a, n, 0, None, &[]).unwrap()["contains_account_identifier"].clone()
        };
        journal(dir.path(), &[vec![], vec![]]);
        assert_eq!(flag(2), false);
        let body = dir.path().join("coding-hosted-2.body.json");
        fs::write(
            &body,
            json!({"model":"claude-haiku-5-5","messages":[],"metadata":{"user_id":
                "{\"device_id\":\"d0\",\"account_uuid\":\"5f0c1a2e-0000-4000-8000-000000000001\",\"session_id\":\"s0\"}"}})
            .to_string(),
        )
        .unwrap();
        assert_eq!(flag(2), true);
        fs::remove_file(&body).unwrap();
        assert_eq!(flag(2), true, "an unread body is not verified");
        assert_eq!(flag(1), false);
    }

    #[test]
    fn responses_calls_join_on_the_call_id_part() {
        let events = vec![
            json!({"type":"response.created","response":{"id":"resp_1"}}),
            json!({"type":"response.output_item.done","item":{"type":"function_call","call_id":"call_1","id":"fc_1"}}),
        ];
        let (response, calls) = response_calls("responses", &events);
        assert_eq!(response.as_deref(), Some("resp_1"));
        assert!(calls.contains(wire_call_id("responses", "call_1|fc_1")));
    }
}
