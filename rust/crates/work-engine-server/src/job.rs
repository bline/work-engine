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
pub const CAPABILITY_LIVE: &str = "coding-job-local-live-v1";
const CLASS_LIVE: &str = "local-live";
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
    /// Host input digest of this exact job: SHA-256 of the Host's typed `Job` serialization
    /// (`serde_json::to_vec`, struct field order, defaults emitted). It is not the job file digest.
    host_input_digest: String,
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

/// Host job schema (`work-engine-manual-host` `pi_docker::SCHEMA`). A different version is a
/// different input-digest algorithm contract and is rejected rather than interpreted.
const HOST_JOB_SCHEMA: &str = "work-engine.pi-docker-custody.v1";
/// Host coding fixture profile schema (`pi_coding::PROFILE`).
const HOST_CODING_SCHEMA: &str = "work-engine.pi-coding-fixture.v1";

/// Serialization-only mirror of the Host's offline-fixture `Job`. The Host computes
/// `input_digest = sha256(serde_json::to_vec(&Job))` over its typed struct, so the digest depends on
/// field order and emitted defaults, not on the job file bytes. The mirror has the same field order
/// and the same default/skip rules; it rejects every field outside the offline class (provider,
/// receiver, unknown keys). The Host binary stays pinned by hash, so its algorithm is fixed for a profile.
mod host {
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Artifact {
        pub path: String,
        pub sha256: String,
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Tree {
        pub root: String,
        pub manifest: Artifact,
    }
    #[derive(Serialize, Deserialize, PartialEq, Eq, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum Boundary {
        #[default]
        LegacyWorkspace,
        SrcLibRs,
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Coding {
        pub schema: String,
        pub shim: Artifact,
        pub toolchain: Tree,
        pub acceptance: Artifact,
        #[serde(default)]
        pub fixture_command: String,
        pub max_tool_calls: u32,
        #[serde(default, skip_serializing_if = "is_legacy")]
        pub workspace_write_boundary: Boundary,
    }
    fn is_legacy(b: &Boundary) -> bool {
        *b == Boundary::LegacyWorkspace
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Job {
        pub schema: String,
        pub job_id: String,
        pub state_dir: String,
        pub authority: Artifact,
        pub packet: Artifact,
        pub docker: Artifact,
        pub image: String,
        pub runtime: Tree,
        pub workspace: Tree,
        pub deadline_unix_seconds: u64,
        pub command_timeout_seconds: u64,
        #[serde(default)]
        pub idle_seconds: u64,
        pub coding: Coding,
    }
}

/// The Host input digest for the bounded offline or lifecycle-live class. Other
/// Host job/profile versions and unknown fields have no supported digest.
fn host_input_digest(job: &Value) -> Option<String> {
    // The live class uses the owner's public versioned DTO, including emitted defaults.
    if job["coding"]["schema"]
        .as_str()
        .is_some_and(work_engine_manual_host::pi_docker::coding::is_live_bridge)
    {
        let typed: work_engine_manual_host::pi_docker::Job =
            serde_json::from_value(job.clone()).ok()?;
        if typed.schema != HOST_JOB_SCHEMA
            || typed.receiver.is_some()
            || typed.coding.as_ref()?.provider.is_none()
        {
            return None;
        }
        return Some(hex(&serde_json::to_vec(&typed).ok()?));
    }
    let typed: host::Job = serde_json::from_value(job.clone()).ok()?;
    if typed.schema != HOST_JOB_SCHEMA || typed.coding.schema != HOST_CODING_SCHEMA {
        return None;
    }
    Some(hex(&serde_json::to_vec(&typed).ok()?))
}

/// Check public immutable admission references before consuming the one-shot authorization.
/// Host admission remains the authority owner and repeats its full checks before effects.
fn verify_live_job(value: &Value, binary: &str) -> Result<(), &'static str> {
    use work_engine_manual_host::pi_docker::{
        Job,
        coding::{SOL_HIGH_BRIDGE_PROFILE, WorkspaceWriteBoundary, is_live_bridge},
    };
    let job: Job = serde_json::from_value(value.clone()).map_err(|_| "host_identity_mismatch")?;
    let coding = job.coding.as_ref().ok_or("capability_mismatch")?;
    let provider = coding.provider.as_ref().ok_or("capability_mismatch")?;
    let sol = coding.schema == SOL_HIGH_BRIDGE_PROFILE;
    if !is_live_bridge(&coding.schema)
        || coding.max_tool_calls != 1
        || coding.workspace_write_boundary != WorkspaceWriteBoundary::SrcLibRs
        || !coding.fixture_command.is_empty()
        || job.receiver.is_some()
        || if sol {
            provider.model != "gpt-6.1-sol" || provider.sol_high.as_ref().is_none_or(|t| !t.valid())
        } else {
            provider.model != "gpt-6-luna" || provider.sol_high.is_some()
        }
        || provider.max_requests != 4
        || provider.budget_microusd == 0
        || provider.budget_microusd > if sol { 17_680_004 } else { 700_000 }
        || (sol && provider.budget_microusd != 17_680_004)
        || !(1024..=24_000).contains(&provider.max_context_bytes)
        || !(1..=30).contains(&job.command_timeout_seconds)
        || job.idle_seconds != 0
    {
        return Err("capability_mismatch");
    }
    let authority: Value =
        serde_json::from_slice(&job.authority.verify().map_err(|_| "job_mismatch")?)
            .map_err(|_| "job_mismatch")?;
    if authority["schema"]
        != if sol {
            "work-engine.pi-coding-runtime-admission.v2"
        } else {
            "work-engine.pi-coding-runtime-admission.v1"
        }
        || (sol
            && authority["sol_high"]
                != serde_json::to_value(&provider.sol_high).map_err(|_| "job_mismatch")?)
        || authority["binary_sha256"] != binary
        || authority["job_id"] != job.job_id
        || authority["model"] != provider.model
        || authority["task_sha256"] != provider.task.sha256
        || authority["catalog_sha256"] != provider.catalog.sha256
        || authority["native_auth_source"]
            != serde_json::to_value(&provider.lane.native_auth_source)
                .map_err(|_| "job_mismatch")?
        || authority["pi_config_manifest_sha256"] != provider.lane.pi_config.manifest.sha256
        || authority["max_requests"] != provider.max_requests
        || authority["budget_microusd"] != provider.budget_microusd
        || authority["allow_provider"] != true
        || authority["allow_native_access_projection"] != true
    {
        return Err("job_mismatch");
    }
    if provider.task.verify().map_err(|_| "job_mismatch")?.len() > 8192 {
        return Err("job_mismatch");
    }
    for artifact in [
        &provider.catalog,
        &provider.access_adapter,
        &coding.shim,
        &coding.acceptance,
        &job.workspace.manifest,
        &coding.toolchain.manifest,
        &job.runtime.manifest,
        &provider.lane.pi_config.manifest,
    ] {
        artifact.verify().map_err(|_| "job_mismatch")?;
    }
    let packet: Value = serde_json::from_slice(&job.packet.verify().map_err(|_| "job_mismatch")?)
        .map_err(|_| "job_mismatch")?;
    if packet["schema"] != "work-engine.coding-live-continuity-packet.v1"
        || packet["bounds"]
            != json!({"inference_containers":1,"tool_containers":1,"validator_containers":1,"sdk_sessions":2,"workspace_exec_calls":1,"fake_provider_requests":0,"physical_resets":1,"live_provider_requests":4})
    {
        return Err("job_mismatch");
    }
    Ok(())
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
/// Absolute with only normal components, so lexical prefix comparison cannot be bypassed by `..`.
fn normal_absolute(path: &Path) -> bool {
    use std::path::Component;
    path.is_absolute()
        && path
            .components()
            .all(|c| matches!(c, Component::RootDir | Component::Normal(_)))
}
fn private_dir(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .is_ok_and(|m| m.is_dir() && m.uid() == unsafe { libc::geteuid() } && m.mode() & 0o077 == 0)
}
fn write_atomic(path: &Path, value: &Value) -> Result<(), &'static str> {
    write_atomic_bytes(path, value.to_string().as_bytes())
}
fn write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), &'static str> {
    let tmp = path.with_extension("tmp");
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)
        .map_err(|_| "owner_unavailable")?;
    f.write_all(bytes)
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
fn read_json(path: &Path) -> Result<Option<Value>, &'static str> {
    match session_query::read_file(&path.to_string_lossy(), OUTPUT_LIMIT + 4096, true) {
        Ok(b) => serde_json::from_slice(&b)
            .map(Some)
            .map_err(|_| "custody_lost"),
        Err("file_unavailable") if !path.exists() => Ok(None),
        Err(_) => Err("custody_lost"),
    }
}

