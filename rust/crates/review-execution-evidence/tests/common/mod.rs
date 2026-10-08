use std::fs;
use std::path::Path;

use claim_evidence::{ProductionPathAccess, ProductionPathAdmissionBinding, ProductionPathStage};
use review_execution_evidence::{ControlledScope, ExecutionBinding, ExecutionProvenance};
use sha2::{Digest, Sha256};

pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn scope(root_id: &str, count_path: &Path, mode: &str) -> ControlledScope {
    let executable = fs::canonicalize(env!("CARGO_BIN_EXE_controlled_peer")).unwrap();
    let workspace = count_path.parent().unwrap().canonicalize().unwrap();
    ControlledScope {
        root_id: root_id.into(),
        provenance: ExecutionProvenance {
            binding: ExecutionBinding {
                campaign_root_id: "campaign-1".into(),
                obligation_id: "obligation-1".into(),
                candidate_digest: "a".repeat(64),
                selection_digest: "b".repeat(64),
                profile_digest: "c".repeat(64),
                prepared_request_sha256: "d".repeat(64),
                review_episode_id: "episode-1".into(),
                attempt_id: "attempt-1".into(),
            },
            dispatch_operation_id: "dispatch-1".into(),
            dispatch_revision: "dispatch-rev-1".into(),
            execution_session_id: Some("controlled-session-1".into()),
        },
        selection_id: "selection-1".into(),
        candidate_commit: "commit-1".into(),
        candidate_tree: "tree-1".into(),
        candidate_patch_identity: "patch-1".into(),
        request_bytes: b"{\"task\":\"inspect\"}".to_vec(),
        executable_sha256: sha(&fs::read(&executable).unwrap()),
        executable,
        arguments: vec![count_path.to_str().unwrap().into(), mode.into()],
        workspace,
        source_sha256: "e".repeat(64),
    }
}

pub fn binding(result_digest: &str) -> ProductionPathAdmissionBinding {
    ProductionPathAdmissionBinding {
        campaign_root_id: "campaign-1".into(),
        campaign_revision: "completion-rev".into(),
        campaign_operation_id: "completion-op".into(),
        obligation_id: "obligation-1".into(),
        candidate_digest: "a".repeat(64),
        selection_digest: "b".repeat(64),
        profile_digest: "c".repeat(64),
        prepared_request_sha256: "d".repeat(64),
        child_request_sha256: "f".repeat(64),
        review_episode_id: "episode-1".into(),
        attempt_id: "attempt-1".into(),
        native_result_sha256: result_digest.into(),
        episode_revision: None,
        session_id: Some("controlled-session-1".into()),
        stage: ProductionPathStage::RecordObservation,
        access: ProductionPathAccess::Original,
    }
}

pub fn invocation_count(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|text| text.lines().count())
        .unwrap_or(0)
}
