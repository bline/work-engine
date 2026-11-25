//! Retain the *original* pidfd and cgroup-v2 directory. No lookup by current
//! PID/path on the terminal side. A removed cgroup that can no longer be
//! sampled is unknown, not empty.

use super::super::{FILE_MASKS, MountKind, MountView, PROC_READONLY};

#[cfg(target_arch = "x86_64")]
#[path = "isolation.rs"]
mod isolation;
#[cfg(target_arch = "x86_64")]
pub(super) use isolation::IsolationCut;
#[cfg(target_arch = "x86_64")]
#[path = "startup.rs"]
mod startup;
#[cfg(target_arch = "x86_64")]
pub(super) use startup::{MaterializedRoot, StartupCut};
#[cfg(target_arch = "x86_64")]
#[path = "stock_mounts.rs"]
mod stock_mounts;
use super::{Effective, TaskKey, boot_key};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{self, Read},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Component, Path, PathBuf},
};
#[cfg(target_arch = "x86_64")]
pub(super) use stock_mounts::DeviceCut;

#[derive(Serialize)]
pub(super) struct KernelCut {
    pub cgroup_events: Vec<u8>,
    pub status: Vec<u8>,
    pub apparmor: Vec<u8>,
    pub procfs: Option<ProcfsCut>,
}

#[cfg(target_arch = "x86_64")]
impl KernelCut {
    pub(super) fn mountinfo(&self) -> Option<&[u8]> {
        self.procfs
            .as_ref()
            .map(|procfs| procfs.mountinfo.as_slice())
    }
}

#[derive(Serialize)]
pub(super) struct ProcfsCut {
    mount_namespace: u64,
    proc_pid_namespace: u64,
    mountinfo: Vec<u8>,
    null_masks: NullMaskCut,
    #[cfg(target_arch = "x86_64")]
    #[serde(skip_serializing_if = "Option::is_none")]
    stock: Option<stock_mounts::StockMountCut>,
}

pub(super) trait Anchor {
    fn check_current(&self, effective: &Effective) -> io::Result<KernelCut>;
    fn live_events(&self) -> io::Result<Vec<u8>>;
    fn end_events(&self) -> io::Result<Vec<u8>>;
    #[cfg(target_arch = "x86_64")]
    fn original_kernel_task(&self) -> Option<&KernelTask> {
        None
    }
}

pub(super) struct KernelTask {
    key: TaskKey,
    pid: OwnedFd,
    group: File,
    member: String,
}

fn unknown() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "original lifetime or cgroup-v2 evidence unavailable",
    )
}

fn bounded(f: File, max: u64) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    f.take(max + 1).read_to_end(&mut data)?;
    if data.len() as u64 > max {
        return Err(unknown());
    }
    Ok(data)
}

fn proc_path(pid: u32, name: &str) -> PathBuf {
    PathBuf::from(format!("/proc/{pid}/{name}"))
}

fn member(pid: u32) -> io::Result<String> {
    let raw = bounded(File::open(proc_path(pid, "cgroup"))?, 4096)?;
    let s = std::str::from_utf8(&raw).map_err(|_| unknown())?;
    // The selected stack uses a unified v2 hierarchy. Never open a path
    // from the runtime if it disagrees with the live task's membership.
    let path = s
        .strip_prefix("0::/")
        .and_then(|s| s.strip_suffix('\n'))
        .ok_or_else(unknown)?;
    if path.is_empty()
        || path.contains(['\n', '\r', '\0'])
        || !path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
        || !Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(unknown());
    }
    Ok(path.to_owned())
}

pub(super) fn ticks_and_parent(pid: u32) -> io::Result<(u64, u32)> {
    let stat = bounded(File::open(proc_path(pid, "stat"))?, 4096)?;
    // comm may contain whitespace, ')' or newlines; the last ')' delimits
    // it. After comm, stat starts with field 3; starttime is field 22.
    let end = stat.iter().rposition(|&b| b == b')').ok_or_else(unknown)?;
    let rest = std::str::from_utf8(&stat[end + 1..]).map_err(|_| unknown())?;
    let mut fields = rest.split_ascii_whitespace();
    let parent = fields
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(unknown)?;
    let ticks = fields
        .nth(17)
        .and_then(|s| s.parse().ok())
        .ok_or_else(unknown)?;
    Ok((ticks, parent))
}

pub(super) fn lifetime(pid: u32) -> io::Result<(u64, u64)> {
    let (ticks, _) = ticks_and_parent(pid)?;
    Ok((ticks, fs::metadata(proc_path(pid, "ns/pid"))?.ino()))
}

fn check_lifetime(key: &TaskKey) -> io::Result<()> {
    if lifetime(key.pid)? != (key.start_ticks, key.pid_namespace) {
        return Err(unknown());
    }
    Ok(())
}

pub(super) fn exited(fd: &OwnedFd) -> io::Result<bool> {
    let mut p = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // Safety: p describes the still-owned pidfd, and is writable.
    match unsafe { libc::poll(&mut p, 1, 0) } {
        0 => Ok(false),
        1 if p.revents & libc::POLLIN != 0 && p.revents & (libc::POLLERR | libc::POLLNVAL) == 0 => {
            Ok(true)
        }
        _ => Err(unknown()),
    }
}

