//! Private S5 operation registry: exactly one finite immutable snapshot read.

pub mod workspace_snapshot_read;

use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use lifecycle_store::StoreError;
use work_engine_types::CodecContract;

use crate::{NativeConfig, NativeFollowupTurn};
use lifecycle_codex::{
    NativeEvent, NativeEventKind, NativeHarness, NativeToolCall, NativeTurnTerminal,
};
use lifecycle_runtime::NativeChildExit;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

pub enum NativeMessage {
    BindThread {
        thread: String,
        reply: oneshot::Sender<Result<Option<String>, StoreError>>,
    },
    PrepareTurn {
        invocation_id: String,
        attempt_id: String,
        prompt: String,
        purpose: lifecycle_core::NativeInvocationPurpose,
        reply: oneshot::Sender<Result<(), StoreError>>,
    },
    BindTurn {
        turn: String,
        reply: oneshot::Sender<Result<(), StoreError>>,
    },
    Ingress {
        event: NativeEvent,
        reply: oneshot::Sender<Result<(), StoreError>>,
    },
    Tool {
        event: NativeEvent,
        call: NativeToolCall,
        reply: oneshot::Sender<Result<Value, StoreError>>,
    },
    Settle {
        final_text: String,
        history_status: String,
        history_transport: String,
        history_sequence: u64,
        reply: oneshot::Sender<Result<(), StoreError>>,
    },
    Done {
        exit: Option<NativeChildExit>,
        outcome_kind: &'static str,
        reply: oneshot::Sender<Result<(), StoreError>>,
    },
}

struct Runner {
    harness: NativeHarness,
    messages: mpsc::Sender<NativeMessage>,
    early: Vec<NativeEvent>,
    bound_turn: bool,
    terminal: Option<NativeTurnTerminal>,
    tool_calls: u64,
    gate: Arc<AtomicBool>,
}

impl Runner {
    async fn receive(&mut self, budget: Duration) -> Result<NativeEvent, StoreError> {
        let deadline = Instant::now() + budget;
        loop {
            if self.gate.load(Ordering::SeqCst) {
                return Err(StoreError::Rejected);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(StoreError::Unavailable);
            }
            tokio::select! {
                event=self.harness.receive()=>return event.map_err(|_|StoreError::Unavailable),
                _=tokio::time::sleep(left.min(Duration::from_millis(50)))=>{},
            }
        }
    }

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

    async fn record(&self, event: NativeEvent) -> Result<(), StoreError> {
        self.ask(|reply| NativeMessage::Ingress { event, reply })
            .await
    }

    async fn tool(&mut self, event: NativeEvent, call: NativeToolCall) -> Result<(), StoreError> {
        let (tx, rx) = oneshot::channel();
        self.messages
            .send(NativeMessage::Tool {
                event,
                call: call.clone(),
                reply: tx,
            })
            .await
            .map_err(|_| StoreError::Unavailable)?;
        let result = rx.await.map_err(|_| StoreError::Unavailable)??;
        if self.gate.load(Ordering::SeqCst) {
            return Err(StoreError::Rejected);
        }
        self.harness
            .send_result(call.rpc_id, result)
            .await
            .map_err(|_| StoreError::Unavailable)?;
        self.tool_calls += 1;
        Ok(())
    }

    async fn handle(&mut self, event: NativeEvent) -> Result<(), StoreError> {
        self.record(event.clone()).await?;
        // Raw ingress belongs to the store before any fallible interpretation.
        // A malformed terminal is still an observed native frame.
        let terminal = event.terminal().map_err(|_| StoreError::Rejected)?;
        match &event.kind {
            NativeEventKind::ServerRequest { id, method } if method == "item/tool/call" => {
                let call = event
                    .tool_call()
                    .map_err(|_| StoreError::Rejected)?
                    .ok_or(StoreError::Rejected)?;
                if self.bound_turn {
                    self.tool(event, call).await?;
                } else {
                    if self.early.len() >= 128 {
                        return Err(StoreError::Unavailable);
                    }
                    self.early.push(event);
                }
            }
            NativeEventKind::ServerRequest { id, .. } => {
                self.harness
                    .deny_request(*id)
                    .await
                    .map_err(|_| StoreError::Unavailable)?;
                return Err(StoreError::ClaimConflict);
            }
            _ => {}
        }
        if let Some(terminal) = terminal {
            if self.terminal.is_some() {
                return Err(StoreError::ClaimConflict);
            }
            self.terminal = Some(terminal);
        }
        Ok(())
    }

