//! Pinned in-sandbox boundary witness. The host waits for its checked READY
//! line before writing one prompt. The guard is not a provider or authority.
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::process::{Command, Stdio};

const ACTIVE_LIMIT: u64 = 16 * 1024 * 1024;
const RESULT_LIMIT: u64 = 4 * 1024 * 1024;
const MAX_FILES: usize = 4096;
const MAGIC: &[u8] = b"WE-RP1-SESSION-V2\0";

fn bounded_tmpfs(mount: &str, limit: u64) -> bool {
    let Ok(path) = CString::new(mount) else {
        return false;
    };
    let mut info = std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::statfs(path.as_ptr(), info.as_mut_ptr()) } != 0 {
        return false;
    }
    let info = unsafe { info.assume_init() };
    info.f_type == libc::TMPFS_MAGIC as _
        && info.f_blocks.saturating_mul(info.f_bsize as u64) <= limit
}

fn export_session(result: &[u8]) -> std::io::Result<()> {
    let base = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/session")?;
    let mut stack = vec![String::new()];
    let mut files = Vec::new();
    let mut directories = Vec::new();
    let mut entries = 0usize;
    while let Some(dir) = stack.pop() {
        let directory = open_beneath(&base, &dir, true)?;
        let identity = FileIdentity::of(&directory.metadata()?);
        for name in directory_names(directory)? {
            let path = if dir.is_empty() {
                name
            } else {
                format!("{dir}/{name}")
            };
            entries += 1;
            if entries > MAX_FILES {
                return Err(std::io::Error::other("session entry limit"));
            }
            if path.len() > 4096 || path.split('/').count() > 16 {
                return Err(std::io::Error::other("session path invalid"));
            }
            let entry = open_beneath(&base, &path, false)?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() && meta.nlink() == 1 {
                files.push((path, FileIdentity::of(&meta)));
            } else {
                return Err(std::io::Error::other("unsafe session entry"));
            }
        }
        directories.push((dir, identity));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut total = 0u64;
    let mut exported = Vec::with_capacity(files.len());
    for (name, expected) in files {
        let mut file = open_regular_beneath(&base, &name)?;
        let before = FileIdentity::of(&file.metadata()?);
        if before != expected || !before.regular || before.links != 1 {
            return Err(std::io::Error::other("session file replaced"));
        }
        total = total
            .checked_add(before.length)
            .ok_or_else(|| std::io::Error::other("session size overflow"))?;
        if total > ACTIVE_LIMIT {
            return Err(std::io::Error::other("session size limit"));
        }
        let remaining = ACTIVE_LIMIT - (total - before.length);
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(remaining + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 != before.length
            || FileIdentity::of(&file.metadata()?) != before
            || FileIdentity::of(&open_beneath(&base, &name, false)?.metadata()?) != before
        {
            return Err(std::io::Error::other("session file changed during export"));
        }
        exported.push((name, bytes));
    }
    for (name, expected) in directories {
        if FileIdentity::of(&open_beneath(&base, &name, true)?.metadata()?) != expected {
            return Err(std::io::Error::other(
                "session directory changed during export",
            ));
        }
    }
    let mut output = std::io::stdout().lock();
    output.write_all(MAGIC)?;
    output.write_all(&(result.len() as u32).to_be_bytes())?;
    output.write_all(result)?;
    output.write_all(&(exported.len() as u32).to_be_bytes())?;
    for (name, bytes) in exported {
        output.write_all(&(name.len() as u16).to_be_bytes())?;
        output.write_all(name.as_bytes())?;
        output.write_all(&(bytes.len() as u64).to_be_bytes())?;
        output.write_all(&bytes)?;
    }
    output.flush()
}

#[derive(PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    length: u64,
    links: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    regular: bool,
}

impl FileIdentity {
    fn of(meta: &fs::Metadata) -> Self {
        Self {
            device: meta.dev(),
            inode: meta.ino(),
            length: meta.len(),
            links: meta.nlink(),
            modified: (meta.mtime(), meta.mtime_nsec()),
            changed: (meta.ctime(), meta.ctime_nsec()),
            regular: meta.is_file(),
        }
    }
}

fn open_beneath(base: &File, relative: &str, directory: bool) -> std::io::Result<File> {
    if relative.is_empty() && directory {
        return base.try_clone();
    }
    let flags = if directory {
        libc::O_RDONLY | libc::O_DIRECTORY
    } else {
        // O_PATH inspects peer-created FIFOs and sockets without blocking or
        // invoking a device before their type has been rejected.
        libc::O_PATH
    };
    open_beneath_flags(base, relative, flags)
}

fn open_regular_beneath(base: &File, relative: &str) -> std::io::Result<File> {
    // An entry can be replaced after enumeration. O_NONBLOCK prevents a
    // replacement FIFO from hanging the guard before its fstat check.
    open_beneath_flags(base, relative, libc::O_RDONLY | libc::O_NONBLOCK)
}

fn open_beneath_flags(base: &File, relative: &str, flags: i32) -> std::io::Result<File> {
    if relative.is_empty()
        || relative.starts_with('/')
        || relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(std::io::Error::other("session path invalid"));
    }
    let name = CString::new(relative).map_err(|_| std::io::Error::other("session path invalid"))?;
    let mut how: libc::open_how = unsafe { std::mem::zeroed() };
    how.flags = (flags | libc::O_CLOEXEC | libc::O_NOFOLLOW) as u64;
    how.resolve = libc::RESOLVE_BENEATH
        | libc::RESOLVE_NO_SYMLINKS
        | libc::RESOLVE_NO_MAGICLINKS
        | libc::RESOLVE_NO_XDEV;
    // SAFETY: the owned root descriptor, C string, and open_how live through the syscall.
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            base.as_raw_fd(),
            name.as_ptr(),
            &how,
            std::mem::size_of::<libc::open_how>(),
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd as i32) })
}

