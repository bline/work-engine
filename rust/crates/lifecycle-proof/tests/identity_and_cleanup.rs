use std::{
    fs,
    io::{BufReader, Read, Write},
    net::TcpStream,
    time::Duration,
};

use lifecycle_proof::{
    identity::{observe_process, sha256_file},
    provider::{Reply, ScriptedProvider, Step},
};
use serde_json::{Value, json};

fn send(url: &str, body: Value) -> String {
    let address = url
        .strip_prefix("http://")
        .unwrap()
        .strip_suffix("/v1")
        .unwrap();
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let bytes = serde_json::to_vec(&body).unwrap();
    write!(
        stream,
        "POST /v1/responses HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n\r\n",
        bytes.len()
    )
    .unwrap();
    stream.write_all(&bytes).unwrap();
    stream.flush().unwrap();
    let mut response = String::new();
    BufReader::new(stream)
        .read_to_string(&mut response)
        .unwrap();
    response
}

#[test]
fn loopback_provider_ledger_precedes_bounded_scripted_reply() {
    let root = tempfile::tempdir().unwrap();
    let hold = root.path().join("release");
    let provider = ScriptedProvider::start(
        vec![
            Step {
                required: vec!["domain-0".into()],
                forbidden: vec!["secret".into()],
                reply: Reply::ToolCall {
                    call_id: "call:read:0".into(),
                    namespace: "workspace".into(),
                    name: "snapshot_read".into(),
                    arguments: json!({"snapshot_id":"snapshot:s8","member_id":"m1","offset":0,"length":7}),
                },
                release_path: Some(hold.clone()),
            },
            Step::text("done"),
        ],
        root.path().join("provider-ledger.jsonl"),
    )
    .unwrap();
    let base = provider.base_url();
    let first = std::thread::spawn({
        let base = base.clone();
        move || send(&base, json!({"input":"domain-0"}))
    });
    let started = std::time::Instant::now();
    while provider.entry_count() != 1 {
        assert!(started.elapsed() < Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(5));
    }
    let before_release = provider.entries().unwrap();
    assert_eq!(before_release.len(), 1);
    assert_eq!(before_release[0].request["input"], "domain-0");
    let second = send(&base, json!({"input":"tool result"}));
    assert!(second.contains("done"));
    assert_eq!(provider.entry_count(), 2);
    fs::write(&hold, b"release").unwrap();
    let first = first.join().unwrap();
    assert!(first.contains("\"name\":\"snapshot_read\""));
    assert!(first.contains("call:read:0"));
    let entries = provider.finish().unwrap();
    assert_eq!(entries.len(), 2);
    assert_ne!(entries[0].request_sha256, entries[1].request_sha256);
}

#[test]
fn linux_identity_binds_executable_and_process_birth() {
    let observed = observe_process(std::process::id()).unwrap();
    assert_eq!(observed.pid, std::process::id());
    assert!(observed.linux_starttime_ticks > 0);
    assert_eq!(
        observed.executable_sha256,
        sha256_file(&observed.executable).unwrap()
    );
}
