use std::fs::{self, OpenOptions};
use std::os::unix::fs::MetadataExt;
use std::process::Command;

use lifecycle_admin::selection::{
    FileLaunchSelectionPort, LaunchSelectionGuard, LaunchSelectionPort, SelectionRecord,
};
use lifecycle_admin::transfer::TransferReceipt;
use lifecycle_core::HandoffId;
use lifecycle_store::{SqliteLifecycleStore, StoreError};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

const TMP_ROOT: &str = "/home/bline/code/.work-engine-tmp/s4-20261007/tmp";
const TARGET_ROOT: &str = "/home/bline/code/.work-engine-tmp/s4-20261007/target";

fn bytes_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn crash_after_adoption_db_copy_never_exposes_second_active_owner() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let capsule_root = temporary.path().join("capsule");
    let destination_root = temporary.path().join("destination");
    let selector_root = temporary.path().join("selector");
    let expected_path = temporary.path().join("expected.json");
    let barrier_path = temporary.path().join("adoption-copy.barrier");
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

    let executable = env!("CARGO_BIN_EXE_lifecycle-admin");
    let args = [
        source_root.as_os_str(),
        selector_root.as_os_str(),
        expected_path.as_os_str(),
        capsule_root.as_os_str(),
        destination_root.as_os_str(),
        std::ffi::OsStr::new("handoff-partial-adoption"),
    ];
    let crashed = Command::new(executable)
        .arg("transfer-current")
        .args(args)
        .env("TMPDIR", TMP_ROOT)
        .env("CARGO_TARGET_DIR", TARGET_ROOT)
        .env("LIFECYCLE_ADMIN_TEST_CUT", "adoption_after_db_copy")
        .env("LIFECYCLE_ADMIN_TEST_BARRIER", &barrier_path)
        .output()
        .unwrap();
    assert_eq!(
        crashed.status.code(),
        Some(86),
        "{}",
        String::from_utf8_lossy(&crashed.stderr)
    );
    assert_eq!(
        fs::read_to_string(&barrier_path).unwrap(),
        "adoption_after_db_copy"
    );
    assert!(SqliteLifecycleStore::open(&destination_root, "test-issuer".into()).is_err());
    let source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    assert_eq!(source.admin_ownership().unwrap().status, "active");
    drop(source);
    assert!(
        SqliteLifecycleStore::open_adopted_inactive(
            &destination_root,
            &HandoffId::parse("handoff-partial-adoption").unwrap(),
            &"b".repeat(64),
            "test-issuer".into(),
        )
        .is_err(),
        "a different capsule digest must not adopt a partial destination"
    );
    let mut selection = FileLaunchSelectionPort::open(&selector_root).unwrap();
    assert_eq!(selection.lock().unwrap().read().unwrap(), expected);

    let wrong_handoff = Command::new(executable)
        .arg("resume-transfer")
        .args(&args[..5])
        .arg("handoff-changed")
        .env("TMPDIR", TMP_ROOT)
        .env("CARGO_TARGET_DIR", TARGET_ROOT)
        .output()
        .unwrap();
    assert!(!wrong_handoff.status.success());
    assert_eq!(
        SqliteLifecycleStore::open(&source_root, "test-issuer".into())
            .unwrap()
            .admin_ownership()
            .unwrap()
            .status,
        "active"
    );

    let resumed = Command::new(executable)
        .arg("resume-transfer")
        .args(args)
        .env("TMPDIR", TMP_ROOT)
        .env("CARGO_TARGET_DIR", TARGET_ROOT)
        .output()
        .unwrap();
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    let receipt: TransferReceipt = serde_json::from_slice(&resumed.stdout).unwrap();
    assert_eq!(
        receipt.adoption.handoff_id,
        HandoffId::parse("handoff-partial-adoption")
            .unwrap()
            .as_str()
    );
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