fn directory_names(directory: File) -> std::io::Result<Vec<String>> {
    // SAFETY: fdopendir takes ownership of this descriptor on success; closedir
    // releases it. Every readdir name is copied before the next call.
    let fd = directory.into_raw_fd();
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        let error = std::io::Error::last_os_error();
        unsafe { libc::close(fd) };
        return Err(error);
    }
    let mut names = Vec::new();
    loop {
        unsafe { *libc::__errno_location() = 0 };
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            let error = std::io::Error::last_os_error();
            unsafe { libc::closedir(stream) };
            return if error.raw_os_error() == Some(0) {
                Ok(names)
            } else {
                Err(error)
            };
        }
        let name = match unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_str() {
            Ok(name) => name,
            Err(_) => {
                unsafe { libc::closedir(stream) };
                return Err(std::io::Error::other("session path encoding"));
            }
        };
        if name != "." && name != ".." {
            if names.len() >= MAX_FILES {
                unsafe { libc::closedir(stream) };
                return Err(std::io::Error::other("session entry limit"));
            }
            names.push(name.to_owned());
        }
    }
}

fn mounted(path: &str, required: &str) -> bool {
    let Ok(name) = CString::new(path) else {
        return false;
    };
    let mut info = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    if unsafe { libc::statvfs(name.as_ptr(), info.as_mut_ptr()) } != 0 {
        return false;
    }
    let readonly = unsafe { info.assume_init() }.f_flag & libc::ST_RDONLY != 0;
    (required == "ro" && readonly) || (required == "rw" && !readonly)
}

fn bounded_memory_policy(quota: &Path, expected: u64) -> bool {
    // All three files must exist in the realized private cgroup. A missing
    // swap controller, an unbounded policy, or an existing swap charge refuses
    // READY even when memory.max itself matches.
    matches!(
        (
            fs::read_to_string(quota.join("memory.max")),
            fs::read_to_string(quota.join("memory.swap.max")),
            fs::read_to_string(quota.join("memory.swap.current")),
        ),
        (Ok(memory), Ok(swap), Ok(current))
            if memory.trim().parse::<u64>().ok() == Some(expected)
                && swap.trim() == "0"
                && current.trim() == "0"
    )
}

