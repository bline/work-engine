//! Explicit fake-loopback HTTP realization. Each call has one journaled send and no replay.
use crate::{
    AssembledRequest, AttemptJournal, AttemptState, BearerAccess, HttpErrorShape, HttpEvidence,
    JournalError, RefreshFailure, RefreshGrant, RegistrationRef, StreamError, StreamObservation,
    StreamObserver, TokenResponse,
};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::redirect::Policy;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::time::Instant;
use url::Url;

const HTTP_HEADER_LIMIT: usize = 16 * 1024;
const HTTP_ERROR_LIMIT: usize = 16 * 1024;
const MODEL_BODY_LIMIT: usize = 128 * 1024;
const REFRESH_BODY_LIMIT: usize = 64 * 1024;
const REFRESH_REQUEST_LIMIT: usize = 20 * 1024;
const CHUNK: usize = 4096;
pub const SELECTED_REASONING_PROFILE_REVISION: &str = "selected-reasoning-v1";

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiscoveredModel {
    pub slug: String,
    pub visibility: String,
}

/// Nonsecret identity of the exact fake refresh exchange. This records the
/// caller's trusted configuration, not issuer attestation by AuthStore.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshExchangeEvidence {
    pub registration: RegistrationRef,
    pub endpoint_url: String,
    pub pinned_loopback: Option<SocketAddr>,
    pub fake_root_sha256: Option<String>,
    pub http_status: Option<u16>,
    pub outcome: RefreshExchangeOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshExchangeOutcome {
    Success,
    Temporary,
    Configuration,
    Terminal,
    Ambiguous,
}

#[derive(Debug, Error)]
pub enum LocalTransportError {
    #[error("invalid explicit loopback endpoint")]
    Endpoint,
    #[error("request, attempt, profile or credential binding mismatch")]
    Binding,
    #[error("bearer snapshot expired")]
    Expired,
    #[error("request already entered or resolved")]
    AlreadyEntered,
    #[error("journal: {0}")]
    Journal(#[from] JournalError),
    #[error("HTTP client could not be prepared")]
    Client,
    #[error("local HTTP operation failed after durable intent")]
    EnteredFailure,
    #[error("model discovery failed")]
    Models,
}

/// A URL with a literal loopback IP or the pinned name `localhost`.
/// The latter is resolved by reqwest only to the loopback address supplied here.
#[derive(Clone, Debug)]
pub struct LocalEndpoint {
    url: Url,
    localhost_addr: Option<SocketAddr>,
}
impl LocalEndpoint {
    pub fn new(raw: &str, localhost_addr: Option<SocketAddr>) -> Result<Self, LocalTransportError> {
        let url = Url::parse(raw).map_err(|_| LocalTransportError::Endpoint)?;
        if !matches!(url.scheme(), "http" | "https")
            || url.username() != ""
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.port().is_none()
            || !matches!(url.path(), "/v1/responses" | "/v1/models" | "/oauth/token")
        {
            return Err(LocalTransportError::Endpoint);
        }
        match url.host_str() {
            Some("localhost") => {
                let addr = localhost_addr.ok_or(LocalTransportError::Endpoint)?;
                if !addr.ip().is_loopback() || Some(addr.port()) != url.port() {
                    return Err(LocalTransportError::Endpoint);
                }
            }
            Some(host) if host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()) => {
                if localhost_addr.is_some() {
                    return Err(LocalTransportError::Endpoint);
                }
            }
            _ => return Err(LocalTransportError::Endpoint),
        }
        Ok(Self {
            url,
            localhost_addr,
        })
    }
    fn client(
        &self,
        deadline: Duration,
        root: Option<reqwest::Certificate>,
    ) -> Result<Client, LocalTransportError> {
        let mut builder = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .retry(reqwest::retry::never())
            .http1_only()
            .http1_max_headers(64)
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .timeout(deadline)
            .connect_timeout(deadline.min(Duration::from_secs(3)));
        if let Some(addr) = self.localhost_addr {
            builder = builder.resolve("localhost", addr);
        }
        if let Some(cert) = root {
            builder = builder.add_root_certificate(cert);
        }
        builder.build().map_err(|_| LocalTransportError::Client)
    }
}

