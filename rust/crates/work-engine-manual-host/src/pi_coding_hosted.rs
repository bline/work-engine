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
use super::*;
use crate::pi_route::{self as route, Decision, Disposition, Router};
use crate::pi_stream::{self as stream, Progress, Session, Unavailable};
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
const MAX_REQUEST_BODY: usize = 45_000;
const MAX_HEADERS: usize = 64;
/// Credential-bearing request headers. The container's values are markers;
/// the transport applies real access itself.
const ACCESS_HEADERS: [&str; 4] = [
    "authorization",
    "chatgpt-account-id",
    "cookie",
    "proxy-authorization",
];

/// One permitted provider route. The order of routes is preference order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedRoute {
    pub id: String,
    pub payer: String,
    /// Nonsecret endpoint identity, never a credential. Only
    /// `loopback:<port>` (offline qualification) is supported; public HTTPS
    /// dispatch is a later phase.
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
            || !self
                .routes
                .iter()
                .all(|r| bounded(&r.id) && bounded(&r.payer) && loopback(&r.endpoint).is_some())
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
    fn router(&self, model: &str, max_requests: u32, expires_ms: u64) -> Router {
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
fn loopback(endpoint: &str) -> Option<std::net::SocketAddr> {
    let port: u16 = endpoint.strip_prefix("loopback:")?.parse().ok()?;
    (port != 0).then(|| std::net::SocketAddr::from(([127, 0, 0, 1], port)))
}

/// The provider request to dispatch, as validated from the container's
/// `provider_fetch`, with credential headers removed.
#[derive(Clone, Debug)]
pub struct HostedRequest {
    pub request: u64,
    pub attempt: u64,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
/// Upstream response reader and its abort.
pub type Upstream = (Box<dyn Read + Send>, Box<dyn FnOnce() + Send>);
/// Trusted composition bound by the embedder. It holds provider access, which
/// never enters the container or the journal, and applies it only in `open`.
pub trait HostedProviderTransport {
    /// Write one attempt's request to `route` and return its response. Called
    /// only inside a gate-admitted dispatch, before `deadline_ms`.
    fn open(
        &mut self,
        route: &HostedRoute,
        request: &HostedRequest,
        deadline_ms: i64,
    ) -> Result<Upstream>;
    /// Exclusion screen over bytes relayed to the container: an error if they
    /// contain access material, otherwise how many trailing bytes to hold
    /// back because they may begin it.
    fn hold(&self, pending: &[u8]) -> std::result::Result<usize, &'static str>;
}
/// Offline-only transport to synthetic loopback routes with a synthetic,
/// Host-generated access token.
pub struct LoopbackTransport {
    access: Vec<u8>,
}
impl LoopbackTransport {
    pub fn new() -> Result<Self> {
        let mut random = [0u8; 32];
        File::open("/dev/urandom")?.read_exact(&mut random)?;
        Ok(Self {
            access: format!("synthetic-A-{}-host-only", B64.encode(random)).into_bytes(),
        })
    }
}
impl HostedProviderTransport for LoopbackTransport {
    fn open(
        &mut self,
        route: &HostedRoute,
        request: &HostedRequest,
        deadline_ms: i64,
    ) -> Result<Upstream> {
        use std::net::{Shutdown, TcpStream};
        let address =
            loopback(&route.endpoint).ok_or_else(|| Error::Refused("route endpoint".into()))?;
        let left = u64::try_from(deadline_ms.saturating_sub(now_ms()?))
            .ok()
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::Refused("provider attempt deadline passed".into()))?;
        let mut s = TcpStream::connect_timeout(&address, Duration::from_millis(left.min(5000)))?;
        s.set_write_timeout(Some(Duration::from_millis(left.min(5000))))?;
        s.set_read_timeout(Some(Duration::from_millis(left.min(READ_IDLE_MS))))?;
        let access =
            std::str::from_utf8(&self.access).map_err(|_| Error::Refused("access".into()))?;
        let mut head = format!(
            "POST /backend-api/codex/responses HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {access}\r\nX-Request-Id: {}-{}\r\nContent-Length: {}\r\nConnection: close\r\n",
            request.request,
            request.attempt,
            request.body.len()
        );
        for (k, v) in &request.headers {
            if !matches!(
                k.as_str(),
                "host" | "content-length" | "connection" | "transfer-encoding"
            ) {
                head.push_str(&format!("{k}: {v}\r\n"));
            }
        }
        head.push_str("\r\n");
        s.write_all(head.as_bytes())?;
        s.write_all(&request.body)?;
        let killer = s.try_clone()?;
        Ok((
            Box::new(s),
            Box::new(move || {
                let _ = killer.shutdown(Shutdown::Both);
            }),
        ))
    }
    fn hold(&self, pending: &[u8]) -> std::result::Result<usize, &'static str> {
        if pending.windows(self.access.len()).any(|w| w == self.access) {
            return Err("hosted_access_exclusion");
        }
        Ok(review_execution_evidence::pi::prefix_suffix(
            pending,
            &self.access,
        ))
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

/// Validated container request, kept for every attempt of the request.
#[derive(Clone)]
struct Fetch {
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}
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
    /// This Host ran the launch: attempts still pending at its end were left
    /// by this process, not by one that is gone.
    ran: bool,
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
        self.hosted_transport = Some(transport);
        Ok(())
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
        let routes = plan
            .hosted
            .clone()
            .ok_or_else(|| Error::Refused("hosted route table absent".into()))?;
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
            profile: HOSTED_BRIDGE_PROFILE.into(),
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
        if !Self::own_stream_frame(s, frame)
            || frame["method"] != "POST"
            || frame["url"] != format!("{base}/codex/responses")
        {
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
        let body = B64
            .decode(frame["body"].as_str().unwrap_or(""))
            .map_err(|_| Error::Refused("provider fetch body encoding".into()))?;
        let parsed: Value = serde_json::from_slice(&body)
            .map_err(|_| Error::Refused("provider fetch body JSON".into()))?;
        if body.len() > MAX_REQUEST_BODY
            || parsed["model"] != plan.model
            || parsed["stream"] != true
        {
            return refuse("provider fetch body bound/model/stream conflict");
        }
        Ok(Fetch { headers, body })
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
                if self.unprocessed(stdout)?.is_empty() {
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
    /// Complete container lines not yet processed by `coding_advance`.
    fn unprocessed(&self, stdout: &Path) -> Result<Vec<Value>> {
        let raw = fs::read(stdout)?;
        let done = self.journal.coding.as_ref().unwrap().frames;
        raw.split_inclusive(|b| *b == b'\n')
            .filter(|l| l.ends_with(b"\n"))
            .skip(done)
            .map(|l| Ok(serde_json::from_slice(l)?))
            .collect()
    }
    /// Admit, reserve and dispatch the next attempt of the stream's request,
    /// or answer that no attempt is permitted.
    fn hosted_attempt(&mut self, s: &mut Stream, fetch: Fetch, stdout: &Path) -> Result<()> {
        let (plan, routes) = self.hosted_plan()?;
        let deadline = self.journal.job.deadline_unix_seconds.saturating_mul(1000);
        let router = self
            .hosted
            .router
            .get_or_insert_with(|| routes.router(&plan.model, plan.max_requests, deadline));
        let (index, routed) = match router.next(s.request, &plan.model, stream::now_millis()) {
            Decision::Attempt { route, attempt } => (route, attempt),
            Decision::Unavailable(reason) => return self.hosted_unavailable(s, reason),
        };
        let route = routes.routes[index].clone();
        let reserve = provider::full_reservation(&plan.selected()?)?;
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
        };
        let Some(mut transport) = self.hosted_transport.take() else {
            self.hosted_uncertain(s.request, attempt, routed, "hosted transport absent")?;
            return refuse("hosted transport absent");
        };
        let Some(mut gate) = self.coding_gate.take() else {
            self.hosted_transport = Some(transport);
            self.hosted_uncertain(s.request, attempt, routed, "coding lifecycle gate missing")?;
            return refuse("coding lifecycle gate missing");
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
            if host.unprocessed(stdout)?.iter().any(|f| {
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
            opened = Some(transport.open(&route, &request, until)?);
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
        let session = match Session::start(
            reader,
            abort,
            &s.binding,
            s.request,
            STREAM,
            route::refusal_status,
        ) {
            Ok(session) => session,
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
                drop(session);
                let sha = self.hosted_retain(s.request, attempt, "refusal.body", &body)?;
                let r = self
                    .journal
                    .coding
                    .as_mut()
                    .unwrap()
                    .provider_requests
                    .get_mut(&s.request)
                    .unwrap();
                let disposition = r.settle_attempt(
                    attempt,
                    AttemptEvidence::HeadRefusal {
                        status,
                        relayed_bytes: relayed,
                        drained_and_screened: true,
                        body_sha256: sha,
                    },
                )?;
                let code = serde_json::from_slice::<Value>(&body)
                    .ok()
                    .and_then(|v| v["error"]["code"].as_str().map(str::to_owned))
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
            &serde_json::to_vec(&relay.evidence())?,
        )?;
        let terminal = stream::sse_data(&relay.forwarded)
            .iter()
            .filter_map(|d| serde_json::from_str::<Value>(d).ok())
            .find(|v| v["type"] == "response.completed");
        let usage = stream::usage(&relay.forwarded);
        let r = self
            .journal
            .coding
            .as_mut()
            .unwrap()
            .provider_requests
            .get_mut(&request)
            .unwrap();
        let disposition = r.settle_attempt(attempt, AttemptEvidence::from_relay(&relay))?;
        debug_assert_eq!(
            r.attempts.last().unwrap().relay_sha256.as_deref(),
            Some(sha.as_str())
        );
        r.http_status = Some(relay.status.into());
        if let Some(v) = terminal.as_ref().filter(|_| usage["failed"].is_null()) {
            let response = &v["response"];
            if let Some(model) = response["model"].as_str() {
                if model != plan.model {
                    self.journal.coding.as_mut().unwrap().provider_failed = true;
                    self.save()?;
                    return refuse("backend model identity conflict");
                }
                r.backend_model = Some(model.into());
            }
            r.raw_terminal = Some(
                json!({"eventType":"response.completed","status":response["status"],
                "usage":{"input_tokens":response["usage"]["input_tokens"],"output_tokens":response["usage"]["output_tokens"],"total_tokens":response["usage"]["total_tokens"]}}),
            );
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
    /// Queue session frames for the container. Before the head is queued its
    /// status is persisted on the attempt record and the attempt's stream
    /// file is created; each relayed chunk is durably appended to that file
    /// before it is queued. A Host crash loses neither. A retention failure closes the attempt uncertain
    /// without relaying the chunk.
    fn hosted_emit(&mut self, s: &mut Stream, attempt: u64, frames: Vec<Value>) -> Result<()> {
        for f in frames {
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
    /// Create-only retention of exact screened bytes, registered as a journal
    /// artifact so collection verifies it; returns the digest.
    fn hosted_retain(
        &mut self,
        request: u64,
        attempt: u64,
        kind: &str,
        bytes: &[u8],
    ) -> Result<String> {
        let dir = self.journal.job.state_dir.clone();
        let path = dir.join(format!("coding-hosted-{request}-{attempt}.{kind}"));
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
        let (sha, retained) = self.hosted_stream_register(s.request, attempt)?;
        let differs = retained != relay.forwarded;
        self.hosted_retain(
            s.request,
            attempt,
            "relay.json",
            &serde_json::to_vec(&relay.evidence())?,
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
    }
    impl HostedProviderTransport for FakeTransport {
        fn open(&mut self, route: &HostedRoute, r: &HostedRequest, _: i64) -> Result<Upstream> {
            self.opened
                .lock()
                .unwrap()
                .push((route.id.clone(), r.headers.clone()));
            let bytes = self
                .responses
                .pop_front()
                .ok_or_else(|| Error::Refused("no scripted response".into()))?;
            Ok((Box::new(std::io::Cursor::new(bytes)), Box::new(|| {})))
        }
        fn hold(&self, pending: &[u8]) -> std::result::Result<usize, &'static str> {
            if pending.windows(6).any(|w| w == b"SECRET") {
                return Err("hosted_access_exclusion");
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
        let body = br#"{"error":{"code":"usage_limit"}}"#;
        let mut b = format!(
            "HTTP/1.1 {status} Refused\r\nContent-Length: {}\r\n\r\n",
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
            plan.hosted = Some(HostedRoutes {
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
            });
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
            "body":B64.encode(br#"{"model":"gpt-6-luna","stream":true,"input":[]}"#)})
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
            err.contains("hosted attempt 1 is Uncertain") && err.contains("provider failure event"),
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
}