fn bounded_active_cgroup() -> bool {
    let Ok(relative) = std::env::var("WORK_ENGINE_QUOTA_REL") else {
        return false;
    };
    let Ok(expected) = std::env::var("WORK_ENGINE_QUOTA_BYTES") else {
        return false;
    };
    let Ok(expected) = expected.parse::<u64>() else {
        return false;
    };
    let Ok(policy) = std::env::var("WORK_ENGINE_QUOTA_POLICY") else {
        return false;
    };
    let Ok(host_charge) = std::env::var("WORK_ENGINE_HOST_CUSTODY_CHARGE") else {
        return false;
    };
    let Ok(host_charge) = host_charge.parse::<u64>() else {
        return false;
    };
    let Ok(page_size) = std::env::var("WORK_ENGINE_QUOTA_PAGE_SIZE") else {
        return false;
    };
    let Ok(page_size) = page_size.parse::<u64>() else {
        return false;
    };
    if !bounded_policy_equation(&policy, host_charge, page_size, expected)
        || !relative.starts_with('/')
    {
        return false;
    }
    let Ok(membership) = fs::read_to_string("/proc/self/cgroup") else {
        return false;
    };
    if !membership
        .lines()
        .any(|line| line == format!("0::{relative}"))
    {
        return false;
    }
    if !bounded_memory_policy(Path::new("/quota"), expected) {
        return false;
    }
    let Ok(path) = CString::new("/quota") else {
        return false;
    };
    let mut info = std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::statfs(path.as_ptr(), info.as_mut_ptr()) } != 0 {
        return false;
    }
    unsafe { info.assume_init() }.f_type == libc::CGROUP2_SUPER_MAGIC as _
}

fn bounded_policy_equation(policy: &str, host_charge: u64, page_size: u64, expected: u64) -> bool {
    let combined = match policy {
        "strict_aggregate_session_64mib_v1" => 64 * 1024 * 1024,
        "session64mib_combined1g_zero_swap_v2" => 1024 * 1024 * 1024,
        _ => return false,
    };
    let actual_page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if actual_page <= 0
        || page_size != actual_page as u64
        || host_charge > 64 * 1024 * 1024
        || host_charge >= combined
    {
        return false;
    }
    let calculated = (combined - host_charge) / page_size * page_size;
    expected != 0 && expected == calculated
}

#[cfg(feature = "test-support")]
fn local_probe_network_denied() -> bool {
    std::env::var("WORK_ENGINE_PROBE_HOST_NET_INO")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(local_probe_network_denied_from)
}

