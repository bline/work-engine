#![allow(dead_code)]
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use claim_evidence::codec::{JsValue, parse_json};
use review_execution_evidence::claude::{
    CredentialSource, PinnedProgram, QualifiedScope, QualifiedSelection, TrustedClaudeProfile,
};
use review_execution_evidence::{ExecutionBinding, ExecutionProvenance};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn tree_sha(root: &Path) -> String {
    let mut paths = Vec::new();
    let mut stack = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        for item in fs::read_dir(dir).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                paths.push(path);
            }
        }
    }
    paths.sort();
    let mut hash = Sha256::new();
    for path in paths {
        let name = path.strip_prefix(root).unwrap().to_str().unwrap();
        let bytes = fs::read(&path).unwrap();
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    format!("{:x}", hash.finalize())
}

pub struct Fixture {
    pub scope: QualifiedScope,
    pub root: PathBuf,
    pub candidate: PathBuf,
    pub witness: PathBuf,
}

pub fn fixture(parent: &Path, mode: &str, horizon: Duration) -> Fixture {
    fixture_with_credential(parent, mode, horizon, None)
}

pub fn fixture_with_credential(
    parent: &Path,
    mode: &str,
    horizon: Duration,
    credential: Option<CredentialSource>,
) -> Fixture {
    let candidate = parent.join("candidate");
    fs::create_dir_all(&candidate).unwrap();
    if !candidate.join("source.rs").exists() {
        fs::write(
            candidate.join("source.rs"),
            b"pub fn exact_candidate() {}\n",
        )
        .unwrap();
    }
    let candidate = candidate.canonicalize().unwrap();
    let executable = parent.join("installed-offline-peer");
    if !executable.exists() {
        fs::copy(env!("CARGO_BIN_EXE_claude_offline_peer"), &executable).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let executable = executable.canonicalize().unwrap();
    let program = PinnedProgram::load(&executable, &sha(&fs::read(&executable).unwrap())).unwrap();
    let profile = TrustedClaudeProfile::load_offline_peer(
        program,
        "offline-test-model-v1",
        "high",
        vec!["offline-test-model-v1".into()],
        b"{\"offline\":true}",
    )
    .unwrap();
    let subject: Value = json!({"commit":"commit-1","tree":"tree-1","patchIdentity":"patch-1"});
    let result = json!({"schemaVersion":1,"subject":subject,"verdict":"acceptable_as_is",
        "findings":[],"decisiveEvidence":[{"path":"source.rs","startLine":1,"endLine":1,
            "sha256":"a".repeat(64)}],"limitations":[]});
    let prompt = json!({"mode":mode,"result":result})
        .to_string()
        .into_bytes();
    let subject: JsValue = parse_json(&subject.to_string()).unwrap();
    let selection = QualifiedSelection {
        root_id: "claude-root-1".into(),
        provenance: ExecutionProvenance {
            binding: ExecutionBinding {
                campaign_root_id: "campaign-1".into(),
                obligation_id: "obligation-1".into(),
                candidate_digest: "b".repeat(64),
                selection_digest: "c".repeat(64),
                profile_digest: profile.profile_digest().into(),
                prepared_request_sha256: sha(&prompt),
                review_episode_id: "episode-1".into(),
                attempt_id: "attempt-1".into(),
            },
            dispatch_operation_id: "dispatch-1".into(),
            dispatch_revision: "dispatch-rev-1".into(),
            execution_session_id: Some("session-1".into()),
        },
        selection_id: "selection-1".into(),
        candidate_commit: "commit-1".into(),
        candidate_tree: "tree-1".into(),
        candidate_patch_identity: "patch-1".into(),
        candidate_path: candidate.clone(),
        candidate_tree_sha256: tree_sha(&candidate),
        cbm_project: "offline-project".into(),
        cbm_store_sha256: "d".repeat(64),
        cbm_generation: "offline-generation".into(),
        cbm_coverage_sha256: "e".repeat(64),
        prompt,
        subject,
        credential,
        offline_witness: Some(parent.join("peer-witness")),
        horizon,
    };
    Fixture {
        scope: QualifiedScope::bind_offline_peer(profile, selection).unwrap(),
        root: parent.join("evidence-root"),
        candidate,
        witness: parent.join("peer-witness"),
    }
}

pub fn counts(path: &Path) -> (usize, usize) {
    let text = fs::read_to_string(path).unwrap_or_default();
    (
        text.lines().filter(|line| *line == "start").count(),
        text.lines().filter(|line| *line == "prompt").count(),
    )
}
