//! Coding-job capability through the real server process and public `LocalClient`, with a fake Host
//! that has the `pi-docker` CLI shape. This qualifies the offline adapter only; it is not live.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    net::SocketAddr,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use work_engine_server::LocalClient;

const CRED: &str = "job-credential-0123456789abcdefghijklmnop";
const BEARER: &str = "service-bearer-0123456789abcdefghijklmnop";
const REQ: &str = "33333333-3333-4333-8333-333333333333";
const REQ2: &str = "44444444-4444-4444-8444-444444444444";
const RECEIPT_ID: &str = "5adaf203ef0b4049b9965502bbbaa0a0739733fc62985b836ca2441a67bb9828";

/// The Host's typed `Job` serialization in its struct field order and compact form; its SHA-256 is
/// the Host input digest, independent of the adapter under test.
fn host_job(state: &Path, packet: &Path, packet_sha: &str, deadline: u64) -> String {
    let q = |p: &Path| serde_json::to_string(&p.to_string_lossy()).unwrap();
    let art = |path: &str, c: char| {
        format!(
            r#"{{"path":"{path}","sha256":"{}"}}"#,
            c.to_string().repeat(64)
        )
    };
    let tree = |root: &str| {
        format!(
            r#"{{"root":"{root}","manifest":{}}}"#,
            art("/m/manifest.json", 'c')
        )
    };
    format!(
        r#"{{"schema":"work-engine.pi-docker-custody.v1","job_id":"job-1","state_dir":{state},"authority":{authority},"packet":{{"path":{packet},"sha256":"{packet_sha}"}},"docker":{docker},"image":"node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da","runtime":{runtime},"workspace":{workspace},"deadline_unix_seconds":{deadline},"command_timeout_seconds":30,"idle_seconds":0,"coding":{{"schema":"work-engine.pi-coding-fixture.v1","shim":{shim},"toolchain":{toolchain},"acceptance":{acceptance},"fixture_command":"true","max_tool_calls":1}}}}"#,
        state = q(state),
        packet = q(packet),
        authority = art("/a/authority.json", 'a'),
        docker = art("/usr/bin/docker", 'd'),
        runtime = tree("/r/runtime"),
        workspace = tree("/w/workspace"),
        shim = art("/s/shim.mjs", 'e'),
        toolchain = tree("/t/toolchain"),
        acceptance = art("/s/acceptance.rs", 'f'),
    )
}

fn hex(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn private(path: &Path, content: &[u8], mode: u32) {
    std::fs::write(path, content).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}
fn host_script(state: &Path, runs: &Path) -> String {
    format!(
        r#"#!/bin/sh
S='{s}'; R='{r}'
T="$(dirname "$S")"
journal() {{ sed -e "s/@P@/$1/" -e "s/@R@/$2/" "$T/journal.tpl" > "$S/pi-state.json"; }}
receipt() {{ cat "$T/receipt.json"; echo; }}
case "$2" in
 run) echo run >> "$R"; [ -e "$S/pi-state.json" ] && exit 1
  journal running false
  [ -e "$S/../hang" ] && sleep 60
  sleep 2; journal collected true; receipt;;
 inspect) [ -e "$S/pi-state.json" ] || exit 1; cat "$S/pi-state.json";;
 cancel) echo c >> "$S/../cancel-marker"; printf '{{"status":"cancellation_requested"}}';;
 reconcile) journal collected true; receipt;;
 *) exit 64;;
esac
"#,
        s = state.display(),
        r = runs.display()
    )
}

