#![recursion_limit = "256"]

mod common;

use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    os::unix::{fs::OpenOptionsExt, net::UnixStream},
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use lifecycle_proof::{
    host::{HostClient, HostReportedIdentity},
    identity::{HostProcess, sha256_file},
    provider::{Reply, ScriptedProvider, Step},
    receipt::ArtifactBinding,
};
use serde_json::{Value, json};

use common::{
    BUILD_ID, SNAPSHOT_ID, TASK_ID, create_fixture, initial_submit, preflight_installed_host,
};

struct PreserveRootOnPanic(std::path::PathBuf);

impl Drop for PreserveRootOnPanic {
    fn drop(&mut self) {
        if std::thread::panicking() && self.0.exists() {
            let retained = self
                .0
                .with_file_name(format!("s8-grant-drain-failure-{}", std::process::id()));
            match fs::rename(&self.0, &retained) {
                Ok(()) => eprintln!("S8 failed drain root retained at {}", retained.display()),
                Err(error) => eprintln!(
                    "S8 failed drain root preservation failed: original={} error={error}",
                    self.0.display()
                ),
            }
        }
    }
}

fn rows(database: &Path, query: &str) -> Value {
    let output = Command::new("/usr/bin/sqlite3")
        .args(["-readonly", "-batch", "-json"])
        .arg(database)
        .arg(query)
        .env("TMPDIR", std::env::var_os("TMPDIR").unwrap())
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR").unwrap(),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "read-only SQLite query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if output.stdout.is_empty() {
        json!([])
    } else {
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

fn lifecycle_close_view(root: &Path) -> Value {
    let database = root.join("lifecycle-store/lifecycle.sqlite");
    json!({
        "grants":rows(&database,"SELECT grant_id,revision,revoked FROM grants WHERE grant_id IN ('grant:s8:result','grant:s8:snapshot') ORDER BY grant_id"),
        "contractions":rows(&database,"SELECT command_id,grant_id,expected_revision,resulting_revision,committed_wall_ms FROM grant_contractions ORDER BY command_id"),
        "close":rows(&database,"SELECT incarnation_id,subject_id,joined_json,unresolved_json,pending_results_json,unsafe_at_close,wall_ms,native_close_json FROM service_closes ORDER BY wall_ms"),
        "order":rows(&database,"SELECT sequence,event_kind,event_ref FROM journal WHERE event_kind IN ('grant_revoked','service_closed') ORDER BY sequence")
    })
}

#[test]
fn installed_grant_lane_drains_before_service_close() {
    let build = preflight_installed_host();
    let tmp = std::env::var_os("TMPDIR").unwrap();
    let provider_root = tempfile::Builder::new()
        .prefix("s8-provider-drain-")
        .tempdir_in(tmp)
        .unwrap();
    let provider = ScriptedProvider::start(
        vec![Step {
            required: vec![SNAPSHOT_ID.into()],
            forbidden: Vec::new(),
            reply: Reply::Text("The approved task reached a bounded terminal result.".into()),
            release_path: None,
        }],
        provider_root.path().join("provider-ledger.jsonl"),
    )
    .unwrap();
    let fixture = create_fixture(&provider.base_url(), build);
    let _preserve_failure = PreserveRootOnPanic(fixture.root.path().to_path_buf());
    let barrier = fixture.root.path().join("barrier");
    let mut host = HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
    let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    let admitted = client.submit(initial_submit()).unwrap();
    assert_eq!(admitted["status"], "admitted", "{admitted}");
    let reported = HostReportedIdentity::from_reply(&admitted).unwrap();
    reported.bind_process(host.identity(), BUILD_ID).unwrap();
    reported.bind_startup(&fixture.startup).unwrap();
    let delivery_id = admitted["lifecycle"]["outcome_ref"].as_str().unwrap();
    let until = Instant::now() + Duration::from_secs(30);
    let settled = loop {
        if let Ok(value) = client.observe(TASK_ID, Some(delivery_id))
            && value["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"]
                == "completed"
            && value["components"]["native_delivery_status"]["value"]["segments"][0]["segment_state"]
                == "completed"
        {
            break value;
        }
        assert!(
            Instant::now() < until,
            "native task did not settle before drain cut"
        );
        thread::sleep(Duration::from_millis(20));
    };
    let owner_before = settled["components"]["owner"]["value"].clone();
    let before = lifecycle_close_view(fixture.root.path());
    assert!(before["contractions"].as_array().unwrap().is_empty());
    assert!(before["close"].as_array().unwrap().is_empty());
    fs::write(barrier.join("grant_lane_inside_fence.arm"), b"arm\n").unwrap();
    let service_socket = fixture.startup["lifecycle"]["socket_path"]
        .as_str()
        .unwrap();
    let mut late_peer = UnixStream::connect(service_socket).unwrap();
    late_peer
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    late_peer
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let first_payload = json!({
        "task_id":TASK_ID,
        "command_id":"command:s8:drain-first",
        "authority_ref":common::ANSWER_AUTHORITY,
        "grant_ref":"grant:s8:result",
        "expected_revision":1
    });
    let control_client = client.clone();
    let first = thread::spawn(move || control_client.contract_grant(first_payload));
    let reached = barrier.join("grant_lane_inside_fence.reached");
    let until = Instant::now() + Duration::from_secs(5);
    while !reached.exists() {
        assert!(
            Instant::now() < until,
            "grant lane did not enter material fence"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let held = lifecycle_close_view(fixture.root.path());
    assert!(held["contractions"].as_array().unwrap().is_empty());
    host.signal_terminate().unwrap();
    let gate = barrier.join("shutdown_gate_closed.reached");
    let until = Instant::now() + Duration::from_secs(5);
    while !gate.exists() {
        assert!(Instant::now() < until, "shutdown gate did not close");
        thread::sleep(Duration::from_millis(5));
    }
    let gated = lifecycle_close_view(fixture.root.path());
    assert!(gated["close"].as_array().unwrap().is_empty());
    assert!(gated["contractions"].as_array().unwrap().is_empty());
    let late = json!({
        "op":"contract_grant",
        "command":{
            "protocol_version":1,
            "command_id":"command:s8:drain-after-gate",
            "subject_id":fixture.startup["lifecycle"]["subject_id"],
            "authority_ref":fixture.startup["lifecycle"]["principal"]["principal_ref"],
            "grant_ref":"grant:s8:snapshot",
            "expected_revision":"1"
        }
    });
    let mut frame = serde_json::to_vec(&late).unwrap();
    frame.push(b'\n');
    let late_request_outcome = match late_peer.write_all(&frame) {
        Ok(()) => {
            let mut line = String::new();
            match BufReader::new(late_peer).read_line(&mut line) {
                Ok(n) if n > 0 => {
                    let reply: Value = serde_json::from_str(line.trim_end()).unwrap();
                    assert_eq!(reply["code"], "observation_unavailable", "{reply}");
                    json!({"kind":"typed_closed_gate_refusal","reply":reply})
                }
                Ok(_) => json!({"kind":"closed_transport"}),
                Err(error) => {
                    json!({"kind":"unavailable_transport","error_kind":format!("{:?}",error.kind())})
                }
            }
        }
        Err(error) => {
            json!({"kind":"unavailable_transport","error_kind":format!("{:?}",error.kind())})
        }
    };
    fs::write(
        barrier.join("grant_lane_inside_fence.release"),
        b"release\n",
    )
    .unwrap();
    let first_reply = match first.join().unwrap() {
        Ok(reply) => json!({"kind":"reply","value":reply}),
        Err(error) => json!({"kind":"reply_unavailable","error":error.to_string()}),
    };
    let process = host.identity().clone();
    let close = host
        .wait_after_terminate_and_reap(Duration::from_secs(20))
        .unwrap();
    assert!(close.reaped);
    let after = lifecycle_close_view(fixture.root.path());
    assert_eq!(after["contractions"].as_array().unwrap().len(), 1);
    assert_eq!(
        after["contractions"][0]["command_id"],
        "command:s8:drain-first"
    );
    assert_eq!(after["contractions"][0]["grant_id"], "grant:s8:result");
    assert_eq!(after["contractions"][0]["resulting_revision"], 2);
    assert_eq!(
        after["grants"][0],
        json!({"grant_id":"grant:s8:result","revision":2,"revoked":1})
    );
    assert_eq!(
        after["grants"][1],
        json!({"grant_id":"grant:s8:snapshot","revision":1,"revoked":0})
    );
    assert_eq!(after["close"].as_array().unwrap().len(), 1);
    assert_eq!(after["close"][0]["unsafe_at_close"], 0);
    assert_eq!(after["order"].as_array().unwrap().len(), 2);
    assert_eq!(after["order"][0]["event_kind"], "grant_revoked");
    assert_eq!(after["order"][0]["event_ref"], "command:s8:drain-first");
    assert_eq!(after["order"][1]["event_kind"], "service_closed");
    assert!(
        after["order"][0]["sequence"].as_i64().unwrap()
            < after["order"][1]["sequence"].as_i64().unwrap()
    );
    assert_eq!(
        rows(&fixture.root.path().join("tasks.sqlite"),
             "SELECT CAST(record_json AS TEXT) AS record_json FROM tasks WHERE task_id='task:s8:analysis'")[0]["record_json"]
            .as_str()
            .map(|text| serde_json::from_str::<Value>(text).unwrap()),
        Some(owner_before)
    );
    thread::sleep(Duration::from_millis(100));
    assert_eq!(lifecycle_close_view(fixture.root.path()), after);
    let ledger = provider.ledger().to_path_buf();
    let entries = provider.finish().unwrap();
    assert_eq!(entries.len(), 1);
    fixture.source_manifest.verify().unwrap();
    for source in &fixture.sources {
        source.verify().unwrap();
    }
    assert_eq!(
        sha256_file(&fixture.host_executable).unwrap(),
        fixture.build_provenance.executable_sha256
    );
    if let Some(destination) = std::env::var_os("S8_EVIDENCE_DIR") {
        let destination = std::path::PathBuf::from(destination);
        assert!(destination.is_absolute());
        fs::create_dir_all(&destination).unwrap();
        let contract = ArtifactBinding::read(Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v9.md")).unwrap();
        assert_eq!(
            contract.sha256,
            "d6560d2625852eff679d0a9c43265b59cb9a13ad9940694e5f985e3ae86edf2d"
        );
        let predecessor_v8 = ArtifactBinding::read(Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v8.md")).unwrap();
        assert_eq!(
            predecessor_v8.sha256,
            "149639c1a0e2777235d543cd28de3a2dd89a933ad3888dd3e93f971986fba046"
        );
        let predecessor_v7 = ArtifactBinding::read(Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v7.md")).unwrap();
        assert_eq!(
            predecessor_v7.sha256,
            "cfbe11a0b6cedcc98bf4a2c2e2bd284d921f3b99dbb326622fce1e2d64ab0276"
        );
        let evidence = json!({
            "schema":"s8-installed-grant-lane-shutdown-drain-v1",
            "contract":contract,
            "predecessor_v8":predecessor_v8,
            "predecessor_v7":predecessor_v7,
            "build_provenance":fixture.build_provenance,
            "host_executable":ArtifactBinding::read(&fixture.host_executable).unwrap(),
            "sqlite_read_only_tool":ArtifactBinding::read(Path::new("/usr/bin/sqlite3")).unwrap(),
            "source_manifest":fixture.source_manifest,
            "source_entries":fixture.sources,
            "startup":ArtifactBinding::read(&fixture.startup_path).unwrap(),
            "provider_ledger":ArtifactBinding::read(&ledger).unwrap(),
            "provider_entries":entries,
            "admission":admitted,
            "settled":settled,
            "host_identity":reported,
            "host_process":process,
            "before":before,
            "held_inside_fence":held,
            "gate_closed":gated,
            "late_request_outcome":late_request_outcome,
            "first_control_reply":first_reply,
            "after":after,
            "close":close
        });
        let path = destination.join(format!("s8-grant-drain-{}.json", std::process::id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        file.write_all(&serde_json::to_vec(&evidence).unwrap())
            .unwrap();
        file.sync_all().unwrap();
        eprintln!(
            "S8 grant drain receipt {} sha256 {}",
            path.display(),
            sha256_file(&path).unwrap()
        );
        let _ = fixture.root.keep();
        let _ = provider_root.keep();
    }
}
