use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use lifecycle_store::SqliteLifecycleStore;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use work_engine_types::CodecContract;

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_connect(socket: &Path) {
    let started = Instant::now();
    while UnixStream::connect(socket).is_err() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "service did not bind"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn config(root: &Path, expires: i64, socket_name: &str, store_root: PathBuf) -> Value {
    json!({"profile":"controlled","store_root":store_root,"socket_path":root.join(socket_name),
        "peer_socket_path":root.join("absent-peer.sock"),"verifier_executable":"/bin/false",
        "verifier_ledger_path":root.join("absent-ledger"),"proof_barrier_dir":null,
        "trusted_issuer":"issuer-sec","principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal-sec","producer_ref":"producer-sec"},
        "subject_id":"subject-sec","context_generation":"context-sec","build_id":"build-sec","proof_run_id":"proof-sec",
        "grants":[
            {"grant_ref":"grant-enqueue","context_generation":"context-sec","scope":"enqueue_input","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-replace","context_generation":"context-sec","scope":"request_replacement","expires_wall_ms":expires,"revision":0}],
        "max_executions":1,"max_wait_ms":1000})
}

fn setup(expires: i64) -> (TempDir, PathBuf, PathBuf, ChildGuard) {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.join("service.sock");
    let db = root.join("store");
    let config_path = root.join("config.json");
    fs::write(
        &config_path,
        serde_json::to_vec(&config(root, expires, "service.sock", db.clone())).unwrap(),
    )
    .unwrap();
    let service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    (temp, socket, db, service)
}

