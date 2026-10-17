use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_private(path: &Path, value: &Value) {
    let mut file = File::create(path).unwrap();
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .unwrap();
    serde_json::to_writer(&mut file, value).unwrap();
    file.write_all(b"\n").unwrap();
    file.sync_all().unwrap();
}

struct Fixture {
    _temp: TempDir,
    directory: PathBuf,
    selection: PathBuf,
    config: PathBuf,
    root: PathBuf,
    socket: PathBuf,
    binary: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().canonicalize().unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let config = directory.join("service.json");
        let selection = directory.join("selection.json");
        let root = directory.join("store");
        let socket = directory.join("service.sock");
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_lifecycle-service"))
            .canonicalize()
            .unwrap();
        let uid = fs::metadata(&directory).unwrap().uid();
        let service = json!({
            "profile": "read_only", "store_root": root, "socket_path": socket,
            "peer_socket_path": directory.join("unused-peer.sock"),
            "verifier_executable": null, "verifier_ledger_path": null,
            "proof_barrier_dir": null, "trusted_issuer": "development-test-issuer",
            "principal": {"uid": uid, "principal_ref": "development-test-principal", "producer_ref": "development-test-producer"},
            "subject_id": "development-test-subject", "context_generation": "development-test-generation",
            "build_id": "development-test-build", "proof_run_id": "development-test-run",
            "grants": [], "max_executions": 1, "max_wait_ms": 1000,
            "verifier_proof_ms": null, "verifier_cleanup_ms": null, "native": null
        });
        write_private(&config, &service);
        let selected = json!({
            "version": 1, "selection_id": "disposable-development-test",
            "executable_path": binary, "executable_sha256": digest(&fs::read(&binary).unwrap()),
            "config_path": config, "config_sha256": digest(&fs::read(&config).unwrap()),
            "store_root": root, "socket_path": socket, "build_id": "development-test-build"
        });
        write_private(&selection, &selected);
        Self {
            _temp: temp,
            directory,
            selection,
            config,
            root,
            socket,
            binary,
        }
    }

    fn run(&self) -> Child {
        Command::new(&self.binary)
            .arg("--development-selected")
            .arg(&self.selection)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }

    fn wait_ready(&self, child: &mut Child) {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            if UnixStream::connect(&self.socket).is_ok() {
                return;
            }
            if let Some(status) = child.try_wait().unwrap() {
                let mut error = String::new();
                child
                    .stderr
                    .take()
                    .unwrap()
                    .read_to_string(&mut error)
                    .unwrap();
                panic!("selected service exited {status}: {error}");
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("selected service did not bind its socket");
    }
}

fn stop(child: &mut Child) {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(child.id().to_string())
        .status()
        .unwrap();
    assert!(status.success());
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "service close: {status}");
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    panic!("service failed to close on SIGTERM");
}

fn send(socket: &Path, request: Value) -> Value {
    let mut stream = UnixStream::connect(socket).unwrap();
    stream
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    stream.write_all(b"\n").unwrap();
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response).unwrap();
    serde_json::from_str(&response).unwrap()
}

fn refuse(fixture: &Fixture) {
    let mut child = Command::new(&fixture.binary)
        .arg("--development-selected")
        .arg(&fixture.selection)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(!status.success(), "selected launch unexpectedly succeeded");
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    panic!("selected launch did not refuse within five seconds");
}

