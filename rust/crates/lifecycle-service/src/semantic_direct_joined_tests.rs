//! The real `bounded-inference` adapter joined to the real service composition.
//!
//! Evidence class: synthetic. `DirectAdapter` + `LoopbackEndpoint` +
//! `assemble_request` + `LocalHttpTransport` + `AttemptJournal` talk HTTP to a
//! loopback peer; `establish_checkpoint` drives them with the real store,
//! assessment, currentness fence and publication. This shows the layers
//! compose and that a loopback endpoint is never eligible, even with the
//! owner's grant. It is not a public HTTPS exchange and says nothing about a
//! provider. The `PublicClassDouble` below wraps the loopback endpoint and
//! only *claims* the public class so the owner-grant logic can run over the
//! real adapter; it is test-only and supplies no production evidence.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bounded_inference::{
    AssembledRequest, AuthStore, Bounds, DirectAdapter, DirectEndpoint, DirectExchange,
    DirectProfile, Issuer, LocalEndpoint, LocalHttpTransport, LoopbackEndpoint, TokenResponse,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use lifecycle_semantic::{
    DirectEvidenceClass, DirectInferencePort, DirectProductionProfile, FindingStatus,
};
use lifecycle_store::{DirectCheckpointOrigin, StoreError};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::semantic_direct_tests::{
    Harness, ISSUER, assert_no_checkpoint, assert_not_authoritative, assert_unadmitted,
};

const JWKS: &str = include_str!("../../bounded-inference/tests/fixtures/synthetic-siwc-jwks.json");
const KEY: &[u8] =
    include_bytes!("../../bounded-inference/tests/fixtures/synthetic-siwc-rsa.pkcs1.der");
const SCOPE: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
const MODEL: &str = "joined-loopback-model";
const TOKEN: &str = "synthetic-access-joined";

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn bounds() -> Bounds {
    Bounds {
        request_bytes: 512 * 1024,
        stream_bytes: 512 * 1024,
        frame_bytes: 256 * 1024,
        output_bytes: 128 * 1024,
        events: 64,
        local_deadline_ms: 5_000,
    }
}

fn bearer_store(root: &Path) -> (Arc<AuthStore>, bounded_inference::RegistrationRef) {
    let issuer = Issuer::new(
        "https://fake-issuer.invalid",
        "https://fake-issuer.invalid/authorize",
        "https://fake-issuer.invalid/token",
        "https://fake-issuer.invalid/revoke",
    )
    .unwrap();
    let store = AuthStore::open(root, "synthetic_host_joined", issuer).unwrap();
    let t = now();
    let start = store
        .begin_registration("Synthetic Tool", "http://127.0.0.1:45123/auth/callback", t)
        .unwrap();
    let url = url::Url::parse(start.authorize_url()).unwrap();
    let value = |key: &str| {
        url.query_pairs().find(|(k, _)| k == key).unwrap().1.to_string()
    };
    let (state, nonce) = (value("state"), value("nonce"));
    let client = "fake-client";
    let grant = store
        .accept_callback(
            &format!(
                "http://127.0.0.1:45123/auth/callback?code=fake&state={state}&client_id={client}"
            ),
            t,
        )
        .unwrap();
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("synthetic-key-1".into());
    let id = encode(
        &header,
        &json!({"iss":"https://fake-issuer.invalid","aud":client,"sub":"fake-subject",
                "exp":t+3600,"iat":t-1,"nonce":nonce}),
        &EncodingKey::from_rsa_der(KEY),
    )
    .unwrap();
    let response = TokenResponse::from_json(
        serde_json::to_string(&json!({"access_token":TOKEN,"refresh_token":"fake-refresh",
            "id_token":id,"token_type":"Bearer","expires_in":3600,"scope":SCOPE}))
        .unwrap()
        .as_bytes(),
    )
    .unwrap();
    let registration = store.finish_exchange(&grant, response, JWKS, t).unwrap();
    (Arc::new(store), registration)
}

// ---- Loopback Responses peer -------------------------------------------------

enum Step {
    /// A complete HTTP 200 event stream.
    Stream(String),
    /// Read the request, then close without answering.
    Hangup,
    /// An HTTP error status with an empty body.
    Status(u16),
}

struct Seen {
    head: String,
    body: Vec<u8>,
}

