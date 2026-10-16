//! The qualified native producer's actual synthetic output is A's input.
//! The sibling module supplies the same schema-12 synthetic fixture; its
//! focused tests also run in this integration binary.
#[path = "legacy_v28.rs"]
mod support;

use lifecycle_admin::legacy_v28::{translate_v28_capture, verify_completed_translation};
use rusqlite::{Connection, ffi};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const BINARY_SHA: &str = "f9b67b1a05b1ab3e975e9c0fe012a1909d29e7f66ca54e77c092c68e6a1eaea9";
const SOURCE_SHA: &str = "969e52ee4254bd9e8a1f6ad2290b77e08d30f59f880da747ba145b3835e7d5e5";
const NAMES: [&str; 4] = [
    "semantic-context.sqlite3",
    "role-bindings.json",
    "generation-state.json",
    "switchboard-attachment.json",
];

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_json(path: &Path, value: &Value) {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap();
}

fn binary() -> PathBuf {
    let path = PathBuf::from(std::env::var("S9_CAPTURE_BIN").expect("frozen native binary path"));
    let pinned = std::env::var("S9_CAPTURE_SHA256").expect("frozen native binary hash");
    assert_eq!(pinned, BINARY_SHA);
    assert_eq!(sha(&fs::read(&path).unwrap()), pinned);
    path
}

fn absent(source: &Path) -> Value {
    for name in [
        "semantic-context.sqlite3-wal",
        "semantic-context.sqlite3-shm",
    ] {
        assert!(fs::symlink_metadata(source.join(name)).is_err());
    }
    json!({
        "semantic-context.sqlite3-wal":{"present":false},
        "semantic-context.sqlite3-shm":{"present":false}
    })
}

fn capture(source: &Path, attempt: &Path) -> (std::process::ExitStatus, Value) {
    capture_with_fault(source, attempt, None)
}

fn capture_with_fault(
    source: &Path,
    attempt: &Path,
    fault: Option<&str>,
) -> (std::process::ExitStatus, Value) {
    let child_root = attempt.join("capture");
    let mut command = Command::new(binary());
    command
        .arg("--source-root")
        .arg(source)
        .arg("--output-root")
        .arg(&child_root)
        .arg("--socket-path")
        .arg(attempt.parent().unwrap().join("absent-synthetic.sock"))
        .arg("--operator")
        .arg("s9-a-synthetic-qualification")
        .arg("--deadline-seconds")
        .arg("120")
        .env_remove("S9_CAPTURE_TEST_BARRIER")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(fault) = fault {
        command.env("S9_CAPTURE_TEST_FAULT", fault);
    } else {
        command.env_remove("S9_CAPTURE_TEST_FAULT");
    }
    let mut child = command.spawn().unwrap();
    let limit = Instant::now() + Duration::from_secs(125);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= limit {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("synthetic native capture exceeded owned child deadline");
        }
        thread::sleep(Duration::from_millis(20));
    };
    let receipt: Value =
        serde_json::from_slice(&fs::read(child_root.join("receipt.json")).unwrap()).unwrap();
    (status, receipt)
}

#[test]
fn actual_native_paired_sidecars_cannot_enter_translator() {
    let f = support::fixture();
    let conn = Connection::open(f.source.join(NAMES[0])).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    let mut persist: i32 = 1;
    let main = std::ffi::CString::new("main").unwrap();
    let code = unsafe {
        ffi::sqlite3_file_control(
            conn.handle(),
            main.as_ptr(),
            ffi::SQLITE_FCNTL_PERSIST_WAL,
            (&mut persist as *mut i32).cast(),
        )
    };
    assert_eq!(code, ffi::SQLITE_OK);
    conn.pragma_update(None, "user_version", 12).unwrap();
    drop(conn);
    assert!(f.source.join("semantic-context.sqlite3-wal").exists());
    assert!(f.source.join("semantic-context.sqlite3-shm").exists());
    fs::remove_dir_all(&f.attempt).unwrap();
    fs::create_dir(&f.attempt).unwrap();
    let (status, child) = capture(&f.source, &f.attempt);
    archive(&f.attempt, "paired-sidecars-native-capture");
    assert!(status.success(), "native child: {child}");
    let error = translate_v28_capture(&f.attempt, &f.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("sidecar"), "{error}");
    assert!(!f.output.exists());
}

