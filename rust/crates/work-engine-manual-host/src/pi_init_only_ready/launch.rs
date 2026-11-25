use super::{
    Refusal, Serving, TaskKey,
    artifact_contract::{ENTRYPOINT, GROUP, PRIVATE_UMASK, USER, reduced_groups},
    full_hex,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use slice_campaign::codec::campaign_digest;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FilePin {
    pub path: String,
    pub sha256: String,
}

impl FilePin {
    fn valid(&self) -> bool {
        self.path.len() <= 255
            && self.path.starts_with('/')
            && self.path[1..].split('/').all(|s| {
                !s.is_empty()
                    && !matches!(s, "." | "..")
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-._+".contains(&b))
            })
            && full_hex(&self.sha256)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ElfClosure {
    Static,
    Dynamic {
        interpreter: FilePin,
        dependencies: Vec<FilePin>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Store {
    DockerImageConfig,
    ContainerdLocalTarget,
}

/// Protected approved values, independent of the eventual readback. This
/// cut supports only an unambiguous linux/amd64 local store target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Startup {
    pub store: Store,
    pub image_id: String,
    pub config_sha256: String,
    pub manifest_sha256: String,
    pub init: FilePin,
    pub elf: ElfClosure,
}

impl Startup {
    fn valid(&self) -> bool {
        let Some(id) = self.image_id.strip_prefix("sha256:") else {
            return false;
        };
        full_hex(id)
            && full_hex(&self.config_sha256)
            && full_hex(&self.manifest_sha256)
            && match self.store {
                Store::DockerImageConfig => id == self.config_sha256,
                Store::ContainerdLocalTarget => id == self.manifest_sha256,
            }
            && self.init.valid()
            && self.init.path == ENTRYPOINT
            && match &self.elf {
                ElfClosure::Static => true,
                ElfClosure::Dynamic {
                    interpreter,
                    dependencies,
                } => {
                    interpreter.valid()
                        && interpreter.path != ENTRYPOINT
                        && !dependencies.is_empty()
                        && dependencies.len() <= 32
                        && dependencies.iter().all(|p| {
                            p.valid() && p.path != interpreter.path && p.path != ENTRYPOINT
                        })
                        && dependencies.windows(2).all(|w| w[0].path < w[1].path)
                }
            }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Recipe {
    pub startup: Startup,
    /// A narrowly selected runtime name; no aliases or fallback.
    pub runtime: String,
    /// Approval of the exact effective security policy identities, not of
    /// what Docker inspect reports. The acquired record below must bind them.
    pub seccomp_sha256: String,
    pub apparmor_profile: String,
    pub apparmor_policy_sha256: String,
}

const USER_GROUP: &str = "65534:65534";
const ARCH: &str = "linux/amd64";

impl Recipe {
    fn valid(&self) -> bool {
        let label = |v: &str| {
            !v.is_empty()
                && v.len() <= 128
                && v.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        };
        self.startup.valid()
            && label(&self.runtime)
            && label(&self.apparmor_profile)
            && self.apparmor_profile != "unconfined"
            && full_hex(&self.seccomp_sha256)
            && full_hex(&self.apparmor_policy_sha256)
    }

    /// The actual reviewed wire body must match exactly. Closed empty image
    /// config and effective immutable materialization are separate checks.
    fn body(&self) -> Value {
        json!({
            "Image": self.startup.image_id,
            "User": USER_GROUP, "Entrypoint": [ENTRYPOINT], "Cmd": [],
            "WorkingDir": "/", "Env": [], "Volumes": {}, "Healthcheck": {"Test":["NONE"]},
            "Tty": false, "OpenStdin": false, "StdinOnce": false,
            "AttachStdin": false, "AttachStdout": true, "AttachStderr": false,
            "HostConfig": {
                "Runtime": self.runtime, "NetworkMode": "none", "CgroupnsMode": "private",
                "ReadonlyRootfs": true, "Privileged": false, "CapDrop":["ALL"], "Init": false,
                "RestartPolicy":{"Name":"no", "MaximumRetryCount":0}, "AutoRemove": false,
                "LogConfig":{"Type":"none", "Config":{}},
                "SecurityOpt":["no-new-privileges", format!("apparmor={}", self.apparmor_profile)]
            }
        })
    }

    pub fn create_body(&self) -> Result<Vec<u8>, Refusal> {
        if !self.valid() {
            return Err(Refusal::Recipe);
        }
        serde_json::to_vec(&self.body()).map_err(|_| Refusal::Recipe)
    }

    pub fn digest(&self) -> Result<String, Refusal> {
        if !self.valid() {
            return Err(Refusal::Recipe);
        }
        campaign_digest(&json!(["b-init-only-create-v1", ARCH, self.body(), self]))
            .map_err(|_| Refusal::Recipe)
    }

    pub(super) fn verify_create(
        &self,
        trace: &CreateTrace,
        serving: &Serving,
        object: &str,
        request: &str,
    ) -> Result<(), Refusal> {
        if trace.serving != *serving
            || trace.campaign_request != request
            || trace.target
                != format!(
                    "POST /{}/containers/create?platform=linux%2Famd64",
                    serving.api
                )
            || trace.body != self.create_body()?
            || trace.status != 201
            || trace.reply.len() > 4096
            || trace.inspect.len() > 32_768
        {
            return Err(Refusal::Create);
        }
        let reply: CreateReply =
            serde_json::from_slice(&trace.reply).map_err(|_| Refusal::Create)?;
        let read: Inspect = serde_json::from_slice(&trace.inspect).map_err(|_| Refusal::Create)?;
        if reply.id != object
            || reply.warnings.is_some_and(|w| !w.is_empty())
            || read.id != object
            || read.image != self.startup.image_id
            || read.path != ENTRYPOINT
            || !empty(&read.args)
            || !read.mounts.is_empty()
            || read.state.status != "created"
            || read.state.running
            || read.state.pid != 0
            || !read.config.matches(self)
            || !read.host_config.matches(self)
        {
            return Err(Refusal::Create);
        }
        Ok(())
    }
}

/// Captured exact protected request/reply and independent no-start inspect
/// readback of the object, not a synthesized echo of the wanted config.
#[derive(Serialize)]
pub struct CreateTrace {
    pub serving: Serving,
    pub campaign_request: String,
    pub target: String,
    pub body: Vec<u8>,
    pub status: u16,
    pub reply: Vec<u8>,
    pub inspect: Vec<u8>,
}

#[cfg(all(target_os = "linux", feature = "private-init-only-acquisition"))]
pub(super) fn create_reply_id(body: &[u8]) -> Result<String, Refusal> {
    let reply: CreateReply = serde_json::from_slice(body).map_err(|_| Refusal::Create)?;
    if full_hex(&reply.id) {
        Ok(reply.id)
    } else {
        Err(Refusal::Create)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct CreateReply {
    id: String,
    #[serde(default)]
    warnings: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Inspect {
    id: String,
    image: String,
    path: String,
    args: Option<Vec<String>>,
    mounts: Vec<Value>,
    config: Config,
    host_config: HostConfig,
    state: State,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct State {
    status: String,
    running: bool,
    pid: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Config {
    user: String,
    image: String,
    entrypoint: Vec<String>,
    cmd: Option<Vec<String>>,
    #[serde(rename = "WorkingDir")]
    cwd: String,
    env: Option<Vec<String>>,
    volumes: Option<serde_json::Map<String, Value>>,
    healthcheck: Healthcheck,
    tty: bool,
    open_stdin: bool,
    stdin_once: bool,
    attach_stdin: bool,
    attach_stdout: bool,
    attach_stderr: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Healthcheck {
    test: Vec<String>,
}

fn empty<T>(vec: &Option<Vec<T>>) -> bool {
    vec.as_ref().is_none_or(Vec::is_empty)
}

impl Config {
    fn matches(&self, recipe: &Recipe) -> bool {
        self.user == USER_GROUP
            && self.image == recipe.startup.image_id
            && self.entrypoint == [ENTRYPOINT]
            && empty(&self.cmd)
            && self.cwd == "/"
            && empty(&self.env)
            && self.volumes.as_ref().is_none_or(|v| v.is_empty())
            && self.healthcheck.test == ["NONE"]
            && !self.tty
            && !self.open_stdin
            && !self.stdin_once
            && !self.attach_stdin
            && self.attach_stdout
            && !self.attach_stderr
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HostConfig {
    runtime: String,
    network_mode: String,
    cgroupns_mode: String,
    readonly_rootfs: bool,
    privileged: bool,
    init: bool,
    cap_drop: Vec<String>,
    cap_add: Option<Vec<String>>,
    binds: Option<Vec<String>>,
    mounts: Option<Vec<Value>>,
    volumes_from: Option<Vec<String>>,
    devices: Option<Vec<Value>>,
    device_requests: Option<Vec<Value>>,
    auto_remove: bool,
    restart_policy: RestartPolicy,
    log_config: LogConfig,
    security_opt: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RestartPolicy {
    name: String,
    maximum_retry_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LogConfig {
    #[serde(rename = "Type")]
    driver: String,
    config: serde_json::Map<String, Value>,
}

impl HostConfig {
    fn matches(&self, recipe: &Recipe) -> bool {
        self.runtime == recipe.runtime
            && self.network_mode == "none"
            && self.cgroupns_mode == "private"
            && self.readonly_rootfs
            && !self.privileged
            && !self.init
            && !self.auto_remove
            && self.cap_drop == ["ALL"]
            && empty(&self.cap_add)
            && empty(&self.binds)
            && empty(&self.mounts)
            && empty(&self.volumes_from)
            && empty(&self.devices)
            && empty(&self.device_requests)
            && self.restart_policy.name == "no"
            && self.restart_policy.maximum_retry_count == 0
            && self.log_config.driver == "none"
            && self.log_config.config.is_empty()
            && self.security_opt
                == [
                    "no-new-privileges".to_owned(),
                    format!("apparmor={}", recipe.apparmor_profile),
                ]
    }
}

/// Same selected protected runtime task association, resolved image and
/// actually materialized OCI startup. Snapshot and stdout pipe identities
/// come from protected runtime/kernel custody, never from request hashing.
#[derive(Clone, Debug, Serialize)]
pub struct ResolvedLaunch {
    pub task: TaskKey,
    pub runtime: String,
    pub platform: String,
    pub argv: Vec<String>,
    pub cwd: String,
    pub env: Vec<String>,
    pub image: Startup,
    pub root_mount_id: u64,
    pub io_pipe_inode: u64,
    pub startup_backing: StartupBacking,
    pub stdout_route: StdoutRoute,
    /// Resolved OCI ReadonlyPaths for this launch, not the Docker request.
    pub proc_readonly_paths: Vec<String>,
    /// Exact MaskedPaths from the selected OCI launch, not Docker inspect.
    pub masked_paths: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum StartupBacking {
    ImmutableImageAndDependencies,
    OverrideOrUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum StdoutRoute {
    ExclusiveSelectedTaskLivePipe,
    AlternateOrUnknown,
}

impl ResolvedLaunch {
    pub(super) fn matches(&self, recipe: &Recipe) -> bool {
        // Exact prospective effective OCI environment; Docker's default
        // HOSTNAME is the truncated selected container ID. No ambient or
        // loader-directed variables. Refuse variants until separately bound.
        let hostname = format!("HOSTNAME={}", &self.task.container[..12]);
        self.runtime == recipe.runtime
            && self.platform == ARCH
            && self.argv == [ENTRYPOINT]
            && self.cwd == "/"
            && self.env
                == [
                    "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                    hostname.as_str(),
                ]
            && self.image == recipe.startup
            && self.root_mount_id != 0
            && self.io_pipe_inode != 0
            && self.startup_backing == StartupBacking::ImmutableImageAndDependencies
            && self.stdout_route == StdoutRoute::ExclusiveSelectedTaskLivePipe
            && if self.runtime == "runc" {
                self.proc_readonly_paths == PROC_READONLY && self.stock_file_mask_intent()
            } else {
                self.proc_readonly_paths.is_empty() && self.masked_paths.is_empty()
            }
    }

    fn stock_file_mask_intent(&self) -> bool {
        let unique: std::collections::BTreeSet<_> =
            self.masked_paths.iter().map(String::as_str).collect();
        unique.len() == self.masked_paths.len() && FILE_MASKS.iter().all(|p| unique.contains(p))
    }

    /// Complete directory portion of the pinned stock OCI mask family. The
    /// kernel reader must separately bind thermal entries to possible CPUs.
    pub(super) fn stock_directory_masks(&self) -> Option<Vec<(&str, Option<u32>)>> {
        if self.runtime != "runc" || !self.stock_file_mask_intent() {
            return None;
        }
        stock_directory_masks(&self.masked_paths)
    }
}

/// Positive kernel/applied observations for this exact live task. These are
/// independent of Docker inspect and the marker. Profile pins must come from
/// bound applied-profile custody (an OCI request or Seccomp:2 alone is not it).
#[derive(Clone, Debug, Serialize)]
pub struct Effective {
    pub task: TaskKey,
    pub uid: [u32; 4],
    pub gid: [u32; 4],
    pub groups: Vec<u32>,
    pub caps: [u64; 5],
    pub nnp: bool,
    pub umask: u16,
    pub seccomp_filters: u32,
    pub applied_seccomp_sha256: String,
    pub apparmor_enforcing: bool,
    pub applied_apparmor_profile: String,
    pub applied_apparmor_sha256: String,
    pub root_mount_id: u64,
    pub read_only_root: bool,
    pub namespaces: Namespaces,
    /// Complete positive inventory normalized by the protected producer; the
    /// route declines if any extra or ambiguous exposure was observed.
    pub inventory: Option<Inventory>,
    pub fds: Vec<Descriptor>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Namespaces {
    /// `[pid, mount, ipc, network, cgroup, uts]` kernel inode IDs in fixed order.
    pub task: [u64; 6],
    pub host: [u64; 6],
}

#[derive(Clone, Debug, Serialize)]
pub struct Inventory {
    /// Complete, sorted non-root mount inventory. Classifications must be
    /// made by the protected source using mount/backing evidence, not by
    /// trusting a path or an OCI mount request. A partial inventory is None.
    pub mounts: Vec<MountView>,
    pub devices: Vec<(u32, u32)>,
    pub interfaces: Vec<String>,
    pub non_loopback_route_count: u32,
    pub externally_reachable_control_handles: u32,
    pub writable_cgroup_control: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MountView {
    pub path: String,
    pub kind: MountKind,
    pub read_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum MountKind {
    PrivateVirtual,
    Procfs(ProcfsMount),
    RuntimeMasked,
    /// The kernel cut must corroborate the bind to the original task-local null inode.
    RuntimeMaskedFile,
    /// Natural-exit only: selected directory with a verified stock runc mask.
    StockMaskedDirectory,
    DaemonGenerated,
    Other,
}

/// Observed mountinfo coordinates in the selected task's mount namespace.
/// A `Procfs` record requires a kernel `proc` filesystem, not just a pathname.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProcfsMount {
    pub namespace: u64,
    pub id: u64,
    pub parent: u64,
    pub device: (u32, u32),
    pub subroot: String,
    pub nosuid: bool,
    pub nodev: bool,
    pub noexec: bool,
}

// Moby's pinned DefaultLinuxSpec sets exactly these ReadonlyPaths. runc
// self-binds them on procfs; they are not MaskedPaths.
pub(super) const PROC_READONLY: [&str; 5] = [
    "/proc/bus",
    "/proc/fs",
    "/proc/irq",
    "/proc/sys",
    "/proc/sysrq-trigger",
];

// File targets in the selected Moby/runc MaskedPaths, including optional
// kernel entries. A missing row must be corroborated by ENOENT in the task.
// Directory masks, including any conditional CPU masks, are not in this family.
pub(super) const FILE_MASKS: [&str; 7] = [
    "/proc/interrupts",
    "/proc/kcore",
    "/proc/keys",
    "/proc/latency_stats",
    "/proc/sched_debug",
    "/proc/timer_list",
    "/proc/timer_stats",
];

pub(super) const STOCK_VIRTUAL: [&str; 7] = [
    "/proc", "/dev", "/dev/pts", "/dev/shm", "/dev/mqueue", "/sys", "/sys/fs/cgroup",
];
pub(super) const GENERATED_FILES: [&str; 3] = ["/etc/hosts", "/etc/hostname", "/etc/resolv.conf"];
pub(super) const STOCK_STATIC_DIRS: [&str; 5] = [
    "/proc/acpi", "/proc/asound", "/proc/scsi", "/sys/devices/virtual/powercap", "/sys/firmware",
];

fn thermal_cpu(path: &str) -> Option<u32> {
    let n = path.strip_prefix("/sys/devices/system/cpu/cpu")?.strip_suffix("/thermal_throttle")?;
    let cpu: u32 = n.parse().ok()?;
    (n == cpu.to_string()).then_some(cpu)
}

pub(super) fn stock_directory_masks(masked: &[String]) -> Option<Vec<(&str, Option<u32>)>> {
    let unique: std::collections::BTreeSet<_> = masked.iter().map(String::as_str).collect();
    if unique.len() != masked.len() || !FILE_MASKS.iter().chain(&STOCK_STATIC_DIRS).all(|p| unique.contains(p)) {
        return None;
    }
    unique.into_iter().filter(|p| !FILE_MASKS.contains(p)).map(|p| {
        if STOCK_STATIC_DIRS.contains(&p) { Some((p, None)) } else { Some((p, Some(thermal_cpu(p)?))) }
    }).collect()
}

impl Inventory {
    pub(super) fn closed(&self, launch: &ResolvedLaunch, mount_namespace: u64, natural: bool) -> bool {
        const MASKED: &[&str] = &[
            "/proc/acpi",
            "/proc/asound",
            "/proc/interrupts",
            "/proc/kcore",
            "/proc/keys",
            "/proc/latency_stats",
            "/proc/sched_debug",
            "/proc/scsi",
            "/proc/timer_list",
            "/proc/timer_stats",
            "/sys/firmware",
            "/sys/devices/virtual/powercap",
        ];
        let proc = self.mounts.iter().find_map(|m| match &m.kind {
            MountKind::Procfs(p) if m.path == "/proc" => Some(p),
            _ => None,
        });
        let stock = launch.runtime == "runc";
        let dirs = if natural {
            let Some(dirs) = launch.stock_directory_masks() else { return false };
            dirs
        } else { Vec::new() };
        let limit = if natural {
            STOCK_VIRTUAL.len() + PROC_READONLY.len() + GENERATED_FILES.len() + FILE_MASKS.len() + dirs.len()
        } else { 32 };
        self.mounts.len() <= limit
            && self.mounts.windows(2).all(|w| w[0].path < w[1].path)
            && (!natural || (STOCK_VIRTUAL.iter().chain(&GENERATED_FILES).all(|p| {
                self.mounts.iter().any(|m| m.path == *p)
            }) && dirs.iter().filter(|(_, cpu)| cpu.is_some()).all(|(p, _)| {
                self.mounts.iter().any(|m| m.path == *p)
            })))
            && ["/proc", "/dev", "/sys", "/sys/fs/cgroup"].iter().all(|required| {
                self.mounts.iter().any(|m| m.path == *required && match &m.kind {
                    MountKind::Procfs(_) => stock && *required == "/proc",
                    MountKind::PrivateVirtual => !stock || *required != "/proc",
                    _ => false,
                })
            })
            && (!stock || (launch.proc_readonly_paths == PROC_READONLY
                && PROC_READONLY.iter().all(|p| self.mounts.iter().any(|m| m.path == *p))))
            && self.mounts.iter().all(|m| match &m.kind {
                MountKind::PrivateVirtual => STOCK_VIRTUAL.contains(&m.path.as_str())
                    && (!m.path.starts_with("/sys") || m.read_only),
                MountKind::Procfs(p) => stock && p.namespace == mount_namespace
                    && p.id != 0 && p.id != launch.root_mount_id && p.device != (0, 0)
                    && p.nosuid && p.nodev && p.noexec
                    && !self.mounts.iter().any(|other| other.path != m.path
                        && matches!(&other.kind, MountKind::Procfs(o) if o.id == p.id))
                    && if m.path == "/proc" {
                        p.parent == launch.root_mount_id && p.subroot == "/"
                    } else {
                        m.read_only && PROC_READONLY.contains(&m.path.as_str())
                            && proc.is_some_and(|base| p.parent == base.id && p.device == base.device)
                            && m.path.strip_prefix("/proc") == Some(p.subroot.as_str())
                    },
                MountKind::RuntimeMasked => !natural && MASKED.contains(&m.path.as_str()) && m.read_only
                    && (!stock || !FILE_MASKS.contains(&m.path.as_str())),
                MountKind::StockMaskedDirectory => natural && m.read_only && dirs.iter().any(|(p, _)| *p == m.path),
                MountKind::RuntimeMaskedFile => stock && launch.stock_file_mask_intent()
                    && FILE_MASKS.contains(&m.path.as_str())
                    && launch.masked_paths.contains(&m.path),
                MountKind::DaemonGenerated => GENERATED_FILES.contains(&m.path.as_str()) && m.read_only,
                MountKind::Other => false,
            })
            && self.devices.len() <= 9
            && self.devices.windows(2).all(|w| w[0] < w[1])
            && self.devices.contains(&(1, 3)) // /dev/null for the no-input fd
            && self.devices.iter().all(|node| matches!(node, (1, 3 | 5 | 7 | 8 | 9) | (5, 0 | 1 | 2)))
            && self.interfaces == ["lo"] && self.non_loopback_route_count == 0
            && self.externally_reachable_control_handles == 0 && !self.writable_cgroup_control
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Descriptor {
    NullInput,
    StdoutPipe(u64),
    NonInputStderr,
}

impl Effective {
    pub(super) fn matches(&self, recipe: &Recipe, launch: &ResolvedLaunch, natural: bool) -> bool {
        self.task == launch.task
            && self.uid == [USER; 4]
            && self.gid == [GROUP; 4]
            && reduced_groups(&self.groups)
            && self.caps == [0; 5]
            && self.nnp
            && self.umask == PRIVATE_UMASK
            && self.seccomp_filters > 0
            && self.applied_seccomp_sha256 == recipe.seccomp_sha256
            && self.apparmor_enforcing
            && self.applied_apparmor_profile == recipe.apparmor_profile
            && self.applied_apparmor_sha256 == recipe.apparmor_policy_sha256
            && self.read_only_root
            && self.root_mount_id == launch.root_mount_id
            && self.namespaces.task[0] == self.task.pid_namespace
            && self
                .namespaces
                .task
                .into_iter()
                .zip(self.namespaces.host)
                .all(|(task, host)| task != 0 && host != 0 && task != host)
            && self
                .inventory
                .as_ref()
                .is_some_and(|inventory| inventory.closed(launch, self.namespaces.task[1], natural))
            && self.fds
                == [
                    Descriptor::NullInput,
                    Descriptor::StdoutPipe(launch.io_pipe_inode),
                    Descriptor::NonInputStderr,
                ]
    }
}