#[cfg(feature = "test-support")]
fn local_probe_network_denied_from(host: u64) -> bool {
    let inside = fs::metadata("/proc/self/ns/net")
        .ok()
        .map(|meta| meta.ino());
    if inside.is_none() || inside == Some(host) {
        return false;
    }
    // Bubblewrap creates a fresh network namespace without an external
    // interface. Reject any realized IPv4/IPv6 route before accepting frames.
    let ipv4 = fs::read_to_string("/proc/net/route").ok();
    let ipv6 = fs::read_to_string("/proc/net/ipv6_route").ok();
    ipv4.is_some_and(|routes| routes.lines().count() == 1)
        && ipv6.is_some_and(|routes| {
            routes
                .lines()
                .all(|line| line.split_whitespace().last() == Some("lo"))
        })
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(nonce) = args.next() else {
        std::process::exit(60)
    };
    let Some(program) = args.next() else {
        std::process::exit(61)
    };
    let Some(mode) = args.next() else {
        std::process::exit(61)
    };
    let Some(project) = args.next() else {
        std::process::exit(61)
    };
    if !nonce.bytes().all(|b| b.is_ascii_hexdigit())
        || nonce.len() != 64
        || program != "/claude/bin"
        || !(matches!(mode.as_str(), "offline" | "installed")
            || cfg!(feature = "test-support") && mode == "local-probe")
        || project.is_empty()
        || project.len() > 64
        || !project
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        || !mounted("/repo/candidate", "ro")
        || !mounted("/", "ro")
        || !mounted("/dev", "ro")
        || !mounted("/proc", "ro")
        || !mounted("/quota", "ro")
        || !mounted("/session", "rw")
        || !bounded_active_cgroup()
        || !bounded_tmpfs("/session", ACTIVE_LIMIT)
        || ["/cache", "/runtime", "/scratch", "/tmp"]
            .iter()
            .any(|path| !bounded_tmpfs(path, 4 * 1024 * 1024))
        || !mounted("/cbm/socket", "ro")
        || !mounted("/cbm/client", "ro")
        || !mounted("/claude/bin", "ro")
    {
        std::process::exit(62);
    }
    #[cfg(feature = "test-support")]
    if mode == "local-probe" && !local_probe_network_denied() {
        std::process::exit(62);
    }
    let Ok(socket) = std::fs::symlink_metadata("/cbm/socket/analysis.sock") else {
        std::process::exit(63)
    };
    if !socket.file_type().is_socket() {
        std::process::exit(64);
    }
    eprintln!("READY:{nonce}:bwrap-candidate-ro-v2");
    let mut input = std::io::stdin();
    let token = read_frame(&mut input, 64 * 1024).unwrap_or_else(|| std::process::exit(65));
    let prompt = read_frame(&mut input, 1024 * 1024).unwrap_or_else(|| std::process::exit(65));
    if prompt.is_empty() {
        std::process::exit(65);
    }
    let mut trailing = [0u8; 1];
    if input.read(&mut trailing).unwrap_or(1) != 0 {
        std::process::exit(65);
    }
    let token = String::from_utf8(token).unwrap_or_else(|_| std::process::exit(65));
    if token.chars().any(char::is_whitespace) {
        std::process::exit(65);
    }
    let mut command = Command::new(&program);
    command
        .args(args)
        .env_clear()
        .env("HOME", "/session")
        .env("CLAUDE_CONFIG_DIR", "/session")
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(if mode == "local-probe" {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });
    if !token.is_empty() {
        command.env("CLAUDE_CODE_OAUTH_TOKEN", &token);
    }
    if mode == "offline" {
        command.env("WORK_ENGINE_OFFLINE_PEER", "1");
        command.env("WORK_ENGINE_CBM_PROJECT", &project);
        #[cfg(feature = "test-support")]
        if std::env::var("WORK_ENGINE_RP2_HOLD_FIXTURE")
            .ok()
            .as_deref()
            == Some("1")
        {
            command.env("WORK_ENGINE_RP2_HOLD_FIXTURE", "1");
        }
    }
    let mut child = command.spawn().unwrap_or_else(|_| std::process::exit(66));
    #[cfg(feature = "test-support")]
    let probe_stderr = if mode == "local-probe" {
        let executable = fs::metadata(program).unwrap_or_else(|_| std::process::exit(66));
        eprintln!(
            "SPAWNED:{nonce}:{}:{}:{}",
            child.id(),
            executable.dev(),
            executable.ino()
        );
        child.stderr.take().map(|mut stderr| {
            std::thread::spawn(move || {
                let mut bounded = Vec::new();
                stderr
                    .by_ref()
                    .take(RESULT_LIMIT + 1)
                    .read_to_end(&mut bounded)
                    .map(|_| bounded)
            })
        })
    } else {
        None
    };
    let mut input = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || input.write_all(&prompt));
    let mut output = child.stdout.take().unwrap();
    let copier = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        output
            .by_ref()
            .take(RESULT_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let status = child.wait().unwrap_or_else(|_| std::process::exit(67));
    #[cfg(feature = "test-support")]
    if let Some(stderr) = probe_stderr {
        let bytes = stderr
            .join()
            .unwrap_or_else(|_| std::process::exit(68))
            .unwrap_or_else(|_| std::process::exit(68));
        if bytes.len() as u64 > RESULT_LIMIT {
            std::process::exit(70);
        }
        let _ = std::io::stderr().write_all(&bytes);
    }
    if !matches!(writer.join(), Ok(Ok(()))) {
        std::process::exit(68);
    }
    let result = copier
        .join()
        .unwrap_or_else(|_| std::process::exit(68))
        .unwrap_or_else(|_| std::process::exit(68));
    if result.len() as u64 > RESULT_LIMIT {
        std::process::exit(70);
    }
    if !status.success() {
        std::process::exit(status.code().unwrap_or(69));
    }
    if export_session(&result).is_err() {
        std::process::exit(71);
    }
    std::process::exit(status.code().unwrap_or(69));
}

