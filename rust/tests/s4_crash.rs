use std::{
    fs,
    io::Write,
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use lifecycle_core::SubjectId;
use lifecycle_store::{RecoveryDisposition, SqliteLifecycleStore};
use serde_json::{Value, json};
use tempfile::tempdir;
use work_engine_types::CodecContract;

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_until(mut predicate: impl FnMut() -> bool, what: &str) {
    let started = Instant::now();
    while !predicate() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "timeout: {what}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn peer_binary() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let status = Command::new("cargo")
        .arg("+1.92.0")
        .args([
            "build",
            "--manifest-path",
            "rust/Cargo.toml",
            "-p",
            "lifecycle-test-support",
            "--bin",
            "controlled_peer",
            "--locked",
        ])
        .current_dir(root.parent().unwrap())
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"))
        .join("debug/controlled_peer")
}

fn enqueue() -> Value {
    let text = "crash cut input";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap()
        .hex();
    let mut value = json!({"protocol_version":2,"principal_ref":"principal-cut","command_id":"command-cut","subject_id":"subject-cut",
        "context_generation":"context-cut","build_id":"build-cut","proof_run_id":"proof-cut","grant_ref":"grant-cut",
        "expected_revision":"0","kind":"enqueue_input","payload":{"input_id":"input-cut","producer_ref":"producer-cut","text":text,"text_digest":digest}});
    value["request_digest"] = json!(
        CodecContract::LifecycleCommandV2
            .digest_json(&value)
            .unwrap()
            .hex()
    );
    value
}

fn run_case(cut: &str, peer_executable: &Path) {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let barrier = root.join("barrier");
    fs::create_dir(&barrier).unwrap();
    for earlier in [
        "before_entry_commit",
        "after_entry_commit",
        "after_result_commit",
    ] {
        if earlier != cut {
            fs::write(barrier.join(format!("{earlier}.release")), b"go").unwrap();
        }
    }
    let socket = root.join("service.sock");
    let peer_socket = root.join("peer.sock");
    let ledger = root.join("ledger.jsonl");
    let peer_release = root.join("peer.release");
    let db = root.join("store");
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 120_000;
    let uid = fs::metadata(root).unwrap().uid();
    let config = json!({"profile":"controlled","store_root":db,"socket_path":socket,"peer_socket_path":peer_socket,
        "verifier_executable":peer_executable,"verifier_ledger_path":ledger,"proof_barrier_dir":barrier,
        "trusted_issuer":"issuer-cut","principal":{"uid":uid,"principal_ref":"principal-cut","producer_ref":"producer-cut"},
        "subject_id":"subject-cut","context_generation":"context-cut","build_id":"build-cut","proof_run_id":"proof-cut",
        "grants":[{"grant_ref":"grant-cut","context_generation":"context-cut","scope":"enqueue_input","expires_wall_ms":expires,"revision":0}],
        "max_executions":1,"max_wait_ms":1000});
    let config_path = root.join("config.json");
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut args = vec![
        peer_socket.as_os_str(),
        ledger.as_os_str(),
        std::ffi::OsStr::new("context-next"),
    ];
    if cut == "after_send" {
        args.push(peer_release.as_os_str());
    }
    let _peer = ChildGuard(
        Command::new(peer_executable)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    wait_until(|| peer_socket.exists(), "peer socket");
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_until(|| UnixStream::connect(&socket).is_ok(), "service socket");
    let mut stream = UnixStream::connect(&socket).unwrap();
    stream
        .write_all(&serde_json::to_vec(&json!({"op":"command","command":enqueue()})).unwrap())
        .unwrap();
    stream.write_all(b"\n").unwrap();
    drop(stream);
    if cut == "after_send" {
        wait_until(
            || {
                fs::read_to_string(&ledger)
                    .unwrap_or_default()
                    .lines()
                    .count()
                    == 1
            },
            "peer entered",
        );
    } else {
        wait_until(
            || barrier.join(format!("{cut}.reached")).exists(),
            "barrier reached",
        );
    }
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    let store = SqliteLifecycleStore::open(&db, "issuer-cut".into()).unwrap();
    let (_, inputs, attempts) = store.counts().unwrap();
    assert_eq!(inputs, 1);
    assert_eq!(attempts, if cut == "before_entry_commit" { 0 } else { 1 });
    let recovery = store.load_recovery().unwrap();
    assert_eq!(
        recovery.unresolved_attempts,
        if matches!(
            cut,
            "before_entry_commit" | "after_entry_commit" | "after_send"
        ) && cut != "before_entry_commit"
        {
            1
        } else {
            0
        }
    );
    let entries = store.load_recovery_entries().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].disposition,
        match cut {
            "before_entry_commit" => RecoveryDisposition::PreparedNeedsGrant,
            "after_result_commit" => RecoveryDisposition::CompletedSettled,
            _ => RecoveryDisposition::EnteredUncertain,
        }
    );
    drop(store);
    if cut == "after_send" {
        fs::write(&peer_release, b"go").unwrap();
    }
    fs::write(barrier.join(format!("{cut}.release")), b"go").unwrap();
    if cut == "before_entry_commit" {
        let mut passive = config.clone();
        passive["profile"] = json!("read_only");
        fs::write(&config_path, serde_json::to_vec(&passive).unwrap()).unwrap();
        let mut reader = ChildGuard(
            Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
                .arg(&config_path)
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        wait_until(
            || UnixStream::connect(&socket).is_ok(),
            "read-only restart socket",
        );
        thread::sleep(Duration::from_millis(200));
        assert_eq!(
            fs::read_to_string(&ledger)
                .unwrap_or_default()
                .lines()
                .count(),
            0,
            "read-only profile dispatched prepared work"
        );
        reader.0.kill().unwrap();
        reader.0.wait().unwrap();
        fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    }
    service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_until(|| UnixStream::connect(&socket).is_ok(), "restarted socket");
    if cut == "before_entry_commit" {
        wait_until(
            || {
                fs::read_to_string(&ledger)
                    .unwrap_or_default()
                    .lines()
                    .count()
                    == 1
            },
            "recovered dispatch",
        );
    } else if cut == "after_result_commit" {
        assert_eq!(fs::read_to_string(&ledger).unwrap().lines().count(), 1);
    } else {
        thread::sleep(Duration::from_millis(100));
        assert_eq!(
            fs::read_to_string(&ledger)
                .unwrap_or_default()
                .lines()
                .count(),
            if cut == "after_send" { 1 } else { 0 },
            "entered work resent"
        );
    }
    let _ = SubjectId::parse("subject-cut").unwrap();
    assert!(service.0.try_wait().unwrap().is_none());
}

#[test]
fn four_real_process_kill_cuts_preserve_no_replay_and_classification() {
    let peer = peer_binary();
    for cut in [
        "before_entry_commit",
        "after_entry_commit",
        "after_send",
        "after_result_commit",
    ] {
        run_case(cut, &peer);
    }
}
