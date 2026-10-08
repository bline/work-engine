use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use lifecycle_client::{
    ClientError, ObservationTransport, ReadClient, SubjectRef, UnixSocketTransport, WaitHintV1,
};
use lifecycle_core::TransitionId;
use lifecycle_store::SqliteLifecycleStore;
use lifecycle_wire::{
    CursorV1, FieldV1, LifecycleSnapshotV1, TransitionStageV1, WaitResultV1, WaitTargetV1,
};
use rusqlite::{Connection, OpenFlags};
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

struct GapOnceTransport {
    inner: UnixSocketTransport,
    gap_seen: Arc<AtomicBool>,
}
impl ObservationTransport for GapOnceTransport {
    fn snapshot(
        &self,
        subject: &SubjectRef,
        target: Option<&WaitTargetV1>,
        budget: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        self.inner.snapshot(subject, target, budget)
    }
    fn wait_hint(
        &self,
        subject: &SubjectRef,
        target: &WaitTargetV1,
        cursor: &CursorV1,
        budget: Duration,
    ) -> Result<WaitHintV1, ClientError> {
        if !self.gap_seen.load(Ordering::SeqCst) {
            let mut future = cursor.clone();
            future.commit_sequence = u64::MAX.to_string();
            let hint = self.inner.wait_hint(subject, target, &future, budget)?;
            assert_eq!(
                hint,
                WaitHintV1::CursorGap,
                "actual service must reject a future cursor"
            );
            self.gap_seen.store(true, Ordering::SeqCst);
            return Ok(hint);
        }
        self.inner.wait_hint(subject, target, cursor, budget)
    }
}

