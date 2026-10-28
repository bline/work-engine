//! Adapter from the lifecycle direct-control seam to the bounded-inference
//! request, transport and attempt-journal owners.
//!
//! The adapter assembles the wire with `assemble_request`, submits it once
//! through an endpoint, and fills a capture only from the journal's recorded
//! `StreamObservation`. It assigns no semantic verdict and no lifecycle identity
//! beyond the operation and attempt it is handed. Credential and grant handling
//! stays with the endpoint's owner.
//!
//! Endpoints: `LoopbackEndpoint` is the local fixture transport and labels its
//! evidence synthetic. `PublicHttpsEndpoint` exercises the fixed public origin
//! with a bearer closure and the selected observer; it has no production
//! constructor, because a bearer alone carries no catalog, funding, custody or
//! single-use admission. Production public sends use
//! `public_host::PublicHostEndpoint`, which admits each exact prepared call
//! through `PublicHost` and parses with `b2-public-siwc-stream-v2`. The
//! `PublicApi` class is a transport fact; it grants no eligibility. The owner's
//! admission grant, read by the lifecycle store at publication, does.
//!
//! No endpoint here is a CLI or live entry.

use std::{
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use lifecycle_semantic::{
    DirectCapture, DirectEvidenceClass, DirectInferencePort, DirectPreparedCall,
    DirectProductionProfile, DirectReleaseState, DirectRole, DirectSubmitError, DirectTerminal,
    DirectWire,
};
use sha2::{Digest, Sha256};

use crate::{
    assemble_request,
    local_http::{bounded, empty_failure, observe_response},
    AssembledRequest, AttemptJournal, AttemptRecord, AttemptState, AuthError,
    BearerAccess, Bounds, LocalHttpTransport, ObservedTerminal, PublicHttps, PublicHttpsError, StreamError,
    StreamObservation, ToolFreeProfile, SELECTED_REASONING_PROFILE_REVISION,
};

/// What one exchange left behind. `NotSent` is claimed only when the journal
/// shows no send intent was committed. `Withheld` is claimed only when the
/// endpoint refused before consuming any release, custody or action identity.
pub enum DirectExchange {
    Observed(Box<AttemptRecord>),
    NotSent,
    OutcomeUnknown,
    Withheld,
}

/// The transport owner behind the adapter.
pub trait DirectEndpoint: Send + Sync {
    /// Provider identity the endpoint actually reaches.
    fn provider(&self) -> &str;
    /// Evidence class this endpoint's captures belong to.
    fn evidence(&self) -> DirectEvidenceClass;
    /// Create the attempt journal at `journal_path` and perform at most one
    /// exchange for `request`.
    fn exchange(
        &self,
        journal_path: &Path,
        operation_id: &str,
        attempt_id: &str,
        request: &AssembledRequest,
    ) -> DirectExchange;
    /// Request and observation profile this endpoint parses. It is explicit
    /// per endpoint and recorded in the prepared wire; never inferred.
    fn profile_revision(&self) -> &str {
        SELECTED_REASONING_PROFILE_REVISION
    }
    /// One exchange for an exact prepared call. Endpoints whose admission binds
    /// the prepared call override this; the default ignores the binding.
    fn exchange_prepared(
        &self,
        journal_path: &Path,
        call: &DirectPreparedCall,
        request: &AssembledRequest,
    ) -> DirectExchange {
        self.exchange(
            journal_path,
            &call.binding.operation_id,
            &call.binding.attempt_id,
            request,
        )
    }
    /// Digest recorded as `captured_stream_sha256`. By default it is the
    /// serialized normalized `StreamObservation`, not raw bytes. An endpoint
    /// that retains the complete raw body may return that body's digest; `None`
    /// leaves the capture unattributable.
    fn capture_digest(&self, record: &AttemptRecord) -> Option<String> {
        normalized_observation_digest(record)
    }
    /// Release state for an exact prepared call that has no journal yet.
    /// Endpoints whose sends need an external owner release override this;
    /// it must consume nothing.
    fn offer_release(&self, journal_path: &Path, call: &DirectPreparedCall) -> DirectReleaseState {
        let _ = (journal_path, call);
        DirectReleaseState::Ready
    }
    /// Whether custody shared beyond this call's journal may already have
    /// consumed its identity, so that an exchange may have begun elsewhere.
    /// It must consume nothing, and answer `true` when that custody cannot be
    /// read. The default is for endpoints whose journal is their only custody.
    fn identity_may_be_consumed(&self, call: &DirectPreparedCall) -> bool {
        let _ = call;
        false
    }
}

/// Digest of the serialized normalized observation; not an SSE byte archive.
pub(crate) fn normalized_observation_digest(record: &AttemptRecord) -> Option<String> {
    let observation = record.observation.as_ref()?;
    serde_json::to_vec(observation).ok().map(|bytes| hex(&bytes))
}

/// Loopback fixture endpoint. Real credentials never pass through it: the
/// bearer is supplied by a closure over the existing `AuthStore`.
pub struct LoopbackEndpoint {
    transport: LocalHttpTransport,
    bearer: Box<dyn Fn() -> Option<BearerAccess> + Send + Sync>,
    fixture_ref: String,
}

impl LoopbackEndpoint {
    pub fn new(
        transport: LocalHttpTransport,
        bearer: impl Fn() -> Option<BearerAccess> + Send + Sync + 'static,
        fixture_ref: impl Into<String>,
    ) -> Self {
        Self {
            transport,
            bearer: Box::new(bearer),
            fixture_ref: fixture_ref.into(),
        }
    }
}

impl DirectEndpoint for LoopbackEndpoint {
    fn provider(&self) -> &str {
        "local_loopback"
    }

    fn evidence(&self) -> DirectEvidenceClass {
        DirectEvidenceClass::SyntheticLocal {
            fixture_ref: self.fixture_ref.clone(),
        }
    }

    fn exchange(
        &self,
        journal_path: &Path,
        operation_id: &str,
        attempt_id: &str,
        request: &AssembledRequest,
    ) -> DirectExchange {
        // An existing journal means an earlier submission may have happened.
        if journal_path.exists() {
            return DirectExchange::OutcomeUnknown;
        }
        let Some(bearer) = (self.bearer)() else {
            return DirectExchange::NotSent;
        };
        let Ok(record) = AttemptRecord::prepared_bound(
            operation_id,
            attempt_id,
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            bearer.registration.clone(),
        ) else {
            return DirectExchange::NotSent;
        };
        let Ok(mut journal) = AttemptJournal::create(journal_path, record) else {
            return DirectExchange::NotSent;
        };
        let cancelled = AtomicBool::new(false);
        let outcome = self
            .transport
            .execute(request, &bearer, &mut journal, &cancelled);
        let record = journal.record().clone();
        match (outcome, record.state) {
            (_, AttemptState::Prepared) => DirectExchange::NotSent,
            (Ok(_), AttemptState::TerminalObserved) => DirectExchange::Observed(Box::new(record)),
            _ => DirectExchange::OutcomeUnknown,
        }
    }
}

/// Fixed public Responses endpoint. One exchange per journal, never replayed.
///
/// The client is built inside each exchange's own runtime so no pooled
/// connection outlives it, and the bearer is read once at the send boundary.
pub struct PublicHttpsEndpoint {
    https: Box<dyn Fn() -> Result<PublicHttps, PublicHttpsError> + Send + Sync>,
    bearer: Box<dyn Fn() -> Option<BearerAccess> + Send + Sync>,
}

/// A bearer that expires inside this margin cannot fund a send.
const BEARER_MARGIN_SECS: u64 = 30;

impl PublicHttpsEndpoint {
    /// Superseded production construction; always fails closed.
    ///
    /// AuthStore readiness alone does not establish the owner's model catalog,
    /// funding, custody or globally unconsumed action identity, so this
    /// endpoint can no longer be composed for public sends. Production Direct
    /// sends use `PublicHostEndpoint::open`, which consumes those owner inputs
    /// through `PublicHost`. The signature is retained so existing composition
    /// reports a configuration failure instead of sending.
    pub fn from_credential_store(
        _credential_dir: &std::path::Path,
        _host_id: &str,
    ) -> Result<Self, AuthError> {
        Err(AuthError::Unavailable)
    }

    #[cfg(test)]
    pub(crate) fn with_client(
        https: impl Fn() -> Result<PublicHttps, PublicHttpsError> + Send + Sync + 'static,
        bearer: impl Fn() -> Option<BearerAccess> + Send + Sync + 'static,
    ) -> Self {
        Self {
            https: Box::new(https),
            bearer: Box::new(bearer),
        }
    }
}

impl DirectEndpoint for PublicHttpsEndpoint {
    fn provider(&self) -> &str {
        "openai_responses"
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
        if journal_path.exists() {
            return DirectExchange::OutcomeUnknown;
        }
        if request.profile_revision != SELECTED_REASONING_PROFILE_REVISION
            || request.bytes.len() > request.bounds.request_bytes
            || hex(&request.bytes) != request.sha256
        {
            return DirectExchange::NotSent;
        }
        let Some(bearer) = (self.bearer)() else {
            return DirectExchange::NotSent;
        };
        let Some(raw_token) = bearer
            .authorization_value()
            .strip_prefix("Bearer ")
            .map(str::to_owned)
        else {
            return DirectExchange::NotSent;
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(u64::MAX);
        if bearer.expires_at < now.saturating_add(BEARER_MARGIN_SECS) {
            return DirectExchange::NotSent;
        }
        let Ok(record) = AttemptRecord::prepared_bound(
            operation_id,
            attempt_id,
            request.sha256.clone(),
            request.profile_revision.clone(),
            request.selected_model.clone(),
            request.bounds.clone(),
            bearer.registration.clone(),
        ) else {
            return DirectExchange::NotSent;
        };
        let Ok(mut journal) = AttemptJournal::create(journal_path, record) else {
            return DirectExchange::NotSent;
        };
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return DirectExchange::NotSent;
        };
        let https = {
            let _entered = runtime.enter();
            match (self.https)() {
                Ok(https) => https,
                Err(_) => return DirectExchange::NotSent,
            }
        };
        let cancelled = AtomicBool::new(false);
        if journal.mark_send_intent().is_err() {
            return DirectExchange::NotSent;
        }
        let deadline_at = tokio::time::Instant::now()
            + std::time::Duration::from_millis(request.bounds.local_deadline_ms);
        let sent = runtime.block_on(bounded(
            https.send_response(&bearer, request.bytes.clone()),
            &cancelled,
            deadline_at,
        ));
        let response = match sent {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => {
                let _ = journal.mark_unknown(empty_failure(StreamError::TransportFailure));
                return DirectExchange::OutcomeUnknown;
            }
            Err(error) => {
                let _ = journal.mark_unknown(empty_failure(error));
                return DirectExchange::OutcomeUnknown;
            }
        };
        let outcome = observe_response(
            &runtime,
            response,
            &raw_token,
            request,
            &mut journal,
            &cancelled,
            deadline_at,
        );
        let record = journal.record().clone();
        match (outcome, record.state) {
            (Ok(_), AttemptState::TerminalObserved) => DirectExchange::Observed(Box::new(record)),
            _ => DirectExchange::OutcomeUnknown,
        }
    }
}

/// Selected request configuration. Never inferred from a response.
#[derive(Clone, Debug)]
pub struct DirectProfile {
    pub model: String,
    pub bounds: Bounds,
}

pub struct DirectAdapter<E> {
    endpoint: E,
    profile: DirectProfile,
    journal_dir: PathBuf,
}

const COMPILE_INSTRUCTIONS: &str =
    "Follow the user message exactly and reply with the single JSON object it requests.";
const VERIFY_INSTRUCTIONS: &str =
    "Independently judge the candidate in the user message and reply with the single JSON verdict it requests.";

impl<E: DirectEndpoint> DirectAdapter<E> {
    pub fn new(endpoint: E, profile: DirectProfile, journal_dir: impl Into<PathBuf>) -> Self {
        Self {
            endpoint,
            profile,
            journal_dir: journal_dir.into(),
        }
    }

    fn tool_free_profile(&self) -> ToolFreeProfile {
        ToolFreeProfile {
            revision: self.endpoint.profile_revision().into(),
            model: self.profile.model.clone(),
            bounds: self.profile.bounds.clone(),
        }
    }

    fn journal_path(&self, call: &DirectPreparedCall) -> PathBuf {
        self.journal_dir
            .join(format!("{}.attempt.json", call.binding.operation_id.replace('/', "_")))
    }
}

fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn capture_from<E: DirectEndpoint>(
    call: &DirectPreparedCall,
    endpoint: &E,
    record: &AttemptRecord,
) -> Result<DirectCapture, DirectSubmitError> {
    let provider = endpoint.provider();
    let evidence = endpoint.evidence();
    let observation: &StreamObservation = record
        .observation
        .as_ref()
        .ok_or(DirectSubmitError::OutcomeUnknown)?;
    if record.operation_id != call.binding.operation_id
        || record.attempt_id != call.binding.attempt_id
        || record.request_sha256 != call.wire_sha256
        || record.profile_revision != call.wire.profile_revision
        || record.selected_model != call.wire.requested_model
    {
        return Err(DirectSubmitError::OutcomeUnknown);
    }
    let captured_stream_sha256 = endpoint
        .capture_digest(record)
        .ok_or(DirectSubmitError::OutcomeUnknown)?;
    let (terminal, output) = match (&observation.terminal, &observation.output) {
        // A completed terminal without model-attributed text is not usable.
        (Some(ObservedTerminal::Completed), Some(text)) if !text.is_empty() => {
            (DirectTerminal::Complete, text.as_bytes().to_vec())
        }
        (Some(ObservedTerminal::Incomplete { .. }), _) => (DirectTerminal::Incomplete, Vec::new()),
        _ => (DirectTerminal::Failed, Vec::new()),
    };
    Ok(DirectCapture {
        operation_id: call.binding.operation_id.clone(),
        attempt_id: call.binding.attempt_id.clone(),
        prepared_wire_sha256: call.wire_sha256.clone(),
        prepared_binding_sha256: call.binding_sha256.clone(),
        capture_ref: format!("capture:{}", call.binding.operation_id),
        captured_stream_sha256,
        terminal,
        response_id: observation.response_id.clone(),
        observed_provider: Some(provider.to_owned()),
        observed_model: observation.observed_model.clone(),
        output_sha256: hex(&output),
        output,
        evidence,
    })
}

impl<E: DirectEndpoint> DirectInferencePort for DirectAdapter<E> {
    fn evidence(&self) -> DirectEvidenceClass {
        self.endpoint.evidence()
    }

    fn wire(&self, role: DirectRole, input: &str) -> Result<DirectWire, DirectSubmitError> {
        let instructions = match role {
            DirectRole::Compile => COMPILE_INSTRUCTIONS,
            DirectRole::Verify => VERIFY_INSTRUCTIONS,
        };
        let request = assemble_request(instructions, input, &self.tool_free_profile())
            .map_err(|_| DirectSubmitError::NotSent)?;
        Ok(DirectWire {
            instructions: instructions.into(),
            wire_bytes: request.bytes,
            profile_revision: request.profile_revision,
            requested_provider: self.endpoint.provider().into(),
            requested_model: request.selected_model,
        })
    }

    /// A journal already at this operation's path means an exchange may have
    /// begun: the release owner is not consulted, and `submit` reports that
    /// journal (never a resend). Otherwise the endpoint decides.
    fn offer_release(&self, call: &DirectPreparedCall) -> DirectReleaseState {
        let path = self.journal_path(call);
        if std::fs::symlink_metadata(&path).is_ok() {
            return DirectReleaseState::Ready;
        }
        self.endpoint.offer_release(&path, call)
    }

    fn submit(&self, call: &DirectPreparedCall) -> Result<DirectCapture, DirectSubmitError> {
        let request = assemble_request(
            &call.wire.instructions,
            &call.input,
            &self.tool_free_profile(),
        )
        .map_err(|_| DirectSubmitError::NotSent)?;
        // The bytes offered for submission must be exactly the prepared wire.
        if request.bytes != call.wire.wire_bytes
            || request.sha256 != call.wire_sha256
            || call.wire.requested_provider != self.endpoint.provider()
            || call.wire.profile_revision != request.profile_revision
            || call.wire.requested_model != request.selected_model
        {
            return Err(DirectSubmitError::NotSent);
        }
        // The journal directory is private to this adapter's owner. Failing to
        // create it refuses before the endpoint, but `Withheld` lets the
        // lifecycle re-offer the call, so it also needs positive evidence that
        // no exchange began: no journal at this call's path and no identity
        // consumed in the endpoint's shared custody. Otherwise the outcome of
        // an earlier exchange stays unknown.
        if std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.journal_dir)
            .is_err()
        {
            let journal_absent = matches!(
                std::fs::symlink_metadata(self.journal_path(call)),
                Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory)
            );
            return Err(if journal_absent && !self.endpoint.identity_may_be_consumed(call) {
                DirectSubmitError::Withheld
            } else {
                DirectSubmitError::OutcomeUnknown
            });
        }
        let path = self.journal_path(call);
        match self.endpoint.exchange_prepared(&path, call, &request) {
            DirectExchange::Observed(record) => capture_from(call, &self.endpoint, &record),
            DirectExchange::NotSent => Err(DirectSubmitError::NotSent),
            DirectExchange::OutcomeUnknown => Err(DirectSubmitError::OutcomeUnknown),
            DirectExchange::Withheld => Err(DirectSubmitError::Withheld),
        }
    }
}

