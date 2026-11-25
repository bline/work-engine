//! Complete bounded mount readback for the unselected natural-exit route.

use super::{
    Anchor, KernelCut, KernelTask, MaskNode, MountRow, bounded, mask_node, node, proc_path, unknown,
};
use crate::pi_docker::init_only_ready::{
    Effective, FILE_MASKS, GENERATED_FILES, MountKind, MountView, PROC_READONLY, ProcfsMount,
    ResolvedLaunch, STOCK_VIRTUAL, stock_directory_masks,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::Path,
    time::Instant,
};

#[derive(Serialize)]
pub(in super::super) struct StockMountCut {
    possible_cpus: String,
    root: MaskNode,
    // None requires an exact rooted ENOENT for an optional static target.
    directory_masks: Vec<(String, Option<MaskNode>)>,
    generated_files: Vec<(String, MaskNode)>,
    pub(in super::super) mounts: Vec<MountView>,
}

#[derive(Serialize)]
pub(in super::super) struct DeviceCut {
    nodes: Vec<(&'static str, MaskNode)>,
    symlinks: Vec<(&'static str, String)>,
}

impl DeviceCut {
    pub(in super::super) fn devices(&self) -> Vec<(u32, u32)> {
        let mut devices: Vec<_> = self.nodes.iter().map(|(_, n)| n.rdev).collect();
        devices.sort_unstable();
        devices
    }
}

const DEVICE_NODES: [(&str, (u32, u32)); 7] = [
    ("/dev/full", (1, 7)),
    ("/dev/null", (1, 3)),
    ("/dev/random", (1, 8)),
    ("/dev/tty", (5, 0)),
    ("/dev/urandom", (1, 9)),
    ("/dev/zero", (1, 5)),
    ("/dev/pts/ptmx", (5, 2)),
];
const DEVICE_LINKS: [(&str, &str); 6] = [
    ("/dev/fd", "/proc/self/fd"),
    ("/dev/stdin", "/proc/self/fd/0"),
    ("/dev/stdout", "/proc/self/fd/1"),
    ("/dev/stderr", "/proc/self/fd/2"),
    ("/dev/ptmx", "pts/ptmx"),
    ("/dev/core", "/proc/kcore"),
];

fn device_entries(root: &File, dir: &str, limit: usize) -> io::Result<BTreeSet<String>> {
    let path = format!(
        "/proc/self/fd/{}/{}",
        root.as_raw_fd(),
        dir.strip_prefix('/').ok_or_else(unknown)?
    );
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(path)? {
        if names.len() >= limit
            || !names.insert(entry?.file_name().into_string().map_err(|_| unknown())?)
        {
            return Err(unknown());
        }
    }
    Ok(names)
}

fn flags(text: &str) -> io::Result<(BTreeSet<&str>, bool)> {
    let mut set = BTreeSet::new();
    if !text.split(',').all(|f| !f.is_empty() && set.insert(f))
        || set.contains("ro") == set.contains("rw")
        || [("nosuid", "suid"), ("nodev", "dev"), ("noexec", "exec")]
            .iter()
            .any(|(a, b)| set.contains(a) && set.contains(b))
    {
        return Err(unknown());
    }
    let ro = set.contains("ro");
    Ok((set, ro))
}

fn project(
    raw: &[u8],
    possible_cpus: String,
    masked: &[String],
    mnt_ns: u64,
    root: MaskNode,
    mut observe: impl FnMut(&str) -> io::Result<Option<MaskNode>>,
) -> io::Result<StockMountCut> {
    let cpus = super::super::retirement::possible_cpus(&possible_cpus).ok_or_else(unknown)?;
    let dirs = stock_directory_masks(masked).ok_or_else(unknown)?;
    if !dirs
        .iter()
        .all(|(_, cpu)| cpu.is_none_or(|n| cpus.binary_search(&n).is_ok()))
    {
        return Err(unknown());
    }
    // Each allowed family contributes exactly one slot per distinct selected
    // destination. A missing optional row does not create space for other rows.
    let capacity = STOCK_VIRTUAL.len() + PROC_READONLY.len() + GENERATED_FILES.len() + masked.len();
    let text = std::str::from_utf8(raw).map_err(|_| unknown())?;
    if !text.ends_with('\n') || root.mode & libc::S_IFMT != libc::S_IFDIR || root.mount_id == 0 {
        return Err(unknown());
    }
    let mut paths = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut all = Vec::new();
    for line in text.lines() {
        let row = MountRow::parse(line)?;
        if all.len() > capacity
            || row.id == 0
            || !ids.insert(row.id)
            || !paths.insert(row.path)
            || row.device().is_none()
            || row.parent.parse::<u64>().is_err()
        {
            return Err(unknown());
        }
        all.push(row);
    }
    let find = |path: &str| all.iter().find(|r| r.path == path).ok_or_else(unknown);
    let root_row = find("/")?;
    if root_row.id != root.mount_id
        || root_row.device() != Some(root.device)
        || root_row.subroot != "/"
    {
        return Err(unknown());
    }
    let proc = find("/proc")?;
    let sys = find("/sys")?;
    if sys.device() == proc.device() || sys.device() == root_row.device() {
        return Err(unknown());
    }
    if !STOCK_VIRTUAL
        .iter()
        .chain(&PROC_READONLY)
        .chain(&GENERATED_FILES)
        .all(|p| paths.contains(p))
    {
        return Err(unknown());
    }

    let mut mounts = Vec::with_capacity(all.len() - 1);
    let mut generated_files = Vec::new();
    for row in &all {
        let (access, ro) = flags(row.flags)?;
        let mut tail = row.tail.split_ascii_whitespace();
        let (fstype, source) = (
            tail.next().ok_or_else(unknown)?,
            tail.next().ok_or_else(unknown)?,
        );
        let (superopts, super_ro) = flags(tail.next().ok_or_else(unknown)?)?;
        if tail.next().is_some() || (!ro && super_ro) {
            return Err(unknown());
        }
        let parent = row.parent.parse().map_err(|_| unknown())?;
        let restricted = ["nosuid", "nodev", "noexec"]
            .iter()
            .all(|f| access.contains(f));
        let kind = match row.path {
            "/" if ro => continue,
            "/proc"
                if parent == root.mount_id
                    && row.subroot == "/"
                    && !ro
                    && restricted
                    && (fstype, source) == ("proc", "proc") =>
            {
                MountKind::Procfs(ProcfsMount {
                    namespace: mnt_ns,
                    id: row.id,
                    parent,
                    device: row.device().ok_or_else(unknown)?,
                    subroot: row.subroot.into(),
                    nosuid: true,
                    nodev: true,
                    noexec: true,
                })
            }
            p if PROC_READONLY.contains(&p)
                && parent == proc.id
                && ro
                && restricted
                && (fstype, source) == ("proc", "proc")
                && row.device() == proc.device()
                && p.strip_prefix("/proc") == Some(row.subroot) =>
            {
                MountKind::Procfs(ProcfsMount {
                    namespace: mnt_ns,
                    id: row.id,
                    parent,
                    device: row.device().ok_or_else(unknown)?,
                    subroot: row.subroot.into(),
                    nosuid: true,
                    nodev: true,
                    noexec: true,
                })
            }
            "/sys"
                if parent == root.mount_id
                    && row.subroot == "/"
                    && ro
                    && restricted
                    && (fstype, source) == ("sysfs", "sysfs") =>
            {
                let n = observe(row.path)?.ok_or_else(unknown)?;
                if !directory_on(row, n) {
                    return Err(unknown());
                }
                MountKind::PrivateVirtual
            }
            "/sys/fs/cgroup" if parent == sys.id && ro && restricted && fstype == "cgroup2" => {
                MountKind::PrivateVirtual
            }
            "/dev"
                if parent == root.mount_id
                    && row.subroot == "/"
                    && (fstype, source) == ("tmpfs", "tmpfs") =>
            {
                MountKind::PrivateVirtual
            }
            "/dev/shm" | "/dev/pts" | "/dev/mqueue"
                if parent == find("/dev")?.id
                    && row.subroot == "/"
                    && fstype
                        == match row.path {
                            "/dev/shm" => "tmpfs",
                            "/dev/pts" => "devpts",
                            _ => "mqueue",
                        } =>
            {
                MountKind::PrivateVirtual
            }
            p if GENERATED_FILES.contains(&p) && parent == root.mount_id && ro => {
                let n = observe(p)?.ok_or_else(unknown)?;
                if n.mode & libc::S_IFMT != libc::S_IFREG
                    || n.mount_id != row.id
                    || n.inode == 0
                    || row.device() != Some(n.device)
                {
                    return Err(unknown());
                }
                generated_files.push((p.to_string(), n));
                MountKind::DaemonGenerated
            }
            p if FILE_MASKS.contains(&p) => MountKind::RuntimeMaskedFile, // checked below with original /dev/null
            p if dirs.iter().any(|(d, _)| *d == p)
                && parent
                    == if p.starts_with("/sys/") {
                        sys.id
                    } else {
                        proc.id
                    }
                && ro
                && super_ro
                && row.subroot == "/"
                && (fstype, source) == ("tmpfs", "tmpfs")
                && superopts.contains("nr_inodes=1")
                && superopts.contains("size=4k")
                && row.device() != sys.device()
                && row.device() != proc.device()
                && row.device() != find("/dev")?.device()
                && row.device() != root_row.device() =>
            {
                // Pinned x86_64 runc creates a one-page, one-inode tmpfs; the
                // shared bind keeps that backing and MNT_READONLY. runc does
                // not set noexec/nodev/nosuid on the mask itself. The original
                // parent is restricted and the empty RO mask has no entry to
                // execute, setid, open as a device, or write.
                MountKind::StockMaskedDirectory
            }
            _ => return Err(unknown()),
        };
        mounts.push(MountView {
            path: row.path.into(),
            kind,
            read_only: ro,
        });
    }
    let mut directory_masks = Vec::with_capacity(dirs.len());
    let mut backing_roots = BTreeMap::new();
    for (path, cpu) in dirs {
        let n = observe(path)?;
        match (find(path).ok(), n) {
            (Some(row), Some(n)) if directory_on(row, n) => {
                if backing_roots
                    .insert(n.device, n.inode)
                    .is_some_and(|prior| prior != n.inode)
                {
                    return Err(unknown());
                }
            }
            (None, None) if cpu.is_none() => (),
            _ => return Err(unknown()),
        }
        directory_masks.push((path.to_string(), n));
    }
    mounts.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    let masks = super::check_proc_readonly(raw, &mounts, mnt_ns, root.mount_id)?;
    super::verify_null_masks(&masks, root.mount_id, observe)?;
    Ok(StockMountCut {
        possible_cpus,
        root,
        directory_masks,
        generated_files,
        mounts,
    })
}

fn directory_on(row: &MountRow<'_>, n: MaskNode) -> bool {
    n.mode & libc::S_IFMT == libc::S_IFDIR
        && n.mount_id == row.id
        && n.inode != 0
        && row.device() == Some(n.device)
}

impl KernelTask {
    pub(in super::super) fn check_stock_current(
        &self,
        launch: &ResolvedLaunch,
        effective: &Effective,
        deadline: Instant,
    ) -> crate::Result<KernelCut> {
        super::super::within(deadline)?;
        if launch.task != self.key
            || launch.runtime != "runc"
            || launch.root_mount_id != effective.root_mount_id
            || launch.proc_readonly_paths != PROC_READONLY
            || launch.stock_directory_masks().is_none()
        {
            return Err(unknown().into());
        }
        let mut checked = self.check_current(effective)?;
        let procfs = checked.procfs.as_mut().ok_or_else(unknown)?;
        let (mountinfo, stock) =
            self.observe_stock_mounts(launch, procfs.mount_namespace, deadline)?;
        let inventory = effective.inventory.as_ref().ok_or_else(unknown)?;
        if stock.mounts != inventory.mounts
            || !inventory.closed(launch, procfs.mount_namespace, true)
            || procfs.mountinfo != mountinfo
        {
            return Err(unknown().into());
        }
        self.live_events()?;
        super::super::within(deadline)?;
        procfs.stock = Some(stock);
        Ok(checked)
    }

    pub(in super::super) fn observe_stock_mounts(
        &self,
        launch: &ResolvedLaunch,
        mount_namespace: u64,
        deadline: Instant,
    ) -> crate::Result<(Vec<u8>, StockMountCut)> {
        use std::{fs, os::unix::fs::MetadataExt};
        self.live_events()?;
        super::super::within(deadline)?;
        let p = self.key.pid;
        let namespace = || Ok::<_, io::Error>(fs::metadata(proc_path(p, "ns/mnt"))?.ino());
        let read_mounts = || bounded(File::open(proc_path(p, "mountinfo"))?, 64 * 1024);
        let read_cpus = || bounded(File::open("/sys/devices/system/cpu/possible")?, 512);
        let cpus = String::from_utf8(read_cpus()?).map_err(|_| unknown())?;
        let root = self.stock_root()?;
        let root_node = node(&root)?;
        if self.key != launch.task
            || root_node.mount_id != launch.root_mount_id
            || mount_namespace == 0
            || mount_namespace != namespace()?
        {
            return Err(unknown().into());
        }
        let raw = read_mounts()?;
        let stock = project(
            &raw,
            cpus,
            &launch.masked_paths,
            mount_namespace,
            root_node,
            |path| mask_node(&root, path),
        )?;
        if raw != read_mounts()?
            || stock.possible_cpus.as_bytes() != read_cpus()?
            || mount_namespace != namespace()?
            || root_node != node(&self.stock_root()?)?
        {
            return Err(unknown().into());
        }
        self.live_events()?;
        super::super::within(deadline)?;
        Ok((raw, stock))
    }

    pub(in super::super) fn stock_root_mount_id(&self) -> io::Result<u64> {
        self.live_events()?;
        let id = node(&self.stock_root()?)?.mount_id;
        self.live_events()?;
        if id == 0 {
            return Err(unknown());
        }
        Ok(id)
    }

    pub(in super::super) fn stock_devices(
        &self,
        mountinfo: &[u8],
        deadline: Instant,
    ) -> crate::Result<DeviceCut> {
        self.live_events()?;
        super::super::within(deadline)?;
        let root = self.stock_root()?;
        let root_node = node(&root)?;
        let mut inventory = BTreeMap::new();
        for line in std::str::from_utf8(mountinfo)
            .map_err(|_| unknown())?
            .lines()
        {
            let row = MountRow::parse(line)?;
            if inventory.insert(row.path, row).is_some() {
                return Err(unknown().into());
            }
        }
        let selected = |path: &str| -> io::Result<MaskNode> {
            let row = inventory.get(path).ok_or_else(unknown)?;
            let n = mask_node(&root, path)?.ok_or_else(unknown)?;
            if !directory_on(row, n) {
                return Err(unknown());
            }
            Ok(n)
        };
        let dev = selected("/dev")?;
        let pts = selected("/dev/pts")?;
        selected("/dev/shm")?;
        selected("/dev/mqueue")?;
        let mut names = device_entries(&root, "/dev", 15)?;
        let mut nodes = Vec::new();
        for (path, rdev) in DEVICE_NODES {
            let parent = if path.starts_with("/dev/pts/") {
                pts
            } else {
                dev
            };
            let n = mask_node(&root, path)?.ok_or_else(unknown)?;
            if n.mount_id != parent.mount_id
                || n.device != parent.device
                || n.inode == 0
                || n.mode & libc::S_IFMT != libc::S_IFCHR
                || n.rdev != rdev
            {
                return Err(unknown().into());
            }
            if parent == dev && !names.remove(path.strip_prefix("/dev/").ok_or_else(unknown)?) {
                return Err(unknown().into());
            }
            nodes.push((path, n));
        }
        let mut symlinks = Vec::new();
        let root_link = format!("/proc/self/fd/{}", root.as_raw_fd());
        for (path, target) in DEVICE_LINKS {
            let name = path.strip_prefix("/dev/").ok_or_else(unknown)?;
            if !names.remove(name) && name == "core" {
                continue;
            }
            let observed = fs::read_link(Path::new(&root_link).join(&path[1..]))?;
            if observed != Path::new(target) {
                return Err(unknown().into());
            }
            // This link may only resolve to the separately proven null bind
            // or to an absent optional target, never to the host proc file.
            if name == "core" && mask_node(&root, target)?.is_some_and(|k| k.rdev != (1, 3)) {
                return Err(unknown().into());
            }
            symlinks.push((path, target.to_owned()));
        }
        if names
            != ["mqueue", "pts", "shm"]
                .map(str::to_owned)
                .into_iter()
                .collect()
            || device_entries(&root, "/dev/pts", 1)? != [String::from("ptmx")].into_iter().collect()
            || !device_entries(&root, "/dev/shm", 0)?.is_empty()
            || !device_entries(&root, "/dev/mqueue", 0)?.is_empty()
            || root_node != node(&self.stock_root()?)?
        {
            return Err(unknown().into());
        }
        self.live_events()?;
        super::super::within(deadline)?;
        Ok(DeviceCut { nodes, symlinks })
    }

    fn stock_root(&self) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_DIRECTORY)
            .open(proc_path(self.key.pid, "root"))
    }
}

#[cfg(test)]
#[path = "stock_mounts_tests.rs"]
mod tests;
