#![cfg(all(feature = "test-support", feature = "test-faults"))]
mod claude_common;

use std::fs;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use claude_common::{counts, fixture};
use review_execution_evidence::claude::{
    InitialEntryPermit, QualifiedReader, QualifiedWriter, SessionEvolutionPermit,
};
use review_execution_evidence::{
    ExecutionEvidenceOwner, ExecutionEvidenceRef, ExecutionPublication,
};

#[tokio::test]
async fn claude_fault_child() {
    let Ok(parent) = std::env::var("WORK_ENGINE_CLAUDE_FAULT_CHILD") else {
        return;
    };
    let mode = std::env::var("WORK_ENGINE_CLAUDE_FAULT_MODE").unwrap_or_else(|_| "success".into());
    let horizon = if mode == "timeout" { 10 } else { 2 };
    let f = fixture(
        std::path::Path::new(&parent),
        &mode,
        Duration::from_secs(horizon),
    );
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope).unwrap();
    let _ = writer.execute_initial(permit).await;
}

#[test]
fn session_evolution_sync_fault_child() {
    let Ok(parent) = std::env::var("WORK_ENGINE_CLAUDE_EVOLUTION_CHILD") else {
        return;
    };
    let parent = std::path::Path::new(&parent);
    let f = fixture(parent, "success", Duration::from_secs(2));
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(parent.join("reference.json")).unwrap()).unwrap();
    let field = |name: &str| value[name].as_str().unwrap().to_owned();
    let reference = ExecutionEvidenceRef {
        schema_version: 1,
        owner: field("owner"),
        root_id: field("root_id"),
        profile: field("profile"),
        attempt_id: field("attempt_id"),
        record_id: field("record_id"),
        revision: field("revision"),
        sha256: field("sha256"),
    };
    let reader = QualifiedReader::open(&f.root, f.scope.reader_selection().unwrap()).unwrap();
    let retained = reader.retained_session(&reference).unwrap();
    let live_nested = f.root.join("session-live/history/turn");
    fs::create_dir_all(&live_nested).unwrap();
    fs::write(live_nested.join("state.json"), b"{\"turn\":2}").unwrap();
    let permit = SessionEvolutionPermit::offline_peer(&f.scope, &retained, "attempt-2").unwrap();
    let mut writer = QualifiedWriter::resume(&f.root, f.scope).unwrap();
    assert!(writer.record_session_evolution(retained, permit).is_err());
}

