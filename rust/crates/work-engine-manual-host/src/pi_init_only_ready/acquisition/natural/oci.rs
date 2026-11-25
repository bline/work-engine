//! Stock runc-v2 artifact selection for the original live task. Do not infer
//! what ran from Docker's Runtime label or from an installed version string.

use super::{Daemon, ExecutableNode, Result, Sender, full_hex, invalid, lifetime, refuse, within};
use crate::pi_docker::init_only_ready::acquisition::{
    kernel::ticks_and_parent,
    launch_state::{LaunchState, RUNC},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    time::Instant,
};

const SHIM: &str = "/usr/bin/containerd-shim-runc-v2";

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OciPin {
    pub(crate) shim_sha256: String,
    pub(crate) runc_sha256: String,
}

#[derive(Serialize)]
pub(super) struct Artifacts {
    shim: ExecutableNode,
    runc: ExecutableNode,
}

#[derive(Serialize)]
pub(super) struct ShimCut {
    sender: Sender,
    shim_ticks: u64,
    init_pid: u32,
    init_ticks: u64,
    argv: Vec<u8>,
    cwd: PathBuf,
    mount_namespace: (u64, u64),
    fs_root: (u64, u64),
    runc_command: String,
    runc_search_path: Option<String>,
}

fn protected_directory(path: &Path) -> Result<()> {
    if !path.is_absolute() || fs::canonicalize(path)?.as_os_str() != path.as_os_str() {
        return refuse("runtime lookup directory is not absolute and canonical");
    }
    for dir in path.ancestors() {
        let m = fs::symlink_metadata(dir)?;
        if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
            return refuse("runtime lookup directory has unprotected ancestry");
        }
    }
    Ok(())
}

fn disk_artifact(path: &Path, sha256: &str, deadline: Instant) -> Result<ExecutableNode> {
    protected_directory(
        path.parent()
            .ok_or_else(|| invalid("missing executable parent"))?,
    )?;
    let exe = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let m = exe.metadata()?;
    if m.mode() & 0o111 == 0 || m.nlink() != 1 {
        return refuse("runtime artifact is not a single-linked executable");
    }
    let node = ExecutableNode::observe(&m)?;
    node.check_hash(&exe, sha256, deadline)?;
    if ExecutableNode::observe(&fs::symlink_metadata(path)?)? != node {
        return refuse("runtime artifact changed during the read");
    }
    Ok(node)
}

fn kernel_bytes(path: &Path, max: u64) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    File::open(path)?.take(max + 1).read_to_end(&mut out)?;
    if out.len() as u64 > max || out.is_empty() {
        return refuse("unbounded or empty original runtime process record");
    }
    Ok(out)
}

fn exact_path(env: &[u8]) -> Result<&str> {
    let env = env
        .strip_suffix(&[0])
        .ok_or_else(|| invalid("incomplete original shim environment"))?;
    let mut paths = env
        .split(|b| *b == 0)
        .filter_map(|v| v.strip_prefix(b"PATH="));
    let value = paths
        .next()
        .filter(|s| !s.is_empty() && s.len() <= 4096)
        .ok_or_else(|| invalid("no bounded original shim PATH"))?;
    if paths.next().is_some() {
        return refuse("ambiguous original shim PATH");
    }
    std::str::from_utf8(value).map_err(|_| invalid("non-UTF8 original shim PATH"))
}

// A deliberately conservative subset of the stock Go exec.LookPath: admit
// no earlier candidate (even non-executable), ambiguous directory or ErrDot
// fallback. Do not execute the candidate or use the controller's PATH.
fn check_runc_path(value: &str, deadline: Instant) -> Result<()> {
    for dir in value.split(':') {
        within(deadline)?;
        let dir = Path::new(dir);
        protected_directory(dir)?;
        let candidate = dir.join("runc");
        if candidate == Path::new(RUNC) {
            return Ok(());
        }
        match fs::symlink_metadata(candidate) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            _ => return refuse("an earlier runc candidate exists or could not be excluded"),
        }
    }
    refuse("original shim PATH cannot select the pinned runc artifact")
}

fn coordinates(path: &Path) -> Result<(u64, u64)> {
    let m = fs::metadata(path)?;
    Ok((m.dev(), m.ino()))
}

fn stock_argv(cmd: &[u8], id: &str, address: &Path) -> bool {
    let Some(cmd) = cmd.strip_suffix(&[0]) else {
        return false;
    };
    let args: Vec<_> = cmd.split(|b| *b == 0).collect();
    let want = [
        SHIM.as_bytes(),
        b"-namespace",
        b"moby",
        b"-id",
        id.as_bytes(),
        b"-address",
        address.as_os_str().as_bytes(),
    ];
    args == want || args.strip_suffix(&[b"-debug".as_slice()]) == Some(want.as_slice())
}

impl OciPin {
    pub(super) fn check(&self) -> bool {
        full_hex(&self.shim_sha256) && full_hex(&self.runc_sha256)
    }

