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
    conn.pragma_update(None, "user_version", 6).unwrap();
    drop(conn);
    let before = fs::read(&database).unwrap();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    assert_eq!(fs::read(&database).unwrap(), before);
}

#[test]
fn genuine_schema_four_migrates_to_five_without_changing_store_identity() {
    let root = tempdir().unwrap();
    let database = root.path().join("lifecycle.sqlite");
    let conn = rusqlite::Connection::open(&database).unwrap();
    for sql in [
        include_str!("../migrations/0001_initial.sql"),
        include_str!("../migrations/0002_service.sql"),
        include_str!("../migrations/0003_native_operation.sql"),
        include_str!("../migrations/0004_composite_s67.sql"),
    ] {
        conn.execute_batch(sql).unwrap();
    }
    conn.execute(
        "INSERT INTO store_identity(singleton,store_id) VALUES(1,'store:schema-four')",
        [],
    )
    .unwrap();
    conn.pragma_update(None, "user_version", 4).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='trigger' AND name LIKE 'custody_revision_%'",
            [], |r| r.get::<_, i64>(0),
        ).unwrap(),
        102
    );
    drop(conn);
    let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
    assert_eq!(store.admin_store_identity().unwrap(), "store:schema-four");
    assert_eq!(store.admin_ownership().unwrap().status, "active");
    drop(store);
    let conn = rusqlite::Connection::open_with_flags(
        &database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='trigger' AND name LIKE 'custody_revision_%'",
            [], |r| r.get::<_, i64>(0),
        ).unwrap(),
        111
    );
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