fn group_coordinates(group: &File) -> io::Result<(u64, u64)> {
    let mut info: libc::statfs = unsafe { std::mem::zeroed() };
    let mut stat: libc::statx = unsafe { std::mem::zeroed() };
    // Safety: the FD stays open and the pointers are to initialized storage
    // of the required types. AT_EMPTY_PATH means inspect that exact FD.
    if unsafe { libc::fstatfs(group.as_raw_fd(), &mut info) } != 0
        || info.f_type != libc::CGROUP2_SUPER_MAGIC
        || unsafe {
            libc::statx(
                group.as_raw_fd(),
                c"".as_ptr(),
                libc::AT_EMPTY_PATH,
                libc::STATX_MNT_ID | libc::STATX_INO,
                &mut stat,
            )
        } != 0
        || stat.stx_mask & (libc::STATX_MNT_ID | libc::STATX_INO)
            != (libc::STATX_MNT_ID | libc::STATX_INO)
        || stat.stx_mnt_id == 0
        || stat.stx_ino == 0
    {
        return Err(unknown());
    }
    Ok((stat.stx_mnt_id, stat.stx_ino))
}

fn group_at(path: &Path) -> io::Result<File> {
    if fs::canonicalize(path)? != path {
        return Err(unknown());
    }
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(target_arch = "x86_64")]
pub(super) fn selected_coordinates(
    pid: u32,
    start_ticks: u64,
    state_path: &str,
) -> io::Result<(u64, u64, u64)> {
    let (ticks, pid_namespace) = lifetime(pid)?;
    let path = Path::new("/sys/fs/cgroup").join(member(pid)?);
    if pid_namespace == 0 || ticks != start_ticks || path != Path::new(state_path) {
        return Err(unknown());
    }
    let (mount_id, inode) = group_coordinates(&group_at(&path)?)?;
    // Still a projection. The caller must retain the pidfd and recheck the
    // original cgroup-v2 handle against these coordinates before proceeding.
    Ok((pid_namespace, mount_id, inode))
}

impl KernelTask {
    pub(super) fn open(key: &TaskKey) -> io::Result<Self> {
        let boot = bounded(File::open("/proc/sys/kernel/random/boot_id")?, 64)?;
        if std::str::from_utf8(&boot)
            .map_err(|_| unknown())?
            .trim()
            .replace('-', "")
            != boot_key(&key.serving.boot)
        {
            return Err(unknown());
        }
        let id = i32::try_from(key.pid).map_err(|_| unknown())?;
        if id == 0 {
            return Err(unknown());
        }
        // Safety: pidfd_open takes a numeric PID and zero flags. On success
        // the new fd is unique and transferred to OwnedFd immediately.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, id, 0u32) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let pid = unsafe { OwnedFd::from_raw_fd(fd as i32) };
        if exited(&pid)? {
            return Err(unknown());
        }
        check_lifetime(key)?;
        let member = member(key.pid)?;
        let group = group_at(&Path::new("/sys/fs/cgroup").join(&member))?;
        if group_coordinates(&group)? != (key.subtree_mount_id, key.subtree_inode) {
            return Err(unknown());
        }
        let task = Self {
            key: key.clone(),
            pid,
            group,
            member,
        };
        task.live_events()?;
        Ok(task)
    }

    #[cfg(target_arch = "x86_64")]
    pub(super) fn retirement_key(&self) -> &TaskKey {
        // open() bound this full inode to the live original cgroup2 member.
        // On x86-64 kernfs exports its full ID as the inode. No pathname
        // lookup or dependence on cgroup.events after retirement.
        &self.key
    }

    #[cfg(target_arch = "x86_64")]
    pub(super) fn original_exited(&self) -> io::Result<bool> {
        exited(&self.pid)
    }

    /// Called only after the separate exact abort authorization is synced.
    /// This FD cannot target a replacement even if the original exits here.
    #[cfg(target_arch = "x86_64")]
    pub(super) fn abort_original(&self) -> io::Result<()> {
        rustix::process::pidfd_send_signal(&self.pid, rustix::process::Signal::KILL)
            .map_err(io::Error::from)
    }