#[test]
fn exact_partial_adoption_retry_preserves_a_contenders_writer_lock() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let capsule_root = temporary.path().join("capsule");
    let destination_root = temporary.path().join("destination");
    let selector_root = temporary.path().join("selector");
    let expected_path = temporary.path().join("expected.json");
    let barrier_path = temporary.path().join("adoption-copy.barrier");
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
    let executable = env!("CARGO_BIN_EXE_lifecycle-admin");
    let args = [
        source_root.as_os_str(),
        selector_root.as_os_str(),
        expected_path.as_os_str(),
        capsule_root.as_os_str(),
        destination_root.as_os_str(),
        std::ffi::OsStr::new("handoff-locked-partial"),
    ];
    let crashed = Command::new(executable)
        .arg("transfer-current")
        .args(args)
        .env("TMPDIR", TMP_ROOT)
        .env("CARGO_TARGET_DIR", TARGET_ROOT)
        .env("LIFECYCLE_ADMIN_TEST_CUT", "adoption_after_db_copy")
        .env("LIFECYCLE_ADMIN_TEST_BARRIER", &barrier_path)
        .output()
        .unwrap();
    assert_eq!(
        crashed.status.code(),
        Some(86),
        "{}",
        String::from_utf8_lossy(&crashed.stderr)
    );
    assert_eq!(
        fs::read_to_string(&barrier_path).unwrap(),
        "adoption_after_db_copy"
    );
    let capsule = SqliteLifecycleStore::read_capsule(&capsule_root).unwrap();
    let handoff = HandoffId::parse("handoff-locked-partial").unwrap();
    let lock_path = destination_root.join("writer.lock");
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .unwrap();
    lock_file.try_lock().unwrap();
    let contender = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    assert!(
        contender.try_lock().is_err(),
        "fixture must hold a real OS writer lock"
    );
    drop(contender);
    let lock_before = fs::metadata(&lock_path).unwrap();
    let database_before = fs::read(destination_root.join("lifecycle.sqlite")).unwrap();
    let intent_path = destination_root.with_file_name("destination.adoption-pending");
    let intent_before = fs::read(&intent_path).unwrap();

    let retry = SqliteLifecycleStore::adopt_inactive(
        &capsule,
        &destination_root,
        &handoff,
        "test-issuer".into(),
    );
    assert!(
        matches!(retry, Err(StoreError::Unavailable)),
        "held writer lock must make exact adoption unavailable"
    );
    let lock_after = fs::metadata(&lock_path).unwrap();
    assert_eq!(
        (lock_after.dev(), lock_after.ino()),
        (lock_before.dev(), lock_before.ino())
    );
    assert_eq!(
        fs::read(destination_root.join("lifecycle.sqlite")).unwrap(),
        database_before
    );
    assert_eq!(fs::read(&intent_path).unwrap(), intent_before);
    assert_eq!(
        SqliteLifecycleStore::open(&source_root, "test-issuer".into())
            .unwrap()
            .admin_ownership()
            .unwrap()
            .status,
        "active"
    );
    let mut selection = FileLaunchSelectionPort::open(&selector_root).unwrap();
    assert_eq!(selection.lock().unwrap().read().unwrap(), expected);
    drop(lock_file);

    let resumed = Command::new(executable)
        .arg("resume-transfer")
        .args(args)
        .env("TMPDIR", TMP_ROOT)
        .env("CARGO_TARGET_DIR", TARGET_ROOT)
        .output()
        .unwrap();
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    let receipt: TransferReceipt = serde_json::from_slice(&resumed.stdout).unwrap();
    assert_eq!(receipt.capsule.capsule_sha256, capsule.capsule_sha256);
    assert_eq!(receipt.adoption.handoff_id, handoff.as_str());
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

