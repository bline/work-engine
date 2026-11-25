//! Kernel observation of the original process's materialized startup. This
//! corroborates, but does not replace, selected OCI and local-image custody.

use super::super::super::{ElfClosure, FilePin, ResolvedLaunch};
use super::{KernelTask, MaskNode, bounded, node, proc_path};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::Read,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    time::Instant,
};

use super::super::{Result, refuse, within};

const MAX_OBJECT: u64 = 64 * 1024 * 1024;
const MAX_PASSWD: u64 = 16 * 1024;

type Object = (u32, u32, u64);

#[derive(Serialize)]
pub(in super::super) struct StartupCut {
    cmdline: Vec<u8>,
    environ: Vec<u8>,
    home: HomeCut,
    root: MaskNode,
    cwd: MaskNode,
    executable: MaskNode,
    maps: Vec<u8>,
    objects: Vec<StartupObject>,
}

#[derive(Serialize)]
pub(in super::super) struct MaterializedRoot {
    resolved_oci_path: PathBuf,
    host: MaskNode,
    task: MaskNode,
}

#[derive(Serialize)]
struct StartupObject {
    path: String,
    node: MaskNode,
    sha256: String,
}

fn identity(node: &MaskNode) -> Object {
    (node.device.0, node.device.1, node.inode)
}

fn exec_maps(text: &str) -> Option<BTreeSet<Object>> {
    let mut executable = BTreeSet::new();
    for line in text.lines() {
        let mut fields = line.split_ascii_whitespace();
        fields.next()?; // address range
        let access = fields.next()?.as_bytes();
        fields.next()?; // offset
        let (major, minor) = fields.next()?.split_once(':')?;
        let obj = (
            u32::from_str_radix(major, 16).ok()?,
            u32::from_str_radix(minor, 16).ok()?,
            fields.next()?.parse::<u64>().ok()?,
        );
        let label = fields.next();
        if access.len() != 4 || !matches!(access[2], b'x' | b'-') {
            return None;
        }
        if access[2] == b'x' {
            if access[1] != b'-' {
                return None;
            }
            if obj.2 != 0 && (obj.0, obj.1) != (0, 0) && label?.starts_with('/') {
                executable.insert(obj);
            } else if obj != (0, 0, 0)
                || !matches!(label, Some("[vdso]" | "[vsyscall]"))
                || fields.next().is_some()
            {
                return None;
            }
        }
    }
    (!executable.is_empty() && text.ends_with('\n')).then_some(executable)
}

fn words(values: &[String]) -> Vec<u8> {
    values.iter().flat_map(|s| s.bytes().chain([0])).collect()
}

#[derive(Serialize)]
struct HomeCut {
    uid: u32,
    value: String,
    // None means a rooted ENOENT, not permission/read/parse failure.
    passwd: Option<(MaskNode, Vec<u8>)>,
}

fn selected_home(passwd: &[u8], uid: u32) -> Option<&str> {
    // A strict, unambiguous subset of the pinned moby/sys/user parser. Do
    // not treat malformed data (which runc may tolerate) as proven absence.
    let mut home = None;
    for line in std::str::from_utf8(passwd).ok()?.split('\n') {
        if line.is_empty() {
            continue;
        }
        if line.trim() != line || line.chars().any(char::is_control) {
            return None;
        }
        let fields: Vec<_> = line.split(':').collect();
        let [name, _, account, group, _, value, _] = fields.as_slice() else {
            return None;
        };
        let account_id: u32 = account.parse().ok()?;
        let group_id: u32 = group.parse().ok()?;
        if name.is_empty() || *account != account_id.to_string() || *group != group_id.to_string() {
            return None;
        }
        if account_id == uid && home.replace(*value).is_some() {
            return None;
        }
    }
    Some(home.unwrap_or("/"))
}

