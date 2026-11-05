//! Bounded coding-job adapter over the existing Pi Host (`work-engine-manual-host pi-docker`).
//!
//! The Host stays the owner of coding admission, effects, custody and receipts. This module only:
//! authorizes one operator-pinned exact job for one explicitly provisioned caller, binds the
//! original public request to that job before any possible execution, owns one collector process
//! group beyond client disconnect, and projects Host results without upgrading them.
//!
//! Authority is deliberately separate from transport authentication and from session-message
//! grants: the caller presents a job credential whose SHA-256 the operator pinned in the profile.
//! No client path, job JSON, executable or hash is ever consumed from a request.
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawFd,
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const CAPABILITY_OFFLINE: &str = "coding-job-offline-fixture-v1";
const SCHEMA: &str = "work-engine.coding-job.v1";
const PROFILE_KIND: &str = "coding-job-profile";
const CLASS_OFFLINE: &str = "offline-fixture";
const OUTPUT_LIMIT: usize = 1024 * 1024;
const JOB_FILE_LIMIT: usize = 64 * 1024;
const QUICK_TIMEOUT: Duration = Duration::from_secs(20);

pub fn operation(name: &str) -> bool {
    matches!(
        name,
        "submit_coding_job"
            | "read_coding_job_operation"
            | "read_coding_job_result"
            | "cancel_coding_job"
    )
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Authorization {
    authorization_id: String,
    caller: String,
    /// SHA-256 of the operator-provisioned job credential for this caller and this job only.
    credential_sha256: String,
    job_file: String,
    job_sha256: String,
    packet_sha256: String,
    job_id: String,
    state_dir: String,
}
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct JobProfile {
    version: u8,
    kind: String,
    capability: String,
    class: String,
    host_binary: String,
    host_binary_sha256: String,
    operation_root: String,
    #[serde(default)]
    env: BTreeMap<String, String>,
    collector_deadline_seconds: u64,
    cancel_grace_seconds: u64,
    authorizations: Vec<Authorization>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Submit {
    request_id: String,
    authorization_id: String,
    job_credential: String,
    job_sha256: String,
    packet_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadArgs {
    request_id: String,
    authorization_id: String,
    job_credential: String,
}

fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
fn private_dir(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .is_ok_and(|m| m.is_dir() && m.uid() == unsafe { libc::geteuid() } && m.mode() & 0o077 == 0)
}
fn write_atomic(path: &Path, value: &Value) -> Result<(), &'static str> {
    let tmp = path.with_extension("tmp");
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)
        .map_err(|_| "owner_unavailable")?;
    f.write_all(value.to_string().as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|_| "owner_unavailable")?;
    std::fs::rename(&tmp, path).map_err(|_| "owner_unavailable")?;
    File::open(path.parent().ok_or("owner_unavailable")?)
        .and_then(|d| d.sync_all())
        .map_err(|_| "owner_unavailable")
}
fn write_new_bytes(path: &Path, bytes: &[u8]) -> Result<(), &'static str> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| "owner_unavailable")?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|_| "owner_unavailable")?;
    File::open(path.parent().ok_or("owner_unavailable")?)
        .and_then(|d| d.sync_all())
        .map_err(|_| "owner_unavailable")
}
fn write_new(path: &Path, value: &Value) -> Result<(), &'static str> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                "already_exists"
            } else {
                "owner_unavailable"
            }
        })?;
    f.write_all(value.to_string().as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|_| "owner_unavailable")?;
    File::open(path.parent().ok_or("owner_unavailable")?)
        .and_then(|d| d.sync_all())
        .map_err(|_| "owner_unavailable")
}
fn receipt_doc_is_reconcile(dir: &Path) -> bool {
    read_json(&dir.join("host-reconcile.json"))
        .ok()
        .flatten()
        .is_some_and(|v| v.get("receipt").is_some() || v.get("status").is_some())
}
fn read_json(path: &Path) -> Result<Option<Value>, &'static str> {
    match session_query::read_file(&path.to_string_lossy(), OUTPUT_LIMIT + 4096, true) {
        Ok(b) => serde_json::from_slice(&b)
            .map(Some)
            .map_err(|_| "custody_lost"),
        Err("file_unavailable") if !path.exists() => Ok(None),
        Err(_) => Err("custody_lost"),
    }
}