#[test]
fn crash_before_copy_complete_marker_recovers_only_exact_partial_adoption() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let capsule_root = temporary.path().join("capsule");
    let destination_root = temporary.path().join("destination");
    let selector_root = temporary.path().join("selector");
    let expected_path = temporary.path().join("expected.json");
    let barrier_path = temporary.path().join("before-copy-marker.barrier");
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
    let executable = env!("CARGO_BIN_EXE_lifecycle-admin");
    let args = [
        source_root.as_os_str(),
        selector_root.as_os_str(),
        expected_path.as_os_str(),
        capsule_root.as_os_str(),
        destination_root.as_os_str(),
        std::ffi::OsStr::new("handoff-before-marker"),
    ];
    let crashed = Command::new(executable)
        .arg("transfer-current")
        .args(args)
        .env("TMPDIR", TMP_ROOT)
        .env("CARGO_TARGET_DIR", TARGET_ROOT)
        .env("LIFECYCLE_ADMIN_TEST_CUT", "adoption_before_copy_marker")
        .env("LIFECYCLE_ADMIN_TEST_BARRIER", &barrier_path)
        .output()
        .unwrap();
    assert_eq!(
        crashed.status.code(),
        Some(87),
        "{}",
        String::from_utf8_lossy(&crashed.stderr)
    );
    assert_eq!(
        fs::read_to_string(&barrier_path).unwrap(),
        "adoption_before_copy_marker"
    );
    assert!(destination_root.exists());
    assert!(
        destination_root
            .with_file_name("destination.adoption-pending")
            .exists()
    );
    assert!(
        !destination_root
            .with_file_name("destination.adoption-copy-complete")
            .exists()
    );
    assert!(SqliteLifecycleStore::open(&destination_root, "test-issuer".into()).is_err());
    assert_eq!(
        SqliteLifecycleStore::open(&source_root, "test-issuer".into())
            .unwrap()
            .admin_ownership()
            .unwrap()
            .status,
        "active"
    );
    let mut selection = FileLaunchSelectionPort::open(&selector_root).unwrap();
    assert_eq!(selection.lock().unwrap().read().unwrap(), expected);

    let capsule = SqliteLifecycleStore::read_capsule(&capsule_root).unwrap();
    let resumed = Command::new(executable)
        .arg("resume-transfer")
        .args(args)
        .env("TMPDIR", TMP_ROOT)
        .env("CARGO_TARGET_DIR", TARGET_ROOT)
        .output()
        .unwrap();
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    let receipt: TransferReceipt = serde_json::from_slice(&resumed.stdout).unwrap();
    assert_eq!(receipt.capsule.capsule_sha256, capsule.capsule_sha256);
    assert_eq!(receipt.adoption.handoff_id, "handoff-before-marker");
    assert_eq!(receipt.adoption.source_store_id, expected.store_id);
    assert_ne!(receipt.adoption.destination_store_id, expected.store_id);
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

