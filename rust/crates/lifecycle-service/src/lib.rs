//! Rust lifecycle service with explicit, route-bound native provider execution.

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

pub use actor::task_effect_hold;

use std::path::PathBuf;

use lifecycle_core::{ContextGeneration, NativeSessionId, SnapshotId, SubjectId};
use lifecycle_store::{NativeRestartEpoch, NativeRestartIntent};
use operations::workspace_snapshot_read::TrustedSnapshot;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

pub use material_port::{CanonicalMaterialPort, MaterialPortError};
pub use task_port::{
    CompletedTaskEffect, CompletedTaskEffectAttribution, CompletedTaskEffectReadback, GrantFence,
    QuestionOpenPayload, ResultPublishPayload, TaskCursorBinding, TaskInputBinding,
    TaskOperationCommit, TaskOperationError, TaskOperationPort, ValidatedTaskOperationEntry,
};

/// Rust callers may leave this absent, but only the selected-store preflight
/// can construct a productive restart grant. Deserialization always skips it.
#[derive(Clone, Debug)]
pub struct SelectedRestartGrant {
    intent: NativeRestartIntent,
}

/// Exact selected-host declaration after the host has verified its receipt,
/// current task owner and predecessor process exclusion. Public Rust callers
/// may supply a request, but only SelectedLifecycleStore can seal the grant.
#[derive(Clone, Debug)]
pub struct SelectedRestartRequest {
    pub subject_id: String,
    pub context_generation: String,
    pub predecessor_session_id: String,
    pub predecessor_epoch: u64,
    pub target_session_id: String,
    pub expected_store_id: String,
    pub expected_subject_revision: u64,
    pub expected_semantic_revision: u64,
    pub expected_queue_revision: u64,
    pub expected_journal_cursor: u64,
    pub intent_sha256: String,
    pub selection_sha256: String,
    pub profile_digest: String,
    pub binary_sha256: String,
    pub protocol_digest: String,
    pub snapshot_id: String,
    pub snapshot_manifest_sha256: String,
}

impl SelectedRestartRequest {
    fn into_intent(self) -> Result<NativeRestartIntent, ServiceError> {
        Ok(NativeRestartIntent {
            subject: SubjectId::parse(self.subject_id).map_err(|_| ServiceError::Configuration)?,
            context: ContextGeneration::parse(self.context_generation)
                .map_err(|_| ServiceError::Configuration)?,
            predecessor_session: NativeSessionId::parse(self.predecessor_session_id)
                .map_err(|_| ServiceError::Configuration)?,
            predecessor_epoch: self.predecessor_epoch,
            target_session: NativeSessionId::parse(self.target_session_id)
                .map_err(|_| ServiceError::Configuration)?,
            expected_store_id: self.expected_store_id,
            expected_subject_revision: self.expected_subject_revision,
            expected_semantic_revision: self.expected_semantic_revision,
            expected_queue_revision: self.expected_queue_revision,
            expected_journal_cursor: self.expected_journal_cursor,
            intent_sha256: self.intent_sha256,
            selection_sha256: self.selection_sha256,
            profile_digest: self.profile_digest,
            binary_sha256: self.binary_sha256,
            protocol_digest: self.protocol_digest,
            snapshot: SnapshotId::parse(self.snapshot_id)
                .map_err(|_| ServiceError::Configuration)?,
            snapshot_manifest_sha256: self.snapshot_manifest_sha256,
        })
    }
}

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
    /// Set only by the selected-store handoff after receipt and restart
    /// preflight. JSON input cannot grant a productive execution epoch.
    #[serde(skip)]
    pub selected_restart: Option<SelectedRestartGrant>,
}

/// Opt-in installed-native simulated profile. The named snapshot is supplied
/// by trusted composition; native tool arguments cannot add a path or source.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeConfig {
    pub executable: PathBuf,
    pub root: PathBuf,
    #[serde(default)]
    pub provider_route: NativeProviderRoute,
    /// Only used by the simulator and controlled-proof fake routes. Live
    /// OpenAI always uses the compiled upstream and requires this to be empty.
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

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum NativeProviderRoute {
    #[default]
    NativeSimProofV1,
    OpenaiResponsesLiveV1,
    OpenaiResponsesFakeV1,
}

impl NativeProviderRoute {
    pub const MODEL: &'static str = "gpt-6-sol";
    pub const LIVE_BASE_URL: &'static str = "https://api.openai.com/v1";