#[test]
fn exact_recovery_cuts_never_reenter_peer() {
    for cut in [
        "prepared",
        "session_ready",
        "launch_intent",
        "spawn_outcome",
        "capture_complete",
        "terminal",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let f = fixture(temp.path(), "success", Duration::from_secs(2));
        let ready = temp.path().join("cut-ready");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "claude_fault_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("WORK_ENGINE_CLAUDE_FAULT_CHILD", temp.path())
            .env("WORK_ENGINE_CLAUDE_FAULT_CUT", cut)
            .env("WORK_ENGINE_CLAUDE_FAULT_READY", &ready)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() && Instant::now() < deadline {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(ready.exists(), "fault cut not reached: {cut}");
        child.kill().unwrap();
        child.wait().unwrap();
        let selection = f.scope.reader_selection().unwrap();
        let mut reopened = QualifiedWriter::resume(&f.root, f.scope).unwrap();
        let first = reopened.reconcile().unwrap();
        let second = reopened.reconcile().unwrap();
        match cut {
            "capture_complete" => {
                let reference = match first {
                    ExecutionPublication::Committed(reference) => reference,
                    other => panic!("unexpected recovery at {cut}: {other:?}"),
                };
                assert_eq!(second, ExecutionPublication::Replayed(reference.clone()));
                drop(reopened);
                let reader = QualifiedReader::open(&f.root, selection).unwrap();
                reader.read_result(&reference).unwrap();
            }
            "terminal" => {
                let reference = match first {
                    ExecutionPublication::Replayed(reference) => reference,
                    other => panic!("unexpected recovery at {cut}: {other:?}"),
                };
                assert_eq!(second, ExecutionPublication::Replayed(reference));
            }
            _ => {
                assert_eq!(first, ExecutionPublication::Unresolved, "{cut}");
                assert_eq!(second, ExecutionPublication::Unresolved, "{cut}");
            }
        }
        let (starts, prompts) = counts(&f.witness);
        assert!(
            starts <= 1 && prompts <= 1,
            "{cut}: {starts} starts, {prompts} prompts"
        );
        if matches!(cut, "prepared" | "session_ready" | "launch_intent") {
            assert_eq!(starts, 0, "{cut}");
        }
        if cut == "spawn_outcome" {
            assert_eq!(prompts, 0, "{cut}");
        }
        if matches!(cut, "capture_complete" | "terminal") {
            assert_eq!((starts, prompts), (1, 1), "{cut}");
        }
        let _ = fs::read_to_string(&ready).unwrap();
    }
}

#[test]
fn interruption_after_prompt_keeps_original_attempt_unresolved() {
    let temp = tempfile::tempdir().unwrap();
    let f = fixture(temp.path(), "timeout", Duration::from_secs(10));
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "claude_fault_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("WORK_ENGINE_CLAUDE_FAULT_CHILD", temp.path())
        .env("WORK_ENGINE_CLAUDE_FAULT_MODE", "timeout")
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while counts(&f.witness).1 == 0 && Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(counts(&f.witness), (1, 1));
    child.kill().unwrap();
    child.wait().unwrap();
    let mut reopened = QualifiedWriter::resume(&f.root, f.scope).unwrap();
    assert_eq!(
        reopened.reconcile().unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(
        reopened.reconcile().unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(counts(&f.witness), (1, 1));
}

#[test]
fn every_snapshot_namespace_sync_failure_precedes_complete_capture() {
    for stage in [
        "live_nested",
        "live_directory_before_capture",
        "snapshot_nested",
        "snapshot_directory",
        "snapshots_parent",
        "snapshot_owner_root",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let f = fixture(temp.path(), "nested", Duration::from_secs(2));
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "claude_fault_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("WORK_ENGINE_CLAUDE_FAULT_CHILD", temp.path())
            .env("WORK_ENGINE_CLAUDE_FAULT_MODE", "nested")
            .env("WORK_ENGINE_CLAUDE_SYNC_FAIL", stage)
            .status()
            .unwrap();
        assert!(status.success(), "fault child at {stage}");
        let mut reopened = QualifiedWriter::resume(&f.root, f.scope).unwrap();
        assert_eq!(
            reopened.reconcile().unwrap(),
            ExecutionPublication::Unresolved,
            "{stage}"
        );
        assert_eq!(
            reopened.reconcile().unwrap(),
            ExecutionPublication::Unresolved,
            "{stage}"
        );
        let db = rusqlite::Connection::open(f.root.join("evidence.sqlite")).unwrap();
        let complete: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM events WHERE stage='capture_complete'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(complete, 0, "{stage}");
        assert_eq!(counts(&f.witness), (1, 1), "{stage}");
    }
}

#[test]
fn nested_namespace_syncs_bottom_up_before_complete_capture() {
    let temp = tempfile::tempdir().unwrap();
    let f = fixture(temp.path(), "nested", Duration::from_secs(2));
    let trace = temp.path().join("sync-trace");
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "claude_fault_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("WORK_ENGINE_CLAUDE_FAULT_CHILD", temp.path())
        .env("WORK_ENGINE_CLAUDE_FAULT_MODE", "nested")
        .env("WORK_ENGINE_CLAUDE_SYNC_TRACE", &trace)
        .status()
        .unwrap();
    assert!(status.success());
    let trace = fs::read_to_string(trace).unwrap();
    let stages: Vec<_> = trace.lines().collect();
    let index = |name: &str| stages.iter().rposition(|stage| *stage == name).unwrap();
    assert_eq!(
        stages
            .iter()
            .filter(|stage| **stage == "snapshot_nested")
            .count(),
        2
    );
    assert_eq!(
        stages
            .iter()
            .filter(|stage| **stage == "live_nested")
            .count(),
        2
    );
    assert!(index("snapshot_nested") < index("snapshot_directory"));
    assert!(index("live_nested") < index("live_directory_before_capture"));
    assert!(index("live_directory_before_capture") < index("snapshot_directory"));
    assert!(index("snapshot_directory") < index("snapshots_parent"));
    assert!(index("snapshots_parent") < index("snapshot_owner_root"));
    assert_eq!(stages.last(), Some(&"snapshot_owner_root"));
    let db = rusqlite::Connection::open(f.root.join("evidence.sqlite")).unwrap();
    let complete: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM events WHERE stage='capture_complete'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(complete, 1);
    assert_eq!(counts(&f.witness), (1, 1));
}

#[tokio::test]
async fn successor_sync_failure_keeps_predecessor_and_does_not_publish_evolution() {
    let temp = tempfile::tempdir().unwrap();
    let f = fixture(temp.path(), "success", Duration::from_secs(2));
    let permit = InitialEntryPermit::offline_peer(&f.scope);
    let mut writer = QualifiedWriter::create(&f.root, f.scope.clone()).unwrap();
    let reference = match writer.execute_initial(permit).await.unwrap() {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("unexpected initial: {other:?}"),
    };
    drop(writer);
    fs::write(
        temp.path().join("reference.json"),
        serde_json::json!({
            "owner":reference.owner,"root_id":reference.root_id,"profile":reference.profile,
            "attempt_id":reference.attempt_id,"record_id":reference.record_id,
            "revision":reference.revision,"sha256":reference.sha256,
        })
        .to_string(),
    )
    .unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "session_evolution_sync_fault_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("WORK_ENGINE_CLAUDE_EVOLUTION_CHILD", temp.path())
        .env("WORK_ENGINE_CLAUDE_SYNC_FAIL", "snapshots_parent")
        .status()
        .unwrap();
    assert!(status.success());
    let reader = QualifiedReader::open(&f.root, f.scope.reader_selection().unwrap()).unwrap();
    reader.read_result(&reference).unwrap();
    assert!(
        reader
            .retained_session_after(&reference, "attempt-2")
            .is_err()
    );
    let db = rusqlite::Connection::open(f.root.join("evidence.sqlite")).unwrap();
    let evolved: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM events WHERE stage='session_evolution'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(evolved, 0);
    assert_eq!(counts(&f.witness), (1, 1));
}
