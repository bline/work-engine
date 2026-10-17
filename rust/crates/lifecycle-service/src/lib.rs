//! Offline Rust lifecycle service. Only the controlled-proof feature permits external text execution.

mod actor;
mod admission;
mod controlled;
pub mod development_startup;
mod engine;
mod material_port;
mod operations;
mod projection;
mod semantic;
mod semantic_control;
mod socket;
mod task_port;

use std::path::PathBuf;

use operations::workspace_snapshot_read::TrustedSnapshot;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

pub use material_port::{CanonicalMaterialPort, MaterialPortError};
pub use task_port::{
    CompletedTaskEffect, GrantFence, QuestionOpenPayload, ResultPublishPayload, TaskCursorBinding,
    TaskInputBinding, TaskOperationCommit, TaskOperationError, TaskOperationPort,
    ValidatedTaskOperationEntry,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    pub profile: String,
    pub store_root: PathBuf,
    pub socket_path: PathBuf,
    pub peer_socket_path: PathBuf,
    pub verifier_executable: Option<PathBuf>,
    pub verifier_ledger_path: Option<PathBuf>,
    pub proof_barrier_dir: Option<PathBuf>,
    pub trusted_issuer: String,
    pub principal: PrincipalConfig,
    pub subject_id: String,
    pub context_generation: String,
    pub build_id: String,
    pub proof_run_id: String,
    pub grants: Vec<GrantConfig>,
    pub max_executions: usize,
    pub max_wait_ms: u64,
    #[serde(default)]
    pub verifier_proof_ms: Option<u64>,
    #[serde(default)]
    pub verifier_cleanup_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native: Option<NativeConfig>,
}

/// Opt-in installed-native simulated profile. The named snapshot is supplied
/// by trusted composition; native tool arguments cannot add a path or source.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeConfig {
    pub executable: PathBuf,
    pub root: PathBuf,
    pub simulator_base_url: String,
    pub session_id: String,
    pub invocation_id: String,
    pub attempt_id: String,
    pub prompt: String,
    pub turn_grant_ref: String,
    pub turn_grant_revision: u64,
    pub snapshot_grant_ref: String,
    pub snapshot_grant_revision: u64,
    #[serde(default)]
    pub same_native_session_required: bool,
    #[serde(default)]
    pub followup_turns: Vec<NativeFollowupTurn>,
    pub snapshot: TrustedSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic: Option<SemanticNativeConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_contexts: Vec<SemanticContextConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticContextConfig {
    pub predecessor_context: String,
    pub semantic: SemanticNativeConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticNativeConfig {
    pub capture_id: String,
    pub compile_session_id: String,
    pub verify_session_id: String,
    pub successor_verify_session_id: String,
    pub compile_invocation_id: String,
    pub compile_attempt_id: String,
    pub candidate_verify_invocation_id: String,
    pub candidate_verify_attempt_id: String,
    pub successor_verify_invocation_id: String,
    pub successor_verify_attempt_id: String,
    pub challenge_id: String,
    pub challenge_text: String,
    pub control_grant_ref: String,
    pub control_grant_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub successor_control_grant_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub successor_control_grant_revision: Option<u64>,
}

impl SemanticNativeConfig {
    fn grant_for_phase(&self, phase: lifecycle_semantic::SemanticPhase) -> (&str, u64) {
        if phase == lifecycle_semantic::SemanticPhase::SuccessorAdequacy
            && let (Some(reference), Some(revision)) = (
                self.successor_control_grant_ref.as_deref(),
                self.successor_control_grant_revision,
            )
        {
            return (reference, revision);
        }
        (&self.control_grant_ref, self.control_grant_revision)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeFollowupTurn {
    pub invocation_id: String,
    pub attempt_id: String,
    pub prompt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrincipalConfig {
    pub uid: u32,
    pub principal_ref: String,
    pub producer_ref: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GrantConfig {
    pub grant_ref: String,
    pub context_generation: String,
    pub scope: String,
    pub expires_wall_ms: i64,
    pub revision: u64,
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("configuration or controlled profile is invalid")]
    Configuration,
    #[error("service I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("lifecycle store failed: {0}")]
    Store(#[from] lifecycle_store::StoreError),
    #[error("runtime failed: {0}")]
    Runtime(#[from] lifecycle_runtime::RuntimeError),
    #[error("canonical material failed: {0}")]
    Material(#[from] MaterialPortError),
}

pub async fn run(config: ServiceConfig) -> Result<(), ServiceError> {
    socket::run(config, None, None).await
}

pub(crate) async fn run_with_selected_store(
    config: ServiceConfig,
    store: lifecycle_store::SqliteLifecycleStore,
) -> Result<(), ServiceError> {
    socket::run_with_preopened_store(config, store).await
}

pub async fn run_with_material_port(
    config: ServiceConfig,
    material_port: Arc<dyn CanonicalMaterialPort>,
) -> Result<(), ServiceError> {
    socket::run(config, Some(material_port), None).await
}

/// Compose one permanent domain owner with the lifecycle service. Both ports
/// belong to the same canonical task owner and are selected by trusted startup.
pub async fn run_with_task_ports(
    config: ServiceConfig,
    material_port: Arc<dyn CanonicalMaterialPort>,
    task_operation_port: Arc<dyn TaskOperationPort>,
) -> Result<(), ServiceError> {
    socket::run(config, Some(material_port), Some(task_operation_port)).await
}

/// An already opened lifecycle store whose writer lock remains owned by the
/// selected host until the composed service exits.
pub struct SelectedLifecycleStore {
    store: lifecycle_store::SqliteLifecycleStore,
    store_id: String,
    pristine: bool,
}

impl SelectedLifecycleStore {
    pub fn open(config: &ServiceConfig) -> Result<Self, ServiceError> {
        let store = lifecycle_store::SqliteLifecycleStore::open(
            &config.store_root,
            config.trusted_issuer.clone(),
        )?;
        if store.root() != config.store_root {
            return Err(ServiceError::Configuration);
        }
        let owner = store.admin_ownership()?;
        if owner.status != "active"
            || owner.handoff_id.is_some()
            || owner.capsule_sha256.is_some()
            || owner.peer_store_id.is_some()
            || owner.selection_revision.is_some()
        {
            return Err(ServiceError::Configuration);
        }
        let recovery = store.load_recovery()?;
        Ok(Self {
            store,
            store_id: owner.store_id,
            pristine: recovery.subjects == 0
                && recovery.commands == 0
                && recovery.unresolved_attempts == 0
                && recovery.journal_cursor == 0,
        })
    }

    pub fn store_id(&self) -> &str {
        &self.store_id
    }

    pub fn pristine(&self) -> bool {
        self.pristine
    }
}

/// The selected task host owns both domain ports and transfers its validated
/// lifecycle store directly to the actor without releasing the writer lock.
pub async fn run_with_selected_task_ports(
    config: ServiceConfig,
    selected: SelectedLifecycleStore,
    material_port: Arc<dyn CanonicalMaterialPort>,
    task_operation_port: Arc<dyn TaskOperationPort>,
) -> Result<(), ServiceError> {
    if config.profile != "retained-repository-analysis-v1"
        || selected.store.root() != config.store_root
    {
        return Err(ServiceError::Configuration);
    }
    socket::run_with_preopened_task_ports(
        config,
        selected.store,
        material_port,
        task_operation_port,
    )
    .await
}
