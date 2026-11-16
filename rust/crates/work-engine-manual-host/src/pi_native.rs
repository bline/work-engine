//! Offline actual native Pi transport. A workload's URL/headers never select
//! host dispatch. This module does not grant campaign or live provider authority.
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use review_execution_evidence::pi::{Binding, LiveStore};
use serde_json::{Value, json};
use session_task_admission::pi_pilot::{EffectLease, PiOwner};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
pub type Result<T> = std::result::Result<T, &'static str>;
pub const IMAGE: &str = "sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da";
pub const MAX_FRAME: usize = 2 * 1024 * 1024;
pub const MAX_AGGREGATE: usize = 8 * 1024 * 1024;
pub const MAX_REQUEST: usize = 131072;
pub const DEADLINE_MS: u64 = 15000;
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn poll(fd: i32, event: i16, deadline: Instant) -> Result<()> {
    let left = deadline
        .saturating_duration_since(Instant::now())
        .as_millis();
    if left == 0 {
        return Err("pipe_deadline");
    };
    let mut p = libc::pollfd {
        fd,
        events: event,
        revents: 0,
    };
    let n = unsafe { libc::poll(&mut p, 1, left.min(100) as i32) };
    if n < 0 {
        return Err("pipe_poll");
    };
    if n == 0 {
        return poll(fd, event, deadline);
    };
    Ok(())
}
fn exact<R: Read + AsRawFd>(r: &mut R, b: &mut [u8], deadline: Instant) -> Result<()> {
    let mut n = 0;
    while n < b.len() {
        poll(r.as_raw_fd(), libc::POLLIN, deadline)?;
        match r.read(&mut b[n..]) {
            Ok(0) => return Err("pipe_eof"),
            Ok(x) => n += x,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err("pipe_read"),
        }
    }
    Ok(())
}
fn write<W: Write + AsRawFd>(w: &mut W, b: &[u8], deadline: Instant) -> Result<()> {
    let mut n = 0;
    while n < b.len() {
        poll(w.as_raw_fd(), libc::POLLOUT, deadline)?;
        let end = (n + 4096).min(b.len());
        match w.write(&b[n..end]) {
            Ok(0) => return Err("pipe_write_zero"),
            Ok(x) => n += x,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err("pipe_write"),
        }
    }
    Ok(())
}
pub fn encode(v: &Value) -> Result<Vec<u8>> {
    let b = serde_json::to_vec(v).map_err(|_| "frame_json")?;
    if b.is_empty() || b.len() > MAX_FRAME {
        return Err("frame_bound");
    };
    let mut out = (b.len() as u32).to_be_bytes().to_vec();
    out.extend(b);
    Ok(out)
}
pub fn decode<R: Read + AsRawFd>(r: &mut R, remaining: &mut usize) -> Result<Value> {
    let d = Instant::now() + Duration::from_millis(DEADLINE_MS);
    let mut h = [0; 4];
    exact(r, &mut h, d)?;
    let n = u32::from_be_bytes(h) as usize;
    if n == 0 || n > MAX_FRAME || n + 4 > *remaining {
        return Err("frame_bound");
    };
    *remaining -= n + 4;
    let mut b = vec![0; n];
    exact(r, &mut b, d)?;
    review_execution_evidence::pi::parse_json(&b)
}
fn cmd(args: &[String], store: &LiveStore) -> Result<Vec<u8>> {
    cmd_effect(args, store, None)
}
fn cmd_effect(args: &[String], store: &LiveStore, lease: Option<&EffectLease>) -> Result<Vec<u8>> {
    store.check(&json!(args))?;
    if let Some(lease) = lease {
        lease.check_current()?;
    }
    let mut child = Command::new("/usr/bin/docker")
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", "/nonexistent")
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "docker_spawn")?;
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    unsafe {
        libc::fcntl(out.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
        libc::fcntl(err.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK);
    };
    let d = Instant::now() + Duration::from_secs(15);
    let mut b = vec![];
    let mut total = 0;
    loop {
        for (which, r) in [
            (0, &mut out as &mut dyn Read),
            (1, &mut err as &mut dyn Read),
        ] {
            let mut buf = [0; 4096];
            loop {
                match r.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        total += n;
                        if total > 131072 {
                            unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
                            let _ = child.wait();
                            return Err("docker_output_bound");
                        };
                        if which == 0 {
                            b.extend_from_slice(&buf[..n]);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }
        }
        if let Some(status) = child.try_wait().map_err(|_| "docker_wait")? {
            if !status.success() {
                return Err("docker_command_failed");
            };
            store.check_raw(&b)?;
            return Ok(b);
        }
        if Instant::now() >= d {
            unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
            let _ = child.wait();
            return Err("docker_deadline");
        };
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn diff_entries(b: &[u8]) -> Vec<&[u8]> {
    let mut lines: Vec<_> = b.split(|x| *x == b'\n').filter(|s| !s.is_empty()).collect();
    lines.sort();
    lines
}
fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| (*s).into()).collect()
}
// Intent alone never grants ownership. Only daemon-verified immutable IDs can
// reach containment. An unclosed intent remains UNKNOWN, including on Drop.
fn contain(id: &str) -> Result<()> {
    if !valid_id(id) {
        return Err("containment_unknown");
    }
    let mut child = Command::new("/usr/bin/docker")
        .args(["rm", "-f", id])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "containment_unknown")?;
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.try_wait().map_err(|_| "containment_unknown")? {
            return if status.success() {
                Ok(())
            } else {
                Err("containment_unknown")
            };
        }
        if Instant::now() >= deadline {
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
            return Err("containment_unknown");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn valid_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
struct AcquisitionIntent {
    name: String,
    attempt: String,
    digest: String,
}
impl AcquisitionIntent {
    fn new(name: String, create_args: &[String]) -> Result<Self> {
        let mut nonce = [0u8; 32];
        std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(&mut nonce))
            .map_err(|_| "acquisition_random")?;
        let attempt = nonce.iter().map(|b| format!("{b:02x}")).collect();
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(create_args).map_err(|_| "acquisition_intent")?)
        );
        Ok(Self {
            name,
            attempt,
            digest,
        })
    }
    fn verify(&self, id: &str, metadata: &Value) -> Result<()> {
        let c = metadata
            .as_array()
            .filter(|v| v.len() == 1)
            .and_then(|v| v.first())
            .ok_or("acquisition_unknown")?;
        if !valid_id(id)
            || c["Id"] != id
            || c["Name"] != format!("/{}", self.name)
            || c["Image"] != IMAGE
            || c["Config"]["Image"] != IMAGE
            || c["Config"]["Labels"]["work-engine.attempt"] != self.attempt
            || c["Config"]["Labels"]["work-engine.intent"] != self.digest
            || c["Config"]["Labels"]["work-engine.owner"] != "01a1db17-native"
            || c["Created"].as_str().is_none_or(str::is_empty)
            || c["State"]["Status"] != "created"
            || c["Config"]["Cmd"] != json!(["node", "--no-warnings", "/receiver.mjs"])
            || c["HostConfig"]["NetworkMode"] != "none"
            || c["HostConfig"]["ReadonlyRootfs"] != true
        {
            return Err("acquisition_unknown");
        }
        Ok(())
    }
}
// Both the ACK path and bounded lost-ACK reconciliation use this exact join.
// Empty or ambiguous observations do not close a possibly outstanding create.
fn attributable(
    intent: &AcquisitionIntent,
    create: Result<Vec<u8>>,
    mut observe: impl FnMut(Option<&str>) -> Result<Vec<(String, Value)>>,
) -> Result<(String, bool)> {
    let acknowledged = create
        .as_ref()
        .ok()
        .and_then(|b| std::str::from_utf8(b).ok())
        .map(str::trim)
        .filter(|id| valid_id(id));
    let observations = observe(acknowledged).map_err(|e| {
        if e == "acquisition_foreign_observed_unknown" {
            e
        } else {
            "acquisition_unknown"
        }
    })?;
    if observations.len() != 1 {
        return Err("acquisition_unknown");
    }
    let (id, metadata) = observations.into_iter().next().unwrap();
    if acknowledged.is_some_and(|ack| ack != id) {
        return Err("acquisition_unknown");
    }
    intent.verify(&id, &metadata)?;
    Ok((id, acknowledged.is_some()))
}
fn observe_acquisition(
    intent: &AcquisitionIntent,
    ack: Option<&str>,
    store: &LiveStore,
) -> Result<Vec<(String, Value)>> {
    let ids = if let Some(id) = ack {
        vec![id.to_owned()]
    } else {
        let raw = cmd(
            &args(&[
                "ps",
                "--all",
                "--quiet",
                "--no-trunc",
                "--filter",
                &format!("label=work-engine.attempt={}", intent.attempt),
            ]),
            store,
        )?;
        std::str::from_utf8(&raw)
            .map_err(|_| "acquisition_unknown")?
            .lines()
            .map(str::to_owned)
            .collect()
    };
    if ids.is_empty() && ack.is_none() {
        // Observing a foreign holder does not settle an outstanding create.
        // It grants neither cleanup of that holder nor an automatic retry.
        if let Ok(raw) = cmd(&args(&["inspect", &intent.name]), store) {
            if let Ok(metadata) = review_execution_evidence::pi::parse_json(&raw) {
                if metadata[0]["Id"].as_str().is_some_and(valid_id)
                    && metadata[0]["Config"]["Labels"]["work-engine.attempt"] != intent.attempt
                {
                    return Err("acquisition_foreign_observed_unknown");
                }
            }
        }
    }
    if ids.len() != 1 || !valid_id(&ids[0]) {
        return Err("acquisition_unknown");
    }
    let raw = cmd(&args(&["inspect", &ids[0]]), store)?;
    let metadata = review_execution_evidence::pi::parse_json(&raw)?;
    Ok(vec![(ids[0].clone(), metadata)])
}
fn cleanup_after<T>(
    id: &str,
    error: &'static str,
    remove: impl FnOnce(&str) -> Result<()>,
) -> Result<T> {
    remove(id)?;
    Err(error)
}
pub struct Workload {
    id: String,
    child: Child,
    input: ChildStdin,
    output: ChildStdout,
    stderr: Option<std::thread::JoinHandle<Vec<u8>>>,
    remaining: usize,
    baseline_diff: Vec<u8>,
    closed: bool,
}
impl Workload {
    pub fn create(
        package: &Path,
        receiver: &Path,
        store: &mut LiveStore,
        lease: &EffectLease,
    ) -> Result<Self> {
        let binding = store.binding().clone();
        lease.check_current()?;
        let name = format!("we-pi-native-{}", binding.session);
        let mut a = args(&[
            "create",
            "--pull=never",
            "--name",
            &name,
            "--label",
            "work-engine.owner=01a1db17-native",
            "--network=none",
            "--read-only",
            "--log-driver=none",
            "--user",
            "1000:1000",
            "--cap-drop=ALL",
            "--security-opt",
            "no-new-privileges",
            "--pids-limit",
            "64",
            "--memory",
            "512m",
            "--memory-swap",
            "512m",
            "--cpus",
            "1",
            "--ulimit",
            "core=0:0",
            "--tmpfs",
            "/scratch:rw,nosuid,nodev,noexec,size=16777216,uid=1000,gid=1000,mode=700",
            "--workdir",
            "/scratch",
            "--env",
            "HOME=/scratch",
            "--env",
            "NODE_ENV=production",
            "--interactive",
        ]);
        for (src, dst) in [(package, "/pi"), (receiver, "/receiver.mjs")] {
            a.extend(args(&[
                "--mount",
                &format!("type=bind,src={},dst={dst},readonly", src.display()),
            ]));
        }
        a.extend(args(&[IMAGE, "node", "--no-warnings", "/receiver.mjs"]));
        let intent = AcquisitionIntent::new(name, &a)?;
        a.splice(
            1..1,
            args(&[
                "--label",
                &format!("work-engine.attempt={}", intent.attempt),
                "--label",
                &format!("work-engine.intent={}", intent.digest),
            ]),
        );
        // Create-only publication fences retries even if an ACK, inspection,
        // cleanup, or later publication is lost. Pending means UNKNOWN.
        store.record_control(
            "create-intent",
            json!({"args":a,"attempt":intent.attempt,
            "intent":intent.digest,"classification":"acquisition-and-cleanup-pending-unknown"}),
        )?;
        lease.check_current()?;
        let created = cmd_effect(&a, store, Some(lease));
        let (id, acknowledged) = attributable(&intent, created, |ack| {
            observe_acquisition(&intent, ack, store)
        })?;
        if !acknowledged {
            return cleanup_after(&id, "create_unacknowledged_contained", contain);
        }
        if let Err(e) = store.record_control(
            "created",
            json!({"id":id,"attempt":intent.attempt,
            "intent":intent.digest,"classification":"verified-acquired-cleanup-pending"}),
        ) {
            return cleanup_after(&id, e, contain);
        }
        if let Err(e) = lease.check_current() {
            return cleanup_after(&id, e, contain);
        }
        let attach = Command::new("/usr/bin/docker")
            .args(["start", "--attach", "--interactive", &id])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", "/nonexistent")
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match attach {
            Ok(c) => c,
            Err(_) => {
                return cleanup_after(&id, "docker_attach_spawn", contain);
            }
        };
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let mut err = child.stderr.take().unwrap();
        let stderr = std::thread::spawn(move || {
            let mut data = Vec::new();
            let mut b = [0; 4096];
            while let Ok(n) = err.read(&mut b) {
                if n == 0 {
                    break;
                }
                data.extend_from_slice(&b[..n]);
                if data.len() > 65536 {
                    break;
                }
            }
            data
        });
        Ok(Self {
            id,
            child,
            input,
            output,
            stderr: Some(stderr),
            remaining: MAX_AGGREGATE,
            baseline_diff: vec![],
            closed: false,
        })
    }
    pub fn send(&mut self, v: &Value) -> Result<()> {
        let b = encode(v)?;
        if b.len() > self.remaining {
            return Err("aggregate_bound");
        };
        self.remaining -= b.len();
        write(
            &mut self.input,
            &b,
            Instant::now() + Duration::from_millis(DEADLINE_MS),
        )
    }
    pub fn send_effect(&mut self, v: &Value, lease: &EffectLease) -> Result<()> {
        let b = encode(v)?;
        if b.len() > self.remaining {
            return Err("aggregate_bound");
        }
        self.remaining -= b.len();
        lease.check_current()?;
        write(
            &mut self.input,
            &b,
            Instant::now() + Duration::from_millis(DEADLINE_MS),
        )
    }
    pub fn receive(&mut self) -> Result<Value> {
        decode(&mut self.output, &mut self.remaining)
    }
    /// Nonblocking readiness of the native output (data, EOF or error).
    pub fn readable(&mut self) -> Result<bool> {
        let mut p = libc::pollfd {
            fd: self.output.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let n = unsafe { libc::poll(&mut p, 1, 0) };
        if n < 0 {
            return Err("pipe_poll");
        }
        Ok(n > 0)
    }
    pub fn inspect(&mut self, store: &LiveStore) -> Result<Value> {
        let b = cmd(&args(&["inspect", &self.id]), store)?;
        let v: Value = serde_json::from_slice(&b).map_err(|_| "inspect_json")?;
        self.baseline_diff = cmd(&args(&["diff", &self.id]), store)?;
        let c = &v[0];
        if c["Image"] != IMAGE
            || c["HostConfig"]["NetworkMode"] != "none"
            || c["HostConfig"]["ReadonlyRootfs"] != true
            || c["HostConfig"]["LogConfig"]["Type"] != "none"
            || c["Config"]["Tty"] != false
            || c["Mounts"]
                .as_array()
                .ok_or("mounts")?
                .iter()
                .any(|m| m["RW"] != false)
        {
            return Err("container_profile");
        };
        Ok(
            json!({"id":self.id,"image":IMAGE,"network":"none","logging":"none","readonly":true,"mounts":c["Mounts"],"memory":c["HostConfig"]["Memory"],"memorySwap":c["HostConfig"]["MemorySwap"],"pids":c["HostConfig"]["PidsLimit"],"baselineDiff":String::from_utf8_lossy(&self.baseline_diff)}),
        )
    }
    pub fn finish(mut self, store: &mut LiveStore) -> Result<Value> {
        let d = Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(s) = self.child.try_wait().map_err(|_| "attach_wait")? {
                break s;
            }
            if Instant::now() > d {
                return Err("attach_exit_deadline");
            };
            std::thread::sleep(Duration::from_millis(5));
        };
        let stderr = self
            .stderr
            .take()
            .unwrap()
            .join()
            .map_err(|_| "stderr_join")?;
        let diff = cmd(&args(&["diff", &self.id]), store)?;
        contain(&self.id)?;
        self.closed = true;
        let remaining = cmd(
            &args(&[
                "ps",
                "--all",
                "--quiet",
                "--filter",
                "label=work-engine.owner=01a1db17-native",
            ]),
            store,
        )?;
        if !remaining.is_empty() {
            return Err("owned_containers_remaining");
        };
        store.check_raw(&stderr)?;
        store.check_raw(&diff)?;
        let closure = json!({"container":self.id,"attachExit":status.code(),"stderrBytes":stderr.len(),"stderr":String::from_utf8_lossy(&stderr),"dockerDiff":String::from_utf8_lossy(&diff),"dockerDiffBytes":diff.len(),"removed":true,"ownedLabelContainersAfter":0});
        store.record_control("closure", closure.clone())?;
        if !status.success() {
            return Err("native_exit");
        };
        if !stderr.is_empty() {
            return Err("native_stderr");
        };
        if diff_entries(&diff) != diff_entries(&self.baseline_diff) {
            return Err("native_filesystem");
        };
        Ok(closure)
    }
}
impl Drop for Workload {
    fn drop(&mut self) {
        if !self.closed {
            unsafe { libc::kill(-(self.child.id() as i32), libc::SIGKILL) };
            let _ = self.child.wait();
            // Best effort only: the durable acquisition record remains pending;
            // Drop never emits a successful closure or settles UNKNOWN.
            let _ = contain(&self.id);
            if let Some(h) = self.stderr.take() {
                let _ = h.join();
            }
        }
    }
}
pub fn joined(v: &Value, kind: &str, binding: &Binding, sequence: u64, stream: u64) -> Result<()> {
    if v["type"] != kind
        || v["binding"] != serde_json::to_value(binding).unwrap()
        || v["sequence"] != sequence
        || v["stream"] != stream
    {
        return Err("foreign_or_stale_frame");
    };
    Ok(())
}
pub fn request(v: &Value, store: &LiveStore) -> Result<Vec<u8>> {
    if v["method"] != "POST" || v["headers"].as_object().map_or(true, |h| h.len() > 32) {
        return Err("request_headers");
    };
    let text = v["body"].as_str().ok_or("request_body")?;
    if text.len() > MAX_REQUEST * 2 {
        return Err("request_compressed_bound");
    };
    let raw = B64.decode(text).map_err(|_| "request_base64")?;
    if raw.len() > MAX_REQUEST {
        return Err("request_compressed_bound");
    };
    let decoded = match v["headers"]["content-encoding"].as_str() {
        None => raw,
        Some("zstd") => {
            let decoder =
                zstd::stream::read::Decoder::new(raw.as_slice()).map_err(|_| "zstd_header")?;
            let mut b = vec![];
            decoder
                .take((MAX_REQUEST + 1) as u64)
                .read_to_end(&mut b)
                .map_err(|_| "zstd_decode")?;
            if b.len() > MAX_REQUEST {
                return Err("request_decompressed_bound");
            };
            b
        }
        _ => return Err("request_encoding"),
    };
    let body: Value = review_execution_evidence::pi::parse_json(&decoded)?;
    store.check(&body)?;
    if body["model"] != "gpt-6-luna"
        || body["stream"] != true
        || body["tools"].as_array().is_some_and(|x| {
            x.len() > 2
                || x.iter().any(|t| {
                    t["type"] != "function"
                        || !matches!(t["name"].as_str(), Some("write" | "bash"))
                        || !t["parameters"].is_object()
                })
        })
        || body["input"].as_array().map_or(true, |x| x.len() > 64)
    {
        return Err("request_schema");
    };
    for item in body["input"].as_array().unwrap() {
        match item["type"].as_str() {
            Some("additional_tools")
                if item["role"] == "developer"
                    && item["tools"].as_array().is_some_and(|tools| {
                        tools.len() <= 2
                            && tools.iter().all(|tool| {
                                tool["type"] == "function"
                                    && matches!(tool["name"].as_str(), Some("write" | "bash"))
                                    && tool["description"].is_string()
                                    && tool["parameters"].is_object()
                                    && tool["strict"].is_boolean()
                            })
                    }) => {}
            Some("function_call")
                if item["call_id"].is_string()
                    && matches!(item["name"].as_str(), Some("write" | "bash"))
                    && item["arguments"].as_str().is_some_and(|s| {
                        review_execution_evidence::pi::parse_json(s.as_bytes())
                            .is_ok_and(|v| v.is_object())
                    }) =>
            {
                ()
            }
            Some("function_call_output")
                if item["call_id"].is_string() && item["output"].is_string() =>
            {
                ()
            }
            None | Some("message") => {
                if !matches!(item["role"].as_str(), Some("user" | "assistant"))
                    || item["content"].as_array().map_or(true, |x| {
                        x.len() > 64
                            || x.iter().any(|c| {
                                !matches!(c["type"].as_str(), Some("input_text" | "output_text"))
                                    || !c["text"].is_string()
                            })
                    })
                {
                    return Err("request_content");
                }
            }
            _ => return Err("request_item"),
        }
    }
    // Reconstruct selected authority; workload URL, auth, account and arbitrary
    // header values are never forwarded to the synthetic endpoint.
    store.check(&json!({"model":"gpt-6-luna","input":body["input"],"instructions":body["instructions"],"tools":body["tools"],"stream":true,"store":false}))
}
pub fn sse(text: &str) -> Vec<u8> {
    let item = json!({"type":"message","id":"msg_offline","role":"assistant","status":"completed","content":[{"type":"output_text","text":text,"annotations":[]}]});
    let events = [
        json!({"type":"response.created","response":{"id":"resp_offline","status":"in_progress"}}),
        json!({"type":"response.output_item.added","output_index":0,"item":{"type":"message","id":"msg_offline","role":"assistant","status":"in_progress","content":[]}}),
        json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":text}),
        json!({"type":"response.output_item.done","output_index":0,"item":item}),
        json!({"type":"response.completed","response":{"id":"resp_offline","status":"completed","output":[item],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}}),
    ];
    events
        .iter()
        .map(|v| format!("data: {v}\n\n"))
        .collect::<String>()
        .into_bytes()
}