/// Root certificate is supplied explicitly for fake TLS peers. Verification stays enabled.
pub struct LocalHttpTransport {
    endpoint: LocalEndpoint,
    fake_root: Option<reqwest::Certificate>,
    fake_root_sha256: Option<String>,
}
impl LocalHttpTransport {
    pub fn new(endpoint: LocalEndpoint, fake_root: Option<reqwest::Certificate>) -> Self {
        Self {
            endpoint,
            fake_root,
            fake_root_sha256: None,
        }
    }

    /// Use DER bytes when an exact fake root fingerprint must be retained.
    pub fn with_fake_root_der(
        endpoint: LocalEndpoint,
        der: &[u8],
    ) -> Result<Self, LocalTransportError> {
        let cert = reqwest::Certificate::from_der(der).map_err(|_| LocalTransportError::Client)?;
        Ok(Self {
            endpoint,
            fake_root: Some(cert),
            fake_root_sha256: Some(format!("{:x}", Sha256::digest(der))),
        })
    }

    /// One request. An error after intent leaves the journal possibly entered or
    /// outcome-unknown; callers cannot invoke this method again on that journal.
    pub fn execute(
        &self,
        request: &AssembledRequest,
        bearer: &BearerAccess,
        journal: &mut AttemptJournal,
        cancelled: &AtomicBool,
    ) -> Result<AttemptState, LocalTransportError> {
        if self.endpoint.url.path() != "/v1/responses" {
            return Err(LocalTransportError::Endpoint);
        }
        let r = journal.record();
        if r.state != AttemptState::Prepared {
            return Err(LocalTransportError::AlreadyEntered);
        }
        if r.registration.as_ref() != Some(&bearer.registration)
            || r.request_sha256 != request.sha256
            || r.profile_revision != request.profile_revision
            || r.selected_model != request.selected_model
            || r.bounds != request.bounds
            || request.bytes.len() > request.bounds.request_bytes
            || request.profile_revision != SELECTED_REASONING_PROFILE_REVISION
            || auth_header_size(bearer) > 20 * 1024
            || format!("{:x}", Sha256::digest(&request.bytes)) != request.sha256
        {
            return Err(LocalTransportError::Binding);
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(LocalTransportError::Binding);
        }
        let deadline = Duration::from_millis(request.bounds.local_deadline_ms);
        let runtime = RuntimeBuilder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| LocalTransportError::Client)?;
        let client = {
            let _entered = runtime.enter();
            self.endpoint.client(deadline, self.fake_root.clone())?
        };
        let auth = bearer.authorization_value();
        let raw_token = auth
            .strip_prefix("Bearer ")
            .ok_or(LocalTransportError::Binding)?;
        let auth_header = HeaderValue::from_str(&auth).map_err(|_| LocalTransportError::Binding)?;
        // The final clock read is adjacent to durable intent and entry.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| LocalTransportError::Expired)?
            .as_secs();
        if now >= bearer.expires_at {
            return Err(LocalTransportError::Expired);
        }
        journal.mark_send_intent()?;
        let deadline_at = Instant::now() + deadline;
        // Intent is durable before this final entry check. If it expired while
        // fsyncing, preserve uncertainty conservatively and enter no socket.
        let entry_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| LocalTransportError::Expired)?
            .as_secs();
        if entry_now >= bearer.expires_at {
            journal.mark_unknown(empty_failure(StreamError::ExpiredBearer))?;
            return Err(LocalTransportError::Expired);
        }
        let response = match runtime.block_on(async {
            let send = client
                .post(self.endpoint.url.clone())
                .header(AUTHORIZATION, auth_header)
                .header(ACCEPT, "text/event-stream")
                .header(CONTENT_TYPE, "application/json")
                .body(request.bytes.clone())
                .send();
            bounded(send, cancelled, deadline_at).await
        }) {
            Ok(Ok(v)) => v,
            Ok(Err(_)) => {
                journal.mark_unknown(empty_failure(StreamError::TransportFailure))?;
                return Err(LocalTransportError::EnteredFailure);
            }
            Err(e) => {
                journal.mark_unknown(empty_failure(e))?;
                return Err(LocalTransportError::EnteredFailure);
            }
        };
        observe_response(&runtime, response, raw_token, request, journal, cancelled, deadline_at)
    }

    /// Bounded fake-registration model discovery. The documented SIWC shape is
    /// a `models` array. `data` is not treated as an alias.
    pub fn discover_models(
        &self,
        bearer: &BearerAccess,
        selected: &RegistrationRef,
    ) -> Result<Vec<DiscoveredModel>, LocalTransportError> {
        if self.endpoint.url.path() != "/v1/models" {
            return Err(LocalTransportError::Endpoint);
        }
        if &bearer.registration != selected {
            return Err(LocalTransportError::Binding);
        }
        let runtime = RuntimeBuilder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| LocalTransportError::Client)?;
        let deadline = Duration::from_secs(5);
        let client = {
            let _entered = runtime.enter();
            self.endpoint.client(deadline, self.fake_root.clone())?
        };
        let auth = HeaderValue::from_str(&bearer.authorization_value())
            .map_err(|_| LocalTransportError::Binding)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| LocalTransportError::Expired)?
            .as_secs();
        if now >= bearer.expires_at {
            return Err(LocalTransportError::Expired);
        }
        let body = runtime
            .block_on(async {
                tokio::time::timeout(deadline, async {
                    let mut response = client
                        .get(self.endpoint.url.clone())
                        .header(AUTHORIZATION, auth)
                        .send()
                        .await
                        .map_err(|_| LocalTransportError::Models)?;
                    if response.status().as_u16() != 200
                        || header_size(response.headers()) > HTTP_HEADER_LIMIT
                    {
                        return Err(LocalTransportError::Models);
                    }
                    let mut body = Vec::new();
                    while let Some(chunk) = response
                        .chunk()
                        .await
                        .map_err(|_| LocalTransportError::Models)?
                    {
                        if body.len().saturating_add(chunk.len()) > MODEL_BODY_LIMIT {
                            return Err(LocalTransportError::Models);
                        }
                        body.extend_from_slice(&chunk);
                    }
                    Ok::<_, LocalTransportError>(body)
                })
                .await
            })
            .map_err(|_| LocalTransportError::Models)??;
        let json: Value = serde_json::from_slice(&body).map_err(|_| LocalTransportError::Models)?;
        let models = json
            .get("models")
            .and_then(Value::as_array)
            .ok_or(LocalTransportError::Models)?;
        if models.len() > 512 {
            return Err(LocalTransportError::Models);
        }
        models
            .iter()
            .map(|m| {
                let slug = m
                    .get("slug")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty() && s.len() <= 256)
                    .ok_or(LocalTransportError::Models)?;
                let visibility = m
                    .get("visibility")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty() && s.len() <= 128)
                    .ok_or(LocalTransportError::Models)?;
                Ok(DiscoveredModel {
                    slug: slug.to_owned(),
                    visibility: visibility.to_owned(),
                })
            })
            .collect()
    }

    /// A single offline refresh exchange for `AuthStore::refresh_with`. The
    /// caller supplies this trusted fake-loopback endpoint independently of
    /// the store's issuer configuration. A sent request is never retried.
    pub fn exchange_refresh(&self, grant: &RefreshGrant) -> Result<TokenResponse, RefreshFailure> {
        self.exchange_refresh_with_evidence(grant).0
    }

    pub fn exchange_refresh_with_evidence(
        &self,
        grant: &RefreshGrant,
    ) -> (
        Result<TokenResponse, RefreshFailure>,
        RefreshExchangeEvidence,
    ) {
        let mut evidence = RefreshExchangeEvidence {
            registration: grant.registration.clone(),
            endpoint_url: self.endpoint.url.as_str().to_owned(),
            pinned_loopback: self.endpoint.localhost_addr,
            fake_root_sha256: self.fake_root_sha256.clone(),
            http_status: None,
            outcome: RefreshExchangeOutcome::Ambiguous,
        };
        let result = self.exchange_refresh_inner(grant, &mut evidence);
        evidence.outcome = match &result {
            Ok(_) => RefreshExchangeOutcome::Success,
            Err(RefreshFailure::Temporary) => RefreshExchangeOutcome::Temporary,
            Err(RefreshFailure::Configuration) => RefreshExchangeOutcome::Configuration,
            Err(RefreshFailure::Terminal) => RefreshExchangeOutcome::Terminal,
            Err(RefreshFailure::Ambiguous) => RefreshExchangeOutcome::Ambiguous,
        };
        (result, evidence)
    }

    fn exchange_refresh_inner(
        &self,
        grant: &RefreshGrant,
        evidence: &mut RefreshExchangeEvidence,
    ) -> Result<TokenResponse, RefreshFailure> {
        if self.endpoint.url.path() != "/oauth/token" || self.endpoint.url.scheme() != "https" {
            return Err(RefreshFailure::Temporary);
        }
        let body = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "refresh_token")
            .append_pair("refresh_token", grant.refresh_token())
            .append_pair("client_id", &grant.client_id)
            .append_pair("resource", &grant.resource)
            .finish();
        if body.len() > REFRESH_REQUEST_LIMIT {
            return Err(RefreshFailure::Temporary);
        }
        let deadline = Duration::from_secs(5);
        let runtime = RuntimeBuilder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| RefreshFailure::Temporary)?;
        let client = {
            let _entered = runtime.enter();
            self.endpoint
                .client(deadline, self.fake_root.clone())
                .map_err(|_| RefreshFailure::Temporary)?
        };
        let deadline_at = Instant::now() + deadline;
        runtime.block_on(async {
            let mut response = match tokio::time::timeout_at(
                deadline_at,
                client
                    .post(self.endpoint.url.clone())
                    .header(ACCEPT, "application/json")
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(body)
                    .send(),
            )
            .await
            {
                Ok(Ok(response)) => response,
                Ok(Err(error)) if error.is_connect() => return Err(RefreshFailure::Temporary),
                _ => return Err(RefreshFailure::Ambiguous),
            };
            if header_size(response.headers()) > HTTP_HEADER_LIMIT {
                return Err(RefreshFailure::Ambiguous);
            }
            let status = response.status().as_u16();
            evidence.http_status = Some(status);
            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(';').next());
            if content_type != Some("application/json") {
                return Err(RefreshFailure::Ambiguous);
            }
            let mut data = Vec::new();
            loop {
                match tokio::time::timeout_at(deadline_at, response.chunk()).await {
                    Ok(Ok(Some(chunk))) => {
                        if data.len().saturating_add(chunk.len()) > REFRESH_BODY_LIMIT {
                            return Err(RefreshFailure::Ambiguous);
                        }
                        data.extend_from_slice(&chunk);
                    }
                    Ok(Ok(None)) => break,
                    _ => return Err(RefreshFailure::Ambiguous),
                }
            }
            if status == 200 {
                return TokenResponse::from_json(&data).map_err(|_| RefreshFailure::Ambiguous);
            }
            // RFC 6749 section 5.2 defines these token endpoint failures.
            // An intact specific rejection ends this grant; it is not a
            // recoverable no-rotation assertion about arbitrary server errors.
            if matches!(status, 400 | 401) {
                let code = serde_json::from_slice::<Value>(&data)
                    .ok()
                    .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_owned));
                match code.as_deref() {
                    Some("invalid_grant") => return Err(RefreshFailure::Terminal),
                    Some("invalid_client") => return Err(RefreshFailure::Configuration),
                    _ => (),
                }
            }
            Err(RefreshFailure::Ambiguous)
        })
    }
}
/// Observe one HTTP response to a request whose send intent is already durable:
/// headers, bounded stream, identity and terminal, recorded in the journal.
/// Shared by the loopback transport and the public HTTPS endpoint so both
/// classify a response identically.
pub(crate) fn observe_response(
    runtime: &tokio::runtime::Runtime,
    mut response: Response,
    raw_token: &str,
    request: &AssembledRequest,
    journal: &mut AttemptJournal,
    cancelled: &AtomicBool,
    deadline_at: Instant,
) -> Result<AttemptState, LocalTransportError> {
    let status = response.status().as_u16();
    let mut evidence = HttpEvidence {
        status: Some(status),
        request_id: None,
        content_type: None,
        content_type_meta: None,
        error_shape: None,
        error_code: None,
        error_param: None,
        body_bytes: 0,
        body_sha256: None,
        body_complete: false,
    };
    let header_ok = capture_headers(response.headers(), raw_token, &mut evidence);
    if !header_ok {
        journal.mark_http(evidence)?;
        journal.mark_unknown(empty_failure(StreamError::HttpFailure))?;
        return Err(LocalTransportError::EnteredFailure);
    }
    if status != 200 {
        runtime.block_on(read_error(
            &mut response,
            raw_token,
            cancelled,
            deadline_at,
            &mut evidence,
        ));
        journal.mark_http(evidence)?;
        journal.mark_unknown(empty_failure(StreamError::HttpFailure))?;
        return Err(LocalTransportError::EnteredFailure);
    }
    if evidence.content_type.as_deref() != Some("text/event-stream") {
        journal.mark_http(evidence)?;
        journal.mark_unknown(empty_failure(StreamError::HttpFailure))?;
        return Err(LocalTransportError::EnteredFailure);
    }
    let mut observer = StreamObserver::selected(request.bounds.clone())
        .map_err(|_| LocalTransportError::Binding)?;
    let mut digest = Sha256::new();
    let mut total = 0u64;
    let mut failure = None;
    let mut reached_eof = false;
    loop {
        match runtime.block_on(bounded(response.chunk(), cancelled, deadline_at)) {
            Ok(Ok(Some(bytes))) => {
                for chunk in bytes.chunks(CHUNK) {
                    total = total.saturating_add(chunk.len() as u64);
                    digest.update(chunk);
                    if let Err(e) = observer.push(chunk) {
                        failure = Some(e);
                        break;
                    }
                }
                if failure.is_some() {
                    break;
                }
            }
            Ok(Ok(None)) => {
                reached_eof = true;
                break;
            }
            Ok(Err(_)) => {
                failure = Some(StreamError::TransportFailure);
                break;
            }
            Err(e) => {
                failure = Some(e);
                break;
            }
        }
    }
    evidence.body_bytes = total;
    evidence.body_sha256 = Some(format!("{:x}", digest.finalize()));
    evidence.body_complete = reached_eof;
    journal.mark_http(evidence)?;
    let mut observation = match failure {
        Some(StreamError::Timeout) => observer.timeout_evidence(),
        Some(StreamError::Cancelled) => observer.failure_evidence(StreamError::Cancelled),
        Some(e) => observer.failure_evidence(e),
        None => observer.finish_evidence(),
    };
    scrub_stream_credential_echo(&mut observation, raw_token);
    if observation.local_failure.is_none()
        && observation.observed_model.as_deref() != Some(request.selected_model.as_str())
    {
        observation.local_failure = Some(StreamError::ConflictingIdentity);
        observation.output = None;
    }
    if observation.local_failure.is_some() {
        journal.mark_unknown(observation)?;
    } else {
        journal.mark_terminal(observation)?;
    }
    Ok(journal.record().state)
}

