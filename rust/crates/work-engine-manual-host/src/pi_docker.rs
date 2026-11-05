//! Provider-disabled Pi realization custody. This journal owns container/process
//! facts only; it grants no task effects, lifecycle transitions or acceptance.
use crate::{
    Artifact, Error, ProcessRecord, Result, STOP, atomic, digest, file_digest, group_members,
    kill_group, now, refuse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[path = "pi_coding.rs"]
pub mod coding;
#[path = "pi_observation.rs"]
pub mod receiver;

pub const SCHEMA: &str = "work-engine.pi-docker-custody.v1";
pub const IMAGE: &str =
    "node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da";
const LIMIT: u64 = 8 * 1024 * 1024;
const PI: &str = "node_modules/@earendil-works/pi-coding-agent/dist/bundle/cli.js";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Entry {
    File { sha256: String },
    Symlink { target: PathBuf },
}

/// Full inventory, including symlinks (whose resolved target must stay inside
/// the root). A selected-file list cannot attest the realized runtime surface.
pub fn inventory(root: &Path) -> Result<BTreeMap<String, Entry>> {
    fn visit(root: &Path, dir: &Path, out: &mut BTreeMap<String, Entry>) -> Result<()> {
        for item in fs::read_dir(dir)? {
            let path = item?.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|_| Error::Refused("inventory root conflict".into()))?;
            let key = relative
                .to_str()
                .ok_or_else(|| Error::Refused("non UTF-8 inventory path".into()))?
                .to_string();
            let meta = fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() {
                if !fs::canonicalize(&path)?.starts_with(root) {
                    return refuse("symlink escapes pinned root");
                }
                out.insert(
                    key,
                    Entry::Symlink {
                        target: fs::read_link(path)?,
                    },
                );
            } else if meta.is_dir() {
                visit(root, &path, out)?;
            } else if meta.is_file() {
                out.insert(
                    key,
                    Entry::File {
                        sha256: file_digest(&path, None)?,
                    },
                );
            } else {
                return refuse("special file in pinned tree");
            }
            if out.len() > 30_000 {
                return refuse("inventory too large");
            }
        }
        Ok(())
    }
    if fs::canonicalize(root)? != root || !root.is_dir() {
        return refuse("tree root must be canonical directory");
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out)?;
    Ok(out)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tree {
    pub root: PathBuf,
    pub manifest: Artifact,
}
impl Tree {
    fn verify(&self) -> Result<()> {
        let expected: BTreeMap<String, Entry> = serde_json::from_slice(&self.manifest.verify()?)?;
        if inventory(&self.root)? != expected {
            return refuse("pinned tree inventory changed");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub schema: String,
    pub job_id: String,
    pub state_dir: PathBuf,
    pub authority: Artifact,
    pub packet: Artifact,
    pub docker: Artifact,
    /// Immutable repository digest, never a mutable tag.
    pub image: String,
    pub runtime: Tree,
    /// Explicit read-only disposable inputs. Only host-owned exports are writable.
    pub workspace: Tree,
    pub deadline_unix_seconds: u64,
    pub command_timeout_seconds: u64,
    /// Optional idle observation window after the four provider-disabled replies.
    /// Keeps a real target alive for bounded cancellation/recovery qualification.
    #[serde(default)]
    pub idle_seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver: Option<receiver::Profile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coding: Option<coding::Profile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Admitted,
    CreateUnknown,
    Created,
    StartUnknown,
    Running,
    Collecting,
    Collected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcCompletion {
    pub process_index: usize,
    pub stdout_sha256: String,
    pub session_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Journal {
    pub schema: String,
    pub binary_sha256: String,
    pub job: Job,
    pub input_digest: String,
    pub container_name: String,
    pub container_id: Option<String>,
    pub session_id: Option<String>,
    pub phase: Phase,
    pub processes: Vec<ProcessRecord>,
    pub failures: Vec<String>,
    pub rpc_verified: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver: Option<receiver::Observation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coding: Option<coding::State>,
    /// Prefix readiness is distinct from successful validation of completed bytes.
    /// Older journals have no final proof and cannot become success on recovery.
    #[serde(default)]
    pub rpc_completion: Option<RpcCompletion>,
    pub daemon_exit: Option<i32>,
    pub removed: bool,
    pub artifacts: Vec<Artifact>,
    pub exported_files: BTreeMap<String, Artifact>,
    pub receipt_id: Option<String>,
    pub acknowledged: BTreeSet<String>,
}

pub struct Host {
    journal: Journal,
    coding_gate: Option<Box<dyn coding::CodingEffectGate>>,
    coding_gate_deadline_ms: Option<i64>,
    pi_owner: Option<session_task_admission::pi_pilot::PiOwner>,
    allocation_lease: Option<session_task_admission::pi_pilot::EffectLease>,
    reply_leases: Vec<session_task_admission::pi_pilot::EffectLease>,
    _lock: File,
}

struct OwnedChild(Child);
impl std::ops::Deref for OwnedChild {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.0
    }
}
impl std::ops::DerefMut for OwnedChild {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            kill_group(self.0.id());
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn pin_is_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}
fn canonical_directory(p: &Path) -> Result<()> {
    if !p.is_absolute() || fs::canonicalize(p)? != p || !p.is_dir() {
        return refuse("directory must be canonical and absolute");
    }
    Ok(())
}
fn verify_executable(artifact: &Artifact) -> Result<()> {
    if !artifact.path.is_absolute() || file_digest(&artifact.path, None)? != artifact.sha256 {
        return refuse("Docker executable pin conflict");
    }
    Ok(())
}
fn check_job(job: &Job, fresh: bool) -> Result<()> {
    if job.schema != SCHEMA
        || job.job_id.is_empty()
        || job.job_id.len() > 80
        || !job
            .job_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        || !(1..=30).contains(&job.command_timeout_seconds)
        || job.idle_seconds > 30
    {
        return refuse("invalid Pi job schema/identity/timeout");
    }
    let (_, image_hash) = job
        .image
        .rsplit_once("@sha256:")
        .ok_or_else(|| Error::Refused("image must be digest pinned".into()))?;
    if !pin_is_hash(image_hash) || job.image != IMAGE {
        return refuse("only the qualified digest-pinned node image is supported");
    }
    if fresh && (job.deadline_unix_seconds <= now() || job.deadline_unix_seconds > now() + 300) {
        return refuse("fresh fixture deadline must be within 300 seconds");
    }
    if job.receiver.is_some() && job.coding.is_some() {
        return refuse("exclusive Pi profile required");
    }
    if let Some(profile) = &job.coding {
        profile.validate(job)?;
    }
    if let Some(profile) = &job.receiver {
        profile.validate(job)?;
        if job.idle_seconds != 0
            || (fresh && job.deadline_unix_seconds > now() + 30)
            || !job
                .runtime
                .root
                .join("node_modules/@earendil-works/pi-coding-agent/dist/index.js")
                .is_file()
        {
            return refuse("receiver SDK runtime/30 second profile conflict");
        }
    }
    canonical_directory(&job.state_dir)?;
    for p in [&job.runtime.root, &job.workspace.root] {
        canonical_directory(p)?;
        if p == Path::new("/") || job.state_dir.starts_with(p) || p.starts_with(&job.state_dir) {
            return refuse("state and mounted trees must be disjoint");
        }
    }
    if job.runtime.root.starts_with(&job.workspace.root)
        || job.workspace.root.starts_with(&job.runtime.root)
    {
        return refuse("runtime and workspace must be disjoint");
    }
    for p in [&job.runtime.root, &job.workspace.root, &job.state_dir] {
        if p.to_string_lossy().contains([',', '\n', '\r']) {
            return refuse("mount paths contain Docker syntax");
        }
    }
    job.authority.verify()?;
    job.packet.verify()?;
    verify_executable(&job.docker)?;
    if !job.docker.path.is_file() {
        return refuse("Docker executable missing");
    }
    job.runtime.verify()?;
    job.workspace.verify()?;
    let package: Value = serde_json::from_slice(&fs::read(
        job.runtime
            .root
            .join("node_modules/@earendil-works/pi-coding-agent/package.json"),
    )?)?;
    if package["name"] != "@earendil-works/pi-coding-agent"
        || package["version"] != "1.5.0"
        || !job.runtime.root.join(PI).is_file()
    {
        return refuse("only pinned Pi 1.5.0 fixture is supported");
    }
    Ok(())
}

impl Host {
    pub fn journal(&self) -> &Journal {
        &self.journal
    }
    pub fn admit(job: Job) -> Result<Self> {
        if job.coding.as_ref().is_some_and(|p| {
            matches!(
                p.schema.as_str(),
                coding::SIGNED_PROFILE | coding::SIGNED_SUBSCRIPTION_PROFILE
            )
        }) {
            return refuse("signed fixture requires Pi owner");
        }
        Self::admit_inner(job, None)
    }
    pub fn admit_signed_fixture(
        job: Job,
        owner: session_task_admission::pi_pilot::PiOwner,
    ) -> Result<Self> {
        if owner.is_subscription()
            || !job.coding.as_ref().is_some_and(|p| {
                p.schema == coding::SIGNED_PROFILE
                    && p.provider.is_none()
                    && p.workspace_write_boundary == coding::WorkspaceWriteBoundary::SrcLibRs
            })
        {
            return refuse("signed fixture profile required");
        }
        owner
            .admit(
                &digest(&serde_json::to_vec(&job)?),
                &digest(&fs::read(std::env::current_exe()?)?),
            )
            .map_err(|e| Error::Refused(e.into()))?;
        Self::admit_inner(job, Some(owner))
    }
    /// Exact signed subscription candidate. Ordinary admission cannot create it.
    pub fn admit_signed_subscription(
        job: Job,
        owner: session_task_admission::pi_pilot::PiOwner,
    ) -> Result<Self> {
        if !owner.is_subscription()
            || !job.coding.as_ref().is_some_and(|p| {
                p.schema == coding::SIGNED_SUBSCRIPTION_PROFILE
                    && p.provider.is_some()
                    && p.workspace_write_boundary == coding::WorkspaceWriteBoundary::SrcLibRs
            })
        {
            return refuse("signed subscription profile required");
        }
        owner
            .admit(
                &digest(&serde_json::to_vec(&job)?),
                &digest(&fs::read(std::env::current_exe()?)?),
            )
            .map_err(|e| Error::Refused(e.into()))?;
        Self::admit_inner(job, Some(owner))
    }
    fn admit_inner(
        job: Job,
        owner: Option<session_task_admission::pi_pilot::PiOwner>,
    ) -> Result<Self> {
        // Caller creates a private state directory; never create arbitrary mounted paths.
        check_job(&job, true)?;
        let lock = Self::lock(&job.state_dir)?;
        if job.state_dir.join("pi-state.json").exists() {
            return refuse("existing job: reconcile instead of launch replay");
        }
        if fs::read_dir(&job.state_dir)?
            .any(|e| e.map(|x| x.file_name() != "pi-lock").unwrap_or(true))
        {
            return refuse("state directory must be fresh");
        }
        fs::create_dir(job.state_dir.join("exports"))?;
        fs::create_dir(job.state_dir.join("snapshots"))?;
        fs::create_dir(job.state_dir.join("receipts"))?;
        let mut input_pins = Vec::new();
        for input in [
            &job.authority,
            &job.packet,
            &job.runtime.manifest,
            &job.workspace.manifest,
        ] {
            input_pins.push(Artifact {
                path: input.snapshot(&job.state_dir.join("snapshots"))?,
                sha256: input.sha256.clone(),
            });
        }
        if let Some(profile) = &job.receiver {
            for input in [&profile.shim, &profile.event] {
                input_pins.push(Artifact {
                    path: input.snapshot(&job.state_dir.join("snapshots"))?,
                    sha256: input.sha256.clone(),
                });
            }
        }
        if let Some(profile) = &job.coding {
            for input in [
                &profile.shim,
                &profile.toolchain.manifest,
                &profile.acceptance,
            ] {
                input_pins.push(Artifact {
                    path: input.snapshot(&job.state_dir.join("snapshots"))?,
                    sha256: input.sha256.clone(),
                });
            }
        }
        if let Some(plan) = job.coding.as_ref().and_then(|p| p.provider.as_ref()) {
            for input in [
                &plan.task,
                &plan.catalog,
                &plan.access_adapter,
                &plan.lane.pi_config.manifest,
            ] {
                input_pins.push(Artifact {
                    path: input.snapshot(&job.state_dir.join("snapshots"))?,
                    sha256: input.sha256.clone(),
                });
            }
        }
        let coding = job.coding.as_ref().map(|_| coding::State::default());
        let receiver = job
            .receiver
            .as_ref()
            .map(|_| receiver::Observation::default());
        let input_digest = digest(&serde_json::to_vec(&job)?);
        let journal = Journal {
            schema: SCHEMA.into(),
            binary_sha256: digest(&fs::read(std::env::current_exe()?)?),
            container_name: format!("we-pi-{}", &input_digest[..24]),
            input_digest,
            job,
            container_id: None,
            session_id: None,
            phase: Phase::Admitted,
            processes: vec![],
            failures: vec![],
            rpc_verified: false,
            receiver,
            coding,
            rpc_completion: None,
            daemon_exit: None,
            removed: false,
            artifacts: input_pins,
            exported_files: BTreeMap::new(),
            receipt_id: None,
            acknowledged: BTreeSet::new(),
        };
        let host = Self {
            journal,
            coding_gate: None,
            coding_gate_deadline_ms: None,
            _lock: lock,
            pi_owner: owner,
            allocation_lease: None,
            reply_leases: vec![],
        };
        host.save()?;
        Ok(host)
    }
    fn lock(dir: &Path) -> Result<File> {
        canonical_directory(dir)?;
        let f = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join("pi-lock"))?;
        f.try_lock().map_err(|_| {
            Error::Refused("Pi realization busy; cancellation marker remains deliverable".into())
        })?;
        Ok(f)
    }
    pub fn open(dir: &Path) -> Result<Self> {
        let lock = Self::lock(dir)?;
        let journal: Journal = serde_json::from_slice(&crate::bytes(&dir.join("pi-state.json"))?)?;
        if journal.schema != SCHEMA
            || journal.job.state_dir != dir
            || journal.input_digest != digest(&serde_json::to_vec(&journal.job)?)
            || journal.container_name != format!("we-pi-{}", &journal.input_digest[..24])
            || journal.binary_sha256 != digest(&fs::read(std::env::current_exe()?)?)
        {
            return refuse("Pi journal identity/binary conflict");
        }
        // A missing/changed input must not prevent custody of an already started container.
        verify_executable(&journal.job.docker)?;
        Ok(Self {
            journal,
            coding_gate: None,
            coding_gate_deadline_ms: None,
            _lock: lock,
            pi_owner: None,
            allocation_lease: None,
            reply_leases: vec![],
        })
    }
    fn save(&self) -> Result<()> {
        atomic(
            &self.journal.job.state_dir.join("pi-state.json"),
            &self.journal,
        )
    }
    fn failure(&mut self, error: impl ToString) {
        self.journal.failures.push(error.to_string());
    }
    fn cancelled(&self) -> bool {
        STOP.load(Ordering::SeqCst) || self.journal.job.state_dir.join("cancel.json").exists()
    }
    fn expired(&self) -> bool {
        now() >= self.journal.job.deadline_unix_seconds
    }
    fn argv(&self, tail: Vec<String>) -> Vec<String> {
        std::iter::once(self.journal.job.docker.path.to_string_lossy().into_owned())
            .chain(tail)
            .collect()
    }
    fn command(&mut self, label: &str, args: Vec<String>, rpc: bool) -> Result<(bool, Vec<u8>)> {
        self.command_with_lease(label, args, rpc, None)
    }
    fn command_with_lease(
        &mut self,
        label: &str,
        args: Vec<String>,
        rpc: bool,
        effect_lease: Option<&session_task_admission::pi_pilot::EffectLease>,
    ) -> Result<(bool, Vec<u8>)> {
        let argv = self.argv(args);
        let number = self.journal.processes.len();
        // Retain and bypass any legacy/unpublished reservation. Publication of
        // the record below precedes creating either log, so a new interruption
        // always advances the next process index without truncating prior bytes.
        let mut reservation = 0_u64;
        let base = loop {
            let candidate = self
                .journal
                .job
                .state_dir
                .join(format!("{number:04}-{label}-{reservation}"));
            if !candidate.with_extension("stdout").try_exists()?
                && !candidate.with_extension("stderr").try_exists()?
            {
                break candidate;
            }
            reservation = reservation
                .checked_add(1)
                .ok_or_else(|| Error::Refused("log reservation exhausted".into()))?;
        };
        let stdout_path = base.with_extension("stdout");
        let stderr_path = base.with_extension("stderr");
        self.journal.processes.push(ProcessRecord {
            argv: argv.clone(),
            pid: None,
            start: None,
            status: None,
            stdout: stdout_path.clone(),
            stderr: stderr_path.clone(),
            stdout_sha256: None,
            stderr_sha256: None,
            timed_out: false,
            leftovers: vec![],
        });
        self.save()?;
        let stdout = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stdout_path)?;
        let stderr = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stderr_path)?;
        let mut cmd = Command::new(&argv[0]);
        // Docker context/host overrides and provider credentials cannot leak from ambient env.
        cmd.args(&argv[1..])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", "/nonexistent")
            .process_group(0)
            .stdin(if rpc { Stdio::piped() } else { Stdio::null() })
            .stdout(stdout)
            .stderr(stderr);
        if rpc && let Some(lease) = &self.allocation_lease {
            lease
                .check_current()
                .map_err(|e| Error::Refused(e.into()))?;
        }
        if let Some(lease) = effect_lease {
            lease
                .check_current()
                .map_err(|e| Error::Refused(e.into()))?;
        }
        if self.signed_pi()
            && (rpc || label == "coding-tool-exec")
            && (self.expired() || self.cancelled())
        {
            return refuse("signed fixture dispatch expired/cancelled");
        }
        if label == "coding-tool-exec"
            && self
                .journal
                .job
                .coding
                .as_ref()
                .is_some_and(|p| coding::is_bridge(&p.schema))
        {
            let wall_ms: i64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Error::Refused("coding effect clock unavailable".into()))?
                .as_millis()
                .try_into()
                .map_err(|_| Error::Refused("coding effect clock range".into()))?;
            if self.expired()
                || self.cancelled()
                || self
                    .coding_gate_deadline_ms
                    .is_none_or(|deadline| wall_ms >= deadline)
            {
                return refuse("coding lifecycle dispatch expired/cancelled/unadmitted");
            }
        }
        let mut child = OwnedChild(cmd.spawn()?);
        if rpc {
            drop(self.allocation_lease.take());
        } // allocation remains durably unknown until actual ID sealing
        self.journal.processes[number].pid = Some(child.id());
        self.journal.processes[number].start = crate::proc_stat(child.id()).map(|s| s.0);
        self.save()?;
        let mut stdin = child.stdin.take();
        let mut write_result = if let Some(input) = stdin.as_mut() {
            let initial =
                if self.journal.job.receiver.is_some() || self.journal.job.coding.is_some() {
                    vec![]
                } else {
                    rpc_input()
                };
            input.write_all(&initial).and_then(|_| input.flush())
        } else {
            Ok(())
        };
        let start = Instant::now();
        let mut verified = false;
        let mut verified_at = None;
        let mut aborted = false;
        loop {
            let output_size = fs::metadata(&stdout_path)?.len();
            let error_size = fs::metadata(&stderr_path)?.len();
            if output_size > LIMIT || error_size > LIMIT {
                self.failure("process output exceeded bound");
                aborted = true;
            }
            if rpc && !verified && output_size <= LIMIT {
                let raw = fs::read(&stdout_path)?;
                if self.journal.job.coding.is_some() {
                    match self.coding_advance(&raw) {
                        Ok(commands) => {
                            let leases = std::mem::take(&mut self.reply_leases);
                            // Hold all owner ordering leases through the actual IPC grant write.
                            for lease in &leases {
                                lease
                                    .check_current()
                                    .map_err(|e| Error::Refused(e.into()))?;
                            }
                            if self.signed_pi() && (self.expired() || self.cancelled()) {
                                return refuse("signed fixture IPC dispatch expired/cancelled");
                            }
                            for command in commands {
                                if let Some(input) = stdin.as_mut() {
                                    if self
                                        .journal
                                        .job
                                        .coding
                                        .as_ref()
                                        .is_some_and(|p| coding::is_bridge(&p.schema))
                                    {
                                        if self.expired() || self.cancelled() {
                                            return refuse(
                                                "coding lifecycle IPC expired/cancelled",
                                            );
                                        }
                                        self.coding_bridge_dispatch(&command, &mut |bytes| {
                                            input.write_all(bytes)?;
                                            input.flush()?;
                                            Ok(())
                                        })?;
                                        write_result = Ok(());
                                    } else {
                                        write_result =
                                            input.write_all(&command).and_then(|_| input.flush());
                                    }
                                    if write_result.is_err() {
                                        break;
                                    }
                                } else {
                                    return refuse("coding stdin absent");
                                }
                            }
                            if write_result.is_ok() {
                                for lease in leases {
                                    lease
                                        .observed(digest(b"host IPC grant flushed").as_str())
                                        .map_err(|e| Error::Refused(e.into()))?;
                                }
                            }
                        }
                        Err(e) => {
                            self.failure(e);
                            aborted = true;
                        }
                    }
                } else if self.journal.job.receiver.is_some() {
                    match self.receiver_advance(&raw, number) {
                        Ok(Some(command)) => {
                            write_result = stdin
                                .as_mut()
                                .ok_or_else(|| Error::Refused("receiver stdin closed".into()))?
                                .write_all(&command);
                            if write_result.is_ok() {
                                write_result = stdin.as_mut().unwrap().flush();
                            }
                            drop(stdin.take());
                            verified = true;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            self.failure(e);
                            aborted = true;
                        }
                    }
                } else {
                    match verify_rpc(&raw, false) {
                        Ok(Some(session)) => {
                            self.journal.session_id = Some(session);
                            self.journal.rpc_verified = true;
                            verified = true;
                            verified_at = Some(Instant::now());
                            self.save()?;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            self.failure(e);
                            aborted = true;
                        }
                    }
                }
            }
            if rpc && self.journal.job.receiver.is_some() && output_size <= LIMIT {
                let raw = fs::read(&stdout_path)?;
                if let Err(e) = self.receiver_progress(&raw) {
                    self.failure(e);
                    aborted = true;
                }
            }
            if verified_at.is_some_and(|at: Instant| {
                at.elapsed() >= Duration::from_secs(self.journal.job.idle_seconds)
            }) {
                drop(stdin.take());
            }
            let timeout = start.elapsed()
                >= Duration::from_secs(if rpc {
                    300
                } else {
                    self.journal.job.command_timeout_seconds
                });
            if timeout
                || ((rpc
                    || label == "coding-tool-exec"
                    || label == "coding-final-acceptance"
                    || label == "coding-validation-start")
                    && (self.expired() || self.cancelled()))
                || write_result.is_err()
                || aborted
            {
                self.journal.processes[number].timed_out = timeout || (rpc && self.expired());
                self.failure(if self.cancelled() {
                    "cancellation requested"
                } else {
                    "process deadline/output/input failure"
                });
                kill_group(child.id());
                let _ = child.kill();
            }
            if let Some(status) = child.try_wait()? {
                self.journal.processes[number].status = status.code();
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(stdin);
        let record = &mut self.journal.processes[number];
        record.leftovers = group_members(child.id());
        if !record.leftovers.is_empty() {
            kill_group(child.id());
            record.leftovers = group_members(child.id());
        }
        record.stdout_sha256 = Some(file_digest(&stdout_path, None)?);
        record.stderr_sha256 = Some(file_digest(&stderr_path, None)?);
        let success = record.status == Some(0)
            && !record.timed_out
            && record.leftovers.is_empty()
            && !aborted
            && write_result.is_ok();
        self.save()?;
        let bytes = crate::bytes(&stdout_path)?;
        if rpc && success {
            let session = if self.journal.job.coding.is_some() {
                self.coding_complete(&bytes)?;
                self.journal
                    .session_id
                    .clone()
                    .ok_or_else(|| Error::Refused("coding session missing".into()))?
            } else if self.journal.job.receiver.is_some() {
                self.receiver_complete(&bytes, number)?;
                self.journal
                    .session_id
                    .clone()
                    .ok_or_else(|| Error::Refused("receiver session missing".into()))?
            } else {
                verify_rpc(&bytes, true)?
                    .ok_or_else(|| Error::Refused("RPC replies incomplete".into()))?
            };
            let stdout_sha256 = digest(&bytes);
            if self.journal.processes[number].stdout_sha256.as_deref() != Some(&stdout_sha256) {
                return refuse("completed RPC output changed during verification");
            }
            self.journal.rpc_completion = Some(RpcCompletion {
                process_index: number,
                stdout_sha256,
                session_id: session.clone(),
            });
            self.journal.session_id = Some(session);
            self.journal.rpc_verified = true;
            self.save()?;
        }
        Ok((success, bytes))
    }
    fn create_args(&self) -> Vec<String> {
        if self.journal.job.coding.is_some() {
            return self.coding_inference_args();
        }
        let j = &self.journal;
        let r = &j.job;
        let mut args = vec![
            "create".into(),
            "--pull".into(),
            "never".into(),
            "--interactive".into(),
            "--name".into(),
            j.container_name.clone(),
            "--label".into(),
            format!("work-engine.pi-job={}", r.job_id),
            "--label".into(),
            format!("work-engine.pi-input={}", j.input_digest),
            "--network".into(),
            "none".into(),
            "--read-only".into(),
            "--cap-drop".into(),
            "ALL".into(),
            "--security-opt".into(),
            "no-new-privileges".into(),
            "--pids-limit".into(),
            "64".into(),
            "--memory".into(),
            "512m".into(),
            "--cpus".into(),
            "1".into(),
            "--user".into(),
            "1000:1000".into(),
            "--tmpfs".into(),
            "/tmp:rw,noexec,nosuid,size=64m".into(),
            "--env".into(),
            "HOME=/tmp/pi-home".into(),
            "--env".into(),
            "PI_CODING_AGENT_DIR=/export/config".into(),
            "--env".into(),
            "PI_OFFLINE=1".into(),
            "--workdir".into(),
            "/workspace".into(),
            "--mount".into(),
            format!(
                "type=bind,src={},dst=/pi,readonly",
                r.runtime.root.display()
            ),
            "--mount".into(),
            format!(
                "type=bind,src={},dst=/workspace,readonly",
                r.workspace.root.display()
            ),
            "--mount".into(),
            format!(
                "type=bind,src={},dst=/export",
                r.state_dir.join("exports").display()
            ),
            r.image.clone(),
            "node".into(),
            format!("/pi/{PI}"),
            "--mode".into(),
            "rpc".into(),
            "--offline".into(),
            "--no-tools".into(),
            "--no-mcp".into(),
            "--no-extensions".into(),
            "--no-skills".into(),
            "--no-prompt-templates".into(),
            "--no-themes".into(),
            "--no-context-files".into(),
            "--no-approve".into(),
            "--session-dir".into(),
            "/export/sessions".into(),
        ];
        if let Some(profile) = &r.receiver {
            let image = args.iter().position(|s| s == &r.image).expect("image");
            args.truncate(image + 1);
            args.extend(runtime_command(r));
            args.splice(
                image..image,
                [
                    "--mount".into(),
                    format!(
                        "type=bind,src={},dst=/receiver/receiver.mjs,readonly",
                        profile.shim.path.display()
                    ),
                ],
            );
        }
        args
    }
    fn inspect_container(&mut self) -> Result<Value> {
        let name = self.journal.container_name.clone();
        let (ok, bytes) = self.command("inspect", vec!["inspect".into(), name], false)?;
        if !ok {
            return refuse("Docker inspect unavailable; container state unknown");
        }
        let array: Vec<Value> = serde_json::from_slice(&bytes)?;
        if array.len() != 1 {
            return refuse("Docker inspect identity ambiguous");
        }
        let v = array.into_iter().next().unwrap();
        verify_container(&self.journal, &v)?;
        let id = v["Id"]
            .as_str()
            .ok_or_else(|| Error::Refused("missing container ID".into()))?;
        self.journal.container_id = Some(id.into());
        self.save()?;
        Ok(v)
    }
    pub fn run(&mut self) -> Result<Value> {
        if self.journal.phase != Phase::Admitted {
            return refuse("existing realization must reconcile; launch is not replayable");
        }
        if self
            .journal
            .job
            .coding
            .as_ref()
            .is_some_and(|p| coding::is_bridge(&p.schema))
            && self.coding_gate.is_none()
        {
            return refuse("coding lifecycle profile requires installed admission gate");
        }
        if self.signed_pi() && self.pi_owner.is_none() {
            return refuse("signed fixture cannot resume allocation");
        }
        let launch = (|| -> Result<()> {
            check_job(&self.journal.job, true)?;
            if self.cancelled() {
                return refuse("cancelled before create");
            }
            if self.journal.job.coding.is_some() {
                self.coding_prepare_tool()?;
            }
            if let Some(owner) = &self.pi_owner {
                self.allocation_lease = Some(
                    owner
                        .enter(session_task_admission::pi_pilot::Effect::Allocation)
                        .map_err(|e| Error::Refused(e.into()))?,
                );
            }
            self.journal.phase = Phase::CreateUnknown;
            self.save()?;
            let (ok, output) = self.command("create", self.create_args(), false)?;
            if !ok {
                return refuse("Docker create uncertain; no automatic retry");
            }
            let id = std::str::from_utf8(&output).unwrap_or("").trim();
            if !pin_is_hash(id) {
                return refuse("Docker create returned invalid identity");
            }
            self.journal.container_id = Some(id.into());
            self.journal.phase = Phase::Created;
            self.save()?;
            self.inspect_container()?;
            if self.expired() || self.cancelled() {
                return refuse("no admission for start after limit/cancel");
            }
            self.journal.phase = Phase::StartUnknown;
            self.save()?;
            let name = self.journal.container_name.clone();
            let (ok, _) = self.command(
                "rpc",
                vec![
                    "start".into(),
                    "--attach".into(),
                    "--interactive".into(),
                    name,
                ],
                true,
            )?;
            if !ok {
                return refuse("Pi attach/start failed or uncertain");
            }
            if self.journal.job.coding.is_some() {
                self.coding_validate_task()?;
            }
            self.journal.phase = Phase::Running;
            self.save()?;
            Ok(())
        })();
        if let Err(error) = launch {
            self.failure(error);
            self.save()?;
        }
        drop(self.allocation_lease.take());
        self.reply_leases.clear();
        let result = self.reconcile()?;
        if let Some(owner) = &self.pi_owner {
            owner
                .record_result(&result)
                .map_err(|e| Error::Refused(e.into()))?;
        }
        Ok(result)
    }
    /// Recovery never issues create/start or resends RPC. A running orphan is
    /// stopped and collected; uncertain facts cannot authorize launch replay.
    pub fn reconcile(&mut self) -> Result<Value> {
        if self.journal.phase == Phase::Collected {
            return self.receipt();
        }
        for process in &mut self.journal.processes {
            if process.status.is_none()
                && let (Some(pid), Some(expected)) = (process.pid, &process.start)
                && let Some((actual, _, _)) = crate::proc_stat(pid)
                && &actual == expected
            {
                kill_group(pid);
                for _ in 0..50 {
                    if group_members(pid).is_empty() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                process.leftovers = group_members(pid);
                if !process.leftovers.is_empty() {
                    return refuse("prior owned CLI process not collected");
                }
            }
            if process.status.is_none() {
                // Lost exit status stays unknown. Retain any bounded raw bytes
                // after the old CLI has stopped, without inventing its exit.
                if process.stdout.exists() && fs::metadata(&process.stdout)?.len() <= LIMIT {
                    process.stdout_sha256 = Some(file_digest(&process.stdout, None)?);
                }
                if process.stderr.exists() && fs::metadata(&process.stderr)?.len() <= LIMIT {
                    process.stderr_sha256 = Some(file_digest(&process.stderr, None)?);
                }
            }
        }
        self.journal.phase = Phase::Collecting;
        self.save()?;
        let collect = (|| -> Result<()> {
            let mut inspected = match self.inspect_container() {
                Ok(value) => value,
                Err(error) if self.journal.job.coding.is_some() => {
                    if self.coding_confirm_inference_absence()? {
                        self.journal.removed = true;
                        self.save()?;
                        return Ok(());
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            };
            let name = self.journal.container_name.clone();
            if inspected["State"]["Running"] == true {
                self.failure("running container required recovery/cancel stop");
                let (stopped, _) = self.command(
                    "stop",
                    vec!["stop".into(), "--time".into(), "1".into(), name.clone()],
                    false,
                )?;
                if !stopped {
                    let _ = self.command("kill", vec!["kill".into(), name.clone()], false)?;
                }
                inspected = self.inspect_container()?;
            }
            if inspected["State"]["Running"] != false || inspected["State"]["OOMKilled"] != false {
                return refuse("daemon exit not established");
            }
            if inspected["State"]["Status"] == "exited" {
                let (waited, output) =
                    self.command("wait", vec!["wait".into(), name.clone()], false)?;
                let exit: i32 = std::str::from_utf8(&output)
                    .unwrap_or("")
                    .trim()
                    .parse()
                    .map_err(|_| Error::Refused("invalid daemon wait output".into()))?;
                if !waited || inspected["State"]["ExitCode"].as_i64() != Some(i64::from(exit)) {
                    return refuse("daemon wait/inspect exit mismatch");
                }
                self.journal.daemon_exit = Some(exit);
                self.save()?;
            } else if inspected["State"]["Status"] != "created" {
                return refuse("daemon state is neither exited nor unstarted");
            }
            let (removed, _) = self.command("remove", vec!["rm".into(), name.clone()], false)?;
            if !removed {
                return refuse("container removal uncertain");
            }
            let (listed, output) = self.command(
                "confirm-removed",
                vec![
                    "ps".into(),
                    "--all".into(),
                    "--no-trunc".into(),
                    "--filter".into(),
                    format!("name=^/{name}$"),
                    "--format".into(),
                    "{{.ID}}".into(),
                ],
                false,
            )?;
            if !listed || !output.iter().all(u8::is_ascii_whitespace) {
                return refuse("container absence not established");
            }
            self.journal.removed = true;
            self.save()?;
            Ok(())
        })();
        if let Err(error) = collect {
            self.failure(error);
        }
        if self.journal.job.coding.is_some()
            && let Err(error) = self.coding_collect_tool()
        {
            self.failure(error);
        }
        if self.journal.job.coding.is_some()
            && let Err(error) = self.coding_collect_validation()
        {
            self.failure(error);
        }
        if self.journal.job.coding.is_some()
            && let Err(error) = self.coding_export_validation()
        {
            self.failure(error);
        }
        if self.journal.job.coding.is_some()
            && let Err(error) = self.coding_cleanup_secret()
        {
            self.failure(error);
        }
        if let Err(error) = self
            .journal
            .job
            .runtime
            .verify()
            .and_then(|_| self.journal.job.workspace.verify())
        {
            self.failure(error);
        }
        if let Err(error) = self.collect_artifacts() {
            self.failure(error);
        }
        if self.journal.removed
            && self.journal.coding.as_ref().is_none_or(|c| {
                (c.tool_removed || !c.tool_create_intent)
                    && (c.validation_removed || !c.validation_intent)
                    && (self.journal.job.coding.as_ref().unwrap().provider.is_none()
                        || c.access_removed)
            })
        {
            self.journal.phase = Phase::Collected;
        }
        self.save()?;
        self.receipt()
    }
    fn collect_artifacts(&mut self) -> Result<()> {
        let root = self.journal.job.state_dir.join("exports");
        let entries = inventory(&root)?;
        for (path, entry) in entries {
            let Entry::File { sha256 } = entry else {
                return refuse("export symlink is not a durable artifact");
            };
            let artifact = Artifact {
                path: root.join(&path),
                sha256,
            };
            let snapshot = artifact.snapshot(&self.journal.job.state_dir.join("snapshots"))?;
            let pin = Artifact {
                path: snapshot,
                sha256: artifact.sha256,
            };
            self.journal.exported_files.insert(path, pin.clone());
            if !self.journal.artifacts.iter().any(|a| a.path == pin.path) {
                self.journal.artifacts.push(pin);
            }
        }
        // Preserve protocol/process bytes as host snapshots, not only mutable logs.
        for process in &self.journal.processes {
            for (path, sha256) in [
                (&process.stdout, &process.stdout_sha256),
                (&process.stderr, &process.stderr_sha256),
            ] {
                if let Some(sha256) = sha256 {
                    let artifact = Artifact {
                        path: path.clone(),
                        sha256: sha256.clone(),
                    };
                    let snapshot =
                        artifact.snapshot(&self.journal.job.state_dir.join("snapshots"))?;
                    if !self.journal.artifacts.iter().any(|a| a.path == snapshot) {
                        self.journal.artifacts.push(Artifact {
                            path: snapshot,
                            sha256: sha256.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }
    pub fn receipt(&mut self) -> Result<Value> {
        for artifact in &self.journal.artifacts {
            artifact.verify()?;
        }
        let pass = self.journal.coding.as_ref().is_none_or(|c| {
            c.tool_removed
                && c.completed
                && self.coding_final_proof_valid()
                && (self.journal.job.coding.as_ref().unwrap().provider.is_none()
                    || c.access_removed)
        }) && self.journal.rpc_verified
            && self.journal.rpc_completion.as_ref().is_some_and(|proof| {
                self.journal.session_id.as_deref() == Some(&proof.session_id)
                    && self
                        .journal
                        .processes
                        .get(proof.process_index)
                        .is_some_and(|p| {
                            p.stdout_sha256.as_deref() == Some(&proof.stdout_sha256)
                                && p.status == Some(0)
                        })
                    && self
                        .journal
                        .artifacts
                        .iter()
                        .any(|a| a.sha256 == proof.stdout_sha256)
            })
            && self.journal.daemon_exit == Some(0)
            && self.journal.removed
            && self.journal.failures.is_empty()
            && self.journal.processes.iter().all(|p| {
                (p.status == Some(0)
                    || (self.journal.coding.is_some()
                        && p.status.is_some()
                        && p.stdout.file_name().is_some_and(|name| {
                            name.to_string_lossy().contains("-coding-tool-exec-")
                        })))
                    && !p.timed_out
                    && p.leftovers.is_empty()
            });
        let mut value = json!({"schema":SCHEMA,"job":self.journal.job,"job_id":self.journal.job.job_id,"input_digest":self.journal.input_digest,"status":if pass {"component_passed"} else {"not_established"},"acceptance":"owner_pending","domain_effect_settlement":"not_claimed","provider":"disabled","container_id":self.journal.container_id,"session_id":self.journal.session_id,"phase":self.journal.phase,"rpc_verified":self.journal.rpc_verified,"rpc_completion":self.journal.rpc_completion,"daemon_exit":self.journal.daemon_exit,"removed":self.journal.removed,"failures":self.journal.failures,"artifacts":self.journal.artifacts,"exported_files":self.journal.exported_files,"processes":self.journal.processes});
        if let Some(observation) = &self.journal.receiver {
            value["receiver"] = serde_json::to_value(observation)?;
            value["profile"] = json!(receiver::PROFILE);
            value["cognition_application_completion"] = json!("not_claimed");
        }
        if let Some(coding) = &self.journal.coding {
            value["coding"] = serde_json::to_value(coding)?;
            value["profile"] = json!(self.journal.job.coding.as_ref().unwrap().schema);
            if self.signed_pi() && !self.signed_subscription() {
                value["provider"] = json!("fake_fixture");
                value["model"] = json!("fake-pi-model");
                value["cost_evidence"] = json!("synthetic zero-cost fixture usage; not billing");
            }
            if self.journal.job.coding.as_ref().unwrap().provider.is_some() {
                value["provider"] = json!("openai-codex");
                value["model"] = json!("gpt-6-luna");
                value["cost_evidence"] =
                    json!("pinned_catalog_proxy_only_not_subscription_billing");
            }
        }
        let bytes = serde_json::to_vec(&value)?;
        let id = digest(&bytes);
        let path = self
            .journal
            .job
            .state_dir
            .join("receipts")
            .join(format!("{id}.json"));
        if path.exists() {
            if fs::read(&path)? != bytes {
                return refuse("receipt content conflict");
            }
        } else {
            let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            File::open(self.journal.job.state_dir.join("receipts"))?.sync_all()?;
        }
        self.journal.receipt_id = Some(id.clone());
        self.save()?;
        Ok(json!({"receipt_id":id,"receipt":value}))
    }
    pub fn acknowledge(&mut self, id: &str) -> Result<Value> {
        if !pin_is_hash(id) {
            return refuse("invalid receipt ID");
        }
        let bytes = fs::read(
            self.journal
                .job
                .state_dir
                .join("receipts")
                .join(format!("{id}.json")),
        )?;
        if digest(&bytes) != id {
            return refuse("receipt integrity conflict");
        }
        let value: Value = serde_json::from_slice(&bytes)?;
        if value["input_digest"] != self.journal.input_digest {
            return refuse("receipt job conflict");
        }
        let artifacts: Vec<Artifact> = serde_json::from_value(value["artifacts"].clone())?;
        for artifact in artifacts {
            artifact.verify()?;
        }
        self.journal.acknowledged.insert(id.into());
        self.save()?;
        Ok(json!({"receipt_id":id,"delivery":"acknowledged","acceptance":"owner_pending"}))
    }
}

pub fn request_cancel(dir: &Path) -> Result<Value> {
    canonical_directory(dir)?;
    let journal: Journal = serde_json::from_slice(&crate::bytes(&dir.join("pi-state.json"))?)?;
    if journal.schema != SCHEMA
        || journal.job.state_dir != dir
        || journal.input_digest != digest(&serde_json::to_vec(&journal.job)?)
    {
        return refuse("cancel job binding conflict");
    }
    atomic(
        &dir.join("cancel.json"),
        &json!({"input_digest":journal.input_digest,"requested_at":now()}),
    )?;
    Ok(
        json!({"status":"cancellation_requested","job_id":journal.job.job_id,"settlement":"not_claimed"}),
    )
}

fn rpc_input() -> Vec<u8> {
    b"{\"id\":\"state-before\",\"type\":\"get_state\"}\n{\"id\":\"disable-auto-compact\",\"type\":\"set_auto_compaction\",\"enabled\":false}\n{\"id\":\"disable-auto-retry\",\"type\":\"set_auto_retry\",\"enabled\":false}\n{\"id\":\"state-after\",\"type\":\"get_state\"}\n".to_vec()
}

fn verify_rpc(bytes: &[u8], complete: bool) -> Result<Option<String>> {
    if bytes.len() as u64 > LIMIT {
        return refuse("RPC output too large");
    }
    let mut responses = BTreeMap::new();
    let end = bytes
        .iter()
        .rposition(|b| *b == b'\n')
        .map(|n| n + 1)
        .unwrap_or(0);
    if complete && end != bytes.len() {
        return refuse("truncated RPC record");
    }
    for line in bytes[..end]
        .split(|b| *b == b'\n')
        .filter(|s| !s.is_empty())
    {
        let v: Value = serde_json::from_slice(line)?;
        if v["type"] != "response" {
            return refuse("unexpected event in provider-disabled fixture");
        }
        let id = v["id"]
            .as_str()
            .ok_or_else(|| Error::Refused("RPC missing correlation ID".into()))?
            .to_string();
        let command = match id.as_str() {
            "state-before" | "state-after" => "get_state",
            "disable-auto-compact" => "set_auto_compaction",
            "disable-auto-retry" => "set_auto_retry",
            _ => return refuse("RPC unexpected correlation ID"),
        };
        if v["command"] != command || v["success"] != true || responses.insert(id, v).is_some() {
            return refuse("RPC failed/mismatched/duplicate response");
        }
    }
    if responses.len() != 4 {
        if complete {
            return refuse("incomplete RPC response set");
        }
        return Ok(None);
    }
    let before = &responses["state-before"]["data"];
    let after = &responses["state-after"]["data"];
    let session = after["sessionId"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Refused("RPC missing session identity".into()))?;
    if before["sessionId"] != session
        || after["autoCompactionEnabled"] != false
        || after["isStreaming"] != false
        || after["isCompacting"] != false
        || after["messageCount"] != 0
        || after["pendingMessageCount"] != 0
        || after["model"]["provider"] != "unknown"
    {
        return refuse("RPC fixture state/identity conflict");
    }
    Ok(Some(session.into()))
}

fn runtime_command(job: &Job) -> Vec<String> {
    if job.receiver.is_some() {
        return vec!["node".into(), "/receiver/receiver.mjs".into()];
    }
    vec![
        "node",
        &format!("/pi/{PI}"),
        "--mode",
        "rpc",
        "--offline",
        "--no-tools",
        "--no-mcp",
        "--no-extensions",
        "--no-skills",
        "--no-prompt-templates",
        "--no-themes",
        "--no-context-files",
        "--no-approve",
        "--session-dir",
        "/export/sessions",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn verify_container(j: &Journal, v: &Value) -> Result<()> {
    if j.job.coding.is_some() {
        return coding::verify_inference(j, v);
    }
    let id = v["Id"].as_str().unwrap_or("");
    if !pin_is_hash(id)
        || j.container_id.as_ref().is_some_and(|old| old != id)
        || v["Name"] != format!("/{}", j.container_name)
        || v["Config"]["Labels"]["work-engine.pi-input"] != j.input_digest
        || v["Config"]["Labels"]["work-engine.pi-job"] != j.job.job_id
        || v["Config"]["Image"] != j.job.image
        || v["Config"]["User"] != "1000:1000"
        || v["HostConfig"]["NetworkMode"] != "none"
        || v["HostConfig"]["ReadonlyRootfs"] != true
        || v["HostConfig"]["Privileged"] != false
        || v["HostConfig"]["CapDrop"] != json!(["ALL"])
        || v["HostConfig"]["Memory"] != 536870912_u64
        || v["HostConfig"]["PidsLimit"] != 64
        || v["HostConfig"]["NanoCpus"] != 1_000_000_000_u64
        || v["HostConfig"]["SecurityOpt"] != json!(["no-new-privileges"])
    {
        return refuse("container identity/containment conflict; refusing custody effects");
    }
    let expected_command = runtime_command(&j.job);
    if v["Config"]["Cmd"] != json!(expected_command)
        || v["Config"]["Entrypoint"] != json!(["docker-entrypoint.sh"])
        || v["Config"]["WorkingDir"] != "/workspace"
        || v["HostConfig"]["Tmpfs"] != json!({"/tmp":"rw,noexec,nosuid,size=64m"})
    {
        return refuse("container command/tmpfs realization conflict");
    }
    let mounts = v["Mounts"]
        .as_array()
        .ok_or_else(|| Error::Refused("missing mount evidence".into()))?;
    let export_root = j.job.state_dir.join("exports");
    let mut expected = vec![
        (&j.job.runtime.root, "/pi", false),
        (&j.job.workspace.root, "/workspace", false),
        (&export_root, "/export", true),
    ];
    if let Some(profile) = &j.job.receiver {
        expected.push((&profile.shim.path, "/receiver/receiver.mjs", false));
    }
    if mounts.len() != expected.len()
        || expected.iter().any(|(source, target, rw)| {
            !mounts.iter().any(|m| {
                m["Source"] == source.to_string_lossy().as_ref()
                    && m["Destination"] == *target
                    && m["RW"] == *rw
                    && m["Type"] == "bind"
            })
        })
    {
        return refuse("container mount realization conflict");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn replies() -> Vec<u8> {
        let mut out = Vec::new();
        for line in rpc_input().split(|b| *b == b'\n').filter(|s| !s.is_empty()) {
            let c: Value = serde_json::from_slice(line).unwrap();
            let mut v = json!({"id":c["id"],"type":"response","command":c["type"],"success":true});
            if c["type"] == "get_state" {
                v["data"] = json!({"sessionId":"session","model":{"provider":"unknown"},"autoCompactionEnabled":c["id"]=="state-before","isStreaming":false,"isCompacting":false,"messageCount":0,"pendingMessageCount":0});
            }
            serde_json::to_writer(&mut out, &v).unwrap();
            out.push(b'\n');
        }
        out
    }
    #[test]
    fn rpc_requires_complete_correlated_identity_and_state() {
        let good = replies();
        assert_eq!(verify_rpc(&good, true).unwrap(), Some("session".into()));
        assert!(verify_rpc(&good[..good.len() - 1], true).is_err());
        assert!(verify_rpc(b"{}\n", true).is_err());
        let mut duplicate = good.clone();
        duplicate.extend_from_slice(&good);
        assert!(verify_rpc(&duplicate, true).is_err());
        let mismatch = String::from_utf8(good)
            .unwrap()
            .replace("state-after", "foreign");
        assert!(verify_rpc(mismatch.as_bytes(), true).is_err());
        assert!(verify_rpc(b"{broken}\n", false).is_err());
        assert_eq!(verify_rpc(b"{\"partial\":", false).unwrap(), None);
    }
    #[test]
    fn inventory_rejects_escape_and_tracks_added_files() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        fs::write(root.path().join("one"), "a").unwrap();
        let first = inventory(root.path()).unwrap();
        fs::write(root.path().join("two"), "b").unwrap();
        assert_ne!(first, inventory(root.path()).unwrap());
        std::os::unix::fs::symlink(other.path(), root.path().join("escape")).unwrap();
        assert!(inventory(root.path()).is_err());
    }
    fn artifact(path: &Path, bytes: &[u8]) -> Artifact {
        fs::write(path, bytes).unwrap();
        Artifact {
            path: path.into(),
            sha256: digest(bytes),
        }
    }
    fn fixture(binary: &Path, mode: &str) -> (tempfile::TempDir, Job) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let runtime = root.join("runtime");
        let workspace = root.join("workspace");
        let state = root.join("custody");
        for p in [&runtime, &workspace, &state] {
            fs::create_dir(p).unwrap();
        }
        fs::create_dir_all(
            runtime.join("node_modules/@earendil-works/pi-coding-agent/dist/bundle"),
        )
        .unwrap();
        fs::write(runtime.join(PI), "fixture only").unwrap();
        fs::write(
            runtime.join("node_modules/@earendil-works/pi-coding-agent/package.json"),
            r#"{"name":"@earendil-works/pi-coding-agent","version":"1.5.0"}"#,
        )
        .unwrap();
        fs::write(workspace.join("input.txt"), "pinned fixture input").unwrap();
        let runtime_manifest = artifact(
            &root.join("runtime.json"),
            &serde_json::to_vec(&inventory(&runtime).unwrap()).unwrap(),
        );
        let workspace_manifest = artifact(
            &root.join("workspace.json"),
            &serde_json::to_vec(&inventory(&workspace).unwrap()).unwrap(),
        );
        let executable = root.join("docker");
        fs::copy(binary, &executable).unwrap();
        fs::write(executable.with_extension("mode"), mode).unwrap();
        let job = Job {
            schema: SCHEMA.into(),
            job_id: "fixture".into(),
            state_dir: state,
            authority: artifact(&root.join("authority"), b"isolated test authority"),
            packet: artifact(&root.join("packet"), b"provider disabled fake daemon test"),
            docker: Artifact {
                path: executable.clone(),
                sha256: file_digest(&executable, None).unwrap(),
            },
            image: IMAGE.into(),
            runtime: Tree {
                root: runtime,
                manifest: runtime_manifest,
            },
            workspace: Tree {
                root: workspace,
                manifest: workspace_manifest,
            },
            deadline_unix_seconds: now() + 20,
            command_timeout_seconds: 2,
            idle_seconds: 0,
            receiver: None,
            coding: None,
        };
        (dir, job)
    }
    #[test]
    fn malformed_admission_and_foreign_inputs_have_no_container_effects() {
        let (dir, job) = fixture(&std::env::current_exe().unwrap(), "unused");
        for variant in 0..5 {
            let mut invalid = job.clone();
            match variant {
                0 => invalid.schema = "foreign".into(),
                1 => invalid.job_id = "../escape".into(),
                2 => invalid.image = "node:latest".into(),
                3 => invalid.deadline_unix_seconds = 0,
                _ => invalid.idle_seconds = 31,
            }
            assert!(Host::admit(invalid).is_err());
        }
        fs::write(job.workspace.root.join("unadmitted"), "extra").unwrap();
        assert!(Host::admit(job.clone()).is_err());
        assert!(!job.state_dir.join("pi-state.json").exists());
        assert!(!dir.path().join("docker.calls").exists());
        let mut value = serde_json::to_value(job).unwrap();
        value["unexpected"] = json!(true);
        assert!(serde_json::from_value::<Job>(value).is_err());
    }

    #[test]
    #[ignore = "requires two explicitly fake adapter binaries; never real Docker or Pi"]
    fn fake_receiver_readiness_native_proof_unknown_and_restart() {
        let base = PathBuf::from(std::env::var_os("WE_PI_DOCKER_FIXTURE").expect("fake base only"));
        let receiver =
            PathBuf::from(std::env::var_os("WE_PI_RECEIVER_FIXTURE").expect("fake receiver only"));
        for mode in [
            "observe",
            "missing-end",
            "missing-messages",
            "missing-history",
            "partial-tail",
            "wrong-session",
            "wrong-attempt",
            "busy",
            "no-release",
        ] {
            let (dir, mut job) = fixture(&base, mode);
            let exe = job.docker.path.clone();
            fs::rename(&exe, exe.with_extension("base")).unwrap();
            fs::copy(&receiver, &exe).unwrap();
            job.docker.sha256 = file_digest(&exe, None).unwrap();
            let sdk = job
                .runtime
                .root
                .join("node_modules/@earendil-works/pi-coding-agent/dist/index.js");
            fs::write(&sdk, "fake runtime never executed").unwrap();
            job.runtime.manifest = artifact(
                &dir.path().join("runtime-v2.json"),
                &serde_json::to_vec(&inventory(&job.runtime.root).unwrap()).unwrap(),
            );
            let event = json!({"event_id":"e".repeat(64),"source_id":"synthetic","repository":"synthetic","source_key":"manual-context-control-v1","revision":"fixture","epoch":"null","message_id":"fixture-event","sequence":0,"observed_at":"","kind":"context_pressure","schema":"manual-context-control-v1","payload_codec":"reader-output-utf8-v1","raw_payload":"{\"event\":{\"level\":\"unavailable\",\"input_tokens\":null}}","payload_sha256":digest(br#"{"event":{"level":"unavailable","input_tokens":null}}"#)});
            job.receiver = Some(receiver::Profile {
                schema: receiver::PROFILE.into(),
                shim: artifact(&dir.path().join("shim.mjs"), receiver::SHIM),
                event: artifact(
                    &dir.path().join("event.json"),
                    &serde_json::to_vec(&event).unwrap(),
                ),
                attempt_number: 1,
                recipient_without_session: json!({"recipient":"target","principal":"fixture","job":"fixture","lane":"lane","workspace":"/workspace","generation":1,"address":"role:target","fixture_mode":"observe","expires_at":4102444800u64,"active":true}),
            });
            job.deadline_unix_seconds = now() + 25;
            let state = job.state_dir.clone();
            let mut host = Host::admit(job).unwrap();
            let receipt = host.run().unwrap();
            let observed = receipt["receipt"]["receiver"]["target_observed"] == true;
            assert_eq!(observed, mode == "observe", "mode={mode},receipt={receipt}");
            assert_eq!(receipt["receipt"]["removed"], true, "{mode}: {receipt}");
            assert_eq!(
                receipt["receipt"]["status"] == "component_passed",
                mode == "observe",
                "{mode}"
            );
            if mode == "observe" {
                assert!(
                    !receipt["receipt"]["receiver"]["native_history"]["sha256"]
                        .as_str()
                        .unwrap()
                        .is_empty()
                );
                host.acknowledge(receipt["receipt_id"].as_str().unwrap())
                    .unwrap();
            }
            drop(host);
            let before = fs::read_to_string(exe.with_extension("calls")).unwrap();
            let mut reopened = Host::open(&state).unwrap();
            reopened.reconcile().unwrap();
            let after = fs::read_to_string(exe.with_extension("calls")).unwrap();
            assert_eq!(
                before, after,
                "no create/start/resend after completed/unknown {mode}"
            );
            assert_eq!(before.lines().filter(|s| *s == "create").count(), 1);
            assert_eq!(before.lines().filter(|s| *s == "start").count(), 1);
            println!(
                "fake-only {mode}: observed={observed}, daemon cleanup and no replay verified"
            );
        }
    }

    #[test]
    #[ignore = "real Docker/Pi provider-disabled proof; requires WE_PI_LIVE_JOB template"]
    fn real_daemon_cancel_and_restart_without_launch_replay() {
        let path = std::env::var_os("WE_PI_LIVE_JOB")
            .expect("supply private admitted provider-disabled job template");
        let template: Job = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let base = template.state_dir.clone();
        for mode in ["cancel", "restart"] {
            let mut job = template.clone();
            job.state_dir = base.join(mode);
            fs::create_dir(&job.state_dir).unwrap();
            job.job_id = format!("{}-{mode}", job.job_id);
            job.deadline_unix_seconds = now() + 120;
            job.idle_seconds = 20;
            let mut host = Host::admit(job.clone()).unwrap();
            if mode == "cancel" {
                let state = job.state_dir.clone();
                let cancel = std::thread::spawn(move || {
                    for _ in 0..5000 {
                        let raw = fs::read(state.join("pi-state.json")).unwrap();
                        let j: Journal = serde_json::from_slice(&raw).unwrap();
                        if j.rpc_verified {
                            request_cancel(&state).unwrap();
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    panic!("real Pi never became observable");
                });
                let result = host.run().unwrap();
                cancel.join().unwrap();
                assert_eq!(result["receipt"]["status"], "not_established");
                assert!(host.journal.rpc_verified);
                assert!(host.journal.removed);
                assert!(host.journal.daemon_exit.is_some());
            } else {
                // Actual Docker daemon starts Pi with its stdin open; deliberately
                // leave the journal at start intent, then recover through a new Host.
                host.journal.phase = Phase::CreateUnknown;
                host.save().unwrap();
                let (ok, _) = host.command("create", host.create_args(), false).unwrap();
                assert!(ok);
                host.inspect_container().unwrap();
                host.journal.phase = Phase::StartUnknown;
                host.save().unwrap();
                let name = host.journal.container_name.clone();
                let (ok, _) = host
                    .command("fixture-detached-start", vec!["start".into(), name], false)
                    .unwrap();
                assert!(ok);
                let container = host.inspect_container().unwrap();
                assert_eq!(container["State"]["Running"], true);
                let orphan_log = job.state_dir.join(format!(
                    "{:04}-inspect-0.stdout",
                    host.journal.processes.len()
                ));
                fs::write(&orphan_log, b"real interrupted inspect reservation\n").unwrap();
                drop(host);
                let mut recovered = Host::open(&job.state_dir).unwrap();
                let result = recovered.reconcile().unwrap();
                assert_eq!(
                    fs::read(orphan_log).unwrap(),
                    b"real interrupted inspect reservation\n"
                );
                assert_eq!(result["receipt"]["status"], "not_established");
                assert!(recovered.journal.removed);
                assert!(recovered.journal.daemon_exit.is_some());
                assert_eq!(
                    recovered
                        .journal
                        .processes
                        .iter()
                        .filter(|p| p.argv.get(1).is_some_and(|a| a == "create"))
                        .count(),
                    1
                );
                assert!(
                    !recovered.journal.processes.iter().any(|p| p
                        .argv
                        .get(1)
                        .is_some_and(|a| a == "start")
                        && p.argv.iter().any(|a| a == "--attach"))
                );
            }
        }
    }
    #[test]
    #[ignore = "requires built pi_docker_fixture example through WE_PI_DOCKER_FIXTURE"]
    fn fake_daemon_failure_restart_cancel_and_integrity() {
        let binary = PathBuf::from(
            std::env::var_os("WE_PI_DOCKER_FIXTURE")
                .expect("build example and supply exact fixture path"),
        );
        for mode in [
            "ok",
            "rpc-mismatch",
            "rpc-truncated",
            "create-uncertain",
            "inspect-error",
            "remove-error",
            "wait-mismatch",
            "bad-mount",
            "start-stall",
        ] {
            let (dir, mut job) = fixture(&binary, mode);
            if mode == "start-stall" {
                job.deadline_unix_seconds = now() + 2;
            }
            let mut host = Host::admit(job.clone()).unwrap();
            let result = host.run().unwrap();
            assert_eq!(
                result["receipt"]["status"],
                if mode == "ok" {
                    "component_passed"
                } else {
                    "not_established"
                },
                "{mode}: {result}"
            );
            let calls = fs::read_to_string(dir.path().join("docker.calls")).unwrap();
            assert_eq!(calls.lines().filter(|s| *s == "create").count(), 1);
            assert!(host.run().is_err());
            if mode == "ok" {
                assert!(host.journal.rpc_completion.is_some());
                let id = result["receipt_id"].as_str().unwrap();
                host.acknowledge(id).unwrap();
                assert_eq!(host.receipt().unwrap()["receipt_id"], id);
                let pin = host.journal.artifacts[0].path.clone();
                fs::write(pin, "tampered").unwrap();
                assert!(host.acknowledge(id).is_err());
                assert!(host.receipt().is_err());
            }
            drop(host);
            if mode != "ok" {
                let mut reopened = Host::open(&job.state_dir).unwrap();
                let _ = reopened.reconcile();
                let calls = fs::read_to_string(dir.path().join("docker.calls")).unwrap();
                assert_eq!(calls.lines().filter(|s| *s == "create").count(), 1);
                assert!(calls.lines().filter(|s| *s == "start").count() <= 1);
            }
        }
        let (_dir, mut job) = fixture(&binary, "start-stall");
        job.deadline_unix_seconds = now() + 20;
        let state = job.state_dir.clone();
        let marker = state.clone();
        let mut host = Host::admit(job).unwrap();
        let cancel = std::thread::spawn(move || {
            // Admission hashes the pinned executable before publishing state.
            // Wait for actual start intent rather than racing that work.
            for _ in 0..500 {
                let data = fs::read(marker.join("pi-state.json")).unwrap();
                let state: Journal = serde_json::from_slice(&data).unwrap();
                if state.phase == Phase::StartUnknown {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            request_cancel(&marker).unwrap();
        });
        let result = host.run().unwrap();
        cancel.join().unwrap();
        assert_eq!(result["receipt"]["status"], "not_established");
        assert!(host.journal.removed);
        assert!(
            host.journal
                .failures
                .iter()
                .any(|s| s.contains("cancellation"))
        );
        let (_dir, mut job) = fixture(&binary, "ok");
        job.image = "node:latest".into();
        assert!(Host::admit(job).is_err());
        let (_dir, job) = fixture(&binary, "ok");
        fs::write(job.workspace.root.join("injected"), "x").unwrap();
        assert!(Host::admit(job).is_err());

        // Model a host restart after a daemon start whose reply was lost.
        let (dir, job) = fixture(&binary, "ok");
        let mut host = Host::admit(job.clone()).unwrap();
        host.journal.phase = Phase::CreateUnknown;
        host.save().unwrap();
        host.command("create", host.create_args(), false).unwrap();
        let mut remote: Value =
            serde_json::from_slice(&fs::read(dir.path().join("docker.state")).unwrap()).unwrap();
        remote["State"]["Status"] = json!("running");
        remote["State"]["Running"] = json!(true);
        fs::write(
            dir.path().join("docker.state"),
            serde_json::to_vec(&remote).unwrap(),
        )
        .unwrap();
        host.journal.phase = Phase::StartUnknown;
        host.save().unwrap();
        drop(host);
        let mut host = Host::open(&job.state_dir).unwrap();
        let result = host.reconcile().unwrap();
        assert!(host.journal.removed);
        assert_eq!(result["receipt"]["status"], "not_established");
        let calls = fs::read_to_string(dir.path().join("docker.calls")).unwrap();
        assert_eq!(calls.lines().filter(|s| *s == "create").count(), 1);
        assert!(!calls.lines().any(|s| s == "start"));
        assert!(calls.lines().any(|s| s == "stop"));
    }

    #[test]
    #[ignore = "requires built pi_docker_fixture example through WE_PI_DOCKER_FIXTURE"]
    fn recovery_interruption_reservations_and_final_stream_proof() {
        let binary = PathBuf::from(
            std::env::var_os("WE_PI_DOCKER_FIXTURE").expect("provide fake Docker fixture"),
        );
        for case in ["orphan-logs", "partial-tail", "valid-without-proof"] {
            let (dir, job) = fixture(&binary, "ok");
            let mut host = Host::admit(job.clone()).unwrap();
            host.journal.phase = Phase::CreateUnknown;
            host.save().unwrap();
            let (ok, _) = host.command("create", host.create_args(), false).unwrap();
            assert!(ok);
            let mut remote: Value =
                serde_json::from_slice(&fs::read(dir.path().join("docker.state")).unwrap())
                    .unwrap();
            if case == "orphan-logs" {
                remote["State"]["Status"] = json!("running");
                remote["State"]["Running"] = json!(true);
                // Crash after file reservation but before any corresponding record.
                // Include both the old spelling and the new reservation spelling.
                for suffix in [
                    "inspect.stdout",
                    "inspect.stderr",
                    "inspect-0.stdout",
                    "inspect-0.stderr",
                ] {
                    fs::write(
                        job.state_dir.join(format!("0001-{suffix}")),
                        b"unpublished evidence\n",
                    )
                    .unwrap();
                }
            } else {
                remote["State"]["Status"] = json!("exited");
                remote["State"]["Running"] = json!(false);
                let mut bytes = replies();
                if case == "partial-tail" {
                    bytes.extend_from_slice(b"{\"partial\":");
                }
                assert_eq!(verify_rpc(&bytes, false).unwrap(), Some("session".into()));
                assert_eq!(verify_rpc(&bytes, true).is_err(), case == "partial-tail");
                let stdout = job.state_dir.join("saved-rpc.stdout");
                fs::write(&stdout, &bytes).unwrap();
                let stderr = job.state_dir.join("saved-rpc.stderr");
                fs::write(&stderr, b"").unwrap();
                // Exact interrupted state after exit/hashes were published and
                // before completed-stream parsing. All previous pass predicates hold.
                host.journal.processes.push(ProcessRecord {
                    argv: host.argv(vec![
                        "start".into(),
                        "--attach".into(),
                        "--interactive".into(),
                        host.journal.container_name.clone(),
                    ]),
                    pid: None,
                    start: None,
                    status: Some(0),
                    stdout,
                    stderr,
                    stdout_sha256: Some(digest(&bytes)),
                    stderr_sha256: Some(digest(b"")),
                    timed_out: false,
                    leftovers: vec![],
                });
                host.journal.rpc_verified = true;
                host.journal.session_id = Some("session".into());
            }
            fs::write(
                dir.path().join("docker.state"),
                serde_json::to_vec(&remote).unwrap(),
            )
            .unwrap();
            host.journal.phase = Phase::StartUnknown;
            host.save().unwrap();
            drop(host);
            // Explicitly model a journal without the newly introduced field.
            let mut raw: Value =
                serde_json::from_slice(&fs::read(job.state_dir.join("pi-state.json")).unwrap())
                    .unwrap();
            raw.as_object_mut().unwrap().remove("rpc_completion");
            fs::write(
                job.state_dir.join("pi-state.json"),
                serde_json::to_vec(&raw).unwrap(),
            )
            .unwrap();
            let mut recovered = Host::open(&job.state_dir).unwrap();
            let result = recovered.reconcile().unwrap();
            assert!(recovered.journal.removed, "{case}: {result}");
            assert_eq!(result["receipt"]["status"], "not_established", "{case}");
            assert!(recovered.journal.rpc_completion.is_none());
            if case == "orphan-logs" {
                for suffix in [
                    "inspect.stdout",
                    "inspect.stderr",
                    "inspect-0.stdout",
                    "inspect-0.stderr",
                ] {
                    assert_eq!(
                        fs::read(job.state_dir.join(format!("0001-{suffix}"))).unwrap(),
                        b"unpublished evidence\n"
                    );
                }
                assert!(
                    recovered
                        .journal
                        .processes
                        .iter()
                        .any(|p| p.argv.get(1).is_some_and(|s| s == "stop"))
                );
            } else {
                assert!(recovered.journal.failures.is_empty());
                assert!(
                    recovered
                        .journal
                        .processes
                        .iter()
                        .all(|p| p.status == Some(0))
                );
            }
            let calls = fs::read_to_string(dir.path().join("docker.calls")).unwrap();
            assert_eq!(calls.lines().filter(|s| *s == "create").count(), 1);
            assert!(!calls.lines().any(|s| s == "start"));
            if std::env::var_os("WE_PI_RETAIN_TEST_EVIDENCE").is_some() {
                println!("retained {case} {}", dir.keep().display());
            }
        }
    }
}
