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
journal() {{
  J="$(mktemp "$S/.pi-state.json.XXXXXX")" || exit 70
  {{
    if [ "$1" = collected ] && [ -e "$T/pause-publication" ]; then
      : > "$T/publication-open"
      n=0
      while [ -e "$T/pause-publication" ]; do
        n=$((n + 1)); [ "$n" -lt 200 ] || {{ rm -f "$J"; exit 70; }}
        sleep 0.05
      done
    fi
    sed -e "s/@P@/$1/" -e "s/@R@/$2/" "$T/journal.tpl"
  }} > "$J" && mv "$J" "$S/pi-state.json" || {{
    rm -f "$J"
    exit 70
  }}
}}
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
        let t = if let Ok(root) = std::env::var("WORK_ENGINE_EVIDENCE_ROOT") {
            tempfile::Builder::new()
                .prefix("coding-job-")
                .tempdir_in(root)
                .unwrap()
        } else {
            tempfile::tempdir().unwrap()
        };
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
    /// Live-class collector fixture. No provider, credentials or Docker are entered.
    fn enable_live_fixture(&mut self) {
        self.enable_live_fixture_with(false);
    }
    fn enable_live_fixture_with(&mut self, sol: bool) {
        self.stop();
        let mut job: Value =
            serde_json::from_slice(&std::fs::read(self.root.join("job.json")).unwrap()).unwrap();
        let old: work_engine_manual_host::pi_docker::Job =
            serde_json::from_value(job.clone()).unwrap();
        let old_canonical = serde_json::to_string(&old).unwrap();
        let old_digest = hex(old_canonical.as_bytes());
        let public = self.root.join("public");
        private(&public, b"public fixture", 0o600);
        let artifact = json!({"path":public,"sha256":hex(b"public fixture")});
        let mut provider = json!({"lane":{"pi_config":{"root":self.root.join("config"),"manifest":artifact},"native_auth_source":self.root.join("unopened-native-source")},"task":artifact,"catalog":artifact,"access_adapter":artifact,"model":"gpt-6-luna","max_requests":4,"budget_microusd":700000,"max_context_bytes":24000});
        if sol {
            provider["model"] = json!("gpt-6.1-sol");
            provider["budget_microusd"] = json!(17_680_004u64);
            provider["sol_high"] = json!({"schema":"work-engine.sol-high.v1","reasoning_effort":"high","payer_account_id":"fixture-payer"});
        }
        job["coding"]["schema"] = json!(if sol {
            work_engine_manual_host::pi_docker::coding::SOL_HIGH_BRIDGE_PROFILE
        } else {
            work_engine_manual_host::pi_docker::coding::LIVE_BRIDGE_PROFILE
        });
        job["coding"]["workspace_write_boundary"] = json!("src_lib_rs");
        job["coding"]["fixture_command"] = json!("");
        job["coding"]["provider"] = provider.clone();
        for key in ["shim", "acceptance"] {
            job["coding"][key] = artifact.clone();
        }
        job["coding"]["toolchain"]["manifest"] = artifact.clone();
        job["runtime"]["manifest"] = artifact.clone();
        job["workspace"]["manifest"] = artifact.clone();
        let mut profile: Value =
            serde_json::from_slice(&std::fs::read(self.root.join("job-profile.json")).unwrap())
                .unwrap();
        let mut auth = json!({"schema":"work-engine.pi-coding-runtime-admission.v1","binary_sha256":profile["host_binary_sha256"],"job_id":"job-1","model":"gpt-6-luna","task_sha256":artifact["sha256"],"catalog_sha256":artifact["sha256"],"native_auth_source":provider["lane"]["native_auth_source"],"pi_config_manifest_sha256":artifact["sha256"],"max_requests":4,"budget_microusd":700000,"allow_provider":true,"allow_native_access_projection":true});
        if sol {
            auth["schema"] = json!("work-engine.pi-coding-runtime-admission.v2");
            auth["model"] = provider["model"].clone();
            auth["budget_microusd"] = provider["budget_microusd"].clone();
            auth["sol_high"] = provider["sol_high"].clone();
        }
        let auth = auth.to_string();
        private(&self.root.join("authority.json"), auth.as_bytes(), 0o600);
        job["authority"] =
            json!({"path":self.root.join("authority.json"),"sha256":hex(auth.as_bytes())});
        let packet = json!({"schema":"work-engine.coding-live-continuity-packet.v1","bounds":{"inference_containers":1,"tool_containers":1,"validator_containers":1,"sdk_sessions":2,"workspace_exec_calls":1,"fake_provider_requests":0,"physical_resets":1,"live_provider_requests":4}}).to_string();
        private(&self.root.join("packet.json"), packet.as_bytes(), 0o600);
        self.packet_sha = hex(packet.as_bytes());
        job["packet"]["sha256"] = json!(self.packet_sha);
        let typed: work_engine_manual_host::pi_docker::Job =
            serde_json::from_value(job.clone()).unwrap();
        let canonical = serde_json::to_string(&typed).unwrap();
        let digest = hex(canonical.as_bytes());
        for name in ["journal.tpl", "receipt.json"] {
            let path = self.root.join(name);
            let text = std::fs::read_to_string(&path)
                .unwrap()
                .replace(&old_canonical, &canonical)
                .replace(&old_digest, &digest);
            private(&path, text.as_bytes(), 0o600);
        }
        let bytes = serde_json::to_vec_pretty(&job).unwrap();
        private(&self.root.join("job.json"), &bytes, 0o600);
        self.job_sha = hex(&bytes);
        profile["version"] = json!(2);
        profile["capability"] = json!("coding-job-local-live-v1");
        profile["class"] = json!("local-live");
        profile["authorizations"][0]["job_sha256"] = json!(self.job_sha);
        profile["authorizations"][0]["packet_sha256"] = json!(self.packet_sha);
        profile["authorizations"][0]["host_input_digest"] = json!(digest);
        private(
            &self.root.join("job-profile.json"),
            profile.to_string().as_bytes(),
            0o600,
        );
        let mut config: Value =
            serde_json::from_slice(&std::fs::read(&self.config).unwrap()).unwrap();
        config["coding_job_capability"] = json!("coding-job-local-live-v1");
        private(&self.config, config.to_string().as_bytes(), 0o600);
        self.start();
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
            if let Some(status) = self.server.as_mut().unwrap().try_wait().unwrap() {
                let mut error = String::new();
                if let Some(mut stderr) = self.server.as_mut().unwrap().stderr.take() {
                    std::io::Read::read_to_string(&mut stderr, &mut error).unwrap();
                }
                panic!("server exited {status}: {error}");
            }
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
        self.wait_request(REQ)
    }
    fn wait_request(&self, request: &str) -> Value {
        for _ in 0..1200 {
            let v = self.read("read_coding_job_operation", request);
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
        eprintln!(
            "CLI {op} {mode}: exit={:?}; stdout={}; stderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
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
    let (ok, out, err) = run("read_coding_job_operation", "--human", &read);
    assert!(
        ok && out.contains("Operation: ")
            && out.contains("Settlement: not established by the adapter"),
        "stdout: {out}; stderr: {err}"
    );
    let (again_ok, again, again_err) = run("submit_coding_job", "--json", &submit);
    assert!(
        again_ok,
        "repeat submit failed: stdout: {again}; stderr: {again_err}"
    );
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

/// Pause the actual Host writer after opening its output, before producing collected JSON.
/// This proves publication behavior in the fixture, not the cause of a historical failure.
#[test]
#[ignore = "requires COMMAND_BINARY pointing at work-engine-command"]
fn command_repeat_during_host_journal_publication_preserves_identity() {
    let binary = std::env::var("COMMAND_BINARY").expect("COMMAND_BINARY is required");
    let fx = Fx::new();
    let profile = fx.root.join("cli-profile.json");
    private(
        &profile,
        json!({"version":1,"kind":"local-service",
        "endpoint":fx.addr.to_string(),"credential_file":fx.cred_file,
        "timeout_seconds":5})
        .to_string()
        .as_bytes(),
        0o600,
    );
    let submit = fx.root.join("submit.json");
    private(
        &submit,
        json!({"request_id":REQ,"authorization_id":"auth-1",
        "job_credential":CRED,"job_sha256":fx.job_sha,
        "packet_sha256":fx.packet_sha})
        .to_string()
        .as_bytes(),
        0o600,
    );
    let run = || {
        Command::new(&binary)
            .args([
                profile.to_str().unwrap(),
                "submit_coding_job",
                "--json",
                submit.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let pause = fx.root.join("pause-publication");
    std::fs::write(&pause, b"").unwrap();
    let first = run();
    assert!(first.status.success(), "first submit: {first:?}");
    let first_reply: Value = serde_json::from_slice(&first.stdout).unwrap();
    let id = first_reply["envelope"]["owner_result"]["operation_id"].clone();
    assert!(id.is_string(), "{first_reply}");
    let journal = fx.root.join("state/pi-state.json");
    let until = Instant::now() + Duration::from_secs(8);
    while !fx.root.join("publication-open").exists() {
        assert!(
            Instant::now() < until,
            "Host writer never reached publication barrier"
        );
        thread::sleep(Duration::from_millis(10));
    }
    let observed = std::fs::read(&journal).unwrap();
    let again = run();
    let reply: Value = serde_json::from_slice(&again.stdout).unwrap();
    let capture = json!({"original_operation_id":id,"host_runs":fx.runs(),
        "first_exit":first.status.code(),"first_stdout":String::from_utf8_lossy(&first.stdout),
        "first_stderr":String::from_utf8_lossy(&first.stderr),
        "exit":again.status.code(),"stdout":String::from_utf8_lossy(&again.stdout),
        "stderr":String::from_utf8_lossy(&again.stderr),"native_reply":reply,
        "journal_while_writer_paused":String::from_utf8_lossy(&observed),
        "host_script":std::fs::read_to_string(fx.root.join("host.sh")).unwrap(),
        "journal_template":std::fs::read_to_string(fx.root.join("journal.tpl")).unwrap(),
        "command_sha256":hex(&std::fs::read(&binary).unwrap())});
    println!("publication capture: {capture}");
    if let Ok(path) = std::env::var("JOURNAL_PUBLICATION_CAPTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&capture).unwrap()).unwrap();
    }
    // Release before assertions so even the expected before-fix failure collects the Host.
    std::fs::remove_file(pause).unwrap();
    let done = fx.wait_collected();
    assert_eq!(fx.runs(), 1, "{capture}");
    assert_eq!(done["owner_result"]["operation_id"], id, "{capture}");
    assert!(again.status.success(), "{capture}");
    let visible: Value =
        serde_json::from_slice(&observed).expect("last valid journal stays visible");
    assert_eq!(visible["phase"], "running");
    assert_eq!(visible["removed"], false);
    assert_eq!(
        reply["envelope"]["owner_result"]["operation_id"], id,
        "{capture}"
    );
    assert_eq!(
        reply["envelope"]["owner_result"]["entry"],
        "existing_operation"
    );
}

/// Actual Events HTTP MCP -> command process -> server job owner -> existing fake Host.
/// The unconditional verifier and Host are fixtures; this does not qualify OAuth or live coding.
#[test]
#[ignore = "requires COMMAND_BINARY pointing at work-engine-command"]
fn events_http_mcp_preserves_the_real_job_operation_across_server_restart() {
    events_job_bridge(false);
}
#[test]
#[ignore = "requires COMMAND_BINARY pointing at work-engine-command"]
fn events_http_mcp_live_class_fixture_preserves_the_original_operation_across_restart() {
    events_job_bridge(true);
}
fn events_job_bridge(live: bool) {
    use std::io::{Read, Write};

    struct Events(Child);
    impl Drop for Events {
        fn drop(&mut self) {
            unsafe { libc::kill(self.0.id() as i32, libc::SIGTERM) };
            let _ = self.0.wait();
        }
    }
    fn rpc(addr: SocketAddr, name: &str, args: Value, authenticated: bool) -> (u16, Value) {
        let body = json!({"jsonrpc":"2.0","id":1,"method":name,"params":args}).to_string();
        let mut stream = std::net::TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        let authorization = if authenticated {
            "Authorization: Bearer fixture-token\r\n"
        } else {
            ""
        };
        write!(stream, "POST /mcp HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n{authorization}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
        (
            status,
            if body.is_empty() {
                Value::Null
            } else {
                serde_json::from_str(body).unwrap()
            },
        )
    }
    fn call(addr: SocketAddr, name: &str, arguments: Value) -> Value {
        let (status, reply) = rpc(
            addr,
            "tools/call",
            json!({"name":name,"arguments":arguments}),
            true,
        );
        assert_eq!(status, 200, "{reply}");
        if reply["result"]["isError"] == true {
            let failure: Value =
                serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap())
                    .unwrap();
            return failure["error"].clone();
        }
        reply["result"]["structuredContent"].clone()
    }
    fn native(reply: &Value) -> &Value {
        assert_eq!(reply["disposition"], "native_result", "{reply}");
        &reply["native_envelope"]["owner_result"]
    }

    let binary =
        std::env::var("COMMAND_BINARY").expect("COMMAND_BINARY must name work-engine-command");
    let mut fx = Fx::new();
    if live {
        fx.enable_live_fixture();
    }
    // Cargo's executable can be hard-linked to its deps entry. Match the existing
    // bridge fixtures: stage a private single-link copy before pinning it.
    let staged_binary = fx.root.join("work-engine-command");
    std::fs::copy(&binary, &staged_binary).unwrap();
    std::fs::set_permissions(&staged_binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let binary = staged_binary;
    let command_profile = fx.root.join("mcp-command-profile.json");
    private(
        &command_profile,
        json!({"version":1,"kind":"local-service","endpoint":fx.addr.to_string(),
        "credential_file":fx.cred_file,"timeout_seconds":5})
        .to_string()
        .as_bytes(),
        0o600,
    );
    let bridge_profile = fx.root.join("mcp-bridge-profile.json");
    private(&bridge_profile, json!({"schema":"work-engine.mcp-coding-job-bridge.v1",
        "subject":"fixture-subject","principal":"fixture-principal","client":"fixture-caller",
        "command_profile":command_profile,"command_profile_sha256":hex(&std::fs::read(&command_profile).unwrap()),
        "authorization_ids":["auth-1"],"timeout_seconds":10}).to_string().as_bytes(), 0o600);
    let verifier = fx.root.join("fixture-verifier.sh");
    private(&verifier, b"#!/bin/sh\nprintf 'allow\\n'\n", 0o700);
    let auth = fx.root.join("mcp-auth.json");
    private(&auth, json!({"resource":"https://mcp.fixture.invalid/mcp","issuer":"https://issuer.fixture.invalid",
        "write_scope":"events.write","verifier":verifier,"subject":"fixture-subject"}).to_string().as_bytes(), 0o600);
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut events = Events(
        Command::new("/usr/bin/python3")
            .arg(fx.root.join("owner.py"))
            .args([
                "--codex-home",
                fx.root.join("codex").to_str().unwrap(),
                "--claude-home",
                fx.root.join("claude").to_str().unwrap(),
                "--state-dir",
                fx.root.join("mcp-state").to_str().unwrap(),
                "serve-http",
                "--listen",
                &addr.to_string(),
                "--path",
                "/mcp",
                "--auth-config",
                auth.to_str().unwrap(),
                "--coding-job-only",
                "--principal",
                "fixture-principal",
            ])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("CODING_JOB_BRIDGE_ENABLED", "1")
            .env("CODING_JOB_BINARY", &binary)
            .env(
                "CODING_JOB_BINARY_SHA256",
                hex(&std::fs::read(&binary).unwrap()),
            )
            .env("CODING_JOB_CONFIG", &bridge_profile)
            .env(
                "CODING_JOB_CONFIG_SHA256",
                hex(&std::fs::read(&bridge_profile).unwrap()),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if std::net::TcpStream::connect(addr).is_ok() {
            break;
        }
        if let Some(status) = events.0.try_wait().unwrap() {
            let mut error = String::new();
            events
                .0
                .stderr
                .take()
                .unwrap()
                .read_to_string(&mut error)
                .unwrap();
            panic!("Events exited {status}: {error}");
        }
        assert!(Instant::now() < until, "Events startup timed out");
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(rpc(addr, "tools/list", json!({}), false).0, 401);
    let (status, initialized) = rpc(
        addr,
        "initialize",
        json!({"protocolVersion":"2025-06-18",
        "capabilities":{},"clientInfo":{"name":"job-composition-test","version":"1"}}),
        true,
    );
    assert_eq!(status, 200, "{initialized}");
    assert!(
        initialized["result"]["protocolVersion"].is_string(),
        "{initialized}"
    );
    let (_, listed) = rpc(addr, "tools/list", json!({}), true);
    let mut names: Vec<_> = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "cancel_coding_job",
            "read_coding_job_operation",
            "read_coding_job_result",
            "submit_coding_job"
        ]
    );
    let read = json!({"request_id":REQ,"authorization_id":"auth-1","job_credential":CRED});
    let submit = json!({"request_id":REQ,"authorization_id":"auth-1","job_credential":CRED,
        "job_sha256":fx.job_sha,"packet_sha256":fx.packet_sha});
    let first = call(addr, "submit_coding_job", submit.clone());
    let id = native(&first)["operation_id"].clone();
    assert!(id.is_string(), "{first}");
    fx.wait_collected();
    // Repeat only after collection: this test does not settle the separately recorded parallel race.
    let again = call(addr, "submit_coding_job", submit);
    assert_eq!(native(&again)["operation_id"], id);
    assert_eq!(native(&again)["entry"], "existing_operation");
    assert!(fx.stop().success());
    fx.start();
    let observed = call(addr, "read_coding_job_operation", read.clone());
    assert_eq!(native(&observed)["operation_id"], id);
    let result = call(addr, "read_coding_job_result", read);
    assert_eq!(native(&result)["operation"]["operation_id"], id);
    assert_eq!(native(&result)["acceptance"], "owner_pending");
    assert_eq!(native(&result)["domain_effect_settlement"], "not_claimed");
    let denied = call(
        addr,
        "read_coding_job_operation",
        json!({"request_id":REQ,
        "authorization_id":"auth-1","job_credential":"wrong-credential-0123456789abcdefghijklmnop"}),
    );
    assert_eq!(denied["disposition"], "native_owner_error", "{denied}");
    assert_eq!(denied["entry_uncertain"], "not_established");
    assert_eq!(
        fx.runs(),
        1,
        "MCP retry and server restart must not launch another Host"
    );
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

/// Replay immutable actual producer stdout through the real consumer processes only.
/// The prebound operator record and shell producer are fixtures: admission/provider execution
/// is not repeated. The original job/journal/receipt are read only and retain their identities.
#[test]
#[ignore = "requires COMMAND_BINARY and RAW_CODING_HOST_RESULT from the immutable pilot"]
fn actual_receipt_numbers_survive_capture_command_and_restart() {
    let binary = std::env::var("COMMAND_BINARY").expect("COMMAND_BINARY");
    let original =
        PathBuf::from(std::env::var("RAW_CODING_HOST_RESULT").expect("RAW_CODING_HOST_RESULT"));
    let raw = std::fs::read(&original).unwrap();
    let wrapper: Value = serde_json::from_slice(&raw).unwrap();
    let receipt_id = "42ba964cb7c9f0430fb16828fdb94de18cfa1fc58d199cdf1b7ece9b8e6b26c6";
    assert_eq!(wrapper["receipt_id"], receipt_id);
    let original_state = PathBuf::from(wrapper["receipt"]["job"]["state_dir"].as_str().unwrap());
    let receipt_path = original_state
        .join("receipts")
        .join(format!("{receipt_id}.json"));
    let receipt_bytes = std::fs::read(&receipt_path).unwrap();
    assert_eq!(hex(&receipt_bytes), receipt_id);
    let original_journal = std::fs::read(original_state.join("pi-state.json")).unwrap();
    let mut fx = Fx::new();
    fx.stop();
    let host = fx.root.join("host.sh");
    private(&fx.root.join("raw-host-result.json"), &raw, 0o600);
    private(
        &fx.root.join("original-journal.json"),
        &original_journal,
        0o600,
    );
    private(
        &host,
        format!(
            "#!/bin/sh\ncase \"$2\" in\nrun) cat '{}';;\ninspect) cat '{}';;\n*) exit 64;;\nesac\n",
            fx.root.join("raw-host-result.json").display(),
            fx.root.join("original-journal.json").display()
        )
        .as_bytes(),
        0o700,
    );
    let job = &wrapper["receipt"]["job"];
    let job_bytes = std::fs::read(original_state.parent().unwrap().join("job.json")).unwrap();
    private(&fx.root.join("original-job.json"), &job_bytes, 0o600);
    let job_sha = hex(&job_bytes);
    let host_sha = hex(&std::fs::read(&host).unwrap());
    let input = wrapper["receipt"]["input_digest"].as_str().unwrap();
    let packet_sha = job["packet"]["sha256"].as_str().unwrap();
    let operation_id = hex(format!(
        "work-engine.coding-job.operation.v1\0auth-1\0{job_sha}\0{host_sha}"
    )
    .as_bytes());
    let opdir = fx.root.join("ops").join(&operation_id);
    std::fs::create_dir(&opdir).unwrap();
    std::fs::set_permissions(&opdir, std::fs::Permissions::from_mode(0o700)).unwrap();
    private(&opdir.join("job.json"), &job_bytes, 0o600);
    private(&opdir.join("binding.json"), json!({"schema":"work-engine.coding-job.v1","operation_id":operation_id,"request_id":REQ,"authorization_id":"auth-1","caller":"caller-1","job_id":job["job_id"],"state_dir":original_state,"job_sha256":job_sha,"packet_sha256":packet_sha,"host_input_digest":input,"host_job_schema":"work-engine.pi-docker-custody.v1","host_binary_sha256":host_sha,"capability":"coding-job-offline-fixture-v1","class":"offline-fixture","bound_at_unix_seconds":0}).to_string().as_bytes(), 0o600);
    let mut profile: Value =
        serde_json::from_slice(&std::fs::read(fx.root.join("job-profile.json")).unwrap()).unwrap();
    profile["host_binary_sha256"] = json!(host_sha);
    let auth = &mut profile["authorizations"][0];
    auth["job_file"] = json!(fx.root.join("original-job.json"));
    auth["job_sha256"] = json!(job_sha);
    auth["packet_sha256"] = json!(packet_sha);
    auth["job_id"] = job["job_id"].clone();
    auth["state_dir"] = json!(original_state);
    auth["host_input_digest"] = json!(input);
    private(
        &fx.root.join("job-profile.json"),
        profile.to_string().as_bytes(),
        0o600,
    );
    fx.job_sha = job_sha;
    fx.packet_sha = packet_sha.into();
    fx.start();
    fx.submit(REQ);
    fx.wait_collected();
    let profile = fx.root.join("cli-profile.json");
    private(&profile, json!({"version":1,"kind":"local-service","endpoint":fx.addr.to_string(),"credential_file":fx.cred_file,"timeout_seconds":5}).to_string().as_bytes(), 0o600);
    let args = fx.root.join("read.json");
    private(
        &args,
        json!({"request_id":REQ,"authorization_id":"auth-1","job_credential":CRED})
            .to_string()
            .as_bytes(),
        0o600,
    );
    let assert_numbers = |receipt: &Value| {
        for (pointer, expected) in [
            (
                "/coding/provider_requests/1/usage/cost/output",
                "0.00020549999999999998",
            ),
            (
                "/coding/provider_requests/2/usage/cost/input",
                "0.00037610000000000003",
            ),
            (
                "/coding/provider_requests/2/usage/cost/total",
                "0.00039660000000000004",
            ),
            (
                "/coding/provider_requests/4/usage/cost/output",
                "2.4999999999999998e-6",
            ),
        ] {
            assert_eq!(
                receipt.pointer(pointer).unwrap().to_string(),
                expected,
                "{pointer}"
            );
        }
    };
    for restart in [false, true] {
        if restart {
            assert!(fx.stop().success());
            fx.start();
        }
        let captured: Value =
            serde_json::from_slice(&std::fs::read(opdir.join("host-run.json")).unwrap()).unwrap();
        assert_eq!(captured["receipt_id"], receipt_id);
        assert_numbers(&captured["receipt"]);
        let output = Command::new(&binary)
            .args([
                profile.to_str().unwrap(),
                "read_coding_job_result",
                "--json",
                args.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        let result = &response["envelope"]["owner_result"];
        assert_eq!(result["receipt_id"], receipt_id);
        assert_numbers(&result["receipt"]);
        assert_eq!(result["receipt"], wrapper["receipt"]);
    }
    assert_eq!(std::fs::read(&receipt_path).unwrap(), receipt_bytes);
    assert_eq!(
        std::fs::read(original_state.join("pi-state.json")).unwrap(),
        original_journal
    );
    assert!(!opdir.join("collector-reconcile.json").exists());
}

#[test]
#[ignore = "requires explicit custody-built WORK_ENGINE_OWNED_HOST fixture"]
fn resistant_owned_child_deadline_quiet_control_and_restart_keep_independent_effect_count() {
    let host = std::env::var("WORK_ENGINE_OWNED_HOST").unwrap();
    for (hanging, interrupted) in [(false, false), (true, false), (true, true)] {
        let mut fx = Fx::new();
        fx._t.disable_cleanup(true);
        println!("retained_collector_case={}", fx.root.display());
        assert!(fx.stop().success());
        let profile_path = fx.root.join("job-profile.json");
        let mut p: Value = serde_json::from_slice(&std::fs::read(&profile_path).unwrap()).unwrap();
        p["host_binary"] = json!(host);
        p["host_binary_sha256"] = json!(hex(&std::fs::read(&host).unwrap()));
        p["env"] = json!({"FIXTURE_ROOT":fx.root});
        p["collector_deadline_seconds"] = json!(if interrupted {
            20
        } else if hanging {
            2
        } else {
            5
        });
        private(&profile_path, &serde_json::to_vec(&p).unwrap(), 0o600);
        if hanging {
            std::fs::write(fx.root.join("hang"), b"").unwrap();
        }
        fx.start();
        let id = fx.submit(REQ)["owner_result"]["operation_id"].clone();
        if hanging {
            let until = Instant::now() + Duration::from_secs(2);
            while !fx.root.join("owned-pid").exists() {
                assert!(Instant::now() < until);
                thread::sleep(Duration::from_millis(10));
            }
            let pid: i32 = std::fs::read_to_string(fx.root.join("owned-pid"))
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(unsafe { libc::kill(pid, libc::SIGTERM) }, 0);
            thread::sleep(Duration::from_millis(100));
            assert_eq!(
                unsafe { libc::kill(pid, 0) },
                0,
                "fixture resists actual graceful signal"
            );
        }
        if interrupted {
            assert!(fx.stop().success());
            fx.start();
        }
        let done = fx.wait_collected();
        assert_eq!(done["owner_result"]["operation_id"], id);
        let op = fx.read("read_coding_job_operation", REQ)["owner_result"].clone();
        std::fs::write(
            fx.root.join("exact-operation.json"),
            serde_json::to_vec(&op).unwrap(),
        )
        .unwrap();
        let result = fx.read("read_coding_job_result", REQ);
        std::fs::write(
            fx.root.join("exact-result.json"),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
        assert_eq!(
            op["collector_run"]["collector_deadline_exceeded"],
            hanging && !interrupted
        );
        assert_eq!(op["collector_run"]["server_stop_requested"], interrupted);
        assert_eq!(op["collector_run"]["killed_after_grace"], hanging);
        assert_eq!(op["collector_run"]["status_collected"], true);
        assert_eq!(
            op["collector_run"]["process_cleanup"],
            "owned_process_group_collected"
        );
        assert_eq!(
            std::fs::read_to_string(fx.root.join("effects")).unwrap(),
            "entered\n"
        );
        if hanging {
            let artifact = &op["collector_run"]["stdout_artifact"];
            let bytes = std::fs::read(artifact["path"].as_str().unwrap()).unwrap();
            assert_eq!(hex(&bytes), artifact["sha256"]);
            assert_eq!(bytes, b"partial-before-deadline\n");
            let artifact = &op["collector_run"]["stderr_artifact"];
            let bytes = std::fs::read(artifact["path"].as_str().unwrap()).unwrap();
            assert_eq!(hex(&bytes), artifact["sha256"]);
            assert_eq!(bytes, b"partial-stderr-before-deadline\n");
        }
        assert!(fx.stop().success());
        fx.start();
        assert_eq!(fx.submit(REQ)["owner_result"]["operation_id"], id);
        assert_eq!(
            std::fs::read_to_string(fx.root.join("effects")).unwrap(),
            "entered\n"
        );
        assert_eq!(fx.runs(), 1);
    }
}

/// Fresh operator-authored inputs and authority, prepared without editing generated JSON.
fn prepared_docker_fixture(
    fx: &mut Fx,
    ordinal: u8,
    command: &str,
    timeout: u64,
) -> (PathBuf, PathBuf) {
    assert!(fx.stop().success());
    let runtime = std::env::var("WORK_ENGINE_PI_ROOT").unwrap();
    let toolchain = std::env::var("WORK_ENGINE_RUSTC_ROOT").unwrap();
    let source_host = std::env::var("WORK_ENGINE_HOST_BINARY").unwrap();
    // Keep this exact producing binary available after later repository rebuilds.
    let host = fx.root.join("pinned-host");
    std::fs::copy(source_host, &host).unwrap();
    std::fs::set_permissions(&host, std::fs::Permissions::from_mode(0o700)).unwrap();
    let work = fx.root.join("prepared-workspace");
    std::fs::create_dir_all(work.join("src")).unwrap();
    std::fs::write(work.join("src/lib.rs"), b"// independent effect counter\n").unwrap();
    let task = fx.root.join("task.txt");
    std::fs::write(
        &task,
        b"Deterministically append the answer function once; no inference.",
    )
    .unwrap();
    let validator = fx.root.join("acceptance.rs");
    std::fs::write(&validator,b"#[path=\"/workspace/src/lib.rs\"] mod task; #[test] fn answer(){assert_eq!(task::answer(),42);}").unwrap();
    let state = fx.root.join("prepared-state");
    let out = fx.root.join("prepared-bundle");
    let spec = json!({"schema":"work-engine.coding-preparation.v1","job_id":format!("prepared-{ordinal}"),"output_dir":out,"state_dir":state,"operation_root":fx.root.join("prepared-ops"),"host_binary":host,"task":task,"workspace":work,"validator":validator,"runtime":runtime,"toolchain":toolchain,"docker":"/usr/bin/docker","image":"node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da","deadline_unix_seconds":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()+180,"command_timeout_seconds":timeout,"collector_deadline_seconds":120,"cancel_grace_seconds":3,"fixture_command":command});
    let spec_path = fx.root.join("preparation.json");
    private(&spec_path, &serde_json::to_vec(&spec).unwrap(), 0o600);
    let prepare = env!("CARGO_BIN_EXE_coding-job-prepare");
    let inspected = Command::new(prepare)
        .args(["inspect", spec_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        inspected.status.success(),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    let inspected: Value = serde_json::from_slice(&inspected.stdout).unwrap();
    // This fixture acts as a separately trusted operator; preparation never writes this grant.
    let authority = json!({"schema":"work-engine.coding-preparation-authority.v1","spec_sha256":inspected["spec_sha256"],"inputs_sha256":inspected["inputs_sha256"],"job_id":spec["job_id"],"authorization_id":"auth-1","caller":"caller-1","credential_sha256":hex(CRED.as_bytes()),"allow_prepare":true,"allow_provider":false});
    let authority_path = fx.root.join("preparation-authority.json");
    private(
        &authority_path,
        &serde_json::to_vec(&authority).unwrap(),
        0o600,
    );
    // Mutation after inspection must fail without producing a bundle.
    std::fs::write(&task, b"changed input").unwrap();
    let denied = Command::new(prepare)
        .args([
            "prepare",
            spec_path.to_str().unwrap(),
            authority_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!denied.status.success());
    assert!(!out.exists());
    std::fs::write(
        &task,
        b"Deterministically append the answer function once; no inference.",
    )
    .unwrap();
    let prepared = Command::new(prepare)
        .args([
            "prepare",
            spec_path.to_str().unwrap(),
            authority_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        prepared.status.success(),
        "{}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    let prepared: Value = serde_json::from_slice(&prepared.stdout).unwrap();
    assert_eq!(prepared["provider_execution"], false);
    let job: work_engine_manual_host::pi_docker::Job =
        serde_json::from_slice(&std::fs::read(out.join("job.json")).unwrap()).unwrap();
    assert!(job.coding.as_ref().unwrap().provider.is_none());
    assert_eq!(
        hex(&serde_json::to_vec(&job).unwrap()),
        prepared["host_input_digest"]
    );
    fx.job_sha = prepared["job"]["sha256"].as_str().unwrap().into();
    fx.packet_sha = prepared["packet_sha256"].as_str().unwrap().into();
    let mut config: Value = serde_json::from_slice(&std::fs::read(&fx.config).unwrap()).unwrap();
    config["coding_job_profile"] = prepared["job_profile"]["path"].clone();
    private(&fx.config, &serde_json::to_vec(&config).unwrap(), 0o600);
    std::fs::write(
        fx.root.join("preparation-result.json"),
        serde_json::to_vec(&prepared).unwrap(),
    )
    .unwrap();
    fx.start();
    (out, state)
}

#[test]
#[ignore = "requires explicit pinned existing Docker/Pi/rustc assets and Host/command binaries"]
fn two_fresh_prepared_docker_jobs_quiet_control_and_exact_command_replay() {
    let mut operations = Vec::new();
    for ordinal in 0..2 {
        let request = if ordinal == 0 { REQ } else { REQ2 };
        let mut fx = Fx::new();
        fx._t.disable_cleanup(true);
        println!("retained_actual_case={}", fx.root.display());
        let edit = "printf 'pub fn answer() -> u32 { 42 } // EFFECT-ONCE\\n' >> src/lib.rs";
        let command = if ordinal == 1 {
            format!("sleep 2; {edit}")
        } else {
            edit.into()
        };
        let (_bundle, state) = prepared_docker_fixture(&mut fx, ordinal, &command, 10);
        let cli = std::env::var("COMMAND_BINARY").unwrap();
        let cli_profile = fx.root.join("prepared-cli.json");
        private(&cli_profile,json!({"version":1,"kind":"local-service","endpoint":fx.addr.to_string(),"credential_file":fx.cred_file,"timeout_seconds":5}).to_string().as_bytes(),0o600);
        let submit = fx.root.join("prepared-submit.json");
        private(&submit,json!({"request_id":request,"authorization_id":"auth-1","job_credential":CRED,"job_sha256":fx.job_sha,"packet_sha256":fx.packet_sha}).to_string().as_bytes(),0o600);
        let submitted = Command::new(&cli)
            .args([
                cli_profile.to_str().unwrap(),
                "submit_coding_job",
                "--json",
                submit.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            submitted.status.success(),
            "{}",
            String::from_utf8_lossy(&submitted.stdout)
        );
        let value: Value = serde_json::from_slice(&submitted.stdout).unwrap();
        let id = value["owner"]["operation_id"].clone();
        fx.wait_request(request);
        let result = fx.read("read_coding_job_result", request);
        std::fs::write(
            fx.root.join("exact-result.json"),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
        println!("actual_result={result}");
        assert_eq!(
            result["owner_result"]["terminal"]["status"],
            "component_passed"
        );
        assert_eq!(result["owner_result"]["acceptance"], "owner_pending");
        let source = state.join("coding-workspace/src/lib.rs");
        let counter = std::fs::read_to_string(&source).unwrap();
        assert_eq!(counter.matches("EFFECT-ONCE").count(), 1);
        assert!(fx.stop().success());
        fx.start();
        let repeated = fx.submit(request);
        assert_eq!(
            repeated["owner_result"]["operation_id"],
            result["owner_result"]["operation"]["operation_id"]
        );
        assert_eq!(std::fs::read_to_string(&source).unwrap(), counter);
        operations.push(result["owner_result"]["operation"]["operation_id"].clone());
        let _ = id;
    }
    assert_ne!(
        operations[0], operations[1],
        "fresh isolated jobs have distinct operation identities"
    );
}

#[test]
#[ignore = "requires explicit pinned existing Docker/Pi/rustc assets and Host binary"]
fn prepared_workspace_hang_deadline_retains_partial_output_and_closes_exact_containers() {
    let mut fx = Fx::new();
    fx._t.disable_cleanup(true);
    println!("retained_workspace_fault={}", fx.root.display());
    let command = "trap '' TERM; printf '// EFFECT-ONCE\\n' >> src/lib.rs; printf 'partial-workspace-hang\\n'; while :; do sleep 1; done";
    let (_bundle, state) = prepared_docker_fixture(&mut fx, 2, command, 2);
    let first = fx.submit(REQ)["owner_result"].clone();
    let id = first["operation_id"].clone();
    fx.wait_collected();
    let result = fx.read("read_coding_job_result", REQ);
    std::fs::write(
        fx.root.join("exact-fault-result.json"),
        serde_json::to_vec(&result).unwrap(),
    )
    .unwrap();
    let result = &result["owner_result"];
    assert_eq!(result["terminal"]["status"], "not_established");
    assert_eq!(result["cleanup"]["removed"], true);
    assert_eq!(result["cleanup"]["tool_removed"], true);
    assert_eq!(result["acceptance"], "owner_pending");
    let processes = result["receipt"]["processes"].as_array().unwrap();
    let execution = processes
        .iter()
        .find(|p| {
            p["stdout"]
                .as_str()
                .is_some_and(|s| s.contains("-coding-tool-exec-"))
        })
        .unwrap();
    assert_eq!(
        execution["timed_out"], true,
        "workspace command deadline observed directly"
    );
    let bytes = std::fs::read(execution["stdout"].as_str().unwrap()).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("partial-workspace-hang"));
    assert_eq!(hex(&bytes), execution["stdout_sha256"]);
    let journal: Value =
        serde_json::from_slice(&std::fs::read(state.join("pi-state.json")).unwrap()).unwrap();
    let name = journal["container_name"].as_str().unwrap();
    for suffix in ["", "-tool", "-validation"] {
        let filter = format!("name=^/{name}{suffix}$");
        let observed = Command::new("/usr/bin/docker")
            .args([
                "ps",
                "--all",
                "--no-trunc",
                "--filter",
                &filter,
                "--format",
                "{{.ID}}",
            ])
            .output()
            .unwrap();
        assert!(observed.status.success());
        assert!(
            observed.stdout.is_empty(),
            "exact selected container remains"
        );
        std::fs::write(
            fx.root.join(format!("absence{suffix}.stdout")),
            observed.stdout,
        )
        .unwrap();
    }
    let source = state.join("coding-workspace/src/lib.rs");
    let counter = std::fs::read_to_string(&source).unwrap();
    assert_eq!(counter.matches("EFFECT-ONCE").count(), 1);
    assert!(fx.stop().success());
    fx.start();
    assert_eq!(fx.submit(REQ)["owner_result"]["operation_id"], id);
    assert_eq!(std::fs::read_to_string(source).unwrap(), counter);
}

#[test]
fn sol_high_frontend_preserves_exact_profile_identity_and_original_receipt() {
    let mut fx = Fx::new();
    fx.enable_live_fixture_with(true);
    let id = fx.submit(REQ)["owner_result"]["operation_id"].clone();
    assert!(id.is_string());
    fx.wait_collected();
    let result = fx.read("read_coding_job_result", REQ);
    assert_eq!(
        result["owner_result"]["receipt"]["job"]["coding"]["schema"],
        work_engine_manual_host::pi_docker::coding::SOL_HIGH_BRIDGE_PROFILE
    );
    assert_eq!(
        result["owner_result"]["receipt"]["job"]["coding"]["provider"]["sol_high"]["reasoning_effort"],
        "high"
    );
    assert_eq!(
        result["owner_result"]["receipt"]["job"]["coding"]["provider"]["budget_microusd"],
        17_680_004u64
    );
    assert_eq!(result["owner_result"]["acceptance"], "owner_pending");
    assert!(fx.stop().success());
    fx.start();
    assert_eq!(fx.submit(REQ)["owner_result"]["operation_id"], id);
}