fn auth_header_size(bearer: &BearerAccess) -> usize {
    bearer.authorization_value().len() + 512
}
fn header_size(headers: &HeaderMap) -> usize {
    headers
        .iter()
        .map(|(n, v)| {
            n.as_str()
                .len()
                .saturating_add(v.as_bytes().len())
                .saturating_add(4)
        })
        .sum()
}
fn capture_headers(headers: &HeaderMap, token: &str, e: &mut HttpEvidence) -> bool {
    if header_size(headers) > HTTP_HEADER_LIMIT {
        return false;
    }
    if let Some(v) = headers.get("x-request-id").and_then(|v| v.to_str().ok()) {
        e.request_id = safe_field(v, token, 128);
    }
    if let Some(v) = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
        let essence = v
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if matches!(
            essence.as_str(),
            "text/event-stream" | "application/json" | "text/plain"
        ) {
            e.content_type = Some(essence);
        }
    }
    true
}
fn safe_field(s: &str, token: &str, max: usize) -> Option<String> {
    if s.is_empty()
        || s.len() > max
        || (!token.is_empty() && s.contains(token))
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/'))
    {
        None
    } else {
        Some(s.to_owned())
    }
}
pub(crate) async fn bounded<F: Future>(
    future: F,
    cancelled: &AtomicBool,
    deadline_at: Instant,
) -> Result<F::Output, StreamError> {
    tokio::select! {
        biased;
        _ = tokio::time::sleep_until(deadline_at) => Err(StreamError::Timeout),
        _ = cancellation(cancelled) => Err(StreamError::Cancelled),
        result = future => Ok(result),
    }
}
async fn cancellation(cancelled: &AtomicBool) {
    loop {
        if cancelled.load(Ordering::Acquire) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
async fn read_error(
    response: &mut Response,
    token: &str,
    cancelled: &AtomicBool,
    deadline_at: Instant,
    e: &mut HttpEvidence,
) {
    let mut data = Vec::new();
    let mut hash = Sha256::new();
    let mut reached_eof = false;
    loop {
        match bounded(response.chunk(), cancelled, deadline_at).await {
            Ok(Ok(Some(bytes))) => {
                let limit = HTTP_ERROR_LIMIT
                    .saturating_add(1)
                    .saturating_sub(data.len());
                let accepted = bytes.len().min(limit);
                hash.update(&bytes[..accepted]);
                data.extend_from_slice(&bytes[..accepted]);
                if accepted < bytes.len() || data.len() > HTTP_ERROR_LIMIT {
                    break;
                }
            }
            Ok(Ok(None)) => {
                reached_eof = true;
                break;
            }
            _ => break,
        }
    }
    e.body_bytes = data.len() as u64;
    e.body_sha256 = Some(format!("{:x}", hash.finalize()));
    e.body_complete = reached_eof;
    if data.len() > HTTP_ERROR_LIMIT {
        e.error_shape = Some(HttpErrorShape::TooLarge);
        return;
    }
    match serde_json::from_slice::<Value>(&data) {
        Ok(v) => {
            e.error_shape = Some(
                match (v.get("detail").is_some(), v.get("error").is_some()) {
                    (true, true) => HttpErrorShape::DetailAndStructured,
                    (true, false) => HttpErrorShape::Detail,
                    (false, true) => HttpErrorShape::Structured,
                    (false, false) => HttpErrorShape::OtherJson,
                },
            );
            if let Some(err) = v.get("error") {
                e.error_code = err
                    .get("code")
                    .and_then(Value::as_str)
                    .and_then(|v| safe_field(v, token, 64));
                e.error_param = err
                    .get("param")
                    .and_then(Value::as_str)
                    .and_then(|v| safe_field(v, token, 64));
            }
        }
        Err(_) => e.error_shape = Some(HttpErrorShape::NonJson),
    }
}
pub(crate) fn empty_failure(error: StreamError) -> StreamObservation {
    StreamObservation {
        response_id: None,
        observed_model: None,
        terminal: None,
        output: None,
        usage: Vec::new(),
        unique_events: 0,
        local_failure: Some(error),
    }
}

/// Provider-controlled stream strings cannot cross the durable journal boundary
/// if they echo the active bearer. Keep numeric usage and other safe evidence,
/// but an echoed identity or terminal detail cannot support completion.
fn scrub_stream_credential_echo(observation: &mut StreamObservation, token: &str) {
    if token.is_empty() {
        return;
    }
    let mut echoed = false;
    for field in [
        &mut observation.response_id,
        &mut observation.observed_model,
        &mut observation.output,
    ] {
        if field.as_ref().is_some_and(|value| value.contains(token)) {
            *field = None;
            echoed = true;
        }
    }
    match &mut observation.terminal {
        Some(crate::ObservedTerminal::Failed { code }) => {
            if code.as_ref().is_some_and(|value| value.contains(token)) {
                *code = None;
                echoed = true;
            }
        }
        Some(crate::ObservedTerminal::Incomplete { reason }) => {
            if reason.as_ref().is_some_and(|value| value.contains(token)) {
                *reason = None;
                echoed = true;
            }
        }
        _ => {}
    }
    if echoed {
        observation.output = None;
        observation.local_failure = Some(StreamError::CredentialEcho);
    }
}