/// Outcome of one bounded Host child, retained verbatim for truthful projection.
struct Child {
    exit_code: Option<i32>,
    exit_signal: Option<i32>,
    timed_out: bool,
    stopped: bool,
    cancel_marker_requested: bool,
    killed: bool,
    waited: bool,
    stdout: Vec<u8>,
    stdout_truncated: bool,
    stderr_bytes: usize,
    readers_joined: bool,
}

struct Collector {
    handle: JoinHandle<()>,
    done: Arc<AtomicBool>,
}

pub struct Jobs {
    profile: JobProfile,
    stop: Arc<AtomicBool>,
    collectors: Mutex<HashMap<String, Collector>>,
}

impl JobProfile {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != 1
            || self.kind != PROFILE_KIND
            || self.capability != CAPABILITY_OFFLINE
            || self.class != CLASS_OFFLINE
            || !Path::new(&self.host_binary).is_absolute()
            || !is_hex64(&self.host_binary_sha256)
            || !Path::new(&self.operation_root).is_absolute()
            || !(1..=900).contains(&self.collector_deadline_seconds)
            || !(1..=60).contains(&self.cancel_grace_seconds)
            || self.authorizations.is_empty()
            || self.authorizations.len() > 4
            || self
                .env
                .iter()
                .any(|(k, v)| !is_id(k) || v.len() > 1024 || v.contains('\0'))
            || self
                .env
                .keys()
                .any(|k| k.contains("TOKEN") || k.contains("KEY"))
        {
            return Err("invalid_job_profile");
        }
        let mut ids = std::collections::BTreeSet::new();
        for a in &self.authorizations {
            if !is_id(&a.authorization_id)
                || !is_id(&a.caller)
                || !is_id(&a.job_id)
                || !is_hex64(&a.credential_sha256)
                || !is_hex64(&a.job_sha256)
                || !is_hex64(&a.packet_sha256)
                || !Path::new(&a.job_file).is_absolute()
                || !Path::new(&a.state_dir).is_absolute()
                || !ids.insert(a.authorization_id.clone())
            {
                return Err("invalid_job_profile");
            }
        }
        if !private_dir(Path::new(&self.operation_root)) {
            return Err("invalid_job_profile");
        }
        Ok(())
    }
}

