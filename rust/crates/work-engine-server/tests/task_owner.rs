//! Real server processes over public HTTP and MCP, real private Git/SQLite owner
//! state and the accepted synthetic signing fixture. No provider or native work.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as b64};
use p256::{
    ecdsa::{Signature, SigningKey, signature::Signer},
    pkcs8::EncodePublicKey,
};
use serde_json::{Value, json};
use session_task_admission::{Ref, owner_boundary::*};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{Duration, Instant},
};
use time::macros::datetime;
use work_engine_server::LocalClient;

fn hash(b: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(b.as_ref()))
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    raw: String,
    task_profile: PathBuf,
    credential: PathBuf,
    database: PathBuf,
    script: PathBuf,
    config: SyntheticConfig,
    proposal: Ref,
}
/// Signing knobs for negative cases. Defaults are the accepted synthetic fixture.
#[derive(Clone)]
struct Sig {
    seed: u8,
    rp: &'static str,
    origin: &'static str,
    flags: u8,
    typ: &'static str,
    challenge: Option<String>,
    credential: &'static str,
}
impl Default for Sig {
    fn default() -> Self {
        Self {
            seed: 7,
            rp: "auth.example",
            origin: "https://auth.example",
            flags: 5,
            typ: "webauthn.get",
            challenge: None,
            credential: "key",
        }
    }
}
fn sign_with(answer: String, s: Sig) -> Value {
    let key = SigningKey::from_slice(&[s.seed; 32]).unwrap();
    let challenge = s
        .challenge
        .unwrap_or_else(|| b64.encode(Sha256::digest(answer.as_bytes())));
    let client = json!({"type":s.typ,"challenge":challenge,"origin":s.origin,"crossOrigin":false})
        .to_string();
    let mut auth = Sha256::digest(s.rp.as_bytes()).to_vec();
    auth.extend_from_slice(&[s.flags, 0, 0, 0, 0]);
    let mut bytes = auth.clone();
    bytes.extend_from_slice(&Sha256::digest(client.as_bytes()));
    let sig: Signature = key.sign(&bytes);
    json!({"answer_json":answer,"credential_id":s.credential,"authenticator_data":b64.encode(auth),"client_data_json":b64.encode(client),"signature":b64.encode(sig.to_der().as_bytes())})
}
fn sign(answer: String) -> Value {
    sign_with(answer, Sig::default())
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("repo");
        fs::create_dir(&root).unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-q"])
                .arg(&root)
                .status()
                .unwrap()
                .success()
        );
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let script = repo.join("skills/durable-state/scripts/codex_chatboard.py");
        let pin = |path: PathBuf| Pin {
            sha256: hash(fs::read(&path).unwrap()),
            path,
        };
        let program = Program {
            python: pin(PathBuf::from("/usr/bin/python3")),
            script: pin(script.clone()),
            dependencies: vec![
                pin(script.parent().unwrap().join("durable_state.py")),
                pin(script
                    .parent()
                    .unwrap()
                    .join("../contracts/codex-chatboard-kinds.v1.json")),
            ],
        };
        let fixture: Value = serde_json::from_str(include_str!(
            "../../session-task-admission/tests/projection-fixture.json"
        ))
        .unwrap();
        let mut row: Value =
            serde_json::from_str(fixture["tasks"][0]["request_json"].as_str().unwrap()).unwrap();
        row["data"]["ask"] = "Synthetic owner decision".into();
        row["data"]["intent"]["objective"] = "Unicode café and λ".into();
        let data = base.join("data.json");
        fs::write(&data, serde_json::to_vec(&row["data"]).unwrap()).unwrap();
        let board = |args: &[&str]| {
            let out = Command::new("/usr/bin/python3")
                .arg("-I")
                .arg(&script)
                .arg("--repository")
                .arg(&root)
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            out.stdout
        };
        board(&[
            "post",
            "--author",
            "fixture-actor",
            "--session-id",
            "11111111-1111-4111-8111-111111111111",
            "--topic",
            "fixture:task",
            "--body",
            "Synthetic only",
            "--kind",
            "request",
            "--to",
            "role:owner",
            "--message-id",
            "22222222-2222-4222-8222-222222222222",
            "--data-file",
            data.to_str().unwrap(),
        ]);
        let snap: Value = serde_json::from_slice(&board(&[
            "owner-boundary-io",
            "--command-json",
            r#"{"operation":"snapshot"}"#,
        ]))
        .unwrap();
        let raw = snap["rows"][0].as_str().unwrap().to_owned();
        let proposal: Ref =
            serde_json::from_value(row["data"]["intent"]["proposal"].clone()).unwrap();
        let key = SigningKey::from_slice(&[7; 32]).unwrap();
        let spki = key.verifying_key().to_public_key_der().unwrap();
        let fp = hash(spki.as_bytes());
        let policy = base.join("policy.json");
        private(&policy,json!({"synthetic":true,"expires_at":"2026-11-04T00:00:00Z","owner":"owner","rp_id":"auth.example","origin":"https://auth.example","keys":{"key":{"spki_b64url":b64.encode(spki.as_bytes()),"fingerprint":fp,"cose_alg":-7}}}).to_string().as_bytes());
        let database = base.join("registry.sqlite");
        let config = SyntheticConfig {
            database: database.clone(),
            repository: root.clone(),
            board: program.clone(),
            policy: pin(policy.clone()),
            credential_id: "key".into(),
            key_fingerprint: fp.clone(),
            owner: "owner".into(),
            actor: "fixture-actor".into(),
            session_id: "33333333-3333-4333-8333-333333333333".into(),
            rp_id: "auth.example".into(),
            origin: "https://auth.example".into(),
            epoch: datetime!(2026-11-02 01:00 UTC),
            scoped_control: None,
        };
        // Test-only producer registration through the owner's exported fixture helpers.
        let mut owner = SyntheticOwner::open_synthetic(config.clone()).unwrap();
        owner
            .register_proposal_fixture(&proposal, "live", None)
            .unwrap();
        owner.register_task_fixture(&raw, "live", None).unwrap();
        drop(owner);
        let pin_json = |p: &Pin| json!({"path":p.path,"sha256":p.sha256});
        let task_profile = base.join("task-profile.json");
        private(&task_profile,json!({"version":1,"database":database,"repository":root,"board":{"python":pin_json(&program.python),"script":pin_json(&program.script),"dependencies":program.dependencies.iter().map(pin_json).collect::<Vec<_>>()},"policy":pin_json(&pin(policy)),"credential_id":"key","key_fingerprint":fp,"owner":"owner","actor":"fixture-actor","session_id":"33333333-3333-4333-8333-333333333333","rp_id":"auth.example","origin":"https://auth.example","epoch":"2026-11-02T01:00:00Z"}).to_string().as_bytes());
        let credential = base.join("token");
        private(&credential, b"synthetic-transport-token-0123456789abcdef");
        Self {
            _temp: temp,
            root,
            raw,
            task_profile,
            credential,
            database,
            script,
            config,
            proposal,
        }
    }
    fn owner(&self) -> SyntheticOwner {
        SyntheticOwner::open_synthetic(self.config.clone()).unwrap()
    }
    /// Writes a variant of the trusted startup profile (never caller input).
    fn profile_with(&self, name: &str, edit: impl FnOnce(&mut Value)) -> PathBuf {
        let mut v: Value = serde_json::from_slice(&fs::read(&self.task_profile).unwrap()).unwrap();
        edit(&mut v);
        let path = self.root.parent().unwrap().join(name);
        private(&path, v.to_string().as_bytes());
        path
    }
    fn config_for(&self, port: u16, mcp: bool, profile: &Path) -> PathBuf {
        let path = self
            .root
            .parent()
            .unwrap()
            .join(format!("server-{port}.json"));
        private(&path,json!({"version":1,"listen":format!("127.0.0.1:{port}"),"mcp":mcp,"credential_file":self.credential,"task_profile":profile,"task_capability":"synthetic-owner-v1","max_connections":8,"max_queries":2,"request_timeout_seconds":30}).to_string().as_bytes());
        path
    }
    fn answer(&self, n: u64, decision: &str) -> String {
        self.answer_with(n, decision, |_| {})
    }
    fn answer_with(&self, n: u64, decision: &str, edit: impl FnOnce(&mut Value)) -> String {
        let mut v = self.answer_value(n, decision);
        edit(&mut v);
        v.to_string()
    }
    fn answer_value(&self, n: u64, decision: &str) -> Value {
        json!({"v":1,"type":"work-engine.owner-answer","answer_id":format!("00000000-0000-4000-8000-{n:012x}"),"board_seq":1,"request_digest":hash(&self.raw),"decision":decision,"comment":if decision=="ask"{"Please clarify"}else{""},"owner":"owner","created_at":"2026-11-02T00:00:00Z","expires_at":"2026-11-03T00:00:00Z","rp_id":"auth.example"})
    }
    fn effect_counts(&self) -> Vec<i64> {
        let db = rusqlite::Connection::open(&self.database).unwrap();
        [
            "admission",
            "outbox",
            "capacity_claim",
            "scoped_claim",
            "pi_claim",
        ]
        .iter()
        .map(|t| {
            db.query_row(&format!("SELECT count(*) FROM {t}"), [], |r| r.get(0))
                .unwrap()
        })
        .collect()
    }
    fn board_rows(&self) -> usize {
        let out = Command::new("/usr/bin/python3")
            .arg("-I")
            .arg(&self.script)
            .arg("--repository")
            .arg(&self.root)
            .args([
                "owner-boundary-io",
                "--command-json",
                r#"{"operation":"snapshot"}"#,
            ])
            .output()
            .unwrap();
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["rows"]
            .as_array()
            .unwrap()
            .len()
    }
    /// Writes a SYNTHETIC checkpoint bundle. The transport token is recorded by
    /// digest only; the policy file holds only the synthetic public key.
    fn export(&self, dir: &Path, observed: &Value, servers: &[(u16, bool)]) {
        fs::create_dir_all(dir.join("inputs")).unwrap();
        let copy = |from: &Path, name: &str| {
            let bytes = fs::read(from).unwrap();
            fs::write(dir.join("inputs").join(name), &bytes).unwrap();
            hash(&bytes)
        };
        let profile: Value =
            serde_json::from_slice(&fs::read(&self.task_profile).unwrap()).unwrap();
        let mut configs = Vec::new();
        for (port, mcp) in servers {
            let path = self
                .root
                .parent()
                .unwrap()
                .join(format!("server-{port}.json"));
            let name = if *mcp {
                "server-config-mcp.json"
            } else {
                "server-config-http.json"
            };
            configs.push(json!({"name":name,"mcp":mcp,"sha256":copy(&path, name)}));
        }
        let binary = fs::read(env!("CARGO_BIN_EXE_work-engine-server")).unwrap();
        let snapshot = Command::new("/usr/bin/python3")
            .arg("-I")
            .arg(&self.script)
            .arg("--repository")
            .arg(&self.root)
            .args([
                "owner-boundary-io",
                "--command-json",
                r#"{"operation":"snapshot"}"#,
            ])
            .output()
            .unwrap();
        let snapshot: Value = serde_json::from_slice(&snapshot.stdout).unwrap();
        let bundle = json!({
            "schema": "ah2t.checkpoint-bundle.v1",
            "label": "SYNTHETIC: disposable fixtures, fixed synthetic policy; no live key, signer or admission",
            "request": {"request_json": self.raw, "request_sha256": hash(&self.raw)},
            "signed_answer": observed["consume_input"]["signed"],
            "selection": observed["consume_input"]["selection"],
            "startup": {
                "task_profile_sha256": copy(&self.task_profile, "task-profile.json"),
                "task_profile": profile,
                "server_configs": configs,
                "policy_sha256": copy(profile["policy"]["path"].as_str().map(Path::new).unwrap(), "policy.json"),
                "transport_token_sha256": hash(fs::read(&self.credential).unwrap()),
                "server_binary_sha256": hash(&binary),
                "server_binary_len": binary.len(),
            },
            "observed": observed,
            "final": {
                "board_snapshot": snapshot,
                "board_rows": self.board_rows(),
                "effect_counts": {"tables": ["admission","outbox","capacity_claim","scoped_claim","pi_claim"], "counts": self.effect_counts()},
            },
        });
        fs::write(
            dir.join("bundle.json"),
            serde_json::to_vec_pretty(&bundle).unwrap(),
        )
        .unwrap();
    }
    fn config(&self, port: u16, mcp: bool) -> PathBuf {
        let path = self
            .root
            .parent()
            .unwrap()
            .join(format!("server-{port}.json"));
        private(&path,json!({"version":1,"listen":format!("127.0.0.1:{port}"),"mcp":mcp,"credential_file":self.credential,"task_profile":self.task_profile,"task_capability":"synthetic-owner-v1","max_connections":8,"max_queries":2,"request_timeout_seconds":30}).to_string().as_bytes());
        path
    }
}
struct Server {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    sequence: u64,
}
impl Server {
    fn start(config: &Path, addr: SocketAddr, token: &Path) -> (Self, LocalClient) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
            .arg(config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().unwrap());
        let client = LocalClient::new(
            addr,
            token.to_str().unwrap().into(),
            Duration::from_secs(30),
        )
        .unwrap();
        let start = Instant::now();
        while std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_err() {
            assert!(
                start.elapsed() < Duration::from_secs(20),
                "server not ready"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        (
            Self {
                child,
                input,
                output,
                sequence: 0,
            },
            client,
        )
    }
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.sequence += 1;
        writeln!(
            self.input.as_mut().unwrap(),
            "{}",
            json!({"jsonrpc":"2.0","id":self.sequence,"method":method,"params":params})
        )
        .unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        assert!(v.get("error").is_none(), "{v}");
        v["result"].clone()
    }
    fn tool(&mut self, name: &str, arguments: Value) -> Value {
        self.rpc("tools/call", json!({"name":name,"arguments":arguments}))["structuredContent"]
            .clone()
    }
    fn tool_full(&mut self, name: &str, arguments: Value) -> Value {
        self.rpc("tools/call", json!({"name":name,"arguments":arguments}))
    }
    /// SIGKILL the server (simulated process death) and collect its signal.
    fn kill9(mut self) -> Option<i32> {
        use std::os::unix::process::ExitStatusExt;
        self.child.kill().unwrap();
        self.child.wait().unwrap().signal()
    }
    /// Stop with SIGTERM, collect it, and return exit code plus unread stdout bytes.
    fn stop_with_output(mut self) -> (Option<i32>, Vec<u8>) {
        use std::io::Read;
        unsafe { libc::kill(self.child.id() as i32, libc::SIGTERM) };
        drop(self.input.take());
        let code = self.child.wait().unwrap().code();
        let mut rest = Vec::new();
        self.output.read_to_end(&mut rest).unwrap();
        (code, rest)
    }
    /// Terminate and collect the owned process; returns its actual exit code.
    fn stop(mut self) -> Option<i32> {
        unsafe { libc::kill(self.child.id() as i32, libc::SIGTERM) };
        drop(self.input.take());
        self.child.wait().unwrap().code()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn signed_answer_is_consumed_by_owner_through_http_and_mcp_with_exact_retry_and_restart() {
    let f = Fixture::new();
    let port = free_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let (server, client) = Server::start(&f.config(port, false), addr, &f.credential);
    // Real canonical request projected; no session-query owner is configured.
    let projected = client
        .task(
            "project_owner_task",
            json!({"task_id":"fixture-task"}),
            "p1",
        )
        .unwrap();
    let p = &projected["owner_result"];
    assert_eq!(p["request_json"], f.raw);
    assert_eq!(p["request_sha256"], hash(&f.raw));
    assert_eq!(p["standing"], "current");
    assert_eq!(p["synthetic"], true);
    assert!(p["execution_admission"].is_null());
    let consume = json!({"selection":p["selection"],"signed":sign(f.answer(0x9003,"approve"))});
    // Unknown before consumption, reported rather than inferred as absent.
    let before = client
        .task("read_owner_answer", consume.clone(), "r0")
        .unwrap();
    assert_eq!(before["owner_error"]["code"], "owner_unknown");
    assert_eq!(f.board_rows(), 1);
    // Valid signed answer durably consumed by the existing owner.
    let first = client
        .task("consume_owner_answer", consume.clone(), "c1")
        .unwrap();
    let decision = &first["owner_result"];
    assert_eq!(decision["historical"], false);
    assert_eq!(decision["signed"], consume["signed"]);
    assert_eq!(decision["request_json"], f.raw);
    assert!(decision["execution_admission"].is_null());
    assert_eq!(f.board_rows(), 2);
    // Exact retry returns the identical owner decision without a new board row.
    let again = client
        .task("consume_owner_answer", consume.clone(), "c2")
        .unwrap();
    assert_eq!(again["owner_result"]["decision"], decision["decision"]);
    assert_eq!(again["owner_result"]["historical"], true);
    assert_eq!(f.board_rows(), 2);
    // Changed answer bytes with the same answer id never reuse the decision.
    let mut changed = consume.clone();
    changed["signed"]["signature"] =
        format!("{}A", consume["signed"]["signature"].as_str().unwrap()).into();
    let conflict = client.task("consume_owner_answer", changed, "c3").unwrap();
    assert_eq!(conflict["owner_error"]["code"], "owner_conflict");
    // Caller-controlled configuration or unknown fields are invalid input.
    let mut extra = consume.clone();
    extra["database"] = "/tmp/other.sqlite".into();
    let invalid = client.task("consume_owner_answer", extra, "c4").unwrap();
    assert_eq!(invalid["owner_error"]["code"], "invalid_arguments");
    // Session-query and command routes are not reachable in tasks-only startup.
    let session = client.query("list_sessions", json!({}), "q1").unwrap();
    assert_eq!(session["error"]["code"], "invalid_request");
    assert_eq!(f.board_rows(), 2);
    assert_eq!(f.effect_counts(), vec![0; 5]);
    let exit_first = server.stop();
    assert_eq!(exit_first, Some(0));
    // Restart (new process, same trusted profile) with MCP enabled: reconcile exactly.
    let port2 = free_port();
    let addr2: SocketAddr = format!("127.0.0.1:{port2}").parse().unwrap();
    let (mut server, client) = Server::start(&f.config(port2, true), addr2, &f.credential);
    let init = server.rpc("initialize",json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}));
    assert_eq!(init["protocolVersion"], "2025-06-18");
    let listing = server.rpc("tools/list", json!({}));
    let names: Vec<_> = listing["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "project_owner_task",
            "consume_owner_answer",
            "read_owner_answer"
        ]
    );
    let http_read = client
        .task("read_owner_answer", consume.clone(), "r1")
        .unwrap();
    let mcp_read = server.tool("read_owner_answer", consume.clone());
    assert_eq!(http_read["owner_result"], again["owner_result"]);
    assert_eq!(mcp_read["owner_result"]["decision"], decision["decision"]);
    assert_eq!(mcp_read["owner_result"]["historical"], true);
    // Same exact consume through MCP: identical owner decision, no new row.
    let mcp_again = server.tool("consume_owner_answer", consume.clone());
    assert_eq!(mcp_again["owner_result"]["decision"], decision["decision"]);
    let mcp_project = server.tool("project_owner_task", json!({"task_id":"fixture-task"}));
    assert_eq!(mcp_project["owner_result"]["request_json"], f.raw);
    assert_eq!(f.board_rows(), 2);
    assert_eq!(f.effect_counts(), vec![0; 5]);
    let exit_second = server.stop();
    assert_eq!(exit_second, Some(0));
    // Optional retained evidence: explicitly SYNTHETIC runtime inputs and observations.
    if let Ok(dir) = std::env::var("TASK_OWNER_CHECKPOINT_DIR") {
        let observed = json!({
            "projection": projected,
            "consume_input": consume,
            "consume_first": first,
            "consume_exact_retry": again,
            "http_read_after_restart": http_read,
            "mcp_read_after_restart": mcp_read,
            "mcp_consume_retry_after_restart": mcp_again,
            "mcp_project_after_restart": mcp_project,
            "tampered_signature_result": conflict,
            "caller_field_result": invalid,
            "server_exit_codes": [exit_first, exit_second],
        });
        f.export(Path::new(&dir), &observed, &[(port, false), (port2, true)]);
    }
}

