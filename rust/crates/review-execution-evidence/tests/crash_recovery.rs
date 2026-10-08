#![cfg(feature = "test-faults")]

#[allow(dead_code)]
mod common;

use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{invocation_count, scope};
use review_execution_evidence::{
    CleanupObservation, EvidenceReader, EvidenceWriter, ExecutionEvidenceOwner,
    ExecutionPublication, FailureKind,
};

#[test]
fn cut_helper() {
    if std::env::var("E1_CUT_HELPER").ok().as_deref() != Some("1") {
        return;
    }
    let root = std::env::var("E1_CUT_ROOT").unwrap();
    let count = std::env::var("E1_CUT_COUNT").unwrap();
    let selected = scope("root-1", std::path::Path::new(&count), "echo");
    let mut writer = EvidenceWriter::create(std::path::Path::new(&root), selected).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _ = runtime.block_on(writer.execute());
}

#[test]
fn cleanup_helper() {
    if std::env::var("E1_CLEANUP_HELPER").ok().as_deref() != Some("1") {
        return;
    }
    let root = std::env::var("E1_CUT_ROOT").unwrap();
    let count = std::env::var("E1_CUT_COUNT").unwrap();
    let selected = scope("root-1", std::path::Path::new(&count), "sleep");
    let mut writer = EvidenceWriter::create(std::path::Path::new(&root), selected).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    assert_eq!(
        runtime.block_on(writer.execute()).unwrap(),
        ExecutionPublication::Unresolved
    );
}

#[test]
fn bounded_unavailable_cleanup_fact_survives_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let count = temp.path().join("count");
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("cleanup_helper")
        .arg("--nocapture")
        .env("E1_CLEANUP_HELPER", "1")
        .env("E1_CUT_ROOT", &root)
        .env("E1_CUT_COUNT", &count)
        .env("WORK_ENGINE_EVIDENCE_FORCE_CLEANUP_UNAVAILABLE", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "cleanup helper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let selected = scope("root-1", &count, "sleep");
    let reader = EvidenceReader::open(&root, selected.reader_pin().unwrap()).unwrap();
    let failure = reader.read_local_failure("attempt-1").unwrap().unwrap();
    assert_eq!(failure.kind, FailureKind::Timeout);
    assert_eq!(failure.cleanup, CleanupObservation::Unavailable);
    assert_eq!(failure.exit_code, None);
    assert_eq!(invocation_count(&count), 1);
    let mut writer = EvidenceWriter::resume(&root, selected).unwrap();
    assert_eq!(
        writer.reconcile().unwrap(),
        ExecutionPublication::Unresolved
    );
    let runtime = tokio::runtime::Runtime::new().unwrap();
    assert_eq!(
        runtime.block_on(writer.execute()).unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(invocation_count(&count), 1);
}

#[test]
fn actual_process_cut_recovery_never_reinvokes() {
    for (cut, committed) in [
        ("prepared", false),
        ("launch_intent", false),
        ("spawn_outcome", false),
        ("result_captured", false),
        ("terminal", true),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let count = temp.path().join("invocations");
        let ready = temp.path().join("cut.ready");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("cut_helper")
            .arg("--nocapture")
            .env("E1_CUT_HELPER", "1")
            .env("E1_CUT_ROOT", &root)
            .env("E1_CUT_COUNT", &count)
            .env("WORK_ENGINE_EVIDENCE_FAULT_CUT", cut)
            .env("WORK_ENGINE_EVIDENCE_FAULT_READY", &ready)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !ready.exists() && Instant::now() < deadline {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("cut helper exited before {cut}: {status}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(ready.exists(), "cut {cut} was not reached");
        child.kill().unwrap();
        child.wait().unwrap();
        // At spawn-outcome the independently spawned peer may observe EOF after
        // its parent dies. Wait for that local effect before comparing recovery.
        std::thread::sleep(Duration::from_millis(200));
        let before = invocation_count(&count);
        let selected = scope("root-1", &count, "echo");
        let pin = selected.reader_pin().unwrap();
        let mut writer = EvidenceWriter::resume(&root, selected).unwrap();
        let result = writer.reconcile().unwrap();
        assert_eq!(
            matches!(result, ExecutionPublication::Replayed(_)),
            committed,
            "cut {cut}"
        );
        let runtime = tokio::runtime::Runtime::new().unwrap();
        assert_eq!(
            runtime.block_on(writer.execute()).unwrap(),
            result,
            "cut {cut}"
        );
        drop(writer);
        if let ExecutionPublication::Replayed(reference) = result {
            let reader = EvidenceReader::open(&root, pin).unwrap();
            reader.read_result(&reference).unwrap();
        }
        assert_eq!(
            invocation_count(&count),
            before,
            "recovery reinvoked at {cut}"
        );
        let stage = fs::read_to_string(&ready).unwrap();
        assert_eq!(stage, cut);
    }
}
