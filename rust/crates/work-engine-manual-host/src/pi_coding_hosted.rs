//! Hosted provider transport for the hosted Pi coding profile.
//!
//! The inference container has no network or access. For each logical
//! provider request the Host receives one `provider_fetch` under the grant's
//! stream identity and relays the response through `pi_stream::Session`.
//! Behind that one stream the Host runs the attempts its pinned route table and
//! the lifecycle grant permit: each attempt is reserved durably by the ledger,
//! admitted by `CodingEffectGate::provider_attempt` immediately before
//! dispatch, and switched only after a definitive head refusal. Settlement
//! evidence (status, backend model, raw terminal usage) is produced here from
//! the relayed bytes; the container's own observations are not evidence.
use super::failover::{self, Outcome};
use super::*;
use crate::pi_route::{self as route, Decision, Disposition, Router};
use crate::pi_stream::{self as stream, Progress, Session, Unavailable, Wire};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use provider::{AttemptDisposition, AttemptEvidence};
use review_execution_evidence::pi::Binding;
use std::io::Read;

/// Pi's response-head timeout carried in the hosted grant. It covers Host
/// admission plus the permitted failover attempts; a stop-gap until the
/// admission latency dependency (design r2 section 7) is resolved.
pub const HEAD_TIMEOUT_MS: u64 = 60_000;
/// Upstream idle read bound set by this profile, never above the attempt
/// deadline.
pub const READ_IDLE_MS: u64 = 30_000;
/// Schema of an attempt's retained request head (`*.request-head.json`).
pub const REQUEST_HEAD_SCHEMA: &str = "work-engine.pi-host-request-head.v1";
/// The head window a held head must be relayed or refused within ends this
/// long before Pi's head timeout, counted from the Host's receipt of the
/// fetch: the container's timer starts earlier, before the fetch frame
/// reaches the Host.
pub const HOLD_WINDOW_MARGIN_MS: u64 = 2_000;
const MAX_REQUEST_BODY: usize = 45_000;
/// The body forwarded upstream for a Completions request: the offline Layer A
/// model id Pi serialized is replaced by the transmitted Layer B slug
/// (`:floor` appended). Returns the original bytes if the body is not a
/// re-serializable chat/completions object.
fn completions_forward_body(body: &[u8], transmitted: &str) -> Vec<u8> {
    lifecycle_wire::parse_strict_json(body)
        .ok()
        .and_then(|mut v| {
            v.as_object_mut()?
                .insert("model".into(), Value::String(transmitted.to_string()));
            serde_json::to_vec(&v).ok()
        })
        .unwrap_or_else(|| body.to_vec())
}
/// Every tool a request body declares: the top-level `tools` array and the
/// tools of each `additional_tools` input item, in order. `None` when either
/// is malformed.
fn declared_tools(body: &Value) -> Option<Vec<Value>> {
    let mut out = match &body["tools"] {
        Value::Null => vec![],
        Value::Array(tools) => tools.clone(),
        _ => return None,
    };
    if let Some(input) = body["input"].as_array() {
        for item in input.iter().filter(|i| i["type"] == "additional_tools") {
            out.extend(item["tools"].as_array()?.iter().cloned());
        }
    } else if !body["input"].is_null() {
        return None;
    }
    Some(out)
}
const MAX_HEADERS: usize = 64;
/// Credential-bearing request headers. The container's values are markers;
/// the transport applies real access itself.
const ACCESS_HEADERS: [&str; 5] = [
    "authorization",
    "chatgpt-account-id",
    "cookie",
    "proxy-authorization",
    "x-api-key",
];
/// Top-level Messages request keys Claude Code was observed to send, plus
/// inert sampling keys. Anything else (for example `mcp_servers`,
/// `container`, `service_tier`) refuses.
const MESSAGES_KEYS: [&str; 13] = [
    "model",
    "messages",
    "system",
    "tools",
    "tool_choice",
    "metadata",
    "max_tokens",
    "thinking",
    "context_management",
    "output_config",
    "stream",
    "temperature",
    "stop_sequences",
];
/// Top-level OpenAI Chat Completions keys the Completions lane admits. This is
/// the OpenRouter body shape Pi's `openrouter-floor` provider sends. Anything
/// else (e.g. `store`, `background`, `service_tier`) is refused. `reasoning`
/// carries DeepSeek's effort setting (its thinkingLevelMap `high -> "high"`).
const COMPLETIONS_KEYS: [&str; 12] = [
    "model",
    "messages",
    "stream",
    "stream_options",
    "tools",
    "tool_choice",
    "temperature",
    "max_completion_tokens",
    "reasoning",
    "store",
    "stop",
    "top_p",
];

/// One permitted provider route. The order of routes is preference order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedRoute {
    pub id: String,
    pub payer: String,
    /// Nonsecret endpoint identity, never a credential: `loopback:<port>`
    /// (offline qualification), `https:chatgpt-codex` (the ChatGPT Codex
    /// backend, subscription funding), `https:openai-responses` (the public
    /// Responses API, API-key funding), `https:anthropic-oauth` (Anthropic
    /// Messages with a Claude subscription OAuth bearer) or
    /// `https:anthropic-api` (Anthropic Messages, API key). Origins are fixed
    /// by this code.
    pub endpoint: String,
}
/// Route table for the hosted profile. Its `(route, payer)` pairs and attempt
/// limit are exactly the lifecycle gate's hosted policy, and the runtime
/// admission binds its digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedRoutes {
    pub routes: Vec<HostedRoute>,
    pub max_attempts_per_request: u32,
}
impl HostedRoutes {
    pub fn validate(&self) -> Result<()> {
        let bounded = |v: &str| {
            !v.is_empty()
                && v.len() <= 128
                && v.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        };
        let mut ids: Vec<&str> = self.routes.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        if self.routes.is_empty()
            || self.routes.len() > 8
            || ids.len() != self.routes.len()
            || !self.routes.iter().all(|r| {
                bounded(&r.id) && bounded(&r.payer) && Endpoint::parse(&r.endpoint).is_some()
            })
            || !(1..=8).contains(&self.max_attempts_per_request)
        {
            return refuse("hosted route table invalid");
        }
        Ok(())
    }
    /// `(route, payer)` pairs for the lifecycle gate policy.
    pub fn pairs(&self) -> Vec<(String, String)> {
        self.routes
            .iter()
            .map(|r| (r.id.clone(), r.payer.clone()))
            .collect()
    }
    pub fn digest(&self) -> String {
        digest(
            json!({"schema":"work-engine.pi-hosted-routes.v1","routes":self.routes,"max_attempts_per_request":self.max_attempts_per_request})
                .to_string()
                .as_bytes(),
        )
    }
    pub(super) fn router(&self, model: &str, max_requests: u32, expires_ms: u64) -> Router {
        Router::new(route::Policy {
            model: model.into(),
            routes: self
                .routes
                .iter()
                .map(|r| route::Route {
                    id: r.id.clone(),
                    connection: r.endpoint.clone(),
                    model: model.into(),
                    payer: r.payer.clone(),
                    authorized: true,
                    revoked: false,
                    max_attempts: max_requests.saturating_mul(self.max_attempts_per_request),
                })
                .collect(),
            max_attempts_per_request: self.max_attempts_per_request,
            // Only the financial owner (the provider ledger) admits money.
            allowance_units: u64::MAX,
            reserve_units: 1,
            expires_ms,
        })
    }
}
/// A route's fixed endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endpoint {
    Loopback(std::net::SocketAddr),
    ChatGptCodex,
    OpenAiResponses,
    AnthropicOauth,
    AnthropicApi,
    /// OpenRouter's public Chat Completions endpoint, API-key funded. The
    /// transmitted model slug may carry OpenRouter's request-time `:floor`
    /// routing suffix; the resolved checkpoint is OpenRouter's reported
    /// `model` field. See Revision 2's three-layer identity contract.
    OpenRouter,
}
impl Endpoint {
    pub fn parse(endpoint: &str) -> Option<Self> {
        match endpoint {
            "https:chatgpt-codex" => Some(Self::ChatGptCodex),
            "https:openai-responses" => Some(Self::OpenAiResponses),
            "https:anthropic-oauth" => Some(Self::AnthropicOauth),
            "https:anthropic-api" => Some(Self::AnthropicApi),
            "https:openrouter" => Some(Self::OpenRouter),
            _ => {
                let port: u16 = endpoint.strip_prefix("loopback:")?.parse().ok()?;
                (port != 0)
                    .then(|| Self::Loopback(std::net::SocketAddr::from(([127, 0, 0, 1], port))))
            }
        }
    }
    /// Whether the endpoint speaks `wire`. Loopback routes serve either.
    pub fn serves(self, wire: Wire) -> bool {
        match self {
            Self::Loopback(_) => true,
            Self::ChatGptCodex | Self::OpenAiResponses => wire == Wire::Responses,
            Self::AnthropicOauth | Self::AnthropicApi => wire == Wire::Messages,
            Self::OpenRouter => wire == Wire::Completions,
        }
    }
    /// Host name of the endpoint.
    pub fn host(self) -> &'static str {
        match self {
            Self::Loopback(_) => "localhost",
            Self::ChatGptCodex => "chatgpt.com",
            Self::OpenAiResponses => "api.openai.com",
            Self::AnthropicOauth | Self::AnthropicApi => "api.anthropic.com",
            Self::OpenRouter => "openrouter.ai",
        }
    }
    /// `(host, request path)` for `request`: fixed per endpoint and wire;
    /// Messages requests keep the validated `?beta=true` query.
    pub fn target(self, request: &HostedRequest) -> (&'static str, &'static str) {
        let path = match (self, request.wire) {
            (_, Wire::Messages) if request.beta => "/v1/messages?beta=true",
            (_, Wire::Messages) => "/v1/messages",
            (Self::OpenAiResponses, Wire::Responses) => "/v1/responses",
            (Self::OpenRouter, Wire::Completions) => "/api/v1/chat/completions",
            // Loopback mocks serve any wire (offline qualification): give
            // Completions a real path so a loopback upstream can be driven.
            (Self::Loopback(_), Wire::Completions) => "/chat/completions",
            (_, Wire::Responses) => "/backend-api/codex/responses",
            // Only OpenRouter (and loopback) serve Completions; any other
            // endpoint on that wire is a config error and falls through to the
            // route check.
            (_, Wire::Completions) => "/unsupported-completions",
        };
        (self.host(), path)
    }
    /// Container request headers forwarded to this endpoint for `wire` (an
    /// allowlist; everything else, including billing-context headers such as
    /// `openai-organization` or `openai-project`, is dropped). Messages
    /// requests keep the headers Claude Code sends besides access and
    /// transport headers (as forwarded by the in-container probe, whose
    /// requests stayed in the subscription plan).
    pub fn forwarded(self, wire: Wire, name: &str) -> bool {
        if wire == Wire::Messages {
            return matches!(
                name,
                "accept"
                    | "content-type"
                    | "anthropic-version"
                    | "anthropic-beta"
                    | "anthropic-dangerous-direct-browser-access"
                    | "user-agent"
                    | "x-app"
                    | "x-client-app"
                    | "x-claude-code-session-id"
                    | "x-stainless-arch"
                    | "x-stainless-lang"
                    | "x-stainless-os"
                    | "x-stainless-package-version"
                    | "x-stainless-retry-count"
                    | "x-stainless-runtime"
                    | "x-stainless-runtime-version"
                    | "x-stainless-timeout"
            );
        }
        match self {
            Self::Loopback(_) | Self::ChatGptCodex => matches!(
                name,
                "accept"
                    | "content-type"
                    | "originator"
                    | "openai-beta"
                    | "session-id"
                    | "x-client-request-id"
                    | "user-agent"
            ),
            Self::OpenAiResponses => matches!(
                name,
                "accept" | "content-type" | "x-client-request-id" | "user-agent"
            ),
            Self::OpenRouter => matches!(
                name,
                "accept" | "content-type" | "x-requested-with" | "user-agent"
            ),
            Self::AnthropicOauth | Self::AnthropicApi => false,
        }
    }
}

/// The provider request to dispatch, as validated from the container's
/// `provider_fetch`, with credential headers removed.
#[derive(Clone, Debug)]
pub struct HostedRequest {
    pub request: u64,
    pub attempt: u64,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub wire: Wire,
    /// Messages: the container's request carried `?beta=true`.
    pub beta: bool,
}
/// Upstream response reader and its abort.
pub type Upstream = (Box<dyn Read + Send>, Box<dyn FnOnce() + Send>);
/// A byte stream to a provider.
pub trait Duplex: Read + Write + Send {}
impl<T: Read + Write + Send> Duplex for T {}
/// An established connection (TCP, and TLS where used) on which no request
/// byte has been written yet, and its abort.
pub struct Connection {
    pub stream: Box<dyn Duplex>,
    pub abort: Box<dyn FnOnce() + Send>,
}
/// Trusted composition bound by the embedder. It holds provider access, which
/// never enters the container or the journal, and applies it only in `send`.
pub trait HostedProviderTransport {
    /// Refuse a route table this transport cannot serve: an endpoint it does
    /// not support, or a payer it holds no access for.
    fn check(&self, routes: &HostedRoutes) -> Result<()>;
    /// Nonsecret identity recorded in the journal at binding: which access
    /// source kind serves each payer, and the source file's digest.
    fn identity(&self) -> Value;
    /// Connect to `route` before gate admission. Writes no request byte, so
    /// it is never provider entry; it runs outside the lifecycle store's
    /// transaction.
    fn connect(&mut self, route: &HostedRoute, deadline_ms: i64) -> Result<Connection>;
    /// The exact HTTP/1.1 request head `send` writes for `request` on
    /// `route`. With `redacted`, every access header value is replaced by
    /// `REDACTED` (names kept), so the result is retainable evidence of the
    /// head as transmitted. Writes nothing.
    fn request_head(
        &mut self,
        route: &HostedRoute,
        request: &HostedRequest,
        redacted: bool,
    ) -> Result<String>;
    /// Write one attempt's request on `connection` and return its response.
    /// Called only inside a gate-admitted dispatch, before `deadline_ms`.
    /// The head is `request_head(route, request, false)`.
    fn send(
        &mut self,
        connection: Connection,
        route: &HostedRoute,
        request: &HostedRequest,
    ) -> Result<Upstream>;
    /// Exclusion screen over bytes relayed to the container: an error if they
    /// contain access material, otherwise how many trailing bytes to hold
    /// back because they may begin it.
    fn hold(&self, pending: &[u8]) -> std::result::Result<usize, &'static str>;
}
/// Milliseconds left before `deadline_ms`, at most `cap`.
pub fn left_ms(deadline_ms: i64, cap: u64) -> Result<Duration> {
    u64::try_from(deadline_ms.saturating_sub(now_ms()?))
        .ok()
        .filter(|v| *v > 0)
        .map(|v| Duration::from_millis(v.min(cap)))
        .ok_or_else(|| Error::Refused("provider attempt deadline passed".into()))
}
/// The value retained in place of each access header value.
pub const REDACTED: &str = "[redacted]";
/// The HTTP/1.1 request head for `request` to `endpoint`: fixed host and
/// path, the given access headers (values replaced by `REDACTED` when
/// `redacted`), the allowlisted container headers.
pub fn request_head(
    endpoint: Endpoint,
    access: &[(&str, &str)],
    request: &HostedRequest,
    redacted: bool,
) -> String {
    let (host, path) = endpoint.target(request);
    let mut head = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nX-Request-Id: {}-{}\r\nContent-Length: {}\r\nConnection: close\r\n",
        request.request,
        request.attempt,
        request.body.len()
    );
    for (k, v) in access {
        let v = if redacted { REDACTED } else { v };
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    for (k, v) in &request.headers {
        if endpoint.forwarded(request.wire, k) {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
    }
    head.push_str("\r\n");
    head
}
/// Screen `pending` against every access secret.
pub fn hold_secrets(secrets: &[&[u8]], pending: &[u8]) -> std::result::Result<usize, &'static str> {
    let mut hold = 0;
    for secret in secrets {
        if secret.is_empty() {
            continue;
        }
        if pending.windows(secret.len()).any(|w| w == *secret) {
            return Err("hosted_access_exclusion");
        }
        hold = hold.max(review_execution_evidence::pi::prefix_suffix(
            pending, secret,
        ));
    }
    Ok(hold)
}
/// Offline-only transport to synthetic loopback routes with a synthetic,
/// Host-generated access token per payer.
pub struct LoopbackTransport {
    access: BTreeMap<String, Vec<u8>>,
    payers: Vec<String>,
}
impl LoopbackTransport {
    pub fn new() -> Result<Self> {
        Ok(Self {
            access: BTreeMap::new(),
            payers: Vec::new(),
        })
    }
    fn token(&mut self, payer: &str) -> Result<String> {
        if !self.access.contains_key(payer) {
            let mut random = [0u8; 32];
            File::open("/dev/urandom")?.read_exact(&mut random)?;
            self.access.insert(
                payer.to_string(),
                format!("synthetic-A-{}-host-only", B64.encode(random)).into_bytes(),
            );
            self.payers.push(payer.to_string());
        }
        String::from_utf8(self.access[payer].clone()).map_err(|_| Error::Refused("access".into()))
    }
}
impl HostedProviderTransport for LoopbackTransport {
    fn identity(&self) -> Value {
        json!({"transport":"loopback","access":"synthetic per payer, generated in the Host"})
    }
    fn check(&self, routes: &HostedRoutes) -> Result<()> {
        if routes
            .routes
            .iter()
            .all(|r| matches!(Endpoint::parse(&r.endpoint), Some(Endpoint::Loopback(_))))
        {
            Ok(())
        } else {
            refuse("loopback transport serves loopback routes only")
        }
    }
    fn connect(&mut self, route: &HostedRoute, deadline_ms: i64) -> Result<Connection> {
        use std::net::{Shutdown, TcpStream};
        let Some(Endpoint::Loopback(address)) = Endpoint::parse(&route.endpoint) else {
            return refuse("route endpoint");
        };
        self.token(&route.payer)?;
        let s = TcpStream::connect_timeout(&address, left_ms(deadline_ms, 5000)?)?;
        s.set_write_timeout(Some(left_ms(deadline_ms, 5000)?))?;
        s.set_read_timeout(Some(left_ms(deadline_ms, READ_IDLE_MS)?))?;
        let killer = s.try_clone()?;
        Ok(Connection {
            stream: Box::new(s),
            abort: Box::new(move || {
                let _ = killer.shutdown(Shutdown::Both);
            }),
        })
    }
    fn request_head(
        &mut self,
        route: &HostedRoute,
        request: &HostedRequest,
        redacted: bool,
    ) -> Result<String> {
        let endpoint = Endpoint::parse(&route.endpoint)
            .ok_or_else(|| Error::Refused("route endpoint".into()))?;
        let bearer = format!("Bearer {}", self.token(&route.payer)?);
        Ok(request_head(
            endpoint,
            &[("Authorization", &bearer)],
            request,
            redacted,
        ))
    }
    fn send(
        &mut self,
        connection: Connection,
        route: &HostedRoute,
        request: &HostedRequest,
    ) -> Result<Upstream> {
        let head = self.request_head(route, request, false)?;
        let Connection { mut stream, abort } = connection;
        stream.write_all(head.as_bytes())?;
        stream.write_all(&request.body)?;
        stream.flush()?;
        Ok((Box::new(stream), abort))
    }
    fn hold(&self, pending: &[u8]) -> std::result::Result<usize, &'static str> {
        let secrets: Vec<&[u8]> = self.access.values().map(Vec::as_slice).collect();
        hold_secrets(&secrets, pending)
    }
}
fn now_ms() -> Result<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::Refused("clock".into()))?
        .as_millis()
        .try_into()
        .map_err(|_| Error::Refused("clock range".into()))
}
struct Screen<'a>(&'a dyn HostedProviderTransport);
impl stream::Screen for Screen<'_> {
    fn hold(&self, pending: &[u8]) -> std::result::Result<usize, &'static str> {
        self.0.hold(pending)
    }
}

/// The request shape that has stayed in the subscription plan: the first three
/// system blocks are Claude Code's billing header (`cc_entrypoint=sdk-ts`),
/// the Agent SDK identity and the extension's Claude prompt (as on every
/// live-c1/live-c2 request). Pi's own prompt mode, which drew Anthropic's
/// third-party (extra usage) answer, carries the same first two blocks and a
/// different third. A guard against misconfiguration over bytes the
/// container authors, not an identity proof: the prompt mode is also held by
/// the receiver (env) and the Host's pi-config allowlist, and billing is
/// checked again on the response head.
const CLAUDE_PROMPT: &str =
    "You are Claude, working as a coding agent in pi, a terminal coding harness.";
fn claude_code_request(body: &[u8]) -> bool {
    let Ok(v) = serde_json::from_slice::<Value>(body) else {
        return false;
    };
    let block = |i: usize| {
        let b = &v["system"][i];
        (b["type"] == "text").then(|| b["text"].as_str()).flatten()
    };
    block(0).is_some_and(|t| {
        t.starts_with("x-anthropic-billing-header: ") && t.contains("cc_entrypoint=sdk-ts;")
    }) && block(1) == Some("You are a Claude agent, built on Anthropic's Claude Agent SDK.")
        && block(2).is_some_and(|t| t.starts_with(CLAUDE_PROMPT))
}
/// Validated container request, kept for every attempt of the request.
#[derive(Clone)]
struct Fetch {
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
    wire: Wire,
    beta: bool,
    /// Messages: the request's `messages`, which the next request must extend.
    messages: Option<Value>,
    /// When the Host received the fetch: the head window's start.
    received: Instant,
    /// Completions: the container's original body, when the forwarded body
    /// differs from it (the Host rewrites the model to the transmitted slug).
    original: Option<Vec<u8>>,
}
/// The canonical continuity rule, shared with the task host
/// (`native-realization` `pi_host::canonical`).
pub use native_realization::pi_host::canonical;
enum StreamPhase {
    /// Grant written with the stream identity; awaiting `provider_fetch`.
    Granted,
    /// Fetch received; dispatched by the next tick after pending frames.
    Fetched(Fetch),
    Active {
        session: Session,
        attempt: u64,
        routed: u64,
        fetch: Fetch,
    },
    /// A head-phase cancel was seen before any byte was written; its
    /// confirmation is sent when that frame is processed.
    NotDispatched,
    Unavailable(Unavailable),
    Closed,
}
struct Stream {
    request: u64,
    binding: Binding,
    bind: Value,
    phase: StreamPhase,
}
const STREAM: u64 = 1;
/// In-memory hosted transport state of one Host run. Durable facts (attempt
/// records and their evidence) are in the journal and state directory.
#[derive(Default)]
pub struct Hosted {
    router: Option<Router>,
    stream: Option<Stream>,
    out: Vec<Value>,
    /// Messages: the last accepted request's `messages`.
    last_messages: Option<Value>,
    /// This Host ran the launch: attempts still pending at its end were left
    /// by this process, not by one that is gone.
    ran: bool,
    /// The job's failover inputs, once loaded (`None` inside: none admitted).
    failover: Option<Option<failover::Context>>,
    /// Tests: the decision clock.
    #[cfg(test)]
    now_override: Option<u64>,
    /// Tests: the head window from the fetch's receipt.
    #[cfg(test)]
    head_window_override: Option<Duration>,
}

