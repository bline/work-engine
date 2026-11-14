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
    net::{TcpListener, TcpStream},
    path::Path,
    time::Duration,
};
use work_engine_manual_host::pi_native::{self as native, Result};

fn endpoint(listener: TcpListener, expected: String) -> Result<Value> {
    let mut receipts = 0;
    let mut effects = 0;
    let deadline = std::time::Instant::now() + Duration::from_secs(45);
    for i in 1..=2 {
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(v) => break v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() > deadline {
                        return Err("endpoint_accept_deadline");
                    };
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return Err("endpoint_accept"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .map_err(|_| "endpoint_timeout")?;
        let mut bytes = vec![];
        let mut byte = [0];
        while !bytes.ends_with(b"\r\n\r\n") {
            if bytes.len() > 8192 {
                return Err("endpoint_headers");
            };
            stream.read_exact(&mut byte).map_err(|_| "endpoint_read")?;
            bytes.push(byte[0]);
        }
        let header = std::str::from_utf8(&bytes).map_err(|_| "endpoint_utf8")?;
        if !header.starts_with("POST /offline-native HTTP/1.1\r\n")
            || !header.contains(&format!("Authorization: Bearer {expected}\r\n"))
            || header.contains("nonsecret-marker")
        {
            return Err("endpoint_authority");
        };
        let n: usize = header
            .lines()
            .find_map(|s| s.strip_prefix("Content-Length: "))
            .ok_or("endpoint_length")?
            .parse()
            .map_err(|_| "endpoint_length")?;
        if n > native::MAX_REQUEST {
            return Err("endpoint_bound");
        };
        let mut body = vec![0; n];
        stream.read_exact(&mut body).map_err(|_| "endpoint_body")?;
        let body: Value = serde_json::from_slice(&body).map_err(|_| "endpoint_json")?;
        if body["model"] != "gpt-6-luna"
            || body["store"] != false
            || body.get("authorization").is_some()
        {
            return Err("endpoint_projection");
        };
        receipts += 1;
        effects += 1;
        let sse = native::sse(&format!("offline native answer {i}"));
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",sse.len()).map_err(|_|"endpoint_write")?;
        stream.write_all(&sse).map_err(|_| "endpoint_write")?;
    }
    Ok(
        json!({"receipts":receipts,"effects":effects,"classification":"separate-host-synthetic-endpoint"}),
    )
}
fn dispatch(address: std::net::SocketAddr, body: &[u8], access: &[u8]) -> Result<Vec<u8>> {
    let mut s = TcpStream::connect_timeout(&address, Duration::from_secs(5))
        .map_err(|_| "synthetic_connect")?;
    s.set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| "synthetic_write_timeout")?;
    s.set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|_| "synthetic_timeout")?;
    let access = std::str::from_utf8(access).map_err(|_| "synthetic_access")?;
    write!(s,"POST /offline-native HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {access}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).map_err(|_|"synthetic_write")?;
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
    let injected_access = access.clone();
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
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| "endpoint_bind")?;
    listener
        .set_nonblocking(true)
        .map_err(|_| "endpoint_nonblocking")?;
    let addr = listener.local_addr().map_err(|_| "endpoint_addr")?;
    // Endpoint owns its observations outside workload; nonblocking accept is
    // bounded and one controller request corresponds to one accepted effect.

    let endpoint = std::thread::spawn(move || endpoint(listener, access));
    let package = Path::new("/home/bline/cbm-work/pi-artifact-handoff-20261113/pi");
    let receiver = Path::new(env!("CARGO_MANIFEST_DIR")).join("receiver/pi-native.mjs");
    let allocation = owner.enter(Effect::Allocation)?;
    let mut workload = native::Workload::create(package, &receiver, &mut store, &allocation)?;
    drop(allocation); // Allocation remains unknown until native UUID is observed.
    let initialization = owner.enter(Effect::Initialization)?;
    initialization.consumption_deadline_millis((native::now() + 120000) / 1000)?;
    workload.send_effect(
        &json!({"type":"init","binding":binding,"sequence":0,"stream":0}),
        &initialization,
    )?;
    let ready = workload.receive()?;
    native::joined(&ready, "ready", &binding, 0, 0)?;
    if ready["sessionId"] != binding.session || ready["tools"] != json!([]) {
        return Err("native_init");
    };
    store.record_control("ready", ready.clone())?;
    initialization.observed(&hash(&serde_json::to_vec(&ready).map_err(|_| "ready")?))?;
    owner.seal(&job, &binding.session)?;
    owner.check_native(&binding.session)?;
    let inspection = workload.inspect(&store)?;
    let header = ready["header"].clone();
    let mut frames = 1;
    let mut admits = 0;
    let mut sends = 0;
    let mut snapshots = vec![];
    for seq in 1..=2 {
        let predecessor = store.head().map(str::to_owned);
        let pending = store.begin(native::now(), seq, predecessor.as_deref())?;
        let lease = owner.enter(Effect::Provider(seq))?;
        let deadline = lease.consumption_deadline_millis((native::now() + 120000) / 1000)?;
        if deadline <= native::now() as i64 {
            return Err("owner_deadline");
        }
        admits += 1;
        workload.send_effect(&json!({"type":"prompt","binding":binding,"sequence":seq,"stream":0,"prompt":format!("Offline synthetic turn {seq}; respond briefly.")}),&lease)?;
        let fetch = workload.receive()?;
        frames += 1;
        native::joined(&fetch, "fetch", &binding, seq, 1)?;
        let body = native::request(&fetch, &store)?;
        let response = store.dispatch(pending, native::now(), |access| {
            lease.check_current()?;
            if native::now() as i64 >= deadline {
                return Err("owner_deadline");
            }
            dispatch(addr, &body, access)
        })?;
        sends += 1;
        workload.send(&json!({"type":"response","binding":binding,"sequence":seq,"stream":1,"status":200,"body":B64.encode(response)}))?;
        let snap = workload.receive()?;
        frames += 1;
        native::joined(&snap, "snapshot", &binding, seq, 1)?;
        store.check(&snap)?;
        if snap["header"] != header
            || snap["sessionId"] != binding.session
            || snap["observation"]["writeAttempts"] != json!([])
            || snap["observation"]["networkAttempts"] != 0
            || snap["observation"]["scratch"] != json!([])
        {
            return Err("native_sink_observation");
        };
        let messages = snap["messages"].as_array().ok_or("native_messages")?;
        if messages
            .last()
            .and_then(|v| v["content"][0]["text"].as_str())
            != Some(format!("offline native answer {seq}").as_str())
        {
            return Err("native_assistant_parse");
        };
        if seq == 1 {
            for class in [
                "header-key",
                "entry-value",
                "json-escape",
                "nested-jsonl",
                "base64",
                "name",
                "final-metadata",
            ] {
                let mut h = snap["header"].clone();
                let mut e = snap["entries"].as_array().unwrap().clone();
                let mut m = json!({});
                match class {
                    "header-key" => h[&injected_access] = json!(true),
                    "entry-value" => e[0]["injected"] = json!(injected_access),
                    "json-escape" => {
                        let escaped = injected_access
                            .chars()
                            .map(|c| format!("\\u{:04x}", c as u32))
                            .collect::<String>();
                        e[0]["injected"] = serde_json::from_str(&format!("\"{escaped}\""))
                            .map_err(|_| "injection_fixture")?
                    }
                    "nested-jsonl" => {
                        e[0]["injected"] =
                            json!(serde_json::to_string(&json!({"text":injected_access})).unwrap())
                    }
                    "base64" => e[0]["injected"] = json!(B64.encode(&injected_access)),
                    "name" => m = json!({"outputName":injected_access}),
                    _ => m = json!({"final":{"metadata":injected_access}}),
                };
                if store
                    .publish(seq, h, e, snap["leaf"].as_str().unwrap().into(), m)
                    .is_ok()
                    || root.join("snapshot-0001.json").exists()
                {
                    return Err("native_injection_retained");
                };
            }
        }
        let checked = store.publish(
            seq,
            snap["header"].clone(),
            snap["entries"].as_array().ok_or("native_entries")?.clone(),
            snap["leaf"].as_str().ok_or("native_leaf")?.into(),
            json!({"native_observation":snap["observation"],"messages":messages}),
        )?;
        let evidence = checked.digest().to_owned();
        lease.observed(&evidence)?;
        snapshots.push(json!({"ordinal":checked.ordinal(),"sha256":checked.digest(),"entries":checked.entries().len(),"leaf":checked.leaf()}));
    }
    workload.send(&json!({"type":"close"}))?;
    let closed = workload.receive()?;
    frames += 1;
    native::joined(&closed, "closed", &binding, 2, 1)?;
    store.check(&closed)?;
    if closed["observation"]["writeAttempts"] != json!([])
        || closed["observation"]["networkAttempts"] != 0
        || closed["observation"]["scratch"] != json!([])
    {
        return Err("close_observation");
    }
    store.record_control("closed-observation", closed.clone())?;
    let closure = workload.finish(&mut store)?;
    let endpoint = endpoint.join().map_err(|_| "endpoint_join")??;
    let report = json!({"classification":"offline-actual-native-sdk-no-tools","sdk":"1.5.0","binding":binding,"profile":{"maxFrames":6,"sdkPackageEntriesSha256":"6e35d15031b96a65cfc445b1231fd7e1bf3dfe0125734ccfa17fd0dee36c0561","maxFrame":native::MAX_FRAME,"aggregate":native::MAX_AGGREGATE,"maxRequest":native::MAX_REQUEST,"deadlineMs":native::DEADLINE_MS,"image":native::IMAGE},"inspection":inspection,"ready":ready,"snapshots":snapshots,"nativeInjectionRefusals":["header-key","entry-value","json-escape","nested-jsonl","base64","name","final-metadata"],"frameReceipts":frames,"storeAdmissions":admits,"campaignAdmissionOwner":"PiOwner/EffectLease; synthetic fixture authority","dispatches":sends,"endpoint":endpoint,"closure":closure,"postDisposal":closed,"sealedRestart":"read-reconcile-only; no A reacquisition/no endpoint request"});
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
    let child = std::process::Command::new(read_exe)
        .arg(root)
        .arg(descriptor.as_raw_fd().to_string())
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
    let reader = SealedReader::open(root, expected)?;
    let sealed = reader.read(2)?;
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