fn stock_home(root: &File, root_id: u64, uid: u32) -> Result<HomeCut> {
    use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};
    if root_id == 0 || node(root)?.mount_id != root_id {
        return refuse("passwd lookup crossed the original task root mount");
    }
    let opened = openat2(
        root,
        "/etc/passwd",
        OFlags::PATH | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::IN_ROOT | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_XDEV,
    );
    let file = match opened {
        Ok(fd) => File::from(fd),
        Err(rustix::io::Errno::NOENT) => {
            return Ok(HomeCut {
                uid,
                value: "/".into(),
                passwd: None,
            });
        }
        Err(error) => return Err(std::io::Error::from(error).into()),
    };
    let account = node(&file)?;
    let before = file.metadata()?;
    if account.mount_id != root_id || !before.is_file() || before.len() > MAX_PASSWD {
        return refuse("passwd is not a bounded regular file on the original task root mount");
    }
    let input = File::open(format!("/proc/self/fd/{}", file.as_raw_fd()))?;
    if node(&input)? != account {
        return refuse("passwd changed while opening the retained file");
    }
    let raw = bounded(input, MAX_PASSWD)?;
    let stamp = |m: &std::fs::Metadata| {
        (
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if raw.len() as u64 != before.len() || stamp(&before) != stamp(&file.metadata()?) {
        return refuse("passwd changed during the original-root read");
    }
    let Some(value) = selected_home(&raw, uid).map(str::to_owned) else {
        return refuse("original-root passwd does not establish an unambiguous HOME");
    };
    Ok(HomeCut {
        uid,
        value,
        passwd: Some((account, raw)),
    })
}

fn matches_stock_exec_env(raw: &[u8], approved_oci: &[String], home: &HomeCut) -> bool {
    // The caller has already verified the exact two-entry OCI environment.
    // Pinned runc preserves it in order and appends exactly one HOME value.
    let mut expected = words(approved_oci);
    expected.extend_from_slice(b"HOME=");
    expected.extend_from_slice(home.value.as_bytes());
    expected.push(0);
    raw == expected
}

fn pinned_object(
    root: &File,
    root_id: u64,
    pin: &FilePin,
    deadline: Instant,
) -> Result<StartupObject> {
    within(deadline)?;
    use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};
    let file = File::from(
        openat2(
            root,
            pin.path.as_str(),
            OFlags::PATH | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::IN_ROOT | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_XDEV,
        )
        .map_err(std::io::Error::from)?,
    );
    let n = node(&file)?;
    let before = file.metadata()?;
    // Never open a device/FIFO for I/O. Reopen only this known regular inode,
    // not the pathname in the task again. The root mount must be read-only.
    if n.mount_id != root_id || !before.is_file() || before.len() == 0 || before.len() > MAX_OBJECT
    {
        return refuse(
            "selected startup object is not a bounded regular file on the original root",
        );
    }
    let input = File::open(format!("/proc/self/fd/{}", file.as_raw_fd()))?;
    if node(&input)? != n {
        return refuse("startup object changed while opening the retained file");
    }
    let mut input = input.take(before.len() + 1);
    let mut buf = [0; 65536];
    let mut digest = Sha256::new();
    let mut bytes = 0;
    loop {
        within(deadline)?;
        let size = input.read(&mut buf)?;
        if size == 0 {
            break;
        }
        bytes += size as u64;
        digest.update(&buf[..size]);
    }
    let after = file.metadata()?;
    let sha256 = format!("{:x}", digest.finalize());
    if bytes != before.len()
        || sha256 != pin.sha256
        || (
            before.len(),
            before.mtime(),
            before.mtime_nsec(),
            before.ctime(),
            before.ctime_nsec(),
        ) != (
            after.len(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec(),
        )
    {
        return refuse("startup bytes do not match the stable approved file pin");
    }
    Ok(StartupObject {
        path: pin.path.clone(),
        node: n,
        sha256,
    })
}

fn root_backing_matches(host: &MaskNode, task: &MaskNode) -> bool {
    // runc recursively bind-mounts the same backing before pivot, then
    // remounts / read-only. Mount IDs differ; backing dev+inode must not.
    identity(host) == identity(task) && host.inode > 0 && host.mount_id > 0 && task.mount_id > 0
}

impl KernelTask {
    pub(in super::super) fn stock_materialized_root(
        &self,
        path: &Path,
        startup: &StartupCut,
        deadline: Instant,
    ) -> Result<MaterializedRoot> {
        use super::Anchor;
        self.live_events()?;
        if !path.is_absolute()
            || path == Path::new("/")
            || fs::canonicalize(path)?.as_os_str() != path.as_os_str()
        {
            return refuse("original OCI root is not a canonical absolute stock backing");
        }
        for dir in path.ancestors() {
            within(deadline)?;
            let m = fs::symlink_metadata(dir)?;
            if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
                return refuse("original OCI root has unprotected ancestry");
            }
        }
        let open = |dir: &Path, nofollow| {
            OpenOptions::new()
                .read(true)
                .custom_flags(
                    libc::O_PATH | libc::O_DIRECTORY | if nofollow { libc::O_NOFOLLOW } else { 0 },
                )
                .open(dir)
        };
        let host = node(&open(path, true)?)?;
        let task_path = proc_path(self.key.pid, "root");
        let task_dir = open(&task_path, false)?;
        let task = node(&task_dir)?;
        let flags = rustix::fs::fstatvfs(&task_dir)
            .map_err(std::io::Error::from)?
            .f_flag;
        if task != startup.root
            || !root_backing_matches(&host, &task)
            || !flags.contains(rustix::fs::StatVfsMountFlags::RDONLY)
            || host != node(&open(path, true)?)?
            || task != node(&open(&task_path, false)?)?
        {
            return refuse("live read-only root no longer matches its original OCI backing");
        }
        self.live_events()?;
        within(deadline)?;
        Ok(MaterializedRoot {
            resolved_oci_path: path.into(),
            host,
            task,
        })
    }

    pub(in super::super) fn stock_startup(
        &self,
        launch: &ResolvedLaunch,
        uid: u32,
        deadline: Instant,
    ) -> Result<StartupCut> {
        use super::Anchor;
        self.live_events()?;
        if self.key != launch.task || launch.cwd != "/" {
            return refuse("startup readback crossed the original task or working directory");
        }
        let p = self.key.pid;
        let link = |name, directory| {
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_PATH | if directory { libc::O_DIRECTORY } else { 0 })
                .open(proc_path(p, name))
        };
        let dir = link("root", true)?;
        let root = node(&dir)?;
        let cwd = node(&link("cwd", true)?)?;
        let executable = node(&link("exe", false)?)?;
        let fs = rustix::fs::fstatvfs(&dir).map_err(std::io::Error::from)?;
        if root.mount_id == 0
            || root.mount_id != launch.root_mount_id
            || cwd != root
            || !fs.f_flag.contains(rustix::fs::StatVfsMountFlags::RDONLY)
        {
            return refuse("materialized task root is not the selected read-only root and cwd");
        }
        let home = stock_home(&dir, root.mount_id, uid)?;
        let mut pins = vec![&launch.image.init];
        if let ElfClosure::Dynamic {
            interpreter,
            dependencies,
        } = &launch.image.elf
        {
            pins.push(interpreter);
            pins.extend(dependencies);
        }
        let objects = pins
            .iter()
            .map(|pin| pinned_object(&dir, root.mount_id, pin, deadline))
            .collect::<Result<Vec<_>>>()?;
        let expected = objects
            .iter()
            .map(|obj| identity(&obj.node))
            .collect::<BTreeSet<_>>();
        let read = |name, max| bounded(File::open(proc_path(p, name))?, max);
        let cmdline = read("cmdline", 4096)?;
        let environ = read("environ", 4096)?;
        let maps = read("maps", 64 * 1024)?;
        let actual = std::str::from_utf8(&maps).ok().and_then(exec_maps);
        if executable != objects[0].node
            || expected.len() != objects.len()
            || actual.as_ref() != Some(&expected)
            || cmdline != words(&launch.argv)
            || !matches_stock_exec_env(&environ, &launch.env, &home)
            || root != node(&link("root", true)?)?
            || cwd != node(&link("cwd", true)?)?
            || executable != node(&link("exe", false)?)?
        {
            return refuse(
                "kernel executable, mappings, cwd or startup arguments/environment did not join",
            );
        }
        self.live_events()?;
        within(deadline)?;
        Ok(StartupCut {
            cmdline,
            environ,
            home,
            root,
            cwd,
            executable,
            maps,
            objects,
        })
    }
}

