use std::fs;
use std::os::unix::fs::symlink;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use lifecycle_admin::selection::{
    FileLaunchSelectionPort, LaunchSelectionGuard, LaunchSelectionPort, SelectionRecord,
};
use lifecycle_admin::transfer::TransferReceipt;
use lifecycle_admin::transfer::transfer_current;
use lifecycle_core::{
    BuildId, ClockSample, ContextGeneration, HandoffId, ProofRunId, SubjectId, WaitBudgetMs,
    WallTimeMs,
};
use lifecycle_store::SqliteLifecycleStore;
use tempfile::tempdir;

fn clock(wall: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(wall),
        wait_budget: WaitBudgetMs::new(100),
    }
}

#[test]
fn aliased_capsule_parent_is_rejected_before_export_or_fence() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let selector_root = temporary.path().join("selector");
    let alias_parent = temporary.path().join("alias-parent");
    fs::create_dir(&selector_root).unwrap();
    symlink(temporary.path(), &alias_parent).unwrap();
    let mut source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let expected = SelectionRecord {
        generation: 0,
        store_id: source.admin_store_identity().unwrap(),
        root: source.root().to_owned(),
        handoff_id: None,
        capsule_digest: None,
    };
    FileLaunchSelectionPort::initialize(&selector_root, expected.clone()).unwrap();
    let mut selection = FileLaunchSelectionPort::open(&selector_root).unwrap();
    let aliased_capsule = alias_parent.join("capsule");
    assert!(
        transfer_current(
            &mut source,
            &mut selection,
            &expected,
            &aliased_capsule,
            &temporary.path().join("destination"),
            &HandoffId::parse("handoff-alias").unwrap(),
            "test-issuer".into(),
        )
        .is_err()
    );
    assert!(!temporary.path().join("capsule").exists());
    assert_eq!(source.admin_ownership().unwrap().status, "active");
    assert_eq!(selection.lock().unwrap().read().unwrap(), expected);
}