    /// Observe descriptor endpoints without opening, reading or duplicating
    /// any task FD. A live original pidfd brackets the numeric proc lookup.
    /// This does not, by itself, establish the rest of the stdout route or
    /// interval coverage of the Held cut.
    #[cfg(target_arch = "x86_64")]
    pub(super) fn stock_stdio(&self, output_pipes: [u64; 2]) -> io::Result<StdioCut> {
        self.live_events()?;
        let pid = self.key.pid;
        let directories = || -> io::Result<Vec<u32>> {
            let mut indexes = Vec::with_capacity(3);
            for row in fs::read_dir(proc_path(pid, "fd"))? {
                let name = row?.file_name();
                let name = name.to_str().ok_or_else(unknown)?;
                let index: u32 = name.parse().map_err(|_| unknown())?;
                if indexes.len() == 3 || name != index.to_string() {
                    return Err(unknown());
                }
                indexes.push(index);
            }
            indexes.sort_unstable();
            Ok(indexes)
        };
        if directories()? != [0, 1, 2] {
            return Err(unknown());
        }
        let mut endpoints = Vec::with_capacity(3);
        for index in 0..3 {
            let path = proc_path(pid, &format!("fd/{index}"));
            let m = fs::metadata(&path)?;
            let target = fs::read_link(&path)?
                .into_os_string()
                .into_string()
                .map_err(|_| unknown())?;
            let fdinfo = bounded(
                File::open(proc_path(pid, &format!("fdinfo/{index}")))?,
                4096,
            )?;
            let after = fs::metadata(&path)?;
            if (m.dev(), m.ino(), m.mode(), m.rdev())
                != (after.dev(), after.ino(), after.mode(), after.rdev())
            {
                return Err(unknown());
            }
            endpoints.push(StdioEndpoint {
                index,
                target,
                device: m.dev(),
                inode: m.ino(),
                mode: m.mode(),
                rdev: (libc::major(m.rdev()), libc::minor(m.rdev())),
                fdinfo,
            });
        }
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_DIRECTORY)
            .open(proc_path(pid, "root"))?;
        let null = mask_node(&root, "/dev/null")?.ok_or_else(unknown)?;
        if directories()? != [0, 1, 2] || !closed_stock_stdio(&endpoints, output_pipes, null) {
            return Err(unknown());
        }
        self.live_events()?;
        Ok(StdioCut { endpoints, null })
    }

    fn events(&self) -> io::Result<Vec<u8>> {
        // Safety: group stays open, the name is static and relative to the
        // original directory, and a successful fresh FD is owned below.
        let fd = unsafe {
            libc::openat(
                self.group.as_raw_fd(),
                c"cgroup.events".as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        bounded(unsafe { File::from_raw_fd(fd) }, 1024)
    }
}

#[cfg(target_arch = "x86_64")]
#[derive(Serialize)]
pub(super) struct StdioCut {
    endpoints: Vec<StdioEndpoint>,
    null: MaskNode,
}

#[cfg(target_arch = "x86_64")]
impl StdioCut {
    pub(super) fn descriptors(&self) -> Vec<super::super::Descriptor> {
        use super::super::Descriptor;
        // Only stock_stdio can construct this cut in production, after it
        // checks the exact descriptor set, access modes and original pipes.
        vec![
            Descriptor::NullInput,
            Descriptor::StdoutPipe(self.endpoints[1].inode),
            Descriptor::NonInputStderr,
        ]
    }
}

#[cfg(target_arch = "x86_64")]
#[derive(Serialize)]
struct StdioEndpoint {
    index: u32,
    target: String,
    device: u64,
    inode: u64,
    mode: u32,
    rdev: (u32, u32),
    fdinfo: Vec<u8>,
}

#[cfg(target_arch = "x86_64")]
fn closed_stock_stdio(endpoints: &[StdioEndpoint], output_pipes: [u64; 2], null: MaskNode) -> bool {
    fn fdinfo(endpoint: &StdioEndpoint) -> Option<(u32, u64)> {
        let text = std::str::from_utf8(&endpoint.fdinfo).ok()?;
        let field = |name: &str| {
            let mut rows = text.lines().filter_map(|s| s.strip_prefix(name));
            let value = rows.next()?.trim();
            rows.next().is_none().then_some(value)
        };
        let flags = u32::from_str_radix(field("flags:")?, 8).ok()?;
        if flags & libc::O_PATH as u32 != 0
            || field("ino:")?.parse::<u64>().ok() != Some(endpoint.inode)
        {
            return None;
        }
        let mount = field("mnt_id:")?.parse().ok()?;
        (mount != 0).then_some((flags & libc::O_ACCMODE as u32, mount))
    }
    let [input, out, err] = endpoints else {
        return false;
    };
    let pipe_writer = |fd: &StdioEndpoint| {
        fd.mode & libc::S_IFMT == libc::S_IFIFO
            && fd.target == format!("pipe:[{}]", fd.inode)
            && fdinfo(fd).is_some_and(|(access, _)| access == libc::O_WRONLY as u32)
    };
    [input.index, out.index, err.index] == [0, 1, 2]
        // runc reopens inherited null stdio after pivot with O_RDWR. Both
        // directions remain inert only for the exact task-local null node.
        && input.mode & libc::S_IFMT == libc::S_IFCHR
        && null.mode & libc::S_IFMT == libc::S_IFCHR
        && input.rdev == (1, 3)
        && null.rdev == (1, 3)
        && input.inode == null.inode
        && (libc::major(input.device), libc::minor(input.device)) == null.device
        && fdinfo(input) == Some((libc::O_RDWR as u32, null.mount_id))
        && [out.inode, err.inode] == output_pipes
        && !output_pipes.contains(&0)
        && out.inode != err.inode
        && out.device == err.device
        && pipe_writer(out)
        && pipe_writer(err)
}

#[cfg(all(test, target_arch = "x86_64"))]
#[test]
fn stock_stdio_requires_exact_null_and_separate_write_only_pipes() {
    let endpoint = |index, inode, mode, flags: u32| StdioEndpoint {
        index,
        target: format!("pipe:[{inode}]"),
        device: 1,
        inode,
        mode,
        rdev: (0, 0),
        fdinfo: format!("pos:\t0\nflags:\t{flags:o}\nmnt_id:\t19\nino:\t{inode}\n").into_bytes(),
    };
    let mut input = endpoint(0, 111, libc::S_IFCHR, 2);
    input.target = "/dev/null".into();
    input.rdev = (1, 3);
    let null = MaskNode {
        mount_id: 19,
        device: (0, 1),
        inode: 111,
        mode: libc::S_IFCHR,
        rdev: (1, 3),
    };
    let mut fds = vec![
        input,
        endpoint(1, 112, libc::S_IFIFO, 0o4001),
        endpoint(2, 113, libc::S_IFIFO, 1),
    ];
    assert!(closed_stock_stdio(&fds, [112, 113], null));
    assert!(!closed_stock_stdio(&fds, [123, 113], null));
    assert!(!closed_stock_stdio(&fds, [112, 114], null));
    assert!(!closed_stock_stdio(
        &fds,
        [112, 113],
        MaskNode { inode: 222, ..null }
    ));
    fds.push(endpoint(3, 114, libc::S_IFSOCK, 2));
    assert!(!closed_stock_stdio(&fds, [112, 113], null));
    fds.pop();
    fds[0].rdev = (1, 5);
    assert!(!closed_stock_stdio(&fds, [112, 113], null));
    fds[0].rdev = (1, 3);
    fds[0].fdinfo.extend_from_slice(b"flags:\t2\n");
    assert!(!closed_stock_stdio(&fds, [112, 113], null));
    let original_len = fds[0].fdinfo.len() - b"flags:\t2\n".len();
    fds[0].fdinfo.truncate(original_len);
    for stderr in [
        endpoint(2, 112, libc::S_IFIFO, 1),
        endpoint(2, 113, libc::S_IFIFO, 2),
        endpoint(2, 113, libc::S_IFREG, 1),
    ] {
        fds[2] = stderr;
        assert!(!closed_stock_stdio(&fds, [112, 113], null));
    }
}

// The private natural route projects these values from the exact retained
// task's raw status. The one-Stop witness checker below keeps its existing
// comparison and wire shape.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, PartialEq, Eq)]
pub(super) struct StockCredentials {
    pub uid: [u32; 4],
    pub gid: [u32; 4],
    pub groups: Vec<u32>,
    pub caps: [u64; 5],
    pub nnp: bool,
    pub umask: u16,
    pub seccomp_filters: u32,
}

#[cfg(target_arch = "x86_64")]
impl StockCredentials {
    fn parse(raw: &[u8]) -> io::Result<Self> {
        let status = std::str::from_utf8(raw).map_err(|_| unknown())?;
        let field = |key: &str| -> io::Result<&str> {
            let mut fields = status.lines().filter_map(|line| {
                let (name, value) = line.split_once(':')?;
                (name == key).then_some(value.trim())
            });
            let value = fields.next().ok_or_else(unknown)?;
            if fields.next().is_some() {
                return Err(unknown());
            }
            Ok(value)
        };
        let numbers = |key| -> io::Result<Vec<u32>> {
            field(key)?
                .split_ascii_whitespace()
                .map(|s| s.parse().map_err(|_| unknown()))
                .collect()
        };
        let nnp = match field("NoNewPrivs")? {
            "1" => true,
            "0" => false,
            _ => return Err(unknown()),
        };
        if field("Seccomp")? != "2" {
            return Err(unknown());
        }
        let mut caps = [0; 5];
        for (slot, name) in caps
            .iter_mut()
            .zip(["CapInh", "CapPrm", "CapEff", "CapBnd", "CapAmb"])
        {
            *slot = u64::from_str_radix(field(name)?, 16).map_err(|_| unknown())?;
        }
        let groups = numbers("Groups")?;
        if groups.len() > 1 {
            return Err(unknown());
        }
        Ok(Self {
            uid: numbers("Uid")?.try_into().map_err(|_| unknown())?,
            gid: numbers("Gid")?.try_into().map_err(|_| unknown())?,
            groups,
            caps,
            nnp,
            umask: u16::from_str_radix(field("Umask")?, 8).map_err(|_| unknown())?,
            seccomp_filters: field("Seccomp_filters")?.parse().map_err(|_| unknown())?,
        })
    }
}

#[cfg(target_arch = "x86_64")]
impl KernelTask {
    pub(super) fn stock_identity(&self) -> io::Result<(Vec<u8>, Vec<u8>, StockCredentials)> {
        self.live_events()?;
        let p = self.key.pid;
        let status = bounded(File::open(proc_path(p, "status"))?, 16_384)?;
        let apparmor = bounded(File::open(proc_path(p, "attr/apparmor/current"))?, 1024)?;
        let credentials = StockCredentials::parse(&status)?;
        self.live_events()?;
        Ok((status, apparmor, credentials))
    }
}

#[cfg(all(test, target_arch = "x86_64"))]
#[test]
fn natural_status_projects_complete_kernel_credentials_without_claim_inputs() {
    let source = concat!(
        "Uid:\t65534\t65534\t65534\t65534\nGid:\t65534\t65534\t65534\t65534\n",
        "Groups:\t65534\nNoNewPrivs:\t1\nSeccomp:\t2\nSeccomp_filters:\t3\nUmask:\t0077\n",
        "CapInh:\t0\nCapPrm:\t0\nCapEff:\t0\nCapBnd:\t0\nCapAmb:\t0\n",
    );
    let seen = StockCredentials::parse(source.as_bytes()).unwrap();
    assert_eq!(seen.uid, [65534; 4]);
    assert_eq!(seen.gid, [65534; 4]);
    assert_eq!(seen.groups, [65534]);
    assert_eq!(seen.caps, [0; 5]);
    assert_eq!(seen.seccomp_filters, 3);
    assert!(seen.nnp);
    assert_eq!(seen.umask, 0o077);
    assert_eq!(
        StockCredentials::parse(source.replace("CapPrm:\t0", "CapPrm:\t2").as_bytes())
            .unwrap()
            .caps[1],
        2
    );
    for wrong in [
        source.replace("Seccomp:\t2\n", ""),
        source.replace("NoNewPrivs:\t1", "NoNewPrivs:\t2"),
        source.replace("CapEff:\t0\n", "CapEff:\t0\nCapEff:\t0\n"),
        source.replace("Groups:\t65534", "Groups:\t65534 0"),
    ] {
        assert!(StockCredentials::parse(wrong.as_bytes()).is_err());
    }
}

fn check_status(raw: &[u8], effective: &Effective) -> io::Result<()> {
    let status = std::str::from_utf8(raw).map_err(|_| unknown())?;
    let field = |key: &str| -> io::Result<&str> {
        let mut values = status.lines().filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            (name == key).then_some(value.trim())
        });
        let value = values.next().ok_or_else(unknown)?;
        if values.next().is_some() {
            return Err(unknown());
        }
        Ok(value)
    };
    let numbers = |key: &str| -> io::Result<Vec<u32>> {
        field(key)?
            .split_ascii_whitespace()
            .map(|s| s.parse().map_err(|_| unknown()))
            .collect()
    };
    if numbers("Uid")? != effective.uid
        || numbers("Gid")? != effective.gid
        || numbers("Groups")? != effective.groups
        || field("NoNewPrivs")? != (if effective.nnp { "1" } else { "0" })
        || u16::from_str_radix(field("Umask")?, 8).ok() != Some(effective.umask)
        || field("Seccomp")? != "2"
        || field("Seccomp_filters")?.parse::<u32>().ok() != Some(effective.seccomp_filters)
    {
        return Err(unknown());
    }
    for (name, cap) in ["CapInh", "CapPrm", "CapEff", "CapBnd", "CapAmb"]
        .into_iter()
        .zip(effective.caps)
    {
        if u64::from_str_radix(field(name)?, 16).ok() != Some(cap) {
            return Err(unknown());
        }
    }
    Ok(())
}