/// Actual native text consumer retains the core result-contract authority.
/// Exclusion and duplicate-sensitive parsing precede the semantic parser.
pub fn review_result(text: &str, store: &LiveStore) -> Result<Value> {
    let v = review_execution_evidence::pi::parse_json(text.as_bytes())?;
    store.check(&v)?;
    let value = review_episode_core::codec::parse_json(text).map_err(|_| "native_review_json")?;
    review_episode_core::implementation_review_v1::ReviewResult::parse(value)
        .map_err(|_| "native_review_contract")?;
    Ok(v)
}

#[cfg(test)]
mod acquisition_tests {
    use super::*;
    fn intent() -> AcquisitionIntent {
        AcquisitionIntent {
            name: "owned-test".into(),
            attempt: "attempt-specific".into(),
            digest: "checked-intent".into(),
        }
    }
    fn metadata(id: &str, i: &AcquisitionIntent) -> Value {
        json!([{"Id":id,"Name":format!("/{}",i.name),"Image":IMAGE,"Created":"2026-11-14T00:00:00Z",
            "Config":{"Image":IMAGE,"Cmd":["node","--no-warnings","/receiver.mjs"],"Labels":{
                "work-engine.attempt":i.attempt,"work-engine.intent":i.digest,"work-engine.owner":"01a1db17-native"}},
            "State":{"Status":"created"},"HostConfig":{"NetworkMode":"none","ReadonlyRootfs":true}}])
    }
    #[test]
    fn foreign_conflict_and_empty_observation_never_acquire() {
        let i = intent();
        let id = "a".repeat(64);
        let mut foreign = metadata(&id, &i);
        foreign[0]["Config"]["Labels"]["work-engine.attempt"] = json!("another-attempt");
        assert_eq!(
            attributable(&i, Err("docker_command_failed"), |_| Ok(vec![(
                id.clone(),
                foreign.clone()
            )])),
            Err("acquisition_unknown")
        );
        for e in [
            "docker_deadline",
            "docker_command_failed",
            "pi_original",
            "docker_output_bound",
        ] {
            assert_eq!(
                attributable(&i, Err(e), |_| Ok(vec![])),
                Err("acquisition_unknown")
            );
        }
    }
    #[test]
    fn lost_ack_and_malformed_response_reconcile_but_cannot_start() {
        let i = intent();
        let id = "b".repeat(64);
        for response in [
            Err("docker_deadline"),
            Ok(b"not-an-id".to_vec()),
            Err("pi_original"),
        ] {
            let (acquired, ack) = attributable(&i, response, |arg| {
                assert!(arg.is_none());
                Ok(vec![(id.clone(), metadata(&id, &i))])
            })
            .unwrap();
            assert!(!ack);
            let mut removed = vec![];
            assert_eq!(
                cleanup_after::<()>(&acquired, "create_unacknowledged_contained", |id| {
                    removed.push(id.to_owned());
                    Ok(())
                }),
                Err("create_unacknowledged_contained")
            );
            assert_eq!(removed, vec![id.clone()]);
        }
    }
    #[test]
    fn immutable_id_and_creation_metadata_join_required() {
        let i = intent();
        let id = "c".repeat(64);
        let raw = format!("{id}\n").into_bytes();
        let got = attributable(&i, Ok(raw.clone()), |arg| {
            assert_eq!(arg, Some(id.as_str()));
            Ok(vec![(id.clone(), metadata(&id, &i))])
        })
        .unwrap();
        assert_eq!(got, (id.clone(), true));
        let other = "d".repeat(64);
        assert!(
            attributable(&i, Ok(raw.clone()), |_| Ok(vec![(
                other.clone(),
                metadata(&other, &i)
            )]))
            .is_err()
        );
        for key in ["Name", "Image", "Created"] {
            let mut bad = metadata(&id, &i);
            bad[0][key] = json!("");
            assert!(i.verify(&id, &bad).is_err());
        }
        let mut bad = metadata(&id, &i);
        bad[0]["Config"]["Labels"]["work-engine.intent"] = json!("foreign");
        assert!(i.verify(&id, &bad).is_err());
        assert!(
            attributable(&i, Ok(raw), |_| Ok(vec![
                (id.clone(), metadata(&id, &i)),
                (other.clone(), metadata(&other, &i))
            ]))
            .is_err()
        );
    }
    #[test]
    fn publication_or_currentness_failure_cleans_id_and_remove_failure_is_unknown() {
        let id = "e".repeat(64);
        for cause in [
            "pi_original",
            "pi_path",
            "pi_lease_expired",
            "docker_attach_spawn",
        ] {
            assert_eq!(
                cleanup_after::<()>(&id, cause, |got| {
                    assert_eq!(got, id);
                    Ok(())
                }),
                Err(cause)
            );
            assert_eq!(
                cleanup_after::<()>(&id, cause, |_| Err("containment_unknown")),
                Err("containment_unknown")
            );
        }
        assert_eq!(contain("requested-name"), Err("containment_unknown"));
    }