#[test]
fn startup_without_any_owner_profile_or_with_mismatched_capability_fails_closed() {
    let f = Fixture::new();
    let port = free_port();
    let base = f.root.parent().unwrap();
    for (name, config) in [
        (
            "none",
            json!({"version":1,"listen":format!("127.0.0.1:{port}"),"mcp":false,"credential_file":f.credential,"max_connections":4,"max_queries":2,"request_timeout_seconds":30}),
        ),
        (
            "capability-only",
            json!({"version":1,"listen":format!("127.0.0.1:{port}"),"mcp":false,"credential_file":f.credential,"task_capability":"synthetic-owner-v1","max_connections":4,"max_queries":2,"request_timeout_seconds":30}),
        ),
        (
            "wrong-capability",
            json!({"version":1,"listen":format!("127.0.0.1:{port}"),"mcp":false,"credential_file":f.credential,"task_profile":f.task_profile,"task_capability":"live","max_connections":4,"max_queries":2,"request_timeout_seconds":30}),
        ),
    ] {
        let path = base.join(format!("bad-{name}.json"));
        private(&path, config.to_string().as_bytes());
        let status = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(1), "{name}");
    }
}

// ---------------------------------------------------------------------------
// Negative, recovery, parity, concurrency and coexistence evidence.
// ---------------------------------------------------------------------------

