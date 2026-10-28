use std::{
    collections::HashSet,
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
    ExecutionReservation, LifecycleExecutor, NativeChildExit, OwnedExecutor, ProviderFailure,
    ResultReceiver, TaskResult, TaskTermination,
};
use lifecycle_store::{
    GrantControlHandle, RecoveryDisposition, SqliteLifecycleStore, StoreError, SubjectProjection,
};
use lifecycle_store::{
    NativeIngressApply, NativeOperationClaim, NativeRestartIntent, NativeSessionEntry,
    NativeTaskOperationClaim, NativeTaskOperationRequest, NativeTurnEntry, NativeTurnRequest,
    SnapshotReadRequest,
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

fn apply_current_semantic_material_block(
    store: &SqliteLifecycleStore,
    subject: &SubjectId,
    transition: &TransitionId,
    capture_id: &str,
    semantic_revision: u64,
    projection: &mut lifecycle_store::TransitionProjection,
) -> Result<(), StoreError> {
    match store.current_semantic_material_block(
        subject,
        transition,
        capture_id,
        semantic_revision,
    )? {
        Some(lifecycle_store::CurrentSemanticMaterialBlock::PromptTooLarge) => {
            projection.semantic_material_block = Some("semantic_prompt_too_large".into());
        }
        Some(lifecycle_store::CurrentSemanticMaterialBlock::CapturedIssues(issues)) => {
            projection.semantic_material_block = Some("semantic_source_basis_ineligible".into());
            projection.semantic_basis_issues = issues;
        }
        None => {}
    }
    Ok(())
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
    // Only an agentMessage item already started in this bound turn may emit
    // text deltas. Deltas are transport evidence, never task results.
    native_agent_message_items: HashSet<(String, String, String)>,
    native_work_pending: bool,
    native_successor_requested: Option<String>,
    native_segment_pending: Option<NativeSegmentIntent>,
    native_task: Option<tokio::task::JoinHandle<()>>,
    native_gate: Arc<NativeGate>,
    native_exit_recorded: bool,
    native_close_unsafe: bool,
    material_port: Option<Arc<dyn CanonicalMaterialPort>>,
    task_operation_port: Option<Arc<dyn TaskOperationPort>>,
    task_effect_hold: Option<String>,
    /// Supplied by trusted composition, never by configuration. When present,
    /// compile and verify calls of a semantic checkpoint go through it.
    direct_control: Option<Arc<dyn lifecycle_semantic::DirectInferencePort>>,
    /// Set while a prepared direct call waits for its owner release. Only the
    /// actor's guarded re-drive reads it; it is never durable state.
    direct_pending: crate::semantic_direct::DirectPendingSlot,
}

/// How often a transition waiting on a direct owner release is re-driven.
const DIRECT_RELEASE_REDRIVE: std::time::Duration = std::time::Duration::from_secs(2);

const TASK_EFFECT_HOLD: &str = "committed-native/owner-attribution-unresolved";

/// Attribution can reopen after durable owner readback; shutdown cannot.
/// Keeping the reasons independent makes a concurrent shutdown close monotonic.
pub(crate) struct NativeGate {
    attribution: AtomicBool,
    shutdown: AtomicBool,
}

impl NativeGate {
    pub(crate) fn new(attribution_held: bool) -> Self {
        Self {
            attribution: AtomicBool::new(attribution_held),
            shutdown: AtomicBool::new(false),
        }
    }

    pub(crate) fn load(&self, order: Ordering) -> bool {
        self.attribution.load(order) || self.shutdown.load(order)
    }

    fn store_attribution(&self, held: bool) {
        self.attribution.store(held, Ordering::SeqCst);
    }

    fn close_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}

/// Reconcile every committed task snapshot read for this subject against the
/// retained owner's durable readback, including reads in older context epochs.
pub fn task_effect_hold(
    store: &SqliteLifecycleStore,
    port: &dyn TaskOperationPort,
    subject: &SubjectId,
) -> Result<Option<String>, StoreError> {
    task_effect_hold_for_reads(store.committed_task_snapshot_reads(subject)?, port, subject)
}

fn task_effect_hold_for_reads(
    reads: Vec<lifecycle_store::NativeCommittedTaskRead>,
    port: &dyn TaskOperationPort,
    subject: &SubjectId,
) -> Result<Option<String>, StoreError> {
    for native in reads {
        let input = match port.validate_input(&native.input_id, &native.input_text) {
            Ok(input) if input.subject_id == subject.as_str() => input,
            _ => {
                return Ok(Some(format!(
                    "{TASK_EFFECT_HOLD}:{}",
                    native.operation_attempt_id
                )));
            }
        };
        let owner = match port.lookup_completed_effect(&input.task_id, &native.operation_attempt_id)
        {
            Ok(Some(owner)) => owner,
            _ => {
                return Ok(Some(format!(
                    "{TASK_EFFECT_HOLD}:{}",
                    native.operation_attempt_id
                )));
            }
        };
        if owner.task_id != input.task_id
            || owner.logical_delivery_id != native.delivery_id
            || owner.effect_id != native.operation_attempt_id
            || owner.receipt_sha256 != native.result_sha256
        {
            return Ok(Some(format!(
                "{TASK_EFFECT_HOLD}:{}",
                native.operation_attempt_id
            )));
        }
        if let Some(a) = owner.attribution
            && (a.parent_invocation_id != native.parent_invocation_id
                || a.call_id != native.call_id
                || a.contract_id != native.contract_id
                || a.snapshot_id != native.snapshot_id
                || a.member_id != native.member_id
                || a.offset != native.offset
                || a.requested_length != native.requested_length
                || a.returned_length != native.returned_length
                || a.result_sha256 != native.result_sha256)
        {
            return Ok(Some(format!(
                "{TASK_EFFECT_HOLD}:{}",
                native.operation_attempt_id
            )));
        }
        // A migrated v1 row proves only its old delivery/effect/digest tuple.
    }
    Ok(None)
}

/// The installed 0.163 app-server emits this notification between an
/// agentMessage start and completion. It has no settlement authority. Require
/// the exact observed envelope and the item already bound to this turn.
pub(crate) fn agent_message_delta_matches(
    value: &Value,
    thread: &str,
    turn: &str,
    started_items: &HashSet<(String, String, String)>,
) -> bool {
    let Some(envelope) = value.as_object() else {
        return false;
    };
    if envelope.len() != 3
        || envelope.get("method").and_then(Value::as_str) != Some("item/agentMessage/delta")
        || envelope
            .get("emittedAtMs")
            .and_then(Value::as_u64)
            .is_none()
    {
        return false;
    }
    let Some(params) = envelope.get("params").and_then(Value::as_object) else {
        return false;
    };
    if params.len() != 4
        || params.get("threadId").and_then(Value::as_str) != Some(thread)
        || params.get("turnId").and_then(Value::as_str) != Some(turn)
    {
        return false;
    }
    let Some(item_id) = params.get("itemId").and_then(Value::as_str) else {
        return false;
    };
    let Some(delta) = params.get("delta").and_then(Value::as_str) else {
        return false;
    };
    !item_id.is_empty()
        && item_id.len() <= 256
        && started_items.contains(&(thread.to_owned(), turn.to_owned(), item_id.to_owned()))
        && !delta.is_empty()
        && delta.len() <= 65_536
}

/// Pinned app-server 0.163 MCP startup status is advisory setup metadata.
/// It never settles a turn or establishes a usable tool catalog.
pub(crate) fn mcp_startup_status_matches(value: &Value, owned_thread: Option<&str>) -> bool {
    let Some(envelope) = value.as_object() else {
        return false;
    };
    if envelope.get("method").and_then(Value::as_str) != Some("mcpServer/startupStatus/updated")
        || envelope
            .keys()
            .any(|key| !matches!(key.as_str(), "method" | "params" | "emittedAtMs"))
        || envelope
            .get("emittedAtMs")
            .is_some_and(|timestamp| timestamp.as_u64().is_none())
    {
        return false;
    }
    let Some(params) = envelope.get("params").and_then(Value::as_object) else {
        return false;
    };
    if params.len() != 5
        || params.keys().any(|key| {
            !matches!(
                key.as_str(),
                "threadId" | "name" | "status" | "error" | "failureReason"
            )
        })
    {
        return false;
    }
    let thread_matches = match params.get("threadId") {
        Some(Value::Null) => true,
        Some(Value::String(thread)) => {
            !thread.is_empty() && owned_thread.is_some_and(|owned| thread == owned)
        }
        _ => false,
    };
    let name_ok = params
        .get("name")
        .and_then(Value::as_str)
        .is_some_and(|name| !name.is_empty() && name.len() <= 256);
    let status = params.get("status").and_then(Value::as_str);
    let status_ok = matches!(status, Some("starting" | "ready" | "failed" | "cancelled"));
    let error_ok = match params.get("error") {
        Some(Value::Null) => true,
        Some(Value::String(error)) => status == Some("failed") && error.len() <= 4096,
        _ => false,
    };
    let failure_reason_ok = match params.get("failureReason") {
        Some(Value::Null) => true,
        Some(Value::String(reason)) => {
            status == Some("failed") && reason == "reauthenticationRequired"
        }
        _ => false,
    };
    thread_matches && name_ok && status_ok && error_ok && failure_reason_ok
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

/// Walk the exact restart lineage, not only the immediately previous native
/// session. An older owner operation may have committed before its lifecycle
/// acknowledgement even when later execution generations were selected.
fn reconcile_predecessor_task_commits(
    store: &mut SqliteLifecycleStore,
    port: &dyn TaskOperationPort,
    subject: &SubjectId,
    intent: &NativeRestartIntent,
) -> Result<(), StoreError> {
    let mut predecessor = intent.predecessor_session.clone();
    for _ in 0..64 {
        reconcile_pending_task_commits(store, port, subject, &predecessor)?;
        let Some(epoch) = store.native_restart_authorization(&predecessor)? else {
            return Ok(());
        };
        let Some(prior) = epoch.predecessor_session_id else {
            return Ok(());
        };
        if prior == predecessor.as_str() {
            return Err(StoreError::ClaimConflict);
        }
        predecessor = NativeSessionId::parse(prior).map_err(|_| StoreError::ClaimConflict)?;
    }
    Err(StoreError::Unavailable)
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

    /// The semantic runtime the engine hands to `establish_checkpoint`: the one
    /// place the control realization is selected. Direct only when trusted
    /// composition supplied a port; otherwise native, exactly as before.
    pub(crate) fn semantic_runtime(
        &mut self,
    ) -> Result<Option<crate::semantic::SemanticRuntime>, StoreError> {
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
                let realization = match &self.direct_control {
                    Some(port) => crate::semantic::ControlRealization::Direct(
                        crate::semantic_direct::DirectRealization::for_semantic(
                            port.clone(),
                            &semantic,
                            self.direct_pending.clone(),
                        ),
                    ),
                    None => crate::semantic::ControlRealization::Native,
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
                    realization,
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
        Ok(semantic_runtime)
    }

    fn direct_release_pending(&self) -> bool {
        self.direct_pending
            .lock()
            .map(|slot| slot.is_some())
            .unwrap_or(false)
    }

    /// The marker is cleared first; a drive that still finds the release
    /// missing sets it again, and any other outcome leaves it clear.
    async fn redrive_direct_release(&mut self) {
        if let Ok(mut slot) = self.direct_pending.lock() {
            *slot = None;
        }
        let _ = self.drive_transition().await;
    }

    async fn drive_transition(&mut self) -> Result<(), StoreError> {
        if self.task_effect_hold.is_some() {
            return Err(StoreError::ClaimConflict);
        }
        let semantic_runtime = self.semantic_runtime()?;
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
        preopened_store: Option<SqliteLifecycleStore>,
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
        let mut store = match preopened_store {
            Some(store) => store,
            None => SqliteLifecycleStore::open(&config.store_root, config.trusted_issuer.clone())?,
        };
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
        let selected_restart = config.selected_restart.as_ref().map(|grant| &grant.intent);
        let recovery_only = if config.profile == "retained-repository-analysis-v1" {
            let native = config.native.as_ref().ok_or(ServiceError::Configuration)?;
            let session = NativeSessionId::parse(native.session_id.clone())
                .map_err(|_| ServiceError::Configuration)?;
            if let Some(intent) = selected_restart {
                if intent.subject != subject
                    || intent.context.as_str() != config.context_generation
                    || intent.target_session != session
                {
                    return Err(ServiceError::Configuration);
                }
                let epoch = store
                    .native_restart_authorization(&session)?
                    .ok_or(ServiceError::Configuration)?;
                if epoch.intent_sha256.as_deref() != Some(&intent.intent_sha256)
                    || epoch.selection_sha256.as_deref() != Some(&intent.selection_sha256)
                    || epoch.predecessor_session_id.as_deref()
                        != Some(intent.predecessor_session.as_str())
                {
                    return Err(ServiceError::Configuration);
                }
            }
            if let Some(prior) = store.native_session_recovery(&session)? {
                if prior.subject_id != config.subject_id
                    || prior.context_id != config.context_generation
                {
                    return Err(ServiceError::Configuration);
                }
                true
            } else {
                if let Some(intent) = selected_restart
                    && store
                        .native_restart_authorization(&intent.target_session)?
                        .is_none_or(|epoch| epoch.state != "authorized")
                {
                    return Err(ServiceError::Configuration);
                }
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
            if let Some(intent) = selected_restart {
                reconcile_predecessor_task_commits(
                    &mut store,
                    task_operation_port
                        .as_deref()
                        .ok_or(ServiceError::Configuration)?,
                    &subject,
                    intent,
                )?;
            }
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
        let task_effect_hold = if config.profile == "retained-repository-analysis-v1" {
            task_effect_hold(
                &store,
                task_operation_port
                    .as_deref()
                    .ok_or(ServiceError::Configuration)?,
                &subject,
            )?
        } else {
            None
        };
        let (executor, results) = OwnedExecutor::new(
            ControlledPort {
                socket: config.peer_socket_path.clone(),
            },
            config.max_executions,
        )?;
        let (native_sender, native_messages) = mpsc::channel(32);
        let native_gate = Arc::new(NativeGate::new(task_effect_hold.is_some()));
        let (native_session, native_snapshot, native_launch) = if !recovery_only
            && task_effect_hold.is_none()
            && matches!(
                config.profile.as_str(),
                "native_simulated" | "retained-repository-analysis-v1"
            ) {
            let native = config.native.as_ref().ok_or(ServiceError::Configuration)?;
            if native.followup_turns.len() > 3 {
                return Err(ServiceError::Configuration);
            }
            let selected = selected_restart.is_some();
            let mut launch = if config.profile == "retained-repository-analysis-v1" {
                if selected {
                    operations::preview_native_task_launch(native, config.principal.uid)?
                } else {
                    operations::prepare_native_task_launch(native, config.principal.uid)?
                }
            } else if selected {
                operations::preview_native_launch(native, config.principal.uid)?
            } else {
                operations::prepare_native_launch(native, config.principal.uid)?
            };
            let snapshot = workspace_snapshot_read::publish(
                &mut store,
                &subject,
                &native.snapshot,
                clock_sample(),
            )?;
            let session = if let Some(intent) = selected_restart {
                if intent.profile_digest != launch.profile_digest
                    || intent.binary_sha256 != NATIVE_BINARY_SHA256
                    || intent.protocol_digest != SCHEMA_THREAD_START_SHA256
                    || intent.snapshot != snapshot.id
                    || intent.snapshot_manifest_sha256 != snapshot.manifest_sha256
                    || native.same_native_session_required
                {
                    return Err(ServiceError::Configuration);
                }
                operations::optional_native_barrier(
                    config.proof_barrier_dir.as_deref(),
                    "selected_before_session_reservation",
                )?;
                let session = store.reserve_authorized_native_session(
                    intent,
                    incarnation.clone(),
                    clock_sample(),
                )?;
                operations::optional_native_barrier(
                    config.proof_barrier_dir.as_deref(),
                    "selected_after_session_reservation",
                )?;
                reconcile_predecessor_task_commits(
                    &mut store,
                    task_operation_port
                        .as_deref()
                        .ok_or(ServiceError::Configuration)?,
                    &subject,
                    intent,
                )?;
                session
            } else {
                store.reserve_native_session(
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
                )?
            };
            if selected {
                let prepared = if config.profile == "retained-repository-analysis-v1" {
                    operations::prepare_native_task_launch(native, config.principal.uid)?
                } else {
                    operations::prepare_native_launch(native, config.principal.uid)?
                };
                if prepared.profile_digest != launch.profile_digest {
                    return Err(ServiceError::Configuration);
                }
                launch = prepared;
            }
            launch.proof_barrier_dir = config.proof_barrier_dir.clone();
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
            native_agent_message_items: HashSet::new(),
            native_work_pending: false,
            native_successor_requested: None,
            native_segment_pending: None,
            native_task: None,
            native_gate,
            native_exit_recorded: false,
            native_close_unsafe: recovery_only,
            material_port,
            task_operation_port,
            task_effect_hold,
            direct_control: None,
            direct_pending: Default::default(),
        })
    }

    /// Trusted composition selects the direct realization for semantic
    /// checkpoints. Eligibility is not decided here: the store admits a direct
    /// result only under the owner's grant at publication.
    fn with_direct_control(
        mut self,
        port: Option<Arc<dyn lifecycle_semantic::DirectInferencePort>>,
    ) -> Self {
        self.direct_control = port;
        self
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
        let mut direct_redrive = tokio::time::interval(DIRECT_RELEASE_REDRIVE);
        direct_redrive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                // A release arriving never sends by itself: the re-drive is an
                // ordinary transition drive, repeating stage, hold, gate,
                // material, capture, grant and send-boundary checks.
                _ = direct_redrive.tick(), if self.direct_release_pending() => {
                    self.redrive_direct_release().await;
                }
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
                if result.is_ok() {
                    self.native_agent_message_items.clear();
                }
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
                isolation_diagnostic,
                reply,
            } => {
                let result = record_native_done(
                    &mut self.store,
                    self.native_session.as_ref(),
                    exit.as_ref(),
                    outcome_kind,
                    isolation_diagnostic,
                    &mut self.native_exit_recorded,
                    &mut self.native_close_unsafe,
                );
                let _ = reply.send(result);
            }
        }
    }

    /// The lifecycle queue remains the sole scheduler. The native runner gets
    /// only an already-committed entry with its immutable envelope.
    async fn schedule_native_once(&mut self) -> Result<(), StoreError> {
        if self.task_effect_hold.is_some() {
            return Err(StoreError::ClaimConflict);
        }
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
        if self.task_effect_hold.is_some() {
            return Err(StoreError::ClaimConflict);
        }
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
            NativeEventKind::Notification { method } if method == "item/agentMessage/delta" => {
                if self.config.profile == "retained-repository-analysis-v1"
                    && self.native_turn.as_ref().is_some_and(|entry| {
                        entry.turn().is_some_and(|turn| {
                            agent_message_delta_matches(
                                &event.value,
                                entry.thread().as_str(),
                                turn.as_str(),
                                &self.native_agent_message_items,
                            )
                        })
                    })
                {
                    "qualified"
                } else {
                    "unsupported"
                }
            }
            NativeEventKind::Notification { method }
                if method == "thread/readState/changed"
                    && lifecycle_codex::read_state_changed_matches(&event.value) =>
            {
                "qualified"
            }
            NativeEventKind::Notification { method }
                if method == "mcpServer/startupStatus/updated"
                    && mcp_startup_status_matches(
                        &event.value,
                        self.native_session
                            .as_ref()
                            .and_then(|session| session.thread())
                            .map(|thread| thread.as_str()),
                    ) =>
            {
                "qualified"
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
        if status == "qualified"
            && applied != NativeIngressApply::ConflictFenced
            && let NativeEventKind::Notification { method } = &event.kind
            && matches!(method.as_str(), "item/started" | "item/completed")
            && params
                .and_then(|p| p.get("item"))
                .and_then(|item| item.get("type"))
                .and_then(Value::as_str)
                == Some("agentMessage")
            && let (Some(thread), Some(turn), Some(id)) = (thread, turn, item)
        {
            let key = (thread.to_owned(), turn.to_owned(), id.to_owned());
            if method == "item/started" {
                self.native_agent_message_items.insert(key);
            } else {
                self.native_agent_message_items.remove(&key);
            }
        }
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
        if self.task_effect_hold.is_some() {
            return Err(StoreError::ClaimConflict);
        }
        if self.config.profile == "retained-repository-analysis-v1"
            && call.namespace.as_deref() == Some("task")
        {
            return self.execute_native_task_tool(event, call);
        }
        if self.native_gate.load(Ordering::SeqCst)
            || self.task_effect_hold.is_some()
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
                self.store
                    .commit_snapshot_result(&entry, &bytes, clock_sample())?
            }
            NativeOperationClaim::Committed(result) => result,
            NativeOperationClaim::EnteredUncertain => return Err(StoreError::ClaimConflict),
        };
        if self.config.profile == "retained-repository-analysis-v1" {
            // The native commit has crossed stores. Every later error leaves
            // the delivery held until exact owner readback on observation or
            // restart; the current callback is never replay authority.
            self.task_effect_hold = Some(format!("{TASK_EFFECT_HOLD}:{}", result.attempt.as_str()));
            self.native_gate.store_attribution(true);
            if let Err(error) = operations::native_proof_barrier(
                self.config.proof_barrier_dir.as_deref(),
                "native_after_result_commit",
            ) {
                self.native_gate.store_attribution(true);
                return Err(error);
            }
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
            if port
                .record_completed_effect(&crate::CompletedTaskEffect {
                    task_id: input.task_id,
                    logical_delivery_id: binding.delivery_id,
                    effect_id: result.attempt.as_str().to_owned(),
                    receipt_sha256: result.sha256.clone(),
                    expected_cursor_revision: cursor.cursor_revision,
                    parent_invocation_id: parent.invocation_id().into(),
                    call_id: result.call.as_str().into(),
                    contract_id: "workspace.snapshot.read.v1".into(),
                    snapshot_id: snapshot.id.as_str().into(),
                    member_id: args.member_id.clone(),
                    offset: args.offset,
                    requested_length: args.length,
                    returned_length: result.bytes.len() as u64,
                    result_bytes: Some(result.bytes.clone()),
                })
                .is_err()
            {
                self.task_effect_hold =
                    Some(format!("{TASK_EFFECT_HOLD}:{}", result.attempt.as_str()));
                self.native_gate.store_attribution(true);
                return Err(StoreError::ClaimConflict);
            }
            operations::optional_native_barrier(
                self.config.proof_barrier_dir.as_deref(),
                "snapshot_after_owner_commit",
            )?;
            let subject = SubjectId::parse(self.config.subject_id.clone())
                .map_err(|_| StoreError::Rejected)?;
            let reads = match self.store.committed_task_snapshot_reads(&subject) {
                Ok(reads) => reads,
                Err(error) => {
                    self.native_gate.store_attribution(true);
                    return Err(error);
                }
            };
            if !reads
                .iter()
                .any(|read| read.operation_attempt_id == result.attempt.as_str())
            {
                self.native_gate.store_attribution(true);
                return Err(StoreError::ClaimConflict);
            }
            match task_effect_hold_for_reads(reads, port.as_ref(), &subject) {
                Ok(None) => {
                    self.task_effect_hold = None;
                    self.native_gate.store_attribution(false);
                }
                Ok(Some(reason)) => {
                    self.task_effect_hold = Some(reason);
                    self.native_gate.store_attribution(true);
                    return Err(StoreError::ClaimConflict);
                }
                Err(error) => {
                    self.native_gate.store_attribution(true);
                    return Err(error);
                }
            }
        } else {
            operations::native_proof_barrier(
                self.config.proof_barrier_dir.as_deref(),
                "native_after_result_commit",
            )?;
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
        self.native_gate.close_shutdown();
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
        if self.task_effect_hold.is_some()
            && prior.is_none()
            && matches!(
                parsed.admission.request().command,
                Command::EnqueueInput { .. } | Command::RequestReplacement
            )
        {
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
        let mut value = native_observation_reply(
            self.store.native_delivery_status(&subject),
            self.recovery_only,
        );
        if self.config.profile == "retained-repository-analysis-v1" {
            let observed = self
                .task_operation_port
                .as_deref()
                .ok_or(StoreError::Unavailable)
                .and_then(|port| task_effect_hold(&self.store, port, &subject));
            match observed {
                Ok(Some(reason)) => value["task_effect_hold"] = json!(reason),
                Ok(None) if self.task_effect_hold.is_some() => {
                    value["task_effect_hold"] = json!(self.task_effect_hold)
                }
                Err(_) => value["task_effect_hold"] = json!(TASK_EFFECT_HOLD),
                _ => {}
            }
        }
        value
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
        let mut projection =
            self.store
                .subject_projection(&subject, delivery.as_ref(), transition.as_ref())?;
        if self.config.profile == "retained-repository-analysis-v1"
            && let Some(current) = projection.transition.as_mut()
            && current.stage == "verifying"
            && let Some(native) = self.config.native.as_ref()
        {
            let semantic = if projection.context_id == self.config.context_generation {
                native.semantic.as_ref()
            } else {
                let matches: Vec<_> = native
                    .semantic_contexts
                    .iter()
                    .filter(|cycle| cycle.predecessor_context == projection.context_id)
                    .collect();
                if matches.len() == 1 {
                    Some(&matches[0].semantic)
                } else {
                    None
                }
            };
            if let Some(semantic) = semantic {
                let transition_id = TransitionId::parse(current.transition_id.clone())
                    .map_err(|_| StoreError::Rejected)?;
                apply_current_semantic_material_block(
                    &self.store,
                    &subject,
                    &transition_id,
                    &semantic.capture_id,
                    projection.semantic_revision,
                    current,
                )?;
            }
        }
        if self.config.profile == "retained-repository-analysis-v1" {
            let observed = self
                .task_operation_port
                .as_deref()
                .ok_or(StoreError::Unavailable)
                .and_then(|port| task_effect_hold(&self.store, port, &subject));
            let hold = match observed {
                Ok(Some(reason)) => Some(reason),
                Ok(None) => self.task_effect_hold.clone(),
                Err(_) => Some(TASK_EFFECT_HOLD.into()),
            };
            if let Some(reason) = hold {
                self.task_effect_hold = Some(reason.clone());
                self.native_gate.store_attribution(true);
                projection.task_effect_hold = Some(reason);
            }
        }
        Ok(projection)
    }
}

fn record_native_done(
    store: &mut SqliteLifecycleStore,
    session: Option<&NativeSessionEntry>,
    exit: Option<&NativeChildExit>,
    outcome_kind: &'static str,
    isolation_diagnostic: Option<lifecycle_core::IsolationDiagnostic>,
    exit_recorded: &mut bool,
    close_unsafe: &mut bool,
) -> Result<(), StoreError> {
    // A failed isolation spawn may have created a partial child. Missing exit
    // evidence cannot establish safe close, whatever diagnostic was retained.
    if exit.is_none() {
        *close_unsafe = true;
    }
    let result = if let Some(session) = session {
        store.record_native_process_exit_with_diagnostic(
            session.id(),
            exit.and_then(|e| e.pid),
            exit.and_then(|e| e.code),
            exit.is_none_or(|e| e.forced),
            outcome_kind,
            isolation_diagnostic,
            clock_sample(),
        )
    } else {
        Err(StoreError::Rejected)
    };
    if result.is_ok() {
        *exit_recorded = true;
    } else {
        *close_unsafe = true;
    }
    result
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
    native: Arc<NativeGate>,
}

impl ShutdownGate {
    pub fn close_gate(&self) {
        self.executor.close_gate();
        self.verifier.close_gate();
        self.native.close_shutdown();
    }
}

pub fn start(
    config: ServiceConfig,
    material_port: Option<Arc<dyn CanonicalMaterialPort>>,
    task_operation_port: Option<Arc<dyn TaskOperationPort>>,
    preopened_store: Option<SqliteLifecycleStore>,
    direct_control: Option<Arc<dyn lifecycle_semantic::DirectInferencePort>>,
) -> Result<(ActorSender, ShutdownGate, Option<GrantControlHandle>), ServiceError> {
    let actor = Actor::new(config, material_port, task_operation_port, preopened_store)?
        .with_direct_control(direct_control);
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
    fn bound_session_done_without_exit_records_unsafe_close() {
        let root = tempfile::tempdir().unwrap();
        let subject = SubjectId::parse("subject:synthetic-done").unwrap();
        let context = ContextGeneration::parse("context:synthetic-done").unwrap();
        let incarnation = RuntimeIncarnation::parse("incarnation:synthetic-done").unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                BuildId::parse("build:synthetic-done").unwrap(),
                ProofRunId::parse("proof:synthetic-done").unwrap(),
                clock_sample(),
            )
            .unwrap();
        let digest = "a".repeat(64);
        let mut session = store
            .reserve_native_session(
                NativeSessionId::parse("session:synthetic-done").unwrap(),
                subject.clone(),
                context,
                incarnation.clone(),
                &digest,
                &digest,
                &digest,
                lifecycle_core::SnapshotId::parse("snapshot:synthetic-done").unwrap(),
                &digest,
                false,
                clock_sample(),
            )
            .unwrap();
        store
            .bind_native_thread(
                &mut session,
                ProviderThreadId::parse("thread:synthetic-done").unwrap(),
            )
            .unwrap();
        let mut recorded = false;
        let mut unsafe_close = false;
        record_native_done(
            &mut store,
            Some(&session),
            None,
            "transport_error_spawn_isolation",
            None,
            &mut recorded,
            &mut unsafe_close,
        )
        .unwrap();
        assert!(recorded);
        assert!(unsafe_close);
        assert_eq!(
            store
                .native_session_recovery(session.id())
                .unwrap()
                .unwrap()
                .process_outcome
                .as_deref(),
            Some("transport_error_spawn_isolation")
        );
        store
            .record_service_close(
                &incarnation,
                &subject,
                &[],
                &[],
                &[],
                unsafe_close,
                clock_sample().wall,
            )
            .unwrap();
        drop(store);
        let conn = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
        let unsafe_recorded: bool = conn
            .query_row(
                "SELECT unsafe_at_close FROM service_closes WHERE incarnation_id=?1",
                [incarnation.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(unsafe_recorded);
    }

    #[test]
    fn isolation_diagnostic_survives_service_store_and_recovery() {
        let root = tempfile::tempdir().unwrap();
        let subject = SubjectId::parse("subject:synthetic-done").unwrap();
        let context = ContextGeneration::parse("context:synthetic-done").unwrap();
        let incarnation = RuntimeIncarnation::parse("incarnation:synthetic-done").unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                BuildId::parse("build:synthetic-done").unwrap(),
                ProofRunId::parse("proof:synthetic-done").unwrap(),
                clock_sample(),
            )
            .unwrap();
        let digest = "a".repeat(64);
        let mut session = store
            .reserve_native_session(
                NativeSessionId::parse("session:synthetic-done").unwrap(),
                subject.clone(),
                context,
                incarnation.clone(),
                &digest,
                &digest,
                &digest,
                lifecycle_core::SnapshotId::parse("snapshot:synthetic-done").unwrap(),
                &digest,
                false,
                clock_sample(),
            )
            .unwrap();
        store
            .bind_native_thread(
                &mut session,
                ProviderThreadId::parse("thread:synthetic-done").unwrap(),
            )
            .unwrap();
        let mut recorded = false;
        let mut unsafe_close = false;
        let diagnostic = lifecycle_core::IsolationDiagnostic {
            phase: lifecycle_core::IsolationPhase::AfterSpawn,
            reason: lifecycle_core::IsolationReason::NamespaceIdentity,
        };
        let error = lifecycle_codex::NativeHarnessError::CleanupUnobservedIsolation(diagnostic);
        let category = operations::launch_failure_category(&error);
        assert_eq!(category, "cleanup_unobserved");
        record_native_done(
            &mut store,
            Some(&session),
            None,
            category,
            error.isolation_diagnostic(),
            &mut recorded,
            &mut unsafe_close,
        )
        .unwrap();
        assert!(recorded);
        assert!(unsafe_close);
        assert_eq!(
            store
                .native_session_recovery(session.id())
                .unwrap()
                .unwrap()
                .process_outcome
                .as_deref(),
            Some("cleanup_unobserved")
        );
        let recovery = store
            .native_session_recovery(session.id())
            .unwrap()
            .unwrap();
        assert_eq!(recovery.isolation_diagnostic, Some(diagnostic));
        // Export only this entirely synthetic fixture for the separate strict
        // controller projection regression. No runtime driver is imported.
        if let Some(output) = std::env::var_os("ISOLATION_DIAGNOSTIC_FIXTURE_OUT") {
            let conn = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
            conn.execute("VACUUM INTO ?1", [output.to_str().unwrap()])
                .unwrap();
        }
        store
            .record_service_close(
                &incarnation,
                &subject,
                &[],
                &[],
                &[],
                unsafe_close,
                clock_sample().wall,
            )
            .unwrap();
        drop(store);
        let conn = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
        let unsafe_recorded: bool = conn
            .query_row(
                "SELECT unsafe_at_close FROM service_closes WHERE incarnation_id=?1",
                [incarnation.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(unsafe_recorded);
    }

    #[test]
    fn shutdown_during_owner_readback_cannot_reopen_native_gate() {
        let native = Arc::new(NativeGate::new(false));
        native.store_attribution(true); // native commit awaits owner callback/readback
        let (executor, _results) = OwnedExecutor::new(
            ControlledPort {
                socket: std::path::PathBuf::from("/tmp/s10-unused-peer"),
            },
            1,
        )
        .unwrap();
        let shutdown = ShutdownGate {
            executor,
            verifier: Arc::new(engine::VerifierGate::new()),
            native: native.clone(),
        };
        shutdown.close_gate(); // socket's pre-queued signal closure
        native.store_attribution(false); // exact owner readback succeeds afterwards
        assert!(native.load(Ordering::SeqCst));

        // The same readback permits progress when shutdown has not begun.
        let normal = NativeGate::new(true);
        normal.store_attribution(false);
        assert!(!normal.load(Ordering::SeqCst));
    }
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

#[cfg(test)]
mod native_mcp_startup_tests {
    use super::*;
    use crate::PrincipalConfig;

    fn advisory(status: &str) -> Value {
        json!({"method":"mcpServer/startupStatus/updated","params":{
            "threadId":"thread:fixture","name":"fixture-mcp","status":status,
            "error":null,"failureReason":null},"emittedAtMs":1})
    }

    #[test]
    fn pinned_mcp_startup_states_are_ingress_only() {
        for state in ["starting", "ready", "failed", "cancelled"] {
            let value = advisory(state);
            assert!(mcp_startup_status_matches(&value, Some("thread:fixture")));
            let event = lifecycle_codex::parse_event(
                "transport:fixture".into(),
                13,
                serde_json::to_vec(&value).unwrap(),
            )
            .unwrap();
            assert!(event.terminal().unwrap().is_none());
            assert!(event.tool_call().unwrap().is_none());
        }
        let mut failed = advisory("failed");
        failed["params"]["error"] = json!("decoy failure");
        failed["params"]["failureReason"] = json!("reauthenticationRequired");
        assert!(mcp_startup_status_matches(&failed, Some("thread:fixture")));
        let mut global = advisory("ready");
        global["params"]["threadId"] = Value::Null;
        assert!(mcp_startup_status_matches(&global, None));
    }

    #[test]
    fn mcp_startup_rejects_unknown_or_authoritative_shapes() {
        let base = advisory("ready");
        assert!(!mcp_startup_status_matches(&base, Some("thread:other")));
        assert!(!mcp_startup_status_matches(&base, None));
        for (path, value) in [
            ("status", json!("completed")),
            ("name", json!("")),
            ("threadId", json!(7)),
            ("error", json!("not a failure")),
            ("failureReason", json!("unknown")),
        ] {
            let mut changed = base.clone();
            changed["params"][path] = value;
            assert!(!mcp_startup_status_matches(
                &changed,
                Some("thread:fixture")
            ));
        }
        let mut extra = base.clone();
        extra["params"]["turnId"] = json!("turn:forbidden");
        assert!(!mcp_startup_status_matches(&extra, Some("thread:fixture")));
        let mut wrong_method = base.clone();
        wrong_method["method"] = json!("mcpServer/other");
        assert!(!mcp_startup_status_matches(
            &wrong_method,
            Some("thread:fixture")
        ));
        let mut bad_envelope = base.clone();
        bad_envelope["result"] = json!({"status":"completed"});
        assert!(!mcp_startup_status_matches(
            &bad_envelope,
            Some("thread:fixture")
        ));
        let mut missing = base.clone();
        missing["params"]
            .as_object_mut()
            .unwrap()
            .remove("failureReason");
        assert!(!mcp_startup_status_matches(
            &missing,
            Some("thread:fixture")
        ));
    }

    #[test]
    fn actor_retains_fake_mcp_startup_ingress_and_fences_coordinate_conflict() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        fs::create_dir(&root).unwrap();
        let mut actor = Actor::new(
            ServiceConfig {
                profile: "controlled".into(),
                store_root: root.clone(),
                socket_path: temp.path().join("service.sock"),
                peer_socket_path: temp.path().join("peer.sock"),
                verifier_executable: None,
                verifier_ledger_path: None,
                proof_barrier_dir: None,
                trusted_issuer: "issuer:fixture".into(),
                principal: PrincipalConfig {
                    uid: 0,
                    principal_ref: "principal:fixture".into(),
                    producer_ref: "producer:fixture".into(),
                },
                subject_id: "subject:fixture".into(),
                context_generation: "context:fixture".into(),
                build_id: "build:fixture".into(),
                proof_run_id: "proof:fixture".into(),
                grants: Vec::new(),
                max_executions: 1,
                max_wait_ms: 1000,
                verifier_proof_ms: None,
                verifier_cleanup_ms: None,
                native: None,
                selected_restart: None,
            },
            None,
            None,
            None,
        )
        .unwrap();
        let digest = "a".repeat(64);
        let mut session = actor
            .store
            .reserve_native_session(
                NativeSessionId::parse("session:fixture").unwrap(),
                SubjectId::parse("subject:fixture").unwrap(),
                ContextGeneration::parse("context:fixture").unwrap(),
                actor.incarnation.clone(),
                &digest,
                &digest,
                &digest,
                lifecycle_core::SnapshotId::parse("snapshot:fixture").unwrap(),
                &digest,
                false,
                clock_sample(),
            )
            .unwrap();
        actor
            .store
            .bind_native_thread(
                &mut session,
                ProviderThreadId::parse("thread:fixture").unwrap(),
            )
            .unwrap();
        actor.native_session = Some(session);

        assert!(matches!(
            actor.store.reserve_native_session(
                NativeSessionId::parse("session:fixture").unwrap(),
                SubjectId::parse("subject:fixture").unwrap(),
                ContextGeneration::parse("context:fixture").unwrap(),
                actor.incarnation.clone(),
                &"b".repeat(64),
                &digest,
                &digest,
                lifecycle_core::SnapshotId::parse("snapshot:fixture").unwrap(),
                &digest,
                false,
                clock_sample(),
            ),
            Err(StoreError::ClaimConflict)
        ));

        let mut owned = advisory("ready");
        owned["params"]["status"] = json!("failed");
        owned["params"]["error"] = json!("fixture failure");
        owned["params"]["failureReason"] = json!("reauthenticationRequired");
        let raw = serde_json::to_vec(&owned).unwrap();
        let event =
            lifecycle_codex::parse_event("transport:fixture".into(), 1, raw.clone()).unwrap();
        actor.record_native_event(&event).unwrap();
        actor.record_native_event(&event).unwrap();

        let mut global = advisory("starting");
        global["params"]["threadId"] = Value::Null;
        let global_raw = serde_json::to_vec(&global).unwrap();
        let global_event =
            lifecycle_codex::parse_event("transport:fixture".into(), 2, global_raw.clone())
                .unwrap();
        actor.record_native_event(&global_event).unwrap();

        let mut foreign = advisory("ready");
        foreign["params"]["threadId"] = json!("thread:foreign");
        let foreign_event = lifecycle_codex::parse_event(
            "transport:fixture".into(),
            3,
            serde_json::to_vec(&foreign).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            actor.record_native_event(&foreign_event),
            Err(StoreError::Rejected)
        ));

        let read_state = json!({"method":"thread/readState/changed","params":{
        "threadId":"thread:fixture","readState":{
            "revision":"revision:fixture","firstUnread":null,
            "error":"failed", "needsInput":true, "unreadCount":3
        }}});
        let read_raw = serde_json::to_vec(&read_state).unwrap();
        let read_event =
            lifecycle_codex::parse_event("transport:fixture".into(), 4, read_raw.clone()).unwrap();
        actor.record_native_event(&read_event).unwrap();
        actor.record_native_event(&read_event).unwrap();
        assert!(read_event.terminal().unwrap().is_none());
        assert!(read_event.tool_call().unwrap().is_none());
        let terminal = json!({"method":"turn/completed","params":{
            "threadId":"thread:fixture","turn":{"id":"turn:fixture","status":"completed"}}});
        let terminal_raw = serde_json::to_vec(&terminal).unwrap();
        actor
            .record_native_event(
                &lifecycle_codex::parse_event("transport:fixture".into(), 5, terminal_raw.clone())
                    .unwrap(),
            )
            .unwrap();
        let history_raw = serde_json::to_vec(&json!({"id":6,"result":{"thread":{
            "id":"thread:fixture","turns":[{"id":"turn:fixture","status":"completed",
            "items":[{"type":"agentMessage","text":"later"}]}]}}}))
        .unwrap();
        actor
            .record_native_event(
                &lifecycle_codex::parse_event("transport:fixture".into(), 6, history_raw.clone())
                    .unwrap(),
            )
            .unwrap();

        // An old stored unsupported row remains unsupported on identical raw replay.
        let old_raw = serde_json::to_vec(&json!({"method":"thread/readState/changed","params":{
            "threadId":"thread:fixture","readState":{"revision":"revision:old","status":"new"}}}))
        .unwrap();
        let session_id = actor.native_session.as_ref().unwrap().id().clone();
        assert_eq!(
            actor
                .store
                .record_native_ingress(
                    &session_id,
                    "transport:fixture",
                    7,
                    &old_raw,
                    "unsupported",
                    Some("thread/readState/changed"),
                    Some("thread:fixture"),
                    None,
                    None,
                    clock_sample()
                )
                .unwrap(),
            NativeIngressApply::Inserted
        );
        actor
            .record_native_event(
                &lifecycle_codex::parse_event("transport:fixture".into(), 7, old_raw.clone())
                    .unwrap(),
            )
            .unwrap();

        let connection = rusqlite::Connection::open(root.join("lifecycle.sqlite")).unwrap();
        let rows: Vec<(i64, Vec<u8>, String, Option<String>)> = {
            let mut query = connection
                .prepare("SELECT sequence,raw,decode_status,thread_id FROM native_ingress ORDER BY sequence")
                .unwrap();
            query
                .query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        assert_eq!(
            rows,
            vec![
                (
                    1,
                    raw.clone(),
                    "qualified".into(),
                    Some("thread:fixture".into())
                ),
                (2, global_raw, "qualified".into(), None),
                (
                    4,
                    read_raw.clone(),
                    "qualified".into(),
                    Some("thread:fixture".into())
                ),
                (
                    5,
                    terminal_raw,
                    "qualified".into(),
                    Some("thread:fixture".into())
                ),
                (6, history_raw, "qualified".into(), None),
                (
                    7,
                    old_raw,
                    "unsupported".into(),
                    Some("thread:fixture".into())
                ),
            ]
        );
        let turns: i64 = connection
            .query_row("SELECT COUNT(*) FROM native_turns", [], |row| row.get(0))
            .unwrap();
        assert_eq!(turns, 0); // The read-state extension created no work or settlement.
        let turn_bad_rows: i64 = connection.query_row(
            "SELECT COUNT(*) FROM native_ingress WHERE thread_id='thread:fixture' AND turn_id='turn:fixture' AND (method IN ('turn/failed','turn/interrupted') OR decode_status NOT IN ('qualified','denied_before_entry'))",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(turn_bad_rows, 0); // The historical unsupported read-state row has no turn coordinate.

        let conflicting = lifecycle_codex::parse_event(
            "transport:fixture".into(),
            1,
            serde_json::to_vec(&advisory("cancelled")).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            actor.record_native_event(&conflicting),
            Err(StoreError::ClaimConflict)
        ));
        let retained: Vec<u8> = connection
            .query_row(
                "SELECT raw FROM native_ingress WHERE transport_session='transport:fixture' AND sequence=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(retained, raw);
        let conflicts: i64 = connection
            .query_row("SELECT COUNT(*) FROM native_ingress_conflicts", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(conflicts, 1);
    }
}

#[cfg(test)]
mod native_agent_delta_tests {
    use super::*;

    fn observed_shape() -> Value {
        // Text and identities are substituted; envelope, method, param names,
        // and value types match the retained 0.163 live ingress at sequence 15.
        json!({"method":"item/agentMessage/delta","params":{
            "threadId":"thread:fixture","turnId":"turn:fixture",
            "itemId":"item:fixture","delta":"x"},"emittedAtMs":1})
    }

    #[test]
    fn exact_started_agent_message_delta_is_transport_only() {
        let started = HashSet::from([(
            "thread:fixture".to_owned(),
            "turn:fixture".to_owned(),
            "item:fixture".to_owned(),
        )]);
        let value = observed_shape();
        assert!(agent_message_delta_matches(
            &value,
            "thread:fixture",
            "turn:fixture",
            &started
        ));
        let event = lifecycle_codex::parse_event(
            "transport:fixture".into(),
            15,
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        assert!(event.terminal().unwrap().is_none());
        assert!(event.tool_call().unwrap().is_none());
    }

    #[test]
    fn delta_rejects_unstarted_or_completed_item_and_wrong_coordinates() {
        let mut started = HashSet::new();
        let value = observed_shape();
        assert!(!agent_message_delta_matches(
            &value,
            "thread:fixture",
            "turn:fixture",
            &started
        ));
        started.insert((
            "thread:other".to_owned(),
            "turn:fixture".to_owned(),
            "item:fixture".to_owned(),
        ));
        assert!(!agent_message_delta_matches(
            &value,
            "thread:fixture",
            "turn:fixture",
            &started
        ));
        let key = (
            "thread:fixture".to_owned(),
            "turn:fixture".to_owned(),
            "item:fixture".to_owned(),
        );
        started.insert(key.clone());
        assert!(!agent_message_delta_matches(
            &value,
            "thread:other",
            "turn:fixture",
            &started
        ));
        assert!(!agent_message_delta_matches(
            &value,
            "thread:fixture",
            "turn:other",
            &started
        ));
        started.remove(&key);
        assert!(!agent_message_delta_matches(
            &value,
            "thread:fixture",
            "turn:fixture",
            &started
        ));
    }

    #[test]
    fn delta_rejects_new_fields_missing_fields_and_nontext_payload() {
        let started = HashSet::from([(
            "thread:fixture".to_owned(),
            "turn:fixture".to_owned(),
            "item:fixture".to_owned(),
        )]);
        let valid = observed_shape();
        let mut variants = Vec::new();
        let mut value = valid.clone();
        value["params"]["extra"] = json!(true);
        variants.push(value);
        let mut value = valid.clone();
        value["extra"] = json!(true);
        variants.push(value);
        let mut value = valid.clone();
        value.as_object_mut().unwrap().remove("emittedAtMs");
        variants.push(value);
        let mut value = valid.clone();
        value["emittedAtMs"] = json!("1");
        variants.push(value);
        let mut value = valid.clone();
        value["params"]["delta"] = json!({"text":"x"});
        variants.push(value);
        let mut value = valid.clone();
        value["params"]["delta"] = json!("");
        variants.push(value);
        let mut value = valid.clone();
        value["params"]["delta"] = json!("x".repeat(65_537));
        variants.push(value);
        let mut value = valid.clone();
        value["params"]["itemId"] = json!("item:other");
        variants.push(value);
        let mut value = valid.clone();
        value["params"].as_object_mut().unwrap().remove("itemId");
        variants.push(value);
        let mut value = valid.clone();
        value["params"]["delivery"] = json!({"state":"completed"});
        variants.push(value);
        let mut value = valid.clone();
        value["method"] = json!("item/agentMessage/other");
        variants.push(value);
        for variant in variants {
            assert!(!agent_message_delta_matches(
                &variant,
                "thread:fixture",
                "turn:fixture",
                &started
            ));
        }
    }
}

#[cfg(test)]
mod restart_epoch_tests {
    use super::*;
    use crate::{
        GrantFence, MaterialPortError, NativeConfig, NativeProviderRoute, PrincipalConfig,
        QuestionOpenPayload, ResultPublishPayload, SelectedLifecycleStore, SelectedRestartGrant,
        SelectedRestartRequest, SemanticNativeConfig, TaskCursorBinding, TaskInputBinding,
        TaskOperationCommit, TaskOperationError, ValidatedTaskOperationEntry,
    };
    use lifecycle_semantic::{DependencyObservation, DependencyRevision, MaterialInput};
    use std::{
        os::unix::fs::{MetadataExt, PermissionsExt},
        path::Path,
    };

    struct EmptyMaterial;
    impl CanonicalMaterialPort for EmptyMaterial {
        fn project(
            &self,
            _: &SubjectId,
            _: &TransitionId,
            _: &str,
        ) -> Result<MaterialInput, MaterialPortError> {
            Err(MaterialPortError::Unavailable)
        }
        fn observe_dependencies(
            &self,
            _: &[DependencyRevision],
        ) -> Result<Vec<DependencyObservation>, MaterialPortError> {
            Ok(Vec::new())
        }
        fn with_current_fence(
            &self,
            _: &[DependencyRevision],
            commit: &mut dyn FnMut() -> Result<(), ServiceError>,
        ) -> Result<(), ServiceError> {
            commit()
        }
    }
    #[derive(Default)]
    pub(super) struct EmptyTask {
        input: Option<TaskInputBinding>,
        effect: Option<crate::CompletedTaskEffectReadback>,
    }
    impl TaskOperationPort for EmptyTask {
        fn lookup_completed_effect(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<crate::CompletedTaskEffectReadback>, TaskOperationError> {
            Ok(self.effect.clone())
        }
        fn validate_input(&self, _: &str, _: &str) -> Result<TaskInputBinding, TaskOperationError> {
            self.input.clone().ok_or(TaskOperationError::Invalid)
        }
        fn current_cursor(&self, _: &str) -> Result<TaskCursorBinding, TaskOperationError> {
            Err(TaskOperationError::Invalid)
        }
        fn open_question(
            &self,
            _: &ValidatedTaskOperationEntry,
            _: QuestionOpenPayload,
            _: &mut GrantFence<'_>,
        ) -> Result<TaskOperationCommit, TaskOperationError> {
            Err(TaskOperationError::Invalid)
        }
        fn publish_result(
            &self,
            _: &ValidatedTaskOperationEntry,
            _: ResultPublishPayload,
            _: &mut GrantFence<'_>,
        ) -> Result<TaskOperationCommit, TaskOperationError> {
            Err(TaskOperationError::Invalid)
        }
        fn lookup_operation(
            &self,
            _: &str,
        ) -> Result<Option<TaskOperationCommit>, TaskOperationError> {
            Ok(None)
        }
        fn record_completed_effect(
            &self,
            _: &crate::CompletedTaskEffect,
        ) -> Result<(), TaskOperationError> {
            Ok(())
        }
    }

    #[test]
    fn committed_read_requires_exact_owner_readback_across_context_epochs() {
        let subject = SubjectId::parse("subject:restart").unwrap();
        let input = TaskInputBinding {
            input_id: "input:one".into(),
            task_id: "task:one".into(),
            subject_id: subject.as_str().into(),
            cursor_ref: "task:one".into(),
            cursor_revision: 1,
            owner_revision: 1,
            text_sha256: "a".repeat(64),
        };
        let read = lifecycle_store::NativeCommittedTaskRead {
            input_id: input.input_id.clone(),
            input_text: "objective".into(),
            delivery_id: "delivery:one".into(),
            effect_id: "effect:one".into(),
            parent_invocation_id: "invocation:old-context".into(),
            call_id: "call:one".into(),
            contract_id: "workspace.snapshot.read.v1".into(),
            snapshot_id: "snapshot:one".into(),
            member_id: "m1".into(),
            offset: 0,
            requested_length: 3,
            returned_length: 3,
            result_sha256: "b".repeat(64),
            operation_attempt_id: "opattempt:one".into(),
        };
        let held = || format!("{TASK_EFFECT_HOLD}:opattempt:one");
        assert_eq!(
            task_effect_hold_for_reads(
                vec![read.clone()],
                &EmptyTask {
                    input: Some(input.clone()),
                    effect: None
                },
                &subject
            )
            .unwrap(),
            Some(held())
        );
        let exact = crate::CompletedTaskEffectReadback {
            task_id: input.task_id.clone(),
            logical_delivery_id: read.delivery_id.clone(),
            effect_id: read.operation_attempt_id.clone(),
            receipt_sha256: read.result_sha256.clone(),
            attribution: Some(crate::CompletedTaskEffectAttribution {
                parent_invocation_id: read.parent_invocation_id.clone(),
                call_id: read.call_id.clone(),
                contract_id: read.contract_id.clone(),
                snapshot_id: read.snapshot_id.clone(),
                member_id: read.member_id.clone(),
                offset: read.offset,
                requested_length: read.requested_length,
                returned_length: read.returned_length,
                result_sha256: read.result_sha256.clone(),
            }),
        };
        let port = EmptyTask {
            input: Some(input.clone()),
            effect: Some(exact.clone()),
        };
        assert_eq!(
            task_effect_hold_for_reads(vec![read.clone()], &port, &subject).unwrap(),
            None
        );
        let mut conflict = exact;
        conflict.attribution.as_mut().unwrap().returned_length += 1;
        assert_eq!(
            task_effect_hold_for_reads(
                vec![read.clone()],
                &EmptyTask {
                    input: Some(input.clone()),
                    effect: Some(conflict)
                },
                &subject
            )
            .unwrap(),
            Some(held())
        );
        let mut other_epoch = read.clone();
        other_epoch.operation_attempt_id = "opattempt:old".into();
        assert_eq!(
            task_effect_hold_for_reads(vec![other_epoch, read], &port, &subject).unwrap(),
            Some(format!("{TASK_EFFECT_HOLD}:opattempt:old"))
        );
    }

    fn private_dir(path: &Path) {
        fs::create_dir(path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }

    #[tokio::test]
    async fn selected_authorization_launches_one_actor_incarnation_then_observes_only() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        private_dir(&root);
        let prior_root = temp.path().join("native-prior");
        private_dir(&prior_root);
        let next_root = temp.path().join("native-next");
        private_dir(&next_root);
        let subject = SubjectId::parse("subject:restart").unwrap();
        let context = ContextGeneration::parse("context:restart").unwrap();
        let member_bytes = b"selected source\n".to_vec();
        let member_sha = CodecContract::BinaryArtifactV1
            .digest_binary(&member_bytes)
            .unwrap()
            .hex();
        let snapshot = crate::operations::workspace_snapshot_read::TrustedSnapshot {
            id: "snapshot:restart".into(),
            source_revision: "source:restart".into(),
            members: vec![crate::operations::workspace_snapshot_read::TrustedMember {
                id: "source".into(),
                sha256: member_sha,
                bytes: member_bytes,
            }],
        };
        let semantic = SemanticNativeConfig {
            capture_id: "capture:restart".into(),
            compile_session_id: "session:compile".into(),
            verify_session_id: "session:verify".into(),
            successor_verify_session_id: "session:successor".into(),
            compile_invocation_id: "invocation:compile".into(),
            compile_attempt_id: "attempt:compile".into(),
            candidate_verify_invocation_id: "invocation:verify".into(),
            candidate_verify_attempt_id: "attempt:verify".into(),
            successor_verify_invocation_id: "invocation:successor".into(),
            successor_verify_attempt_id: "attempt:successor".into(),
            challenge_id: "challenge:restart".into(),
            challenge_text: "offline".into(),
            control_grant_ref: "grant:semantic".into(),
            control_grant_revision: 1,
            successor_control_grant_ref: Some("grant:semantic:next".into()),
            successor_control_grant_revision: Some(1),
        };
        let mut native = NativeConfig {
            executable: temp.path().join("never-executed"),
            root: prior_root,
            provider_route: NativeProviderRoute::NativeSimProofV1,
            chatgpt_access_source: None,
            simulator_base_url: "http://127.0.0.1:12345/v1".into(),
            session_id: "session:prior".into(),
            invocation_id: "invocation:prior".into(),
            attempt_id: "attempt:prior".into(),
            prompt: "unused".into(),
            turn_grant_ref: "grant:turn".into(),
            turn_grant_revision: 1,
            snapshot_grant_ref: "grant:snapshot".into(),
            snapshot_grant_revision: 1,
            same_native_session_required: false,
            followup_turns: Vec::new(),
            snapshot,
            semantic: Some(semantic),
            semantic_contexts: Vec::new(),
        };
        let profile = operations::prepare_native_task_launch(
            &native,
            fs::metadata("/proc/self").unwrap().uid(),
        )
        .unwrap()
        .profile_digest;
        let mut store = SqliteLifecycleStore::open(&root, "issuer:restart".into()).unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                BuildId::parse("build:restart").unwrap(),
                ProofRunId::parse("proof:restart").unwrap(),
                clock_sample(),
            )
            .unwrap();
        let published = workspace_snapshot_read::publish(
            &mut store,
            &subject,
            &native.snapshot,
            clock_sample(),
        )
        .unwrap();
        let mut prior_allocation = store
            .reserve_native_session(
                NativeSessionId::parse(native.session_id.clone()).unwrap(),
                subject.clone(),
                context.clone(),
                RuntimeIncarnation::parse("incarnation:prior").unwrap(),
                &profile,
                NATIVE_BINARY_SHA256,
                SCHEMA_THREAD_START_SHA256,
                published.id,
                &published.manifest_sha256,
                false,
                clock_sample(),
            )
            .unwrap();
        let grant = GrantId::parse("grant:prior-settle").unwrap();
        store
            .install_trusted_grant(TrustedGrant {
                id: grant.clone(),
                issuer: "issuer:restart".into(),
                principal: "principal:restart".into(),
                subject: subject.clone(),
                context: context.clone(),
                build: BuildId::parse("build:restart").unwrap(),
                proof_run: ProofRunId::parse("proof:restart").unwrap(),
                scope: GrantScope::parse("native_turn").unwrap(),
                expires: AuthorityExpiresAt::new(WallTimeMs::new(
                    clock_sample().wall.get() + 100_000,
                )),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
        store
            .bind_native_thread(
                &mut prior_allocation,
                ProviderThreadId::parse("thread:prior-settle").unwrap(),
            )
            .unwrap();
        let mut prior_turn = store
            .claim_native_turn(
                &prior_allocation,
                NativeTurnRequest {
                    invocation_id: "invocation:prior-settle".into(),
                    attempt: AttemptId::parse("attempt:prior-settle").unwrap(),
                    grant,
                    grant_revision: 1,
                    purpose: lifecycle_core::NativeInvocationPurpose::DomainWork,
                    prompt: b"prior delivery".to_vec(),
                },
                "principal:restart",
                clock_sample(),
            )
            .unwrap();
        let connection = rusqlite::Connection::open(root.join("lifecycle.sqlite")).unwrap();
        connection.execute_batch("\
            INSERT INTO inputs(input_id,subject_id,sequence,delivery_id,producer_ref,text,text_digest,grant_id,state)
            VALUES('input:prior-settle','subject:restart',1,'delivery:prior-settle','producer:restart',
                   X'7072696F722064656C6976657279','digest:prior-settle','grant:prior-settle','entered');
            INSERT INTO effects(effect_id,subject_id,input_id,expected_revision,state)
            VALUES('effect:prior-settle','subject:restart','input:prior-settle',0,'entered');
            INSERT INTO attempts(attempt_id,effect_id,incarnation_id,state,entered_wall_ms)
            VALUES('queue-attempt:prior-settle','effect:prior-settle','incarnation:prior','entered',1);
        ").unwrap();
        store
            .bind_native_delivery_segment(
                prior_turn.invocation_id(),
                &AttemptId::parse("queue-attempt:prior-settle").unwrap(),
                0,
                None,
                None,
            )
            .unwrap();
        store.mark_native_send_intent(&prior_turn).unwrap();
        store
            .bind_native_turn(
                &mut prior_turn,
                ProviderTurnId::parse("turn:prior-settle").unwrap(),
            )
            .unwrap();
        let final_item = json!({"type":"agentMessage","id":"final:prior-settle","text":"done"});
        for (sequence, method, value) in [
            (
                1,
                Some("turn/completed"),
                json!({"method":"turn/completed","params":{
                "threadId":"thread:prior-settle","turn":{"id":"turn:prior-settle","status":"completed"}}}),
            ),
            (
                2,
                Some("item/completed"),
                json!({"method":"item/completed","params":{
                "threadId":"thread:prior-settle","turnId":"turn:prior-settle","item":final_item.clone()}}),
            ),
            (
                3,
                None,
                json!({"id":3,"result":{"thread":{"id":"thread:prior-settle",
                "historyMode":"legacy","turns":[{"id":"turn:prior-settle","status":"completed",
                "itemsView":"full","items":[final_item]}]}}}),
            ),
        ] {
            store
                .record_native_ingress(
                    prior_allocation.id(),
                    "transport:prior-settle",
                    sequence,
                    &serde_json::to_vec(&value).unwrap(),
                    "qualified",
                    method,
                    method.map(|_| "thread:prior-settle"),
                    method.map(|_| "turn:prior-settle"),
                    None,
                    clock_sample(),
                )
                .unwrap();
        }
        let store_id = store.admin_ownership().unwrap().store_id;
        drop(store);
        native.root = next_root;
        native.session_id = "session:next".into();
        native.invocation_id = "invocation:next".into();
        native.attempt_id = "attempt:next".into();
        let mut config = ServiceConfig {
            profile: "retained-repository-analysis-v1".into(),
            store_root: root.clone(),
            socket_path: temp.path().join("service.sock"),
            peer_socket_path: temp.path().join("peer.sock"),
            verifier_executable: None,
            verifier_ledger_path: None,
            proof_barrier_dir: None,
            trusted_issuer: "issuer:restart".into(),
            principal: PrincipalConfig {
                uid: fs::metadata("/proc/self").unwrap().uid(),
                principal_ref: "principal:restart".into(),
                producer_ref: "producer:restart".into(),
            },
            subject_id: subject.as_str().into(),
            context_generation: context.as_str().into(),
            build_id: "build:restart".into(),
            proof_run_id: "proof:restart".into(),
            grants: Vec::new(),
            max_executions: 1,
            max_wait_ms: 1000,
            verifier_proof_ms: None,
            verifier_cleanup_ms: None,
            native: Some(native),
            selected_restart: None,
        };
        let mut selected = SelectedLifecycleStore::open_for_restart(&config, &store_id).unwrap();
        let basis = selected
            .store
            .native_restart_basis(&subject, &context)
            .unwrap();
        let request = SelectedRestartRequest {
            subject_id: subject.as_str().into(),
            context_generation: context.as_str().into(),
            predecessor_session_id: basis.predecessor_session_id,
            predecessor_epoch: basis.predecessor_epoch,
            target_session_id: "session:next".into(),
            expected_store_id: store_id.clone(),
            expected_subject_revision: basis.subject_revision,
            expected_semantic_revision: basis.semantic_revision,
            expected_queue_revision: basis.queue_revision,
            expected_journal_cursor: basis.journal_cursor,
            intent_sha256: "a".repeat(64),
            selection_sha256: "b".repeat(64),
            profile_digest: basis.profile_digest,
            binary_sha256: basis.binary_sha256,
            protocol_digest: basis.protocol_digest,
            snapshot_id: basis.snapshot_id,
            snapshot_manifest_sha256: basis.snapshot_manifest_sha256,
        };
        let epoch = selected.authorize_restart(request.clone()).unwrap();
        assert_eq!(epoch.state, "authorized");
        config.selected_restart = Some(SelectedRestartGrant {
            intent: selected.restart_intent.clone().unwrap(),
        });
        let mut first = Actor::new(
            config.clone(),
            Some(Arc::new(EmptyMaterial)),
            Some(Arc::new(EmptyTask::default())),
            Some(selected.store),
        )
        .unwrap();
        assert!(!first.recovery_only);
        assert_eq!(
            first.native_session.as_ref().unwrap().id().as_str(),
            "session:next"
        );
        let active = first
            .store
            .native_restart_epoch(&subject, &context)
            .unwrap()
            .unwrap();
        assert_eq!(
            (
                active.epoch,
                active.session_id.as_str(),
                active.state.as_str()
            ),
            (1, "session:next", "active")
        );
        // Exercise selected-profile actor admission with an allocated native
        // session and an attribution hold, then inspect the persistent rows.
        first.task_effect_hold = Some("task_effect_attribution_unresolved:test".into());
        first.native_gate.store_attribution(true);
        let (reply, received) = oneshot::channel();
        first
            .handle_native_message(NativeMessage::PrepareTurn {
                invocation_id: "invocation:held-new".into(),
                attempt_id: "attempt:held-new".into(),
                prompt: "must not send".into(),
                purpose: lifecycle_core::NativeInvocationPurpose::DomainWork,
                reply,
            })
            .await;
        assert!(matches!(received.await.unwrap(), Err(StoreError::Rejected)));
        assert!(matches!(
            first.schedule_native_once().await,
            Err(StoreError::ClaimConflict)
        ));
        let event = lifecycle_codex::parse_event(
            "transport:held".into(),
            1,
            serde_json::to_vec(&json!({"method":"item/tool/call","params":{}})).unwrap(),
        )
        .unwrap();
        for namespace in ["workspace", "task"] {
            let call = lifecycle_codex::NativeToolCall {
                rpc_id: 1,
                thread_id: "thread:held".into(),
                turn_id: "turn:held".into(),
                call_id: format!("call:{namespace}"),
                namespace: Some(namespace.into()),
                tool: if namespace == "task" {
                    "result_publish"
                } else {
                    "snapshot_read"
                }
                .into(),
                arguments: json!({}),
            };
            assert!(matches!(
                first.execute_native_tool(&event, &call),
                Err(StoreError::ClaimConflict)
            ));
        }
        assert!(matches!(
            first.settle_native_delivery("must not advance").await,
            Err(StoreError::ClaimConflict)
        ));
        for table in [
            "native_turns",
            "native_operations",
            "native_task_operations",
            "native_delivery_segments",
        ] {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(
                count,
                if table == "native_turns" || table == "native_delivery_segments" {
                    1
                } else {
                    0
                },
                "{table}"
            );
        }
        // Join a real prior native turn and its existing delivery segment to
        // the held selected actor. Settlement must commit history before the
        // delivery guard refuses to advance the retained task.
        connection
            .execute(
                "UPDATE subjects SET context_id='context:later' WHERE subject_id=?1",
                [subject.as_str()],
            )
            .unwrap();
        let before: (String, i64) = connection.query_row(
            "SELECT state,segment_no FROM native_delivery_segments WHERE invocation_id='invocation:prior-settle'",
            [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(before, ("entered".into(), 0));
        let queue_before: (String, String, String) = connection.query_row(
            "SELECT i.state,e.state,a.state FROM inputs i JOIN effects e ON e.input_id=i.input_id JOIN attempts a ON a.effect_id=e.effect_id WHERE a.attempt_id='queue-attempt:prior-settle'",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
        assert_eq!(
            queue_before,
            ("entered".into(), "entered".into(), "entered".into())
        );
        first.native_turn = Some(prior_turn);
        let held_reason = first.task_effect_hold.clone();
        let (reply, received) = oneshot::channel();
        first
            .handle_native_message(NativeMessage::Settle {
                final_text: "done".into(),
                history_status: "observed_complete".into(),
                history_transport: "transport:prior-settle".into(),
                history_sequence: 3,
                reply,
            })
            .await;
        assert!(matches!(
            received.await.unwrap(),
            Err(StoreError::ClaimConflict)
        ));
        assert_eq!(
            first
                .store
                .native_recovery("invocation:prior-settle")
                .unwrap()
                .unwrap()
                .class,
            "CompletedSettled"
        );
        let settled: (String, String) = connection.query_row(
            "SELECT n.state,s.context_id FROM native_turns n JOIN native_sessions s ON s.session_id=n.session_id WHERE n.invocation_id='invocation:prior-settle'",
            [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(
            settled,
            ("completed_settled".into(), "context:restart".into())
        );
        let current_context: String = connection
            .query_row(
                "SELECT context_id FROM subjects WHERE subject_id=?1",
                [subject.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(current_context, "context:later");
        assert_eq!(first.task_effect_hold, held_reason);
        assert!(first.native_gate.load(Ordering::SeqCst));
        assert_eq!(
            first.native_turn.as_ref().unwrap().invocation_id(),
            "invocation:prior-settle"
        );
        let after: (String, i64) = connection.query_row(
            "SELECT state,segment_no FROM native_delivery_segments WHERE invocation_id='invocation:prior-settle'",
            [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(after, before);
        let queue_after: (String, String, String) = connection.query_row(
            "SELECT i.state,e.state,a.state FROM inputs i JOIN effects e ON e.input_id=i.input_id JOIN attempts a ON a.effect_id=e.effect_id WHERE a.attempt_id='queue-attempt:prior-settle'",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
        assert_eq!(queue_after, queue_before);
        for table in ["native_operations", "native_task_operations"] {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "{table}");
        }
        assert!(!first.native_work_pending);
        assert!(first.native_segment_pending.is_none());
        connection
            .execute(
                "UPDATE subjects SET context_id='context:restart' WHERE subject_id=?1",
                [subject.as_str()],
            )
            .unwrap();
        drop(first);
        let mut selected = SelectedLifecycleStore::open_for_restart(&config, &store_id).unwrap();
        assert_eq!(selected.authorize_restart(request).unwrap().state, "active");
        config.selected_restart = Some(SelectedRestartGrant {
            intent: selected.restart_intent.clone().unwrap(),
        });
        let second = Actor::new(
            config,
            Some(Arc::new(EmptyMaterial)),
            Some(Arc::new(EmptyTask::default())),
            Some(selected.store),
        )
        .unwrap();
        assert!(second.recovery_only);
        assert!(second.native_session.is_none());
        assert_eq!(
            second
                .store
                .native_session_recovery(&NativeSessionId::parse("session:next").unwrap())
                .unwrap()
                .unwrap()
                .allocation_state,
            "intent"
        );
        assert!(matches!(
            second.store.native_restart_epoch(&subject, &context),
            Ok(Some(_))
        ));
        assert!(
            second
                .store
                .native_session_recovery(&NativeSessionId::parse("session:third").unwrap())
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn held_semantic_entries_leave_capture_checkpoint_and_control_session_absent() {
        use crate::semantic::{SemanticRuntime, establish_checkpoint};
        use crate::semantic_control::{ControlTurnSpec, run_control_turn};
        use lifecycle_semantic::SemanticPhase;
        use lifecycle_store::SemanticControlRole;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        private_dir(&root);
        let native_root = temp.path().join("native");
        private_dir(&native_root);
        let mut store = SqliteLifecycleStore::open(&root, "issuer:held".into()).unwrap();
        let subject = SubjectId::parse("subject:held").unwrap();
        let context = ContextGeneration::parse("context:held").unwrap();
        let transition = TransitionId::parse("transition:held").unwrap();
        let semantic = SemanticNativeConfig {
            capture_id: "capture:held".into(),
            compile_session_id: "session:compile-held".into(),
            verify_session_id: "session:verify-held".into(),
            successor_verify_session_id: "session:successor-held".into(),
            compile_invocation_id: "invocation:compile-held".into(),
            compile_attempt_id: "attempt:compile-held".into(),
            candidate_verify_invocation_id: "invocation:verify-held".into(),
            candidate_verify_attempt_id: "attempt:verify-held".into(),
            successor_verify_invocation_id: "invocation:successor-held".into(),
            successor_verify_attempt_id: "attempt:successor-held".into(),
            challenge_id: "challenge:held".into(),
            challenge_text: "offline".into(),
            control_grant_ref: "grant:held".into(),
            control_grant_revision: 1,
            successor_control_grant_ref: None,
            successor_control_grant_revision: None,
        };
        let native = NativeConfig {
            executable: temp.path().join("never-executed"),
            root: native_root.clone(),
            provider_route: NativeProviderRoute::NativeSimProofV1,
            chatgpt_access_source: None,
            simulator_base_url: "http://127.0.0.1:12345/v1".into(),
            session_id: "session:held".into(),
            invocation_id: "invocation:held".into(),
            attempt_id: "attempt:held".into(),
            prompt: "unused".into(),
            turn_grant_ref: "grant:held".into(),
            turn_grant_revision: 1,
            snapshot_grant_ref: "grant:snapshot-held".into(),
            snapshot_grant_revision: 1,
            same_native_session_required: false,
            followup_turns: Vec::new(),
            snapshot: workspace_snapshot_read::TrustedSnapshot {
                id: "snapshot:held".into(),
                source_revision: "source:held".into(),
                members: Vec::new(),
            },
            semantic: Some(semantic.clone()),
            semantic_contexts: Vec::new(),
        };
        let gate = Arc::new(NativeGate::new(true));
        let runtime = SemanticRuntime {
            material_port: Arc::new(EmptyMaterial),
            native: native.clone(),
            semantic: semantic.clone(),
            incarnation: RuntimeIncarnation::parse("incarnation:held").unwrap(),
            snapshot: lifecycle_core::SnapshotId::parse("snapshot:held").unwrap(),
            snapshot_manifest_sha256: "0".into(),
            gate: gate.clone(),
            uid: fs::metadata("/proc/self").unwrap().uid(),
            principal: "principal:held".into(),
            proof_dir: None,
            realization: crate::semantic::ControlRealization::Native,
        };
        assert!(matches!(
            establish_checkpoint(&mut store, &runtime, &subject, &context, &transition, 1).await,
            Err(StoreError::ClaimConflict)
        ));
        let spec = ControlTurnSpec {
            subject: subject.clone(),
            context,
            transition: transition.clone(),
            incarnation: runtime.incarnation.clone(),
            snapshot: runtime.snapshot.clone(),
            snapshot_manifest_sha256: "0",
            role: SemanticControlRole::Compile,
            phase: SemanticPhase::Compile,
            session_id: "session:compile-held",
            invocation_id: "invocation:compile-held",
            attempt_id: "attempt:compile-held",
            prompt: "unused",
            policy_id: "policy:held",
            exposure_id: "exposure:held".into(),
        };
        assert!(matches!(
            run_control_turn(
                &mut store,
                &native,
                &semantic,
                spec,
                "principal:held",
                runtime.uid,
                None,
                &gate
            )
            .await,
            Err(StoreError::ClaimConflict)
        ));
        assert_eq!(store.semantic_capture_record("capture:held").unwrap(), None);
        assert!(store.semantic_checkpoint(&transition).unwrap().is_none());
        assert_eq!(fs::read_dir(&native_root).unwrap().count(), 0);
        let connection = rusqlite::Connection::open(root.join("lifecycle.sqlite")).unwrap();
        for table in [
            "semantic_captures",
            "semantic_results",
            "native_sessions",
            "native_turns",
        ] {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "{table}");
        }
    }

    #[tokio::test]
    async fn service_material_blocks_are_current_durable_and_send_nothing() {
        use crate::semantic::{SemanticRuntime, establish_checkpoint};
        use lifecycle_semantic::{
            MaterialIssue, ObligationKind, RequiredObligation, SemanticPolicy, SourceAvailability,
            SourceKind, SourceMaterial, SourceTrust, project_material, sha256_hex,
        };

        struct FixedMaterial(lifecycle_semantic::MaterialInput);
        impl CanonicalMaterialPort for FixedMaterial {
            fn project(
                &self,
                _: &SubjectId,
                _: &TransitionId,
                _: &str,
            ) -> Result<lifecycle_semantic::MaterialInput, crate::MaterialPortError> {
                Ok(self.0.clone())
            }
            fn observe_dependencies(
                &self,
                _: &[lifecycle_semantic::DependencyRevision],
            ) -> Result<Vec<lifecycle_semantic::DependencyObservation>, crate::MaterialPortError>
            {
                Ok(Vec::new())
            }
            fn with_current_fence(
                &self,
                _: &[lifecycle_semantic::DependencyRevision],
                commit: &mut dyn FnMut() -> Result<(), ServiceError>,
            ) -> Result<(), ServiceError> {
                commit()
            }
        }

        for prompt_overflow in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("store");
            private_dir(&root);
            let native_root = temp.path().join("native");
            private_dir(&native_root);
            let subject = SubjectId::parse("subject:material").unwrap();
            let context = ContextGeneration::parse("context:material").unwrap();
            let transition = TransitionId::parse("transition:material").unwrap();
            let mut store = SqliteLifecycleStore::open(&root, "issuer:material".into()).unwrap();
            store
                .register_subject(
                    subject.clone(),
                    context.clone(),
                    BuildId::parse("build:material").unwrap(),
                    ProofRunId::parse("proof:material").unwrap(),
                    clock_sample(),
                )
                .unwrap();
            let connection = rusqlite::Connection::open(root.join("lifecycle.sqlite")).unwrap();
            connection.execute("INSERT INTO transitions(transition_id,subject_id,predecessor_context,stage,basis_revision,created_wall_ms) VALUES(?1,?2,?3,'verifying',0,100)",
                rusqlite::params![transition.as_str(),subject.as_str(),context.as_str()]).unwrap();
            connection
                .execute(
                    "UPDATE transitions SET semantic_required=1 WHERE transition_id=?1",
                    [transition.as_str()],
                )
                .unwrap();
            connection.execute("UPDATE subjects SET owner_kind='transition',owner_ref=?1,semantic_revision=1 WHERE subject_id=?2",
                rusqlite::params![transition.as_str(),subject.as_str()]).unwrap();
            let bytes = vec![b'x'; 600];
            let source = |id: &str, bytes: Vec<u8>, availability| SourceMaterial {
                id: id.into(),
                owner_namespace: "task".into(),
                owner_ref: "task:material".into(),
                revision: 1,
                dependency_key: id.into(),
                current: availability == SourceAvailability::Complete,
                dependency_required: true,
                kind: SourceKind::ExternalData,
                trust: SourceTrust::UntrustedData,
                sha256: sha256_hex(if availability == SourceAvailability::Complete {
                    &bytes
                } else {
                    b"missing"
                }),
                availability,
                bytes,
            };
            let sources = if prompt_overflow {
                vec![source(
                    "result",
                    bytes.clone(),
                    SourceAvailability::Complete,
                )]
            } else {
                vec![
                    source("first", bytes.clone(), SourceAvailability::Complete),
                    source("second", bytes, SourceAvailability::Complete),
                    source("missing", Vec::new(), SourceAvailability::Unavailable),
                ]
            };
            let mut policy = SemanticPolicy::retained_repository_analysis();
            if prompt_overflow {
                policy.max_control_input_bytes = 1024;
            } else {
                policy.max_source_items = 2;
                policy.max_source_item_bytes = 512;
                policy.max_source_total_bytes = 1024;
            }
            let input = lifecycle_semantic::MaterialInput {
                subject: subject.clone(),
                transition: transition.clone(),
                capture_id: "capture:material".into(),
                obligations: vec![RequiredObligation {
                    id: "obligation:result".into(),
                    kind: ObligationKind::CompletedConsequence,
                    verifier_scope: "result".into(),
                    description: "preserve observed result".into(),
                    sources: vec![sources.last().unwrap().reference()],
                }],
                sources,
                policy,
            };
            let expected = if prompt_overflow {
                Vec::new()
            } else {
                project_material(input.clone()).unwrap().issues
            };
            let semantic = SemanticNativeConfig {
                capture_id: "capture:material".into(),
                compile_session_id: "session:compile".into(),
                verify_session_id: "session:verify".into(),
                successor_verify_session_id: "session:successor".into(),
                compile_invocation_id: "invocation:compile".into(),
                compile_attempt_id: "attempt:compile".into(),
                candidate_verify_invocation_id: "invocation:verify".into(),
                candidate_verify_attempt_id: "attempt:verify".into(),
                successor_verify_invocation_id: "invocation:successor".into(),
                successor_verify_attempt_id: "attempt:successor".into(),
                challenge_id: "challenge:material".into(),
                challenge_text: "offline".into(),
                control_grant_ref: "grant:material".into(),
                control_grant_revision: 1,
                successor_control_grant_ref: None,
                successor_control_grant_revision: None,
            };
            let native = NativeConfig {
                executable: temp.path().join("never-executed"),
                root: native_root.clone(),
                provider_route: NativeProviderRoute::NativeSimProofV1,
                chatgpt_access_source: None,
                simulator_base_url: "http://127.0.0.1:1/v1".into(),
                session_id: "session:material".into(),
                invocation_id: "invocation:material".into(),
                attempt_id: "attempt:material".into(),
                prompt: "unused".into(),
                turn_grant_ref: "grant:material".into(),
                turn_grant_revision: 1,
                snapshot_grant_ref: "grant:snapshot".into(),
                snapshot_grant_revision: 1,
                same_native_session_required: false,
                followup_turns: Vec::new(),
                snapshot: workspace_snapshot_read::TrustedSnapshot {
                    id: "snapshot:material".into(),
                    source_revision: "source:material".into(),
                    members: Vec::new(),
                },
                semantic: Some(semantic.clone()),
                semantic_contexts: Vec::new(),
            };
            let runtime = SemanticRuntime {
                material_port: Arc::new(FixedMaterial(input.clone())),
                native,
                semantic,
                incarnation: RuntimeIncarnation::parse("incarnation:material").unwrap(),
                snapshot: lifecycle_core::SnapshotId::parse("snapshot:material").unwrap(),
                snapshot_manifest_sha256: "0".into(),
                gate: Arc::new(NativeGate::new(false)),
                uid: fs::metadata("/proc/self").unwrap().uid(),
                principal: "principal:material".into(),
                proof_dir: None,
                realization: crate::semantic::ControlRealization::Native,
            };
            for _ in 0..2 {
                assert!(
                    establish_checkpoint(&mut store, &runtime, &subject, &context, &transition, 1)
                        .await
                        .unwrap()
                        .is_none()
                );
            }
            let capture = store.semantic_capture_record("capture:material").unwrap();
            let rejection = store
                .semantic_material_rejection("capture:material")
                .unwrap();
            assert_eq!(capture.is_none(), prompt_overflow);
            assert_eq!(rejection.is_some(), prompt_overflow);
            assert!(store.semantic_checkpoint(&transition).unwrap().is_none());
            let count: i64 = connection
                .query_row("SELECT COUNT(*) FROM native_sessions", [], |row| row.get(0))
                .unwrap();
            assert_eq!(count, 0);
            assert_eq!(fs::read_dir(&native_root).unwrap().count(), 0);
            drop(store);
            let mut store = SqliteLifecycleStore::open(&root, "issuer:material".into()).unwrap();
            let mut projection = store
                .subject_projection(&subject, None, Some(&transition))
                .unwrap();
            apply_current_semantic_material_block(
                &store,
                &subject,
                &transition,
                "capture:material",
                projection.semantic_revision,
                projection.transition.as_mut().unwrap(),
            )
            .unwrap();
            let block = projection.transition.as_ref().unwrap();
            assert_eq!(
                block.semantic_material_block.as_deref(),
                Some(if prompt_overflow {
                    "semantic_prompt_too_large"
                } else {
                    "semantic_source_basis_ineligible"
                })
            );
            assert_eq!(block.semantic_basis_issues, expected);
            if !prompt_overflow {
                assert!(
                    expected
                        .iter()
                        .any(|issue| matches!(issue, MaterialIssue::TooManyItems))
                );
                assert!(
                    expected
                        .iter()
                        .any(|issue| matches!(issue, MaterialIssue::ItemTooLarge { .. }))
                );
                assert!(
                    expected
                        .iter()
                        .any(|issue| matches!(issue, MaterialIssue::TotalTooLarge))
                );
                assert!(
                    expected
                        .iter()
                        .any(|issue| matches!(issue, MaterialIssue::Unavailable { .. }))
                );
                assert!(
                    expected.iter().any(|issue| matches!(
                        issue,
                        MaterialIssue::RequiredSourceUnavailable { .. }
                    ))
                );
                assert!(
                    expected
                        .iter()
                        .any(|issue| matches!(issue, MaterialIssue::NoCurrentRevision { .. }))
                );
            }
            let snapshot = crate::projection::into_snapshot(projection, true);
            assert!(matches!(
                snapshot.blocking_reason,
                lifecycle_wire::FieldV1::Known(_)
            ));
            connection
                .execute(
                    "UPDATE subjects SET semantic_revision=2 WHERE subject_id=?1",
                    [subject.as_str()],
                )
                .unwrap();
            assert!(
                store
                    .current_semantic_material_block(&subject, &transition, "capture:material", 2)
                    .unwrap()
                    .is_none()
            );
            assert!(
                store
                    .current_semantic_material_block(&subject, &transition, "capture:other", 2)
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[test]
    fn held_gate_does_not_prevent_prior_context_native_history_settlement() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        private_dir(&root);
        let mut store = SqliteLifecycleStore::open(&root, "issuer:prior".into()).unwrap();
        let subject = SubjectId::parse("subject:prior").unwrap();
        let prior = ContextGeneration::parse("context:prior").unwrap();
        let build = BuildId::parse("build:prior").unwrap();
        let proof = ProofRunId::parse("proof:prior").unwrap();
        store
            .register_subject(
                subject.clone(),
                prior.clone(),
                build.clone(),
                proof.clone(),
                clock_sample(),
            )
            .unwrap();
        let grant = GrantId::parse("grant:prior").unwrap();
        store
            .install_trusted_grant(TrustedGrant {
                id: grant.clone(),
                issuer: "issuer:prior".into(),
                principal: "principal:prior".into(),
                subject: subject.clone(),
                context: prior.clone(),
                build,
                proof_run: proof,
                scope: GrantScope::parse("native_turn").unwrap(),
                expires: AuthorityExpiresAt::new(WallTimeMs::new(
                    clock_sample().wall.get() + 100_000,
                )),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
        let hash = "a".repeat(64);
        let session_id = NativeSessionId::parse("session:prior-held").unwrap();
        let mut session = store
            .reserve_native_session(
                session_id.clone(),
                subject.clone(),
                prior,
                RuntimeIncarnation::parse("incarnation:prior-held").unwrap(),
                &hash,
                &hash,
                &hash,
                lifecycle_core::SnapshotId::parse("snapshot:prior").unwrap(),
                &hash,
                false,
                clock_sample(),
            )
            .unwrap();
        store
            .bind_native_thread(
                &mut session,
                ProviderThreadId::parse("thread:prior-held").unwrap(),
            )
            .unwrap();
        let mut turn = store
            .claim_native_turn(
                &session,
                NativeTurnRequest {
                    invocation_id: "invocation:prior-held".into(),
                    attempt: AttemptId::parse("attempt:prior-held").unwrap(),
                    grant,
                    grant_revision: 1,
                    purpose: lifecycle_core::NativeInvocationPurpose::DomainWork,
                    prompt: b"prior".to_vec(),
                },
                "principal:prior",
                clock_sample(),
            )
            .unwrap();
        store.mark_native_send_intent(&turn).unwrap();
        store
            .bind_native_turn(&mut turn, ProviderTurnId::parse("turn:prior-held").unwrap())
            .unwrap();
        let final_item = json!({"type":"agentMessage","id":"final:prior","text":"done"});
        for (sequence, method, value) in [
            (
                1,
                Some("turn/completed"),
                json!({"method":"turn/completed","params":{
                "threadId":"thread:prior-held","turn":{"id":"turn:prior-held","status":"completed"}}}),
            ),
            (
                2,
                Some("item/completed"),
                json!({"method":"item/completed","params":{
                "threadId":"thread:prior-held","turnId":"turn:prior-held","item":final_item.clone()}}),
            ),
            (
                3,
                None,
                json!({"id":3,"result":{"thread":{"id":"thread:prior-held",
                "historyMode":"legacy","turns":[{"id":"turn:prior-held","status":"completed",
                "itemsView":"full","items":[final_item]}]}}}),
            ),
        ] {
            store
                .record_native_ingress(
                    &session_id,
                    "transport:prior-held",
                    sequence,
                    &serde_json::to_vec(&value).unwrap(),
                    "qualified",
                    method,
                    method.map(|_| "thread:prior-held"),
                    method.map(|_| "turn:prior-held"),
                    None,
                    clock_sample(),
                )
                .unwrap();
        }
        let connection = rusqlite::Connection::open(root.join("lifecycle.sqlite")).unwrap();
        connection
            .execute(
                "UPDATE subjects SET context_id='context:later' WHERE subject_id=?1",
                [subject.as_str()],
            )
            .unwrap();
        let gate = NativeGate::new(true);
        assert!(gate.load(Ordering::SeqCst));
        store
            .settle_native_turn(
                &turn,
                "done",
                "observed_complete",
                "transport:prior-held",
                3,
                clock_sample(),
            )
            .unwrap();
        assert!(gate.load(Ordering::SeqCst));
        assert_eq!(
            store
                .native_recovery("invocation:prior-held")
                .unwrap()
                .unwrap()
                .class,
            "CompletedSettled"
        );
        let state: (String, String) = connection.query_row(
            "SELECT n.state,s.context_id FROM native_turns n JOIN native_sessions s ON s.session_id=n.session_id WHERE n.invocation_id='invocation:prior-held'",
            [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(state, ("completed_settled".into(), "context:prior".into()));
        let current: String = connection
            .query_row(
                "SELECT context_id FROM subjects WHERE subject_id=?1",
                [subject.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(current, "context:later");
        for table in [
            "native_delivery_segments",
            "native_operations",
            "native_task_operations",
        ] {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "{table}");
        }
    }
}

#[cfg(test)]
#[path = "actor_direct_tests.rs"]
mod direct_selection_tests;
