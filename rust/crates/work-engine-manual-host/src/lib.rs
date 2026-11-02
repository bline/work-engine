//! Outer manual execution bookkeeping. Domain evidence remains owned by the
//! existing budget, provider, context and custody adapters.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const SCHEMA: &str = "work-engine.manual-host.v1";
const MAX_BYTES: u64 = 8 * 1024 * 1024;
static STOP: AtomicBool = AtomicBool::new(false);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Refused(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
fn refuse<T>(s: impl Into<String>) -> Result<T> {
    Err(Error::Refused(s.into()))
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bytes(path: &Path) -> Result<Vec<u8>> {
    if fs::metadata(path)?.len() > MAX_BYTES {
        return refuse(format!("oversize input {}", path.display()));
    }
    Ok(fs::read(path)?)
}
fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&bytes(path)?)?)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let tmp = path.with_extension("tmp");
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp)?;
    f.write_all(&serde_json::to_vec_pretty(value)?)?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    fs::rename(&tmp, path)?;
    File::open(
        path.parent()
            .ok_or_else(|| Error::Refused("no parent".into()))?,
    )?
    .sync_all()?;
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub path: PathBuf,
    pub sha256: String,
}
impl Artifact {
    pub fn verify(&self) -> Result<Vec<u8>> {
        let b = bytes(&self.path)?;
        if !self.path.is_absolute() || digest(&b) != self.sha256 {
            return refuse(format!("artifact conflict: {}", self.path.display()));
        }
        Ok(b)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub session_id: String,
    pub rollout: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tools {
    pub python: PathBuf,
    pub bundle_manifest: Artifact,
    pub role_host: Artifact,
    pub scope_budget: Artifact,
    pub usage_ledger: Artifact,
    pub child_context: Artifact,
    pub context_binary: Artifact,
    pub chatboard: Artifact,
    pub task_profile: Option<Artifact>,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub resource: String,
    pub claim_id: String,
    pub ttl_seconds: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub run_dir: PathBuf,
    pub episode: String,
    pub role: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub operation_id: String,
    pub revision: u64,
    pub state_dir: PathBuf,
    pub repository: PathBuf,
    pub cwd: PathBuf,
    pub authority: Artifact,
    pub packet: Artifact,
    pub assignment: PathBuf,
    pub objective_id: String,
    pub root: Identity,
    pub tools: Tools,
    pub model: String,
    pub effort: String,
    pub service_tier: String,
    pub sandbox: String,
    pub require_read_only: bool,
    pub bootstrap_context: bool,
    pub retained: Option<Identity>,
    #[serde(default)]
    pub result_from_turn: bool,
    pub result_path: PathBuf,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    #[serde(default)]
    pub custody: Vec<Artifact>,
    pub claim: Option<Claim>,
    pub profile: Option<Profile>,
    #[serde(default = "default_poll")]
    pub poll_seconds: u64,
    #[serde(default = "default_wait")]
    pub wait_seconds: u64,
}
fn default_poll() -> u64 {
    5
}
fn default_wait() -> u64 {
    300
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Admitting,
    CreateUnknown,
    Prepared,
    StartUnknown,
    Running,
    Terminal,
    Collected,
    Closed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Pending,
    Unknown,
    Done,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessRecord {
    pub argv: Vec<String>,
    pub pid: Option<u32>,
    pub start: Option<String>,
    pub status: Option<i32>,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub stdout_sha256: Option<String>,
    pub stderr_sha256: Option<String>,
    pub timed_out: bool,
    pub leftovers: Vec<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Operation {
    pub request: Request,
    pub input_digest: String,
    pub phase: Phase,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub rollout: Option<PathBuf>,
    pub claim: Effect,
    pub budget_binding: Effect,
    pub profile: Effect,
    pub release: Effect,
    pub objective_close: Effect,
    pub evidence: BTreeMap<String, Value>,
    pub failures: Vec<String>,
    pub processes: Vec<ProcessRecord>,
    pub receipt: Option<String>,
    pub closure_digest: Option<String>,
    pub result_reservation: PathBuf,
    pub started_at: Option<u64>,
    pub context_established: bool,
    pub observer_events: BTreeMap<String, Value>,
    pub budget_cursor: Option<String>,
    pub acknowledged_budget_cursor: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub schema: String,
    pub binary_sha256: String,
    pub assignment: PathBuf,
    pub objective_id: String,
    pub operations: BTreeMap<String, Operation>,
    pub acknowledged: BTreeSet<String>,
    pub acknowledged_events: BTreeSet<String>,
    pub objective_closure_digest: Option<String>,
}
pub struct Host {
    pub dir: PathBuf,
    pub state: State,
    _lock: File,
    pub binary: String,
}

extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, Ordering::SeqCst);
}
pub fn install_signals() {
    // Handler performs only an atomic store; subprocess cleanup runs in normal code.
    unsafe {
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
    }
}
fn proc_stat(pid: u32) -> Option<(String, u32, String)> {
    let s = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let parts: Vec<_> = s.get(s.rfind(')')? + 2..)?.split_whitespace().collect();
    Some((
        parts.get(19)?.to_string(),
        parts.get(2)?.parse().ok()?,
        parts.first()?.to_string(),
    ))
}
fn group_members(pid: u32) -> Vec<u32> {
    fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let n = e.ok()?.file_name().to_str()?.parse::<u32>().ok()?;
            let (_, group, state) = proc_stat(n)?;
            (group == pid && state != "Z").then_some(n)
        })
        .collect()
}
fn kill_group(pid: u32) {
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
}
impl Host {
    pub fn open(dir: &Path, request: Option<&Request>) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join("lock"))?;
        lock.try_lock()
            .map_err(|_| Error::Refused("executor busy; no second owner admitted".into()))?;
        let binary = digest(&fs::read(std::env::current_exe()?)?);
        let state = if dir.join("state.json").exists() {
            let raw = read_json(&dir.join("state.json"))?;
            if raw["schema"] != SCHEMA || raw["binary_sha256"] != binary {
                return refuse(
                    "incompatible state/binary; inspect raw state and restore the pinned executor",
                );
            }
            serde_json::from_value::<State>(raw)?
        } else {
            let r = request.ok_or_else(|| Error::Refused("state missing".into()))?;
            State {
                schema: SCHEMA.into(),
                binary_sha256: binary.clone(),
                assignment: r.assignment.clone(),
                objective_id: r.objective_id.clone(),
                operations: BTreeMap::new(),
                acknowledged: BTreeSet::new(),
                acknowledged_events: BTreeSet::new(),
                objective_closure_digest: None,
            }
        };
        let h = Self {
            dir: dir.into(),
            state,
            _lock: lock,
            binary,
        };
        h.save()?;
        Ok(h)
    }
    pub fn save(&self) -> Result<()> {
        atomic(&self.dir.join("state.json"), &self.state)
    }
    fn op(&self, id: &str) -> &Operation {
        &self.state.operations[id]
    }
    fn op_mut(&mut self, id: &str) -> &mut Operation {
        self.state
            .operations
            .get_mut(id)
            .expect("validated operation")
    }
    pub fn acknowledge(&mut self, receipt: &str) -> Result<()> {
        if receipt.len() != 64 || !receipt.chars().all(|c| c.is_ascii_hexdigit()) {
            return refuse("invalid receipt identity");
        }
        let path = self.dir.join("receipts").join(format!("{receipt}.json"));
        let raw = bytes(&path)?;
        if digest(&raw) != receipt {
            return refuse("receipt identity conflict");
        }
        let content: Value = serde_json::from_slice(&raw)?;
        if let Some(events) = content["observer_events"].as_object() {
            self.state
                .acknowledged_events
                .extend(events.keys().cloned());
        }
        if let Some(id) = content["operation_id"].as_str()
            && self.state.operations.contains_key(id)
            && let Some(cursor) = content["budget_cursor"].as_str()
        {
            self.op_mut(id).acknowledged_budget_cursor = Some(cursor.into());
        }
        self.state.acknowledged.insert(receipt.into());
        self.save()
    }
    fn verify_bundle(tools: &Tools) -> Result<()> {
        let manifest: BTreeMap<String, String> =
            serde_json::from_slice(&tools.bundle_manifest.verify()?)?;
        let root = tools
            .bundle_manifest
            .path
            .parent()
            .ok_or_else(|| Error::Refused("bundle parent missing".into()))?;
        for (path, sha) in &manifest {
            let relative = Path::new(path);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|p| !matches!(p, std::path::Component::Normal(_)))
            {
                return refuse("invalid bundle member path");
            }
            if digest(&fs::read(root.join(relative))?) != *sha {
                return refuse(format!("bundle dependency conflict: {path}"));
            }
        }
        for tool in [
            &tools.role_host,
            &tools.scope_budget,
            &tools.usage_ledger,
            &tools.child_context,
            &tools.chatboard,
        ] {
            let relative = tool
                .path
                .strip_prefix(root)
                .map_err(|_| Error::Refused("tool outside pinned bundle".into()))?;
            if manifest.get(&relative.to_string_lossy().to_string()) != Some(&tool.sha256) {
                return refuse("tool missing from bundle manifest");
            }
        }
        Ok(())
    }
    fn verify_tools(r: &Request) -> Result<()> {
        Self::verify_bundle(&r.tools)?;
        for a in [
            &r.tools.role_host,
            &r.tools.scope_budget,
            &r.tools.usage_ledger,
            &r.tools.child_context,
            &r.tools.context_binary,
            &r.tools.chatboard,
        ] {
            a.verify()?;
        }
        if let Some(a) = &r.tools.task_profile {
            a.verify()?;
        }
        Ok(())
    }
    pub fn dispatch(&mut self, r: Request) -> Result<String> {
        let id = r.operation_id.clone();
        if r.schema != SCHEMA
            || id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            || r.poll_seconds == 0
            || r.poll_seconds > 60
            || r.wait_seconds == 0
            || r.wait_seconds > 36000
        {
            return refuse("invalid request identity/schema/poll policy");
        }
        for p in [
            &r.state_dir,
            &r.repository,
            &r.cwd,
            &r.assignment,
            &r.root.rollout,
            &r.result_path,
            &r.tools.python,
        ] {
            if !p.is_absolute() {
                return refuse("all paths must be absolute");
            }
        }
        if r.state_dir != self.dir
            || r.assignment != self.state.assignment
            || r.objective_id != self.state.objective_id
        {
            return refuse("objective/state binding conflict");
        }
        if let Some(c) = &r.claim
            && (c.resource != format!("manual-host:{}:{}", r.objective_id, id)
                || c.ttl_seconds == 0)
        {
            return refuse("claim must use this host operation namespace and positive lifetime");
        }
        let input_digest = digest(&serde_json::to_vec(&r)?);
        if let Some(old) = self.state.operations.get(&id) {
            if old.input_digest != input_digest {
                return refuse("same operation identity with conflicting input");
            }
            return Ok(id);
        }
        let result_reservation = fs::canonicalize(
            r.result_path
                .parent()
                .ok_or_else(|| Error::Refused("result parent missing".into()))?,
        )?
        .join(
            r.result_path
                .file_name()
                .ok_or_else(|| Error::Refused("result filename missing".into()))?,
        );
        if self
            .state
            .operations
            .values()
            .any(|op| op.result_reservation == result_reservation)
        {
            return refuse("result path reserved by another operation");
        }
        r.authority.verify()?;
        r.packet.verify()?;
        Self::verify_tools(&r)?;
        if r.result_path.exists() {
            return refuse(
                "result already exists before dispatch; choose a fresh assignment output",
            );
        }
        let sb = read_json(&r.assignment)?;
        if sb["scope_budget"]["objective_id"] != r.objective_id
            || !sb["scope_budget"]["closed_at"].is_null()
        {
            return refuse("objective missing, mismatched or closed");
        }
        if !sb["scope_budget"]["sessions"].as_array().is_some_and(|a| {
            a.iter()
                .any(|x| x["session_id"] == r.root.session_id && x["provider"] == "codex")
        }) {
            return refuse("verified root is not enrolled in this objective");
        }
        // Independently observe the root's rollout identity, never an env var.
        let first = fs::read_to_string(&r.root.rollout)?
            .lines()
            .next()
            .unwrap_or("")
            .to_string();
        let meta: Value = serde_json::from_str(&first)?;
        if meta["type"] != "session_meta" || meta["payload"]["id"] != r.root.session_id {
            return refuse("root identity mismatch");
        }
        if let Some(retained) = &r.retained {
            let prior = self
                .state
                .operations
                .values()
                .filter(|o| o.session_id.as_deref() == Some(retained.session_id.as_str()))
                .collect::<Vec<_>>();
            if prior.is_empty()
                || prior
                    .iter()
                    .any(|o| !matches!(o.phase, Phase::Collected | Phase::Closed))
                || prior
                    .iter()
                    .all(|o| o.rollout.as_ref() != Some(&retained.rollout))
            {
                return refuse(
                    "retained session must be an owned, collected, verified thread in this objective",
                );
            }
            if r.bootstrap_context {
                return refuse("bootstrap allowance is fresh-thread only");
            }
        }
        self.state.operations.insert(
            id.clone(),
            Operation {
                request: r.clone(),
                input_digest,
                phase: Phase::Admitting,
                session_id: None,
                turn_id: None,
                rollout: None,
                claim: Effect::Pending,
                budget_binding: Effect::Pending,
                profile: Effect::Pending,
                release: Effect::Pending,
                objective_close: Effect::Pending,
                evidence: BTreeMap::new(),
                failures: vec![],
                processes: vec![],
                receipt: None,
                closure_digest: None,
                result_reservation,
                started_at: None,
                context_established: false,
                observer_events: BTreeMap::new(),
                budget_cursor: None,
                acknowledged_budget_cursor: None,
            },
        );
        self.save()?;
        self.observe(&id, false)?;
        if !self.headroom(&id) {
            return self
                .failure(&id, "pre-admission observation/headroom unavailable")
                .map(|_| id);
        }
        if let Some(c) = &r.claim {
            self.op_mut(&id).claim = Effect::Unknown;
            self.save()?;
            let args = vec![
                "--repository".into(),
                r.repository.display().to_string(),
                "claim".into(),
                "--resource".into(),
                c.resource.clone(),
                "--claim-id".into(),
                c.claim_id.clone(),
                "--author".into(),
                "manual-host".into(),
                "--session-id".into(),
                r.root.session_id.clone(),
                "--ttl-seconds".into(),
                c.ttl_seconds.to_string(),
                "--note".into(),
                format!("manual-host operation {id}; collection custody only"),
            ];
            match self.python(&id, "claim", &r.tools.chatboard, args) {
                Ok(v) => {
                    self.evidence(&id, "claim", v)?;
                    self.op_mut(&id).claim = Effect::Done;
                    self.save()?;
                }
                Err(e) => {
                    self.failure(&id, &format!("claim_unknown: {e}"))?;
                    return Ok(id);
                }
            }
        }
        if let Some(retained) = &r.retained {
            self.op_mut(&id).session_id = Some(retained.session_id.clone());
            self.op_mut(&id).rollout = Some(retained.rollout.clone());
            self.op_mut(&id).phase = Phase::Prepared;
            self.save()?;
            if r.require_read_only {
                self.failure(
                    &id,
                    "retained read-only realization unestablished; no turn started",
                )?;
                return Ok(id);
            }
            self.admit_prepared(&id)?;
            return Ok(id);
        }
        self.op_mut(&id).phase = Phase::CreateUnknown;
        self.save()?;
        let mut args = vec![
            "--state".into(),
            self.dir.join("role-host").display().to_string(),
            "prepare".into(),
            "codex".into(),
            "--cwd".into(),
            r.cwd.display().to_string(),
            "--name".into(),
            format!("manual-host-{id}"),
            "--parent".into(),
            r.root.session_id.clone(),
        ];
        for a in [
            "-m".to_string(),
            r.model.clone(),
            "-s".into(),
            r.sandbox.clone(),
            "-c".into(),
            format!("model_reasoning_effort={:?}", r.effort),
            "-c".into(),
            format!("service_tier={:?}", r.service_tier),
        ] {
            args.push(format!("--host-arg={a}"));
        }
        let prepared = match self.python(&id, "prepare", &r.tools.role_host, args) {
            Ok(v) => v,
            Err(e) => {
                self.failure(&id, &format!("create_unknown: {e}"))?;
                return Ok(id);
            }
        };
        let sid = prepared["session_id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if sid.is_none() {
            self.failure(&id, "create_unknown: response identity absent")?;
            return Ok(id);
        }
        self.op_mut(&id).session_id = sid;
        self.op_mut(&id).phase = Phase::Prepared;
        self.evidence(&id, "prepared", prepared.clone())?;
        // Only actual returned realization evidence can establish the grant.
        if r.require_read_only && prepared["realized"]["sandbox"]["type"] != "readOnly" {
            self.failure(
                &id,
                "required read-only realization unestablished; no turn started",
            )?;
            return Ok(id);
        }
        self.admit_prepared(&id)?;
        Ok(id)
    }
    fn failure(&mut self, id: &str, s: &str) -> Result<()> {
        self.op_mut(id).failures.push(s.into());
        self.save()
    }
    fn observer_event(&mut self, id: &str, event: Value) -> Result<()> {
        let event_id = digest(&serde_json::to_vec(&event)?);
        self.op_mut(id).observer_events.insert(event_id, event);
        self.save()
    }
    fn evidence(&mut self, id: &str, key: &str, v: Value) -> Result<()> {
        if matches!(key, "root_context" | "child_context")
            && let Some(prior) = self.op(id).evidence.get(key)
        {
            let signature = |x: &Value| json!([x["status"], x["recommendation"], x["epoch"]]);
            let initial_healthy = key == "child_context"
                && self.op(id).request.bootstrap_context
                && !self.op(id).context_established
                && v["status"] == "observed"
                && v["recommendation"] == "retain_ok";
            if signature(prior) != signature(&v) && !initial_healthy {
                self.observer_event(
                    id,
                    json!({"kind":"context_state_changed","source":key,"observation":v}),
                )?;
            }
        }
        if key == "budget" {
            if v["schema"] == "objective-budget-control-v1"
                && v["topic"] == format!("control:budget:{}", self.state.objective_id)
                && let Some(cursor) = v["next_budget_cursor"].as_str()
            {
                self.op_mut(id).budget_cursor = Some(cursor.into());
            }
            if let Some(events) = v["events"].as_array() {
                for e in events {
                    self.observer_event(id, e.clone())?;
                }
            }
            if v["status"] == "unavailable" {
                self.observer_event(
                    id,
                    json!({"kind":"budget_reader_unavailable","observation":v}),
                )?;
            }
        }
        self.op_mut(id).evidence.insert(key.into(), v);
        self.save()
    }
    fn python(
        &mut self,
        id: &str,
        label: &str,
        script: &Artifact,
        args: Vec<String>,
    ) -> Result<Value> {
        script.verify()?;
        Self::verify_bundle(&self.op(id).request.tools)?;
        let mut argv = vec![
            self.op(id).request.tools.python.display().to_string(),
            "-E".into(),
            "-s".into(),
            script.path.display().to_string(),
        ];
        argv.extend(args);
        self.run(id, label, argv, Duration::from_secs(125))
    }
    pub fn run(
        &mut self,
        id: &str,
        label: &str,
        argv: Vec<String>,
        timeout: Duration,
    ) -> Result<Value> {
        let n = self.op(id).processes.len();
        let logs = self.dir.join("processes");
        fs::create_dir_all(&logs)?;
        let stdout = logs.join(format!("{id}-{n}-{label}.stdout"));
        let stderr = logs.join(format!("{id}-{n}-{label}.stderr"));
        self.op_mut(id).processes.push(ProcessRecord {
            argv: argv.clone(),
            pid: None,
            start: None,
            status: None,
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            stdout_sha256: None,
            stderr_sha256: None,
            timed_out: false,
            leftovers: vec![],
        });
        self.save()?;
        let mut command = Command::new(&argv[0]);
        command.current_dir(&self.op(id).request.repository);
        command
            .args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(File::create(&stdout)?)
            .stderr(File::create(&stderr)?)
            .process_group(0);
        let parent = std::process::id();
        // On executor death the immediate child cannot continue a pending send.
        // Durable attempt state still reports unknown remote effects.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() != parent as i32 {
                    libc::raise(libc::SIGKILL);
                }
                Ok(())
            });
        }
        let mut child = command.spawn()?;
        let pid = child.id();
        let start = proc_stat(pid).map(|s| s.0);
        {
            let p = &mut self.op_mut(id).processes[n];
            p.pid = Some(pid);
            p.start = start;
        }
        self.save()?;
        let began = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait()? {
                break s;
            }
            let oversized = fs::metadata(&stdout)
                .map(|m| m.len() > MAX_BYTES)
                .unwrap_or(false)
                || fs::metadata(&stderr)
                    .map(|m| m.len() > MAX_BYTES)
                    .unwrap_or(false);
            if began.elapsed() >= timeout || STOP.load(Ordering::SeqCst) || oversized {
                self.op_mut(id).processes[n].timed_out = true;
                kill_group(pid);
                break child.wait()?;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        if !group_members(pid).is_empty() {
            kill_group(pid);
            std::thread::sleep(Duration::from_millis(50));
        }
        let leftovers = group_members(pid);
        {
            let p = &mut self.op_mut(id).processes[n];
            p.status = Some(status.code().unwrap_or(-1));
            p.leftovers = leftovers;
            p.stdout_sha256 = Some(digest(&fs::read(&stdout)?));
            p.stderr_sha256 = Some(digest(&fs::read(&stderr)?));
        }
        self.save()?;
        if !(status.success() || (label == "scope-check" && matches!(status.code(), Some(3 | 4))))
            || self.op(id).processes[n].timed_out
            || !self.op(id).processes[n].leftovers.is_empty()
        {
            return refuse(format!("{label} unavailable; process evidence {n}"));
        }
        read_json(&stdout)
    }
    fn admit_prepared(&mut self, id: &str) -> Result<()> {
        let r = self.op(id).request.clone();
        let sid = self.op(id).session_id.clone().unwrap();
        self.op_mut(id).budget_binding = Effect::Unknown;
        self.save()?;
        let sb = read_json(&r.assignment)?;
        let bound = sb["scope_budget"]["bindings"].as_array().is_some_and(|a| {
            a.iter().any(|x| {
                x["session_id"] == sid
                    && x["parent_session_id"] == r.root.session_id
                    && x["provider"] == "codex"
            })
        });
        if !bound {
            let args = vec![
                "bind".into(),
                "--record".into(),
                r.assignment.display().to_string(),
                "--session".into(),
                sid.clone(),
                "--provider".into(),
                "codex".into(),
                "--parent".into(),
                r.root.session_id.clone(),
                "--bound-by".into(),
                "supervisor".into(),
                "--reason".into(),
                format!("manual-host operation {id}"),
            ];
            if let Err(e) = self.python(id, "budget-bind", &r.tools.scope_budget, args) {
                self.failure(id, &format!("budget_binding_unknown: {e}"))?;
                return Ok(());
            }
        }
        self.op_mut(id).budget_binding = Effect::Done;
        self.save()?;
        if let Some(p) = &r.profile {
            let Some(tool) = &r.tools.task_profile else {
                self.failure(id, "profile tool missing")?;
                return Ok(());
            };
            self.op_mut(id).profile = Effect::Unknown;
            self.save()?;
            let args = vec![
                "assign".into(),
                "--run-dir".into(),
                p.run_dir.display().to_string(),
                "--episode".into(),
                p.episode.clone(),
                "--role".into(),
                p.role.clone(),
                "--requested-model".into(),
                r.model.clone(),
                "--requested-effort".into(),
                r.effort.clone(),
                "--harness".into(),
                format!("codex/manual-host;tier={}", r.service_tier),
                "--run".into(),
                id.into(),
                "--dispatched-packet".into(),
                r.packet.path.display().to_string(),
                "--child-session".into(),
                sid.clone(),
            ];
            match self.python(id, "profile", tool, args) {
                Ok(v) => {
                    self.evidence(id, "profile", v)?;
                    self.op_mut(id).profile = Effect::Done;
                    self.save()?;
                }
                Err(e) => {
                    self.failure(id, &format!("profile_unknown: {e}"))?;
                    return Ok(());
                }
            }
        }
        self.observe(id, true)?;
        if !self.headroom(id) {
            self.failure(id, "admission headroom lost")?;
            return Ok(());
        }
        let available = self
            .op(id)
            .evidence
            .get("child_context")
            .is_some_and(|v| v["status"] == "observed" && v["recommendation"] == "retain_ok");
        if r.retained.is_some() {
            let tier = if r.service_tier == "fast" {
                "priority"
            } else {
                &r.service_tier
            };
            if !self.op(id).evidence.get("settings").is_some_and(|v| {
                v["model"] == r.model
                    && v["reasoning_effort"] == r.effort
                    && v["service_tier"] == tier
            }) {
                self.failure(
                    id,
                    "retained settings mismatch/unavailable; no turn started",
                )?;
                return Ok(());
            }
        }
        if !available && !r.bootstrap_context {
            self.failure(id, "initial context unavailable; bootstrap not authorized")?;
            return Ok(());
        }
        let packet = String::from_utf8(r.packet.verify()?)
            .map_err(|_| Error::Refused("packet not UTF8".into()))?;
        let prompt = self.dir.join(format!("{id}-prompt.txt"));
        let binding = format!(
            "\n\nHost identity binding (data): session {sid}; parent/collector {}; manual operation {id}; result path {}; continuation and artifact authority remain in the assignment packet.\n",
            r.root.session_id,
            r.result_path.display()
        );
        let controls = format!(
            "Pinned context reader: {} SHA-256 {}. Native rollout may not exist before the first turn; the host records the verified rollout/identity in {} when available. No inherited session variable establishes identity.\n",
            r.tools.context_binary.path.display(),
            r.tools.context_binary.sha256,
            self.dir.join("state.json").display()
        );
        fs::write(&prompt, format!("{packet}{binding}{controls}"))?;
        File::open(&prompt)?.sync_all()?;
        self.evidence(
            id,
            "prompt",
            json!({"path":prompt,"sha256":digest(&bytes(&prompt)?)}),
        )?;
        self.op_mut(id).phase = Phase::StartUnknown;
        self.save()?;
        let args = if r.retained.is_some() {
            vec![
                "--state".into(),
                self.dir.join("role-host").display().to_string(),
                "deliver".into(),
                "codex".into(),
                sid,
                "--cwd".into(),
                r.cwd.display().to_string(),
                "--message-file".into(),
                prompt.display().to_string(),
                format!("--host-arg=-c"),
                format!("--host-arg=model_reasoning_effort={:?}", r.effort),
            ]
        } else {
            vec![
                "--state".into(),
                self.dir.join("role-host").display().to_string(),
                "start-prepared".into(),
                "codex".into(),
                sid,
                "--prompt-file".into(),
                prompt.display().to_string(),
                "--effort".into(),
                r.effort,
            ]
        };
        match self.python(id, "start", &r.tools.role_host, args) {
            Ok(v) => {
                if let Some(t) = v["turn_id"].as_str().filter(|t| !t.is_empty()) {
                    self.op_mut(id).turn_id = Some(t.into());
                    self.op_mut(id).phase = Phase::Running;
                    self.op_mut(id).started_at = Some(now());
                    self.evidence(id, "start", v)?;
                } else {
                    self.failure(id, "start_unknown: no exact turn id")?;
                }
            }
            Err(e) => self.failure(id, &format!("start_unknown: {e}"))?,
        }
        Ok(())
    }
    fn context(&mut self, id: &str, identity: &Identity, label: &str) -> Result<Value> {
        let r = self.op(id).request.clone();
        r.tools.context_binary.verify()?;
        let args = vec![
            "--binary".into(),
            r.tools.context_binary.path.display().to_string(),
            "--sha256".into(),
            r.tools.context_binary.sha256.clone(),
            "check".into(),
            "--session".into(),
            identity.session_id.clone(),
            "--rollout".into(),
            identity.rollout.display().to_string(),
        ];
        self.python(id, label, &r.tools.child_context, args)
    }
    fn observe(&mut self, id: &str, child: bool) -> Result<()> {
        let r = self.op(id).request.clone();
        let root = self
            .context(id, &r.root, "root-context")
            .unwrap_or_else(|e| json!({"status":"unavailable","detail":e.to_string()}));
        self.evidence(id, "root_context", root)?;
        let mut args = vec![
            "control".into(),
            "--record".into(),
            r.assignment.display().to_string(),
            "--role-host-ledger".into(),
            self.dir
                .join("role-host/ledger.jsonl")
                .display()
                .to_string(),
        ];
        if let Some(cursor) = &self.op(id).budget_cursor {
            args.extend(["--cursor".into(), cursor.clone()]);
        }
        let budget = self
            .python(id, "budget", &r.tools.usage_ledger, args)
            .unwrap_or_else(|e| json!({"status":"unavailable","detail":e.to_string()}));
        self.evidence(id, "budget", budget)?;
        let args = vec![
            "check".into(),
            "--record".into(),
            r.assignment.display().to_string(),
            "--rollout".into(),
            r.root.rollout.display().to_string(),
        ];
        let policy = self
            .python(id, "scope-check", &r.tools.scope_budget, args)
            .unwrap_or_else(|e| json!({"status":"unavailable","detail":e.to_string()}));
        self.evidence(id, "budget_policy", policy)?;
        if child && self.op(id).session_id.is_some() {
            let sid = self.op(id).session_id.clone().unwrap();
            if self.op(id).rollout.is_none() {
                let args = vec!["identity".into(), "codex".into(), sid.clone()];
                if let Ok(v) = self.python(id, "identity", &r.tools.role_host, args)
                    && v["session_id"] == sid
                    && let Some(p) = v["rollout"].as_str()
                {
                    self.op_mut(id).rollout = Some(PathBuf::from(p));
                    self.evidence(id, "identity", v)?;
                }
            }
            let context = if let Some(rollout) = self.op(id).rollout.clone() {
                self.context(
                    id,
                    &Identity {
                        session_id: sid.clone(),
                        rollout: rollout.clone(),
                    },
                    "child-context",
                )
                .unwrap_or_else(|e| json!({"status":"unavailable","detail":e.to_string()}))
            } else {
                json!({"status":"unavailable","reason":"no verified child rollout"})
            };
            let established = context["status"] == "observed";
            self.evidence(id, "child_context", context)?;
            if established {
                self.op_mut(id).context_established = true;
                self.save()?;
            }
            if let Some(rollout) = self.op(id).rollout.clone() {
                let mut args = vec![
                    "settings".into(),
                    "codex".into(),
                    sid,
                    "--rollout".into(),
                    rollout.display().to_string(),
                ];
                if let Some(turn) = &self.op(id).turn_id {
                    args.extend(["--turn-id".into(), turn.clone()]);
                }
                let settings = self
                    .python(id, "settings", &r.tools.role_host, args)
                    .unwrap_or_else(|e| json!({"status":"unavailable","detail":e.to_string()}));
                self.evidence(id, "settings", settings)?;
            }
        }
        Ok(())
    }
    fn headroom(&self, id: &str) -> bool {
        let e = &self.op(id).evidence;
        let root = e.get("root_context");
        let budget = e.get("budget");
        let context_ok =
            root.is_some_and(|v| v["status"] == "observed" && v["recommendation"] == "retain_ok");
        let budget_ok = budget.is_some_and(|v| {
            v["schema"] == "objective-budget-control-v1"
                && v["topic"] == format!("control:budget:{}", self.state.objective_id)
                && v["snapshot"]["status"] == "complete"
                && v["closed_at"].is_null()
        });
        let reached = budget
            .and_then(|v| v["events"].as_array())
            .is_some_and(|a| {
                a.iter().any(|v| {
                    matches!(
                        v["kind"].as_str(),
                        Some("limit_reached" | "hours_reached" | "account_ceiling")
                    )
                })
            });
        // Recheck actual controls, not only edge-triggered notifications: a
        // replayed cursor is not fresh budget headroom.
        let controls_ok = budget
            .and_then(|v| v["snapshot"]["cost_controls"].as_array())
            .is_some_and(|a| {
                a.iter().any(|c| c["max_usd"].as_f64().is_some())
                    && a.iter().all(|c| {
                        c["max_usd"].is_null()
                            || c["remaining_usd"].as_f64().is_some_and(|n| n > 0.0)
                    })
            });
        let policy = e.get("budget_policy").is_some_and(|v| {
            v["stop"] == false && v["unknown"].as_array().is_some_and(Vec::is_empty)
        });
        context_ok && budget_ok && !reached && controls_ok && policy
    }
    fn verify_processes(&mut self, id: &str) -> Result<()> {
        let records = self.op(id).processes.clone();
        for (n, p) in records
            .iter()
            .enumerate()
            .filter(|(_, p)| p.status.is_none() || !p.leftovers.is_empty())
        {
            let Some(pid) = p.pid else {
                return refuse("process launch outcome unknown; no automatic replacement");
            };
            if let Some((start, _, _)) = proc_stat(pid) {
                if p.start.as_deref() != Some(&start) {
                    return refuse("process PID reused; custody unresolved");
                }
                kill_group(pid);
                std::thread::sleep(Duration::from_millis(50));
            }
            let leftovers = group_members(pid);
            if !leftovers.is_empty() {
                self.op_mut(id).processes[n].leftovers = leftovers;
                self.save()?;
                return refuse("orphan process group remains; custody unresolved");
            }
            // It is not our child after restart. Absence establishes no running
            // process; the original exit status remains unavailable (-2).
            self.op_mut(id).processes[n].status = Some(-2);
            self.op_mut(id).processes[n].leftovers = vec![];
            self.save()?;
        }
        Ok(())
    }
    pub fn collect(&mut self, id: &str, wait: bool) -> Result<Value> {
        if !self.state.operations.contains_key(id) {
            return refuse("unknown operation");
        }
        self.verify_processes(id)?;
        if matches!(self.op(id).phase, Phase::Closed | Phase::Collected) {
            self.verify_snapshot(id)?;
            return self.receipt(id);
        }
        let begin = Instant::now();
        let r = self.op(id).request.clone();
        loop {
            match self.op(id).phase {
                Phase::Running | Phase::Terminal => {
                    let sid = self.op(id).session_id.clone().unwrap();
                    let turn = self.op(id).turn_id.clone().unwrap();
                    let args = vec![
                        "exact-turn".into(),
                        "codex".into(),
                        sid,
                        "--turn-id".into(),
                        turn,
                    ];
                    match self.python(id, "turn", &r.tools.role_host, args) {
                        Ok(v) => {
                            let terminal = matches!(
                                v["status"].as_str(),
                                Some("completed" | "failed" | "interrupted")
                            );
                            self.evidence(id, "turn", v)?;
                            if terminal {
                                self.op_mut(id).phase = Phase::Terminal;
                                self.save()?;
                            }
                        }
                        Err(e) => self.evidence(
                            id,
                            "turn",
                            json!({"status":"unavailable","detail":e.to_string()}),
                        )?,
                    }
                    self.observe(id, true)?;
                    if matches!(self.op(id).phase, Phase::Terminal) {
                        self.collect_artifacts(id)?;
                        return self.receipt(id);
                    }
                }
                // Unknown create/start is never retried or inferred absent from
                // latest-turn reads. Keep custody for the root's disposition.
                _ => return self.receipt(id),
            }
            let changed = self
                .op(id)
                .observer_events
                .keys()
                .any(|key| !self.state.acknowledged_events.contains(key));
            let context = self.op(id).evidence.get("child_context");
            let bootstrap = r.bootstrap_context
                && !self.op(id).context_established
                && self
                    .op(id)
                    .started_at
                    .is_some_and(|at| now().saturating_sub(at) < 30);
            let pressure = context.is_some_and(|v| {
                (v["status"] != "observed" && !bootstrap)
                    || (v["status"] == "observed" && v["recommendation"] != "retain_ok")
            });
            if !wait
                || changed
                || pressure
                || !self.headroom(id)
                || begin.elapsed().as_secs() >= r.wait_seconds
                || STOP.load(Ordering::SeqCst)
            {
                return self.receipt(id);
            }
            std::thread::sleep(Duration::from_secs(r.poll_seconds));
        }
    }
    fn collect_artifacts(&mut self, id: &str) -> Result<()> {
        let r = self.op(id).request.clone();
        let current = fs::canonicalize(r.result_path.parent().unwrap())?
            .join(r.result_path.file_name().unwrap());
        if current != self.op(id).result_reservation {
            return refuse("result path reservation changed");
        }
        if r.result_from_turn {
            self.capture_native_result(id)?;
        }
        if !r.result_path.exists() {
            self.failure(id, "result_missing: terminal turn has no assignment result")?;
            return Ok(());
        }
        if self.op(id).evidence.contains_key("collection") {
            self.verify_snapshot(id)?;
            return Ok(());
        }
        if !fs::symlink_metadata(&r.result_path)?.is_file() {
            return refuse("result must be a regular file");
        }
        let result = bytes(&r.result_path)?;
        let doc: Value = serde_json::from_slice(&result)?;
        let mut artifacts = r.artifacts.clone();
        let mut custody = r.custody.clone();
        let output_root = fs::canonicalize(
            r.result_path
                .parent()
                .ok_or_else(|| Error::Refused("result parent missing".into()))?,
        )?;
        for (key, target) in [("artifacts", &mut artifacts), ("custody", &mut custody)] {
            if let Some(v) = doc.get(key) {
                let entries: Vec<Artifact> = serde_json::from_value(v.clone())?;
                for a in &entries {
                    if !fs::canonicalize(&a.path)?.starts_with(&output_root) {
                        return refuse("child artifact outside authorized result directory");
                    }
                }
                target.extend(entries);
            }
        }
        let snapshots = self.dir.join("snapshots");
        fs::create_dir_all(&snapshots)?;
        let mut bindings = vec![];
        for a in artifacts.iter().chain(custody.iter()) {
            let content = a.verify()?;
            let pin = snapshots.join(&a.sha256);
            if pin.exists() {
                if digest(&bytes(&pin)?) != a.sha256 {
                    return refuse("snapshot conflict");
                }
            } else {
                let mut f = File::create(&pin)?;
                f.write_all(&content)?;
                f.sync_all()?;
            }
            bindings.push(json!({"path":a.path,"sha256":a.sha256,"snapshot":pin}));
        }
        // Custody records must demonstrate collected processes rather than a
        // caller's hand-written success boolean. Preserve their full source.
        for a in &custody {
            let v: Value = serde_json::from_slice(&a.verify()?)?;
            if v["schema"] != "manual-orchestration.custody.run.v1" {
                return refuse("unsupported custody evidence schema");
            }
            let clean = v["leftover_processes"]
                .as_array()
                .is_some_and(Vec::is_empty)
                || v["leftovers_after_kill"]
                    .as_array()
                    .is_some_and(Vec::is_empty);
            if !clean || v["returncode"].as_i64().is_none() {
                return refuse("custody leftovers/exit unavailable");
            }
        }
        let result_hash = digest(&result);
        let snapshot = snapshots.join(&result_hash);
        let mut f = File::create(&snapshot)?;
        f.write_all(&result)?;
        f.sync_all()?;
        if digest(&bytes(&r.result_path)?) != result_hash {
            return refuse("result mutated during collection");
        }
        self.evidence(id,"collection",json!({"result":{"path":r.result_path,"sha256":result_hash,"snapshot":snapshot},"artifacts":bindings,"semantic_acceptance":"root_owned"}))?;
        self.verify_snapshot(id)?;
        let settings = self.op(id).evidence.get("settings");
        let expected_tier = if r.service_tier == "fast" {
            "priority"
        } else {
            &r.service_tier
        };
        if !settings.is_some_and(|s| {
            s["turn_id"].as_str() == self.op(id).turn_id.as_deref()
                && s["turn_attribution"] == "exact_turn_interval"
                && s["model"] == r.model
                && s["reasoning_effort"] == r.effort
                && s["service_tier"] == expected_tier
        }) {
            self.failure(
                id,
                "observed settings missing/mismatched; configuration dependent acceptance unmet",
            )?;
        }
        self.op_mut(id).phase = Phase::Collected;
        self.save()
    }
    fn verify_snapshot(&self, id: &str) -> Result<()> {
        let path = &self.op(id).request.result_path;
        if fs::canonicalize(path.parent().unwrap())?.join(path.file_name().unwrap())
            != self.op(id).result_reservation
        {
            return refuse("result reservation path changed");
        }
        if self.op(id).request.result_from_turn && self.op(id).evidence.contains_key("collection") {
            self.verify_native_result(id)?;
        }
        if let Some(c) = self.op(id).evidence.get("collection") {
            let mut entries = vec![&c["result"]];
            if let Some(a) = c["artifacts"].as_array() {
                entries.extend(a);
            }
            for e in entries {
                let path = e["path"]
                    .as_str()
                    .ok_or_else(|| Error::Refused("invalid collection".into()))?;
                let sha = e["sha256"].as_str().unwrap_or("");
                if digest(&bytes(Path::new(path))?) != sha
                    || digest(&bytes(Path::new(e["snapshot"].as_str().unwrap_or("")))?) != sha
                {
                    return refuse(format!("collected artifact mutated: {path}"));
                }
            }
        }
        Ok(())
    }
    pub fn receipt(&mut self, id: &str) -> Result<Value> {
        let op = self.op(id);
        let content = json!({"schema":SCHEMA,"operation_id":id,"input_digest":op.input_digest,"phase":op.phase,
            "session_id":op.session_id,"turn_id":op.turn_id,"rollout":op.rollout,"evidence":op.evidence,
            "failures":op.failures,"process_records":op.processes,"observer_events":op.observer_events,"budget_cursor":op.budget_cursor,"state":self.dir.join("state.json"),
            "semantic_acceptance":"root_owned","integration":"root_owned","monitoring":"foreground_observation_scope_ended",
            "unknown_effects":{"create":matches!(op.phase,Phase::CreateUnknown),"start":matches!(op.phase,Phase::StartUnknown),"claim":matches!(op.claim,Effect::Unknown),"budget_binding":matches!(op.budget_binding,Effect::Unknown)}});
        let raw = serde_json::to_vec_pretty(&content)?;
        let receipt = digest(&raw);
        fs::create_dir_all(self.dir.join("receipts"))?;
        let path = self.dir.join("receipts").join(format!("{receipt}.json"));
        if path.exists() {
            if fs::read(&path)? != raw {
                return refuse("receipt conflict");
            }
        } else {
            let mut f = File::create(&path)?;
            f.write_all(&raw)?;
            f.sync_all()?;
            File::open(path.parent().unwrap())?.sync_all()?;
        }
        self.op_mut(id).receipt = Some(receipt.clone());
        self.save()?;
        Ok(
            json!({"receipt_id":receipt,"receipt":path,"delivery":if self.state.acknowledged.contains(&receipt){"acknowledged"}else{"pending"},"summary":{"operation_id":id,"phase":self.op(id).phase,"session_id":self.op(id).session_id,"turn_id":self.op(id).turn_id,"turn_outcome":self.op(id).evidence.get("turn").map(|v|v["status"].clone()),"failures":self.op(id).failures,"acceptance":"root_owned","observations":{"root_context":self.op(id).evidence.get("root_context").map(|v|v["status"].clone()),"child_context":self.op(id).evidence.get("child_context").map(|v|v["status"].clone()),"child_context_established":self.op(id).context_established},"accounting":{"coverage":self.op(id).evidence.get("budget").map(|v|v["snapshot"]["status"].clone()),"totals":self.op(id).evidence.get("budget").map(|v|v["snapshot"]["totals"].clone()),"post_close":self.op(id).evidence.get("budget").map(|v|v["snapshot"]["post_close"].clone())},"result":self.op(id).evidence.get("collection").map(|v|v["result"].clone())}}),
        )
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseRequest {
    pub schema: String,
    pub operation_id: String,
    pub authority: Artifact,
    pub ack_receipt: String,
    pub objective_end: bool,
    pub disposition: String,
    pub reason: String,
}
impl Host {
    pub fn close(&mut self, c: CloseRequest) -> Result<Value> {
        if c.schema != SCHEMA
            || !matches!(c.disposition.as_str(), "accepted" | "abandoned")
            || !self.state.operations.contains_key(&c.operation_id)
        {
            return refuse("invalid closure binding");
        }
        c.authority.verify()?;
        let id = &c.operation_id;
        let close_digest = digest(&serde_json::to_vec(
            &json!({"authority":c.authority,"disposition":c.disposition,"reason":c.reason}),
        )?);
        if matches!(self.op(id).phase, Phase::Closed) && c.objective_end {
            if self.state.objective_closure_digest.is_none()
                && self.op(id).receipt.as_deref() != Some(c.ack_receipt.as_str())
            {
                return refuse("objective transition must acknowledge latest task receipt");
            }
            self.acknowledge(&c.ack_receipt)?;
            self.verify_processes(id)?;
            self.verify_snapshot(id)?;
            let already_done = matches!(self.op(id).objective_close, Effect::Done);
            self.close_objective(id, &c)?;
            if !already_done {
                self.observe(id, self.op(id).session_id.is_some())?;
            }
            return self.receipt(id);
        }
        if let Some(old) = &self.op(id).closure_digest {
            if old != &close_digest {
                return refuse("closure input conflict");
            }
            if matches!(self.op(id).phase, Phase::Closed) {
                self.acknowledge(&c.ack_receipt)?;
                self.verify_snapshot(id)?;
                return self.receipt(id);
            }
        }
        self.acknowledge(&c.ack_receipt)?;
        if self.op(id).receipt.as_deref() != Some(c.ack_receipt.as_str()) {
            return refuse("closure must acknowledge latest operation receipt");
        }
        self.verify_processes(id)?;
        self.verify_snapshot(id)?;
        let abandoned_known = c.disposition == "abandoned"
            && matches!(
                self.op(id).phase,
                Phase::Admitting | Phase::Prepared | Phase::Terminal
            );
        if !matches!(self.op(id).phase, Phase::Collected | Phase::Closed) && !abandoned_known {
            return refuse("closure refused: assignment/effects not collected; custody retained");
        }
        if abandoned_known {
            if matches!(self.op(id).phase, Phase::Prepared) {
                let sid = self.op(id).session_id.clone().unwrap();
                let r = self.op(id).request.clone();
                let v = self.python(
                    id,
                    "retire-prepared",
                    &r.tools.role_host,
                    vec![
                        "--state".into(),
                        self.dir.join("role-host").display().to_string(),
                        "retire".into(),
                        "codex".into(),
                        sid,
                    ],
                )?;
                self.evidence(id, "retire_prepared", v)?;
            }
            self.evidence(id,"abandonment",json!({"result_available":self.op(id).evidence.contains_key("collection"),"authority":c.authority,"semantic_acceptance":"not_established"}))?;
        }
        if matches!(self.op(id).claim, Effect::Unknown)
            || matches!(self.op(id).budget_binding, Effect::Unknown)
            || matches!(self.op(id).profile, Effect::Unknown)
        {
            return refuse("unsettled owned effects");
        }
        let r = self.op(id).request.clone();
        self.op_mut(id).closure_digest = Some(close_digest);
        self.save()?;
        self.evidence(id,"closure_request",serde_json::to_value(json!({"authority":c.authority,"objective_end":c.objective_end,"disposition":c.disposition,"reason":c.reason}))?)?;
        if c.objective_end {
            self.close_objective(id, &c)?;
        }
        if let Some(claim) = &r.claim
            && matches!(self.op(id).claim, Effect::Done)
            && !matches!(self.op(id).release, Effect::Done)
        {
            self.op_mut(id).release = Effect::Unknown;
            self.save()?;
            let args = vec![
                "--repository".into(),
                r.repository.display().to_string(),
                "release".into(),
                "--resource".into(),
                claim.resource.clone(),
                "--claim-id".into(),
                claim.claim_id.clone(),
                "--session-id".into(),
                r.root.session_id.clone(),
            ];
            match self.python(id, "release", &r.tools.chatboard, args) {
                Ok(v) => {
                    self.evidence(id, "release", v)?;
                    self.op_mut(id).release = Effect::Done;
                    self.save()?;
                }
                Err(e) => {
                    self.failure(id, &format!("release_unknown: {e}"))?;
                    return self.receipt(id);
                }
            }
        }
        self.observe(id, true)?;
        self.op_mut(id).phase = Phase::Closed;
        self.save()?;
        self.receipt(id)
    }
}

/// Generated public DTO schemas; the same version/disposition restrictions are
/// enforced by the executable before effects. No internal permit is serialized.
pub fn public_schemas() -> Value {
    let mut request =
        serde_json::to_value(schemars::schema_for!(Request)).expect("schema serializes");
    let mut close =
        serde_json::to_value(schemars::schema_for!(CloseRequest)).expect("schema serializes");
    request["$id"] = json!("urn:work-engine:manual-host:request:v1");
    close["$id"] = json!("urn:work-engine:manual-host:close:v1");
    request["properties"]["schema"]["const"] = json!(SCHEMA);
    close["properties"]["schema"]["const"] = json!(SCHEMA);
    close["properties"]["disposition"]["enum"] = json!(["accepted", "abandoned"]);
    json!({"schema":SCHEMA,"request":request,"close":close})
}

/// Bind a root-authored draft without requiring root-authored hashing programs.
/// The frozen full request remains the only execution input in durable state.
pub fn bind_draft(mut draft: Value) -> Result<Request> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct DraftTools {
        bundle: PathBuf,
        context_binary: PathBuf,
        python: PathBuf,
    }
    let tools: DraftTools = serde_json::from_value(draft["tools"].clone())?;
    fn reference(path: &Path) -> Result<Artifact> {
        if !path.is_absolute() {
            return refuse("draft paths must be absolute");
        }
        Ok(Artifact {
            path: path.into(),
            sha256: digest(&bytes(path)?),
        })
    }
    for key in ["authority", "packet"] {
        let path = draft[key]
            .as_str()
            .ok_or_else(|| Error::Refused(format!("draft {key} must be an absolute path")))?;
        draft[key] = serde_json::to_value(reference(Path::new(path))?)?;
    }
    for key in ["artifacts", "custody"] {
        if let Some(list) = draft[key].as_array_mut() {
            for value in list {
                if let Some(path) = value.as_str() {
                    *value = serde_json::to_value(reference(Path::new(path))?)?;
                }
            }
        }
    }
    let bundle = &tools.bundle;
    let t = Tools {
        python: tools.python,
        bundle_manifest: reference(&bundle.join("manifest.json"))?,
        role_host: reference(&bundle.join("skills/manual-orchestration/scripts/role_host.py"))?,
        scope_budget: reference(
            &bundle.join("skills/manual-orchestration/scripts/scope_budget.py"),
        )?,
        usage_ledger: reference(&bundle.join("metrics/scripts/usage_ledger.py"))?,
        child_context: reference(
            &bundle.join("skills/manual-orchestration/scripts/child_context.py"),
        )?,
        context_binary: reference(&tools.context_binary)?,
        chatboard: reference(&bundle.join("skills/durable-state/scripts/codex_chatboard.py"))?,
        task_profile: if draft["profile"].is_null() {
            None
        } else {
            Some(reference(
                &bundle.join("metrics/scripts/task_profile_record.py"),
            )?)
        },
    };
    draft["tools"] = serde_json::to_value(t)?;
    let request: Request = serde_json::from_value(draft)?;
    Host::verify_tools(&request)?;
    Ok(request)
}