/// Retained SYNTHETIC observations, written only when the evidence directory is set.
fn observe(name: &str, value: &Value) {
    if let Ok(dir) = std::env::var("TASK_OWNER_CHECKPOINT_DIR") {
        let dir = Path::new(&dir).join("observations");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(format!("{name}.json")),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }
}
fn code(envelope: &Value) -> String {
    for k in ["owner_error", "error"] {
        if let Some(c) = envelope[k]["code"].as_str() {
            return format!("{k}:{c}");
        }
    }
    assert!(envelope.get("owner_result").is_some(), "{envelope}");
    "ok".into()
}
/// Raw public HTTP exchange; the body is parsed from whatever the server returned.
fn http(
    addr: SocketAddr,
    line: &str,
    token: Option<&str>,
    body: Option<(&str, &[u8])>,
    extra: &[&str],
    host: Option<&str>,
) -> (u16, Value) {
    use std::io::Read;
    let mut s = std::net::TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
    s.set_write_timeout(Some(Duration::from_secs(30))).unwrap();
    let host = host.map(str::to_owned).unwrap_or_else(|| addr.to_string());
    let mut h = format!("{line}\r\nHost: {host}\r\n");
    if let Some(t) = token {
        h += &format!("Authorization: Bearer {t}\r\n");
    }
    if let Some((ct, b)) = body {
        h += &format!("Content-Type: {ct}\r\nContent-Length: {}\r\n", b.len());
    }
    for e in extra {
        h += e;
        h += "\r\n";
    }
    h += "\r\n";
    s.write_all(h.as_bytes()).unwrap();
    if let Some((_, b)) = body {
        s.write_all(b).unwrap();
    }
    let mut data = Vec::new();
    let _ = s.read_to_end(&mut data);
    let split = data
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("no response to {line}: {data:?}"));
    let status: u16 = std::str::from_utf8(&data[..split]).unwrap()[9..12]
        .parse()
        .unwrap();
    (status, serde_json::from_slice(&data[split + 4..]).unwrap())
}
const TOKEN: &str = "synthetic-transport-token-0123456789abcdef";
fn post(addr: SocketAddr, body: &[u8]) -> (u16, Value) {
    http(
        addr,
        "POST /v1/session-queries HTTP/1.1",
        Some(TOKEN),
        Some(("application/json", body)),
        &[],
        None,
    )
}
fn envelope(op: &str, args: &Value, id: &str) -> Vec<u8> {
    json!({"schema_version":1,"operation":op,"arguments":args,"request_id":id})
        .to_string()
        .into_bytes()
}
/// Collects every mismatch so one run pins the whole table.
struct Table(Vec<(String, String, String)>);
impl Table {
    fn new() -> Self {
        Self(Vec::new())
    }
    fn check(&mut self, name: &str, expected: &str, envelope: &Value) {
        self.0.push((name.into(), expected.into(), code(envelope)));
    }
    fn finish(self, observation: &str) {
        let rows: Vec<Value> = self
            .0
            .iter()
            .map(|(n, e, a)| json!({"case":n,"expected":e,"observed":a}))
            .collect();
        observe(observation, &json!(rows));
        let bad: Vec<_> = self.0.iter().filter(|(_, e, a)| e != a).collect();
        assert!(bad.is_empty(), "mismatches: {bad:#?}");
    }
}
fn flip(s: &str) -> String {
    let mut c: Vec<char> = s.chars().collect();
    c[0] = if c[0] == 'A' { 'B' } else { 'A' };
    c.into_iter().collect()
}
fn tasks_only(f: &Fixture, mcp: bool) -> (Server, LocalClient, SocketAddr) {
    let port = free_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let (server, client) = Server::start(&f.config(port, mcp), addr, &f.credential);
    (server, client, addr)
}
fn unestablished() -> Value {
    json!({"task_owner":false,"task_owner_readiness":{"state":"unestablished","reason":"no_nonmutating_owner_probe","capability":"synthetic-owner-v1"}})
}
fn unavailable_board() -> Value {
    json!({"task_owner":false,"task_owner_readiness":{"state":"unavailable","reason":"board_executable_missing","capability":"synthetic-owner-v1"}})
}
fn project(client: &LocalClient, id: &str) -> Value {
    client
        .task("project_owner_task", json!({"task_id":"fixture-task"}), id)
        .unwrap()["owner_result"]
        .clone()
}