fn read_frame(input: &mut impl Read, limit: usize) -> Option<Vec<u8>> {
    let mut length = [0u8; 4];
    input.read_exact(&mut length).ok()?;
    let length = u32::from_be_bytes(length) as usize;
    if length > limit {
        return None;
    }
    let mut bytes = vec![0u8; length];
    input.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::bounded_memory_policy;
    use std::fs;

    #[test]
    fn realized_quota_refuses_wrong_or_unavailable_swap_policy() {
        let quota = tempfile::tempdir().unwrap();
        let memory = quota.path().join("memory.max");
        let max = quota.path().join("memory.swap.max");
        let current = quota.path().join("memory.swap.current");
        let accepted = || bounded_memory_policy(quota.path(), 64 * 1024 * 1024);
        fs::write(&memory, b"67108864\n").unwrap();
        assert!(!accepted(), "missing swap policy");
        fs::write(&max, b"max\n").unwrap();
        fs::write(&current, b"0\n").unwrap();
        assert!(!accepted(), "unbounded policy");
        fs::write(&max, b"1\n").unwrap();
        assert!(!accepted(), "nonzero policy");
        fs::write(&max, b"0\n").unwrap();
        fs::write(&current, b"1\n").unwrap();
        assert!(!accepted(), "preexisting swap charge");
        fs::remove_file(&current).unwrap();
        assert!(!accepted(), "missing current charge");
        fs::write(&current, b"0\n").unwrap();
        assert!(accepted(), "realized zero swap");
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn local_probe_refuses_host_network_namespace() {
        use std::os::unix::fs::MetadataExt;

        let host = fs::metadata("/proc/self/ns/net").unwrap().ino();
        assert!(!super::local_probe_network_denied_from(host));
        // A forged different inode is insufficient while the host retains an
        // external route. The actual Bubblewrap test proves the positive case.
        assert!(!super::local_probe_network_denied_from(u64::MAX));
    }

    #[test]
    fn policy_equation_is_versioned_and_refuses_wrong_charge_or_limit() {
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
        assert!(super::bounded_policy_equation(
            "strict_aggregate_session_64mib_v1",
            8192,
            page,
            64 * 1024 * 1024 - 8192
        ));
        assert!(super::bounded_policy_equation(
            "session64mib_combined1g_zero_swap_v2",
            8192,
            page,
            1024 * 1024 * 1024 - 8192
        ));
        assert!(!super::bounded_policy_equation(
            "session64mib_combined1g_zero_swap_v2",
            8192,
            page,
            64 * 1024 * 1024 - 8192
        ));
        assert!(!super::bounded_policy_equation(
            "session64mib_combined1g_zero_swap_v2",
            64 * 1024 * 1024 + 1,
            page,
            1024 * 1024 * 1024
        ));
        assert!(!super::bounded_policy_equation(
            "session64mib_combined1g_zero_swap_v2",
            0,
            page * 2,
            1024 * 1024 * 1024
        ));
        assert!(!super::bounded_policy_equation(
            "unknown",
            0,
            page,
            1024 * 1024 * 1024
        ));
    }
}