impl Host {
    fn native_content(&self, id: &str) -> Result<(Value, Vec<u8>)> {
        let op = self.op(id);
        let turn = op
            .evidence
            .get("turn")
            .ok_or_else(|| Error::Refused("native turn evidence unavailable".into()))?;
        if turn["session_id"].as_str() != op.session_id.as_deref()
            || turn["turn_id"].as_str() != op.turn_id.as_deref()
            || turn["turn"]["itemsView"] != "full"
        {
            return refuse("selected native turn/full items unavailable");
        }
        let finals = turn["turn"]["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|item| {
                        item["type"] == "agentMessage" && item["phase"] == "final_answer"
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if finals.len() != 1 {
            return refuse("no unique explicit native final answer");
        }
        let item = finals[0];
        let item_id = item["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Refused("native final item identity unavailable".into()))?;
        let value: Value = serde_json::from_str(
            item["text"]
                .as_str()
                .ok_or_else(|| Error::Refused("native final text unavailable".into()))?,
        )?;
        let mut content = serde_json::to_vec_pretty(&value)?;
        content.push(b'\n');
        let stage = op
            .result_reservation
            .with_file_name(format!(".manual-host-native-{}.json", op.input_digest));
        Ok((
            json!({"operation_id":id,"input_digest":op.input_digest,"session_id":op.session_id,"turn_id":op.turn_id,"item_id":item_id,"native_text_sha256":digest(item["text"].as_str().unwrap().as_bytes()),"capture_encoding":"json_pretty_with_newline","sha256":digest(&content),"result_path":op.result_reservation,"stage":stage,"source":"explicit_final_answer"}),
            content,
        ))
    }
    fn same_native_file(&self, id: &str, intent: &Value) -> Result<()> {
        use std::os::unix::fs::MetadataExt;
        let path = &self.op(id).request.result_path;
        let stage = Path::new(
            intent["stage"]
                .as_str()
                .ok_or_else(|| Error::Refused("native stage missing".into()))?,
        );
        let a = fs::symlink_metadata(stage)?;
        let b = fs::symlink_metadata(path)?;
        if !a.is_file()
            || !b.is_file()
            || a.ino() != b.ino()
            || a.dev() != b.dev()
            || digest(&bytes(path)?) != intent["sha256"].as_str().unwrap_or("")
        {
            return refuse("native output path/content is not the reserved capture publication");
        }
        Ok(())
    }
    fn capture_native_result(&mut self, id: &str) -> Result<()> {
        let (intent, content) = self.native_content(id)?;
        let prior = self.op(id).evidence.get("native_capture_intent").cloned();
        if let Some(old) = &prior {
            if old != &intent {
                return refuse("native source identity/content changed");
            }
        } else if self.op(id).request.result_path.exists() {
            return refuse("foreign result exists before native capture");
        }
        self.evidence(id, "native_capture_intent", intent.clone())?;
        let stage = Path::new(intent["stage"].as_str().unwrap());
        match OpenOptions::new().create_new(true).write(true).open(stage) {
            Ok(mut f) => {
                f.write_all(&content)?;
                f.sync_all()?;
                File::open(stage.parent().unwrap())?.sync_all()?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && prior.is_some() => {
                if bytes(stage)? != content {
                    return refuse("native capture stage conflict/partial write");
                }
            }
            Err(e) => return Err(e.into()),
        }
        let path = self.op(id).request.result_path.clone();
        match fs::hard_link(stage, &path) {
            Ok(()) => File::open(path.parent().unwrap())?.sync_all()?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
        self.same_native_file(id, &intent)?;
        self.evidence(id, "native_result_source", intent)
    }
    fn verify_native_result(&self, id: &str) -> Result<()> {
        let (intent, _) = self.native_content(id)?;
        if self.op(id).evidence.get("native_capture_intent") != Some(&intent)
            || self.op(id).evidence.get("native_result_source") != Some(&intent)
        {
            return refuse("native capture binding unestablished");
        }
        self.same_native_file(id, &intent)
    }
    fn close_objective(&mut self, id: &str, c: &CloseRequest) -> Result<()> {
        let wanted = digest(&serde_json::to_vec(
            &json!({"authority":c.authority,"disposition":c.disposition,"reason":c.reason}),
        )?);
        if let Some(previous) = &self.state.objective_closure_digest
            && previous != &wanted
        {
            return refuse("objective closure input conflict");
        }
        if self
            .state
            .operations
            .iter()
            .any(|(other, o)| other != id && !matches!(o.phase, Phase::Collected | Phase::Closed))
        {
            return refuse("other objective work remains uncollected");
        }
        let r = self.op(id).request.clone();
        let sb = read_json(&r.assignment)?;
        self.state.objective_closure_digest = Some(wanted);
        self.save()?;
        if sb["scope_budget"]["closed_at"].is_null() {
            if matches!(self.op(id).objective_close, Effect::Unknown) {
                return refuse("objective close unknown; owner reconciliation required");
            }
            self.op_mut(id).objective_close = Effect::Unknown;
            self.save()?;
            let args = vec![
                "close".into(),
                "--record".into(),
                r.assignment.display().to_string(),
                "--disposition".into(),
                c.disposition.clone(),
                "--reason".into(),
                c.reason.clone(),
                "--set-by".into(),
                "supervisor".into(),
            ];
            let v = self.python(id, "objective-close", &r.tools.scope_budget, args)?;
            self.evidence(id, "objective_close", v)?;
        } else if !matches!(self.op(id).objective_close, Effect::Done) {
            if sb["scope_budget"]["close"]["disposition"] != c.disposition
                || sb["scope_budget"]["close"]["reason"] != c.reason
            {
                return refuse("existing objective closure differs from requested transition");
            }
            self.evidence(id, "objective_close", sb["scope_budget"]["close"].clone())?;
        }
        self.op_mut(id).objective_close = Effect::Done;
        self.save()
    }
    pub fn reconcile_accounting(&mut self, id: &str) -> Result<Value> {
        if !self.state.operations.contains_key(id) {
            return refuse("unknown operation");
        }
        self.verify_processes(id)?;
        self.observe(id, false)?;
        let op = self.op(id);
        let content = json!({"schema":SCHEMA,"receipt_kind":"accounting_reconciliation","operation_id":id,"input_digest":op.input_digest,"task_receipt":op.receipt,"task_evidence":"historical_not_revalidated","task_effects_replayed":false,"budget":op.evidence.get("budget"),"budget_cursor":op.budget_cursor,"observer_events":op.observer_events,"process_records":op.processes});
        let raw = serde_json::to_vec_pretty(&content)?;
        let receipt = digest(&raw);
        let path = self.dir.join("receipts").join(format!("{receipt}.json"));
        fs::create_dir_all(path.parent().unwrap())?;
        if path.exists() {
            if bytes(&path)? != raw {
                return refuse("accounting receipt conflict");
            }
        } else {
            let mut f = File::create(&path)?;
            f.write_all(&raw)?;
            f.sync_all()?;
            File::open(path.parent().unwrap())?.sync_all()?;
        }
        Ok(
            json!({"receipt_id":receipt,"receipt":path,"receipt_kind":"accounting_reconciliation","delivery":if self.state.acknowledged.contains(&receipt){"acknowledged"}else{"pending"},"accounting":content["budget"],"task_receipt":op.receipt}),
        )
    }
}
