//! Permanent ordinary entry for retained repository-analysis tasks.

mod observation;
mod selected_startup;

use lifecycle_service::{GrantConfig, ServiceConfig};
use lifecycle_wire::{
    CommandDtoV2, CommandOperationV2, EnqueueInputPayloadV2, RequestReplacementPayloadV1,
    parse_snapshot,
};
use retained_task_workflow::{
    AdmissionCommand, AnswerQuestion, ControlCommand, PROFILE, SubmitTask, TaskCommit, TaskError,
    TaskStore, json_digest,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    fs,
    io::{Read, Write},
    os::unix::{
        fs::{FileTypeExt, MetadataExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use thiserror::Error;
use tokio::{
    net::{UnixListener, UnixStream as AsyncUnixStream},
    signal::unix::{SignalKind, signal},
};
use work_engine_types::CodecContract;

const MAX_FRAME: usize = 131_072;
const IO_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum HostError {
    #[error("task host configuration is invalid")]
    Configuration,
    #[error("task host I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("task owner failed: {0}")]
    Task(#[from] retained_task_workflow::TaskError),
    #[error("task encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error("lifecycle service failed: {0}")]
    Lifecycle(#[from] lifecycle_service::ServiceError),
    #[error("lifecycle admission or observation unavailable")]
    LifecycleUnavailable,
    #[error("host is in recovery-only observation mode")]
    RecoveryOnly,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostStartup {
    pub profile: String,
    pub task_store_path: PathBuf,
    pub socket_path: PathBuf,
    pub approved_snapshot_ref: String,
    pub answer_authority_ref: String,
    pub enqueue_grant_ref: String,
    pub replacement_grant_ref: String,
    pub executable_sha256: String,
    pub source_root: PathBuf,
    pub source_manifest_path: PathBuf,
    pub source_sha256: String,
    pub lock_path: PathBuf,
    pub lock_sha256: String,
    pub toolchain_path: PathBuf,
    pub toolchain_sha256: String,
    pub features: Vec<String>,
    pub provider_identity: String,
    pub lifecycle: ServiceConfig,
}

#[derive(Clone, Debug, Serialize)]
pub struct HostIdentity {
    pub pid: u32,
    pub linux_starttime_ticks: String,
    pub incarnation: String,
    pub build_id: String,
    pub profile: String,
    pub executable_sha256: String,
    pub source_sha256: String,
    pub lock_sha256: String,
    pub toolchain_sha256: String,
    pub features: Vec<String>,
    pub provider_identity: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceManifestEntry {
    path: String,
    sha256: String,
}

struct Host {
    startup: HostStartup,
    store: Arc<TaskStore>,
    identity: HostIdentity,
    admission_gate: Mutex<()>,
    recovery_only: OnceLock<bool>,
}

pub fn run() -> Result<(), HostError> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("serve") {
        return Err(HostError::Configuration);
    }
    let route = args.next().ok_or(HostError::Configuration)?;
    if route != "--startup" && route != "--selected" {
        return Err(HostError::Configuration);
    }
    let path = PathBuf::from(args.next().ok_or(HostError::Configuration)?);
    if args.next().is_some() {
        return Err(HostError::Configuration);
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    if route == "--selected" {
        runtime.block_on(serve_selected(&path))
    } else {
        runtime.block_on(serve(&path))
    }
}

pub async fn serve(path: &Path) -> Result<(), HostError> {
    validate_file(path)?;
    let startup: HostStartup = serde_json::from_slice(&fs::read(path)?)?;
    validate_startup(&startup)?;
    let store = TaskStore::open(
        &startup.task_store_path,
        startup.answer_authority_ref.clone(),
    )?;
    #[cfg(feature = "controlled-proof")]
    let store = store.with_controlled_proof_barrier(startup.lifecycle.proof_barrier_dir.clone())?;
    serve_composed(startup, store, None).await
}

/// One sealed host selection binds the executable and both stores before
/// either admission socket opens. The lock survives until the service closes.
pub async fn serve_selected(path: &Path) -> Result<(), HostError> {
    let selected_startup::Prepared {
        startup,
        task_store,
        lifecycle_store,
        selection_lock,
    } = selected_startup::prepare(path)?;
    let result = serve_composed(startup, task_store, Some(lifecycle_store)).await;
    drop(selection_lock);
    result
}

async fn serve_composed(
    startup: HostStartup,
    store: TaskStore,
    selected_store: Option<lifecycle_service::SelectedLifecycleStore>,
) -> Result<(), HostError> {
    let store = Arc::new(store);
    let identity = host_identity(&startup)?;
    let host = Arc::new(Host {
        startup,
        store,
        identity,
        admission_gate: Mutex::new(()),
        recovery_only: OnceLock::new(),
    });
    let config = host.startup.lifecycle.clone();
    let material = host.store.clone();
    let operations = host.store.clone();
    let mut service = tokio::spawn(async move {
        if let Some(selected_store) = selected_store {
            lifecycle_service::run_with_selected_task_ports(
                config,
                selected_store,
                material,
                operations,
            )
            .await
        } else {
            lifecycle_service::run_with_task_ports(config, material, operations).await
        }
    });
    wait_for_socket(&host.startup.lifecycle.socket_path, &mut service).await?;
    let recovery_only = read_recovery_only(
        &host.startup.lifecycle.socket_path,
        &host.startup.lifecycle.subject_id,
    )?;
    host.recovery_only
        .set(recovery_only)
        .map_err(|_| HostError::LifecycleUnavailable)?;
    if let Ok(meta) = fs::symlink_metadata(&host.startup.socket_path) {
        if !meta.file_type().is_socket() || meta.uid() != host.startup.lifecycle.principal.uid {
            return Err(HostError::Configuration);
        }
        fs::remove_file(&host.startup.socket_path)?;
    }
    let listener = UnixListener::bind(&host.startup.socket_path)?;
    fs::set_permissions(&host.startup.socket_path, fs::Permissions::from_mode(0o600))?;
    let mut terminate = signal(SignalKind::terminate())?;
    loop {
        tokio::select! {
            accepted=listener.accept()=>{let(stream,_)=accepted?;let host=host.clone();tokio::spawn(async move{let _=handle(stream,host).await;});},
            _=terminate.recv()=>break,
            result=&mut service=>{let _=fs::remove_file(&host.startup.socket_path);return result.map_err(|_|HostError::LifecycleUnavailable)?.map_err(HostError::from);}
        }
    }
    drop(listener);
    let _ = fs::remove_file(&host.startup.socket_path);
    let _ = tokio::time::timeout(Duration::from_secs(15), &mut service).await;
    Ok(())
}

async fn handle(stream: AsyncUnixStream, host: Arc<Host>) -> Result<(), HostError> {
    if stream.peer_cred()?.uid() != host.startup.lifecycle.principal.uid {
        return Err(HostError::Configuration);
    }
    let (mut reader, mut writer) = stream.into_split();
    let mut bytes = Vec::new();
    loop {
        let mut one = [0u8; 1];
        tokio::time::timeout(
            IO_TIMEOUT,
            tokio::io::AsyncReadExt::read_exact(&mut reader, &mut one),
        )
        .await
        .map_err(|_| HostError::LifecycleUnavailable)??;
        if one[0] == b'\n' {
            break;
        }
        if bytes.len() == MAX_FRAME {
            let refusal = encode_response(&json!({"status":"error","code":"request_too_large"}))?;
            tokio::time::timeout(
                IO_TIMEOUT,
                tokio::io::AsyncWriteExt::write_all(&mut writer, &refusal),
            )
            .await
            .map_err(|_| HostError::LifecycleUnavailable)??;
            return Ok(());
        }
        bytes.push(one[0]);
    }
    let reply = match lifecycle_wire::parse_strict_json(&bytes) {
        Ok(request) => tokio::task::spawn_blocking(move || dispatch(&host, request))
            .await
            .unwrap_or_else(|_| json!({"status":"error","code":"host_unavailable"})),
        Err(_) => json!({"status":"error","code":"invalid_request"}),
    };
    let output = encode_response(&reply)?;
    tokio::time::timeout(
        IO_TIMEOUT,
        tokio::io::AsyncWriteExt::write_all(&mut writer, &output),
    )
    .await
    .map_err(|_| HostError::LifecycleUnavailable)??;
    Ok(())
}
fn encode_response(reply: &Value) -> Result<Vec<u8>, HostError> {
    let mut output = serde_json::to_vec(reply)?;
    if output.len() > MAX_FRAME {
        output = serde_json::to_vec(&json!({"status":"error","code":"response_too_large"}))?;
    }
    output.push(b'\n');
    Ok(output)
}

fn dispatch(host: &Host, request: Value) -> Value {
    let result: Result<Value, HostError> = (|| {
        let recovery_only = *host
            .recovery_only
            .get()
            .ok_or(HostError::LifecycleUnavailable)?;
        let object = request.as_object().ok_or(HostError::Configuration)?;
        match request.get("op").and_then(Value::as_str) {
            Some("task.submit.v1") if object.len() == 2 => {
                let _gate = host
                    .admission_gate
                    .lock()
                    .map_err(|_| HostError::LifecycleUnavailable)?;
                let input: SubmitTask = serde_json::from_value(request["payload"].clone())?;
                if input.subject_id != host.startup.lifecycle.subject_id
                    || input.snapshot_ref != host.startup.approved_snapshot_ref
                {
                    return Err(HostError::Configuration);
                }
                if recovery_only {
                    let owner = host
                        .store
                        .lookup_command_receipt(
                            "submit",
                            &input.task_id,
                            &input.command_id,
                            &json_digest(&input)?,
                        )?
                        .ok_or(HostError::RecoveryOnly)?;
                    let lifecycle = read_command_result(host, &input.command_id).ok().flatten();
                    return Ok(
                        json!({"status":qualified_status(lifecycle.as_ref(),"admitted","rejected_admission","pending_admission"),"owner":owner,"lifecycle":lifecycle,"host_identity":host.identity}),
                    );
                }
                if host.store.input_text(&input.command_id)?.is_none() {
                    preflight_grant(host, &input.subject_id, "enqueue_input")?;
                }
                let (owner, first_publication) = host.store.submit_with_status(&input)?;
                let lifecycle = if first_publication {
                    admit(host, &input, &owner).ok()
                } else {
                    read_command_result(host, &input.command_id).ok().flatten()
                };
                Ok(
                    json!({"status":qualified_status(lifecycle.as_ref(),"admitted","rejected_admission","pending_admission"),"owner":owner,"lifecycle":lifecycle,"host_identity":host.identity}),
                )
            }
            Some("task.answer.v1") if object.len() == 2 => {
                let input: AnswerQuestion = serde_json::from_value(request["payload"].clone())?;
                let owner = if recovery_only {
                    host.store
                        .lookup_command_receipt(
                            "answer",
                            &input.task_id,
                            &input.command_id,
                            &json_digest(&input)?,
                        )?
                        .ok_or(HostError::RecoveryOnly)?
                } else {
                    host.store.answer(&input)?
                };
                Ok(json!({"status":"answered","owner":owner,"host_identity":host.identity}))
            }
            Some("task.observe.v1") => {
                Ok(json!({"status":"error","code":"unsupported_observation_version"}))
            }
            Some("task.observe.v2") => observation::observe(host, &request),
            Some("task.request_replacement.v1") if object.len() == 2 => {
                let _gate = host
                    .admission_gate
                    .lock()
                    .map_err(|_| HostError::LifecycleUnavailable)?;
                let input: ReplacementRequest = serde_json::from_value(request["payload"].clone())?;
                let task = host
                    .store
                    .observe(&input.task_id)?
                    .ok_or(HostError::Configuration)?;
                if input.authority_ref != host.startup.answer_authority_ref {
                    return Err(HostError::Task(TaskError::Unauthorized));
                }
                let request_sha256 = json_digest(&request["payload"])?;
                let lifecycle =
                    if let Some(prior) = host.store.lookup_control_command(&input.command_id)? {
                        if prior.kind != "replacement" || prior.request_sha256 != request_sha256 {
                            return Err(HostError::Configuration);
                        }
                        read_control_result(host, &prior).ok()
                    } else {
                        if recovery_only {
                            return Err(HostError::RecoveryOnly);
                        }
                        if task.revision != input.expected_task_revision {
                            return Err(HostError::Configuration);
                        }
                        preflight_grant(host, &task.subject_id, "request_replacement")?;
                        command(
                            host,
                            &task.subject_id,
                            &input.command_id,
                            "request_replacement",
                            CommandOperationV2::RequestReplacement(RequestReplacementPayloadV1 {
                                reason: input.reason,
                            }),
                            Some(("replacement", request_sha256.as_str())),
                        )
                        .ok()
                    };
                Ok(
                    json!({"status":qualified_status(lifecycle.as_ref(),"replacement_requested","replacement_rejected","pending_replacement"),"lifecycle":lifecycle,"host_identity":host.identity}),
                )
            }
            Some("task.contract_grant.v1") if object.len() == 2 => {
                let _gate = host
                    .admission_gate
                    .lock()
                    .map_err(|_| HostError::LifecycleUnavailable)?;
                let input: GrantContractRequest =
                    serde_json::from_value(request["payload"].clone())?;
                let task = host
                    .store
                    .observe(&input.task_id)?
                    .ok_or(HostError::Configuration)?;
                if input.authority_ref != host.startup.answer_authority_ref {
                    return Err(HostError::Task(TaskError::Unauthorized));
                }
                if task.subject_id != host.startup.lifecycle.subject_id
                    || !host
                        .startup
                        .lifecycle
                        .grants
                        .iter()
                        .any(|g| g.grant_ref == input.grant_ref)
                {
                    return Err(HostError::Configuration);
                }
                let request_sha256 = json_digest(&request["payload"])?;
                let lifecycle = if let Some(prior) =
                    host.store.lookup_control_command(&input.command_id)?
                {
                    if prior.kind != "contract_grant" || prior.request_sha256 != request_sha256 {
                        return Err(HostError::Configuration);
                    }
                    read_control_result(host, &prior).ok()
                } else {
                    if recovery_only {
                        return Err(HostError::RecoveryOnly);
                    }
                    let service_command = json!({"protocol_version":1,"command_id":input.command_id,"subject_id":task.subject_id,"authority_ref":host.startup.lifecycle.principal.principal_ref,"grant_ref":input.grant_ref,"expected_revision":input.expected_revision.to_string()});
                    let prior = ControlCommand {
                        command_id: input.command_id,
                        kind: "contract_grant".into(),
                        request_sha256,
                        service_command_json: serde_json::to_string(&service_command)?,
                    };
                    host.store.record_control_command(&prior)?;
                    exchange(
                        &host.startup.lifecycle.socket_path,
                        &json!({"op":"contract_grant","command":service_command}),
                    )
                    .ok()
                    .and_then(|reply| verified_grant_reply(&prior, reply).ok())
                };
                Ok(
                    json!({"status":qualified_status(lifecycle.as_ref(),"grant_contracted","grant_rejected","pending_grant_contraction"),"lifecycle":lifecycle,"host_identity":host.identity}),
                )
            }
            _ => Err(HostError::Configuration),
        }
    })();
    match result {
        Ok(v) => v,
        Err(HostError::RecoveryOnly) => json!({"status":"error","code":"recovery_only"}),
        Err(HostError::Task(e)) => json!({"status":"error","code":task_code(&e)}),
        Err(_) => json!({"status":"error","code":"invalid_or_unavailable"}),
    }
}
fn qualified_status<'a>(
    value: Option<&Value>,
    accepted: &'a str,
    rejected: &'a str,
    pending: &'a str,
) -> &'a str {
    match value
        .and_then(|v| v.get("accepted"))
        .and_then(Value::as_bool)
    {
        Some(true) => accepted,
        Some(false) => rejected,
        None if value
            .and_then(|v| v.get("code"))
            .and_then(Value::as_str)
            .is_some_and(|code| !matches!(code, "observation_unavailable" | "cursor_gap")) =>
        {
            rejected
        }
        None => pending,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplacementRequest {
    task_id: String,
    command_id: String,
    expected_task_revision: u64,
    authority_ref: String,
    reason: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrantContractRequest {
    task_id: String,
    command_id: String,
    authority_ref: String,
    grant_ref: String,
    expected_revision: u64,
}

fn admit(host: &Host, input: &SubmitTask, _owner: &TaskCommit) -> Result<Value, HostError> {
    let text = host
        .store
        .input_text(&input.command_id)?
        .ok_or(HostError::LifecycleUnavailable)?;
    let text_digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .map_err(|_| HostError::Configuration)?
        .hex();
    command(
        host,
        &input.subject_id,
        &input.command_id,
        "enqueue_input",
        CommandOperationV2::EnqueueInput(EnqueueInputPayloadV2 {
            input_id: format!("input:{}", input.command_id),
            producer_ref: host.startup.lifecycle.principal.producer_ref.clone(),
            text,
            text_digest,
        }),
        None,
    )
}
fn command(
    host: &Host,
    subject: &str,
    command_id: &str,
    grant_scope: &str,
    operation: CommandOperationV2,
    control: Option<(&str, &str)>,
) -> Result<Value, HostError> {
    let current = read_subject_revision(&host.startup.lifecycle.socket_path, subject)?;
    let revision = current
        .get("revision")
        .and_then(Value::as_str)
        .ok_or(HostError::LifecycleUnavailable)?;
    let context = current
        .get("context_generation")
        .and_then(Value::as_str)
        .ok_or(HostError::LifecycleUnavailable)?;
    let grant_ref = grant_for_context(&host.startup.lifecycle.grants, context, grant_scope)?
        .grant_ref
        .clone();
    let mut command = CommandDtoV2 {
        protocol_version: 2,
        principal_ref: host.startup.lifecycle.principal.principal_ref.clone(),
        command_id: command_id.into(),
        subject_id: subject.into(),
        context_generation: context.into(),
        build_id: host.startup.lifecycle.build_id.clone(),
        proof_run_id: host.startup.lifecycle.proof_run_id.clone(),
        grant_ref,
        expected_revision: revision.into(),
        request_digest: String::new(),
        operation,
    };
    command.request_digest = CodecContract::LifecycleCommandV2
        .digest_json(&command.digest_basis())
        .map_err(|_| HostError::Configuration)?
        .hex();
    if matches!(&command.operation, CommandOperationV2::EnqueueInput(_)) {
        host.store.record_admission_command(&AdmissionCommand {
            command_id: command_id.into(),
            request_digest: command.request_digest.clone(),
            command_json: serde_json::to_string(&command)?,
        })?;
    } else if let Some((kind, request_sha256)) = control {
        host.store.record_control_command(&ControlCommand {
            command_id: command_id.into(),
            kind: kind.into(),
            request_sha256: request_sha256.into(),
            service_command_json: serde_json::to_string(&command)?,
        })?;
    }
    let reply = exchange(
        &host.startup.lifecycle.socket_path,
        &json!({"op":"command","command":command}),
    )?;
    if reply.get("code").is_some() {
        return Ok(reply);
    }
    if reply.get("protocol_version").and_then(Value::as_u64) != Some(2)
        || reply.get("command_id").and_then(Value::as_str) != Some(command_id)
    {
        return Err(HostError::LifecycleUnavailable);
    }
    Ok(reply)
}
fn read_subject_revision(path: &Path, subject: &str) -> Result<Value, HostError> {
    let reply = exchange(path, &json!({"op":"subject_revision","subject_id":subject}))?;
    if reply.get("protocol_version").and_then(Value::as_u64) != Some(1)
        || reply.get("subject_id").and_then(Value::as_str) != Some(subject)
        || reply.get("store_id").and_then(Value::as_str).is_none()
        || reply
            .get("context_generation")
            .and_then(Value::as_str)
            .is_none()
        || reply
            .get("revision")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<u64>().ok())
            .is_none()
    {
        return Err(HostError::LifecycleUnavailable);
    }
    Ok(reply)
}
fn preflight_grant(host: &Host, subject: &str, scope: &str) -> Result<(), HostError> {
    let current = read_subject_revision(&host.startup.lifecycle.socket_path, subject)?;
    let context = current
        .get("context_generation")
        .and_then(Value::as_str)
        .ok_or(HostError::LifecycleUnavailable)?;
    grant_for_context(&host.startup.lifecycle.grants, context, scope)?;
    Ok(())
}
fn grant_for_context<'a>(
    grants: &'a [GrantConfig],
    context: &str,
    scope: &str,
) -> Result<&'a GrantConfig, HostError> {
    let mut matches = grants
        .iter()
        .filter(|grant| grant.context_generation == context && grant.scope == scope);
    let grant = matches.next().ok_or(HostError::Configuration)?;
    if matches.next().is_some() || grant.grant_ref.is_empty() || grant.revision == 0 {
        return Err(HostError::Configuration);
    }
    Ok(grant)
}
fn validate_initial_grants(
    grants: &[GrantConfig],
    context: &str,
    enqueue_ref: &str,
    replacement_ref: &str,
) -> Result<(), HostError> {
    let initial_enqueue = grant_for_context(grants, context, "enqueue_input")?;
    let initial_replacement = grant_for_context(grants, context, "request_replacement")?;
    if initial_enqueue.grant_ref != enqueue_ref || initial_replacement.grant_ref != replacement_ref
    {
        return Err(startup_invalid(
            "initial enqueue or replacement grant binding",
        ));
    }
    Ok(())
}
fn read_native_delivery_status(path: &Path, subject: &str) -> Result<Value, HostError> {
    let reply = exchange(
        path,
        &json!({"op":"native_delivery_status","subject_id":subject}),
    )?;
    if reply.get("code").is_some() {
        return Ok(reply);
    }
    if reply.get("protocol_version").and_then(Value::as_u64) != Some(1)
        || reply.get("subject_id").and_then(Value::as_str) != Some(subject)
        || !reply.get("segments").is_some_and(Value::is_array)
    {
        return Err(HostError::LifecycleUnavailable);
    }
    Ok(reply)
}
fn read_recovery_only(path: &Path, subject: &str) -> Result<bool, HostError> {
    recovery_mode_from_status(&read_native_delivery_status(path, subject)?)
}
fn recovery_mode_from_status(status: &Value) -> Result<bool, HostError> {
    status
        .get("recovery_only")
        .and_then(Value::as_bool)
        .ok_or(HostError::LifecycleUnavailable)
}
fn read_command_result(host: &Host, command_id: &str) -> Result<Option<Value>, HostError> {
    let Some(command) = host.store.lookup_admission_command(command_id)? else {
        return Ok(None);
    };
    let reply = exchange(
        &host.startup.lifecycle.socket_path,
        &json!({"op":"command_result","subject_id":host.startup.lifecycle.subject_id,"command_id":command_id,"request_digest":command.request_digest}),
    )?;
    if reply.get("status").and_then(Value::as_str) == Some("unknown") || reply.get("code").is_some()
    {
        return Ok(Some(reply));
    }
    if reply.get("protocol_version").and_then(Value::as_u64) != Some(2)
        || reply.get("command_id").and_then(Value::as_str) != Some(command_id)
    {
        return Err(HostError::LifecycleUnavailable);
    }
    Ok(Some(reply))
}
fn read_control_result(host: &Host, control: &ControlCommand) -> Result<Value, HostError> {
    let command: Value = serde_json::from_str(&control.service_command_json)?;
    let request = if control.kind == "replacement" {
        json!({"op":"command_result","subject_id":host.startup.lifecycle.subject_id,"command_id":control.command_id,"request_digest":command.get("request_digest").and_then(Value::as_str).ok_or(HostError::Configuration)?})
    } else if control.kind == "contract_grant" {
        json!({"op":"grant_result","command":command})
    } else {
        return Err(HostError::Configuration);
    };
    let reply = exchange(&host.startup.lifecycle.socket_path, &request)?;
    if reply.get("status").and_then(Value::as_str) == Some("unknown") || reply.get("code").is_some()
    {
        return Ok(reply);
    }
    if reply.get("command_id").and_then(Value::as_str) != Some(control.command_id.as_str()) {
        return Err(HostError::LifecycleUnavailable);
    }
    if control.kind == "contract_grant" {
        verified_grant_reply(control, reply)
    } else {
        Ok(reply)
    }
}
fn verified_grant_reply(control: &ControlCommand, reply: Value) -> Result<Value, HostError> {
    if reply.get("code").is_some() {
        return Ok(reply);
    }
    let command = lifecycle_wire::parse_grant_read_locator(control.service_command_json.as_bytes())
        .map_err(|_| HostError::Configuration)?;
    lifecycle_wire::validate_grant_result(
        &reply,
        &control.command_id,
        &command.grant_ref,
        &command.expected_revision,
    )
    .map_err(|_| HostError::LifecycleUnavailable)?;
    Ok(reply)
}
fn exchange(path: &Path, request: &Value) -> Result<Value, HostError> {
    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let bytes = serde_json::to_vec(request)?;
    if bytes.len() > MAX_FRAME {
        return Err(HostError::Configuration);
    }
    stream.write_all(&bytes)?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    let mut reply = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte)?;
        if byte[0] == b'\n' {
            break;
        }
        if reply.len() == MAX_FRAME {
            return Err(HostError::LifecycleUnavailable);
        }
        reply.push(byte[0]);
    }
    lifecycle_wire::parse_strict_json(&reply).map_err(|_| HostError::LifecycleUnavailable)
}
fn validate_file(path: &Path) -> Result<(), HostError> {
    if !path.is_absolute() {
        return Err(HostError::Configuration);
    }
    let meta = fs::symlink_metadata(path)?;
    if !meta.file_type().is_file()
        || meta.uid() != current_uid()?
        || meta.permissions().mode() & 0o077 != 0
    {
        return Err(HostError::Configuration);
    }
    Ok(())
}
fn validate_startup(s: &HostStartup) -> Result<(), HostError> {
    if s.profile != PROFILE
        || s.lifecycle.subject_id.is_empty()
        || s.approved_snapshot_ref.is_empty()
        || s.enqueue_grant_ref.is_empty()
        || s.replacement_grant_ref.is_empty()
        || s.answer_authority_ref.is_empty()
        || s.lifecycle.principal.uid != current_uid()?
    {
        return Err(startup_invalid("profile, principal or trusted selector"));
    }
    validate_initial_grants(
        &s.lifecycle.grants,
        &s.lifecycle.context_generation,
        &s.enqueue_grant_ref,
        &s.replacement_grant_ref,
    )?;
    for path in [
        &s.task_store_path,
        &s.socket_path,
        &s.lifecycle.socket_path,
        &s.lifecycle.store_root,
        &s.source_root,
        &s.source_manifest_path,
        &s.lock_path,
        &s.toolchain_path,
    ] {
        if !path.is_absolute() {
            return Err(startup_invalid("non-absolute trusted path"));
        }
    }
    for parent in [s.task_store_path.parent(), s.socket_path.parent()] {
        let meta = fs::metadata(parent.ok_or(HostError::Configuration)?)?;
        if !meta.is_dir()
            || meta.uid() != current_uid()?
            || meta.permissions().mode() & 0o777 != 0o700
        {
            return Err(startup_invalid(
                "task store or socket parent ownership/mode",
            ));
        }
    }
    if s.socket_path == s.lifecycle.socket_path {
        return Err(startup_invalid("socket path collision"));
    }
    if s.provider_identity != "native-sim-proof-v1"
        || s.lifecycle.native.is_none()
        || s.lifecycle.profile != PROFILE
    {
        return Err(startup_invalid("provider identity or profile"));
    }
    validate_compiled_features(&s.features)?;
    for digest in [
        &s.executable_sha256,
        &s.source_sha256,
        &s.lock_sha256,
        &s.toolchain_sha256,
    ] {
        if !valid_sha(digest) {
            return Err(startup_invalid("malformed pinned digest"));
        }
    }
    if sha_file(&fs::read_link("/proc/self/exe")?)? != s.executable_sha256
        || sha_file(&s.lock_path)? != s.lock_sha256
        || sha_file(&s.toolchain_path)? != s.toolchain_sha256
        || sha_file(&s.source_manifest_path)? != s.source_sha256
    {
        return Err(startup_invalid(
            "pinned executable/source/lock/toolchain digest mismatch",
        ));
    }
    let root = fs::canonicalize(&s.source_root)?;
    if !root.is_dir() {
        return Err(startup_invalid("source root is not a directory"));
    }
    let manifest: Vec<SourceManifestEntry> =
        serde_json::from_slice(&fs::read(&s.source_manifest_path)?)?;
    if manifest.is_empty() {
        return Err(startup_invalid("empty source manifest"));
    }
    let mut prior = None::<String>;
    for entry in manifest {
        let relative = Path::new(&entry.path);
        if relative.is_absolute()
            || relative.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
            || !valid_sha(&entry.sha256)
            || prior.as_ref().is_some_and(|p| p >= &entry.path)
        {
            return Err(startup_invalid("source manifest path/order/digest"));
        }
        let path = fs::canonicalize(root.join(relative))?;
        if !path.starts_with(&root) || sha_file(&path)? != entry.sha256 {
            return Err(startup_invalid("source artifact digest mismatch"));
        }
        prior = Some(entry.path);
    }
    Ok(())
}
fn startup_invalid(reason: &'static str) -> HostError {
    eprintln!("task host startup validation: {reason}");
    HostError::Configuration
}
fn compiled_features() -> Vec<String> {
    if cfg!(feature = "controlled-proof") {
        vec!["controlled-proof".into()]
    } else {
        Vec::new()
    }
}
fn validate_compiled_features(declared: &[String]) -> Result<(), HostError> {
    if declared != compiled_features() {
        return Err(startup_invalid(
            "declared features do not match compiled host",
        ));
    }
    Ok(())
}
fn current_uid() -> Result<u32, HostError> {
    Ok(fs::metadata("/proc/self")?.uid())
}
fn host_identity(startup: &HostStartup) -> Result<HostIdentity, HostError> {
    let stat = fs::read_to_string("/proc/self/stat")?;
    let tail = stat
        .rsplit_once(')')
        .ok_or(HostError::Configuration)?
        .1
        .trim();
    let starttime = tail
        .split_whitespace()
        .nth(19)
        .ok_or(HostError::Configuration)?;
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| HostError::Configuration)?;
    Ok(HostIdentity {
        pid: std::process::id(),
        linux_starttime_ticks: starttime.into(),
        incarnation: format!(
            "host:{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ),
        build_id: startup.lifecycle.build_id.clone(),
        profile: PROFILE.into(),
        executable_sha256: startup.executable_sha256.clone(),
        source_sha256: startup.source_sha256.clone(),
        lock_sha256: startup.lock_sha256.clone(),
        toolchain_sha256: startup.toolchain_sha256.clone(),
        features: compiled_features(),
        provider_identity: startup.provider_identity.clone(),
    })
}
fn valid_sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn sha_file(path: &Path) -> Result<String, HostError> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.file_type().is_file() {
        return Err(HostError::Configuration);
    }
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
async fn wait_for_socket(
    path: &Path,
    service: &mut tokio::task::JoinHandle<Result<(), lifecycle_service::ServiceError>>,
) -> Result<(), HostError> {
    let until = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if tokio::net::UnixStream::connect(path).await.is_ok() {
            return Ok(());
        }
        if service.is_finished() {
            return match service.await {
                Ok(Err(error)) => Err(HostError::Lifecycle(error)),
                Ok(Ok(())) | Err(_) => Err(HostError::LifecycleUnavailable),
            };
        }
        if tokio::time::Instant::now() >= until {
            return Err(HostError::LifecycleUnavailable);
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
fn task_code(e: &retained_task_workflow::TaskError) -> &'static str {
    use retained_task_workflow::TaskError::*;
    match e {
        Invalid => "invalid",
        Conflict => "conflict",
        Unavailable => "unavailable",
        Stale => "stale",
        Unauthorized => "unauthorized",
        PendingQuestion => "pending_question",
        Busy => "busy",
        Uncertain | Storage(_) | Encoding(_) => "uncertain",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(reference: &str, context: &str, scope: &str) -> GrantConfig {
        GrantConfig {
            grant_ref: reference.into(),
            context_generation: context.into(),
            scope: scope.into(),
            expires_wall_ms: i64::MAX,
            revision: 1,
        }
    }

    #[test]
    fn trusted_grant_selection_is_exact_and_unambiguous() {
        let grants = vec![
            grant("enqueue-g0", "g0", "enqueue_input"),
            grant("replace-g0", "g0", "request_replacement"),
            grant("enqueue-g1", "g1", "enqueue_input"),
            grant("replace-g1", "g1", "request_replacement"),
        ];
        assert_eq!(
            grant_for_context(&grants, "g1", "enqueue_input")
                .unwrap()
                .grant_ref,
            "enqueue-g1"
        );
        assert_eq!(
            grant_for_context(&grants, "g1", "request_replacement")
                .unwrap()
                .grant_ref,
            "replace-g1"
        );
        assert!(grant_for_context(&grants, "g2", "enqueue_input").is_err());
        assert!(grant_for_context(&grants, "g1", "snapshot_read").is_err());
        let mut duplicate = grants;
        duplicate.push(grant("another-enqueue-g1", "g1", "enqueue_input"));
        assert!(grant_for_context(&duplicate, "g1", "enqueue_input").is_err());
    }

    #[test]
    fn initial_binding_refs_must_match_unique_g0_grants() {
        let grants = vec![
            grant("enqueue-g0", "g0", "enqueue_input"),
            grant("replace-g0", "g0", "request_replacement"),
            grant("enqueue-g1", "g1", "enqueue_input"),
        ];
        assert!(validate_initial_grants(&grants, "g0", "enqueue-g0", "replace-g0").is_ok());
        assert!(validate_initial_grants(&grants, "g0", "enqueue-g1", "replace-g0").is_err());
        assert!(validate_initial_grants(&grants, "g1", "enqueue-g1", "replace-g0").is_err());
    }

    #[test]
    fn declared_features_must_match_compiled_host() {
        assert!(validate_compiled_features(&compiled_features()).is_ok());
        assert!(validate_compiled_features(&["native-sim-proof".into()]).is_err());
        assert!(validate_compiled_features(&["controlled-proof".into(), "extra".into()]).is_err());
    }

    #[test]
    fn bounded_response_refuses_oversize_without_partial_json() {
        let within = json!({"value":"x".repeat(MAX_FRAME - 24)});
        let encoded = encode_response(&within).unwrap();
        assert!(encoded.len() <= MAX_FRAME + 1);
        assert_eq!(
            lifecycle_wire::parse_strict_json(&encoded[..encoded.len() - 1]).unwrap(),
            within
        );
        let over = json!({"value":"x".repeat(MAX_FRAME)});
        let encoded = encode_response(&over).unwrap();
        assert_eq!(
            lifecycle_wire::parse_strict_json(&encoded[..encoded.len() - 1]).unwrap(),
            json!({"status":"error","code":"response_too_large"})
        );
    }

    #[test]
    fn recovery_mode_requires_explicit_lifecycle_boolean() {
        assert!(recovery_mode_from_status(&json!({"recovery_only":true})).unwrap());
        assert!(!recovery_mode_from_status(&json!({"recovery_only":false})).unwrap());
        assert!(recovery_mode_from_status(&json!({})).is_err());
        assert!(recovery_mode_from_status(&json!({"recovery_only":"false"})).is_err());
    }

    #[test]
    fn recovery_only_host_refuses_new_mutation_but_returns_exact_submit_receipt() {
        let root = tempfile::tempdir().unwrap();
        let lifecycle = ServiceConfig {
            profile: PROFILE.into(),
            store_root: root.path().join("lifecycle-store"),
            socket_path: root.path().join("unavailable-lifecycle.sock"),
            peer_socket_path: root.path().join("unused-peer.sock"),
            verifier_executable: None,
            verifier_ledger_path: None,
            proof_barrier_dir: None,
            trusted_issuer: "issuer-0".into(),
            principal: lifecycle_service::PrincipalConfig {
                uid: current_uid().unwrap(),
                principal_ref: "principal-0".into(),
                producer_ref: "producer-0".into(),
            },
            subject_id: "subject-0".into(),
            context_generation: "context-0".into(),
            build_id: "build-0".into(),
            proof_run_id: "proof-0".into(),
            grants: vec![grant("grant-0", "context-0", "enqueue_input")],
            max_executions: 1,
            max_wait_ms: 1000,
            verifier_proof_ms: None,
            verifier_cleanup_ms: None,
            native: None,
        };
        let startup = HostStartup {
            profile: PROFILE.into(),
            task_store_path: root.path().join("tasks.sqlite"),
            socket_path: root.path().join("host.sock"),
            approved_snapshot_ref: "snapshot-0".into(),
            answer_authority_ref: "human-operator".into(),
            enqueue_grant_ref: "grant-0".into(),
            replacement_grant_ref: "grant-1".into(),
            executable_sha256: String::new(),
            source_root: root.path().to_path_buf(),
            source_manifest_path: root.path().join("manifest.json"),
            source_sha256: String::new(),
            lock_path: root.path().join("Cargo.lock"),
            lock_sha256: String::new(),
            toolchain_path: root.path().join("rust-toolchain.toml"),
            toolchain_sha256: String::new(),
            features: compiled_features(),
            provider_identity: "native-sim-proof-v1".into(),
            lifecycle,
        };
        let store =
            Arc::new(TaskStore::open(&startup.task_store_path, "human-operator".into()).unwrap());
        let mode = OnceLock::new();
        mode.set(true).unwrap();
        let host = Host {
            startup,
            store,
            identity: HostIdentity {
                pid: 1,
                linux_starttime_ticks: "1".into(),
                incarnation: "host-test".into(),
                build_id: "build-0".into(),
                profile: PROFILE.into(),
                executable_sha256: String::new(),
                source_sha256: String::new(),
                lock_sha256: String::new(),
                toolchain_sha256: String::new(),
                features: compiled_features(),
                provider_identity: "native-sim-proof-v1".into(),
            },
            admission_gate: Mutex::new(()),
            recovery_only: mode,
        };
        let submit = json!({"command_id":"command-0","task_id":"task-0","subject_id":"subject-0","kind":"repository_analysis","objective":"Analyze approved source","engineering_question":"Who owns this?","snapshot_ref":"snapshot-0"});
        let new_work = dispatch(&host, json!({"op":"task.submit.v1","payload":submit}));
        assert_eq!(new_work["code"], "recovery_only");
        assert!(host.store.observe("task-0").unwrap().is_none());
        let absent = dispatch(
            &host,
            json!({"op":"task.observe.v2","task_id":"task-0","command_id":"command-0"}),
        );
        assert_eq!(absent["components"]["owner"]["state"], "absent");
        assert_eq!(absent["components"]["lifecycle"]["state"], "not_applicable");
        assert_eq!(
            absent["components"]["command_result"]["record_kind"],
            "admission_command_locator"
        );
        assert_eq!(
            absent["components"]["control_result"]["record_kind"],
            "control_command_locator"
        );
        assert_eq!(
            dispatch(&host, json!({"op":"task.observe.v1","task_id":"task-0"}))["code"],
            "unsupported_observation_version"
        );

        let typed: SubmitTask = serde_json::from_value(submit.clone()).unwrap();
        let receipt = host.store.submit(&typed).unwrap();
        let observed = dispatch(&host, json!({"op":"task.observe.v2","task_id":"task-0"}));
        assert_eq!(observed["components"]["owner"]["state"], "present");
        assert_eq!(
            observed["components"]["owner"]["revision"]["owner_revision"],
            receipt.owner_revision
        );
        assert_eq!(observed["components"]["lifecycle"]["state"], "unavailable");
        assert_eq!(
            observed["components"]["native_delivery_status"]["state"],
            "unavailable"
        );
        assert_eq!(
            observed["components"]["command_result"]["state"],
            "not_requested"
        );
        let replay = dispatch(&host, json!({"op":"task.submit.v1","payload":submit}));
        assert_eq!(replay["owner"]["owner_revision"], receipt.owner_revision);
        assert_eq!(replay["status"], "pending_admission");
        assert!(
            host.store
                .lookup_admission_command("command-0")
                .unwrap()
                .is_none()
        );
        let changed = dispatch(
            &host,
            json!({"op":"task.submit.v1","payload":{
                "command_id":"command-0","task_id":"task-0","subject_id":"subject-0","kind":"repository_analysis","objective":"different","engineering_question":"Who owns this?","snapshot_ref":"snapshot-0"
            }}),
        );
        assert_eq!(changed["code"], "conflict");
        assert_eq!(host.store.observe("task-0").unwrap().unwrap().revision, 1);

        let answer = dispatch(
            &host,
            json!({"op":"task.answer.v1","payload":{
                "command_id":"answer-new","task_id":"task-0","question_id":"question-0","recipient_ref":"human-operator","scope_ref":"scope-0","answer":"No","expected_revision":1,"authority_ref":"human-operator"
            }}),
        );
        assert_eq!(answer["code"], "recovery_only");
        assert_eq!(host.store.observe("task-0").unwrap().unwrap().revision, 1);

        let replacement = dispatch(
            &host,
            json!({"op":"task.request_replacement.v1","payload":{
                "task_id":"task-0","command_id":"replace-new","expected_task_revision":1,
                "authority_ref":"human-operator","reason":"pressure"
            }}),
        );
        assert_eq!(replacement["code"], "recovery_only");
        let contraction = dispatch(
            &host,
            json!({"op":"task.contract_grant.v1","payload":{
                "task_id":"task-0","command_id":"contract-new","authority_ref":"human-operator",
                "grant_ref":"grant-0","expected_revision":1
            }}),
        );
        assert_eq!(contraction["code"], "recovery_only");
        assert!(
            host.store
                .lookup_control_command("replace-new")
                .unwrap()
                .is_none()
        );
        assert!(
            host.store
                .lookup_control_command("contract-new")
                .unwrap()
                .is_none()
        );
        assert_eq!(host.store.observe("task-0").unwrap().unwrap().revision, 1);
        let state = host.store.observe("task-0").unwrap().unwrap();
        let question = retained_task_workflow::QuestionOpenPayload {
            question_id: "question-0".into(),
            recipient_ref: "human-operator".into(),
            scope_ref: "scope-0".into(),
            text: "Approval?".into(),
        };
        let entry = retained_task_workflow::OperationEntry {
            operation_id: "operation-0".into(),
            task_id: "task-0".into(),
            subject_id: "subject-0".into(),
            delivery_id: "delivery-0".into(),
            attempt_id: "attempt-0".into(),
            expected_task_revision: state.revision,
            expected_cursor_revision: state.cursor_revision,
            grant_ref: "grant-0".into(),
            grant_revision: 1,
            payload_sha256: json_digest(&question).unwrap(),
        };
        host.store
            .open_question(&entry, &question, &mut |write| write())
            .unwrap();
        let before_answer = host.store.observe("task-0").unwrap().unwrap();
        let answer_payload = json!({
            "command_id":"answer-existing","task_id":"task-0","question_id":"question-0",
            "recipient_ref":"human-operator","scope_ref":"scope-0","answer":"Approved",
            "expected_revision":before_answer.revision,"authority_ref":"human-operator"
        });
        let typed_answer: AnswerQuestion = serde_json::from_value(answer_payload.clone()).unwrap();
        let owner_answer = host.store.answer(&typed_answer).unwrap();
        let answer_replay = dispatch(
            &host,
            json!({"op":"task.answer.v1","payload":answer_payload}),
        );
        assert_eq!(answer_replay["status"], "answered");
        assert_eq!(
            answer_replay["owner"]["owner_revision"],
            owner_answer.owner_revision
        );
        assert_eq!(
            host.store.observe("task-0").unwrap().unwrap().revision,
            owner_answer.owner_revision
        );
    }
}