fn send(socket: &Path, request: Value) -> Value {
    let mut stream = UnixStream::connect(socket).unwrap();
    stream
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    stream.write_all(b"\n").unwrap();
    stream.flush().unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

fn command(version: u16, kind: &str, payload: Value, id: &str, grant: &str) -> Value {
    let mut value = json!({"protocol_version":version,"principal_ref":"principal-sec","command_id":id,
        "subject_id":"subject-sec","context_generation":"context-sec","build_id":"build-sec","proof_run_id":"proof-sec",
        "grant_ref":grant,"expected_revision":"0","kind":kind,"payload":payload});
    let contract = if version == 1 {
        CodecContract::LifecycleCommandV1
    } else {
        CodecContract::LifecycleCommandV2
    };
    value["request_digest"] = json!(contract.digest_json(&value).unwrap().hex());
    value
}

fn wall_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[test]
fn authenticated_socket_refuses_v1_text_and_untrusted_v2_material_before_store() {
    let (_temp, socket, db, mut service) = setup(wall_now() + 120_000);
    let v1 = command(
        1,
        "enqueue_input",
        json!({"input_id":"input-one"}),
        "v1-enqueue",
        "grant-enqueue",
    );
    let response = send(&socket, json!({"op":"command","command":v1}));
    assert_eq!(response["code"], "unsupported_capability");
    let text = "secret text not for snapshot";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap()
        .hex();
    let v2 = command(
        2,
        "enqueue_input",
        json!({"input_id":"input-one","producer_ref":"forged-producer","text":text,"text_digest":digest}),
        "v2-forged",
        "grant-enqueue",
    );
    assert_eq!(
        send(&socket, json!({"op":"command","command":v2}))["code"],
        "semantic_invalidity"
    );
    let mut altered = command(
        2,
        "enqueue_input",
        json!({"input_id":"input-one","producer_ref":"producer-sec","text":text,"text_digest":digest}),
        "v2-altered",
        "grant-enqueue",
    );
    altered["payload"]["text"] = json!("tampered");
    assert_eq!(
        send(&socket, json!({"op":"command","command":altered}))["code"],
        "semantic_invalidity"
    );
    let wrong_principal = command(
        1,
        "request_replacement",
        json!({"reason":"okay"}),
        "wrong-principal",
        "grant-replace",
    );
    let mut wrong_principal = wrong_principal;
    wrong_principal["principal_ref"] = json!("someone-else");
    assert_eq!(
        send(&socket, json!({"op":"command","command":wrong_principal}))["code"],
        "semantic_invalidity"
    );
    assert_eq!(
        send(
            &socket,
            json!({"op":"snapshot","subject_id":"other-subject"})
        )["code"],
        "observation_unavailable"
    );
    let snapshot = send(&socket, json!({"op":"snapshot","subject_id":"subject-sec"}));
    assert_eq!(snapshot["protocol_version"], 1);
    assert!(!snapshot.to_string().contains(text));
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    let store = SqliteLifecycleStore::open(&db, "issuer-sec".into()).unwrap();
    assert_eq!(store.counts().unwrap(), (0, 0, 0));
}

#[test]
fn v1_replacement_and_expired_grant_have_exact_durable_replay() {
    let (_temp, socket, db, mut service) = setup(wall_now() - 1);
    let replacement = command(
        1,
        "request_replacement",
        json!({"reason":"expired"}),
        "v1-replace",
        "grant-replace",
    );
    let first = send(&socket, json!({"op":"command","command":replacement}));
    assert_eq!(first["accepted"], false);
    assert_eq!(first["rejection_code"], "expired_grant");
    let second = send(&socket, json!({"op":"command","command":replacement}));
    assert_eq!(first, second);
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    let store = SqliteLifecycleStore::open(&db, "issuer-sec".into()).unwrap();
    assert_eq!(store.counts().unwrap(), (1, 0, 0));
    drop(store);
    let (_temp, socket, db, mut service) = setup(wall_now() + 120_000);
    let replacement = command(
        1,
        "request_replacement",
        json!({"reason":"supported"}),
        "v1-replace",
        "grant-replace",
    );
    let accepted = send(&socket, json!({"op":"command","command":replacement}));
    assert_eq!(accepted["accepted"], true, "{accepted}");
    assert_eq!(accepted["protocol_version"], 2);
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    let store = SqliteLifecycleStore::open(&db, "issuer-sec".into()).unwrap();
    assert_eq!(store.counts().unwrap(), (1, 0, 0));
}

#[test]
fn second_binary_refuses_canonical_symlink_and_hardlink_store_aliases() {
    let (temp, _socket, db, mut first) = setup(wall_now() + 120_000);
    let root = temp.path();
    let alias = root.join("store-link");
    std::os::unix::fs::symlink(&db, &alias).unwrap();
    let config_path = root.join("alias.json");
    fs::write(
        &config_path,
        serde_json::to_vec(&config(root, wall_now() + 120_000, "alias.sock", alias)).unwrap(),
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
        .arg(&config_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!status.success());
    let config_path = root.join("same.json");
    fs::write(
        &config_path,
        serde_json::to_vec(&config(root, wall_now() + 120_000, "same.sock", db.clone())).unwrap(),
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
        .arg(&config_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!status.success());
    first.0.kill().unwrap();
    first.0.wait().unwrap();
    let other = root.join("other-store");
    fs::create_dir(&other).unwrap();
    fs::hard_link(db.join("lifecycle.sqlite"), other.join("lifecycle.sqlite")).unwrap();
    let config_path = root.join("hardlink.json");
    fs::write(
        &config_path,
        serde_json::to_vec(&config(root, wall_now() + 120_000, "hardlink.sock", other)).unwrap(),
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
        .arg(&config_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!status.success());
}

#[test]
fn grant_expiring_before_entry_blocks_dispatch_but_preserves_accepted_custody() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let db = root.join("store");
    let socket = root.join("service.sock");
    let barrier = root.join("barrier");
    fs::create_dir(&barrier).unwrap();
    let expires = wall_now() + 1_500;
    let mut service_config = config(root, expires, "service.sock", db.clone());
    service_config["proof_barrier_dir"] = json!(barrier);
    let config_path = root.join("config.json");
    fs::write(&config_path, serde_json::to_vec(&service_config).unwrap()).unwrap();
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    let text = "expires before entry";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap()
        .hex();
    let enqueue = command(
        2,
        "enqueue_input",
        json!({"input_id":"input-expire-entry","producer_ref":"producer-sec","text":text,"text_digest":digest}),
        "command-expire-entry",
        "grant-enqueue",
    );
    let request = json!({"op":"command","command":enqueue});
    let socket_for_sender = socket.clone();
    let sender = thread::spawn(move || send(&socket_for_sender, request));
    let started = Instant::now();
    while !barrier.join("before_entry_commit.reached").exists() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "entry barrier not reached"
        );
        thread::sleep(Duration::from_millis(5));
    }
    while wall_now() <= expires {
        thread::sleep(Duration::from_millis(5));
    }
    fs::write(barrier.join("before_entry_commit.release"), b"go").unwrap();
    let accepted = sender.join().unwrap();
    assert_eq!(accepted["accepted"], true, "{accepted}");
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    let store = SqliteLifecycleStore::open(&db, "issuer-sec".into()).unwrap();
    assert_eq!(store.counts().unwrap(), (1, 1, 0));
    assert_eq!(
        store.load_recovery_entries().unwrap()[0].disposition,
        lifecycle_store::RecoveryDisposition::PreparedNeedsGrant
    );
    drop(store);
    service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    assert_eq!(
        send(&socket, json!({"op":"snapshot","subject_id":"subject-sec"}))["protocol_version"],
        1
    );
    assert!(service.0.try_wait().unwrap().is_none());
}

#[test]
fn absent_peer_persists_primary_task_failure_without_settlement_or_replay() {
    let (_temp, socket, db, mut service) = setup(wall_now() + 120_000);
    let text = "peer is absent";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap()
        .hex();
    let enqueue = command(
        2,
        "enqueue_input",
        json!({"input_id":"input-peer-absent","producer_ref":"producer-sec","text":text,"text_digest":digest}),
        "command-peer-absent",
        "grant-enqueue",
    );
    let accepted = send(&socket, json!({"op":"command","command":enqueue}));
    assert_eq!(accepted["accepted"], true, "{accepted}");
    let connection = Connection::open_with_flags(
        db.join("lifecycle.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let started = Instant::now();
    let result = loop {
        let result:Option<(String,bool,Option<String>,Option<String>)>=connection.query_row(
            "SELECT termination,local_task_ended,observation_source,final_text_sha256 FROM task_results LIMIT 1",[],
            |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
        ).optional().unwrap();
        if let Some(result) = result {
            break result;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "owned task failure not persisted"
        );
        thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(result, ("observation_unavailable".into(), true, None, None));
    drop(connection);
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    let store = SqliteLifecycleStore::open(&db, "issuer-sec".into()).unwrap();
    assert_eq!(store.counts().unwrap(), (1, 1, 1));
    assert_eq!(
        store.load_recovery_entries().unwrap()[0].disposition,
        lifecycle_store::RecoveryDisposition::EnteredUncertain
    );
    assert_eq!(store.load_recovery().unwrap().unresolved_attempts, 1);
}

#[test]
fn missing_controlled_verifier_configuration_refuses_startup_before_store_or_socket() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let mut invalid = config(
        root,
        wall_now() + 120_000,
        "service.sock",
        root.join("store"),
    );
    invalid["verifier_executable"] = Value::Null;
    let config_path = root.join("config.json");
    fs::write(&config_path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
        .arg(&config_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!root.join("service.sock").exists());
    assert!(!root.join("store").exists());
}