/// `PublicHttpsEndpoint` against a synthetic TLS peer.
///
/// Evidence class: synthetic. The peer presents a throwaway certificate for
/// `api.openai.com`, which only the test-only `PublicHttps::synthetic` client
/// trusts and resolves to it. These tests exercise the endpoint's own
/// mechanics: fixed request shape, one exchange per journal, send-intent
/// custody, bearer handling and the shared response observer. The `public_api`
/// class they observe is what the endpoint reports by construction, not an
/// observation of OpenAI. No constructor reachable outside `cfg(test)` can
/// substitute the trust root or the peer.
#[cfg(test)]
mod public_tls_tests {
    use super::*;
    use crate::{AuthStore, Issuer, TokenResponse};
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use serde_json::{json, Value};
    use std::{
        io::{Read, Write},
        net::{SocketAddr, TcpListener},
        sync::{
            atomic::{AtomicU64, AtomicUsize, Ordering},
            Arc, Mutex,
        },
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    static SERIAL: AtomicU64 = AtomicU64::new(0);
    const JWKS: &str = include_str!("../tests/fixtures/synthetic-siwc-jwks.json");
    const KEY: &[u8] = include_bytes!("../tests/fixtures/synthetic-siwc-rsa.pkcs1.der");
    const SCOPE: &str =
        "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
    const MODEL: &str = "fake-public-model";
    const TOKEN: &str = "synthetic-access-tls";

    fn root() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "b3-public-tls-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        p
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn bounds() -> Bounds {
        Bounds {
            request_bytes: 8192,
            stream_bytes: 16384,
            frame_bytes: 8192,
            output_bytes: 1024,
            events: 32,
            local_deadline_ms: 3000,
        }
    }

    fn bearer_store(root: &Path, expires_in: u64) -> (Arc<AuthStore>, crate::RegistrationRef) {
        let issuer = Issuer::new(
            "https://fake-issuer.invalid",
            "https://fake-issuer.invalid/authorize",
            "https://fake-issuer.invalid/token",
            "https://fake-issuer.invalid/revoke",
        )
        .unwrap();
        ready_store(root, "synthetic_host_tls", issuer, "https://fake-issuer.invalid", expires_in)
    }

    /// A store made ready from synthetic token material only. `iss` is the
    /// issuer the synthetic id token claims.
    fn ready_store(
        root: &Path,
        host_id: &str,
        issuer: Issuer,
        iss: &str,
        expires_in: u64,
    ) -> (Arc<AuthStore>, crate::RegistrationRef) {
        let store = AuthStore::open(root, host_id, issuer).unwrap();
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
            &json!({"iss":iss,"aud":client,"sub":"fake-subject",
                    "exp":t+3600,"iat":t-1,"nonce":nonce}),
            &EncodingKey::from_rsa_der(KEY),
        )
        .unwrap();
        let response = TokenResponse::from_json(
            serde_json::to_string(&json!({"access_token":TOKEN,"refresh_token":"fake-refresh",
                "id_token":id,"token_type":"Bearer","expires_in":expires_in,"scope":SCOPE}))
            .unwrap()
            .as_bytes(),
        )
        .unwrap();
        let registration = store.finish_exchange(&grant, response, JWKS, t).unwrap();
        (Arc::new(store), registration)
    }

