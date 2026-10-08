use std::fs;

use lifecycle_core::{
    BuildId, ClockSample, ContextGeneration, ProofRunId, SubjectId, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::SqliteLifecycleStore;
use tempfile::tempdir;

#[test]
fn incompatible_future_store_is_refused_before_database_mutation() {
    let root = tempdir().unwrap();
    let database = root.path().join("lifecycle.sqlite");
    let conn = rusqlite::Connection::open(&database).unwrap();
    conn.pragma_update(None, "user_version", 3).unwrap();
    drop(conn);
    let before = fs::read(&database).unwrap();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    assert_eq!(fs::read(&database).unwrap(), before);
}

#[test]
fn empty_unversioned_file_can_finish_first_migration_but_unknown_tables_cannot() {
    let root = tempdir().unwrap();
    let database = root.path().join("lifecycle.sqlite");
    drop(rusqlite::Connection::open(&database).unwrap());
    let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
    assert_eq!(store.load_recovery().unwrap().subjects, 0);
    drop(store);

    let other = tempdir().unwrap();
    let database = other.path().join("lifecycle.sqlite");
    let conn = rusqlite::Connection::open(&database).unwrap();
    conn.execute_batch("CREATE TABLE alien(x INTEGER);")
        .unwrap();
    drop(conn);
    assert!(SqliteLifecycleStore::open(other.path(), "trusted".into()).is_err());
}

#[test]
fn recovery_refuses_projection_with_missing_journal_fact() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
    store
        .register_subject(
            SubjectId::parse("subject-coherence").unwrap(),
            ContextGeneration::parse("context-coherence").unwrap(),
            BuildId::parse("build-coherence").unwrap(),
            ProofRunId::parse("proof-coherence").unwrap(),
            ClockSample {
                wall: WallTimeMs::new(1),
                wait_budget: WaitBudgetMs::new(1),
            },
        )
        .unwrap();
    drop(store);
    let database = root.path().join("lifecycle.sqlite");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.execute("INSERT INTO artifacts(digest_hex,subject_id,byte_length,relative_path,committed_wall_ms) VALUES ('missing','subject-coherence',1,'artifacts/blobs/missing',1)",[]).unwrap();
    drop(connection);
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
}