    pub fn identity(self) -> &'static str {
        match self {
            Self::NativeSimProofV1 => "native-sim-proof-v1",
            Self::OpenaiResponsesLiveV1 => "openai-responses-live-v1",
            Self::OpenaiResponsesFakeV1 => "openai-responses-fake-v1",
        }
    }

    pub fn provider_name(self) -> &'static str {
        match self {
            Self::NativeSimProofV1 => "local_probe",
            Self::OpenaiResponsesLiveV1 => "openai_responses_live",
            Self::OpenaiResponsesFakeV1 => "openai_responses_fake",
        }
    }

    pub fn requires_runtime_key(self) -> bool {
        self != Self::NativeSimProofV1
    }

    pub fn configured_base_url(self, simulator_base_url: &str) -> Option<String> {
        fn loopback(url: &str) -> Option<String> {
            let port = url
                .strip_prefix("http://127.0.0.1:")?
                .strip_suffix("/v1")?
                .parse::<u16>()
                .ok()?;
            (port != 0).then(|| url.to_owned())
        }
        match self {
            Self::NativeSimProofV1 => loopback(simulator_base_url),
            Self::OpenaiResponsesLiveV1 if simulator_base_url.is_empty() => {
                Some(Self::LIVE_BASE_URL.to_owned())
            }
            Self::OpenaiResponsesFakeV1 if cfg!(feature = "controlled-proof") => {
                loopback(simulator_base_url)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod provider_route_tests {
    use super::*;

    #[test]
    fn provider_routes_bind_identity_and_endpoint_without_proxy_relabeling() {
        assert_eq!(
            NativeProviderRoute::default().identity(),
            "native-sim-proof-v1"
        );
        assert_eq!(
            NativeProviderRoute::OpenaiResponsesLiveV1.configured_base_url(""),
            Some("https://api.openai.com/v1".into())
        );
        for proxy in ["http://127.0.0.1:1234/v1", "https://proxy.example/v1"] {
            assert!(
                NativeProviderRoute::OpenaiResponsesLiveV1
                    .configured_base_url(proxy)
                    .is_none()
            );
        }
        assert!(
            NativeProviderRoute::NativeSimProofV1
                .configured_base_url("https://api.openai.com/v1")
                .is_none()
        );
        assert_eq!(
            NativeProviderRoute::OpenaiResponsesFakeV1
                .configured_base_url("http://127.0.0.1:1234/v1")
                .is_some(),
            cfg!(feature = "controlled-proof")
        );
    }
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
    restart_intent: Option<NativeRestartIntent>,
}

impl SelectedLifecycleStore {
    pub fn open(config: &ServiceConfig) -> Result<Self, ServiceError> {
        let store = lifecycle_store::SqliteLifecycleStore::open(
            &config.store_root,
            config.trusted_issuer.clone(),
        )?;
        Self::from_store(config, store)
    }

    /// The selected restart path refuses invalid existing bytes before any
    /// mutating open and keeps the same writer lock for authorization.
    pub fn open_for_restart(
        config: &ServiceConfig,
        expected_store_id: &str,
    ) -> Result<Self, ServiceError> {
        let store = lifecycle_store::SqliteLifecycleStore::open_selected_restart(
            &config.store_root,
            config.trusted_issuer.clone(),
            expected_store_id,
        )?;
        Self::from_store(config, store)
    }

    fn from_store(
        config: &ServiceConfig,
        store: lifecycle_store::SqliteLifecycleStore,
    ) -> Result<Self, ServiceError> {
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
            restart_intent: None,
        })
    }

    pub fn store_id(&self) -> &str {
        &self.store_id
    }

    pub fn pristine(&self) -> bool {
        self.pristine
    }

    /// Form and record the exact authorized epoch while the selected host
    /// retains both store locks. A duplicate exact intent is read back; it
    /// never grants a second allocation after the target session exists.
    pub fn authorize_restart(
        &mut self,
        request: SelectedRestartRequest,
    ) -> Result<NativeRestartEpoch, ServiceError> {
        let intent = request.into_intent()?;
        if intent.expected_store_id != self.store_id {
            return Err(ServiceError::Configuration);
        }
        let duplicate = self
            .store
            .native_restart_authorization(&intent.target_session)?;
        let basis = self
            .store
            .native_restart_basis(&intent.subject, &intent.context)?;
        if basis.store_id != intent.expected_store_id
            || basis.subject_id != intent.subject.as_str()
            || basis.context_id != intent.context.as_str()
            || basis.owner_kind != "domain"
        {
            return Err(ServiceError::Configuration);
        }
        if duplicate
            .as_ref()
            .is_some_and(|epoch| epoch.state == "active")
        {
            if basis.predecessor_session_id != intent.target_session.as_str()
                || Some(basis.predecessor_epoch) != intent.predecessor_epoch.checked_add(1)
            {
                return Err(ServiceError::Configuration);
            }
        } else if basis.predecessor_session_id != intent.predecessor_session.as_str()
            || basis.predecessor_epoch != intent.predecessor_epoch
            || basis.subject_revision != intent.expected_subject_revision
            || basis.semantic_revision != intent.expected_semantic_revision
            || basis.queue_revision != intent.expected_queue_revision
            || basis.journal_cursor != intent.expected_journal_cursor
            || basis.profile_digest != intent.profile_digest
            || basis.binary_sha256 != intent.binary_sha256
            || basis.protocol_digest != intent.protocol_digest
            || basis.snapshot_id != intent.snapshot.as_str()
            || basis.snapshot_manifest_sha256 != intent.snapshot_manifest_sha256
        {
            return Err(ServiceError::Configuration);
        }
        let epoch = self.store.authorize_native_restart(&intent)?;
        self.restart_intent = Some(intent);
        Ok(epoch)
    }
}

/// The selected task host owns both domain ports and transfers its validated
/// lifecycle store directly to the actor without releasing the writer lock.
pub async fn run_with_selected_task_ports(
    mut config: ServiceConfig,
    selected: SelectedLifecycleStore,
    material_port: Arc<dyn CanonicalMaterialPort>,
    task_operation_port: Arc<dyn TaskOperationPort>,
) -> Result<(), ServiceError> {
    if config.profile != "retained-repository-analysis-v1"
        || selected.store.root() != config.store_root
    {
        return Err(ServiceError::Configuration);
    }
    config.selected_restart = selected
        .restart_intent
        .map(|intent| SelectedRestartGrant { intent });
    socket::run_with_preopened_task_ports(
        config,
        selected.store,
        material_port,
        task_operation_port,
    )
    .await
}
