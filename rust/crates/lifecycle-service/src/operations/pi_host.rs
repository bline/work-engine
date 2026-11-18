//! Pi Host realization runner. The Host speaks the protocol in
//! `native-realization` `PI_HOST_PROTOCOL.md`; this runner drives it with the
//! same actor messages the Codex runner uses, so turn claims, ingress, tool
//! execution, input retention and settlement stay with the actor and store.
//! No Codex handshake, code-mode setting or Codex identity is used. The Host
//! job (credentials custody, routes, budget, deadline) is admitted by trusted
//! composition before launch; this runner checks it by digest and never
//! holds provider access.

use super::{NativeLaunch, NativeMessage, NativeTaskWork};
use crate::{NativeConfig, NativeFollowupTurn, actor::NativeGate};
use lifecycle_codex::{NativeAuthMode, NativeEvent, NativeEventKind, parse_event};
use lifecycle_runtime::NativeStdio;
use lifecycle_store::{NativeInputPayload, StoreError};
use native_realization::pi_host;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::Ordering};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

/// The store refuses a raw native ingress frame above 1 MiB; the Host keeps
/// every frame, including its `thread/read` view, within that bound.
const MAX_FRAME: usize = 1_048_576;
const MAX_FRAMES: u64 = 100_000;
const LIFECYCLE_PROFILE: &str = "work-engine.pi-lifecycle-hosted.v1";
const SERVE_SCHEMA: &str = "work-engine.pi-host-serve.v1";
/// Container start and Pi session readiness precede the `thread/start` reply.
const THREAD_START_SECS: u64 = 180;
/// The Host's orderly close after end of input (bridge close, collection,
/// container removal).
const CLOSE_SECS: u64 = 120;

/// Selects the Pi Host realization; `NativeConfig::executable` is then the
/// Host binary (`work-engine-pi-host`).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PiHostConfig {
    pub host_sha256: String,
    /// The Host's serve configuration, written by trusted composition; it
    /// names the admitted Host job by digest.
    pub serve_config: PathBuf,
    pub serve_config_sha256: String,
    pub job_sha256: String,
    /// The job's state directory, inside the native root: every capture path
    /// lies in it.
    pub state_dir: PathBuf,
    /// `claude-sdk` (Messages) or `openai-codex` (Responses).
    pub provider: String,
    pub model: String,
    pub turn_deadline_secs: u64,
}

impl PiHostConfig {
    fn wire(&self) -> &'static str {
        if self.provider == "claude-sdk" {
            "messages"
        } else {
            "responses"
        }
    }
}

/// The realization-checked input value the actor retains (C1). Built here
/// from the Host capture, whose payload bytes were re-read and re-digested.
pub struct HostCapturedInput {
    pub transport: String,
    pub terminal_sequence: u64,
    pub terminal_raw: Vec<u8>,
    pub metadata: Vec<u8>,
    pub payloads: Vec<NativeInputPayload>,
    pub tool_registry: Vec<u8>,
    pub eligible: bool,
    pub outcome: String,
    pub causal_bundle: Option<Vec<u8>>,
}

/// What the runner launched; a capture must state exactly these identities.
#[derive(Clone)]
pub(crate) struct Expected {
    pub host_sha256: String,
    pub job_sha256: String,
    pub provider: String,
    pub model: String,
    pub wire: String,
    pub routes: Value,
    pub state_dir: PathBuf,
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn file_sha(path: &Path) -> Result<String, StoreError> {
    std::fs::read(path)
        .map(|b| sha(&b))
        .map_err(|_| StoreError::Rejected)
}

/// Read `<root>/pi-host.json`, written by trusted composition.
pub(crate) fn load(config: &NativeConfig) -> Result<PiHostConfig, StoreError> {
    let path = config.root.join("pi-host.json");
    let meta = std::fs::symlink_metadata(&path).map_err(|_| StoreError::Rejected)?;
    if !meta.is_file() || meta.len() > 1024 * 1024 {
        return Err(StoreError::Rejected);
    }
    serde_json::from_slice(&std::fs::read(&path).map_err(|_| StoreError::Rejected)?)
        .map_err(|_| StoreError::Rejected)
}

pub(crate) fn validate(config: &NativeConfig, host: &PiHostConfig) -> Result<(), StoreError> {
    crate::operations::workspace_snapshot_read::validate(&config.snapshot)?;
    use lifecycle_core::{AttemptId, NativeSessionId};
    NativeSessionId::parse(config.session_id.clone()).map_err(|_| StoreError::Rejected)?;
    AttemptId::parse(config.attempt_id.clone()).map_err(|_| StoreError::Rejected)?;
    let digest = |s: &str| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit());
    // `semantic` configuration (required by the retained profile) is used only
    // by successor transitions, which this realization refuses: fresh start only.
    if config.invocation_id.is_empty()
        || !config.root.is_absolute()
        || !config.executable.is_absolute()
        || !host.serve_config.is_absolute()
        || !host.state_dir.is_absolute()
        || !host.state_dir.starts_with(&config.root)
        || config.followup_turns.len() > 3
        // No Codex provider field may accompany the Pi Host.
        || config.provider_route != crate::NativeProviderRoute::PiHostV1
        || !config.simulator_base_url.is_empty()
        || config.chatgpt_access_source.is_some()
        || !digest(&host.host_sha256)
        || !digest(&host.serve_config_sha256)
        || !digest(&host.job_sha256)
        || !matches!(host.provider.as_str(), "claude-sdk" | "openai-codex")
        || host.model.is_empty()
        || !(1..=3600).contains(&host.turn_deadline_secs)
    {
        return Err(StoreError::Rejected);
    }
    Ok(())
}