#[test]
fn native_primary_and_cleanup_failure_preserves_receipt() {
    let f = support::fixture();
    fs::remove_dir_all(&f.attempt).unwrap();
    fs::create_dir(&f.attempt).unwrap();
    let (status, child) = capture_with_fault(&f.source, &f.attempt, Some("primary_and_cleanup"));
    archive(&f.attempt, "native-primary-cleanup-failure");
    assert!(!status.success());
    assert_eq!(child["status"], "failed_unqualified");
    assert!(
        child["primary_error"]
            .as_str()
            .unwrap()
            .contains("injected primary")
    );
    assert!(
        child["cleanup_error"]
            .as_str()
            .unwrap()
            .contains("injected cleanup")
    );
    assert!(!f.output.exists());
}

#[test]
fn native_deadline_failure_retains_owned_child_receipt() {
    let f = support::fixture();
    fs::remove_dir_all(&f.attempt).unwrap();
    fs::create_dir(&f.attempt).unwrap();
    let mut child = Command::new(binary())
        .arg("--source-root")
        .arg(&f.source)
        .arg("--output-root")
        .arg(f.attempt.join("capture"))
        .arg("--socket-path")
        .arg(f.attempt.join("absent-synthetic.sock"))
        .arg("--operator")
        .arg("s9-a-synthetic-deadline")
        .arg("--deadline-seconds")
        .arg("1")
        .env("S9_CAPTURE_TEST_BARRIER", "after_baseline")
        .env_remove("S9_CAPTURE_TEST_FAULT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    stdout.read_line(&mut ready).unwrap();
    assert_eq!(ready.trim(), "S9_TEST_BARRIER_READY");
    thread::sleep(Duration::from_millis(1_150));
    child.stdin.take().unwrap().write_all(b"c").unwrap();
    let limit = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= limit {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("synthetic deadline child exceeded owned termination bound");
        }
        thread::sleep(Duration::from_millis(20));
    };
    let child_receipt: Value =
        serde_json::from_slice(&fs::read(f.attempt.join("capture/receipt.json")).unwrap()).unwrap();
    archive(&f.attempt, "native-deadline-failure");
    assert!(!status.success());
    assert_eq!(child_receipt["status"], "failed_unqualified");
    assert!(
        child_receipt["primary_error"]
            .as_str()
            .unwrap()
            .contains("deadline")
    );
    assert!(!f.output.exists());
}

fn outer_from_actual(source: &Path, attempt: &Path, child: &Value, pre: Value, post: Value) {
    let child_root = attempt.join("capture");
    let manifest_path = child_root.join("source-manifest.json");
    let manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    let declared = manifest["files"].as_array().unwrap();
    assert_eq!(declared.len(), 4);
    let mut artifacts = BTreeMap::new();
    for name in NAMES {
        let path = child_root.join("image").join(name);
        let digest = sha(&fs::read(&path).unwrap());
        let entry = declared
            .iter()
            .find(|entry| entry["copy"] == json!(path))
            .unwrap();
        assert_eq!(entry["sha256"].as_str(), Some(digest.as_str()));
        assert_eq!(entry["copy_sha256"].as_str(), Some(digest.as_str()));
        assert_eq!(digest, sha(&fs::read(source.join(name)).unwrap()));
        artifacts.insert(format!("capture/image/{name}"), json!({"sha256":digest}));
    }
    let manifest_sha = sha(&fs::read(&manifest_path).unwrap());
    let child_sha = sha(&fs::read(child_root.join("receipt.json")).unwrap());
    let outer = json!({
        "schema_version":1,
        "status":"qualified_candidate_root",
        "raw_exit_status":0,
        "primary_error":null,
        "cleanup_error":null,
        "child_primary_error":child["primary_error"],
        "child_cleanup_error":child["cleanup_error"],
        "child_status":child["status"],
        "child_event_ledger":child["event_ledger"],
        "preflight_source_metadata":pre,
        "postflight_source_metadata":post,
        "binary":{"sha256":BINARY_SHA},
        "source_aggregate_sha256":SOURCE_SHA,
        "child_receipt":{"sha256":child_sha},
        "child_manifest_sha256":manifest_sha,
        "source_hashes_before":child["source_hashes"],
        "source_hashes_after":child["source_hashes"],
        "selection_claim":"current_selection_unestablished",
        "duration_seconds":0.381,
        "argv":[binary(),"--source-root",source,"--output-root",child_root,
            "--socket-path",attempt.parent().unwrap().join("absent-synthetic.sock"),
            "--operator","s9-a-synthetic-qualification","--deadline-seconds","120"],
        "artifacts":artifacts
    });
    write_json(&attempt.join("attempt-receipt.json"), &outer);
}

