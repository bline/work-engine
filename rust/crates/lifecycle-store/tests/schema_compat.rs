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
    conn.pragma_update(None, "user_version", 7).unwrap();
    drop(conn);
    let before = fs::read(&database).unwrap();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    assert_eq!(fs::read(&database).unwrap(), before);
}

#[test]
fn genuine_schema_four_migrates_to_six_without_changing_store_identity() {
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
        6
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='trigger' AND name LIKE 'custody_revision_%'",
            [], |r| r.get::<_, i64>(0),
        ).unwrap(),
        123
    );
}

#[test]
fn genuine_schema_five_migrates_to_six_without_changing_store_identity() {
    let root = tempdir().unwrap();
    let database = root.path().join("lifecycle.sqlite");
    let conn = rusqlite::Connection::open(&database).unwrap();
    for sql in [
        include_str!("../migrations/0001_initial.sql"),
        include_str!("../migrations/0002_service.sql"),
        include_str!("../migrations/0003_native_operation.sql"),
        include_str!("../migrations/0004_composite_s67.sql"),
        include_str!("../migrations/0005_retained_task.sql"),
    ] {
        conn.execute_batch(sql).unwrap();
    }
    conn.execute(
        "INSERT INTO store_identity(singleton,store_id) VALUES(1,'store:schema-five')",
        [],
    )
    .unwrap();
    conn.pragma_update(None, "user_version", 5).unwrap();
    drop(conn);
    let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
    assert_eq!(store.admin_store_identity().unwrap(), "store:schema-five");
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
        6
    );
    assert_eq!(conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name GLOB 'custody_revision_*'",[],|r|r.get::<_,i64>(0)).unwrap(),123);
    assert_eq!(
        conn.query_row(
            "SELECT mode FROM legacy_held_state WHERE singleton=1",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "ordinary"
    );
}

fn predecessor_database(root: &std::path::Path, version: i64, injected_sql: &str) {
    let conn = rusqlite::Connection::open(root.join("lifecycle.sqlite")).unwrap();
    for sql in [
        include_str!("../migrations/0001_initial.sql"),
        include_str!("../migrations/0002_service.sql"),
        include_str!("../migrations/0003_native_operation.sql"),
        include_str!("../migrations/0004_composite_s67.sql"),
    ] {
        conn.execute_batch(sql).unwrap();
    }
    if version == 5 {
        conn.execute_batch(include_str!("../migrations/0005_retained_task.sql"))
            .unwrap();
    }
    conn.execute(
        "INSERT INTO store_identity(singleton,store_id) VALUES(1,'store:failed-upgrade')",
        [],
    )
    .unwrap();
    conn.execute_batch(injected_sql).unwrap();
    conn.pragma_update(None, "user_version", version).unwrap();
}

#[test]
fn incoherent_schema_five_refuses_before_any_database_byte_changes() {
    let root = tempdir().unwrap();
    predecessor_database(
        root.path(),
        5,
        "DROP TRIGGER custody_revision_grants_insert;",
    );
    let database = root.path().join("lifecycle.sqlite");
    let before = fs::read(&database).unwrap();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    assert_eq!(fs::read(&database).unwrap(), before);
}

#[test]
fn failed_schema_six_migration_rolls_back_both_predecessor_versions() {
    for version in [4, 5] {
        let root = tempdir().unwrap();
        // This collides with 0006 only after the valid predecessor preflight.
        predecessor_database(
            root.path(),
            version,
            "CREATE TABLE legacy_held_state(x INTEGER);",
        );
        let database = root.path().join("lifecycle.sqlite");
        let before = fs::read(&database).unwrap();
        assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
        assert_eq!(fs::read(&database).unwrap(), before);
    }
}

#[test]
fn failed_wal_schema_five_upgrade_preserves_existing_database_bytes() {
    let root = tempdir().unwrap();
    predecessor_database(root.path(), 5, "CREATE TABLE legacy_held_state(x INTEGER);");
    let database = root.path().join("lifecycle.sqlite");
    let conn = rusqlite::Connection::open(&database).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    drop(conn);
    fs::write(root.path().join("writer.lock"), b"").unwrap();
    let snapshot = || {
        fs::read_dir(root.path())
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                (
                    path.file_name().unwrap().to_os_string(),
                    fs::read(&path).unwrap(),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let before = snapshot();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    assert_eq!(snapshot(), before);
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