    pub(super) fn prepare(&self, deadline: Instant) -> Result<Artifacts> {
        Ok(Artifacts {
            shim: disk_artifact(Path::new(SHIM), &self.shim_sha256, deadline)?,
            runc: disk_artifact(Path::new(RUNC), &self.runc_sha256, deadline)?,
        })
    }

    pub(super) fn original_shim(
        &self,
        artifacts: &Artifacts,
        launch: &LaunchState,
        id: &str,
        init: (u32, u64),
        address: &Path,
        deadline: Instant,
    ) -> Result<(Daemon, ShimCut)> {
        let (init_pid, init_ticks) = init;
        let (ticks, parent) = ticks_and_parent(init_pid)?;
        if ticks != init_ticks || parent <= 1 || i32::try_from(parent).is_err() {
            return refuse("no exact original runc init parent");
        }
        // The stock shim is a child subreaper. Join the exact init's parent
        // to the selected shim by task (argv and cwd), serving endpoint and
        // loaded protected artifact, not a package label or process name.
        let shim = Daemon::pin(
            Sender {
                pid: parent as i32,
                uid: 0,
                gid: 0,
            },
            &self.shim_sha256,
            lifetime(parent)?.0,
            deadline,
        )?;
        let proc = PathBuf::from(format!("/proc/{parent}"));
        let argv = kernel_bytes(&proc.join("cmdline"), 4096)?;
        let kernel_status = kernel_bytes(&proc.join("status"), 16384)?;
        let status = std::str::from_utf8(&kernel_status)
            .map_err(|_| invalid("invalid original shim credentials"))?;
        let root_ids = |name: &str| {
            let mut rows = status.lines().filter_map(|s| s.strip_prefix(name));
            rows.next()
                .is_some_and(|r| r.split_ascii_whitespace().eq(["0", "0", "0", "0"]))
                && rows.next().is_none()
        };
        let cwd = fs::read_link(proc.join("cwd"))?;
        let mount_namespace = coordinates(&proc.join("ns/mnt"))?;
        let fs_root = coordinates(&proc.join("root"))?;
        if shim.node != artifacts.shim
            || !root_ids("Uid:")
            || !root_ids("Gid:")
            || !stock_argv(&argv, id, address)
            || cwd != launch.bundle()
            || mount_namespace != coordinates(Path::new("/proc/self/ns/mnt"))?
            || fs_root != coordinates(Path::new("/"))?
        {
            return refuse(
                "stock shim is not the approved original task service and filesystem view",
            );
        }
        let command = launch.runc_command()?;
        let runc_search_path = if command == "runc" {
            let env = kernel_bytes(&proc.join("environ"), 65536)?;
            let path = exact_path(&env)?;
            check_runc_path(path, deadline)?;
            Some(path.to_owned())
        } else {
            None
        };
        if disk_artifact(Path::new(RUNC), &self.runc_sha256, deadline)? != artifacts.runc {
            return refuse("runc artifact changed since pre-effect selection");
        }
        shim.check()?;
        if ticks_and_parent(init_pid)? != (init_ticks, parent) {
            return refuse("original init changed while pinning the selected shim");
        }
        within(deadline)?;
        let cut = ShimCut {
            sender: shim.sender,
            shim_ticks: shim.ticks,
            init_pid,
            init_ticks,
            argv,
            cwd,
            mount_namespace,
            fs_root,
            runc_command: command.into(),
            runc_search_path,
        };
        Ok((shim, cut))
    }
}

#[test]
fn stock_shim_projection_rejects_crossed_task_route_and_ambiguous_path() {
    let id = "3".repeat(64);
    let address = Path::new("/run/containerd/containerd.sock");
    let command = format!(
        "{SHIM}\0-namespace\0moby\0-id\0{id}\0-address\0{}\0",
        address.display()
    );
    assert!(stock_argv(command.as_bytes(), &id, address));
    assert!(stock_argv(
        format!("{command}-debug\0").as_bytes(),
        &id,
        address
    ));
    assert!(!stock_argv(command.as_bytes(), &"1".repeat(64), address));
    assert!(!stock_argv(
        command.as_bytes(),
        &id,
        Path::new("/other.sock")
    ));
    assert!(!stock_argv(
        command.trim_end_matches('\0').as_bytes(),
        &id,
        address
    ));
    assert_eq!(exact_path(b"X=1\0PATH=/usr/bin\0").unwrap(), "/usr/bin");
    for bad in [
        b"PATH=\0".as_slice(),
        b"PATH=relative",
        b"PATH=/a\0PATH=/b\0",
        b"HOME=/\0",
    ] {
        assert!(exact_path(bad).is_err(), "{bad:?}");
    }
    assert!(
        check_runc_path(
            ":/usr/bin",
            Instant::now() + std::time::Duration::from_secs(1)
        )
        .is_err()
    );
}
