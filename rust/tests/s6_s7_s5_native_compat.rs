//! Installed native S5 path with the optional S6 semantic port disabled.

#[path = "support/native_binding.rs"]
mod native_binding;

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixStream,
    },
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use work_engine_types::CodecContract;

const PREDECESSOR: &str = "{\"checkpoint_ready\":true,\"predecessor_safe\":true,\"protocol_version\":1,\"source_frozen\":true,\"successor_context\":\"context:s5-next\",\"switch_authorized\":true}";

struct Simulator {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<Result<(), String>>>,
}

impl Simulator {
    fn start() -> Self {
        native_binding::checked();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let observed = requests.clone();
        let stopping = stop.clone();
        let worker = thread::spawn(move || {
            let outputs = [
                PREDECESSOR,
                "S5 successor retained the predecessor checkpoint.",
                "S5 successor domain work completed.",
            ];
            let deadline = Instant::now() + Duration::from_secs(60);
            while !stopping.load(Ordering::SeqCst) && Instant::now() < deadline {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let body = read_request(stream.try_clone().map_err(|e| e.to_string())?)?;
                        let count = {
                            let mut requests = observed.lock().map_err(|_| "capture poisoned")?;
                            requests.push(body);
                            requests.len()
                        };
                        let output = outputs.get(count - 1).ok_or("unexpected native request")?;
                        write_response(stream, count, output)?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => return Err(error.to_string()),
                }
            }
            Ok(())
        });
        Self {
            address,
            requests,
            stop,
            worker: Some(worker),
        }
    }

    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.address.port())
    }

    fn finish(mut self) -> Vec<Value> {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap().unwrap();
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for Simulator {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn read_request(stream: TcpStream) -> Result<Value, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    if !line.starts_with("POST /v1/responses ") {
        return Err(format!("unexpected native route: {}", line.trim()));
    }
    let mut length = None;
    loop {
        line.clear();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if line == "\r\n" {
            break;
        }
        if line.is_empty() {
            return Err("native header EOF".into());
        }
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("content-length:") {
            length = Some(value.trim().parse::<usize>().map_err(|e| e.to_string())?);
        }
        if lower.starts_with("authorization:") {
            return Err("native simulator received authorization".into());
        }
    }
    let length = length.ok_or("native content length missing")?;
    if !(1..=2_000_000).contains(&length) {
        return Err("native request out of bounds".into());
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn write_response(mut stream: TcpStream, count: usize, text: &str) -> Result<(), String> {
    let item = json!({
        "type":"message","id":format!("s5-message-{count}"),"role":"assistant",
        "status":"completed","content":[{"type":"output_text","text":text,"annotations":[]}]
    });
    let response = json!({
        "id":format!("s5-response-{count}"),"object":"response","created_at":1,
        "status":"completed","output":[item.clone()],
        "usage":{"input_tokens":3,"output_tokens":2,"total_tokens":5}
    });
    let events = [
        json!({"type":"response.created","response":{"id":format!("s5-response-{count}"),"status":"in_progress","output":[]}}),
        json!({"type":"response.output_item.done","output_index":0,"item":item}),
        json!({"type":"response.completed","response":response}),
    ];
    let wire = events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect::<String>();
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{wire}", wire.len()).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())
}

fn config(root: &Path, base_url: &str, generation: u8, verifier: &Path) -> Value {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let grant = |id: &str, context: &str, scope: &str| {
        json!({
            "grant_ref":id,"context_generation":context,"scope":scope,
            "expires_wall_ms":now+120_000,"revision":1
        })
    };
    let grants = vec![
        grant("grant:s5-native", "context:s5", "native_turn"),
        grant("grant:s5-snapshot", "context:s5", "snapshot_read"),
        grant("grant:s5-replace", "context:s5", "request_replacement"),
        grant(
            "grant:s5-successor-input",
            "context:s5-next",
            "enqueue_input",
        ),
        grant("grant:s5-next-native", "context:s5-next", "native_turn"),
        grant("grant:s5-next-snapshot", "context:s5-next", "snapshot_read"),
    ];
    let suffix = format!("g{generation}");
    let native_root = root.join(format!("native-{suffix}"));
    fs::create_dir(&native_root).unwrap();
    fs::set_permissions(&native_root, fs::Permissions::from_mode(0o700)).unwrap();
    let context = if generation == 1 {
        "context:s5"
    } else {
        "context:s5-next"
    };
    let native_grant = if generation == 1 {
        "grant:s5-native"
    } else {
        "grant:s5-next-native"
    };
    let snapshot_grant = if generation == 1 {
        "grant:s5-snapshot"
    } else {
        "grant:s5-next-snapshot"
    };
    let binary = native_binding::checked().binary.clone();
    json!({
        "profile":"native_simulated","store_root":root.join("store"),
        "socket_path":root.join(format!("service-{suffix}.sock")),
        "peer_socket_path":root.join("unused-peer.sock"),
        "verifier_executable":verifier,"verifier_ledger_path":root.join("verifier-ledger"),
        "proof_barrier_dir":null,"trusted_issuer":"issuer:s5",
        "principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal:s5","producer_ref":"producer:s5"},
        "subject_id":"subject:s5","context_generation":context,"build_id":"build:s5","proof_run_id":"proof:s5",
        "grants":grants,"max_executions":1,"max_wait_ms":1000,
        "native":{
            "executable":binary,"root":native_root,"simulator_base_url":base_url,
            "session_id":format!("session:s5:{suffix}"),
            "invocation_id":format!("invocation:s5:domain:{suffix}"),
            "attempt_id":format!("attempt:s5:domain:{suffix}"),
            "prompt":"Execute the S5 native domain turn.",
            "turn_grant_ref":native_grant,"turn_grant_revision":1,
            "snapshot_grant_ref":snapshot_grant,"snapshot_grant_revision":1,
            "same_native_session_required":false,
            "snapshot":{"id":format!("snapshot:s5:{suffix}"),"source_revision":format!("synthetic:s5:{suffix}"),
                "members":[{"id":"marker","sha256":lifecycle_semantic::sha256_hex(b"S5"),"bytes":[83,53]}]}
        }
    })
}

struct ServiceChild(Child);

struct FailureEvidence {
    root: Option<TempDir>,
    requests: Arc<Mutex<Vec<Value>>>,
}

impl FailureEvidence {
    fn path(&self) -> &Path {
        self.root.as_ref().unwrap().path()
    }
}

impl Drop for FailureEvidence {
    fn drop(&mut self) {
        if thread::panicking() {
            let root = self.root.take().unwrap().keep();
            if let Ok(requests) = self.requests.lock()
                && let Ok(bytes) = serde_json::to_vec_pretty(&*requests)
            {
                let _ = fs::write(root.join("simulator-requests.json"), bytes);
            }
            eprintln!(
                "S5 compatibility failure evidence retained at {}",
                root.display()
            );
        }
    }
}

impl ServiceChild {
    fn start(config_path: &Path) -> Self {
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "s5_service_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("S5_CHILD_CONFIG", config_path)
            .env("TMPDIR", &native_binding::checked().temporary)
            .env("CARGO_TARGET_DIR", &native_binding::checked().target)
            .stdout(Stdio::from(
                fs::File::create(config_path.with_extension("stdout")).unwrap(),
            ))
            .stderr(Stdio::from(
                fs::File::create(config_path.with_extension("stderr")).unwrap(),
            ))
            .spawn()
            .unwrap();
        Self(child)
    }

    fn terminate(&mut self) {
        assert!(
            Command::new("kill")
                .args(["-TERM", &self.0.id().to_string()])
                .env("TMPDIR", &native_binding::checked().temporary)
                .env("CARGO_TARGET_DIR", &native_binding::checked().target)
                .status()
                .unwrap()
                .success()
        );
        let started = Instant::now();
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success(), "S5 service child failed: {status}");
                return;
            }
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "S5 service child did not stop"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for ServiceChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[test]
fn s5_service_child() {
    let Ok(path) = std::env::var("S5_CHILD_CONFIG") else {
        return;
    };
    let config: lifecycle_service::ServiceConfig =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert!(config.native.as_ref().unwrap().semantic.is_none());
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(lifecycle_service::run(config)).unwrap();
}

fn wait_until(label: &str, deadline: Duration, mut check: impl FnMut() -> bool) {
    let started = Instant::now();
    while !check() {
        assert!(
            started.elapsed() < deadline,
            "timed out waiting for {label}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn read_db(db: &Path, sql: &str) -> Option<String> {
    let connection = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    connection
        .query_row(sql, [], |row| {
            let value: rusqlite::types::Value = row.get(0)?;
            Ok(match value {
                rusqlite::types::Value::Text(text) => text,
                rusqlite::types::Value::Integer(number) => number.to_string(),
                other => format!("{other:?}"),
            })
        })
        .ok()
}

fn wait_for_gen1_settlement(db: &Path) {
    wait_until("gen1 domain settlement", Duration::from_secs(30), || {
        // Exit is committed after turn state. Read it first so a concurrent
        // successful completion cannot pair an old turn state with a new exit.
        let outcome = read_db(
            db,
            "SELECT outcome_kind FROM native_process_exits WHERE session_id='session:s5:g1'",
        );
        let state = read_db(
            db,
            "SELECT state FROM native_turns WHERE invocation_id='invocation:s5:domain:g1'",
        );
        if state.as_deref() == Some("completed_settled") {
            return true;
        }
        assert!(
            outcome.is_none(),
            "gen1 native process ended before settlement: outcome={outcome:?}, turn_state={state:?}"
        );
        false
    });
}

#[test]
fn native_failure_reports_outcome_before_settlement_timeout() {
    let root = tempdir().unwrap();
    let db = root.path().join("failure.sqlite");
    let connection = Connection::open(&db).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE native_turns(invocation_id TEXT, state TEXT);
         CREATE TABLE native_process_exits(session_id TEXT, outcome_kind TEXT);
         INSERT INTO native_process_exits VALUES('session:s5:g1', 'transport_error');",
        )
        .unwrap();
    let panic = std::panic::catch_unwind(|| wait_for_gen1_settlement(&db))
        .expect_err("terminal native failure must reject settlement");
    let message = panic.downcast_ref::<String>().unwrap();
    assert!(message.contains("gen1 native process ended before settlement"));
    assert!(message.contains("transport_error"));
}

fn revision(db: &Path) -> u64 {
    read_db(
        db,
        "SELECT revision FROM subjects WHERE subject_id='subject:s5'",
    )
    .unwrap()
    .parse()
    .unwrap()
}

fn request_replacement(socket: &Path, revision: u64) -> Value {
    let mut command = json!({
        "protocol_version":2,"principal_ref":"principal:s5","command_id":"command:s5:replace",
        "subject_id":"subject:s5","context_generation":"context:s5","build_id":"build:s5",
        "proof_run_id":"proof:s5","grant_ref":"grant:s5-replace","expected_revision":revision.to_string(),
        "kind":"request_replacement","payload":{"reason":"S5 native compatibility"}
    });
    command["request_digest"] = json!(
        CodecContract::LifecycleCommandV2
            .digest_json(&command)
            .unwrap()
            .hex()
    );
    let mut stream = UnixStream::connect(socket).unwrap();
    serde_json::to_writer(&mut stream, &json!({"op":"command","command":command})).unwrap();
    stream.write_all(b"\n").unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[test]
fn installed_native_s5_rehydrates_without_semantic_control() {
    let simulator = Simulator::start();
    let root = FailureEvidence {
        root: Some(tempdir().unwrap()),
        requests: simulator.requests.clone(),
    };
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let verifier = root.path().join("verifier.sh");
    fs::write(&verifier, "#!/bin/sh\n[ \"$1\" = verify ] || exit 2\n[ -f \"$2\" ] || exit 2\n[ -n \"$3\" ] || exit 2\n[ ${#4} -eq 64 ] || exit 2\nIFS= read -r activation\n[ \"$activation\" = GO ] || exit 2\n").unwrap();
    fs::set_permissions(&verifier, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(root.path().join("verifier-ledger"), b"").unwrap();
    let db = root.path().join("store/lifecycle.sqlite");

    let first = config(root.path(), &simulator.base_url(), 1, &verifier);
    assert!(first["native"].get("semantic").is_none());
    let first_path = root.path().join("gen1.json");
    fs::write(&first_path, serde_json::to_vec(&first).unwrap()).unwrap();
    let first_socket = root.path().join("service-g1.sock");
    let mut first_child = ServiceChild::start(&first_path);
    wait_until("gen1 socket", Duration::from_secs(20), || {
        UnixStream::connect(&first_socket).is_ok()
    });
    wait_for_gen1_settlement(&db);
    let reply = request_replacement(&first_socket, revision(&db));
    assert_eq!(reply["accepted"], true, "{reply}");
    wait_until("S5 switching", Duration::from_secs(30), || {
        read_db(
            &db,
            "SELECT stage FROM transitions ORDER BY rowid DESC LIMIT 1",
        )
        .as_deref()
            == Some("switching")
    });
    first_child.terminate();

    let second = config(root.path(), &simulator.base_url(), 2, &verifier);
    assert!(second["native"].get("semantic").is_none());
    let second_path = root.path().join("gen2.json");
    fs::write(&second_path, serde_json::to_vec(&second).unwrap()).unwrap();
    let second_socket = root.path().join("service-g2.sock");
    let mut second_child = ServiceChild::start(&second_path);
    wait_until("gen2 socket", Duration::from_secs(20), || {
        UnixStream::connect(&second_socket).is_ok()
    });
    wait_until("S5 reconciliation", Duration::from_secs(30), || {
        read_db(
            &db,
            "SELECT stage FROM transitions ORDER BY rowid DESC LIMIT 1",
        )
        .as_deref()
            == Some("reconciled")
    });
    wait_until("gen2 native exit", Duration::from_secs(30), || {
        read_db(
            &db,
            "SELECT COUNT(*) FROM native_process_exits WHERE session_id='session:s5:g2'",
        )
        .as_deref()
            == Some("1")
    });
    second_child.terminate();

    let connection = Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (stage, required): (String, i64) = connection
        .query_row(
            "SELECT stage,semantic_required FROM transitions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((stage.as_str(), required), ("reconciled", 0));
    let (purpose, state, prompt, history): (String, String, Vec<u8>, String) = connection.query_row(
        "SELECT purpose,state,prompt,history_status FROM native_turns WHERE purpose='successor_rehydrate'",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    ).unwrap();
    assert_eq!(purpose, "successor_rehydrate");
    assert_eq!(state, "completed_settled");
    assert_eq!(prompt, PREDECESSOR.as_bytes());
    assert_eq!(history, "observed_complete");
    let (final_text, final_sha): (Vec<u8>, String) = connection.query_row(
        "SELECT final_text,final_sha256 FROM native_turns WHERE invocation_id='invocation:s5:domain:g1'",
        [], |row| Ok((row.get(0)?, row.get(1)?))
    ).unwrap();
    assert_eq!(final_text, PREDECESSOR.as_bytes());
    assert_eq!(
        final_sha,
        lifecycle_semantic::sha256_hex(PREDECESSOR.as_bytes())
    );
    let verifier_success: i64 = connection
        .query_row(
            "SELECT exit_success FROM native_verifier_results",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(verifier_success, 1);
    for table in [
        "semantic_captures",
        "semantic_attempts",
        "semantic_results",
        "semantic_material_rejections",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "unexpected S6 semantic state in {table}");
    }
    let roles: Vec<String> = connection
        .prepare("SELECT role FROM native_sessions ORDER BY created_wall_ms,session_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(roles, ["domain", "successor"]);
    let purposes: Vec<String> = connection
        .prepare("SELECT purpose FROM native_turns ORDER BY entered_wall_ms,invocation_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        purposes,
        ["domain_work", "successor_rehydrate", "domain_work"]
    );
    drop(connection);

    let requests = simulator.finish();
    assert_eq!(
        requests.len(),
        3,
        "expected predecessor, rehydrate and successor domain turns"
    );
    assert!(
        requests[1].to_string().contains("checkpoint_ready"),
        "rehydrate request lacked predecessor text"
    );
}