struct MountRow<'a> {
    id: u64,
    parent: &'a str,
    device: &'a str,
    subroot: &'a str,
    path: &'a str,
    tail: &'a str,
    flags: &'a str,
    read_only: bool,
}

impl MountRow<'_> {
    fn parse(line: &str) -> io::Result<MountRow<'_>> {
        let (fields, tail) = line.split_once(" - ").ok_or_else(unknown)?;
        let mut fields = fields.split_ascii_whitespace();
        let id = fields
            .next()
            .and_then(|v| v.parse().ok())
            .ok_or_else(unknown)?;
        let parent = fields.next().ok_or_else(unknown)?;
        let device = fields.next().ok_or_else(unknown)?;
        let subroot = fields.next().ok_or_else(unknown)?;
        let path = fields.next().ok_or_else(unknown)?;
        let flags = fields.next().ok_or_else(unknown)?;
        let read_only = flags.split(',').any(|f| f == "ro");
        Ok(MountRow {
            id,
            parent,
            device,
            subroot,
            path,
            tail,
            flags,
            read_only,
        })
    }

    fn tmpfs(&self) -> bool {
        let mut tail = self.tail.split_ascii_whitespace();
        if tail.next() != Some("tmpfs") || tail.next() != Some("tmpfs") {
            return false;
        }
        let Some(flags) = tail.next() else {
            return false;
        };
        let mut modes = flags.split(',').filter(|f| matches!(*f, "ro" | "rw"));
        matches!(modes.next(), Some("rw")) && modes.next().is_none() && tail.next().is_none()
    }

    fn device(&self) -> Option<(u32, u32)> {
        let (a, b) = self.device.split_once(':')?;
        Some((a.parse().ok()?, b.parse().ok()?))
    }
}

