//! Codex 0.160.1 native-turn observations. Nothing in this module issues a seal.
use crate::provider::{ProviderAction, ProviderCommand, ProviderReply, ProviderRequestMessage};
use crate::storage::Store;
use crate::*;
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command as ProcessCommand, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

pub const CLI_VERSION: &str = "0.160.1";
pub const NATIVE_BINARY: &str = "/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex";
pub const NATIVE_SHA256: &str = "f34a4d2301892ae96c90097786bfe5dc269f187b6f69faf42a7b357b8c081e35";
pub fn profile_digest() -> String {
    digest(
        &json!({"native_sha256":NATIVE_SHA256,"native_version":CLI_VERSION,"model":"gpt-6-sol","provider":"local_probe","approval":"never","sandbox":"read-only","goals":false,"code_mode_host":false,"shell":false,"agents":false,"hooks":false,"web_search":"disabled","apps":false}),
    )
}
pub fn profile_digest_for(mode: &Mode) -> Option<String> {
    match mode {
        Mode::CodexText => Some(profile_digest()),
        Mode::CodexTextLive => Some(digest(
            &json!({"native_sha256":NATIVE_SHA256,"native_version":CLI_VERSION,"model":"gpt-6-sol","provider":"openai","approval":"never","sandbox":"read-only","goals":false,"code_mode_host":false,"shell":false,"agents":false,"hooks":false,"web_search":"disabled","apps":false,"prepared_live":true}),
        )),
        _ => None,
    }
}
const LIVE_MARKER: &[u8] = b"context-lifecycle-rust prepared live profile v1\n";
const LIVE_PROFILE_CONFIG: &str = r#"model = "gpt-6-sol"
model_provider = "openai"
approval_policy = "never"
sandbox_mode = "read-only"
web_search = "disabled"
project_doc_max_bytes = 0
project_root_markers = [".context-lifecycle-project"]
[analytics]
enabled = false
[features]
shell_tool = false
multi_agent = false
hooks = false
goals = false
code_mode_host = false
[agents]
enabled = false
[apps._default]
enabled = false
"#;

/// Prepare only configuration and empty state. Login and live inference are
/// deliberately separate parent-controlled operations.
pub fn prepare_live_profile(home: &Path) -> Result<()> {
    ensure!(
        home.is_absolute(),
        "absolute disposable profile path required"
    );
    if home.exists() {
        ensure!(
            fs::read_dir(home)?.next().is_none(),
            "profile directory must be empty"
        );
    }
    fs::create_dir_all(home.join("codex"))?;
    fs::create_dir_all(home.join("user"))?;
    fs::create_dir_all(home.join("work"))?;
    fs::write(home.join("experiment-owned"), LIVE_MARKER)?;
    fs::write(home.join("codex/config.toml"), LIVE_PROFILE_CONFIG)?;
    fs::write(
        home.join("work/.context-lifecycle-project"),
        b"isolated project root\n",
    )?;
    Ok(())
}

#[cfg(test)]
mod live_profile_tests {
    use super::*;