#[test]
fn wrong_key_and_every_meaningful_tamper_are_rejected_without_effect() {
    let f = Fixture::new();
    let (server, client, _) = tasks_only(&f, false);
    let sel = project(&client, "p")["selection"].clone();
    let valid_answer = f.answer(0xA001, "approve");
    let valid = sign(valid_answer.clone());
    let with = |signed: Value| json!({"selection":sel,"signed":signed});
    let sel_with = |edit: &dyn Fn(&mut Value)| {
        let mut s = sel.clone();
        edit(&mut s);
        json!({"selection":s,"signed":valid})
    };
    let resign = |edit: &dyn Fn(&mut Value)| with(sign(f.answer_with(0xA001, "approve", edit)));
    let mut t = Table::new();
    let run = |t: &mut Table, name: &str, expected: &str, input: Value| {
        let r = client.task("consume_owner_answer", input, name).unwrap();
        t.check(name, expected, &r);
    };
    // Wrong key, wrong credential, and each of the five signed strings changed in place.
    run(
        &mut t,
        "wrong-key",
        "owner_error:owner_invalid",
        with(sign_with(
            valid_answer.clone(),
            Sig {
                seed: 9,
                ..Sig::default()
            },
        )),
    );
    for field in [
        "answer_json",
        "credential_id",
        "authenticator_data",
        "client_data_json",
        "signature",
    ] {
        let mut signed = valid.clone();
        let old = signed[field].as_str().unwrap().to_owned();
        signed[field] = if field == "answer_json" {
            old.replace("approve", "reject")
        } else {
            flip(&old)
        }
        .into();
        run(
            &mut t,
            &format!("tamper-{field}"),
            "owner_error:owner_invalid",
            with(signed),
        );
    }
    let mut dropped = valid.clone();
    dropped["signature"] = "".into();
    run(
        &mut t,
        "empty-signature",
        "owner_error:owner_invalid",
        with(dropped),
    );
    // Validly signed but semantically wrong assertions or answers.
    for (name, sig) in [
        (
            "assertion-wrong-rp-hash",
            Sig {
                rp: "evil.example",
                ..Sig::default()
            },
        ),
        (
            "assertion-wrong-origin",
            Sig {
                origin: "https://evil.example",
                ..Sig::default()
            },
        ),
        (
            "assertion-no-user-presence",
            Sig {
                flags: 0,
                ..Sig::default()
            },
        ),
        (
            "assertion-wrong-type",
            Sig {
                typ: "webauthn.create",
                ..Sig::default()
            },
        ),
        (
            "assertion-wrong-challenge",
            Sig {
                challenge: Some(b64.encode(Sha256::digest(b"other"))),
                ..Sig::default()
            },
        ),
        (
            "assertion-unknown-credential",
            Sig {
                credential: "other",
                ..Sig::default()
            },
        ),
    ] {
        run(
            &mut t,
            name,
            "owner_error:owner_invalid",
            with(sign_with(valid_answer.clone(), sig)),
        );
    }
    let e = "owner_error:owner_invalid";
    run(
        &mut t,
        "answer-wrong-request-digest",
        e,
        resign(&|v| v["request_digest"] = "0".repeat(64).into()),
    );
    run(
        &mut t,
        "answer-wrong-owner",
        e,
        resign(&|v| v["owner"] = "mallory".into()),
    );
    run(
        &mut t,
        "answer-wrong-rp-id",
        e,
        resign(&|v| v["rp_id"] = "evil.example".into()),
    );
    run(
        &mut t,
        "answer-unknown-decision",
        e,
        resign(&|v| v["decision"] = "maybe".into()),
    );
    run(
        &mut t,
        "answer-wrong-board-seq",
        e,
        resign(&|v| v["board_seq"] = 2.into()),
    );
    run(
        &mut t,
        "answer-wrong-type",
        e,
        resign(&|v| v["type"] = "other".into()),
    );
    run(
        &mut t,
        "answer-wrong-version",
        e,
        resign(&|v| v["v"] = 2.into()),
    );
    run(
        &mut t,
        "answer-not-uuid",
        e,
        resign(&|v| v["answer_id"] = "not-a-uuid".into()),
    );
    run(
        &mut t,
        "answer-extra-field",
        e,
        resign(&|v| v["extra"] = 1.into()),
    );
    run(
        &mut t,
        "answer-missing-field",
        e,
        resign(&|v| {
            v.as_object_mut().unwrap().remove("comment");
        }),
    );
    // Expiry before the synthetic epoch, and not yet created.
    run(
        &mut t,
        "answer-expired",
        e,
        resign(&|v| v["expires_at"] = "2026-11-02T00:30:00Z".into()),
    );
    run(
        &mut t,
        "answer-created-in-future",
        e,
        resign(&|v| v["created_at"] = "2026-11-03T00:00:00Z".into()),
    );
    // Selection mismatches, each checked by the owner against its registry/board.
    run(
        &mut t,
        "selection-stale-board-revision",
        "owner_error:owner_conflict",
        sel_with(&|s| s["board_revision"] = "0".repeat(40).into()),
    );
    run(
        &mut t,
        "selection-wrong-task-sha",
        "owner_error:owner_held",
        sel_with(&|s| s["task"]["sha256"] = "0".repeat(64).into()),
    );
    run(
        &mut t,
        "selection-stale-task-generation",
        "owner_error:owner_held",
        sel_with(&|s| s["task_generation"] = 7.into()),
    );
    run(
        &mut t,
        "selection-stale-proposal-generation",
        "owner_error:owner_held",
        sel_with(&|s| s["proposal_generation"] = 7.into()),
    );
    run(
        &mut t,
        "selection-unknown-task",
        "owner_error:owner_held",
        sel_with(&|s| s["task"]["id"] = "other-task".into()),
    );
    // Projection of an unregistered task.
    let r = client
        .task("project_owner_task", json!({"task_id":"other-task"}), "unk")
        .unwrap();
    t.check(
        "project-unregistered-task",
        "owner_error:owner_unavailable",
        &r,
    );
    assert_eq!(f.board_rows(), 1);
    assert_eq!(f.effect_counts(), vec![0; 5]);
    // Rejections did not poison the exact valid answer.
    let ok = client
        .task("consume_owner_answer", with(valid.clone()), "ok")
        .unwrap();
    t.check("valid-after-rejections", "ok", &ok);
    assert_eq!(f.board_rows(), 2);
    assert_eq!(f.effect_counts(), vec![0; 5]);
    t.finish("negative-matrix");
    assert_eq!(server.stop(), Some(0));
}

#[test]
fn revoked_replaced_and_expired_requests_are_not_consumable_and_history_survives() {
    let mut t = Table::new();
    // Task withdrawn, proposal withdrawn, task re-registered, proposal re-registered.
    for scenario in [
        "task-withdrawn",
        "proposal-withdrawn",
        "task-replaced",
        "proposal-replaced",
    ] {
        let f = Fixture::new();
        let (server, client, _) = tasks_only(&f, false);
        let sel = project(&client, "p")["selection"].clone();
        let consume = json!({"selection":sel,"signed":sign(f.answer(0xB001, "approve"))});
        let mut owner = f.owner();
        match scenario {
            "task-withdrawn" => owner
                .register_task_fixture(&f.raw, "withdrawn", Some(1))
                .unwrap(),
            "proposal-withdrawn" => owner
                .register_proposal_fixture(&f.proposal, "withdrawn", Some(1))
                .unwrap(),
            "task-replaced" => owner
                .register_task_fixture(&f.raw, "live", Some(1))
                .unwrap(),
            _ => owner
                .register_proposal_fixture(&f.proposal, "live", Some(1))
                .unwrap(),
        };
        drop(owner);
        let after = project(&client, "p2");
        observe(
            &format!("projection-{scenario}"),
            &json!({"standing":after["standing"],"selection":after["selection"]}),
        );
        let r = client
            .task("consume_owner_answer", consume.clone(), "c")
            .unwrap();
        t.check(
            &format!("{scenario}:stale-consume"),
            "owner_error:owner_held",
            &r,
        );
        assert_eq!(f.board_rows(), 1, "{scenario}");
        if scenario.ends_with("replaced") {
            // The current replacement selection can be answered; the stale one cannot.
            let fresh =
                json!({"selection":after["selection"],"signed":sign(f.answer(0xB002, "approve"))});
            let r = client.task("consume_owner_answer", fresh, "fresh").unwrap();
            t.check(&format!("{scenario}:fresh-selection"), "ok", &r);
            assert_eq!(f.board_rows(), 2);
        } else {
            assert_eq!(after["standing"], "revoked", "{scenario}");
        }
        assert_eq!(f.effect_counts(), vec![0; 5]);
        assert_eq!(server.stop(), Some(0));
    }
    // Revoked after consumption: history is retained and not re-granted.
    let f = Fixture::new();
    let (server, client, _) = tasks_only(&f, false);
    let sel = project(&client, "p")["selection"].clone();
    let consume = json!({"selection":sel,"signed":sign(f.answer(0xB003, "approve"))});
    let first = client
        .task("consume_owner_answer", consume.clone(), "c1")
        .unwrap();
    f.owner()
        .register_task_fixture(&f.raw, "withdrawn", Some(1))
        .unwrap();
    let read = client
        .task("read_owner_answer", consume.clone(), "r")
        .unwrap();
    let retry = client
        .task("consume_owner_answer", consume.clone(), "c2")
        .unwrap();
    t.check("consumed-then-revoked:read", "ok", &read);
    t.check("consumed-then-revoked:retry", "ok", &retry);
    assert_eq!(
        read["owner_result"]["decision"],
        first["owner_result"]["decision"]
    );
    assert_eq!(
        retry["owner_result"]["decision"],
        first["owner_result"]["decision"]
    );
    assert_eq!(read["owner_result"]["current_standing"], "revoked");
    assert_eq!(read["owner_result"]["historical"], true);
    assert_eq!(f.board_rows(), 2);
    assert_eq!(server.stop(), Some(0));
    // Profile epoch after the request's admit_until: projection expired, not consumable.
    let f = Fixture::new();
    let profile = f.profile_with("expired-profile.json", |v| {
        v["epoch"] = "2026-11-03T00:30:00Z".into()
    });
    let port = free_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let (server, client) = Server::start(&f.config_for(port, false, &profile), addr, &f.credential);
    let p = project(&client, "p");
    assert_eq!(p["standing"], "expired");
    let r = client
        .task(
            "consume_owner_answer",
            json!({"selection":p["selection"],"signed":sign(f.answer(0xB004, "approve"))}),
            "c",
        )
        .unwrap();
    t.check(
        "epoch-after-admit-deadline:consume",
        "owner_error:owner_held",
        &r,
    );
    assert_eq!(f.board_rows(), 1);
    assert_eq!(server.stop(), Some(0));
    t.finish("revoked-replaced-expired");
}