#[test]
fn stock_root_backing_checks_object_not_mount_alias() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let host = node(&File::open(first.path()).unwrap()).unwrap();
    let mut task = node(&File::open(first.path()).unwrap()).unwrap();
    task.mount_id += 1;
    assert!(root_backing_matches(&host, &task));
    let other = node(&File::open(second.path()).unwrap()).unwrap();
    assert!(!root_backing_matches(&host, &other));
    task.mount_id = 0;
    assert!(!root_backing_matches(&host, &task));
}

#[test]
fn stock_startup_exec_env_requires_only_the_rooted_home_append() {
    let fixture = tempfile::tempdir().unwrap();
    let root = File::open(fixture.path()).unwrap();
    let id = node(&root).unwrap().mount_id;
    let etc = fixture.path().join("etc");
    let passwd = etc.join("passwd");
    std::fs::create_dir(etc).unwrap();
    let oci = ["PATH=/bin".into(), "HOSTNAME=original".into()];
    let uid = 65534;
    let absent = stock_home(&root, id, uid).unwrap();
    assert!(absent.passwd.is_none());
    assert!(matches_stock_exec_env(
        b"PATH=/bin\0HOSTNAME=original\0HOME=/\0",
        &oci,
        &absent
    ));
    std::fs::write(&passwd, "root:x:0:0:root:/root:/bin/sh\n").unwrap();
    let unmatched = stock_home(&root, id, uid).unwrap();
    assert!(unmatched.passwd.is_some());
    assert_eq!(unmatched.value, "/");
    std::fs::write(
        &passwd,
        "root:x:0:0:root:/root:/bin/sh\nnobody:x:65534:65534::/nonexistent:/bin/false\n",
    )
    .unwrap();
    let home = stock_home(&root, id, uid).unwrap();
    assert_ne!(home.value, stock_home(&root, id, 0).unwrap().value);
    assert!(matches_stock_exec_env(
        b"PATH=/bin\0HOSTNAME=original\0HOME=/nonexistent\0",
        &oci,
        &home
    ));
    for alternate in [
        &b"PATH=/bin\0HOSTNAME=original\0"[..],
        b"PATH=/bin\0HOSTNAME=original\0HOME=/\0",
        b"PATH=/bin\0HOSTNAME=original\0HOME=/nonexistent\0HOME=/nonexistent\0",
        b"PATH=/bin\0HOSTNAME=original\0HOME=/nonexistent\0EXTRA=1\0",
        b"PATH=/bin\0HOSTNAME=original\0HOME=/nonexistent\0LISTEN_PID=1\0",
        b"PATH=/bin\0HOSTNAME=original\0NOTIFY_SOCKET=/run/notify/socket\0HOME=/nonexistent\0",
        b"PATH=/usr/bin\0HOSTNAME=original\0HOME=/nonexistent\0",
        b"HOSTNAME=original\0PATH=/bin\0HOME=/nonexistent\0",
    ] {
        assert!(!matches_stock_exec_env(alternate, &oci, &home));
    }
    assert!(stock_home(&root, id + 1, uid).is_err());
    for ambiguous in [
        "one:x:65534:1::/a:/bin/false\ntwo:x:65534:1::/b:/bin/false\n",
        "nobody:x:65534:65534\n",
        "one:x:065534:1::/a:/bin/false\n",
        "nobody:x:65534:65534::/a\0b:/bin/false\n",
    ] {
        std::fs::write(&passwd, ambiguous).unwrap();
        assert!(stock_home(&root, id, uid).is_err());
    }
    std::fs::remove_file(&passwd).unwrap();
    std::os::unix::fs::symlink("/does-not-exist", &passwd).unwrap();
    assert!(stock_home(&root, id, uid).is_err());
}