#[test]
fn rejected_retry_after_inactive_commit_preserves_adopted_destination() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let capsule_root = temporary.path().join("capsule");
    let destination_root = temporary.path().join("destination");
    let selector_root = temporary.path().join("selector");
    let expected_path = temporary.path().join("expected.json");
    let barrier_path = temporary.path().join("inactive-commit.barrier");
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
    let executable = env!("CARGO_BIN_EXE_lifecycle-admin");
    let args = [
        source_root.as_os_str(),
        selector_root.as_os_str(),
        expected_path.as_os_str(),
        capsule_root.as_os_str(),
        destination_root.as_os_str(),
        std::ffi::OsStr::new("handoff-after-commit"),
    ];
    let crashed = Command::new(executable)
        .arg("transfer-current")
        .args(args)
        .env("TMPDIR", TMP_ROOT)
        .env("CARGO_TARGET_DIR", TARGET_ROOT)
        .env("LIFECYCLE_ADMIN_TEST_CUT", "adoption_after_inactive_commit")
        .env("LIFECYCLE_ADMIN_TEST_BARRIER", &barrier_path)
        .output()
        .unwrap();
    assert_eq!(
        crashed.status.code(),
        Some(88),
        "{}",
        String::from_utf8_lossy(&crashed.stderr)
    );
    assert_eq!(
        fs::read_to_string(&barrier_path).unwrap(),
        "adoption_after_inactive_commit"
    );
    assert!(SqliteLifecycleStore::open(&destination_root, "test-issuer".into()).is_err());
    let capsule = SqliteLifecycleStore::read_capsule(&capsule_root).unwrap();
    let handoff = HandoffId::parse("handoff-after-commit").unwrap();
    let lock_path = destination_root.join("writer.lock");
    let database_path = destination_root.join("lifecycle.sqlite");
    let intent_path = destination_root.with_file_name("destination.adoption-pending");
    let marker_path = destination_root.with_file_name("destination.adoption-copy-complete");
    let root_before = fs::metadata(&destination_root).unwrap();
    let database_before = fs::metadata(&database_path).unwrap();
    let lock_before = fs::metadata(&lock_path).unwrap();
    let intent_before = fs::read(&intent_path).unwrap();
    let marker_before = fs::read(&marker_path).unwrap();
    let database_bytes_before = fs::read(&database_path).unwrap();

    assert!(matches!(
        SqliteLifecycleStore::adopt_inactive(&capsule, &destination_root, &handoff, String::new()),
        Err(StoreError::Rejected)
    ));
    let root_after = fs::metadata(&destination_root).unwrap();
    let database_after = fs::metadata(&database_path).unwrap();
    let lock_after = fs::metadata(&lock_path).unwrap();
    assert_eq!(
        (root_after.dev(), root_after.ino()),
        (root_before.dev(), root_before.ino())
    );
    assert_eq!(
        (database_after.dev(), database_after.ino()),
        (database_before.dev(), database_before.ino())
    );
    assert_eq!(
        (lock_after.dev(), lock_after.ino()),
        (lock_before.dev(), lock_before.ino())
    );
    assert_eq!(
        bytes_sha256(&fs::read(&database_path).unwrap()),
        bytes_sha256(&database_bytes_before)
    );
    assert_eq!(fs::read(&intent_path).unwrap(), intent_before);
    assert_eq!(fs::read(&marker_path).unwrap(), marker_before);
    assert_eq!(
        SqliteLifecycleStore::open(&source_root, "test-issuer".into())
            .unwrap()
            .admin_ownership()
            .unwrap()
            .status,
        "active"
    );
    let mut selection = FileLaunchSelectionPort::open(&selector_root).unwrap();
    assert_eq!(selection.lock().unwrap().read().unwrap(), expected);

    // An owner-side readback failure is distinct from a proven partial copy.
    // Run the opt-in fault in a child so parallel tests never share an env var.
    let unavailable = Command::new(executable)
        .arg("resume-transfer")
        .args(args)
        .env("TMPDIR", TMP_ROOT)
        .env("CARGO_TARGET_DIR", TARGET_ROOT)
        .env("LIFECYCLE_ADMIN_TEST_CUT", "adoption_readback_unavailable")
        .output()
        .unwrap();
    assert!(!unavailable.status.success());
    assert!(String::from_utf8_lossy(&unavailable.stderr).contains("store is unavailable"));
    let root_after_unavailable = fs::metadata(&destination_root).unwrap();
    let database_after_unavailable = fs::metadata(&database_path).unwrap();
    let lock_after_unavailable = fs::metadata(&lock_path).unwrap();
    assert_eq!(
        (root_after_unavailable.dev(), root_after_unavailable.ino()),
        (root_before.dev(), root_before.ino())
    );
    assert_eq!(
        (
            database_after_unavailable.dev(),
            database_after_unavailable.ino()
        ),
        (database_before.dev(), database_before.ino())
    );
    assert_eq!(
        (lock_after_unavailable.dev(), lock_after_unavailable.ino()),
        (lock_before.dev(), lock_before.ino())
    );
    assert_eq!(fs::read(&intent_path).unwrap(), intent_before);
    assert_eq!(fs::read(&marker_path).unwrap(), marker_before);
    assert_eq!(
        SqliteLifecycleStore::open(&source_root, "test-issuer".into())
            .unwrap()
            .admin_ownership()
            .unwrap()
            .status,
        "active"
    );
    assert_eq!(selection.lock().unwrap().read().unwrap(), expected);

    let resumed = Command::new(executable)
        .arg("resume-transfer")
        .args(args)
        .env("TMPDIR", TMP_ROOT)
        .env("CARGO_TARGET_DIR", TARGET_ROOT)
        .output()
        .unwrap();
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    let receipt: TransferReceipt = serde_json::from_slice(&resumed.stdout).unwrap();
    assert_eq!(receipt.capsule.capsule_sha256, capsule.capsule_sha256);
    assert_eq!(receipt.adoption.handoff_id, handoff.as_str());
    assert_eq!(
        receipt.selected.store_id,
        receipt.adoption.destination_store_id
    );
    assert_ne!(receipt.selected.store_id, expected.store_id);
    let final_root = fs::metadata(&destination_root).unwrap();
    let final_database = fs::metadata(&database_path).unwrap();
    let final_lock = fs::metadata(&lock_path).unwrap();
    assert_eq!(
        (final_root.dev(), final_root.ino()),
        (root_before.dev(), root_before.ino())
    );
    assert_eq!(
        (final_database.dev(), final_database.ino()),
        (database_before.dev(), database_before.ino())
    );
    assert_eq!(
        (final_lock.dev(), final_lock.ino()),
        (lock_before.dev(), lock_before.ino())
    );
    assert!(!intent_path.exists());
    assert!(!marker_path.exists());
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