fn archive(attempt: &Path, label: &str) {
    if let Ok(root) = std::env::var("S9_A_SYNTHETIC_ARCHIVE_DIR") {
        let destination = PathBuf::from(root).join(label);
        fs::create_dir(&destination).unwrap();
        fn copy_tree(from: &Path, to: &Path) {
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                let next = to.join(entry.file_name());
                if path.is_dir() {
                    fs::create_dir(&next).unwrap();
                    copy_tree(&path, &next);
                } else {
                    fs::copy(&path, &next).unwrap();
                }
            }
        }
        copy_tree(attempt, &destination);
    }
}

#[test]
fn actual_native_capture_to_held_capsule() {
    let f = support::fixture();
    let source_before: Vec<_> = NAMES
        .iter()
        .map(|name| sha(&fs::read(f.source.join(name)).unwrap()))
        .collect();
    fs::remove_dir_all(&f.attempt).unwrap();
    fs::create_dir(&f.attempt).unwrap();
    let pre = absent(&f.source);
    let (status, child) = capture(&f.source, &f.attempt);
    let post = absent(&f.source);
    assert!(status.success(), "native child receipt: {child}");
    assert_eq!(child["status"], "qualified_candidate_root");
    assert_eq!(child["event_ledger"], json!([]));
    outer_from_actual(&f.source, &f.attempt, &child, pre, post);
    let translation = translate_v28_capture(&f.attempt, &f.output).unwrap();
    verify_completed_translation(&f.attempt, &f.output, &translation).unwrap();
    assert_eq!(
        translation.capsule_version.as_deref(),
        Some("s9-v28-obligations-v2")
    );
    assert_eq!(
        translation.receipt_sha256,
        sha(&fs::read(f.output.join("receipt.json")).unwrap())
    );
    assert_eq!(translation.translation_status, "qualified");
    assert_eq!(
        translation.transfer_eligibility,
        "requires_separate_B_authority"
    );
    let source_after: Vec<_> = NAMES
        .iter()
        .map(|name| sha(&fs::read(f.source.join(name)).unwrap()))
        .collect();
    assert_eq!(source_before, source_after);
    let capsule: Value =
        serde_json::from_slice(&fs::read(f.output.join("capsule.json")).unwrap()).unwrap();
    assert_eq!(capsule["schema_version"], "s9-v28-obligations-v2");
    assert_eq!(capsule["relation_census"].as_array().unwrap().len(), 28);
    let report: Value =
        serde_json::from_slice(&fs::read(f.output.join("report-redacted.json")).unwrap()).unwrap();
    assert_eq!(report["schema_version"], 2);
    assert_eq!(capsule["q3"]["physical_delivery"], "not_applicable");
    assert_eq!(capsule["q4"]["physical_delivery"], "not_proved");
    assert_eq!(capsule["no_rust_custody"], true);
    archive(&f.attempt, "positive-actual-native-capture");
    archive(&f.output, "positive-translation");
    println!(
        "actual synthetic child_sha256={} capsule_sha256={}",
        sha(&fs::read(f.attempt.join("capture/receipt.json")).unwrap()),
        translation.capsule_sha256
    );
}

#[test]
fn incomplete_source_sidecar_pair_fails_at_native_boundary() {
    let f = support::fixture();
    fs::remove_dir_all(&f.attempt).unwrap();
    fs::create_dir(&f.attempt).unwrap();
    fs::write(
        f.source.join("semantic-context.sqlite3-wal"),
        b"synthetic-incomplete",
    )
    .unwrap();
    let (status, child) = capture(&f.source, &f.attempt);
    archive(&f.attempt, "incomplete-pair-native-rejection");
    assert!(!status.success());
    assert_eq!(child["status"], "failed_unqualified");
    assert!(
        child["primary_error"]
            .as_str()
            .unwrap()
            .contains("pair incomplete")
    );
    assert!(!f.output.exists());
}