    #[test]
    #[ignore = "uses existing local Docker image and newly owned synthetic containers only"]
    fn docker_conflict_lost_ack_name_replacement_and_remove_failure() {
        let t = tempfile::tempdir().unwrap();
        let store = LiveStore::create(
            &t.path().join("store"),
            b"synthetic-original-only-host-value".to_vec(),
            Binding::native(
                "11111111-1111-4111-8111-111111111111".into(),
                "cleanup-control".into(),
                1,
            ),
            u64::MAX,
        )
        .unwrap();
        struct Resources(Vec<String>);
        impl Drop for Resources {
            fn drop(&mut self) {
                for id in &self.0 {
                    let _ = contain(id);
                }
            }
        }
        let mut resources = Resources(vec![]);
        let unique = AcquisitionIntent::new("test".into(), &[]).unwrap().attempt;
        let name = format!("we-ft02-{unique}");
        let make = |name: &str| {
            let mut a = args(&[
                "create",
                "--pull=never",
                "--name",
                name,
                "--label",
                "work-engine.owner=01a1db17-native",
                "--network=none",
                "--read-only",
                IMAGE,
                "node",
                "--no-warnings",
                "/receiver.mjs",
            ]);
            let i = AcquisitionIntent::new(name.into(), &a).unwrap();
            a.splice(
                1..1,
                args(&[
                    "--label",
                    &format!("work-engine.attempt={}", i.attempt),
                    "--label",
                    &format!("work-engine.intent={}", i.digest),
                ]),
            );
            (i, a)
        };
        let (foreign_i, foreign_a) = make(&name);
        let foreign_raw = cmd(&foreign_a, &store).unwrap();
        let foreign = std::str::from_utf8(&foreign_raw).unwrap().trim().to_owned();
        resources.0.push(foreign.clone());
        let (conflict_i, conflict_a) = make(&name);
        let conflict = cmd(&conflict_a, &store);
        assert!(conflict.is_err());
        assert_eq!(
            attributable(&conflict_i, conflict, |ack| observe_acquisition(
                &conflict_i,
                ack,
                &store
            )),
            Err("acquisition_foreign_observed_unknown")
        );
        // The actual pre-existing container remains intact after failed create.
        foreign_i
            .verify(
                &foreign,
                &observe_acquisition(&foreign_i, Some(&foreign), &store).unwrap()[0].1,
            )
            .unwrap();
        contain(&foreign).unwrap();
        let (i, a) = make(&name);
        let raw = cmd(&a, &store).unwrap();
        let id = std::str::from_utf8(&raw).unwrap().trim().to_owned();
        resources.0.push(id.clone());
        let (lost, ack) = attributable(&i, Err("docker_deadline"), |a| {
            observe_acquisition(&i, a, &store)
        })
        .unwrap();
        assert_eq!(lost, id);
        assert!(!ack);
        let renamed = format!("{name}-old");
        cmd(&args(&["rename", &id, &renamed]), &store).unwrap();
        let (replacement_i, replacement_a) = make(&name);
        let replacement_raw = cmd(&replacement_a, &store).unwrap();
        let replacement = std::str::from_utf8(&replacement_raw)
            .unwrap()
            .trim()
            .to_owned();
        resources.0.push(replacement.clone());
        assert_eq!(
            cleanup_after::<()>(&id, "publication_denied", contain),
            Err("publication_denied")
        );
        replacement_i
            .verify(
                &replacement,
                &observe_acquisition(&replacement_i, Some(&replacement), &store).unwrap()[0].1,
            )
            .unwrap();
        // Docker rm -f is idempotent for an absent immutable ID. Actual
        // removal failure is injected by the focused control above.
        assert_eq!(contain(&id), Ok(()));
        contain(&replacement).unwrap();
        for id in &resources.0 {
            assert!(cmd(&args(&["inspect", id]), &store).is_err());
        }
        println!(
            "{}",
            json!({"control":"ft02-actual-docker","foreignPreserved":foreign,"lostAckReconciled":id,
            "replacementPreserved":replacement,"absentRemoval":"idempotent-success","leftovers":0,"image":IMAGE})
        );
    }
}