/// The Host writes its journal with its own umask (group-readable), so the private-file rule used for
/// adapter records does not apply; the owner is still required to be this user and symlinks are refused.
fn read_host_journal(path: &Path) -> Result<Option<Value>, &'static str> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Ok(m) if m.is_file() && m.uid() == unsafe { libc::geteuid() } => {}
        _ => return Err("custody_lost"),
    }
    let bytes = session_query::read_file(&path.to_string_lossy(), 4 * 1024 * 1024, false)
        .map_err(|_| "custody_lost")?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "custody_lost")
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
    stderr: Vec<u8>,
    stderr_truncated: bool,
    stdout_collected: bool,
    stderr_collected: bool,
    readers_joined: bool,
}

struct Collector {
    handle: JoinHandle<()>,
    done: Arc<AtomicBool>,
}

/// Collector registry. `closed` and the map share one lock so a collector start and shutdown are
/// atomic with respect to each other: after shutdown closes the registry no start can succeed, and
/// every collector admitted before that is in the map being drained.
struct Registry {
    closed: bool,
    map: HashMap<String, Collector>,
}

pub struct Jobs {
    profile: JobProfile,
    stop: Arc<AtomicBool>,
    collectors: Mutex<Registry>,
}

impl JobProfile {
    pub fn capability(&self) -> &str {
        &self.capability
    }
    fn live(&self) -> bool {
        self.version == 2 && self.capability == CAPABILITY_LIVE && self.class == CLASS_LIVE
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if !((self.version == 1
            && self.capability == CAPABILITY_OFFLINE
            && self.class == CLASS_OFFLINE)
            || self.live())
            || self.kind != PROFILE_KIND
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
                || !is_hex64(&a.host_input_digest)
                || !Path::new(&a.job_file).is_absolute()
                || !normal_absolute(Path::new(&a.state_dir))
                || !ids.insert(a.authorization_id.clone())
            {
                return Err("invalid_job_profile");
            }
        }
        // One Host state, one job identity and one container name per authorization: two
        // authorizations must never be able to bind or act on the same Host state.
        let root = Path::new(&self.operation_root);
        if !normal_absolute(root) || !private_dir(root) {
            return Err("invalid_job_profile");
        }
        for (i, a) in self.authorizations.iter().enumerate() {
            let state = Path::new(&a.state_dir);
            if state.starts_with(root) || root.starts_with(state) {
                return Err("overlapping_state_ownership");
            }
            // An existing path must already be canonical: the Host refuses aliased paths.
            if std::fs::symlink_metadata(state).is_ok()
                && std::fs::canonicalize(state).ok().as_deref() != Some(state)
            {
                return Err("invalid_job_profile");
            }
            for b in &self.authorizations[i + 1..] {
                let other = Path::new(&b.state_dir);
                if a.job_id == b.job_id
                    || a.host_input_digest == b.host_input_digest
                    || state.starts_with(other)
                    || other.starts_with(state)
                {
                    return Err("overlapping_state_ownership");
                }
            }
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
            collectors: Mutex::new(Registry {
                closed: false,
                map: HashMap::new(),
            }),
        };
        jobs.host_binary()?;
        if jobs.profile.live() {
            for authorization in &jobs.profile.authorizations {
                // Recovery reads the original operation even after deadlines or input expiry.
                // Only a fresh authorization can enter and needs fresh admission now.
                if !jobs.op_dir(authorization).join("binding.json").exists() {
                    jobs.verify_job(authorization)?;
                }
            }
        }
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
        let latest = if self.profile.live() { 240 } else { 300 };
        if deadline <= now() + 5 || deadline > now() + latest {
            return Err("job_deadline_window");
        }
        // Offline class: provider-disabled coding profile only, no receiver/native auth lane.
        let coding = job
            .get("coding")
            .filter(|c| c.is_object())
            .ok_or("job_mismatch")?;
        if job.get("receiver").is_some()
            || (!self.profile.live() && coding.get("provider").is_some_and(|p| !p.is_null()))
        {
            return Err("capability_mismatch");
        }
        if self.profile.live() {
            verify_live_job(&job, &self.profile.host_binary_sha256)?;
        }
        // Bind the exact Host serialized Job before any entry is possible. A job outside the offline
        // shape or a different Host job/profile version has no digest; a different job has another one.
        let digest = host_input_digest(&job).ok_or("host_identity_mismatch")?;
        if digest != a.host_input_digest {
            return Err("host_identity_mismatch");
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
                            "packet_sha256":auth.packet_sha256,"host_input_digest":auth.host_input_digest,
                            "host_job_schema":HOST_JOB_SCHEMA,
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
        self.verify_host_state(&auth)?;
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
            || b["packet_sha256"] != a.packet_sha256
            || b["host_input_digest"] != a.host_input_digest
            || b["host_job_schema"] != HOST_JOB_SCHEMA
            || b["host_binary_sha256"] != self.profile.host_binary_sha256
        {
            return Err("identity_conflict");
        }
        Ok(())
    }

