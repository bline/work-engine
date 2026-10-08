use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

use lifecycle_core::{
    BuildId, ClockSample, ContextGeneration, ProofRunId, SubjectId, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::SqliteLifecycleStore;
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use work_engine_types::CodecContract;

fn clock(wall: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(wall),
        wait_budget: WaitBudgetMs::new(100),
    }
}

#[test]
fn coherent_export_includes_committed_artifact_and_rejects_partial_or_corrupt_closure() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let capsule_one = temporary.path().join("capsule-one");
    let capsule_two = temporary.path().join("capsule-two");
    let capsule_three = temporary.path().join("capsule-three");
    let mut store = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let subject = SubjectId::parse("subject-export").unwrap();
    store
        .register_subject(
            subject.clone(),
            ContextGeneration::parse("context-export").unwrap(),
            BuildId::parse("build-export").unwrap(),
            ProofRunId::parse("proof-export").unwrap(),
            clock(100),
        )
        .unwrap();
    let staged = store
        .stage_artifact(subject, b"committed artifact in WAL era")
        .unwrap();
    let published = store.publish_artifact(staged, clock(101)).unwrap();

    let first = store.export_current(&capsule_one).unwrap();
    assert_eq!(first.source_schema, 4);
    assert_eq!(first.artifacts.len(), 1);
    assert_eq!(first.artifacts[0].digest_hex, published.digest_hex);
    assert_eq!(store.export_current(&capsule_one).unwrap(), first);
    assert_eq!(
        SqliteLifecycleStore::read_capsule(&capsule_one).unwrap(),
        first
    );

    fs::write(
        capsule_one.join(&first.artifacts[0].relative_path),
        b"tampered artifact",
    )
    .unwrap();
    assert!(SqliteLifecycleStore::read_capsule(&capsule_one).is_err());
    assert!(store.export_current(&capsule_one).is_err());

    let second = store.export_current(&capsule_two).unwrap();
    fs::remove_file(capsule_two.join(&second.artifacts[0].relative_path)).unwrap();
    assert!(SqliteLifecycleStore::read_capsule(&capsule_two).is_err());

    let mut wrong_codec = store.export_current(&capsule_three).unwrap();
    wrong_codec.artifacts[0].codec = "unknown-artifact-v2".into();
    wrong_codec.capsule_sha256.clear();
    wrong_codec.capsule_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&wrong_codec).unwrap())
    );
    fs::write(
        capsule_three.join("manifest.json"),
        serde_json::to_vec(&wrong_codec).unwrap(),
    )
    .unwrap();
    assert!(SqliteLifecycleStore::read_capsule(&capsule_three).is_err());
}

#[test]
fn actual_process_exit_after_blob_sync_never_exports_uncommitted_reference() {
    let executable = env!("CARGO_BIN_EXE_lifecycle-admin");
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let partial_capsule = temporary.path().join("partial-capsule");
    let complete_capsule = temporary.path().join("complete-capsule");
    let barrier = temporary.path().join("barrier");
    let bytes_path = temporary.path().join("artifact-bytes");
    let bytes = b"blob synchronized before database reference";
    fs::create_dir(&barrier).unwrap();
    fs::write(&bytes_path, bytes).unwrap();
    let digest = CodecContract::BinaryArtifactV1
        .digest_binary(bytes)
        .unwrap()
        .hex();
    let mut store = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let subject = SubjectId::parse("subject-blob-cut").unwrap();
    store
        .register_subject(
            subject.clone(),
            ContextGeneration::parse("context-blob-cut").unwrap(),
            BuildId::parse("build-blob-cut").unwrap(),
            ProofRunId::parse("proof-blob-cut").unwrap(),
            clock(100),
        )
        .unwrap();
    drop(store);

    let mut child = Command::new(executable)
        .arg("publish-artifact-cut")
        .arg(&source_root)
        .arg(subject.as_str())
        .arg(&bytes_path)
        .arg(&barrier)
        .spawn()
        .unwrap();
    let reached = barrier.join("artifact_after_blob_sync.reached");
    let deadline = Instant::now() + Duration::from_secs(3);
    while !reached.exists() && Instant::now() < deadline {
        assert!(
            child.try_wait().unwrap().is_none(),
            "artifact publisher exited before cut"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read(&reached).unwrap(), b"reached\n");
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());

    let mut store = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    assert_eq!(store.read_committed_artifact(&digest).unwrap(), None);
    let partial = store.export_current(&partial_capsule).unwrap();
    assert!(partial.artifacts.is_empty());
    assert_eq!(
        SqliteLifecycleStore::read_capsule(&partial_capsule).unwrap(),
        partial
    );
    let staged = store.stage_artifact(subject, bytes).unwrap();
    let published = store.publish_artifact(staged, clock(201)).unwrap();
    assert_eq!(published.digest_hex, digest);
    let complete = store.export_current(&complete_capsule).unwrap();
    assert_eq!(complete.artifacts.len(), 1);
    assert_eq!(complete.artifacts[0].digest_hex, digest);
}