    enum Step {
        Stream(String),
        Status(u16),
    }

    /// Request head and body as the peer received them.
    type Received = Vec<(String, Vec<u8>)>;

    struct Peer {
        address: SocketAddr,
        root_der: Vec<u8>,
        accepted: Arc<AtomicUsize>,
        seen: Arc<Mutex<Received>>,
        stop: Arc<std::sync::atomic::AtomicBool>,
        worker: Option<thread::JoinHandle<()>>,
    }

    fn read_request(io: &mut impl Read) -> Option<(String, Vec<u8>)> {
        let mut all = Vec::new();
        let mut buf = [0u8; 4096];
        let end = loop {
            let n = io.read(&mut buf).ok()?;
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
            let n = io.read(&mut buf).ok()?;
            if n == 0 {
                return None;
            }
            body.extend_from_slice(&buf[..n]);
        }
        Some((head, body))
    }

    impl Peer {
        fn start(steps: Vec<Step>) -> Self {
            let cert = rcgen::generate_simple_self_signed(vec!["api.openai.com".into()]).unwrap();
            let root_der = cert.cert.der().to_vec();
            let private =
                rustls::pki_types::PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
            let config = Arc::new(
                rustls::ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(
                        vec![rustls::pki_types::CertificateDer::from(root_der.clone())],
                        private.into(),
                    )
                    .unwrap(),
            );
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            listener.set_nonblocking(true).unwrap();
            let accepted = Arc::new(AtomicUsize::new(0));
            let seen = Arc::new(Mutex::new(Vec::new()));
            let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let (counted, recorded, stopping) = (accepted.clone(), seen.clone(), stop.clone());
            let worker = thread::spawn(move || {
                let mut steps = steps.into_iter();
                while !stopping.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((socket, _)) => {
                            counted.fetch_add(1, Ordering::SeqCst);
                            socket.set_nonblocking(false).unwrap();
                            socket
                                .set_read_timeout(Some(Duration::from_secs(3)))
                                .unwrap();
                            let mut tls = rustls::StreamOwned::new(
                                rustls::ServerConnection::new(config.clone()).unwrap(),
                                socket,
                            );
                            let Some(request) = read_request(&mut tls) else {
                                continue;
                            };
                            recorded.lock().unwrap().push(request);
                            let reply = match steps.next() {
                                Some(Step::Stream(body)) => format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nX-Request-Id: req_tls\r\nConnection: close\r\n\r\n{body}",
                                    body.len()
                                ),
                                Some(Step::Status(code)) => format!(
                                    "HTTP/1.1 {code} Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                                ),
                                None => continue,
                            };
                            let _ = tls.write_all(reply.as_bytes());
                            let _ = tls.flush();
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => return,
                    }
                }
            });
            Self {
                address,
                root_der,
                accepted,
                seen,
                stop,
                worker: Some(worker),
            }
        }