    /// Verifies the Host state (when one exists) is this authorization's exact job: the journal's own
    /// digest, recomputed from its job, must equal the digest bound before entry. Reads the journal file
    /// without the Host lock so it also holds while the Host runs. `None` means no Host state yet.
    fn verify_host_state(&self, a: &Authorization) -> Result<Option<Value>, &'static str> {
        let Some(journal) = read_host_journal(&Path::new(&a.state_dir).join("pi-state.json"))?
        else {
            return Ok(None);
        };
        let digest = journal["input_digest"].as_str().ok_or("custody_lost")?;
        if journal["schema"] != HOST_JOB_SCHEMA
            || journal["job"]["job_id"] != a.job_id
            || journal["job"]["state_dir"] != a.state_dir
            || digest != a.host_input_digest
            || host_input_digest(&journal["job"]).as_deref() != Some(digest)
        {
            return Err("identity_conflict");
        }
        Ok(Some(journal))
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
        // Read, reconcile, cancel and result all pass through here: none acts on a foreign Host state.
        self.verify_host_state(a)?;
        Ok((dir, binding))
    }

    fn alive(&self, id: &str) -> bool {
        let reg = self.collectors.lock().unwrap();
        reg.map
            .get(id)
            .is_some_and(|c| !c.done.load(Ordering::SeqCst))
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
        let mut reg = self.collectors.lock().unwrap();
        // Fence: checked under the registry lock, strictly before the flock and the entry marker,
        // so a request that outlives its transport cannot enter the Host after shutdown began.
        if reg.closed || self.stop.load(Ordering::SeqCst) {
            return Err("shutting_down");
        }
        if reg
            .map
            .get(id)
            .is_some_and(|c| !c.done.load(Ordering::SeqCst))
        {
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
        reg.map.insert(id.to_string(), Collector { handle, done });
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
            stderr: Vec::new(),
            stderr_truncated: false,
            stdout_collected: false,
            stderr_collected: false,
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
            result.stdout_collected = true;
            result.stdout_truncated = total > kept.len();
            result.stdout = kept;
        }
        if let Some((kept, total)) = e {
            result.stderr_collected = true;
            result.stderr_truncated = total > kept.len();
            result.stderr = kept;
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
        let admission = if !snapshot_ok {
            Err("job_snapshot_mismatch")
        } else if action == "run" && self.profile.live() {
            self.verify_job(a).map(|_| ())
        } else {
            Ok(())
        };
        if let Err(code) = admission {
            let _ = write_atomic(
                &dir.join("collector-run.json"),
                &json!({"schema":SCHEMA,"action":action,"spawn_error":code,"process_cleanup":"not_established"}),
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
        // Keep the bounded exact prefix even when a child hangs, is interrupted,
        // or emits malformed/non-JSON output. A digest alone cannot recover it.
        let stdout_artifact = outcome
            .as_ref()
            .ok()
            .filter(|c| c.stdout_collected)
            .and_then(|c| {
                let path = dir.join(format!("collector-{action}.stdout"));
                write_atomic_bytes(&path, &c.stdout)
                    .ok()
                    .map(|()| json!({"path":path,"sha256":hex(&c.stdout),"bytes":c.stdout.len()}))
            });
        let stderr_artifact = outcome
            .as_ref()
            .ok()
            .filter(|c| c.stderr_collected)
            .and_then(|c| {
                let path = dir.join(format!("collector-{action}.stderr"));
                write_atomic_bytes(&path, &c.stderr)
                    .ok()
                    .map(|()| json!({"path":path,"sha256":hex(&c.stderr),"bytes":c.stderr.len()}))
            });
        let record = match &outcome {
            Ok(c) => json!({
                "schema":SCHEMA,"action":action,"started_unix_seconds":started,"ended_unix_seconds":now(),
                "exit_code":c.exit_code,"exit_signal":c.exit_signal,"collector_deadline_exceeded":c.timed_out,
                "server_stop_requested":c.stopped,"cancel_marker_requested":c.cancel_marker_requested,
                "killed_after_grace":c.killed,"status_collected":c.waited,
                "stdout_sha256":hex(&c.stdout),"stdout_truncated":c.stdout_truncated,"stdout_artifact":stdout_artifact,
                "stderr_bytes":c.stderr_bytes,"stderr_sha256":if c.stderr_collected {Some(hex(&c.stderr))} else {None},"stderr_truncated":c.stderr_truncated,"stderr_artifact":stderr_artifact,"stdout_collected":c.stdout_collected,"stderr_collected":c.stderr_collected,"readers_joined":c.readers_joined,
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
        let mut limitations = vec!["settlement_not_established_by_adapter"];
        let (host, digest_observed) = match &journal {
            Ok(Some(j)) => {
                // The expected digest was bound before entry; the Host never supplies its own.
                if j["job"]["job_id"] != a.job_id
                    || j["job"]["state_dir"] != a.state_dir
                    || j["input_digest"] != a.host_input_digest
                {
                    return Err("identity_conflict");
                }
                if !j["receipt_id"].is_null() && !j["receipt_id"].as_str().is_some_and(is_hex64) {
                    return Err("custody_lost");
                }
                (Some(j.clone()), Some(a.host_input_digest.clone()))
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
            "job_sha256":a.job_sha256,"expected_host_input_digest":a.host_input_digest,
            "host_input_digest":digest_observed,
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
        // Reconcile output is the Host's receipt wrapper; a run that failed before a receipt writes none.
        let (source, wrapper) = match read_json(&dir.join("host-reconcile.json"))? {
            Some(v) => ("reconcile", Some(v)),
            None => ("run", read_json(&dir.join("host-run.json"))?),
        };
        let source_exit = read_json(&dir.join(format!("collector-{source}.json")))?
            .map(|v| v["exit_code"].clone())
            .unwrap_or(json!("not_established"));
        let Some(wrapper) = wrapper else {
            return Ok(json!({"schema":SCHEMA,"result":"not_available","operation":operation}));
        };
        let (receipt_id, receipt) = self.host_receipt(a, &operation, &wrapper)?;
        let get = |k: &str| receipt.get(k).cloned().unwrap_or(json!("not_established"));
        let coding = |k: &str| {
            receipt["coding"]
                .get(k)
                .cloned()
                .unwrap_or(json!("not_established"))
        };
        Ok(json!({
            "schema":SCHEMA,"result":"available","operation":operation,
            // Independent projections; none is derived from another.
            "terminal":{"status":get("status"),"phase":get("phase")},
            "validation":{"rpc_verified":get("rpc_verified"),"failures":get("failures"),
                "coding_completed":coding("completed")},
            "cleanup":{"removed":get("removed"),"daemon_exit":get("daemon_exit"),
                "tool_removed":coding("tool_removed"),
                "collector":operation["process_cleanup"].clone()},
            "acceptance":get("acceptance"),"domain_effect_settlement":get("domain_effect_settlement"),
            // The Host wrapper's own ID (outside the receipt body), checked against the original operation.
            "receipt_id":receipt_id,"input_digest":a.host_input_digest,
            "receipt":receipt,
            "host_output":{"action":source,"exit_code":source_exit,"note":"Host receipt projection with numeric values preserved; exit code is reported separately and is not a validation result"},
        }))
    }

    /// Validates the actual Host wrapper `{receipt_id, receipt}` against this authorization and the
    /// operation's journal. The ID lives outside the receipt body; the receipt payload stays opaque
    /// Host data beyond identity.
    fn host_receipt(
        &self,
        a: &Authorization,
        operation: &Value,
        wrapper: &Value,
    ) -> Result<(String, Value), &'static str> {
        let object = wrapper.as_object().ok_or("custody_lost")?;
        if object.len() != 2 {
            return Err("custody_lost");
        }
        let id = object
            .get("receipt_id")
            .and_then(Value::as_str)
            .filter(|id| is_hex64(id))
            .ok_or("custody_lost")?;
        let receipt = object
            .get("receipt")
            .filter(|r| r.is_object())
            .ok_or("custody_lost")?;
        if receipt["job_id"] != a.job_id
            || receipt["job"]["job_id"] != a.job_id
            || receipt["job"]["state_dir"] != a.state_dir
            || receipt["input_digest"] != a.host_input_digest
            || host_input_digest(&receipt["job"]).as_deref() != Some(a.host_input_digest.as_str())
            // Once the journal records a receipt it must be this one.
            || operation["host_receipt_id"].as_str().is_some_and(|j| j != id)
        {
            return Err("identity_conflict");
        }
        Ok((id.to_string(), receipt.clone()))
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
        // Identity was verified against the bound digest in `bound`; the Host re-checks its own binding.
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
            .map
            .values()
            .all(|c| c.done.load(Ordering::SeqCst))
    }

    /// Request cancellation of every owned collector, then collect actual process status.
    ///
    /// Closing the registry and draining it happen under one lock, so every collector ever admitted is
    /// either joined here or was refused by `start`. Idempotent: a later call drains nothing and
    /// reports the registry fenced and empty.
    pub fn shutdown(&self, wait: Duration) -> bool {
        self.stop.store(true, Ordering::SeqCst);
        let handles: Vec<Collector> = {
            let mut reg = self.collectors.lock().unwrap();
            reg.closed = true;
            reg.map.drain().map(|(_, c)| c).collect()
        };
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
