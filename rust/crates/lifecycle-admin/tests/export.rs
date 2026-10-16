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

fn write_capsule_manifest(receipt: &mut lifecycle_store::CapsuleReceipt) {
    receipt.capsule_sha256.clear();
    receipt.capsule_sha256 = format!("{:x}", Sha256::digest(serde_json::to_vec(receipt).unwrap()));
    fs::write(
        receipt.capsule_root.join("manifest.json"),
        serde_json::to_vec(receipt).unwrap(),
    )
    .unwrap();
}

// This is a real empty schema4 image, never a schema5 image with its version
// changed. No execution/custody row is fabricated to stand in for adoption.
fn schema_four_capsule(root: &std::path::Path) -> lifecycle_store::CapsuleReceipt {
    fs::create_dir(root).unwrap();
    fs::create_dir_all(root.join("artifacts/blobs")).unwrap();
    let database = root.join("lifecycle.sqlite");
    let conn = rusqlite::Connection::open(&database).unwrap();
    for sql in [
        include_str!("../../lifecycle-store/migrations/0001_initial.sql"),
        include_str!("../../lifecycle-store/migrations/0002_service.sql"),
        include_str!("../../lifecycle-store/migrations/0003_native_operation.sql"),
        include_str!("../../lifecycle-store/migrations/0004_composite_s67.sql"),
    ] {
        conn.execute_batch(sql).unwrap();
    }
    conn.execute(
        "INSERT INTO store_identity(singleton,store_id) VALUES(1,'store:archived-four')",
        [],
    )
    .unwrap();
    conn.pragma_update(None, "user_version", 4).unwrap();
    let revision = conn
        .query_row(
            "SELECT revision FROM admin_custody_revision WHERE singleton=1",
            [],
            |r| r.get::<_, u64>(0),
        )
        .unwrap();
    drop(conn);
    let mut receipt = lifecycle_store::CapsuleReceipt {
        version: 1,
        capsule_root: root.to_owned(),
        source_store_id: "store:archived-four".into(),
        source_revision: revision,
        source_schema: 4,
        database_sha256: format!("{:x}", Sha256::digest(fs::read(database).unwrap())),
        artifacts: Vec::new(),
        semantic_artifact_digests: Vec::new(),
        native_invocation_ids: Vec::new(),
        capsule_sha256: String::new(),
    };
    write_capsule_manifest(&mut receipt);
    receipt
}

fn schema_five_capsule(root: &std::path::Path) -> lifecycle_store::CapsuleReceipt {
    fs::create_dir(root).unwrap();
    fs::create_dir_all(root.join("artifacts/blobs")).unwrap();
    let database = root.join("lifecycle.sqlite");
    let conn = rusqlite::Connection::open(&database).unwrap();
    for sql in [
        include_str!("../../lifecycle-store/migrations/0001_initial.sql"),
        include_str!("../../lifecycle-store/migrations/0002_service.sql"),
        include_str!("../../lifecycle-store/migrations/0003_native_operation.sql"),
        include_str!("../../lifecycle-store/migrations/0004_composite_s67.sql"),
        include_str!("../../lifecycle-store/migrations/0005_retained_task.sql"),
    ] {
        conn.execute_batch(sql).unwrap();
    }
    conn.execute(
        "INSERT INTO store_identity(singleton,store_id) VALUES(1,'store:archived-five')",
        [],
    )
    .unwrap();
    conn.pragma_update(None, "user_version", 5).unwrap();
    let revision = conn
        .query_row(
            "SELECT revision FROM admin_custody_revision WHERE singleton=1",
            [],
            |r| r.get::<_, u64>(0),
        )
        .unwrap();
    drop(conn);
    let mut receipt = lifecycle_store::CapsuleReceipt {
        version: 1,
        capsule_root: root.to_owned(),
        source_store_id: "store:archived-five".into(),
        source_revision: revision,
        source_schema: 5,
        database_sha256: format!("{:x}", Sha256::digest(fs::read(database).unwrap())),
        artifacts: Vec::new(),
        semantic_artifact_digests: Vec::new(),
        native_invocation_ids: Vec::new(),
        capsule_sha256: String::new(),
    };
    write_capsule_manifest(&mut receipt);
    receipt
}