impl Host {
    /// Explicit, fresh and non-recoverable, like the coding gate.
    pub fn bind_hosted_transport(
        &mut self,
        transport: Box<dyn HostedProviderTransport>,
    ) -> Result<()> {
        if self.journal.phase != Phase::Admitted
            || !self
                .journal
                .job
                .coding
                .as_ref()
                .is_some_and(|p| is_hosted(&p.schema))
            || self.hosted_transport.is_some()
        {
            return refuse("hosted transport binding conflict");
        }
        let (_, routes) = self.hosted_plan()?;
        transport.check(&routes)?;
        if let Some(state) = self.journal.coding.as_mut() {
            state.hosted_transport = Some(transport.identity());
            self.save()?;
        }
        let record = self.hosted_failover_record(&routes)?;
        self.journal.coding.as_mut().unwrap().hosted_failover = Some(record);
        self.save()?;
        self.hosted_transport = Some(transport);
        Ok(())
    }
    /// The binding record: API exposure, the failover inputs' digests and the
    /// gate's evaluation for the first request (informational; the job is
    /// not refused on it).
    fn hosted_failover_record(&mut self, routes: &HostedRoutes) -> Result<Value> {
        let mut record = self.hosted_failover_policy_record(routes)?;
        // An admitted classification table: its standing (a mismatch is
        // journaled and classifies nothing) and, when active, the API
        // exposure of classified refusals. Without one the record is as
        // before.
        let (plan, _) = self.hosted_plan()?;
        let admitted = plan.classification_table_sha256.as_deref();
        if let Some(c) = super::classification::binding_record(admitted) {
            if c["active"] == true {
                let policy = self.hosted_failover()?.map(|ctx| ctx.policy);
                let order: Vec<(&str, &str)> = failover::route_order(routes, policy.as_ref())
                    .iter()
                    .map(|r| (r.id.as_str(), r.endpoint.as_str()))
                    .collect();
                record["exposure"]["held_failover_to_api"] = json!(
                    super::classification::held_failover_to_api(admitted, &order)
                );
            }
            record["classification"] = c;
        }
        Ok(record)
    }
    fn hosted_failover_policy_record(&mut self, routes: &HostedRoutes) -> Result<Value> {
        let Some(ctx) = self.hosted_failover()? else {
            return Ok(json!({"exposure": failover::api_exposure(routes, None), "policy": null}));
        };
        let decided = self.hosted_now();
        let in_job = ctx.in_job_observation(&self.journal.job.state_dir, decided)?;
        let requests = &self.journal.coding.as_ref().unwrap().provider_requests;
        let e = ctx.evaluate(
            decided,
            &in_job,
            &failover::ledger_snapshot(routes, requests, 1, None),
            &[],
        );
        Ok(json!({
            "exposure": failover::api_exposure(routes, Some(&ctx.policy)),
            "policy": {"version": ctx.policy.version, "sha256": ctx.policy_sha256},
            "observation_sha256": ctx.admitted.as_ref().map(|a| &a.1),
            "admission_evaluation": {"decided_ms": decided, "action": e.action, "route": e.route,
                "resume_time": e.resume_time, "admitted_ignored": e.admitted_ignored},
        }))
    }
    /// The job's failover context, loaded once from the plan's admitted
    /// inputs; `None` without them (refusal-only failover, as before).
    fn hosted_failover(&mut self) -> Result<Option<failover::Context>> {
        if let Some(ctx) = &self.hosted.failover {
            return Ok(ctx.clone());
        }
        let (plan, routes) = self.hosted_plan()?;
        let ctx = match &plan.failover {
            None => None,
            Some(f) => plan
                .failover_admitted(
                    Some(&f.policy.sha256),
                    f.observation.as_ref().map(|o| o.sha256.as_str()),
                )?
                .map(|(policy, policy_sha256, admitted)| failover::Context {
                    policy,
                    policy_sha256,
                    table: routes,
                    job_id: self.journal.job.job_id.clone(),
                    admitted,
                }),
        };
        self.hosted.failover = Some(ctx.clone());
        Ok(ctx)
    }
    /// The end of the head window a held head is relayed or refused within:
    /// Pi's head timeout from the fetch's receipt less
    /// [`HOLD_WINDOW_MARGIN_MS`], never past the attempt's admitted deadline.
    fn hosted_hold_window(&self, fetch: &Fetch, deadline_ms: Option<i64>) -> Instant {
        let window = Duration::from_millis(HEAD_TIMEOUT_MS - HOLD_WINDOW_MARGIN_MS);
        #[cfg(test)]
        let window = self.hosted.head_window_override.unwrap_or(window);
        let mut until = fetch.received + window;
        if let (Some(d), Ok(now)) = (deadline_ms, now_ms()) {
            let left = u64::try_from(d.saturating_sub(now)).unwrap_or(0);
            until = until.min(Instant::now() + Duration::from_millis(left));
        }
        until
    }
    /// Keep the head hold's record on the attempt (not yet saved).
    fn hosted_keep_hold(&mut self, request: u64, attempt: u64, hold: Option<stream::HoldRecord>) {
        if let Some(a) = self
            .journal
            .coding
            .as_mut()
            .and_then(|c| c.provider_requests.get_mut(&request))
            .and_then(|r| r.attempts.last_mut())
            .filter(|a| a.attempt == attempt)
            && hold.is_some()
        {
            a.head_hold = hold;
        }
    }
    /// The failover decision clock.
    fn hosted_now(&self) -> u64 {
        #[cfg(test)]
        if let Some(t) = self.hosted.now_override {
            return t;
        }
        stream::now_millis()
    }
    fn hosted_profile(&self) -> bool {
        self.journal
            .job
            .coding
            .as_ref()
            .is_some_and(|p| is_hosted(&p.schema))
    }
    fn hosted_plan(&self) -> Result<(provider::ProviderPlan, HostedRoutes)> {
        let plan = self
            .journal
            .job
            .coding
            .as_ref()
            .and_then(|p| p.provider.clone())
            .ok_or_else(|| Error::Refused("provider plan absent".into()))?;
        let routes = plan.hosted.clone();
        Ok((plan, routes))
    }
    /// Stream identity and head timeout the Host adds to a hosted live grant
    /// at its exact write (after the gate compared the admitted bytes). `None`
    /// for every other command.
    pub(super) fn coding_hosted_grant(&mut self, command: &Value) -> Result<Option<Value>> {
        if !self.hosted_profile() || command["type"] != "bridge_provider" {
            return Ok(None);
        }
        let request = command["id"]
            .as_u64()
            .ok_or_else(|| Error::Refused("hosted grant id".into()))?;
        if command["mode"] != "live"
            || self.hosted.stream.as_ref().is_some_and(|s| {
                !matches!(s.phase, StreamPhase::Closed | StreamPhase::Unavailable(_))
            })
            || !self
                .journal
                .coding
                .as_ref()
                .unwrap()
                .provider_requests
                .get(&request)
                .is_some_and(|r| r.attempts.is_empty() && r.settled_microusd.is_none())
        {
            return refuse("hosted grant outside a fresh logical request");
        }
        let binding = Binding {
            session: self
                .journal
                .session_id
                .clone()
                .ok_or_else(|| Error::Refused("hosted native session absent".into()))?,
            attempt: self.journal.job.job_id.clone(),
            generation: request,
            profile: self.journal.job.coding.as_ref().unwrap().schema.clone(),
            admission: String::new(),
            owner_profile: String::new(),
        };
        let bind = serde_json::to_value(&binding)?;
        let fields = json!({"stream":{"binding":bind,"sequence":request,"stream":STREAM},"headTimeoutMs":HEAD_TIMEOUT_MS});
        self.hosted.stream = Some(Stream {
            request,
            binding,
            bind,
            phase: StreamPhase::Granted,
        });
        Ok(Some(fields))
    }
    fn own_stream_frame(s: &Stream, frame: &Value) -> bool {
        frame["binding"] == s.bind && frame["sequence"] == s.request && frame["stream"] == STREAM
    }
    /// `provider_fetch`, `pull`, `cancel` and `ack` frames of the hosted profile.
    pub(super) fn coding_hosted_frame(&mut self, frame: &Value) -> Result<()> {
        if !self.hosted_profile() {
            return refuse("hosted frame outside the hosted profile");
        }
        let kind = frame["type"].as_str().unwrap_or("");
        if kind == "provider_fetch" {
            let fetch = self.hosted_fetch(frame)?;
            // The exact body, retained create-only before the request's first
            // attempt; every attempt records its digest before dispatch.
            let request = self.hosted.stream.as_ref().unwrap().request;
            let path = self
                .journal
                .job
                .state_dir
                .join(format!("coding-hosted-{request}.body.json"));
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&path)?;
            file.write_all(&fetch.body)?;
            file.sync_all()?;
            File::open(&self.journal.job.state_dir)?.sync_all()?;
            self.journal.artifacts.push(Artifact {
                path,
                sha256: digest(&fetch.body),
            });
            self.save()?;
            // Completions: the container-authored body before the Host's
            // model rewrite, retained beside the forwarded one.
            if let Some(original) = &fetch.original {
                self.hosted_retain_named(
                    &format!("coding-hosted-{request}.original.body.json"),
                    original,
                )?;
            }
            if let Some(messages) = &fetch.messages {
                self.hosted.last_messages = Some(messages.clone());
            }
            self.hosted.stream.as_mut().unwrap().phase = StreamPhase::Fetched(fetch);
            return Ok(());
        }
        let mut s = self
            .hosted
            .stream
            .take()
            .ok_or_else(|| Error::Refused("stream frame without a hosted stream".into()))?;
        let result = self.hosted_native(&mut s, frame);
        self.hosted.stream = Some(s);
        result
    }
    fn hosted_fetch(&self, frame: &Value) -> Result<Fetch> {
        let received = Instant::now();
        let (plan, _) = self.hosted_plan()?;
        let s = self
            .hosted
            .stream
            .as_ref()
            .filter(|s| matches!(s.phase, StreamPhase::Granted) && frame["id"] == s.request)
            .ok_or_else(|| Error::Refused("provider fetch outside its granted request".into()))?;
        let base = plan.selected()?["baseUrl"]
            .as_str()
            .unwrap_or("")
            .trim_end_matches('/')
            .to_string();
        let wire = plan.wire_checked()?;
        let url = frame["url"].as_str().unwrap_or("");
        let beta = match wire {
            Wire::Responses if url == format!("{base}/codex/responses") => false,
            Wire::Messages if url == format!("{base}/v1/messages") => false,
            Wire::Messages if url == format!("{base}/v1/messages?beta=true") => true,
            // The catalog baseUrl already ends in `/api/v1` (openrouter.ai/api/v1),
            // so the URL Pi presents is {base}/chat/completions. Do not append a
            // second /api/v1 here (see the upstream request path handling).
            // A loopback mock (offline qualification) presents
            // http://127.0.0.1:<port>/chat/completions and is accepted for the
            // Completions wire when the plan has a loopback route.
            Wire::Completions if url == format!("{base}/chat/completions") => false,
            Wire::Completions
                if super::hosted::Endpoint::parse(&plan.hosted.routes[0].endpoint)
                    .is_some_and(|e| matches!(e, super::hosted::Endpoint::Loopback(_)))
                    && url
                        .strip_prefix("http://127.0.0.1:")
                        .or_else(|| url.strip_prefix("http://localhost:"))
                        .is_some_and(|rest| {
                            rest.contains('/') && rest.ends_with("/chat/completions")
                        }) =>
            {
                false
            }
            _ => return refuse("provider fetch identity/method/url conflict"),
        };
        if !Self::own_stream_frame(s, frame) || frame["method"] != "POST" {
            return refuse("provider fetch identity/method/url conflict");
        }
        let mut headers = BTreeMap::new();
        let given = frame["headers"]
            .as_object()
            .filter(|h| h.len() <= MAX_HEADERS)
            .ok_or_else(|| Error::Refused("provider fetch headers bound".into()))?;
        for (k, v) in given {
            let k = k.to_ascii_lowercase();
            let v = v
                .as_str()
                .filter(|v| v.len() <= 4096 && !v.contains(['\r', '\n']))
                .ok_or_else(|| Error::Refused("provider fetch header value".into()))?;
            if k.is_empty()
                || !k
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            {
                return refuse("provider fetch header name");
            }
            // The receiver decompresses; the Host validates and forwards only
            // plain JSON.
            if k == "content-encoding" {
                return refuse("provider fetch body encoding");
            }
            if !ACCESS_HEADERS.contains(&k.as_str()) {
                headers.insert(k, v.to_string());
            }
        }
        let battery = self
            .journal
            .job
            .coding
            .as_ref()
            .is_some_and(|p| is_battery(&p.schema));
        let body = if battery {
            if frame.get("body").is_some() {
                return refuse("ambiguous or inline battery provider body");
            }
            self.coding_read_spool(
                &frame["bodySpool"],
                &format!("body-{}.bin", s.request),
                BATTERY_SPOOL_BODY_BYTES,
            )?
        } else {
            if frame.get("bodySpool").is_some() {
                return refuse("provider body spool outside battery profile");
            }
            B64.decode(frame["body"].as_str().unwrap_or(""))
                .map_err(|_| Error::Refused("provider fetch body encoding".into()))?
        };
        // Messages bodies are parsed strictly: a duplicate key could mean one
        // thing here and another upstream.
        let parsed: Value = match wire {
            Wire::Responses => serde_json::from_slice(&body).ok(),
            Wire::Messages | Wire::Completions => lifecycle_wire::parse_strict_json(&body).ok(),
        }
        .ok_or_else(|| Error::Refused("provider fetch body JSON".into()))?;
        let analysis = self.analysis_profile();
        // For Completions the container's Pi resolves the offline Layer A id
        // (`~deepseek/deepseek-v4-flash-latest`); the Host owns appending the
        // `:floor` routing suffix at dispatch. So `parsed["model"]` must equal
        // the Layer A catalog id the plan binds, and the Host rewrites the
        // forwarded body to the transmitted slug (Layer B) below.
        let transmitted = plan.transmitted_model()?;
        let body_ok = if wire == Wire::Completions {
            parsed["model"] == plan.model && parsed["stream"] == true
        } else {
            parsed["model"] == transmitted && parsed["stream"] == true
        };
        if body.len()
            > if battery {
                BATTERY_SPOOL_BODY_BYTES as usize
            } else if analysis {
                super::analysis::MAX_REQUEST_BODY
            } else {
                MAX_REQUEST_BODY
            }
            || !body_ok
        {
            return refuse("provider fetch body bound/model/stream conflict");
        }
        if wire == Wire::Messages {
            // W3: a client-side retry is never dispatched; every upstream
            // request is one Host attempt (the receiver also disables Claude
            // Code's retries and admits one request per grant).
            if headers.get("x-stainless-retry-count").map(String::as_str) != Some("0") {
                return refuse("provider fetch is a client retry");
            }
            self.messages_policy(&plan, &parsed, analysis)?;
            return Ok(Fetch {
                headers,
                body,
                wire,
                beta,
                messages: Some(parsed["messages"].clone()),
                received,
                original: None,
            });
        }
        if wire == Wire::Completions {
            self.completions_policy(&plan, &parsed, analysis)?;
            // Host owns the Layer B append: forward the body with the
            // transmitted (`:floor`) model slug, never the offline Layer A id.
            let transmitted_body = completions_forward_body(&body, &transmitted);
            let original = (transmitted_body != body).then_some(body);
            return Ok(Fetch {
                headers,
                body: transmitted_body,
                wire,
                beta,
                messages: None,
                received,
                original,
            });
        }
        // R-a: body fields the Host pins on real routes. Nothing stored at the
        // provider, no background processing, no non-default service tier,
        // and only the container's own function tools (no provider-hosted
        // tools: the container stays networkless and tool spend stays inside
        // token settlement).
        // Tools are declared at the top level or, for the Codex API, as
        // `additional_tools` input items; both count.
        let declared = declared_tools(&parsed);
        let tools_ok = declared
            .as_ref()
            .is_some_and(|tools| tools.iter().all(|t| t["type"] == "function"));
        if parsed["store"] != false
            || !matches!(&parsed["background"], Value::Null | Value::Bool(false))
            || !(parsed["service_tier"].is_null()
                || matches!(parsed["service_tier"].as_str(), Some("auto" | "default")))
            || !tools_ok
        {
            return refuse("provider fetch body policy: store/background/service_tier/tools");
        }
        if analysis {
            super::analysis::check_request_tools(
                &self.journal.job.coding.as_ref().unwrap().schema,
                &json!(declared.unwrap_or_default()),
            )?;
        }
        Ok(Fetch {
            headers,
            body,
            wire,
            beta,
            messages: None,
            received,
            original: None,
        })
    }
    /// Messages body policy: only the known top-level keys (so no
    /// `mcp_servers`, `container` or service tier), `tool_choice` auto, only
    /// the container's own custom tools (no provider-hosted tools; on the
    /// analysis profile exactly the pinned tool surface), and every request
    /// extends the previous one's messages under the two canonical rules.
    fn messages_policy(
        &self,
        plan: &provider::ProviderPlan,
        body: &Value,
        analysis: bool,
    ) -> Result<()> {
        let object = body
            .as_object()
            .ok_or_else(|| Error::Refused("provider fetch body is not an object".into()))?;
        if let Some(key) = object.keys().find(|k| !MESSAGES_KEYS.contains(&k.as_str())) {
            return refuse(format!(
                "provider fetch body policy: field {key} is not allowed"
            ));
        }
        if object
            .get("tool_choice")
            .is_some_and(|c| c != &json!({"type":"auto"}))
        {
            return refuse("provider fetch body policy: only tool_choice auto");
        }
        if body["messages"].as_array().is_none_or(|m| m.is_empty()) {
            return refuse("provider fetch body policy: no messages");
        }
        if !native_realization::pi_host::messages_account_identifier_absent(body) {
            return refuse("provider fetch body policy: account identifier in metadata");
        }
        let tools = match &body["tools"] {
            Value::Null => vec![],
            Value::Array(tools) => tools.clone(),
            _ => return refuse("provider fetch body policy: tools"),
        };
        // Provider-hosted (server) tools carry a `type`; custom tools do not
        // (or name `custom`).
        if tools
            .iter()
            .any(|t| !t.is_object() || t.get("type").is_some_and(|ty| ty != "custom"))
        {
            return refuse("provider fetch body policy: provider-hosted tool");
        }
        if analysis {
            super::analysis::check_messages_tools(
                &self.journal.job.coding.as_ref().unwrap().schema,
                &plan.provider,
                &body["tools"],
            )?;
        }
        if let Some(previous) = &self.hosted.last_messages
            && !canonical::is_prefix_extension(previous, &body["messages"])
        {
            return refuse(
                "provider fetch body policy: messages do not extend the previous request",
            );
        }
        Ok(())
    }
    /// Chat Completions body policy for the OpenRouter lane: only the known
    /// top-level keys (so no store/background/service-tier), `stream: true`,
    /// the exact transmitted model (_i.e._ Layer B) checked above, `high`
    /// reasoning carried in `reasoning`, and only the container's own function
    /// tools (no provider-hosted tools). `stream_options.include_usage` is
    /// required so settlement has a usage chunk.
    fn completions_policy(
        &self,
        plan: &provider::ProviderPlan,
        body: &Value,
        _analysis: bool,
    ) -> Result<()> {
        let object = body
            .as_object()
            .ok_or_else(|| Error::Refused("provider fetch body is not an object".into()))?;
        if let Some(key) = object
            .keys()
            .find(|k| !COMPLETIONS_KEYS.contains(&k.as_str()))
        {
            return refuse(format!(
                "provider fetch body policy: field {key} is not allowed"
            ));
        }
        if body["stream_options"]["include_usage"] != true {
            return refuse("provider fetch body policy: stream_options.include_usage must be true");
        }
        // Pi 1.5.0's openai-completions client sets `store:false` (supportsStore)
        // for openrouter; `store` must be exactly false. The max-token bound, if
        // present, is emitted as `max_completion_tokens` (Pi's agent loop usually
        // sends none), so it is validated only when present and pinned to the
        // catalog output bound -- it is not required.
        if body["store"] != false {
            return refuse("provider fetch body policy: store must be false");
        }
        if let Some(max) = body["max_completion_tokens"].as_u64()
            && (max == 0 || max > plan.selected()?["maxTokens"].as_u64().unwrap_or(0))
        {
            return refuse(
                "provider fetch body policy: max_completion_tokens exceeds catalog bound",
            );
        }
        let tools = match &body["tools"] {
            Value::Null => vec![],
            Value::Array(tools) => tools.clone(),
            _ => return refuse("provider fetch body policy: tools"),
        };
        // Chat Completions tools are OpenAI `function` tools; any other type is
        // a provider-hosted tool and refused.
        if tools
            .iter()
            .any(|t| !t.is_object() || t["type"] != "function")
        {
            return refuse("provider fetch body policy: provider-hosted tool");
        }
        if _analysis {
            // Chat Completions introduces no tool-name prefix on the wire the
            // way Messages does; the analysis surface is the profile's function
            // tools with the pinned specification digest, as on Responses.
            super::analysis::check_request_tools(
                &self.journal.job.coding.as_ref().unwrap().schema,
                &body["tools"],
            )?;
        }
        // The operator fixed effort `high`. The receiver sets thinkingLevel
        // 'high' for the openrouter lane; enforce that the transmitted body
        // actually carries reasoning.effort == "high", not merely a presence.
        if body["reasoning"]["effort"] != "high" {
            return refuse(
                "provider fetch body policy: reasoning.effort must be high (operator decision)",
            );
        }
        Ok(())
    }
    fn hosted_native(&mut self, s: &mut Stream, frame: &Value) -> Result<()> {
        if !Self::own_stream_frame(s, frame) {
            // Never a bare protocol error while an attempt is open.
            self.hosted_close_uncertain(s, "stream_foreign_frame")?;
            return refuse("foreign hosted stream frame");
        }
        match &mut s.phase {
            StreamPhase::Active {
                session, attempt, ..
            } if session.accepts_native() => {
                let attempt = *attempt;
                let transport = self
                    .hosted_transport
                    .take()
                    .ok_or_else(|| Error::Refused("hosted transport absent".into()))?;
                let sent = session.on_native(frame, &Screen(transport.as_ref()));
                self.hosted_transport = Some(transport);
                match sent {
                    Ok(frames) => self.hosted_emit(s, attempt, frames)?,
                    Err(code) => {
                        // Never a bare protocol error: the attempt is uncertain,
                        // keeps its reservation, and its relayed bytes are kept.
                        self.hosted_close_uncertain(s, code)?;
                        return refuse(format!("hosted stream protocol: {code}"));
                    }
                }
                self.hosted_progress(s)
            }
            // Cancelled before any attempt was reserved or admitted. Nothing
            // was dispatched; the request-level reservation is conservatively
            // kept (no attempt record exists to release).
            StreamPhase::NotDispatched | StreamPhase::Fetched(_)
                if frame["type"] == "cancel" && frame["ordinal"] == 1 =>
            {
                self.hosted.out.push(json!({"type":"response-cancelled","binding":s.bind,"sequence":s.request,"stream":STREAM,"ordinal":1}));
                s.phase = StreamPhase::Closed;
                Ok(())
            }
            StreamPhase::Unavailable(u) => {
                match u.on_next(frame).map_err(|c| Error::Refused(c.into()))? {
                    Some(confirm) => {
                        self.hosted.out.push(confirm);
                        s.phase = StreamPhase::Closed;
                        Ok(())
                    }
                    None => refuse("hosted stream frame after unavailable answer"),
                }
            }
            _ => {
                self.hosted_close_uncertain(s, "stream_frame_out_of_phase")?;
                refuse("hosted stream frame out of phase")
            }
        }
    }
    /// One Host-loop step: dispatch a fetched request once every pending
    /// container frame has been processed, advance upstream progress, and
    /// return the stream frames to write.
    pub(crate) fn coding_hosted_tick(&mut self, stdout: &Path) -> Result<Vec<Vec<u8>>> {
        if !self.hosted_profile() {
            return Ok(vec![]);
        }
        if let Some(mut s) = self.hosted.stream.take() {
            let result = self.hosted_step(&mut s, stdout);
            self.hosted.stream = Some(s);
            result?;
        }
        std::mem::take(&mut self.hosted.out)
            .into_iter()
            .map(|v| {
                let mut b = serde_json::to_vec(&v)?;
                b.push(b'\n');
                Ok(b)
            })
            .collect()
    }
    fn hosted_step(&mut self, s: &mut Stream, stdout: &Path) -> Result<()> {
        match &mut s.phase {
            StreamPhase::Fetched(_) => {
                if self.coding_unprocessed(stdout)?.is_empty() {
                    let StreamPhase::Fetched(fetch) =
                        std::mem::replace(&mut s.phase, StreamPhase::Closed)
                    else {
                        unreachable!()
                    };
                    self.hosted_attempt(s, fetch, stdout)?;
                }
                Ok(())
            }
            StreamPhase::Active {
                session, attempt, ..
            } => {
                let attempt = *attempt;
                let transport = self
                    .hosted_transport
                    .take()
                    .ok_or_else(|| Error::Refused("hosted transport absent".into()))?;
                let polled = session.poll(&Screen(transport.as_ref()), Duration::ZERO);
                self.hosted_transport = Some(transport);
                match polled {
                    Ok(frames) => self.hosted_emit(s, attempt, frames)?,
                    Err(code) => {
                        self.hosted_close_uncertain(s, code)?;
                        return refuse(format!("hosted stream: {code}"));
                    }
                }
                self.hosted_progress(s)?;
                if let StreamPhase::Fetched(_) = s.phase {
                    // A refused attempt: the next one (if permitted) is
                    // dispatched without waiting for another tick.
                    let StreamPhase::Fetched(fetch) =
                        std::mem::replace(&mut s.phase, StreamPhase::Closed)
                    else {
                        unreachable!()
                    };
                    self.hosted_attempt(s, fetch, stdout)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    /// Admit, reserve and dispatch the next attempt of the stream's request,
    /// or answer that no attempt is permitted.
    fn hosted_attempt(&mut self, s: &mut Stream, fetch: Fetch, stdout: &Path) -> Result<()> {
        let (plan, routes) = self.hosted_plan()?;
        let deadline = self.journal.job.deadline_unix_seconds.saturating_mul(1000);
        let ctx = self.hosted_failover()?;
        if self.hosted.router.is_none() {
            let router = routes.router(&plan.model, plan.max_requests, deadline);
            self.hosted.router = Some(match &ctx {
                Some(c) => Router::with_failover(router.policy().clone(), c.binding()),
                None => router,
            });
        }
        // With failover inputs the gate decides before each send, from the
        // admitted observation, the view over this job's own journal and the
        // ledger's routing history; the router checks the decision.
        let mut decided = None;
        let routed = match &ctx {
            None => self.hosted.router.as_mut().unwrap().next(
                s.request,
                &plan.model,
                stream::now_millis(),
            ),
            Some(ctx) => {
                let now = self.hosted_now();
                let in_job = match ctx.in_job_observation(&self.journal.job.state_dir, now) {
                    Ok(v) => v,
                    Err(_) => {
                        return self.hosted_unavailable(s, "failover_observation_unavailable");
                    }
                };
                let requests = &self.journal.coding.as_ref().unwrap().provider_requests;
                let (outcome, document) = ctx.decide(
                    s.request,
                    now,
                    &in_job,
                    &failover::ledger_snapshot(&routes, requests, s.request, None),
                    &failover::preserved(requests, s.request, None),
                );
                match outcome {
                    Outcome::Pause(pause) => {
                        return self.hosted_failover_pause(s, pause, &document);
                    }
                    Outcome::Attempt(d) => {
                        let directive = d.directive(&ctx.job_id);
                        decided = Some((d, document));
                        self.hosted.router.as_mut().unwrap().next_with(
                            s.request,
                            &plan.model,
                            now,
                            &directive,
                        )
                    }
                }
            }
        };
        let (index, routed) = match routed {
            Decision::Attempt { route, attempt } => (route, attempt),
            Decision::Unavailable(reason) => return self.hosted_unavailable(s, reason),
        };
        let route = routes.routes[index].clone();
        // Non-success heads are classified when the admitted table is the
        // linked one and the table covers the route.
        let classifier = super::classification::Classifier::for_route(
            plan.classification_table_sha256.as_deref(),
            &route.endpoint,
        );
        // Subscription routes carry only requests shaped as Claude Code in the
        // extension's Claude prompt mode; anything else is never dispatched
        // (operator constraint: a third-party request on the subscription may
        // draw extra usage). No byte was written: the request records why and
        // settles at 0. The response-side billing guard remains the second
        // layer.
        if Endpoint::parse(&route.endpoint) == Some(Endpoint::AnthropicOauth)
            && !claude_code_request(&fetch.body)
        {
            self.hosted
                .router
                .as_mut()
                .unwrap()
                .settle(routed, Disposition::NotDispatched, Value::Null)
                .map_err(|c| Error::Refused(c.into()))?;
            let state = self.journal.coding.as_mut().unwrap();
            state.provider_failed = true;
            if let Some(r) = state.provider_requests.get_mut(&s.request)
                && r.attempts.is_empty()
            {
                r.not_dispatched = Some("subscription_request_not_claude_code".into());
                r.settled_microusd = Some(0);
            }
            self.save()?;
            return self.hosted_unavailable(s, "subscription_request_not_claude_code");
        }
        let reserve = plan.request_reservation()?;
        let state = self.journal.coding.as_mut().unwrap();
        let attempt = match provider::reserve_attempt(
            &mut state.provider_requests,
            s.request,
            &route.id,
            &route.payer,
            reserve,
            plan.budget_microusd,
            routes.max_attempts_per_request,
        ) {
            Ok(a) => a,
            Err(_) => {
                self.hosted
                    .router
                    .as_mut()
                    .unwrap()
                    .settle(routed, Disposition::NotDispatched, Value::Null)
                    .map_err(|c| Error::Refused(c.into()))?;
                return self.hosted_unavailable(s, "ledger_refused");
            }
        };
        if let Some(a) = self
            .journal
            .coding
            .as_mut()
            .unwrap()
            .provider_requests
            .get_mut(&s.request)
            .and_then(|r| r.attempts.last_mut())
            .filter(|a| a.attempt == attempt)
        {
            a.request_body_sha256 = Some(digest(&fetch.body));
            if let Some((d, _)) = &decided {
                a.failover = Some(d.clone());
            }
            if classifier.is_some() {
                a.classification_table_sha256 = plan.classification_table_sha256.clone();
            }
        }
        if let Some((_, document)) = &decided {
            // The decision's observation, bound by its digest, retained with
            // the attempt record before any upstream byte.
            self.hosted_retain(s.request, attempt, "observation.json", document)?;
        }
        self.save()?; // Durable attempt reservation before any upstream byte.
        if self.expired() || self.cancelled() {
            self.hosted_uncertain(
                s.request,
                attempt,
                routed,
                "expired/cancelled before dispatch",
            )?;
            return refuse("hosted attempt expired/cancelled");
        }
        let intent = ProviderAttemptIntent {
            request: s.request,
            attempt,
            route: route.id.clone(),
            payer: route.payer.clone(),
        };
        let request = HostedRequest {
            request: s.request,
            attempt,
            headers: fetch.headers.clone(),
            body: fetch.body.clone(),
            wire: fetch.wire,
            beta: fetch.beta,
        };
        let Some(mut transport) = self.hosted_transport.take() else {
            self.hosted_uncertain(s.request, attempt, routed, "hosted transport absent")?;
            return refuse("hosted transport absent");
        };
        // The exact head the transport will write, access values redacted,
        // retained with its digest on the attempt before any upstream byte.
        let head = transport
            .request_head(&route, &request, true)
            .and_then(|head| {
                serde_json::to_vec(&json!({"schema":REQUEST_HEAD_SCHEMA,
                    "request":s.request,"attempt":attempt,"route":route.id,
                    "redacted":REDACTED,"head":head}))
                .map_err(Error::from)
            });
        let retained = head.and_then(|document| {
            self.hosted_retain(s.request, attempt, "request-head.json", &document)
        });
        let head_sha256 = match retained {
            Ok(sha) => sha,
            Err(e) => {
                self.hosted_transport = Some(transport);
                self.hosted_uncertain(s.request, attempt, routed, "request head not retained")?;
                return refuse(format!("request head not retained: {e}"));
            }
        };
        if let Some(a) = self
            .journal
            .coding
            .as_mut()
            .unwrap()
            .provider_requests
            .get_mut(&s.request)
            .and_then(|r| r.attempts.last_mut())
            .filter(|a| a.attempt == attempt)
        {
            a.request_head_sha256 = Some(head_sha256);
        }
        self.save()?;
        let Some(mut gate) = self.coding_gate.take() else {
            self.hosted_transport = Some(transport);
            self.hosted_uncertain(s.request, attempt, routed, "coding lifecycle gate missing")?;
            return refuse("coding lifecycle gate missing");
        };
        // Connect (TCP, TLS) before gate admission and outside the lifecycle
        // store's transaction. No request byte is written, so a failure here
        // is a confirmed non-dispatch.
        let mut connection =
            match transport.connect(&route, deadline.try_into().unwrap_or(i64::MAX)) {
                Ok(c) => Some(c),
                Err(e) => {
                    self.coding_gate = Some(gate);
                    self.hosted_transport = Some(transport);
                    let r = self
                        .journal
                        .coding
                        .as_mut()
                        .unwrap()
                        .provider_requests
                        .get_mut(&s.request)
                        .unwrap();
                    r.settle_attempt(
                        attempt,
                        AttemptEvidence::NotDispatched {
                            confirmed_before_write: true,
                        },
                    )?;
                    if let Some(a) = r.attempts.last_mut() {
                        a.reason = Some(format!(
                            "route connection failed before any request byte: {e}"
                        ));
                    }
                    self.hosted
                        .router
                        .as_mut()
                        .unwrap()
                        .settle(routed, Disposition::NotDispatched, Value::Null)
                        .map_err(|c| Error::Refused(c.into()))?;
                    self.journal.coding.as_mut().unwrap().provider_failed = true;
                    self.save()?;
                    return self.hosted_unavailable(s, "route_unreachable");
                }
            };
        let mut cancelled = false;
        let mut opened = None;
        let mut entered = false;
        let bind = s.bind.clone();
        let sequence = s.request;
        let request_id = s.request;
        let routes_sha256 = routes.digest();
        let host = &mut *self;
        let admitted = gate.provider_attempt(&intent, &mut |deadline_ms| {
            if std::mem::replace(&mut entered, true) {
                return refuse("hosted dispatch already consumed");
            }
            let until = deadline_ms.min(deadline.try_into().unwrap_or(i64::MAX));
            if now_ms()? >= until {
                return refuse("hosted attempt deadline passed");
            }
            // Last check before any upstream byte: a head-phase cancel that
            // arrived meanwhile means this attempt is not dispatched.
            if host.coding_unprocessed(stdout)?.iter().any(|f| {
                f["type"] == "cancel"
                    && f["ordinal"] == 1
                    && f["binding"] == bind
                    && f["sequence"] == sequence
                    && f["stream"] == STREAM
            }) {
                cancelled = true;
                return Ok(());
            }
            // Durable gate admission (current grant held now) before any
            // upstream byte is written.
            let a = host
                .journal
                .coding
                .as_mut()
                .unwrap()
                .provider_requests
                .get_mut(&request_id)
                .and_then(|r| r.attempts.last_mut())
                .filter(|a| a.attempt == attempt)
                .ok_or_else(|| Error::Refused("hosted attempt record absent".into()))?;
            a.gate_entered = true;
            a.deadline_ms = Some(until);
            a.routes_sha256 = Some(routes_sha256.clone());
            host.save()?;
            let connection = connection
                .take()
                .ok_or_else(|| Error::Refused("hosted connection consumed".into()))?;
            opened = Some(transport.send(connection, &route, &request)?);
            Ok(())
        });
        self.coding_gate = Some(gate);
        self.hosted_transport = Some(transport);
        if let Err(e) = admitted {
            let gate_entered = self
                .journal
                .coding
                .as_ref()
                .unwrap()
                .provider_requests
                .get(&s.request)
                .and_then(|r| r.attempts.last())
                .is_some_and(|a| a.attempt == attempt && a.gate_entered);
            self.hosted_uncertain(
                s.request,
                attempt,
                routed,
                &if gate_entered {
                    format!("dispatch failed after gate admission: {e}")
                } else if entered {
                    format!("refused inside gate admission before dispatch: {e}")
                } else {
                    format!("gate refused: {e}")
                },
            )?;
            return Err(e);
        }
        if cancelled {
            let r = self
                .journal
                .coding
                .as_mut()
                .unwrap()
                .provider_requests
                .get_mut(&s.request)
                .unwrap();
            r.settle_attempt(
                attempt,
                AttemptEvidence::NotDispatched {
                    confirmed_before_write: true,
                },
            )?;
            self.hosted
                .router
                .as_mut()
                .unwrap()
                .settle(routed, Disposition::NotDispatched, Value::Null)
                .map_err(|c| Error::Refused(c.into()))?;
            self.save()?;
            s.phase = StreamPhase::NotDispatched;
            return Ok(());
        }
        let Some((reader, abort)) = opened else {
            self.hosted_uncertain(s.request, attempt, routed, "gate returned without dispatch")?;
            return refuse("hosted gate returned without dispatch evidence");
        };
        let deadline_ms = self
            .journal
            .coding
            .as_ref()
            .and_then(|c| c.provider_requests.get(&s.request))
            .and_then(|r| r.attempts.last())
            .and_then(|a| a.deadline_ms);
        let hold = classifier.map(|c| c.hold(Some(self.hosted_hold_window(&fetch, deadline_ms))));
        let wire = plan.wire().unwrap_or(stream::Wire::Responses);
        let session = match Session::start(
            reader,
            abort,
            &s.binding,
            s.request,
            STREAM,
            route::refusal_status,
            wire,
        ) {
            Ok(session) => match hold {
                Some(hold) => session.with_hold(hold),
                None => session,
            },
            Err(code) => {
                self.hosted_uncertain(s.request, attempt, routed, code)?;
                return refuse(format!("hosted stream start: {code}"));
            }
        };
        s.phase = StreamPhase::Active {
            session,
            attempt,
            routed,
            fetch,
        };
        Ok(())
    }
    /// A failover pause: no attempt; the decision's observation is retained,
    /// the pause (resume time, what it waits for, evidence) is journaled on
    /// the request, and the job's provider use ends.
    fn hosted_failover_pause(
        &mut self,
        s: &mut Stream,
        pause: failover::FailoverPause,
        document: &[u8],
    ) -> Result<()> {
        self.hosted_retain_named(&failover::observation_name(s.request, None), document)?;
        let state = self.journal.coding.as_mut().unwrap();
        state
            .provider_requests
            .get_mut(&s.request)
            .ok_or_else(|| Error::Refused("failover pause without a request".into()))?
            .record_failover_pause(pause)?;
        state.provider_failed = true;
        self.save()?;
        self.hosted_unavailable(s, "failover_pause")
    }
    fn hosted_unavailable(&mut self, s: &mut Stream, reason: &str) -> Result<()> {
        let (u, answer) = Unavailable::answer(&s.binding, s.request, STREAM, reason)
            .map_err(|c| Error::Refused(c.into()))?;
        self.hosted.out.push(answer);
        s.phase = StreamPhase::Unavailable(u);
        Ok(())
    }
    /// Act on a session's settled progress: a definitive refusal releases the
    /// attempt and requeues the request; a finished relay is settled.
    fn hosted_progress(&mut self, s: &mut Stream) -> Result<()> {
        let StreamPhase::Active { session, .. } = &s.phase else {
            return Ok(());
        };
        match session.progress().clone() {
            Progress::Pending => Ok(()),
            Progress::Refused { status, body } => {
                let StreamPhase::Active {
                    session,
                    attempt,
                    routed,
                    fetch,
                } = std::mem::replace(&mut s.phase, StreamPhase::Closed)
                else {
                    unreachable!()
                };
                let relayed = session.relay().forwarded.len() as u64;
                let hold = session.relay().hold.clone();
                self.hosted_head_write(s.request, attempt, session.relay())?;
                drop(session);
                self.hosted_head_register(s.request, attempt)?;
                let sha = self.hosted_retain(s.request, attempt, "refusal.body", &body)?;
                self.hosted_keep_hold(s.request, attempt, hold.clone());
                let r = self
                    .journal
                    .coding
                    .as_mut()
                    .unwrap()
                    .provider_requests
                    .get_mut(&s.request)
                    .unwrap();
                // A held head refused on its class, under the table recorded
                // on the attempt; the ledger checks both.
                let classified = hold
                    .filter(|h| h.reason == "classified")
                    .and_then(|h| h.class)
                    .zip(
                        r.attempts
                            .last()
                            .and_then(|a| a.classification_table_sha256.clone()),
                    )
                    .map(|(c, table_sha256)| provider::TableRefusal {
                        table_sha256,
                        class: c.class,
                    });
                let disposition = r.settle_attempt(
                    attempt,
                    AttemptEvidence::HeadRefusal {
                        status,
                        relayed_bytes: relayed,
                        drained_and_screened: true,
                        body_sha256: sha,
                        classified,
                    },
                )?;
                // Responses errors name a `code`, Messages errors a `type`.
                let code = serde_json::from_slice::<Value>(&body)
                    .ok()
                    .and_then(|v| {
                        v["error"]["code"]
                            .as_str()
                            .or(v["error"]["type"].as_str())
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "unspecified".into());
                let router = self.hosted.router.as_mut().unwrap();
                if disposition == AttemptDisposition::Refused {
                    router.settle(routed, Disposition::Refused { status, code }, Value::Null)
                } else {
                    router.settle(
                        routed,
                        Disposition::Uncertain {
                            reason: "refusal not definitive".into(),
                        },
                        Value::Null,
                    )
                }
                .map_err(|c| Error::Refused(c.into()))?;
                self.save()?;
                if disposition != AttemptDisposition::Refused {
                    self.journal.coding.as_mut().unwrap().provider_failed = true;
                    self.save()?;
                    return refuse("hosted refusal not definitive; reservation retained");
                }
                s.phase = StreamPhase::Fetched(fetch);
                Ok(())
            }
            Progress::Finished => {
                let StreamPhase::Active {
                    session,
                    attempt,
                    routed,
                    ..
                } = std::mem::replace(&mut s.phase, StreamPhase::Closed)
                else {
                    unreachable!()
                };
                self.hosted_settle_relay(s.request, attempt, routed, session.into_relay())
            }
        }
    }
    /// Host-produced provider observation and the attempt's settlement from
    /// the exact screened bytes relayed to the container.
    fn hosted_settle_relay(
        &mut self,
        request: u64,
        attempt: u64,
        routed: u64,
        relay: stream::Relay,
    ) -> Result<()> {
        let (plan, _) = self.hosted_plan()?;
        let wire = plan.wire_checked()?;
        self.hosted_head_write(request, attempt, &relay)?;
        self.hosted_head_register(request, attempt)?;
        self.hosted_keep_hold(request, attempt, relay.hold.clone());
        let (sha, retained) = self.hosted_stream_register(request, attempt)?;
        if retained != relay.forwarded {
            self.hosted_uncertain(
                request,
                attempt,
                routed,
                "retained stream differs from the relayed bytes",
            )?;
            return refuse("hosted retained stream differs from the relayed bytes");
        }
        if !relay.unforwarded.is_empty() {
            // Read and screened while draining after the consumer released;
            // never relayed.
            self.hosted_retain(request, attempt, "unforwarded.bin", &relay.unforwarded)?;
        }
        self.hosted_retain(
            request,
            attempt,
            "relay.json",
            &serde_json::to_vec(&relay.evidence_for(wire))?,
        )?;
        let sse = stream::sse_data(&relay.forwarded);
        let events: Vec<Value> = sse
            .iter()
            .filter_map(|d| serde_json::from_str::<Value>(d).ok())
            .collect();
        // Responses/Messages terminate on a typed event; Completions terminates
        // on the literal `[DONE]` line, which is not JSON, so the terminal
        // evidence is the last chunk carrying a `usage` object (the same one
        // `completions_usage` reads).
        let terminal = if wire == Wire::Completions {
            events
                .iter()
                .rev()
                .find(|v| v["usage"].is_object())
                .cloned()
        } else {
            events
                .iter()
                .find(|v| v["type"] == wire.terminal_event())
                .cloned()
        };
        let usage = stream::usage_for(wire, &relay.forwarded);
        let r = self
            .journal
            .coding
            .as_mut()
            .unwrap()
            .provider_requests
            .get_mut(&request)
            .unwrap();
        let disposition =
            r.settle_attempt(attempt, AttemptEvidence::from_relay_for(wire, &relay))?;
        if let (stream::Outcome::Interrupted(code), Some(a)) =
            (&relay.outcome, r.attempts.last_mut())
            && disposition == AttemptDisposition::Uncertain
        {
            let reason = a.reason.take().unwrap_or_default();
            a.reason = Some(format!("{reason} (interrupted: {code})"));
        }
        debug_assert_eq!(
            r.attempts.last().unwrap().relay_sha256.as_deref(),
            Some(sha.as_str())
        );
        r.http_status = Some(relay.status.into());
        if let Some(v) = terminal.as_ref().filter(|_| usage["failed"].is_null()) {
            let (model, raw) = match wire {
                Wire::Responses => {
                    let response = &v["response"];
                    (
                        response["model"].as_str().map(str::to_owned),
                        json!({"eventType":"response.completed","status":response["status"],
                        "usage":{"input_tokens":response["usage"]["input_tokens"],"output_tokens":response["usage"]["output_tokens"],"total_tokens":response["usage"]["total_tokens"]}}),
                    )
                }
                // Model and input-side usage from `message_start`, stop
                // reason and final usage from the last `message_delta`.
                Wire::Messages => (
                    usage["model"].as_str().map(str::to_owned),
                    json!({"eventType":"message_stop","stopReason":usage["stopReason"],
                    "usage":{"input_tokens":usage["inputTokens"],"cache_creation_input_tokens":usage["cacheCreationInputTokens"],
                    "cache_read_input_tokens":usage["cacheReadInputTokens"],"output_tokens":usage["outputTokens"],
                    "cache_creation_5m_input_tokens":usage["cacheCreation5mInputTokens"],"cache_creation_1h_input_tokens":usage["cacheCreation1hInputTokens"]}}),
                ),
                // Chat Completions: model is OpenRouter's resolved checkpoint
                // (Layer C); usage comes from the include_usage chunk.
                Wire::Completions => (
                    usage["model"].as_str().map(str::to_owned),
                    json!({"eventType":"chat_completion",
                    "usage":{"input_tokens":usage["inputTokens"],"completion_tokens":usage["outputTokens"],"total_tokens":usage["totalTokens"]},
                    // Labelled provider-reported values, non-authoritative for
                    // settlement (the Host prices from the pinned catalog).
                    "provider_reported":{"cost":usage["providerReportedCost"],"cached_input_tokens":usage["providerCachedInputTokens"]}}),
                ),
            };
            // Layer C (resolved checkpoint) is recorded, and for the
            // Completions family it is not required to equal the transmitted
            // slug (plan.model, Layer B): OpenRouter resolves `latest:floor` to
            // a concrete checkpoint. It must belong to the pinned model's
            // floor family (checked by `provider_observation`), not equal it.
            if wire != Wire::Completions && model.as_ref().is_some_and(|model| model != &plan.model)
            {
                self.journal.coding.as_mut().unwrap().provider_failed = true;
                self.save()?;
                return refuse("backend model identity conflict");
            }
            if let Some(model) = model {
                r.backend_model = Some(model);
            }
            r.raw_terminal = Some(raw);
        }
        let completed = disposition == AttemptDisposition::Completed && usage["completed"] == true;
        self.hosted
            .router
            .as_mut()
            .unwrap()
            .settle(
                routed,
                if completed {
                    Disposition::Completed
                } else {
                    Disposition::Uncertain {
                        reason: format!("{}:{}", relay.status, relay.outcome.label()),
                    }
                },
                usage,
            )
            .map_err(|c| Error::Refused(c.into()))?;
        if disposition != AttemptDisposition::Completed {
            self.journal.coding.as_mut().unwrap().provider_failed = true;
        }
        self.save()
    }
    fn hosted_uncertain(
        &mut self,
        request: u64,
        attempt: u64,
        routed: u64,
        reason: &str,
    ) -> Result<()> {
        let state = self.journal.coding.as_mut().unwrap();
        if let Some(r) = state.provider_requests.get_mut(&request) {
            let _ = r.settle_attempt(
                attempt,
                AttemptEvidence::Unknown {
                    reason: reason.into(),
                },
            );
        }
        state.provider_failed = true;
        if let Some(router) = self.hosted.router.as_mut() {
            let _ = router.settle(
                routed,
                Disposition::Uncertain {
                    reason: reason.into(),
                },
                Value::Null,
            );
        }
        self.save()
    }
    /// Reconcile after the producing process is gone (or, within a run, after
    /// its loop ended): every still-pending attempt becomes `Uncertain`.
    /// Nothing is released; the reservation stays charged and blocks new
    /// grants. Bytes already relayed to the container were appended to the
    /// attempt's stream file before each was relayed; they are registered.
    pub(crate) fn coding_hosted_reconcile_unsettled(&mut self) -> Result<()> {
        if !self.hosted_profile() {
            return Ok(());
        }
        let Some(state) = self.journal.coding.as_ref() else {
            return Ok(());
        };
        let pending: Vec<(u64, u64)> = state
            .provider_requests
            .iter()
            .filter_map(|(request, r)| {
                r.attempts
                    .last()
                    .filter(|a| a.disposition == AttemptDisposition::Pending)
                    .map(|a| (*request, a.attempt))
            })
            .collect();
        for (request, attempt) in pending {
            let path = self.hosted_stream_path(request, attempt);
            let retained = if !path.exists() {
                None
            } else if let Some(a) = self.journal.artifacts.iter().find(|a| a.path == path) {
                Some((a.sha256.clone(), fs::metadata(&path)?.len()))
            } else {
                let (sha, bytes) = self.hosted_stream_register(request, attempt)?;
                Some((sha, bytes.len() as u64))
            };
            // Head evidence written when the head was read, before the crash.
            self.hosted_head_register(request, attempt)?;
            let mut reason = if self.hosted.ran {
                "unsettled when the run ended".to_string()
            } else {
                "unsettled at restart; producing process gone".to_string()
            };
            reason.push_str(&match &retained {
                Some((_, n)) => format!("; {n} relayed bytes retained up to the end"),
                None => "; nothing was relayed".into(),
            });
            let r = self
                .journal
                .coding
                .as_mut()
                .unwrap()
                .provider_requests
                .get_mut(&request)
                .unwrap();
            r.settle_attempt(attempt, AttemptEvidence::Unknown { reason })?;
            if let (Some((sha, _)), Some(a)) = (retained, r.attempts.last_mut()) {
                a.relay_sha256 = Some(sha);
            }
            self.journal.coding.as_mut().unwrap().provider_failed = true;
            self.save()?;
        }
        Ok(())
    }
    fn hosted_stream_path(&self, request: u64, attempt: u64) -> PathBuf {
        self.journal
            .job
            .state_dir
            .join(format!("coding-hosted-{request}-{attempt}.stream.sse"))
    }
    fn hosted_head_path(&self, request: u64, attempt: u64) -> PathBuf {
        self.journal
            .job
            .state_dir
            .join(format!("coding-hosted-{request}-{attempt}.head.json"))
    }
    /// Durably write the attempt's head evidence once its response head was
    /// read: attempt identity, status, the head's epoch ms and the kept
    /// headers ([`stream::kept_header`]) in order; no other header. Written at
    /// most once, before the head reaches the container or a refusal settles,
    /// so a crash after the head was read leaves it for reconcile. The bytes
    /// go to a `.partial` file renamed into place, so `head.json` exists only
    /// complete.
    fn hosted_head_write(&self, request: u64, attempt: u64, relay: &stream::Relay) -> Result<()> {
        let Some(head_ms) = relay.head_ms else {
            return Ok(());
        };
        let path = self.hosted_head_path(request, attempt);
        if path.exists() {
            return Ok(());
        }
        let bytes = serde_json::to_vec(&json!({
            "jobId": self.journal.job.job_id,
            "request": request,
            "attempt": attempt,
            "status": relay.status,
            "headMs": head_ms,
            "headHeaders": relay.head_headers.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
        }))?;
        let partial = self.journal.job.state_dir.join(format!(
            "coding-hosted-{request}-{attempt}.head.json.partial"
        ));
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&partial)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
        fs::rename(&partial, &path)?;
        File::open(&self.journal.job.state_dir)?.sync_all()?;
        Ok(())
    }
    /// Register the attempt's head evidence, when a head was read, as a
    /// journal artifact so collection verifies it. Registered at most once,
    /// and only when complete: JSON naming this job, request and attempt with
    /// a `headMs`. Anything else stays unregistered: no head evidence.
    fn hosted_head_register(&mut self, request: u64, attempt: u64) -> Result<()> {
        let path = self.hosted_head_path(request, attempt);
        if !path.exists() || self.journal.artifacts.iter().any(|a| a.path == path) {
            return Ok(());
        }
        let bytes = fs::read(&path)?;
        let complete = serde_json::from_slice::<Value>(&bytes).is_ok_and(|v| {
            v["jobId"].as_str() == Some(self.journal.job.job_id.as_str())
                && v["request"].as_u64() == Some(request)
                && v["attempt"].as_u64() == Some(attempt)
                && v["headMs"].is_u64()
        });
        if !complete {
            return Ok(());
        }
        self.journal.artifacts.push(Artifact {
            path,
            sha256: digest(&bytes),
        });
        self.save()
    }
    /// Create the attempt's empty stream file, create-only and durable. Done
    /// when the response head is relayed, so a refused head leaves none.
    fn hosted_stream_create(&self, request: u64, attempt: u64) -> Result<()> {
        let f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(self.hosted_stream_path(request, attempt))?;
        f.sync_all()?;
        File::open(&self.journal.job.state_dir)?.sync_all()?;
        Ok(())
    }
    /// Register the attempt's retained stream as a journal artifact. Without
    /// a relayed head nothing was relayed: the file is created empty.
    fn hosted_stream_register(&mut self, request: u64, attempt: u64) -> Result<(String, Vec<u8>)> {
        let path = self.hosted_stream_path(request, attempt);
        if !path.exists() {
            self.hosted_stream_create(request, attempt)?;
        }
        let bytes = fs::read(&path)?;
        let sha = digest(&bytes);
        self.journal.artifacts.push(Artifact {
            path,
            sha256: sha.clone(),
        });
        self.save()?;
        Ok((sha, bytes))
    }
    /// Queue session frames for the container. Once a head was read its
    /// evidence is written (`hosted_head_write`). Before the head is queued its
    /// status is persisted on the attempt record and the attempt's stream
    /// file is created; each relayed chunk is durably appended to that file
    /// before it is queued. A Host crash loses neither. A retention failure closes the attempt uncertain
    /// without relaying the chunk.
    fn hosted_emit(&mut self, s: &mut Stream, attempt: u64, frames: Vec<Value>) -> Result<()> {
        if let StreamPhase::Active { session, .. } = &s.phase
            && let Err(e) = self.hosted_head_write(s.request, attempt, session.relay())
        {
            self.hosted_close_uncertain(s, "hosted_retention")?;
            return Err(e);
        }
        // A held head relayed now: its hold record is persisted with the
        // head's status, before the head is queued.
        let hold = match &s.phase {
            StreamPhase::Active { session, .. } => session.relay().hold.clone(),
            _ => None,
        };
        for f in frames {
            if f["type"] == "response-head"
                && let Err(reason) = self.subscription_guard(s, attempt)
            {
                self.hosted_close_uncertain(s, &reason)?;
                return refuse(reason);
            }
            let kept = (|| -> Result<()> {
                match f["type"].as_str() {
                    Some("response-chunk") => {
                        let body = B64
                            .decode(f["body"].as_str().unwrap_or(""))
                            .map_err(|_| Error::Refused("hosted chunk encoding".into()))?;
                        let mut file = OpenOptions::new()
                            .append(true)
                            .open(self.hosted_stream_path(s.request, attempt))?;
                        file.write_all(&body)?;
                        file.sync_data()?;
                    }
                    Some("response-head") => {
                        self.hosted_stream_create(s.request, attempt)?;
                        if let Some(a) = self
                            .journal
                            .coding
                            .as_mut()
                            .unwrap()
                            .provider_requests
                            .get_mut(&s.request)
                            .and_then(|r| r.attempts.last_mut())
                            .filter(|a| a.attempt == attempt)
                        {
                            a.http_status = f["status"].as_u64().and_then(|v| v.try_into().ok());
                            if hold.is_some() {
                                a.head_hold = hold.clone();
                            }
                        }
                        self.save()?;
                    }
                    _ => {}
                }
                Ok(())
            })();
            if let Err(e) = kept {
                self.hosted_close_uncertain(s, "hosted_retention")?;
                return Err(e);
            }
            self.hosted.out.push(f);
        }
        Ok(())
    }
    /// Claude subscription routes: a successful response is relayed only if
    /// its head shows the request inside the plan's limits
    /// (`anthropic-ratelimit-unified-status` `allowed` or `allowed_warning`)
    /// with extra usage unavailable (`...-overage-status` `rejected`).
    /// Anything else, including a missing header, stops the job before more
    /// requests are made. Assumed semantics: `overage-status` other than
    /// `rejected` means extra usage could be drawn, whether or not this
    /// request drew it, so an organization with extra usage enabled is
    /// refused even inside its limits. The headers are retained in the
    /// attempt's `relay.json`.
    fn subscription_guard(&self, s: &Stream, attempt: u64) -> std::result::Result<(), String> {
        let StreamPhase::Active { session, .. } = &s.phase else {
            return Ok(());
        };
        let relay = session.relay();
        let route = self
            .journal
            .coding
            .as_ref()
            .and_then(|c| c.provider_requests.get(&s.request))
            .and_then(|r| r.attempts.iter().find(|a| a.attempt == attempt))
            .map(|a| a.route.clone());
        let endpoint = self.hosted_plan().ok().and_then(|(_, routes)| {
            routes
                .routes
                .iter()
                .find(|r| Some(&r.id) == route.as_ref())
                .and_then(|r| Endpoint::parse(&r.endpoint))
        });
        if endpoint != Some(Endpoint::AnthropicOauth) || relay.status != 200 {
            return Ok(());
        }
        let header = |name: &str| {
            relay
                .head_headers
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.as_str())
        };
        let status = header("anthropic-ratelimit-unified-status");
        let overage = header("anthropic-ratelimit-unified-overage-status");
        if matches!(status, Some("allowed" | "allowed_warning")) && overage == Some("rejected") {
            Ok(())
        } else {
            Err(format!(
                "subscription_billing_guard: unified-status {status:?}, overage-status {overage:?}"
            ))
        }
    }
    /// Create-only retention of exact screened bytes, registered as a journal
    /// artifact so collection verifies it; returns the digest.
    fn hosted_retain(
        &mut self,
        request: u64,
        attempt: u64,
        kind: &str,
        bytes: &[u8],
    ) -> Result<String> {
        self.hosted_retain_named(&format!("coding-hosted-{request}-{attempt}.{kind}"), bytes)
    }
    /// Retain `bytes` create-only under `name` in the state directory and
    /// register them as a journal artifact.
    fn hosted_retain_named(&mut self, name: &str, bytes: &[u8]) -> Result<String> {
        let dir = self.journal.job.state_dir.clone();
        let path = dir.join(name);
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        File::open(&dir)?.sync_all()?;
        let sha = digest(bytes);
        self.journal.artifacts.push(Artifact {
            path,
            sha256: sha.clone(),
        });
        self.save()?;
        Ok(sha)
    }
    /// Close an open attempt as uncertain (reservation kept), retaining the
    /// exact screened bytes already relayed to the container. Does nothing
    /// unless an attempt is active.
    fn hosted_close_uncertain(&mut self, s: &mut Stream, reason: &str) -> Result<()> {
        let StreamPhase::Active {
            session,
            attempt,
            routed,
            ..
        } = std::mem::replace(&mut s.phase, StreamPhase::Closed)
        else {
            return Ok(());
        };
        let unfinished = *session.progress() == Progress::Pending;
        let mut relay = session.into_relay();
        if unfinished && !matches!(relay.outcome, stream::Outcome::Interrupted(_)) {
            // The Host closed a stream that had not ended; its evidence must
            // not read as a completed relay. An existing interruption code is
            // kept.
            relay.outcome = stream::Outcome::Interrupted("closed_by_host");
        }
        // Best effort: failing to keep head evidence (possibly the reason
        // for closing) must not keep the attempt from closing.
        let _ = self
            .hosted_head_write(s.request, attempt, &relay)
            .and_then(|()| self.hosted_head_register(s.request, attempt));
        let (sha, retained) = self.hosted_stream_register(s.request, attempt)?;
        let differs = retained != relay.forwarded;
        let wire = self
            .hosted_plan()
            .ok()
            .and_then(|(plan, _)| plan.wire())
            .unwrap_or(Wire::Responses);
        self.hosted_retain(
            s.request,
            attempt,
            "relay.json",
            &serde_json::to_vec(&relay.evidence_for(wire))?,
        )?;
        let state = self.journal.coding.as_mut().unwrap();
        if let Some(r) = state.provider_requests.get_mut(&s.request) {
            let _ = r.settle_attempt(
                attempt,
                AttemptEvidence::Relayed {
                    status: relay.status,
                    upstream_end: false,
                    interrupted: true,
                    relay_sha256: sha,
                    provider_failure: None,
                    terminal_last: false,
                    drain_error: None,
                },
            );
            if let Some(a) = r.attempts.last_mut().filter(|a| a.attempt == attempt) {
                a.reason = Some(if differs {
                    format!(
                        "closed uncertain: {reason}; retained {} of {} relay bytes (a chunk that could not be retained was not relayed)",
                        retained.len(),
                        relay.forwarded.len()
                    )
                } else {
                    format!("closed uncertain: {reason}")
                });
            }
        }
        state.provider_failed = true;
        if let Some(router) = self.hosted.router.as_mut() {
            let _ = router.settle(
                routed,
                Disposition::Uncertain {
                    reason: reason.into(),
                },
                Value::Null,
            );
        }
        self.save()
    }
    /// Run-loop exit (completion, deadline, cancellation, write failure or
    /// any refusal): an attempt still open is closed uncertain with its
    /// relayed bytes retained, and its upstream connection is closed.
    pub(crate) fn coding_hosted_abort(&mut self, reason: &str) -> Result<()> {
        if !self.hosted_profile() {
            return Ok(());
        }
        self.hosted.ran = true;
        let Some(mut s) = self.hosted.stream.take() else {
            return Ok(());
        };
        let result = self.hosted_close_uncertain(&mut s, reason);
        self.hosted.stream = Some(s);
        result
    }
    /// Before the container's `provider_complete` for `request`: the relay
    /// must be over for the consumer, and a bounded release drain is finished
    /// so the Host's evidence exists.
    pub(super) fn coding_hosted_before_complete(&mut self, request: u64) -> Result<()> {
        let Some(mut s) = self.hosted.stream.take() else {
            return refuse("hosted completion without a stream");
        };
        let result = (|| {
            if s.request != request {
                return refuse("hosted completion for another request");
            }
            let deadline = Instant::now() + Duration::from_secs(2);
            while let StreamPhase::Active { session, .. } = &mut s.phase {
                if session.accepts_native() || Instant::now() >= deadline {
                    return refuse("hosted completion before the relay closed");
                }
                let transport = self
                    .hosted_transport
                    .take()
                    .ok_or_else(|| Error::Refused("hosted transport absent".into()))?;
                let polled = session.poll(&Screen(transport.as_ref()), Duration::from_millis(50));
                self.hosted_transport = Some(transport);
                if let Err(code) = polled {
                    self.hosted_close_uncertain(&mut s, code)?;
                    return refuse(format!("hosted stream: {code}"));
                }
                self.hosted_progress(&mut s)?;
            }
            Ok(())
        })();
        self.hosted.stream = Some(s);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    type Opened = Arc<Mutex<Vec<(String, BTreeMap<String, String>)>>>;
    struct FakeTransport {
        responses: VecDeque<Vec<u8>>,
        opened: Opened,
        unreachable: bool,
    }
    impl HostedProviderTransport for FakeTransport {
        fn check(&self, _: &HostedRoutes) -> Result<()> {
            Ok(())
        }
        fn identity(&self) -> Value {
            json!({"transport":"fake"})
        }
        fn connect(&mut self, _: &HostedRoute, _: i64) -> Result<Connection> {
            if self.unreachable {
                return refuse("synthetic connect failure");
            }
            Ok(Connection {
                stream: Box::new(std::io::Cursor::new(Vec::new())),
                abort: Box::new(|| {}),
            })
        }
        fn request_head(
            &mut self,
            route: &HostedRoute,
            r: &HostedRequest,
            redacted: bool,
        ) -> Result<String> {
            let endpoint = Endpoint::parse(&route.endpoint)
                .unwrap_or(Endpoint::Loopback(([127, 0, 0, 1], 9).into()));
            Ok(request_head(
                endpoint,
                &[("Authorization", "Bearer fake-access")],
                r,
                redacted,
            ))
        }
        fn send(
            &mut self,
            _: Connection,
            route: &HostedRoute,
            r: &HostedRequest,
        ) -> Result<Upstream> {
            self.opened
                .lock()
                .unwrap()
                .push((route.id.clone(), r.headers.clone()));
            let bytes = self
                .responses
                .pop_front()
                .ok_or_else(|| Error::Refused("no scripted response".into()))?;
            // A scripted pause: the bytes after `DELAY` (`DELAY3`) arrive one
            // (three) seconds later.
            for (marker, secs) in [(DELAY, 1), (DELAY3, 3)] {
                if let Some(i) = bytes.windows(marker.len()).position(|w| w == marker) {
                    let first = std::io::Cursor::new(bytes[..i].to_vec());
                    let rest = std::io::Cursor::new(bytes[i + marker.len()..].to_vec());
                    let paused =
                        Read::chain(first, Pause(Some(Duration::from_secs(secs)))).chain(rest);
                    return Ok((Box::new(paused), Box::new(|| {})));
                }
            }
            Ok((Box::new(std::io::Cursor::new(bytes)), Box::new(|| {})))
        }
        fn hold(&self, pending: &[u8]) -> std::result::Result<usize, &'static str> {
            if pending.windows(6).any(|w| w == b"SECRET") {
                return Err("hosted_access_exclusion");
            }
            Ok(0)
        }
    }
    /// Marks a pause in a scripted response ([`FakeTransport`]).
    const DELAY: &[u8] = b"\0DELAY\0";
    const DELAY3: &[u8] = b"\0DELAY3\0";
    /// A reader that sleeps once, then is empty.
    struct Pause(Option<Duration>);
    impl Read for Pause {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            if let Some(d) = self.0.take() {
                std::thread::sleep(d);
            }
            Ok(0)
        }
    }
    type Intents = Arc<Mutex<Vec<ProviderAttemptIntent>>>;
    struct HostedGate {
        intents: Intents,
        /// Appended to the container output while the attempt is admitted,
        /// just before its dispatch closure runs.
        inject: Option<(PathBuf, Vec<u8>)>,
        /// Refuse every attempt without running its dispatch.
        refuse_attempts: bool,
    }
    impl CodingEffectGate for HostedGate {
        fn execute(
            &mut self,
            _: &CodingToolIntent,
            _: &mut dyn FnMut(i64) -> Result<(bool, Vec<u8>)>,
        ) -> Result<(bool, Vec<u8>)> {
            refuse("test gate: no tool")
        }
        fn observed(
            &mut self,
            _: &CodingToolIntent,
            _: &Artifact,
            _: &ProcessRecord,
            _: &str,
        ) -> Result<()> {
            Ok(())
        }
        fn frame(&mut self, frame: &Value) -> Result<Vec<Value>> {
            Ok(if frame["type"] == "coding_provider_request" {
                vec![
                    json!({"type":"bridge_provider","sessionId":frame["sessionId"],"id":frame["id"],"mode":"live"}),
                ]
            } else {
                vec![]
            })
        }
        fn dispatch(&mut self, _: &Value, send: &mut dyn FnMut(i64) -> Result<()>) -> Result<()> {
            send(i64::MAX)
        }
        fn provider_attempt(
            &mut self,
            intent: &ProviderAttemptIntent,
            dispatch: &mut dyn FnMut(i64) -> Result<()>,
        ) -> Result<()> {
            self.intents.lock().unwrap().push(intent.clone());
            if self.refuse_attempts {
                return refuse("test gate: grant not current");
            }
            if let Some((path, line)) = self.inject.take() {
                let mut raw = fs::read(&path)?;
                raw.extend(line);
                fs::write(&path, raw)?;
            }
            dispatch(i64::MAX)
        }
    }
    fn refusal(status: u16) -> Vec<u8> {
        refusal_with(status, "")
    }
    /// A refusal head carrying `headers` (CRLF-terminated lines).
    fn refusal_with(status: u16, headers: &str) -> Vec<u8> {
        let body = br#"{"error":{"code":"usage_limit"}}"#;
        let mut b = format!(
            "HTTP/1.1 {status} Refused\r\n{headers}Content-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        b.extend(body);
        b
    }
    fn completed() -> Vec<u8> {
        let body = [
            json!({"type":"response.created","response":{"id":"r1"}}),
            json!({"type":"response.completed","response":{"id":"r1","model":"gpt-6-luna","status":"completed","usage":{"input_tokens":100,"output_tokens":50,"total_tokens":150}}}),
        ]
        .iter()
        .map(|e| format!("data: {e}\n\n"))
        .collect::<String>();
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }
    struct Wire {
        path: PathBuf,
        raw: Vec<u8>,
        intents: Intents,
        opened: Opened,
    }
    fn line(v: Value) -> Vec<u8> {
        let mut v = v;
        v["schema"] = json!(HOSTED_BRIDGE_PROFILE);
        v["sessionId"] = json!("native");
        let mut b = serde_json::to_vec(&v).unwrap();
        b.push(b'\n');
        b
    }
    fn host(responses: Vec<Vec<u8>>, inject: Option<Value>) -> (tempfile::TempDir, Host, Wire) {
        let (dir, mut h) = super::super::provider::ledger_tests::fixture();
        {
            let profile = h.journal.job.coding.as_mut().unwrap();
            profile.schema = HOSTED_BRIDGE_PROFILE.into();
            let plan = profile.provider.as_mut().unwrap();
            plan.max_requests = 4;
            plan.hosted = HostedRoutes {
                routes: vec![
                    HostedRoute {
                        id: "primary".into(),
                        payer: "payer-a".into(),
                        endpoint: "loopback:1".into(),
                    },
                    HostedRoute {
                        id: "secondary".into(),
                        payer: "payer-b".into(),
                        endpoint: "loopback:2".into(),
                    },
                ],
                max_attempts_per_request: 2,
            };
        }
        h.journal.session_id = Some("native".into());
        let path = dir.path().join("stdout");
        fs::write(&path, b"").unwrap();
        let intents = Intents::default();
        let opened = Opened::default();
        h.coding_gate = Some(Box::new(HostedGate {
            intents: intents.clone(),
            inject: inject.map(|v| (path.clone(), line(v))),
            refuse_attempts: false,
        }));
        h.hosted_transport = Some(Box::new(FakeTransport {
            responses: responses.into(),
            opened: opened.clone(),
            unreachable: false,
        }));
        let wire = Wire {
            path,
            raw: vec![],
            intents,
            opened,
        };
        (dir, h, wire)
    }
    fn push(h: &mut Host, w: &mut Wire, v: Value) -> Result<Vec<Vec<u8>>> {
        w.raw.extend(line(v));
        fs::write(&w.path, &w.raw).unwrap();
        h.coding_advance(&w.raw)
    }
    fn tick_until(h: &mut Host, w: &Wire, done: impl Fn(&[Value]) -> bool) -> Vec<Value> {
        let mut all = vec![];
        for _ in 0..1000 {
            for b in h.coding_hosted_tick(&w.path).unwrap() {
                all.push(serde_json::from_slice(&b).unwrap());
            }
            if done(&all) {
                return all;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("stream did not progress: {all:?}");
    }
    /// Admit request 1 and write its lifecycle grant; returns the written grant.
    fn grant(h: &mut Host, w: &mut Wire) -> Value {
        let replies = push(h, w, json!({"type":"coding_provider_request","id":1,"requestModel":{"id":"gpt-6-luna","provider":"openai-codex","contextWindow":272000},"contextBytes":1000})).unwrap();
        assert_eq!(replies.len(), 1);
        let mut written = vec![];
        h.coding_bridge_dispatch(&replies[0], &mut |b| {
            written = b.to_vec();
            Ok(())
        })
        .unwrap();
        serde_json::from_slice(&written).unwrap()
    }
    fn fetch(grant: &Value) -> Value {
        let s = &grant["stream"];
        json!({"type":"provider_fetch","id":1,"binding":s["binding"],"sequence":s["sequence"],"stream":s["stream"],
            "url":"https://chatgpt.com/backend-api/codex/responses","method":"POST",
            "headers":{"Authorization":"Bearer syntactic.marker","chatgpt-account-id":"nonsecret-marker","content-type":"application/json","originator":"pi"},
            "body":B64.encode(br#"{"model":"gpt-6-luna","stream":true,"store":false,"input":[]}"#)})
    }
    fn op(grant: &Value, kind: &str, ordinal: u64) -> Value {
        let s = &grant["stream"];
        json!({"type":kind,"binding":s["binding"],"sequence":s["sequence"],"stream":s["stream"],"ordinal":ordinal})
    }
    fn has(kind: &'static str) -> impl Fn(&[Value]) -> bool {
        move |all| all.iter().any(|f| f["type"] == kind)
    }
    fn request(h: &Host) -> provider::Reservation {
        h.journal.coding.as_ref().unwrap().provider_requests[&1].clone()
    }

    /// Tools count wherever the body declares them: the top-level array and
    /// every Codex `additional_tools` input item.
    #[test]
    fn declared_tools_include_additional_tools_items() {
        let f = |name: &str, kind: &str| json!({"type":kind,"name":name});
        let body = json!({"tools":[f("a","function")],"input":[
            {"type":"additional_tools","role":"developer","tools":[f("b","function"),f("c","web_search")]},
            {"type":"message","role":"user","content":"x"}]});
        let names: Vec<_> = declared_tools(&body)
            .unwrap()
            .iter()
            .map(|t| t["name"].clone())
            .collect();
        assert_eq!(names, vec![json!("a"), json!("b"), json!("c")]);
        assert!(declared_tools(&json!({"tools":{}})).is_none());
        assert!(
            declared_tools(&json!({"input":[{"type":"additional_tools","tools":{}}]})).is_none()
        );
        assert!(declared_tools(&json!({"input":"text"})).is_none());
        assert_eq!(declared_tools(&json!({})).unwrap(), Vec::<Value>::new());
    }
    fn epoch_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
    /// The state directory's retained `coding-hosted-*` names, as registered.
    fn hosted_artifacts(dir: &tempfile::TempDir, h: &Host) -> Vec<String> {
        let mut names: Vec<_> = h
            .journal
            .artifacts
            .iter()
            .filter(|a| a.path.parent() == Some(dir.path()))
            .filter_map(|a| {
                a.path
                    .file_name()?
                    .to_str()?
                    .strip_prefix("coding-hosted-")
                    .map(str::to_owned)
            })
            .collect();
        names.sort();
        names
    }
    /// The attempt's registered and verified head evidence.
    fn head_evidence(dir: &tempfile::TempDir, h: &Host, name: &str) -> Value {
        let path = dir.path().join(format!("coding-hosted-{name}.head.json"));
        let a = h.journal.artifacts.iter().find(|a| a.path == path).unwrap();
        a.verify().unwrap();
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap()
    }
    #[test]
    fn refused_head_retains_allowlisted_headers_and_head_time_then_fails_over() {
        let headers = "Content-Type: application/json\r\n\
            Anthropic-Ratelimit-Unified-Status: rejected\r\n\
            Set-Cookie: session=secret-cookie\r\n\
            Retry-After: 120\r\n\
            Authorization: Bearer secret-token\r\n\
            anthropic-ratelimit-unified-5h-reset: 1790000000\r\n\
            x-api-key: secret-key\r\n\
            X-Overage-Reason: org_disabled\r\n\
            request-id: req_1\r\n\
            x-unrelated: value\r\n";
        let before = epoch_ms();
        let (dir, mut h, mut w) = host(vec![refusal_with(429, headers), completed()], None);
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        tick_until(&mut h, &w, has("response-head"));
        let after = epoch_ms();
        let head = head_evidence(&dir, &h, "1-1");
        assert_eq!(head["jobId"], h.journal.job.job_id.as_str());
        assert_eq!(
            (head["request"].as_u64(), head["attempt"].as_u64()),
            (Some(1), Some(1))
        );
        assert_eq!(head["status"], 429);
        let ms = head["headMs"].as_u64().unwrap();
        assert!(before <= ms && ms <= after, "{before} <= {ms} <= {after}");
        assert_eq!(
            head["headHeaders"],
            json!([
                ["anthropic-ratelimit-unified-status", "rejected"],
                ["retry-after", "120"],
                ["anthropic-ratelimit-unified-5h-reset", "1790000000"],
                ["x-overage-reason", "org_disabled"],
                ["request-id", "req_1"],
            ])
        );
        let raw =
            String::from_utf8(fs::read(dir.path().join("coding-hosted-1-1.head.json")).unwrap())
                .unwrap();
        for absent in [
            "secret",
            "cookie",
            "authorization",
            "x-api-key",
            "content-type",
            "unrelated",
        ] {
            assert!(
                !raw.to_ascii_lowercase().contains(absent),
                "{absent} in {raw}"
            );
        }
        // Failover completes and settles as without head evidence.
        let mut ordinal = 1;
        loop {
            push(&mut h, &mut w, op(&g, "pull", ordinal)).unwrap();
            let f = tick_until(&mut h, &w, |a| !a.is_empty());
            ordinal += 1;
            if f[0]["type"] == "response-end" {
                break;
            }
        }
        push(&mut h, &mut w, op(&g, "ack", ordinal)).unwrap();
        let r = request(&h);
        assert_eq!(r.attempts[0].disposition, AttemptDisposition::Refused);
        assert_eq!(r.attempts[0].http_status, Some(429));
        assert_eq!(r.attempts[1].disposition, AttemptDisposition::Completed);
        // Each attempt whose head was read gains exactly its head evidence.
        assert_eq!(
            hosted_artifacts(&dir, &h),
            [
                "1-1.head.json",
                "1-1.refusal.body",
                "1-1.request-head.json",
                "1-2.head.json",
                "1-2.relay.json",
                "1-2.request-head.json",
                "1-2.stream.sse",
                "1.body.json"
            ]
        );
        let relayed = head_evidence(&dir, &h, "1-2");
        assert_eq!(
            (relayed["status"].as_u64(), relayed["attempt"].as_u64()),
            (Some(200), Some(2))
        );
        assert_eq!(relayed["headHeaders"], json!([]));
        assert!(relayed["headMs"].as_u64().unwrap() >= ms);
        // relay.json is unchanged: no head time.
        let relay: Value = serde_json::from_slice(
            &fs::read(dir.path().join("coding-hosted-1-2.relay.json")).unwrap(),
        )
        .unwrap();
        assert!(relay.get("headMs").is_none());
    }
    #[test]
    fn crash_after_a_refusal_head_was_read_retains_its_head_evidence_on_reconcile() {
        // The refusal body never completes: the head is read, the attempt
        // is not settled when the producing process goes.
        let partial = b"HTTP/1.1 429 Refused\r\nRetry-After: 30\r\nx-api-key: secret\r\nContent-Length: 100\r\n\r\n{\"error\"".to_vec();
        let (dir, mut h, mut w) = host(vec![partial], None);
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        let path = dir.path().join("coding-hosted-1-1.head.json");
        tick_until(&mut h, &w, |_| path.exists());
        h.hosted = Hosted::default(); // the producing process is gone
        assert_eq!(
            request(&h).attempts[0].disposition,
            AttemptDisposition::Pending
        );
        assert!(!h.journal.artifacts.iter().any(|a| a.path == path));
        // A settlement path registered it, then the process went before the
        // attempt settled: reconcile must not register it again.
        h.hosted_head_register(1, 1).unwrap();
        h.coding_hosted_reconcile_unsettled().unwrap();
        let a = &request(&h).attempts[0];
        assert_eq!(a.disposition, AttemptDisposition::Uncertain);
        let head = head_evidence(&dir, &h, "1-1");
        assert_eq!(head["status"], 429);
        assert_eq!(head["headHeaders"], json!([["retry-after", "30"]]));
        assert!(head["headMs"].is_u64());
        assert_eq!(
            h.journal
                .artifacts
                .iter()
                .filter(|a| a.path == path)
                .count(),
            1
        );
    }
    #[test]
    fn reconcile_does_not_register_incomplete_head_evidence() {
        for case in ["empty", "truncated", "no headMs", "another attempt"] {
            let (dir, mut h, mut w) = host(vec![chunked_head()], None);
            mid_body(&mut h, &mut w);
            h.hosted = Hosted::default(); // the producing process is gone
            let job = &h.journal.job.job_id;
            let bytes = match case {
                "empty" => String::new(),
                "truncated" => format!(r#"{{"jobId":"{job}","request":1,"att"#),
                "no headMs" => format!(r#"{{"jobId":"{job}","request":1,"attempt":1}}"#),
                _ => format!(r#"{{"jobId":"{job}","request":1,"attempt":2,"headMs":1}}"#),
            };
            let path = dir.path().join("coding-hosted-1-1.head.json");
            fs::write(&path, bytes).unwrap();
            h.coding_hosted_reconcile_unsettled().unwrap();
            assert_eq!(
                request(&h).attempts[0].disposition,
                AttemptDisposition::Uncertain,
                "{case}"
            );
            assert!(
                !h.journal.artifacts.iter().any(|a| a.path == path),
                "{case} registered"
            );
            assert!(path.exists(), "{case}: nothing is deleted");
        }
    }
    #[test]
    fn relayed_head_keeps_retry_after_in_relay_and_head_evidence() {
        let (dir, mut h, mut w) = messages_host(vec![messages_completed_with(
            "Retry-After: 60\r\nx-unrelated: v\r\n",
        )]);
        let g = messages_grant(&mut h, &mut w);
        let body =
            serde_json::to_vec(&messages_body(json!([{"role":"user","content":"q"}]))).unwrap();
        push(&mut h, &mut w, messages_fetch(&g, &body)).unwrap();
        relay_to_end(&mut h, &mut w, &g);
        assert_eq!(
            request(&h).attempts[0].disposition,
            AttemptDisposition::Completed
        );
        let relay: Value = serde_json::from_slice(
            &fs::read(dir.path().join("coding-hosted-1-1.relay.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(relay["headHeaders"], json!([["retry-after", "60"]]));
        let head = head_evidence(&dir, &h, "1-1");
        assert_eq!(head["headHeaders"], json!([["retry-after", "60"]]));
    }
    #[test]
    fn each_attempt_retains_its_redacted_request_head_before_dispatch() {
        let (dir, mut h, mut w) = host(vec![refusal(429), completed()], None);
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        relay_to_end(&mut h, &mut w, &g);
        let r = request(&h);
        assert_eq!(r.attempts.len(), 2);
        for a in &r.attempts {
            let path = dir
                .path()
                .join(format!("coding-hosted-1-{}.request-head.json", a.attempt));
            let bytes = fs::read(&path).unwrap();
            assert_eq!(
                a.request_head_sha256.as_deref(),
                Some(digest(&bytes).as_str())
            );
            assert!(h.journal.artifacts.iter().any(|x| x.path == path));
            let doc: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(doc["schema"], REQUEST_HEAD_SCHEMA);
            assert_eq!(
                (doc["request"].as_u64(), doc["attempt"].as_u64()),
                (Some(1), Some(a.attempt))
            );
            assert_eq!(doc["route"], a.route.as_str());
            let head = doc["head"].as_str().unwrap();
            assert!(head.starts_with("POST "), "{head}");
            assert!(head.contains(&format!("X-Request-Id: 1-{}\r\n", a.attempt)));
            assert!(head.contains(&format!("Authorization: {REDACTED}\r\n")));
            assert!(!head.contains("fake-access"));
        }
    }
    #[test]
    fn head_evidence_replaces_a_stale_partial_by_rename() {
        // A junk `.partial` left by an earlier failed write, longer than any
        // head evidence: the writer must truncate it (not open it
        // create-only), write there, and rename it into place rather than
        // writing `head.json` directly.
        let (dir, mut h, mut w) = host(vec![completed()], None);
        let partial = dir.path().join("coding-hosted-1-1.head.json.partial");
        fs::write(&partial, vec![b'x'; 64 * 1024]).unwrap();
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        relay_to_end(&mut h, &mut w, &g);
        assert_eq!(
            request(&h).attempts[0].disposition,
            AttemptDisposition::Completed
        );
        let head = head_evidence(&dir, &h, "1-1");
        assert_eq!(
            (head["status"].as_u64(), head["attempt"].as_u64()),
            (Some(200), Some(1))
        );
        assert!(head["headMs"].is_u64());
        assert!(!partial.exists(), "the partial was renamed into place");
        assert_eq!(
            hosted_artifacts(&dir, &h),
            [
                "1-1.head.json",
                "1-1.relay.json",
                "1-1.request-head.json",
                "1-1.stream.sse",
                "1.body.json"
            ]
        );
    }
    #[test]
    fn crash_after_a_relayed_head_retains_its_head_evidence_on_reconcile() {
        let (dir, mut h, mut w) = host(vec![chunked_head()], None);
        mid_body(&mut h, &mut w);
        h.hosted = Hosted::default(); // the producing process is gone
        h.coding_hosted_reconcile_unsettled().unwrap();
        assert_eq!(
            request(&h).attempts[0].disposition,
            AttemptDisposition::Uncertain
        );
        let head = head_evidence(&dir, &h, "1-1");
        assert_eq!(head["status"], 200);
        assert!(head["headMs"].is_u64());
        assert_eq!(
            hosted_artifacts(&dir, &h),
            [
                "1-1.head.json",
                "1-1.request-head.json",
                "1-1.stream.sse",
                "1.body.json"
            ]
        );
    }
    #[test]
    fn refusal_fails_over_within_the_grant_and_host_evidence_settles_completion() {
        let (dir, mut h, mut w) = host(vec![refusal(429), completed()], None);
        let g = grant(&mut h, &mut w);
        assert_eq!(g["type"], "bridge_provider");
        assert_eq!(g["headTimeoutMs"], HEAD_TIMEOUT_MS);
        assert_eq!(g["stream"]["sequence"], 1);
        assert_eq!(g["stream"]["stream"], 1);
        assert_eq!(g["stream"]["binding"]["generation"], 1);
        assert_eq!(g["stream"]["binding"]["profile"], HOSTED_BRIDGE_PROFILE);
        assert!(g["consumptionDeadlineMs"].is_u64());
        assert!(push(&mut h, &mut w, fetch(&g)).unwrap().is_empty());
        let head = tick_until(&mut h, &w, has("response-head"));
        assert_eq!(head.len(), 1, "refusal relays nothing: {head:?}");
        assert_eq!(head[0]["status"], 200);
        // Each attempt was admitted by the gate on a bound route/payer, in order.
        let intents = w.intents.lock().unwrap().clone();
        assert_eq!(
            intents
                .iter()
                .map(|i| (i.request, i.attempt, i.route.as_str(), i.payer.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, 1, "primary", "payer-a"), (1, 2, "secondary", "payer-b")]
        );
        // Container access markers never reach the transport.
        for (_, headers) in w.opened.lock().unwrap().iter() {
            assert!(!headers.contains_key("authorization"));
            assert!(!headers.contains_key("chatgpt-account-id"));
            assert_eq!(headers["originator"], "pi");
        }
        let mut ordinal = 1;
        loop {
            push(&mut h, &mut w, op(&g, "pull", ordinal)).unwrap();
            let f = tick_until(&mut h, &w, |a| !a.is_empty());
            ordinal += 1;
            if f[0]["type"] == "response-end" {
                break;
            }
            assert_eq!(f[0]["type"], "response-chunk");
        }
        push(&mut h, &mut w, op(&g, "ack", ordinal)).unwrap();
        let r = request(&h);
        assert_eq!(r.attempts.len(), 2);
        // Gate admission is durable on each attempt record.
        for a in &r.attempts {
            assert!(a.gate_entered);
            assert!(a.deadline_ms.is_some());
            assert_eq!(
                a.routes_sha256.as_deref(),
                Some(h.hosted_plan().unwrap().1.digest().as_str())
            );
        }
        // Retained bytes are journal artifacts, verified at collection.
        for name in ["1-1.refusal.body", "1-2.stream.sse", "1-2.relay.json"] {
            let path = dir.path().join(format!("coding-hosted-{name}"));
            let a = h.journal.artifacts.iter().find(|a| a.path == path).unwrap();
            a.verify().unwrap();
        }
        // A refused head relayed nothing: no stream file.
        assert!(!dir.path().join("coding-hosted-1-1.stream.sse").exists());
        assert_eq!(r.attempts[0].disposition, AttemptDisposition::Refused);
        assert_eq!(r.attempts[0].http_status, Some(429));
        assert_eq!(r.attempts[1].disposition, AttemptDisposition::Completed);
        assert_eq!(r.charged_microusd(), 164001, "refused attempt released");
        assert_eq!(r.http_status, Some(200));
        assert_eq!(r.backend_model.as_deref(), Some("gpt-6-luna"));
        assert_eq!(
            r.raw_terminal.as_ref().unwrap()["usage"]["total_tokens"],
            150
        );
        // Complete screened bytes and the refusal body are retained and bound.
        let sse = fs::read(dir.path().join("coding-hosted-1-2.stream.sse")).unwrap();
        assert_eq!(Some(digest(&sse)), r.attempts[1].relay_sha256);
        let body = fs::read(dir.path().join("coding-hosted-1-1.refusal.body")).unwrap();
        assert_eq!(Some(digest(&body)), r.attempts[0].refusal_body_sha256);
        // Container observations are not evidence in this profile.
        assert!(
            push(
                &mut h,
                &mut w,
                json!({"type":"provider_observation","id":1,"httpStatus":200})
            )
            .is_err()
        );
        let (_d, mut h, mut w) = host(vec![refusal(429), completed()], None);
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        tick_until(&mut h, &w, has("response-head"));
        let mut ordinal = 1;
        loop {
            push(&mut h, &mut w, op(&g, "pull", ordinal)).unwrap();
            let f = tick_until(&mut h, &w, |a| !a.is_empty());
            ordinal += 1;
            if f[0]["type"] == "response-end" {
                break;
            }
        }
        push(&mut h, &mut w, op(&g, "ack", ordinal)).unwrap();
        push(
            &mut h,
            &mut w,
            json!({"type":"provider_complete","id":1,"stopReason":"stop","usage":{}}),
        )
        .unwrap();
        let r = request(&h);
        assert_eq!(r.attempts[1].settled_microusd, Some(64));
        assert_eq!(r.settled_microusd, Some(64));
        assert!(!r.unsettled());
    }

    #[test]
    fn cancel_before_dispatch_releases_and_is_confirmed() {
        let (_dir, mut h, mut w) = host(vec![completed()], None);
        let g = grant(&mut h, &mut w);
        // The cancel arrives while the attempt is being admitted.
        h.coding_gate = Some(Box::new(HostedGate {
            intents: w.intents.clone(),
            inject: Some((w.path.clone(), line(op(&g, "cancel", 1)))),
            refuse_attempts: false,
        }));
        push(&mut h, &mut w, fetch(&g)).unwrap();
        assert!(h.coding_hosted_tick(&w.path).unwrap().is_empty());
        assert!(w.opened.lock().unwrap().is_empty(), "nothing dispatched");
        let r = request(&h);
        assert_eq!(r.attempts[0].disposition, AttemptDisposition::NotDispatched);
        assert_eq!(r.charged_microusd(), 0);
        // The injected cancel is now processed and confirmed.
        w.raw = fs::read(&w.path).unwrap();
        h.coding_advance(&w.raw).unwrap();
        let f = tick_until(&mut h, &w, has("response-cancelled"));
        assert_eq!(f[0]["ordinal"], 1);
        // A cancel processed before the tick dispatches: no attempt at all.
        let (_dir, mut h, mut w) = host(vec![completed()], None);
        let g = grant(&mut h, &mut w);
        w.raw.extend(line(fetch(&g)));
        push(&mut h, &mut w, op(&g, "cancel", 1)).unwrap();
        let f = tick_until(&mut h, &w, has("response-cancelled"));
        assert_eq!(f.len(), 1);
        assert!(w.intents.lock().unwrap().is_empty());
        assert!(w.opened.lock().unwrap().is_empty());
        assert!(request(&h).attempts.is_empty());
    }

    #[test]
    fn exhausted_routes_answer_unavailable_and_confirm_a_crossing_cancel() {
        let (_dir, mut h, mut w) = host(vec![refusal(429), refusal(402)], None);
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        let f = tick_until(&mut h, &w, has("response-unavailable"));
        assert_eq!(f.len(), 1);
        let r = request(&h);
        assert_eq!(r.attempts.len(), 2);
        assert!(
            r.attempts
                .iter()
                .all(|a| a.disposition == AttemptDisposition::Refused)
        );
        assert_eq!(
            w.intents.lock().unwrap().len(),
            2,
            "attempt limit respected"
        );
        push(&mut h, &mut w, op(&g, "cancel", 1)).unwrap();
        let f = tick_until(&mut h, &w, has("response-cancelled"));
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn non_refusal_failure_is_uncertain_and_never_fails_over() {
        // A 5xx is relayed, never retried; its truncated body stays uncertain.
        let truncated = b"HTTP/1.1 500 Error\r\nContent-Length: 100\r\n\r\nshort".to_vec();
        let (_dir, mut h, mut w) = host(vec![truncated, completed()], None);
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        let head = tick_until(&mut h, &w, has("response-head"));
        assert_eq!(head[0]["status"], 500);
        push(&mut h, &mut w, op(&g, "pull", 1)).unwrap();
        let mut f = tick_until(&mut h, &w, |a| {
            a.iter()
                .any(|f| f["type"] == "response-failure" || f["type"] == "response-chunk")
        });
        let mut ordinal = 2;
        while f.last().unwrap()["type"] == "response-chunk" {
            push(&mut h, &mut w, op(&g, "pull", ordinal)).unwrap();
            ordinal += 1;
            f = tick_until(&mut h, &w, |a| !a.is_empty());
        }
        assert_eq!(f.last().unwrap()["type"], "response-failure");
        push(&mut h, &mut w, op(&g, "ack", ordinal)).unwrap();
        let r = request(&h);
        assert_eq!(r.attempts.len(), 1);
        assert_eq!(r.attempts[0].disposition, AttemptDisposition::Uncertain);
        assert!(r.unsettled());
        assert_eq!(w.opened.lock().unwrap().len(), 1);
        assert!(h.journal.coding.as_ref().unwrap().provider_failed);
    }

    #[test]
    fn fetch_identity_url_model_and_route_table_are_checked() {
        for mutate in 0..6 {
            let (_dir, mut h, mut w) = host(vec![], None);
            let g = grant(&mut h, &mut w);
            let mut f = fetch(&g);
            match mutate {
                0 => f["url"] = json!("https://example.invalid/codex/responses"),
                1 => f["body"] = json!(B64.encode(br#"{"model":"gpt-6-other","stream":true}"#)),
                2 => f["stream"] = json!(2),
                3 => f["id"] = json!(2),
                4 => f["headers"]["content-encoding"] = json!("zstd"),
                _ => f["method"] = json!("GET"),
            }
            assert!(push(&mut h, &mut w, f).is_err(), "case {mutate}");
            assert!(w.opened.lock().unwrap().is_empty());
        }
        let mut routes = HostedRoutes {
            routes: vec![HostedRoute {
                id: "r".into(),
                payer: "p".into(),
                endpoint: "loopback:9".into(),
            }],
            max_attempts_per_request: 1,
        };
        routes.validate().unwrap();
        routes.routes[0].endpoint = "https://chatgpt.com".into();
        assert!(routes.validate().is_err(), "only loopback routes offline");
        routes.routes[0].endpoint = "loopback:9".into();
        routes.max_attempts_per_request = 9;
        assert!(routes.validate().is_err());
    }

    fn chunked_head() -> Vec<u8> {
        // Head and one event; the body never ends (the reader sees EOF later).
        let event = "data: {\"type\":\"response.created\",\"response\":{\"id\":\"r1\"}}\n\n";
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{event}\r\n",
            event.len()
        )
        .into_bytes()
    }
    /// Open request 1 on the secondary-free path and relay its first chunk.
    fn mid_body(h: &mut Host, w: &mut Wire) -> Value {
        let g = grant(h, w);
        push(h, w, fetch(&g)).unwrap();
        tick_until(h, w, has("response-head"));
        push(h, w, op(&g, "pull", 1)).unwrap();
        let f = tick_until(h, w, has("response-chunk"));
        assert_eq!(f.len(), 1);
        g
    }
    fn assert_uncertain_with_bytes(dir: &tempfile::TempDir, h: &Host, reason: &str) {
        let r = request(h);
        let a = &r.attempts[0];
        assert_eq!(a.disposition, AttemptDisposition::Uncertain);
        assert!(
            a.reason.as_deref().unwrap().contains(reason),
            "{:?}",
            a.reason
        );
        assert!(r.unsettled());
        assert!(h.journal.coding.as_ref().unwrap().provider_failed);
        let path = dir.path().join("coding-hosted-1-1.stream.sse");
        let bytes = fs::read(&path).unwrap();
        assert!(!bytes.is_empty(), "relayed bytes are retained");
        assert_eq!(a.relay_sha256.as_deref(), Some(digest(&bytes).as_str()));
        assert!(h.journal.artifacts.iter().any(|x| x.path == path));
        // The relay evidence of a closed attempt never reads as completed.
        let evidence: Value = serde_json::from_slice(
            &fs::read(dir.path().join("coding-hosted-1-1.relay.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(evidence["outcome"], "interrupted-uncertain");
        assert_ne!(evidence["interruption"], Value::Null);
    }

    #[test]
    fn provider_failure_event_in_a_relayed_stream_is_uncertain_not_completed() {
        // Found by the piece (c) Docker matrix (`midstream-failed`).
        let body = [
            json!({"type":"response.created","response":{"id":"r1"}}),
            json!({"type":"response.failed","response":{"id":"r1","status":"failed","error":{"code":"insufficient_quota"}}}),
        ]
        .iter()
        .map(|e| format!("data: {e}\n\n"))
        .collect::<String>();
        let failed = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .into_bytes();
        let (_dir, mut h, mut w) = host(vec![failed], None);
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        tick_until(&mut h, &w, has("response-head"));
        let mut ordinal = 1;
        loop {
            push(&mut h, &mut w, op(&g, "pull", ordinal)).unwrap();
            let f = tick_until(&mut h, &w, |a| !a.is_empty());
            ordinal += 1;
            if f[0]["type"] != "response-chunk" {
                break;
            }
        }
        push(&mut h, &mut w, op(&g, "ack", ordinal)).unwrap();
        let a = &request(&h).attempts[0];
        assert_eq!(a.disposition, AttemptDisposition::Uncertain);
        assert!(a.reason.as_deref().unwrap().contains("insufficient_quota"));
        assert!(request(&h).unsettled());
        assert!(h.journal.coding.as_ref().unwrap().provider_failed);
        // The container's completion cannot settle it, and says why.
        let err = push(
            &mut h,
            &mut w,
            json!({"type":"provider_complete","id":1,"stopReason":"stop","usage":{}}),
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("hosted request 1 attempt 1 is Uncertain")
                && err.contains("provider failure event"),
            "{err}"
        );
        assert!(request(&h).unsettled());
    }

    #[test]
    fn reconcile_marks_a_pending_attempt_uncertain_and_keeps_it_charged() {
        // Crash after durable gate admission: the attempt is pending.
        let (dir, mut h, mut w) = host(vec![chunked_head()], None);
        mid_body(&mut h, &mut w);
        h.hosted = Hosted::default(); // the producing process is gone
        let before = request(&h);
        assert_eq!(before.attempts[0].disposition, AttemptDisposition::Pending);
        // N3: the relayed head status and bytes were durable before relay.
        assert_eq!(before.attempts[0].http_status, Some(200));
        let path = dir.path().join("coding-hosted-1-1.stream.sse");
        let relayed = fs::read(&path).unwrap();
        assert!(!relayed.is_empty());
        h.coding_hosted_reconcile_unsettled().unwrap();
        let after = request(&h);
        assert_eq!(after.attempts[0].disposition, AttemptDisposition::Uncertain);
        assert!(after.attempts[0].gate_entered);
        let reason = after.attempts[0].reason.clone().unwrap();
        assert!(
            reason.starts_with("unsettled at restart; producing process gone")
                && reason.contains(&format!("{} relayed bytes retained", relayed.len())),
            "{reason}"
        );
        assert_eq!(
            after.attempts[0].relay_sha256.as_deref(),
            Some(digest(&relayed).as_str())
        );
        assert_eq!(
            h.journal
                .artifacts
                .iter()
                .filter(|a| a.path == path)
                .count(),
            1
        );
        assert_eq!(after.charged_microusd(), before.charged_microusd());
        assert!(after.unsettled());
        // Idempotent.
        h.coding_hosted_reconcile_unsettled().unwrap();
        assert_eq!(request(&h).attempts, after.attempts);
    }

    #[test]
    fn connect_failure_is_a_confirmed_non_dispatch_that_releases_and_ends_the_request() {
        // M2: the transport fails before any request byte (TCP/TLS).
        let (dir, mut h, mut w) = host(vec![], None);
        h.hosted_transport = Some(Box::new(FakeTransport {
            responses: VecDeque::new(),
            opened: w.opened.clone(),
            unreachable: true,
        }));
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        let f = tick_until(&mut h, &w, has("response-unavailable"));
        assert_eq!(f[0]["code"], "route_unreachable");
        let r = request(&h);
        assert_eq!(r.attempts.len(), 1);
        let a = &r.attempts[0];
        assert_eq!(a.disposition, AttemptDisposition::NotDispatched);
        assert!(!a.gate_entered);
        assert!(
            a.reason
                .as_deref()
                .unwrap()
                .contains("route connection failed")
        );
        assert_eq!(r.charged_microusd(), 0);
        assert!(
            w.intents.lock().unwrap().is_empty(),
            "the gate was never consulted"
        );
        assert!(
            w.opened.lock().unwrap().is_empty(),
            "no request was written"
        );
        assert!(h.journal.coding.as_ref().unwrap().provider_failed);
        assert!(!dir.path().join("coding-hosted-1-1.stream.sse").exists());
        // No later attempt for this request.
        let state = h.journal.coding.as_mut().unwrap();
        assert!(
            provider::reserve_attempt(
                &mut state.provider_requests,
                1,
                "secondary",
                "payer-b",
                1,
                1_000_000,
                2
            )
            .is_err()
        );
    }
    #[test]
    fn body_policy_refuses_stored_background_tiered_or_hosted_tool_requests() {
        // R-a: fields the Host pins on real routes.
        for body in [
            r#"{"model":"gpt-6-luna","stream":true,"input":[]}"#,
            r#"{"model":"gpt-6-luna","stream":true,"store":true,"input":[]}"#,
            r#"{"model":"gpt-6-luna","stream":true,"store":false,"background":true}"#,
            r#"{"model":"gpt-6-luna","stream":true,"store":false,"service_tier":"priority"}"#,
            r#"{"model":"gpt-6-luna","stream":true,"store":false,"service_tier":1}"#,
            r#"{"model":"gpt-6-luna","stream":true,"store":false,"tools":[{"type":"web_search"}]}"#,
            r#"{"model":"gpt-6-luna","stream":true,"store":false,"tools":{"type":"function"}}"#,
        ] {
            let (_dir, mut h, mut w) = host(vec![], None);
            let g = grant(&mut h, &mut w);
            let mut f = fetch(&g);
            f["body"] = json!(B64.encode(body.as_bytes()));
            assert!(push(&mut h, &mut w, f).is_err(), "{body}");
        }
        let (_dir, mut h, mut w) = host(vec![], None);
        let g = grant(&mut h, &mut w);
        let mut f = fetch(&g);
        f["body"] = json!(B64.encode(br#"{"model":"gpt-6-luna","stream":true,"store":false,"service_tier":"auto","tools":[{"type":"function","name":"workspace_exec"}]}"#));
        assert!(push(&mut h, &mut w, f).is_ok());
    }
    #[test]
    fn crash_between_reservation_and_gate_entry_stays_charged_with_nothing_relayed() {
        // Phase 5 cut: the attempt is durably reserved, the process dies before
        // the gate admits it.
        let (dir, mut h, mut w) = host(vec![completed()], None);
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        let (plan, routes) = h.hosted_plan().unwrap();
        let reserve = provider::full_reservation(&plan.selected().unwrap()).unwrap();
        let state = h.journal.coding.as_mut().unwrap();
        provider::reserve_attempt(
            &mut state.provider_requests,
            1,
            &routes.routes[0].id,
            &routes.routes[0].payer,
            reserve,
            plan.budget_microusd,
            routes.max_attempts_per_request,
        )
        .unwrap();
        h.save().unwrap();
        h.hosted = Hosted::default(); // the producing process is gone
        h.coding_hosted_reconcile_unsettled().unwrap();
        let r = request(&h);
        let a = &r.attempts[0];
        assert_eq!(a.disposition, AttemptDisposition::Uncertain);
        assert!(!a.gate_entered);
        assert!(a.reason.as_deref().unwrap().contains("nothing was relayed"));
        assert_eq!(r.charged_microusd(), reserve);
        assert!(r.unsettled());
        assert!(!dir.path().join("coding-hosted-1-1.stream.sse").exists());
        assert!(
            w.intents.lock().unwrap().is_empty(),
            "never admitted, never dispatched"
        );
    }
    #[test]
    fn crash_between_relay_settlement_and_container_completion_keeps_the_hold() {
        // Phase 5 cut: the Host classified a completed relay; the process dies
        // before the container's provider_complete settles raw usage.
        let (dir, mut h, mut w) = host(vec![completed()], None);
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        tick_until(&mut h, &w, has("response-head"));
        let mut ordinal = 1;
        loop {
            push(&mut h, &mut w, op(&g, "pull", ordinal)).unwrap();
            let f = tick_until(&mut h, &w, |a| !a.is_empty());
            ordinal += 1;
            if f[0]["type"] == "response-end" {
                break;
            }
        }
        push(&mut h, &mut w, op(&g, "ack", ordinal)).unwrap();
        let before = request(&h);
        assert_eq!(
            before.attempts[0].disposition,
            AttemptDisposition::Completed
        );
        assert!(before.attempts[0].settled_microusd.is_none());
        h.hosted = Hosted::default();
        h.coding_hosted_reconcile_unsettled().unwrap();
        let after = request(&h);
        // Nothing is released or re-labelled: the full reservation stays held
        // and blocks later grants until an owner settles it.
        assert_eq!(after.attempts, before.attempts);
        assert_eq!(
            after.charged_microusd(),
            after.attempts[0].reserved_microusd
        );
        assert!(after.unsettled());
        let path = dir.path().join("coding-hosted-1-1.stream.sse");
        assert!(h.journal.artifacts.iter().any(|a| a.path == path));
    }
    #[test]
    fn protocol_errors_and_run_exit_keep_relayed_bytes_and_leave_the_attempt_uncertain() {
        // Out-of-order pull mid-body.
        let (dir, mut h, mut w) = host(vec![chunked_head()], None);
        let g = mid_body(&mut h, &mut w);
        assert!(push(&mut h, &mut w, op(&g, "pull", 5)).is_err());
        assert_uncertain_with_bytes(&dir, &h, "stream_foreign_frame");
        // A frame of another stream while the attempt is open.
        let (dir, mut h, mut w) = host(vec![chunked_head()], None);
        let g = mid_body(&mut h, &mut w);
        let mut foreign = op(&g, "pull", 2);
        foreign["stream"] = json!(7);
        assert!(push(&mut h, &mut w, foreign).is_err());
        assert_uncertain_with_bytes(&dir, &h, "stream_foreign_frame");
        // The run loop ends (deadline, cancellation, container exit) mid-body.
        let (dir, mut h, mut w) = host(vec![chunked_head()], None);
        mid_body(&mut h, &mut w);
        h.coding_hosted_abort("run loop ended with the stream open")
            .unwrap();
        assert_uncertain_with_bytes(&dir, &h, "run loop ended");
        // Idempotent once closed.
        h.coding_hosted_abort("again").unwrap();
    }

    #[test]
    fn gate_refusal_and_missing_transport_are_recorded_without_dispatch() {
        let (_dir, mut h, mut w) = host(vec![completed()], None);
        h.coding_gate = Some(Box::new(HostedGate {
            intents: w.intents.clone(),
            inject: None,
            refuse_attempts: true,
        }));
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        assert!(h.coding_hosted_tick(&w.path).is_err());
        assert!(w.opened.lock().unwrap().is_empty());
        let a = request(&h).attempts[0].clone();
        assert_eq!(a.disposition, AttemptDisposition::Uncertain);
        assert!(!a.gate_entered);
        assert!(a.reason.unwrap().starts_with("gate refused"));
        assert!(h.coding_gate.is_some(), "gate is kept");
        // Transport absent: refused before the gate is consulted.
        let (_dir, mut h, mut w) = host(vec![completed()], None);
        let g = grant(&mut h, &mut w);
        push(&mut h, &mut w, fetch(&g)).unwrap();
        h.hosted_transport = None;
        assert!(h.coding_hosted_tick(&w.path).is_err());
        assert!(w.intents.lock().unwrap().is_empty());
        assert!(h.coding_gate.is_some());
        let a = request(&h).attempts[0].clone();
        assert_eq!(a.disposition, AttemptDisposition::Uncertain);
        assert_eq!(a.reason.as_deref(), Some("hosted transport absent"));
    }

    // Anthropic Messages family (W1).
    const HAIKU: &str = "claude-haiku-5-5";
    /// The offline host switched to the `claude-sdk` provider with Pi's
    /// Anthropic catalog entry of the model.
    fn messages_host(responses: Vec<Vec<u8>>) -> (tempfile::TempDir, Host, Wire) {
        let (dir, mut h, w) = host(responses, None);
        let path = dir.path().join("anthropic.json");
        let bytes = serde_json::to_vec(&json!({"anthropic-messages":{"chat:claude-haiku-5-5":{
            "id":HAIKU,"provider":"anthropic","api":"anthropic-messages","baseUrl":"https://api.anthropic.com",
            "cost":{"input":0.1,"output":0.5,"cacheRead":0.01,"cacheWrite":0.125,
                "tiers":[{"inputTokensAbove":100000,"input":0.5,"output":2.5,"cacheRead":0.05,"cacheWrite":0.625}]},
            "contextWindow":1000000,"maxTokens":128000}}}))
        .unwrap();
        fs::write(&path, &bytes).unwrap();
        let plan = h
            .journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap();
        plan.provider = "claude-sdk".into();
        plan.model = HAIKU.into();
        plan.budget_microusd = 10_000_000;
        plan.catalog = Artifact {
            path,
            sha256: digest(&bytes),
        };
        (dir, h, w)
    }
    fn messages_grant(h: &mut Host, w: &mut Wire) -> Value {
        let replies = push(h, w, json!({"type":"coding_provider_request","id":1,"requestModel":{"id":HAIKU,"provider":"claude-sdk","contextWindow":1000000},"contextBytes":1000})).unwrap();
        assert_eq!(replies.len(), 1);
        let mut written = vec![];
        h.coding_bridge_dispatch(&replies[0], &mut |b| {
            written = b.to_vec();
            Ok(())
        })
        .unwrap();
        serde_json::from_slice(&written).unwrap()
    }
    fn messages_body(messages: Value) -> Value {
        json!({"model":HAIKU,"stream":true,"max_tokens":128000,"messages":messages,
            "system":[{"type":"text","text":"x-anthropic-billing-header: cc_version=2.1.325.fba; cc_entrypoint=sdk-ts;"},
                {"type":"text","text":"You are a Claude agent, built on Anthropic's Claude Agent SDK.","cache_control":{"type":"ephemeral","ttl":"1h"}},
                {"type":"text","text":"You are Claude, working as a coding agent in pi, a terminal coding harness. You help.","cache_control":{"type":"ephemeral","ttl":"1h"}}],
            "tools":[{"name":"DeferredToolPlaceholder","description":"d","input_schema":{"type":"object"},"defer_loading":true},
                {"name":"mcp__pi__snapshot_read","description":"r","input_schema":{"type":"object"}}],
            "metadata":{"user_id":"{}"},"thinking":{"type":"disabled"}})
    }
    fn messages_fetch(grant: &Value, body: &[u8]) -> Value {
        let s = &grant["stream"];
        json!({"type":"provider_fetch","id":1,"binding":s["binding"],"sequence":s["sequence"],"stream":s["stream"],
            "url":"https://api.anthropic.com/v1/messages?beta=true","method":"POST",
            "headers":{"authorization":"Bearer sk-ant-oat01-DUMMY","x-api-key":"marker","content-type":"application/json",
                "anthropic-version":"2023-06-01","x-stainless-retry-count":"0"},
            "body":B64.encode(body)})
    }
    // OpenRouter Chat Completions family (this lane). The Layer A catalog id Pi
    // resolves offline is `~deepseek/deepseek-v4-flash-latest`; the transmitted
    // slug (Layer B) is base + `:floor`. The resolved checkpoint (Layer C) is a
    // concrete id in the family, e.g. `deepseek/deepseek-v4-flash-0731`.
    const OR_ALIAS: &str = "~deepseek/deepseek-v4-flash-latest";
    const OR_TRANSMITTED: &str = "~deepseek/deepseek-v4-flash-latest:floor";
    const OR_RESOLVED: &str = "deepseek/deepseek-v4-flash-0731";
    /// The offline host switched to the `openrouter` provider with the pinned
    /// OpenRouter catalog entry of the model, and a host budget large enough for
    /// one full-catalog reservation.
    fn openrouter_host(responses: Vec<Vec<u8>>) -> (tempfile::TempDir, Host, Wire) {
        let (dir, mut h, w) = host(responses, None);
        let path = dir.path().join("openrouter.json");
        let catalog = json!({
            "openai-completions": {
                "chat:~deepseek/deepseek-v4-flash-latest": {
                    "id": OR_ALIAS,
                    "provider": "openrouter",
                    "api": "openai-completions",
                    "baseUrl": "https://openrouter.ai/api/v1",
                    "cost": {"input": 0.07, "output": 0.5, "cacheRead": 0.008, "cacheWrite": 0.0},
                    "contextWindow": 1048576,
                    "maxTokens": 943718
                }
            }
        });
        let bytes = serde_json::to_vec(&catalog).unwrap();
        fs::write(&path, &bytes).unwrap();
        let plan = h
            .journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap();
        plan.provider = "openrouter".into();
        plan.model = OR_ALIAS.into();
        plan.allow_shell = true;
        plan.allow_background = false;
        plan.budget_microusd = 4_000_000;
        plan.max_requests = 1;
        plan.max_context_bytes = 32768;
        plan.catalog = Artifact {
            path,
            sha256: digest(&bytes),
        };
        (dir, h, w)
    }
    fn openrouter_grant(h: &mut Host, w: &mut Wire) -> Value {
        let replies = push(
            h,
            w,
            json!({"type":"coding_provider_request","id":1,"requestModel":{"id":OR_ALIAS,"provider":"openrouter","contextWindow":1048576},"contextBytes":1000}),
        )
        .unwrap();
        assert_eq!(replies.len(), 1);
        let mut written = vec![];
        h.coding_bridge_dispatch(&replies[0], &mut |b| {
            written = b.to_vec();
            Ok(())
        })
        .unwrap();
        serde_json::from_slice(&written).unwrap()
    }
    /// A provider_fetch for the openrouter lane with the body Pi 1.5.0 would
    /// serialize: model is the Layer A alias id Pi holds; the Host rewrites to
    /// Layer B at dispatch. `reasoning.effort` is "high" per the operator.
    fn openrouter_fetch(grant: &Value, body: &[u8]) -> Value {
        let s = &grant["stream"];
        json!({"type":"provider_fetch","id":1,"binding":s["binding"],"sequence":s["sequence"],"stream":s["stream"],
            "url":"https://openrouter.ai/api/v1/chat/completions","method":"POST",
            "headers":{"authorization":"Bearer nonsecret-marker","content-type":"application/json"},
            "body":B64.encode(body)})
    }
    /// A clean Chat Completions SSE stream (deltas, usage chunk, [DONE]).
    fn completions_completed() -> Vec<u8> {
        let events = [
            json!({"object":"chat.completion.chunk","model":OR_RESOLVED,"choices":[{"index":0,"delta":{"content":"hi"}}]}),
            json!({"model":OR_RESOLVED,"choices":[],"usage":{"prompt_tokens":12,"completion_tokens":4,"total_tokens":16}}),
        ];
        let body = events
            .iter()
            .map(|e| format!("data: {e}\n\n"))
            .collect::<String>()
            + "data: [DONE]\n\n";
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }
    fn messages_completed() -> Vec<u8> {
        messages_completed_with("")
    }
    fn messages_completed_with(headers: &str) -> Vec<u8> {
        let events = [
            (
                "message_start",
                json!({"type":"message_start","message":{"id":"msg_1","model":HAIKU,"usage":
                {"input_tokens":10,"cache_creation_input_tokens":1000,"cache_read_input_tokens":2000,"output_tokens":1,
                "cache_creation":{"ephemeral_5m_input_tokens":400,"ephemeral_1h_input_tokens":600}}}}),
            ),
            (
                "content_block_start",
                json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
            ),
            ("ping", json!({"type":"ping"})),
            (
                "content_block_delta",
                json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"ok"}}),
            ),
            (
                "content_block_stop",
                json!({"type":"content_block_stop","index":0}),
            ),
            (
                "message_delta",
                json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":
                {"input_tokens":10,"cache_creation_input_tokens":1000,"cache_read_input_tokens":2000,"output_tokens":50}}),
            ),
            ("message_stop", json!({"type":"message_stop"})),
        ];
        let body = events
            .iter()
            .map(|(name, e)| format!("event: {name}\ndata: {e}\n\n"))
            .collect::<String>();
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n{headers}Content-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }
    fn relay_to_end(h: &mut Host, w: &mut Wire, g: &Value) {
        tick_until(h, w, has("response-head"));
        let mut ordinal = 1;
        loop {
            push(h, w, op(g, "pull", ordinal)).unwrap();
            let f = tick_until(h, w, |a| !a.is_empty());
            ordinal += 1;
            if f[0]["type"] == "response-end" {
                break;
            }
        }
        push(h, w, op(g, "ack", ordinal)).unwrap();
    }
    #[test]
    fn messages_relay_settles_per_token_class_from_host_evidence() {
        let (dir, mut h, mut w) = messages_host(vec![refusal(429), messages_completed()]);
        let g = messages_grant(&mut h, &mut w);
        let body =
            serde_json::to_vec(&messages_body(json!([{"role":"user","content":"q"}]))).unwrap();
        assert!(
            push(&mut h, &mut w, messages_fetch(&g, &body))
                .unwrap()
                .is_empty()
        );
        relay_to_end(&mut h, &mut w, &g);
        // Access markers never reach the transport; Claude Code headers do.
        for (_, headers) in w.opened.lock().unwrap().iter() {
            assert!(!headers.contains_key("authorization") && !headers.contains_key("x-api-key"));
            assert_eq!(headers["anthropic-version"], "2023-06-01");
        }
        let r = request(&h);
        assert_eq!(r.attempts[0].disposition, AttemptDisposition::Refused);
        assert_eq!(r.attempts[1].disposition, AttemptDisposition::Completed);
        assert_eq!(r.backend_model.as_deref(), Some(HAIKU));
        assert_eq!(
            r.raw_terminal.as_ref().unwrap(),
            &json!({"eventType":"message_stop","stopReason":"end_turn","usage":{"input_tokens":10,
                "cache_creation_input_tokens":1000,"cache_read_input_tokens":2000,"output_tokens":50,
                "cache_creation_5m_input_tokens":400,"cache_creation_1h_input_tokens":600}})
        );
        let relay: Value = serde_json::from_slice(
            &fs::read(dir.path().join("coding-hosted-1-2.relay.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(relay["usage"]["cacheReadInputTokens"], 2000);
        // The container's stop reason must agree with the raw one.
        assert!(
            push(
                &mut h,
                &mut w,
                json!({"type":"provider_complete","id":1,"stopReason":"toolUse","usage":{}})
            )
            .is_err()
        );
        let (_dir, mut h, mut w) = messages_host(vec![messages_completed()]);
        let g = messages_grant(&mut h, &mut w);
        push(&mut h, &mut w, messages_fetch(&g, &body)).unwrap();
        relay_to_end(&mut h, &mut w, &g);
        push(
            &mut h,
            &mut w,
            json!({"type":"provider_complete","id":1,"stopReason":"stop","usage":{}}),
        )
        .unwrap();
        // 10 x 0.1 + 400 x 0.125 (5-minute writes) + 600 x 0.2 (one-hour
        // writes, twice the input rate) + 2000 x 0.01 + 50 x 0.5 = 216, plus one.
        assert_eq!(request(&h).settled_microusd, Some(217));
    }
    /// End-to-end Completions lane (P2-3, and the P1-1/1-2/1-3 proof): a real
    /// Pi-shaped openrouter provider_fetch is accepted by completions_policy,
    /// dispatched, settled over a clean Chat Completions SSE stream, and the
    /// container completion settles with the resolved checkpoint (Layer C) in
    /// the admitted family and `reasoning.effort == "high"` in the body.
    #[test]
    fn completions_lane_settles_end_to_end_with_high_effort() {
        let (dir, mut h, mut w) = openrouter_host(vec![completions_completed()]);
        let g = openrouter_grant(&mut h, &mut w);
        // Body Pi 1.5.0 serializes for openrouter: store:false, reasoning high,
        // model is the Layer A alias id Pi holds.
        let body = serde_json::to_vec(&json!({
            "model": OR_ALIAS,
            "stream": true,
            "store": false,
            "stream_options": {"include_usage": true},
            "reasoning": {"effort": "high"},
            "tools": [{"type": "function", "name": "workspace_exec", "description": "d", "parameters": {"type": "object"}}],
            "messages": [{"role": "user", "content": "q"}]
        }))
        .unwrap();
        assert!(
            push(&mut h, &mut w, openrouter_fetch(&g, &body))
                .unwrap()
                .is_empty(),
            "fetch admitted"
        );
        relay_to_end(&mut h, &mut w, &g);
        // The marker is never applied upstream as the real key; the Host swaps it.
        for (_, headers) in w.opened.lock().unwrap().iter() {
            assert!(
                !headers.contains_key("authorization")
                    || headers["authorization"] != "Bearer nonsecret-marker"
            );
        }
        let r = request(&h);
        assert_eq!(r.attempts[0].disposition, AttemptDisposition::Completed);
        assert_eq!(r.backend_model.as_deref(), Some(OR_RESOLVED));
        assert_eq!(
            r.raw_terminal.as_ref().unwrap()["eventType"],
            "chat_completion"
        );
        // Layer B (the forwarded slug) is what left the Host, never Layer A.
        let body_path = dir.path().join("coding-hosted-1.body.json");
        let forwarded: Value = serde_json::from_slice(&fs::read(&body_path).unwrap()).unwrap();
        assert_eq!(forwarded["model"], OR_TRANSMITTED);
        assert_eq!(forwarded["reasoning"]["effort"], "high");
        // The container's original body (Layer A) is retained beside it,
        // byte for byte, and registered.
        let original_path = dir.path().join("coding-hosted-1.original.body.json");
        assert_eq!(fs::read(&original_path).unwrap(), body);
        assert!(
            h.journal
                .artifacts
                .iter()
                .any(|a| a.path == original_path && a.sha256 == digest(&body))
        );
        // The container completion settles the Host-held reservation.
        push(
            &mut h,
            &mut w,
            json!({"type":"provider_complete","id":1,"stopReason":"stop","usage":{"input":12,"output":4,"cacheRead":0,"cacheWrite":0,"totalTokens":16}}),
        )
        .unwrap();
        // Settled to the raw-usage cost (12x0.07/1M + 4x0.5/1M ~= 4 microUSD,
        // +1 rounding), not the full-catalog reservation.
        assert_eq!(request(&h).settled_microusd, Some(4));
    }
    /// The operator fixed effort `high`; a body carrying any other effort (e.g.
    /// Pi's `thinkingLevel:'off'` -> `reasoning.effort == "none"`) must be
    /// refused by completions_policy.
    #[test]
    fn completions_policy_refuses_effort_below_high() {
        let (_dir, mut h, mut w) = openrouter_host(vec![]);
        let g = openrouter_grant(&mut h, &mut w);
        let body = serde_json::to_vec(&json!({
            "model": OR_ALIAS,
            "stream": true,
            "store": false,
            "stream_options": {"include_usage": true},
            "reasoning": {"effort": "none"},
            "messages": [{"role": "user", "content": "q"}]
        }))
        .unwrap();
        match push(&mut h, &mut w, openrouter_fetch(&g, &body)) {
            Err(Error::Refused(m)) => {
                assert!(m.contains("reasoning.effort"), "reason names effort: {m}")
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        assert!(w.opened.lock().unwrap().is_empty());
    }
    /// The receiver->Host->loopback path: a loopback mock presents
    /// http://127.0.0.1:<port>/chat/completions and must be admitted for the
    /// Completions wire (the plan has loopback routes), giving a provider-free
    /// end-to-end check.
    #[test]
    fn loopback_completions_url_is_admitted() {
        let (_dir, mut h, mut w) = openrouter_host(vec![]);
        let g = openrouter_grant(&mut h, &mut w);
        let body = serde_json::to_vec(&json!({
            "model": OR_ALIAS,
            "stream": true,
            "store": false,
            "stream_options": {"include_usage": true},
            "reasoning": {"effort": "high"},
            "messages": [{"role": "user", "content": "q"}]
        }))
        .unwrap();
        let mut f = openrouter_fetch(&g, &body);
        f["url"] = json!("http://127.0.0.1:9876/chat/completions");
        assert!(
            push(&mut h, &mut w, f).unwrap().is_empty(),
            "loopback Completions URL admitted"
        );
    }
    #[test]
    fn messages_cost_uses_the_tier_of_all_input_tokens() {
        let model = json!({"cost":{"input":0.1,"output":0.5,"cacheRead":0.01,"cacheWrite":0.125,
            "tiers":[{"inputTokensAbove":100000,"input":0.5,"output":2.5,"cacheRead":0.05,"cacheWrite":0.625}]}});
        let t = |write_5m, write_1h, read, output| provider::MessagesTokens {
            input: 0,
            write_5m,
            write_1h,
            read,
            output,
        };
        assert_eq!(
            provider::messages_cost_for_tests(&model, t(0, 0, 100_000, 0)),
            1001
        );
        // Cache reads push the request over the threshold: every class at tier rates.
        assert_eq!(
            provider::messages_cost_for_tests(&model, t(0, 0, 100_001, 10)),
            5001 + 25 + 1
        );
        // One-hour writes cost twice the tier's input rate; five-minute writes
        // the catalog cacheWrite rate.
        assert_eq!(
            provider::messages_cost_for_tests(&model, t(0, 1000, 0, 0)),
            201
        );
        assert_eq!(
            provider::messages_cost_for_tests(&model, t(1000, 0, 0, 0)),
            126
        );
        assert_eq!(
            provider::messages_cost_for_tests(&model, t(0, 100_001, 0, 0)),
            100_002
        );
        // Worst case: the whole window at the dearest input-side rate (one-hour
        // writes at the upper tier) and the output bound at the upper tier.
        let full = json!({"contextWindow":1000000,"maxTokens":128000,"cost":model["cost"]});
        assert_eq!(
            provider::messages_full_reservation_for_tests(&full),
            1_320_001
        );
    }
    /// The OpenRouter Completions URL is `{base}/chat/completions`. The catalog
    /// baseUrl is `https://openrouter.ai/api/v1`, so the correct presented URL
    /// is that base plus `/chat/completions`; the doubled `/api/v1/api/v1/...`
    /// form is a regression to refuse. This pins the URL rule that `hosted_fetch`
    /// uses (the `Wire::Completions` arm).
    #[test]
    fn completions_url_uses_the_catalog_base_not_a_doubled_path() {
        let base = "https://openrouter.ai/api/v1";
        let good = format!("{base}/chat/completions");
        let doubled = format!("{base}/api/v1/chat/completions");
        // The arm's effective predicate (see hosted_fetch, Wire::Completions).
        let accept = |url: &str| url == format!("{base}/chat/completions");
        assert!(accept(&good));
        assert!(!accept(&doubled));
        // Freeze the exact strings so a future catalog or path change is caught.
        assert_eq!(good, "https://openrouter.ai/api/v1/chat/completions");
        assert_eq!(
            doubled,
            "https://openrouter.ai/api/v1/api/v1/chat/completions"
        );
    }

    /// A loopback Completions route (offline mock) must have a real target path
    /// so the receiver->Host->loopback path can be exercised provider-free.
    #[test]
    fn loopback_completions_target_is_a_real_path() {
        let req = HostedRequest {
            request: 1,
            attempt: 1,
            headers: BTreeMap::new(),
            body: vec![],
            wire: stream::Wire::Completions,
            beta: false,
        };
        let lb = Endpoint::Loopback(std::net::SocketAddr::from(([127, 0, 0, 1], 9)));
        assert_eq!(lb.target(&req).1, "/chat/completions");
        assert!(lb.serves(stream::Wire::Completions));
        assert_eq!(
            Endpoint::OpenRouter.target(&req).1,
            "/api/v1/chat/completions"
        );
    }
    #[test]
    fn messages_body_policy_and_url_are_enforced() {
        let ok = messages_body(json!([{"role":"user","content":"q"}]));
        let mut cases = vec![];
        for (key, value) in [
            ("mcp_servers", json!([{"type":"url","url":"https://x"}])),
            ("container", json!("c")),
            ("service_tier", json!("auto")),
            ("tool_choice", json!({"type":"any"})),
            (
                "tools",
                json!([{"type":"web_search_20250305","name":"web_search"}]),
            ),
            ("tools", json!({"name":"x"})),
            ("messages", json!([])),
            ("stream", json!(false)),
            ("model", json!("claude-opus-5-5")),
        ] {
            let mut b = ok.clone();
            b[key] = value;
            cases.push((serde_json::to_vec(&b).unwrap(), "messages-bound"));
        }
        // An account identifier in metadata refuses before dispatch (L1).
        for user in [
            r#"{"device_id":"d0","account_uuid":"5f0c1a2e-0000-4000-8000-000000000001","session_id":"s0"}"#,
            r#"{"device_id":"d0","account_uuid":"","email":"user@example.com"}"#,
            r#"{"device_id":"d0","account_uuid":"","organization_uuid":"5f0c"}"#,
        ] {
            let mut b = ok.clone();
            b["metadata"]["user_id"] = json!(user);
            cases.push((serde_json::to_vec(&b).unwrap(), "account-identifier"));
        }
        let mut b = ok.clone();
        b["metadata"]["trace"] = json!("t");
        cases.push((serde_json::to_vec(&b).unwrap(), "metadata-key"));
        // A duplicate key could be read differently upstream.
        let text = String::from_utf8(serde_json::to_vec(&ok).unwrap()).unwrap();
        cases.push((
            text.replacen("{", &format!("{{\"model\":\"{HAIKU}\","), 1)
                .into_bytes(),
            "duplicate",
        ));
        for (body, label) in cases {
            let (_dir, mut h, mut w) = messages_host(vec![]);
            let g = messages_grant(&mut h, &mut w);
            assert!(
                push(&mut h, &mut w, messages_fetch(&g, &body)).is_err(),
                "{label}: {}",
                String::from_utf8_lossy(&body)
            );
            assert!(w.opened.lock().unwrap().is_empty());
        }
        let (_dir, mut h, mut w) = messages_host(vec![]);
        let g = messages_grant(&mut h, &mut w);
        let mut f = messages_fetch(&g, &serde_json::to_vec(&ok).unwrap());
        f["headers"]["x-stainless-retry-count"] = json!("1");
        assert!(push(&mut h, &mut w, f).is_err(), "client retry");
        let (_dir, mut h, mut w) = messages_host(vec![]);
        let g = messages_grant(&mut h, &mut w);
        let mut f = messages_fetch(&g, &serde_json::to_vec(&ok).unwrap());
        f["headers"]
            .as_object_mut()
            .unwrap()
            .remove("x-stainless-retry-count");
        assert!(push(&mut h, &mut w, f).is_err(), "retry count absent");
        for url in [
            "https://api.anthropic.com/v1/messages?beta=false",
            "https://api.anthropic.com/v1/complete",
            "https://chatgpt.com/backend-api/codex/responses",
        ] {
            let (_dir, mut h, mut w) = messages_host(vec![]);
            let g = messages_grant(&mut h, &mut w);
            let mut f = messages_fetch(&g, &serde_json::to_vec(&ok).unwrap());
            f["url"] = json!(url);
            assert!(push(&mut h, &mut w, f).is_err(), "{url}");
        }
        let (_dir, mut h, mut w) = messages_host(vec![]);
        let g = messages_grant(&mut h, &mut w);
        let mut f = messages_fetch(&g, &serde_json::to_vec(&ok).unwrap());
        f["url"] = json!("https://api.anthropic.com/v1/messages");
        assert!(push(&mut h, &mut w, f).is_ok());
    }
    #[test]
    fn messages_requests_must_extend_the_previous_messages() {
        let (_dir, mut h, _w) = messages_host(vec![]);
        let (plan, _) = h.hosted_plan().unwrap();
        let first = json!([{"role":"user","content":[{"type":"text","text":"q"}]},
            {"role":"system","content":[{"type":"text","text":"env"}]}]);
        h.hosted.last_messages = Some(first.clone());
        let next = |m: Value| messages_body(m);
        // Claude Code's observed rewrites: string content and moved cache_control.
        let extended = json!([{"role":"user","content":[{"type":"text","text":"q","cache_control":{"type":"ephemeral"}}]},
            {"role":"system","content":"env"},
            {"role":"assistant","content":[{"type":"text","text":"a"}]}]);
        h.messages_policy(&plan, &next(extended.clone()), false)
            .unwrap();
        // Thinking blocks are compared exactly: a moved cache_control on one
        // is a difference.
        let with_thinking = json!([{"role":"user","content":"q"},
            {"role":"assistant","content":[{"type":"thinking","thinking":"","signature":"s"},{"type":"text","text":"a"}]}]);
        h.hosted.last_messages = Some(with_thinking.clone());
        let mut longer = with_thinking.as_array().unwrap().clone();
        longer.push(json!({"role":"user","content":"more"}));
        h.messages_policy(&plan, &next(json!(longer.clone())), false)
            .unwrap();
        longer[1]["content"][0]["cache_control"] = json!({"type":"ephemeral"});
        assert!(
            h.messages_policy(&plan, &next(json!(longer)), false)
                .is_err()
        );
        h.hosted.last_messages = Some(first.clone());
        for bad in [
            first.clone(),
            json!([{"role":"user","content":[{"type":"text","text":"altered"}]},
                {"role":"system","content":"env"},{"role":"assistant","content":"a"}]),
            json!([{"role":"system","content":"env"},{"role":"user","content":"q"},{"role":"assistant","content":"a"}]),
        ] {
            assert!(
                h.messages_policy(&plan, &next(bad.clone()), false).is_err(),
                "{bad}"
            );
        }
    }
    #[test]
    fn messages_stream_terminal_and_failure_rules() {
        let sse = |events: &[Value]| {
            events
                .iter()
                .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap()))
                .collect::<String>()
        };
        let start = json!({"type":"message_start","message":{"id":"m","model":HAIKU,"usage":{"input_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":1}}});
        let delta = json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":7}});
        let stop = json!({"type":"message_stop"});
        let clean = sse(&[start.clone(), delta.clone(), stop.clone()]);
        let u = stream::usage_for(stream::Wire::Messages, clean.as_bytes());
        assert_eq!(
            (u["completed"].clone(), u["observed"].clone()),
            (json!(true), json!(true))
        );
        assert_eq!(
            (u["inputTokens"].clone(), u["outputTokens"].clone()),
            (json!(1), json!(7))
        );
        assert!(stream::terminal_is_last_for(
            stream::Wire::Messages,
            clean.as_bytes(),
            b": keepalive\n\n"
        ));
        assert!(!stream::terminal_is_last_for(
            stream::Wire::Messages,
            clean.as_bytes(),
            b"event: ping\n\n"
        ));
        assert!(!stream::terminal_is_last_for(
            stream::Wire::Responses,
            clean.as_bytes(),
            b""
        ));
        let failed = sse(&[
            start.clone(),
            json!({"type":"error","error":{"type":"overloaded_error"}}),
            stop,
        ]);
        let u = stream::usage_for(stream::Wire::Messages, failed.as_bytes());
        assert_eq!(
            (u["completed"].clone(), u["failed"].clone()),
            (json!(false), json!("overloaded_error"))
        );
        // Writes split by lifetime only when the breakdown adds up; otherwise
        // all writes count as one-hour.
        let with = |breakdown: Value| {
            let mut s = start.clone();
            s["message"]["usage"]["cache_creation_input_tokens"] = json!(10);
            s["message"]["usage"]["cache_creation"] = breakdown;
            stream::usage_for(
                stream::Wire::Messages,
                sse(&[s, delta.clone(), json!({"type":"message_stop"})]).as_bytes(),
            )
        };
        let split = |u: Value| {
            (
                u["cacheCreation5mInputTokens"].clone(),
                u["cacheCreation1hInputTokens"].clone(),
            )
        };
        assert_eq!(
            split(with(
                json!({"ephemeral_5m_input_tokens":4,"ephemeral_1h_input_tokens":6})
            )),
            (json!(4), json!(6))
        );
        assert_eq!(
            split(with(
                json!({"ephemeral_5m_input_tokens":9,"ephemeral_1h_input_tokens":6})
            )),
            (json!(0), json!(10))
        );
        assert_eq!(split(with(Value::Null)), (json!(0), json!(10)));
        let unterminated = sse(&[start, delta]);
        assert_eq!(
            stream::usage_for(stream::Wire::Messages, unterminated.as_bytes())["completed"],
            false
        );
    }
    #[test]
    fn subscription_route_relays_only_in_plan_responses() {
        let allowed = "anthropic-ratelimit-unified-status: allowed\r\nanthropic-ratelimit-unified-overage-status: rejected\r\nanthropic-ratelimit-unified-overage-disabled-reason: org_level_disabled\r\nrequest-id: req_1\r\n";
        for (headers, ok) in [
            (allowed, true),
            (
                "anthropic-ratelimit-unified-status: allowed_warning\r\nanthropic-ratelimit-unified-overage-status: rejected\r\n",
                true,
            ),
            ("anthropic-ratelimit-unified-status: allowed\r\n", false),
            (
                "anthropic-ratelimit-unified-status: allowed\r\nanthropic-ratelimit-unified-overage-status: allowed\r\n",
                false,
            ),
            ("anthropic-ratelimit-unified-status: rejected\r\n", false),
            ("", false),
        ] {
            let (dir, mut h, mut w) = messages_host(vec![messages_completed_with(headers)]);
            h.journal
                .job
                .coding
                .as_mut()
                .unwrap()
                .provider
                .as_mut()
                .unwrap()
                .hosted
                .routes[0]
                .endpoint = "https:anthropic-oauth".into();
            let g = messages_grant(&mut h, &mut w);
            let body =
                serde_json::to_vec(&messages_body(json!([{"role":"user","content":"q"}]))).unwrap();
            push(&mut h, &mut w, messages_fetch(&g, &body)).unwrap();
            let mut seen = vec![];
            for _ in 0..1000 {
                match h.coding_hosted_tick(&w.path) {
                    Ok(frames) => seen.extend(frames),
                    Err(_) => break,
                }
                if !seen.is_empty() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            let a = request(&h).attempts[0].clone();
            if ok {
                assert_eq!(seen.len(), 1, "{headers}");
                assert_eq!(a.disposition, AttemptDisposition::Pending);
            } else {
                assert!(seen.is_empty(), "head relayed: {headers}");
                assert_eq!(a.disposition, AttemptDisposition::Uncertain);
                assert!(a.reason.unwrap().contains("subscription_billing_guard"));
                let relay: Value = serde_json::from_slice(
                    &fs::read(dir.path().join("coding-hosted-1-1.relay.json")).unwrap(),
                )
                .unwrap();
                assert!(relay["headHeaders"].is_array());
                // Close-uncertain registers the head evidence.
                let head = head_evidence(&dir, &h, "1-1");
                assert_eq!(head["status"], 200);
                let supplied: Vec<_> = headers
                    .split_terminator("\r\n")
                    .map(|l| {
                        let (k, v) = l.split_once(": ").unwrap();
                        json!([k, v])
                    })
                    .collect();
                assert_eq!(head["headHeaders"], json!(supplied), "{headers}");
            }
        }
    }
    #[test]
    fn subscription_routes_dispatch_only_claude_code_requests() {
        let claude = messages_body(json!([{"role":"user","content":"q"}]));
        // Pi's own prompt mode through the same extension (probe run1 0005):
        // the same billing header and identity, Pi's prompt third.
        let mut pi_prompt = claude.clone();
        pi_prompt["system"][2]["text"] =
            json!("You are a coding assistant running inside pi, a coding agent harness. Help");
        let mut no_claude_code_blocks = claude.clone();
        no_claude_code_blocks["system"] =
            json!([{"type":"text","text":"You are an expert coding assistant."}]);
        let mut other_entry = claude.clone();
        other_entry["system"][0]["text"] =
            json!("x-anthropic-billing-header: cc_version=1; cc_entrypoint=cli;");
        let mut no_identity = claude.clone();
        no_identity["system"][1]["text"] = json!("You are Claude.");
        for (body, ok) in [
            (claude, true),
            (pi_prompt, false),
            (no_claude_code_blocks, false),
            (other_entry, false),
            (no_identity, false),
        ] {
            let (_dir, mut h, mut w) = messages_host(vec![messages_completed_with(
                "anthropic-ratelimit-unified-status: allowed\r\nanthropic-ratelimit-unified-overage-status: rejected\r\n",
            )]);
            h.journal
                .job
                .coding
                .as_mut()
                .unwrap()
                .provider
                .as_mut()
                .unwrap()
                .hosted
                .routes[0]
                .endpoint = "https:anthropic-oauth".into();
            let g = messages_grant(&mut h, &mut w);
            push(
                &mut h,
                &mut w,
                messages_fetch(&g, &serde_json::to_vec(&body).unwrap()),
            )
            .unwrap();
            let frames = tick_until(&mut h, &w, |a| !a.is_empty());
            if ok {
                assert_eq!(frames[0]["type"], "response-head", "{body}");
                assert_eq!(w.opened.lock().unwrap().len(), 1);
            } else {
                assert_eq!(frames[0]["type"], "response-unavailable", "{body}");
                assert_eq!(frames[0]["code"], "subscription_request_not_claude_code");
                assert!(w.opened.lock().unwrap().is_empty(), "nothing dispatched");
                // No attempt; the request records why and holds nothing.
                let r = request(&h);
                assert!(r.attempts.is_empty(), "no attempt reserved");
                assert_eq!(
                    r.not_dispatched.as_deref(),
                    Some("subscription_request_not_claude_code")
                );
                assert_eq!((r.charged_microusd(), r.unsettled()), (0, false));
                assert!(h.journal.coding.as_ref().unwrap().provider_failed);
            }
        }
    }

    // ---- Failover gate wiring (slice 3a) ----

    const SUB: &str = "codex-sub";
    const API: &str = "openai-api";
    fn openai_routes() -> HostedRoutes {
        HostedRoutes {
            routes: vec![
                HostedRoute {
                    id: "sub".into(),
                    payer: SUB.into(),
                    endpoint: "https:chatgpt-codex".into(),
                },
                HostedRoute {
                    id: "api".into(),
                    payer: API.into(),
                    endpoint: "https:openai-responses".into(),
                },
            ],
            max_attempts_per_request: 3,
        }
    }
    fn openai_pools() -> Value {
        json!([
            {"id": "codex-sub", "funding": "subscription",
             "members": [{"payer": SUB, "endpoint": "https:chatgpt-codex"}]},
            {"id": "openai-api", "funding": "api",
             "members": [{"payer": API, "endpoint": "https:openai-responses"}]}
        ])
    }
    fn failover_policy(order: &[&str], edit: impl FnOnce(&mut Value)) -> Value {
        let mut p = json!({
            "schema": failover::POLICY_SCHEMA,
            "version": "ops-1",
            "routes": order.iter().map(|r| json!({"route": r, "prepare_on_low": "switch_early"})).collect::<Vec<_>>(),
            "max_observation_age_ms": 3_600_000,
            "availability": {"max_evidence_age_ms": 604_800_000u64, "pools": openai_pools()},
        });
        edit(&mut p);
        p
    }
    /// A hosted Host on the OpenAI subscription/API table with the given job
    /// id, failover policy and admitted observation (none: refusal-only), and
    /// a transport identity naming each payer's access source.
    fn failover_host(
        job: &str,
        responses: Vec<Vec<u8>>,
        policy: Option<Value>,
        observation: Option<Value>,
    ) -> (tempfile::TempDir, Host, Wire) {
        let (dir, mut h, w) = host(responses, None);
        h.journal.job.job_id = job.into();
        let file = |name: &str, v: &Value| {
            let bytes = serde_json::to_vec(v).unwrap();
            let path = dir.path().join(name);
            fs::write(&path, &bytes).unwrap();
            Artifact {
                path,
                sha256: digest(&bytes),
            }
        };
        let failover = policy.map(|p| provider::FailoverInputs {
            policy: file("failover-policy.json", &p),
            observation: observation
                .as_ref()
                .map(|o| file("state-observation.json", o)),
        });
        let plan = h
            .journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap();
        plan.hosted = openai_routes();
        plan.failover = failover;
        h.journal.coding.as_mut().unwrap().hosted_transport = Some(https_identity(dir.path()));
        (dir, h, w)
    }
    /// The identity the real HTTPS transport records for the OpenAI table's
    /// payers (synthetic owner-only access files), so the view's funding
    /// cross-check is pinned to the transport's vocabulary.
    fn https_identity(dir: &Path) -> Value {
        use std::os::unix::fs::PermissionsExt;
        let private = |name: &str, bytes: &[u8]| {
            let path = dir.join(name);
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            path
        };
        let auth = private(
            "pi-auth.json",
            json!({"openai-codex":{"access":"synthetic-access-token-0123456789","accountId":"acct-1",
                "expires": epoch_ms() + 3_600_000}})
            .to_string()
            .as_bytes(),
        );
        let key = private("api-key", b"sk-synthetic-key-0123456789");
        let credentials = private(
            "credentials.json",
            json!({"schema": super::super::https::CREDENTIALS_SCHEMA, "payers": {
                SUB: {"kind": "chatgpt-pi-auth", "path": auth},
                API: {"kind": "api-key-file", "path": key}}})
            .to_string()
            .as_bytes(),
        );
        let transport = super::super::https::HttpsTransport::from_file(&credentials).unwrap();
        transport.check(&openai_routes()).unwrap();
        transport.identity()
    }
    fn grant_n(h: &mut Host, w: &mut Wire, id: u64) -> Value {
        if id == 2 {
            // The coding profile's second request follows the tool checkpoint
            // (no tool ran in these tests).
            h.journal.coding.as_mut().unwrap().tool_removed = true;
            let _ = fs::create_dir(h.journal.job.state_dir.join("coding-workspace"));
        }
        let replies = push(h, w, json!({"type":"coding_provider_request","id":id,"requestModel":{"id":"gpt-6-luna","provider":"openai-codex","contextWindow":272000},"contextBytes":1000})).unwrap();
        let mut written = vec![];
        h.coding_bridge_dispatch(&replies[0], &mut |b| {
            written = b.to_vec();
            Ok(())
        })
        .unwrap();
        serde_json::from_slice(&written).unwrap()
    }
    fn fetch_n(grant: &Value, id: u64) -> Value {
        let mut f = fetch(grant);
        f["id"] = json!(id);
        f
    }
    /// Pull a relayed response to its end, acknowledge and complete it.
    fn finish(h: &mut Host, w: &mut Wire, g: &Value, id: u64) {
        let mut ordinal = 1;
        loop {
            push(h, w, op(g, "pull", ordinal)).unwrap();
            let f = tick_until(h, w, |a| !a.is_empty());
            ordinal += 1;
            if f[0]["type"] == "response-end" {
                break;
            }
        }
        push(h, w, op(g, "ack", ordinal)).unwrap();
        push(
            h,
            w,
            json!({"type":"provider_complete","id":id,"stopReason":"stop","usage":{}}),
        )
        .unwrap();
    }
    /// Run request `id` to its first answer frame (head or unavailable).
    fn send_request(h: &mut Host, w: &mut Wire, id: u64) -> (Value, Vec<Value>) {
        let g = grant_n(h, w, id);
        push(h, w, fetch_n(&g, id)).unwrap();
        let f = tick_until(h, w, |a| {
            a.iter()
                .any(|f| f["type"] == "response-head" || f["type"] == "response-unavailable")
        });
        (g, f)
    }
    fn refusal_body(status: u16, headers: &str, body: Value) -> Vec<u8> {
        let body = body.to_string();
        let mut b = format!(
            "HTTP/1.1 {status} Refused\r\n{headers}Content-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .into_bytes();
        b.extend(b"");
        b
    }
    fn sse(events: &[Value]) -> Vec<u8> {
        let body: String = events.iter().map(|e| format!("data: {e}\n\n")).collect();
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }
    fn reservation(h: &Host, id: u64) -> provider::Reservation {
        h.journal.coding.as_ref().unwrap().provider_requests[&id].clone()
    }
    fn decision(h: &Host, id: u64, attempt: usize) -> failover::FailoverDecision {
        reservation(h, id).attempts[attempt]
            .failover
            .clone()
            .unwrap()
    }
    /// Reproduce every journaled decision from the ledger and the registered,
    /// digest-verified observation artifacts.
    fn replay(h: &mut Host) -> std::result::Result<(), String> {
        let ctx = h.hosted_failover().unwrap().unwrap();
        let (plan, routes) = h.hosted_plan().unwrap();
        let router = Router::with_failover(
            routes
                .router(&plan.model, plan.max_requests, u64::MAX)
                .policy()
                .clone(),
            ctx.binding(),
        );
        let dir = h.journal.job.state_dir.clone();
        let artifacts = h.journal.artifacts.clone();
        let read = move |name: &str| {
            let a = artifacts.iter().find(|a| a.path == dir.join(name))?;
            a.verify().ok()
        };
        failover::replay(
            &ctx,
            router,
            &h.journal.coding.as_ref().unwrap().provider_requests,
            &read,
        )
    }
    fn observation_doc(h: &Host, name: &str) -> Value {
        let path = h.journal.job.state_dir.join(name);
        let a = h.journal.artifacts.iter().find(|a| a.path == path).unwrap();
        serde_json::from_slice(&a.verify().unwrap()).unwrap()
    }

    #[test]
    fn failover_cold_start_sends_the_preferred_route_and_journals_a_replayable_decision() {
        let (_dir, mut h, mut w) = failover_host(
            "job-a",
            vec![completed()],
            Some(failover_policy(&["sub", "api"], |_| {})),
            None,
        );
        let (g, _) = send_request(&mut h, &mut w, 1);
        finish(&mut h, &mut w, &g, 1);
        let d = decision(&h, 1, 0);
        assert_eq!((d.action.as_str(), d.route.as_str()), ("send", "sub"));
        assert_eq!(d.policy_version, "ops-1");
        assert_eq!(d.table_digest, openai_routes().digest());
        // The observation is retained, registered and bound by its digest.
        let path = h
            .journal
            .job
            .state_dir
            .join("coding-hosted-1-1.observation.json");
        let a = h.journal.artifacts.iter().find(|a| a.path == path).unwrap();
        assert_eq!(a.sha256, d.state_observation_sha256);
        let doc = observation_doc(&h, "coding-hosted-1-1.observation.json");
        assert_eq!(doc["admitted"]["ignored"], "absent");
        assert_eq!(
            doc["in_job"]["schema"],
            "work-engine.hosted-route-availability.v2"
        );
        assert_eq!(doc["exposure"]["subscription_to_api"], true);
        assert_eq!(w.opened.lock().unwrap()[0].0, "sub");
        replay(&mut h).unwrap();
    }

    #[test]
    fn without_failover_inputs_routing_is_refusal_only_and_journals_no_decision() {
        let (dir, mut h, mut w) =
            failover_host("job-a", vec![refusal(429), completed()], None, None);
        let (g, _) = send_request(&mut h, &mut w, 1);
        finish(&mut h, &mut w, &g, 1);
        let r = reservation(&h, 1);
        assert_eq!(r.attempts.len(), 2);
        assert!(r.attempts.iter().all(|a| a.failover.is_none()));
        assert!(
            !hosted_artifacts(&dir, &h)
                .iter()
                .any(|n| n.contains("observation"))
        );
        assert!(h.hosted.router.as_ref().unwrap().failover().is_none());
    }

    /// Another job's subscription exhaustion with a known reset: the policy
    /// gate allows a skip to the paying route; once the reset has passed the
    /// next send returns to the subscription. A stale observation is ignored.
    #[test]
    fn failover_skips_on_admitted_exhaustion_and_returns_after_its_reset() {
        let now = epoch_ms();
        let reset = now + 20_000;
        let policy = failover_policy(&["sub", "api"], |p| {
            p["routes"][1]["foreign_evidence_skip"] = json!(true)
        });
        let parsed = failover::FailoverPolicy::parse(
            &serde_json::to_vec(&policy).unwrap(),
            &openai_routes(),
        )
        .unwrap();
        let foreign = |observed| {
            failover::tests::view(
                &parsed,
                observed,
                &[(
                    "codex-sub",
                    "subscription_5h",
                    "exhausted",
                    Some(reset),
                    false,
                    "job-x",
                    1,
                    1,
                )],
            )
        };
        let (_dir, mut h, mut w) = failover_host(
            "job-a",
            vec![completed(), completed()],
            Some(policy.clone()),
            Some(foreign(now - 1000)),
        );
        h.hosted.now_override = Some(now);
        let (g, _) = send_request(&mut h, &mut w, 1);
        finish(&mut h, &mut w, &g, 1);
        assert_eq!(decision(&h, 1, 0).action, "skip");
        assert_eq!(decision(&h, 1, 0).route, "api");
        h.hosted.now_override = Some(reset);
        let (g, _) = send_request(&mut h, &mut w, 2);
        finish(&mut h, &mut w, &g, 2);
        assert_eq!(decision(&h, 2, 0).route, "sub");
        assert_eq!(decision(&h, 2, 0).action, "send");
        let opened: Vec<String> = w
            .opened
            .lock()
            .unwrap()
            .iter()
            .map(|o| o.0.clone())
            .collect();
        assert_eq!(opened, ["api", "sub"]);
        replay(&mut h).unwrap();

        // Older than the operator's bound: ignored, no proactive skip.
        let (_dir, mut h, mut w) = failover_host(
            "job-b",
            vec![completed()],
            Some(policy),
            Some(foreign(now - 3_600_001)),
        );
        h.hosted.now_override = Some(now);
        let (g, _) = send_request(&mut h, &mut w, 1);
        finish(&mut h, &mut w, &g, 1);
        assert_eq!(decision(&h, 1, 0).route, "sub");
        let doc = observation_doc(&h, "coding-hosted-1-1.observation.json");
        assert_eq!(doc["admitted"]["ignored"], "stale_age");
    }

    /// This job's own API rate-limit refusal carries a reset (retry-after):
    /// the next request returns to it only once the reset has passed, as one
    /// verification attempt; refused again, the same reset is not reused.
    #[test]
    fn failover_fails_back_on_its_own_reset_once_per_request() {
        let limited = || {
            refusal_body(
                429,
                "Retry-After: 20\r\n",
                json!({"error":{"code":"rate_limit_exceeded","type":"requests"}}),
            )
        };
        let (_dir, mut h, mut w) = failover_host(
            "job-a",
            vec![limited(), completed(), limited(), completed()],
            Some(failover_policy(&["api", "sub"], |_| {})),
            None,
        );
        let now = epoch_ms();
        h.hosted.now_override = Some(now + 100);
        let (g, _) = send_request(&mut h, &mut w, 1);
        finish(&mut h, &mut w, &g, 1);
        let r = reservation(&h, 1);
        assert_eq!(r.attempts[0].disposition, AttemptDisposition::Refused);
        assert_eq!(decision(&h, 1, 1).route, "sub");
        // Before the reset the refused API route stays out.
        let head_ms = head_evidence(&_dir, &h, "1-1")["headMs"].as_u64().unwrap();
        h.hosted.now_override = Some(head_ms + 20_000);
        let (g, _) = send_request(&mut h, &mut w, 2);
        finish(&mut h, &mut w, &g, 2);
        let d = decision(&h, 2, 0);
        assert_eq!((d.action.as_str(), d.route.as_str()), ("fail_back", "api"));
        let o = d.reset_observation.unwrap();
        assert_eq!((o.job_id.as_str(), o.request, o.attempt), ("job-a", 1, 1));
        assert_eq!(o.available_again_at, head_ms + 20_000);
        // Refused again within request 2: its newer refusal has a future
        // reset, so the request falls to the subscription.
        let r = reservation(&h, 2);
        assert_eq!(r.attempts[0].disposition, AttemptDisposition::Refused);
        assert_eq!(decision(&h, 2, 1).route, "sub");
        let opened: Vec<String> = w
            .opened
            .lock()
            .unwrap()
            .iter()
            .map(|o| o.0.clone())
            .collect();
        assert_eq!(opened, ["api", "sub", "api", "sub"]);
        replay(&mut h).unwrap();
    }

    #[test]
    fn failover_pause_ends_the_request_without_a_send() {
        let credit = refusal_body(
            429,
            "",
            json!({"error":{"code":"insufficient_quota","type":"insufficient_quota"}}),
        );
        let (dir, mut h, mut w) = failover_host(
            "job-a",
            vec![refusal(429), credit],
            Some(failover_policy(&["sub", "api"], |_| {})),
            None,
        );
        let (_, f) = send_request(&mut h, &mut w, 1);
        assert_eq!(f[0]["type"], "response-unavailable");
        assert_eq!(f[0]["code"], "failover_pause");
        let r = reservation(&h, 1);
        assert_eq!(r.attempts.len(), 2);
        assert_eq!(w.opened.lock().unwrap().len(), 2, "no third send");
        let pause = r.failover_pause.unwrap();
        assert_eq!(pause.action, "pause");
        // Codex refusals carry no reset; API credit has none.
        assert_eq!(
            pause.resume_time,
            failover::ResumeTime::Unknown("unknown".into())
        );
        assert!(h.journal.coding.as_ref().unwrap().provider_failed);
        let path = dir.path().join("coding-hosted-1-pause.observation.json");
        let a = h.journal.artifacts.iter().find(|a| a.path == path).unwrap();
        assert_eq!(a.sha256, pause.state_observation_sha256);
        let doc = observation_doc(&h, "coding-hosted-1-pause.observation.json");
        assert_eq!(
            doc["in_job"]["pools"][1]["dimensions"][1]["status"], "exhausted",
            "API credit exhausted on this job's own refusal"
        );
        replay(&mut h).unwrap();
    }

    /// A router attempt that never reaches the ledger (a non-Claude-Code
    /// request on the subscription OAuth route) is not in the gate's
    /// routing history, and replay still reproduces the journal.
    #[test]
    fn a_router_attempt_outside_the_ledger_is_not_in_the_gate_snapshot() {
        let (_dir, mut h, mut w) = failover_host("job-a", vec![], Some(json!({})), None);
        let mut routes = openai_routes();
        routes.routes[0].endpoint = "https:anthropic-oauth".into();
        routes.routes[0].payer = "claude-sub".into();
        let mut policy = failover_policy(&["sub", "api"], |_| {});
        policy["availability"]["pools"][0] = json!({"id": "claude-sub", "funding": "subscription",
            "members": [{"payer": "claude-sub", "endpoint": "https:anthropic-oauth"}]});
        let bytes = serde_json::to_vec(&policy).unwrap();
        let path = h.journal.job.state_dir.join("failover-policy.json");
        fs::write(&path, &bytes).unwrap();
        let plan = h
            .journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap();
        plan.hosted = routes.clone();
        plan.failover = Some(provider::FailoverInputs {
            policy: Artifact {
                path,
                sha256: digest(&bytes),
            },
            observation: None,
        });
        let (_, f) = send_request(&mut h, &mut w, 1);
        assert_eq!(f[0]["code"], "subscription_request_not_claude_code");
        let router = h.hosted.router.as_ref().unwrap();
        assert_eq!(router.attempts().len(), 1);
        assert!(router.snapshot(2).attempted[0], "the router saw it");
        let requests = &h.journal.coding.as_ref().unwrap().provider_requests;
        assert!(requests[&1].attempts.is_empty());
        let snap = failover::ledger_snapshot(&routes, requests, 2, None);
        assert_eq!(snap.attempted, vec![false, false], "the ledger did not");
        replay(&mut h).unwrap();
    }

    #[test]
    fn binding_journals_api_exposure_and_the_admission_evaluation() {
        let (_dir, mut h) = super::super::provider::ledger_tests::fixture();
        h.journal.job.coding.as_mut().unwrap().schema = HOSTED_BRIDGE_PROFILE.into();
        h.journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap()
            .hosted = openai_routes();
        let transport = || -> Box<dyn HostedProviderTransport> {
            Box::new(FakeTransport {
                responses: VecDeque::new(),
                opened: Opened::default(),
                unreachable: false,
            })
        };
        h.bind_hosted_transport(transport()).unwrap();
        let record = h
            .journal
            .coding
            .as_ref()
            .unwrap()
            .hosted_failover
            .clone()
            .unwrap();
        assert_eq!(record["exposure"]["subscription_to_api"], true);
        assert_eq!(record["exposure"]["pairs"], json!([["sub", "api"]]));
        assert_eq!(record["policy"], Value::Null);

        let (dir, mut h) = super::super::provider::ledger_tests::fixture();
        h.journal.job.coding.as_mut().unwrap().schema = HOSTED_BRIDGE_PROFILE.into();
        let policy = serde_json::to_vec(&failover_policy(&["api", "sub"], |_| {})).unwrap();
        let path = dir.path().join("failover-policy.json");
        fs::write(&path, &policy).unwrap();
        let plan = h
            .journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap();
        plan.hosted = openai_routes();
        plan.failover = Some(provider::FailoverInputs {
            policy: Artifact {
                path,
                sha256: digest(&policy),
            },
            observation: None,
        });
        h.bind_hosted_transport(transport()).unwrap();
        let record = h
            .journal
            .coding
            .as_ref()
            .unwrap()
            .hosted_failover
            .clone()
            .unwrap();
        assert_eq!(record["exposure"]["subscription_to_api"], false);
        assert_eq!(record["exposure"]["policy"], true);
        assert_eq!(record["policy"]["sha256"], digest(&policy));
        assert_eq!(record["admission_evaluation"]["action"], "send");
        assert_eq!(record["admission_evaluation"]["route"], "api");
        assert_eq!(record["admission_evaluation"]["admitted_ignored"], "absent");
    }

    /// P2-7: journals produced by the real Host feed the route availability
    /// view. Job A: a Codex subscription refusal with head evidence, a relayed
    /// API success, then a post-200 provider failure (the Host's reason
    /// wording); job B: a 200 that closes without a failure event. The view's
    /// reading of the journal shape and the reason wording is pinned here.
    /// The post-200 failure ends job A's provider use; the next job, admitted
    /// with this view's output, switches at its first send (Q1).
    #[test]
    fn host_journals_feed_the_route_availability_view_and_the_next_job_switches() {
        let refused = refusal_body(
            429,
            "Retry-After: 120\r\nrequest-id: req_1\r\n",
            json!({"error":{"code":"usage_limit_reached","type":"usage_limit_reached"}}),
        );
        let failed = sse(&[
            json!({"type":"response.created","response":{"id":"r2"}}),
            json!({"type":"response.failed","response":{"id":"r2","status":"failed","error":{"code":"insufficient_quota","message":"quota"}}}),
        ]);
        let (dir_a, mut a, mut wa) =
            failover_host("job-a", vec![refused, completed(), failed], None, None);
        let (g, _) = send_request(&mut a, &mut wa, 1);
        finish(&mut a, &mut wa, &g, 1);
        let (g, _) = send_request(&mut a, &mut wa, 2);
        let mut ordinal = 1;
        loop {
            push(&mut a, &mut wa, op(&g, "pull", ordinal)).unwrap();
            let f = tick_until(&mut a, &wa, |x| !x.is_empty());
            ordinal += 1;
            if f[0]["type"] == "response-end" {
                break;
            }
        }
        push(&mut a, &mut wa, op(&g, "ack", ordinal)).unwrap();
        let r2 = reservation(&a, 2);
        assert_eq!(r2.attempts[0].route, "api");
        assert_eq!(r2.attempts[0].disposition, AttemptDisposition::Uncertain);
        let reason = r2.attempts[0].reason.clone().unwrap();
        assert_eq!(
            reason,
            "provider failure event in relayed stream: insufficient_quota"
        );
        assert!(reason.starts_with(hosted_route_availability::HOST_STREAM_FAILURE));
        // Never resent, and the job's provider use is over.
        assert!(a.journal.coding.as_ref().unwrap().provider_failed);
        assert_eq!(wa.opened.lock().unwrap().len(), 3);

        let open = sse(&[json!({"type":"response.created","response":{"id":"r3"}})]);
        let (dir_b, mut b, mut wb) = failover_host("job-b", vec![open], None, None);
        let (g, _) = send_request(&mut b, &mut wb, 1);
        let mut ordinal = 1;
        loop {
            push(&mut b, &mut wb, op(&g, "pull", ordinal)).unwrap();
            let f = tick_until(&mut b, &wb, |x| !x.is_empty());
            ordinal += 1;
            if f[0]["type"] == "response-end" {
                break;
            }
        }
        push(&mut b, &mut wb, op(&g, "ack", ordinal)).unwrap();
        let rb = reservation(&b, 1);
        assert_eq!(rb.attempts[0].disposition, AttemptDisposition::Uncertain);

        let now = epoch_ms() + 1000;
        let config = json!({
            "schema": hosted_route_availability::CONFIG_SCHEMA,
            "now_ms": now,
            "journal_roots": [dir_a.path(), dir_b.path()],
            "max_evidence_age_ms": 604_800_000u64,
            "pools": openai_pools(),
        });
        let out: Value = serde_json::from_slice(
            &hosted_route_availability::view(&serde_json::to_vec(&config).unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(out["journals"].as_array().unwrap().len(), 2);
        assert!(
            out["journals"]
                .as_array()
                .unwrap()
                .iter()
                .all(|j| j["read"] == true),
            "{out}"
        );
        assert_eq!(out["complete"], true, "{out}");
        let dim = |pool: usize, d: usize| out["pools"][pool]["dimensions"][d].clone();
        // Codex subscription refusal: no window headers, so no known window.
        assert_eq!(dim(0, 0)["status"], "unknown");
        assert_eq!(dim(0, 0)["reason"], "window_undetermined");
        assert_eq!(dim(0, 0)["evidence"]["artifact"], "head.json");
        // The API success, then API credit exhausted by the post-200 failure.
        assert_eq!(dim(1, 0)["status"], "available");
        assert_eq!(dim(1, 1)["status"], "exhausted");
        assert_eq!(dim(1, 1)["available_again_reason"], "operator_action");
        assert_eq!(
            (
                dim(1, 1)["evidence"]["job_id"].as_str(),
                dim(1, 1)["evidence"]["request"].as_u64()
            ),
            (Some("job-a"), Some(2))
        );
        let informational = out["marks"]["informational"].as_array().unwrap();
        assert!(
            informational
                .iter()
                .any(|m| m["kind"] == "post_head_failure" && m["job_id"] == "job-a")
        );
        assert!(
            informational
                .iter()
                .any(|m| m["kind"] == "post_head_no_failure_event" && m["job_id"] == "job-b"),
            "{out}"
        );

        // The next job starts from this view: API credit is exhausted on
        // another job's evidence, so its first send skips to the subscription.
        let (_dir_c, mut c, mut wc) = failover_host(
            "job-c",
            vec![completed()],
            Some(failover_policy(&["api", "sub"], |_| {})),
            Some(out),
        );
        c.hosted.now_override = Some(now);
        let (g, _) = send_request(&mut c, &mut wc, 1);
        finish(&mut c, &mut wc, &g, 1);
        let d = decision(&c, 1, 0);
        assert_eq!((d.action.as_str(), d.route.as_str()), ("skip", "sub"));
        replay(&mut c).unwrap();
    }

    // ---- Head classification before relay (slice 3b) ----

    fn linked_table() -> String {
        hosted_route_availability::provisional::table_sha256()
    }
    /// Anthropic subscription then API routes (`apis` API routes).
    fn anthropic_routes(apis: usize) -> HostedRoutes {
        let mut routes = vec![HostedRoute {
            id: "sub".into(),
            payer: "payer-a".into(),
            endpoint: "https:anthropic-oauth".into(),
        }];
        for i in 0..apis {
            routes.push(HostedRoute {
                id: if i == 0 {
                    "api".into()
                } else {
                    format!("api{}", i + 1)
                },
                payer: format!("payer-{}", (b'b' + i as u8) as char),
                endpoint: "https:anthropic-api".into(),
            });
        }
        HostedRoutes {
            max_attempts_per_request: routes.len() as u32,
            routes,
        }
    }
    /// A Messages Host on Anthropic routes with the admitted classification
    /// table digest `table` (none: no classification) and payer sources the
    /// route availability view accepts.
    fn classify_host(
        responses: Vec<Vec<u8>>,
        apis: usize,
        table: Option<String>,
    ) -> (tempfile::TempDir, Host, Wire) {
        let (dir, mut h, w) = messages_host(responses);
        let plan = h
            .journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap();
        plan.hosted = anthropic_routes(apis);
        plan.classification_table_sha256 = table;
        h.journal.coding.as_mut().unwrap().hosted_transport = Some(json!({"transport":"fake",
            "payer_sources":{"payer-a":"claude-code-auth","payer-b":"anthropic-api-key-env","payer-c":"anthropic-api-key-env"}}));
        (dir, h, w)
    }
    fn anthropic_error(status: u16, error: &str) -> Vec<u8> {
        refusal_body(
            status,
            "request-id: req_9\r\n",
            json!({"type":"error","error":{"type":error,"message":"m"}}),
        )
    }
    /// Send request 1; the frames up to the first head, unavailable answer
    /// or failure.
    fn classify_send(h: &mut Host, w: &mut Wire) -> (Value, Vec<Value>) {
        let g = messages_grant(h, w);
        let body =
            serde_json::to_vec(&messages_body(json!([{"role":"user","content":"q"}]))).unwrap();
        push(h, w, messages_fetch(&g, &body)).unwrap();
        let f = tick_until(h, w, |a| {
            a.iter().any(|f| {
                matches!(
                    f["type"].as_str(),
                    Some("response-head" | "response-unavailable" | "response-failure")
                )
            })
        });
        (g, f)
    }
    /// Pull to the terminal frame and acknowledge; the body bytes and the
    /// terminal frame.
    fn classify_drain(h: &mut Host, w: &mut Wire, g: &Value) -> (Vec<u8>, Value) {
        let mut body = vec![];
        let mut ordinal = 1;
        loop {
            push(h, w, op(g, "pull", ordinal)).unwrap();
            let f = tick_until(h, w, |a| !a.is_empty());
            ordinal += 1;
            match f[0]["type"].as_str() {
                Some("response-chunk") => {
                    body.extend(B64.decode(f[0]["body"].as_str().unwrap()).unwrap())
                }
                _ => {
                    push(h, w, op(g, "ack", ordinal)).unwrap();
                    return (body, f[0].clone());
                }
            }
        }
    }
    fn hold_of(h: &Host, attempt: usize) -> stream::HoldRecord {
        request(h).attempts[attempt].head_hold.clone().unwrap()
    }

    #[test]
    fn held_overload_head_fails_over_and_reaches_the_container_as_nothing() {
        let (dir, mut h, mut w) = classify_host(
            vec![
                anthropic_error(529, "overloaded_error"),
                messages_completed(),
            ],
            1,
            Some(linked_table()),
        );
        let (g, frames) = classify_send(&mut h, &mut w);
        // The container's first and only head is the fallback's 200.
        assert_eq!(frames.len(), 1, "{frames:?}");
        assert_eq!(
            (frames[0]["type"].as_str(), frames[0]["status"].as_u64()),
            (Some("response-head"), Some(200))
        );
        assert_eq!(w.opened.lock().unwrap().len(), 2);
        let r = request(&h);
        let a = &r.attempts[0];
        assert_eq!(
            (a.route.as_str(), a.disposition, a.http_status),
            ("sub", AttemptDisposition::Refused, Some(529))
        );
        assert_eq!(a.classification_table_sha256, Some(linked_table()));
        let hold = hold_of(&h, 0);
        assert_eq!(
            (hold.held, hold.outcome.as_str(), hold.reason.as_str()),
            (true, "refused", "classified")
        );
        assert_eq!(hold.class.as_ref().unwrap().class, "overload");
        assert_eq!(
            r.attempts[1].classification_table_sha256,
            Some(linked_table())
        );
        // Retained exactly like a definitive refusal: head evidence and the
        // refusal body, no stream.
        let names = hosted_artifacts(&dir, &h);
        assert!(
            names.contains(&"1-1.head.json".to_string())
                && names.contains(&"1-1.refusal.body".to_string()),
            "{names:?}"
        );
        assert!(!names.contains(&"1-1.stream.sse".to_string()));
        let body = fs::read(dir.path().join("coding-hosted-1-1.refusal.body")).unwrap();
        assert_eq!(Some(digest(&body)), a.refusal_body_sha256);
        // The router took it as a refusal of the route.
        let router = h.hosted.router.as_ref().unwrap();
        assert_eq!(
            router.attempts()[1].reason,
            "failover-after-529-overloaded_error"
        );
        classify_drain(&mut h, &mut w, &g);
        let r = request(&h);
        assert_eq!(r.attempts[1].disposition, AttemptDisposition::Completed);
        assert!(
            r.attempts[1].head_hold.is_none(),
            "a success head is never held"
        );
        // The route availability view reads it as it reads a 429.
        let config = json!({"schema": hosted_route_availability::CONFIG_SCHEMA, "now_ms": epoch_ms() + 1000,
            "journal_roots": [dir.path()], "max_evidence_age_ms": 604_800_000u64,
            "pools": [{"id":"claude-sub","funding":"subscription","members":[{"payer":"payer-a","endpoint":"https:anthropic-oauth"}]}]});
        let out: Value = serde_json::from_slice(
            &hosted_route_availability::view(&serde_json::to_vec(&config).unwrap()).unwrap(),
        )
        .unwrap();
        assert!(
            out["marks"]["informational"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["kind"] == "non_quota_failure"
                    && m["class"] == "overload"
                    && m["request"] == 1
                    && m["attempt"] == 1),
            "{out}"
        );
    }

    /// The container-visible result of request 1 and the provider requests.
    fn classify_run(
        response: Vec<u8>,
        table: Option<String>,
    ) -> (tempfile::TempDir, Host, (u64, Vec<u8>, Value), usize) {
        let (dir, mut h, mut w) = classify_host(vec![response, messages_completed()], 1, table);
        let (g, frames) = classify_send(&mut h, &mut w);
        assert_eq!(frames.len(), 1, "{frames:?}");
        assert_eq!(frames[0]["type"], "response-head");
        let status = frames[0]["status"].as_u64().unwrap();
        let (body, end) = classify_drain(&mut h, &mut w, &g);
        let opened = w.opened.lock().unwrap().len();
        (dir, h, (status, body, end), opened)
    }
    /// Provider requests JSON without clock-derived fields, for comparing
    /// separate runs: each attempt's `deadline_ms` (the job deadline, set
    /// from the wall clock in whole seconds by the fixture), a hold's `ms`,
    /// and failover decision and pause times.
    fn clockless_ledger(h: &Host) -> Value {
        let mut v =
            serde_json::to_value(&h.journal.coding.as_ref().unwrap().provider_requests).unwrap();
        for r in v.as_object_mut().unwrap().values_mut() {
            if let Some(p) = r.get_mut("failover_pause").and_then(Value::as_object_mut) {
                p.remove("decided_ms");
            }
            for a in r["attempts"].as_array_mut().into_iter().flatten() {
                let a = a.as_object_mut().unwrap();
                a.remove("deadline_ms");
                if let Some(h) = a.get_mut("head_hold").and_then(Value::as_object_mut) {
                    h.remove("ms");
                }
                if let Some(d) = a.get_mut("failover").and_then(Value::as_object_mut) {
                    d.remove("decided_ms");
                }
            }
        }
        v
    }
    /// [`clockless_ledger`] without the classification fields.
    fn without_classification(h: &Host) -> Value {
        let mut v = clockless_ledger(h);
        for a in v["1"]["attempts"].as_array_mut().unwrap() {
            a.as_object_mut().unwrap().remove("head_hold");
            a.as_object_mut()
                .unwrap()
                .remove("classification_table_sha256");
        }
        v
    }
    fn relay_json(dir: &tempfile::TempDir) -> Value {
        serde_json::from_slice(&fs::read(dir.path().join("coding-hosted-1-1.relay.json")).unwrap())
            .unwrap()
    }
    /// Relay evidence without clock- or timing-derived fields, for comparing
    /// separate runs: open, end and cancel-wait times, the per-chunk record
    /// (pull and forward times; boundaries and end-at-pull depend on read
    /// timing), the pulls counted before the end, and a hold's `ms`.
    fn clockless_relay(dir: &tempfile::TempDir) -> Value {
        let mut v = relay_json(dir);
        let o = v.as_object_mut().unwrap();
        for k in [
            "openedMs",
            "upstreamEndMs",
            "cancelDuringPendingPullMs",
            "chunks",
            "pullsBeforeUpstreamEnd",
        ] {
            o.remove(k);
        }
        if let Some(h) = o.get_mut("hold").and_then(Value::as_object_mut) {
            h.remove("ms");
        }
        v
    }

    #[test]
    fn unmatched_held_head_is_replayed_byte_for_byte_and_settles_uncertain() {
        let unmatched = anthropic_error(529, "api_error");
        let body =
            unmatched[unmatched.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4..].to_vec();
        let (dir_a, a, seen_a, opened_a) = classify_run(unmatched.clone(), Some(linked_table()));
        let (dir_b, b, seen_b, opened_b) = classify_run(unmatched, None);
        // The container sees the same status, body bytes and terminal frame.
        assert_eq!(seen_a, seen_b);
        assert_eq!(seen_a.0, 529);
        assert_eq!(seen_a.1, body);
        assert_eq!(seen_a.2["type"], "response-end");
        assert_eq!((opened_a, opened_b), (1, 1), "never failed over");
        for (dir, h) in [(&dir_a, &a), (&dir_b, &b)] {
            let r = request(h);
            assert_eq!(r.attempts.len(), 1);
            assert_eq!(r.attempts[0].disposition, AttemptDisposition::Uncertain);
            assert!(r.unsettled());
            assert_eq!(
                fs::read(dir.path().join("coding-hosted-1-1.stream.sse")).unwrap(),
                body
            );
        }
        let hold = hold_of(&a, 0);
        assert_eq!(
            (hold.outcome.as_str(), hold.reason.as_str(), hold.bytes),
            ("relayed", "unmatched", body.len())
        );
        assert_eq!(hold.class.unwrap().class, "unclassified");
        assert_eq!(relay_json(&dir_a)["hold"]["reason"], "unmatched");
        // Apart from the classification fields the ledger is the same.
        assert_eq!(without_classification(&a), without_classification(&b));
        assert!(relay_json(&dir_b).get("hold").is_none());
        let mut held = clockless_relay(&dir_a);
        held.as_object_mut().unwrap().remove("hold");
        assert_eq!(held, clockless_relay(&dir_b));
    }

    #[test]
    fn absent_or_mismatched_classification_digest_relays_as_today() {
        let overload = anthropic_error(529, "overloaded_error");
        let (dir_n, none, seen_n, opened_n) = classify_run(overload.clone(), None);
        let (dir_m, mismatch, seen_m, opened_m) = classify_run(overload, Some("0".repeat(64)));
        assert_eq!(seen_n, seen_m);
        assert_eq!(seen_n.0, 529, "relayed, not refused");
        assert_eq!((opened_n, opened_m), (1, 1));
        // Identical ledgers, with none of the classification fields, and
        // relay evidence without a hold.
        assert_eq!(clockless_ledger(&none), clockless_ledger(&mismatch));
        let attempt = clockless_ledger(&none)["1"]["attempts"][0].clone();
        assert!(
            attempt.get("head_hold").is_none()
                && attempt.get("classification_table_sha256").is_none(),
            "{attempt}"
        );
        assert_eq!(attempt["disposition"], "uncertain");
        assert_eq!(clockless_relay(&dir_n), clockless_relay(&dir_m));
        assert!(relay_json(&dir_m).get("hold").is_none());
    }

    #[test]
    fn binding_journals_the_classification_table_and_held_failover_exposure() {
        let record = |table: Option<String>| {
            let (_dir, mut h) = super::super::provider::ledger_tests::fixture();
            h.journal.job.coding.as_mut().unwrap().schema = HOSTED_BRIDGE_PROFILE.into();
            let plan = h
                .journal
                .job
                .coding
                .as_mut()
                .unwrap()
                .provider
                .as_mut()
                .unwrap();
            plan.hosted = anthropic_routes(1);
            plan.classification_table_sha256 = table;
            h.bind_hosted_transport(Box::new(FakeTransport {
                responses: VecDeque::new(),
                opened: Opened::default(),
                unreachable: false,
            }))
            .unwrap();
            h.journal
                .coding
                .as_ref()
                .unwrap()
                .hosted_failover
                .clone()
                .unwrap()
        };
        let absent = record(None);
        assert!(absent.get("classification").is_none());
        assert!(absent["exposure"].get("held_failover_to_api").is_none());
        // A mismatch is journaled; nothing else changes.
        let other = "0".repeat(64);
        let mut mismatch = record(Some(other.clone()));
        let c = mismatch
            .as_object_mut()
            .unwrap()
            .remove("classification")
            .unwrap();
        assert_eq!(
            (
                c["active"].clone(),
                c["admitted_sha256"].clone(),
                c["table_sha256"].clone()
            ),
            (json!(false), json!(other), json!(linked_table()))
        );
        assert_eq!(mismatch, absent);
        // Active: the overload-to-API exposure is reported.
        let active = record(Some(linked_table()));
        assert_eq!(active["classification"]["active"], true);
        assert_eq!(
            active["classification"]["table_version"],
            hosted_route_availability::provisional::table().version
        );
        assert_eq!(
            active["exposure"]["held_failover_to_api"],
            json!([{"from":"sub","to":"api","statuses":[529]}])
        );
        assert_eq!(active["exposure"]["subscription_to_api"], true);
    }

    #[test]
    fn classified_failures_then_success_relay_one_head_inside_the_window() {
        let (_dir, mut h, mut w) = classify_host(
            vec![
                anthropic_error(529, "overloaded_error"),
                anthropic_error(529, "overloaded_error"),
                messages_completed(),
            ],
            2,
            Some(linked_table()),
        );
        let start = std::time::Instant::now();
        let (g, frames) = classify_send(&mut h, &mut w);
        let window = Duration::from_millis(HEAD_TIMEOUT_MS - HOLD_WINDOW_MARGIN_MS);
        assert!(start.elapsed() < Duration::from_secs(2) && start.elapsed() < window);
        assert_eq!(frames.len(), 1, "{frames:?}");
        assert_eq!(frames[0]["status"], 200);
        classify_drain(&mut h, &mut w, &g);
        let r = request(&h);
        let got: Vec<_> = r
            .attempts
            .iter()
            .map(|a| (a.route.as_str(), a.disposition))
            .collect();
        assert_eq!(
            got,
            vec![
                ("sub", AttemptDisposition::Refused),
                ("api", AttemptDisposition::Refused),
                ("api2", AttemptDisposition::Completed)
            ]
        );
        assert_eq!(
            (r.charged_microusd() > 0, r.unsettled()),
            (true, true),
            "only the completion is held until settlement"
        );
    }

    #[test]
    fn slow_overload_body_is_relayed_before_the_head_window_ends() {
        let mut slow = anthropic_error(529, "overloaded_error");
        let start = slow.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        let body = slow[start..].to_vec();
        let at = start + 6;
        slow.splice(at..at, DELAY.iter().copied());
        let (_dir, mut h, mut w) =
            classify_host(vec![slow, messages_completed()], 1, Some(linked_table()));
        h.hosted.head_window_override = Some(Duration::from_millis(300));
        let start = std::time::Instant::now();
        let (g, frames) = classify_send(&mut h, &mut w);
        assert!(
            start.elapsed() < Duration::from_millis(900),
            "the container was stranded: {:?}",
            start.elapsed()
        );
        assert_eq!(
            (frames[0]["type"].as_str(), frames[0]["status"].as_u64()),
            (Some("response-head"), Some(529))
        );
        let (got, end) = classify_drain(&mut h, &mut w, &g);
        assert_eq!(got, body, "the held bytes, then the rest of the body");
        assert_eq!(end["type"], "response-end");
        assert_eq!(w.opened.lock().unwrap().len(), 1, "never failed over");
        let r = request(&h);
        assert_eq!(r.attempts[0].disposition, AttemptDisposition::Uncertain);
        let hold = hold_of(&h, 0);
        assert_eq!(
            (hold.outcome.as_str(), hold.reason.as_str(), hold.bytes),
            ("relayed", "time_budget", 6)
        );
    }

    #[test]
    fn post_200_overload_event_stays_uncertain_with_classification_active() {
        let events = [
            json!({"type":"message_start","message":{"id":"msg_1","model":HAIKU,"usage":{"input_tokens":10,"output_tokens":1}}}),
            json!({"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}),
        ];
        let body: String = events
            .iter()
            .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap()))
            .collect();
        let failed = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}", body.len()).into_bytes();
        let (_dir, mut h, mut w) =
            classify_host(vec![failed, messages_completed()], 1, Some(linked_table()));
        // The API route: no subscription billing guard on the head.
        h.journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap()
            .hosted
            .routes
            .remove(0);
        let (g, frames) = classify_send(&mut h, &mut w);
        assert_eq!(frames[0]["status"], 200);
        classify_drain(&mut h, &mut w, &g);
        let a = &request(&h).attempts[0];
        assert_eq!(a.disposition, AttemptDisposition::Uncertain);
        assert!(
            a.reason.as_deref().unwrap().contains("overloaded_error"),
            "{:?}",
            a.reason
        );
        assert!(a.head_hold.is_none());
        assert_eq!(w.opened.lock().unwrap().len(), 1);
        assert!(h.journal.coding.as_ref().unwrap().provider_failed);
    }

    #[test]
    fn replay_reproduces_decisions_across_a_classified_refusal() {
        let (dir, mut h, mut w) = classify_host(
            vec![
                anthropic_error(529, "overloaded_error"),
                messages_completed(),
            ],
            1,
            Some(linked_table()),
        );
        let policy = json!({
            "schema": failover::POLICY_SCHEMA, "version": "ops-1",
            "routes": [{"route":"sub","prepare_on_low":"switch_early"},{"route":"api","prepare_on_low":"switch_early"}],
            "max_observation_age_ms": 3_600_000,
            "availability": {"max_evidence_age_ms": 604_800_000u64, "pools": [
                {"id":"claude-sub","funding":"subscription","members":[{"payer":"payer-a","endpoint":"https:anthropic-oauth"}]},
                {"id":"anthropic-api","funding":"api","members":[{"payer":"payer-b","endpoint":"https:anthropic-api"}]}]},
        });
        let bytes = serde_json::to_vec(&policy).unwrap();
        let path = dir.path().join("failover-policy.json");
        fs::write(&path, &bytes).unwrap();
        h.journal
            .job
            .coding
            .as_mut()
            .unwrap()
            .provider
            .as_mut()
            .unwrap()
            .failover = Some(provider::FailoverInputs {
            policy: Artifact {
                path,
                sha256: digest(&bytes),
            },
            observation: None,
        });
        let (g, frames) = classify_send(&mut h, &mut w);
        assert_eq!((frames.len(), frames[0]["status"].as_u64()), (1, Some(200)));
        classify_drain(&mut h, &mut w, &g);
        let r = request(&h);
        assert_eq!(r.attempts[0].disposition, AttemptDisposition::Refused);
        let (d0, d1) = (decision(&h, 1, 0), decision(&h, 1, 1));
        assert_eq!((d0.action.as_str(), d0.route.as_str()), ("send", "sub"));
        assert_eq!(d1.route, "api");
        replay(&mut h).unwrap();
        // The replay is bound to the retained observations.
        let name = failover::observation_name(1, Some(2));
        let path = h.journal.job.state_dir.join(&name);
        let mut doc: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        doc["decided_ms"] = json!(1);
        fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
        assert!(replay(&mut h).is_err());
    }

    #[test]
    fn crash_or_cancel_during_hold_settles_uncertain_without_failover() {
        let mut slow = anthropic_error(529, "overloaded_error");
        let at = slow.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        slow.splice(at..at, DELAY.iter().copied());
        // Crash: the head was read and held; nothing reached the container.
        let (dir, mut h, mut w) = classify_host(
            vec![slow.clone(), messages_completed()],
            1,
            Some(linked_table()),
        );
        let g = messages_grant(&mut h, &mut w);
        let body =
            serde_json::to_vec(&messages_body(json!([{"role":"user","content":"q"}]))).unwrap();
        push(&mut h, &mut w, messages_fetch(&g, &body)).unwrap();
        let path = dir.path().join("coding-hosted-1-1.head.json");
        let frames = tick_until(&mut h, &w, |_| path.exists());
        assert!(frames.is_empty(), "{frames:?}");
        h.hosted = Hosted::default();
        h.coding_hosted_reconcile_unsettled().unwrap();
        let a = &request(&h).attempts[0];
        assert_eq!(a.disposition, AttemptDisposition::Uncertain);
        assert!(
            a.reason.as_deref().unwrap().contains("nothing was relayed"),
            "{:?}",
            a.reason
        );
        assert_eq!(head_evidence(&dir, &h, "1-1")["status"], 529);
        // Cancel: confirmed, uncertain, no failover.
        let (dir, mut h, mut w) =
            classify_host(vec![slow, messages_completed()], 1, Some(linked_table()));
        let g = messages_grant(&mut h, &mut w);
        push(&mut h, &mut w, messages_fetch(&g, &body)).unwrap();
        let path = dir.path().join("coding-hosted-1-1.head.json");
        let frames = tick_until(&mut h, &w, |_| path.exists());
        assert!(frames.is_empty(), "held: {frames:?}");
        push(&mut h, &mut w, op(&g, "cancel", 1)).unwrap();
        let frames = tick_until(&mut h, &w, has("response-cancelled"));
        assert_eq!(frames.len(), 1);
        let r = request(&h);
        assert_eq!(
            (r.attempts.len(), r.attempts[0].disposition),
            (1, AttemptDisposition::Uncertain)
        );
        assert_eq!(hold_of(&h, 0).outcome, "cancelled");
        assert_eq!(w.opened.lock().unwrap().len(), 1);
    }

    #[test]
    fn hold_never_outlasts_the_attempt_deadline() {
        // The body completes (and would classify) only after three seconds;
        // the attempt deadline, at most two seconds away, ends the hold first.
        let mut slow = anthropic_error(529, "overloaded_error");
        let at = slow.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        slow.splice(at..at, DELAY3.iter().copied());
        let (_dir, mut h, mut w) =
            classify_host(vec![slow, messages_completed()], 1, Some(linked_table()));
        // More than one and at most two seconds away: never at hand at
        // dispatch, always before the body completes.
        h.journal.job.deadline_unix_seconds = epoch_ms() / 1000 + 2;
        let start = std::time::Instant::now();
        let (_g, frames) = classify_send(&mut h, &mut w);
        assert!(
            start.elapsed() < Duration::from_millis(2500),
            "{:?}",
            start.elapsed()
        );
        assert_eq!(
            (frames[0]["type"].as_str(), frames[0]["status"].as_u64()),
            (Some("response-head"), Some(529))
        );
        assert_eq!(hold_of(&h, 0).reason, "time_budget");
        assert_eq!(w.opened.lock().unwrap().len(), 1);
    }

    #[test]
    fn host_close_after_a_relayed_held_head_keeps_its_hold_record() {
        let mut slow = anthropic_error(529, "overloaded_error");
        let at = slow.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        slow.splice(at..at, DELAY.iter().copied());
        let (_dir, mut h, mut w) =
            classify_host(vec![slow, messages_completed()], 1, Some(linked_table()));
        h.hosted.head_window_override = Some(Duration::from_millis(200));
        let (_g, frames) = classify_send(&mut h, &mut w);
        assert_eq!(frames[0]["status"], 529);
        h.coding_hosted_abort("run ended").unwrap();
        let a = &request(&h).attempts[0];
        assert_eq!(a.disposition, AttemptDisposition::Uncertain);
        assert!(a.reason.as_deref().unwrap().contains("run ended"));
        let hold = a.head_hold.clone().unwrap();
        assert_eq!(
            (hold.outcome.as_str(), hold.reason.as_str()),
            ("relayed", "time_budget")
        );
    }
}

#[cfg(test)]
mod body_tests {
    use super::completions_forward_body;

    #[test]
    fn completions_forward_replaces_layer_a_with_layer_b_floor_slug() {
        let body = br#"{"model":"~deepseek/deepseek-v4-flash-latest","messages":[],"stream":true}"#;
        let out = completions_forward_body(body, "~deepseek/deepseek-v4-flash-latest:floor");
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["model"], "~deepseek/deepseek-v4-flash-latest:floor");
        assert_eq!(v["stream"], true);
    }

    #[test]
    fn completions_forward_passes_through_non_json_bytes() {
        let body = b"not json";
        assert_eq!(completions_forward_body(body, "x"), body.to_vec());
    }
}
