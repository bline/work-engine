//! Adapter tests over the loopback fixture endpoint. Evidence class: synthetic.
//! They exercise `DirectAdapter` + `LoopbackEndpoint` + `assemble_request` +
//! `LocalHttpTransport` + `AttemptJournal`; they are not a public HTTPS adapter
//! and establish nothing about a provider.

use bounded_inference::{
    AuthStore, Bounds, DirectAdapter, DirectProfile, Issuer, LocalEndpoint, LocalHttpTransport,
    LoopbackEndpoint, TokenResponse,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use lifecycle_semantic::{
    DirectInferencePort, DirectPreparedCall, DirectRole, DirectSubmitError, DirectTerminal,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

static SERIAL: AtomicU64 = AtomicU64::new(0);
const JWKS: &str = include_str!("fixtures/synthetic-siwc-jwks.json");
const KEY: &[u8] = include_bytes!("fixtures/synthetic-siwc-rsa.pkcs1.der");
const SCOPE: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
const MODEL: &str = "fake-account-model";

fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "b3-direct-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&p).unwrap();
    p
}
fn bounds() -> Bounds {
    Bounds {
        request_bytes: 8192,
        stream_bytes: 16384,
        frame_bytes: 8192,
        output_bytes: 1024,
        events: 32,
        local_deadline_ms: 2000,
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
fn event(v: Value) -> String {
    format!(
        "event: {}\r\ndata: {}\r\n\r\n",
        v["type"].as_str().unwrap(),
        v
    )
}
fn bearer(root: &Path, access: &str) -> (AuthStore, bounded_inference::BearerAccess) {
    let issuer = Issuer::new(
        "https://fake-issuer.invalid",
        "https://fake-issuer.invalid/authorize",
        "https://fake-issuer.invalid/token",
        "https://fake-issuer.invalid/revoke",
    )
    .unwrap();
    let store = AuthStore::open(root, "synthetic_host_b2b", issuer).unwrap();
    let t = now();
    let start = store
        .begin_registration("Synthetic Tool", "http://127.0.0.1:45123/auth/callback", t)
        .unwrap();
    let url = url::Url::parse(start.authorize_url()).unwrap();
    let value = |key: &str| {
        url.query_pairs()
            .find(|(k, _)| k == key)
            .unwrap()
            .1
            .to_string()
    };
    let state = value("state");
    let nonce = value("nonce");
    let client = "fake-client";
    let grant = store
        .accept_callback(
            &format!(
                "http://127.0.0.1:45123/auth/callback?code=fake&state={state}&client_id={client}"
            ),
            t,
        )
        .unwrap();
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some("synthetic-key-1".into());
    let id = encode(&h, &json!({"iss":"https://fake-issuer.invalid","aud":client,"sub":"fake-subject","exp":t+3600,"iat":t-1,"nonce":nonce}), &EncodingKey::from_rsa_der(KEY)).unwrap();
    let response = TokenResponse::from_json(serde_json::to_string(&json!({"access_token":access,"refresh_token":"fake-refresh","id_token":id,"token_type":"Bearer","expires_in":3600,"scope":SCOPE})).unwrap().as_bytes()).unwrap();
    let reg = store.finish_exchange(&grant, response, JWKS, t).unwrap();
    let snapshot = store.bearer(&reg, t).unwrap();
    (store, snapshot)
}
fn read_request(mut socket: &std::net::TcpStream) -> (String, Vec<u8>) {
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .unwrap();
    let mut all = Vec::new();
    let mut buf = [0u8; 2048];
    let header_end = loop {
        let n = socket.read(&mut buf).unwrap();
        assert!(n > 0);
        all.extend_from_slice(&buf[..n]);
        if let Some(i) = all.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        assert!(all.len() < 64 * 1024);
    };
    let head = String::from_utf8(all[..header_end].to_vec()).unwrap();
    let len: usize = head
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length: ")
                .and_then(|n| n.trim().parse().ok())
        })
        .unwrap();
    let mut body = all[header_end..].to_vec();
    while body.len() < len {
        let n = socket.read(&mut buf).unwrap();
        assert!(n > 0);
        body.extend_from_slice(&buf[..n]);
    }
    (head, body)
}

fn stream(text: &str, terminal: &str) -> String {
    let mut s = String::new();
    let completed = json!({"type":"response.completed","sequence_number":4,"response":{"id":"resp-local","model":MODEL,"status":"completed","output":[{"id":"msg-1","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":text}]}]}});
    let failed = json!({"type":"response.failed","sequence_number":4,"response":{"id":"resp-local","model":MODEL,"status":"failed","error":{"code":"server_error","message":"synthetic"}}});
    for v in [
        json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-local","model":MODEL}}),
        json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"msg-1","type":"message"}}),
        json!({"type":"response.output_text.delta","sequence_number":3,"output_index":0,"item_id":"msg-1","content_index":0,"delta":text}),
        if terminal == "failed" { failed } else { completed },
    ] {
        s.push_str(&event(v));
    }
    s
}