#[test]
fn actual_process_exit_at_each_acknowledged_handoff_cut_resumes_same_identity() {
    let executable = env!("CARGO_BIN_EXE_lifecycle-admin");
    for cut in [
        "exported",
        "adopted_inactive",
        "source_fenced",
        "selection_committed",
        "destination_owned",
    ] {
        let temporary = tempdir().unwrap();
        let source_root = temporary.path().join("source");
        let capsule_root = temporary.path().join("capsule");
        let destination_root = temporary.path().join("destination");
        let selector_root = temporary.path().join("selector");
        let expected_path = temporary.path().join("expected.json");
        let barrier_path = temporary.path().join("barrier.txt");
        fs::create_dir(&selector_root).unwrap();
        let mut source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
        let subject = SubjectId::parse("subject-crash").unwrap();
        source
            .register_subject(
                subject.clone(),
                ContextGeneration::parse("context-crash").unwrap(),
                BuildId::parse("build-crash").unwrap(),
                ProofRunId::parse("proof-crash").unwrap(),
                clock(100),
            )
            .unwrap();
        let artifact = source
            .stage_artifact(subject, b"crash cut artifact")
            .unwrap();
        let artifact = source.publish_artifact(artifact, clock(101)).unwrap();
        let expected = SelectionRecord {
            generation: 0,
            store_id: source.admin_store_identity().unwrap(),
            root: source.root().to_owned(),
            handoff_id: None,
            capsule_digest: None,
        };
        FileLaunchSelectionPort::initialize(&selector_root, expected.clone()).unwrap();
        fs::write(&expected_path, serde_json::to_vec(&expected).unwrap()).unwrap();
        drop(source);

        let args = [
            source_root.as_os_str(),
            selector_root.as_os_str(),
            expected_path.as_os_str(),
            capsule_root.as_os_str(),
            destination_root.as_os_str(),
            std::ffi::OsStr::new("handoff-crash"),
        ];
        let crashed = Command::new(executable)
            .arg("transfer-current")
            .args(args)
            .env("LIFECYCLE_ADMIN_TEST_CUT", cut)
            .env("LIFECYCLE_ADMIN_TEST_BARRIER", &barrier_path)
            .output()
            .unwrap();
        assert_eq!(
            crashed.status.code(),
            Some(86),
            "{cut}: {}",
            String::from_utf8_lossy(&crashed.stderr)
        );
        assert_eq!(fs::read_to_string(&barrier_path).unwrap(), cut);

        let source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
        let source_state = source.admin_ownership().unwrap();
        let destination_state = if destination_root.exists() {
            Some(
                SqliteLifecycleStore::open(&destination_root, "test-issuer".into())
                    .unwrap()
                    .admin_ownership()
                    .unwrap(),
            )
        } else {
            None
        };
        let mut selector = FileLaunchSelectionPort::open(&selector_root).unwrap();
        let selected = selector.lock().unwrap().read().unwrap();
        match cut {
            "exported" => {
                assert_eq!(source_state.status, "active");
                assert!(destination_state.is_none());
                assert_eq!(selected, expected);
            }
            "adopted_inactive" => {
                assert_eq!(source_state.status, "active");
                assert_eq!(destination_state.unwrap().status, "inactive");
                assert_eq!(selected, expected);
            }
            "source_fenced" => {
                assert_eq!(source_state.status, "fenced");
                assert_eq!(destination_state.unwrap().status, "inactive");
                assert_eq!(selected, expected);
            }
            "selection_committed" => {
                assert_eq!(source_state.status, "fenced");
                assert_eq!(destination_state.unwrap().status, "inactive");
                assert_eq!(selected.generation, 1);
            }
            "destination_owned" => {
                assert_eq!(source_state.status, "fenced");
                assert_eq!(destination_state.unwrap().status, "active");
                assert_eq!(selected.generation, 1);
            }
            _ => unreachable!(),
        }
        drop(source);

        let resumed = Command::new(executable)
            .arg("resume-transfer")
            .args(args)
            .output()
            .unwrap();
        assert!(
            resumed.status.success(),
            "{cut}: {}",
            String::from_utf8_lossy(&resumed.stderr)
        );
        let receipt: TransferReceipt = serde_json::from_slice(&resumed.stdout).unwrap();
        assert_eq!(receipt.selected.generation, 1);
        assert_eq!(receipt.capsule.artifacts[0].digest_hex, artifact.digest_hex);
        let retried = Command::new(executable)
            .arg("resume-transfer")
            .args(args)
            .output()
            .unwrap();
        assert!(
            retried.status.success(),
            "{cut}: {}",
            String::from_utf8_lossy(&retried.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<TransferReceipt>(&retried.stdout).unwrap(),
            receipt
        );
        let source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
        assert_eq!(source.admin_ownership().unwrap().status, "fenced");
        let destination =
            SqliteLifecycleStore::open(&destination_root, "test-issuer".into()).unwrap();
        assert_eq!(destination.admin_ownership().unwrap().status, "active");
        assert_eq!(
            destination
                .read_committed_artifact(&artifact.digest_hex)
                .unwrap()
                .unwrap(),
            b"crash cut artifact"
        );
    }
}

#[test]
fn paused_launch_holder_excludes_transfer_until_its_process_exits() {
    let executable = env!("CARGO_BIN_EXE_lifecycle-admin");
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let capsule_root = temporary.path().join("capsule");
    let destination_root = temporary.path().join("destination");
    let selector_root = temporary.path().join("selector");
    let expected_path = temporary.path().join("expected.json");
    let barrier_path = temporary.path().join("launch-held.txt");
    fs::create_dir(&selector_root).unwrap();
    let source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let expected = SelectionRecord {
        generation: 0,
        store_id: source.admin_store_identity().unwrap(),
        root: source.root().to_owned(),
        handoff_id: None,
        capsule_digest: None,
    };
    FileLaunchSelectionPort::initialize(&selector_root, expected.clone()).unwrap();
    fs::write(&expected_path, serde_json::to_vec(&expected).unwrap()).unwrap();
    drop(source);

    let mut holder = Command::new(executable)
        .arg("hold-selection")
        .arg(&selector_root)
        .arg(&barrier_path)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !barrier_path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(&barrier_path).unwrap(), "selection_held");
    assert!(holder.try_wait().unwrap().is_none());

    let mut transfer = Command::new(executable)
        .arg("transfer-current")
        .arg(&source_root)
        .arg(&selector_root)
        .arg(&expected_path)
        .arg(&capsule_root)
        .arg(&destination_root)
        .arg("handoff-paused")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(80));
    assert!(transfer.try_wait().unwrap().is_none());
    assert!(!destination_root.exists());
    assert_eq!(
        serde_json::from_slice::<SelectionRecord>(
            &fs::read(selector_root.join("selection.json")).unwrap()
        )
        .unwrap(),
        expected
    );
    holder.kill().unwrap();
    holder.wait().unwrap();
    let finished = transfer.wait_with_output().unwrap();
    assert!(
        finished.status.success(),
        "{}",
        String::from_utf8_lossy(&finished.stderr)
    );
    let receipt: TransferReceipt = serde_json::from_slice(&finished.stdout).unwrap();
    assert_eq!(receipt.selected.generation, 1);
    assert_eq!(
        SqliteLifecycleStore::open(&source_root, "test-issuer".into())
            .unwrap()
            .admin_ownership()
            .unwrap()
            .status,
        "fenced"
    );
    assert_eq!(
        SqliteLifecycleStore::open(&destination_root, "test-issuer".into())
            .unwrap()
            .admin_ownership()
            .unwrap()
            .status,
        "active"
    );
}
