use std::time::Duration;

use review_episode_core::codec::{field, parse_json};
use review_episode_core::state::ReviewEpisodeState;
use review_episode_store::{
    EpisodeStore, StoreError, StoreOptions, WriteDisposition, init_offline_root,
};
use rusqlite::Connection;

const STATES: &str =
    include_str!("../../../../app-server/tests/fixtures/review-episode-rust/states-v1-v2.json");
fn fixture_states() -> (String, Vec<ReviewEpisodeState>) {
    let fixture = parse_json(STATES).unwrap();
    let key = field(&fixture, "identityKey")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let values = field(&fixture, "states").unwrap();
    let states = ["v1_begin", "v2_blocked"]
        .iter()
        .map(|name| {
            let json = field(field(values, name).unwrap(), "canonical")
                .unwrap()
                .as_text()
                .unwrap()
                .to_string_checked()
                .unwrap();
            ReviewEpisodeState::from_stored_json(&json).unwrap()
        })
        .collect();
    (key, states)
}
fn apply(
    store: &mut EpisodeStore,
    key: &str,
    observed: Option<&str>,
    state: ReviewEpisodeState,
) -> Result<String, StoreError> {
    store.write(key, observed, 1024, |_| {
        Ok(WriteDisposition::Applied {
            state: Box::new(state),
            reply_json: "ok".into(),
        })
    })
}
#[test]
fn conditional_revision_conflict_and_sqlite_busy_are_distinct() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    init_offline_root(&root).unwrap();
    let (key, states) = fixture_states();
    let mut store = EpisodeStore::open(
        &root,
        StoreOptions {
            busy_timeout: Duration::from_millis(10),
        },
    )
    .unwrap();
    let holder = Connection::open(root.join("review-episodes.sqlite")).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(matches!(
        apply(&mut store, &key, None, states[0].clone()),
        Err(StoreError::Busy)
    ));
    holder.execute_batch("ROLLBACK").unwrap();
    drop(holder);
    assert_eq!(
        apply(&mut store, &key, None, states[0].clone()).unwrap(),
        "ok"
    );
    assert!(matches!(
        apply(&mut store, &key, None, states[0].clone()),
        Err(StoreError::RevisionConflict)
    ));
    assert_eq!(store.read(&key).unwrap().history.len(), 1);
}
#[test]
fn statement_failure_after_history_insert_rolls_back_both_rows() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("offline");
    init_offline_root(&root).unwrap();
    let (key, states) = fixture_states();
    let mut store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    apply(&mut store, &key, None, states[0].clone()).unwrap();
    let db = Connection::open(root.join("review-episodes.sqlite")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_current BEFORE UPDATE ON review_episode_current BEGIN SELECT RAISE(FAIL,'injected statement failure'); END").unwrap();
    let predecessor = states[0].revision().0.clone();
    assert!(matches!(
        apply(&mut store, &key, Some(&predecessor), states[1].clone()),
        Err(StoreError::Io(_))
    ));
    db.execute_batch("DROP TRIGGER fail_current").unwrap();
    drop(db);
    let snapshot = store.read(&key).unwrap();
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(snapshot.current.unwrap().revision().0, predecessor);
    drop(store);
    let mut reopened = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    assert_eq!(reopened.validate_all().unwrap(), (1, 1));
}