    async fn rpc(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(Value, (String, u64)), StoreError> {
        if self.gate.load(Ordering::SeqCst) {
            return Err(StoreError::Rejected);
        }
        let id = self
            .harness
            .send_request(method, params)
            .await
            .map_err(|_| StoreError::Unavailable)?;
        loop {
            let event = self.receive(Duration::from_secs(20)).await?;
            let response =
                matches!(event.kind,NativeEventKind::Response{id:response_id} if response_id==id);
            if matches!(event.kind, NativeEventKind::Response { .. }) && !response {
                self.record(event).await?;
                return Err(StoreError::ClaimConflict);
            }
            let result = if response {
                event.value.get("result").cloned()
            } else {
                None
            };
            let error = response && event.value.get("error").is_some();
            let coordinate = (event.transport_session.clone(), event.sequence);
            self.handle(event).await?;
            if error || (response && result.is_none()) {
                return Err(StoreError::Unavailable);
            }
            if let Some(result) = result {
                return Ok((result, coordinate));
            }
        }
    }

    async fn flush_early(&mut self) -> Result<(), StoreError> {
        let early = std::mem::take(&mut self.early);
        for event in early {
            if let Some(call) = event.tool_call().map_err(|_| StoreError::Rejected)? {
                self.tool(event, call).await?;
            }
        }
        Ok(())
    }

    async fn run(
        &mut self,
        config: &NativeConfig,
        launch: &NativeLaunch,
    ) -> Result<(), StoreError> {
        let (init,_)=self.rpc("initialize",json!({"clientInfo":{"name":"work_engine_s5","version":"0.1.0"},"capabilities":{"experimentalApi":true}})).await?;
        if !init
            .get("userAgent")
            .and_then(Value::as_str)
            .is_some_and(|s| s.contains("0.160.1"))
            || init.get("codexHome").and_then(Value::as_str) != launch.codex_home.to_str()
        {
            return Err(StoreError::Rejected);
        }
        self.harness
            .send_notification("initialized", json!({}))
            .await
            .map_err(|_| StoreError::Unavailable)?;
        let (effective, _) = self
            .rpc("config/read", json!({"includeLayers":false}))
            .await?;
        let c = &effective["config"];
        if c["model_provider"] != "local_probe"
            || c["sandbox_mode"] != "read-only"
            || c["approval_policy"] != "never"
            || c["web_search"] != "disabled"
            || c["features"]["shell_tool"] != false
            || c["features"]["multi_agent"] != false
            || c["features"]["code_mode_host"] != false
            || c["agents"]["enabled"] != false
            || c["apps"]["_default"]["enabled"] != false
        {
            return Err(StoreError::Rejected);
        }
        let (started,_)=self.rpc("thread/start",json!({"cwd":launch.work,"model":"gpt-6-sol",
            "modelProvider":"local_probe","approvalPolicy":"never","sandbox":"read-only",
            "environments":[],"dynamicTools":[{"type":"namespace","name":"workspace",
            "description":"Read exact approved immutable workspace snapshot bytes.","tools":[{
                "type":"function","name":"snapshot_read",
                "description":"Read one approved snapshot member and bounded range.",
                "inputSchema":{"type":"object","properties":{"snapshot_id":{"type":"string"},
                    "member_id":{"type":"string"},"offset":{"type":"integer","minimum":0},
                    "length":{"type":"integer","minimum":1,"maximum":65536}},
                    "required":["snapshot_id","member_id","offset","length"],"additionalProperties":false}
            }]}],"ephemeral":false,"historyMode":"legacy"})).await?;
        let thread = started["thread"]["id"]
            .as_str()
            .ok_or(StoreError::Unavailable)?
            .to_owned();
        if started["thread"]["ephemeral"] != false {
            return Err(StoreError::Rejected);
        }
        let (reply, answer) = oneshot::channel();
        self.messages
            .send(NativeMessage::BindThread {
                thread: thread.clone(),
                reply,
            })
            .await
            .map_err(|_| StoreError::Unavailable)?;
        let rehydration = answer.await.map_err(|_| StoreError::Unavailable)??;
        native_proof_barrier(
            launch.proof_barrier_dir.as_deref(),
            "native_before_turn_entry",
        )?;
        if let Some(prompt) = rehydration {
            let receipt = NativeFollowupTurn {
                invocation_id: format!("rehydrate:{}", config.session_id),
                attempt_id: format!("rehydrate:{}", config.attempt_id),
                prompt,
            };
            self.run_turn(
                &thread,
                &receipt,
                lifecycle_core::NativeInvocationPurpose::SuccessorRehydrate,
                launch.proof_barrier_dir.as_deref(),
            )
            .await?;
        }
        let mut turns = vec![NativeFollowupTurn {
            invocation_id: config.invocation_id.clone(),
            attempt_id: config.attempt_id.clone(),
            prompt: config.prompt.clone(),
        }];
        turns.extend(config.followup_turns.iter().cloned());
        for turn in turns {
            self.run_turn(
                &thread,
                &turn,
                lifecycle_core::NativeInvocationPurpose::DomainWork,
                launch.proof_barrier_dir.as_deref(),
            )
            .await?;
        }
        Ok(())
    }

    async fn run_turn(
        &mut self,
        thread: &str,
        spec: &NativeFollowupTurn,
        purpose: lifecycle_core::NativeInvocationPurpose,
        proof_dir: Option<&Path>,
    ) -> Result<(), StoreError> {
        self.bound_turn = false;
        self.tool_calls = 0;
        self.ask(|reply| NativeMessage::PrepareTurn {
            invocation_id: spec.invocation_id.clone(),
            attempt_id: spec.attempt_id.clone(),
            prompt: spec.prompt.clone(),
            purpose,
            reply,
        })
        .await?;
        native_proof_barrier(proof_dir, "native_after_turn_send_intent")?;
        let (response,_)=self.rpc("turn/start",json!({"threadId":thread,"input":[{"type":"text","text":spec.prompt}],"environments":[]})).await?;
        let turn = response["turn"]["id"]
            .as_str()
            .ok_or(StoreError::Unavailable)?
            .to_owned();
        self.ask(|reply| NativeMessage::BindTurn {
            turn: turn.clone(),
            reply,
        })
        .await?;
        self.bound_turn = true;
        self.flush_early().await?;
        let terminal = loop {
            if let Some(terminal) = self.terminal.take() {
                if terminal.thread_id == thread && terminal.turn_id == turn {
                    break terminal;
                }
                return Err(StoreError::ClaimConflict);
            }
            let event = self.receive(Duration::from_secs(30)).await?;
            self.handle(event).await?;
        };
        if terminal.status != "completed" {
            return Err(StoreError::ClaimConflict);
        }
        let (read, (history_transport, history_sequence)) = self
            .rpc(
                "thread/read",
                json!({"threadId":thread,"includeTurns":true}),
            )
            .await?;
        let turns = read["thread"]["turns"]
            .as_array()
            .ok_or(StoreError::Unavailable)?;
        let exact = turns
            .iter()
            .find(|t| t["id"] == turn)
            .ok_or(StoreError::Unavailable)?;
        if exact["status"] != "completed" {
            return Err(StoreError::ClaimConflict);
        }
        let items = exact["items"].as_array().ok_or(StoreError::Unavailable)?;
        let final_text = items
            .iter()
            .rev()
            .find(|item| item["type"] == "agentMessage")
            .and_then(|item| item["text"].as_str())
            .ok_or(StoreError::Unavailable)?
            .to_owned();
        let history_status =
            if self.tool_calls > 0 && !items.iter().any(|i| i["type"] == "dynamicToolCall") {
                "observed_tool_omitted"
            } else {
                "observed_complete"
            };
        self.ask(|reply| NativeMessage::Settle {
            final_text,
            history_status: history_status.into(),
            history_transport,
            history_sequence,
            reply,
        })
        .await
    }
}

pub async fn run_native(
    config: NativeConfig,
    launch: NativeLaunch,
    transport_session: String,
    messages: mpsc::Sender<NativeMessage>,
    gate: Arc<AtomicBool>,
) {
    if gate.load(Ordering::SeqCst) {
        let (reply, acknowledged) = oneshot::channel();
        if messages
            .send(NativeMessage::Done {
                exit: None,
                outcome_kind: "shutdown",
                reply,
            })
            .await
            .is_ok()
        {
            let _ = acknowledged.await;
        }
        return;
    }
    match NativeHarness::spawn(
        &config.executable,
        &launch.work,
        &launch.home,
        &launch.codex_home,
        transport_session,
    ) {
        Ok(harness) => {
            let mut runner = Runner {
                harness,
                messages: messages.clone(),
                early: Vec::new(),
                bound_turn: false,
                terminal: None,
                tool_calls: 0,
                gate: gate.clone(),
            };
            let outcome = runner.run(&config, &launch).await;
            let cleanup = runner.harness.terminate_and_reap().await;
            let (exit, outcome_kind) = match cleanup {
                Ok(exit) if gate.load(Ordering::SeqCst) => (Some(exit), "shutdown"),
                Ok(exit) if outcome.is_ok() => (Some(exit), "completed"),
                Ok(exit) => (Some(exit), "transport_error"),
                Err(_) => (None, "cleanup_unobserved"),
            };
            // The runner still owns the child until the store acknowledges
            // its observed exit or durable unsafe cleanup classification.
            let (reply, acknowledged) = oneshot::channel();
            if messages
                .send(NativeMessage::Done {
                    exit,
                    outcome_kind,
                    reply,
                })
                .await
                .is_ok()
            {
                let _ = acknowledged.await;
            }
        }
        Err(_) => {
            let (reply, acknowledged) = oneshot::channel();
            if messages
                .send(NativeMessage::Done {
                    exit: None,
                    outcome_kind: "transport_error",
                    reply,
                })
                .await
                .is_ok()
            {
                let _ = acknowledged.await;
            }
        }
    }
}

pub struct NativeLaunch {
    pub home: PathBuf,
    pub codex_home: PathBuf,
    pub work: PathBuf,
    pub profile_digest: String,
    pub proof_barrier_dir: Option<PathBuf>,
}

pub fn native_proof_barrier(root: Option<&Path>, point: &str) -> Result<(), StoreError> {
    if !cfg!(feature = "native-sim-proof") {
        return Ok(());
    }
    let Some(root) = root else {
        return Ok(());
    };
    let reached = root.join(format!("{point}.reached"));
    let release = root.join(format!("{point}.release"));
    let mut file = std::fs::File::create(reached).map_err(|_| StoreError::Unavailable)?;
    use std::io::Write as _;
    file.write_all(b"reached\n")
        .map_err(|_| StoreError::Unavailable)?;
    file.sync_all().map_err(|_| StoreError::Unavailable)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !release.exists() {
        if std::time::Instant::now() >= deadline {
            return Err(StoreError::Unavailable);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(())
}

/// Materialize an auth-free, finite local simulator profile. The caller's
/// network namespace is a separate proof condition; config alone is not one.
pub fn prepare_native_launch(config: &NativeConfig, uid: u32) -> Result<NativeLaunch, StoreError> {
    let url = config.simulator_base_url.as_str();
    let Some(port) = url
        .strip_prefix("http://127.0.0.1:")
        .and_then(|tail| tail.strip_suffix("/v1"))
        .and_then(|p| p.parse::<u16>().ok())
    else {
        return Err(StoreError::Rejected);
    };
    if port == 0 || !config.root.is_absolute() {
        return Err(StoreError::Rejected);
    }
    fs::create_dir_all(&config.root).map_err(|_| StoreError::Unavailable)?;
    let root = fs::canonicalize(&config.root).map_err(|_| StoreError::Unavailable)?;
    let metadata = fs::symlink_metadata(&root).map_err(|_| StoreError::Unavailable)?;
    if !metadata.is_dir() || metadata.uid() != uid || metadata.permissions().mode() & 0o077 != 0 {
        return Err(StoreError::Rejected);
    }
    let home = root.join("user");
    let codex_home = root.join("codex");
    let work = root.join("work");
    for p in [&home, &codex_home, &work] {
        if p.exists() {
            return Err(StoreError::Rejected);
        }
        fs::create_dir(p).map_err(|_| StoreError::Unavailable)?;
        fs::set_permissions(p, fs::Permissions::from_mode(0o700))
            .map_err(|_| StoreError::Unavailable)?;
    }
    fs::write(
        work.join(".context-lifecycle-project"),
        b"isolated project root\n",
    )
    .map_err(|_| StoreError::Unavailable)?;
    let toml = format!(
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
remote_plugin = false
tool_suggest = false
[agents]
enabled = false
[apps._default]
enabled = false
[model_providers.local_probe]
name = "Isolated loopback S5"
base_url = "{url}"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
"#
    );
    let path = codex_home.join("config.toml");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| StoreError::Unavailable)?;
    use std::io::Write;
    file.write_all(toml.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|_| StoreError::Unavailable)?;
    if codex_home.join("auth.json").exists() {
        return Err(StoreError::Rejected);
    }
    let profile_digest = CodecContract::BinaryArtifactV1
        .digest_binary(toml.as_bytes())
        .map_err(|_| StoreError::Unavailable)?
        .hex();
    Ok(NativeLaunch {
        home,
        codex_home,
        work,
        profile_digest,
        proof_barrier_dir: None,
    })
}
