use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::{ProofError, Result};

const MAX_REQUEST: usize = 2_000_000;
const MAX_HEADER: usize = 16_384;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Reply {
    Text(String),
    /// Test-only Responses SSE streaming for the actual native control runner.
    Streamed(Box<Reply>),
    /// A bounded fixture compiler over the actual owner-authored source prompt.
    /// It never sets lifecycle readiness or mutates owner state.
    SemanticCompile,
    SemanticVerify,
    SuccessorUse,
    SemanticSuccessorVerify,
    ToolCall {
        call_id: String,
        namespace: String,
        name: String,
        arguments: Value,
    },
    /// Arguments are frozen by the proof from an actual owner readback before
    /// the held provider response is released.
    ToolCallFromFile {
        call_id: String,
        namespace: String,
        name: String,
        arguments_path: PathBuf,
    },
    HttpError(u16),
    DropConnection,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Step {
    /// Exact substrings required in the parsed request's canonical JSON form.
    pub required: Vec<String>,
    pub forbidden: Vec<String>,
    pub reply: Reply,
    /// A proof-owned external barrier. The request is ledgered before release.
    pub release_path: Option<PathBuf>,
}

impl Step {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            required: Vec::new(),
            forbidden: Vec::new(),
            reply: Reply::Text(text.into()),
            release_path: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub ordinal: usize,
    pub request_sha256: String,
    pub request: Value,
    pub reply_kind: String,
}

/// A finite, auth-free loopback Responses endpoint. The script controls only
/// external responses and timing. Every request is independently fsynced before
/// the corresponding response is released.
pub struct ScriptedProvider {
    address: SocketAddr,
    ledger: PathBuf,
    entries: Arc<AtomicUsize>,
    replies: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<Result<()>>>,
    expected_entries: usize,
}

impl ScriptedProvider {
    pub fn start(script: Vec<Step>, ledger: PathBuf) -> Result<Self> {
        if script.is_empty() || !ledger.is_absolute() || ledger.exists() {
            return Err(ProofError::Contract(
                "invalid provider script or ledger".into(),
            ));
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&ledger)?;
        file.sync_all()?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let entries = Arc::new(AtomicUsize::new(0));
        let replies = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let observed = entries.clone();
        let completed_replies = replies.clone();
        let stopping = stop.clone();
        let expected_entries = script.len();
        let worker = thread::spawn(move || {
            serve(
                listener,
                script,
                &mut file,
                observed,
                completed_replies,
                stopping,
            )
        });
        Ok(Self {
            address,
            ledger,
            entries,
            replies,
            stop,
            worker: Some(worker),
            expected_entries,
        })
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.address.port())
    }

    pub fn entry_count(&self) -> usize {
        self.entries.load(Ordering::SeqCst)
    }

    pub fn response_count(&self) -> usize {
        self.replies.load(Ordering::SeqCst)
    }

    pub fn ledger(&self) -> &Path {
        &self.ledger
    }

    pub fn entries(&self) -> Result<Vec<ProviderEntry>> {
        let data = std::fs::read_to_string(&self.ledger)?;
        data.lines()
            .map(|line| serde_json::from_str(line).map_err(ProofError::from))
            .collect()
    }

    pub fn finish(mut self) -> Result<Vec<ProviderEntry>> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| ProofError::Contract("provider thread panicked".into()))??;
        }
        let entries = self.entries()?;
        if entries.len() != self.expected_entries || self.entry_count() != entries.len() {
            return Err(ProofError::Contract(format!(
                "provider entries {} != expected {}",
                entries.len(),
                self.expected_entries
            )));
        }
        Ok(entries)
    }
}