#[test]
fn changed_content_with_a_used_answer_id_never_reuses_the_decision() {
    let f = Fixture::new();
    let (server, client, _) = tasks_only(&f, false);
    let sel = project(&client, "p")["selection"].clone();
    let original = json!({"selection":sel,"signed":sign(f.answer(0xC001, "approve"))});
    let first = client
        .task("consume_owner_answer", original.clone(), "c1")
        .unwrap();
    let decision = first["owner_result"]["decision"].clone();
    let mut t = Table::new();
    // Same id, different decision, validly signed.
    let reuse = json!({"selection":sel,"signed":sign(f.answer(0xC001, "reject"))});
    let r = client
        .task("consume_owner_answer", reuse.clone(), "c2")
        .unwrap();
    t.check(
        "same-id-different-decision",
        "owner_error:owner_conflict",
        &r,
    );
    // Same id, same decision, different comment (different signed bytes).
    let mut other_bytes = f.answer_value(0xC001, "approve");
    other_bytes["comment"] = "changed".into();
    let r = client
        .task(
            "consume_owner_answer",
            json!({"selection":sel,"signed":sign(other_bytes.to_string())}),
            "c3",
        )
        .unwrap();
    t.check(
        "same-id-same-decision-different-bytes",
        "owner_error:owner_conflict",
        &r,
    );
    // Different id for the same already-answered request.
    let second = json!({"selection":sel,"signed":sign(f.answer(0xC002, "reject"))});
    let r = client
        .task("consume_owner_answer", second.clone(), "c4")
        .unwrap();
    t.check(
        "new-id-for-answered-request",
        "owner_error:owner_conflict",
        &r,
    );
    let r = client.task("read_owner_answer", second, "r4").unwrap();
    t.check("read-new-id", "owner_error:owner_unknown", &r);
    let r = client.task("read_owner_answer", reuse, "r5").unwrap();
    t.check("read-reused-id-changed", "owner_error:owner_conflict", &r);
    // The original answer still reconciles to its one decision.
    let again = client.task("consume_owner_answer", original, "c5").unwrap();
    assert_eq!(again["owner_result"]["decision"], decision);
    assert_eq!(f.board_rows(), 2);
    assert_eq!(f.effect_counts(), vec![0; 5]);
    t.finish("answer-id-reuse");
    assert_eq!(server.stop(), Some(0));
}

