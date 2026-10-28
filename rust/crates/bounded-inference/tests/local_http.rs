use bounded_inference::{
    assemble_request, AttemptJournal, AttemptRecord, AttemptState, AuthStore, Bounds, Issuer,
    LocalEndpoint, LocalHttpTransport, ObservedTerminal, StreamError, StreamObserver,
    TokenResponse, ToolFreeProfile,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use sha2::Digest;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

static SERIAL: AtomicU64 = AtomicU64::new(0);
const JWKS: &str = include_str!("fixtures/synthetic-siwc-jwks.json");
const KEY: &[u8] = include_bytes!("fixtures/synthetic-siwc-rsa.pkcs1.der");
const SCOPE: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "b2b-joined-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&p).unwrap();
    p
}
fn bounds() -> Bounds {
    Bounds {
        request_bytes: 4096,
        stream_bytes: 16384,
        frame_bytes: 8192,
        output_bytes: 1024,
        events: 32,
        local_deadline_ms: 2000,
    }
}
fn profile() -> ToolFreeProfile {
    ToolFreeProfile {
        revision: "selected-reasoning-v1".into(),
        model: "fake-account-model".into(),
        bounds: bounds(),
    }
}
fn event(v: Value) -> String {
    format!(
        "event: {}\r\ndata: {}\r\n\r\n",
        v["type"].as_str().unwrap(),
        v
    )
}
fn stream() -> String {
    let mut s = String::new();
    for v in [
        json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-local","model":"fake-account-model"}}),
        json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"reason-1","type":"reasoning"}}),
        json!({"type":"response.reasoning_summary_text.delta","sequence_number":3,"output_index":0,"item_id":"reason-1","summary_index":0,"delta":"internal"}),
        json!({"type":"response.reasoning_summary_text.done","sequence_number":4,"output_index":0,"item_id":"reason-1","summary_index":0,"text":"internal"}),
        json!({"type":"response.output_item.done","sequence_number":5,"output_index":0,"item":{"id":"reason-1","type":"reasoning","status":"completed","summary":[{"type":"summary_text","text":"internal"}]}}),
        json!({"type":"response.output_item.added","sequence_number":6,"output_index":1,"item":{"id":"msg-1","type":"message"}}),
        json!({"type":"response.output_text.delta","sequence_number":7,"output_index":1,"item_id":"msg-1","content_index":0,"delta":"αβ"}),
        json!({"type":"response.completed","sequence_number":8,"response":{"id":"resp-local","model":"fake-account-model","status":"completed","output":[{"id":"reason-1","type":"reasoning","status":"completed","summary":[{"type":"summary_text","text":"internal"}]},{"id":"msg-1","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"αβ"}]}]}}),
    ] {
        s.push_str(&event(v));
    }
    s
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
fn read_tls_request<R: Read>(socket: &mut R) -> (String, Vec<u8>) {
    let mut all = Vec::new();
    let mut buf = [0u8; 2048];
    let end = loop {
        let n = socket.read(&mut buf).unwrap();
        assert!(n > 0);
        all.extend_from_slice(&buf[..n]);
        if let Some(i) = all.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        assert!(all.len() < 32 * 1024);
    };
    let head = String::from_utf8(all[..end].to_vec()).unwrap();
    let len: usize = head
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length: ")
                .and_then(|n| n.trim().parse().ok())
        })
        .unwrap();
    let mut body = all[end..].to_vec();
    while body.len() < len {
        let n = socket.read(&mut buf).unwrap();
        assert!(n > 0);
        body.extend_from_slice(&buf[..n]);
    }
    (head, body)
}
#[test]
fn joined_auth_socket_reasoning_journal_and_exact_body() {
    let root = root();
    let (store, snapshot) = bearer(&root.join("auth"), "synthetic-access-b2b");
    let request = assemble_request("instruction", "input", &profile()).unwrap();
    let path = root.join("attempt.json");
    let record = AttemptRecord::prepared_bound(
        "op-1",
        "attempt-1",
        request.sha256.clone(),
        request.profile_revision.clone(),
        request.selected_model.clone(),
        request.bounds.clone(),
        snapshot.registration.clone(),
    )
    .unwrap();
    let mut journal = AttemptJournal::create(&path, record).unwrap();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = peer.local_addr().unwrap();
    let expected = request.bytes.clone();
    let body = stream().into_bytes();
    let handle = thread::spawn(move || {
        let (mut socket, _) = peer.accept().unwrap();
        let (head, actual) = read_request(&socket);
        assert!(head.starts_with("POST /v1/responses HTTP/1.1\r\n"));
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: bearer synthetic-access-b2b\r\n"));
        assert_eq!(actual, expected);
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nX-Request-Id: req_fake_1\r\n\r\n", body.len());
        socket.write_all(response.as_bytes()).unwrap();
        for chunk in body.chunks(3) {
            socket.write_all(chunk).unwrap();
        }
        1usize
    });
    let endpoint = LocalEndpoint::new(&format!("http://{address}/v1/responses"), None).unwrap();
    let transport = LocalHttpTransport::new(endpoint, None);
    assert_eq!(
        transport
            .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
            .unwrap(),
        AttemptState::TerminalObserved
    );
    assert_eq!(handle.join().unwrap(), 1);
    let recovered = AttemptJournal::open(&path).unwrap();
    assert_eq!(
        recovered.record().registration.as_ref(),
        Some(&snapshot.registration)
    );
    assert_eq!(
        recovered.record().observation.as_ref().unwrap().terminal,
        Some(ObservedTerminal::Completed)
    );
    assert_eq!(
        recovered
            .record()
            .observation
            .as_ref()
            .unwrap()
            .output
            .as_deref(),
        Some("αβ")
    );
    assert_eq!(
        recovered.record().observation.as_ref().unwrap().usage.len(),
        0
    );
    assert_eq!(
        recovered
            .record()
            .http
            .as_ref()
            .unwrap()
            .request_id
            .as_deref(),
        Some("req_fake_1")
    );
    assert_eq!(
        recovered.record().http.as_ref().unwrap().body_bytes,
        body_len()
    );
    assert!(transport
        .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
        .is_err());
    assert_eq!(
        store
            .bearer(&snapshot.registration, now())
            .unwrap()
            .registration,
        snapshot.registration
    );
    std::fs::remove_dir_all(root).unwrap();
}
fn body_len() -> u64 {
    stream().len() as u64
}
#[test]
fn http_error_echoed_credential_is_not_durable_or_retried() {
    let root = root();
    let (_, snapshot) = bearer(&root.join("auth"), "secret-echo-value");
    let request = assemble_request("instruction", "input", &profile()).unwrap();
    let path = root.join("attempt.json");
    let record = AttemptRecord::prepared_bound(
        "op-2",
        "attempt-2",
        request.sha256.clone(),
        request.profile_revision.clone(),
        request.selected_model.clone(),
        request.bounds.clone(),
        snapshot.registration.clone(),
    )
    .unwrap();
    let mut journal = AttemptJournal::create(&path, record).unwrap();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = peer.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut socket, _) = peer.accept().unwrap();
        let _ = read_request(&socket);
        let body = br#"{"error":{"code":"secret-echo-value","param":"secret-echo-value"},"detail":"secret-echo-value"}"#;
        let head = format!("HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nX-Request-Id: secret-echo-value\r\nContent-Length: {}\r\n\r\n", body.len());
        socket.write_all(head.as_bytes()).unwrap();
        socket.write_all(body).unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_millis(200)))
            .unwrap();
        peer.set_nonblocking(true).unwrap();
        assert!(peer.accept().is_err());
    });
    let endpoint = LocalEndpoint::new(&format!("http://{address}/v1/responses"), None).unwrap();
    let transport = LocalHttpTransport::new(endpoint, None);
    assert!(transport
        .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
        .is_err());
    handle.join().unwrap();
    let recovered = AttemptJournal::open(&path).unwrap();
    assert_eq!(recovered.record().state, AttemptState::OutcomeUnknown);
    assert_eq!(
        recovered.record().observation.as_ref().unwrap().response_id,
        None
    );
    assert_eq!(recovered.record().http.as_ref().unwrap().status, Some(401));
    assert!(!std::fs::read_to_string(&path)
        .unwrap()
        .contains("secret-echo-value"));
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn selected_profile_rejects_tools_refusal_and_reasoning_identity_conflict() {
    let created =
        event(json!({"type":"response.created","sequence_number":1,"response":{"id":"r"}}));
    let mut tool = StreamObserver::selected(bounds()).unwrap();
    tool.push(created.as_bytes()).unwrap();
    assert_eq!(tool.push(event(json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"tool","type":"function_call"}})).as_bytes()).unwrap_err(), StreamError::UnsupportedTool);
    let mut refusal = StreamObserver::selected(bounds()).unwrap();
    refusal.push(created.as_bytes()).unwrap();
    assert_eq!(
        refusal
            .push(
                event(json!({"type":"response.refusal.delta","sequence_number":2,"delta":"no"}))
                    .as_bytes()
            )
            .unwrap_err(),
        StreamError::UnsupportedContent
    );
    let mut reasoning = StreamObserver::selected(bounds()).unwrap();
    reasoning.push(created.as_bytes()).unwrap();
    reasoning.push(event(json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"reason","type":"reasoning"}})).as_bytes()).unwrap();
    assert_eq!(reasoning.push(event(json!({"type":"response.reasoning_summary_text.delta","sequence_number":3,"output_index":0,"item_id":"other","summary_index":0,"delta":"x"})).as_bytes()).unwrap_err(), StreamError::ConflictingIdentity);
}

