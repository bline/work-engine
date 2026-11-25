//! Stock rootful Moby/containerd/runc launch inputs and retained init state.
//! These records identify the selected launch; they do not by themselves
//! establish applied policy, stdout provenance, or coverage of a Held cut.

use super::super::{ResolvedLaunch, Startup, StartupBacking, StdoutRoute, TaskKey};
use super::{Result, StartRpc, campaign_digest, full_hex, invalid, refuse, within};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Instant,
};

const BUNDLES: &str = "/run/containerd/io.containerd.runtime.v2.task/moby";
const RUNC_ROOT: &str = "/run/docker/runtime-runc";
const MAX_FILE: u64 = 32 * 1024;
pub(super) const RUNC: &str = "/usr/bin/runc";

#[derive(Serialize)]
pub(super) struct LaunchState {
    bundle: PathBuf,
    spec: Raw,
    state: Raw,
    runtime: Raw,
    options: Raw,
}

#[derive(Deserialize)]
struct RuncOptions {
    binary_name: String,
    root: String,
    #[serde(default)]
    no_pivot_root: bool,
}

#[derive(Serialize)]
struct Raw {
    path: PathBuf,
    device: u64,
    inode: u64,
    text: String,
}

#[derive(Deserialize)]
struct State {
    id: String,
    init_process_pid: u32,
    init_process_start: u64,
    created: String,
    external_descriptors: Option<Value>,
    cgroup_paths: BTreeMap<String, String>,
    config: Config,
}

#[derive(Deserialize)]
struct Config {
    rootfs: String,
    labels: Vec<String>,
    #[serde(default)]
    readonlyfs: bool,
    #[serde(default)]
    rootless_euid: bool,
    #[serde(default)]
    rootless_cgroups: bool,
    #[serde(default)]
    no_new_privileges: bool,
    seccomp: Option<Value>,
    mask_paths: Vec<String>,
    readonly_paths: Vec<String>,
}

