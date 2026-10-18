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

use crate::{NativeConfig, NativeFollowupTurn, NativeProviderRoute};
use lifecycle_codex::{
    NativeEvent, NativeEventKind, NativeHarness, NativeToolCall, NativeTurnTerminal,
};
use lifecycle_runtime::NativeChildExit;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
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

/// Commands from the lifecycle actor to the one owned native transport. The
/// actor alone chooses FIFO work and successor allocation; the runner cannot
/// advance a delivery from configured followups or a caller callback.
pub enum NativeTaskWork {
    RunTurn(NativeFollowupTurn),
    StartSuccessor,
}

fn task_owner_id_schema(description: &str) -> Value {
    json!({"type":"string","minLength":1,"maxLength":128,
        "pattern":"^[A-Za-z0-9._:-]+$","description":description})
}

fn task_owner_source_ref_string_schema(description: &str) -> Value {
    // These are copied from an exact current owner reference, not new IDs
    // submitted for valid_id validation. The owner can prefix a valid 128-byte
    // question ID with `question-`, yielding a 137-byte source identifier.
    json!({"type":"string","minLength":1,"description":description})
}

fn task_owner_text_schema(description: &str) -> Value {
    // JSON Schema lengths count characters. The owner also checks the
    // authoritative 8 KiB UTF-8 byte limit and rejects NUL.
    json!({"type":"string","minLength":1,"maxLength":8192,
        "description":description})
}