        fn accepted(&self) -> usize {
            self.accepted.load(Ordering::SeqCst)
        }
    }

    impl Drop for Peer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    fn event(v: Value) -> String {
        format!("event: {}\r\ndata: {}\r\n\r\n", v["type"].as_str().unwrap(), v)
    }

    fn stream(text: &str, terminal: &str) -> String {
        let completed = json!({"type":"response.completed","sequence_number":4,"response":{
            "id":"resp-tls","model":MODEL,"status":"completed","output":[{"id":"msg-1",
            "type":"message","role":"assistant","status":"completed",
            "content":[{"type":"output_text","text":text}]}]}});
        let failed = json!({"type":"response.failed","sequence_number":4,"response":{
            "id":"resp-tls","model":MODEL,"status":"failed",
            "error":{"code":"server_error","message":"synthetic"}}});
        [
            json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-tls","model":MODEL}}),
            json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"msg-1","type":"message"}}),
            json!({"type":"response.output_text.delta","sequence_number":3,"output_index":0,"item_id":"msg-1","content_index":0,"delta":text}),
            if terminal == "failed" { failed } else { completed },
        ]
        .into_iter()
        .map(event)
        .collect()
    }

    fn adapter(
        root: &Path,
        trusted_root: Vec<u8>,
        address: SocketAddr,
        bearer: Arc<dyn Fn() -> Option<BearerAccess> + Send + Sync>,
    ) -> DirectAdapter<PublicHttpsEndpoint> {
        let endpoint = PublicHttpsEndpoint::with_client(
            move || Ok(PublicHttps::synthetic(&trusted_root, address)),
            move || bearer(),
        );
        DirectAdapter::new(
            endpoint,
            DirectProfile {
                model: MODEL.into(),
                bounds: bounds(),
            },
            root.join("journals"),
        )
    }

    fn live_bearer(root: &Path, expires_in: u64) -> Arc<dyn Fn() -> Option<BearerAccess> + Send + Sync> {
        let (store, registration) = bearer_store(&root.join("auth"), expires_in);
        Arc::new(move || store.bearer(&registration, now()).ok())
    }

    fn prepared(adapter: &DirectAdapter<PublicHttpsEndpoint>, operation: &str) -> DirectPreparedCall {
        let input = "synthetic compile input";
        let wire = adapter.wire(DirectRole::Compile, input).unwrap();
        let sha = |bytes: &[u8]| hex(bytes);
        serde_json::from_value(json!({
            "binding": {
                "subject": "subject-x", "transition": "transition-x", "capture_id": "capture-x",
                "role": "compile", "policy_id": "policy-x", "operation_id": operation,
                "attempt_id": format!("attempt-{operation}"), "exposure_id": format!("exposure-{operation}")
            },
            "source_basis_sha256": sha(b"basis"),
            "candidate_sha256": null,
            "compile_binding_sha256": null,
            "input": input,
            "input_sha256": sha(input.as_bytes()),
            "wire_sha256": sha(&wire.wire_bytes),
            "wire": wire,
            "max_output_bytes": 1024,
            "binding_sha256": sha(b"binding"),
        }))
        .unwrap()
    }

    #[test]
    fn success_is_observed_once_over_tls_with_the_fixed_request_shape() {
        let root = root();
        let peer = Peer::start(vec![Step::Stream(stream("{\"ok\":true}", "completed"))]);
        let adapter = adapter(&root, peer.root_der.clone(), peer.address, live_bearer(&root, 3600));
        let call = prepared(&adapter, "tls-op-1");
        assert_eq!(adapter.evidence().label(), "public_api");
        let capture = adapter.submit(&call).unwrap();
        assert_eq!(capture.terminal, DirectTerminal::Complete);
        assert_eq!(capture.output, b"{\"ok\":true}".to_vec());
        assert_eq!(capture.observed_model.as_deref(), Some(MODEL));
        assert_eq!(capture.observed_provider.as_deref(), Some("openai_responses"));
        assert_eq!(capture.evidence.label(), "public_api");
        assert_eq!(capture.evidence.reference(), DirectProductionProfile::PUBLIC_ENDPOINT);
        assert!(DirectProductionProfile::covers(&capture.evidence));
        assert_eq!(capture.prepared_wire_sha256, call.wire_sha256);

        let seen = peer.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        let head = seen[0].0.to_ascii_lowercase();
        assert!(head.starts_with("post /v1/responses http/1.1\r\n"), "{head}");
        assert!(head.contains(&format!("authorization: bearer {TOKEN}")));
        assert!(head.contains("host: api.openai.com"));
        assert!(head.contains("accept: text/event-stream"));
        assert_eq!(seen[0].1, call.wire.wire_bytes);
        drop(seen);

        // The journal holds this operation: a second submit opens no connection.
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert_eq!(peer.accepted(), 1);
        // The credential is not in the journal.
        let journal = std::fs::read(root.join("journals/tls-op-1.attempt.json")).unwrap();
        assert!(!journal.windows(TOKEN.len()).any(|w| w == TOKEN.as_bytes()));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_or_near_expiry_bearer_is_not_sent_and_opens_no_connection() {
        let root = root();
        let peer = Peer::start(Vec::new());
        let none = adapter(&root, peer.root_der.clone(), peer.address, Arc::new(|| None));
        let call = prepared(&none, "tls-op-2");
        assert_eq!(none.submit(&call).unwrap_err(), DirectSubmitError::NotSent);
        assert!(!root.join("journals/tls-op-2.attempt.json").exists());

        // A bearer inside the 30 second margin cannot fund a send.
        let expiring = adapter(
            &root,
            peer.root_der.clone(),
            peer.address,
            live_bearer(&root, BEARER_MARGIN_SECS - 10),
        );
        let call = prepared(&expiring, "tls-op-3");
        assert_eq!(expiring.submit(&call).unwrap_err(), DirectSubmitError::NotSent);
        assert!(!root.join("journals/tls-op-3.attempt.json").exists());
        assert_eq!(peer.accepted(), 0, "no connection may be made");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn untrusted_peer_or_unreachable_peer_leaves_the_outcome_unknown_and_is_not_replayed() {
        let root = root();
        // A peer presenting a certificate the client does not trust.
        let peer = Peer::start(vec![Step::Stream(stream("x", "completed"))]);
        let other = Peer::start(Vec::new());
        let bearer = live_bearer(&root, 3600);
        let untrusted = adapter(&root, other.root_der.clone(), peer.address, bearer.clone());
        let call = prepared(&untrusted, "tls-op-4");
        assert_eq!(untrusted.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert!(peer.seen.lock().unwrap().is_empty(), "no request completed the handshake");
        assert_eq!(untrusted.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);

        // Nothing listening at all.
        let dead = {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap()
        };
        let unreachable = adapter(&root, peer.root_der.clone(), dead, bearer);
        let call = prepared(&unreachable, "tls-op-5");
        assert_eq!(unreachable.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert_eq!(unreachable.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A journal directory that cannot be created is `Withheld` only while the
    /// journal is positively absent; an unreadable journal leaves a prior send
    /// possible, so the outcome is unknown and the call is never re-offered.
    #[test]
    fn journal_directory_failure_is_withheld_only_when_the_journal_is_absent() {
        use std::os::unix::fs::PermissionsExt;
        let root = root();
        let peer = Peer::start(vec![Step::Stream(stream("{\"ok\":true}", "completed"))]);
        let adapter = adapter(&root, peer.root_der.clone(), peer.address, live_bearer(&root, 3600));
        let call = prepared(&adapter, "tls-op-dir");

        // The directory is a regular file: no journal can exist under it, so
        // the refusal is withheld, nothing is sent, and the call is re-offered.
        std::fs::write(root.join("journals"), b"").unwrap();
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::Withheld);
        assert_eq!(peer.accepted(), 0);
        std::fs::remove_file(root.join("journals")).unwrap();
        assert_eq!(adapter.offer_release(&call), DirectReleaseState::Ready);
        assert_eq!(adapter.submit(&call).unwrap().terminal, DirectTerminal::Complete);
        assert_eq!(peer.accepted(), 1);

        // After that send the directory becomes unreachable. Its journal is
        // the only record of the send, so the refusal is not withheld.
        let lock = |mode| std::fs::set_permissions(&root, std::fs::Permissions::from_mode(mode)).unwrap();
        lock(0o000);
        let refused = adapter.submit(&call);
        lock(0o700);
        assert_eq!(refused.unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert_eq!(peer.accepted(), 1, "never a resend");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_terminal_http_error_and_malformed_stream_yield_no_complete_capture() {
        let root = root();
        let peer = Peer::start(vec![
            Step::Stream(stream("ignored", "failed")),
            Step::Status(401),
            Step::Stream("event: response.created\r\ndata: {not json}\r\n\r\n".into()),
        ]);
        let adapter = adapter(&root, peer.root_der.clone(), peer.address, live_bearer(&root, 3600));
        let failed = adapter.submit(&prepared(&adapter, "tls-op-6")).unwrap();
        assert_eq!(failed.terminal, DirectTerminal::Failed);
        assert!(failed.output.is_empty());
        // An HTTP error status and a malformed stream are both observed as lost
        // outcomes: the request was sent, so the operation is never offered again.
        for operation in ["tls-op-7", "tls-op-8"] {
            let call = prepared(&adapter, operation);
            assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown, "{operation}");
            assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown, "{operation}");
        }
        assert_eq!(peer.accepted(), 3);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn credential_store_construction_fails_closed_and_creates_nothing_when_not_ready() {
        let base = root();
        let missing = base.join("absent");
        assert!(PublicHttpsEndpoint::from_credential_store(&missing, "synthetic_host_cs").is_err());
        assert!(!missing.exists(), "composition must not initialize a store");
        // A relative path and an invalid host id are refused before any I/O.
        assert!(PublicHttpsEndpoint::from_credential_store(Path::new("relative"), "synthetic_host_cs").is_err());
        // An initialized but signed-out store of the fixed issuer is not ready.
        let signed_out = base.join("signed-out");
        AuthStore::open(&signed_out, "synthetic_host_cs", crate::public_https::local_issuer().unwrap()).unwrap();
        assert!(PublicHttpsEndpoint::from_credential_store(&signed_out, "synthetic_host_cs").is_err());
        // A ready store for a different issuer cannot be opened as the fixed one.
        let other = base.join("other-issuer");
        let (_store, _registration) = bearer_store(&other, 3600);
        assert!(PublicHttpsEndpoint::from_credential_store(&other, "synthetic_host_tls").is_err());
    }

    /// Readiness alone no longer composes a public sender: the unadmitted
    /// route is superseded by `PublicHostEndpoint` and fails closed even for a
    /// ready store of the fixed issuer.
    #[test]
    fn ready_credential_store_alone_no_longer_composes_a_public_sender() {
        let base = root();
        let dir = base.join("ready");
        let (store, _registration) = ready_store(
            &dir,
            "synthetic_host_cs",
            crate::public_https::local_issuer().unwrap(),
            "https://auth.openai.com",
            3600,
        );
        drop(store);
        assert!(matches!(
            PublicHttpsEndpoint::from_credential_store(&dir, "synthetic_host_cs"),
            Err(AuthError::Unavailable)
        ));
    }
}