fn wait_path(path: &Path) {
    let started = Instant::now();
    while !path.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "missing {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_connect(path: &Path) {
    let started = Instant::now();
    loop {
        if UnixStream::connect(path).is_ok() {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "socket not accepting: {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn command(
    kind: &str,
    payload: Value,
    id: &str,
    grant: &str,
    revision: u64,
    context: &str,
) -> Value {
    let mut value = json!({
        "protocol_version":2,"principal_ref":"principal-one","command_id":id,
        "subject_id":"subject-one","context_generation":context,"build_id":"build-one",
        "proof_run_id":"proof-one","grant_ref":grant,"expected_revision":revision.to_string(),
        "kind":kind,"payload":payload,
    });
    let digest = CodecContract::LifecycleCommandV2
        .digest_json(&value)
        .unwrap()
        .hex();
    value["request_digest"] = json!(digest);
    value
}

fn send(socket: &Path, command: &Value, read_reply: bool) -> Option<Value> {
    let mut stream = UnixStream::connect(socket).unwrap();
    let bytes = serde_json::to_vec(&json!({"op":"command","command":command})).unwrap();
    stream.write_all(&bytes).unwrap();
    stream.write_all(b"\n").unwrap();
    stream.flush().unwrap();
    if !read_reply {
        return None;
    }
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).unwrap();
    Some(serde_json::from_str(&line).unwrap())
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
        .status()
        .unwrap();
    assert!(status.success());
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"))
        .join("debug/controlled_peer")
}

#[test]
fn disconnected_commands_drive_committed_text_transition_and_reconnect() {
    for early_result in [false, true] {
        disconnected_transition_case(early_result);
    }
}

fn disconnected_transition_case(early_result: bool) {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        fs::metadata(root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let socket = root.join("service.sock");
    let peer_socket = root.join("peer.sock");
    let ledger = root.join("peer-ledger.jsonl");
    let release = root.join("release");
    let db = root.join("store");
    let config_path = root.join("config.json");
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 120_000;
    let uid = fs::metadata(root).unwrap().uid();
    let peer_executable = peer_binary();
    let config = json!({
        "profile":"controlled","store_root":db,"socket_path":socket,"peer_socket_path":peer_socket,
        "verifier_executable":peer_executable,"verifier_ledger_path":ledger,
        "trusted_issuer":"issuer-one","principal":{"uid":uid,"principal_ref":"principal-one","producer_ref":"producer-one"},
        "subject_id":"subject-one","context_generation":"context-one","build_id":"build-one","proof_run_id":"proof-one",
        "grants":[
            {"grant_ref":"grant-enqueue","context_generation":"context-one","scope":"enqueue_input","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-replace","context_generation":"context-one","scope":"request_replacement","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-next","context_generation":"context-next","scope":"enqueue_input","expires_wall_ms":expires,"revision":0}
        ],"max_executions":2,"max_wait_ms":3000,
    });
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut peer = ChildGuard(
        Command::new(&peer_executable)
            .args([&peer_socket, &ledger, Path::new("context-next"), &release])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_path(&peer_socket);
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    let text = "exact controlled input Ω";
    let text_digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap()
        .hex();
    let enqueue = command(
        "enqueue_input",
        json!({"input_id":"input-one","producer_ref":"producer-one","text":text,"text_digest":text_digest}),
        "command-enqueue",
        "grant-enqueue",
        0,
        "context-one",
    );
    send(&socket, &enqueue, false);
    let started = Instant::now();
    while fs::read_to_string(&ledger).unwrap_or_default().is_empty() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "peer did not receive entered attempt"
        );
        thread::sleep(Duration::from_millis(10));
    }
    if early_result {
        fs::write(&release, b"go").unwrap();
        let client = ReadClient::new(UnixSocketTransport::new(&socket));
        let subject = SubjectRef::parse("subject-one").unwrap();
        let started = Instant::now();
        loop {
            let snapshot = client.snapshot(&subject, Duration::from_secs(1)).unwrap();
            if matches!(snapshot.delivery,FieldV1::Known(ref delivery)
                if delivery.outcome==lifecycle_wire::ExecutionOutcomeV1::Completed)
            {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "early peer result not committed"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }
    let replacement = command(
        "request_replacement",
        json!({"reason":"controlled checkpoint"}),
        "command-replace",
        "grant-replace",
        if early_result { 2 } else { 1 },
        "context-one",
    );
    let reply = send(&socket, &replacement, true).unwrap();
    assert_eq!(reply["accepted"], true, "{reply}");
    let transition = reply["outcome_ref"].as_str().unwrap().to_owned();
    let subject = SubjectRef::parse("subject-one").unwrap();
    let target = WaitTargetV1::TransitionStage {
        transition_id: transition.clone(),
        stage: TransitionStageV1::Reconciled,
    };
    let result = if early_result {
        ReadClient::new(UnixSocketTransport::new(&socket)).wait(
            &subject,
            &target,
            Duration::from_secs(8),
        )
    } else {
        let gap_seen = Arc::new(AtomicBool::new(false));
        let wait_gap = gap_seen.clone();
        let wait_socket = socket.clone();
        let wait_subject = subject.clone();
        let waiter = thread::spawn(move || {
            ReadClient::new(GapOnceTransport {
                inner: UnixSocketTransport::new(wait_socket),
                gap_seen: wait_gap,
            })
            .wait(&wait_subject, &target, Duration::from_secs(8))
        });
        let started = Instant::now();
        while !gap_seen.load(Ordering::SeqCst) {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "actual service did not return cursor_gap"
            );
            thread::sleep(Duration::from_millis(5));
        }
        fs::write(&release, b"go").unwrap();
        waiter.join().unwrap()
    };
    let observed = match result {
        WaitResultV1::Observed(value) => value,
        other => panic!("transition was not observed: {other:?}"),
    };
    assert!(matches!(observed.transition,FieldV1::Known(ref value) if value.completed));
    assert_eq!(observed.context_generation, "context-next");
    let before = observed.cursor.clone();
    assert_eq!(fs::read_to_string(&ledger).unwrap().lines().count(), 1);
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    let store = SqliteLifecycleStore::open(&db, "issuer-one".into()).unwrap();
    let receipt = store
        .committed_text_for_transition(&TransitionId::parse(transition).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(
        store
            .verifier_result(&TransitionId::parse(reply["outcome_ref"].as_str().unwrap()).unwrap())
            .unwrap(),
        Some(true)
    );
    let verifier = store
        .verifier_report(&TransitionId::parse(reply["outcome_ref"].as_str().unwrap()).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(verifier.primary_error, "none");
    assert_eq!(verifier.disposition, "Resolved");
    assert_eq!(verifier.exit_code, Some(0));
    let ledger_record: Value =
        serde_json::from_str(fs::read_to_string(&ledger).unwrap().lines().next().unwrap()).unwrap();
    assert_eq!(
        receipt.final_text,
        ledger_record["final_text"].as_str().unwrap()
    );
    assert_eq!(
        receipt.source_id,
        ledger_record["source_id"].as_str().unwrap()
    );
    let recovery = store.load_recovery().unwrap();
    assert_eq!(recovery.unresolved_attempts, 0);
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
    let after = ReadClient::new(UnixSocketTransport::new(&socket))
        .reconnect(&subject, &before, Duration::from_secs(3))
        .unwrap();
    assert_eq!(after.context_generation, "context-next");
    assert_eq!(after.cursor.store_id, before.store_id);
    assert_eq!(fs::read_to_string(&ledger).unwrap().lines().count(), 1);
    assert!(service.0.try_wait().unwrap().is_none());
    assert!(peer.0.try_wait().unwrap().is_none());
}

#[test]
fn two_inputs_remain_fifo_while_first_entered_attempt_is_held() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.join("service.sock");
    let peer_socket = root.join("peer.sock");
    let ledger = root.join("ledger.jsonl");
    let release = root.join("release");
    let db = root.join("store");
    let config_path = root.join("config.json");
    let peer_executable = peer_binary();
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 120_000;
    let config = json!({"profile":"controlled","store_root":db,"socket_path":socket,"peer_socket_path":peer_socket,
        "verifier_executable":peer_executable,"verifier_ledger_path":ledger,"trusted_issuer":"issuer-one",
        "principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal-one","producer_ref":"producer-one"},
        "subject_id":"subject-one","context_generation":"context-one","build_id":"build-one","proof_run_id":"proof-one",
        "grants":[{"grant_ref":"grant-enqueue","context_generation":"context-one","scope":"enqueue_input","expires_wall_ms":expires,"revision":0}],
        "max_executions":2,"max_wait_ms":100});
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut peer = ChildGuard(
        Command::new(&peer_executable)
            .args([&peer_socket, &ledger, Path::new("context-next"), &release])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_path(&peer_socket);
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    let mk_enqueue = |id: &str, input: &str, revision: u64| {
        let text = format!("text for {input}");
        let digest = CodecContract::LifecycleTextInputV1
            .digest_binary(text.as_bytes())
            .unwrap()
            .hex();
        command(
            "enqueue_input",
            json!({"input_id":input,"producer_ref":"producer-one","text":text,"text_digest":digest}),
            id,
            "grant-enqueue",
            revision,
            "context-one",
        )
    };
    let first_command = mk_enqueue("command-first", "input-first", 0);
    let first = send(&socket, &first_command, true).unwrap();
    assert_eq!(first["accepted"], true);
    let started = Instant::now();
    while fs::read_to_string(&ledger)
        .unwrap_or_default()
        .lines()
        .count()
        != 1
    {
        assert!(started.elapsed() < Duration::from_secs(10));
        thread::sleep(Duration::from_millis(10));
    }
    let second = send(
        &socket,
        &mk_enqueue("command-second", "input-second", 1),
        true,
    )
    .unwrap();
    assert_eq!(second["accepted"], true);
    assert_eq!(fs::read_to_string(&ledger).unwrap().lines().count(), 1);
    // Both execution/result slots are held, but an authenticated exact replay
    // must resolve from S1 without asking S2 for fresh capacity.
    assert_eq!(send(&socket, &first_command, true).unwrap(), first);
    let client = ReadClient::new(UnixSocketTransport::new(&socket));
    let pending_target = WaitTargetV1::DeliveryOutcome {
        delivery_id: second["outcome_ref"].as_str().unwrap().into(),
        outcome: lifecycle_wire::ExecutionOutcomeV1::Completed,
    };
    let wait_started = Instant::now();
    assert!(matches!(
        client.wait(
            &SubjectRef::parse("subject-one").unwrap(),
            &pending_target,
            Duration::from_millis(550),
        ),
        WaitResultV1::DeadlineExpired { last_seen: Some(_) }
    ));
    assert!(wait_started.elapsed() >= Duration::from_millis(350));
    fs::write(&release, b"go").unwrap();
    let target = WaitTargetV1::DeliveryOutcome {
        delivery_id: second["outcome_ref"].as_str().unwrap().into(),
        outcome: lifecycle_wire::ExecutionOutcomeV1::Completed,
    };
    let observed = match client.wait(
        &SubjectRef::parse("subject-one").unwrap(),
        &target,
        Duration::from_secs(8),
    ) {
        WaitResultV1::Observed(value) => value,
        other => panic!("second input did not complete: {other:?}"),
    };
    assert!(
        matches!(observed.delivery,FieldV1::Known(ref value) if value.delivery_id==second["outcome_ref"].as_str().unwrap())
    );
    let ledger_lines = fs::read_to_string(&ledger).unwrap();
    let ids: Vec<String> = ledger_lines
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line).unwrap()["input_id"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(ids, ["input-first", "input-second"]);
    assert!(service.0.try_wait().unwrap().is_none());
    assert!(peer.0.try_wait().unwrap().is_none());
}

#[test]
fn replacement_uses_newest_entered_attempt_result_not_older_completed_text() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.join("service.sock");
    let peer_socket = root.join("peer.sock");
    let ledger = root.join("ledger.jsonl");
    let release = root.join("peer.release");
    let db = root.join("store");
    let config_path = root.join("config.json");
    let peer_executable = peer_binary();
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 120_000;
    let config = json!({"profile":"controlled","store_root":db,"socket_path":socket,"peer_socket_path":peer_socket,
        "verifier_executable":peer_executable,"verifier_ledger_path":ledger,"trusted_issuer":"issuer-one",
        "principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal-one","producer_ref":"producer-one"},
        "subject_id":"subject-one","context_generation":"context-one","build_id":"build-one","proof_run_id":"proof-one",
        "grants":[
            {"grant_ref":"grant-enqueue","context_generation":"context-one","scope":"enqueue_input","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-replace","context_generation":"context-one","scope":"request_replacement","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-next","context_generation":"context-next","scope":"enqueue_input","expires_wall_ms":expires,"revision":0}],
        "max_executions":1,"max_wait_ms":100});
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let peer = ChildGuard(
        Command::new(&peer_executable)
            .args([&peer_socket, &ledger, Path::new("context-next"), &release])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_path(&peer_socket);
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    let enqueue = |id: &str, input: &str, revision: u64| {
        let text = format!("continuation from {input}");
        let digest = CodecContract::LifecycleTextInputV1
            .digest_binary(text.as_bytes())
            .unwrap()
            .hex();
        command(
            "enqueue_input",
            json!({"input_id":input,"producer_ref":"producer-one","text":text,"text_digest":digest}),
            id,
            "grant-enqueue",
            revision,
            "context-one",
        )
    };
    let first = send(&socket, &enqueue("command-old", "input-old", 0), true).unwrap();
    assert_eq!(first["accepted"], true);
    let started = Instant::now();
    while fs::read_to_string(&ledger)
        .unwrap_or_default()
        .lines()
        .count()
        < 1
    {
        assert!(started.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    fs::write(&release, b"go").unwrap();
    let client = ReadClient::new(UnixSocketTransport::new(&socket));
    let subject = SubjectRef::parse("subject-one").unwrap();
    let first_target = WaitTargetV1::DeliveryOutcome {
        delivery_id: first["outcome_ref"].as_str().unwrap().into(),
        outcome: lifecycle_wire::ExecutionOutcomeV1::Completed,
    };
    assert!(matches!(
        client.wait(&subject, &first_target, Duration::from_secs(5)),
        WaitResultV1::Observed(_)
    ));
    fs::remove_file(&release).unwrap();
    let second = send(&socket, &enqueue("command-new", "input-new", 2), true).unwrap();
    assert_eq!(second["accepted"], true);
    let started = Instant::now();
    while fs::read_to_string(&ledger)
        .unwrap_or_default()
        .lines()
        .count()
        < 2
    {
        assert!(started.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    let replacement = command(
        "request_replacement",
        json!({"reason":"newest entered attempt is predecessor"}),
        "command-replace-newest",
        "grant-replace",
        3,
        "context-one",
    );
    let response = send(&socket, &replacement, true).unwrap();
    assert_eq!(response["accepted"], true, "{response}");
    let transition_id = response["outcome_ref"].as_str().unwrap().to_owned();
    let pending = client.snapshot(&subject, Duration::from_secs(1)).unwrap();
    assert!(
        matches!(pending.transition, FieldV1::Known(ref value) if value.transition_id==transition_id && value.stage==TransitionStageV1::Quiescing)
    );
    fs::write(&release, b"go").unwrap();
    let target = WaitTargetV1::TransitionStage {
        transition_id: transition_id.clone(),
        stage: TransitionStageV1::Reconciled,
    };
    assert!(matches!(
        client.wait(&subject, &target, Duration::from_secs(8)),
        WaitResultV1::Observed(_)
    ));
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    let events: Vec<Value> = fs::read_to_string(&ledger)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 2);
    let store = SqliteLifecycleStore::open(&db, "issuer-one".into()).unwrap();
    let basis = store
        .committed_text_for_transition(&TransitionId::parse(transition_id).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(basis.source_id, events[1]["source_id"].as_str().unwrap());
    assert_ne!(basis.source_id, events[0]["source_id"].as_str().unwrap());
    assert!(peer.0.id() > 0);
}

#[test]
fn final_return_after_grant_expiry_still_settles_entered_attempt() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.join("service.sock");
    let peer_socket = root.join("peer.sock");
    let ledger = root.join("ledger.jsonl");
    let release = root.join("release");
    let peer_executable = peer_binary();
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 2_000;
    let config = json!({"profile":"controlled","store_root":root.join("store"),"socket_path":socket,"peer_socket_path":peer_socket,
        "verifier_executable":peer_executable,"verifier_ledger_path":ledger,"trusted_issuer":"issuer-one",
        "principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal-one","producer_ref":"producer-one"},
        "subject_id":"subject-one","context_generation":"context-one","build_id":"build-one","proof_run_id":"proof-one",
        "grants":[{"grant_ref":"grant-enqueue","context_generation":"context-one","scope":"enqueue_input","expires_wall_ms":expires,"revision":0}],
        "max_executions":1,"max_wait_ms":3000});
    let config_path = root.join("config.json");
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut peer = ChildGuard(
        Command::new(&peer_executable)
            .args([&peer_socket, &ledger, Path::new("context-next"), &release])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_path(&peer_socket);
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    let text = "return after expiry";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap()
        .hex();
    let enqueue = command(
        "enqueue_input",
        json!({"input_id":"input-expiry","producer_ref":"producer-one","text":text,"text_digest":digest}),
        "command-expiry",
        "grant-enqueue",
        0,
        "context-one",
    );
    let response = send(&socket, &enqueue, true).unwrap();
    assert_eq!(response["accepted"], true);
    let started = Instant::now();
    while fs::read_to_string(&ledger)
        .unwrap_or_default()
        .lines()
        .count()
        != 1
    {
        assert!(started.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(10));
    }
    while SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        <= expires
    {
        thread::sleep(Duration::from_millis(5));
    }
    fs::write(&release, b"go").unwrap();
    let target = WaitTargetV1::DeliveryOutcome {
        delivery_id: response["outcome_ref"].as_str().unwrap().into(),
        outcome: lifecycle_wire::ExecutionOutcomeV1::Completed,
    };
    let client = ReadClient::new(UnixSocketTransport::new(&socket));
    assert!(matches!(
        client.wait(
            &SubjectRef::parse("subject-one").unwrap(),
            &target,
            Duration::from_secs(5)
        ),
        WaitResultV1::Observed(_)
    ));
    assert!(service.0.try_wait().unwrap().is_none());
    assert!(peer.0.try_wait().unwrap().is_none());
}

#[test]
fn verifier_child_failure_is_durable_and_never_claims_successor_ready() {
    for mode in ["validation", "launch", "deadline", "cleanup_timeout"] {
        let temp = tempdir().unwrap();
        let root = temp.path();
        fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = root.join("service.sock");
        let peer_socket = root.join("peer.sock");
        let ledger = root.join("ledger.jsonl");
        let release = root.join("release");
        let db = root.join("store");
        let peer_executable = peer_binary();
        let barrier = root.join("barrier");
        if matches!(mode, "deadline" | "cleanup_timeout") {
            fs::create_dir(&barrier).unwrap();
            for point in [
                "before_entry_commit",
                "after_entry_commit",
                "after_result_commit",
                "verifier_before_launch",
                "verifier_after_spawn",
            ] {
                fs::write(barrier.join(format!("{point}.release")), b"go").unwrap();
            }
            fs::write(barrier.join("verifier_hold.enabled"), b"hold").unwrap();
        }
        let verifier_executable = if mode == "launch" {
            root.join("missing-verifier")
        } else {
            peer_executable.clone()
        };
        let verifier_ledger = if mode == "validation" {
            root.join("missing-ledger")
        } else {
            ledger.clone()
        };
        let expires = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
            + 120_000;
        let config = json!({"profile":"controlled","store_root":db,"socket_path":socket,"peer_socket_path":peer_socket,
        "verifier_executable":verifier_executable,"verifier_ledger_path":verifier_ledger,
        "proof_barrier_dir":if matches!(mode,"deadline"|"cleanup_timeout") {Some(barrier.clone())} else {None},
        "verifier_proof_ms":if mode=="cleanup_timeout" {Some(1)} else {None},
        "verifier_cleanup_ms":if mode=="cleanup_timeout" {Some(1)} else {None},
        "trusted_issuer":"issuer-one","principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal-one","producer_ref":"producer-one"},
        "subject_id":"subject-one","context_generation":"context-one","build_id":"build-one","proof_run_id":"proof-one",
        "grants":[
            {"grant_ref":"grant-enqueue","context_generation":"context-one","scope":"enqueue_input","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-replace","context_generation":"context-one","scope":"request_replacement","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-next","context_generation":"context-next","scope":"enqueue_input","expires_wall_ms":expires,"revision":0}],
        "max_executions":1,"max_wait_ms":3000});
        let config_path = root.join("config.json");
        fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
        let mut peer = ChildGuard(
            Command::new(&peer_executable)
                .args([&peer_socket, &ledger, Path::new("context-next"), &release])
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        wait_path(&peer_socket);
        let mut service = ChildGuard(
            Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
                .arg(&config_path)
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        wait_connect(&socket);
        let text = "verify me";
        let digest = CodecContract::LifecycleTextInputV1
            .digest_binary(text.as_bytes())
            .unwrap()
            .hex();
        let enqueue = command(
            "enqueue_input",
            json!({"input_id":"input-fail","producer_ref":"producer-one","text":text,"text_digest":digest}),
            "command-fail",
            "grant-enqueue",
            0,
            "context-one",
        );
        assert_eq!(send(&socket, &enqueue, true).unwrap()["accepted"], true);
        let started = Instant::now();
        while fs::read_to_string(&ledger)
            .unwrap_or_default()
            .lines()
            .count()
            != 1
        {
            assert!(started.elapsed() < Duration::from_secs(5));
            thread::sleep(Duration::from_millis(10));
        }
        let replacement = command(
            "request_replacement",
            json!({"reason":"verify failure"}),
            "command-replace-fail",
            "grant-replace",
            1,
            "context-one",
        );
        let reply = send(&socket, &replacement, true).unwrap();
        assert_eq!(reply["accepted"], true);
        let transition = reply["outcome_ref"].as_str().unwrap().to_owned();
        fs::write(&release, b"go").unwrap();
        let target = WaitTargetV1::TransitionStage {
            transition_id: transition.clone(),
            stage: TransitionStageV1::RecoveryRequired,
        };
        let client = ReadClient::new(UnixSocketTransport::new(&socket));
        let observed = match client.wait(
            &SubjectRef::parse("subject-one").unwrap(),
            &target,
            Duration::from_secs(9),
        ) {
            WaitResultV1::Observed(value) => value,
            other => panic!("verifier failure not observed: {other:?}"),
        };
        assert_eq!(observed.context_generation, "context-one");
        assert!(matches!(
            observed.current_admission,
            lifecycle_wire::AdmissionV1::Fenced { .. }
        ));
        if matches!(mode, "deadline" | "cleanup_timeout") {
            let pid = fs::read_to_string(barrier.join("verifier_started.reached"))
                .unwrap()
                .parse::<u32>()
                .unwrap();
            let proc_path = PathBuf::from(format!("/proc/{pid}"));
            let started = Instant::now();
            while proc_path.exists() {
                assert!(
                    started.elapsed() < Duration::from_secs(5),
                    "verifier child remained live after cleanup: {mode}"
                );
                thread::sleep(Duration::from_millis(5));
            }
        }
        service.0.kill().unwrap();
        service.0.wait().unwrap();
        let store = SqliteLifecycleStore::open(&db, "issuer-one".into()).unwrap();
        assert_eq!(
            store
                .verifier_result(&TransitionId::parse(transition).unwrap())
                .unwrap(),
            Some(false)
        );
        let report = store
            .verifier_report(&TransitionId::parse(reply["outcome_ref"].as_str().unwrap()).unwrap())
            .unwrap()
            .unwrap();
        let (primary, activation, disposition, exit_code, late) = match mode {
            "validation" => (
                "verification_failed",
                "Confirmed",
                "Resolved",
                Some(1),
                false,
            ),
            "launch" => ("launch_rejected", "NotAttempted", "Rejected", None, false),
            "deadline" => (
                "proof_deadline_expired",
                "Confirmed",
                "ResolvedLate",
                None,
                true,
            ),
            "cleanup_timeout" => ("cleanup_unresolved", "Confirmed", "TimedOut", None, false),
            _ => unreachable!(),
        };
        assert_eq!(report.primary_error, primary, "{mode}: {report:?}");
        assert_eq!(report.activation, activation, "{mode}: {report:?}");
        assert_eq!(report.disposition, disposition, "{mode}: {report:?}");
        assert_eq!(report.exit_code, exit_code, "{mode}: {report:?}");
        assert_eq!(report.late_at_close, late, "{mode}: {report:?}");
        if matches!(mode, "deadline" | "cleanup_timeout") {
            assert!(report.unsafe_at_close, "{mode}: {report:?}");
        }
        assert_eq!(store.load_recovery().unwrap().unresolved_attempts, 0);
        assert_eq!(fs::read_to_string(&ledger).unwrap().lines().count(), 1);
        assert!(peer.0.try_wait().unwrap().is_none());
    }
}

fn verifier_launch_cut(cut: &str, graceful: bool) {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let barrier = root.join("barrier");
    fs::create_dir(&barrier).unwrap();
    for point in [
        "before_entry_commit",
        "after_entry_commit",
        "after_result_commit",
        "verifier_before_launch",
        "verifier_after_spawn",
    ] {
        if point != cut {
            fs::write(barrier.join(format!("{point}.release")), b"go").unwrap();
        }
    }
    let socket = root.join("service.sock");
    let peer_socket = root.join("peer.sock");
    let ledger = root.join("ledger.jsonl");
    let peer_release = root.join("peer.release");
    let db = root.join("store");
    let config_path = root.join("config.json");
    let peer_executable = peer_binary();
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 120_000;
    let config = json!({"profile":"controlled","store_root":db,"socket_path":socket,"peer_socket_path":peer_socket,
        "verifier_executable":peer_executable,"verifier_ledger_path":ledger,"proof_barrier_dir":barrier,
        "trusted_issuer":"issuer-one","principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal-one","producer_ref":"producer-one"},
        "subject_id":"subject-one","context_generation":"context-one","build_id":"build-one","proof_run_id":"proof-one",
        "grants":[
            {"grant_ref":"grant-enqueue","context_generation":"context-one","scope":"enqueue_input","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-replace","context_generation":"context-one","scope":"request_replacement","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-next","context_generation":"context-next","scope":"enqueue_input","expires_wall_ms":expires,"revision":0}],
        "max_executions":1,"max_wait_ms":3000});
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let peer = ChildGuard(
        Command::new(&peer_executable)
            .args([
                &peer_socket,
                &ledger,
                Path::new("context-next"),
                &peer_release,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_path(&peer_socket);
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    let text = "verifier launch barrier";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap()
        .hex();
    let enqueue = command(
        "enqueue_input",
        json!({"input_id":"input-launch","producer_ref":"producer-one","text":text,"text_digest":digest}),
        "command-launch",
        "grant-enqueue",
        0,
        "context-one",
    );
    assert_eq!(send(&socket, &enqueue, true).unwrap()["accepted"], true);
    let started = Instant::now();
    while fs::read_to_string(&ledger)
        .unwrap_or_default()
        .lines()
        .count()
        != 1
    {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "peer entry not recorded"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let replacement = command(
        "request_replacement",
        json!({"reason":"verifier launch cut"}),
        "command-replace-launch",
        "grant-replace",
        1,
        "context-one",
    );
    let response = send(&socket, &replacement, true).unwrap();
    assert_eq!(response["accepted"], true, "{response}");
    let transition = response["outcome_ref"].as_str().unwrap().to_owned();
    fs::write(&peer_release, b"go").unwrap();
    let reached = barrier.join(format!("{cut}.reached"));
    let started = Instant::now();
    while !reached.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(6),
            "verifier launch cut not reached: {cut}"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let old_child_pid = if cut == "verifier_after_spawn" {
        let marker = barrier.join("verifier_started.reached");
        let started = Instant::now();
        while !marker.exists() {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "verifier child did not start"
            );
            thread::sleep(Duration::from_millis(5));
        }
        Some(fs::read_to_string(marker).unwrap().parse::<u32>().unwrap())
    } else {
        None
    };
    if graceful {
        assert!(
            Command::new("kill")
                .arg("-TERM")
                .arg(service.0.id().to_string())
                .status()
                .unwrap()
                .success()
        );
        // This marker is emitted by the real signal loop only after it has
        // closed both execution and verifier launch gates.
        let closed = barrier.join("shutdown_gate_closed.reached");
        let started = Instant::now();
        while !closed.exists() {
            assert!(
                service.0.try_wait().unwrap().is_none(),
                "service exited before shutdown gate marker"
            );
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "signal loop did not close launch gates"
            );
            thread::sleep(Duration::from_millis(5));
        }
        fs::write(barrier.join(format!("{cut}.release")), b"go").unwrap();
        let status = service.0.wait().unwrap();
        assert!(status.success(), "graceful verifier close failed: {status}");
        if let Some(pid) = old_child_pid {
            let proc_path = PathBuf::from(format!("/proc/{pid}"));
            let started = Instant::now();
            while proc_path.exists() {
                assert!(started.elapsed() < Duration::from_secs(5));
                thread::sleep(Duration::from_millis(5));
            }
        }
        let mut store = SqliteLifecycleStore::open(&db, "issuer-one".into()).unwrap();
        let id = TransitionId::parse(transition).unwrap();
        assert!(store.committed_text_for_transition(&id).unwrap().is_some());
        assert_eq!(store.verifier_result(&id).unwrap(), Some(false));
        let report = store.verifier_report(&id).unwrap().unwrap();
        assert_eq!(report.activation, "NotAttempted");
        if cut == "verifier_before_launch" {
            assert_eq!(report.primary_error, "launch_rejected");
            assert!(!barrier.join("verifier_started.reached").exists());
        } else {
            assert_eq!(report.primary_error, "activation_uncertain");
        }
        let snapshot = store
            .subject_projection(
                &lifecycle_core::SubjectId::parse("subject-one").unwrap(),
                None,
                Some(&id),
            )
            .unwrap();
        assert_ne!(snapshot.transition.unwrap().stage, "reconciled");
        assert_eq!(fs::read_to_string(&ledger).unwrap().lines().count(), 1);
        assert!(peer.0.id() > 0);
        return;
    }
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    if let Some(pid) = old_child_pid {
        let proc_path = PathBuf::from(format!("/proc/{pid}"));
        let started = Instant::now();
        while proc_path.exists() {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "unactivated verifier child outlived killed service"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }
    let store = SqliteLifecycleStore::open(&db, "issuer-one".into()).unwrap();
    let id = TransitionId::parse(transition.clone()).unwrap();
    assert!(store.committed_text_for_transition(&id).unwrap().is_some());
    assert_eq!(store.verifier_result(&id).unwrap(), None);
    assert_eq!(store.counts().unwrap().2, 1);
    drop(store);
    fs::write(barrier.join(format!("{cut}.release")), b"go").unwrap();
    service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    let target = WaitTargetV1::TransitionStage {
        transition_id: transition.clone(),
        stage: TransitionStageV1::Reconciled,
    };
    let observed = ReadClient::new(UnixSocketTransport::new(&socket)).wait(
        &SubjectRef::parse("subject-one").unwrap(),
        &target,
        Duration::from_secs(6),
    );
    assert!(
        matches!(observed, WaitResultV1::Observed(_)),
        "{observed:?}"
    );
    assert_eq!(fs::read_to_string(&ledger).unwrap().lines().count(), 1);
    assert!(service.0.try_wait().unwrap().is_none());
    assert!(peer.0.id() > 0);
}

#[test]
fn verifier_launch_barriers_survive_service_kill_without_orphan_or_resend() {
    for cut in ["verifier_before_launch", "verifier_after_spawn"] {
        verifier_launch_cut(cut, false);
    }
}

#[test]
fn sigterm_fences_verifier_launch_and_activation_before_actor_unblocks() {
    for cut in ["verifier_before_launch", "verifier_after_spawn"] {
        verifier_launch_cut(cut, true);
    }
}

#[test]
fn sigterm_waits_for_active_verifier_custody_beyond_old_socket_deadline() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let barrier = root.join("barrier");
    fs::create_dir(&barrier).unwrap();
    for point in [
        "before_entry_commit",
        "after_entry_commit",
        "after_result_commit",
        "verifier_before_launch",
        "verifier_after_spawn",
    ] {
        fs::write(barrier.join(format!("{point}.release")), b"go").unwrap();
    }
    fs::write(barrier.join("verifier_hold.enabled"), b"hold").unwrap();
    let socket = root.join("service.sock");
    let peer_socket = root.join("peer.sock");
    let ledger = root.join("ledger.jsonl");
    let peer_release = root.join("peer.release");
    let db = root.join("store");
    let config_path = root.join("config.json");
    let peer_executable = peer_binary();
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 120_000;
    let config = json!({"profile":"controlled","store_root":db,"socket_path":socket,
        "peer_socket_path":peer_socket,"verifier_executable":peer_executable,
        "verifier_ledger_path":ledger,"proof_barrier_dir":barrier,
        "verifier_proof_ms":32_000,"verifier_cleanup_ms":36_000,
        "trusted_issuer":"issuer-one","principal":{"uid":fs::metadata(root).unwrap().uid(),
            "principal_ref":"principal-one","producer_ref":"producer-one"},
        "subject_id":"subject-one","context_generation":"context-one","build_id":"build-one",
        "proof_run_id":"proof-one","grants":[
            {"grant_ref":"grant-enqueue","context_generation":"context-one","scope":"enqueue_input","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-replace","context_generation":"context-one","scope":"request_replacement","expires_wall_ms":expires,"revision":0},
            {"grant_ref":"grant-next","context_generation":"context-next","scope":"enqueue_input","expires_wall_ms":expires,"revision":0}],
        "max_executions":1,"max_wait_ms":3000});
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut peer = ChildGuard(
        Command::new(&peer_executable)
            .args([
                &peer_socket,
                &ledger,
                Path::new("context-next"),
                &peer_release,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_path(&peer_socket);
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    let text = "long verifier custody";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap()
        .hex();
    let enqueue = command(
        "enqueue_input",
        json!({"input_id":"input-long-close","producer_ref":"producer-one","text":text,"text_digest":digest}),
        "command-long-close",
        "grant-enqueue",
        0,
        "context-one",
    );
    assert_eq!(send(&socket, &enqueue, true).unwrap()["accepted"], true);
    let started = Instant::now();
    while fs::read_to_string(&ledger)
        .unwrap_or_default()
        .lines()
        .count()
        != 1
    {
        assert!(started.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(5));
    }
    let replacement = command(
        "request_replacement",
        json!({"reason":"long verifier custody"}),
        "command-replace-long-close",
        "grant-replace",
        1,
        "context-one",
    );
    let response = send(&socket, &replacement, true).unwrap();
    assert_eq!(response["accepted"], true);
    let transition = TransitionId::parse(response["outcome_ref"].as_str().unwrap()).unwrap();
    fs::write(&peer_release, b"go").unwrap();
    wait_path(&barrier.join("verifier_activated.reached"));
    let verifier_pid = fs::read_to_string(barrier.join("verifier_started.reached"))
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let signalled_at = Instant::now();
    assert!(
        Command::new("kill")
            .arg("-TERM")
            .arg(service.0.id().to_string())
            .status()
            .unwrap()
            .success()
    );
    wait_path(&barrier.join("shutdown_gate_closed.reached"));
    let status = loop {
        if let Some(status) = service.0.try_wait().unwrap() {
            break status;
        }
        assert!(signalled_at.elapsed() < Duration::from_secs(45));
        thread::sleep(Duration::from_millis(10));
    };
    assert!(signalled_at.elapsed() >= Duration::from_secs(30));
    assert!(
        status.success(),
        "active verifier close lost custody: {status}"
    );
    assert!(!PathBuf::from(format!("/proc/{verifier_pid}")).exists());
    let store = SqliteLifecycleStore::open(&db, "issuer-one".into()).unwrap();
    assert_eq!(store.verifier_result(&transition).unwrap(), Some(false));
    let report = store.verifier_report(&transition).unwrap().unwrap();
    assert_eq!(report.primary_error, "proof_deadline_expired");
    assert!(report.unsafe_at_close);
    assert!(report.late_at_close);
    assert_eq!(fs::read_to_string(&ledger).unwrap().lines().count(), 1);
    let connection = Connection::open_with_flags(
        db.join("lifecycle.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let close_count: u64 = connection
        .query_row("SELECT COUNT(*) FROM service_closes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(close_count, 1);
    assert!(peer.0.try_wait().unwrap().is_none());
}

#[test]
fn sigterm_fences_executor_submit_after_durable_entry_commit() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let barrier = root.join("barrier");
    fs::create_dir(&barrier).unwrap();
    fs::write(barrier.join("before_entry_commit.release"), b"go").unwrap();
    let socket = root.join("service.sock");
    let peer_socket = root.join("peer.sock");
    let ledger = root.join("ledger.jsonl");
    let db = root.join("store");
    let config_path = root.join("config.json");
    let peer_executable = peer_binary();
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 120_000;
    let config = json!({"profile":"controlled","store_root":db,"socket_path":socket,"peer_socket_path":peer_socket,
        "verifier_executable":peer_executable,"verifier_ledger_path":ledger,"proof_barrier_dir":barrier,
        "trusted_issuer":"issuer-one","principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal-one","producer_ref":"producer-one"},
        "subject_id":"subject-one","context_generation":"context-one","build_id":"build-one","proof_run_id":"proof-one",
        "grants":[{"grant_ref":"grant-enqueue","context_generation":"context-one","scope":"enqueue_input","expires_wall_ms":expires,"revision":0}],
        "max_executions":1,"max_wait_ms":100});
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let peer = ChildGuard(
        Command::new(&peer_executable)
            .args([&peer_socket, &ledger, Path::new("context-next")])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_path(&peer_socket);
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    let text = "entry committed before signal";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap()
        .hex();
    let enqueue = command(
        "enqueue_input",
        json!({"input_id":"input-signal-entry","producer_ref":"producer-one","text":text,"text_digest":digest}),
        "command-signal-entry",
        "grant-enqueue",
        0,
        "context-one",
    );
    send(&socket, &enqueue, false);
    wait_path(&barrier.join("after_entry_commit.reached"));
    assert!(
        Command::new("kill")
            .arg("-TERM")
            .arg(service.0.id().to_string())
            .status()
            .unwrap()
            .success()
    );
    wait_path(&barrier.join("shutdown_gate_closed.reached"));
    fs::write(barrier.join("after_entry_commit.release"), b"go").unwrap();
    let status = service.0.wait().unwrap();
    assert!(status.success(), "graceful close failed: {status}");
    assert!(fs::read_to_string(&ledger).unwrap().is_empty());
    let store = SqliteLifecycleStore::open(&db, "issuer-one".into()).unwrap();
    assert_eq!(store.counts().unwrap(), (1, 1, 1));
    let entry = store.load_recovery_entries().unwrap().pop().unwrap();
    assert_eq!(
        entry.disposition,
        lifecycle_store::RecoveryDisposition::EnteredUncertain
    );
    assert!(peer.0.id() > 0);
}

#[test]
fn held_real_effect_across_sigterm_retains_unsafe_close_and_never_resends() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = root.join("service.sock");
    let peer_socket = root.join("peer.sock");
    let ledger = root.join("ledger.jsonl");
    let release = root.join("peer.release");
    let db = root.join("store");
    let config_path = root.join("config.json");
    let peer_executable = peer_binary();
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 120_000;
    let config = json!({"profile":"controlled","store_root":db,"socket_path":socket,"peer_socket_path":peer_socket,
        "verifier_executable":peer_executable,"verifier_ledger_path":ledger,"trusted_issuer":"issuer-one",
        "principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal-one","producer_ref":"producer-one"},
        "subject_id":"subject-one","context_generation":"context-one","build_id":"build-one","proof_run_id":"proof-one",
        "grants":[{"grant_ref":"grant-enqueue","context_generation":"context-one","scope":"enqueue_input","expires_wall_ms":expires,"revision":0}],
        "max_executions":1,"max_wait_ms":1000});
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let peer = ChildGuard(
        Command::new(&peer_executable)
            .args([&peer_socket, &ledger, Path::new("context-next"), &release])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_path(&peer_socket);
    let mut service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    let text = "held across termination";
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap()
        .hex();
    let enqueue = command(
        "enqueue_input",
        json!({"input_id":"input-close","producer_ref":"producer-one","text":text,"text_digest":digest}),
        "command-close",
        "grant-enqueue",
        0,
        "context-one",
    );
    assert_eq!(send(&socket, &enqueue, true).unwrap()["accepted"], true);
    let started = Instant::now();
    while fs::read_to_string(&ledger)
        .unwrap_or_default()
        .lines()
        .count()
        != 1
    {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "peer did not enter before close"
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        Command::new("kill")
            .arg("-TERM")
            .arg(service.0.id().to_string())
            .status()
            .unwrap()
            .success()
    );
    let status = service.0.wait().unwrap();
    assert!(status.success(), "graceful service close failed: {status}");
    fs::write(&release, b"go").unwrap();
    let store = SqliteLifecycleStore::open(&db, "issuer-one".into()).unwrap();
    let entry = store.load_recovery_entries().unwrap().pop().unwrap();
    assert_eq!(
        entry.disposition,
        lifecycle_store::RecoveryDisposition::EnteredUncertain
    );
    let attempt = entry.attempt_id.unwrap();
    assert_eq!(store.counts().unwrap(), (1, 1, 1));
    drop(store);
    let connection = Connection::open_with_flags(
        db.join("lifecycle.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (joined,unresolved,pending,unsafe_at_close):(String,String,String,bool)=connection.query_row(
        "SELECT joined_json,unresolved_json,pending_results_json,unsafe_at_close FROM service_closes",[],
        |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
    ).unwrap();
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&joined).unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&unresolved).unwrap(),
        vec![attempt]
    );
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&pending).unwrap(),
        Vec::<String>::new()
    );
    assert!(unsafe_at_close);
    drop(connection);
    service = ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_lifecycle-service"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait_connect(&socket);
    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        fs::read_to_string(&ledger).unwrap().lines().count(),
        1,
        "entered effect resent after close"
    );
    assert!(service.0.try_wait().unwrap().is_none());
    assert!(peer.0.id() > 0);
}
