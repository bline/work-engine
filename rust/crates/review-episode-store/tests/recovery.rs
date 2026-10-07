use std::path::Path;

use review_episode_core::codec::{JsString, JsValue, canonical_json, field, parse_json};
use review_episode_core::state::ReviewEpisodeState;
use review_episode_store::{EpisodeStore, StoreError, StoreOptions, init_offline_root};
use rusqlite::{Connection, params};

const STATES: &str =
    include_str!("../../../../app-server/tests/fixtures/review-episode-rust/states-v1-v2.json");

fn fixture() -> (String, Vec<(String, String)>) {
    let value = parse_json(STATES).unwrap();
    let key = field(&value, "identityKey")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let states = field(&value, "states").unwrap();
    let rows = ["v1_begin", "v2_blocked", "v2_succeeded"]
        .iter()
        .map(|name| {
            let record = field(states, name).unwrap();
            (
                field(record, "revision")
                    .unwrap()
                    .as_text()
                    .unwrap()
                    .to_string_checked()
                    .unwrap(),
                field(record, "canonical")
                    .unwrap()
                    .as_text()
                    .unwrap()
                    .to_string_checked()
                    .unwrap(),
            )
        })
        .collect();
    (key, rows)
}
fn populate(root: &Path, sequences: [i64; 3]) -> String {
    init_offline_root(root).unwrap();
    let db = root.join("review-episodes.sqlite");
    let conn = Connection::open(db).unwrap();
    let (key, rows) = fixture();
    let mut predecessor = None;
    for ((revision, json), sequence) in rows.iter().zip(sequences) {
        conn.execute("INSERT INTO review_episode_history(sequence,identity_key,revision,predecessor_revision,state_json) VALUES(?1,?2,?3,?4,?5)",params![sequence,key,revision,predecessor,json]).unwrap();
        predecessor = Some(revision.as_str());
    }
    conn.execute(
        "INSERT INTO review_episode_current(identity_key,revision,state_json) VALUES(?1,?2,?3)",
        params![key, rows[2].0, rows[2].1],
    )
    .unwrap();
    key
}
fn expect_integrity<F: FnOnce(&Connection)>(mutate: F) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let key = populate(&root, [1, 2, 3]);
    let conn = Connection::open(root.join("review-episodes.sqlite")).unwrap();
    mutate(&conn);
    drop(conn);
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    assert!(matches!(store.read(&key), Err(StoreError::Integrity(_))));
}

