//! Exact retained-S5 startup proof. Run with LIFECYCLE_S5_BINARY set to a
//! binary built from published S5 commit b4c6df9b9eed05e7ff833290e2e990b4957e59ed.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use lifecycle_admin::selection::{
    FileLaunchSelectionPort, LaunchSelectionGuard, LaunchSelectionPort, SelectionRecord,
};
use lifecycle_admin::startup::verify_selected_launch;
use lifecycle_admin::transfer::transfer_current;
use lifecycle_core::{
    BuildId, ClockSample, ContextGeneration, HandoffId, NativeSessionId, ProofRunId,
    RuntimeIncarnation, SnapshotId, SubjectId, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::SqliteLifecycleStore;
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn all_bytes(root: &Path) -> BTreeMap<PathBuf, String> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                pending.push(path);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    format!("{:x}", Sha256::digest(fs::read(path).unwrap())),
                );
            }
        }
    }
    files
}

#[test]
#[ignore = "requires LIFECYCLE_S5_BINARY built from the published S5 commit"]
fn published_s5_binary_refuses_v4_without_mutating_any_store_byte() {
    let binary = PathBuf::from(std::env::var_os("LIFECYCLE_S5_BINARY").unwrap());
    let binary_hash = format!("{:x}", Sha256::digest(fs::read(&binary).unwrap()));
    assert_eq!(
        binary_hash, "e6bac55cbe68e300ca404c7502007bcefb61e27a3773103791702c94867cce95",
        "retained executable differs from the published-S5 archived build"
    );
    let temporary = tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        fs::metadata(temporary.path()).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let source_root = temporary.path().join("source");
    let capsule_root = temporary.path().join("v4-capsule");
    let socket_path = temporary.path().join("old-service.sock");
    let peer_path = temporary.path().join("no-peer.sock");
    let config_path = temporary.path().join("old-service.json");
    let mut store = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let subject = SubjectId::parse("subject-old-s5").unwrap();
    store
        .register_subject(
            subject.clone(),
            ContextGeneration::parse("context-old-s5").unwrap(),
            BuildId::parse("build-old-s5").unwrap(),
            ProofRunId::parse("proof-old-s5").unwrap(),
            ClockSample {
                wall: WallTimeMs::new(100),
                wait_budget: WaitBudgetMs::new(100),
            },
        )
        .unwrap();
    let staged = store
        .stage_artifact(subject, b"retained S5 refusal marker")
        .unwrap();
    store
        .publish_artifact(
            staged,
            ClockSample {
                wall: WallTimeMs::new(101),
                wait_budget: WaitBudgetMs::new(100),
            },
        )
        .unwrap();
    let receipt = store.export_current(&capsule_root).unwrap();
    assert_eq!(receipt.source_schema, 4);
    drop(store);
    // Adopted store roots carry this empty lock; the export capsule itself does not.
    fs::write(capsule_root.join("writer.lock"), b"").unwrap();

    let config = json!({
        "profile": "read_only",
        "store_root": capsule_root,
        "socket_path": socket_path,
        "peer_socket_path": peer_path,
        "verifier_executable": null,
        "verifier_ledger_path": null,
        "proof_barrier_dir": null,
        "trusted_issuer": "test-issuer",
        "principal": {
            "uid": fs::metadata(temporary.path()).unwrap().uid(),
            "principal_ref": "operator:controlled",
            "producer_ref": "fixture:old-s5"
        },
        "subject_id": "subject-old-s5",
        "context_generation": "context-old-s5",
        "build_id": "build-old-s5",
        "proof_run_id": "proof-old-s5",
        "grants": [],
        "max_executions": 1,
        "max_wait_ms": 100
    });
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let before = all_bytes(&capsule_root);
    let output = Command::new(&binary).arg(&config_path).output().unwrap();
    let after = all_bytes(&capsule_root);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("lifecycle store failed"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(before, after, "S5 startup changed the v4 store directory");
    assert!(!temporary.path().join("old-service.sock").exists());
    assert_eq!(
        SqliteLifecycleStore::read_capsule(&capsule_root).unwrap(),
        receipt
    );
    eprintln!("retained S5 executable SHA256: {binary_hash}");
}

#[test]
#[ignore = "requires LIFECYCLE_CURRENT_SERVICE_BINARY from the current candidate"]
fn current_service_reopens_after_socket_ready_process_kill_without_new_custody() {
    let binary = PathBuf::from(std::env::var_os("LIFECYCLE_CURRENT_SERVICE_BINARY").unwrap());
    let temporary = tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = temporary.path().join("store");
    let socket = temporary.path().join("service.sock");
    let config_path = temporary.path().join("service.json");
    let mut store = SqliteLifecycleStore::open(&root, "test-issuer".into()).unwrap();
    let subject = SubjectId::parse("subject-current-startup").unwrap();
    store
        .register_subject(
            subject.clone(),
            ContextGeneration::parse("context-current-startup").unwrap(),
            BuildId::parse("build-current-startup").unwrap(),
            ProofRunId::parse("proof-current-startup").unwrap(),
            ClockSample {
                wall: WallTimeMs::new(100),
                wait_budget: WaitBudgetMs::new(100),
            },
        )
        .unwrap();
    let initial = store.load_recovery_entries().unwrap();
    drop(store);
    let config = json!({
        "profile": "read_only",
        "store_root": root,
        "socket_path": socket,
        "peer_socket_path": temporary.path().join("no-peer.sock"),
        "verifier_executable": null,
        "verifier_ledger_path": null,
        "proof_barrier_dir": null,
        "trusted_issuer": "test-issuer",
        "principal": {
            "uid": fs::metadata(temporary.path()).unwrap().uid(),
            "principal_ref": "operator:controlled",
            "producer_ref": "fixture:startup"
        },
        "subject_id": "subject-current-startup",
        "context_generation": "context-current-startup",
        "build_id": "build-current-startup",
        "proof_run_id": "proof-current-startup",
        "grants": [],
        "max_executions": 1,
        "max_wait_ms": 100
    });
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();

    for _ in 0..2 {
        let mut child = Command::new(&binary)
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut ready = false;
        while Instant::now() < deadline {
            if UnixStream::connect(&socket).is_ok() {
                ready = true;
                break;
            }
            assert!(
                child.try_wait().unwrap().is_none(),
                "service exited before socket-ready"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(ready, "service did not reach socket-ready startup boundary");
        assert!(SqliteLifecycleStore::open(&root, "test-issuer".into()).is_err());
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let reopened = SqliteLifecycleStore::open(&root, "test-issuer".into()).unwrap();
        assert_eq!(reopened.load_recovery_entries().unwrap(), initial);
        assert_eq!(reopened.admin_ownership().unwrap().status, "active");
        drop(reopened);
    }
}

#[test]
#[ignore = "requires LIFECYCLE_CURRENT_SERVICE_BINARY built with native-sim-proof startup barriers"]
fn current_service_reopens_after_each_acknowledged_startup_crash_cut() {
    let binary = PathBuf::from(std::env::var_os("LIFECYCLE_CURRENT_SERVICE_BINARY").unwrap());
    for cut in [
        "startup_before_store_open",
        "startup_before_recovery",
        "startup_after_recovery",
    ] {
        let temporary = tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let root = temporary.path().join("store");
        let barriers = temporary.path().join("barriers");
        fs::create_dir(&barriers).unwrap();
        fs::write(barriers.join(format!("{cut}.arm")), b"armed").unwrap();
        let config_path = temporary.path().join("service.json");
        let mut store = SqliteLifecycleStore::open(&root, "test-issuer".into()).unwrap();
        let subject = SubjectId::parse("subject-startup-cut").unwrap();
        store
            .register_subject(
                subject,
                ContextGeneration::parse("context-startup-cut").unwrap(),
                BuildId::parse("build-startup-cut").unwrap(),
                ProofRunId::parse("proof-startup-cut").unwrap(),
                ClockSample {
                    wall: WallTimeMs::new(100),
                    wait_budget: WaitBudgetMs::new(100),
                },
            )
            .unwrap();
        let initial = store.load_recovery_entries().unwrap();
        drop(store);
        let config = json!({
            "profile": "read_only",
            "store_root": root,
            "socket_path": temporary.path().join("service.sock"),
            "peer_socket_path": temporary.path().join("no-peer.sock"),
            "verifier_executable": null,
            "verifier_ledger_path": null,
            "proof_barrier_dir": barriers,
            "trusted_issuer": "test-issuer",
            "principal": {
                "uid": fs::metadata(temporary.path()).unwrap().uid(),
                "principal_ref": "operator:controlled",
                "producer_ref": "fixture:startup"
            },
            "subject_id": "subject-startup-cut",
            "context_generation": "context-startup-cut",
            "build_id": "build-startup-cut",
            "proof_run_id": "proof-startup-cut",
            "grants": [],
            "max_executions": 1,
            "max_wait_ms": 100
        });
        fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
        let mut child = Command::new(&binary)
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let reached = barriers.join(format!("{cut}.reached"));
        let deadline = Instant::now() + Duration::from_secs(3);
        while !reached.exists() && Instant::now() < deadline {
            assert!(
                child.try_wait().unwrap().is_none(),
                "{cut}: service exited early"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(fs::read(&reached).unwrap(), b"reached\n", "{cut}");
        assert!(!temporary.path().join("service.sock").exists(), "{cut}");
        if cut != "startup_before_store_open" {
            assert!(
                SqliteLifecycleStore::open(&root, "test-issuer".into()).is_err(),
                "{cut}"
            );
        }
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let reopened = SqliteLifecycleStore::open(&root, "test-issuer".into()).unwrap();
        assert_eq!(reopened.load_recovery_entries().unwrap(), initial, "{cut}");
        assert_eq!(
            reopened.admin_ownership().unwrap().status,
            "active",
            "{cut}"
        );
    }
}

#[test]
#[ignore = "requires pinned S5 and current service binaries for the selected-root process join"]
fn selected_destination_launches_service_while_fenced_source_entry_and_s5_refuse() {
    let current = PathBuf::from(std::env::var_os("LIFECYCLE_CURRENT_SERVICE_BINARY").unwrap());
    let old = PathBuf::from(std::env::var_os("LIFECYCLE_S5_BINARY").unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&old).unwrap())),
        "e6bac55cbe68e300ca404c7502007bcefb61e27a3773103791702c94867cce95"
    );
    let temporary = tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let source_root = temporary.path().join("source");
    let capsule_root = temporary.path().join("capsule");
    let destination_root = temporary.path().join("destination");
    let selector_root = temporary.path().join("selector");
    fs::create_dir(&selector_root).unwrap();
    let mut source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let subject = SubjectId::parse("subject-selected-service").unwrap();
    source
        .register_subject(
            subject,
            ContextGeneration::parse("context-selected-service").unwrap(),
            BuildId::parse("build-selected-service").unwrap(),
            ProofRunId::parse("proof-selected-service").unwrap(),
            ClockSample {
                wall: WallTimeMs::new(100),
                wait_budget: WaitBudgetMs::new(100),
            },
        )
        .unwrap();
    let expected = SelectionRecord {
        generation: 0,
        store_id: source.admin_store_identity().unwrap(),
        root: source.root().to_owned(),
        handoff_id: None,
        capsule_digest: None,
    };
    let mut selection =
        FileLaunchSelectionPort::initialize(&selector_root, expected.clone()).unwrap();
    let handoff = HandoffId::parse("handoff-selected-service").unwrap();
    let receipt = transfer_current(
        &mut source,
        &mut selection,
        &expected,
        &capsule_root,
        &destination_root,
        &handoff,
        "test-issuer".into(),
    )
    .unwrap();
    let hash = "a".repeat(64);
    assert!(
        source
            .reserve_native_session(
                NativeSessionId::parse("session-fenced-source").unwrap(),
                SubjectId::parse("subject-selected-service").unwrap(),
                ContextGeneration::parse("context-selected-service").unwrap(),
                RuntimeIncarnation::parse("incarnation-fenced-source").unwrap(),
                &hash,
                &hash,
                &hash,
                SnapshotId::parse("snapshot-fenced-source").unwrap(),
                &hash,
                false,
                ClockSample {
                    wall: WallTimeMs::new(101),
                    wait_budget: WaitBudgetMs::new(100),
                },
            )
            .is_err()
    );
    let guard = selection.lock().unwrap();
    let destination = SqliteLifecycleStore::open(&destination_root, "test-issuer".into()).unwrap();
    assert_eq!(
        verify_selected_launch(&guard, &receipt, &source, &destination).unwrap(),
        destination_root
    );
    assert!(verify_selected_launch(&guard, &receipt, &destination, &source).is_err());
    assert_eq!(guard.read().unwrap(), receipt.selected);
    drop(destination);

    let socket = temporary.path().join("selected-service.sock");
    let config_path = temporary.path().join("selected-service.json");
    let mut config = json!({
        "profile": "read_only",
        "store_root": receipt.selected.root,
        "socket_path": socket,
        "peer_socket_path": temporary.path().join("no-peer.sock"),
        "verifier_executable": null,
        "verifier_ledger_path": null,
        "proof_barrier_dir": null,
        "trusted_issuer": "test-issuer",
        "principal": {
            "uid": fs::metadata(temporary.path()).unwrap().uid(),
            "principal_ref": "operator:controlled",
            "producer_ref": "fixture:selected-service"
        },
        "subject_id": "subject-selected-service",
        "context_generation": "context-selected-service",
        "build_id": "build-selected-service",
        "proof_run_id": "proof-selected-service",
        "grants": [],
        "max_executions": 1,
        "max_wait_ms": 100
    });
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut child = Command::new(&current)
        .arg(&config_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut ready = false;
    while Instant::now() < deadline {
        if UnixStream::connect(&socket).is_ok() {
            ready = true;
            break;
        }
        assert!(
            child.try_wait().unwrap().is_none(),
            "selected service exited before socket-ready"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ready, "selected destination never served");
    assert!(SqliteLifecycleStore::open(&destination_root, "test-issuer".into()).is_err());
    assert_eq!(source.admin_ownership().unwrap().status, "fenced");
    assert_eq!(guard.read().unwrap(), receipt.selected);
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());

    config["socket_path"] = json!(temporary.path().join("old-selected-service.sock"));
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let before = all_bytes(&destination_root);
    let rejected = Command::new(&old).arg(&config_path).output().unwrap();
    assert_eq!(rejected.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("lifecycle store failed"));
    assert_eq!(before, all_bytes(&destination_root));
    assert!(!temporary.path().join("old-selected-service.sock").exists());
    let destination = SqliteLifecycleStore::open(&destination_root, "test-issuer".into()).unwrap();
    assert_eq!(
        verify_selected_launch(&guard, &receipt, &source, &destination).unwrap(),
        destination_root
    );
}