fn wrapper(f: &Fixture, mode: &str) -> Pin {
    use std::os::unix::fs::PermissionsExt;
    let base = f.root.parent().unwrap();
    let path = base.join("crash-python");
    let body = format!(
        r##"#!/usr/bin/python3
import sys,json,time,os
from pathlib import Path
script=sys.argv[2]
sys.path.insert(0,str(Path(script).parent))
import codex_chatboard as b
args=sys.argv[3:]
root=Path({root:?})
base=Path({base:?})
mode={mode:?}
command=json.loads(args[args.index('--command-json')+1])
if command['operation']=='conditional_reply':
 (base/'wrapper-pid').write_text(str(os.getpid()))
 if mode=='before':
  (base/'in-flight').write_text('before-commit')
  time.sleep(600)
 result=b.owner_boundary_io(root,command)
 (base/'in-flight').write_text('after-commit')
 time.sleep(600)
 print(json.dumps(result))
else:
 print(json.dumps(b.owner_boundary_io(root,command)))
"##,
        root = f.root.to_string_lossy(),
        base = base.to_string_lossy(),
        mode = mode
    );
    fs::write(&path, body).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    Pin {
        sha256: hash(fs::read(&path).unwrap()),
        path,
    }
}
fn gone(pid: i32) -> bool {
    let end = Instant::now() + Duration::from_secs(10);
    while Instant::now() < end {
        // This test process is a subreaper: an orphaned helper is reaped here.
        let mut status = 0;
        if unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) } == pid {
            return true;
        }
        if unsafe { libc::kill(pid, 0) } != 0 {
            return true;
        }
        // A killed orphan awaiting its reaper is dead: state Z in /proc.
        if fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|s| s.rsplit_once(") ").map(|(_, r)| r.starts_with('Z')))
            == Some(true)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn unknown_consumption_after_real_process_death_or_timeout_reconciles_without_new_decision() {
    // Orphaned board helpers (their server was killed) reparent to this process.
    assert_eq!(unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) }, 0);
    let mut observations = Vec::new();
    for variant in [
        "kill-before-commit",
        "kill-after-commit",
        "timeout-after-commit",
    ] {
        let f = Fixture::new();
        let base = f.root.parent().unwrap().to_owned();
        let mode = if variant.contains("before") {
            "before"
        } else {
            "after"
        };
        let python = wrapper(&f, mode);
        let profile = f.profile_with("crash-profile.json", |v| {
            v["board"]["python"] = json!({"path":python.path,"sha256":python.sha256});
        });
        let port = free_port();
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let (server, client) =
            Server::start(&f.config_for(port, false, &profile), addr, &f.credential);
        let sel = project(&client, "p")["selection"].clone();
        let consume = json!({"selection":sel,"signed":sign(f.answer(0xD001, "approve"))});
        let thread_client = LocalClient::new(
            addr,
            f.credential.to_str().unwrap().into(),
            Duration::from_secs(30),
        )
        .unwrap();
        let input = consume.clone();
        let inflight = std::thread::spawn(move || {
            thread_client.task("consume_owner_answer", input, "c-inflight")
        });
        let end = Instant::now() + Duration::from_secs(30);
        while !base.join("in-flight").is_file() {
            assert!(
                Instant::now() < end,
                "{variant}: consume never reached publication"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let wrapper_pid: i32 = fs::read_to_string(base.join("wrapper-pid"))
            .unwrap()
            .parse()
            .unwrap();
        let marker = fs::read_to_string(base.join("in-flight")).unwrap();
        let (inflight_result, server_exit, wrapper_gone);
        if variant.starts_with("kill") {
            let signal = server.kill9();
            let result = inflight.join().unwrap();
            // The wrapper child is a separate process group; collect it explicitly.
            unsafe { libc::kill(-wrapper_pid, libc::SIGKILL) };
            wrapper_gone = gone(wrapper_pid);
            inflight_result = json!({"client_result": format!("{result:?}")});
            assert!(
                matches!(result, Err("transport_unavailable" | "invalid_response")),
                "{variant}: {result:?}"
            );
            server_exit = json!({"signal": signal});
            assert_eq!(signal, Some(9));
        } else {
            // The owner's own 10 s bound kills the stalled helper group; no auto retry.
            let result = inflight.join().unwrap().unwrap();
            assert_eq!(code(&result), "owner_error:owner_unknown", "{variant}");
            wrapper_gone = gone(wrapper_pid);
            let read = client
                .task("read_owner_answer", consume.clone(), "r-same-server")
                .unwrap();
            assert_eq!(code(&read), "ok", "{variant}");
            inflight_result = json!({"envelope": result, "read_on_same_server": read});
            server_exit = json!({"code": server.stop()});
        }
        assert!(
            wrapper_gone,
            "{variant}: wrapper process {wrapper_pid} not collected"
        );
        // Restart with the normal pinned helper and reconcile exactly.
        let port2 = free_port();
        let addr2: SocketAddr = format!("127.0.0.1:{port2}").parse().unwrap();
        let (server, client) = Server::start(&f.config(port2, false), addr2, &f.credential);
        let read = client
            .task("read_owner_answer", consume.clone(), "r1")
            .unwrap();
        let outcome = if mode == "before" {
            assert_eq!(code(&read), "owner_error:owner_unknown", "{variant}");
            assert_eq!(f.board_rows(), 1);
            // No decision existed, so a fresh consume is a first decision, not a duplicate.
            let fresh = client
                .task("consume_owner_answer", consume.clone(), "c-fresh")
                .unwrap();
            assert_eq!(fresh["owner_result"]["historical"], false);
            assert_eq!(f.board_rows(), 2);
            json!({"read": read, "fresh_consume": fresh})
        } else {
            assert_eq!(code(&read), "ok", "{variant}");
            assert_eq!(read["owner_result"]["historical"], true);
            assert_eq!(f.board_rows(), 2, "{variant}: publication was durable");
            let retry = client
                .task("consume_owner_answer", consume.clone(), "c-retry")
                .unwrap();
            assert_eq!(
                retry["owner_result"]["decision"],
                read["owner_result"]["decision"]
            );
            assert_eq!(f.board_rows(), 2, "{variant}: retry appended nothing");
            json!({"read": read, "retry": retry})
        };
        assert_eq!(f.effect_counts(), vec![0; 5]);
        assert_eq!(server.stop(), Some(0));
        observations.push(json!({"variant":variant,"marker":marker,"wrapper_pid":wrapper_pid,"wrapper_collected":wrapper_gone,"server_exit":server_exit,"inflight":inflight_result,"after_restart":outcome,"final_board_rows":f.board_rows()}));
    }
    observe("unknown-consumption-recovery", &json!(observations));
}

#[test]
fn http_framing_authentication_and_limits_fail_closed_with_mcp_off() {
    let f = Fixture::new();
    let (mut server, client, addr) = tasks_only(&f, false);
    // MCP is off: a line written to stdin is never read or answered.
    writeln!(
        server.input.as_mut().unwrap(),
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{}}}}"#
    )
    .unwrap();
    let sel = project(&client, "p")["selection"].clone();
    let consume = json!({"selection":sel,"signed":sign(f.answer(0xE001, "approve"))});
    let good = envelope("consume_owner_answer", &consume, "x");
    let mut results = Vec::new();
    let mut expect = |name: &str, got: (u16, Value), status: u16| {
        results.push(json!({"case":name,"status":got.0,"body":got.1}));
        assert_eq!(got.0, status, "{name}: {}", got.1);
    };
    let live = http(addr, "GET /live HTTP/1.1", None, None, &[], None);
    expect("live-no-auth", live, 200);
    let ready = http(addr, "GET /ready HTTP/1.1", Some(TOKEN), None, &[], None);
    // Construction is not readiness: the owner has no nonmutating probe, so the
    // task capability is reported as unestablished, never `true`.
    assert_eq!(ready.1, unestablished());
    expect("ready", ready, 200);
    let json_ct = "application/json";
    let line = "POST /v1/session-queries HTTP/1.1";
    expect(
        "ready-wrong-token",
        http(
            addr,
            "GET /ready HTTP/1.1",
            Some("wrong-token-wrong-token-wrong-token-xx"),
            None,
            &[],
            None,
        ),
        401,
    );
    expect(
        "post-wrong-token",
        http(
            addr,
            line,
            Some("wrong-token-wrong-token-wrong-token-xx"),
            Some((json_ct, &good)),
            &[],
            None,
        ),
        401,
    );
    expect(
        "post-no-token",
        http(addr, line, None, Some((json_ct, &good)), &[], None),
        401,
    );
    expect(
        "post-short-token",
        http(addr, line, Some("x"), Some((json_ct, &good)), &[], None),
        401,
    );
    expect("malformed-json", post(addr, b"{not json"), 400);
    expect("top-level-duplicate-key", post(addr, br#"{"schema_version":1,"schema_version":1,"operation":"project_owner_task","arguments":{"task_id":"fixture-task"},"request_id":"x"}"#), 400);
    let signed = serde_json::to_string(&consume["signed"]).unwrap();
    let dup_signed = signed.replacen("\"signature\":", "\"signature\":\"zz\",\"signature\":", 1);
    let dup_body = format!(
        r#"{{"schema_version":1,"operation":"consume_owner_answer","arguments":{{"selection":{},"signed":{dup_signed}}},"request_id":"x"}}"#,
        consume["selection"]
    );
    expect(
        "nested-duplicate-key-in-signed",
        post(addr, dup_body.as_bytes()),
        400,
    );
    expect("unknown-envelope-field", post(addr, br#"{"schema_version":1,"operation":"project_owner_task","arguments":{"task_id":"fixture-task"},"request_id":"x","profile":"/tmp/p"}"#), 400);
    expect("not-an-object", post(addr, b"[1]"), 400);
    expect(
        "wrong-content-type",
        http(
            addr,
            line,
            Some(TOKEN),
            Some(("text/plain", &good)),
            &[],
            None,
        ),
        400,
    );
    expect(
        "wrong-host",
        http(
            addr,
            line,
            Some(TOKEN),
            Some((json_ct, &good)),
            &[],
            Some("evil.example:1"),
        ),
        400,
    );
    expect(
        "origin-header",
        http(
            addr,
            line,
            Some(TOKEN),
            Some((json_ct, &good)),
            &["Origin: https://evil.example"],
            None,
        ),
        400,
    );
    expect(
        "transfer-encoding",
        http(
            addr,
            line,
            Some(TOKEN),
            Some((json_ct, &good)),
            &["Transfer-Encoding: chunked"],
            None,
        ),
        400,
    );
    expect(
        "wrong-path",
        http(
            addr,
            "POST /v1/other HTTP/1.1",
            Some(TOKEN),
            Some((json_ct, &good)),
            &[],
            None,
        ),
        400,
    );
    expect(
        "get-on-query-path",
        http(
            addr,
            "GET /v1/session-queries HTTP/1.1",
            Some(TOKEN),
            None,
            &[],
            None,
        ),
        400,
    );
    expect(
        "oversize-content-length",
        http(
            addr,
            line,
            Some(TOKEN),
            None,
            &["Content-Type: application/json", "Content-Length: 300000"],
            None,
        ),
        400,
    );
    let big = envelope(
        "project_owner_task",
        &json!({"task_id":"x".repeat(300_000)}),
        "x",
    );
    expect(
        "oversize-body",
        http(
            addr,
            line,
            Some(TOKEN),
            None,
            &[
                "Content-Type: application/json",
                &format!("Content-Length: {}", big.len()),
            ],
            None,
        ),
        400,
    );
    // Envelope-level rejections are complete envelopes with no owner effect.
    let mut table = Table::new();
    for (name, body, expected) in [
        (
            "schema-version-2",
            json!({"schema_version":2,"operation":"project_owner_task","arguments":{"task_id":"fixture-task"},"request_id":"x"}),
            "error:invalid_request",
        ),
        (
            "empty-request-id",
            json!({"schema_version":1,"operation":"project_owner_task","arguments":{"task_id":"fixture-task"},"request_id":""}),
            "error:invalid_request",
        ),
        (
            "long-request-id",
            json!({"schema_version":1,"operation":"project_owner_task","arguments":{"task_id":"fixture-task"},"request_id":"r".repeat(129)}),
            "error:invalid_request",
        ),
        (
            "unknown-operation",
            json!({"schema_version":1,"operation":"drop_database","arguments":{},"request_id":"x"}),
            "error:invalid_request",
        ),
        (
            "missing-task-id",
            json!({"schema_version":1,"operation":"project_owner_task","arguments":{},"request_id":"x"}),
            "owner_error:invalid_arguments",
        ),
        (
            "caller-selects-database",
            json!({"schema_version":1,"operation":"project_owner_task","arguments":{"task_id":"fixture-task","database":"/tmp/x"},"request_id":"x"}),
            "owner_error:invalid_arguments",
        ),
        (
            "caller-selects-policy-in-consume",
            json!({"schema_version":1,"operation":"consume_owner_answer","arguments":{"selection":consume["selection"],"signed":consume["signed"],"policy":"/tmp/p"},"request_id":"x"}),
            "owner_error:invalid_arguments",
        ),
        (
            "signed-field-missing",
            json!({"schema_version":1,"operation":"consume_owner_answer","arguments":{"selection":consume["selection"],"signed":{"answer_json":"{}"}},"request_id":"x"}),
            "owner_error:invalid_arguments",
        ),
        (
            "arguments-not-object",
            json!({"schema_version":1,"operation":"consume_owner_answer","arguments":[],"request_id":"x"}),
            "owner_error:invalid_arguments",
        ),
        (
            "session-query-route-without-owner",
            json!({"schema_version":1,"operation":"list_sessions","arguments":{},"request_id":"x"}),
            "error:invalid_request",
        ),
        (
            "command-route-without-owner",
            json!({"schema_version":1,"operation":"submit_session_request","arguments":{},"request_id":"x"}),
            "error:invalid_request",
        ),
    ] {
        let (status, v) = post(addr, body.to_string().as_bytes());
        assert_eq!(status, 200, "{name}");
        table.check(name, expected, &v);
    }
    table.finish("http-envelope-rejections");
    // LocalClient guards.
    let bad = LocalClient::new(
        addr,
        f.root
            .parent()
            .unwrap()
            .join("wrong-token")
            .to_str()
            .unwrap()
            .into(),
        Duration::from_secs(5),
    )
    .unwrap();
    private(
        &f.root.parent().unwrap().join("wrong-token"),
        b"wrong-token-wrong-token-wrong-token-xx",
    );
    assert_eq!(
        bad.task("project_owner_task", json!({"task_id":"fixture-task"}), "x")
            .err(),
        Some("transport_rejected")
    );
    assert_eq!(
        client.task("drop_database", json!({}), "x").err(),
        Some("invalid_request")
    );
    assert_eq!(
        client.task("project_owner_task", json!({}), "").err(),
        Some("invalid_request")
    );
    assert_eq!(
        client
            .task(
                "project_owner_task",
                json!({"task_id":"x".repeat(300_000)}),
                "x"
            )
            .err(),
        Some("input_too_large")
    );
    observe("http-framing", &json!(results));
    assert_eq!(f.board_rows(), 1);
    assert_eq!(f.effect_counts(), vec![0; 5]);
    let (code, unread) = server.stop_with_output();
    assert_eq!(code, Some(0));
    assert!(
        unread.is_empty(),
        "MCP-off server answered stdin: {unread:?}"
    );
}

#[test]
fn http_mcp_and_local_client_return_identical_envelopes() {
    let f = Fixture::new();
    let (mut server, client, addr) = tasks_only(&f, true);
    server.rpc("initialize", json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}));
    let sel = project(&client, "p")["selection"].clone();
    let consume = json!({"selection":sel,"signed":sign(f.answer(0xF001, "approve"))});
    let mut changed = consume.clone();
    changed["signed"]["signature"] =
        format!("{}A", consume["signed"]["signature"].as_str().unwrap()).into();
    let mut extra = consume.clone();
    extra["database"] = "/tmp/x".into();
    let project_args = json!({"task_id":"fixture-task"});
    let mut rows = Vec::new();
    // All three routes use request_id "mcp" so complete envelopes are comparable.
    let mut all =
        |server: &mut Server, op: &str, args: &Value, name: &str, allow_history_flip: bool| {
            let h = post(addr, &envelope(op, args, "mcp")).1;
            let c = client.task(op, args.clone(), "mcp").unwrap();
            let m = server.tool_full(op, args.clone());
            let isr = m["isError"].as_bool().unwrap();
            let mut m = m["structuredContent"].clone();
            let (mut h2, mut c2) = (h.clone(), c.clone());
            if allow_history_flip {
                // The first mutating call is the only one that may differ in `historical`.
                h2["owner_result"]["historical"] = Value::Null;
                c2["owner_result"]["historical"] = Value::Null;
                m["owner_result"]["historical"] = Value::Null;
                assert_eq!(c2, m, "{name}");
            }
            assert_eq!(h2, c2, "{name}: http vs client");
            assert_eq!(c2, m, "{name}: client vs mcp");
            assert_eq!(
                isr,
                h.get("owner_error").is_some() || h.get("error").is_some(),
                "{name}: isError"
            );
            rows.push(json!({"case":name,"operation":op,"code":code(&h),"envelope":h}));
        };
    all(
        &mut server,
        "project_owner_task",
        &project_args,
        "project-current",
        false,
    );
    all(
        &mut server,
        "read_owner_answer",
        &consume,
        "read-before-consume",
        false,
    );
    all(
        &mut server,
        "consume_owner_answer",
        &consume,
        "consume-first-then-retries",
        true,
    );
    all(
        &mut server,
        "consume_owner_answer",
        &consume,
        "exact-retry",
        false,
    );
    all(
        &mut server,
        "read_owner_answer",
        &consume,
        "read-after",
        false,
    );
    all(
        &mut server,
        "project_owner_task",
        &project_args,
        "project-after",
        false,
    );
    all(
        &mut server,
        "consume_owner_answer",
        &changed,
        "changed-answer",
        false,
    );
    all(
        &mut server,
        "consume_owner_answer",
        &extra,
        "caller-field",
        false,
    );
    all(
        &mut server,
        "project_owner_task",
        &json!({"task_id":"other-task"}),
        "unregistered-task",
        false,
    );
    // An unknown tool is rejected by the shared dispatcher on both routes
    // (LocalClient refuses it before sending, so it is outside the three-way table).
    let h = post(addr, &envelope("session_query_absent", &json!({}), "mcp")).1;
    let m = server.tool_full("session_query_absent", json!({}));
    assert_eq!(m["structuredContent"], h);
    assert_eq!(m["isError"], true);
    assert_eq!(code(&h), "error:invalid_request");
    observe("parity", &json!(rows));
    assert_eq!(f.board_rows(), 2);
    assert_eq!(f.effect_counts(), vec![0; 5]);
    // MCP framing: duplicate keys and malformed lines are parse errors with no effect.
    let line = |server: &mut Server, text: &str| {
        writeln!(server.input.as_mut().unwrap(), "{text}").unwrap();
        let mut out = String::new();
        server.output.read_line(&mut out).unwrap();
        serde_json::from_str::<Value>(&out).unwrap()
    };
    assert_eq!(
        line(
            &mut server,
            r#"{"jsonrpc":"2.0","id":9,"id":10,"method":"tools/list"}"#
        )["error"]["code"],
        -32700
    );
    assert_eq!(line(&mut server, "{nope")["error"]["code"], -32700);
    assert_eq!(
        line(
            &mut server,
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"consume_owner_answer","arguments":{"signed":{"signature":"a","signature":"b"}}}}"#
        )["error"]["code"],
        -32700
    );
    assert_eq!(f.board_rows(), 2);
    assert_eq!(server.stop(), Some(0));
}