#[test]
fn status_and_content_type_faults_recover_without_provider_id() {
    for (status, type_header, body, expected_shape) in [
        (
            403,
            "application/json",
            r#"{"detail":"denied"}"#,
            Some(bounded_inference::HttpErrorShape::Detail),
        ),
        (
            429,
            "application/json",
            r#"{"error":{"code":"rate_limit","param":"model"}}"#,
            Some(bounded_inference::HttpErrorShape::Structured),
        ),
        (
            503,
            "text/plain",
            "temporary",
            Some(bounded_inference::HttpErrorShape::NonJson),
        ),
        (
            302,
            "text/plain",
            "redirect",
            Some(bounded_inference::HttpErrorShape::NonJson),
        ),
        (200, "application/json", "{}", None),
    ] {
        let root = root();
        let (_, snapshot) = bearer(&root.join("auth"), "a-safe-token");
        let request = assemble_request("i", "x", &profile()).unwrap();
        let path = root.join("attempt.json");
        let mut journal = AttemptJournal::create(
            &path,
            AttemptRecord::prepared_bound(
                "op",
                "attempt",
                request.sha256.clone(),
                request.profile_revision.clone(),
                request.selected_model.clone(),
                request.bounds.clone(),
                snapshot.registration.clone(),
            )
            .unwrap(),
        )
        .unwrap();
        let peer = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = peer.local_addr().unwrap();
        let body = body.as_bytes().to_vec();
        let content_type = type_header.to_string();
        let handle = thread::spawn(move || {
            let (mut socket, _) = peer.accept().unwrap();
            let _ = read_request(&socket);
            let head = format!("HTTP/1.1 {status} Test\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nLocation: http://127.0.0.1:9/v1/responses\r\n\r\n", body.len());
            socket.write_all(head.as_bytes()).unwrap();
            socket.write_all(&body).unwrap();
            peer.set_nonblocking(true).unwrap();
            assert!(
                peer.accept().is_err(),
                "redirect or retry sent a second request"
            );
        });
        let transport = LocalHttpTransport::new(
            LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
            None,
        );
        assert!(transport
            .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
            .is_err());
        handle.join().unwrap();
        let recovered = AttemptJournal::open(&path).unwrap();
        assert_eq!(recovered.record().state, AttemptState::OutcomeUnknown);
        assert!(recovered
            .record()
            .observation
            .as_ref()
            .unwrap()
            .response_id
            .is_none());
        assert_eq!(
            recovered.record().http.as_ref().unwrap().status,
            Some(status)
        );
        assert_eq!(
            recovered.record().http.as_ref().unwrap().error_shape,
            expected_shape
        );
        if status == 429 {
            assert_eq!(
                recovered
                    .record()
                    .http
                    .as_ref()
                    .unwrap()
                    .error_code
                    .as_deref(),
                Some("rate_limit")
            );
            assert_eq!(
                recovered
                    .record()
                    .http
                    .as_ref()
                    .unwrap()
                    .error_param
                    .as_deref(),
                Some("model")
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn model_shape_and_endpoint_guards() {
    assert!(LocalEndpoint::new("http://example.com:123/v1/responses", None).is_err());
    assert!(LocalEndpoint::new("http://127.0.0.1:123/v1/responses?x=1", None).is_err());
    assert!(LocalEndpoint::new("http://user@127.0.0.1:123/v1/responses", None).is_err());
    assert!(LocalEndpoint::new("http://127.0.0.1:123/other", None).is_err());
    for (body, valid) in [
        (
            r#"{"models":[{"slug":"fake-account-model","visibility":"public"}]}"#,
            true,
        ),
        (r#"{"data":[{"id":"fake-account-model"}]}"#, false),
    ] {
        let root = root();
        let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
        let peer = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = peer.local_addr().unwrap();
        let body = body.as_bytes().to_vec();
        let handle = thread::spawn(move || {
            let (mut socket, _) = peer.accept().unwrap();
            let mut buf = [0u8; 4096];
            let n = socket.read(&mut buf).unwrap();
            let head = String::from_utf8_lossy(&buf[..n]);
            assert!(head.starts_with("GET /v1/models HTTP/1.1"));
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: bearer synthetic-access"));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                body.len()
            );
            socket.write_all(response.as_bytes()).unwrap();
            socket.write_all(&body).unwrap();
        });
        let transport = LocalHttpTransport::new(
            LocalEndpoint::new(&format!("http://{addr}/v1/models"), None).unwrap(),
            None,
        );
        let result = transport.discover_models(&snapshot, &snapshot.registration);
        handle.join().unwrap();
        assert_eq!(result.is_ok(), valid);
        if valid {
            assert_eq!(result.unwrap()[0].slug, "fake-account-model");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn connection_failure_retains_intent_and_cannot_be_replayed() {
    let root = root();
    let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = root.join("attempt.json");
    let mut journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = peer.local_addr().unwrap();
    drop(peer);
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
        None,
    );
    assert!(transport
        .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
        .is_err());
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::OutcomeUnknown
    );
    assert!(transport
        .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
        .is_err());
    std::fs::remove_dir_all(root).unwrap();
}

fn tls_peer(
    san: &str,
) -> (
    std::net::SocketAddr,
    reqwest::Certificate,
    thread::JoinHandle<()>,
) {
    let key = rcgen::generate_simple_self_signed(vec![san.to_string()]).unwrap();
    let cert_der = key.cert.der().to_vec();
    let cert = reqwest::Certificate::from_der(&cert_der).unwrap();
    let private = rustls::pki_types::PrivatePkcs8KeyDer::from(key.signing_key.serialize_der());
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![rustls::pki_types::CertificateDer::from(cert_der)],
            private.into(),
        )
        .unwrap();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = peer.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (socket, _) = peer.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let mut tls = rustls::ServerConnection::new(std::sync::Arc::new(config)).unwrap();
        let mut io = socket;
        while tls.is_handshaking() {
            if tls.complete_io(&mut io).is_err() {
                break;
            }
        }
        assert!(
            tls.is_handshaking(),
            "client unexpectedly accepted invalid TLS identity"
        );
    });
    (addr, cert, handle)
}
#[test]
fn local_tls_invalid_certificate_and_hostname_are_rejected() {
    for (san, trusted) in [("localhost", false), ("wrong.local", true)] {
        let root = root();
        let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
        let request = assemble_request("i", "x", &profile()).unwrap();
        let path = root.join("attempt.json");
        let mut journal = AttemptJournal::create(
            &path,
            AttemptRecord::prepared_bound(
                "op",
                "attempt",
                request.sha256.clone(),
                request.profile_revision.clone(),
                request.selected_model.clone(),
                request.bounds.clone(),
                snapshot.registration.clone(),
            )
            .unwrap(),
        )
        .unwrap();
        let (addr, cert, handle) = tls_peer(san);
        let endpoint = LocalEndpoint::new(
            &format!("https://localhost:{}/v1/responses", addr.port()),
            Some(addr),
        )
        .unwrap();
        let transport = LocalHttpTransport::new(endpoint, trusted.then_some(cert));
        assert!(transport
            .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
            .is_err());
        handle.join().unwrap();
        assert_eq!(
            AttemptJournal::open(&path).unwrap().record().state,
            AttemptState::OutcomeUnknown
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

fn drive_stream(body: Vec<u8>, limits: Bounds) -> bounded_inference::AttemptRecord {
    drive_stream_with_token(body, limits, "synthetic-access")
}
fn drive_stream_with_token(
    body: Vec<u8>,
    limits: Bounds,
    token: &str,
) -> bounded_inference::AttemptRecord {
    let root = root();
    let (_, snapshot) = bearer(&root.join("auth"), token);
    let profile = ToolFreeProfile {
        revision: "selected-reasoning-v1".into(),
        model: "fake-account-model".into(),
        bounds: limits,
    };
    let request = assemble_request("i", "x", &profile).unwrap();
    let path = root.join("attempt.json");
    let mut journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = peer.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut socket, _) = peer.accept().unwrap();
        let _ = read_request(&socket);
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        socket.write_all(head.as_bytes()).unwrap();
        for chunk in body.chunks(1) {
            if socket.write_all(chunk).is_err() {
                break;
            }
        }
        peer.set_nonblocking(true).unwrap();
        assert!(peer.accept().is_err());
    });
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
        None,
    );
    let _ = transport.execute(&request, &snapshot, &mut journal, &AtomicBool::new(false));
    handle.join().unwrap();
    let result = AttemptJournal::open(&path).unwrap().record().clone();
    std::fs::remove_dir_all(root).unwrap();
    result
}
#[test]
fn live_socket_stream_faults_preserve_unknown_and_specific_reason() {
    let created = event(
        json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-local","model":"fake-account-model"}}),
    );
    let mut truncated = created.clone().into_bytes();
    truncated.extend_from_slice(b"data: {");
    let r = drive_stream(truncated, bounds());
    assert_eq!(r.state, AttemptState::OutcomeUnknown);
    assert_eq!(
        r.observation.unwrap().local_failure,
        Some(StreamError::MalformedEvent)
    );

    let conflict = [
        created.clone(),
        event(json!({"type":"response.in_progress","sequence_number":2,"response":{"id":"other","status":"in_progress"}})),
    ].concat().into_bytes();
    assert_eq!(
        drive_stream(conflict, bounds())
            .observation
            .unwrap()
            .local_failure,
        Some(StreamError::ConflictingIdentity)
    );

    let tool = [created.clone(), event(json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"x","type":"function_call"}}))].concat().into_bytes();
    assert_eq!(
        drive_stream(tool, bounds())
            .observation
            .unwrap()
            .local_failure,
        Some(StreamError::UnsupportedTool)
    );

    let refusal = [
        created.clone(),
        event(json!({"type":"response.refusal.delta","sequence_number":2,"delta":"denied"})),
    ]
    .concat()
    .into_bytes();
    assert_eq!(
        drive_stream(refusal, bounds())
            .observation
            .unwrap()
            .local_failure,
        Some(StreamError::UnsupportedContent)
    );

    let mut tiny = bounds();
    tiny.frame_bytes = 32;
    assert_eq!(
        drive_stream(created.clone().into_bytes(), tiny)
            .observation
            .unwrap()
            .local_failure,
        Some(StreamError::FrameTooLarge)
    );
    let mut tiny = bounds();
    tiny.stream_bytes = 64;
    tiny.frame_bytes = 64;
    assert_eq!(
        drive_stream(created.clone().into_bytes(), tiny)
            .observation
            .unwrap()
            .local_failure,
        Some(StreamError::StreamTooLarge)
    );
    let mut tiny = bounds();
    tiny.events = 1;
    let two = [created.clone(), event(json!({"type":"response.in_progress","sequence_number":2,"response":{"id":"resp-local","status":"in_progress"}}))].concat().into_bytes();
    assert_eq!(
        drive_stream(two, tiny).observation.unwrap().local_failure,
        Some(StreamError::TooManyEvents)
    );
    let mut tiny = bounds();
    tiny.output_bytes = 1;
    let output = [
        created,
        event(json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"x","type":"message"}})),
        event(json!({"type":"response.output_text.delta","sequence_number":3,"output_index":0,"item_id":"x","content_index":0,"delta":"too much"})),
    ].concat().into_bytes();
    assert_eq!(
        drive_stream(output, tiny)
            .observation
            .unwrap()
            .local_failure,
        Some(StreamError::OutputTooLarge)
    );

    let wrong_model = [
        event(json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-local","model":"other-model"}})),
        event(json!({"type":"response.completed","sequence_number":2,"response":{"id":"resp-local","model":"other-model","status":"completed","output":[]}})),
    ].concat().into_bytes();
    assert_eq!(
        drive_stream(wrong_model, bounds())
            .observation
            .unwrap()
            .local_failure,
        Some(StreamError::ConflictingIdentity)
    );
}
#[test]
fn failed_and_incomplete_terminals_are_not_completed_output() {
    for (kind, details) in [
        ("failed", json!({"error":{"code":"quota"}})),
        (
            "incomplete",
            json!({"incomplete_details":{"reason":"limit"}}),
        ),
    ] {
        let mut response = json!({"id":"resp-local","model":"fake-account-model","status":kind});
        response
            .as_object_mut()
            .unwrap()
            .extend(details.as_object().unwrap().clone());
        let body = [
            event(json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-local","model":"fake-account-model"}})),
            event(json!({"type":format!("response.{kind}"),"sequence_number":2,"response":response})),
        ].concat().into_bytes();
        let r = drive_stream(body, bounds());
        assert_eq!(r.state, AttemptState::TerminalObserved);
        assert!(r.observation.as_ref().unwrap().output.is_none());
        assert!(r.observation.as_ref().unwrap().usage.is_empty());
        assert_eq!(r.http.as_ref().unwrap().status, Some(200));
    }
}

#[test]
fn deadline_and_header_limits_do_not_make_completion() {
    {
        let root = root();
        let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
        let mut p = profile();
        p.bounds.local_deadline_ms = 120;
        let request = assemble_request("i", "x", &p).unwrap();
        let path = root.join("attempt.json");
        let mut journal = AttemptJournal::create(
            &path,
            AttemptRecord::prepared_bound(
                "op",
                "attempt",
                request.sha256.clone(),
                request.profile_revision.clone(),
                request.selected_model.clone(),
                request.bounds.clone(),
                snapshot.registration.clone(),
            )
            .unwrap(),
        )
        .unwrap();
        let peer = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = peer.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (socket, _) = peer.accept().unwrap();
            let _ = read_request(&socket);
            thread::sleep(std::time::Duration::from_millis(250));
        });
        let transport = LocalHttpTransport::new(
            LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
            None,
        );
        assert!(transport
            .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
            .is_err());
        handle.join().unwrap();
        assert_eq!(
            AttemptJournal::open(&path).unwrap().record().state,
            AttemptState::OutcomeUnknown
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    let root = root();
    let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = root.join("attempt.json");
    let mut journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = peer.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut socket, _) = peer.accept().unwrap();
        let _ = read_request(&socket);
        let large = "x".repeat(17 * 1024);
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nX-Large: {large}\r\nContent-Length: 0\r\n\r\n");
        let _ = socket.write_all(response.as_bytes());
    });
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
        None,
    );
    let _ = transport.execute(&request, &snapshot, &mut journal, &AtomicBool::new(false));
    handle.join().unwrap();
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::OutcomeUnknown
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn pre_send_cancel_keeps_prepared() {
    let root = root();
    let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = root.join("attempt.json");
    let mut journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new("http://127.0.0.1:9/v1/responses", None).unwrap(),
        None,
    );
    assert!(transport
        .execute(&request, &snapshot, &mut journal, &AtomicBool::new(true))
        .is_err());
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::Prepared
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn post_send_cancel_preserves_uncertainty() {
    let root = root();
    let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = root.join("attempt.json");
    let mut journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = peer.local_addr().unwrap();
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let (sent, received) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut socket, _) = peer.accept().unwrap();
        let _ = read_request(&socket);
        let first = event(
            json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-local"}}),
        );
        let second = event(
            json!({"type":"response.in_progress","sequence_number":2,"response":{"id":"resp-local","status":"in_progress"}}),
        );
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap();
        socket
            .write_all(format!("{:x}\r\n{}\r\n", first.len(), first).as_bytes())
            .unwrap();
        sent.send(()).unwrap();
        thread::sleep(std::time::Duration::from_millis(80));
        let _ = socket.write_all(format!("{:x}\r\n{}\r\n", second.len(), second).as_bytes());
    });
    let setter = {
        let cancel = cancel.clone();
        thread::spawn(move || {
            received.recv().unwrap();
            cancel.store(true, Ordering::Release);
        })
    };
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
        None,
    );
    let _ = transport.execute(&request, &snapshot, &mut journal, &cancel);
    setter.join().unwrap();
    handle.join().unwrap();
    let recovered = AttemptJournal::open(&path).unwrap();
    assert_eq!(recovered.record().state, AttemptState::OutcomeUnknown);
    assert_eq!(
        recovered
            .record()
            .observation
            .as_ref()
            .unwrap()
            .local_failure,
        Some(StreamError::Cancelled)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ambiguous_socket_close_does_not_retransmit_original_attempt() {
    let root = root();
    let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = root.join("attempt.json");
    let mut journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = peer.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (socket, _) = peer.accept().unwrap();
        let _ = read_request(&socket);
        drop(socket);
        peer.set_nonblocking(true).unwrap();
        for _ in 0..20 {
            if let Ok((_second, _)) = peer.accept() {
                panic!("original attempt retransmitted");
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
        1usize
    });
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
        None,
    );
    assert!(transport
        .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
        .is_err());
    assert_eq!(handle.join().unwrap(), 1);
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::OutcomeUnknown
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn credential_rotation_cannot_retarget_issued_attempt() {
    let root = root();
    let (store, snapshot) = bearer(&root.join("auth"), "old-synthetic-access");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = root.join("attempt.json");
    let mut journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let t = now();
    let newer = store.refresh_with(&snapshot.registration, t, JWKS, |_| {
        let mut h = Header::new(Algorithm::RS256); h.kid = Some("synthetic-key-1".into());
        let id = encode(&h, &json!({"iss":"https://fake-issuer.invalid","aud":"fake-client","sub":"fake-subject","exp":t+3600,"iat":t-1}), &EncodingKey::from_rsa_der(KEY)).unwrap();
        Ok(TokenResponse::from_json(serde_json::to_string(&json!({"access_token":"new-synthetic-access","refresh_token":"new-refresh","id_token":id,"token_type":"Bearer","expires_in":3600,"scope":SCOPE})).unwrap().as_bytes()).unwrap())
    }).unwrap();
    assert_ne!(
        newer.credential_revision,
        snapshot.registration.credential_revision
    );
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = peer.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut socket, _) = peer.accept().unwrap();
        let (head, _) = read_request(&socket);
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: bearer old-synthetic-access"));
        assert!(!head.contains("new-synthetic-access"));
        let body = stream();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        socket.write_all(response.as_bytes()).unwrap();
        socket.write_all(body.as_bytes()).unwrap();
    });
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
        None,
    );
    assert_eq!(
        transport
            .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
            .unwrap(),
        AttemptState::TerminalObserved
    );
    handle.join().unwrap();
    assert_eq!(
        AttemptJournal::open(&path)
            .unwrap()
            .record()
            .registration
            .as_ref(),
        Some(&snapshot.registration)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn absolute_deadline_covers_preheader_stall_and_body_drip() {
    for preheader in [true, false] {
        let root = root();
        let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
        let mut p = profile();
        p.bounds.local_deadline_ms = 120;
        let request = assemble_request("i", "x", &p).unwrap();
        let path = root.join("attempt.json");
        let mut journal = AttemptJournal::create(
            &path,
            AttemptRecord::prepared_bound(
                "op",
                "attempt",
                request.sha256.clone(),
                request.profile_revision.clone(),
                request.selected_model.clone(),
                request.bounds.clone(),
                snapshot.registration.clone(),
            )
            .unwrap(),
        )
        .unwrap();
        let peer = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = peer.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut socket, _) = peer.accept().unwrap();
            let _ = read_request(&socket);
            if preheader {
                thread::sleep(std::time::Duration::from_millis(250));
                let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 0\r\n\r\n");
            } else {
                let body = stream();
                let head = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n", body.len());
                socket.write_all(head.as_bytes()).unwrap();
                for b in body.as_bytes().iter().take(10) {
                    if socket.write_all(&[*b]).is_err() {
                        break;
                    }
                    thread::sleep(std::time::Duration::from_millis(30));
                }
            }
        });
        let transport = LocalHttpTransport::new(
            LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
            None,
        );
        let start = std::time::Instant::now();
        let _ = transport.execute(&request, &snapshot, &mut journal, &AtomicBool::new(false));
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(220),
            "elapsed={elapsed:?}"
        );
        handle.join().unwrap();
        let r = AttemptJournal::open(&path).unwrap();
        assert_eq!(r.record().state, AttemptState::OutcomeUnknown);
        assert_eq!(
            r.record().observation.as_ref().unwrap().local_failure,
            Some(StreamError::Timeout)
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn library_header_ceiling_rejects_oversized_wire_header() {
    let root = root();
    let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = root.join("attempt.json");
    let mut journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = peer.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut socket, _) = peer.accept().unwrap();
        let _ = read_request(&socket);
        let header = "x".repeat(420_000);
        let response = format!("HTTP/1.1 200 OK\r\nX-Huge: {header}\r\nContent-Type: text/event-stream\r\nContent-Length: 0\r\n\r\n");
        for chunk in response.as_bytes().chunks(8192) {
            if socket.write_all(chunk).is_err() {
                break;
            }
        }
    });
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
        None,
    );
    assert!(transport
        .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
        .is_err());
    handle.join().unwrap();
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::OutcomeUnknown
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn journal_faults_before_and_after_entry_preserve_recovery_truth() {
    // A failed durable send-intent commit forbids any network entry.
    {
        let root = root();
        let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
        let request = assemble_request("i", "x", &profile()).unwrap();
        let path = root.join("attempt.json");
        let mut journal = AttemptJournal::create(
            &path,
            AttemptRecord::prepared_bound(
                "op",
                "attempt",
                request.sha256.clone(),
                request.profile_revision.clone(),
                request.selected_model.clone(),
                request.bounds.clone(),
                snapshot.registration.clone(),
            )
            .unwrap(),
        )
        .unwrap();
        let lock = path.with_extension("writer-lock");
        std::fs::create_dir(&lock).unwrap();
        let peer = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = peer.local_addr().unwrap();
        peer.set_nonblocking(true).unwrap();
        let transport = LocalHttpTransport::new(
            LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
            None,
        );
        assert!(transport
            .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
            .is_err());
        assert!(peer.accept().is_err());
        assert_eq!(
            AttemptJournal::open(&path).unwrap().record().state,
            AttemptState::Prepared
        );
        std::fs::remove_dir(&lock).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    // Once a request was seen, a failed evidence/terminal commit keeps the
    // durable send intent. Recovery cannot infer completion or authorize replay.
    let root = root();
    let (_, snapshot) = bearer(&root.join("auth"), "synthetic-access");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = root.join("attempt.json");
    let mut journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let lock = path.with_extension("writer-lock");
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = peer.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut socket, _) = peer.accept().unwrap();
        let _ = read_request(&socket);
        std::fs::remove_file(&lock).unwrap();
        std::fs::create_dir(&lock).unwrap();
        let body = stream();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        socket.write_all(response.as_bytes()).unwrap();
        socket.write_all(body.as_bytes()).unwrap();
    });
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new(&format!("http://{addr}/v1/responses"), None).unwrap(),
        None,
    );
    assert!(transport
        .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
        .is_err());
    handle.join().unwrap();
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::PossiblyEntered
    );
    assert!(transport
        .execute(&request, &snapshot, &mut journal, &AtomicBool::new(false))
        .is_err());
    std::fs::remove_dir(path.with_extension("writer-lock")).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