/// Session binary identity: the Host executable and the admitted Host job.
pub(crate) fn binary_identity(host: &PiHostConfig) -> String {
    sha(format!("pi-host-binary-v1:{}:{}", host.host_sha256, host.job_sha256).as_bytes())
}

/// The Host job behind the serve configuration, checked by digest against
/// the launch configuration. Returns the job's nonsecret route table.
fn check_job(host: &PiHostConfig) -> Result<Value, StoreError> {
    let serve: Value = serde_json::from_slice(
        &std::fs::read(&host.serve_config).map_err(|_| StoreError::Rejected)?,
    )
    .map_err(|_| StoreError::Rejected)?;
    if file_sha(&host.serve_config)? != host.serve_config_sha256
        || serve["schema"] != SERVE_SCHEMA
        || serve["job"]["sha256"] != host.job_sha256.as_str()
        || serve["turn_deadline_secs"] != host.turn_deadline_secs
    {
        return Err(StoreError::Rejected);
    }
    let job_path = PathBuf::from(serve["job"]["path"].as_str().ok_or(StoreError::Rejected)?);
    let bytes = std::fs::read(&job_path).map_err(|_| StoreError::Rejected)?;
    let job: Value = serde_json::from_slice(&bytes).map_err(|_| StoreError::Rejected)?;
    let plan = &job["coding"]["provider"];
    if sha(&bytes) != host.job_sha256
        || job["coding"]["schema"] != LIFECYCLE_PROFILE
        || plan["provider"] != host.provider.as_str()
        || plan["model"] != host.model.as_str()
        || job["state_dir"].as_str().map(PathBuf::from).as_ref() != Some(&host.state_dir)
    {
        return Err(StoreError::Rejected);
    }
    let routes = plan["hosted"]["routes"].clone();
    let list = routes.as_array().ok_or(StoreError::Rejected)?;
    // A loopback upstream is offline qualification only, gated like the
    // Codex fake route.
    let loopback = list.iter().any(|r| {
        r["endpoint"]
            .as_str()
            .is_none_or(|e| !e.starts_with("https:"))
    });
    if list.is_empty() || (loopback && !cfg!(feature = "controlled-proof")) {
        return Err(StoreError::Rejected);
    }
    Ok(routes)
}

pub(crate) fn prepare_launch(
    config: &NativeConfig,
    uid: u32,
    materialize: bool,
) -> Result<NativeLaunch, StoreError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    // The run root holds the exact request bodies: owner-private, as the
    // Codex launch requires of its root.
    if !config.root.is_absolute() {
        return Err(StoreError::Rejected);
    }
    if materialize {
        std::fs::create_dir_all(&config.root).map_err(|_| StoreError::Unavailable)?;
    }
    let root = std::fs::canonicalize(&config.root).map_err(|_| StoreError::Unavailable)?;
    let metadata = std::fs::symlink_metadata(&root).map_err(|_| StoreError::Unavailable)?;
    if !metadata.is_dir() || metadata.uid() != uid || metadata.permissions().mode() & 0o077 != 0 {
        return Err(StoreError::Rejected);
    }
    let host = &load(config)?;
    validate(config, host)?;
    if file_sha(&config.executable)? != host.host_sha256 {
        return Err(StoreError::Rejected);
    }
    let routes = check_job(host)?;
    let profile = json!({"realization":pi_host::REALIZATION,"protocol":pi_host::PROTOCOL_DIGEST,
        "host":host.host_sha256,"job":host.job_sha256,"serve_config":host.serve_config_sha256,
        "provider":host.provider,"model":host.model,"routes":routes,
        "turn_deadline_secs":host.turn_deadline_secs,"dynamic_tools":pi_host::tools::dynamic_tools()});
    let work = config.root.join("pi-host");
    Ok(NativeLaunch {
        home: work.clone(),
        codex_home: work.clone(),
        work,
        profile_digest: sha(&serde_json::to_vec(&profile).map_err(|_| StoreError::Rejected)?),
        proof_barrier_dir: None,
        provider_name: format!("pi-host-{}", host.provider),
        provider_base_url: String::new(),
        requires_runtime_key: false,
        auth_mode: NativeAuthMode::None,
        chatgpt_access_source: None,
        code_mode_host: false,
        pi_host: Some(host.clone()),
    })
}

struct Runner {
    pipe: NativeStdio,
    transport: String,
    messages: mpsc::Sender<NativeMessage>,
    gate: Arc<NativeGate>,
    next_id: u64,
    bound_turn: bool,
    /// The active turn's receive bound, set before `turn/start` (the Host's
    /// turn begins when it answers) and cleared at the turn's terminal.
    turn_until: Option<Instant>,
    terminal: Option<(NativeEvent, String)>,
    deadline: Duration,
    expected: Expected,
}