#[derive(Default)]
struct MaskRows<'a> {
    dev: Option<MountRow<'a>>,
    proc_id: u64,
    files: Vec<MountRow<'a>>,
}

fn check_proc_readonly<'a>(
    raw: &'a [u8],
    mounts: &[MountView],
    namespace: u64,
    root_mount: u64,
) -> io::Result<MaskRows<'a>> {
    let text = std::str::from_utf8(raw).map_err(|_| unknown())?;
    let mut seen = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut masks = MaskRows::default();
    for line in text.lines() {
        let row = MountRow::parse(line)?;
        let MountRow {
            id,
            parent,
            device,
            subroot,
            path,
            tail,
            flags,
            ..
        } = row;
        let flag = |name| flags.split(',').any(|f| f == name);
        let check_mode =
            |ro| flag(if ro { "ro" } else { "rw" }) && !flag(if ro { "rw" } else { "ro" });
        if id == 0 || !ids.insert(id) || !seen.insert(path) {
            return Err(unknown());
        }
        if path == "/" {
            if id != root_mount || !check_mode(true) {
                return Err(unknown());
            }
            continue;
        }
        // The protected inventory must describe every actual non-root entry;
        // don't hide unrelated mounts while interpreting this one family.
        let mount = mounts.iter().find(|m| m.path == path).ok_or_else(unknown)?;
        if !check_mode(mount.read_only) {
            return Err(unknown());
        }
        if path == "/dev" || FILE_MASKS.contains(&path) {
            if path == "/dev" {
                if !matches!(mount.kind, MountKind::PrivateVirtual) {
                    return Err(unknown());
                }
                masks.dev = Some(row);
            } else {
                if !matches!(mount.kind, MountKind::RuntimeMaskedFile) {
                    return Err(unknown());
                }
                masks.files.push(row);
            }
        } else if matches!(mount.kind, MountKind::RuntimeMaskedFile) {
            return Err(unknown());
        }
        if path != "/proc" && !PROC_READONLY.contains(&path) {
            continue;
        }
        if path == "/proc" {
            masks.proc_id = id;
        }
        let MountKind::Procfs(backing) = &mount.kind else {
            return Err(unknown());
        };
        let (major, minor) = device.split_once(':').ok_or_else(unknown)?;
        if tail.split_ascii_whitespace().next() != Some("proc")
            || backing.namespace != namespace
            || id != backing.id
            || parent.parse().ok() != Some(backing.parent)
            || (major.parse().ok(), minor.parse().ok())
                != (Some(backing.device.0), Some(backing.device.1))
            || subroot != backing.subroot
            || ["nosuid", "nodev", "noexec"].iter().any(|&f| !flag(f))
            || ["suid", "dev", "exec"].iter().any(|&f| flag(f))
        {
            return Err(unknown());
        }
    }
    if seen.len() != mounts.len() + 1
        || !["/", "/proc"]
            .into_iter()
            .chain(PROC_READONLY)
            .all(|p| seen.contains(p))
    {
        return Err(unknown());
    }
    Ok(masks)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