struct Api {
    address: SocketAddr,
    seen: Arc<Mutex<Vec<Seen>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

fn read_request(mut socket: &TcpStream) -> Option<Seen> {
    socket.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    let mut all = Vec::new();
    let mut buf = [0u8; 4096];
    let end = loop {
        let n = socket.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        all.extend_from_slice(&buf[..n]);
        if let Some(i) = all.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = String::from_utf8(all[..end].to_vec()).ok()?;
    let len: usize = head.lines().find_map(|line| {
        line.to_ascii_lowercase()
            .strip_prefix("content-length: ")
            .and_then(|n| n.trim().parse().ok())
    })?;
    let mut body = all[end..].to_vec();
    while body.len() < len {
        let n = socket.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        body.extend_from_slice(&buf[..n]);
    }
    Some(Seen { head, body })
}

impl Api {
    fn start(steps: Vec<Step>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (record, stopping) = (seen.clone(), stop.clone());
        let worker = thread::spawn(move || {
            let mut steps = steps.into_iter();
            while !stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        socket.set_nonblocking(false).unwrap();
                        let Some(request) = read_request(&socket) else { continue };
                        record.lock().unwrap().push(request);
                        match steps.next() {
                            Some(Step::Stream(body)) => {
                                let head = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nX-Request-Id: req_joined\r\n\r\n",
                                    body.len()
                                );
                                let _ = socket.write_all(head.as_bytes());
                                let _ = socket.write_all(body.as_bytes());
                            }
                            Some(Step::Status(code)) => {
                                let _ = socket.write_all(
                                    format!("HTTP/1.1 {code} Error\r\nContent-Length: 0\r\n\r\n")
                                        .as_bytes(),
                                );
                            }
                            Some(Step::Hangup) | None => {}
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => return,
                }
            }
        });
        Self { address, seen, stop, worker: Some(worker) }
    }

    fn requests(&self) -> usize {
        self.seen.lock().unwrap().len()
    }
}

impl Drop for Api {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn event(value: Value) -> String {
    format!("event: {}\r\ndata: {}\r\n\r\n", value["type"].as_str().unwrap(), value)
}

/// One complete Responses stream whose output text is `text` from `model`.
fn stream(text: &str, model: &str, terminal: &str) -> String {
    let completed = json!({"type":"response.completed","sequence_number":4,"response":{
        "id":"resp-joined","model":model,"status":"completed","output":[{"id":"msg-1",
        "type":"message","role":"assistant","status":"completed",
        "content":[{"type":"output_text","text":text}]}]}});
    let failed = json!({"type":"response.failed","sequence_number":4,"response":{
        "id":"resp-joined","model":model,"status":"failed",
        "error":{"code":"server_error","message":"synthetic"}}});
    [
        json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-joined","model":model}}),
        json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"msg-1","type":"message"}}),
        json!({"type":"response.output_text.delta","sequence_number":3,"output_index":0,"item_id":"msg-1","content_index":0,"delta":text}),
        if terminal == "failed" { failed } else { completed },
    ]
    .into_iter()
    .map(event)
    .collect()
}

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap()
}

// ---- Adapters ----------------------------------------------------------------

struct Rig {
    dir: tempfile::TempDir,
    auth: Arc<AuthStore>,
    registration: bounded_inference::RegistrationRef,
}

impl Rig {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let (auth, registration) = bearer_store(&dir.path().join("auth"));
        Self { dir, auth, registration }
    }

    fn journals(&self) -> PathBuf {
        self.dir.path().join("journals")
    }

    fn loopback(&self, address: SocketAddr) -> LoopbackEndpoint {
        let auth = self.auth.clone();
        let registration = self.registration.clone();
        LoopbackEndpoint::new(
            LocalHttpTransport::new(
                LocalEndpoint::new(&format!("http://{address}/v1/responses"), None).unwrap(),
                None,
            ),
            move || auth.bearer(&registration, now()).ok(),
            "fixture:joined-loopback",
        )
    }

    fn profile() -> DirectProfile {
        DirectProfile { model: MODEL.into(), bounds: bounds() }
    }

    /// The real adapter over the loopback endpoint (synthetic class).
    fn adapter(&self, address: SocketAddr) -> Arc<dyn DirectInferencePort> {
        Arc::new(DirectAdapter::new(self.loopback(address), Self::profile(), self.journals()))
    }

    /// A scripted double that only claims the public class over the real
    /// adapter mechanics. Test-only; not provider evidence.
    fn public_class_double(&self, address: SocketAddr) -> Arc<dyn DirectInferencePort> {
        Arc::new(DirectAdapter::new(
            PublicClassDouble(self.loopback(address)),
            Self::profile(),
            self.journals(),
        ))
    }
}

struct PublicClassDouble(LoopbackEndpoint);

impl DirectEndpoint for PublicClassDouble {
    fn provider(&self) -> &str {
        self.0.provider()
    }

    fn evidence(&self) -> DirectEvidenceClass {
        DirectProductionProfile::public_api()
    }

