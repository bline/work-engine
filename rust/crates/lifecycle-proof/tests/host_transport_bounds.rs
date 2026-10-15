#![recursion_limit = "256"]

mod common;

use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    os::unix::{fs::OpenOptionsExt, net::UnixStream},
    time::Duration,
};

use lifecycle_proof::{
    host::{HostClient, MAX_HOST_FRAME},
    identity::{HostProcess, sha256_file},
    receipt::ArtifactBinding,
};
use serde_json::{Value, json};

use common::{TASK_ID, create_fixture, initial_submit, preflight_installed_host};

#[test]
fn installed_host_bounds_request_and_response_frames() {
    let build = preflight_installed_host();
    let fixture = create_fixture("http://127.0.0.1:9/v1", build.clone());
    assert_eq!(fixture.startup["features"], json!(["controlled-proof"]));
    assert_eq!(
        fixture.build_provenance.executable_sha256,
        build.executable_sha256
    );
    let mut host = HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
    let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    assert_eq!(
        client
            .exchange(&json!({"op":"task.observe.v1","task_id":TASK_ID}))
            .unwrap(),
        json!({"status":"error","code":"unsupported_observation_version"})
    );
    let mut raw = UnixStream::connect(&fixture.host_socket).unwrap();
    raw.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    raw.write_all(&vec![b'x'; MAX_HOST_FRAME + 1]).unwrap();
    raw.shutdown(std::net::Shutdown::Write).unwrap();
    let mut line = String::new();
    BufReader::new(raw).read_line(&mut line).unwrap();
    let oversized_request: Value = serde_json::from_str(line.trim_end()).unwrap();
    assert_eq!(
        oversized_request,
        json!({"status":"error","code":"request_too_large"})
    );

    let mut submit = initial_submit();
    let facts = submit["facts"].as_array_mut().unwrap();
    for ordinal in 0..8 {
        facts.push(json!({"id":format!("large-{ordinal}"),"kind":"canonical_reference","text":"x".repeat(7000)}));
    }
    let admitted = client.submit(submit).unwrap();
    assert_eq!(admitted["status"], "admitted", "{admitted}");
    let mut queued_admissions = Vec::new();
    for ordinal in 0..10 {
        let mut queued = initial_submit();
        queued["command_id"] = json!(format!("command:s8:large-queued-{ordinal}"));
        queued["engineering_question"] = json!(format!("queued-{ordinal}:{}", "q".repeat(7000)));
        queued["facts"] = json!([]);
        let reply = client.submit(queued).unwrap();
        assert!(
            reply["owner"]["owner_revision"].as_u64().is_some(),
            "queued owner publication absent: {reply}"
        );
        queued_admissions.push(reply);
    }
    let oversized_response = client.observe(TASK_ID, None).unwrap();
    assert_eq!(
        oversized_response,
        json!({"status":"error","code":"response_too_large"})
    );
    host.assert_same_execution().unwrap();
    let identity = host.identity().clone();
    let close = host.terminate_and_reap(Duration::from_secs(15)).unwrap();
    assert!(close.reaped);
    fixture.source_manifest.verify().unwrap();
    for source in &fixture.sources {
        source.verify().unwrap();
    }
    if let Some(destination) = std::env::var_os("S8_EVIDENCE_DIR") {
        let destination = std::path::PathBuf::from(destination);
        assert!(destination.is_absolute());
        fs::create_dir_all(&destination).unwrap();
        let evidence = json!({
            "schema":"s8-installed-host-frame-bounds-v1",
            "build_provenance":build,
            "host_executable":ArtifactBinding::read(&fixture.host_executable).unwrap(),
            "source_manifest":fixture.source_manifest,
            "source_entries":fixture.sources,
            "startup":ArtifactBinding::read(&fixture.startup_path).unwrap(),
            "oversized_request_reply":oversized_request,
            "admission":admitted,
            "queued_admissions":queued_admissions,
            "oversized_response_reply":oversized_response,
            "identity":identity,
            "close":close,
            "provider_entries":0
        });
        let path = destination.join(format!("s8-host-frame-bounds-{}.json", std::process::id()));
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
            "S8 host frame receipt {} sha256 {}",
            path.display(),
            sha256_file(&path).unwrap()
        );
        let _ = fixture.root.keep();
    }
}