fn adapter(
    root: &Path,
    address: std::net::SocketAddr,
) -> DirectAdapter<LoopbackEndpoint> {
    let (store, snapshot) = bearer(&root.join("auth"), "synthetic-access-b3");
    let store = Arc::new(store);
    let registration = snapshot.registration.clone();
    let endpoint = LocalEndpoint::new(&format!("http://{address}/v1/responses"), None).unwrap();
    let loopback = LoopbackEndpoint::new(
        LocalHttpTransport::new(endpoint, None),
        move || store.bearer(&registration, now()).ok(),
        "fixture:adapter-loopback",
    );
    DirectAdapter::new(
        loopback,
        DirectProfile {
            model: MODEL.into(),
            bounds: bounds(),
        },
        root.join("journals"),
    )
}

fn prepared(adapter: &DirectAdapter<LoopbackEndpoint>, operation: &str) -> DirectPreparedCall {
    let input = "synthetic compile input";
    let wire = adapter.wire(DirectRole::Compile, input).unwrap();
    let hex = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    serde_json::from_value(json!({
        "binding": {
            "subject": "subject-x", "transition": "transition-x", "capture_id": "capture-x",
            "role": "compile", "policy_id": "policy-x", "operation_id": operation,
            "attempt_id": format!("attempt-{operation}"), "exposure_id": format!("exposure-{operation}")
        },
        "source_basis_sha256": hex(b"basis"),
        "candidate_sha256": null,
        "compile_binding_sha256": null,
        "input": input,
        "input_sha256": hex(input.as_bytes()),
        "wire_sha256": hex(&wire.wire_bytes),
        "wire": wire,
        "max_output_bytes": 1024,
        "binding_sha256": hex(b"binding"),
    }))
    .unwrap()
}

fn serve(body: Vec<u8>) -> (std::net::SocketAddr, thread::JoinHandle<Vec<u8>>) {
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = peer.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut socket, _) = peer.accept().unwrap();
        let (head, actual) = read_request(&socket);
        assert!(head.starts_with("POST /v1/responses HTTP/1.1\r\n"));
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nX-Request-Id: req_fake_1\r\n\r\n", body.len());
        socket.write_all(response.as_bytes()).unwrap();
        socket.write_all(&body).unwrap();
        actual
    });
    (address, handle)
}

#[test]
fn completed_capture_comes_from_the_journal_observation_and_is_sent_once() {
    let root = root();
    let (address, server) = serve(stream("{\"ok\":true}", "completed").into_bytes());
    let adapter = adapter(&root, address);
    let call = prepared(&adapter, "direct-op-1");
    let capture = adapter.submit(&call).unwrap();
    assert_eq!(server.join().unwrap(), call.wire.wire_bytes);
    assert_eq!(capture.terminal, DirectTerminal::Complete);
    assert_eq!(capture.output, b"{\"ok\":true}".to_vec());
    assert_eq!(capture.response_id.as_deref(), Some("resp-local"));
    assert_eq!(capture.observed_model.as_deref(), Some(MODEL));
    assert_eq!(capture.observed_provider.as_deref(), Some("local_loopback"));
    assert_eq!(capture.prepared_wire_sha256, call.wire_sha256);
    assert_eq!(capture.evidence.label(), "synthetic_local");
    assert_eq!(capture.evidence.reference(), "fixture:adapter-loopback");
    // The journal already holds this operation: a second submit cannot resend.
    assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_terminal_yields_no_output() {
    let root = root();
    let (address, server) = serve(stream("ignored", "failed").into_bytes());
    let adapter = adapter(&root, address);
    let call = prepared(&adapter, "direct-op-2");
    let capture = adapter.submit(&call).unwrap();
    server.join().unwrap();
    assert_eq!(capture.terminal, DirectTerminal::Failed);
    assert!(capture.output.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn altered_wire_is_not_sent() {
    let root = root();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let adapter = adapter(&root, peer.local_addr().unwrap());
    let mut call = prepared(&adapter, "direct-op-3");
    call.wire.wire_bytes.push(b' ');
    assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::NotSent);
    peer.set_nonblocking(true).unwrap();
    assert!(peer.accept().is_err(), "no connection may be made");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn connection_failure_leaves_the_outcome_unknown_and_is_not_replayed() {
    let root = root();
    let address = {
        let peer = TcpListener::bind("127.0.0.1:0").unwrap();
        peer.local_addr().unwrap()
    };
    let adapter = adapter(&root, address);
    let call = prepared(&adapter, "direct-op-4");
    assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
    assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
    std::fs::remove_dir_all(root).unwrap();
}