    #[test]
    fn prepared_live_profile_is_explicit_empty_config_only() -> Result<()> {
        let parent = tempfile::tempdir()?;
        let home = parent.path().join("fresh-live-profile");
        prepare_live_profile(&home)?;
        assert_eq!(fs::read(home.join("experiment-owned"))?, LIVE_MARKER);
        assert_eq!(
            fs::read_to_string(home.join("codex/config.toml"))?,
            LIVE_PROFILE_CONFIG
        );
        assert!(!home.join("codex/auth.json").exists());
        assert!(!home.join("adapter.db").exists());
        assert_ne!(
            profile_digest_for(&Mode::CodexTextLive),
            profile_digest_for(&Mode::CodexText)
        );
        assert!(prepare_live_profile(&home).is_err());
        Ok(())
    }
}
const RPC_BUDGET: Duration = Duration::from_secs(5);
const MAX_FRAME: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum NativeStatus {
    Completed,
    Interrupted,
    Failed,
    InProgress,
}
/// Deliberately narrow, version-bound projection of generated Turn schema.
/// Unknown item variants remain opaque; item content is never settlement proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub id: String,
    pub status: NativeStatus,
    pub items: Vec<Value>,
    #[serde(default)]
    pub error: Option<Value>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TurnNotice {
    pub thread_id: String,
    pub turn: Turn,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationState {
    Pending,
    Completed,
    Failed,
    Interrupted,
    Unknown,
    Conflicted,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterruptState {
    Entered,
    Acknowledged,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeProcess {
    pub generation: u64,
    pub pid: u32,
    pub exit: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextReceipt {
    pub effect: EffectId,
    pub delivery: DeliveryId,
    pub context: ContextId,
    pub epoch: Epoch,
    pub request_digest: String,
    pub profile_digest: String,
    pub thread_id: String,
    pub turn_id: String,
    pub final_item_id: String,
    pub output: String,
    pub output_digest: String,
    pub full_turn_digest: String,
    pub package_revision: Option<Revision>,
    pub package_digest: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub effect: EffectId,
    pub request_digest: String,
    pub input_digest: String,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub start_rpc_id: Option<u64>,
    pub state: ObservationState,
    pub terminals: Vec<TurnNotice>,
    pub activity: Vec<Value>,
    pub notes: Vec<String>,
    pub processes: Vec<NativeProcess>,
    pub interrupt: Option<InterruptState>,
    pub context: ContextId,
    pub epoch: Epoch,
    pub delivery: DeliveryId,
    pub purpose: EffectPurpose,
    pub profile_digest: String,
    pub full_turn: Option<Value>,
    pub text_receipt: Option<TextReceipt>,
    pub text_violation: Option<String>,
    #[serde(default)]
    pub readback_attempted: bool,
}
impl Observation {
    fn new(request: &ProviderRequest, process: NativeProcess) -> Self {
        Self {
            effect: request.effect.clone(),
            request_digest: digest(request),
            input_digest: request.input_digest.clone(),
            thread_id: None,
            turn_id: None,
            start_rpc_id: None,
            state: ObservationState::Unknown,
            terminals: vec![],
            activity: vec![],
            notes: vec![],
            processes: vec![process],
            interrupt: None,
            context: request.context.clone(),
            epoch: request.epoch,
            delivery: request.delivery.clone(),
            purpose: request.purpose.clone(),
            profile_digest: request.profile_digest.clone(),
            full_turn: None,
            text_receipt: None,
            text_violation: None,
            readback_attempted: false,
        }
    }
    fn note(&mut self, note: impl Into<String>) {
        unique(&mut self.notes, note.into());
    }
    fn terminal(&mut self, notice: TurnNotice) -> Result<()> {
        ensure!(
            self.thread_id.as_ref() == Some(&notice.thread_id)
                && self.turn_id.as_ref() == Some(&notice.turn.id),
            "native identity mismatch"
        );
        ensure!(
            notice.turn.status != NativeStatus::InProgress,
            "shape-valid inProgress is not terminal evidence"
        );
        unique(&mut self.terminals, notice);
        let content_conflict = self.terminals.iter().enumerate().any(|(i, a)| {
            self.terminals.iter().skip(i + 1).any(|b| {
                a.turn.items.iter().any(|left| {
                    b.turn.items.iter().any(|right| {
                        left["id"].as_str().is_some()
                            && left["id"] == right["id"]
                            && left["type"] == right["type"]
                            && left["text"].as_str().is_some()
                            && right["text"].as_str().is_some()
                            && left["text"] != right["text"]
                    })
                })
            })
        });
        self.state = if content_conflict
            || self
                .terminals
                .iter()
                .any(|n| n.turn.status != self.terminals[0].turn.status)
        {
            ObservationState::Conflicted
        } else {
            match self.terminals[0].turn.status {
                NativeStatus::Completed => ObservationState::Completed,
                NativeStatus::Failed => ObservationState::Failed,
                NativeStatus::Interrupted => ObservationState::Interrupted,
                NativeStatus::InProgress => unreachable!(),
            }
        };
        Ok(())
    }
}
fn completed_item_conflict(left: &Value, right: &Value) -> bool {
    let (Some(a), Some(b)) = (left["items"].as_array(), right["items"].as_array()) else {
        return true;
    };
    a.iter().any(|first| {
        b.iter().any(|second| {
            first["id"].as_str().is_some()
                && first["id"] == second["id"]
                && first["type"] == second["type"]
                && first["text"].as_str().is_some()
                && second["text"].as_str().is_some()
                && first["text"] != second["text"]
        })
    })
}
fn validate_text_activity(
    observation: &Observation,
    user_text: &str,
    output: &str,
) -> std::result::Result<(), String> {
    let mut lifecycle: BTreeMap<String, (String, bool, Option<Value>)> = BTreeMap::new();
    let mut deltas: Vec<(&str, &Value)> = vec![];
    let mut agent_ids = vec![];
    let mut user_ids = vec![];
    for frame in &observation.activity {
        let method = frame["method"]
            .as_str()
            .ok_or("UnsupportedActivity: method missing")?;
        let params = &frame["params"];
        if params["threadId"].as_str() != observation.thread_id.as_deref()
            || params["turnId"].as_str() != observation.turn_id.as_deref()
        {
            return Err("native activity identity mismatch".into());
        }
        match method {
            "item/started" | "item/completed" => {
                let item = &params["item"];
                let id = item["id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .ok_or("native lifecycle item identity missing")?;
                let kind = item["type"]
                    .as_str()
                    .ok_or("native lifecycle item type missing")?;
                if !matches!(kind, "userMessage" | "agentMessage" | "reasoning") {
                    return Err("UnsupportedActivity: native lifecycle item kind".into());
                }
                if kind == "agentMessage"
                    && (!item["questions"].is_null() || item["delivery"] == "async")
                {
                    return Err("UserInputRequested: native item activity".into());
                }
                if kind == "userMessage" {
                    if item["clientId"] != observation.effect.as_str()
                        || item["content"][0]["text"] != user_text
                    {
                        return Err("native user lifecycle content mismatch".into());
                    }
                    unique(&mut user_ids, id.to_owned());
                }
                if kind == "agentMessage" {
                    unique(&mut agent_ids, id.to_owned());
                    if method == "item/completed" && item["text"] != output {
                        return Err("native final lifecycle content mismatch".into());
                    }
                }
                let record =
                    lifecycle
                        .entry(id.to_owned())
                        .or_insert((kind.to_owned(), false, None));
                if record.0 != kind {
                    return Err("native lifecycle item kind conflict".into());
                }
                if method == "item/started" {
                    record.1 = true;
                }
                if method == "item/completed" {
                    if record.2.as_ref().is_some_and(|prior| prior != item) {
                        return Err("native finalized lifecycle content conflict".into());
                    }
                    record.2 = Some(item.clone());
                }
            }
            "item/agentMessage/delta"
            | "item/reasoning/textDelta"
            | "item/reasoning/summaryTextDelta"
            | "item/reasoning/summaryPartAdded" => {
                let id = params["itemId"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .ok_or("native delta item identity missing")?;
                if method != "item/reasoning/summaryPartAdded" && !params["delta"].is_string() {
                    return Err("native text delta missing".into());
                }
                if method == "item/reasoning/textDelta" && !params["contentIndex"].is_i64() {
                    return Err("native reasoning content index missing".into());
                }
                if matches!(
                    method,
                    "item/reasoning/summaryTextDelta" | "item/reasoning/summaryPartAdded"
                ) && !params["summaryIndex"].is_i64()
                {
                    return Err("native reasoning summary index missing".into());
                }
                let _ = id;
                deltas.push((method, params));
            }
            _ => return Err("UnsupportedActivity: native item method".into()),
        }
    }
    if user_ids.len() > 1 || agent_ids.len() > 1 {
        return Err("multiple native user or final agent items".into());
    }
    if lifecycle
        .values()
        .any(|(_, started, completed)| !started || completed.is_none())
    {
        return Err("incomplete native item lifecycle".into());
    }
    for (method, params) in deltas {
        let id = params["itemId"].as_str().expect("validated delta id");
        let Some((kind, started, completed)) = lifecycle.get(id) else {
            return Err("native delta has no exact lifecycle item".into());
        };
        if !started
            || completed.is_none()
            || (method == "item/agentMessage/delta" && kind != "agentMessage")
            || (method != "item/agentMessage/delta" && kind != "reasoning")
        {
            return Err("native delta lifecycle kind/status mismatch".into());
        }
    }
    Ok(())
}

pub(crate) fn text_receipt(
    observation: &Observation,
    full: &Value,
) -> std::result::Result<TextReceipt, String> {
    if observation.state != ObservationState::Completed
        || full["status"] != "completed"
        || full["itemsView"] != "full"
        || !full["error"].is_null()
    {
        return Err("complete exact native item view unavailable".into());
    }
    if !observation.notes.is_empty() || observation.interrupt.is_some() {
        return Err("native transport/request uncertainty or interruption".into());
    }
    let items = full["items"]
        .as_array()
        .ok_or_else(|| "full native items absent".to_string())?;
    if items.iter().any(|item| {
        item["type"] == "agentMessage"
            && (!item["questions"].is_null() || item["delivery"] == "async")
    }) {
        return Err("UserInputRequested: exact native item view".into());
    }
    if items.len() < 2 {
        return Err("incomplete exact native item sequence".into());
    }
    let user = &items[0];
    let agent = items.last().expect("at least two items");
    if items[1..items.len() - 1].iter().any(|item| {
        item["type"] != "reasoning"
            || item["id"].as_str().is_none_or(str::is_empty)
            || ["content", "summary"].iter().any(|field| {
                !item[*field].is_null()
                    && !item[*field]
                        .as_array()
                        .is_some_and(|parts| parts.iter().all(Value::is_string))
            })
    }) {
        return Err("UnsupportedActivity: non-reasoning intermediate item".into());
    }
    if user["type"] != "userMessage" || user["clientId"] != observation.effect.as_str() {
        return Err("native user item identity mismatch".into());
    }
    let user_text = user["content"][0]["text"]
        .as_str()
        .ok_or_else(|| "native user text unavailable".to_string())?;
    if digest(&user_text.as_bytes().to_vec()) != observation.input_digest {
        return Err("native user text digest mismatch".into());
    }
    if agent["type"] != "agentMessage"
        || !agent["questions"].is_null()
        || agent["delivery"] == "async"
    {
        return Err("UserInputRequested or unsupported final native item".into());
    }
    let output = agent["text"]
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| "incomplete final text output".to_string())?;
    let final_item_id = agent["id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "missing final item identity".to_string())?;
    validate_text_activity(observation, user_text, output)?;
    let (package_revision, package_digest) = match &observation.purpose {
        EffectPurpose::Input => (None, None),
        EffectPurpose::Readiness {
            package_revision,
            package_digest,
            expected_ack,
            ..
        } => {
            if output != expected_ack {
                return Err("readiness acknowledgment mismatch".into());
            }
            (Some(*package_revision), Some(package_digest.clone()))
        }
    };
    Ok(TextReceipt {
        effect: observation.effect.clone(),
        delivery: observation.delivery.clone(),
        context: observation.context.clone(),
        epoch: observation.epoch,
        request_digest: observation.request_digest.clone(),
        profile_digest: observation.profile_digest.clone(),
        thread_id: observation
            .thread_id
            .clone()
            .ok_or_else(|| "missing native thread".to_string())?,
        turn_id: observation
            .turn_id
            .clone()
            .ok_or_else(|| "missing native turn".to_string())?,
        final_item_id: final_item_id.to_owned(),
        output: output.to_owned(),
        output_digest: digest(&output),
        full_turn_digest: digest(full),
        package_revision,
        package_digest,
    })
}
fn unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Endpoint {
    Fixture { scenario: String },
    Loopback(SocketAddr),
    PreparedLive,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Launch {
    pub executable: PathBuf,
    pub home: PathBuf,
    pub endpoint: Endpoint,
}

#[derive(Debug, Serialize)]
pub struct LivePreflight {
    pub native_version: &'static str,
    pub native_sha256: &'static str,
    pub profile_digest: String,
    pub effective_config_digest: String,
    pub account_type: String,
    pub plan_type: Option<String>,
    pub model: String,
    pub model_id: String,
    pub model_catalog_digest: String,
    pub input_modalities: Value,
    pub supported_reasoning_efforts: Value,
    pub catalog_source: &'static str,
    pub inference_turns: u8,
}
impl Launch {
    fn expected_text_profile_digest(&self) -> Option<String> {
        match &self.endpoint {
            Endpoint::Loopback(_) => profile_digest_for(&Mode::CodexText),
            Endpoint::PreparedLive => profile_digest_for(&Mode::CodexTextLive),
            Endpoint::Fixture { .. } => None,
        }
    }
    fn prepare(&self) -> Result<()> {
        ensure!(
            self.home.is_absolute() && self.executable.is_absolute(),
            "absolute owned paths required"
        );
        if matches!(self.endpoint, Endpoint::PreparedLive) {
            ensure!(
                self.executable == Path::new(NATIVE_BINARY),
                "pinned native binary required"
            );
            ensure!(
                format!("{:x}", Sha256::digest(fs::read(&self.executable)?)) == NATIVE_SHA256,
                "pinned native hash mismatch"
            );
            ensure!(
                !Path::new("/etc/codex/managed_config.toml").exists()
                    && !Path::new("/etc/codex/requirements.toml").exists(),
                "unreviewed system Codex configuration"
            );
            ensure!(
                fs::read(self.home.join("experiment-owned"))? == LIVE_MARKER,
                "prepared-live profile marker mismatch"
            );
            ensure!(
                fs::read_to_string(self.home.join("codex/config.toml"))? == LIVE_PROFILE_CONFIG,
                "prepared-live profile config mismatch"
            );
            ensure!(
                fs::read(self.home.join("work/.context-lifecycle-project"))?
                    == b"isolated project root\n",
                "prepared-live work root mismatch"
            );
            ensure!(
                self.home.join("user").is_dir() && self.home.join("work").is_dir(),
                "prepared-live directories missing"
            );
            return Ok(());
        }
        if self.home.exists() && !self.home.join("experiment-owned").exists() {
            ensure!(
                fs::read_dir(&self.home)?.next().is_none(),
                "refusing existing unowned state directory"
            );
        }
        fs::create_dir_all(self.home.join("codex"))?;
        fs::create_dir_all(self.home.join("user"))?;
        fs::create_dir_all(self.home.join("work"))?;
        ensure!(
            !self.home.join("codex/auth.json").exists(),
            "authentication file forbidden in offline profile"
        );
        fs::write(
            self.home.join("experiment-owned"),
            b"context-lifecycle-rust offline adapter\n",
        )?;
        if let Endpoint::Loopback(address) = self.endpoint {
            ensure!(
                !std::path::Path::new("/etc/codex/managed_config.toml").exists()
                    && !std::path::Path::new("/etc/codex/requirements.toml").exists(),
                "unreviewed system Codex configuration prevents the isolated probe"
            );
            ensure!(
                address.ip().is_loopback() && address.port() != 0,
                "inference endpoint must be loopback"
            );
            ensure!(
                self.executable == std::path::Path::new(NATIVE_BINARY),
                "installed probe requires pinned native executable"
            );
            let hash = format!("{:x}", Sha256::digest(fs::read(&self.executable)?));
            ensure!(
                hash == NATIVE_SHA256,
                "installed native binary differs from pinned contract"
            );
            let config = format!(
                r#"model = "gpt-6-sol"
model_provider = "local_probe"
approval_policy = "never"
sandbox_mode = "read-only"
web_search = "disabled"
project_doc_max_bytes = 0
project_root_markers = [".context-lifecycle-project"]
[analytics]
enabled = false
[features]
shell_tool = false
multi_agent = false
hooks = false
goals = false
code_mode_host = false
[agents]
enabled = false
[apps._default]
enabled = false
[model_providers.local_probe]
name = "Isolated loopback contract probe"
base_url = "http://{address}/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
"#
            );
            fs::write(
                self.home.join("work/.context-lifecycle-project"),
                b"isolated project root\n",
            )?;
            fs::write(self.home.join("codex/config.toml"), config)?;
        }
        Ok(())
    }
}

struct Session {
    child: Child,
    stdin: ChildStdin,
    frames: mpsc::Receiver<std::result::Result<Value, String>>,
    buffered: Vec<Value>,
    next_id: u64,
    fault: Option<String>,
    denied_requests: Vec<String>,
    trace: Option<Arc<Mutex<(File, usize)>>>,
}
fn trace_frame(
    trace: &Option<Arc<Mutex<(File, usize)>>>,
    direction: &str,
    bytes: &[u8],
) -> Result<()> {
    if let Some(trace) = trace {
        let raw = std::str::from_utf8(bytes)?;
        let record = serde_json::to_vec(&json!({"direction":direction,"raw":raw}))?;
        let mut guard = trace
            .lock()
            .map_err(|_| anyhow::anyhow!("native trace lock poisoned"))?;
        ensure!(
            guard.1 + record.len() < 16 * 1024 * 1024,
            "bounded native trace exhausted"
        );
        guard.0.write_all(&record)?;
        guard.0.write_all(b"\n")?;
        guard.0.flush()?;
        guard.1 += record.len() + 1;
    }
    Ok(())
}
impl Session {
    fn spawn(launch: &Launch) -> Result<Self> {
        let trace = if matches!(launch.endpoint, Endpoint::Loopback(_)) {
            Some(Arc::new(Mutex::new((
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(launch.home.join("native-stdio.jsonl"))?,
                0,
            ))))
        } else {
            None
        };
        let mut command = ProcessCommand::new(&launch.executable);
        command
            .env_clear()
            .env("HOME", launch.home.join("user"))
            .env("CODEX_HOME", launch.home.join("codex"))
            .env("PATH", "/usr/bin:/bin")
            .env("RUST_LOG", "error")
            .current_dir(launch.home.join("work"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0);
        match &launch.endpoint {
            Endpoint::Fixture { scenario } => {
                command.arg(scenario).arg(&launch.home);
            }
            Endpoint::Loopback(_) | Endpoint::PreparedLive => {
                command.args(["app-server", "--strict-config", "--listen", "stdio://"]);
            }
        }
        // Linux-only experiment: also terminate the owned child if its adapter is SIGKILLed.
        let parent = std::process::id() as libc::pid_t;
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    return Err(std::io::Error::other("adapter exited during spawn"));
                }
                Ok(())
            });
        }
        let mut child = command.spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let (send, frames) = mpsc::channel();
        let reader_trace = trace.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = vec![];
                let parsed = (|| -> Result<Value> {
                    (&mut reader)
                        .take(MAX_FRAME as u64 + 1)
                        .read_until(b'\n', &mut line)?;
                    ensure!(!line.is_empty(), "app-server stdout EOF");
                    ensure!(
                        line.len() <= MAX_FRAME && line.last() == Some(&b'\n'),
                        "malformed/oversized app-server frame"
                    );
                    trace_frame(&reader_trace, "native_to_adapter", &line)?;
                    Ok(serde_json::from_slice(&line)?)
                })()
                .map_err(|error| error.to_string());
                let failed = parsed.is_err();
                if send.send(parsed).is_err() || failed {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            frames,
            buffered: vec![],
            next_id: 1,
            fault: None,
            denied_requests: vec![],
            trace,
        })
    }
    fn reserve(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    fn write(&mut self, value: &Value) -> Result<()> {
        let mut bytes = serde_json::to_vec(value)?;
        ensure!(
            bytes.len() < 4096,
            "offline adapter request exceeds bounded pipe frame"
        );
        bytes.push(b'\n');
        trace_frame(&self.trace, "adapter_to_native", &bytes)?;
        self.stdin.write_all(&bytes)?;
        self.stdin.flush()?;
        Ok(())
    }
    fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.reserve();
        self.call_id(id, method, params)
    }
    fn call_id(&mut self, id: u64, method: &str, params: Value) -> Result<Value> {
        ensure!(self.fault.is_none(), "native transport unavailable");
        self.write(&json!({"id":id,"method":method,"params":params}))?;
        let until = Instant::now() + RPC_BUDGET;
        loop {
            let frame = self
                .frames
                .recv_timeout(until.saturating_duration_since(Instant::now()))?
                .map_err(anyhow::Error::msg)?;
            if frame.get("method").is_some() {
                if self.deny_server_request(&frame)? {
                    continue;
                }
                ensure!(
                    self.buffered.len() < 128,
                    "native notification buffer exhausted"
                );
                self.buffered.push(frame);
            } else {
                ensure!(
                    frame.get("id") == Some(&json!(id)),
                    "unexpected JSON-RPC response identity"
                );
                if let Some(error) = frame.get("error") {
                    bail!("native request rejected: {error}");
                }
                return frame
                    .get("result")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("missing native result"));
            }
        }
    }
    fn deny_server_request(&mut self, frame: &Value) -> Result<bool> {
        let (Some(method), Some(id)) = (frame["method"].as_str(), frame.get("id")) else {
            return Ok(false);
        };
        // A native server request never receives human input or tool authority here.
        self.write(&json!({"id":id,"error":{"code":-32601,"message":"observer adapter does not grant server-request authority"}}))?;
        ensure!(
            self.denied_requests.len() < 128,
            "native server-request denial log exhausted"
        );
        self.denied_requests
            .push(format!("server request {method} denied at rpc id {id}"));
        Ok(true)
    }
    fn take_denials(&mut self) -> Vec<String> {
        std::mem::take(&mut self.denied_requests)
    }
    fn drain(&mut self) -> Vec<Value> {
        loop {
            match self.frames.try_recv() {
                Ok(Ok(frame)) => {
                    match self.deny_server_request(&frame) {
                        Ok(true) => continue,
                        Err(error) => {
                            self.fault =
                                Some(format!("native server-request denial failed: {error}"));
                            break;
                        }
                        Ok(false) => {}
                    }
                    if self.buffered.len() >= 128 {
                        self.fault = Some("native notification buffer exhausted".into());
                        break;
                    }
                    self.buffered.push(frame);
                }
                Ok(Err(error)) => {
                    self.fault = Some(error);
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.fault
                        .get_or_insert("native reader disconnected".into());
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        std::mem::take(&mut self.buffered)
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        // Own the local process group; this does not establish external-effect settlement.
        unsafe {
            libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn initialize_session(session: &mut Session, launch: &Launch) -> Result<()> {
    let initialized = session.call("initialize", json!({"clientInfo":{"name":"context_lifecycle_rust","version":"0.1.0"},"capabilities":{"experimentalApi":true}}))?;
    ensure!(
        initialized["userAgent"]
            .as_str()
            .is_some_and(|agent| agent.contains(CLI_VERSION)),
        "app-server version is outside the pinned contract"
    );
    ensure!(
        initialized["codexHome"].as_str() == launch.home.join("codex").to_str(),
        "unexpected native state directory"
    );
    session.write(&json!({"method":"initialized","params":{}}))?;
    Ok(())
}

fn inspect_live_session(session: &mut Session) -> Result<LivePreflight> {
    let effective = session.call("config/read", json!({"includeLayers":false}))?;
    let config = &effective["config"];
    ensure!(
        config["model_provider"] == "openai" && config["model"] == "gpt-6-sol",
        "authenticated model/provider config mismatch"
    );
    ensure!(
        config["approval_policy"] == "never" && config["sandbox_mode"] == "read-only",
        "authenticated permission config mismatch"
    );
    ensure!(
        config["features"]["goals"] == false
            && config["features"]["code_mode_host"] == false
            && config["features"]["shell_tool"] == false
            && config["features"]["multi_agent"] == false
            && config["features"]["hooks"] == false
            && config["web_search"] == "disabled"
            && config["agents"]["enabled"] == false
            && config["apps"]["_default"]["enabled"] == false,
        "authenticated capability config mismatch"
    );
    let account = session.call("account/read", json!({"refreshToken":false}))?;
    ensure!(
        account["requiresOpenaiAuth"] == true && account["account"]["type"] == "chatgpt",
        "prepared profile is not an authenticated ChatGPT account"
    );
    let account_type = account["account"]["type"]
        .as_str()
        .expect("checked type")
        .to_owned();
    let plan_type = account["account"]["planType"].as_str().map(str::to_owned);
    let mut cursor: Option<String> = None;
    let mut selected: Option<Value> = None;
    for _ in 0..10 {
        let response = session.call(
            "model/list",
            json!({"cursor":cursor,"includeHidden":false,"limit":100}),
        )?;
        let models = response["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("native model catalog data missing"))?;
        for model in models
            .iter()
            .filter(|model| model["model"] == "gpt-6-sol" && model["hidden"] == false)
        {
            ensure!(
                selected.as_ref().is_none_or(|prior| prior == model),
                "contradictory gpt-6-sol catalog entries"
            );
            selected = Some(model.clone());
        }
        cursor = response["nextCursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    ensure!(
        cursor.is_none(),
        "native model catalog pagination exceeded bound"
    );
    let selected = selected.ok_or_else(|| {
        anyhow::anyhow!("gpt-6-sol absent from authenticated native model catalog")
    })?;
    let model_id = selected["id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow::anyhow!("native model id absent"))?
        .to_owned();
    Ok(LivePreflight {
        native_version: CLI_VERSION,
        native_sha256: NATIVE_SHA256,
        profile_digest: profile_digest_for(&Mode::CodexTextLive).expect("live digest"),
        effective_config_digest: digest(config),
        account_type,
        plan_type,
        model: "gpt-6-sol".into(),
        model_id,
        model_catalog_digest: digest(&selected),
        input_modalities: selected["inputModalities"].clone(),
        supported_reasoning_efforts: selected["supportedReasoningEfforts"].clone(),
        catalog_source: "authenticated pinned native model/list",
        inference_turns: 0,
    })
}

/// Parent-only read-only preflight. It neither prints account email nor sends a turn.
pub fn preflight_live_profile(home: &Path) -> Result<LivePreflight> {
    let launch = Launch {
        executable: PathBuf::from(NATIVE_BINARY),
        home: home.to_path_buf(),
        endpoint: Endpoint::PreparedLive,
    };
    launch.prepare()?;
    let mut session = Session::spawn(&launch)?;
    initialize_session(&mut session, &launch)?;
    inspect_live_session(&mut session)
}

type RecordedCommand = (ProviderCommand, std::result::Result<ProviderReply, String>);
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ThreadBinding {
    context: ContextId,
    epoch: Epoch,
    profile_digest: String,
    thread_id: Option<String>,
}
#[derive(Default, Clone, Serialize, Deserialize)]
struct Ledger {
    generation: u64,
    records: BTreeMap<EffectId, Observation>,
    commands: BTreeMap<CommandId, RecordedCommand>,
    unassociated: Vec<Value>,
    #[serde(default)]
    bindings: BTreeMap<String, ThreadBinding>,
}
pub struct Adapter {
    store: Store,
    ledger: Ledger,
    session: Session,
    launch: Launch,
}
impl Adapter {
    pub fn open(launch: Launch) -> Result<Self> {
        launch.prepare()?;
        let mut store = Store::open(&launch.home.join("adapter.db"))?;
        let mut ledger: Ledger = store.load()?;
        ledger.generation += 1;
        store.save(&ledger)?;
        let mut session = Session::spawn(&launch)?;
        initialize_session(&mut session, &launch)?;
        if matches!(launch.endpoint, Endpoint::PreparedLive) {
            let _ = inspect_live_session(&mut session)?;
        } else if matches!(launch.endpoint, Endpoint::Loopback(_)) {
            let effective = session.call("config/read", json!({"includeLayers":false}))?;
            ensure!(
                effective["config"]["model_provider"] == "local_probe",
                "effective provider differs from isolated profile"
            );
            ensure!(
                effective["config"]["approval_policy"] == "never"
                    && effective["config"]["sandbox_mode"] == "read-only",
                "effective permission profile differs"
            );
            ensure!(
                effective["config"]["model"] == "gpt-6-sol"
                    && effective["config"]["features"]["goals"] == false
                    && effective["config"]["features"]["code_mode_host"] == false
                    && effective["config"]["features"]["shell_tool"] == false
                    && effective["config"]["features"]["multi_agent"] == false
                    && effective["config"]["features"]["hooks"] == false
                    && effective["config"]["web_search"] == "disabled"
                    && effective["config"]["agents"]["enabled"] == false
                    && effective["config"]["apps"]["_default"]["enabled"] == false,
                "effective model/goals profile differs"
            );
        }
        let mut adapter = Self {
            store,
            ledger,
            session,
            launch,
        };
        // Readback may add positive exact-ID observations; missing records are never absence proof.
        let ids: Vec<_> = adapter.ledger.records.keys().cloned().collect();
        for id in ids {
            adapter.readback(&id)?;
        }
        adapter.refresh()?;
        Ok(adapter)
    }
    fn process(&self) -> NativeProcess {
        NativeProcess {
            generation: self.ledger.generation,
            pid: self.session.child.id(),
            exit: None,
        }
    }
    fn persist(&mut self) -> Result<()> {
        self.store.save(&self.ledger)
    }
    fn unknown(&mut self, id: &EffectId, error: impl ToString) {
        let observation = self.ledger.records.get_mut(id).expect("record");
        observation.note(error.to_string());
        if observation.terminals.is_empty() {
            observation.state = ObservationState::Unknown;
        }
    }
    fn start(&mut self, request: &ProviderRequest) -> Result<()> {
        let id = &request.effect;
        let result = (|| -> Result<()> {
            if matches!(self.launch.endpoint, Endpoint::PreparedLive) {
                ensure!(
                    request.authority.mode == Mode::CodexTextLive,
                    "prepared-live endpoint requires explicit live text authority"
                );
            }
            let text = String::from_utf8(request.payload.clone())?;
            ensure!(
                text.len() <= 2048,
                "native observation adapter accepts at most 2048 UTF-8 bytes"
            );
            let thread = if matches!(
                request.authority.mode,
                Mode::CodexText | Mode::CodexTextLive
            ) {
                ensure!(
                    profile_digest_for(&request.authority.mode).as_deref()
                        == Some(request.profile_digest.as_str())
                        && matches!(
                            (&self.launch.endpoint, &request.authority.mode),
                            (
                                Endpoint::Loopback(_) | Endpoint::Fixture { .. },
                                Mode::CodexText
                            ) | (Endpoint::PreparedLive, Mode::CodexTextLive)
                        ),
                    "native text profile mismatch"
                );
                let model_provider = if request.authority.mode == Mode::CodexTextLive {
                    "openai"
                } else {
                    "local_probe"
                };
                let key = digest(&json!([request.context, request.epoch]));
                if let Some(binding) = self.ledger.bindings.get(&key) {
                    ensure!(
                        binding.context == request.context
                            && binding.epoch == request.epoch
                            && binding.profile_digest == request.profile_digest,
                        "native generation binding conflict"
                    );
                    binding.thread_id.clone().ok_or_else(|| {
                        anyhow::anyhow!(
                            "ambiguous native thread creation intent; cannot create again"
                        )
                    })?
                } else {
                    self.ledger.bindings.insert(
                        key.clone(),
                        ThreadBinding {
                            context: request.context.clone(),
                            epoch: request.epoch,
                            profile_digest: request.profile_digest.clone(),
                            thread_id: None,
                        },
                    );
                    self.persist()?; // Creation intent precedes thread/start bytes.
                    let started = self.session.call("thread/start", json!({"cwd":self.launch.home.join("work"),"model":"gpt-6-sol","modelProvider":model_provider,"approvalPolicy":"never","sandbox":"read-only","environments":[],"dynamicTools":[],"ephemeral":false,"historyMode":"legacy"}))?;
                    let thread = started["thread"]["id"]
                        .as_str()
                        .filter(|id| !id.is_empty())
                        .ok_or_else(|| anyhow::anyhow!("missing native thread identity"))?
                        .to_owned();
                    ensure!(
                        started["thread"]["ephemeral"] == false,
                        "native thread must persist"
                    );
                    self.ledger
                        .bindings
                        .get_mut(&key)
                        .expect("intent")
                        .thread_id = Some(thread.clone());
                    self.persist()?; // Native identity precedes turn/start.
                    thread
                }
            } else {
                let started = self.session.call("thread/start", json!({"cwd":self.launch.home.join("work"),"model":"gpt-6-sol","modelProvider":"local_probe","approvalPolicy":"never","sandbox":"read-only","environments":[],"dynamicTools":[],"ephemeral":false,"historyMode":"legacy"}))?;
                let thread = started["thread"]["id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("missing native thread identity"))?
                    .to_owned();
                ensure!(
                    started["thread"]["ephemeral"] == false,
                    "native thread must persist"
                );
                thread
            };
            let rpc_id = self.session.reserve();
            let observation = self.ledger.records.get_mut(id).expect("record");
            observation.thread_id = Some(thread.clone());
            observation.start_rpc_id = Some(rpc_id);
            self.persist()?; // Correlation is durable before turn/start bytes leave this process.
            let response = self.session.call_id(rpc_id, "turn/start", json!({"threadId":thread,"clientUserMessageId":id.as_str(),"input":[{"type":"text","text":text}],"environments":[]}))?;
            let turn: Turn = serde_json::from_value(response["turn"].clone())?;
            ensure!(!turn.id.is_empty(), "empty native turn identity");
            let observation = self.ledger.records.get_mut(id).expect("record");
            observation.turn_id = Some(turn.id);
            observation.state = ObservationState::Pending;
            self.persist()?;
            Ok(())
        })();
        if let Err(error) = result {
            self.unknown(id, format!("turn start uncertainty: {error}"));
        }
        self.refresh()?;
        self.persist()
    }
    fn readback(&mut self, id: &EffectId) -> Result<()> {
        let process = self.process();
        let observation = self.ledger.records.get_mut(id).expect("record");
        if observation
            .processes
            .last()
            .is_none_or(|last| last.generation != process.generation)
        {
            observation.processes.push(process);
        }
        if observation.interrupt == Some(InterruptState::Entered) {
            observation.interrupt = Some(InterruptState::Unknown);
        }
        let (Some(thread), Some(turn)) =
            (observation.thread_id.clone(), observation.turn_id.clone())
        else {
            self.unknown(
                id,
                "restart has no exact native turn identity; never resending turn/start",
            );
            return self.persist();
        };
        let result = (|| -> Result<()> {
            let value = self.session.call(
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
            )?;
            ensure!(
                value["thread"]["id"] == thread,
                "readback thread identity mismatch"
            );
            ensure!(
                value["thread"]["historyMode"].is_null()
                    || value["thread"]["historyMode"] == "legacy",
                "unsupported history contract; no absence inference"
            );
            let turns = value["thread"]["turns"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("readback turns missing"))?;
            let exact: Vec<_> = turns
                .iter()
                .filter(|candidate| candidate["id"] == turn)
                .collect();
            ensure!(
                !exact.is_empty(),
                "exact native turn unavailable; not absence proof"
            );
            for candidate in exact {
                let turn: Turn = serde_json::from_value(candidate.clone())?;
                if turn.status != NativeStatus::InProgress {
                    let observation = self.ledger.records.get_mut(id).expect("record");
                    observation.terminal(TurnNotice {
                        thread_id: thread.clone(),
                        turn,
                    })?;
                    if self.launch.expected_text_profile_digest().as_deref()
                        == Some(observation.profile_digest.as_str())
                    {
                        observation.readback_attempted = true;
                        if candidate["itemsView"] == "full" && candidate["status"] == "completed" {
                            if observation
                                .full_turn
                                .as_ref()
                                .is_some_and(|prior| completed_item_conflict(prior, candidate))
                            {
                                observation.text_violation =
                                    Some("conflict: contradictory finalized native content".into());
                                observation.text_receipt = None;
                            } else if observation.text_violation.is_none() {
                                observation.full_turn = Some(candidate.clone());
                                match text_receipt(observation, candidate) {
                                    Ok(receipt) => observation.text_receipt = Some(receipt),
                                    Err(reason) => observation.text_violation = Some(reason),
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.unknown(id, format!("readback uncertainty: {error}"));
        }
        self.persist()
    }
    pub fn refresh(&mut self) -> Result<()> {
        let before = digest(&self.ledger);
        for frame in self.session.drain() {
            let method = frame["method"].as_str().unwrap_or("");
            if method == "turn/completed" {
                match serde_json::from_value::<TurnNotice>(frame["params"].clone()) {
                    Ok(notice) => {
                        if let Some(observation) = self.ledger.records.values_mut().find(|o| {
                            o.thread_id.as_ref() == Some(&notice.thread_id)
                                && o.turn_id.as_ref() == Some(&notice.turn.id)
                        }) {
                            if let Err(error) = observation.terminal(notice) {
                                observation.note(error.to_string());
                                if observation.terminals.is_empty() {
                                    observation.state = ObservationState::Unknown;
                                }
                            }
                        } else {
                            unique(&mut self.ledger.unassociated, frame);
                        }
                    }
                    Err(error) => {
                        for observation in self.ledger.records.values_mut() {
                            observation.note(format!("malformed terminal notification: {error}"));
                            if observation.terminals.is_empty() {
                                observation.state = ObservationState::Unknown;
                            }
                        }
                    }
                }
            } else if method.starts_with("item/") {
                let params = &frame["params"];
                let mut associated = false;
                for observation in self.ledger.records.values_mut() {
                    if params["threadId"].as_str() == observation.thread_id.as_deref()
                        && params["turnId"].as_str() == observation.turn_id.as_deref()
                    {
                        unique(&mut observation.activity, frame.clone());
                        associated = true;
                    }
                }
                if !associated {
                    unique(&mut self.ledger.unassociated, frame.clone());
                    if params["threadId"].as_str().is_none() || params["turnId"].as_str().is_none()
                    {
                        for observation in self.ledger.records.values_mut() {
                            if self.launch.expected_text_profile_digest().as_deref()
                                == Some(observation.profile_digest.as_str())
                            {
                                observation.note("unidentified native item activity");
                            }
                        }
                    }
                }
            }
        }
        for denial in self.session.take_denials() {
            for observation in self.ledger.records.values_mut() {
                observation.note(denial.clone());
            }
        }
        if self.ledger.unassociated.len() > 128 {
            self.ledger.unassociated.truncate(128);
        }
        let exit = self
            .session
            .child
            .try_wait()?
            .map(|status| status.to_string());
        for observation in self.ledger.records.values_mut() {
            if let Some(error) = &self.session.fault {
                observation.note(error.clone());
                if observation.terminals.is_empty() {
                    observation.state = ObservationState::Unknown;
                }
            }
            if let Some(process) = observation.processes.last_mut() {
                process.exit = exit.clone();
            }
        }
        let readback_ids: Vec<_> = self
            .ledger
            .records
            .iter()
            .filter(|(_, observation)| {
                self.launch.expected_text_profile_digest().as_deref()
                    == Some(observation.profile_digest.as_str())
                    && observation.state == ObservationState::Completed
                    && !observation.readback_attempted
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in readback_ids {
            self.readback(&id)?;
        }
        if digest(&self.ledger) != before {
            self.persist()?;
        }
        Ok(())
    }
    pub fn handle(&mut self, request: ProviderRequestMessage) -> Result<ProviderReply> {
        match request {
            ProviderRequestMessage::Lookup(id) => self
                .ledger
                .records
                .get(&id)
                .cloned()
                .map(ProviderReply::Codex)
                .ok_or_else(|| anyhow::anyhow!("native evidence unavailable")),
            ProviderRequestMessage::Snapshot => bail!("use exact effect observation lookup"),
            ProviderRequestMessage::Command(command) => {
                ensure!(!command.key.as_str().is_empty(), "empty command identity");
                if let Some((prior, reply)) = self.ledger.commands.get(&command.key) {
                    ensure!(*prior == command, "native command key conflict");
                    return reply.clone().map_err(anyhow::Error::msg);
                }
                let journal_command = command.clone();
                let result: Result<ProviderReply> = (|| match &command.action {
                    ProviderAction::Submit(request) => {
                        ensure!(
                            matches!(
                                request.authority.mode,
                                Mode::ObserveCodex | Mode::CodexText | Mode::CodexTextLive
                            ),
                            "explicit native authority required"
                        );
                        if let Some(prior) = self.ledger.records.get(&request.effect) {
                            ensure!(
                                prior.request_digest == digest(request),
                                "effect request conflict"
                            );
                            return Ok(ProviderReply::Codex(prior.clone())); // Never resend, including unknown native ID.
                        }
                        let observation = Observation::new(request, self.process());
                        self.ledger
                            .records
                            .insert(request.effect.clone(), observation.clone());
                        // Entry receipt survives any later stdout/response association failure.
                        self.ledger.commands.insert(
                            command.key.clone(),
                            (command.clone(), Ok(ProviderReply::Codex(observation))),
                        );
                        self.persist()?;
                        self.start(request)?;
                        let reply =
                            ProviderReply::Codex(self.ledger.records[&request.effect].clone());
                        self.ledger
                            .commands
                            .insert(command.key.clone(), (command, Ok(reply.clone())));
                        self.persist()?;
                        Ok(reply)
                    }
                    ProviderAction::Interrupt { effect } => {
                        let observation = self
                            .ledger
                            .records
                            .get_mut(effect)
                            .ok_or_else(|| anyhow::anyhow!("unknown native effect"))?;
                        let (Some(thread), Some(turn)) =
                            (observation.thread_id.clone(), observation.turn_id.clone())
                        else {
                            bail!("interrupt needs exact native IDs")
                        };
                        ensure!(
                            observation.interrupt.is_none(),
                            "interrupt already entered; never automatically retrying"
                        );
                        observation.interrupt = Some(InterruptState::Entered);
                        let reply = ProviderReply::Codex(observation.clone());
                        self.ledger
                            .commands
                            .insert(command.key.clone(), (command.clone(), Ok(reply)));
                        self.persist()?;
                        let result = self
                            .session
                            .call("turn/interrupt", json!({"threadId":thread,"turnId":turn}));
                        let observation = self.ledger.records.get_mut(effect).expect("record");
                        observation.interrupt =
                            Some(if result.as_ref().is_ok_and(Value::is_object) {
                                InterruptState::Acknowledged
                            } else {
                                InterruptState::Unknown
                            });
                        let reply = ProviderReply::Codex(observation.clone());
                        self.ledger
                            .commands
                            .insert(command.key.clone(), (command, Ok(reply.clone())));
                        self.persist()?;
                        Ok(reply)
                    }
                    _ => bail!("Codex observations do not support synthetic settlement controls"),
                })();
                if result.is_err() || !self.ledger.commands.contains_key(&journal_command.key) {
                    self.ledger.commands.insert(
                        journal_command.key.clone(),
                        (
                            journal_command,
                            result.as_ref().cloned().map_err(ToString::to_string),
                        ),
                    );
                    self.persist()?;
                }
                result
            }
        }
    }
}

pub fn run_provider(launch: Launch) -> Result<()> {
    let mut adapter = Adapter::open(launch)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    println!(
        "{}",
        json!({"address":listener.local_addr()?,"pid":std::process::id()})
    );
    std::io::stdout().flush()?;
    loop {
        adapter.refresh()?;
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                stream.set_write_timeout(Some(Duration::from_secs(2)))?;
                let result: std::result::Result<ProviderReply, String> =
                    crate::process::read_message(&stream)
                        .and_then(|r| adapter.handle(r))
                        .map_err(|e| e.to_string());
                let _ = crate::process::write_message(&mut stream, &result);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(3))
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(test)]
mod wire_schema_tests {
    use super::*;

    #[test]
    fn text_delta_requires_exact_lifecycle_and_turn_identity() {
        let mut observation = Observation {
            effect: EffectId::new("effect-1"),
            request_digest: "request".into(),
            input_digest: "input".into(),
            thread_id: Some("thread-1".into()),
            turn_id: Some("turn-1".into()),
            start_rpc_id: None,
            state: ObservationState::Completed,
            terminals: vec![],
            activity: vec![
                json!({"method":"item/started","params":{"threadId":"thread-1","turnId":"turn-1","item":{"id":"answer-1","type":"agentMessage","text":"ok","questions":null,"delivery":null}}}),
                json!({"method":"item/agentMessage/delta","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"wrong-answer","delta":"ok"}}),
                json!({"method":"item/completed","params":{"threadId":"thread-1","turnId":"turn-1","item":{"id":"answer-1","type":"agentMessage","text":"ok","questions":null,"delivery":null}}}),
            ],
            notes: vec![],
            processes: vec![],
            interrupt: None,
            context: ContextId::new("context-1"),
            epoch: Epoch(1),
            delivery: DeliveryId::new("delivery-1"),
            purpose: EffectPurpose::Input,
            profile_digest: String::new(),
            full_turn: None,
            text_receipt: None,
            text_violation: None,
            readback_attempted: false,
        };
        assert!(validate_text_activity(&observation, "input", "ok").is_err());
        observation.activity[1]["params"]["itemId"] = json!("answer-1");
        assert!(validate_text_activity(&observation, "input", "ok").is_ok());
        observation.activity[1]["params"]["turnId"] = json!("other-turn");
        assert!(validate_text_activity(&observation, "input", "ok").is_err());
        observation.activity[1]["params"]["turnId"] = json!("turn-1");
        observation.activity[1]["method"] = json!("item/commandExecution/delta");
        assert!(validate_text_activity(&observation, "input", "ok").is_err());
    }

    #[test]
    fn derived_turn_and_notice_schemas_match_selected_serde_projection() {
        let turn_schema = serde_json::to_value(schemars::schema_for!(Turn)).unwrap();
        let notice_schema = serde_json::to_value(schemars::schema_for!(TurnNotice)).unwrap();
        // Both schemas are generated from the same Rust DTOs used by the adapter.
        let turn_validator = jsonschema::options().offline().build(&turn_schema).unwrap();
        let notice_validator = jsonschema::options()
            .offline()
            .build(&notice_schema)
            .unwrap();

        let valid_turn = json!({
            "id": "turn-1",
            "status": "completed",
            "items": [{"unknownNativeItem": {"any": [1, true, null]}}]
        });
        let valid_notice = json!({"threadId": "thread-1", "turn": valid_turn});
        assert!(turn_validator.is_valid(&valid_notice["turn"]));
        assert!(notice_validator.is_valid(&valid_notice));
        let parsed: TurnNotice = serde_json::from_value(valid_notice.clone()).unwrap();
        let serialized = serde_json::to_value(parsed).unwrap();
        assert!(notice_validator.is_valid(&serialized));
        assert_eq!(serialized["turn"]["items"], valid_notice["turn"]["items"]);
        for status in ["completed", "failed", "interrupted", "inProgress"] {
            let turn = json!({"id": "turn-1", "status": status, "items": [], "error": null});
            assert!(turn_validator.is_valid(&turn), "rejected {status}");
            assert!(serde_json::from_value::<Turn>(turn).is_ok());
        }

        for invalid in [
            json!({"status": "completed", "items": []}),
            json!({"id": 1, "status": "completed", "items": []}),
            json!({"id": "turn-1", "items": []}),
            json!({"id": "turn-1", "status": "unknown", "items": []}),
            json!({"id": "turn-1", "status": "completed", "items": {}}),
        ] {
            assert!(!turn_validator.is_valid(&invalid), "accepted {invalid}");
            assert!(serde_json::from_value::<Turn>(invalid).is_err());
        }
        for invalid in [
            json!({"turn": valid_notice["turn"]}),
            json!({"threadId": 7, "turn": valid_notice["turn"]}),
            json!({"threadId": "thread-1"}),
            json!({"threadId": "thread-1", "turn": {"id": "turn-1"}}),
        ] {
            assert!(!notice_validator.is_valid(&invalid), "accepted {invalid}");
            assert!(serde_json::from_value::<TurnNotice>(invalid).is_err());
        }

        // JSON Schema describes a Turn, not whether a notice is terminal evidence.
        let in_progress = json!({"threadId": "thread-1", "turn": {
            "id": "turn-1", "status": "inProgress", "items": []
        }});
        assert!(notice_validator.is_valid(&in_progress));
        let notice: TurnNotice = serde_json::from_value(in_progress).unwrap();
        let mut observation = Observation {
            effect: EffectId::new("effect-1"),
            request_digest: "digest".into(),
            input_digest: "input".into(),
            thread_id: Some("thread-1".into()),
            turn_id: Some("turn-1".into()),
            start_rpc_id: None,
            state: ObservationState::Unknown,
            terminals: vec![],
            activity: vec![],
            notes: vec![],
            processes: vec![],
            interrupt: None,
            context: ContextId::new("context-1"),
            epoch: Epoch(1),
            delivery: DeliveryId::new("delivery-1"),
            purpose: EffectPurpose::Input,
            profile_digest: String::new(),
            full_turn: None,
            text_receipt: None,
            text_violation: None,
            readback_attempted: false,
        };
        assert!(observation.terminal(notice).is_err());
        assert_eq!(observation.state, ObservationState::Unknown);
        assert!(observation.terminals.is_empty());
    }
}
