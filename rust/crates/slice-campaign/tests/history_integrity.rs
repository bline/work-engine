use rusqlite::{Connection, params};
use slice_campaign::{Campaign, Preparation};

mod support;

#[test]
fn current_read_rechecks_exact_commit_receipt_between_calls_and_after_reopen() {
    let (temp, app, identity, _, reopen) = support::selected_campaign(1, true);
    let root = temp.path().join("campaign");
    let path = root.join("slice-campaign.sqlite");
    let before = app.read(&identity).unwrap().unwrap();
    let sql = Connection::open(&path).unwrap();
    let original: String = sql
        .query_row(
            "SELECT kind FROM operation_receipt WHERE operation_id='selection-many'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    sql.execute(
        "UPDATE operation_receipt SET kind='admit' WHERE operation_id='selection-many'",
        [],
    )
    .unwrap();
    assert!(app.read(&identity).is_err());
    drop(app);
    let reopened = Campaign::open(reopen()).unwrap();
    assert!(reopened.read(&identity).is_err());
    sql.execute(
        "UPDATE operation_receipt SET kind=?1 WHERE operation_id='selection-many'",
        [original],
    )
    .unwrap();
    assert_eq!(
        reopened.read(&identity).unwrap().unwrap().revision,
        before.revision
    );
}

#[test]
fn historical_replay_allows_later_slot_change_while_current_read_rejects_it() {
    let (temp, mut app, identity, selected, _) = support::selected_campaign(2, true);
    let prepared = app
        .prepare_initial(&identity, &selected.revision, "review-0", "prepare-many-0")
        .unwrap();
    assert!(matches!(prepared, Preparation::Applied(_)));
    let before = app.read(&identity).unwrap().unwrap();
    let sql = Connection::open(temp.path().join("campaign/slice-campaign.sqlite")).unwrap();
    sql.execute(
        "UPDATE request_slot SET active=0 WHERE operation_id=?1",
        params!["prepare-many-0"],
    )
    .unwrap();
    assert!(app.read(&identity).is_err());
    assert!(matches!(
        app.prepare_initial(&identity, &selected.revision, "review-0", "prepare-many-0")
            .unwrap(),
        Preparation::Replayed { .. }
    ));
    sql.execute(
        "UPDATE request_slot SET active=1 WHERE operation_id=?1",
        params!["prepare-many-0"],
    )
    .unwrap();
    assert_eq!(
        app.read(&identity).unwrap().unwrap().revision,
        before.revision
    );
}

#[test]
fn current_read_rejects_deleted_prior_revision_receipt() {
    let (temp, mut app, identity, selected, _) = support::selected_campaign(1, true);
    assert!(matches!(
        app.prepare_initial(&identity, &selected.revision, "review-0", "prepare-many-0")
            .unwrap(),
        Preparation::Applied(_)
    ));
    assert!(app.read(&identity).is_ok());
    let sql = Connection::open(temp.path().join("campaign/slice-campaign.sqlite")).unwrap();
    sql.execute(
        "DELETE FROM operation_receipt WHERE operation_id='selection-many'",
        [],
    )
    .unwrap();
    assert!(app.read(&identity).is_err());
}