impl Jobs {
    pub fn new(profile: JobProfile) -> Result<Self, &'static str> {
        profile.validate()?;
        let jobs = Self {
            profile,
            stop: Arc::new(AtomicBool::new(false)),
            collectors: Mutex::new(HashMap::new()),
        };
        jobs.host_binary()?;
        Ok(jobs)
    }

    fn host_binary(&self) -> Result<(), &'static str> {
        let bytes = session_query::read_file(&self.profile.host_binary, 512 * 1024 * 1024, false)
            .map_err(|_| "runtime_unavailable")?;
        if hex(&bytes) != self.profile.host_binary_sha256 {
            return Err("runtime_pin_mismatch");
        }
        Ok(())
    }

    /// Dispatches one job operation. All outcomes are projected, none rewrites Host authority.
    pub fn call(self: &Arc<Self>, name: &str, arguments: Value) -> Result<Value, &'static str> {
        match name {
            "submit_coding_job" => {
                let a: Submit =
                    serde_json::from_value(arguments).map_err(|_| "invalid_arguments")?;
                self.submit(a)
            }
            "read_coding_job_operation" | "read_coding_job_result" | "cancel_coding_job" => {
                let a: ReadArgs =
                    serde_json::from_value(arguments).map_err(|_| "invalid_arguments")?;
                let auth = self.authorize(&a.authorization_id, &a.job_credential)?;
                let (dir, binding) = self.bound(&auth, &a.request_id)?;
                match name {
                    "read_coding_job_operation" => self.read_operation(&auth, &dir, &binding),
                    "read_coding_job_result" => self.read_result(&auth, &dir, &binding),
                    _ => self.cancel(&auth, &dir, &binding),
                }
            }
            _ => Err("invalid_request"),
        }
    }

    /// Caller-to-job authority. Unknown authorization and wrong credential are indistinguishable.
    fn authorize(&self, id: &str, credential: &str) -> Result<Authorization, &'static str> {
        let found = self
            .profile
            .authorizations
            .iter()
            .find(|a| a.authorization_id == id);
        let supplied = hex(credential.as_bytes());
        let expected = found.map(|a| a.credential_sha256.as_str()).unwrap_or("");
        let mut diff = supplied.len() ^ expected.len();
        for (i, b) in supplied.bytes().enumerate() {
            diff |= usize::from(b ^ expected.as_bytes().get(i).copied().unwrap_or(0));
        }
        if credential.len() < 32
            || credential.len() > 256
            || credential.bytes().any(|b| !b.is_ascii_graphic())
            || diff != 0
        {
            return Err("unauthorized");
        }
        Ok(found.unwrap().clone())
    }

    fn operation_id(&self, a: &Authorization) -> String {
        hex(format!(
            "work-engine.coding-job.operation.v1\0{}\0{}\0{}",
            a.authorization_id, a.job_sha256, self.profile.host_binary_sha256
        )
        .as_bytes())
    }

    fn op_dir(&self, a: &Authorization) -> PathBuf {
        Path::new(&self.profile.operation_root).join(self.operation_id(a))
    }

    /// Validate the pinned job again at admission time; nothing here comes from the request.
    fn verify_job(&self, a: &Authorization) -> Result<Vec<u8>, &'static str> {
        let bytes = session_query::read_file(&a.job_file, JOB_FILE_LIMIT, false)
            .map_err(|_| "job_unavailable")?;
        if hex(&bytes) != a.job_sha256 {
            return Err("job_mismatch");
        }
        let job: Value = serde_json::from_slice(&bytes).map_err(|_| "job_mismatch")?;
        let packet = &job["packet"];
        let packet_path = packet["path"].as_str().ok_or("job_mismatch")?;
        if job["job_id"] != a.job_id
            || job["state_dir"] != a.state_dir
            || packet["sha256"] != a.packet_sha256
        {
            return Err("target_mismatch");
        }
        let packet_bytes = session_query::read_file(packet_path, JOB_FILE_LIMIT * 16, false)
            .map_err(|_| "job_unavailable")?;
        if hex(&packet_bytes) != a.packet_sha256 {
            return Err("job_mismatch");
        }
        // The Host only admits a fresh job whose deadline is inside its 300 second window. Check it
        // before binding so a stale prepared job does not consume the one-shot authorization.
        let deadline = job["deadline_unix_seconds"]
            .as_u64()
            .ok_or("job_mismatch")?;
        if deadline <= now() + 5 || deadline > now() + 300 {
            return Err("job_deadline_window");
        }
        // Offline class: provider-disabled coding profile only, no receiver/native auth lane.
        let coding = job
            .get("coding")
            .filter(|c| c.is_object())
            .ok_or("job_mismatch")?;
        if coding.get("provider").is_some_and(|p| !p.is_null()) || job.get("receiver").is_some() {
            return Err("capability_mismatch");
        }
        Ok(bytes)
    }

    fn submit(self: &Arc<Self>, r: Submit) -> Result<Value, &'static str> {
        if !is_uuid(&r.request_id) {
            return Err("invalid_arguments");
        }
        let auth = self.authorize(&r.authorization_id, &r.job_credential)?;
        if r.job_sha256 != auth.job_sha256 || r.packet_sha256 != auth.packet_sha256 {
            return Err("job_mismatch");
        }
        let dir = self.op_dir(&auth);
        let operation_id = self.operation_id(&auth);
        // Existing binding: exact repeat observes the same operation; anything else conflicts.
        // This never re-enters the Host while it is unobserved.
        match std::fs::create_dir(&dir) {
            Ok(()) => {
                // Until the binding is durable nothing was entered; remove the empty directory on
                // any failure so the authorization stays usable.
                let bound = (|| -> Result<(), &'static str> {
                    std::fs::set_permissions(
                        &dir,
                        std::os::unix::fs::PermissionsExt::from_mode(0o700),
                    )
                    .map_err(|_| "owner_unavailable")?;
                    let verified = self.verify_job(&auth)?;
                    self.host_binary()?;
                    let state = Path::new(&auth.state_dir);
                    if !private_dir(state) || state.join("pi-state.json").exists() {
                        return Err("state_not_fresh");
                    }
                    write_new(
                        &dir.join("binding.json"),
                        &json!({
                            "schema":SCHEMA,"operation_id":operation_id,"request_id":r.request_id,
                            "authorization_id":auth.authorization_id,"caller":auth.caller,
                            "job_id":auth.job_id,"state_dir":auth.state_dir,"job_sha256":auth.job_sha256,
                            "packet_sha256":auth.packet_sha256,
                            "host_binary_sha256":self.profile.host_binary_sha256,
                            "capability":self.profile.capability,"class":self.profile.class,
                            "bound_at_unix_seconds":now(),
                        }),
                    )?;
                    // The Host runs this verified snapshot, not the mutable operator path.
                    write_new_bytes(&dir.join("job.json"), &verified)
                })();
                if let Err(e) = bound {
                    if !dir.join("binding.json").exists() {
                        let _ = std::fs::remove_dir(&dir);
                    }
                    return Err(e);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err("owner_unavailable"),
        }
        let binding = read_json(&dir.join("binding.json"))?.ok_or("custody_lost")?;
        if binding["request_id"] != r.request_id {
            return Err("authorization_consumed");
        }
        self.verify_binding(&auth, &binding)?;
        let entered = dir.join("entry-attempt.json").exists();
        if !entered && !self.alive(&operation_id) {
            self.start(&auth, &dir, &operation_id, "run")?;
        }
        let mut out = self.read_operation(&auth, &dir, &binding)?;
        out["entry"] = json!(if entered {
            "existing_operation"
        } else {
            "collector_started"
        });
        Ok(out)
    }

    fn verify_binding(&self, a: &Authorization, b: &Value) -> Result<(), &'static str> {
        if b["schema"] != SCHEMA
            || b["operation_id"] != self.operation_id(a)
            || b["authorization_id"] != a.authorization_id
            || b["job_id"] != a.job_id
            || b["state_dir"] != a.state_dir
            || b["job_sha256"] != a.job_sha256
            || b["host_binary_sha256"] != self.profile.host_binary_sha256
        {
            return Err("identity_conflict");
        }
        Ok(())
    }

    fn bound(&self, a: &Authorization, request_id: &str) -> Result<(PathBuf, Value), &'static str> {
        if !is_uuid(request_id) {
            return Err("invalid_arguments");
        }
        let dir = self.op_dir(a);
        let binding = read_json(&dir.join("binding.json"))?.ok_or("operation_unknown")?;
        self.verify_binding(a, &binding)?;
        if binding["request_id"] != request_id {
            return Err("identity_conflict");
        }
        Ok((dir, binding))
    }

    fn alive(&self, id: &str) -> bool {
        let map = self.collectors.lock().unwrap();
        map.get(id).is_some_and(|c| !c.done.load(Ordering::SeqCst))
    }

    /// Start the one owned collector. The exclusive flock is taken before the entry marker so a
    /// second process or thread cannot own the same operation.
    fn start(
        self: &Arc<Self>,
        a: &Authorization,
        dir: &Path,
        id: &str,
        action: &'static str,
    ) -> Result<(), &'static str> {
        let mut map = self.collectors.lock().unwrap();
        if map.get(id).is_some_and(|c| !c.done.load(Ordering::SeqCst)) {
            return Ok(());
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(dir.join("collector.lock"))
            .map_err(|_| "owner_unavailable")?;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Ok(()); // another owner holds this operation
        }
        if action == "run" {
            // Durable "entry possible" mark strictly before the Host child is spawned.
            write_new(
                &dir.join("entry-attempt.json"),
                &json!({"schema":SCHEMA,"at_unix_seconds":now(),"action":"pi-docker run"}),
            )?;
        }
        let done = Arc::new(AtomicBool::new(false));
        let this = self.clone();
        let (a, dir, flag) = (a.clone(), dir.to_path_buf(), done.clone());
        let handle = thread::spawn(move || {
            let _lock = lock;
            this.collect(&a, &dir, action);
            flag.store(true, Ordering::SeqCst);
        });
        map.insert(id.to_string(), Collector { handle, done });
        Ok(())
    }

    fn host(
        &self,
        args: &[&str],
        timeout: Duration,
        cancel_on_deadline: Option<&Authorization>,
    ) -> Result<Child, &'static str> {
        self.host_binary()?;
        let mut command = Command::new(&self.profile.host_binary);
        command
            .arg("pi-docker")
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .envs(&self.profile.env)
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // A concurrently forked sibling can briefly hold a freshly written executable open (ETXTBSY).
        let mut child = {
            let mut attempts = 0;
            loop {
                match command.spawn() {
                    Ok(c) => break c,
                    Err(e) if e.raw_os_error() == Some(libc::ETXTBSY) && attempts < 50 => {
                        attempts += 1;
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => return Err("runtime_unavailable"),
                }
            }
        };
        let pgid = child.id() as i32;
        let pump = |mut pipe: Box<dyn Read + Send>| {
            thread::spawn(move || {
                let mut kept = Vec::new();
                let mut total = 0usize;
                let mut chunk = [0u8; 16384];
                loop {
                    match pipe.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            total += n;
                            if kept.len() < OUTPUT_LIMIT {
                                let take = n.min(OUTPUT_LIMIT - kept.len());
                                kept.extend_from_slice(&chunk[..take]);
                            }
                        }
                    }
                }
                (kept, total)
            })
        };
        let out_reader = pump(Box::new(child.stdout.take().unwrap()));
        let err_reader = pump(Box::new(child.stderr.take().unwrap()));
        let started = Instant::now();
        let mut result = Child {
            exit_code: None,
            exit_signal: None,
            timed_out: false,
            stopped: false,
            cancel_marker_requested: false,
            killed: false,
            waited: false,
            stdout: Vec::new(),
            stdout_truncated: false,
            stderr_bytes: 0,
            readers_joined: false,
        };
        let mut grace_until: Option<Instant> = None;
        loop {
            // Observe exit without reaping so the process-group id cannot be reused before cleanup.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            let observed = unsafe {
                libc::waitid(
                    libc::P_PID,
                    child.id(),
                    &mut info,
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if observed != 0 || unsafe { info.si_pid() } != 0 {
                break;
            }
            let stopping = self.stop.load(Ordering::SeqCst);
            if grace_until.is_none() && (stopping || started.elapsed() >= timeout) {
                result.timed_out = !stopping;
                result.stopped = stopping;
                if let Some(a) = cancel_on_deadline {
                    // Cancellation is a request to the Host; closure is established only by its receipt.
                    result.cancel_marker_requested = self
                        .host(&["cancel", "--state", &a.state_dir], QUICK_TIMEOUT, None)
                        .is_ok_and(|c| c.exit_code == Some(0));
                }
                grace_until =
                    Some(Instant::now() + Duration::from_secs(self.profile.cancel_grace_seconds));
            }
            if grace_until.is_some_and(|g| Instant::now() >= g) {
                unsafe { libc::kill(-pgid, libc::SIGKILL) };
                result.killed = true;
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        // Kill any stragglers while the leader is still unreaped, then collect actual status.
        unsafe { libc::kill(-pgid, libc::SIGKILL) };
        if let Ok(status) = child.wait() {
            result.waited = true;
            use std::os::unix::process::ExitStatusExt;
            result.exit_code = status.code();
            result.exit_signal = status.signal();
        }
        let join = |h: JoinHandle<(Vec<u8>, usize)>| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !h.is_finished() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            if h.is_finished() { h.join().ok() } else { None }
        };
        let o = join(out_reader);
        let e = join(err_reader);
        result.readers_joined = o.is_some() && e.is_some();
        if let Some((kept, total)) = o {
            result.stdout_truncated = total > kept.len();
            result.stdout = kept;
        }
        if let Some((_, total)) = e {
            result.stderr_bytes = total;
        }
        Ok(result)
    }

    /// Runs or reconciles under the exclusive collector lock and records the actual child outcome.
    fn collect(&self, a: &Authorization, dir: &Path, action: &str) {
        let snapshot = dir.join("job.json").to_string_lossy().into_owned();
        // Never hand the Host bytes other than the verified, pinned job.
        let snapshot_ok = action != "run"
            || session_query::read_file(&snapshot, JOB_FILE_LIMIT, true)
                .is_ok_and(|b| hex(&b) == a.job_sha256);
        if !snapshot_ok {
            let _ = write_atomic(
                &dir.join("collector-run.json"),
                &json!({"schema":SCHEMA,"action":action,"spawn_error":"job_snapshot_mismatch","process_cleanup":"not_established"}),
            );
            return;
        }
        let args: Vec<&str> = if action == "run" {
            vec!["run", "--request", &snapshot]
        } else {
            vec!["reconcile", "--state", &a.state_dir]
        };
        let started = now();
        let outcome = self.host(
            &args,
            Duration::from_secs(self.profile.collector_deadline_seconds),
            Some(a),
        );
        let record = match &outcome {
            Ok(c) => json!({
                "schema":SCHEMA,"action":action,"started_unix_seconds":started,"ended_unix_seconds":now(),
                "exit_code":c.exit_code,"exit_signal":c.exit_signal,"collector_deadline_exceeded":c.timed_out,
                "server_stop_requested":c.stopped,"cancel_marker_requested":c.cancel_marker_requested,
                "killed_after_grace":c.killed,"status_collected":c.waited,
                "stdout_sha256":hex(&c.stdout),"stdout_truncated":c.stdout_truncated,
                "stderr_bytes":c.stderr_bytes,"readers_joined":c.readers_joined,
                "process_cleanup":if c.waited && c.readers_joined {"owned_process_group_collected"} else {"not_established"},
            }),
            Err(code) => json!({
                "schema":SCHEMA,"action":action,"started_unix_seconds":started,"ended_unix_seconds":now(),
                "spawn_error":code,"process_cleanup":"not_established",
            }),
        };
        if let Ok(c) = &outcome
            && !c.stdout_truncated
            && let Ok(v) = serde_json::from_slice::<Value>(c.stdout.trim_ascii())
        {
            let _ = write_atomic(&dir.join(format!("host-{action}.json")), &v);
        }
        let _ = write_atomic(&dir.join(format!("collector-{action}.json")), &record);
    }

    /// Bounded read-only Host inspection of the original state.
    fn journal(&self, a: &Authorization) -> Result<Option<Value>, &'static str> {
        if !Path::new(&a.state_dir).join("pi-state.json").exists() {
            return Ok(None);
        }
        let c = self.host(&["inspect", "--state", &a.state_dir], QUICK_TIMEOUT, None)?;
        if c.exit_code != Some(0) || c.stdout_truncated {
            return Err("host_busy");
        }
        serde_json::from_slice(c.stdout.trim_ascii())
            .map(Some)
            .map_err(|_| "custody_lost")
    }

    fn read_operation(
        self: &Arc<Self>,
        a: &Authorization,
        dir: &Path,
        binding: &Value,
    ) -> Result<Value, &'static str> {
        let id = self.operation_id(a);
        let live = self.alive(&id);
        let entered = dir.join("entry-attempt.json").exists();
        let journal = if live {
            Err("host_busy")
        } else {
            self.journal(a)
        };
        let mut limitations = vec![
            "settlement_not_established_by_adapter",
            "pre_execution_binding_uses_job_file_sha256_host_input_digest_observed_after_admission",
        ];
        let (host, digest_observed) = match &journal {
            Ok(Some(j)) => {
                if j["job"]["job_id"] != a.job_id || j["job"]["state_dir"] != a.state_dir {
                    return Err("identity_conflict");
                }
                // The Host's own input digest is pinned on first sight and must not change.
                let digest = j["input_digest"]
                    .as_str()
                    .ok_or("custody_lost")?
                    .to_string();
                match write_new(
                    &dir.join("host-binding.json"),
                    &json!({"input_digest":digest}),
                ) {
                    Ok(()) | Err("already_exists") => {}
                    Err(e) => return Err(e),
                }
                let pinned = read_json(&dir.join("host-binding.json"))?.ok_or("custody_lost")?;
                if pinned["input_digest"] != digest {
                    return Err("identity_conflict");
                }
                (Some(j.clone()), Some(digest))
            }
            _ => (None, None),
        };
        let phase = host.as_ref().and_then(|j| j["phase"].as_str());
        let removed = host.as_ref().and_then(|j| j["removed"].as_bool());
        let run_outcome = read_json(&dir.join("collector-run.json"))?;
        let rec_outcome = read_json(&dir.join("collector-reconcile.json"))?;
        let state = if live {
            "collecting"
        } else if !entered {
            "bound_not_entered"
        } else if journal.is_err() {
            // Entered, nothing owned here, Host state present but not readable (typically still
            // locked by an unobserved Host). Never reconcile blind and never relaunch.
            "host_state_unreadable"
        } else if matches!((phase, removed), (Some("collected"), Some(true))) {
            "collected"
        } else if host.is_some() {
            // Journal exists but custody is incomplete and nothing owns it: reconcile the SAME state.
            self.start(a, dir, &id, "reconcile")?;
            "reconciling"
        } else {
            "entry_uncertain"
        };
        if state == "entry_uncertain" {
            limitations.push("entry_possible_host_state_absent_no_replacement_admitted");
            if run_outcome
                .as_ref()
                .is_some_and(|o| o["exit_code"].as_i64().is_some_and(|c| c != 0))
            {
                limitations
                    .push("collector_exited_nonzero_without_host_state_not_proof_of_no_entry");
            }
        }
        if matches!(&journal, Err(e) if *e == "host_busy") && !live {
            limitations.push("host_inspection_unavailable_state_locked_or_unreadable");
        }
        let cleanup = |o: &Option<Value>| o.as_ref().map(|v| v["process_cleanup"].clone());
        Ok(json!({
            "schema":SCHEMA,"operation_id":id,"request_id":binding["request_id"],
            "authorization_id":a.authorization_id,"job_id":a.job_id,"state_dir":a.state_dir,
            "job_sha256":a.job_sha256,"host_input_digest":digest_observed,
            "state":state,"entered_possible":entered,"collector_live":live,
            "host_phase":phase,"host_removed":removed,
            "host_receipt_id":host.as_ref().and_then(|j| j["receipt_id"].as_str()),
            "collector_run":run_outcome,"collector_reconcile":rec_outcome,
            "process_cleanup":{"run":cleanup(&run_outcome),"reconcile":cleanup(&rec_outcome)},
            "limitations":limitations,
            "settlement":"not_established_by_submission",
        }))
    }

    fn read_result(
        self: &Arc<Self>,
        a: &Authorization,
        dir: &Path,
        binding: &Value,
    ) -> Result<Value, &'static str> {
        let operation = self.read_operation(a, dir, binding)?;
        let receipt_doc = read_json(&dir.join("host-reconcile.json"))?
            .filter(|v| v.get("receipt").is_some() || v.get("status").is_some())
            .or(read_json(&dir.join("host-run.json"))?);
        let source = if dir.join("host-reconcile.json").exists() && receipt_doc_is_reconcile(dir) {
            "reconcile"
        } else {
            "run"
        };
        let source_exit = read_json(&dir.join(format!("collector-{source}.json")))?
            .map(|v| v["exit_code"].clone())
            .unwrap_or(json!("not_established"));
        let Some(doc) = receipt_doc else {
            return Ok(json!({"schema":SCHEMA,"result":"not_available","operation":operation}));
        };
        let receipt = doc.get("receipt").cloned().unwrap_or(doc);
        let get = |k: &str| receipt.get(k).cloned().unwrap_or(json!("not_established"));
        Ok(json!({
            "schema":SCHEMA,"result":"available","operation":operation,
            // Independent projections; none is derived from another.
            "terminal":{"status":get("status"),"phase":get("phase")},
            "validation":{"rpc_verified":get("rpc_verified"),"failures":get("failures"),
                "coding_completed":receipt["coding"].get("completed").cloned().unwrap_or(json!("not_established"))},
            "cleanup":{"removed":get("removed"),"daemon_exit":get("daemon_exit"),
                "tool_removed":receipt["coding"].get("tool_removed").cloned().unwrap_or(json!("not_established")),
                "collector":operation["process_cleanup"].clone()},
            "acceptance":get("acceptance"),"domain_effect_settlement":get("domain_effect_settlement"),
            "receipt_id":get("receipt_id"),"input_digest":get("input_digest"),
            "receipt":receipt,
            "host_output":{"action":source,"exit_code":source_exit,"note":"verbatim Host stdout; exit code is reported separately and is not a validation result"},
        }))
    }

    fn cancel(
        self: &Arc<Self>,
        a: &Authorization,
        dir: &Path,
        binding: &Value,
    ) -> Result<Value, &'static str> {
        if !dir.join("entry-attempt.json").exists() {
            return Err("not_entered");
        }
        let c = self.host(&["cancel", "--state", &a.state_dir], QUICK_TIMEOUT, None)?;
        let marker = c.exit_code == Some(0);
        Ok(json!({
            "schema":SCHEMA,"operation_id":self.operation_id(a),"request_id":binding["request_id"],
            "cancellation":if marker {"requested"} else {"request_not_recorded"},
            "cancellation_established":false,"container_closure_established":false,
            "settlement":"not_claimed",
            "next":"read_coding_job_operation until collector and Host receipt report closure",
        }))
    }

    #[cfg(test)]
    fn read_collector_idle(&self) -> bool {
        self.collectors
            .lock()
            .unwrap()
            .values()
            .all(|c| c.done.load(Ordering::SeqCst))
    }

    /// Request cancellation of every owned collector, then collect actual process status.
    pub fn shutdown(&self, wait: Duration) -> bool {
        self.stop.store(true, Ordering::SeqCst);
        let handles: Vec<Collector> = self
            .collectors
            .lock()
            .unwrap()
            .drain()
            .map(|(_, c)| c)
            .collect();
        let deadline = Instant::now() + wait;
        let mut clean = true;
        for c in handles {
            while !c.handle.is_finished() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(20));
            }
            if c.handle.is_finished() {
                clean &= c.handle.join().is_ok();
            } else {
                clean = false;
            }
        }
        clean
    }
}

#[cfg(test)]
mod tests;
