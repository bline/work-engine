//! Offline Rust lifecycle service. Only the controlled-proof feature permits external text execution.

mod actor;
mod admission;
mod controlled;
mod engine;
mod projection;
mod socket;

use std::path::PathBuf;

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