#[test]
fn fresh_selected_boot_and_exact_restart_keep_store_identity() {
    let fixture = Fixture::new();
    let mut first = fixture.run();
    fixture.wait_ready(&mut first);
    let first_receipt: Value =
        serde_json::from_slice(&fs::read(fixture.directory.join("startup-receipt.json")).unwrap())
            .unwrap();
    assert_eq!(first_receipt["version"], 1);
    assert!(
        first_receipt["store_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    let first_snapshot = send(
        &fixture.socket,
        json!({"op":"snapshot","subject_id":"development-test-subject"}),
    );
    assert!(
        first_snapshot.get("code").is_none(),
        "snapshot refused: {first_snapshot}"
    );
    let refused_command = send(&fixture.socket, json!({"op":"command","command":{}}));
    assert_eq!(refused_command["code"], "unsupported_capability");
    let refused_grant = send(&fixture.socket, json!({"op":"contract_grant","command":{}}));
    assert_eq!(refused_grant["code"], "unsupported_capability");
    // The stable selection lock excludes an accidental second selected process.
    refuse(&fixture);
    stop(&mut first);

    let mut second = fixture.run();
    fixture.wait_ready(&mut second);
    let second_receipt: Value =
        serde_json::from_slice(&fs::read(fixture.directory.join("startup-receipt.json")).unwrap())
            .unwrap();
    assert_eq!(first_receipt, second_receipt);
    let second_snapshot = send(
        &fixture.socket,
        json!({"op":"snapshot","subject_id":"development-test-subject"}),
    );
    assert!(
        second_snapshot.get("code").is_none(),
        "restart snapshot refused: {second_snapshot}"
    );
    stop(&mut second);
}

#[test]
fn mismatch_and_foreign_root_refuse_before_socket() {
    let fixture = Fixture::new();
    fs::write(&fixture.config, b"changed\n").unwrap();
    refuse(&fixture);
    assert!(!fixture.root.exists());
    assert!(!fixture.socket.exists());

    let fixture = Fixture::new();
    fs::create_dir(&fixture.root).unwrap();
    refuse(&fixture);
    assert!(!fixture.socket.exists());
    assert!(!fixture.directory.join("startup-intent.json").exists());
}

#[test]
fn selected_executable_and_profile_cannot_be_rebound_to_effectful_route() {
    let fixture = Fixture::new();
    let mut selected: Value =
        serde_json::from_slice(&fs::read(&fixture.selection).unwrap()).unwrap();
    selected["executable_sha256"] = json!("0".repeat(64));
    write_private(&fixture.selection, &selected);
    refuse(&fixture);
    assert!(!fixture.root.exists());

    let fixture = Fixture::new();
    let mut config: Value = serde_json::from_slice(&fs::read(&fixture.config).unwrap()).unwrap();
    config["profile"] = json!("controlled");
    write_private(&fixture.config, &config);
    let mut selected: Value =
        serde_json::from_slice(&fs::read(&fixture.selection).unwrap()).unwrap();
    selected["config_sha256"] = json!(digest(&fs::read(&fixture.config).unwrap()));
    write_private(&fixture.selection, &selected);
    refuse(&fixture);
    assert!(!fixture.root.exists());
}

#[test]
fn changed_selection_and_store_identity_refuse_on_restart() {
    let fixture = Fixture::new();
    let mut child = fixture.run();
    fixture.wait_ready(&mut child);
    stop(&mut child);

    let mut selected: Value =
        serde_json::from_slice(&fs::read(&fixture.selection).unwrap()).unwrap();
    selected["selection_id"] = json!("different-selection");
    write_private(&fixture.selection, &selected);
    refuse(&fixture);

    selected["selection_id"] = json!("disposable-development-test");
    write_private(&fixture.selection, &selected);
    let original = fixture.directory.join("original-store");
    fs::rename(&fixture.root, &original).unwrap();
    fs::create_dir(&fixture.root).unwrap();
    refuse(&fixture);
    assert!(UnixStream::connect(&fixture.socket).is_err());
}

#[test]
fn matching_intent_resumes_empty_store_but_rejects_ambiguous_intent() {
    let fixture = Fixture::new();
    let intent = json!({
        "version": 1,
        "selection_sha256": digest(&fs::read(&fixture.selection).unwrap()),
        "store_root": fixture.root
    });
    write_private(&fixture.directory.join("startup-intent.json"), &intent);
    // Simulate interruption after the durable intent and fresh store creation,
    // before the startup receipt or service subject exists.
    drop(
        lifecycle_store::SqliteLifecycleStore::open(
            &fixture.root,
            "development-test-issuer".into(),
        )
        .unwrap(),
    );
    let mut child = fixture.run();
    fixture.wait_ready(&mut child);
    assert!(fixture.directory.join("startup-receipt.json").exists());
    stop(&mut child);

    let fixture = Fixture::new();
    let mut wrong = intent;
    wrong["store_root"] = json!(fixture.root);
    write_private(&fixture.directory.join("startup-intent.json"), &wrong);
    refuse(&fixture);
    assert!(!fixture.root.exists());
}

#[test]
fn incompatible_schema_and_symlinked_root_refuse() {
    let fixture = Fixture::new();
    let mut child = fixture.run();
    fixture.wait_ready(&mut child);
    stop(&mut child);
    let database = fixture.root.join("lifecycle.sqlite");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.pragma_update(None, "user_version", 999).unwrap();
    drop(connection);
    refuse(&fixture);
    assert!(UnixStream::connect(&fixture.socket).is_err());

    let fixture = Fixture::new();
    let target = fixture.directory.join("other-root");
    fs::create_dir(&target).unwrap();
    std::os::unix::fs::symlink(&target, &fixture.root).unwrap();
    refuse(&fixture);
    assert!(!fixture.directory.join("startup-intent.json").exists());
}

#[test]
fn nonempty_unsealed_store_does_not_resume_as_an_empty_bootstrap() {
    let fixture = Fixture::new();
    let mut child = fixture.run();
    fixture.wait_ready(&mut child);
    stop(&mut child);
    fs::remove_file(fixture.directory.join("startup-receipt.json")).unwrap();
    refuse(&fixture);
    assert!(UnixStream::connect(&fixture.socket).is_err());
}