    fn exchange(
        &self,
        journal_path: &Path,
        operation_id: &str,
        attempt_id: &str,
        request: &AssembledRequest,
    ) -> DirectExchange {
        self.0.exchange(journal_path, operation_id, attempt_id, request)
    }
}

fn good_steps(harness: &Harness) -> Vec<Step> {
    vec![
        Step::Stream(stream(&text(harness.candidate_json()), MODEL, "completed")),
        Step::Stream(stream(&text(harness.report_json(FindingStatus::Satisfied)), MODEL, "completed")),
    ]
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// ---- Tests -------------------------------------------------------------------

#[tokio::test]
async fn real_adapter_carries_two_distinct_requests_and_loopback_stays_proof_only_even_with_owner_grant() {
    let (harness, mut store) = Harness::new();
    harness.install_admission(&mut store, ISSUER).unwrap();
    let rig = Rig::new();
    let api = Api::start(good_steps(&harness));
    let runtime = harness.runtime_with(rig.adapter(api.address));
    let result = harness.establish_production(&mut store, &runtime).await;
    // The owner's grant is installed, but a loopback capture is synthetic.
    assert!(matches!(result, Ok(None)), "{result:?}");
    assert_eq!(api.requests(), 2);
    assert_unadmitted(&store, &harness.transition);
    assert!(store.direct_proof_checkpoint(&harness.transition).unwrap().is_some());
    assert!(matches!(
        store.semantic_checkpoint_origin(&harness.transition).unwrap(),
        Some(DirectCheckpointOrigin::Direct { .. })
    ));

    // Separate compile and verify requests reached the wire, with the bearer
    // supplied by the credential owner, and each is exactly the prepared wire
    // the store recorded before sending.
    let seen = api.seen.lock().unwrap();
    for (index, operation) in ["direct-compile-1", "direct-verify-1"].iter().enumerate() {
        assert!(seen[index].head.starts_with("POST /v1/responses HTTP/1.1\r\n"));
        assert!(
            seen[index].head.to_ascii_lowercase().contains(&format!("authorization: bearer {TOKEN}")),
        );
        let request: Value = serde_json::from_slice(&seen[index].body).unwrap();
        assert_eq!(request["model"], MODEL);
        assert_eq!(request["stream"], true);
        let recorded: String = rusqlite::Connection::open(harness.root.join("lifecycle.sqlite"))
            .unwrap()
            .query_row(
                "SELECT prepared_wire_sha256 FROM direct_semantic_attempts WHERE operation_id=?1",
                [operation],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sha(&seen[index].body), recorded, "{operation}");
    }
    assert_ne!(seen[0].body, seen[1].body);
    drop(seen);

    // The credential never reaches durable lifecycle state.
    let database = std::fs::read(harness.root.join("lifecycle.sqlite")).unwrap();
    assert!(!database.windows(TOKEN.len()).any(|w| w == TOKEN.as_bytes()));
}

#[tokio::test]
async fn real_adapter_with_a_scripted_public_class_and_owner_grant_admits_without_inventing_evidence() {
    // Grant logic only: the double claims the class; nothing observed one.
    let (harness, mut store) = Harness::new();
    harness.install_admission(&mut store, ISSUER).unwrap();
    let rig = Rig::new();
    let api = Api::start(good_steps(&harness));
    let runtime = harness.runtime_with(rig.public_class_double(api.address));
    let result = harness.establish_production(&mut store, &runtime).await;
    assert!(matches!(result, Ok(Some(_))), "{result:?}");
    assert_eq!(api.requests(), 2);
    assert!(store.admitted_direct_checkpoint(&harness.transition).unwrap().is_some());
    assert!(matches!(
        store.semantic_checkpoint_origin(&harness.transition).unwrap(),
        Some(DirectCheckpointOrigin::DirectAdmitted { .. })
    ));
    assert!(store.semantic_checkpoint(&harness.transition).unwrap().is_none());

    // The same double without the owner's grant is proof only.
    let (harness, mut store) = Harness::new();
    let rig = Rig::new();
    let api = Api::start(good_steps(&harness));
    let runtime = harness.runtime_with(rig.public_class_double(api.address));
    let result = harness.establish_production(&mut store, &runtime).await;
    assert!(matches!(result, Ok(None)), "{result:?}");
    assert_unadmitted(&store, &harness.transition);
}

#[tokio::test]
async fn rejected_malformed_failed_wrong_model_and_http_error_verifiers_cannot_publish() {
    type Variant = (&'static str, Box<dyn Fn(&Harness) -> Step>);
    let variants: Vec<Variant> = vec![
        ("rejecting_verdict", Box::new(|h| {
            Step::Stream(stream(&text(h.rejecting_report_json()), MODEL, "completed"))
        })),
        ("malformed_output", Box::new(|_| {
            Step::Stream(stream("not json", MODEL, "completed"))
        })),
        ("failed_terminal", Box::new(|h| {
            Step::Stream(stream(&text(h.report_json(FindingStatus::Satisfied)), MODEL, "failed"))
        })),
        ("wrong_model", Box::new(|h| {
            Step::Stream(stream(&text(h.report_json(FindingStatus::Satisfied)), "other-model", "completed"))
        })),
        ("http_500", Box::new(|_| Step::Status(500))),
    ];
    for (name, verifier) in variants {
        let (harness, mut store) = Harness::new();
        harness.install_admission(&mut store, ISSUER).unwrap();
        let rig = Rig::new();
        let api = Api::start(vec![
            Step::Stream(stream(&text(harness.candidate_json()), MODEL, "completed")),
            verifier(&harness),
        ]);
        // Even a double claiming the public class with the owner's grant cannot
        // publish an unusable verifier call.
        let runtime = harness.runtime_with(rig.public_class_double(api.address));
        let result = harness.establish_production(&mut store, &runtime).await;
        assert_eq!(api.requests(), 2, "{name}");
        assert_no_checkpoint(&harness, &store, result).await;
        assert_eq!(
            harness.count("SELECT COUNT(*) FROM direct_semantic_results WHERE verdict='accepted'"),
            0,
            "{name}"
        );
        assert!(store.admitted_direct_checkpoint(&harness.transition).unwrap().is_none(), "{name}");

        // A restart cannot resend either call.
        drop(store);
        let mut store = harness.reopen();
        let again = harness.runtime_with(rig.public_class_double(api.address));
        let _ = harness.establish_production(&mut store, &again).await;
        assert_eq!(api.requests(), 2, "{name}: restart resent a call");
        assert!(store.admitted_direct_checkpoint(&harness.transition).unwrap().is_none(), "{name}");
        assert_not_authoritative(&store, &harness.transition);
    }
}

#[tokio::test]
async fn lost_compile_outcome_is_fenced_and_never_resent_after_restart() {
    let (harness, mut store) = Harness::new();
    let rig = Rig::new();
    // The peer reads the request and hangs up, then would answer anything.
    let mut steps = vec![Step::Hangup];
    steps.extend(good_steps(&harness));
    let api = Api::start(steps);
    let runtime = harness.runtime_with(rig.adapter(api.address));
    let result = harness.establish_production(&mut store, &runtime).await;
    assert!(matches!(result, Err(StoreError::ClaimConflict)), "{result:?}");
    assert_eq!(api.requests(), 1);
    drop(store);

    // New process: fresh store handle, fresh adapter, same journals.
    let mut store = harness.reopen();
    let runtime = harness.runtime_with(rig.adapter(api.address));
    let result = harness.establish_production(&mut store, &runtime).await;
    assert!(matches!(result, Err(StoreError::ClaimConflict)), "{result:?}");
    assert_eq!(api.requests(), 1, "an unknown outcome is never offered again");
    assert_no_checkpoint(&harness, &store, result).await;
}

#[tokio::test]
async fn completed_result_survives_restart_without_any_new_request() {
    let (harness, mut store) = Harness::new();
    harness.install_admission(&mut store, ISSUER).unwrap();
    let rig = Rig::new();
    let api = Api::start(good_steps(&harness));
    let runtime = harness.runtime_with(rig.public_class_double(api.address));
    let first = harness.establish_production(&mut store, &runtime).await.unwrap();
    assert!(first.is_some());
    assert_eq!(api.requests(), 2);
    drop(store);

    let mut store = harness.reopen();
    let runtime = harness.runtime_with(rig.public_class_double(api.address));
    let second = harness.establish_production(&mut store, &runtime).await.unwrap();
    assert_eq!(second, first, "the same checkpoint, idempotently");
    assert_eq!(api.requests(), 2, "restart must not retransmit");
}

#[tokio::test]
async fn source_stale_after_the_real_calls_fences_the_output() {
    let (harness, mut store) = Harness::new();
    harness.install_admission(&mut store, ISSUER).unwrap();
    let rig = Rig::new();
    let api = Api::start(good_steps(&harness));
    let runtime = harness.runtime_with(rig.public_class_double(api.address));
    harness.material.stale.store(true, Ordering::SeqCst);
    let result = harness.establish_production(&mut store, &runtime).await;
    assert!(matches!(&result, Ok(None)), "{result:?}");
    assert_eq!(api.requests(), 2);
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_results"), 0);
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM semantic_captures WHERE eligibility!='current'"),
        1
    );
    assert_no_checkpoint(&harness, &store, result).await;
}
