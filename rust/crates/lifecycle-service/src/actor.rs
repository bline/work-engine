use std::{
    fs,
    io::Write,
    sync::Arc,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use lifecycle_codex::{NATIVE_BINARY_SHA256, NativeEventKind, SCHEMA_THREAD_START_SHA256};
use lifecycle_core::{
    AttemptId, NativeSessionId, NativeToolCallId, OperationAttemptId, OperationContractId,
    OperationExecutorId, OperationImplementationId, ProviderThreadId, ProviderTurnId,
};
use lifecycle_core::{
    AuthorityExpiresAt, BuildId, ClockSample, Command, CommandOutcome, ContextGeneration, GrantId,
    GrantScope, ProofRunId, Revision, RuntimeIncarnation, SubjectId, TrustedGrant, WaitBudgetMs,
    WallTimeMs,
};
use lifecycle_runtime::{
    ExecutionReservation, LifecycleExecutor, OwnedExecutor, ProviderFailure, ResultReceiver,
    TaskResult, TaskTermination,
};
use lifecycle_store::{
    NativeIngressApply, NativeOperationClaim, NativeSessionEntry, NativeTurnEntry,
    NativeTurnRequest, SnapshotReadRequest,
};
use lifecycle_store::{RecoveryDisposition, SqliteLifecycleStore, StoreError, SubjectProjection};
use lifecycle_wire::{CommandResultV2, WireErrorCode};
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use work_engine_types::CodecContract;

use crate::{
    ServiceConfig, ServiceError,
    admission::{AdmitError, parse_authenticated},
    controlled::ControlledPort,
    engine,
    operations::workspace_snapshot_read::{self, PublishedSnapshot},
    operations::{self, NativeLaunch, NativeMessage},
};

pub enum Request {
    Command {
        uid: u32,
        bytes: Vec<u8>,
        reply: oneshot::Sender<Value>,
    },
    Snapshot {
        subject: String,
        delivery: Option<String>,
        transition: Option<String>,
        reply: oneshot::Sender<Result<SubjectProjection, StoreError>>,
    },
    Shutdown {
        reply: oneshot::Sender<Result<(), StoreError>>,
    },
}

pub fn clock_sample() -> ClockSample {
    let wall = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    ClockSample {
        wall: WallTimeMs::new(i64::try_from(wall.as_millis()).unwrap_or(i64::MAX)),
        wait_budget: WaitBudgetMs::new(1_000),
    }
}

pub struct Actor {
    config: ServiceConfig,
    store: SqliteLifecycleStore,
    executor: OwnedExecutor<ControlledPort>,
    results: ResultReceiver,
    incarnation: RuntimeIncarnation,
    entered_uncertain_on_open: bool,
    verifier_gate: Arc<engine::VerifierGate>,
    native_session: Option<NativeSessionEntry>,
    native_turn: Option<NativeTurnEntry>,
    native_snapshot: Option<PublishedSnapshot>,
    native_launch: Option<NativeLaunch>,
    native_sender: mpsc::Sender<NativeMessage>,
    native_messages: mpsc::Receiver<NativeMessage>,
    native_task: Option<tokio::task::JoinHandle<()>>,
    native_gate: Arc<AtomicBool>,
    native_exit_recorded: bool,
    native_close_unsafe: bool,
}

impl Actor {
    pub fn new(config: ServiceConfig) -> Result<Self, ServiceError> {
        let mut store =
            SqliteLifecycleStore::open(&config.store_root, config.trusted_issuer.clone())?;
        let recovery = store.load_recovery()?;
        let entered_uncertain_on_open = store
            .load_recovery_entries()?
            .iter()
            .any(|entry| entry.disposition == RecoveryDisposition::EnteredUncertain);
        if recovery.subjects == 0 {
            store.register_subject(
                SubjectId::parse(config.subject_id.clone())
                    .map_err(|_| ServiceError::Configuration)?,
                ContextGeneration::parse(config.context_generation.clone())
                    .map_err(|_| ServiceError::Configuration)?,
                BuildId::parse(config.build_id.clone()).map_err(|_| ServiceError::Configuration)?,
                ProofRunId::parse(config.proof_run_id.clone())
                    .map_err(|_| ServiceError::Configuration)?,
                clock_sample(),
            )?;
            for grant in &config.grants {
                store.install_trusted_grant(TrustedGrant {
                    id: GrantId::parse(grant.grant_ref.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    issuer: config.trusted_issuer.clone(),
                    principal: config.principal.principal_ref.clone(),
                    subject: SubjectId::parse(config.subject_id.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    context: ContextGeneration::parse(grant.context_generation.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    build: BuildId::parse(config.build_id.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    proof_run: ProofRunId::parse(config.proof_run_id.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    scope: GrantScope::parse(&grant.scope).ok_or(ServiceError::Configuration)?,
                    expires: AuthorityExpiresAt::new(WallTimeMs::new(grant.expires_wall_ms)),
                    revision: Revision::new(grant.revision),
                    revoked: false,
                })?;
            }
        } else if recovery.subjects != 1 {
            return Err(ServiceError::Configuration);
        }
        let subject =
            SubjectId::parse(config.subject_id.clone()).map_err(|_| ServiceError::Configuration)?;
        store.subject_projection(&subject, None, None)?;
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| ServiceError::Configuration)?;
        let incarnation = RuntimeIncarnation::parse(format!(
            "incarnation:{}",
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ))
        .map_err(|_| ServiceError::Configuration)?;
        let (executor, results) = OwnedExecutor::new(
            ControlledPort {
                socket: config.peer_socket_path.clone(),
            },
            config.max_executions,
        )?;
        let (native_sender, native_messages) = mpsc::channel(32);
        let native_gate = Arc::new(AtomicBool::new(false));
        let (native_session, native_snapshot, native_launch) =
            if config.profile == "native_simulated" {
                let native = config.native.as_ref().ok_or(ServiceError::Configuration)?;
                if native.followup_turns.len() > 3 {
                    return Err(ServiceError::Configuration);
                }
                let mut launch = operations::prepare_native_launch(native, config.principal.uid)?;
                launch.proof_barrier_dir = config.proof_barrier_dir.clone();
                let snapshot = workspace_snapshot_read::publish(
                    &mut store,
                    &subject,
                    &native.snapshot,
                    clock_sample(),
                )?;
                let session = store.reserve_native_session(
                    NativeSessionId::parse(native.session_id.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    subject.clone(),
                    ContextGeneration::parse(config.context_generation.clone())
                        .map_err(|_| ServiceError::Configuration)?,
                    incarnation.clone(),
                    &launch.profile_digest,
                    NATIVE_BINARY_SHA256,
                    SCHEMA_THREAD_START_SHA256,
                    snapshot.id.clone(),
                    &snapshot.manifest_sha256,
                    native.same_native_session_required,
                    clock_sample(),
                )?;
                (Some(session), Some(snapshot), Some(launch))
            } else {
                (None, None, None)
            };
        Ok(Self {
            config,
            store,
            executor,
            results,
            incarnation,
            entered_uncertain_on_open,
            verifier_gate: Arc::new(engine::VerifierGate::new()),
            native_session,
            native_turn: None,
            native_snapshot,
            native_launch,
            native_sender,
            native_messages,
            native_task: None,
            native_gate,
            native_exit_recorded: false,
            native_close_unsafe: false,
        })
    }

    pub async fn serve(mut self, mut receiver: mpsc::Receiver<Request>) {
        if self.config.profile == "native_simulated"
            && let (Some(config), Some(launch)) =
                (self.config.native.clone(), self.native_launch.take())
        {
            let transport = format!(
                "transport:{}:{}",
                config.session_id,
                self.incarnation.as_str()
            );
            let sender = self.native_sender.clone();
            let gate = self.native_gate.clone();
            self.native_task = Some(tokio::spawn(async move {
                operations::run_native(config, launch, transport, sender, gate).await;
            }));
        }
        if self.config.profile == "controlled" {
            let _ =
                engine::drive_transition(&mut self.store, &self.config, &self.verifier_gate).await;
        }
        if self.config.profile == "controlled" && !self.entered_uncertain_on_open {
            let _ = self.schedule_once(None).await;
        }
        loop {
            tokio::select! {
                maybe = receiver.recv() => {
                    match maybe {
                        Some(Request::Command { uid,bytes,reply }) => {
                            let result = self.handle_command(uid,&bytes).await;
                            let _ = reply.send(result);
                        }
                        Some(Request::Snapshot { subject,delivery,transition,reply }) => {
                            let result = self.snapshot(&subject,delivery.as_deref(),transition.as_deref());
                            let _ = reply.send(result);
                        }
                        Some(Request::Shutdown {reply}) => {
                            let result=self.shutdown().await;
                            let _=reply.send(result);
                            return;
                        }
                        None => {let _=self.shutdown().await;return;},
                    }
                }
                maybe = self.results.recv() => {
                    let Some(result) = maybe else { break; };
                    if self.handle_result(result,false).await.is_err() { break; }
                }
                maybe = self.native_messages.recv() => {
                    if let Some(message)=maybe { self.handle_native_message(message).await; }
                }
            }
        }
        let _ = self.shutdown().await;
    }

    async fn handle_native_message(&mut self, message: NativeMessage) {
        match message {
            NativeMessage::BindThread { thread, reply } => {
                let result = ProviderThreadId::parse(thread)
                    .map_err(|_| StoreError::Rejected)
                    .and_then(|thread| {
                        self.store.bind_native_thread(
                            self.native_session.as_mut().ok_or(StoreError::Rejected)?,
                            thread,
                        )
                    });
                let result = match result {
                    Ok(()) => {
                        engine::drive_transition(&mut self.store, &self.config, &self.verifier_gate)
                            .await
                            .and_then(|()| {
                                let subject = SubjectId::parse(self.config.subject_id.clone())
                                    .map_err(|_| StoreError::Rejected)?;
                                let projection =
                                    self.store.subject_projection(&subject, None, None)?;
                                if projection.owner_kind != "transition" {
                                    return Ok(None);
                                }
                                let transition =
                                    projection.transition.ok_or(StoreError::Unavailable)?;
                                if transition.stage != "rehydrating" {
                                    return Err(StoreError::Rejected);
                                }
                                let id =
                                    lifecycle_core::TransitionId::parse(transition.transition_id)
                                        .map_err(|_| StoreError::Unavailable)?;
                                self.store
                                    .committed_text_for_transition(&id)?
                                    .map(|value| value.final_text)
                                    .ok_or(StoreError::Rejected)
                                    .map(Some)
                            })
                    }
                    Err(error) => Err(error),
                };
                let _ = reply.send(result);
            }
            NativeMessage::PrepareTurn {
                invocation_id,
                attempt_id,
                prompt,
                purpose,
                reply,
            } => {
                let result = (|| {
                    if self.native_gate.load(Ordering::SeqCst) {
                        return Err(StoreError::Rejected);
                    }
                    let native = self.config.native.as_ref().ok_or(StoreError::Rejected)?;
                    let session = self.native_session.as_ref().ok_or(StoreError::Rejected)?;
                    let entry = self.store.claim_native_turn(
                        session,
                        NativeTurnRequest {
                            invocation_id,
                            attempt: AttemptId::parse(attempt_id)
                                .map_err(|_| StoreError::Rejected)?,
                            grant: GrantId::parse(native.turn_grant_ref.clone())
                                .map_err(|_| StoreError::Rejected)?,
                            grant_revision: native.turn_grant_revision,
                            purpose,
                            prompt: prompt.into_bytes(),
                        },
                        &self.config.principal.principal_ref,
                        clock_sample(),
                    )?;
                    self.store.mark_native_send_intent(&entry)?;
                    self.native_turn = Some(entry);
                    Ok(())
                })();
                let _ = reply.send(result);
            }
            NativeMessage::BindTurn { turn, reply } => {
                let result = ProviderTurnId::parse(turn)
                    .map_err(|_| StoreError::Rejected)
                    .and_then(|turn| {
                        self.store.bind_native_turn(
                            self.native_turn.as_mut().ok_or(StoreError::Rejected)?,
                            turn,
                        )
                    });
                let _ = reply.send(result);
            }
            NativeMessage::Ingress { event, reply } => {
                let result = self.record_native_event(&event);
                let _ = reply.send(result);
            }
            NativeMessage::Tool { event, call, reply } => {
                let result = self.execute_native_tool(&event, &call);
                let _ = reply.send(result);
            }
            NativeMessage::Settle {
                final_text,
                history_status,
                history_transport,
                history_sequence,
                reply,
            } => {
                let result = self
                    .native_turn
                    .as_ref()
                    .ok_or(StoreError::Rejected)
                    .and_then(|entry| {
                        self.store.settle_native_turn(
                            entry,
                            &final_text,
                            &history_status,
                            &history_transport,
                            history_sequence,
                            clock_sample(),
                        )
                    });
                let result = if result.is_ok() {
                    engine::drive_transition(&mut self.store, &self.config, &self.verifier_gate)
                        .await
                } else {
                    result
                };
                let _ = reply.send(result);
            }
            NativeMessage::Done {
                exit,
                outcome_kind,
                reply,
            } => {
                if outcome_kind == "cleanup_unobserved" {
                    self.native_close_unsafe = true;
                }
                let result = if let Some(session) = self.native_session.as_ref() {
                    self.store.record_native_process_exit(
                        session.id(),
                        exit.as_ref().and_then(|e| e.pid),
                        exit.as_ref().and_then(|e| e.code),
                        exit.as_ref().is_none_or(|e| e.forced),
                        outcome_kind,
                        clock_sample(),
                    )
                } else {
                    Err(StoreError::Rejected)
                };
                if result.is_ok() {
                    self.native_exit_recorded = true;
                } else {
                    self.native_close_unsafe = true;
                }
                let _ = reply.send(result);
            }
        }
    }

    fn record_native_event(
        &mut self,
        event: &lifecycle_codex::NativeEvent,
    ) -> Result<(), StoreError> {
        let (method, params) = match &event.kind {
            NativeEventKind::Malformed => (None, None),
            NativeEventKind::Response { .. } => (None, None),
            NativeEventKind::ServerRequest { method, .. }
            | NativeEventKind::Notification { method } => {
                (Some(method.as_str()), event.value.get("params"))
            }
        };
        let thread = params
            .and_then(|p| p.get("threadId"))
            .and_then(Value::as_str);
        let turn = params
            .and_then(|p| p.get("turnId").or_else(|| p.get("turn")?.get("id")))
            .and_then(Value::as_str);
        let item = params
            .and_then(|p| p.get("item"))
            .and_then(|i| i.get("id"))
            .and_then(Value::as_str);
        let status = match &event.kind {
            NativeEventKind::Malformed => "malformed",
            NativeEventKind::ServerRequest { method, .. } if method == "item/tool/call" => {
                match event.tool_call() {
                    Ok(Some(call))
                        if call.namespace.as_deref() == Some("workspace")
                            && call.tool == "snapshot_read"
                            && workspace_snapshot_read::parse_arguments(call.arguments.clone())
                                .is_ok() =>
                    {
                        "qualified"
                    }
                    _ => "unsupported",
                }
            }
            NativeEventKind::ServerRequest { .. } => "unsupported",
            NativeEventKind::Notification { method }
                if method == "item/started" || method == "item/completed" =>
            {
                let value = params.and_then(|p| p.get("item"));
                match value.and_then(|i| i.get("type")).and_then(Value::as_str) {
                    Some("agentMessage")
                        if value.is_some_and(|i| {
                            !i["questions"].is_null() || !i["delivery"].is_null()
                        }) =>
                    {
                        "unsupported"
                    }
                    Some("userMessage" | "agentMessage" | "reasoning") => "qualified",
                    Some("dynamicToolCall")
                        if value
                            .and_then(|i| i.get("namespace"))
                            .and_then(Value::as_str)
                            == Some("workspace")
                            && value.and_then(|i| i.get("tool")).and_then(Value::as_str)
                                == Some("snapshot_read") =>
                    {
                        "qualified"
                    }
                    _ => "unsupported",
                }
            }
            NativeEventKind::Notification { method } if method == "turn/completed" => {
                if event
                    .terminal()
                    .ok()
                    .flatten()
                    .is_some_and(|t| t.status == "completed")
                {
                    "qualified"
                } else {
                    "unsupported"
                }
            }
            NativeEventKind::Notification { method }
                if matches!(
                    method.as_str(),
                    "configWarning"
                        | "remoteControl/status/changed"
                        | "warning"
                        | "thread/status/changed"
                        | "thread/tokenUsage/updated"
                        | "account/rateLimits/updated"
                        | "thread/started"
                        | "turn/started"
                        | "turn/failed"
                        | "turn/interrupted"
                ) =>
            {
                "qualified"
            }
            NativeEventKind::Notification { .. } => "unsupported",
            NativeEventKind::Response { .. } => "qualified",
        };
        let applied = self.store.record_native_ingress(
            self.native_session
                .as_ref()
                .ok_or(StoreError::Rejected)?
                .id(),
            &event.transport_session,
            event.sequence,
            &event.raw,
            status,
            method,
            thread,
            turn,
            item,
            clock_sample(),
        )?;
        if applied == NativeIngressApply::ConflictFenced
            || matches!(status, "unsupported" | "malformed")
        {
            Err(StoreError::ClaimConflict)
        } else {
            Ok(())
        }
    }

    fn execute_native_tool(
        &mut self,
        event: &lifecycle_codex::NativeEvent,
        call: &lifecycle_codex::NativeToolCall,
    ) -> Result<Value, StoreError> {
        if self.native_gate.load(Ordering::SeqCst)
            || call.namespace.as_deref() != Some("workspace")
            || call.tool != "snapshot_read"
        {
            return Err(StoreError::Rejected);
        }
        let parent = self.native_turn.as_ref().ok_or(StoreError::Rejected)?;
        let turn = parent.turn().ok_or(StoreError::ClaimConflict)?;
        if call.thread_id != parent.thread().as_str() || call.turn_id != turn.as_str() {
            return Err(StoreError::ClaimConflict);
        }
        let args = workspace_snapshot_read::parse_arguments(call.arguments.clone())?;
        let snapshot = self.native_snapshot.as_ref().ok_or(StoreError::Rejected)?;
        if args.snapshot_id != snapshot.id.as_str() {
            return Err(StoreError::Rejected);
        }
        let native = self.config.native.as_ref().ok_or(StoreError::Rejected)?;
        let basis = format!("{}:{}", parent.invocation_id(), call.call_id);
        let digest = CodecContract::BinaryArtifactV1
            .digest_binary(basis.as_bytes())
            .map_err(|_| StoreError::Rejected)?
            .hex();
        let request = SnapshotReadRequest {
            attempt: OperationAttemptId::parse(format!("opattempt:{digest}"))
                .map_err(|_| StoreError::Rejected)?,
            call: NativeToolCallId::parse(call.call_id.clone())
                .map_err(|_| StoreError::Rejected)?,
            transport_session: event.transport_session.clone(),
            sequence: event.sequence,
            grant: GrantId::parse(native.snapshot_grant_ref.clone())
                .map_err(|_| StoreError::Rejected)?,
            grant_revision: native.snapshot_grant_revision,
            contract: OperationContractId::parse("workspace.snapshot.read.v1")
                .map_err(|_| StoreError::Rejected)?,
            implementation: OperationImplementationId::parse("rust.snapshot.read.v1")
                .map_err(|_| StoreError::Rejected)?,
            executor: OperationExecutorId::parse("lifecycle.service.snapshot")
                .map_err(|_| StoreError::Rejected)?,
            snapshot: snapshot.id.clone(),
            member_id: args.member_id.clone(),
            range_start: args.offset,
            range_length: args.length,
        };
        let result = match self.store.claim_snapshot_read(
            parent,
            request,
            &self.config.principal.principal_ref,
            clock_sample(),
        )? {
            NativeOperationClaim::Entered(entry) => {
                let bytes = workspace_snapshot_read::execute(&self.store, snapshot, &entry)?;
                let result = self
                    .store
                    .commit_snapshot_result(&entry, &bytes, clock_sample())?;
                operations::native_proof_barrier(
                    self.config.proof_barrier_dir.as_deref(),
                    "native_after_result_commit",
                )?;
                result
            }
            NativeOperationClaim::Committed(result) => result,
            NativeOperationClaim::EnteredUncertain => return Err(StoreError::ClaimConflict),
        };
        let value = if let Ok(text) = std::str::from_utf8(&result.bytes) {
            json!({"encoding":"utf8","text":text,"sha256":result.sha256,
                "snapshot_id":snapshot.id.as_str(),"member_id":args.member_id,
                "offset":args.offset,"length":args.length})
        } else {
            let hex = result
                .bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            json!({"encoding":"hex","bytes_hex":hex,"sha256":result.sha256,
                "snapshot_id":snapshot.id.as_str(),"member_id":args.member_id,
                "offset":args.offset,"length":args.length})
        };
        Ok(json!({"success":true,"contentItems":[{"type":"inputText","text":value.to_string()}]}))
    }

    async fn shutdown(&mut self) -> Result<(), StoreError> {
        self.native_gate.store(true, Ordering::SeqCst);
        self.executor.close_gate();
        self.verifier_gate.close_gate();
        let report = self
            .executor
            .close_and_drain(Instant::now() + Duration::from_secs(5))
            .await;
        let subject = SubjectId::parse(self.config.subject_id.clone())
            .map_err(|_| StoreError::Unavailable)?;
        for result in self.executor.pending_results() {
            self.handle_result(result, true).await?;
        }
        if let Some(mut task) = self.native_task.take() {
            let deadline = Instant::now() + Duration::from_secs(7);
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    self.native_close_unsafe = true;
                    if !self.native_exit_recorded
                        && let Some(session) = self.native_session.as_ref()
                    {
                        self.store.record_native_process_exit(
                            session.id(),
                            None,
                            None,
                            true,
                            "cleanup_unobserved",
                            clock_sample(),
                        )?;
                        self.native_exit_recorded = true;
                    }
                    task.abort();
                    let _ = task.await;
                    break;
                }
                tokio::select! {
                    result=&mut task=>{let _=result;break;}
                    maybe=self.native_messages.recv()=>{
                        if let Some(message)=maybe {self.handle_native_message(message).await;}
                    }
                    _=tokio::time::sleep(left)=>{}
                }
            }
            while let Ok(message) = self.native_messages.try_recv() {
                self.handle_native_message(message).await;
            }
        }
        if self.store.load_recovery_entries()?.iter().any(|entry| {
            entry.input_id.starts_with("native_")
                && entry.incarnation_id.as_deref() == Some(self.incarnation.as_str())
                && entry.disposition != RecoveryDisposition::CompletedSettled
        }) {
            self.native_close_unsafe = true;
        }
        self.store.record_service_close(
            &self.incarnation,
            &subject,
            &report.joined,
            &report.unresolved,
            &report.pending_results,
            report.unsafe_at_close || self.native_close_unsafe,
            clock_sample().wall,
        )?;
        Ok(())
    }

    async fn handle_command(&mut self, uid: u32, bytes: &[u8]) -> Value {
        let parsed = match parse_authenticated(&self.config, uid, bytes) {
            Ok(value) => value,
            Err(AdmitError::Wire(error)) => return error_reply(error.code()),
            Err(_) => return error_reply(WireErrorCode::SemanticInvalidity),
        };
        if self.config.profile == "native_simulated"
            && !matches!(
                parsed.admission.request().command,
                Command::RequestReplacement
            )
        {
            return error_reply(WireErrorCode::UnsupportedCapability);
        }
        let prior = match self.store.lookup_checked_command(&parsed.admission) {
            Ok(value) => value,
            Err(StoreError::ClaimConflict) => return error_reply(WireErrorCode::Conflict),
            Err(_) => return error_reply(WireErrorCode::ObservationUnavailable),
        };
        let replayed = prior.is_some();
        let reservation = if !replayed
            && matches!(
                parsed.admission.request().command,
                Command::EnqueueInput { .. }
            ) {
            match self.executor.try_reserve() {
                Ok(value) => Some(value),
                Err(_) => return error_reply(WireErrorCode::ObservationUnavailable),
            }
        } else {
            None
        };
        let result = match prior.map(Ok).unwrap_or_else(|| {
            self.store
                .apply_checked_command(parsed.admission, clock_sample())
        }) {
            Ok(value) => value,
            Err(StoreError::ClaimConflict) => return error_reply(WireErrorCode::Conflict),
            Err(StoreError::Rejected) => return error_reply(WireErrorCode::SemanticInvalidity),
            Err(_) => return error_reply(WireErrorCode::ObservationUnavailable),
        };
        let (accepted, outcome_kind, outcome_ref, rejection_code) = match &result.outcome {
            CommandOutcome::Enqueued { delivery, .. } => {
                (true, "enqueued", Some(delivery.as_str().to_owned()), None)
            }
            CommandOutcome::ReplacementRequested { transition } => (
                true,
                "replacement_requested",
                Some(transition.as_str().to_owned()),
                None,
            ),
            CommandOutcome::InterruptionRequested { attempt } => (
                true,
                "interruption_requested",
                Some(attempt.as_str().to_owned()),
                None,
            ),
            CommandOutcome::Rejected(code) => {
                (false, "rejected", None, Some(code.as_str().to_owned()))
            }
        };
        if accepted && !replayed {
            if outcome_kind == "enqueued" {
                let _ = self.schedule_once(reservation).await;
            }
            if outcome_kind == "replacement_requested" {
                let _ =
                    engine::drive_transition(&mut self.store, &self.config, &self.verifier_gate)
                        .await;
            }
        }
        serde_json::to_value(CommandResultV2 {
            protocol_version: 2,
            command_id: parsed.command_id,
            accepted,
            outcome_kind: outcome_kind.into(),
            outcome_ref,
            rejection_code,
            resulting_revision: result.revision.get().to_string(),
        })
        .expect("result serializes")
    }

    async fn schedule_once(
        &mut self,
        reserved: Option<ExecutionReservation<ControlledPort>>,
    ) -> Result<(), StoreError> {
        let reservation = match reserved {
            Some(value) => value,
            None => match self.executor.try_reserve() {
                Ok(value) => value,
                Err(_) => return Ok(()),
            },
        };
        let subject = SubjectId::parse(self.config.subject_id.clone())
            .map_err(|_| StoreError::Unavailable)?;
        let plan = match self.store.prepare_next_input(&subject) {
            Ok(value) => value,
            Err(StoreError::ClaimConflict) => return Ok(()),
            Err(error) => return Err(error),
        };
        proof_barrier(&self.config, "before_entry_commit").await?;
        let entry = match self.store.claim_prepared_entry(
            &plan.effect,
            self.incarnation.clone(),
            clock_sample(),
        ) {
            Ok(value) => value,
            Err(StoreError::ClaimConflict | StoreError::Rejected) => return Ok(()),
            Err(error) => return Err(error),
        };
        proof_barrier(&self.config, "after_entry_commit").await?;
        self.executor
            .submit(reservation, entry)
            .map_err(|_| StoreError::Unavailable)?;
        Ok(())
    }

    async fn handle_result(&mut self, result: TaskResult, closing: bool) -> Result<(), StoreError> {
        let termination = match &result.exit.termination {
            TaskTermination::Completed => "completed",
            TaskTermination::ProviderError(ProviderFailure::UnsupportedProfile) => {
                "unsupported_profile"
            }
            TaskTermination::ProviderError(ProviderFailure::EntryUncertain) => "entry_uncertain",
            TaskTermination::ProviderError(ProviderFailure::ObservationUnavailable) => {
                "observation_unavailable"
            }
            TaskTermination::InvalidObservation => "invalid_observation",
            TaskTermination::Panic => "panic",
            TaskTermination::Cancelled => "cancelled",
        };
        self.store.record_owned_task_result(
            &result.exit.attempt,
            &self.incarnation,
            termination,
            result.exit.local_task_ended,
            result.exit.child_exit_code,
            result.finished_wall.as_ref().ok().copied(),
            result.observation.as_ref().map(|value| &value.source),
            result.final_text.as_deref(),
        )?;
        if let Some(observation) = result.observation {
            self.store.apply_bound_result(
                observation,
                result.final_text.as_deref(),
                clock_sample(),
            )?;
            if !closing {
                proof_barrier(&self.config, "after_result_commit").await?;
                engine::drive_transition(&mut self.store, &self.config, &self.verifier_gate)
                    .await?;
            }
        }
        if !self.executor.acknowledge_result(&result.exit.attempt).await {
            return Err(StoreError::Unavailable);
        }
        // An entered attempt with no qualified observation remains blocked in S1.
        if !closing {
            self.schedule_once(None).await?;
        }
        Ok(())
    }

    fn snapshot(
        &mut self,
        subject: &str,
        delivery: Option<&str>,
        transition: Option<&str>,
    ) -> Result<SubjectProjection, StoreError> {
        if subject != self.config.subject_id {
            return Err(StoreError::Rejected);
        }
        let subject = SubjectId::parse(subject.to_owned()).map_err(|_| StoreError::Rejected)?;
        let delivery = delivery
            .map(|value| {
                lifecycle_core::DeliveryId::parse(value.to_owned())
                    .map_err(|_| StoreError::Rejected)
            })
            .transpose()?;
        let transition = transition
            .map(|value| {
                lifecycle_core::TransitionId::parse(value.to_owned())
                    .map_err(|_| StoreError::Rejected)
            })
            .transpose()?;
        self.store
            .subject_projection(&subject, delivery.as_ref(), transition.as_ref())
    }
}

pub(crate) async fn proof_barrier(config: &ServiceConfig, point: &str) -> Result<(), StoreError> {
    if !cfg!(feature = "controlled-proof") {
        return Ok(());
    }
    let Some(root) = config.proof_barrier_dir.clone() else {
        return Ok(());
    };
    let point = point.to_owned();
    // Keep the signal loop schedulable even while a controlled launch cut is
    // held by the external test process.
    tokio::task::spawn_blocking(move || {
        if !root.exists() {
            return Err(StoreError::Unavailable);
        }
        let reached = root.join(format!("{point}.reached"));
        let release = root.join(format!("{point}.release"));
        let mut file = fs::File::create(reached).map_err(|_| StoreError::Unavailable)?;
        file.write_all(b"reached\n")
            .map_err(|_| StoreError::Unavailable)?;
        file.sync_all().map_err(|_| StoreError::Unavailable)?;
        let started = Instant::now();
        while !release.exists() {
            if started.elapsed() > Duration::from_secs(15) {
                return Err(StoreError::Unavailable);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    })
    .await
    .map_err(|_| StoreError::Unavailable)?
}

fn error_reply(code: WireErrorCode) -> Value {
    json!({"protocol_version":1,"code":code,"detail":"request unavailable"})
}

pub type ActorSender = mpsc::Sender<Request>;

pub struct ShutdownGate {
    executor: OwnedExecutor<ControlledPort>,
    verifier: Arc<engine::VerifierGate>,
    native: Arc<AtomicBool>,
}

impl ShutdownGate {
    pub fn close_gate(&self) {
        self.executor.close_gate();
        self.verifier.close_gate();
        self.native.store(true, Ordering::SeqCst);
    }
}

pub fn start(config: ServiceConfig) -> Result<(ActorSender, ShutdownGate), ServiceError> {
    let actor = Actor::new(config)?;
    let gate = ShutdownGate {
        executor: actor.executor.clone(),
        verifier: actor.verifier_gate.clone(),
        native: actor.native_gate.clone(),
    };
    let (sender, receiver) = mpsc::channel(64);
    tokio::spawn(actor.serve(receiver));
    Ok((sender, gate))
}
