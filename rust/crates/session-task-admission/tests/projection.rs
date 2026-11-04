use serde_json::Value;
use session_task_admission::projection::{TaskProjection, snapshot};
#[test]
fn snapshot_preserves_exact_request_and_fences_cross_task_results() {
    let fixture: Value = serde_json::from_str(include_str!("projection-fixture.json")).unwrap();
    let mut tasks: Vec<TaskProjection> = serde_json::from_value(fixture["tasks"].clone()).unwrap();
    let raw = tasks[0].request_json.clone();
    let published: Value = serde_json::from_str(
        &snapshot("2026-11-02T00:02:00Z", "fixture-snapshot-r1", &tasks).unwrap(),
    )
    .unwrap();
    assert_eq!(
        published["tasks"][0]["request_json"].as_str(),
        Some(raw.as_str())
    );
    tasks[0].result.as_mut().unwrap()["task"]["revision"] = "other".into();
    assert!(snapshot("2026-11-02T00:02:00Z", "r1", &tasks).is_err());
}

#[test]
fn unknown_intent_is_still_visible_without_becoming_authority() {
    use sha2::{Digest, Sha256};
    let fixture: Value = serde_json::from_str(include_str!("projection-fixture.json")).unwrap();
    let mut tasks: Vec<TaskProjection> = serde_json::from_value(fixture["tasks"].clone()).unwrap();
    let mut request: Value = serde_json::from_str(&tasks[0].request_json).unwrap();
    request["data"]["intent"]["schema"] = "dot.future/v2".into();
    tasks[0].request_json = request.to_string();
    tasks[0].task.sha256 = format!("{:x}", Sha256::digest(tasks[0].request_json.as_bytes()));
    tasks[0].admission = None;
    tasks[0].result = None;
    assert!(snapshot("2026-11-02T00:02:00Z", "r1", &tasks).is_ok());
}

#[cfg(unix)]
#[test]
fn publishes_snapshot_and_refuses_symlink_target() {
    use session_task_admission::projection::publish;
    use std::os::unix::fs::symlink;
    let fixture: Value = serde_json::from_str(include_str!("projection-fixture.json")).unwrap();
    let tasks: Vec<TaskProjection> = serde_json::from_value(fixture["tasks"].clone()).unwrap();
    let dir = std::env::temp_dir().join(format!("task-projection-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir(&dir).unwrap();
    let dest = dir.join("snapshot.json");
    publish(&dest, "2026-11-02T00:02:00Z", "r1", &tasks).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&dest).unwrap()).unwrap()["schema"],
        "dot.task-projection/v1-proposal"
    );
    let link = dir.join("link");
    symlink(&dest, &link).unwrap();
    assert!(publish(&link, "2026-11-02T00:02:00Z", "r1", &tasks).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}