struct Fx {
    _t: tempfile::TempDir,
    root: PathBuf,
    addr: SocketAddr,
    config: PathBuf,
    cred_file: String,
    job_sha: String,
    packet_sha: String,
    server: Option<Child>,
}
impl Fx {
    fn new() -> Self {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().to_path_buf();
        for d in ["state", "ops", "codex", "claude", "events"] {
            std::fs::create_dir(root.join(d)).unwrap();
            std::fs::set_permissions(root.join(d), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let state = root.join("state");
        let host = root.join("host.sh");
        private(
            &host,
            host_script(&state, &root.join("runs")).as_bytes(),
            0o700,
        );
        let packet = b"{\"task\":\"fixture\"}";
        private(&root.join("packet.json"), packet, 0o600);
        let deadline = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 200;
        let canonical = host_job(&state, &root.join("packet.json"), &hex(packet), deadline);
        let host_digest = hex(canonical.as_bytes());
        // The operator's job file is pretty and key-sorted: its digest is not the Host digest.
        let job = serde_json::to_string_pretty(&serde_json::from_str::<Value>(&canonical).unwrap())
            .unwrap();
        private(&root.join("job.json"), job.as_bytes(), 0o600);
        private(
            &root.join("journal.tpl"),
            format!(
                r#"{{"schema":"work-engine.pi-docker-custody.v1","job":{canonical},"input_digest":"{host_digest}","phase":"@P@","removed":@R@,"receipt_id":"{RECEIPT_ID}"}}"#
            )
            .as_bytes(),
            0o600,
        );
        private(
            &root.join("receipt.json"),
            format!(
                r#"{{"receipt_id":"{RECEIPT_ID}","receipt":{{"schema":"work-engine.pi-docker-custody.v1","job":{canonical},"job_id":"job-1","input_digest":"{host_digest}","status":"component_passed","acceptance":"owner_pending","domain_effect_settlement":"not_claimed","phase":"collected","rpc_verified":true,"daemon_exit":0,"removed":true,"failures":[],"coding":{{"completed":true,"tool_removed":true}}}}}}"#
            )
            .as_bytes(),
            0o600,
        );
        let profile = json!({"version":1,"kind":"coding-job-profile","capability":"coding-job-offline-fixture-v1",
            "class":"offline-fixture","host_binary":host,"host_binary_sha256":hex(&std::fs::read(&host).unwrap()),
            "operation_root":root.join("ops"),"collector_deadline_seconds":20,"cancel_grace_seconds":1,
            "authorizations":[{"authorization_id":"auth-1","caller":"caller-1","credential_sha256":hex(CRED.as_bytes()),
              "job_file":root.join("job.json"),"job_sha256":hex(job.as_bytes()),"packet_sha256":hex(packet),
              "job_id":"job-1","state_dir":state,"host_input_digest":host_digest}]});
        private(
            &root.join("job-profile.json"),
            profile.to_string().as_bytes(),
            0o600,
        );
        let owner = root.join("owner.py");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../skills/session-events/scripts/session_events.py"),
            &owner,
        )
        .unwrap();
        let query = json!({"version":1,"owner":owner,"owner_sha256":hex(&std::fs::read(&owner).unwrap()),
            "codex_home":root.join("codex"),"claude_home":root.join("claude"),"state_dir":root.join("events"),
            "bridge":null,"context":null});
        private(
            &root.join("query.json"),
            query.to_string().as_bytes(),
            0o600,
        );
        private(&root.join("bearer"), BEARER.as_bytes(), 0o600);
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let config = root.join("server.json");
        private(
            &config,
            json!({"version":1,"listen":format!("127.0.0.1:{port}"),"mcp":false,
                "query_profile":root.join("query.json"),"credential_file":root.join("bearer"),
                "coding_job_profile":root.join("job-profile.json"),
                "coding_job_capability":"coding-job-offline-fixture-v1",
                "max_connections":4,"max_queries":2,"request_timeout_seconds":30})
            .to_string()
            .as_bytes(),
            0o600,
        );
        let mut fx = Fx {
            cred_file: root.join("bearer").to_string_lossy().into(),
            job_sha: hex(job.as_bytes()),
            packet_sha: hex(packet),
            addr: format!("127.0.0.1:{port}").parse().unwrap(),
            config,
            root,
            _t: t,
            server: None,
        };
        fx.start();
        fx
    }
    fn start(&mut self) {
        let child = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
            .arg(&self.config)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        self.server = Some(child);
        for _ in 0..200 {
            if std::net::TcpStream::connect(self.addr).is_ok() {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("server unavailable");
    }
    /// SIGTERM and wait: the server must drain its owned collectors before exiting.
    fn stop(&mut self) -> std::process::ExitStatus {
        let mut child = self.server.take().unwrap();
        unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
        child.wait().unwrap()
    }
    fn client(&self, timeout: Duration) -> LocalClient {
        LocalClient::new(self.addr, self.cred_file.clone(), timeout).unwrap()
    }
    fn submit(&self, request: &str) -> Value {
        self.client(Duration::from_secs(5))
            .job(
                "submit_coding_job",
                json!({"request_id":request,"authorization_id":"auth-1","job_credential":CRED,
                "job_sha256":self.job_sha,"packet_sha256":self.packet_sha}),
                "t",
            )
            .unwrap()
    }
    fn read(&self, op: &str, request: &str) -> Value {
        self.client(Duration::from_secs(5))
            .job(
                op,
                json!({"request_id":request,"authorization_id":"auth-1","job_credential":CRED}),
                "t",
            )
            .unwrap()
    }
    fn runs(&self) -> usize {
        std::fs::read_to_string(self.root.join("runs")).map_or(0, |s| s.lines().count())
    }
    fn wait_collected(&self) -> Value {
        for _ in 0..300 {
            let v = self.read("read_coding_job_operation", REQ);
            if v["owner_result"]["state"] == "collected" {
                return v;
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("job never collected");
    }
}
impl Drop for Fx {
    fn drop(&mut self) {
        if let Some(mut c) = self.server.take() {
            unsafe { libc::kill(c.id() as i32, libc::SIGTERM) };
            let _ = c.wait();
        }
    }
}

#[test]
fn job_capability_is_distinct_and_authority_is_not_the_bearer() {
    let fx = Fx::new();
    let client = fx.client(Duration::from_secs(5));
    // Session-message operations are not enabled by the job capability.
    let e = client
        .command("submit_session_request", json!({}), "t")
        .unwrap();
    assert_eq!(e["error"]["code"], "invalid_request");
    // Bearer without the job credential, and a session-grant-shaped credential, are refused.
    for credential in ["session-grant-0123456789abcdefghijklmnopq", &"x".repeat(40)] {
        let e = client
            .job(
                "submit_coding_job",
                json!({"request_id":REQ,"authorization_id":"auth-1","job_credential":credential,
                "job_sha256":fx.job_sha,"packet_sha256":fx.packet_sha}),
                "t",
            )
            .unwrap();
        assert_eq!(e["owner_error"]["code"], "unauthorized");
    }
    let wrong_bearer = std::fs::write(fx.root.join("wrong"), "w".repeat(40)).and_then(|_| {
        std::fs::set_permissions(
            fx.root.join("wrong"),
            std::fs::Permissions::from_mode(0o600),
        )
    });
    wrong_bearer.unwrap();
    let wrong = LocalClient::new(
        fx.addr,
        fx.root.join("wrong").to_string_lossy().into(),
        Duration::from_secs(5),
    )
    .unwrap();
    assert!(
        wrong
            .job("read_coding_job_operation", json!({}), "t")
            .is_err()
    );
    assert_eq!(fx.runs(), 0);
}

#[test]
fn startup_refuses_other_capabilities_and_half_configured_jobs() {
    let mut fx = Fx::new();
    // Startup holds exclusive profile locks, so stop the fixture server before probing refusals.
    assert!(fx.stop().success());
    let base: Value = serde_json::from_slice(&std::fs::read(&fx.config).unwrap()).unwrap();
    for (name, edit) in [
        (
            "live",
            ("coding_job_capability", json!("coding-job-live-v1")),
        ),
        (
            "synthetic",
            ("coding_job_capability", json!("offline-synthetic-v1")),
        ),
    ] {
        let mut cfg = base.clone();
        cfg[edit.0] = edit.1;
        cfg["listen"] = json!("127.0.0.1:1");
        let path = fx.root.join(format!("{name}.json"));
        private(&path, cfg.to_string().as_bytes(), 0o600);
        let out = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
            .arg(&path)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("invalid_job_profile"),
            "{name}"
        );
    }
    let mut cfg = base;
    cfg.as_object_mut().unwrap().remove("coding_job_capability");
    cfg["listen"] = json!("127.0.0.1:1");
    let path = fx.root.join("half.json");
    private(&path, cfg.to_string().as_bytes(), 0o600);
    let out = Command::new(env!("CARGO_BIN_EXE_work-engine-server"))
        .arg(&path)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn client_wait_expiry_server_stop_and_restart_keep_one_original_operation() {
    let mut fx = Fx::new();
    // Job runs for 2 s; a 1 s client times out only if it waited, but submit must return promptly.
    let t = Instant::now();
    let first = fx
        .client(Duration::from_secs(1))
        .job(
            "submit_coding_job",
            json!({"request_id":REQ,"authorization_id":"auth-1","job_credential":CRED,
            "job_sha256":fx.job_sha,"packet_sha256":fx.packet_sha}),
            "t",
        )
        .unwrap();
    assert!(
        t.elapsed() < Duration::from_secs(1),
        "submit must not wait for job completion"
    );
    let first = first["owner_result"].clone();
    assert_eq!(first["state"], "collecting");
    let id = first["operation_id"].clone();
    // Reconnect with a fresh client: same operation, no second admission.
    let repeat = fx.submit(REQ)["owner_result"].clone();
    assert_eq!(repeat["operation_id"], id);
    assert_eq!(
        fx.submit(REQ2)["owner_error"]["code"],
        "authorization_consumed"
    );
    let done = fx.wait_collected();
    assert_eq!(done["owner_result"]["operation_id"], id);
    assert_eq!(fx.runs(), 1);
    let result = fx.read("read_coding_job_result", REQ)["owner_result"].clone();
    assert_eq!(result["terminal"]["status"], "component_passed");
    assert_eq!(result["acceptance"], "owner_pending");
    assert_eq!(result["domain_effect_settlement"], "not_claimed");
    // The actual outer Host wrapper ID survives composition and ties to the operation.
    assert_eq!(result["receipt_id"], RECEIPT_ID);
    assert_eq!(result["operation"]["host_receipt_id"], RECEIPT_ID);
    // The server stops and restarts: reads and repeats still resolve to the original operation.
    assert!(fx.stop().success());
    fx.start();
    assert_eq!(fx.submit(REQ)["owner_result"]["operation_id"], id);
    assert_eq!(
        fx.read("read_coding_job_operation", REQ)["owner_result"]["state"],
        "collected"
    );
    assert_eq!(fx.runs(), 1);
}

#[test]
fn server_stop_during_a_running_job_cancels_collects_and_reconciles_the_same_state() {
    let mut fx = Fx::new();
    std::fs::write(fx.root.join("hang"), b"").unwrap();
    let id = fx.submit(REQ)["owner_result"]["operation_id"].clone();
    thread::sleep(Duration::from_millis(300));
    // Bounded cancellation request never claims cancellation or closure.
    let cancel = fx.read("cancel_coding_job", REQ)["owner_result"].clone();
    assert_eq!(cancel["cancellation_established"], false);
    assert_eq!(cancel["container_closure_established"], false);
    let started = Instant::now();
    assert!(
        fx.stop().success(),
        "owned collector status must be collected before exit"
    );
    assert!(started.elapsed() < Duration::from_secs(20));
    assert!(fx.root.join("cancel-marker").exists());
    std::fs::remove_file(fx.root.join("hang")).unwrap();
    fx.start();
    let after = fx.read("read_coding_job_operation", REQ)["owner_result"].clone();
    assert_eq!(after["operation_id"], id);
    assert_eq!(after["collector_run"]["killed_after_grace"], true);
    assert_eq!(
        after["collector_run"]["process_cleanup"],
        "owned_process_group_collected"
    );
    let done = fx.wait_collected();
    assert_eq!(done["owner_result"]["operation_id"], id);
    assert_eq!(fx.runs(), 1, "reconciliation, never replacement");
}

/// CLI composition through the real `work-engine-command` binary. Requires `COMMAND_BINARY`
/// (built by `cargo build -p work-engine-command`); run with `--ignored`.
#[test]
#[ignore = "requires COMMAND_BINARY pointing at work-engine-command"]
fn command_cli_submits_and_reads_the_same_operation_in_json_and_human_modes() {
    let binary =
        std::env::var("COMMAND_BINARY").expect("COMMAND_BINARY must name work-engine-command");
    let fx = Fx::new();
    let profile = fx.root.join("cli-profile.json");
    private(
        &profile,
        json!({"version":1,"kind":"local-service","endpoint":fx.addr.to_string(),
            "credential_file":fx.cred_file,"timeout_seconds":5})
        .to_string()
        .as_bytes(),
        0o600,
    );
    let submit = fx.root.join("submit.json");
    private(
        &submit,
        json!({"request_id":REQ,"authorization_id":"auth-1","job_credential":CRED,
            "job_sha256":fx.job_sha,"packet_sha256":fx.packet_sha})
        .to_string()
        .as_bytes(),
        0o600,
    );
    let read = fx.root.join("read.json");
    private(
        &read,
        json!({"request_id":REQ,"authorization_id":"auth-1","job_credential":CRED})
            .to_string()
            .as_bytes(),
        0o600,
    );
    let run = |op: &str, mode: &str, file: &Path| {
        let out = Command::new(&binary)
            .args([profile.to_str().unwrap(), op, mode, file.to_str().unwrap()])
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8(out.stdout).unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        )
    };
    let (ok, out, _) = run("submit_coding_job", "--json", &submit);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(ok && v["schema"] == "work-engine.coding-job-command.service.v1" && v["ok"] == true);
    let id = v["envelope"]["owner_result"]["operation_id"].clone();
    let (ok, out, _) = run("read_coding_job_operation", "--human", &read);
    assert!(
        ok && out.contains("Operation: ")
            && out.contains("Settlement: not established by the adapter"),
        "{out}"
    );
    let (_, again, _) = run("submit_coding_job", "--json", &submit);
    assert_eq!(
        serde_json::from_str::<Value>(&again).unwrap()["envelope"]["owner_result"]["operation_id"],
        id
    );
    // Strict client shape: extra field, wrong operation and path-bearing arguments never reach the server.
    let extra = fx.root.join("extra.json");
    private(&extra, json!({"request_id":REQ,"authorization_id":"auth-1","job_credential":CRED,"job_file":"/tmp/x"}).to_string().as_bytes(), 0o600);
    let (ok, out, _) = run("read_coding_job_operation", "--json", &extra);
    assert!(!ok && out.contains("invalid_arguments"));
    let (ok, out, _) = run("run_anything", "--json", &read);
    assert!(!ok && out.contains("invalid_invocation"));
    fx.wait_collected();
    assert_eq!(fx.runs(), 1);
}

/// A server that returns a well-formed transport envelope around an owner result that breaks the
/// published contract must be rejected by the public client, not passed on.
#[test]
fn public_client_rejects_malformed_owner_results() {
    let dir = tempfile::tempdir().unwrap();
    let cred = dir.path().join("bearer");
    private(&cred, BEARER.as_bytes(), 0o600);
    let args = json!({"request_id":REQ,"authorization_id":"auth-1","job_credential":CRED});
    let malformed = [
        json!({"schema":false,"result":"available","operation":null}),
        json!({"schema":"work-engine.coding-job.v1","result":"available","operation":null}),
        json!({"schema":"work-engine.coding-job.v9","result":"not_available","operation":{}}),
        json!({}),
        json!(null),
    ];
    for owner_result in malformed {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body = json!({"schema_version":1,"request_id":"t","server_identity":format!("sha256:{}","1".repeat(64)),
            "server_build":format!("sha256:{}","2".repeat(64)),"owner_result":owner_result})
        .to_string();
        let server = thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut stream, _) = listener.accept().unwrap();
            // Drain the whole request (header, then body) before answering.
            stream
                .set_read_timeout(Some(Duration::from_millis(300)))
                .unwrap();
            let mut buf = [0u8; 8192];
            while stream.read(&mut buf).is_ok_and(|n| n > 0) {}
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        });
        let client =
            LocalClient::new(addr, cred.to_string_lossy().into(), Duration::from_secs(5)).unwrap();
        assert_eq!(
            client.job("read_coding_job_result", args.clone(), "t"),
            Err("invalid_response")
        );
        server.join().unwrap();
    }
}