#[test]
fn concurrent_exact_retries_across_http_and_mcp_yield_one_decision() {
    let f = Fixture::new();
    let (mut server, client, addr) = tasks_only(&f, true);
    server.rpc("initialize", json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}));
    let sel = project(&client, "p")["selection"].clone();
    let consume = json!({"selection":sel,"signed":sign(f.answer(0xAA01, "approve"))});
    let token = f.credential.to_str().unwrap().to_owned();
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let (consume, token) = (consume.clone(), token.clone());
            std::thread::spawn(move || {
                LocalClient::new(addr, token, Duration::from_secs(60))
                    .unwrap()
                    .task("consume_owner_answer", consume, &format!("t{i}"))
                    .unwrap()
            })
        })
        .collect();
    let mcp = server.tool("consume_owner_answer", consume.clone());
    let mut results: Vec<Value> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    results.push(mcp);
    assert!(results.iter().all(|r| code(r) == "ok"), "{results:#?}");
    let first = &results[0]["owner_result"]["decision"];
    assert!(
        results
            .iter()
            .all(|r| &r["owner_result"]["decision"] == first)
    );
    let originals = results
        .iter()
        .filter(|r| r["owner_result"]["historical"] == false)
        .count();
    assert_eq!(originals, 1, "exactly one caller published");
    assert_eq!(f.board_rows(), 2);
    assert_eq!(f.effect_counts(), vec![0; 5]);
    observe(
        "concurrent-retries",
        &json!({"callers":results.len(),"original_publications":originals,"decision":first,"board_rows":f.board_rows()}),
    );
    assert_eq!(server.stop(), Some(0));
}

fn query_fixture(base: &Path) -> PathBuf {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    for d in ["codex", "claude", "state"] {
        fs::create_dir(base.join(d)).unwrap();
        fs::set_permissions(base.join(d), fs::Permissions::from_mode(0o700)).unwrap();
    }
    let owner = base.join("owner.py");
    fs::write(
        &owner,
        fs::read(repo.join("skills/session-events/scripts/session_events.py")).unwrap(),
    )
    .unwrap();
    let profile = base.join("query-profile.json");
    private(&profile, json!({"version":1,"owner":owner,"owner_sha256":hash(fs::read(&owner).unwrap()),"codex_home":base.join("codex"),"claude_home":base.join("claude"),"state_dir":base.join("state"),"bridge":null,"context":null}).to_string().as_bytes());
    profile
}

