//! Persistent-client transport failure tests against scripted fake MCP servers
//! (shell scripts; SYNTHETIC servers, no registry). Terminal transport failures
//! must poison the client; recoverable owner refusals must not.
use repository_claims_handoff::consumer::*;
use repository_claims_handoff::*;
use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

static SERIAL: Mutex<()> = Mutex::new(());

fn sha(path: &Path) -> String {
    file_sha256(path).unwrap()
}
/// `body` runs after `initialize` and `tools/list` were answered.
fn server(dir: &Path, log: &Path, body: &str) -> TrustedServer {
    let script = dir.join("fake-server.sh");
    let text = format!(
        r#"#!/bin/sh
reply() {{ id=$(printf '%s' "$1" | sed -n 's/.*"id":\([0-9]*\).*/\1/p'); printf '{{"jsonrpc":"2.0","id":%s,"result":%s}}\n' "$id" "$2"; }}
read l; printf '%s\n' "$l" >> "{log}"; reply "$l" '{{"protocolVersion":"2025-06-18"}}'
read l; printf '%s\n' "$l" >> "{log}"; reply "$l" '{{"tools":[{{"name":"discover_repository_facts"}},{{"name":"resolve_repository_fact"}}]}}'
{body}
"#,
        log = log.display()
    );
    fs::write(&script, text).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let profile = dir.join("profile.json");
    fs::write(&profile, b"{}").unwrap();
    TrustedServer {
        executable_sha256: sha(&script),
        executable: script,
        read_profile_sha256: sha(&profile),
        read_profile: profile,
    }
}
fn open(s: &TrustedServer, ms: u64) -> ClaimsConsumer {
    ClaimsConsumer::open(
        s,
        &Options {
            page_bytes: 4096,
            deadline: Duration::from_millis(ms),
        },
        16,
    )
    .unwrap()
}
fn lines(log: &Path) -> usize {
    fs::read_to_string(log).unwrap().lines().count()
}

#[test]
fn late_response_poisons_the_transport_and_later_calls_refuse_promptly() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile_dir("late");
    let log = dir.join("log");
    // First tool call is answered only after the client deadline; later requests
    // would be answered at once with their own id. A reusable client would read
    // the late reply for call 1 as a protocol mismatch on call 2.
    let s = server(
        &dir,
        &log,
        r#"read l; printf '%s\n' "$l" >> "LOG"; sleep 2; reply "$l" '{"isError":false,"structuredContent":{"owner_result":{}}}'
while read l; do printf '%s\n' "$l" >> "LOG"; reply "$l" '{"isError":false,"structuredContent":{"owner_result":{}}}'; done"#
            .replace("LOG", &log.display().to_string())
            .as_str(),
    );
    let mut c = open(&s, 300);
    let first = c
        .call(TOOL_DISCOVER, &json!({"query":"x"}), None)
        .unwrap_err();
    assert_eq!(first.code(), "server_unavailable");
    let before = lines(&log);
    let t = Instant::now();
    let second = c
        .call(TOOL_DISCOVER, &json!({"query":"y"}), None)
        .unwrap_err();
    assert_eq!(second.code(), "server_unavailable");
    assert!(t.elapsed() < Duration::from_millis(200), "prompt refusal");
    std::thread::sleep(Duration::from_millis(2300));
    assert_eq!(
        lines(&log),
        before,
        "no request reached the poisoned server"
    );
}

#[test]
fn server_that_stops_reading_cannot_hold_the_client_past_its_deadline() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile_dir("stopped");
    let log = dir.join("log");
    let s = server(&dir, &log, "exec sleep 20");
    let mut c = open(&s, 300);
    let t = Instant::now();
    let e = c
        .call(TOOL_DISCOVER, &json!({"query":"x"}), None)
        .unwrap_err();
    assert_eq!(e.code(), "server_unavailable");
    // Repeated attempts never queue behind the stopped reader.
    for _ in 0..5 {
        let e = c
            .call(TOOL_DISCOVER, &json!({"query":"z".repeat(512)}), None)
            .unwrap_err();
        assert_eq!(e.code(), "server_unavailable");
    }
    assert!(t.elapsed() < Duration::from_secs(2), "bounded");
}

#[test]
fn invalid_page_size_refuses_before_any_owner_request() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile_dir("static");
    let log = dir.join("log");
    let s = server(&dir, &log, "exec sleep 5");
    let mut c = open(&s, 300);
    let baseline = lines(&log);
    for (tool, args) in [
        (TOOL_DISCOVER, json!({"query":"x","max_bytes":8193})),
        (TOOL_DISCOVER, json!({"query":"x","max_bytes":3})),
        (
            TOOL_RESOLVE,
            json!({"claim_id":"c","revision_id":"r","revision_sha256":"0".repeat(64),"max_bytes":8193}),
        ),
        (
            TOOL_EVIDENCE,
            json!({"claim_id":"c","revision_id":"r","revision_sha256":"0".repeat(64),"sha256":"0".repeat(64),"max_bytes":0}),
        ),
    ] {
        assert_eq!(
            c.call(tool, &args, None).unwrap_err().code(),
            "invalid_arguments"
        );
    }
    assert_eq!(lines(&log), baseline, "no owner request was written");
}

fn tempfile_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("claims-transport-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}