#[test]
fn valid_global_sequence_gaps_preserve_exact_history() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let key = populate(&root, [2, 5, 9]);
    let (_, fixture_rows) = fixture();
    let original = ReviewEpisodeState::from_stored_json(&fixture_rows[0].1).unwrap();
    let mut second_semantic = original.semantic();
    if let JsValue::Object(map) = &mut second_semantic
        && let JsValue::Object(identity) = map.get_mut(&JsString::new("identity")).unwrap()
    {
        identity.insert(JsString::new("runId"), JsValue::text("interleaved-run"));
    }
    let second = ReviewEpisodeState::publish(second_semantic).unwrap();
    let second_key = second.identity().key().0;
    let conn = Connection::open(root.join("review-episodes.sqlite")).unwrap();
    conn.execute("INSERT INTO review_episode_history(sequence,identity_key,revision,predecessor_revision,state_json) VALUES(4,?1,?2,NULL,?3)",params![second_key,second.revision().0,canonical_json(second.value())]).unwrap();
    conn.execute(
        "INSERT INTO review_episode_current(identity_key,revision,state_json) VALUES(?1,?2,?3)",
        params![
            second_key,
            second.revision().0,
            canonical_json(second.value())
        ],
    )
    .unwrap();
    drop(conn);
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let snapshot = store.read(&key).unwrap();
    assert_eq!(
        snapshot
            .history
            .iter()
            .map(|h| h.sequence)
            .collect::<Vec<_>>(),
        vec![2, 5, 9]
    );
    assert_eq!(store.validate_all().unwrap(), (2, 4));
}
#[test]
fn wrong_identity_revision_predecessor_or_current_is_rejected() {
    expect_integrity(|db| {
        db.execute(
            "UPDATE review_episode_history SET identity_key='wrong' WHERE sequence=1",
            [],
        )
        .unwrap();
    });
    expect_integrity(|db| {
        db.execute(
            "UPDATE review_episode_history SET revision=?1 WHERE sequence=2",
            ["0".repeat(64)],
        )
        .unwrap();
    });
    expect_integrity(|db| {
        db.execute(
            "UPDATE review_episode_history SET predecessor_revision=?1 WHERE sequence=3",
            ["0".repeat(64)],
        )
        .unwrap();
    });
    expect_integrity(|db| {
        db.execute("UPDATE review_episode_current SET state_json='{}'", [])
            .unwrap();
    });
    expect_integrity(|db| {
        db.execute("DELETE FROM review_episode_current", [])
            .unwrap();
    });
}
#[test]
fn invalid_json_and_changed_retained_transition_digest_are_rejected() {
    expect_integrity(|db| {
        db.execute(
            "UPDATE review_episode_history SET state_json=?1 WHERE sequence=2",
            ["{bad"],
        )
        .unwrap();
    });
    expect_integrity(|db| {
        let (_, rows) = fixture();
        let state = ReviewEpisodeState::from_stored_json(&rows[1].1).unwrap();
        let mut semantic = state.semantic();
        if let JsValue::Object(map) = &mut semantic
            && let JsValue::Object(handled) =
                map.get_mut(&JsString::new("handledTransitions")).unwrap()
        {
            handled.insert(JsString::new("begin"), JsValue::text(&"f".repeat(64)));
        }
        let changed = ReviewEpisodeState::publish(semantic).unwrap();
        let json = canonical_json(changed.value());
        db.execute("DELETE FROM review_episode_history WHERE sequence=3", [])
            .unwrap();
        db.execute(
            "UPDATE review_episode_history SET revision=?1,state_json=?2 WHERE sequence=2",
            params![changed.revision().0, json],
        )
        .unwrap();
        db.execute(
            "UPDATE review_episode_current SET revision=?1,state_json=?2",
            params![changed.revision().0, json],
        )
        .unwrap();
    });
}
#[test]
fn rejection_identifies_first_failing_table_identity_and_sequence() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    let key = populate(&root, [1, 2, 3]);
    let conn = Connection::open(root.join("review-episodes.sqlite")).unwrap();
    conn.execute(
        "UPDATE review_episode_history SET state_json='{bad' WHERE sequence=2",
        [],
    )
    .unwrap();
    drop(conn);
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let Err(StoreError::Integrity(message)) = store.read(&key) else {
        panic!("expected located history integrity rejection");
    };
    assert!(message.contains("review_episode_history"), "{message}");
    assert!(
        message.contains(&format!("identity_key={key:?}")),
        "{message}"
    );
    assert!(message.contains("sequence=2"), "{message}");

    let conn = Connection::open(root.join("review-episodes.sqlite")).unwrap();
    let (_, rows) = fixture();
    conn.execute(
        "UPDATE review_episode_history SET state_json=?1 WHERE sequence=2",
        [&rows[1].1],
    )
    .unwrap();
    conn.execute("UPDATE review_episode_current SET state_json='{}'", [])
        .unwrap();
    drop(conn);
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let Err(StoreError::Integrity(message)) = store.read(&key) else {
        panic!("expected located current integrity rejection");
    };
    assert!(message.contains("review_episode_current"), "{message}");
    assert!(
        message.contains(&format!("identity_key={key:?}")),
        "{message}"
    );
}
#[test]
fn unknown_schema_and_missing_table_fail_without_repair() {
    for sql in [
        "CREATE TRIGGER illicit AFTER INSERT ON review_episode_current BEGIN DELETE FROM review_episode_history; END",
        "DROP TABLE review_episode_history",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("offline");
        init_offline_root(&root).unwrap();
        let conn = Connection::open(root.join("review-episodes.sqlite")).unwrap();
        conn.execute_batch(sql).unwrap();
        drop(conn);
        assert!(matches!(
            EpisodeStore::open(&root, StoreOptions::default()),
            Err(StoreError::Schema(_))
        ));
    }
}

#[cfg(unix)]
#[test]
fn private_profile_rejects_symlink_hardlink_and_public_modes() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("real");
    init_offline_root(&root).unwrap();
    let link = temp.path().join("linked-root");
    symlink(&root, &link).unwrap();
    assert!(matches!(
        EpisodeStore::open(&link, StoreOptions::default()),
        Err(StoreError::Path(_))
    ));
    let db = root.join("review-episodes.sqlite");
    let hardlink = temp.path().join("db-hardlink");
    std::fs::hard_link(&db, &hardlink).unwrap();
    assert!(matches!(
        EpisodeStore::open(&root, StoreOptions::default()),
        Err(StoreError::Path(_))
    ));
    std::fs::remove_file(&hardlink).unwrap();
    std::fs::set_permissions(&db, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        EpisodeStore::open(&root, StoreOptions::default()),
        Err(StoreError::Path(_))
    ));
    std::fs::set_permissions(&db, std::fs::Permissions::from_mode(0o600)).unwrap();
    let wal = root.join("review-episodes.sqlite-wal");
    symlink(&db, &wal).unwrap();
    assert!(matches!(
        EpisodeStore::open(&root, StoreOptions::default()),
        Err(StoreError::Path(_))
    ));
}
