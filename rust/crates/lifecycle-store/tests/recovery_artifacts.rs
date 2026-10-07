use lifecycle_core::{
    BuildId, ClockSample, ContextGeneration, ProofRunId, SubjectId, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::SqliteLifecycleStore;
use tempfile::tempdir;

fn clock() -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(100),
        wait_budget: WaitBudgetMs::new(100),
    }
}

fn subject() -> SubjectId {
    SubjectId::parse("subject-artifact").unwrap()
}

fn setup(root: &std::path::Path) -> SqliteLifecycleStore {
    let mut store = SqliteLifecycleStore::open(root, "trusted".into()).unwrap();
    store
        .register_subject(
            subject(),
            ContextGeneration::parse("context-1").unwrap(),
            BuildId::parse("build-1").unwrap(),
            ProofRunId::parse("proof-1").unwrap(),
            clock(),
        )
        .unwrap();
    store
}

#[test]
fn staged_bytes_are_invisible_until_complete_publication_and_commit() {
    let root = tempdir().unwrap();
    let mut store = setup(root.path());
    let staged = store.stage_artifact(subject(), b"complete\nbytes").unwrap();
    assert!(
        store
            .read_committed_artifact(staged.digest_hex())
            .unwrap()
            .is_none()
    );
    let digest = staged.digest_hex().to_owned();
    let published = store.publish_artifact(staged, clock()).unwrap();
    assert_eq!(published.digest_hex, digest);
    assert_eq!(
        store.read_committed_artifact(&digest).unwrap(),
        Some(b"complete\nbytes".to_vec())
    );
    assert!(
        std::fs::metadata(root.path().join("lifecycle.sqlite-wal"))
            .unwrap()
            .len()
            > 0
    );
    let snapshot = store
        .snapshot_to(&root.path().join("coherent.sqlite"))
        .unwrap();
    assert_eq!(snapshot.subjects, 1);
    assert!(snapshot.journal_cursor >= 1);
    let image = rusqlite::Connection::open(root.path().join("coherent.sqlite")).unwrap();
    let image_artifacts: i64 = image
        .query_row(
            "SELECT COUNT(*) FROM artifacts WHERE digest_hex=?1",
            [digest.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(image_artifacts, 1);
    let image_cursor: i64 = image
        .query_row("SELECT MAX(sequence) FROM journal", [], |row| row.get(0))
        .unwrap();
    assert_eq!(image_cursor, snapshot.journal_cursor);
    drop(store);
    let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
    assert_eq!(
        store.read_committed_artifact(&digest).unwrap(),
        Some(b"complete\nbytes".to_vec())
    );
}

#[test]
fn conflicting_existing_blob_bytes_are_not_overwritten() {
    let root = tempdir().unwrap();
    let mut store = setup(root.path());
    let staged = store.stage_artifact(subject(), b"correct").unwrap();
    let digest = staged.digest_hex().to_owned();
    let blobs = root.path().join("artifacts/blobs");
    std::fs::create_dir_all(&blobs).unwrap();
    std::fs::write(blobs.join(&digest), b"conflicting").unwrap();
    assert!(store.publish_artifact(staged, clock()).is_err());
    assert_eq!(std::fs::read(blobs.join(&digest)).unwrap(), b"conflicting");
    assert!(store.read_committed_artifact(&digest).unwrap().is_none());
}

#[test]
fn unreferenced_staging_after_restart_cannot_be_read_as_committed_evidence() {
    let root = tempdir().unwrap();
    let store = setup(root.path());
    let staged = store.stage_artifact(subject(), b"orphan").unwrap();
    let digest = staged.digest_hex().to_owned();
    drop(store);
    let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
    assert!(store.read_committed_artifact(&digest).unwrap().is_none());
    assert_eq!(store.collect_orphan_staging().unwrap(), 1);
}