impl Runner {
    async fn ask<F>(&self, make: F) -> Result<(), StoreError>
    where
        F: FnOnce(oneshot::Sender<Result<(), StoreError>>) -> NativeMessage,
    {
        let (tx, rx) = oneshot::channel();
        self.messages
            .send(make(tx))
            .await
            .map_err(|_| StoreError::Unavailable)?;
        rx.await.map_err(|_| StoreError::Unavailable)?
    }

    async fn send(&mut self, value: &Value) -> Result<(), StoreError> {
        if self.gate.load(Ordering::SeqCst) {
            return Err(StoreError::Rejected);
        }
        let bytes = serde_json::to_vec(value).map_err(|_| StoreError::Rejected)?;
        self.pipe
            .send(&bytes)
            .await
            .map(|_| ())
            .map_err(|_| StoreError::Unavailable)
    }

    async fn receive(&mut self, until: Instant) -> Result<NativeEvent, StoreError> {
        loop {
            if self.gate.load(Ordering::SeqCst) {
                return Err(StoreError::Rejected);
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(StoreError::Unavailable);
            }
            tokio::select! {
                frame = self.pipe.receive() => {
                    let (sequence, raw) = frame.map_err(|_| StoreError::Unavailable)?.ok_or(StoreError::Unavailable)?;
                    return parse_event(self.transport.clone(), sequence, raw).map_err(|_| StoreError::Rejected);
                }
                _ = tokio::time::sleep(left.min(Duration::from_millis(50))) => {}
            }
        }
    }