struct MaskNode {
    mount_id: u64,
    device: (u32, u32),
    inode: u64,
    mode: u32,
    rdev: (u32, u32),
}

impl MaskNode {
    fn on_mount(self, row: &MountRow<'_>) -> bool {
        self.mount_id == row.id
            && Some(self.device) == row.device()
            && self.inode != 0
            && self.mode & libc::S_IFMT == libc::S_IFCHR
            && self.rdev == (1, 3)
    }
}

#[derive(Serialize)]
struct NullMaskCut {
    source: Option<MaskNode>,
    /// None records an ENOENT from the original task root, not a missing row alone.
    targets: Vec<(&'static str, Option<MaskNode>)>,
}

fn verify_null_masks(
    rows: &MaskRows<'_>,
    root_mount: u64,
    mut observe: impl FnMut(&str) -> io::Result<Option<MaskNode>>,
) -> io::Result<NullMaskCut> {
    let dev = rows.dev.as_ref();
    let source = if rows.files.is_empty() {
        None
    } else {
        let dev = dev.ok_or_else(unknown)?;
        let node = observe("/dev/null")?.ok_or_else(unknown)?;
        if !dev.tmpfs()
            || dev.subroot != "/"
            || dev.parent.parse().ok() != Some(root_mount)
            || !node.on_mount(dev)
        {
            return Err(unknown());
        }
        Some(node)
    };
    let mut targets = Vec::with_capacity(FILE_MASKS.len());
    for path in FILE_MASKS {
        let node = observe(path)?;
        match (rows.files.iter().find(|row| row.path == path), node) {
            (None, None) => (),
            (Some(row), Some(node))
                if source.zip(dev).is_some_and(|(null, dev)| {
                    row.tail == dev.tail
                        && row.subroot == "/null"
                        && row.parent.parse().ok() == Some(rows.proc_id)
                        && row.read_only == dev.read_only
                        && node.on_mount(row)
                        && node.device == null.device
                        && node.inode == null.inode
                }) =>
            {
                ()
            }
            _ => return Err(unknown()),
        }
        targets.push((path, node));
    }
    Ok(NullMaskCut { source, targets })
}

fn mask_node(root: &File, path: &str) -> io::Result<Option<MaskNode>> {
    // The proc root FD is already pinned; refuse symlinks (including magic
    // links) below it, while allowing crossing the mounts we are proving.
    let path = std::ffi::CString::new(path.strip_prefix('/').ok_or_else(unknown)?)?;
    let mut how: libc::open_how = unsafe { std::mem::zeroed() };
    how.flags = (libc::O_PATH | libc::O_CLOEXEC) as u64;
    how.resolve = libc::RESOLVE_IN_ROOT | libc::RESOLVE_NO_SYMLINKS;
    // Safety: all parameters point to live owned storage for the syscall.
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            root.as_raw_fd(),
            path.as_ptr(),
            &how,
            std::mem::size_of_val(&how),
        )
    };
    if fd < 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(libc::ENOENT) {
            Ok(None)
        } else {
            Err(error)
        };
    }
    let file = unsafe { File::from_raw_fd(fd as i32) };
    Ok(Some(node(&file)?))
}