#[test]
fn genuine_five_capsule_is_read_only_compatible_and_adopts_to_inactive_six() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("five");
    let receipt = schema_five_capsule(&root);
    let before = capsule_bytes(&root);
    assert_eq!(SqliteLifecycleStore::read_capsule(&root).unwrap(), receipt);
    assert_eq!(capsule_bytes(&root), before);
    let destination = temp.path().join("destination");
    let (adopted, accepted) = SqliteLifecycleStore::adopt_inactive(
        &receipt,
        &destination,
        &lifecycle_core::HandoffId::parse("handoff:five-to-six").unwrap(),
        "test-issuer".into(),
    )
    .unwrap();
    assert_eq!(accepted.source_revision, receipt.source_revision);
    assert_eq!(adopted.admin_ownership().unwrap().status, "inactive");
    assert_eq!(
        adopted.admin_store_identity().unwrap(),
        accepted.destination_store_id
    );
    assert_eq!(capsule_bytes(&root), before);
    let conn = rusqlite::Connection::open_with_flags(
        destination.join("lifecycle.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        6
    );
}

fn capsule_bytes(
    root: &std::path::Path,
) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut result = std::collections::BTreeMap::new();
    let mut pending = vec![root.to_owned()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    result
}

#[test]
fn genuine_four_capsule_read_is_immutable_and_adoption_migrates_only_inactive_copy() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("four");
    let receipt = schema_four_capsule(&root);
    let before = capsule_bytes(&root);
    assert_eq!(SqliteLifecycleStore::read_capsule(&root).unwrap(), receipt);
    assert_eq!(capsule_bytes(&root), before);
    let destination = temp.path().join("destination");
    let handoff = lifecycle_core::HandoffId::parse("handoff:four-to-five").unwrap();
    let (mut adopted, accepted) = SqliteLifecycleStore::adopt_inactive(
        &receipt,
        &destination,
        &handoff,
        "test-issuer".into(),
    )
    .unwrap();
    assert_eq!(accepted.source_store_id, receipt.source_store_id);
    assert_eq!(accepted.source_revision, receipt.source_revision);
    assert_eq!(accepted.capsule_sha256, receipt.capsule_sha256);
    assert_ne!(accepted.destination_store_id, receipt.source_store_id);
    assert_eq!(adopted.admin_ownership().unwrap().status, "inactive");
    assert!(
        adopted
            .prepare_next_input(&SubjectId::parse("subject:absent").unwrap())
            .is_err()
    );
    assert!(
        adopted
            .export_current(&temp.path().join("must-not-export"))
            .is_err()
    );
    let connection = rusqlite::Connection::open_with_flags(
        destination.join("lifecycle.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        6
    );
    drop(connection);
    drop(adopted);
    let (reopened, same) = SqliteLifecycleStore::open_adopted_inactive(
        &destination,
        &handoff,
        &receipt.capsule_sha256,
        "test-issuer".into(),
    )
    .unwrap();
    assert_eq!(same, accepted);
    assert_eq!(reopened.admin_ownership().unwrap().status, "inactive");
    assert_eq!(capsule_bytes(&root), before);
    assert_eq!(SqliteLifecycleStore::read_capsule(&root).unwrap(), receipt);
}