#[test]
fn session_query_coexists_and_task_capability_is_disabled_without_its_profile() {
    let f = Fixture::new();
    let base = f.root.parent().unwrap().to_owned();
    let query = query_fixture(&base);
    let port = free_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let cfg = base.join("both.json");
    let both = json!({"version":1,"listen":format!("127.0.0.1:{port}"),"mcp":true,"query_profile":query,"credential_file":f.credential,"task_profile":f.task_profile,"task_capability":"synthetic-owner-v1","max_connections":8,"max_queries":2,"request_timeout_seconds":30});
    private(&cfg, both.to_string().as_bytes());
    let (mut server, client) = Server::start(&cfg, addr, &f.credential);
    server.rpc("initialize", json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}));
    let names = |server: &mut Server| -> Vec<String> {
        server.rpc("tools/list", json!({}))["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(
        names(&mut server),
        [
            "list_sessions",
            "read_session",
            "project_owner_task",
            "consume_owner_answer",
            "read_owner_answer"
        ]
    );
    let ready = http(addr, "GET /ready HTTP/1.1", Some(TOKEN), None, &[], None).1;
    // A healthy query probe must not conceal the unestablished task readiness.
    let mut expected = unestablished();
    expected["list_sessions"] = true.into();
    expected["read_session"] = true.into();
    assert_eq!(ready, expected);
    let http_list = client.query("list_sessions", json!({}), "mcp").unwrap();
    assert!(http_list.get("owner_result").is_some(), "{http_list}");
    let mcp_list = server.tool("list_sessions", json!({}));
    assert_eq!(http_list, mcp_list);
    // Task operations work in the same process, with the same decision semantics.
    let sel = project(&client, "p")["selection"].clone();
    let consume = json!({"selection":sel,"signed":sign(f.answer(0xAB01, "approve"))});
    let first = client
        .task("consume_owner_answer", consume.clone(), "c")
        .unwrap();
    assert_eq!(first["owner_result"]["historical"], false);
    let retry = server.tool("consume_owner_answer", consume.clone());
    assert_eq!(
        retry["owner_result"]["decision"],
        first["owner_result"]["decision"]
    );
    assert_eq!(f.board_rows(), 2);
    assert_eq!(f.effect_counts(), vec![0; 5]);
    assert_eq!(server.stop(), Some(0));
    // Same config plus a command capability is rejected (documented unsupported combination).
    let port_c = free_port();
    let command_path = base.join("command-profile.json");
    private(&command_path, b"{}");
    let mut combined = both.clone();
    combined["listen"] = format!("127.0.0.1:{port_c}").into();
    combined["command_profile"] = command_path.to_str().unwrap().into();
    combined["command_capability"] = "offline-synthetic-v1".into();
    let combined_path = base.join("combined.json");
    private(&combined_path, combined.to_string().as_bytes());
    let status = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
        .arg(&combined_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(status.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&status.stderr).contains("invalid_config"),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    // Query-only server: the task capability is disabled and never advertised.
    let port_q = free_port();
    let addr_q: SocketAddr = format!("127.0.0.1:{port_q}").parse().unwrap();
    let cfg_q = base.join("query-only.json");
    private(&cfg_q, json!({"version":1,"listen":format!("127.0.0.1:{port_q}"),"mcp":true,"query_profile":query,"credential_file":f.credential,"max_connections":8,"max_queries":2,"request_timeout_seconds":30}).to_string().as_bytes());
    let (mut server, client) = Server::start(&cfg_q, addr_q, &f.credential);
    server.rpc("initialize", json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}}));
    assert_eq!(names(&mut server), ["list_sessions", "read_session"]);
    let disabled = client
        .task(
            "project_owner_task",
            json!({"task_id":"fixture-task"}),
            "mcp",
        )
        .unwrap();
    assert_eq!(code(&disabled), "error:invalid_request");
    let m = server.tool_full("consume_owner_answer", consume.clone());
    assert_eq!(m["isError"], true);
    assert_eq!(
        m["structuredContent"],
        client.task("consume_owner_answer", consume, "mcp").unwrap()
    );
    assert_eq!(f.board_rows(), 2);
    observe(
        "coexistence-and-disabled",
        &json!({"both_tools":["list_sessions","read_session","project_owner_task","consume_owner_answer","read_owner_answer"],"query_only_tools":["list_sessions","read_session"],"combined_with_command":{"exit":1,"stderr_contains":"invalid_config"},"disabled_envelope":disabled}),
    );
    assert_eq!(server.stop(), Some(0));
}

#[test]
fn trusted_profile_mutations_fail_startup_closed() {
    let f = Fixture::new();
    let mut seen = Vec::new();
    type Mutation = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, Mutation)> = vec![
        (
            "unknown-field",
            Box::new(|v| v["database_override"] = "/tmp/x".into()),
        ),
        ("version-2", Box::new(|v| v["version"] = 2.into())),
        (
            "bad-board-script-pin",
            Box::new(|v| v["board"]["script"]["sha256"] = "0".repeat(64).into()),
        ),
        (
            "bad-policy-pin",
            Box::new(|v| v["policy"]["sha256"] = "0".repeat(64).into()),
        ),
        (
            "wrong-key-fingerprint",
            Box::new(|v| v["key_fingerprint"] = "0".repeat(64).into()),
        ),
        (
            "wrong-credential-id",
            Box::new(|v| v["credential_id"] = "other".into()),
        ),
        ("wrong-owner", Box::new(|v| v["owner"] = "mallory".into())),
        (
            "epoch-after-policy-expiry",
            Box::new(|v| v["epoch"] = "2026-11-04T00:00:01Z".into()),
        ),
        (
            "epoch-not-rfc3339",
            Box::new(|v| v["epoch"] = "yesterday".into()),
        ),
        (
            "relative-database",
            Box::new(|v| v["database"] = "registry.sqlite".into()),
        ),
        (
            "session-id-not-uuid",
            Box::new(|v| v["session_id"] = "nope".into()),
        ),
    ];
    for (name, edit) in cases {
        let profile = f.profile_with(&format!("mut-{name}.json"), |v| edit(v));
        let port = free_port();
        let cfg = f.config_for(port, false, &profile);
        let out = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
            .arg(&cfg)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        seen.push(json!({"case":name,"exit":out.status.code(),"stderr":String::from_utf8_lossy(&out.stderr)}));
        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("invalid_task_profile"),
            "{name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    // A readable profile that is group-accessible is also refused (private-file rule).
    let loose = f.profile_with("loose.json", |_| {});
    fs::set_permissions(&loose, fs::Permissions::from_mode(0o644)).unwrap();
    let port = free_port();
    let out = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
        .arg(f.config_for(port, false, &loose))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    seen.push(json!({"case":"group-readable-profile","exit":out.status.code(),"stderr":String::from_utf8_lossy(&out.stderr)}));
    observe("profile-mutations", &json!(seen));
}

/// A trusted profile naming a nonexistent board interpreter passes startup (the
/// owner does not validate it), so readiness must not be derived from
/// construction: it is attributed unavailable while liveness stays intact and
/// every real task operation fails closed as `owner_unavailable`.
#[test]
fn missing_board_executable_is_unavailable_in_tasks_only_and_with_query() {
    let f = Fixture::new();
    let missing = f.profile_with("missing-board.json", |v| {
        v["board"]["python"] =
            json!({"path":"/nonexistent/work-engine/python3","sha256":"0".repeat(64)});
    });
    let mut seen = Vec::new();
    // Tasks only.
    let port = free_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let (server, client) = Server::start(&f.config_for(port, false, &missing), addr, &f.credential);
    let live = http(addr, "GET /live HTTP/1.1", None, None, &[], None);
    assert_eq!((live.0, live.1), (200, json!({"live":true})));
    let ready = http(addr, "GET /ready HTTP/1.1", Some(TOKEN), None, &[], None);
    assert_eq!((ready.0, ready.1.clone()), (200, unavailable_board()));
    let unauthenticated = http(addr, "GET /ready HTTP/1.1", None, None, &[], None);
    assert_eq!(unauthenticated.0, 401);
    let op = client
        .task("project_owner_task", json!({"task_id":"fixture-task"}), "m")
        .unwrap();
    assert_eq!(op["owner_error"]["code"], "owner_unavailable", "{op}");
    assert!(op.get("owner_result").is_none());
    seen.push(json!({"case":"tasks-only","ready":ready.1,"operation":op}));
    assert_eq!(server.stop(), Some(0));
    // Task plus query: both capabilities are reported; the healthy query probe
    // does not hide the unavailable task owner.
    let base = f.root.parent().unwrap().to_owned();
    let query = query_fixture(&base);
    let port = free_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let cfg = base.join("both-missing.json");
    private(&cfg, json!({"version":1,"listen":format!("127.0.0.1:{port}"),"mcp":false,"query_profile":query,"credential_file":f.credential,"task_profile":missing,"task_capability":"synthetic-owner-v1","max_connections":8,"max_queries":2,"request_timeout_seconds":30}).to_string().as_bytes());
    let (server, client) = Server::start(&cfg, addr, &f.credential);
    let ready = http(addr, "GET /ready HTTP/1.1", Some(TOKEN), None, &[], None);
    let mut expected = unavailable_board();
    expected["list_sessions"] = true.into();
    expected["read_session"] = true.into();
    assert_eq!((ready.0, ready.1.clone()), (200, expected));
    let list = client.query("list_sessions", json!({}), "q").unwrap();
    assert!(list.get("owner_result").is_some(), "{list}");
    seen.push(json!({"case":"task-plus-query","ready":ready.1}));
    assert_eq!(server.stop(), Some(0));
    observe("readiness-missing-board-executable", &json!(seen));
}