fn node(file: &File) -> io::Result<MaskNode> {
    let mut stat: libc::statx = unsafe { std::mem::zeroed() };
    let fields = libc::STATX_TYPE | libc::STATX_INO | libc::STATX_MNT_ID;
    if unsafe {
        libc::statx(
            file.as_raw_fd(),
            c"".as_ptr(),
            libc::AT_EMPTY_PATH,
            fields,
            &mut stat,
        )
    } != 0
        || stat.stx_mask & fields != fields
    {
        return Err(unknown());
    }
    Ok(MaskNode {
        mount_id: stat.stx_mnt_id,
        device: (stat.stx_dev_major, stat.stx_dev_minor),
        inode: stat.stx_ino,
        mode: stat.stx_mode.into(),
        rdev: (stat.stx_rdev_major, stat.stx_rdev_minor),
    })
}

impl Anchor for KernelTask {
    #[cfg(target_arch = "x86_64")]
    fn original_kernel_task(&self) -> Option<&KernelTask> {
        Some(self)
    }

    fn check_current(&self, effective: &Effective) -> io::Result<KernelCut> {
        if effective.task != self.key {
            return Err(unknown());
        }
        self.live_events()?;
        let status = bounded(File::open(proc_path(self.key.pid, "status"))?, 16_384)?;
        let apparmor = bounded(
            File::open(proc_path(self.key.pid, "attr/apparmor/current"))?,
            1024,
        )?;
        check_status(&status, effective)?;
        if !effective.apparmor_enforcing
            || apparmor
                != format!("{} (enforce)\n", effective.applied_apparmor_profile).into_bytes()
        {
            return Err(unknown());
        }
        let procfs = if let Some(inventory) = effective.inventory.as_ref().filter(|inventory| {
            inventory
                .mounts
                .iter()
                .any(|m| matches!(m.kind, MountKind::Procfs(_)))
        }) {
            let p = self.key.pid;
            let mount_namespace = fs::metadata(proc_path(p, "ns/mnt"))?.ino();
            let proc_pid_namespace = fs::metadata(proc_path(p, "root/proc/1/ns/pid"))?.ino();
            let mountinfo = bounded(File::open(proc_path(p, "mountinfo"))?, 64 * 1024)?;
            if mount_namespace != effective.namespaces.task[1]
                || proc_pid_namespace != self.key.pid_namespace
                || fs::metadata(proc_path(p, "ns/mnt"))?.ino() != mount_namespace
            {
                return Err(unknown());
            }
            let masks = check_proc_readonly(
                &mountinfo,
                &inventory.mounts,
                mount_namespace,
                effective.root_mount_id,
            )?;
            let root = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_PATH | libc::O_DIRECTORY)
                .open(proc_path(p, "root"))?;
            let null_masks = verify_null_masks(&masks, effective.root_mount_id, |path| {
                mask_node(&root, path)
            })?;
            if fs::metadata(proc_path(p, "ns/mnt"))?.ino() != mount_namespace {
                return Err(unknown());
            }
            Some(ProcfsCut {
                mount_namespace,
                proc_pid_namespace,
                mountinfo,
                null_masks,
                #[cfg(target_arch = "x86_64")]
                stock: None,
            })
        } else {
            None
        };
        Ok(KernelCut {
            cgroup_events: self.live_events()?,
            status,
            apparmor,
            procfs,
        })
    }

    fn live_events(&self) -> io::Result<Vec<u8>> {
        if exited(&self.pid)? || member(self.key.pid)? != self.member {
            return Err(unknown());
        }
        check_lifetime(&self.key)?;
        let raw = self.events()?;
        if exited(&self.pid)? || member(self.key.pid)? != self.member {
            return Err(unknown());
        }
        Ok(raw)
    }

    fn end_events(&self) -> io::Result<Vec<u8>> {
        if !exited(&self.pid)? {
            return Err(unknown());
        }
        self.events()
    }
}

#[cfg(test)]
const PROC_MOUNT_ROWS: &str = concat!(
    "17 1 0:21 / / ro - overlay overlay rw\n",
    "20 17 0:31 / /proc rw,nosuid,nodev,noexec - proc proc rw\n",
    "21 20 0:31 /bus /proc/bus ro,nosuid,nodev,noexec - proc proc rw\n",
    "22 20 0:31 /fs /proc/fs ro,nosuid,nodev,noexec - proc proc rw\n",
    "23 20 0:31 /irq /proc/irq ro,nosuid,nodev,noexec - proc proc rw\n",
    "24 20 0:31 /sys /proc/sys ro,nosuid,nodev,noexec - proc proc rw\n",
    "25 20 0:31 /sysrq-trigger /proc/sysrq-trigger ro,nosuid,nodev,noexec - proc proc rw\n",
);

#[cfg(test)]
#[test]
fn proc_readonly_checks_raw_kernel_mountinfo_not_only_normalized_claims() {
    use super::super::tests::proc_readonly_mounts;

    let mounts: Vec<_> = proc_readonly_mounts().collect();
    let raw = PROC_MOUNT_ROWS;
    assert!(check_proc_readonly(raw.as_bytes(), &mounts, 12, 17).is_ok());
    assert!(check_proc_readonly(raw.as_bytes(), &mounts, 99, 17).is_err());
    assert!(check_proc_readonly(raw.as_bytes(), &mounts, 12, 99).is_err());

    let fs = raw
        .lines()
        .find(|line| line.contains(" /proc/fs "))
        .unwrap();
    for (from, to) in [
        (" - proc ", " - tmpfs "),
        ("0:31", "0:32"),
        ("22 20", "22 17"),
        (" /fs ", " / "),
        (" /proc/fs ", " /proc/fs-unknown "),
        ("ro,nosuid", "rw,nosuid"),
        ("ro,nosuid", "ro,rw,nosuid"),
        (",nosuid", ""),
        (",nodev", ""),
        (",noexec", ""),
    ] {
        let bad = raw.replace(fs, &fs.replace(from, to));
        assert!(
            check_proc_readonly(bad.as_bytes(), &mounts, 12, 17).is_err(),
            "{to}"
        );
    }
    for bad in [
        raw.replace(fs, ""),
        format!("{raw}{fs}\n"),
        format!("{raw}26 17 0:42 / /hidden rw - tmpfs tmpfs rw\n"),
    ] {
        assert!(check_proc_readonly(bad.as_bytes(), &mounts, 12, 17).is_err());
    }
}

