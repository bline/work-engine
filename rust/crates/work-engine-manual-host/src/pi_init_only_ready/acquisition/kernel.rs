//! Retain the *original* pidfd and cgroup-v2 directory. No lookup by current
//! PID/path on the terminal side. A removed cgroup that can no longer be
//! sampled is unknown, not empty.

use super::{Effective, TaskKey, boot_key};
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Component, Path, PathBuf},
};

#[derive(Serialize)]
pub(super) struct KernelCut {
    pub cgroup_events: Vec<u8>,
    pub status: Vec<u8>,
    pub apparmor: Vec<u8>,
}

pub(super) trait Anchor {
    fn check_current(&self, effective: &Effective) -> io::Result<KernelCut>;
    fn live_events(&self) -> io::Result<Vec<u8>>;
    fn end_events(&self) -> io::Result<Vec<u8>>;
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

fn check_lifetime(key: &TaskKey) -> io::Result<()> {
    let stat = bounded(File::open(proc_path(key.pid, "stat"))?, 4096)?;
    // comm may contain whitespace, ')' or newlines; the last ')' delimits
    // it. After comm, stat starts with field 3; starttime is field 22.
    let end = stat.iter().rposition(|&b| b == b')').ok_or_else(unknown)?;
    let rest = std::str::from_utf8(&stat[end + 1..]).map_err(|_| unknown())?;
    let ticks = rest
        .split_ascii_whitespace()
        .nth(19)
        .and_then(|s| s.parse::<u64>().ok());
    if ticks != Some(key.start_ticks)
        || fs::metadata(proc_path(key.pid, "ns/pid"))?.ino() != key.pid_namespace
    {
        return Err(unknown());
    }
    Ok(())
}

fn exited(fd: &OwnedFd) -> io::Result<bool> {
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

fn check_group(group: &File, key: &TaskKey) -> io::Result<()> {
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
        || stat.stx_mnt_id != key.subtree_mount_id
        || stat.stx_ino != key.subtree_inode
    {
        return Err(unknown());
    }
    Ok(())
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
        let path = Path::new("/sys/fs/cgroup").join(&member);
        if fs::canonicalize(&path)? != path {
            return Err(unknown());
        }
        let group = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(path)?;
        check_group(&group, key)?;
        let task = Self {
            key: key.clone(),
            pid,
            group,
            member,
        };
        task.live_events()?;
        Ok(task)
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

impl Anchor for KernelTask {
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
        Ok(KernelCut {
            cgroup_events: self.live_events()?,
            status,
            apparmor,
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
