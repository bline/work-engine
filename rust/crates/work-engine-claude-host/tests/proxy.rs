use serde_json::{Value, json};
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use work_engine_claude_host::fixture::{Fixture, Script};
use work_engine_claude_host::proxy::{Config, Proxy, Upstream};
use work_engine_claude_host::{http, spec};

const SECRET: &str = "sk-ant-oat01-SECRET-TOKEN-must-not-be-retained";

fn setup(max_requests: u32) -> (tempfile::TempDir, Fixture, Proxy) {
    let dir = tempfile::tempdir().unwrap();
    let fixture = Fixture::start(
        0,
        Script {
            snapshot_id: "snap-1".into(),
            member_id: "docs/a.md".into(),
            length: 5,
            tool_name: "mcp__we__snapshot_read".into(),
        },
    )
    .unwrap();
    let proxy = Proxy::start(Config {
        dir: dir.path().to_path_buf(),
        model: "claude-test".into(),
        expected_tools: spec::expected_request_tools(&spec::analysis_tools()),
        max_requests,
        max_body: 64 * 1024,
        max_response: 1024 * 1024,
        upstream: Upstream::Fixture(fixture.port),
    })
    .unwrap();
    (dir, fixture, proxy)
}

fn body(tools: Value, with_result: bool) -> Vec<u8> {
    let mut messages = vec![json!({"role":"user","content":[{"type":"text","text":"read it"}]})];
    if with_result {
        messages.push(json!({"role":"assistant","content":[{"type":"tool_use","id":"toolu_fixture_1","name":"mcp__we__snapshot_read","input":{}}]}));
        messages.push(json!({"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_fixture_1","content":[{"type":"text","text":"x"}]}]}));
    }
    serde_json::to_vec(
        &json!({"model":"claude-test","stream":true,"tools":tools,"messages":messages}),
    )
    .unwrap()
}

fn post(port: u16, path: &str, body: &[u8]) -> (u16, Vec<u8>) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {SECRET}\r\nUser-Agent: test-agent\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .unwrap();
    s.write_all(body).unwrap();
    let mut r = BufReader::new(s);
    let head = http::read_head(&mut r).unwrap().unwrap();
    let status: u16 = head.start.split(' ').nth(1).unwrap().parse().unwrap();
    let mut out = Vec::new();
    if head.get("transfer-encoding").is_some() {
        http::read_chunked(&mut r, 1 << 20, &mut |c| {
            out.extend_from_slice(c);
            Ok(())
        })
        .unwrap();
    } else {
        r.read_to_end(&mut out).unwrap();
    }
    (status, out)
}

fn files_contain(dir: &std::path::Path, needle: &str) -> bool {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if String::from_utf8_lossy(&std::fs::read(&p).unwrap()).contains(needle) {
                return true;
            }
        }
    }
    false
}

fn expected() -> Value {
    Value::Array(spec::expected_request_tools(&spec::analysis_tools()))
}

#[test]
fn forwards_pinned_requests_and_retains_exact_bytes_before_dispatch() {
    let (dir, fixture, proxy) = setup(4);
    let b1 = body(expected(), false);
    let (status, resp) = post(proxy.port, "/v1/messages?beta=true", &b1);
    assert_eq!(status, 200);
    assert!(String::from_utf8_lossy(&resp).contains("toolu_fixture_1"));
    let (status, resp) = post(
        proxy.port,
        "/v1/messages?beta=true",
        &body(expected(), true),
    );
    assert_eq!(status, 200);
    assert!(String::from_utf8_lossy(&resp).contains("Fixture answer"));
    let records = proxy.records();
    assert_eq!(records.len(), 2);
    assert!(
        records
            .iter()
            .all(|r| r.forwarded && r.complete && r.status == Some(200))
    );
    assert_eq!(
        std::fs::read(dir.path().join("requests/0001.body")).unwrap(),
        b1
    );
    assert_eq!(
        records[0].body_sha256.as_deref(),
        Some(spec::sha256_hex(&b1).as_str())
    );
    assert_eq!(records[0].tool_uses[0].0, "toolu_fixture_1");
    assert_eq!(records[0].tool_uses[0].2["member_id"], "docs/a.md");
    assert_eq!(records[0].message_id.as_deref(), Some("msg_fixture_1"));
    assert_eq!(records[1].stop_reason.as_deref(), Some("end_turn"));
    assert_eq!(
        std::fs::read(dir.path().join("requests/0001.response")).unwrap(),
        resp_bytes_of(&dir, 1)
    );
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 2);
    assert!(!files_contain(dir.path(), SECRET), "credential retained");
    assert!(
        records[0]
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v.is_empty())
    );
    assert!(
        records[0]
            .headers
            .iter()
            .any(|(k, v)| k == "User-Agent" && v == "test-agent")
    );
}

fn resp_bytes_of(dir: &tempfile::TempDir, n: u32) -> Vec<u8> {
    std::fs::read(dir.path().join(format!("requests/{n:04}.response"))).unwrap()
}

#[test]
fn refuses_before_any_upstream_byte() {
    let (dir, fixture, proxy) = setup(1);
    // Extra tool.
    let mut tools = spec::expected_request_tools(&spec::analysis_tools());
    tools.push(json!({"name":"Bash","description":"run","input_schema":{"type":"object"}}));
    let (status, _) = post(
        proxy.port,
        "/v1/messages",
        &body(Value::Array(tools), false),
    );
    assert_eq!(status, 400);
    // Changed schema.
    let mut tools = spec::expected_request_tools(&spec::analysis_tools());
    tools[1]["input_schema"]["properties"]["length"]["maximum"] = json!(1_000_000);
    let (status, _) = post(
        proxy.port,
        "/v1/messages",
        &body(Value::Array(tools), false),
    );
    assert_eq!(status, 400);
    // Wrong model.
    let mut v: Value = serde_json::from_slice(&body(expected(), false)).unwrap();
    v["model"] = json!("other");
    let (status, _) = post(proxy.port, "/v1/messages", &serde_json::to_vec(&v).unwrap());
    assert_eq!(status, 400);
    // Path outside the API.
    let (status, _) = post(
        proxy.port,
        "/v1/messages/count_tokens",
        &body(expected(), false),
    );
    assert_eq!(status, 404);
    // Body over the bound.
    let big = vec![b' '; 70 * 1024];
    let (status, _) = post(proxy.port, "/v1/messages", &big);
    assert_eq!(status, 413);
    assert_eq!(
        fixture.hits.load(Ordering::SeqCst),
        0,
        "a refused request reached upstream"
    );
    // Count bound: first forwarded, second refused.
    let (status, _) = post(proxy.port, "/v1/messages", &body(expected(), false));
    assert_eq!(status, 200);
    let (status, _) = post(proxy.port, "/v1/messages", &body(expected(), true));
    assert_eq!(status, 400);
    assert_eq!(fixture.hits.load(Ordering::SeqCst), 1);
    let records = proxy.records();
    assert_eq!(records.iter().filter(|r| r.forwarded).count(), 1);
    assert_eq!(records.iter().filter(|r| r.refused.is_some()).count(), 6);
    // Refused bodies are retained too (except the oversized one, never read).
    assert!(dir.path().join("requests/0001.body").exists());
    assert!(!dir.path().join("requests/0005.body").exists());
    assert!(!files_contain(dir.path(), SECRET));
}