#[derive(Deserialize)]
struct Spec {
    process: Process,
    root: Root,
    linux: Linux,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Process {
    #[serde(default)]
    terminal: bool,
    args: Vec<String>,
    cwd: String,
    env: Vec<String>,
    no_new_privileges: bool,
    apparmor_profile: String,
    user: User,
}

#[derive(Deserialize)]
struct User {
    uid: u32,
}

#[derive(Deserialize)]
struct Root {
    path: String,
    readonly: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Linux {
    masked_paths: Vec<String>,
    readonly_paths: Vec<String>,
    seccomp: Option<Value>,
}

pub(super) struct InitCoordinates {
    pub(super) ticks: u64,
    pub(super) cgroup: String,
}

fn pipe_inode(link: &str) -> Option<u64> {
    let inode = link.strip_prefix("pipe:[")?.strip_suffix(']')?;
    let number: u64 = inode.parse().ok()?;
    (number != 0 && inode == number.to_string()).then_some(number)
}

fn raw(path: PathBuf) -> Result<Raw> {
    if fs::canonicalize(&path)? != path {
        return refuse("selected launch records are not on the canonical stock path");
    }
    for dir in path.ancestors().skip(1) {
        let m = fs::symlink_metadata(dir)?;
        // A POSIX ACL write grant to a named user/group must also be in
        // the group-class mask; do not allow either outside write bit.
        if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
            return refuse("selected launch record has unprotected ancestry");
        }
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&path)?;
    let m = file.metadata()?;
    if !m.is_file() || m.uid() != 0 || m.mode() & 0o022 != 0 || m.nlink() != 1 {
        return refuse("selected launch record is not root-controlled");
    }
    let mut text = String::new();
    file.take(MAX_FILE + 1).read_to_string(&mut text)?;
    let after = fs::symlink_metadata(&path)?;
    if text.is_empty()
        || text.len() as u64 > MAX_FILE
        || text.len() as u64 != m.len()
        || (
            m.dev(),
            m.ino(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        ) != (
            after.dev(),
            after.ino(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec(),
        )
    {
        return refuse("selected launch record changed during the read");
    }
    Ok(Raw {
        path,
        device: m.dev(),
        inode: m.ino(),
        text,
    })
}

impl LaunchState {
    pub(super) fn read(start: &StartRpc, deadline: Instant) -> Result<Self> {
        within(deadline)?;
        if !full_hex(&start.full_id) {
            return refuse("selected launch requires a full ID");
        }
        let bundle = Path::new(BUNDLES).join(&start.full_id);
        let spec = raw(bundle.join("config.json"))?;
        let state = raw(Path::new(RUNC_ROOT)
            .join("moby")
            .join(&start.full_id)
            .join("state.json"))?;
        let runtime = raw(bundle.join("runtime"))?;
        let options = raw(bundle.join("options.json"))?;
        within(deadline)?;
        let retained = Self {
            bundle,
            spec,
            state,
            runtime,
            options,
        };
        retained.runc_command()?;
        Ok(retained)
    }

    /// The pinned containerd runc-v2 shim writes both values from the same
    /// decoded options it passes into process.NewRunc. runc's state root is
    /// per namespace. A Docker runtime name or a features/version string
    /// cannot substitute for these operands of the actual launch.
    pub(super) fn runc_command(&self) -> Result<&str> {
        let options: RuncOptions = serde_json::from_str(&self.options.text)?;
        if options.binary_name != self.runtime.text
            || !matches!(self.runtime.text.as_str(), "runc" | RUNC)
            || options.root != RUNC_ROOT
            || options.no_pivot_root
        {
            return refuse(
                "original shim selected a different runc command, state root or pivot mode",
            );
        }
        Ok(&self.runtime.text)
    }

    pub(super) fn bundle(&self) -> &Path {
        &self.bundle
    }

    /// Pinned stock Moby supplies the mounted layer's absolute BaseFS as
    /// spec.Root.Path. runc keeps an absolute root unchanged when converting
    /// its spec. A bundle-relative root can be valid OCI but is not evidence
    /// of this selected Moby materialization path.
    pub(super) fn materialized_root(&self) -> Result<PathBuf> {
        let spec: Spec = serde_json::from_str(&self.spec.text)?;
        let state: State = serde_json::from_str(&self.state.text)?;
        let root = PathBuf::from(&spec.root.path);
        if !root.is_absolute()
            || root == Path::new("/")
            || spec.root.path != state.config.rootfs
            || !spec.root.readonly
            || !state.config.readonlyfs
        {
            return refuse("retained root is not the selected stock read-only BaseFS");
        }
        Ok(root)
    }

    /// Project observed fields from the retained OCI bundle. The selected
    /// image has its own metadata/local-inspect join. Neither is a substitute
    /// for rooted startup observations or materialization continuity.
    pub(super) fn project_launch(
        &self,
        task: &TaskKey,
        image: &Startup,
        apparmor_profile: &str,
        root_mount_id: u64,
        startup_backing: StartupBacking,
        stdout_route: StdoutRoute,
    ) -> Result<ResolvedLaunch> {
        let spec: Spec = serde_json::from_str(&self.spec.text)?;
        let [stdout, _] = self.output_pipes()?;
        let observed = ResolvedLaunch {
            task: task.clone(),
            // This reader is for the stock runc state path only. The private
            // selector must separately bind the actual shim and runc artifacts.
            runtime: "runc".into(),
            platform: "linux/amd64".into(),
            argv: spec.process.args,
            cwd: spec.process.cwd,
            env: spec.process.env,
            image: image.clone(),
            root_mount_id,
            io_pipe_inode: stdout,
            startup_backing,
            stdout_route,
            proc_readonly_paths: spec.linux.readonly_paths,
            masked_paths: spec.linux.masked_paths,
        };
        self.check_launch(&observed, apparmor_profile)?;
        Ok(observed)
    }

    /// Join the startup and mount-policy inputs to the authoritative bundle
    /// retained at live selection. Live executable, mount and loaded-policy
    /// checks are still required; the bundle is not applied-state evidence.
    pub(super) fn check_launch(
        &self,
        launch: &ResolvedLaunch,
        apparmor_profile: &str,
    ) -> Result<u32> {
        let spec: Spec = serde_json::from_str(&self.spec.text)?;
        if !spec.root.readonly
            || spec.process.user.uid != super::super::artifact_contract::USER
            || spec.process.terminal
            || !spec.process.no_new_privileges
            || spec.process.args != launch.argv
            || spec.process.cwd != launch.cwd
            || spec.process.env != launch.env
            || spec.process.apparmor_profile != apparmor_profile
            || spec.linux.masked_paths != launch.masked_paths
            || spec.linux.readonly_paths != launch.proc_readonly_paths
        {
            return refuse("applied startup or mount protections crossed the original OCI bundle");
        }
        Ok(spec.process.user.uid)
    }

    /// The pin designates the exact converted runc input, not the Docker
    /// request, a raw BPF measurement, or the kernel's filter count. In the
    /// pinned standard-init path a successful exec with NNP first loads this
    /// Config.Seccomp and aborts on a load error. The controller requires that
    /// same original task, checked serving semantics and a live kernel cut.
    pub(super) fn seccomp_pin(&self) -> Result<String> {
        let spec: Spec = serde_json::from_str(&self.spec.text)?;
        let state: State = serde_json::from_str(&self.state.text)?;
        let converted = state.config.seccomp.as_ref().and_then(Value::as_object);
        if !state.config.no_new_privileges
            || !spec.linux.seccomp.as_ref().is_some_and(Value::is_object)
        {
            return refuse("selected OCI launch lacks NNP/seccomp policy inputs");
        }
        let config =
            converted.ok_or_else(|| invalid("selected runc launch lacks converted seccomp"))?;
        if !config.get("default_action").is_some_and(Value::is_number)
            || !config
                .get("syscalls")
                .and_then(Value::as_array)
                .is_some_and(|rules| !rules.is_empty())
            || config
                .get("listener_path")
                .is_some_and(|path| path.as_str() != Some(""))
        {
            return refuse("unknown or externally mediated runc seccomp configuration");
        }
        campaign_digest(&json!(["b-runc-v1.4.3-converted-seccomp-v1", config])).map_err(invalid)
    }

    pub(super) fn init_coordinates(&self, id: &str, pid: u32) -> Result<InitCoordinates> {
        let spec: Spec = serde_json::from_str(&self.spec.text)?;
        let state: State = serde_json::from_str(&self.state.text)?;
        let bundle_label = format!("bundle={}", self.bundle.display());
        let mut bundles = state
            .config
            .labels
            .iter()
            .filter(|s| s.starts_with("bundle="));
        let rootfs = self.bundle.join(&spec.root.path);
        if state.id != id
            || pid == 0
            || state.init_process_pid != pid
            || state.init_process_start == 0
            || state.created.is_empty()
            || bundles.next() != Some(&bundle_label)
            || bundles.next().is_some()
            || rootfs != Path::new(&state.config.rootfs)
            || !spec.root.readonly
            || !state.config.readonlyfs
            || state.config.rootless_euid
            || state.config.rootless_cgroups
            || spec.linux.masked_paths != state.config.mask_paths
            || spec.linux.readonly_paths != state.config.readonly_paths
            || state.cgroup_paths.len() != 1
        {
            return refuse("stock OCI bundle and retained runc init state do not join");
        }
        let cgroup = state
            .cgroup_paths
            .get("")
            .ok_or_else(|| invalid("selected init lacks a unified runtime cgroup"))?;
        if !cgroup.starts_with("/sys/fs/cgroup/") {
            return refuse("selected init lacks an absolute stock cgroup path");
        }
        Ok(InitCoordinates {
            ticks: state.init_process_start,
            cgroup: cgroup.clone(),
        })
    }

    pub(super) fn output_pipes(&self) -> Result<[u64; 2]> {
        let state: State = serde_json::from_str(&self.state.text)?;
        // Pinned runc saves these links before the init can move its FDs.
        // After pivot it reopens the null input but preserves output pipes.
        let Some([_, Value::String(out), Value::String(err)]) = state
            .external_descriptors
            .as_ref()
            .and_then(Value::as_array)
            .map(Vec::as_slice)
        else {
            return refuse("stock runc launch did not retain original output pipes");
        };
        pipe_inode(out)
            .zip(pipe_inode(err))
            .filter(|(out, err)| out != err)
            .map(|(out, err)| [out, err])
            .ok_or_else(|| invalid("stock runc launch lacks the selected stdout/stderr endpoints"))
    }
}

#[cfg(test)]
#[test]
fn stock_state_joins_exact_init_bundle_and_unified_cgroup() {
    let id = "1".repeat(64);
    let bundle = Path::new(BUNDLES).join(&id);
    let launch = super::super::tests::Synthetic::new().cut().launch.clone();
    let spec = json!({"process":{"args":launch.argv,"cwd":launch.cwd,"env":launch.env,"user":{"uid":65534},"apparmorProfile":"docker-default","noNewPrivileges":true},"root":{"path":"rootfs","readonly":true},"linux":{"maskedPaths":["/proc/kcore"],"readonlyPaths":["/proc/sys"],"seccomp":{"defaultAction":"SCMP_ACT_ERRNO"}}});
    let state = json!({"id":id,"init_process_pid":633,"init_process_start":741,"created":"2026-11-24T00:00:00Z","external_descriptors":["/dev/null","pipe:[112]","pipe:[113]"],"cgroup_paths":{"":"/sys/fs/cgroup/system.slice/test.scope"},"config":{"rootfs":bundle.join("rootfs"),"labels":[format!("bundle={}",bundle.display())],"readonlyfs":true,"mask_paths":["/proc/kcore"],"readonly_paths":["/proc/sys"],"no_new_privileges":true,"seccomp":{"default_action":2,"syscalls":[{"name":"read","action":1}]}}});
    let raw = |value: serde_json::Value| Raw {
        path: PathBuf::new(),
        device: 1,
        inode: 2,
        text: value.to_string(),
    };
    let build = |value| LaunchState {
        bundle: bundle.clone(),
        spec: raw(spec.clone()),
        state: raw(value),
        runtime: Raw {
            path: PathBuf::new(),
            device: 1,
            inode: 3,
            text: "runc".into(),
        },
        options: raw(json!({"binary_name":"runc","root":RUNC_ROOT})),
    };
    assert_eq!(build(state.clone()).runc_command().unwrap(), "runc");
    for invalid in [
        json!({"binary_name":"/usr/bin/runc","root":RUNC_ROOT}),
        json!({"binary_name":"runc","root":"/elsewhere"}),
        json!({"binary_name":"runc","root":RUNC_ROOT,"no_pivot_root":true}),
        json!({"root":RUNC_ROOT}),
    ] {
        let mut crossed = build(state.clone());
        crossed.options = raw(invalid);
        assert!(crossed.runc_command().is_err());
    }
    let mut applied = launch;
    let launch = build(state.clone());
    assert!(launch.materialized_root().is_err());
    let stock_root = "/var/lib/docker/rootfs/overlayfs/selected";
    let mut materialized = build(state.clone());
    let mut stock_spec = spec.clone();
    stock_spec["root"]["path"] = json!(stock_root);
    materialized.spec = raw(stock_spec);
    assert!(materialized.materialized_root().is_err());
    let mut stock_state = state.clone();
    stock_state["config"]["rootfs"] = json!(stock_root);
    materialized.state = raw(stock_state);
    assert_eq!(
        materialized.materialized_root().unwrap(),
        Path::new(stock_root)
    );
    assert!(materialized.init_coordinates(&id, 633).is_ok());
    applied.masked_paths = vec!["/proc/kcore".into()];
    applied.proc_readonly_paths = vec!["/proc/sys".into()];
    assert_eq!(
        launch.check_launch(&applied, "docker-default").unwrap(),
        65534
    );
    assert!(launch.check_launch(&applied, "other").is_err());
    let mut changed_uid = build(state.clone());
    let mut wrong_spec = spec.clone();
    wrong_spec["process"]["user"]["uid"] = json!(0);
    changed_uid.spec = raw(wrong_spec);
    assert!(
        changed_uid
            .check_launch(&applied, "docker-default")
            .is_err()
    );
    let mut crossed = applied.clone();
    crossed.env.push("EXTRA=1".into());
    assert!(launch.check_launch(&crossed, "docker-default").is_err());
    applied.masked_paths.push("/proc/keys".into());
    assert!(launch.check_launch(&applied, "docker-default").is_err());
    let got = launch.init_coordinates(&id, 633).unwrap();
    assert_eq!(got.ticks, 741);
    assert_eq!(got.cgroup, "/sys/fs/cgroup/system.slice/test.scope");
    assert_eq!(launch.output_pipes().unwrap(), [112, 113]);
    for (slot, invalid) in [(1, "pipe:[113]"), (2, "/run/alternate")] {
        let mut changed = state.clone();
        changed["external_descriptors"][slot] = json!(invalid);
        let changed = build(changed);
        assert!(changed.init_coordinates(&id, 633).is_ok());
        assert!(changed.output_pipes().is_err());
    }
    let pin = launch.seccomp_pin().unwrap();
    let mut changed = state.clone();
    changed["config"]["seccomp"]["default_action"] = json!(3);
    assert_ne!(build(changed.clone()).seccomp_pin().unwrap(), pin);
    changed["config"]["seccomp"]["listener_path"] = json!("/alternate");
    assert!(build(changed).seccomp_pin().is_err());
    let mut changed = state.clone();
    changed["config"]["seccomp"] = Value::Null;
    assert!(build(changed).seccomp_pin().is_err());
    assert!(launch.init_coordinates(&id, 634).is_err());
    assert!(launch.init_coordinates(&"2".repeat(64), 633).is_err());
    for (pointer, value) in [
        ("/config/labels", json!(["bundle=/unrelated"])),
        ("/config/rootfs", json!("/unrelated")),
        ("/config/mask_paths", json!(["/proc/keys"])),
        ("/cgroup_paths/", json!("/other/test.scope")),
    ] {
        let mut changed = state.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert!(
            build(changed).init_coordinates(&id, 633).is_err(),
            "{pointer}"
        );
    }
}
