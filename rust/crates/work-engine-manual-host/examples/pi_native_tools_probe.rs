//! Offline synthetic endpoint qualification. No live-provider implementation.
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use review_execution_evidence::pi::{Binding, LiveStore, SealedReader};
use session_task_admission::{
    canonical_source::Pin,
    pi_pilot::{Effect, PiOwner},
};
use sha2::{Digest, Sha256};
use std::os::fd::AsRawFd;
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    time::Duration,
};
use work_engine_manual_host::pi_native::{self as native, Result};
use work_engine_manual_host::pi_route::{self as route, Decision, Disposition};
use work_engine_manual_host::pi_stream as stream;

fn connect(
    address: std::net::SocketAddr,
    body: &[u8],
    access: &[u8],
    request_id: &str,
    lease: &session_task_admission::pi_pilot::EffectLease,
) -> Result<TcpStream> {
    let mut s = TcpStream::connect_timeout(&address, Duration::from_secs(5))
        .map_err(|_| "synthetic_connect")?;
    s.set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| "synthetic_write_timeout")?;
    s.set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|_| "synthetic_timeout")?;
    let access = std::str::from_utf8(access).map_err(|_| "synthetic_access")?;
    lease.check_current()?;
    write!(s,"POST /offline-native HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {access}\r\nX-Request-Id: {request_id}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).map_err(|_|"synthetic_write")?;
    s.write_all(body).map_err(|_| "synthetic_write")?;
    Ok(s)
}
/// Buffered duplicate used only for the endpoint deduplication control.
fn drain(s: TcpStream) -> Result<Vec<u8>> {
    let mut up = stream::Upstream::open(s)?;
    if up.status != 200 {
        return Err("synthetic_status");
    }
    let mut b = vec![];
    while let Some(x) = up.next(stream::CHUNK)? {
        b.extend(x);
        if b.len() > stream::MAX_RESPONSE {
            return Err("synthetic_response_bound");
        }
    }
    Ok(b)
}
/// Native channel for one relayed stream: every received frame is recorded
/// and every host frame is sent under the provider effect lease.
struct Native<'a> {
    workload: &'a mut native::Workload,
    store: &'a mut LiveStore,
    lease: &'a session_task_admission::pi_pilot::EffectLease,
    frames: &'a mut u64,
    checked: std::time::Instant,
}
const LEASE_RECHECK: Duration = Duration::from_secs(5);
impl stream::Screen for Native<'_> {
    fn hold(&self, pending: &[u8]) -> Result<usize> {
        self.store.stream_hold(pending)
    }
}
impl stream::Channel for Native<'_> {
    fn send(&mut self, v: &Value) -> Result<()> {
        // Lease currentness is rechecked at most every LEASE_RECHECK while
        // relaying one admitted provider response; revocation therefore stops
        // delivery within that bounded window rather than per chunk.
        if self.checked.elapsed() >= LEASE_RECHECK {
            self.workload.send_effect(v, self.lease)?;
            self.checked = std::time::Instant::now();
            Ok(())
        } else {
            self.workload.send(v)
        }
    }
    fn receive(&mut self) -> Result<Value> {
        // Content-free pulls are summarized per chunk in the stream control;
        // every other native frame is retained individually.
        let f = match self.workload.receive() {
            Ok(f) if f["type"] == "pull" => f,
            Ok(f) => {
                self.store.record_frame(*self.frames, f.clone())?;
                *self.frames += 1;
                f
            }
            Err(code) => {
                self.store.record_control(
                    "failure",
                    json!({"frameOrdinal":*self.frames,"transportError":code}),
                )?;
                return Err(code);
            }
        };
        self.store.check(&f)?;
        Ok(f)
    }
    fn readable(&mut self) -> Result<bool> {
        self.workload.readable()
    }
}
fn receive_checked(
    workload: &mut native::Workload,
    store: &mut LiveStore,
    ordinal: u64,
) -> Result<Value> {
    match workload.receive() {
        Ok(frame) => {
            store.record_frame(ordinal, frame.clone())?;
            Ok(frame)
        }
        Err(code) => {
            store.record_control(
                "failure",
                json!({"frameOrdinal":ordinal,"transportError":code}),
            )?;
            Err(code)
        }
    }
}
fn run() -> Result<()> {
    let root = std::env::args().nth(1).ok_or("out_required")?;
    let root = Path::new(&root);
    let argv: Vec<_> = std::env::args().collect();

    let mut uuid = [0u8; 16];
    fs::File::open("/dev/urandom")
        .map_err(|_| "uuid")?
        .read_exact(&mut uuid)
        .map_err(|_| "uuid")?;
    uuid[6] = (uuid[6] & 15) | 64;
    uuid[8] = (uuid[8] & 63) | 128;
    let h = uuid.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let session = format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    );
    let mut access_random = [0u8; 32];
    fs::File::open("/dev/urandom")
        .map_err(|_| "access_source")?
        .read_exact(&mut access_random)
        .map_err(|_| "access_source")?;
    let access = format!("synthetic-A-{}-host-only", B64.encode(access_random));

    let binding = Binding::native(session, "offline-native-attempt".into(), 1);
    let mut store = LiveStore::create(
        root,
        access.as_bytes().to_vec(),
        binding.clone(),
        native::now() + 120000,
    )?;
    let registry = store.create_registry(&root.join("registry"))?;
    let owner = PiOwner::open_checked(
        Pin {
            path: argv.get(2).ok_or("descriptor_required")?.into(),
            sha256: argv.get(3).ok_or("pin_required")?.clone(),
        },
        registry,
    )?;
    let job = hash(b"pi-native-offline-repair-v1");
    let executable =
        hash(&fs::read(std::env::current_exe().map_err(|_| "exe")?).map_err(|_| "exe")?);
    owner.admit(&job, &executable)?;
    let (operation, owner_profile, native_old) = owner.retention_identity()?;
    if native_old.is_some() {
        return Err("no_replay");
    }
    store.bind_admission(&operation, &owner_profile)?;
    let binding = store.binding().clone();
    // One or more loopback synthetic routes, in preference order.
    let addrs: Vec<std::net::SocketAddr> = argv
        .get(4)
        .ok_or("external_observer_required")?
        .split(',')
        .map(|a| a.parse().map_err(|_| "observer_address"))
        .collect::<Result<_>>()?;
    if addrs.is_empty() || addrs.len() > 2 || addrs.iter().any(|a| !a.ip().is_loopback()) {
        return Err("offline_observer_only");
    }
    let policy = route::Policy {
        model: "gpt-6-luna".into(),
        routes: addrs
            .iter()
            .enumerate()
            .map(|(i, a)| route::Route {
                id: ["primary", "secondary"][i].into(),
                connection: format!("synthetic-loopback:{}", a.port()),
                model: "gpt-6-luna".into(),
                payer: ["synthetic-subscription", "synthetic-api"][i].into(),
                authorized: true,
                revoked: false,
                max_attempts: 16,
            })
            .collect(),
        max_attempts_per_request: 2,
        allowance_units: 16,
        reserve_units: 1,
        expires_ms: native::now() + 3_600_000,
    };
    // The pinned policy is retained first so a fresh process can recover
    // routing state and verify every attempt's policy digest.
    store.record_control(
        "route-000",
        json!({"policy":policy,"policyDigest":policy.digest()}),
    )?;
    let mut router = route::Router::new(policy);
    // Crash injection for restart controls: `intent:N` exits right after the
    // durable intent of attempt N; `connect:N` after its request was sent.
    let crash = std::env::var("PI_PROBE_CRASH").unwrap_or_default();
    let crash_at = |point: &str, attempt: u64| {
        if crash == format!("{point}:{attempt}") {
            eprintln!("pi_native_tools_probe: injected crash {point}:{attempt}");
            std::process::exit(86);
        }
    };
    let mut route_records = 0u64;
    let mut stream_requests = 0u64;
    let package = Path::new("/home/bline/cbm-work/pi-artifact-handoff-20261113/pi");
    let receiver = Path::new(env!("CARGO_MANIFEST_DIR")).join("receiver/pi-native-tools.mjs");
    let allocation = owner.enter(Effect::Allocation)?;
    let mut workload = native::Workload::create(package, &receiver, &mut store, &allocation)?;
    drop(allocation); // Allocation remains unknown until native UUID is observed.
    // Native consumer aborts its session if a pulled chunk does not arrive in
    // time; the abort reaches the host as a stream cancel.
    let stream_idle_ms: u64 = std::env::var("PI_PROBE_STREAM_IDLE_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|v| (500..=60000).contains(v))
        .unwrap_or(30000);
    let initialization = owner.enter(Effect::Initialization)?;
    initialization.consumption_deadline_millis((native::now() + 120000) / 1000)?;
    workload.send_effect(
        &json!({"type":"init","binding":binding,"sequence":0,"stream":0,"streamIdleMs":stream_idle_ms}),
        &initialization,
    )?;
    let ready = receive_checked(&mut workload, &mut store, 0)?;
    native::joined(&ready, "ready", &binding, 0, 0)?;
    if ready["sessionId"] != binding.session || ready["tools"] != json!(["write", "bash"]) {
        return Err("native_init");
    };
    store.record_control("ready", ready.clone())?;
    initialization.observed(&hash(&serde_json::to_vec(&ready).map_err(|_| "ready")?))?;
    owner.seal(&job, &binding.session)?;
    owner.check_native(&binding.session)?;
    let inspection = workload.inspect(&store)?;
    let header = ready["header"].clone();
    let mut frames: u64 = 1;
    let mut streams = vec![];
    let mut sends = 0;
    let mut provider_attempts = 0;
    let mut tools = vec![];
    let mut snapshots = vec![];
    let mut native_result = json!(null);
    for seq in 1..=3 {
        let predecessor = store.head().map(str::to_owned);
        let pending = store.begin(native::now(), seq, predecessor.as_deref())?;
        store.consume(pending, native::now())?;
        // Prompt enqueue is not tool or provider authority. Each productive
        // consumer below acquires its own unknown-before-I/O lease.
        workload.send(&json!({"type":"prompt","binding":binding,"sequence":seq,"stream":0,"prompt":format!("Offline development turn {seq}; build and test the expression module, then return the review result.")}))?;
        let mut stream = 0;
        let snap = loop {
            let frame = receive_checked(&mut workload, &mut store, frames)?;
            frames += 1;
            store.check(&frame)?;
            match frame["type"].as_str() {
                Some("fetch") => {
                    stream += 1;
                    native::joined(&frame, "fetch", &binding, seq, stream)?;
                    let body = match native::request(&frame, &store) {
                        Ok(body) => body,
                        Err(code) => {
                            store.record_control(
                                "failure",
                                json!({"stage":"request","code":code}),
                            )?;
                            return Err(code);
                        }
                    };
                    // Route admission precedes every provider attempt. Only a
                    // definitive pre-output refusal can lead to another attempt.
                    let (lease, request_id, addr, relay, evidence, attempt) = loop {
                        let decision =
                            router.next(stream_requests + 1, "gpt-6-luna", native::now());
                        let (route, attempt) = match decision {
                            Decision::Attempt { route, attempt } => (route, attempt),
                            Decision::Unavailable(reason) => {
                                store.record_control(&format!("route-{:03}", route_records + 1), json!({"decision":"unavailable","request":stream_requests + 1,"reason":reason,"router":router.evidence()}))?;
                                workload.send(&json!({"type":"response-unavailable","binding":binding,"sequence":seq,"stream":stream,"code":reason}))?;
                                if let Ok(next) = receive_checked(&mut workload, &mut store, frames)
                                {
                                    let _ = store.record_control("failure", json!({"stage":"route-unavailable","reason":reason,"nativeNextType":next["type"],"nativeLastMessage":next["messages"].as_array().and_then(|m| m.last()).map(|m| json!({"role":m["role"],"stopReason":m["stopReason"],"errorMessage":m["errorMessage"]}))}));
                                }
                                return Err("route_unavailable");
                            }
                        };
                        route_records += 1;
                        store.record_control(
                            &format!("route-{route_records:03}"),
                            json!({"decision":"attempt","intent":router.last()}),
                        )?;
                        crash_at("intent", attempt);
                        let addr = addrs[route];
                        let lease = owner.enter(Effect::Provider(attempt))?;
                        let request_id = format!("{}-{}", binding.session, attempt);
                        let socket = store.transport(native::now(), |access| {
                            lease.check_current()?;
                            provider_attempts += 1;
                            connect(addr, &body, access, &request_id, &lease)
                        })?;
                        crash_at("connect", attempt);
                        let killer = socket.try_clone().map_err(|_| "synthetic_clone")?;
                        let mut upstream = match stream::Upstream::open(socket) {
                            Ok(u) => u,
                            Err(code) => {
                                router.settle(
                                    attempt,
                                    Disposition::Uncertain {
                                        reason: code.into(),
                                    },
                                    json!(null),
                                )?;
                                store.record_control("failure", json!({"stage":"upstream-head","code":code,"classification":"provider-attempt-unknown","router":router.evidence()}))?;
                                return Err(code);
                            }
                        };
                        if route::refusal_status(upstream.status) {
                            // Definitive refusal at the head: nothing is relayed.
                            let mut refusal = vec![];
                            while let Some(b) = upstream.next(stream::CHUNK)? {
                                refusal.extend(b);
                                if refusal.len() > 65536 {
                                    return Err("refusal_bound");
                                }
                            }
                            store.check_raw(&refusal)?;
                            let code = serde_json::from_slice::<Value>(&refusal)
                                .ok()
                                .and_then(|v| v["error"]["code"].as_str().map(str::to_owned))
                                .unwrap_or_else(|| "unspecified".into());
                            let status = upstream.status;
                            lease.observed(&hash(
                                &[status.to_be_bytes().as_slice(), &refusal].concat(),
                            ))?;
                            router.settle(
                                attempt,
                                Disposition::Refused { status, code },
                                json!(null),
                            )?;
                            store.record_control(&format!("settle-{attempt:03}"), json!({"attempt":router.attempts().iter().find(|a| a.attempt == attempt),"reservedUnits":router.reserved_units()}))?;
                            continue;
                        }
                        let relay = {
                            // The lease was checked immediately before the request in `connect`.
                            let mut chan = Native {
                                workload: &mut workload,
                                store: &mut store,
                                lease: &lease,
                                frames: &mut frames,
                                checked: std::time::Instant::now(),
                            };
                            stream::relay(
                                &mut chan,
                                upstream,
                                move || {
                                    let _ = killer.shutdown(std::net::Shutdown::Both);
                                },
                                &binding,
                                seq,
                                stream,
                            )
                        };
                        let relay = match relay {
                            Ok(r) => r,
                            Err(code) => {
                                router.settle(
                                    attempt,
                                    Disposition::Uncertain {
                                        reason: code.into(),
                                    },
                                    json!(null),
                                )?;
                                store.record_control("failure", json!({"stage":"stream-relay","code":code,"classification":"provider-attempt-unknown","router":router.evidence()}))?;
                                return Err(code);
                            }
                        };
                        let evidence = relay.evidence();
                        let usage = evidence["usage"].clone();
                        let disposition = if relay.status == 200
                            && matches!(
                                relay.outcome,
                                stream::Outcome::Completed
                                    | stream::Outcome::Cancelled { upstream_end: true }
                            )
                            && usage["completed"] == true
                        {
                            Disposition::Completed
                        } else if let Some(code) = usage["failed"].as_str() {
                            Disposition::Uncertain {
                                reason: format!("provider-failed-after-output:{code}"),
                            }
                        } else {
                            Disposition::Uncertain {
                                reason: format!("{}:{}", relay.status, relay.outcome.label()),
                            }
                        };
                        router.settle(attempt, disposition, usage)?;
                        store.record_control(&format!("settle-{attempt:03}"), json!({"attempt":router.attempts().iter().find(|a| a.attempt == attempt),"reservedUnits":router.reserved_units()}))?;
                        break (lease, request_id, addr, relay, evidence, attempt);
                    };
                    stream_requests += 1;
                    store.record_control(&format!("stream-{:03}", streams.len() + 1), json!({"sequence":seq,"stream":stream,"requestId":request_id,"attempt":attempt,"relay":evidence}))?;
                    if router
                        .attempts()
                        .iter()
                        .find(|a| a.attempt == attempt)
                        .and_then(|a| a.disposition.clone())
                        != Some(Disposition::Completed)
                    {
                        // Lease stays UNKNOWN: no settlement or replay is inferred.
                        // Record the native consumer's next frame to show how the
                        // interruption reached the actual session.
                        if let Ok(next) = receive_checked(&mut workload, &mut store, frames) {
                            let _ = store.record_control("failure", json!({"stage":"stream-not-completed","outcome":relay.outcome.label(),"nativeNextType":next["type"],"nativeLastMessage":next["messages"].as_array().and_then(|m| m.last()).map(|m| json!({"role":m["role"],"stopReason":m["stopReason"],"errorMessage":m["errorMessage"]})),"classification":"provider-attempt-unknown","router":router.evidence()}));
                        }
                        return Err("stream_not_completed");
                    }
                    if sends == 0 {
                        // Same live operation, same exact request identity. The
                        // external endpoint must deduplicate the second receipt.
                        let duplicate = store.transport(native::now(), |access| {
                            provider_attempts += 1;
                            drain(connect(addr, &body, access, &request_id, &lease)?)
                        })?;
                        if duplicate != relay.forwarded {
                            return Err("duplicate_response");
                        }
                    }
                    streams.push(evidence);
                    lease.observed(&relay.sha256())?;
                    sends += 1;
                }
                Some("tool") => {
                    native::joined(&frame, "tool", &binding, seq, stream)?;
                    let id = frame["id"].as_str().ok_or("tool_id")?;
                    if !matches!(frame["name"].as_str(), Some("write" | "bash"))
                        || !frame["parameters"].is_object()
                    {
                        return Err("tool_profile");
                    }
                    let lease = owner.enter(Effect::Tool(id.into()))?;
                    workload.send_effect(&json!({"type":"tool-grant","binding":binding,"sequence":seq,"stream":stream,"id":id,"toolOrdinal":frame["toolOrdinal"],"allow":true}),&lease)?;
                    let result = receive_checked(&mut workload, &mut store, frames)?;
                    frames += 1;
                    native::joined(&result, "tool-observed", &binding, seq, stream)?;
                    store.check(&result)?;
                    if result["id"] != id || result["toolOrdinal"] != frame["toolOrdinal"] {
                        return Err("tool_join");
                    }
                    lease.observed(&hash(&store.check(&result)?))?;
                    tools.push(result);
                }
                Some("snapshot") => {
                    native::joined(&frame, "snapshot", &binding, seq, stream)?;
                    break frame;
                }
                _ => return Err("native_frame_kind"),
            }
        };
        if snap["header"] != header
            || snap["sessionId"] != binding.session
            || snap["observation"]["networkAttempts"] != 0
        {
            return Err("native_observation");
        }
        let messages = snap["messages"].as_array().ok_or("messages")?;
        let text = messages
            .last()
            .and_then(|v| v["content"][0]["text"].as_str())
            .ok_or("assistant_parse")?;
        native_result = native::review_result(text, &store)?;
        if seq == 1 {
            if tools.len() != 2 {
                return Err("native_tool_execution");
            }
            let output = tools[1]["result"]["content"][0]["text"]
                .as_str()
                .ok_or("test_output")?;
            if !output.contains("compiled formula.expr -> dist.mjs")
                || !output.contains("# pass 3")
                || !output.contains("# fail 0")
            {
                return Err("actual_build_test");
            }
            let exports = snap["exports"].as_array().ok_or("export")?;
            if exports.len() != 1 || exports[0]["name"] != "dist.mjs" {
                return Err("export");
            }
            let bytes = B64
                .decode(exports[0]["data"].as_str().ok_or("export")?)
                .map_err(|_| "export")?;
            store.export("dist.mjs", &bytes)?;
        }
        let published = store.publish(
            seq,
            snap["header"].clone(),
            snap["entries"].as_array().ok_or("entries")?.clone(),
            snap["leaf"].as_str().ok_or("leaf")?.into(),
            json!({"observation":snap["observation"],"nativeResult":native_result}),
        ).map(|checked| json!({"ordinal":checked.ordinal(),"sha256":checked.digest(),"entries":checked.entries().len(),"leaf":checked.leaf()}));
        match published {
            Ok(checked) => snapshots.push(checked),
            Err(code) => {
                store.record_control(
                    "failure",
                    json!({"stage":"snapshot-publication","sequence":seq,"code":code}),
                )?;
                return Err(code);
            }
        }
    }
    workload.send(&json!({"type":"close"}))?;
    let closed = receive_checked(&mut workload, &mut store, frames)?;
    frames += 1;
    native::joined(&closed, "closed", &binding, 3, 1)?;
    store.check(&closed)?;
    if closed["observation"]["writeAttempts"] != json!([])
        || closed["observation"]["networkAttempts"] != 0
    {
        return Err("close_observation");
    }
    store.record_control("closed-observation", closed.clone())?;
    let closure = workload.finish(&mut store)?;
    let report = json!({"classification":"offline-actual-native-development","sdk":"1.5.0","binding":binding,"inspection":inspection,"ready":ready,"snapshots":snapshots,"frameReceipts":frames,"providerDispatches":provider_attempts,"providerResponsesConsumed":sends,"duplicateFirstRequest":true,"nativeTools":tools,"streams":streams,"routing":router.evidence(),"nativeResult":native_result,"closure":closure,"postDisposal":closed,"endpointEvidence":"externally root-owned observer; actor does not assert remote counts","sealedRestart":"read-only no reacquisition/no dispatch"});
    store.check(&report)?;
    owner.record_native_result(store.result_publication(report.clone())?)?;
    store.emit_report(report)?;
    let digest = store.head().unwrap().to_owned();
    let expected = store.expected_seal()?;
    let descriptor = expected.custody_descriptor()?;
    // Root remains live with original A; fresh process inherits only the sealed
    // expectation descriptor and root path, no A/source/endpoint capability.
    let read_exe = std::env::current_exe()
        .map_err(|_| "read_exe")?
        .with_file_name("pi_native_read");
    drop(owner);
    drop(store);
    for _ in 0..3 {
        let child = std::process::Command::new(&read_exe)
            .arg(root)
            .arg(descriptor.as_raw_fd().to_string())
            .arg("3")
            .env_clear()
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|_| "read_spawn")?;
        // Reading is performed below after releasing the exclusive writer fence.
        let out = child.wait_with_output().map_err(|_| "read_wait")?;
        if !out.status.success() {
            return Err("read_child");
        }
    }
    let reader = SealedReader::open(root, expected)?;
    let sealed = reader.read(3)?;
    if sealed.digest() != digest {
        return Err("sealed_read");
    };
    Ok(())
}
fn main() {
    match run() {
        Ok(()) => (),
        Err(code) => {
            eprintln!("pi_native_tools_probe: {code}");
            std::process::exit(1)
        }
    }
}