#[test]
fn stock_startup_hashes_the_opened_root_object_and_refuses_substitution() {
    let fixture = tempfile::tempdir().unwrap();
    let root = File::open(fixture.path()).unwrap();
    let root_id = node(&root).unwrap().mount_id;
    let deadline = Instant::now() + std::time::Duration::from_secs(5);
    let path = fixture.path().join("init");
    std::fs::write(&path, b"approved bytes").unwrap();
    let pin = FilePin {
        path: "/init".into(),
        sha256: format!("{:x}", Sha256::digest(b"approved bytes")),
    };
    assert_eq!(
        pinned_object(&root, root_id, &pin, deadline)
            .unwrap()
            .sha256,
        pin.sha256
    );
    std::fs::write(&path, b"changed bytes!").unwrap();
    assert!(pinned_object(&root, root_id, &pin, deadline).is_err());
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink("/proc/self/exe", &path).unwrap();
    assert!(pinned_object(&root, root_id, &pin, deadline).is_err());
}

#[test]
fn stock_startup_requires_closed_file_backed_executable_mappings() {
    let file = "400000-401000 r-xp 00000000 08:02 10 /work-engine-b-init-only\n";
    let data = "500000-501000 rw-p 00000000 00:00 0 [heap]\n";
    let vdso = "800000-801000 r-xp 00000000 00:00 0 [vdso]\n";
    let lib = "600000-601000 r-xp 00001000 00:2a 13 /lib/libc.so.6\n";
    assert_eq!(
        exec_maps(&[file, data, vdso, lib].concat()),
        Some(BTreeSet::from([(8, 2, 10), (0, 42, 13)]))
    );
    for alternate in [
        lib.replace("r-xp", "rwxp"),
        vdso.replace("[vdso]", "[heap]"),
        vdso.replace(" [vdso]", ""),
        lib.replace("/lib/libc.so.6\n", ""),
    ] {
        assert_eq!(exec_maps(&[file, &alternate].concat()), None);
    }
}