#[cfg(test)]
#[test]
fn file_mask_kernel_join_keeps_raw_inventory_and_rejects_substitution_or_missing_mask() {
    use super::super::tests::proc_readonly_mounts;

    const DEV: &str = "26 17 0:32 / /dev rw,nosuid - tmpfs tmpfs rw,size=65536k\n";
    const MASK: &str = "27 20 0:32 /null /proc/kcore rw,nosuid - tmpfs tmpfs rw,size=65536k\n";
    let raw = format!("{PROC_MOUNT_ROWS}{DEV}{MASK}");
    let mounts: Vec<_> = proc_readonly_mounts()
        .chain([
            MountView {
                path: "/dev".into(),
                kind: MountKind::PrivateVirtual,
                read_only: false,
            },
            MountView {
                path: "/proc/kcore".into(),
                kind: MountKind::RuntimeMaskedFile,
                read_only: false,
            },
        ])
        .collect();
    let null = MaskNode {
        mount_id: 26,
        device: (0, 32),
        inode: 3,
        mode: libc::S_IFCHR,
        rdev: (1, 3),
    };
    let masked = MaskNode {
        mount_id: 27,
        ..null
    };
    let observe = |path: &str| {
        Ok(match path {
            "/dev/null" => Some(null),
            "/proc/kcore" => Some(masked),
            _ => None,
        })
    };
    let rows = check_proc_readonly(raw.as_bytes(), &mounts, 12, 17).unwrap();
    let check = |raw: &[u8]| {
        check_proc_readonly(raw, &mounts, 12, 17).and_then(|r| verify_null_masks(&r, 17, observe))
    };
    let valid = verify_null_masks(&rows, 17, observe).unwrap();
    assert_eq!(valid.source.unwrap().mount_id, 26);
    assert_eq!(
        valid
            .targets
            .iter()
            .filter(|(_, val)| val.is_some())
            .count(),
        1
    );
    assert_eq!(valid.targets.len(), FILE_MASKS.len());

    for node in [
        MaskNode {
            mode: libc::S_IFREG,
            ..masked
        },
        MaskNode { inode: 4, ..masked },
        MaskNode {
            rdev: (1, 5),
            ..masked
        },
        MaskNode {
            device: (0, 33),
            ..masked
        },
        MaskNode {
            mount_id: 26,
            ..masked
        },
    ] {
        assert!(
            verify_null_masks(&rows, 17, |path| {
                if path == "/proc/kcore" {
                    Ok(Some(node))
                } else {
                    observe(path)
                }
            })
            .is_err(),
            "{node:?}"
        );
    }
    for path in ["/dev/null", "/proc/kcore"] {
        assert!(
            verify_null_masks(&rows, 17, |p| {
                if p == path { Ok(None) } else { observe(p) }
            })
            .is_err()
        );
    }
    assert!(
        verify_null_masks(&rows, 17, |path| {
            if path == "/proc/timer_stats" {
                Ok(Some(masked))
            } else {
                observe(path)
            }
        })
        .is_err()
    );
    assert!(
        verify_null_masks(&rows, 17, |path| {
            if path == "/proc/timer_stats" {
                Err(io::Error::from_raw_os_error(libc::EACCES))
            } else {
                observe(path)
            }
        })
        .is_err()
    );

    for changed in [
        MASK.replace("/null", "/substituted"),
        MASK.replace("27 20", "27 17"),
        MASK.replace("0:32", "0:42"),
        MASK.replace("- tmpfs tmpfs", "- tmpfs unknown"),
        MASK.replace(" rw,nosuid", " ro,nosuid"),
        MASK.replace(" rw,nosuid", " rw,ro,nosuid"),
        MASK.replace("/proc/kcore", "/proc/kcore-extra"),
        format!("{MASK}{MASK}"),
        format!("{MASK}28 17 0:42 / /hidden rw - tmpfs tmpfs rw\n"),
        String::new(),
    ] {
        let bad = format!("{PROC_MOUNT_ROWS}{DEV}{changed}");
        assert!(check(bad.as_bytes()).is_err(), "{changed}");
    }
    for dev in [
        DEV.replace("26 17", "26 20"),
        DEV.replace(" / /dev ", " /subtree /dev "),
        DEV.replace("tmpfs tmpfs", "tmpfs unknown"),
        DEV.replace("tmpfs rw,", "tmpfs ro,"),
    ] {
        let bad = format!("{PROC_MOUNT_ROWS}{dev}{MASK}");
        assert!(check(bad.as_bytes()).is_err(), "{dev}");
    }
}
