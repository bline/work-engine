use std::collections::BTreeMap;
use std::fs;

use lifecycle_core::{
    BuildId, ClockSample, ContextGeneration, HandoffId, NativeSessionId, ProofRunId,
    RuntimeIncarnation, SnapshotId, SubjectId, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::{NativeRestartIntent, SqliteLifecycleStore};
use tempfile::{tempdir, tempdir_in};

fn sqlite_image_bytes(root: &std::path::Path) -> BTreeMap<std::ffi::OsString, Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name()?.to_os_string();
            matches!(
                name.to_str(),
                Some("lifecycle.sqlite" | "lifecycle.sqlite-wal")
            )
            .then(|| (name, fs::read(path).unwrap()))
        })
        .collect()
}

fn retained_table_rows(database: &std::path::Path) -> BTreeMap<String, Vec<String>> {
    let connection = rusqlite::Connection::open(database).unwrap();
    let mut tables = connection
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('native_execution_epochs','admin_custody_revision') ORDER BY name")
        .unwrap();
    let names: Vec<String> = tables
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut result = BTreeMap::new();
    for name in names {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM \"{name}\""))
            .unwrap();
        let columns = statement.column_count();
        let mut rows: Vec<String> = statement
            .query_map([], |row| {
                let mut cells = Vec::with_capacity(columns);
                for index in 0..columns {
                    cells.push(format!("{:?}", row.get_ref(index)?));
                }
                Ok(cells.join("|"))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        rows.sort();
        result.insert(name, rows);
    }
    result
}

fn assert_legacy_current_capsule_roundtrip(root: &std::path::Path, store: &SqliteLifecycleStore) {
    let peer = tempdir_in(root.parent().unwrap()).unwrap();
    let capsule = peer.path().join("capsule");
    let receipt = store.export_current(&capsule).unwrap();
    assert_eq!(receipt.source_schema, 6);
    assert_eq!(
        SqliteLifecycleStore::read_capsule(&capsule).unwrap(),
        receipt
    );
    let destination = peer.path().join("adopted");
    let (adopted, _) = SqliteLifecycleStore::adopt_inactive(
        &receipt,
        &destination,
        &HandoffId::parse("handoff:legacy-current").unwrap(),
        "trusted".into(),
    )
    .unwrap();
    assert_eq!(adopted.admin_ownership().unwrap().status, "inactive");
}

#[test]
fn incompatible_future_store_is_refused_before_database_mutation() {
    let root = tempdir().unwrap();
    let database = root.path().join("lifecycle.sqlite");
    let conn = rusqlite::Connection::open(&database).unwrap();
    conn.pragma_update(None, "user_version", 8).unwrap();
    drop(conn);
    let before = fs::read(&database).unwrap();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    assert_eq!(fs::read(&database).unwrap(), before);
}

#[test]
fn selected_restart_refuses_inactive_and_invalid_six_without_image_mutation() {
    for change in [
        "UPDATE admin_ownership SET status='inactive' WHERE singleton=1",
        "DROP TRIGGER custody_revision_native_sessions_insert",
    ] {
        let root = tempdir().unwrap();
        let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        let identity = store.admin_store_identity().unwrap();
        drop(store);
        let db = root.path().join("lifecycle.sqlite");
        let connection = rusqlite::Connection::open(&db).unwrap();
        connection.execute_batch(change).unwrap();
        let before = sqlite_image_bytes(root.path());
        assert!(
            SqliteLifecycleStore::open_selected_restart(root.path(), "trusted".into(), &identity)
                .is_err(),
            "{change}"
        );
        assert!(
            sqlite_image_bytes(root.path()) == before,
            "{change} changed the image"
        );
        drop(connection);
        let readback = rusqlite::Connection::open(&db).unwrap();
        assert_eq!(
            readback
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            6,
            "{change}"
        );
    }
}

fn restart_clock() -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(10),
        wait_budget: WaitBudgetMs::new(1),
    }
}