fn serve(
    listener: TcpListener,
    script: Vec<Step>,
    file: &mut File,
    observed: Arc<AtomicUsize>,
    completed_replies: Arc<AtomicUsize>,
    stopping: Arc<AtomicBool>,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut script = script.into_iter();
    let mut responders = Vec::new();
    let mut outcome = Ok(());
    loop {
        if stopping.load(Ordering::SeqCst) {
            break;
        }
        if Instant::now() >= deadline {
            outcome = Err(ProofError::Deadline("provider request"));
            break;
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let prepare = (|| {
                    let request = read_request(&stream)?;
                    let ordinal = observed.load(Ordering::SeqCst) + 1;
                    let step = script.next().ok_or_else(|| {
                        ProofError::Contract(format!("unexpected provider entry {ordinal}"))
                    })?;
                    let normalized = serde_json::to_string(&request)?;
                    if step.required.iter().any(|s| !normalized.contains(s))
                        || step.forbidden.iter().any(|s| normalized.contains(s))
                    {
                        return Err(ProofError::Contract(format!(
                            "provider entry {ordinal} violated script expectation"
                        )));
                    }
                    let reply_kind = match &step.reply {
                        Reply::Text(_) => "text",
                        Reply::Streamed(_) => "streamed",
                        Reply::SemanticCompile => "semantic_compile",
                        Reply::SemanticVerify => "semantic_verify",
                        Reply::SuccessorUse => "successor_use",
                        Reply::SemanticSuccessorVerify => "semantic_successor_verify",
                        Reply::ToolCall { .. } => "tool_call",
                        Reply::ToolCallFromFile { .. } => "tool_call_from_file",
                        Reply::HttpError(_) => "http_error",
                        Reply::DropConnection => "dropped",
                    };
                    let entry = ProviderEntry {
                        ordinal,
                        request_sha256: format!("{:x}", Sha256::digest(normalized.as_bytes())),
                        request,
                        reply_kind: reply_kind.into(),
                    };
                    serde_json::to_writer(&mut *file, &entry)?;
                    file.write_all(b"\n")?;
                    file.sync_all()?;
                    observed.store(ordinal, Ordering::SeqCst);
                    Ok((ordinal, step, entry.request))
                })();
                match prepare {
                    Ok((ordinal, step, request)) => {
                        let stop = stopping.clone();
                        let completed_replies = completed_replies.clone();
                        responders.push(thread::spawn(move || {
                            if let Some(path) = step.release_path {
                                let hold = Instant::now() + Duration::from_secs(30);
                                while !path.exists() {
                                    if stop.load(Ordering::SeqCst) {
                                        return Ok(());
                                    }
                                    if Instant::now() >= hold {
                                        return Err(ProofError::Deadline("provider release"));
                                    }
                                    thread::sleep(Duration::from_millis(5));
                                }
                            }
                            write_reply(&mut stream, ordinal, step.reply, &request)?;
                            completed_replies.fetch_add(1, Ordering::SeqCst);
                            Ok(())
                        }));
                    }
                    Err(error) => {
                        outcome = Err(error);
                        break;
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => {
                outcome = Err(error.into());
                break;
            }
        }
    }
    stopping.store(true, Ordering::SeqCst);
    for responder in responders {
        let result = responder
            .join()
            .map_err(|_| ProofError::Contract("provider response thread panicked".into()))?;
        if outcome.is_ok() {
            outcome = result;
        }
    }
    outcome
}

impl Drop for ScriptedProvider {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn read_request(stream: &TcpStream) -> Result<Value> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if !line.starts_with("POST /v1/responses ") {
        return Err(ProofError::Contract("provider route mismatch".into()));
    }
    let mut header_bytes = line.len();
    let mut length = None;
    loop {
        line.clear();
        reader.read_line(&mut line)?;
        header_bytes += line.len();
        if header_bytes > MAX_HEADER {
            return Err(ProofError::Contract("provider header too large".into()));
        }
        if line == "\r\n" {
            break;
        }
        if line.is_empty() {
            return Err(ProofError::Contract("provider header EOF".into()));
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("authorization:") {
            return Err(ProofError::Contract(
                "provider received authorization".into(),
            ));
        }
        if let Some(value) = lower.strip_prefix("content-length:") {
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| ProofError::Contract("invalid content length".into()))?,
            );
        }
    }
    let length = length.ok_or_else(|| ProofError::Contract("content length absent".into()))?;
    if !(1..=MAX_REQUEST).contains(&length) {
        return Err(ProofError::Contract(
            "provider request out of bounds".into(),
        ));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

fn semantic_prompt(request: &Value, task: &str) -> Result<Value> {
    let input = request["input"]
        .as_array()
        .ok_or_else(|| ProofError::Contract("semantic request input missing".into()))?;
    let prompt_text = input
        .iter()
        .rev()
        .filter(|item| item["role"] == "user")
        .find_map(|item| {
            item["content"]
                .as_array()?
                .iter()
                .find_map(|part| part["text"].as_str())
        })
        .ok_or_else(|| ProofError::Contract("semantic user prompt absent".into()))?;
    let prompt: Value = serde_json::from_str(prompt_text)?;
    if prompt["task"] != task {
        return Err(ProofError::Contract("unexpected semantic task".into()));
    }
    Ok(prompt)
}

fn semantic_source<'a>(prompt: &'a Value, id: &str) -> Result<&'a Value> {
    let sources = prompt["sources"]
        .as_array()
        .ok_or_else(|| ProofError::Contract("semantic sources absent".into()))?;
    let mut found = sources.iter().filter(|source| source["id"] == id);
    let source = found
        .next()
        .ok_or_else(|| ProofError::Contract(format!("owner source {id} absent")))?;
    if found.next().is_some()
        || source["owner_namespace"] != "retained-task-workflow"
        || source["owner_ref"] != "task:s8:analysis"
        || source["current"] != true
        || source["revision"].as_u64().is_none()
        || source["sha256"]
            .as_str()
            .is_none_or(|hash| hash.len() != 64)
        || source["bytes_utf8"].as_str().is_none()
    {
        return Err(ProofError::Contract(format!("owner source {id} invalid")));
    }
    Ok(source)
}

fn source_reference(source: &Value) -> Value {
    json!({
        "id":source["id"],
        "owner_namespace":source["owner_namespace"],
        "owner_ref":source["owner_ref"],
        "revision":source["revision"],
        "sha256":source["sha256"]
    })
}

fn source_statement(source: &Value) -> Value {
    json!({"text":source["bytes_utf8"],"sources":[source_reference(source)]})
}

fn semantic_compile_output(request: &Value) -> Result<String> {
    let prompt = semantic_prompt(request, "compile_attributed_continuation")?;
    let sources = prompt["sources"]
        .as_array()
        .ok_or_else(|| ProofError::Contract("semantic sources absent".into()))?;
    if sources.len() != 10 || prompt["schema_version"] != 1 {
        return Err(ProofError::Contract(
            "S8 transition A source inventory changed".into(),
        ));
    }
    let objective = semantic_source(&prompt, "objective")?;
    let question = semantic_source(&prompt, "engineering-question")?;
    let snapshot = semantic_source(&prompt, "snapshot-reference")?;
    let rule = semantic_source(&prompt, "rule")?;
    let decision = semantic_source(&prompt, "decision")?;
    let commitment = semantic_source(&prompt, "commitment")?;
    let uncertainty = semantic_source(&prompt, "uncertainty")?;
    let canonical = semantic_source(&prompt, "reference")?;
    let approval = semantic_source(&prompt, "approval")?;
    let effect = sources
        .iter()
        .find(|source| {
            source["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("effect-opattempt:"))
        })
        .ok_or_else(|| ProofError::Contract("snapshot child effect source absent".into()))?;
    if objective["bytes_utf8"]
        != "Analyze the approved repository snapshot and identify the ORION-7 marker."
        || rule["bytes_utf8"] != "Use only the approved snapshot member m1."
        || rule["trust"] != "host_governing"
        || approval["bytes_utf8"] != "Approval for repository mutation remains pending."
        || effect["kind"] != "delivery_consequence"
        || !effect["bytes_utf8"]
            .as_str()
            .is_some_and(|text| text.starts_with("Completed effect opattempt:"))
    {
        return Err(ProofError::Contract(
            "S8 transition A owner material differs from fixed facts".into(),
        ));
    }
    let plan = json!({
        "schema_version":1,
        "objective":source_statement(objective),
        "work_position":source_statement(question),
        "completed":[source_statement(effect)],
        "commitments":[source_statement(commitment)],
        "decisions":[{
            "statement":source_statement(decision),"current":true,
            "supersedes":[],"invalidation_condition":null
        }],
        "uncertainties":[source_statement(uncertainty)],
        "interactions":[{
            "question_id":"question:s8:mutation-approval",
            "recipient":"operator:s8:authorized",
            "scope":"repository mutation approval",
            "status":"pending","answer_id":null,
            "statement":source_statement(approval)
        }],
        "governing":[source_statement(rule)],
        "canonical_refs":[source_statement(snapshot),source_statement(canonical)],
        "delivery_custody":[source_statement(effect)],
        "next_action":{
            "text":"Continue analysis within the approved snapshot while repository mutation approval remains pending.",
            "sources":[source_reference(rule),source_reference(approval)]
        }
    });
    Ok(serde_json::to_string(&plan)?)
}

fn semantic_verify_output(request: &Value, successor: bool) -> Result<String> {
    let task = if successor {
        "verify_successor_use_of_continuation"
    } else {
        "verify_candidate_semantics"
    };
    let prompt = semantic_prompt(request, task)?;
    let candidate = &prompt["candidate"];
    let objective = semantic_source(&prompt, "objective")?;
    let question = semantic_source(&prompt, "engineering-question")?;
    let rule = semantic_source(&prompt, "rule")?;
    let decision = semantic_source(&prompt, "decision")?;
    let commitment = semantic_source(&prompt, "commitment")?;
    let uncertainty = semantic_source(&prompt, "uncertainty")?;
    let approval = semantic_source(&prompt, "approval")?;
    let sources = prompt["sources"]
        .as_array()
        .ok_or_else(|| ProofError::Contract("semantic sources absent".into()))?;
    let effect = sources
        .iter()
        .find(|source| {
            source["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("effect-opattempt:"))
        })
        .ok_or_else(|| ProofError::Contract("snapshot child effect source absent".into()))?;
    if candidate["objective"] != source_statement(objective)
        || candidate["work_position"] != source_statement(question)
        || candidate["completed"] != json!([source_statement(effect)])
        || candidate["delivery_custody"] != json!([source_statement(effect)])
        || candidate["commitments"] != json!([source_statement(commitment)])
        || candidate["uncertainties"] != json!([source_statement(uncertainty)])
        || candidate["governing"] != json!([source_statement(rule)])
        || candidate["decisions"][0]["statement"] != source_statement(decision)
        || candidate["interactions"][0]["statement"] != source_statement(approval)
        || candidate["interactions"][0]["status"] != "pending"
        || candidate["interactions"][0]["answer_id"] != Value::Null
        || candidate["next_action"]["sources"]
            != json!([source_reference(rule), source_reference(approval)])
    {
        return Err(ProofError::Contract(
            "candidate does not retain the fixed owner facts".into(),
        ));
    }
    if successor {
        let response = prompt["successor_response"]
            .as_str()
            .ok_or_else(|| ProofError::Contract("successor response absent".into()))?;
        for required in [
            rule["bytes_utf8"].as_str().unwrap(),
            decision["bytes_utf8"].as_str().unwrap(),
            commitment["bytes_utf8"].as_str().unwrap(),
            uncertainty["bytes_utf8"].as_str().unwrap(),
            approval["bytes_utf8"].as_str().unwrap(),
            effect["bytes_utf8"].as_str().unwrap(),
        ] {
            if !response.contains(required) {
                return Err(ProofError::Contract(
                    "successor omitted a fixed owner consequence".into(),
                ));
            }
        }
    }
    let obligations = prompt["required_obligations"]
        .as_array()
        .ok_or_else(|| ProofError::Contract("verifier obligations absent".into()))?;
    if obligations.len() != sources.len() || obligations.len() != 10 {
        return Err(ProofError::Contract(
            "verifier obligation inventory changed".into(),
        ));
    }
    let mut findings = Vec::new();
    for obligation in obligations {
        let id = obligation["id"]
            .as_str()
            .ok_or_else(|| ProofError::Contract("obligation id missing".into()))?;
        let source_id = id
            .strip_prefix("obligation-")
            .ok_or_else(|| ProofError::Contract("obligation id changed".into()))?;
        let source = semantic_source(&prompt, source_id)?;
        let exact = json!([source_reference(source)]);
        if obligation["sources"] != exact {
            return Err(ProofError::Contract(format!(
                "obligation {id} source scope changed"
            )));
        }
        findings.push(json!({
            "obligation_id":id,"status":"satisfied","sources":exact,
            "explanation":format!("Candidate retains current task-owner source {source_id} with its exact citation and pending authority scope.")
        }));
    }
    Ok(serde_json::to_string(&json!({
        "whole_response_assessed":true,"findings":findings,"critical_errors":[]
    }))?)
}

fn successor_use_output(request: &Value) -> Result<String> {
    let prompt = semantic_prompt(request, "use_attributed_continuation")?;
    if prompt["challenge"]
        != "Choose the next authorized analysis action; preserve the unresolved approval."
        && prompt["challenge"]
            != "Choose the next authorized analysis action for queued Q1; preserve Q2 custody."
    {
        return Err(ProofError::Contract("successor challenge changed".into()));
    }
    let candidate = &prompt["candidate"];
    let statement = |path: &str| -> Result<&str> {
        candidate
            .pointer(path)
            .and_then(Value::as_str)
            .ok_or_else(|| ProofError::Contract(format!("successor source {path} absent")))
    };
    let objective = statement("/objective/text")?;
    let rule = statement("/governing/0/text")?;
    let decision = statement("/decisions/0/statement/text")?;
    let commitment = statement("/commitments/0/text")?;
    let uncertainty = statement("/uncertainties/0/text")?;
    let approval = statement("/interactions/0/statement/text")?;
    let effect = statement("/completed/0/text")?;
    let reference = statement("/canonical_refs/1/text")?;
    if !objective.contains("ORION-7")
        || candidate["interactions"][0]["status"] != "pending"
        || !effect.starts_with("Completed effect opattempt:")
    {
        return Err(ProofError::Contract(
            "successor candidate lacks fixed task facts".into(),
        ));
    }
    Ok(format!(
        "The approved member m1 contains ORION-7. {rule} {reference} {effect} That read is complete and will not be repeated. {commitment} {decision} {uncertainty} {approval} My next action is to continue attributed repository analysis from the approved snapshot; repository mutation stays outside this task until an authorized answer changes the task-owner state."
    ))
}

fn write_reply(
    stream: &mut TcpStream,
    ordinal: usize,
    reply: Reply,
    request: &Value,
) -> Result<()> {
    let (streamed, reply) = match reply {
        Reply::Streamed(inner) => (true, *inner),
        other => (false, other),
    };
    let item = match reply {
        Reply::Streamed(_) => return Err(ProofError::Contract("nested streamed reply".into())),
        Reply::DropConnection => return Ok(()),
        Reply::HttpError(code) => {
            write!(
                stream,
                "HTTP/1.1 {code} Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )?;
            stream.flush()?;
            return Ok(());
        }
        Reply::Text(text) => json!({
            "type":"message","id":format!("proof-message-{ordinal}"),
            "role":"assistant","status":"completed",
            "content":[{"type":"output_text","text":text,"annotations":[]}]
        }),
        Reply::SemanticCompile => json!({
            "type":"message","id":format!("proof-message-{ordinal}"),
            "role":"assistant","status":"completed",
            "content":[{"type":"output_text","text":semantic_compile_output(request)?,"annotations":[]}]
        }),
        Reply::SemanticVerify => json!({
            "type":"message","id":format!("proof-message-{ordinal}"),
            "role":"assistant","status":"completed",
            "content":[{"type":"output_text","text":semantic_verify_output(request, false)?,"annotations":[]}]
        }),
        Reply::SuccessorUse => json!({
            "type":"message","id":format!("proof-message-{ordinal}"),
            "role":"assistant","status":"completed",
            "content":[{"type":"output_text","text":successor_use_output(request)?,"annotations":[]}]
        }),
        Reply::SemanticSuccessorVerify => json!({
            "type":"message","id":format!("proof-message-{ordinal}"),
            "role":"assistant","status":"completed",
            "content":[{"type":"output_text","text":semantic_verify_output(request, true)?,"annotations":[]}]
        }),
        Reply::ToolCall {
            call_id,
            namespace,
            name,
            arguments,
        } => json!({
            "type":"function_call","id":format!("proof-tool-{ordinal}"),
            "status":"completed",
            "call_id":call_id,"namespace":namespace,"name":name,
            "arguments":serde_json::to_string(&arguments)?
        }),
        Reply::ToolCallFromFile {
            call_id,
            namespace,
            name,
            arguments_path,
        } => {
            let bytes = std::fs::read(arguments_path)?;
            if bytes.is_empty() || bytes.len() > 8192 {
                return Err(ProofError::Contract(
                    "provider tool arguments out of bounds".into(),
                ));
            }
            let arguments: Value = serde_json::from_slice(&bytes)?;
            json!({
                "type":"function_call","id":format!("proof-tool-{ordinal}"),
                "status":"completed",
                "call_id":call_id,"namespace":namespace,"name":name,
                "arguments":serde_json::to_string(&arguments)?
            })
        }
    };
    let response_id = format!("proof-response-{ordinal}");
    let response = json!({
        "id":response_id,"object":"response","created_at":1,
        "status":"completed","output":[item.clone()],
        "usage":{"input_tokens":3,"output_tokens":2,"total_tokens":5}
    });
    let mut events = vec![
        json!({"type":"response.created","response":{"id":response_id,"status":"in_progress","output":[]}}),
    ];
    if streamed {
        if item["type"] != "message" {
            return Err(ProofError::Contract(
                "streaming fixture needs text message".into(),
            ));
        }
        let text = item["content"][0]["text"]
            .as_str()
            .ok_or_else(|| ProofError::Contract("streaming text absent".into()))?;
        let item_id = item["id"].clone();
        events.push(json!({"type":"response.output_item.added","output_index":0,
            "item":{"type":"message","id":item_id,"role":"assistant","status":"in_progress","content":[]}}));
        events.push(
            json!({"type":"response.content_part.added","output_index":0,"content_index":0,
            "part":{"type":"output_text","text":"","annotations":[]}}),
        );
        for chunk in text.as_bytes().chunks(12) {
            let delta = std::str::from_utf8(chunk)
                .map_err(|_| ProofError::Contract("fixture text is not ASCII".into()))?;
            events.push(json!({"type":"response.output_text.delta","output_index":0,
                "content_index":0,"item_id":item_id,"delta":delta}));
        }
        events.push(json!({"type":"response.output_text.done","output_index":0,
            "content_index":0,"item_id":item_id,"text":text}));
        events.push(json!({"type":"response.content_part.done","output_index":0,
            "content_index":0,"part":item["content"][0]}));
    }
    events.push(json!({"type":"response.output_item.done","output_index":0,"item":item}));
    events.push(json!({"type":"response.completed","response":response}));
    let wire = events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect::<String>();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{wire}",
        wire.len()
    )?;
    stream.flush()?;
    Ok(())
}