fn refresh_peer(
    reply: Option<&'static str>,
) -> (std::net::SocketAddr, Vec<u8>, thread::JoinHandle<Vec<u8>>) {
    let key = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let der = key.cert.der().to_vec();
    let root_der = der.clone();
    let private = rustls::pki_types::PrivatePkcs8KeyDer::from(key.signing_key.serialize_der());
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![rustls::pki_types::CertificateDer::from(der)],
            private.into(),
        )
        .unwrap();
    let peer = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = peer.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (socket, _) = peer.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let mut tls = rustls::StreamOwned::new(
            rustls::ServerConnection::new(std::sync::Arc::new(config)).unwrap(),
            socket,
        );
        let (head, body) = read_tls_request(&mut tls);
        assert!(head.starts_with("POST /oauth/token HTTP/1.1\r\n"));
        if let Some(reply) = reply {
            tls.write_all(reply.as_bytes()).unwrap();
            tls.flush().unwrap();
        }
        body
    });
    (addr, root_der, handle)
}

fn retain_refresh_evidence(name: &str, evidence: &bounded_inference::RefreshExchangeEvidence) {
    if let Ok(dir) = std::env::var("B2B_REFRESH_EVIDENCE_DIR") {
        std::fs::write(
            Path::new(&dir).join(format!("{name}.json")),
            serde_json::to_vec_pretty(evidence).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn refresh_http_classifies_preentry_and_terminal_and_never_replays_inference() {
    use bounded_inference::{AuthError, Readiness};
    let root = root();
    let (store, snapshot) = bearer(&root.join("auth"), "refresh-test-access");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = root.join("unentered.json");
    let _journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256,
            request.profile_revision,
            request.selected_model,
            request.bounds,
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = closed.local_addr().unwrap();
    drop(closed);
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new(
            &format!("https://localhost:{}/oauth/token", addr.port()),
            Some(addr),
        )
        .unwrap(),
        None,
    );
    let mut pre_evidence = None;
    assert!(matches!(
        store.refresh_with(&snapshot.registration, now(), JWKS, |grant| {
            let (result, evidence) = transport.exchange_refresh_with_evidence(grant);
            pre_evidence = Some(evidence);
            result
        }),
        Err(AuthError::Unavailable)
    ));
    let pre_evidence = pre_evidence.unwrap();
    assert_eq!(pre_evidence.registration, snapshot.registration);
    assert_eq!(pre_evidence.pinned_loopback, Some(addr));
    assert_eq!(
        pre_evidence.outcome,
        bounded_inference::RefreshExchangeOutcome::Temporary
    );
    retain_refresh_evidence("pre-entry", &pre_evidence);
    assert!(matches!(store.readiness(now()), Ok(Readiness::Ready(_))));
    let reply = "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: 25\r\n\r\n{\"error\":\"invalid_grant\"}";
    let (addr, der, handle) = refresh_peer(Some(reply));
    let transport = LocalHttpTransport::with_fake_root_der(
        LocalEndpoint::new(
            &format!("https://localhost:{}/oauth/token", addr.port()),
            Some(addr),
        )
        .unwrap(),
        &der,
    )
    .unwrap();
    let mut terminal_evidence = None;
    assert!(matches!(
        store.refresh_with(&snapshot.registration, now(), JWKS, |grant| {
            let (result, evidence) = transport.exchange_refresh_with_evidence(grant);
            terminal_evidence = Some(evidence);
            result
        }),
        Err(AuthError::Unavailable)
    ));
    let terminal_evidence = terminal_evidence.unwrap();
    assert_eq!(terminal_evidence.registration, snapshot.registration);
    assert_eq!(terminal_evidence.pinned_loopback, Some(addr));
    assert_eq!(terminal_evidence.http_status, Some(400));
    assert_eq!(
        terminal_evidence.outcome,
        bounded_inference::RefreshExchangeOutcome::Terminal
    );
    assert_eq!(
        terminal_evidence.fake_root_sha256.as_deref(),
        Some(format!("{:x}", sha2::Sha256::digest(&der)).as_str())
    );
    retain_refresh_evidence("terminal", &terminal_evidence);
    let body = handle.join().unwrap();
    let fields: std::collections::HashMap<_, _> =
        url::form_urlencoded::parse(&body).into_owned().collect();
    assert_eq!(
        fields.get("grant_type").map(String::as_str),
        Some("refresh_token")
    );
    assert_eq!(
        fields.get("refresh_token").map(String::as_str),
        Some("fake-refresh")
    );
    assert_eq!(
        fields.get("client_id").map(String::as_str),
        Some("fake-client")
    );
    assert!(matches!(
        store.readiness(now()),
        Ok(Readiness::ReauthorizationRequired(_))
    ));
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::Prepared
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn refresh_invalid_client_keeps_offline_credential_renewable() {
    use bounded_inference::{AuthError, Readiness, RefreshExchangeOutcome};
    let root = root();
    let (store, snapshot) = bearer(&root.join("auth"), "refresh-test-access");
    let reply = "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: 26\r\n\r\n{\"error\":\"invalid_client\"}";
    let (addr, der, handle) = refresh_peer(Some(reply));
    let transport = LocalHttpTransport::with_fake_root_der(
        LocalEndpoint::new(
            &format!("https://localhost:{}/oauth/token", addr.port()),
            Some(addr),
        )
        .unwrap(),
        &der,
    )
    .unwrap();
    let mut observed = None;
    assert!(matches!(
        store.refresh_with(&snapshot.registration, now(), JWKS, |grant| {
            let (result, evidence) = transport.exchange_refresh_with_evidence(grant);
            observed = Some(evidence);
            result
        }),
        Err(AuthError::RefreshConfiguration)
    ));
    assert_eq!(
        observed.unwrap().outcome,
        RefreshExchangeOutcome::Configuration
    );
    assert!(matches!(store.readiness(now()), Ok(Readiness::Ready(_))));
    assert_eq!(
        store
            .bearer(&snapshot.registration, now())
            .unwrap()
            .registration,
        snapshot.registration
    );
    handle.join().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn refresh_lost_response_is_ambiguous_and_does_not_enter_inference() {
    use bounded_inference::{AuthError, Readiness};
    let root = root();
    let (store, snapshot) = bearer(&root.join("auth"), "refresh-test-access");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let path = root.join("unentered.json");
    let _journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "op",
            "attempt",
            request.sha256,
            request.profile_revision,
            request.selected_model,
            request.bounds,
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    let (addr, der, handle) = refresh_peer(None);
    let transport = LocalHttpTransport::with_fake_root_der(
        LocalEndpoint::new(
            &format!("https://localhost:{}/oauth/token", addr.port()),
            Some(addr),
        )
        .unwrap(),
        &der,
    )
    .unwrap();
    let mut lost_evidence = None;
    assert!(matches!(
        store.refresh_with(&snapshot.registration, now(), JWKS, |grant| {
            let (result, evidence) = transport.exchange_refresh_with_evidence(grant);
            lost_evidence = Some(evidence);
            result
        }),
        Err(AuthError::AmbiguousRefresh)
    ));
    let lost_evidence = lost_evidence.unwrap();
    assert_eq!(lost_evidence.registration, snapshot.registration);
    assert_eq!(lost_evidence.pinned_loopback, Some(addr));
    assert_eq!(
        lost_evidence.outcome,
        bounded_inference::RefreshExchangeOutcome::Ambiguous
    );
    assert_eq!(
        lost_evidence.fake_root_sha256.as_deref(),
        Some(format!("{:x}", sha2::Sha256::digest(&der)).as_str())
    );
    retain_refresh_evidence("ambiguous", &lost_evidence);
    let body = handle.join().unwrap();
    assert!(body
        .windows(b"fake-refresh".len())
        .any(|w| w == b"fake-refresh"));
    assert!(matches!(
        store.readiness(now()),
        Ok(Readiness::RefreshAmbiguous(_))
    ));
    assert_eq!(
        AttemptJournal::open(&path).unwrap().record().state,
        AttemptState::Prepared
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn sse_bearer_echo_is_scrubbed_from_every_durable_field() {
    let secret = "bearer-review-secret";
    let created = |id: &str| {
        event(json!({"type":"response.created","sequence_number":1,
        "response":{"id":id,"model":"fake-account-model"}}))
    };
    let terminal = |kind: &str, details: Value| {
        let mut response = json!({"id":"safe-id","model":"fake-account-model","status":kind,
            "usage":{"input_tokens":2,"output_tokens":1,"total_tokens":3}});
        response
            .as_object_mut()
            .unwrap()
            .extend(details.as_object().unwrap().clone());
        event(json!({"type":format!("response.{kind}"),"sequence_number":2,"response":response}))
    };
    let cases = [
        [
            created("safe-id"),
            terminal("failed", json!({"error":{"code":secret}})),
        ]
        .concat(),
        [
            created("safe-id"),
            terminal(
                "incomplete",
                json!({"incomplete_details":{"reason":secret}}),
            ),
        ]
        .concat(),
        [
            created("safe-id"),
            event(json!({"type":"error","sequence_number":2,"code":secret})),
        ]
        .concat(),
        event(json!({"type":"response.created","sequence_number":1,
            "response":{"id":secret,"model":"fake-account-model","status":"completed"}})),
        event(json!({"type":"response.created","sequence_number":1,
            "response":{"id":"safe-id","model":secret,"status":"completed"}})),
        stream().replace("αβ", secret),
        [
            created("safe-id"),
            terminal("failed", json!({"error":{"code":"safe-code"},"output":[]})),
        ]
        .concat(),
    ];
    for (case, body) in cases.into_iter().enumerate() {
        let record = drive_stream_with_token(body.into_bytes(), bounds(), secret);
        let serialized = serde_json::to_vec(&record).unwrap();
        assert!(
            !serialized
                .windows(secret.len())
                .any(|v| v == secret.as_bytes()),
            "case {case}"
        );
        if case < 6 {
            assert_eq!(record.state, AttemptState::OutcomeUnknown, "case {case}");
            let observation = record.observation.unwrap();
            assert_eq!(observation.local_failure, Some(StreamError::CredentialEcho));
            assert!(observation.output.is_none());
            if case < 2 {
                assert_eq!(observation.usage.len(), 1);
            }
        } else {
            assert_eq!(record.state, AttemptState::TerminalObserved);
        }
    }
}

#[test]
fn reasoning_nested_summary_fields_fail_at_added_done_and_terminal() {
    let bad = json!({"type":"summary_text","text":"internal",
        "tool_calls":[{"type":"function_call","name":"execute","arguments":"{}"}]});
    for boundary in ["added", "done", "terminal"] {
        let mut events = vec![
            event(
                json!({"type":"response.created","sequence_number":1,"response":{"id":"r","model":"fake-account-model"}}),
            ),
            event(
                json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,
                "item":{"id":"reason","type":"reasoning","summary":if boundary=="added" {vec![bad.clone()]} else {vec![]}}}),
            ),
        ];
        if boundary != "added" {
            events.push(event(json!({"type":"response.output_item.done","sequence_number":3,"output_index":0,
                "item":{"id":"reason","type":"reasoning","status":"completed",
                    "summary":if boundary=="done" {vec![bad.clone()]} else {vec![json!({"type":"summary_text","text":"internal"})]}}})));
        }
        if boundary == "terminal" {
            events.push(event(json!({"type":"response.completed","sequence_number":4,
                "response":{"id":"r","model":"fake-account-model","status":"completed",
                    "output":[{"id":"reason","type":"reasoning","status":"completed","summary":[bad.clone()]}]}})));
        }
        let record = drive_stream(events.concat().into_bytes(), bounds());
        assert_eq!(record.state, AttemptState::OutcomeUnknown, "{boundary}");
        assert_eq!(
            record.observation.unwrap().local_failure,
            Some(StreamError::UnsupportedContent)
        );
    }
}

#[cfg(feature = "journal-test-faults")]
#[test]
fn journal_persistence_fault_child() {
    let Ok(spec) = std::env::var("B2B_JOURNAL_FAULT_SPEC") else {
        return;
    };
    let path = PathBuf::from(std::env::var("B2B_JOURNAL_FAULT_PATH").unwrap());
    let address: std::net::SocketAddr = std::env::var("B2B_JOURNAL_PEER").unwrap().parse().unwrap();
    let (store, snapshot) = bearer(&path.parent().unwrap().join("auth"), "fault-only-bearer");
    let request = assemble_request("i", "x", &profile()).unwrap();
    let mut journal = AttemptJournal::create(
        &path,
        AttemptRecord::prepared_bound(
            "fault-op",
            "fault-attempt",
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            snapshot.registration.clone(),
        )
        .unwrap(),
    )
    .unwrap();
    drop(store);
    let transport = LocalHttpTransport::new(
        LocalEndpoint::new(&format!("http://{address}/v1/responses"), None).unwrap(),
        None,
    );
    let result = transport.execute(&request, &snapshot, &mut journal, &AtomicBool::new(false));
    assert!(spec.starts_with("io:"), "exit fault did not fire at {spec}");
    assert!(result.is_err(), "injected I/O fault did not fail execution");
}

#[cfg(feature = "journal-test-faults")]
#[test]
fn journal_real_persistence_boundaries_survive_fault_and_crash() {
    use std::process::Command;
    use std::sync::Arc;
    use std::time::Duration;
    let binary = std::env::current_exe().unwrap();
    for stage in ["intent", "terminal"] {
        for point in [
            "before_write",
            "after_write",
            "after_file_sync",
            "after_rename",
            "after_parent_sync",
        ] {
            for mode in ["io", "exit"] {
                let root = root();
                let path = root.join("attempt.json");
                let peer = TcpListener::bind("127.0.0.1:0").unwrap();
                peer.set_nonblocking(true).unwrap();
                let address = peer.local_addr().unwrap();
                let stop = Arc::new(AtomicBool::new(false));
                let server_stop = stop.clone();
                let body = stream().into_bytes();
                let server = thread::spawn(move || {
                    let mut count = 0;
                    while !server_stop.load(Ordering::Acquire) {
                        match peer.accept() {
                            Ok((mut socket, _)) => {
                                count += 1;
                                let _ = read_request(&socket);
                                let head = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n", body.len());
                                socket.write_all(head.as_bytes()).unwrap();
                                socket.write_all(&body).unwrap();
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(2))
                            }
                            Err(e) => panic!("listener error: {e}"),
                        }
                    }
                    count
                });
                let spec = format!("{mode}:{stage}:{point}");
                let child = Command::new(&binary)
                    .args(["--exact", "journal_persistence_fault_child", "--nocapture"])
                    .env("B2B_JOURNAL_FAULT_SPEC", &spec)
                    .env("B2B_JOURNAL_FAULT_PATH", &path)
                    .env("B2B_JOURNAL_PEER", address.to_string())
                    .output()
                    .unwrap();
                stop.store(true, Ordering::Release);
                let sends = server.join().unwrap();
                assert_eq!(
                    child.status.code(),
                    Some(if mode == "exit" { 73 } else { 0 }),
                    "{spec}: {}",
                    String::from_utf8_lossy(&child.stderr)
                );
                assert_eq!(sends, if stage == "intent" { 0 } else { 1 }, "{spec}");
                let mut recovered = AttemptJournal::open(&path).unwrap();
                let published = matches!(point, "after_rename" | "after_parent_sync");
                let expected = match (stage, published) {
                    ("intent", false) => AttemptState::Prepared,
                    ("intent", true) | ("terminal", false) => AttemptState::PossiblyEntered,
                    ("terminal", true) => AttemptState::TerminalObserved,
                    _ => unreachable!(),
                };
                assert_eq!(recovered.record().state, expected, "{spec}");
                if expected != AttemptState::Prepared {
                    assert!(
                        recovered.mark_send_intent().is_err(),
                        "{spec} allowed replay"
                    );
                }
                if stage == "terminal" {
                    assert_eq!(recovered.record().http.as_ref().unwrap().status, Some(200));
                }
                if let Ok(dir) = std::env::var("B2B_JOURNAL_EVIDENCE_DIR") {
                    let receipt = json!({"fault":spec,"binary":binary,"profile_revision":"selected-reasoning-v1",
                        "child_exit":child.status.code(),"socket_requests":sends,"recovered_state":expected,
                        "http_status":recovered.record().http.as_ref().and_then(|h|h.status)});
                    std::fs::write(
                        Path::new(&dir).join(format!("{mode}-{stage}-{point}.json")),
                        serde_json::to_vec_pretty(&receipt).unwrap(),
                    )
                    .unwrap();
                }
                std::fs::remove_dir_all(root).unwrap();
            }
        }
    }
}