    /// Record the frame, serve a tool call through the actor, note a terminal.
    async fn handle(&mut self, event: NativeEvent) -> Result<(), StoreError> {
        let recorded = self
            .ask(|reply| NativeMessage::Ingress {
                event: event.clone(),
                reply,
            })
            .await;
        let terminal = event.terminal().map_err(|_| StoreError::Rejected)?;
        if let Some(t) = &terminal {
            if self.terminal.is_some() {
                return Err(StoreError::ClaimConflict);
            }
            self.terminal = Some((event.clone(), t.status.clone()));
        }
        // A failed terminal is recorded as unsupported; capture still runs.
        if terminal.is_none() {
            recorded?;
        }
        if let NativeEventKind::ServerRequest { id, method } = &event.kind {
            if method != "item/tool/call" || !self.bound_turn {
                self.send(
                    &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":"refused"}}),
                )
                .await?;
                return Err(StoreError::ClaimConflict);
            }
            let call = event
                .tool_call()
                .map_err(|_| StoreError::Rejected)?
                .ok_or(StoreError::Rejected)?;
            let (tx, rx) = oneshot::channel();
            self.messages
                .send(NativeMessage::Tool {
                    event: event.clone(),
                    call: call.clone(),
                    reply: tx,
                })
                .await
                .map_err(|_| StoreError::Unavailable)?;
            match rx.await.map_err(|_| StoreError::Unavailable)? {
                Ok(result) => {
                    self.send(&json!({"jsonrpc":"2.0","id":call.rpc_id,"result":result}))
                        .await?
                }
                Err(error) => {
                    let _ = self
                        .send(&json!({"jsonrpc":"2.0","id":call.rpc_id,"error":{"code":-32000,"message":"tool refused"}}))
                        .await;
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    async fn rpc(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<(Value, (String, u64)), StoreError> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .await?;
        let until = Instant::now() + timeout;
        loop {
            let event = self.receive(until).await?;
            let mine = matches!(event.kind, NativeEventKind::Response { id: r } if r == id);
            let coordinate = (event.transport_session.clone(), event.sequence);
            let result = event.value.get("result").cloned();
            let error = event.value.get("error").is_some();
            if matches!(event.kind, NativeEventKind::Response { .. }) && !mine {
                // Recorded as ingress before the conflict, as the Codex runner does.
                let _ = self
                    .ask(|reply| NativeMessage::Ingress { event, reply })
                    .await;
                return Err(StoreError::ClaimConflict);
            }
            self.handle(event).await?;
            if mine {
                return match (error, result) {
                    (false, Some(result)) => Ok((result, coordinate)),
                    _ => Err(StoreError::Unavailable),
                };
            }
        }
    }

    async fn capture(
        &mut self,
        thread: &str,
        turn: &str,
        prompt: &str,
    ) -> Result<HostCapturedInput, StoreError> {
        let (reply, _) = self
            .rpc(
                "pi-host/capture",
                json!({"threadId":thread,"turnId":turn}),
                Duration::from_secs(30),
            )
            .await?;
        let (terminal, status) = self.terminal.clone().ok_or(StoreError::Rejected)?;
        let path = PathBuf::from(reply["path"].as_str().ok_or(StoreError::Rejected)?);
        let bytes = read_inside(&path, &self.expected.state_dir)?;
        if Some(sha(&bytes).as_str()) != reply["capture_sha256"].as_str() {
            return Err(StoreError::Rejected);
        }
        let expected = self.expected.clone();
        check_capture(&bytes, terminal, status, &expected, turn, prompt)
    }
}

/// Read a Host artifact only if it lies in the job's state directory.
fn read_inside(path: &Path, root: &Path) -> Result<Vec<u8>, StoreError> {
    let canonical = std::fs::canonicalize(path).map_err(|_| StoreError::Rejected)?;
    let root = std::fs::canonicalize(root).map_err(|_| StoreError::Rejected)?;
    if !canonical.starts_with(&root) {
        return Err(StoreError::Rejected);
    }
    std::fs::read(&canonical).map_err(|_| StoreError::Rejected)
}

/// Re-read and re-digest every payload; derive eligibility from the capture
/// and the observed terminal.
pub(crate) fn check_capture(
    bytes: &[u8],
    terminal: NativeEvent,
    status: String,
    expected: &Expected,
    turn: &str,
    prompt: &str,
) -> Result<HostCapturedInput, StoreError> {
    let capture: Value = serde_json::from_slice(bytes).map_err(|_| StoreError::Rejected)?;
    let ids = &capture["identities"];
    if capture["schema"] != pi_host::CAPTURE_SCHEMA
        || capture["realization"] != pi_host::REALIZATION
        || capture["protocol_digest"] != pi_host::PROTOCOL_DIGEST
        || capture["evidence_class"] != pi_host::EVIDENCE_CLASS
        || capture["wire"] != expected.wire.as_str()
        || ids["host_sha256"] != expected.host_sha256.as_str()
        || ids["job_sha256"] != expected.job_sha256.as_str()
        || ids["provider"] != expected.provider.as_str()
        || ids["model"] != expected.model.as_str()
        || ids["routes"] != expected.routes
        || capture["turn"]["id"] != turn
        || capture["turn"]["prompt_sha256"] != sha(prompt.as_bytes()).as_str()
    {
        return Err(StoreError::Rejected);
    }
    let mut payloads = Vec::new();
    let mut total = 0usize;
    let mut sends = std::collections::BTreeSet::new();
    for p in capture["payloads"].as_array().ok_or(StoreError::Rejected)? {
        // `request-<n>` names the Host's retained body `coding-hosted-<n>.body.json`.
        let call = p["inference_call_id"]
            .as_str()
            .ok_or(StoreError::Rejected)?;
        let n = call
            .strip_prefix("request-")
            .and_then(|n| n.parse::<u64>().ok())
            .filter(|n| *n >= 1)
            .ok_or(StoreError::Rejected)?;
        let path = PathBuf::from(p["path"].as_str().ok_or(StoreError::Rejected)?);
        if path.file_name()
            != Some(std::ffi::OsStr::new(&format!(
                "coding-hosted-{n}.body.json"
            )))
        {
            return Err(StoreError::Rejected);
        }
        let body = read_inside(&path, &expected.state_dir)?;
        total += body.len();
        let send = p["physical_send"]
            .as_u64()
            .filter(|n| *n >= 1)
            .ok_or(StoreError::Rejected)?;
        if Some(sha(&body).as_str()) != p["sha256"].as_str()
            || p["representation"] != "wire"
            || p["original_codec"] != "identity"
            || !sends.insert((call.to_owned(), send))
            || total as u64 > pi_host::MAX_TURN_BODY_BYTES
        {
            return Err(StoreError::Rejected);
        }
        // The pinned request tools, checked here over the retained bytes
        // rather than taken from the Host's own check.
        if expected.wire == "messages" {
            let parsed: Value = serde_json::from_slice(&body).map_err(|_| StoreError::Rejected)?;
            let tools = serde_json::to_vec(&parsed["tools"]).map_err(|_| StoreError::Rejected)?;
            if sha(&tools) != pi_host::tools::MESSAGES_TOOLS_SHA256 {
                return Err(StoreError::Rejected);
            }
        } else {
            return Err(StoreError::Rejected);
        }
        payloads.push(NativeInputPayload {
            inference_call_id: call.into(),
            physical_send: send,
            representation: "wire".into(),
            original_sha256: sha(&body),
            original_codec: "identity".into(),
            bytes: body,
        });
    }
    if payloads.len() as u64 > pi_host::MAX_TURN_SENDS {
        return Err(StoreError::Rejected);
    }
    // Every relayed call names a retained physical send.
    let calls_named = capture["calls"].as_array().is_some_and(|calls| {
        calls.iter().all(|c| {
            c["inference_call_id"]
                .as_str()
                .zip(c["physical_send"].as_u64())
                .is_some_and(|(call, send)| sends.contains(&(call.to_owned(), send)))
        })
    });
    let eligible = capture["eligibility"]["eligible"] == true
        && status == "completed"
        && !payloads.is_empty()
        && calls_named
        && capture["observation"]["containment_verified"] == true;
    let outcome = if eligible {
        "checked_completed".to_owned()
    } else if status != "completed" {
        "native_terminal_ineligible".to_owned()
    } else {
        "capture_ineligible".to_owned()
    };
    let metadata = json!({"realization":capture["realization"],"evidence_class":capture["evidence_class"],
        "protocol_digest":capture["protocol_digest"],"wire":capture["wire"],
        "contains_account_identifier":capture["contains_account_identifier"],
        "capture_sha256":sha(bytes),"identities":capture["identities"],"turn":capture["turn"],
        "responses":capture["responses"],"results":capture["results"],"history":capture["history"],
        "observation":capture["observation"],"eligibility":capture["eligibility"]});
    let metadata = serde_json::to_vec(&metadata).map_err(|_| StoreError::Rejected)?;
    if metadata.len() > 128 * 1024 {
        return Err(StoreError::Rejected);
    }
    // Recomputed from the shared pins, not taken from the Host.
    let request_tools = if expected.wire == "messages" {
        Some(pi_host::tools::MESSAGES_TOOLS_SHA256)
    } else {
        pi_host::tools::RESPONSES_TOOLS_SHA256
    };
    let tool_registry = serde_json::to_vec(&json!({"request_tools_sha256":request_tools,
        "receiver_tools":pi_host::tools::receiver_tools(),
        "dynamic_tools":pi_host::tools::dynamic_tools(),
        "check":"host-exact-request-tools-digest-before-dispatch; runner re-digested every retained body"}))
    .map_err(|_| StoreError::Rejected)?;
    let causal_bundle = if eligible {
        let calls: Vec<Value> = capture["calls"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| {
                json!({"call_id":c["call_id"],"inference_call_id":c["inference_call_id"],
                "physical_send":c["physical_send"],"response_id":c["response_id"],
                "namespace":c["namespace"],"tool":c["tool"],"arguments":c["arguments"]})
            })
            .collect();
        Some(serde_json::to_vec(&json!({"calls":calls})).map_err(|_| StoreError::Rejected)?)
    } else {
        None
    };
    Ok(HostCapturedInput {
        transport: terminal.transport_session.clone(),
        terminal_sequence: terminal.sequence,
        terminal_raw: terminal.raw.clone(),
        metadata,
        payloads,
        tool_registry,
        eligible,
        outcome,
        causal_bundle,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn run_pi_host(
    config: NativeConfig,
    launch: NativeLaunch,
    transport: String,
    messages: mpsc::Sender<NativeMessage>,
    gate: Arc<NativeGate>,
    task_work: Option<mpsc::Receiver<NativeTaskWork>>,
    startup_hold: Option<crate::native_startup::NativeStartupHold>,
) {
    let outcome = async {
        let Some(host) = launch.pi_host.clone() else {
            return Err(("transport_error_guard_barrier", None));
        };
        // No installed-research admission exists for this realization yet
        // (the startup hold needs the protection owner's contract, R9').
        if startup_hold.is_some() || gate.load(Ordering::SeqCst) {
            return Err(("transport_error_guard_barrier", None));
        }
        let routes = check_job(&host).map_err(|_| ("transport_error_guard_barrier", None))?;
        let expected = Expected {
            host_sha256: host.host_sha256.clone(),
            job_sha256: host.job_sha256.clone(),
            provider: host.provider.clone(),
            model: host.model.clone(),
            wire: host.wire().into(),
            routes,
            state_dir: host.state_dir.clone(),
        };
        std::fs::create_dir_all(&launch.work).map_err(|_| ("transport_error_launch", None))?;
        let mut command = tokio::process::Command::new(&config.executable);
        command
            .arg("serve")
            .arg("--config")
            .arg(&host.serve_config)
            .current_dir(&launch.work);
        let pipe = NativeStdio::spawn(&mut command, MAX_FRAME, MAX_FRAMES)
            .map_err(|_| ("transport_error_launch", None))?;
        let mut runner = Runner {
            pipe,
            transport: transport.clone(),
            messages: messages.clone(),
            gate: gate.clone(),
            next_id: 0,
            bound_turn: false,
            turn_until: None,
            terminal: None,
            deadline: Duration::from_secs(host.turn_deadline_secs + 60),
            expected,
        };
        let result = drive(&mut runner, &config, &host, task_work).await;
        // End of input closes the Host's thread: it closes Pi's bridge,
        // collects and removes the container, then exits. A forced kill would
        // leave the container for reconciliation, so it is the last resort.
        // Mid-turn, the Host may notice only when Pi's current inference
        // attempt ends, so the wait then covers the turn bound.
        runner.pipe.close_stdin();
        let close = Instant::now() + Duration::from_secs(CLOSE_SECS);
        let until = match runner.turn_until {
            Some(turn) if runner.terminal.is_none() => turn.max(close),
            _ => close,
        };
        while let Ok(Ok(Some(_))) = tokio::time::timeout_at(until, runner.pipe.receive()).await {}
        let exit = runner
            .pipe
            .terminate_and_reap(Duration::from_secs(10))
            .await;
        match (result, exit) {
            (_, Err(_)) => Err(("cleanup_unobserved", None)),
            (Ok(()), Ok(exit)) => Ok(Some(exit)),
            (Err(_), Ok(exit)) => Err(("transport_error_turn", Some(exit))),
        }
    }
    .await;
    let (exit, kind) = match outcome {
        Ok(exit) => (exit, "completed"),
        Err((kind, exit)) => (exit, kind),
    };
    let (reply, acknowledged) = oneshot::channel();
    if messages
        .send(NativeMessage::Done {
            exit,
            outcome_kind: if gate.load(Ordering::SeqCst) {
                "shutdown"
            } else {
                kind
            },
            isolation_diagnostic: None,
            reply,
        })
        .await
        .is_ok()
    {
        let _ = acknowledged.await;
    }
}

async fn drive(
    runner: &mut Runner,
    config: &NativeConfig,
    host: &PiHostConfig,
    mut task_work: Option<mpsc::Receiver<NativeTaskWork>>,
) -> Result<(), StoreError> {
    let (init, _) = runner
        .rpc("pi-host/initialize", json!({}), Duration::from_secs(30))
        .await?;
    if init["realization"] != pi_host::REALIZATION
        || init["protocol_digest"] != pi_host::PROTOCOL_DIGEST
        || init["evidence_class"] != pi_host::EVIDENCE_CLASS
        || init["host_sha256"] != host.host_sha256.as_str()
        || init["job_sha256"] != host.job_sha256.as_str()
        || init["provider"] != host.provider.as_str()
        || init["model"] != host.model.as_str()
        || init["wire"] != host.wire()
    {
        return Err(StoreError::Rejected);
    }
    let (started, _) = runner
        .rpc(
            "thread/start",
            json!({"model":host.model,"dynamicTools":pi_host::tools::dynamic_tools()}),
            Duration::from_secs(THREAD_START_SECS),
        )
        .await?;
    if started["thread"]["ephemeral"] != false {
        return Err(StoreError::Rejected);
    }
    let thread = started["thread"]["id"]
        .as_str()
        .ok_or(StoreError::Unavailable)?
        .to_owned();
    // H5: the protected surface as launched (recorded as ingress).
    let (observed, _) = runner
        .rpc(
            "pi-host/observe",
            json!({"threadId":thread}),
            Duration::from_secs(30),
        )
        .await?;
    if observed["containment_verified"] != true
        || observed["running"] != true
        || observed["network_mode"] != "none"
        || observed["auth_mount"] != false
    {
        return Err(StoreError::Rejected);
    }
    let (reply, answer) = oneshot::channel();
    runner
        .messages
        .send(NativeMessage::BindThread {
            thread: thread.clone(),
            reply,
        })
        .await
        .map_err(|_| StoreError::Unavailable)?;
    // Fresh start only: a rehydration prompt means a successor, refused here.
    if answer
        .await
        .map_err(|_| StoreError::Unavailable)??
        .is_some()
    {
        return Err(StoreError::Rejected);
    }
    if let Some(receiver) = task_work.as_mut() {
        while let Some(work) = receiver.recv().await {
            if runner.gate.load(Ordering::SeqCst) {
                return Err(StoreError::Rejected);
            }
            match work {
                NativeTaskWork::RunTurn(turn) => run_turn(runner, &thread, &turn).await?,
                NativeTaskWork::StartSuccessor => return Err(StoreError::Rejected),
            }
        }
        return Ok(());
    }
    let mut turns = vec![NativeFollowupTurn {
        invocation_id: config.invocation_id.clone(),
        attempt_id: config.attempt_id.clone(),
        prompt: config.prompt.clone(),
    }];
    turns.extend(config.followup_turns.iter().cloned());
    for turn in turns {
        run_turn(runner, &thread, &turn).await?;
    }
    Ok(())
}

async fn run_turn(
    runner: &mut Runner,
    thread: &str,
    spec: &NativeFollowupTurn,
) -> Result<(), StoreError> {
    runner.bound_turn = false;
    runner.terminal = None;
    runner.turn_until = Some(Instant::now() + runner.deadline);
    runner
        .ask(|reply| NativeMessage::PrepareTurn {
            invocation_id: spec.invocation_id.clone(),
            attempt_id: spec.attempt_id.clone(),
            prompt: spec.prompt.clone(),
            purpose: lifecycle_core::NativeInvocationPurpose::DomainWork,
            reply,
        })
        .await?;
    let (response, _) = runner
        .rpc(
            "turn/start",
            json!({"threadId":thread,"input":[{"type":"text","text":spec.prompt}]}),
            Duration::from_secs(30),
        )
        .await?;
    let turn = response["turn"]["id"]
        .as_str()
        .ok_or(StoreError::Unavailable)?
        .to_owned();
    runner
        .ask(|reply| NativeMessage::BindTurn {
            turn: turn.clone(),
            reply,
        })
        .await?;
    runner.bound_turn = true;
    let until = runner
        .turn_until
        .unwrap_or_else(|| Instant::now() + runner.deadline);
    while runner.terminal.is_none() {
        let event = runner.receive(until).await?;
        let terminal = event.terminal().ok().flatten();
        let had_terminal = runner.terminal.is_some();
        if let Err(error) = runner.handle(event).await
            && (terminal.is_none() || had_terminal)
        {
            return Err(error);
        }
    }
    let (terminal_event, status) = runner.terminal.clone().ok_or(StoreError::Rejected)?;
    let terminal = terminal_event
        .terminal()
        .map_err(|_| StoreError::Rejected)?
        .ok_or(StoreError::Rejected)?;
    if terminal.thread_id != thread || terminal.turn_id != turn {
        return Err(StoreError::ClaimConflict);
    }
    // Input is retained for every terminal, eligible or not.
    let input = runner.capture(thread, &turn, &spec.prompt).await?;
    runner
        .ask(|reply| NativeMessage::CaptureHostInput { input, reply })
        .await?;
    if status != "completed" {
        return Err(StoreError::ClaimConflict);
    }
    let (read, (history_transport, history_sequence)) = runner
        .rpc(
            "thread/read",
            json!({"threadId":thread,"includeTurns":true}),
            Duration::from_secs(30),
        )
        .await?;
    let turns = read["thread"]["turns"]
        .as_array()
        .ok_or(StoreError::Unavailable)?;
    let exact = turns
        .iter()
        .find(|t| t["id"] == turn.as_str())
        .filter(|t| t["status"] == "completed")
        .ok_or(StoreError::Unavailable)?;
    let items = exact["items"].as_array().ok_or(StoreError::Unavailable)?;
    let final_text = items
        .iter()
        .rev()
        .find(|item| item["type"] == "agentMessage")
        .and_then(|item| item["text"].as_str())
        .ok_or(StoreError::Unavailable)?
        .to_owned();
    runner
        .ask(|reply| NativeMessage::Settle {
            final_text,
            history_status: "observed_complete".into(),
            history_transport,
            history_sequence,
            reply,
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native(root: &Path) -> (NativeConfig, PiHostConfig) {
        let config: NativeConfig = serde_json::from_value(json!({
            "executable":"/bin/true","root":root,"provider_route":"pi-host-v1",
            "simulator_base_url":"","session_id":"session:t","invocation_id":"invocation:t",
            "attempt_id":"attempt:t","prompt":"p","turn_grant_ref":"g","turn_grant_revision":1,
            "snapshot_grant_ref":"g","snapshot_grant_revision":1,
            "snapshot":{"id":"snapshot:t","source_revision":"r","members":[{"id":"m",
                "sha256":work_engine_types::CodecContract::BinaryArtifactV1.digest_binary(b"x").unwrap().hex(),"bytes":[120]}]}
        }))
        .unwrap();
        let host = PiHostConfig {
            host_sha256: "a".repeat(64),
            serve_config: root.join("serve.json"),
            serve_config_sha256: "b".repeat(64),
            job_sha256: "c".repeat(64),
            state_dir: root.join("state"),
            provider: "claude-sdk".into(),
            model: "claude-haiku-5-5".into(),
            turn_deadline_secs: 300,
        };
        (config, host)
    }

    #[test]
    fn configuration_is_pi_host_only_and_state_inside_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let (config, host) = native(dir.path());
        assert!(validate(&config, &host).is_ok());
        let (mut config2, _) = native(dir.path());
        config2.simulator_base_url = "http://127.0.0.1:1/v1".into();
        assert!(validate(&config2, &host).is_err());
        let mut outside = host.clone();
        outside.state_dir = PathBuf::from("/tmp/elsewhere");
        assert!(validate(&config, &outside).is_err());
        let mut provider = host.clone();
        provider.provider = "anthropic".into();
        assert!(validate(&config, &provider).is_err());
    }

    /// A job with a loopback route is admitted only under controlled proof.
    #[test]
    fn loopback_routes_are_controlled_proof_only() {
        let dir = tempfile::tempdir().unwrap();
        let (_, mut host) = native(dir.path());
        let job = json!({"state_dir":host.state_dir,"coding":{"schema":LIFECYCLE_PROFILE,
            "provider":{"provider":"claude-sdk","model":"claude-haiku-5-5",
                "hosted":{"routes":[{"id":"p","payer":"x","endpoint":"loopback:1"}]}}}});
        let job_bytes = serde_json::to_vec(&job).unwrap();
        std::fs::write(dir.path().join("job.json"), &job_bytes).unwrap();
        host.job_sha256 = sha(&job_bytes);
        let serve = json!({"schema":SERVE_SCHEMA,"job":{"path":dir.path().join("job.json"),
            "sha256":host.job_sha256},"turn_deadline_secs":300});
        let serve_bytes = serde_json::to_vec(&serve).unwrap();
        std::fs::write(&host.serve_config, &serve_bytes).unwrap();
        host.serve_config_sha256 = sha(&serve_bytes);
        assert_eq!(check_job(&host).is_ok(), cfg!(feature = "controlled-proof"));
        let mut other = host.clone();
        other.model = "claude-other".into();
        assert!(check_job(&other).is_err());
    }

    fn capture_fixture(dir: &Path, eligible: bool) -> (Vec<u8>, Expected, NativeEvent) {
        let state = dir.join("state");
        std::fs::create_dir_all(&state).unwrap();
        let body = serde_json::to_vec(
            &json!({"messages":[],"tools":pi_host::tools::messages_request_tools()}),
        )
        .unwrap();
        let body = body.as_slice();
        let body_path = state.join("coding-hosted-1.body.json");
        std::fs::write(&body_path, body).unwrap();
        let routes = json!([{"id":"primary","payer":"claude-subscription","endpoint":"https:anthropic-oauth"}]);
        let expected = Expected {
            host_sha256: "a".repeat(64),
            job_sha256: "c".repeat(64),
            provider: "claude-sdk".into(),
            model: "claude-haiku-5-5".into(),
            wire: "messages".into(),
            routes: routes.clone(),
            state_dir: state.clone(),
        };
        let capture = json!({"schema":pi_host::CAPTURE_SCHEMA,"realization":pi_host::REALIZATION,
            "protocol_digest":pi_host::PROTOCOL_DIGEST,"evidence_class":pi_host::EVIDENCE_CLASS,
            "wire":"messages","contains_account_identifier":false,
            "identities":{"host_sha256":expected.host_sha256,"job_sha256":expected.job_sha256,
                "provider":"claude-sdk","model":"claude-haiku-5-5","routes":routes},
            "turn":{"id":"u","prompt_sha256":sha(b"prompt")},
            "payloads":[{"inference_call_id":"request-1","physical_send":1,"representation":"wire",
                "original_codec":"identity","sha256":sha(body),"path":body_path}],
            "calls":[],"observation":{"containment_verified":true},
            "eligibility":{"eligible":eligible,"reasons":[]}});
        let terminal = parse_event(
            "transport:t".into(),
            7,
            br#"{"jsonrpc":"2.0","method":"turn/completed","params":{"threadId":"t","turn":{"id":"u","status":"completed","error":null}}}"#.to_vec(),
        )
        .unwrap();
        (serde_json::to_vec(&capture).unwrap(), expected, terminal)
    }

    #[test]
    fn capture_must_state_the_launched_identities() {
        let dir = tempfile::tempdir().unwrap();
        let (bytes, expected, terminal) = capture_fixture(dir.path(), true);
        let check = |e: &Expected, turn: &str, prompt: &str, status: &str| {
            check_capture(&bytes, terminal.clone(), status.into(), e, turn, prompt)
        };
        let ok = check(&expected, "u", "prompt", "completed").unwrap();
        assert!(ok.eligible);
        assert_eq!(ok.payloads.len(), 1);
        assert!(ok.causal_bundle.is_some());
        for change in [
            |e: &mut Expected| e.model = "other".into(),
            |e: &mut Expected| e.host_sha256 = "d".repeat(64),
            |e: &mut Expected| e.job_sha256 = "d".repeat(64),
            |e: &mut Expected| e.provider = "openai-codex".into(),
            |e: &mut Expected| e.wire = "responses".into(),
            |e: &mut Expected| e.routes = json!([]),
        ] {
            let mut other = expected.clone();
            change(&mut other);
            assert!(check(&other, "u", "prompt", "completed").is_err());
        }
        assert!(check(&expected, "u", "other prompt", "completed").is_err());
        assert!(check(&expected, "v", "prompt", "completed").is_err());
        // A failed terminal or an ineligible capture is retained, not eligible.
        let failed = check(&expected, "u", "prompt", "failed").unwrap();
        assert!(!failed.eligible && failed.causal_bundle.is_none());
        // A payload outside the state directory refuses.
        let mut outside = expected.clone();
        outside.state_dir = dir.path().join("other");
        std::fs::create_dir_all(&outside.state_dir).unwrap();
        assert!(check(&outside, "u", "prompt", "completed").is_err());
        // Capture shapes the runner refuses or never makes eligible.
        let base: Value = serde_json::from_slice(&bytes).unwrap();
        let run = |v: &Value| {
            check_capture(
                &serde_json::to_vec(v).unwrap(),
                terminal.clone(),
                "completed".into(),
                &expected,
                "u",
                "prompt",
            )
        };
        let mut v = base.clone();
        v["payloads"][0]["inference_call_id"] = json!("request-2");
        assert!(run(&v).is_err(), "call id names another body file");
        let mut v = base.clone();
        let p = v["payloads"][0].clone();
        v["payloads"] = json!([p.clone(), p]);
        assert!(run(&v).is_err(), "duplicate physical send");
        let mut v = base.clone();
        v["calls"] = json!([{"call_id":"c","inference_call_id":"request-1","physical_send":2}]);
        assert!(!run(&v).unwrap().eligible, "call names no retained send");
        let mut v = base.clone();
        v["observation"] = Value::Null;
        assert!(!run(&v).unwrap().eligible, "no verified observation");
        // A body whose tools differ from the pin refuses.
        let body_path = expected.state_dir.join("coding-hosted-1.body.json");
        let other = br#"{"messages":[],"tools":[]}"#;
        std::fs::write(&body_path, other).unwrap();
        let mut v = base.clone();
        v["payloads"][0]["sha256"] = json!(sha(other));
        assert!(run(&v).is_err(), "request tools differ from the pin");
        // A tampered payload refuses.
        std::fs::write(&body_path, b"{}").unwrap();
        assert!(check(&expected, "u", "prompt", "completed").is_err());
    }
}
