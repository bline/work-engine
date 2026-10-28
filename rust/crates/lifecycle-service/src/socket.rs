use std::{
    fs,
    os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use lifecycle_runtime::{BoundedFrameReader, BoundedFrameWriter};
use lifecycle_semantic::DirectInferencePort;
use lifecycle_store::{GrantControlHandle, SqliteLifecycleStore};
use lifecycle_wire::{CursorV1, WaitTargetV1, WireErrorCode, parse_strict_json};
use serde_json::{Value, json};
use tokio::{
    net::{UnixListener, UnixStream},
    signal::unix::{SignalKind, signal},
    sync::{Semaphore, mpsc, oneshot},
};

use crate::{
    CanonicalMaterialPort, ServiceConfig, ServiceError, TaskOperationPort,
    actor::{self, Request},
    projection,
};

const FRAME_MAX: usize = 131_072;
const IO_BUDGET: Duration = Duration::from_secs(5);

struct GrantLane {
    control: Mutex<GrantControlHandle>,
    material: Arc<dyn CanonicalMaterialPort>,
    open: AtomicBool,
    in_flight: AtomicUsize,
}

struct GrantInFlight(Arc<GrantLane>);
impl Drop for GrantInFlight {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

pub async fn run(
    config: ServiceConfig,
    material_port: Option<Arc<dyn CanonicalMaterialPort>>,
    task_operation_port: Option<Arc<dyn TaskOperationPort>>,
) -> Result<(), ServiceError> {
    run_inner(config, material_port, task_operation_port, None, None).await
}

/// As `run`, with the direct realization of semantic compile and verify calls
/// selected by trusted composition.
pub(crate) async fn run_with_direct_control(
    config: ServiceConfig,
    material_port: Arc<dyn CanonicalMaterialPort>,
    task_operation_port: Option<Arc<dyn TaskOperationPort>>,
    direct_control: Arc<dyn DirectInferencePort>,
) -> Result<(), ServiceError> {
    run_inner(
        config,
        Some(material_port),
        task_operation_port,
        None,
        Some(direct_control),
    )
    .await
}

pub(crate) async fn run_with_preopened_store(
    config: ServiceConfig,
    store: SqliteLifecycleStore,
) -> Result<(), ServiceError> {
    run_inner(config, None, None, Some(store), None).await
}

pub(crate) async fn run_with_preopened_task_ports(
    config: ServiceConfig,
    store: SqliteLifecycleStore,
    material_port: Arc<dyn CanonicalMaterialPort>,
    task_operation_port: Arc<dyn TaskOperationPort>,
    direct_control: Option<Arc<dyn DirectInferencePort>>,
) -> Result<(), ServiceError> {
    run_inner(
        config,
        Some(material_port),
        Some(task_operation_port),
        Some(store),
        direct_control,
    )
    .await
}

async fn run_inner(
    config: ServiceConfig,
    material_port: Option<Arc<dyn CanonicalMaterialPort>>,
    task_operation_port: Option<Arc<dyn TaskOperationPort>>,
    preopened_store: Option<SqliteLifecycleStore>,
    direct_control: Option<Arc<dyn DirectInferencePort>>,
) -> Result<(), ServiceError> {
    // The direct realization drives the semantic checkpoint compile and verify
    // calls, so it needs a configured semantic capture and its material owner.
    if direct_control.is_some()
        && (config.profile != "retained-repository-analysis-v1"
            || material_port.is_none()
            || config
                .native
                .as_ref()
                .is_none_or(|native| native.semantic.is_none()))
    {
        return Err(ServiceError::Configuration);
    }
    if config.max_executions == 0
        || config.max_executions > 16
        || config.max_wait_ms == 0
        || config.max_wait_ms > 30_000
        || config.trusted_issuer.is_empty()
        || config.principal.principal_ref.is_empty()
        || !matches!(
            config.profile.as_str(),
            "read_only" | "controlled" | "native_simulated" | "retained-repository-analysis-v1"
        )
    {
        return Err(ServiceError::Configuration);
    }
    if config.profile == "controlled" && !cfg!(feature = "controlled-proof") {
        return Err(ServiceError::Configuration);
    }
    if config.profile == "native_simulated"
        && (!cfg!(feature = "native-sim-proof") || config.native.is_none())
    {
        return Err(ServiceError::Configuration);
    }
    if config.profile == "retained-repository-analysis-v1"
        && (config.native.is_none() || material_port.is_none() || task_operation_port.is_none())
    {
        return Err(ServiceError::Configuration);
    }
    if !matches!(
        config.profile.as_str(),
        "native_simulated" | "retained-repository-analysis-v1"
    ) && config.native.is_some()
    {
        return Err(ServiceError::Configuration);
    }
    if config
        .native
        .as_ref()
        .is_some_and(|native| native.semantic.is_some())
        && material_port.is_none()
    {
        return Err(ServiceError::Configuration);
    }
    if config.profile == "controlled"
        && (config.verifier_executable.is_none() || config.verifier_ledger_path.is_none())
    {
        return Err(ServiceError::Configuration);
    }
    let proof_ms = config.verifier_proof_ms.unwrap_or(5_000);
    let cleanup_ms = config.verifier_cleanup_ms.unwrap_or(6_000);
    if proof_ms == 0 || cleanup_ms < proof_ms || cleanup_ms > 60_000 {
        return Err(ServiceError::Configuration);
    }
    let parent = config
        .socket_path
        .parent()
        .ok_or(ServiceError::Configuration)?;
    let metadata = fs::metadata(parent)?;
    if !metadata.is_dir()
        || metadata.uid() != config.principal.uid
        || metadata.permissions().mode() & 0o777 != 0o700
    {
        return Err(ServiceError::Configuration);
    }
    let grant_material = material_port.clone();
    let (sender, shutdown_gate, grant_control) = actor::start(
        config.clone(),
        material_port,
        task_operation_port,
        preopened_store,
        direct_control,
    )?;
    let grant_lane = grant_control.map(|control| {
        Arc::new(GrantLane {
            control: Mutex::new(control),
            material: grant_material.expect("task profile requires material owner"),
            open: AtomicBool::new(true),
            in_flight: AtomicUsize::new(0),
        })
    });
    if let Ok(metadata) = fs::symlink_metadata(&config.socket_path) {
        if !metadata.file_type().is_socket() || metadata.uid() != config.principal.uid {
            return Err(ServiceError::Configuration);
        }
        fs::remove_file(&config.socket_path)?;
    }
    let listener = UnixListener::bind(&config.socket_path)?;
    fs::set_permissions(&config.socket_path, fs::Permissions::from_mode(0o600))?;
    let config = Arc::new(config);
    let slots = Arc::new(Semaphore::new(64));
    let mut terminate = signal(SignalKind::terminate())?;
    loop {
        let (stream, _) = tokio::select! {
            accepted=listener.accept()=>accepted?,
            _=terminate.recv()=>break,
        };
        let Ok(slot) = slots.clone().try_acquire_owned() else {
            continue;
        };
        let sender = sender.clone();
        let config = config.clone();
        let grant_lane = grant_lane.clone();
        tokio::spawn(async move {
            let _slot = slot;
            let _ = handle(stream, sender, config, grant_lane).await;
        });
    }
    // Fence both launch owners on the signal path, before the actor can
    // finish a blocked transition or receive its queued Shutdown request.
    shutdown_gate.close_gate();
    if let Some(lane) = &grant_lane {
        lane.open.store(false, Ordering::SeqCst);
    }
    if cfg!(feature = "controlled-proof")
        && let Some(root) = &config.proof_barrier_dir
    {
        let _ = fs::write(root.join("shutdown_gate_closed.reached"), b"reached\n");
    }
    drop(listener);
    // Drain every already-admitted grant fence before the actor records the
    // service close. A fence that entered before gate closure may still own
    // the task-then-lifecycle commit path; closing first would make that
    // durable close report stale immediately after it was written.
    if let Some(lane) = &grant_lane {
        tokio::time::timeout(Duration::from_secs(60), async {
            while lane.in_flight.load(Ordering::SeqCst) != 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .map_err(|_| ServiceError::Configuration)?;
    }
    let (reply, done) = oneshot::channel();
    sender
        .send(Request::Shutdown { reply })
        .await
        .map_err(|_| ServiceError::Configuration)?;
    // The actor owns the active verifier's exact child handle and must finish
    // its configured cleanup before the process exits. Controlled proof cuts
    // can hold three sequential barriers while the actor is busy; the signal
    // path has already fenced new launches, but must still wait for custody.
    let verifier_cleanup = if config.profile == "controlled" {
        Duration::from_millis(config.verifier_cleanup_ms.unwrap_or(6_000))
    } else {
        Duration::ZERO
    };
    let proof_barriers = if cfg!(feature = "controlled-proof") && config.proof_barrier_dir.is_some()
    {
        Duration::from_secs(45)
    } else {
        Duration::ZERO
    };
    let shutdown_budget = verifier_cleanup + proof_barriers + Duration::from_secs(15);
    tokio::time::timeout(shutdown_budget, done)
        .await
        .map_err(|_| ServiceError::Configuration)?
        .map_err(|_| ServiceError::Configuration)??;
    fs::remove_file(&config.socket_path)?;
    Ok(())
}

async fn handle(
    stream: UnixStream,
    sender: mpsc::Sender<Request>,
    config: Arc<ServiceConfig>,
    grant_lane: Option<Arc<GrantLane>>,
) -> Result<(), ()> {
    let uid = stream.peer_cred().map_err(|_| ())?.uid();
    if uid != config.principal.uid {
        return Err(());
    }
    let (read, write) = stream.into_split();
    let mut reader = BoundedFrameReader::new(read, FRAME_MAX).map_err(|_| ())?;
    let mut writer = BoundedFrameWriter::new(write, FRAME_MAX).map_err(|_| ())?;
    let bytes = tokio::time::timeout(IO_BUDGET, reader.read_frame())
        .await
        .map_err(|_| ())?
        .map_err(|_| ())?
        .ok_or(())?;
    let reply = dispatch(&bytes, uid, &sender, &config, grant_lane.as_ref()).await;
    let bytes = serde_json::to_vec(&reply).map_err(|_| ())?;
    tokio::time::timeout(IO_BUDGET, writer.write_frame(&bytes))
        .await
        .map_err(|_| ())?
        .map_err(|_| ())?;
    Ok(())
}

async fn dispatch(
    bytes: &[u8],
    uid: u32,
    sender: &mpsc::Sender<Request>,
    config: &ServiceConfig,
    grant_lane: Option<&Arc<GrantLane>>,
) -> Value {
    let value = match parse_strict_json(bytes) {
        Ok(value) => value,
        Err(wire_error) => return error(wire_error.code()),
    };
    let Some(object) = value.as_object() else {
        return error(WireErrorCode::SemanticInvalidity);
    };
    match value.get("op").and_then(Value::as_str) {
        Some("command") => {
            if object.len() != 2 || !object.contains_key("command") {
                return error(WireErrorCode::SemanticInvalidity);
            }
            if !matches!(
                config.profile.as_str(),
                "controlled" | "native_simulated" | "retained-repository-analysis-v1"
            ) {
                return error(WireErrorCode::UnsupportedCapability);
            }
            let Some(command) = value.get("command") else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let Ok(bytes) = serde_json::to_vec(command) else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let (reply_tx, reply_rx) = oneshot::channel();
            if sender
                .send(Request::Command {
                    uid,
                    bytes,
                    reply: reply_tx,
                })
                .await
                .is_err()
            {
                return error(WireErrorCode::ObservationUnavailable);
            }
            reply_rx
                .await
                .unwrap_or_else(|_| error(WireErrorCode::ObservationUnavailable))
        }
        Some("snapshot") => {
            if object.keys().any(|key| {
                !["op", "subject_id", "delivery_id", "transition_id"].contains(&key.as_str())
            }) || object.contains_key("delivery_id")
                && object.contains_key("transition_id")
                && !value["delivery_id"].is_null()
                && !value["transition_id"].is_null()
            {
                return error(WireErrorCode::SemanticInvalidity);
            }
            let Some(subject) = value.get("subject_id").and_then(Value::as_str) else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let delivery = value
                .get("delivery_id")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let transition = value
                .get("transition_id")
                .and_then(Value::as_str)
                .map(str::to_owned);
            read_snapshot(sender, subject.to_owned(), delivery, transition).await
        }
        Some("subject_revision") => {
            if config.profile != "retained-repository-analysis-v1"
                || object.len() != 2
                || !object.contains_key("subject_id")
            {
                return error(WireErrorCode::SemanticInvalidity);
            }
            let Some(subject) = value.get("subject_id").and_then(Value::as_str) else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let (reply_tx, reply_rx) = oneshot::channel();
            if sender
                .send(Request::SubjectRevision {
                    subject: subject.to_owned(),
                    reply: reply_tx,
                })
                .await
                .is_err()
            {
                return error(WireErrorCode::ObservationUnavailable);
            }
            reply_rx
                .await
                .unwrap_or_else(|_| error(WireErrorCode::ObservationUnavailable))
        }
        Some("native_delivery_status") => {
            if config.profile != "retained-repository-analysis-v1"
                || object.len() != 2
                || !object.contains_key("subject_id")
            {
                return error(WireErrorCode::SemanticInvalidity);
            }
            let Some(subject) = value.get("subject_id").and_then(Value::as_str) else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let (reply_tx, reply_rx) = oneshot::channel();
            if sender
                .send(Request::NativeDeliveryStatus {
                    subject: subject.to_owned(),
                    reply: reply_tx,
                })
                .await
                .is_err()
            {
                return error(WireErrorCode::ObservationUnavailable);
            }
            reply_rx
                .await
                .unwrap_or_else(|_| error(WireErrorCode::ObservationUnavailable))
        }
        Some("command_result") => {
            if object.len() != 4
                || ["subject_id", "command_id", "request_digest"]
                    .iter()
                    .any(|key| !object.contains_key(*key))
            {
                return error(WireErrorCode::SemanticInvalidity);
            }
            let Some(subject) = value.get("subject_id").and_then(Value::as_str) else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let Some(command_id) = value.get("command_id").and_then(Value::as_str) else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let Some(digest) = value.get("request_digest").and_then(Value::as_str) else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let (reply_tx, reply_rx) = oneshot::channel();
            if sender
                .send(Request::CommandReadback {
                    subject: subject.to_owned(),
                    command_id: command_id.to_owned(),
                    request_digest: digest.to_owned(),
                    reply: reply_tx,
                })
                .await
                .is_err()
            {
                return error(WireErrorCode::ObservationUnavailable);
            }
            reply_rx
                .await
                .unwrap_or_else(|_| error(WireErrorCode::ObservationUnavailable))
        }
        Some("contract_grant" | "grant_result") => {
            if object.len() != 2 || !object.contains_key("command") {
                return error(WireErrorCode::SemanticInvalidity);
            }
            let Some(command) = value.get("command") else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let Ok(bytes) = serde_json::to_vec(command) else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            if let Some(lane) = grant_lane {
                if !lane.open.load(Ordering::SeqCst) {
                    return error(WireErrorCode::ObservationUnavailable);
                }
                lane.in_flight.fetch_add(1, Ordering::SeqCst);
                let ticket = GrantInFlight(lane.clone());
                let lane = lane.clone();
                let config = config.clone();
                let read_only = value["op"] == "grant_result";
                return tokio::task::spawn_blocking(move || {
                    let _ticket = ticket;
                    if !lane.open.load(Ordering::SeqCst) {
                        return error(WireErrorCode::ObservationUnavailable);
                    }
                    let mut reply = None;
                    let fenced = lane.material.with_current_fence(&[], &mut || {
                        if !lane.open.load(Ordering::SeqCst) {
                            return Err(ServiceError::Configuration);
                        }
                        if !read_only {
                            crate::operations::optional_native_barrier(
                                config.proof_barrier_dir.as_deref(),
                                "grant_lane_inside_fence",
                            )?;
                        }
                        let mut control = lane
                            .control
                            .lock()
                            .map_err(|_| ServiceError::Configuration)?;
                        reply = Some(actor::grant_control_on_store(
                            &mut *control,
                            &config,
                            uid,
                            &bytes,
                            read_only,
                        ));
                        Ok(())
                    });
                    if fenced.is_err() {
                        error(WireErrorCode::ObservationUnavailable)
                    } else {
                        reply.unwrap_or_else(|| error(WireErrorCode::ObservationUnavailable))
                    }
                })
                .await
                .unwrap_or_else(|_| error(WireErrorCode::ObservationUnavailable));
            }
            let (reply_tx, reply_rx) = oneshot::channel();
            if sender
                .send(Request::GrantControl {
                    uid,
                    bytes,
                    read_only: value["op"] == "grant_result",
                    reply: reply_tx,
                })
                .await
                .is_err()
            {
                return error(WireErrorCode::ObservationUnavailable);
            }
            reply_rx
                .await
                .unwrap_or_else(|_| error(WireErrorCode::ObservationUnavailable))
        }
        Some("wait") => {
            if object.len() != 5
                || ["subject_id", "cursor", "target", "max_wait_ms"]
                    .iter()
                    .any(|key| !object.contains_key(*key))
            {
                return error(WireErrorCode::SemanticInvalidity);
            }
            let Some(subject) = value.get("subject_id").and_then(Value::as_str) else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let Some(cursor_value) = value.get("cursor") else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let Some(target_value) = value.get("target") else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let Ok(cursor): Result<CursorV1, _> = serde_json::from_value(cursor_value.clone())
            else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            if cursor.validate().is_err() {
                return error(WireErrorCode::SemanticInvalidity);
            }
            let Ok(target): Result<WaitTargetV1, _> = serde_json::from_value(target_value.clone())
            else {
                return error(WireErrorCode::SemanticInvalidity);
            };
            let budget = value
                .get("max_wait_ms")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
                .min(config.max_wait_ms);
            if budget == 0 {
                return json!({"hint":"no_change"});
            }
            let (delivery, transition) = match target {
                WaitTargetV1::DeliveryOutcome { delivery_id, .. } => (Some(delivery_id), None),
                WaitTargetV1::TransitionStage { transition_id, .. } => (None, Some(transition_id)),
            };
            let started = Instant::now();
            loop {
                let snapshot = read_snapshot(
                    sender,
                    subject.to_owned(),
                    delivery.clone(),
                    transition.clone(),
                )
                .await;
                if snapshot.get("code").is_some() {
                    return snapshot;
                }
                let current: Result<CursorV1, _> =
                    serde_json::from_value(snapshot["cursor"].clone());
                let Ok(current) = current else {
                    return error(WireErrorCode::ObservationUnavailable);
                };
                if current.store_id != cursor.store_id || current.stream_id != cursor.stream_id {
                    return json!({"hint":"cursor_gap"});
                }
                let current_seq = current.commit_sequence.parse::<u64>().unwrap_or(0);
                let prior_seq = cursor.commit_sequence.parse::<u64>().unwrap_or(u64::MAX);
                if current_seq < prior_seq {
                    return json!({"hint":"cursor_gap"});
                }
                if current_seq > prior_seq {
                    return json!({"hint":"changed"});
                }
                if started.elapsed() >= Duration::from_millis(budget) {
                    return json!({"hint":"no_change"});
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
        _ => error(WireErrorCode::UnsupportedCapability),
    }
}

async fn read_snapshot(
    sender: &mpsc::Sender<Request>,
    subject: String,
    delivery: Option<String>,
    transition: Option<String>,
) -> Value {
    let targeted = delivery.is_some() || transition.is_some();
    let (reply_tx, reply_rx) = oneshot::channel();
    if sender
        .send(Request::Snapshot {
            subject,
            delivery,
            transition,
            reply: reply_tx,
        })
        .await
        .is_err()
    {
        return error(WireErrorCode::ObservationUnavailable);
    }
    match reply_rx.await {
        Ok(Ok(value)) => serde_json::to_value(projection::into_snapshot(value, targeted))
            .unwrap_or_else(|_| error(WireErrorCode::ObservationUnavailable)),
        _ => error(WireErrorCode::ObservationUnavailable),
    }
}

fn error(code: WireErrorCode) -> Value {
    json!({"protocol_version":1,"code":code,"detail":"request unavailable"})
}
