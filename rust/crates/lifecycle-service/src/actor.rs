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
    GrantScope, ProofRunId, Revision, RuntimeIncarnation, SubjectId, TransitionId, TrustedGrant,
    WaitBudgetMs, WallTimeMs,
};
use lifecycle_runtime::{
    ExecutionReservation, LifecycleExecutor, OwnedExecutor, ProviderFailure, ResultReceiver,
    TaskResult, TaskTermination,
};
use lifecycle_store::{
    GrantControlHandle, RecoveryDisposition, SqliteLifecycleStore, StoreError, SubjectProjection,
};
use lifecycle_store::{
    NativeIngressApply, NativeOperationClaim, NativeSessionEntry, NativeTaskOperationClaim,
    NativeTaskOperationRequest, NativeTurnEntry, NativeTurnRequest, SnapshotReadRequest,
};
use lifecycle_wire::{CommandResultV2, WireErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::sync::{mpsc, oneshot};
use work_engine_types::CodecContract;

use crate::{
    CanonicalMaterialPort, ServiceConfig, ServiceError, TaskOperationPort,
    admission::{AdmitError, parse_authenticated},
    controlled::ControlledPort,
    engine,
    operations::workspace_snapshot_read::{self, PublishedSnapshot},
    operations::{self, NativeLaunch, NativeMessage, NativeTaskWork},
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
    CommandReadback {
        subject: String,
        command_id: String,
        request_digest: String,
        reply: oneshot::Sender<Value>,
    },
    SubjectRevision {
        subject: String,
        reply: oneshot::Sender<Value>,
    },
    NativeDeliveryStatus {
        subject: String,
        reply: oneshot::Sender<Value>,
    },
    GrantControl {
        uid: u32,
        bytes: Vec<u8>,
        read_only: bool,
        reply: oneshot::Sender<Value>,
    },
    Shutdown {
        reply: oneshot::Sender<Result<(), StoreError>>,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GrantControlCommand {
    protocol_version: u16,
    command_id: String,
    subject_id: String,
    authority_ref: String,
    grant_ref: String,
    expected_revision: String,
}

pub(crate) trait GrantControlStore {
    #[allow(clippy::too_many_arguments)]
    fn contract(
        &mut self,
        command_id: &str,
        subject: &SubjectId,
        principal: &str,
        grant: &GrantId,
        expected: Revision,
        canonical: &[u8],
        clock: ClockSample,
    ) -> Result<Revision, StoreError>;
    fn readback(
        &self,
        command_id: &str,
        subject: &SubjectId,
        principal: &str,
        grant: &GrantId,
        expected: Revision,
        canonical: &[u8],
    ) -> Result<Option<Revision>, StoreError>;
}

impl GrantControlStore for SqliteLifecycleStore {
    fn contract(
        &mut self,
        command_id: &str,
        subject: &SubjectId,
        principal: &str,
        grant: &GrantId,
        expected: Revision,
        canonical: &[u8],
        clock: ClockSample,
    ) -> Result<Revision, StoreError> {
        self.contract_grant_command(
            command_id, subject, principal, grant, expected, canonical, clock,
        )
    }
    fn readback(
        &self,
        command_id: &str,
        subject: &SubjectId,
        principal: &str,
        grant: &GrantId,
        expected: Revision,
        canonical: &[u8],
    ) -> Result<Option<Revision>, StoreError> {
        self.grant_contraction_readback(command_id, subject, principal, grant, expected, canonical)
    }
}

impl GrantControlStore for GrantControlHandle {
    fn contract(
        &mut self,
        command_id: &str,
        subject: &SubjectId,
        principal: &str,
        grant: &GrantId,
        expected: Revision,
        canonical: &[u8],
        clock: ClockSample,
    ) -> Result<Revision, StoreError> {
        GrantControlHandle::contract(
            self, command_id, subject, principal, grant, expected, canonical, clock,
        )
    }
    fn readback(
        &self,
        command_id: &str,
        subject: &SubjectId,
        principal: &str,
        grant: &GrantId,
        expected: Revision,
        canonical: &[u8],
    ) -> Result<Option<Revision>, StoreError> {
        GrantControlHandle::readback(
            self, command_id, subject, principal, grant, expected, canonical,
        )
    }
}

/// The actor and the bounded concurrent control lane use identical wire
/// validation, durable CAS, replay readback and reply mapping.
pub(crate) fn grant_control_on_store<S: GrantControlStore>(
    store: &mut S,
    config: &ServiceConfig,
    uid: u32,
    bytes: &[u8],
    read_only: bool,
) -> Value {
    if config.profile != "retained-repository-analysis-v1" || uid != config.principal.uid {
        return error_reply(WireErrorCode::UnsupportedCapability);
    }
    let command: GrantControlCommand = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return error_reply(WireErrorCode::SemanticInvalidity),
    };
    if command.protocol_version != 1
        || command.subject_id != config.subject_id
        || command.authority_ref != config.principal.principal_ref
        || !config
            .grants
            .iter()
            .any(|item| item.grant_ref == command.grant_ref)
        || lifecycle_core::CommandId::parse(command.command_id.clone()).is_err()
    {
        return error_reply(WireErrorCode::SemanticInvalidity);
    }
    let Ok(expected) = command.expected_revision.parse::<u64>() else {
        return error_reply(WireErrorCode::SemanticInvalidity);
    };
    if expected.to_string() != command.expected_revision {
        return error_reply(WireErrorCode::SemanticInvalidity);
    }
    let Ok(subject) = SubjectId::parse(command.subject_id.clone()) else {
        return error_reply(WireErrorCode::SemanticInvalidity);
    };
    let Ok(grant) = GrantId::parse(command.grant_ref.clone()) else {
        return error_reply(WireErrorCode::SemanticInvalidity);
    };
    let canonical = serde_json::to_vec(&command).expect("typed grant command serializes");
    let result = if read_only {
        store.readback(
            &command.command_id,
            &subject,
            &command.authority_ref,
            &grant,
            Revision::new(expected),
            &canonical,
        )
    } else {
        store
            .contract(
                &command.command_id,
                &subject,
                &command.authority_ref,
                &grant,
                Revision::new(expected),
                &canonical,
                clock_sample(),
            )
            .map(Some)
    };
    match result {
        Ok(Some(revision)) => json!({
            "protocol_version":1,
            "command_id":command.command_id,
            "accepted":true,
            "grant_ref":command.grant_ref,
            "resulting_revision":revision.get().to_string(),
            "revoked":true,
        }),
        Ok(None) => json!({"status":"unknown"}),
        Err(StoreError::ClaimConflict) => error_reply(WireErrorCode::Conflict),
        Err(StoreError::Rejected) => error_reply(WireErrorCode::SemanticInvalidity),
        Err(_) => error_reply(WireErrorCode::ObservationUnavailable),
    }
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
    recovery_only: bool,
    verifier_gate: Arc<engine::VerifierGate>,
    native_session: Option<NativeSessionEntry>,
    native_turn: Option<NativeTurnEntry>,
    native_snapshot: Option<PublishedSnapshot>,
    native_launch: Option<NativeLaunch>,
    native_sender: mpsc::Sender<NativeMessage>,
    native_messages: mpsc::Receiver<NativeMessage>,
    native_task_work: Option<mpsc::Sender<NativeTaskWork>>,
    native_work_pending: bool,
    native_successor_requested: Option<String>,
    native_segment_pending: Option<NativeSegmentIntent>,
    native_task: Option<tokio::task::JoinHandle<()>>,
    native_gate: Arc<AtomicBool>,
    native_exit_recorded: bool,
    native_close_unsafe: bool,
    material_port: Option<Arc<dyn CanonicalMaterialPort>>,
    task_operation_port: Option<Arc<dyn TaskOperationPort>>,
}

struct NativeSegmentIntent {
    queue_attempt: AttemptId,
    segment_no: u64,
    predecessor: Option<String>,
    transition: Option<TransitionId>,
}

fn reconcile_pending_task_commits(
    store: &mut SqliteLifecycleStore,
    port: &dyn TaskOperationPort,
    subject: &SubjectId,
    session: &NativeSessionId,
) -> Result<(), StoreError> {
    for pending in store.pending_native_task_operations(subject, session)? {
        let entry: crate::ValidatedTaskOperationEntry =
            serde_json::from_slice(&pending.entry_json).map_err(|_| StoreError::ClaimConflict)?;
        let entry_digest = format!("{:x}", Sha256::digest(&pending.entry_json));
        if entry_digest != pending.entry_sha256
            || entry.operation_id != pending.operation_id
            || entry.contract_id != pending.contract_id
            || entry.payload_sha256 != pending.payload_sha256
            || entry.subject_id != subject.as_str()
            || entry.grant_ref != pending.grant_id
            || entry.grant_revision != pending.grant_revision
        {
            return Err(StoreError::ClaimConflict);
        }
        let Some(commit) = port
            .lookup_operation(&pending.operation_id)
            .map_err(|_| StoreError::Unavailable)?
        else {
            continue;
        };
        if commit.operation_id != pending.operation_id
            || commit.task_id != entry.task_id
            || commit.owner_ref != entry.task_id
            || commit.kind != pending.contract_id
            || Some(commit.owner_revision) != entry.expected_task_revision.checked_add(1)
            || commit.sha256.len() != 64
            || !commit.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(StoreError::ClaimConflict);
        }
        let bytes = serde_json::to_vec(&commit).map_err(|_| StoreError::Unavailable)?;
        store.commit_native_task_operation(&pending.operation_id, &bytes, clock_sample())?;
    }
    Ok(())
}

impl Actor {
    fn ensure_semantic_owner_current(
        &mut self,
        transition: &TransitionId,
    ) -> Result<(), StoreError> {
        let package = self
            .store
            .semantic_checkpoint(transition)?
            .ok_or(StoreError::Rejected)?;
        let port = self.material_port.as_ref().ok_or(StoreError::Rejected)?;
        let observed = port
            .observe_dependencies(&package.basis.dependencies)
            .map_err(|_| StoreError::Rejected)?;
        if !lifecycle_semantic::revalidate_dependencies(&package.basis.dependencies, &observed)
            .map_err(|_| StoreError::Rejected)?
            .is_empty()
        {
            self.store.mark_semantic_capture_stale(
                &package.basis.capture_id,
                &package.basis.sha256,
                clock_sample(),
            )?;
            return Err(StoreError::Rejected);
        }
        Ok(())
    }

    async fn drive_transition(&mut self) -> Result<(), StoreError> {
        let semantic_runtime = match (
            self.material_port.as_ref(),
            self.config.native.as_ref(),
            self.native_snapshot.as_ref(),
        ) {
            (Some(port), Some(native), Some(snapshot)) if native.semantic.is_some() => {
                let semantic = if self.config.profile == "retained-repository-analysis-v1" {
                    let subject = SubjectId::parse(self.config.subject_id.clone())
                        .map_err(|_| StoreError::Rejected)?;
                    let projection = self.store.subject_projection(&subject, None, None)?;
                    if projection.owner_kind == "transition"
                        && projection.context_id != self.config.context_generation
                    {
                        let matches: Vec<_> = native
                            .semantic_contexts
                            .iter()
                            .filter(|cycle| cycle.predecessor_context == projection.context_id)
                            .collect();
                        matches
                            .as_slice()
                            .first()
                            .filter(|_| matches.len() == 1)
                            .ok_or(StoreError::Rejected)?
                            .semantic
                            .clone()
                    } else {
                        native.semantic.clone().ok_or(StoreError::Rejected)?
                    }
                } else {
                    native.semantic.clone().ok_or(StoreError::Rejected)?
                };
                Some(crate::semantic::SemanticRuntime {
                    material_port: port.clone(),
                    native: native.clone(),
                    semantic,
                    incarnation: self.incarnation.clone(),
                    snapshot: snapshot.id.clone(),
                    snapshot_manifest_sha256: snapshot.manifest_sha256.clone(),
                    gate: self.native_gate.clone(),
                    uid: self.config.principal.uid,
                    principal: self.config.principal.principal_ref.clone(),
                    proof_dir: self.config.proof_barrier_dir.clone(),
                })
            }
            _ => None,
        };
        if self
            .config
            .native
            .as_ref()
            .is_some_and(|native| native.semantic.is_some())
            && semantic_runtime.is_none()
        {
            return Err(StoreError::Rejected);
        }
        engine::drive_transition(
            &mut self.store,
            &self.config,
            &self.verifier_gate,
            semantic_runtime.as_ref(),
        )
        .await?;
        if self.config.profile == "retained-repository-analysis-v1" {
            let subject = SubjectId::parse(self.config.subject_id.clone())
                .map_err(|_| StoreError::Rejected)?;
            let projection = self.store.subject_projection(&subject, None, None)?;
            if let Some(transition) = projection.transition
                && transition.stage == "switching"
                && self.native_successor_requested.as_deref() != Some(&transition.transition_id)
            {
                let sender = self.native_task_work.as_ref().ok_or(StoreError::Rejected)?;
                sender
                    .send(NativeTaskWork::StartSuccessor)
                    .await
                    .map_err(|_| StoreError::Unavailable)?;
                self.native_successor_requested = Some(transition.transition_id);
            }
        }
        Ok(())
    }

    pub fn new(
        config: ServiceConfig,
        material_port: Option<Arc<dyn CanonicalMaterialPort>>,
        task_operation_port: Option<Arc<dyn TaskOperationPort>>,
    ) -> Result<Self, ServiceError> {
        if config.profile == "retained-repository-analysis-v1"
            && (material_port.is_none() || task_operation_port.is_none())
        {
            return Err(ServiceError::Configuration);
        }
        if config.profile == "retained-repository-analysis-v1" {
            let native = config.native.as_ref().ok_or(ServiceError::Configuration)?;
            let initial = native
                .semantic
                .as_ref()
                .ok_or(ServiceError::Configuration)?;
            let successor_grant = |semantic: &crate::SemanticNativeConfig| {
                semantic
                    .successor_control_grant_ref
                    .as_ref()
                    .is_some_and(|s| !s.is_empty())
                    && semantic
                        .successor_control_grant_revision
                        .is_some_and(|r| r > 0)
            };
            if !successor_grant(initial) {
                return Err(ServiceError::Configuration);
            }
            let mut contexts = std::collections::BTreeSet::new();
            contexts.insert(config.context_generation.as_str());
            for cycle in &native.semantic_contexts {
                if ContextGeneration::parse(cycle.predecessor_context.clone()).is_err()
                    || !contexts.insert(cycle.predecessor_context.as_str())
                    || !successor_grant(&cycle.semantic)
                {
                    return Err(ServiceError::Configuration);
                }
            }
        }
        if config
            .native
            .as_ref()
            .is_some_and(|native| native.semantic.is_some())
            && material_port.is_none()
        {
            return Err(ServiceError::Configuration);
        }
        if cfg!(feature = "native-sim-proof") {
            operations::optional_native_barrier(
                config.proof_barrier_dir.as_deref(),
                "startup_before_store_open",
            )?;
        }
        let mut store =
            SqliteLifecycleStore::open(&config.store_root, config.trusted_issuer.clone())?;
        if cfg!(feature = "native-sim-proof") {
            operations::optional_native_barrier(
                config.proof_barrier_dir.as_deref(),
                "startup_before_recovery",
            )?;
        }
        let recovery = store.load_recovery()?;
        if cfg!(feature = "native-sim-proof") {
            operations::optional_native_barrier(
                config.proof_barrier_dir.as_deref(),
                "startup_after_recovery",
            )?;
        }
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
        let projection = store.subject_projection(&subject, None, None)?;
        let recovery_only = if config.profile == "retained-repository-analysis-v1" {
            let native = config.native.as_ref().ok_or(ServiceError::Configuration)?;
            let session = NativeSessionId::parse(native.session_id.clone())
                .map_err(|_| ServiceError::Configuration)?;
            if let Some(prior) = store.native_session_recovery(&session)? {
                if prior.subject_id != config.subject_id
                    || prior.context_id != config.context_generation
                {
                    return Err(ServiceError::Configuration);
                }
                true
            } else {
                false
            }
        } else {
            false
        };
        if recovery_only {
            let session = NativeSessionId::parse(
                config
                    .native
                    .as_ref()
                    .ok_or(ServiceError::Configuration)?
                    .session_id
                    .clone(),
            )
            .map_err(|_| ServiceError::Configuration)?;
            reconcile_pending_task_commits(
                &mut store,
                task_operation_port
                    .as_deref()
                    .ok_or(ServiceError::Configuration)?,
                &subject,
                &session,
            )?;
        }
        if !recovery_only
            && config
                .native
                .as_ref()
                .is_some_and(|native| native.semantic.is_some())
            && let Some(transition) = projection.transition.as_ref()
        {
            let id = TransitionId::parse(transition.transition_id.clone())
                .map_err(|_| ServiceError::Configuration)?;
            if store.semantic_transition_required(&id)?
                && matches!(
                    transition.stage.as_str(),
                    "switching" | "rehydrating" | "ready_to_commit"
                )
            {
                let package = store
                    .semantic_checkpoint(&id)?
                    .ok_or(ServiceError::Configuration)?;
                let observed = material_port
                    .as_ref()
                    .ok_or(ServiceError::Configuration)?
                    .observe_dependencies(&package.basis.dependencies)?;
                if !lifecycle_semantic::revalidate_dependencies(
                    &package.basis.dependencies,
                    &observed,
                )
                .map_err(|_| ServiceError::Configuration)?
                .is_empty()
                {
                    store.mark_semantic_capture_stale(
                        &package.basis.capture_id,
                        &package.basis.sha256,
                        clock_sample(),
                    )?;
                    return Err(ServiceError::Configuration);
                }
            }
        }
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
        let (native_session, native_snapshot, native_launch) = if !recovery_only
            && matches!(
                config.profile.as_str(),
                "native_simulated" | "retained-repository-analysis-v1"
            ) {
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
            recovery_only,
            verifier_gate: Arc::new(engine::VerifierGate::new()),
            native_session,
            native_turn: None,
            native_snapshot,
            native_launch,
            native_sender,
            native_messages,
            native_task_work: None,
            native_work_pending: false,
            native_successor_requested: None,
            native_segment_pending: None,
            native_task: None,
            native_gate,
            native_exit_recorded: false,
            native_close_unsafe: recovery_only,
            material_port,
            task_operation_port,
        })
    }

    pub async fn serve(mut self, mut receiver: mpsc::Receiver<Request>) {
        if matches!(
            self.config.profile.as_str(),
            "native_simulated" | "retained-repository-analysis-v1"
        ) && let (Some(config), Some(launch)) =
            (self.config.native.clone(), self.native_launch.take())
        {
            let transport = format!(
                "transport:{}:{}",
                config.session_id,
                self.incarnation.as_str()
            );
            let sender = self.native_sender.clone();
            let gate = self.native_gate.clone();
            let task_work = if self.config.profile == "retained-repository-analysis-v1" {
                let (tx, rx) = mpsc::channel(8);
                self.native_task_work = Some(tx);
                Some(rx)
            } else {
                None
            };
            self.native_task = Some(tokio::spawn(async move {
                operations::run_native(config, launch, transport, sender, gate, task_work).await;
            }));
        }
        if self.config.profile == "controlled" {
            let _ = self.drive_transition().await;
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
                        Some(Request::CommandReadback { subject,command_id,request_digest,reply }) => {
                            let result=self.command_readback(&subject,&command_id,&request_digest);
                            let _=reply.send(result);
                        }
                        Some(Request::SubjectRevision { subject,reply }) => {
                            let result=self.subject_revision(&subject);
                            let _=reply.send(result);
                        }
                        Some(Request::NativeDeliveryStatus { subject,reply }) => {
                            let result=self.native_delivery_status(&subject);
                            let _=reply.send(result);
                        }
                        Some(Request::GrantControl { uid,bytes,read_only,reply }) => {
                            let result=self.grant_control(uid,&bytes,read_only);
                            let _=reply.send(result);
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
                let reserve_successor = (|| {
                    if self.config.profile != "retained-repository-analysis-v1" {
                        return Ok(());
                    }
                    let subject = SubjectId::parse(self.config.subject_id.clone())
                        .map_err(|_| StoreError::Rejected)?;
                    let projection = self.store.subject_projection(&subject, None, None)?;
                    let Some(transition) = projection.transition else {
                        return Ok(());
                    };
                    if transition.stage != "switching" {
                        return Ok(());
                    }
                    let id = TransitionId::parse(transition.transition_id)
                        .map_err(|_| StoreError::Rejected)?;
                    let context = self
                        .store
                        .task_successor_intent(&id)?
                        .ok_or(StoreError::Rejected)?;
                    let session = self.native_session.as_ref().ok_or(StoreError::Rejected)?;
                    let snapshot = self.native_snapshot.as_ref().ok_or(StoreError::Rejected)?;
                    let native = self.config.native.as_ref().ok_or(StoreError::Rejected)?;
                    let successor = self.store.reserve_native_session(
                        NativeSessionId::parse(format!("session:successor:{}", id.as_str()))
                            .map_err(|_| StoreError::Rejected)?,
                        subject,
                        ContextGeneration::parse(context).map_err(|_| StoreError::Rejected)?,
                        self.incarnation.clone(),
                        session.profile_digest(),
                        NATIVE_BINARY_SHA256,
                        SCHEMA_THREAD_START_SHA256,
                        snapshot.id.clone(),
                        &snapshot.manifest_sha256,
                        native.same_native_session_required,
                        clock_sample(),
                    )?;
                    self.native_session = Some(successor);
                    Ok(())
                })();
                if let Err(error) = reserve_successor {
                    let _ = reply.send(Err(error));
                    return;
                }
                let result = ProviderThreadId::parse(thread)
                    .map_err(|_| StoreError::Rejected)
                    .and_then(|thread| {
                        self.store.bind_native_thread(
                            self.native_session.as_mut().ok_or(StoreError::Rejected)?,
                            thread,
                        )
                    });
                let result = match result {
                    Ok(()) => self.drive_transition().await.and_then(|()| {
                        let subject = SubjectId::parse(self.config.subject_id.clone())
                            .map_err(|_| StoreError::Rejected)?;
                        let projection = self.store.subject_projection(&subject, None, None)?;
                        if projection.owner_kind != "transition" {
                            return Ok(None);
                        }
                        let transition = projection.transition.ok_or(StoreError::Unavailable)?;
                        if transition.stage != "rehydrating" {
                            return Err(StoreError::Rejected);
                        }
                        let id = lifecycle_core::TransitionId::parse(transition.transition_id)
                            .map_err(|_| StoreError::Unavailable)?;
                        if self
                            .config
                            .native
                            .as_ref()
                            .and_then(|n| n.semantic.as_ref())
                            .is_some()
                        {
                            self.ensure_semantic_owner_current(&id)?;
                            self.store
                                .semantic_checkpoint(&id)?
                                .map(|package| package.successor_prompt)
                                .ok_or(StoreError::Rejected)
                                .map(Some)
                        } else {
                            self.store
                                .committed_text_for_transition(&id)?
                                .map(|value| value.final_text)
                                .ok_or(StoreError::Rejected)
                                .map(Some)
                        }
                    }),
                    Err(error) => Err(error),
                };
                if matches!(result, Ok(None))
                    && self.config.profile == "retained-repository-analysis-v1"
                {
                    let _ = self.schedule_native_once().await;
                }
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
                    if purpose == lifecycle_core::NativeInvocationPurpose::SuccessorRehydrate
                        && self
                            .config
                            .native
                            .as_ref()
                            .is_some_and(|native| native.semantic.is_some())
                    {
                        let subject = SubjectId::parse(self.config.subject_id.clone())
                            .map_err(|_| StoreError::Rejected)?;
                        let projection = self.store.subject_projection(&subject, None, None)?;
                        let transition = projection.transition.ok_or(StoreError::Rejected)?;
                        let id = TransitionId::parse(transition.transition_id)
                            .map_err(|_| StoreError::Rejected)?;
                        self.ensure_semantic_owner_current(&id)?;
                    }
                    let native = self.config.native.as_ref().ok_or(StoreError::Rejected)?;
                    let session = self.native_session.as_ref().ok_or(StoreError::Rejected)?;
                    let (grant_ref, grant_revision) =
                        if self.config.profile == "retained-repository-analysis-v1" {
                            let matched: Vec<_> = self
                                .config
                                .grants
                                .iter()
                                .filter(|grant| {
                                    grant.scope == "native_turn"
                                        && grant.context_generation == session.context().as_str()
                                })
                                .collect();
                            let grant = matched
                                .as_slice()
                                .first()
                                .filter(|_| matched.len() == 1)
                                .ok_or(StoreError::Rejected)?;
                            (grant.grant_ref.clone(), grant.revision)
                        } else {
                            (native.turn_grant_ref.clone(), native.turn_grant_revision)
                        };
                    let entry = self.store.claim_native_turn(
                        session,
                        NativeTurnRequest {
                            invocation_id,
                            attempt: AttemptId::parse(attempt_id)
                                .map_err(|_| StoreError::Rejected)?,
                            grant: GrantId::parse(grant_ref).map_err(|_| StoreError::Rejected)?,
                            grant_revision,
                            purpose,
                            prompt: prompt.into_bytes(),
                        },
                        &self.config.principal.principal_ref,
                        clock_sample(),
                    )?;
                    if self.config.profile == "retained-repository-analysis-v1"
                        && purpose == lifecycle_core::NativeInvocationPurpose::DomainWork
                    {
                        let segment = self
                            .native_segment_pending
                            .as_ref()
                            .ok_or(StoreError::Rejected)?;
                        self.store.bind_native_delivery_segment(
                            entry.invocation_id(),
                            &segment.queue_attempt,
                            segment.segment_no,
                            segment.predecessor.as_deref(),
                            segment.transition.as_ref(),
                        )?;
                        self.native_segment_pending = None;
                    }
                    self.store.mark_native_send_intent(&entry)?;
                    self.native_turn = Some(entry);
                    self.native_work_pending = false;
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
                    if self.config.profile == "retained-repository-analysis-v1" {
                        self.settle_native_delivery(&final_text).await
                    } else {
                        self.drive_transition().await
                    }
                } else {
                    result
                };
                if result.is_ok() {
                    self.native_turn = None;
                }
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

    /// The lifecycle queue remains the sole scheduler. The native runner gets
    /// only an already-committed entry with its immutable envelope.
    async fn schedule_native_once(&mut self) -> Result<(), StoreError> {
        if self.native_work_pending {
            return Ok(());
        }
        if self
            .native_session
            .as_ref()
            .and_then(NativeSessionEntry::thread)
            .is_none()
        {
            return Ok(());
        }
        let subject =
            SubjectId::parse(self.config.subject_id.clone()).map_err(|_| StoreError::Rejected)?;
        let plan = match self.store.prepare_next_input(&subject) {
            Ok(plan) => plan,
            Err(StoreError::ClaimConflict | StoreError::Rejected) => return Ok(()),
            Err(error) => return Err(error),
        };
        let entry = match self.store.claim_prepared_entry(
            &plan.effect,
            self.incarnation.clone(),
            clock_sample(),
        ) {
            Ok(entry) => entry,
            Err(StoreError::ClaimConflict | StoreError::Rejected) => return Ok(()),
            Err(error) => return Err(error),
        };
        let lifecycle_core::EffectInput::ControlledText(input) = entry.input();
        let turn = crate::NativeFollowupTurn {
            invocation_id: format!(
                "delivery:{}:{}",
                entry.effect().as_str(),
                entry.attempt().as_str()
            ),
            attempt_id: format!("native-attempt:delivery:{}:0", entry.attempt().as_str()),
            prompt: input.text().to_owned(),
        };
        self.native_segment_pending = Some(NativeSegmentIntent {
            queue_attempt: entry.attempt().clone(),
            segment_no: 0,
            predecessor: None,
            transition: None,
        });
        self.native_work_pending = true;
        self.native_task_work
            .as_ref()
            .ok_or(StoreError::Rejected)?
            .send(NativeTaskWork::RunTurn(turn))
            .await
            .map_err(|_| StoreError::Unavailable)
    }

    /// Native terminal history is first settled under its own identity, then
    /// joined to the exact lifecycle attempt before any later FIFO entry.
    async fn settle_native_delivery(&mut self, final_text: &str) -> Result<(), StoreError> {
        use lifecycle_core::{EffectObservation, EffectSettlement, EvidenceId, ExecutionOutcome};
        let turn = self.native_turn.as_ref().ok_or(StoreError::Rejected)?;
        let invocation_id = turn.invocation_id().to_owned();
        let binding = self.store.native_delivery_binding(&invocation_id)?;
        let yielded = if binding.is_some() {
            self.store
                .settle_native_delivery_segment(&invocation_id, clock_sample())?
        } else {
            false
        };
        if let Some(binding) = binding.as_ref() {
            if binding.native_attempt_id != turn.attempt().as_str() {
                return Err(StoreError::ObservationMismatch);
            }
            let source = EvidenceId::parse(format!("native:{invocation_id}"))
                .map_err(|_| StoreError::Rejected)?;
            if !yielded {
                let observation = EffectObservation {
                    source: source.clone(),
                    effect: lifecycle_core::EffectId::parse(binding.effect_id.clone())
                        .map_err(|_| StoreError::Rejected)?,
                    attempt: AttemptId::parse(binding.attempt_id.clone())
                        .map_err(|_| StoreError::Rejected)?,
                    incarnation: self.incarnation.clone(),
                    provider_thread: turn.thread().clone(),
                    provider_turn: turn.turn().ok_or(StoreError::ClaimConflict)?.clone(),
                    outcome: ExecutionOutcome::Completed,
                    settlement: EffectSettlement::Established(source.clone()),
                };
                self.store.record_owned_task_result(
                    &observation.attempt,
                    &self.incarnation,
                    "completed",
                    true,
                    Some(0),
                    Some(clock_sample().wall),
                    Some(&source),
                    Some(final_text),
                )?;
                self.store
                    .apply_bound_result(observation, Some(final_text), clock_sample())?;
            }
        }
        self.drive_transition().await?;
        if binding.is_some() && !yielded {
            self.schedule_native_once().await?;
        } else if binding.is_none() {
            self.schedule_native_resume().await?;
        }
        Ok(())
    }

    async fn schedule_native_resume(&mut self) -> Result<(), StoreError> {
        if self.native_work_pending || self.native_segment_pending.is_some() {
            return Ok(());
        }
        if self
            .native_session
            .as_ref()
            .and_then(NativeSessionEntry::thread)
            .is_none()
        {
            return Ok(());
        }
        let subject =
            SubjectId::parse(self.config.subject_id.clone()).map_err(|_| StoreError::Rejected)?;
        let Some(candidate) = self.store.native_resume_candidate(&subject)? else {
            return Ok(());
        };
        let queue_attempt = AttemptId::parse(candidate.queue_attempt_id.clone())
            .map_err(|_| StoreError::Rejected)?;
        let transition = TransitionId::parse(candidate.transition_id.clone())
            .map_err(|_| StoreError::Rejected)?;
        let turn = crate::NativeFollowupTurn {
            invocation_id: format!(
                "delivery:{}:{}:{}",
                candidate.effect_id, candidate.queue_attempt_id, candidate.next_segment_no
            ),
            attempt_id: format!(
                "native-attempt:delivery:{}:{}",
                candidate.queue_attempt_id, candidate.next_segment_no
            ),
            prompt: candidate.text,
        };
        self.native_segment_pending = Some(NativeSegmentIntent {
            queue_attempt,
            segment_no: candidate.next_segment_no,
            predecessor: Some(candidate.predecessor_invocation_id),
            transition: Some(transition),
        });
        self.native_work_pending = true;
        self.native_task_work
            .as_ref()
            .ok_or(StoreError::Rejected)?
            .send(NativeTaskWork::RunTurn(turn))
            .await
            .map_err(|_| StoreError::Unavailable)
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
                    Ok(Some(call))
                        if self.config.profile == "retained-repository-analysis-v1"
                            && call.namespace.as_deref() == Some("task")
                            && match call.tool.as_str() {
                                "question_open" => {
                                    serde_json::from_value::<crate::QuestionOpenPayload>(
                                        call.arguments.clone(),
                                    )
                                    .is_ok()
                                }
                                "result_publish" => {
                                    serde_json::from_value::<crate::ResultPublishPayload>(
                                        call.arguments.clone(),
                                    )
                                    .is_ok()
                                }
                                _ => false,
                            } =>
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
                    Some("dynamicToolCall")
                        if self.config.profile == "retained-repository-analysis-v1"
                            && value.is_some_and(|i| {
                                i["namespace"] == "task"
                                    && match i["tool"].as_str() {
                                        Some("question_open") => {
                                            serde_json::from_value::<crate::QuestionOpenPayload>(
                                                i["arguments"].clone(),
                                            )
                                            .is_ok()
                                        }
                                        Some("result_publish") => {
                                            serde_json::from_value::<crate::ResultPublishPayload>(
                                                i["arguments"].clone(),
                                            )
                                            .is_ok()
                                        }
                                        _ => false,
                                    }
                            }) =>
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
        if self.config.profile == "retained-repository-analysis-v1"
            && call.namespace.as_deref() == Some("task")
        {
            return self.execute_native_task_tool(event, call);
        }
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
        if self.config.profile == "retained-repository-analysis-v1" {
            let binding = self
                .store
                .native_delivery_binding(parent.invocation_id())?
                .ok_or(StoreError::Rejected)?;
            let port = self
                .task_operation_port
                .as_ref()
                .ok_or(StoreError::Rejected)?;
            let input = port
                .validate_input(&binding.input_id, &binding.text)
                .map_err(|_| StoreError::Rejected)?;
            let cursor = port
                .current_cursor(&input.task_id)
                .map_err(|_| StoreError::Unavailable)?;
            if cursor.task_id != input.task_id || cursor.subject_id != self.config.subject_id {
                return Err(StoreError::ObservationMismatch);
            }
            port.record_completed_effect(&crate::CompletedTaskEffect {
                task_id: input.task_id,
                logical_delivery_id: binding.delivery_id,
                effect_id: result.attempt.as_str().to_owned(),
                receipt_sha256: result.sha256.clone(),
                expected_cursor_revision: cursor.cursor_revision,
            })
            .map_err(|_| StoreError::Unavailable)?;
        }
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

    fn execute_native_task_tool(
        &mut self,
        event: &lifecycle_codex::NativeEvent,
        call: &lifecycle_codex::NativeToolCall,
    ) -> Result<Value, StoreError> {
        use crate::{
            QuestionOpenPayload, ResultPublishPayload, TaskOperationError,
            ValidatedTaskOperationEntry,
        };
        if self.native_gate.load(Ordering::SeqCst) {
            return Err(StoreError::Rejected);
        }
        let parent = self.native_turn.as_ref().ok_or(StoreError::Rejected)?;
        let turn = parent.turn().ok_or(StoreError::ClaimConflict)?;
        if call.thread_id != parent.thread().as_str() || call.turn_id != turn.as_str() {
            return Err(StoreError::ClaimConflict);
        }
        let binding = self
            .store
            .native_delivery_binding(parent.invocation_id())?
            .ok_or(StoreError::Rejected)?;
        let (contract, scope, question, result, payload_json) = match call.tool.as_str() {
            "question_open" => {
                let payload: QuestionOpenPayload = serde_json::from_value(call.arguments.clone())
                    .map_err(|_| StoreError::Rejected)?;
                let bytes = serde_json::to_vec(&payload).map_err(|_| StoreError::Rejected)?;
                (
                    "task.question.open.v1",
                    "task_question_open",
                    Some(payload),
                    None,
                    bytes,
                )
            }
            "result_publish" => {
                let payload: ResultPublishPayload = serde_json::from_value(call.arguments.clone())
                    .map_err(|_| StoreError::Rejected)?;
                let bytes = serde_json::to_vec(&payload).map_err(|_| StoreError::Rejected)?;
                (
                    "task.result.publish.v1",
                    "task_result_publish",
                    None,
                    Some(payload),
                    bytes,
                )
            }
            _ => return Err(StoreError::Rejected),
        };
        let port = self
            .task_operation_port
            .as_ref()
            .ok_or(StoreError::Rejected)?
            .clone();
        let input = port
            .validate_input(&binding.input_id, &binding.text)
            .map_err(|_| StoreError::Rejected)?;
        if input.subject_id != self.config.subject_id {
            return Err(StoreError::ObservationMismatch);
        }
        let session = self.native_session.as_ref().ok_or(StoreError::Rejected)?;
        if session.context().as_str() != binding.context_id {
            return Err(StoreError::ObservationMismatch);
        }
        let grants: Vec<_> = self
            .config
            .grants
            .iter()
            .filter(|grant| grant.scope == scope && grant.context_generation == binding.context_id)
            .collect();
        let grant = grants
            .as_slice()
            .first()
            .filter(|_| grants.len() == 1)
            .ok_or(StoreError::Rejected)?;
        let operation_id = format!(
            "taskop:{:x}",
            Sha256::digest(format!("{}:{}", parent.invocation_id(), call.call_id).as_bytes())
        );
        let prior_entry = self.store.native_task_operation_entry(&operation_id)?;
        let entry = if let Some(bytes) = prior_entry {
            let entry: ValidatedTaskOperationEntry =
                serde_json::from_slice(&bytes).map_err(|_| StoreError::Unavailable)?;
            if entry.operation_id != operation_id
                || entry.task_id != input.task_id
                || entry.subject_id != self.config.subject_id
                || entry.logical_delivery_id != binding.delivery_id
                || entry.attempt_id != binding.native_attempt_id
                || entry.grant_ref != grant.grant_ref
                || entry.grant_revision != grant.revision
                || entry.payload_sha256 != format!("{:x}", Sha256::digest(&payload_json))
                || entry.contract_id != contract
            {
                return Err(StoreError::ClaimConflict);
            }
            entry
        } else {
            let cursor = port
                .current_cursor(&input.task_id)
                .map_err(|_| StoreError::Unavailable)?;
            if cursor.subject_id != input.subject_id
                || cursor.task_id != input.task_id
                || cursor.cursor_ref != input.task_id
            {
                return Err(StoreError::ObservationMismatch);
            }
            ValidatedTaskOperationEntry {
                operation_id: operation_id.clone(),
                task_id: input.task_id,
                subject_id: self.config.subject_id.clone(),
                logical_delivery_id: binding.delivery_id,
                attempt_id: binding.native_attempt_id,
                cursor_ref: cursor.cursor_ref,
                cursor_revision: cursor.cursor_revision,
                expected_task_revision: cursor.owner_revision,
                grant_ref: grant.grant_ref.clone(),
                grant_revision: grant.revision,
                payload_sha256: format!("{:x}", Sha256::digest(&payload_json)),
                contract_id: contract.into(),
            }
        };
        let entry_json = serde_json::to_vec(&entry).map_err(|_| StoreError::Rejected)?;
        let request = NativeTaskOperationRequest {
            operation_id: operation_id.clone(),
            call: NativeToolCallId::parse(call.call_id.clone())
                .map_err(|_| StoreError::Rejected)?,
            transport_session: event.transport_session.clone(),
            sequence: event.sequence,
            contract_id: contract.into(),
            implementation_id: if question.is_some() {
                "rust.question.open.v1"
            } else {
                "rust.result.publish.v1"
            }
            .into(),
            executor_id: "lifecycle.service.task".into(),
            grant: GrantId::parse(grant.grant_ref.clone()).map_err(|_| StoreError::Rejected)?,
            grant_revision: grant.revision,
            payload_json,
            entry_json,
        };
        let claim = self.store.claim_native_task_operation(
            parent,
            &request,
            &self.config.principal.principal_ref,
            clock_sample(),
        )?;
        let commit_json = match claim {
            NativeTaskOperationClaim::Committed(bytes) => bytes,
            NativeTaskOperationClaim::EnteredUncertain => {
                let commit = port
                    .lookup_operation(&operation_id)
                    .map_err(|_| StoreError::Unavailable)?
                    .ok_or(StoreError::Unavailable)?;
                let bytes = serde_json::to_vec(&commit).map_err(|_| StoreError::Unavailable)?;
                self.store
                    .commit_native_task_operation(&operation_id, &bytes, clock_sample())?;
                bytes
            }
            NativeTaskOperationClaim::Entered => {
                operations::optional_native_barrier(
                    self.config.proof_barrier_dir.as_deref(),
                    "task_before_owner_commit",
                )?;
                let subject =
                    SubjectId::parse(entry.subject_id.clone()).map_err(|_| StoreError::Rejected)?;
                let context = ContextGeneration::parse(binding.context_id)
                    .map_err(|_| StoreError::Rejected)?;
                let grant_id =
                    GrantId::parse(entry.grant_ref.clone()).map_err(|_| StoreError::Rejected)?;
                let principal = self.config.principal.principal_ref.clone();
                let mut fence = |commit: &mut dyn FnMut() -> Result<(), TaskOperationError>| {
                    self.store
                        .with_task_grant_fence(
                            &subject,
                            &context,
                            &grant_id,
                            entry.grant_revision,
                            &principal,
                            scope,
                            clock_sample(),
                            &mut || {
                                commit().map_err(|error| match error {
                                    TaskOperationError::Busy | TaskOperationError::Uncertain => {
                                        StoreError::Unavailable
                                    }
                                    _ => StoreError::Rejected,
                                })
                            },
                        )
                        .map_err(|error| match error {
                            StoreError::Unavailable => TaskOperationError::Busy,
                            _ => TaskOperationError::Unauthorized,
                        })
                };
                let commit = if let Some(payload) = question {
                    port.open_question(&entry, payload, &mut fence)
                } else {
                    port.publish_result(&entry, result.ok_or(StoreError::Rejected)?, &mut fence)
                }
                .map_err(|error| match error {
                    TaskOperationError::Busy | TaskOperationError::Uncertain => {
                        StoreError::Unavailable
                    }
                    _ => StoreError::Rejected,
                })?;
                let bytes = serde_json::to_vec(&commit).map_err(|_| StoreError::Unavailable)?;
                operations::optional_native_barrier(
                    self.config.proof_barrier_dir.as_deref(),
                    "task_after_owner_commit",
                )?;
                self.store
                    .commit_native_task_operation(&operation_id, &bytes, clock_sample())?;
                bytes
            }
        };
        let text = std::str::from_utf8(&commit_json).map_err(|_| StoreError::Unavailable)?;
        Ok(json!({"success":true,"contentItems":[{"type":"inputText","text":text}]}))
    }

    async fn shutdown(&mut self) -> Result<(), StoreError> {
        self.native_gate.store(true, Ordering::SeqCst);
        // The task-profile runner can be idle after a settled delivery while
        // retaining the receiver for later FIFO work. Closing its sole sender
        // wakes it so it can terminate and reap the owned native child.
        self.native_task_work.take();
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
        if self.config.profile == "retained-repository-analysis-v1"
            && let Command::EnqueueInput { .. } = &parsed.admission.request().command
        {
            let Some(input) = parsed.admission.input() else {
                return error_reply(WireErrorCode::SemanticInvalidity);
            };
            let Some(port) = self.task_operation_port.as_ref() else {
                return error_reply(WireErrorCode::UnsupportedCapability);
            };
            let binding = match port.validate_input(input.input_id().as_str(), input.text()) {
                Ok(binding) => binding,
                Err(_) => return error_reply(WireErrorCode::SemanticInvalidity),
            };
            if binding.input_id != input.input_id().as_str()
                || binding.subject_id != self.config.subject_id
                || binding.text_sha256 != format!("{:x}", Sha256::digest(input.text().as_bytes()))
            {
                return error_reply(WireErrorCode::SemanticInvalidity);
            }
        }
        let prior = match self.store.lookup_checked_command(&parsed.admission) {
            Ok(value) => value,
            Err(StoreError::ClaimConflict) => return error_reply(WireErrorCode::Conflict),
            Err(_) => return error_reply(WireErrorCode::ObservationUnavailable),
        };
        if self.recovery_only && prior.is_none() {
            return error_reply(WireErrorCode::ObservationUnavailable);
        }
        let replayed = prior.is_some();
        let reservation = if self.config.profile != "retained-repository-analysis-v1"
            && !replayed
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
                if self.config.profile == "retained-repository-analysis-v1" {
                    let _ = self.schedule_native_once().await;
                } else {
                    let _ = self.schedule_once(reservation).await;
                }
            }
            if outcome_kind == "replacement_requested" {
                let _ = self.drive_transition().await;
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

    fn command_readback(&self, subject: &str, command_id: &str, digest: &str) -> Value {
        if subject != self.config.subject_id {
            return error_reply(WireErrorCode::SemanticInvalidity);
        }
        let subject = match SubjectId::parse(subject.to_owned()) {
            Ok(value) => value,
            Err(_) => return error_reply(WireErrorCode::SemanticInvalidity),
        };
        match self.store.command_result_readback(
            &self.config.principal.principal_ref,
            &subject,
            command_id,
            digest,
        ) {
            Ok(Some(result)) => {
                let (accepted, outcome_kind, outcome_ref, rejection_code) = match result.outcome {
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
                serde_json::to_value(CommandResultV2 {
                    protocol_version: 2,
                    command_id: command_id.to_owned(),
                    accepted,
                    outcome_kind: outcome_kind.into(),
                    outcome_ref,
                    rejection_code,
                    resulting_revision: result.revision.get().to_string(),
                })
                .expect("result serializes")
            }
            Ok(None) => json!({"status":"unknown"}),
            Err(StoreError::ClaimConflict) => error_reply(WireErrorCode::Conflict),
            Err(StoreError::Rejected) => error_reply(WireErrorCode::SemanticInvalidity),
            Err(_) => error_reply(WireErrorCode::ObservationUnavailable),
        }
    }

    fn subject_revision(&mut self, subject: &str) -> Value {
        if subject != self.config.subject_id {
            return error_reply(WireErrorCode::SemanticInvalidity);
        }
        let Ok(subject_id) = SubjectId::parse(subject.to_owned()) else {
            return error_reply(WireErrorCode::SemanticInvalidity);
        };
        match self.store.subject_projection(&subject_id, None, None) {
            Ok(value) => json!({"protocol_version":1,"subject_id":value.subject_id,
                "store_id":value.store_id,"context_generation":value.context_id,
                "revision":value.revision.to_string()}),
            Err(_) => error_reply(WireErrorCode::ObservationUnavailable),
        }
    }

    fn native_delivery_status(&self, subject: &str) -> Value {
        if subject != self.config.subject_id {
            return error_reply(WireErrorCode::SemanticInvalidity);
        }
        let Ok(subject) = SubjectId::parse(subject.to_owned()) else {
            return error_reply(WireErrorCode::SemanticInvalidity);
        };
        native_observation_reply(
            self.store.native_delivery_status(&subject),
            self.recovery_only,
        )
    }

    fn grant_control(&mut self, uid: u32, bytes: &[u8], read_only: bool) -> Value {
        if self.recovery_only && !read_only {
            return error_reply(WireErrorCode::ObservationUnavailable);
        }
        grant_control_on_store(&mut self.store, &self.config, uid, bytes, read_only)
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
                self.drive_transition().await?;
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

fn native_observation_reply(
    result: Result<lifecycle_store::NativeDeliveryObservation, StoreError>,
    recovery_only: bool,
) -> Value {
    match result {
        Ok(lifecycle_store::NativeDeliveryObservation::Complete(mut value)) => {
            value["recovery_only"] = json!(recovery_only);
            value
        }
        Ok(lifecycle_store::NativeDeliveryObservation::CapacityExceeded { limit }) => {
            let mut value = error_reply(WireErrorCode::ObservationUnavailable);
            value["reason"] = json!("capacity_exceeded");
            value["limit"] = json!(limit);
            value
        }
        Err(_) => error_reply(WireErrorCode::ObservationUnavailable),
    }
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

pub fn start(
    config: ServiceConfig,
    material_port: Option<Arc<dyn CanonicalMaterialPort>>,
    task_operation_port: Option<Arc<dyn TaskOperationPort>>,
) -> Result<(ActorSender, ShutdownGate, Option<GrantControlHandle>), ServiceError> {
    let actor = Actor::new(config, material_port, task_operation_port)?;
    let grant_control =
        if actor.config.profile == "retained-repository-analysis-v1" && !actor.recovery_only {
            Some(actor.store.grant_control_handle()?)
        } else {
            None
        };
    let gate = ShutdownGate {
        executor: actor.executor.clone(),
        verifier: actor.verifier_gate.clone(),
        native: actor.native_gate.clone(),
    };
    let (sender, receiver) = mpsc::channel(64);
    tokio::spawn(actor.serve(receiver));
    Ok((sender, gate, grant_control))
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    #[test]
    fn native_capacity_is_only_reported_for_the_typed_owner_outcome() {
        use lifecycle_store::NativeDeliveryObservation as O;
        let empty = native_observation_reply(Ok(O::Complete(json!({"segments":[]}))), false);
        assert_eq!(empty["segments"], json!([]));
        assert_eq!(empty["recovery_only"], false);
        let full = native_observation_reply(Ok(O::CapacityExceeded { limit: 64 }), false);
        assert_eq!(full["reason"], "capacity_exceeded");
        assert_eq!(full["limit"], 64);
        assert!(full.get("segments").is_none());
        let unavailable = native_observation_reply(Err(StoreError::Unavailable), false);
        assert_eq!(unavailable["code"], "observation_unavailable");
        assert!(unavailable.get("reason").is_none());
    }
}
