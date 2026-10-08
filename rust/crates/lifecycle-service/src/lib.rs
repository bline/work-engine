//! Offline Rust lifecycle service. Only the controlled-proof feature permits external text execution.

mod actor;
mod admission;
mod controlled;
mod engine;
mod operations;
mod projection;
mod socket;

use std::path::PathBuf;

use operations::workspace_snapshot_read::TrustedSnapshot;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
}

pub async fn run(config: ServiceConfig) -> Result<(), ServiceError> {
    socket::run(config).await
}
