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

fn dispatch(
    address: std::net::SocketAddr,
    body: &[u8],
    access: &[u8],
    request_id: &str,
    lease: &session_task_admission::pi_pilot::EffectLease,
) -> Result<Vec<u8>> {
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
    let mut b = vec![];
    s.take(131073)
        .read_to_end(&mut b)
        .map_err(|_| "synthetic_read")?;
    if b.len() > 131072 {
        return Err("synthetic_response_bound");
    };
    let at = b
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("synthetic_response")?;
    if !b.starts_with(b"HTTP/1.1 200 OK\r\n") {
        return Err("synthetic_status");
    };
    Ok(b[at + 4..].to_vec())
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
    let addr: std::net::SocketAddr = argv
        .get(4)
        .ok_or("external_observer_required")?
        .parse()
        .map_err(|_| "observer_address")?;
    if !addr.ip().is_loopback() {
        return Err("offline_observer_only");
    }
    let package = Path::new("/home/bline/cbm-work/pi-artifact-handoff-20261113/pi");
    let receiver = Path::new(env!("CARGO_MANIFEST_DIR")).join("receiver/pi-native-tools.mjs");
    let allocation = owner.enter(Effect::Allocation)?;
    let mut workload = native::Workload::create(package, &receiver, &mut store, &allocation)?;
    drop(allocation); // Allocation remains unknown until native UUID is observed.
    let initialization = owner.enter(Effect::Initialization)?;
    initialization.consumption_deadline_millis((native::now() + 120000) / 1000)?;
    workload.send_effect(
        &json!({"type":"init","binding":binding,"sequence":0,"stream":0}),
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
    let mut frames = 1;
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
                    let lease = owner.enter(Effect::Provider(sends + 1))?;
                    let response = store.transport(native::now(), |access| {
                        lease.check_current()?;
                        let request_id = format!("{}-{}", binding.session, sends + 1);
                        provider_attempts += 1;
                        let response = dispatch(addr, &body, access, &request_id, &lease)?;
                        if sends == 0 {
                            // Same live operation, same exact request identity. The
                            // external endpoint must deduplicate the second receipt.
                            provider_attempts += 1;
                            let duplicate = dispatch(addr, &body, access, &request_id, &lease)?;
                            if response != duplicate {
                                return Err("duplicate_response");
                            }
                        }
                        Ok(response)
                    })?;
                    store.check_raw(&response)?;
                    workload.send_effect(&json!({"type":"response","binding":binding,"sequence":seq,"stream":stream,"status":200,"body":B64.encode(&response)}),&lease)?;
                    lease.observed(&hash(&response))?;
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
    let report = json!({"classification":"offline-actual-native-development","sdk":"1.5.0","binding":binding,"inspection":inspection,"ready":ready,"snapshots":snapshots,"frameReceipts":frames,"providerDispatches":provider_attempts,"providerResponsesConsumed":sends,"duplicateFirstRequest":true,"nativeTools":tools,"nativeResult":native_result,"closure":closure,"postDisposal":closed,"endpointEvidence":"externally root-owned observer; actor does not assert remote counts","sealedRestart":"read-only no reacquisition/no dispatch"});
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
        Err(_) => std::process::exit(1),
    }
}