fn domain_thread_start(work: &Path, task_profile: bool, provider: &str) -> Value {
    let mut tools = vec![json!({"type":"namespace","name":"workspace",
    "description":"Read exact approved immutable workspace snapshot bytes.","tools":[{
        "type":"function","name":"snapshot_read",
        "description":"Read one approved snapshot member and bounded range.",
        "inputSchema":{"type":"object","properties":{"snapshot_id":{"type":"string"},
            "member_id":{"type":"string"},"offset":{"type":"integer","minimum":0},
            "length":{"type":"integer","minimum":1,"maximum":65536}},
            "required":["snapshot_id","member_id","offset","length"],"additionalProperties":false}
    }]})];
    if task_profile {
        tools.push(json!({"type":"namespace","name":"task",
            "description":"Commit bounded task-owner questions or attributed results through lifecycle custody.",
            "tools":[
                {"type":"function","name":"question_open","description":"Open a pending question for an authorized human recipient; this does not answer it.",
                 "inputSchema":{"type":"object","properties":{
                     "question_id":task_owner_id_schema("Stable question identifier."),
                     "recipient_ref":task_owner_id_schema("Authorized recipient identifier."),
                     "scope_ref":task_owner_id_schema("Task scope identifier."),
                     "text":task_owner_text_schema("Nonempty question, at most 8192 UTF-8 bytes, without NUL.")},
                 "required":["question_id","recipient_ref","scope_ref","text"],"additionalProperties":false}},
                {"type":"function","name":"result_publish","description":"Publish an attributed result under current owner and grant authority. source_refs must be exact current task-owner source references; cite snapshot member evidence in the body, not as a source_refs object.",
                 "inputSchema":{"type":"object","properties":{
                     "result_id":task_owner_id_schema("Stable result identifier."),
                     "body":task_owner_text_schema("Nonempty result, at most 8192 UTF-8 bytes, without NUL."),
                     "source_refs":{"type":"array","minItems":1,"items":{"type":"object",
                         "properties":{
                             "id":task_owner_source_ref_string_schema("Copy the exact current source identifier from the task owner; this may be longer than an input identifier."),
                             "owner_namespace":task_owner_source_ref_string_schema("Copy the exact current source owner namespace."),
                             "owner_ref":task_owner_source_ref_string_schema("Copy the exact current source owner reference."),
                             "revision":{"type":"integer","minimum":0},
                             "sha256":{"type":"string","minLength":64,"maxLength":64,
                                 "pattern":"^[0-9a-f]{64}$"}},
                         "required":["id","owner_namespace","owner_ref","revision","sha256"],
                         "additionalProperties":false}},
                     "attribution":task_owner_id_schema("Attribution identifier such as native:model; no spaces or prose.")},
                 "required":["result_id","body","source_refs","attribution"],"additionalProperties":false}}
            ]}));
    }
    json!({"cwd":work,"model":NativeProviderRoute::MODEL,
        "modelProvider":provider,"approvalPolicy":"never","sandbox":"read-only",
        "environments":[],"dynamicTools":tools,"ephemeral":false,"historyMode":"legacy"})
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
        mut task_work: Option<mpsc::Receiver<NativeTaskWork>>,
    ) -> Result<(), StoreError> {
        let (init,_)=self.rpc("initialize",json!({"clientInfo":{"name":"work_engine_s5","version":"0.1.0"},"capabilities":{"experimentalApi":true}})).await?;
        if !init
            .get("userAgent")
            .and_then(Value::as_str)
            .is_some_and(|s| lifecycle_codex::user_agent_matches(s, "work_engine_s5"))
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
        validate_effective_provider(c, launch)?;
        if c["sandbox_mode"] != "read-only"
            || c["approval_policy"] != "never"
            || c["web_search"] != "disabled"
            || c["features"]["shell_tool"] != false
            || c["features"]["multi_agent"] != false
            || c["features"]["code_mode_host"] != launch.code_mode_host
            || c["agents"]["enabled"] != false
            || c["apps"]["_default"]["enabled"] != false
        {
            return Err(StoreError::Rejected);
        }
        let task_profile = task_work.is_some();
        let (started, _) = self
            .rpc(
                "thread/start",
                domain_thread_start(&launch.work, task_profile, &launch.provider_name),
            )
            .await?;
        let mut thread = started["thread"]["id"]
            .as_str()
            .ok_or(StoreError::Unavailable)?
            .to_owned();
        if started["thread"]["ephemeral"] != false {
            return Err(StoreError::Rejected);
        }
        optional_native_barrier(
            launch.proof_barrier_dir.as_deref(),
            "native_after_thread_start_before_bind",
        )?;
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
        if let Some(receiver) = task_work.as_mut() {
            while let Some(work) = receiver.recv().await {
                // Dropping the sender during shutdown may leave previously
                // buffered work. The closed gate forbids a fresh native entry.
                if self.gate.load(Ordering::SeqCst) {
                    return Err(StoreError::Rejected);
                }
                match work {
                    NativeTaskWork::RunTurn(turn) => {
                        self.run_turn(
                            &thread,
                            &turn,
                            lifecycle_core::NativeInvocationPurpose::DomainWork,
                            launch.proof_barrier_dir.as_deref(),
                        )
                        .await?;
                    }
                    NativeTaskWork::StartSuccessor => {
                        let (started, _) = self
                            .rpc(
                                "thread/start",
                                domain_thread_start(&launch.work, true, &launch.provider_name),
                            )
                            .await?;
                        let successor = started["thread"]["id"]
                            .as_str()
                            .ok_or(StoreError::Unavailable)?
                            .to_owned();
                        if started["thread"]["ephemeral"] != false || successor == thread {
                            return Err(StoreError::Rejected);
                        }
                        let (reply, answer) = oneshot::channel();
                        self.messages
                            .send(NativeMessage::BindThread {
                                thread: successor.clone(),
                                reply,
                            })
                            .await
                            .map_err(|_| StoreError::Unavailable)?;
                        let rehydration = answer.await.map_err(|_| StoreError::Unavailable)??;
                        let prompt = rehydration.ok_or(StoreError::Rejected)?;
                        let receipt = NativeFollowupTurn {
                            invocation_id: format!("rehydrate:{}", successor),
                            attempt_id: format!("rehydrate:{}", successor),
                            prompt,
                        };
                        self.run_turn(
                            &successor,
                            &receipt,
                            lifecycle_core::NativeInvocationPurpose::SuccessorRehydrate,
                            launch.proof_barrier_dir.as_deref(),
                        )
                        .await?;
                        thread = successor;
                    }
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
    task_work: Option<mpsc::Receiver<NativeTaskWork>>,
) {
    let before_spawn = if gate.load(Ordering::SeqCst) {
        Ok(())
    } else {
        optional_native_barrier(launch.proof_barrier_dir.as_deref(), "native_before_spawn")
            .and_then(|()| {
                if launch.code_mode_host {
                    verify_code_mode_host(&config.executable, CODE_MODE_HOST_SHA256)
                } else {
                    Ok(())
                }
            })
    };
    if before_spawn.is_err() || gate.load(Ordering::SeqCst) {
        let (reply, acknowledged) = oneshot::channel();
        if messages
            .send(NativeMessage::Done {
                exit: None,
                outcome_kind: if gate.load(Ordering::SeqCst) {
                    "shutdown"
                } else {
                    "transport_error"
                },
                reply,
            })
            .await
            .is_ok()
        {
            let _ = acknowledged.await;
        }
        return;
    }
    match NativeHarness::spawn_with_runtime_key(
        &config.executable,
        &launch.work,
        &launch.home,
        &launch.codex_home,
        transport_session,
        launch.requires_runtime_key,
    ) {
        Ok(harness) => {
            let before_registration = (|| {
                if cfg!(feature = "native-sim-proof")
                    && let Some(root) = launch.proof_barrier_dir.as_deref()
                    && root
                        .join("native_after_spawn_before_registration.arm")
                        .exists()
                {
                    let pid = harness.pid().ok_or(StoreError::Unavailable)?;
                    let mut marker =
                        fs::File::create(root.join("native_after_spawn_before_registration.pid"))
                            .map_err(|_| StoreError::Unavailable)?;
                    use std::io::Write as _;
                    writeln!(marker, "{pid}").map_err(|_| StoreError::Unavailable)?;
                    marker.sync_all().map_err(|_| StoreError::Unavailable)?;
                }
                optional_native_barrier(
                    launch.proof_barrier_dir.as_deref(),
                    "native_after_spawn_before_registration",
                )
            })();
            let mut runner = Runner {
                harness,
                messages: messages.clone(),
                early: Vec::new(),
                bound_turn: false,
                terminal: None,
                tool_calls: 0,
                gate: gate.clone(),
            };
            let outcome = if before_registration.is_err() || gate.load(Ordering::SeqCst) {
                Err(StoreError::Rejected)
            } else {
                runner.run(&config, &launch, task_work).await
            };
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
    pub provider_name: String,
    pub provider_base_url: String,
    pub requires_runtime_key: bool,
    pub code_mode_host: bool,
}

// Companion executable shipped with the pinned native 0.163 package. The
// code-mode bridge is a separate process and must match its qualified bytes.
const CODE_MODE_HOST_SHA256: &str =
    "8724d73ec190536233ab4628d24a071bfbcf27e5124f08fe35129f26fe48b916";

fn verify_code_mode_host(executable: &Path, expected_sha256: &str) -> Result<(), StoreError> {
    let companion = executable.with_file_name("codex-code-mode-host");
    let bytes = fs::read(companion).map_err(|_| StoreError::Rejected)?;
    if format!("{:x}", Sha256::digest(bytes)) != expected_sha256 {
        return Err(StoreError::Rejected);
    }
    Ok(())
}

/// Config/read is the child's effective profile, not merely the file the
/// service wrote. Refuse another provider, endpoint, auth source or retry
/// policy before a native turn receives an entry permit.
pub(crate) fn validate_effective_provider(
    c: &Value,
    launch: &NativeLaunch,
) -> Result<(), StoreError> {
    let provider = &c["model_providers"][&launch.provider_name];
    let expected_key = if launch.requires_runtime_key {
        Value::String("OPENAI_API_KEY".into())
    } else {
        Value::Null
    };
    if c["model_provider"] != launch.provider_name
        || c["model"] != NativeProviderRoute::MODEL
        || provider["base_url"] != launch.provider_base_url
        || provider["wire_api"] != "responses"
        || provider["env_key"] != expected_key
        || provider["requires_openai_auth"] != false
        || provider["request_max_retries"] != 0
        || provider["stream_max_retries"] != 0
        || provider["supports_websockets"] != false
        || !provider["experimental_bearer_token"].is_null()
        || !provider["auth"].is_null()
        || !provider["aws"].is_null()
        || !provider["http_headers"].is_null()
        || !provider["env_http_headers"].is_null()
    {
        return Err(StoreError::Rejected);
    }
    Ok(())
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

/// Test-controlled startup cuts are armed explicitly so existing proof runs
/// with a barrier directory do not acquire new waits.
pub fn optional_native_barrier(root: Option<&Path>, point: &str) -> Result<(), StoreError> {
    if root.is_some_and(|path| path.join(format!("{point}.arm")).exists()) {
        native_proof_barrier(root, point)?;
    }
    Ok(())
}

/// Materialize a route-bound native profile. Network confinement for the
/// simulator is a separate proof condition; config alone is not one.
pub fn prepare_native_launch(config: &NativeConfig, uid: u32) -> Result<NativeLaunch, StoreError> {
    prepare_native_launch_with_code_mode(config, uid, false, true)
}

/// Compute the exact launch digest while leaving the selected native root
/// empty. Reservation must commit before materializing its process files.
pub fn preview_native_launch(config: &NativeConfig, uid: u32) -> Result<NativeLaunch, StoreError> {
    prepare_native_launch_with_code_mode(config, uid, false, false)
}

/// The retained task profile needs the native code-mode bridge to reach its
/// three dynamic tools. Control and simulator-only profiles keep it disabled.
pub fn prepare_native_task_launch(
    config: &NativeConfig,
    uid: u32,
) -> Result<NativeLaunch, StoreError> {
    prepare_native_launch_with_code_mode(config, uid, true, true)
}

pub fn preview_native_task_launch(
    config: &NativeConfig,
    uid: u32,
) -> Result<NativeLaunch, StoreError> {
    prepare_native_launch_with_code_mode(config, uid, true, false)
}

fn prepare_native_launch_with_code_mode(
    config: &NativeConfig,
    uid: u32,
    code_mode_host: bool,
    materialize: bool,
) -> Result<NativeLaunch, StoreError> {
    let url = config
        .provider_route
        .configured_base_url(&config.simulator_base_url)
        .ok_or(StoreError::Rejected)?;
    if !config.root.is_absolute() {
        return Err(StoreError::Rejected);
    }
    if materialize {
        fs::create_dir_all(&config.root).map_err(|_| StoreError::Unavailable)?;
    }
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
        if materialize {
            fs::create_dir(p).map_err(|_| StoreError::Unavailable)?;
            fs::set_permissions(p, fs::Permissions::from_mode(0o700))
                .map_err(|_| StoreError::Unavailable)?;
        }
    }
    if materialize {
        fs::write(
            work.join(".context-lifecycle-project"),
            b"isolated project root\n",
        )
        .map_err(|_| StoreError::Unavailable)?;
    }
    let provider_name = config.provider_route.provider_name();
    let requires_runtime_key = config.provider_route.requires_runtime_key();
    let env_key = if requires_runtime_key {
        "env_key = \"OPENAI_API_KEY\"\n"
    } else {
        ""
    };
    let provider_label = if requires_runtime_key {
        "Selected OpenAI Responses"
    } else {
        "Isolated loopback S5"
    };
    let code_mode_binding = if code_mode_host {
        format!("# code_mode_host_sha256 = {CODE_MODE_HOST_SHA256}\n")
    } else {
        String::new()
    };
    let toml = format!(
        r#"model = "gpt-6-sol"
model_provider = "{provider_name}"
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
code_mode_host = {code_mode_host}
{code_mode_binding}remote_plugin = false
tool_suggest = false
[agents]
enabled = false
[apps._default]
enabled = false
[model_providers.{provider_name}]
name = "{provider_label}"
base_url = "{url}"
wire_api = "responses"
{env_key}requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
"#
    );
    if materialize {
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
        provider_name: provider_name.to_owned(),
        provider_base_url: url,
        requires_runtime_key,
        code_mode_host,
    })
}

#[cfg(test)]
mod provider_launch_tests {
    use super::*;

    #[test]
    fn result_tool_exposes_exact_owner_source_reference_shape() {
        let thread = domain_thread_start(Path::new("/disposable"), true, "local_probe");
        let task = thread["dynamicTools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "task")
            .unwrap();
        let result = task["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "result_publish")
            .unwrap();
        let refs = &result["inputSchema"]["properties"]["source_refs"];
        assert_eq!(refs["minItems"], 1);
        assert_eq!(refs["items"]["additionalProperties"], false);
        assert_eq!(
            refs["items"]["required"],
            json!(["id", "owner_namespace", "owner_ref", "revision", "sha256"])
        );
        assert_eq!(refs["items"]["properties"]["revision"]["minimum"], 0);
        // A valid 128-byte question ID becomes a 137-byte owner source ID.
        // Source references are exact owner output, not new valid_id inputs.
        let maximal_question_id = "q".repeat(128);
        let answered_source_id = format!("question-{maximal_question_id}");
        assert_eq!(answered_source_id.len(), 137);
        for field in ["id", "owner_namespace", "owner_ref"] {
            let schema = &refs["items"]["properties"][field];
            assert_eq!(schema["minLength"], 1);
            assert!(schema.get("maxLength").is_none());
            assert!(schema.get("pattern").is_none());
        }
        assert_eq!(
            refs["items"]["properties"]["sha256"]["pattern"],
            "^[0-9a-f]{64}$"
        );
        assert!(
            result["description"]
                .as_str()
                .unwrap()
                .contains("cite snapshot member evidence in the body")
        );
        let valid = json!({"result_id":"result:test","body":"bounded evidence",
            "attribution":"author:test","source_refs":[
                {"id":answered_source_id,"owner_namespace":"retained-task-workflow",
                "owner_ref":"task:test","revision":1,"sha256":"a".repeat(64)}]});
        assert!(serde_json::from_value::<crate::ResultPublishPayload>(valid.clone()).is_ok());
        let mut invalid = valid;
        invalid["source_refs"]
            .as_array_mut()
            .unwrap()
            .push(json!({"snapshot_id":"snapshot:test",
                "member_id":"member:test","sha256":"b".repeat(64)}));
        assert!(serde_json::from_value::<crate::ResultPublishPayload>(invalid).is_err());
    }

    #[test]
    fn task_tool_schemas_expose_owner_id_and_text_limits() {
        let thread = domain_thread_start(Path::new("/disposable"), true, "local_probe");
        let task = thread["dynamicTools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "task")
            .unwrap();
        let tools = task["tools"].as_array().unwrap();
        let question = tools
            .iter()
            .find(|tool| tool["name"] == "question_open")
            .unwrap();
        let result = tools
            .iter()
            .find(|tool| tool["name"] == "result_publish")
            .unwrap();
        for (tool, fields) in [
            (
                question,
                ["question_id", "recipient_ref", "scope_ref"].as_slice(),
            ),
            (result, ["result_id", "attribution"].as_slice()),
        ] {
            for field in fields {
                let schema = &tool["inputSchema"]["properties"][field];
                assert_eq!(schema["pattern"], "^[A-Za-z0-9._:-]+$");
                assert_eq!(schema["minLength"], 1);
                assert_eq!(schema["maxLength"], 128);
            }
        }
        for (tool, field) in [(question, "text"), (result, "body")] {
            let schema = &tool["inputSchema"]["properties"][field];
            assert_eq!(schema["minLength"], 1);
            assert_eq!(schema["maxLength"], 8192);
            assert!(
                schema["description"]
                    .as_str()
                    .unwrap()
                    .contains("UTF-8 bytes")
            );
        }
        assert!(
            result["inputSchema"]["properties"]["attribution"]["description"]
                .as_str()
                .unwrap()
                .contains("no spaces or prose")
        );
    }

    fn config(root: &Path, route: NativeProviderRoute, base: &str) -> NativeConfig {
        serde_json::from_value(json!({
            "executable":"/not-executed/native",
            "root":root,
            "provider_route":route,
            "simulator_base_url":base,
            "session_id":"session:test",
            "invocation_id":"invocation:test",
            "attempt_id":"attempt:test",
            "prompt":"offline",
            "turn_grant_ref":"grant:turn",
            "turn_grant_revision":1,
            "snapshot_grant_ref":"grant:snapshot",
            "snapshot_grant_revision":1,
            "snapshot":{"id":"snapshot:test","source_revision":"source:test","members":[]}
        }))
        .unwrap()
    }

    #[test]
    fn live_profile_pins_upstream_and_key_name_without_persisting_key() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("native");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let uid = fs::metadata(&root).unwrap().uid();
        let launch = prepare_native_launch(
            &config(&root, NativeProviderRoute::OpenaiResponsesLiveV1, ""),
            uid,
        )
        .unwrap();
        assert_eq!(launch.provider_name, "openai_responses_live");
        assert_eq!(launch.provider_base_url, NativeProviderRoute::LIVE_BASE_URL);
        assert!(launch.requires_runtime_key);
        let generated = fs::read_to_string(launch.codex_home.join("config.toml")).unwrap();
        assert!(generated.contains("base_url = \"https://api.openai.com/v1\""));
        assert!(generated.contains("env_key = \"OPENAI_API_KEY\""));
        assert!(generated.contains("request_max_retries = 0"));
        assert!(generated.contains("stream_max_retries = 0"));
        assert!(!launch.codex_home.join("auth.json").exists());
        assert!(generated.contains("requires_openai_auth = false"));
        assert!(!launch.code_mode_host);
        assert!(generated.contains("code_mode_host = false"));
    }

    #[test]
    fn retained_task_profile_enables_only_the_code_mode_bridge() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("native");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let uid = fs::metadata(&root).unwrap().uid();
        let launch = prepare_native_task_launch(
            &config(&root, NativeProviderRoute::OpenaiResponsesLiveV1, ""),
            uid,
        )
        .unwrap();
        assert!(launch.code_mode_host);
        let generated = fs::read_to_string(launch.codex_home.join("config.toml")).unwrap();
        assert!(generated.contains("code_mode_host = true"));
        assert!(generated.contains(&format!(
            "# code_mode_host_sha256 = {CODE_MODE_HOST_SHA256}"
        )));
        for disabled in [
            "shell_tool",
            "multi_agent",
            "hooks",
            "goals",
            "remote_plugin",
            "tool_suggest",
        ] {
            assert!(generated.contains(&format!("{disabled} = false")));
        }
        assert!(!launch.codex_home.join("auth.json").exists());
    }

    #[test]
    fn code_mode_companion_refuses_absent_or_changed_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let executable = temp.path().join("codex");
        assert!(verify_code_mode_host(&executable, CODE_MODE_HOST_SHA256).is_err());
        fs::write(
            temp.path().join("codex-code-mode-host"),
            b"synthetic companion",
        )
        .unwrap();
        assert!(verify_code_mode_host(&executable, CODE_MODE_HOST_SHA256).is_err());
        let synthetic_sha256 = format!("{:x}", Sha256::digest(b"synthetic companion"));
        assert!(verify_code_mode_host(&executable, &synthetic_sha256).is_ok());
    }

    #[test]
    fn simulator_profile_keeps_auth_free_loopback() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("native");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let uid = fs::metadata(&root).unwrap().uid();
        let launch = prepare_native_launch(
            &config(
                &root,
                NativeProviderRoute::NativeSimProofV1,
                "http://127.0.0.1:1234/v1",
            ),
            uid,
        )
        .unwrap();
        assert_eq!(launch.provider_name, "local_probe");
        assert!(!launch.requires_runtime_key);
        let generated = fs::read_to_string(launch.codex_home.join("config.toml")).unwrap();
        assert!(!generated.contains("env_key"));
    }

    #[test]
    fn effective_readback_rejects_provider_endpoint_auth_and_retry_drift() {
        let launch = NativeLaunch {
            home: PathBuf::new(),
            codex_home: PathBuf::new(),
            work: PathBuf::new(),
            profile_digest: String::new(),
            proof_barrier_dir: None,
            provider_name: "openai_responses_live".into(),
            provider_base_url: NativeProviderRoute::LIVE_BASE_URL.into(),
            requires_runtime_key: true,
            code_mode_host: false,
        };
        let valid = json!({"model":"gpt-6-sol","model_provider":"openai_responses_live",
            "model_providers":{"openai_responses_live":{
                "base_url":"https://api.openai.com/v1","wire_api":"responses",
                "env_key":"OPENAI_API_KEY","requires_openai_auth":false,
                "request_max_retries":0,"stream_max_retries":0,
                "supports_websockets":false,
                "experimental_bearer_token":null,"auth":null,"aws":null,
                "http_headers":null,"env_http_headers":null}}});
        assert!(validate_effective_provider(&valid, &launch).is_ok());
        for (path, wrong) in [
            ("model", json!("other-model")),
            ("model_provider", json!("local_probe")),
        ] {
            let mut changed = valid.clone();
            changed[path] = wrong;
            assert!(validate_effective_provider(&changed, &launch).is_err());
        }
        for (field, wrong) in [
            ("base_url", json!("http://127.0.0.1:1234/v1")),
            ("env_key", json!(null)),
            ("requires_openai_auth", json!(true)),
            ("request_max_retries", json!(1)),
            ("stream_max_retries", json!(1)),
            ("experimental_bearer_token", json!("hidden-override")),
        ] {
            let mut changed = valid.clone();
            changed["model_providers"]["openai_responses_live"][field] = wrong;
            assert!(validate_effective_provider(&changed, &launch).is_err());
        }
    }
}