#[test]
fn selected_restart_opt_in_preserves_predecessor_and_requires_exact_authorization() {
    let root = tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
    let subject = SubjectId::parse("subject:restart").unwrap();
    let context = ContextGeneration::parse("context:restart").unwrap();
    let predecessor = NativeSessionId::parse("session:old").unwrap();
    let target = NativeSessionId::parse("session:new").unwrap();
    let snapshot = SnapshotId::parse("snapshot:restart").unwrap();
    store
        .register_subject(
            subject.clone(),
            context.clone(),
            BuildId::parse("build:restart").unwrap(),
            ProofRunId::parse("proof:restart").unwrap(),
            restart_clock(),
        )
        .unwrap();
    let profile = "a".repeat(64);
    let binary = "b".repeat(64);
    let protocol = "c".repeat(64);
    let manifest = "d".repeat(64);
    let _predecessor_entry = store
        .reserve_native_session(
            predecessor.clone(),
            subject.clone(),
            context.clone(),
            RuntimeIncarnation::parse("incarnation:old").unwrap(),
            &profile,
            &binary,
            &protocol,
            snapshot.clone(),
            &manifest,
            false,
            restart_clock(),
        )
        .unwrap();
    let store_id = store.admin_store_identity().unwrap();
    drop(store);
    let wal_keeper = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    wal_keeper
        .execute(
            "UPDATE subjects SET last_wall_ms=last_wall_ms WHERE subject_id='subject:restart'",
            [],
        )
        .unwrap();
    let before = sqlite_image_bytes(root.path());
    assert!(before.contains_key(std::ffi::OsStr::new("lifecycle.sqlite-wal")));
    assert!(
        SqliteLifecycleStore::open_selected_restart(root.path(), "trusted".into(), "store:wrong")
            .is_err()
    );
    assert_eq!(sqlite_image_bytes(root.path()), before);
    drop(wal_keeper);
    let mut store =
        SqliteLifecycleStore::open_selected_restart(root.path(), "trusted".into(), &store_id)
            .unwrap();
    assert!(
        SqliteLifecycleStore::open_selected_restart(root.path(), "trusted".into(), &store_id)
            .is_err()
    );
    let basis = store.native_restart_basis(&subject, &context).unwrap();
    assert_eq!(basis.store_id, store_id);
    assert_eq!(basis.predecessor_session_id, predecessor.as_str());
    assert_eq!(basis.predecessor_epoch, 0);
    assert_eq!(basis.profile_digest, profile);
    assert_eq!(basis.binary_sha256, binary);
    assert_eq!(basis.protocol_digest, protocol);
    assert_eq!(basis.snapshot_id, snapshot.as_str());
    assert_eq!(basis.snapshot_manifest_sha256, manifest);
    let projection = store.subject_projection(&subject, None, None).unwrap();
    let intent = NativeRestartIntent {
        subject: subject.clone(),
        context: context.clone(),
        predecessor_session: predecessor.clone(),
        predecessor_epoch: 0,
        target_session: target.clone(),
        expected_store_id: projection.store_id,
        expected_subject_revision: projection.revision,
        expected_semantic_revision: projection.semantic_revision,
        expected_queue_revision: 0,
        expected_journal_cursor: projection.journal_cursor,
        intent_sha256: "e".repeat(64),
        selection_sha256: "f".repeat(64),
        profile_digest: profile.clone(),
        binary_sha256: binary.clone(),
        protocol_digest: protocol.clone(),
        snapshot: snapshot.clone(),
        snapshot_manifest_sha256: manifest.clone(),
    };
    assert_eq!(
        store
            .native_restart_epoch(&subject, &context)
            .unwrap()
            .unwrap()
            .epoch,
        0
    );
    for invalid in [
        NativeRestartIntent {
            expected_store_id: "store:wrong".into(),
            ..intent.clone()
        },
        NativeRestartIntent {
            expected_subject_revision: intent.expected_subject_revision + 1,
            ..intent.clone()
        },
        NativeRestartIntent {
            binary_sha256: "0".repeat(64),
            ..intent.clone()
        },
        NativeRestartIntent {
            target_session: predecessor.clone(),
            ..intent.clone()
        },
    ] {
        assert!(store.authorize_native_restart(&invalid).is_err());
    }
    let inspection = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    assert_eq!(
        inspection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        6
    );
    drop(inspection);
    let retained_before = retained_table_rows(&root.path().join("lifecycle.sqlite"));
    assert_eq!(
        store.authorize_native_restart(&intent).unwrap().state,
        "authorized"
    );
    assert_eq!(
        retained_table_rows(&root.path().join("lifecycle.sqlite")),
        retained_before,
        "authorization may add epoch and custody revision, but not rewrite historical rows or revisions"
    );
    assert_eq!(
        store.authorize_native_restart(&intent).unwrap().state,
        "authorized"
    );
    let wrong = NativeRestartIntent {
        selection_sha256: "0".repeat(64),
        ..intent.clone()
    };
    assert!(store.authorize_native_restart(&wrong).is_err());
    assert!(
        store
            .reserve_authorized_native_session(
                &wrong,
                RuntimeIncarnation::parse("incarnation:wrong-selection").unwrap(),
                restart_clock()
            )
            .is_err()
    );
    assert!(
        store
            .reserve_native_session(
                target.clone(),
                subject.clone(),
                context.clone(),
                RuntimeIncarnation::parse("incarnation:wrong").unwrap(),
                &profile,
                &binary,
                &protocol,
                snapshot.clone(),
                &manifest,
                false,
                restart_clock()
            )
            .is_err()
    );
    let selected = store
        .reserve_authorized_native_session(
            &intent,
            RuntimeIncarnation::parse("incarnation:new").unwrap(),
            restart_clock(),
        )
        .unwrap();
    assert_eq!(selected.id(), &target);
    let basis = store.native_restart_basis(&subject, &context).unwrap();
    assert_eq!(basis.predecessor_session_id, target.as_str());
    assert_eq!(basis.predecessor_epoch, 1);
    assert_eq!(
        store
            .native_restart_epoch(&subject, &context)
            .unwrap()
            .unwrap()
            .epoch,
        1
    );
    assert!(
        store
            .reserve_authorized_native_session(
                &intent,
                RuntimeIncarnation::parse("incarnation:later").unwrap(),
                restart_clock()
            )
            .is_err()
    );
    assert_eq!(
        store.authorize_native_restart(&intent).unwrap().state,
        "active"
    );
    // A unique temporary sibling keeps export out of the live store root.
    let capsule_dir = tempdir_in(root.path().parent().unwrap()).unwrap();
    let capsule_root = capsule_dir.path().join("capsule");
    let receipt = store.export_current(&capsule_root).unwrap();
    assert_eq!(receipt.source_schema, 7);
    assert_eq!(
        SqliteLifecycleStore::read_capsule(&capsule_root).unwrap(),
        receipt
    );
    let adopted_root = capsule_dir.path().join("adopted");
    let (adopted, _adoption) = SqliteLifecycleStore::adopt_inactive(
        &receipt,
        &adopted_root,
        &HandoffId::parse("handoff:restart").unwrap(),
        "trusted".into(),
    )
    .unwrap();
    assert_eq!(adopted.admin_ownership().unwrap().status, "inactive");
    assert_eq!(
        adopted
            .native_restart_epoch(&subject, &context)
            .unwrap()
            .unwrap()
            .session_id,
        target.as_str()
    );
    drop(adopted);
    let malformed = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    malformed.execute_batch("DROP TRIGGER custody_revision_native_execution_epochs_insert; CREATE TRIGGER custody_revision_native_execution_epochs_insert AFTER INSERT ON subjects BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;").unwrap();
    let bad_capsule = capsule_dir.path().join("malformed-export");
    assert!(store.export_current(&bad_capsule).is_err());
    assert!(!bad_capsule.join("manifest.json").exists());
    malformed.execute_batch("DROP TRIGGER custody_revision_native_execution_epochs_insert; CREATE TRIGGER custody_revision_native_execution_epochs_insert AFTER INSERT ON native_execution_epochs BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;").unwrap();
    drop(malformed);
    drop(store);
    let reopened =
        SqliteLifecycleStore::open_selected_restart(root.path(), "trusted".into(), &store_id)
            .unwrap();
    assert_eq!(
        reopened
            .native_restart_epoch(&subject, &context)
            .unwrap()
            .unwrap()
            .session_id,
        target.as_str()
    );
    let db = rusqlite::Connection::open(root.path().join("lifecycle.sqlite")).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM native_sessions WHERE subject_id='subject:restart'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        2
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM native_execution_epochs WHERE state='active'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    drop(reopened);
    db.execute_batch("DROP TRIGGER custody_revision_native_execution_epochs_insert;")
        .unwrap();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    db.execute_batch("CREATE TRIGGER custody_revision_native_execution_epochs_insert AFTER INSERT ON subjects BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;").unwrap();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    db.execute_batch("DROP TRIGGER custody_revision_native_execution_epochs_insert; CREATE TRIGGER custody_revision_native_execution_epochs_insert AFTER INSERT ON native_execution_epochs WHEN 0 BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;").unwrap();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    db.execute_batch("DROP TRIGGER custody_revision_native_execution_epochs_insert; CREATE TRIGGER custody_revision_native_execution_epochs_insert AFTER INSERT ON native_execution_epochs BEGIN UPDATE admin_custody_revision SET revision=revision+1 WHERE singleton=1; END;").unwrap();
    db.execute_batch("DROP INDEX one_active_native_execution_epoch; CREATE INDEX one_active_native_execution_epoch ON native_execution_epochs(subject_id,context_id) WHERE state='active';").unwrap();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    db.execute_batch("DROP INDEX one_active_native_execution_epoch; CREATE UNIQUE INDEX one_active_native_execution_epoch ON native_execution_epochs(subject_id,context_id) WHERE state='active';").unwrap();
    db.execute_batch("UPDATE native_execution_epochs SET state='historical' WHERE state='active';")
        .unwrap();
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
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
    assert_legacy_current_capsule_roundtrip(root.path(), &store);
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
fn selected_restart_refuses_legacy_four_before_upgrade() {
    let root = tempdir().unwrap();
    predecessor_database(root.path(), 4, "");
    fs::write(root.path().join("writer.lock"), b"").unwrap();
    let database = root.path().join("lifecycle.sqlite");
    let before = sqlite_image_bytes(root.path());
    assert!(
        SqliteLifecycleStore::open_selected_restart(
            root.path(),
            "trusted".into(),
            "store:failed-upgrade"
        )
        .is_err()
    );
    assert_eq!(sqlite_image_bytes(root.path()), before);
    let inspection = rusqlite::Connection::open_with_flags(
        &database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        inspection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
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
    assert_legacy_current_capsule_roundtrip(root.path(), &store);
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