#[test]
fn manifest_schema_must_equal_actual_database_and_unknown_or_incoherent_images_refuse() {
    let temp = tempdir().unwrap();
    let four = temp.path().join("four");
    let mut old = schema_four_capsule(&four);
    old.source_schema = 6;
    write_capsule_manifest(&mut old);
    let before = capsule_bytes(&four);
    assert!(SqliteLifecycleStore::read_capsule(&four).is_err());
    assert_eq!(capsule_bytes(&four), before);

    let source =
        SqliteLifecycleStore::open(&temp.path().join("source"), "test-issuer".into()).unwrap();
    let six = temp.path().join("six");
    let mut current = source.export_current(&six).unwrap();
    assert_eq!(current.source_schema, 6);
    assert_eq!(SqliteLifecycleStore::read_capsule(&six).unwrap(), current);
    for unsupported_or_mismatched in [3, 4, 7] {
        current.source_schema = unsupported_or_mismatched;
        write_capsule_manifest(&mut current);
        let before = capsule_bytes(&six);
        assert!(SqliteLifecycleStore::read_capsule(&six).is_err());
        assert_eq!(capsule_bytes(&six), before);
    }
    current.source_schema = 6;
    let connection = rusqlite::Connection::open(six.join("lifecycle.sqlite")).unwrap();
    connection
        .execute_batch("DROP TRIGGER custody_revision_store_identity_insert")
        .unwrap();
    drop(connection);
    current.database_sha256 = format!(
        "{:x}",
        Sha256::digest(fs::read(six.join("lifecycle.sqlite")).unwrap())
    );
    write_capsule_manifest(&mut current);
    let before = capsule_bytes(&six);
    assert!(SqliteLifecycleStore::read_capsule(&six).is_err());
    assert_eq!(capsule_bytes(&six), before);
}

fn source_custody_revision(root: &std::path::Path) -> u64 {
    rusqlite::Connection::open_with_flags(
        root.join("lifecycle.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .unwrap()
    .query_row(
        "SELECT revision FROM admin_custody_revision WHERE singleton=1",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn capsule_revision_must_match_snapshot_for_schema_four_and_five() {
    let temporary = tempdir().unwrap();
    let four_root = temporary.path().join("four");
    let mut four = schema_four_capsule(&four_root);
    four.source_revision += 1;
    write_capsule_manifest(&mut four);
    let four_before = capsule_bytes(&four_root);
    assert!(SqliteLifecycleStore::read_capsule(&four_root).is_err());
    assert_eq!(capsule_bytes(&four_root), four_before);

    let source_root = temporary.path().join("source");
    let five_root = temporary.path().join("five");
    let mut source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    source
        .register_subject(
            SubjectId::parse("subject-revision-before").unwrap(),
            ContextGeneration::parse("context-revision-before").unwrap(),
            BuildId::parse("build-revision-before").unwrap(),
            ProofRunId::parse("proof-revision-before").unwrap(),
            clock(100),
        )
        .unwrap();
    let mut five = source.export_current(&five_root).unwrap();
    let snapshot_revision = five.source_revision;
    source
        .register_subject(
            SubjectId::parse("subject-revision-after").unwrap(),
            ContextGeneration::parse("context-revision-after").unwrap(),
            BuildId::parse("build-revision-after").unwrap(),
            ProofRunId::parse("proof-revision-after").unwrap(),
            clock(101),
        )
        .unwrap();
    let current_revision = source_custody_revision(&source_root);
    assert!(current_revision > snapshot_revision);
    five.source_revision = current_revision;
    write_capsule_manifest(&mut five);

    // The forged manifest is internally rehashed and claims the current source
    // revision, while the immutable snapshot database still carries the prior one.
    let five_before = capsule_bytes(&five_root);
    let source_before = capsule_bytes(&source_root);
    assert!(SqliteLifecycleStore::read_capsule(&five_root).is_err());
    assert_eq!(capsule_bytes(&five_root), five_before);
    assert_eq!(capsule_bytes(&source_root), source_before);
    assert!(source.export_current(&five_root).is_err());
    assert_eq!(capsule_bytes(&five_root), five_before);
    assert_eq!(capsule_bytes(&source_root), source_before);

    let destination = temporary.path().join("destination");
    assert!(
        SqliteLifecycleStore::adopt_inactive(
            &five,
            &destination,
            &lifecycle_core::HandoffId::parse("handoff:stale-capsule").unwrap(),
            "test-issuer".into(),
        )
        .is_err()
    );
    assert!(!destination.exists());
    assert_eq!(capsule_bytes(&five_root), five_before);
    assert_eq!(capsule_bytes(&source_root), source_before);
}

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
    assert_eq!(first.source_schema, 6);
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
