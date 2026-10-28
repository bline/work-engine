//! One pinned Linux mount/PID/network profile for an access-only Codex child.
//! No credential is loaded before every observation in `spawn` succeeds.

use std::{
    collections::BTreeSet,
    fs::{self, File, Metadata},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _},
        unix::net::UnixStream,
    },
    path::{Path, PathBuf},
    time::Duration,
};

use lifecycle_runtime::{NativeChildExit, NativeStdio};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use tokio::process::Command;

use crate::transport::NativeHarnessError;
use crate::{IsolationPhase, IsolationReason};

const BWRAP: &str = "/usr/bin/bwrap";
const BWRAP_SHA256: &str = "523da3e7399044be5163aee6f57a77a6bef7454376e28f0a0627920bae1b76b6";
const CODE_MODE_SHA256: &str = "8724d73ec190536233ab4628d24a071bfbcf27e5124f08fe35129f26fe48b916";
const HELPER_SHA256: &str = "386bdea885e2af49f534db71fe2ea8c9c235f36ece15f0963fc23bccf0f2033a";
#[cfg(feature = "synthetic-access-test")]
const SYNTHETIC_HELPER_SHA256: &str =
    "62b5dc7e5c6d1d504a7558a336c6209a93d4c275f041abaebd03fb5dfca93ea4";
const PROXY_POLICY_SHA256: &str =
    "4fef1370d37ef52b128fe2ab89c778e6b7854c74440d3a84f4ccc6822b1a4dfb";
#[cfg(feature = "synthetic-access-test")]
const SYNTHETIC_PROXY_POLICY_SHA256: &str =
    "ac4ae29897a24d22e41867e46e26728cc95500a7765cb6d87b3c59952c0f7d23";
const PROXY_PYTHON_SHA256: &str =
    "52e0a13e60a981d8c4b6478be2ba5176f69da07948a056bf49cf6f077e30cb41";
#[cfg(feature = "synthetic-access-test")]
const HTTP_FIXTURE_SHA256: &str =
    "601d5ecaa942eba075c65e610dfe362c5235ccde7b1c93e8d931dd588ec918da";
const CA: &str = "/etc/ssl/certs/ca-certificates.crt";
const CA_SHA256: &str = "2e89522ad15c0874c0833ed8de5a93596acf34e6651eabbe41c5e823f11bc8e9";
const INFO_FD: i32 = 10;
const STATUS_FD: i32 = 11;
const PROXY_TARGET: &str = "/proxy/proxy.sock";
#[cfg(feature = "synthetic-access-test")]
const HTTP_TARGET: &str = "/synthetic/http.sock";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProxyAdmission {
    Real,
    #[cfg(any(test, feature = "synthetic-access-test"))]
    SyntheticTest,
}

#[derive(Clone, Debug)]
pub struct AccessIsolationConfig {
    pub helper_executable: PathBuf,
    pub helper_sha256: String,
    pub proxy_socket: PathBuf,
    pub proxy_policy_executable: PathBuf,
    pub proxy_policy_sha256: String,
    pub source_home: PathBuf,
    #[cfg(feature = "synthetic-access-test")]
    pub synthetic_http_socket: Option<PathBuf>,
    #[cfg(feature = "synthetic-access-test")]
    pub synthetic_http_port: Option<u16>,
    #[cfg(feature = "synthetic-access-test")]
    pub synthetic_http_fixture_executable: Option<PathBuf>,
    #[cfg(feature = "synthetic-access-test")]
    pub synthetic_http_fixture_sha256: Option<String>,
}

pub struct IsolatedNative {
    pub native_pid: u32,
    native_starttime: u64,
    native_pidfd: OwnedFd,
    relay_pidfd: OwnedFd,
    #[cfg(feature = "synthetic-access-test")]
    http_relay_pidfd: Option<OwnedFd>,
    proxy_pid: u32,
    proxy_starttime: u64,
    proxy_pidfd: OwnedFd,
    proxy_socket: PathBuf,
    proxy_socket_dev: u64,
    proxy_socket_ino: u64,
    #[cfg(feature = "synthetic-access-test")]
    http_fixture: Option<ObservedHttpFixture>,
    status: File,
}

#[cfg(feature = "synthetic-access-test")]
struct ObservedHttpFixture {
    pid: u32,
    starttime: u64,
    pidfd: OwnedFd,
    socket: PathBuf,
    dev: u64,
    ino: u64,
}

fn unavailable(reason: IsolationReason) -> NativeHarnessError {
    NativeHarnessError::isolation(reason)
}

fn hash_is(path: &Path, expected: &str) -> bool {
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return false;
    }
    fs::read(path)
        .ok()
        .is_some_and(|data| format!("{:x}", Sha256::digest(data)) == expected)
}

fn private(meta: &Metadata) -> bool {
    meta.uid() == unsafe { libc::geteuid() } && meta.permissions().mode() & 0o077 == 0
}

fn no_symlink_components(path: &Path) -> bool {
    let mut current = PathBuf::new();
    for part in path.components() {
        current.push(part);
        if fs::symlink_metadata(&current).is_ok_and(|m| m.file_type().is_symlink()) {
            return false;
        }
    }
    true
}

fn disjoint(a: &Path, b: &Path) -> bool {
    !a.starts_with(b) && !b.starts_with(a)
}

fn file_pipe() -> Result<(OwnedFd, OwnedFd), NativeHarnessError> {
    let mut fds = [0i32; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err(unavailable(IsolationReason::PipeSetup));
    }
    Ok((unsafe { OwnedFd::from_raw_fd(fds[0]) }, unsafe {
        OwnedFd::from_raw_fd(fds[1])
    }))
}

fn status_exit_code(fd: &impl AsRawFd, timeout_ms: i32) -> Result<Option<i32>, NativeHarnessError> {
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms as u64);
    let mut bytes = Vec::with_capacity(256);
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(unavailable(IsolationReason::StatusObservation));
        }
        let mut pollfd = libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        if unsafe {
            libc::poll(
                &mut pollfd,
                1,
                remaining.as_millis().min(i32::MAX as u128) as i32,
            )
        } <= 0
        {
            return Err(unavailable(IsolationReason::StatusObservation));
        }
        let mut chunk = [0u8; 512];
        let count = unsafe { libc::read(fd.as_raw_fd(), chunk.as_mut_ptr().cast(), chunk.len()) };
        if count < 0 {
            return Err(unavailable(IsolationReason::StatusObservation));
        }
        if count == 0 {
            break;
        }
        if bytes.len() + count as usize > 4096 {
            return Err(unavailable(IsolationReason::StatusObservation));
        }
        bytes.extend_from_slice(&chunk[..count as usize]);
    }
    let mut code = None;
    for record in serde_json::Deserializer::from_slice(&bytes).into_iter::<Value>() {
        let record = record.map_err(|_| unavailable(IsolationReason::StatusObservation))?;
        if let Some(value) = record.get("exit-code") {
            if code.is_some() {
                return Err(unavailable(IsolationReason::StatusObservation));
            }
            code = Some(
                value
                    .as_i64()
                    .and_then(|number| i32::try_from(number).ok())
                    .ok_or_else(|| unavailable(IsolationReason::StatusObservation))?,
            );
        }
    }
    Ok(code)
}

fn bounded_info(fd: &impl AsRawFd, timeout_ms: i32) -> Result<Value, NativeHarnessError> {
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms as u64);
    let mut bytes = Vec::with_capacity(128);
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(unavailable(IsolationReason::InfoTimeout));
        }
        let mut pollfd = libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let polled = unsafe {
            libc::poll(
                &mut pollfd,
                1,
                remaining.as_millis().min(i32::MAX as u128) as i32,
            )
        };
        if polled <= 0 || pollfd.revents & libc::POLLIN == 0 {
            let reason = if polled == 0 {
                IsolationReason::InfoTimeout
            } else if polled > 0 && pollfd.revents & libc::POLLHUP != 0 {
                IsolationReason::InfoEof
            } else {
                IsolationReason::InfoPoll
            };
            return Err(unavailable(reason));
        }
        let mut byte = 0u8;
        if unsafe { libc::read(fd.as_raw_fd(), (&mut byte as *mut u8).cast(), 1) } != 1 {
            return Err(unavailable(IsolationReason::InfoEof));
        }
        if bytes.len() == 1024 {
            return Err(unavailable(IsolationReason::InfoBound));
        }
        bytes.push(byte);
        match serde_json::from_slice(&bytes) {
            Ok(value) => return Ok(value),
            Err(error) if error.is_eof() => {}
            Err(_) => return Err(unavailable(IsolationReason::InfoMalformed)),
        }
    }
}

fn children(pid: u32) -> Result<Vec<u32>, NativeHarnessError> {
    let content = fs::read_to_string(format!("/proc/{pid}/task/{pid}/children"))
        .map_err(|_| unavailable(IsolationReason::ChildTopology))?;
    content
        .split_whitespace()
        .map(|s| {
            s.parse::<u32>()
                .map_err(|_| unavailable(IsolationReason::ChildTopology))
        })
        .collect()
}

fn only_child(pid: u32) -> Result<u32, NativeHarnessError> {
    let found = children(pid)?;
    if found.len() != 1 {
        return Err(unavailable(IsolationReason::ChildTopology));
    }
    Ok(found[0])
}

#[cfg(feature = "synthetic-access-test")]
fn relay_mode(pid: u32, mode: &[u8]) -> bool {
    if !proc_exe_is(pid, "/helper/access_lease_launcher") {
        return false;
    }
    fs::read(format!("/proc/{pid}/cmdline"))
        .ok()
        .is_some_and(|bytes| {
            let args = bytes
                .split(|b| *b == 0)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            args.len() == 4 && args[0] == b"/helper/access_lease_launcher" && args[1] == mode
        })
}

fn starttime(pid: u32) -> Result<u64, NativeHarnessError> {
    let text = fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|_| unavailable(IsolationReason::ProcessIdentity))?;
    let end = text
        .rfind(") ")
        .ok_or_else(|| unavailable(IsolationReason::ProcessIdentity))?;
    text[end + 2..]
        .split_whitespace()
        .nth(19)
        .ok_or_else(|| unavailable(IsolationReason::ProcessIdentity))?
        .parse()
        .map_err(|_| unavailable(IsolationReason::ProcessIdentity))
}

fn nspid(pid: u32) -> Result<Vec<u32>, NativeHarnessError> {
    let status = fs::read_to_string(format!("/proc/{pid}/status"))
        .map_err(|_| unavailable(IsolationReason::NamespaceIdentity))?;
    let field = status
        .lines()
        .find_map(|l| l.strip_prefix("NSpid:"))
        .ok_or_else(|| unavailable(IsolationReason::NamespaceIdentity))?;
    field
        .split_whitespace()
        .map(|s| {
            s.parse()
                .map_err(|_| unavailable(IsolationReason::NamespaceIdentity))
        })
        .collect()
}

fn pidfd(pid: u32) -> Result<OwnedFd, NativeHarnessError> {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
    if fd < 0 {
        return Err(unavailable(IsolationReason::Pidfd));
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd as i32) })
}

fn pidfd_exited(fd: &OwnedFd) -> bool {
    let mut p = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    (unsafe { libc::poll(&mut p, 1, 0) }) == 1 && p.revents & libc::POLLIN != 0
}

fn strict_cmdline(bytes: &[u8]) -> Option<Vec<&[u8]>> {
    if bytes.last() != Some(&0) {
        return None;
    }
    let args = bytes[..bytes.len() - 1]
        .split(|b| *b == 0)
        .collect::<Vec<_>>();
    (!args.is_empty() && args.iter().all(|arg| !arg.is_empty())).then_some(args)
}

fn proxy_command_matches(
    args: &[&[u8]],
    selected: &AccessIsolationConfig,
    admission: ProxyAdmission,
    expected_journal: Option<&Path>,
) -> bool {
    if admission == ProxyAdmission::Real {
        let [
            python,
            isolated,
            no_site,
            script,
            socket_flag,
            socket,
            journal_flag,
            journal,
            mode_flag,
            mode,
            max_flag,
            max,
            tunnel_flag,
            tunnel,
        ] = args
        else {
            return false;
        };
        return *python == b"/usr/bin/python3"
            && *isolated == b"-I"
            && *no_site == b"-S"
            && *script
                == selected
                    .proxy_policy_executable
                    .as_os_str()
                    .as_encoded_bytes()
            && *socket_flag == b"--socket"
            && *socket == selected.proxy_socket.as_os_str().as_encoded_bytes()
            && *journal_flag == b"--journal"
            && expected_journal
                .is_some_and(|path| *journal == path.as_os_str().as_encoded_bytes())
            && *mode_flag == b"--mode"
            && *mode == b"real"
            && *max_flag == b"--max-seconds"
            && *max == b"115"
            && *tunnel_flag == b"--tunnel-seconds"
            && *tunnel == b"10";
    }
    let _ = expected_journal;
    if args.len() < 8
        || !args.len().is_multiple_of(2)
        || args[0] != b"/usr/bin/python3"
        || args[1]
            != selected
                .proxy_policy_executable
                .as_os_str()
                .as_encoded_bytes()
    {
        return false;
    }
    let mut seen = BTreeSet::new();
    let mut mode = None;
    let mut socket = None;
    let mut journal = None;
    let mut test_target = None;
    for pair in args[2..].chunks_exact(2) {
        if !seen.insert(pair[0]) {
            return false;
        }
        match pair[0] {
            b"--mode" => mode = Some(pair[1]),
            b"--socket" => socket = Some(pair[1]),
            b"--journal" => journal = Some(pair[1]),
            b"--test-target" => test_target = Some(pair[1]),
            b"--max-seconds" | b"--tunnel-seconds" => {
                if std::str::from_utf8(pair[1])
                    .ok()
                    .and_then(|v| v.parse::<u32>().ok())
                    .is_none_or(|n| n == 0)
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
    let selected_socket = selected.proxy_socket.as_os_str().as_encoded_bytes();
    let journal_is_absolute = journal.is_some_and(|path| path.starts_with(b"/"));
    let selected_mode = match admission {
        ProxyAdmission::Real => mode == Some(b"real".as_slice()) && test_target.is_none(),
        #[cfg(any(test, feature = "synthetic-access-test"))]
        ProxyAdmission::SyntheticTest => {
            mode == Some(b"synthetic".as_slice())
                && test_target.is_some_and(|target| target.starts_with(b"127.0.0.1:"))
        }
    };
    socket == Some(selected_socket) && journal_is_absolute && selected_mode
}

fn validate_proxy_command(
    args: &[&[u8]],
    selected: &AccessIsolationConfig,
    admission: ProxyAdmission,
    journal: Option<&Path>,
) -> Result<(), NativeHarnessError> {
    if proxy_command_matches(args, selected, admission, journal) {
        Ok(())
    } else {
        Err(unavailable(IsolationReason::ProxyCmdline))
    }
}

fn proxy_policy_pin(admission: ProxyAdmission) -> &'static str {
    match admission {
        ProxyAdmission::Real => PROXY_POLICY_SHA256,
        #[cfg(feature = "synthetic-access-test")]
        ProxyAdmission::SyntheticTest => SYNTHETIC_PROXY_POLICY_SHA256,
        #[cfg(all(test, not(feature = "synthetic-access-test")))]
        ProxyAdmission::SyntheticTest => PROXY_POLICY_SHA256,
    }
}

fn attest_proxy(
    selected: &AccessIsolationConfig,
    admission: ProxyAdmission,
) -> Result<(u32, u64, OwnedFd, u64, u64), NativeHarnessError> {
    let socket = UnixStream::connect(&selected.proxy_socket)
        .map_err(|_| unavailable(IsolationReason::ProxyConnect))?;
    let mut peer = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut peer as *mut libc::ucred).cast(),
            &mut length,
        )
    } != 0
        || length as usize != std::mem::size_of::<libc::ucred>()
        || peer.pid <= 0
        || peer.uid != unsafe { libc::geteuid() }
    {
        return Err(unavailable(IsolationReason::ProxyPeer));
    }
    let proxy_pid = peer.pid as u32;
    let cmdline = fs::read(format!("/proc/{proxy_pid}/cmdline"))
        .map_err(|_| unavailable(IsolationReason::ProxyCmdline))?;
    let args = match admission {
        ProxyAdmission::Real => {
            strict_cmdline(&cmdline).ok_or_else(|| unavailable(IsolationReason::ProxyCmdline))?
        }
        #[cfg(any(test, feature = "synthetic-access-test"))]
        ProxyAdmission::SyntheticTest => cmdline
            .split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>(),
    };
    let expected_journal = if admission == ProxyAdmission::Real {
        let cwd = fs::read_link(format!("/proc/{proxy_pid}/cwd"))
            .map_err(|_| unavailable(IsolationReason::ProxyCwd))?;
        if !cwd.is_absolute()
            || !no_symlink_components(&cwd)
            || !private(&fs::metadata(&cwd).map_err(|_| unavailable(IsolationReason::ProxyCwd))?)
        {
            return Err(unavailable(IsolationReason::ProxyCwd));
        }
        Some(cwd.join("proxy-metadata.jsonl"))
    } else {
        None
    };
    let python = fs::canonicalize("/usr/bin/python3")
        .map_err(|_| unavailable(IsolationReason::ProxyInterpreter))?;
    validate_proxy_command(&args, selected, admission, expected_journal.as_deref())?;
    if fs::read_link(format!("/proc/{proxy_pid}/exe")).ok() != Some(python.clone())
        || !hash_is(&python, PROXY_PYTHON_SHA256)
    {
        return Err(unavailable(IsolationReason::ProxyInterpreter));
    }
    if !hash_is(
        &selected.proxy_policy_executable,
        proxy_policy_pin(admission),
    ) {
        return Err(unavailable(IsolationReason::ProxyPolicyPin));
    }
    let meta = fs::symlink_metadata(&selected.proxy_socket)
        .map_err(|_| unavailable(IsolationReason::ProxyMetadata))?;
    let start = starttime(proxy_pid)?;
    let open = pidfd(proxy_pid)?;
    if pidfd_exited(&open) || starttime(proxy_pid)? != start {
        return Err(unavailable(IsolationReason::ProxyIdentity));
    }
    Ok((proxy_pid, start, open, meta.dev(), meta.ino()))
}

#[cfg(feature = "synthetic-access-test")]
fn attest_http_fixture(
    selected: &AccessIsolationConfig,
) -> Result<ObservedHttpFixture, NativeHarnessError> {
    let socket_path = selected
        .synthetic_http_socket
        .as_ref()
        .ok_or_else(|| unavailable(IsolationReason::HttpFixture))?;
    let executable = selected
        .synthetic_http_fixture_executable
        .as_ref()
        .ok_or_else(|| unavailable(IsolationReason::HttpFixture))?;
    if selected.synthetic_http_fixture_sha256.as_deref() != Some(HTTP_FIXTURE_SHA256)
        || !hash_is(executable, HTTP_FIXTURE_SHA256)
    {
        return Err(unavailable(IsolationReason::HttpFixture));
    }
    let stream =
        UnixStream::connect(socket_path).map_err(|_| unavailable(IsolationReason::HttpFixture))?;
    let mut peer = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut peer as *mut libc::ucred).cast(),
            &mut length,
        )
    } != 0
        || length as usize != std::mem::size_of::<libc::ucred>()
        || peer.pid <= 0
        || peer.uid != unsafe { libc::geteuid() }
    {
        return Err(unavailable(IsolationReason::HttpFixture));
    }
    let pid = peer.pid as u32;
    let cmdline = fs::read(format!("/proc/{pid}/cmdline"))
        .map_err(|_| unavailable(IsolationReason::HttpFixture))?;
    let args = cmdline
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let socket = socket_path.as_os_str().as_encoded_bytes();
    let script = executable.as_os_str().as_encoded_bytes();
    let command_ok =
        if let [python, script_arg, a, socket_arg, b, journal, c, seconds] = args.as_slice() {
            *python == b"/usr/bin/python3"
                && *script_arg == script
                && *a == b"--socket"
                && *socket_arg == socket
                && *b == b"--journal"
                && journal.starts_with(b"/")
                && *c == b"--max-seconds"
                && std::str::from_utf8(seconds)
                    .ok()
                    .and_then(|n| n.parse::<u32>().ok())
                    .is_some_and(|n| n > 0)
        } else {
            false
        };
    let python = fs::canonicalize("/usr/bin/python3")
        .map_err(|_| unavailable(IsolationReason::HttpFixture))?;
    if !command_ok
        || fs::read_link(format!("/proc/{pid}/exe")).ok() != Some(python.clone())
        || !hash_is(&python, PROXY_PYTHON_SHA256)
    {
        return Err(unavailable(IsolationReason::HttpFixture));
    }
    let meta =
        fs::symlink_metadata(socket_path).map_err(|_| unavailable(IsolationReason::HttpFixture))?;
    let started = starttime(pid)?;
    let pidfd = pidfd(pid)?;
    if pidfd_exited(&pidfd) || starttime(pid)? != started {
        return Err(unavailable(IsolationReason::HttpFixture));
    }
    Ok(ObservedHttpFixture {
        pid,
        starttime: started,
        pidfd,
        socket: socket_path.clone(),
        dev: meta.dev(),
        ino: meta.ino(),
    })
}

fn proc_exe_is(pid: u32, expected: &str) -> bool {
    fs::read_link(format!("/proc/{pid}/exe")).is_ok_and(|p| p == Path::new(expected))
}

fn descendants_omit_fd(pid: u32, forbidden: &[String], source: &Path) -> bool {
    let Ok(dir) = fs::read_dir(format!("/proc/{pid}/fd")) else {
        return false;
    };
    for entry in dir.flatten() {
        let Ok(target) = fs::read_link(entry.path()) else {
            return false;
        };
        let t = target.to_string_lossy();
        if forbidden.iter().any(|value| t == *value) || target.starts_with(source) {
            return false;
        }
    }
    true
}

fn inspect_tree(path: &Path, count: &mut usize) -> Result<(), NativeHarnessError> {
    *count += 1;
    if *count > 128 {
        return Err(unavailable(IsolationReason::TreeBound));
    }
    for item in fs::read_dir(path).map_err(|_| unavailable(IsolationReason::TreeRead))? {
        let entry = item.map_err(|_| unavailable(IsolationReason::TreeRead))?;
        let meta = fs::symlink_metadata(entry.path())
            .map_err(|_| unavailable(IsolationReason::TreeRead))?;
        if meta.file_type().is_symlink() {
            return Err(unavailable(IsolationReason::TreeSymlink));
        }
        if meta.is_file() && meta.nlink() > 1 {
            return Err(unavailable(IsolationReason::TreeHardlink));
        }
        if meta.is_dir() {
            inspect_tree(&entry.path(), count)?;
        } else if !meta.is_file() {
            return Err(unavailable(IsolationReason::TreeType));
        }
    }
    Ok(())
}

fn validate_paths(
    root: &Path,
    home: &Path,
    codex_home: &Path,
    work: &Path,
    selected: &AccessIsolationConfig,
    admission: ProxyAdmission,
) -> Result<(), NativeHarnessError> {
    let [root, source, helper, socket] = [
        root,
        &selected.source_home,
        &selected.helper_executable,
        &selected.proxy_socket,
    ];
    if ![root, source, helper, socket]
        .iter()
        .all(|p| p.is_absolute() && no_symlink_components(p))
    {
        return Err(unavailable(IsolationReason::PathTopology));
    }
    if !selected.proxy_policy_executable.is_absolute()
        || !no_symlink_components(&selected.proxy_policy_executable)
        || selected.proxy_policy_sha256 != proxy_policy_pin(admission)
        || !hash_is(
            &selected.proxy_policy_executable,
            proxy_policy_pin(admission),
        )
    {
        return Err(unavailable(IsolationReason::PolicyPin));
    }
    let proxy_parent = socket
        .parent()
        .ok_or_else(|| unavailable(IsolationReason::PathDisjoint))?;
    if !disjoint(root, source)
        || !disjoint(root, proxy_parent)
        || !disjoint(source, proxy_parent)
        || !disjoint(root, helper)
    {
        return Err(unavailable(IsolationReason::PathDisjoint));
    }
    #[cfg(feature = "synthetic-access-test")]
    match admission {
        ProxyAdmission::Real
            if selected.synthetic_http_socket.is_some()
                || selected.synthetic_http_port.is_some()
                || selected.synthetic_http_fixture_executable.is_some()
                || selected.synthetic_http_fixture_sha256.is_some() =>
        {
            return Err(unavailable(IsolationReason::SyntheticProfile));
        }
        ProxyAdmission::SyntheticTest => {
            let http = selected
                .synthetic_http_socket
                .as_ref()
                .ok_or_else(|| unavailable(IsolationReason::SyntheticProfile))?;
            let fixture = selected
                .synthetic_http_fixture_executable
                .as_ref()
                .ok_or_else(|| unavailable(IsolationReason::SyntheticProfile))?;
            let parent = http
                .parent()
                .ok_or_else(|| unavailable(IsolationReason::SyntheticProfile))?;
            if selected.synthetic_http_port.is_none_or(|port| port == 0)
                || ![http, fixture]
                    .iter()
                    .all(|path| path.is_absolute() && no_symlink_components(path))
                || !disjoint(root, parent)
                || !disjoint(source, parent)
                || !disjoint(proxy_parent, parent)
                || !disjoint(root, fixture)
                || !disjoint(source, fixture)
                || !fs::symlink_metadata(parent).is_ok_and(|m| m.is_dir() && private(&m))
                || !fs::symlink_metadata(http)
                    .is_ok_and(|m| m.file_type().is_socket() && private(&m))
            {
                return Err(unavailable(IsolationReason::SyntheticProfile));
            }
        }
        _ => {}
    }
    #[cfg(not(feature = "synthetic-access-test"))]
    let _ = admission;
    if ![root, home, codex_home, work]
        .iter()
        .all(|p| p.starts_with(root))
    {
        return Err(unavailable(IsolationReason::DirectoryTopology));
    }
    if home != root.join("user") || codex_home != root.join("codex") || work != root.join("work") {
        return Err(unavailable(IsolationReason::DirectoryTopology));
    }
    let root_meta =
        fs::symlink_metadata(root).map_err(|_| unavailable(IsolationReason::DirectoryMetadata))?;
    let proxy_meta = fs::symlink_metadata(proxy_parent)
        .map_err(|_| unavailable(IsolationReason::DirectoryMetadata))?;
    let socket_meta = fs::symlink_metadata(socket)
        .map_err(|_| unavailable(IsolationReason::DirectoryMetadata))?;
    if !root_meta.is_dir()
        || !private(&root_meta)
        || !proxy_meta.is_dir()
        || !private(&proxy_meta)
        || !socket_meta.file_type().is_socket()
        || !private(&socket_meta)
    {
        return Err(unavailable(IsolationReason::DirectoryMetadata));
    }
    for path in [home, codex_home, work] {
        let meta = fs::symlink_metadata(path)
            .map_err(|_| unavailable(IsolationReason::DirectoryMetadata))?;
        if !meta.is_dir() || !private(&meta) {
            return Err(unavailable(IsolationReason::DirectoryMetadata));
        }
    }
    inspect_tree(root, &mut 0)?;
    // A fresh launch root may not contain a mounted alias to a host source.
    let mountinfo = fs::read_to_string("/proc/self/mountinfo")
        .map_err(|_| unavailable(IsolationReason::RootMount))?;
    if mountinfo.lines().any(|line| {
        line.split_whitespace()
            .nth(4)
            .is_some_and(|p| Path::new(p).starts_with(root))
    }) {
        return Err(unavailable(IsolationReason::RootMount));
    }
    let helper_pin = match admission {
        ProxyAdmission::Real => HELPER_SHA256,
        #[cfg(feature = "synthetic-access-test")]
        ProxyAdmission::SyntheticTest => SYNTHETIC_HELPER_SHA256,
        #[cfg(all(test, not(feature = "synthetic-access-test")))]
        ProxyAdmission::SyntheticTest => HELPER_SHA256,
    };
    if selected.helper_sha256 != helper_pin || !hash_is(helper, &selected.helper_sha256) {
        return Err(unavailable(IsolationReason::HelperPin));
    }
    if selected.proxy_policy_sha256.len() != 64
        || !selected
            .proxy_policy_sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(unavailable(IsolationReason::PolicyDigest));
    }
    if !hash_is(Path::new(BWRAP), BWRAP_SHA256) {
        return Err(unavailable(IsolationReason::BwrapPin));
    }
    if !hash_is(Path::new(CA), CA_SHA256) {
        return Err(unavailable(IsolationReason::CaPin));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // Bound launch paths remain explicit at the custody gate.
pub fn spawn(
    executable: &Path,
    root: &Path,
    work: &Path,
    home: &Path,
    codex_home: &Path,
    https_proxy: &str,
    selected: &AccessIsolationConfig,
    admission: ProxyAdmission,
) -> Result<(NativeStdio, IsolatedNative), NativeHarnessError> {
    validate_paths(root, home, codex_home, work, selected, admission)?;
    let (proxy_pid, proxy_starttime, proxy_pidfd, proxy_socket_dev, proxy_socket_ino) =
        attest_proxy(selected, admission)?;
    #[cfg(feature = "synthetic-access-test")]
    let http_fixture = match admission {
        ProxyAdmission::Real => None,
        ProxyAdmission::SyntheticTest => Some(attest_http_fixture(selected)?),
    };
    let companion = executable.with_file_name("codex-code-mode-host");
    if !hash_is(&companion, CODE_MODE_SHA256) {
        return Err(unavailable(IsolationReason::CompanionPin));
    }
    let port = https_proxy
        .strip_prefix("http://127.0.0.1:")
        .and_then(|v| v.parse::<u16>().ok())
        .filter(|p| *p != 0)
        .ok_or_else(|| unavailable(IsolationReason::ProxyEndpoint))?;
    let tmpdir = root.join("tmp");
    let target = root.join("target");
    for path in [&tmpdir, &target] {
        if path.exists() {
            return Err(unavailable(IsolationReason::TemporaryExists));
        }
        fs::create_dir(path).map_err(|_| unavailable(IsolationReason::TemporaryCreate))?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| unavailable(IsolationReason::TemporaryMode))?;
    }
    let (info_r, info_w) = file_pipe()?;
    let (status_r, status_w) = file_pipe()?;
    let mut dirs = BTreeSet::new();
    for path in [root, Path::new(CA)] {
        for parent in path.ancestors().skip(1) {
            if parent != Path::new("/") {
                dirs.insert(parent.to_path_buf());
            }
        }
    }
    dirs.insert(PathBuf::from("/native"));
    dirs.insert(PathBuf::from("/helper"));
    dirs.insert(PathBuf::from("/proxy"));
    #[cfg(feature = "synthetic-access-test")]
    if http_fixture.is_some() {
        dirs.insert(PathBuf::from("/synthetic"));
    }
    let mut dirs = dirs.into_iter().collect::<Vec<_>>();
    dirs.sort_by_key(|p| (p.components().count(), p.to_path_buf()));
    let mut command = Command::new(BWRAP);
    command.args([
        "--unshare-user",
        "--unshare-pid",
        "--unshare-net",
        "--die-with-parent",
        "--tmpfs",
        "/",
    ]);
    for dir in dirs {
        command.arg("--dir").arg(dir);
    }
    command.args([
        "--info-fd",
        &INFO_FD.to_string(),
        "--json-status-fd",
        &STATUS_FD.to_string(),
    ]);
    command
        .arg("--ro-bind")
        .arg(executable)
        .arg("/native/codex")
        .arg("--ro-bind")
        .arg(&companion)
        .arg("/native/codex-code-mode-host")
        .arg("--ro-bind")
        .arg(&selected.helper_executable)
        .arg("/helper/access_lease_launcher")
        .arg("--ro-bind")
        .arg(CA)
        .arg(CA)
        .arg("--ro-bind")
        .arg(&selected.proxy_socket)
        .arg(PROXY_TARGET);
    #[cfg(feature = "synthetic-access-test")]
    if let Some(http) = &http_fixture {
        command.arg("--ro-bind").arg(&http.socket).arg(HTTP_TARGET);
    }
    command
        .arg("--bind")
        .arg(root)
        .arg(root)
        .args(["--proc", "/proc", "--dev", "/dev"])
        .arg("--setenv")
        .arg("HOME")
        .arg(home)
        .arg("--setenv")
        .arg("CODEX_HOME")
        .arg(codex_home)
        .arg("--setenv")
        .arg("TMPDIR")
        .arg(&tmpdir)
        .arg("--setenv")
        .arg("CARGO_TARGET_DIR")
        .arg(&target)
        .args([
            "--setenv", "PATH", "/native", "--setenv", "RUST_LOG", "error",
        ])
        .arg("--setenv")
        .arg("HTTPS_PROXY")
        .arg(https_proxy)
        .args(["--setenv", "NO_PROXY", ""])
        .arg("--chdir")
        .arg(work)
        .arg("--")
        .arg("/helper/access_lease_launcher");
    #[cfg(feature = "synthetic-access-test")]
    if http_fixture.is_some() {
        command.args([
            "--launch-synthetic",
            "/native/codex",
            PROXY_TARGET,
            &port.to_string(),
            HTTP_TARGET,
        ]);
        command.arg(
            selected
                .synthetic_http_port
                .ok_or_else(|| unavailable(IsolationReason::SyntheticProfile))?
                .to_string(),
        );
    } else {
        command.args(["--launch", "/native/codex", PROXY_TARGET, &port.to_string()]);
    }
    #[cfg(not(feature = "synthetic-access-test"))]
    command.args(["--launch", "/native/codex", PROXY_TARGET, &port.to_string()]);
    command.env_clear();
    let info_safe = unsafe { libc::fcntl(info_w.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 20) };
    let status_safe = unsafe { libc::fcntl(status_w.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 20) };
    if info_safe < 0 || status_safe < 0 {
        if info_safe >= 0 {
            unsafe { libc::close(info_safe) };
        }
        if status_safe >= 0 {
            unsafe { libc::close(status_safe) };
        }
        return Err(unavailable(IsolationReason::FdSetup));
    }
    let info_safe = unsafe { OwnedFd::from_raw_fd(info_safe) };
    let status_safe = unsafe { OwnedFd::from_raw_fd(status_safe) };
    drop(info_w);
    drop(status_w);
    let info_write = info_safe.as_raw_fd();
    let status_write = status_safe.as_raw_fd();
    unsafe {
        command.pre_exec(move || {
            if libc::dup2(info_write, INFO_FD) < 0 || libc::dup2(status_write, STATUS_FD) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::syscall(libc::SYS_close_range, 3u32, 9u32, 0u32) < 0
                || libc::syscall(libc::SYS_close_range, 12u32, u32::MAX, 0u32) < 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut pipe = NativeStdio::spawn(&mut command, 1_048_576, 4096).map_err(|_| {
        NativeHarnessError::IsolationLaunchFailure(crate::IsolationDiagnostic {
            phase: IsolationPhase::LaunchInvoked,
            reason: IsolationReason::LaunchInvoke,
        })
    })?;
    drop(info_safe);
    drop(status_safe);
    let observed = (|| -> Result<IsolatedNative, NativeHarnessError> {
        let frame = bounded_info(&info_r, 5000)?;
        let monitor = frame["child-pid"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| unavailable(IsolationReason::InfoMalformed))?;
        let deadline = std::time::Instant::now() + Duration::from_secs(8);
        let (native, relay, http_relay) = loop {
            if let Ok(candidate) = only_child(monitor)
                && proc_exe_is(candidate, "/native/codex")
                && let Ok(descendants) = children(candidate)
            {
                #[cfg(feature = "synthetic-access-test")]
                if http_fixture.is_some() {
                    if descendants.len() == 2 {
                        let proxy = descendants
                            .iter()
                            .copied()
                            .find(|pid| relay_mode(*pid, b"--relay"));
                        let http = descendants
                            .iter()
                            .copied()
                            .find(|pid| relay_mode(*pid, b"--relay-http"));
                        if let (Some(proxy), Some(http)) = (proxy, http) {
                            break (candidate, proxy, Some(http));
                        }
                    }
                } else if descendants.len() == 1 && relay_mode(descendants[0], b"--relay") {
                    break (candidate, descendants[0], None);
                }
                #[cfg(not(feature = "synthetic-access-test"))]
                if descendants.len() == 1
                    && proc_exe_is(descendants[0], "/helper/access_lease_launcher")
                {
                    break (candidate, descendants[0], None::<u32>);
                }
            }
            if std::time::Instant::now() >= deadline {
                return Err(unavailable(IsolationReason::ChildTopology));
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let ns = nspid(native)?;
        if ns.len() != 2 || ns[0] != native || ns[1] == 0 {
            return Err(unavailable(IsolationReason::NamespaceIdentity));
        }
        let native_starttime = starttime(native)?;
        let native_pidfd = pidfd(native)?;
        let relay_pidfd = pidfd(relay)?;
        #[cfg(feature = "synthetic-access-test")]
        let http_relay_pidfd = http_relay.map(pidfd).transpose()?;
        #[cfg(not(feature = "synthetic-access-test"))]
        let _ = http_relay;
        let info_pipe = fs::read_link(format!("/proc/self/fd/{}", info_r.as_raw_fd()))
            .map_err(|_| unavailable(IsolationReason::DescriptorExposure))?
            .to_string_lossy()
            .into_owned();
        let status_pipe = fs::read_link(format!("/proc/self/fd/{}", status_r.as_raw_fd()))
            .map_err(|_| unavailable(IsolationReason::DescriptorExposure))?
            .to_string_lossy()
            .into_owned();
        let forbidden = [info_pipe, status_pipe];
        if !descendants_omit_fd(native, &forbidden, &selected.source_home)
            || !descendants_omit_fd(relay, &forbidden, &selected.source_home)
            || http_relay
                .is_some_and(|pid| !descendants_omit_fd(pid, &forbidden, &selected.source_home))
        {
            return Err(unavailable(IsolationReason::DescriptorExposure));
        }
        let hidden = PathBuf::from(format!("/proc/{native}/root")).join(
            selected
                .source_home
                .strip_prefix("/")
                .map_err(|_| unavailable(IsolationReason::HiddenSource))?,
        );
        if hidden.exists() {
            return Err(unavailable(IsolationReason::HiddenSource));
        }
        let child_root = PathBuf::from(format!("/proc/{native}/root"));
        let child_socket = fs::symlink_metadata(child_root.join("proxy/proxy.sock"))
            .map_err(|_| unavailable(IsolationReason::MountedSocket))?;
        if !child_socket.file_type().is_socket()
            || child_socket.dev() != proxy_socket_dev
            || child_socket.ino() != proxy_socket_ino
        {
            return Err(unavailable(IsolationReason::MountedSocket));
        }
        #[cfg(feature = "synthetic-access-test")]
        if let Some(http) = &http_fixture {
            let child_http = fs::symlink_metadata(child_root.join("synthetic/http.sock"))
                .map_err(|_| unavailable(IsolationReason::MountedSocket))?;
            if !child_http.file_type().is_socket()
                || child_http.dev() != http.dev
                || child_http.ino() != http.ino
            {
                return Err(unavailable(IsolationReason::MountedSocket));
            }
        }
        for (relative, expected) in [
            ("native/codex", crate::NATIVE_BINARY_SHA256),
            ("native/codex-code-mode-host", CODE_MODE_SHA256),
            (
                "helper/access_lease_launcher",
                selected.helper_sha256.as_str(),
            ),
            ("etc/ssl/certs/ca-certificates.crt", CA_SHA256),
        ] {
            if !hash_is(&child_root.join(relative), expected) {
                return Err(unavailable(IsolationReason::MountedBinary));
            }
        }
        let child_mountinfo = fs::read_to_string(format!("/proc/{native}/mountinfo"))
            .map_err(|_| unavailable(IsolationReason::MountReadonly))?;
        if !child_mountinfo.lines().any(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            fields.get(4) == Some(&PROXY_TARGET)
                && fields
                    .get(5)
                    .is_some_and(|opts| opts.split(',').any(|o| o == "ro"))
        }) {
            return Err(unavailable(IsolationReason::MountReadonly));
        }
        #[cfg(feature = "synthetic-access-test")]
        if http_fixture.is_some()
            && !child_mountinfo.lines().any(|line| {
                let fields = line.split_whitespace().collect::<Vec<_>>();
                fields.get(4) == Some(&HTTP_TARGET)
                    && fields
                        .get(5)
                        .is_some_and(|opts| opts.split(',').any(|o| o == "ro"))
            })
        {
            return Err(unavailable(IsolationReason::MountReadonly));
        }
        if starttime(native)? != native_starttime
            || pidfd_exited(&native_pidfd)
            || pidfd_exited(&relay_pidfd)
            || {
                #[cfg(feature = "synthetic-access-test")]
                {
                    http_relay_pidfd.as_ref().is_some_and(pidfd_exited)
                        || http_fixture.as_ref().is_some_and(|http| {
                            pidfd_exited(&http.pidfd)
                                || starttime(http.pid).ok() != Some(http.starttime)
                        })
                }
                #[cfg(not(feature = "synthetic-access-test"))]
                {
                    false
                }
            }
            || pidfd_exited(&proxy_pidfd)
            || starttime(proxy_pid)? != proxy_starttime
        {
            return Err(unavailable(IsolationReason::FinalLiveness));
        }
        Ok(IsolatedNative {
            native_pid: native,
            native_starttime,
            native_pidfd,
            relay_pidfd,
            #[cfg(feature = "synthetic-access-test")]
            http_relay_pidfd,
            proxy_pid,
            proxy_starttime,
            proxy_pidfd,
            proxy_socket: selected.proxy_socket.clone(),
            proxy_socket_dev,
            proxy_socket_ino,
            #[cfg(feature = "synthetic-access-test")]
            http_fixture,
            status: File::from(status_r),
        })
    })();
    match observed {
        Ok(observed) => Ok((pipe, observed)),
        Err(error) => {
            let error = error.after_spawn();
            let cleanup_observed = pipe.abort_before_credential(Duration::from_secs(3)).is_ok();
            Err(error.with_cleanup(cleanup_observed))
        }
    }
}

impl IsolatedNative {
    pub fn route_current(&self) -> bool {
        !pidfd_exited(&self.proxy_pidfd)
            && starttime(self.proxy_pid).ok() == Some(self.proxy_starttime)
            && fs::symlink_metadata(&self.proxy_socket)
                .is_ok_and(|m| m.dev() == self.proxy_socket_dev && m.ino() == self.proxy_socket_ino)
            && !pidfd_exited(&self.native_pidfd)
            && !pidfd_exited(&self.relay_pidfd)
            && {
                #[cfg(feature = "synthetic-access-test")]
                {
                    self.http_relay_pidfd
                        .as_ref()
                        .is_none_or(|fd| !pidfd_exited(fd))
                        && self.http_fixture.as_ref().is_none_or(|http| {
                            !pidfd_exited(&http.pidfd)
                                && starttime(http.pid).ok() == Some(http.starttime)
                                && fs::symlink_metadata(&http.socket)
                                    .is_ok_and(|m| m.dev() == http.dev && m.ino() == http.ino)
                        })
                }
                #[cfg(not(feature = "synthetic-access-test"))]
                {
                    true
                }
            }
    }
    pub async fn terminate_and_reap(
        &mut self,
        pipe: &mut NativeStdio,
    ) -> Result<NativeChildExit, NativeHarnessError> {
        pipe.close_stdin();
        let (wrapper, forced) = match pipe.wait_for_wrapper(Duration::from_secs(5)).await {
            Ok(exit) => (exit, false),
            Err(_) => {
                let exit = pipe
                    .terminate_and_reap(Duration::from_secs(2))
                    .await
                    .map_err(|_| NativeHarnessError::CleanupUnobserved)?;
                let forced = exit.forced;
                (exit, forced)
            }
        };
        let closure_deadline = std::time::Instant::now() + Duration::from_secs(2);
        while (!pidfd_exited(&self.native_pidfd) || !pidfd_exited(&self.relay_pidfd) || {
            #[cfg(feature = "synthetic-access-test")]
            {
                self.http_relay_pidfd
                    .as_ref()
                    .is_some_and(|fd| !pidfd_exited(fd))
            }
            #[cfg(not(feature = "synthetic-access-test"))]
            {
                false
            }
        }) && std::time::Instant::now() < closure_deadline
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let native_closed = pidfd_exited(&self.native_pidfd)
            && starttime(self.native_pid).ok() != Some(self.native_starttime);
        let relay_closed = pidfd_exited(&self.relay_pidfd);
        #[cfg(feature = "synthetic-access-test")]
        let http_relay_closed = self.http_relay_pidfd.as_ref().is_none_or(pidfd_exited);
        #[cfg(not(feature = "synthetic-access-test"))]
        let http_relay_closed = true;
        if !native_closed || !relay_closed || !http_relay_closed {
            return Err(NativeHarnessError::CleanupUnobserved);
        }
        let code = status_exit_code(&self.status, 2000).unwrap_or(None);
        Ok(NativeChildExit {
            pid: Some(self.native_pid),
            code,
            success: code == Some(0) && wrapper.success && !forced,
            forced,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead as _, Write as _},
        os::unix::net::UnixListener,
        process::{Command as StdCommand, Stdio},
        thread,
    };

    #[test]
    fn isolation_diagnostics_exercise_tree_guards_without_launch() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let path = root.path();
        fs::write(path.join("plain"), b"synthetic").unwrap();
        assert!(inspect_tree(path, &mut 0).is_ok());
        symlink(path.join("plain"), path.join("link")).unwrap();
        let error = inspect_tree(path, &mut 0).unwrap_err();
        assert_eq!(
            error.isolation_diagnostic().unwrap().reason,
            IsolationReason::TreeSymlink
        );
        fs::remove_file(path.join("link")).unwrap();
        fs::hard_link(path.join("plain"), path.join("hard")).unwrap();
        assert_eq!(
            inspect_tree(path, &mut 0)
                .unwrap_err()
                .isolation_diagnostic()
                .unwrap()
                .reason,
            IsolationReason::TreeHardlink
        );
        fs::remove_file(path.join("hard")).unwrap();
        let socket = UnixListener::bind(path.join("socket")).unwrap();
        assert_eq!(
            inspect_tree(path, &mut 0)
                .unwrap_err()
                .isolation_diagnostic()
                .unwrap()
                .reason,
            IsolationReason::TreeType
        );
        drop(socket);
        fs::remove_file(path.join("socket")).unwrap();
        assert_eq!(
            inspect_tree(path, &mut 128)
                .unwrap_err()
                .isolation_diagnostic()
                .unwrap()
                .reason,
            IsolationReason::TreeBound
        );
        assert!(inspect_tree(path, &mut 0).is_ok());
    }

    #[test]
    fn isolation_diagnostics_info_and_cleanup_preserve_attribution() {
        for (wire, expected) in [
            (b"!".as_slice(), IsolationReason::InfoMalformed),
            (b" ".repeat(1025).as_slice(), IsolationReason::InfoBound),
        ] {
            let (read, write) = file_pipe().unwrap();
            assert_eq!(
                unsafe { libc::write(write.as_raw_fd(), wire.as_ptr().cast(), wire.len()) },
                wire.len() as isize
            );
            drop(write);
            let error = bounded_info(&read, 100).unwrap_err().after_spawn();
            let diagnostic = error.isolation_diagnostic().unwrap();
            assert_eq!(diagnostic.phase, IsolationPhase::AfterSpawn);
            assert_eq!(diagnostic.reason, expected);
            let error = error.with_cleanup(false);
            assert!(matches!(
                error,
                NativeHarnessError::CleanupUnobservedIsolation(_)
            ));
            assert_eq!(error.isolation_diagnostic(), Some(diagnostic));
            let success = NativeHarnessError::IsolationFailure(diagnostic).with_cleanup(true);
            assert!(matches!(success, NativeHarnessError::IsolationFailure(_)));
        }
        let (read, write) = file_pipe().unwrap();
        drop(write);
        assert_eq!(
            bounded_info(&read, 100)
                .unwrap_err()
                .isolation_diagnostic()
                .unwrap()
                .reason,
            IsolationReason::InfoEof
        );
        let (read, _write) = file_pipe().unwrap();
        assert_eq!(
            bounded_info(&read, 1)
                .unwrap_err()
                .isolation_diagnostic()
                .unwrap()
                .reason,
            IsolationReason::InfoTimeout
        );
        assert_eq!(
            bounded_info(&read, 0)
                .unwrap_err()
                .isolation_diagnostic()
                .unwrap()
                .reason,
            IsolationReason::InfoTimeout
        );
    }

    #[test]
    fn real_proxy_argv_requires_exact_pinned_interpreter_policy_and_bounds() {
        let selected = AccessIsolationConfig {
            helper_executable: "/helper".into(),
            helper_sha256: String::new(),
            proxy_socket: "/private/proxy/s".into(),
            proxy_policy_executable: "/private/real-policy.py".into(),
            proxy_policy_sha256: String::new(),
            source_home: "/private/source".into(),
            #[cfg(feature = "synthetic-access-test")]
            synthetic_http_socket: None,
            #[cfg(feature = "synthetic-access-test")]
            synthetic_http_port: None,
            #[cfg(feature = "synthetic-access-test")]
            synthetic_http_fixture_executable: None,
            #[cfg(feature = "synthetic-access-test")]
            synthetic_http_fixture_sha256: None,
        };
        let journal = Path::new("/private/run/proxy-metadata.jsonl");
        // Generated offline from the controller's pure argv producer using
        // synthetic paths. The projection battery checks producer parity.
        let produced: Vec<String> =
            serde_json::from_str(include_str!("../tests/fixtures/real_proxy_argv.json")).unwrap();
        let good: Vec<&[u8]> = produced.iter().map(|arg| arg.as_bytes()).collect();
        let check = |args: &[&[u8]]| {
            let result =
                validate_proxy_command(args, &selected, ProxyAdmission::Real, Some(journal));
            if let Err(ref error) = result {
                assert_eq!(
                    error.isolation_diagnostic().unwrap().reason,
                    IsolationReason::ProxyCmdline
                );
            }
            result.is_ok()
        };
        assert!(check(&good));
        assert!(!proxy_command_matches(
            &good,
            &selected,
            ProxyAdmission::Real,
            Some(Path::new("/wrong/proxy-metadata.jsonl"))
        ));
        for at in 0..good.len() {
            let mut bad = good.clone();
            bad[at] = b"invalid";
            assert!(!check(&bad));
        }
        for missing in 0..good.len() {
            let mut bad = good.clone();
            bad.remove(missing);
            assert!(!check(&bad));
        }
        for (at, value) in [
            (0, b"/bin/python3".as_slice()),
            (1, b"-S"),
            (2, b"-I"),
            (3, b"/wrong.py"),
            (5, b"/wrong/s"),
            (7, b"/wrong/journal.jsonl"),
            (9, b"synthetic"),
            (11, b"114"),
            (13, b"11"),
        ] {
            let mut bad = good.clone();
            bad[at] = value;
            assert!(!check(&bad));
        }
        let mut extra = good.clone();
        extra.extend([b"--mode".as_slice(), b"real"]);
        assert!(!check(&extra));
        let mut reordered = good.clone();
        reordered.swap(4, 6);
        assert!(!check(&reordered));
        assert!(strict_cmdline(b"python\0-I\0-S\0").is_some());
        assert!(strict_cmdline(b"python\0-I\0-S").is_none());
        assert!(strict_cmdline(b"python\0\0-S\0").is_none());
        assert!(!proc_exe_is(std::process::id(), "/usr/bin/python3"));
    }

    #[test]
    fn status_records_preserve_zero_nonzero_and_split_json() {
        for (wire, expected) in [
            (b"{\"child-pid\":2}\n{\"exit-code\":0}\n".as_slice(), 0),
            (
                b"{\n\"child-pid\":2\n}\n{\n\"exit-code\":111\n}\n".as_slice(),
                111,
            ),
        ] {
            let (read, write) = file_pipe().unwrap();
            for chunk in wire.chunks(3) {
                let n =
                    unsafe { libc::write(write.as_raw_fd(), chunk.as_ptr().cast(), chunk.len()) };
                assert_eq!(n, chunk.len() as isize);
            }
            drop(write);
            assert_eq!(status_exit_code(&read, 1000).unwrap(), Some(expected));
        }
    }

    #[test]
    fn status_records_reject_conflicting_exit_and_truncation() {
        for wire in [
            b"{\"exit-code\":0}\n{\"exit-code\":111}\n".as_slice(),
            b"{\"child-pid\":2}\n{\"exit-code\":".as_slice(),
        ] {
            let (read, write) = file_pipe().unwrap();
            let n = unsafe { libc::write(write.as_raw_fd(), wire.as_ptr().cast(), wire.len()) };
            assert_eq!(n, wire.len() as isize);
            drop(write);
            assert!(status_exit_code(&read, 1000).is_err());
        }
    }

    #[cfg(not(feature = "synthetic-access-test"))]
    #[tokio::test]
    async fn exact_static_helper_narrow_mount_preflight_and_reap() {
        let (Ok(native), Ok(helper), Ok(policy)) = (
            std::env::var("S10_NATIVE_BINARY"),
            std::env::var("S10_ISOLATION_HELPER"),
            std::env::var("S10_PROXY_POLICY_SCRIPT"),
        ) else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let root = temp.path().join("launch");
        let proxy = temp.path().join("proxy");
        for dir in [&source, &root, &proxy] {
            fs::create_dir(dir).unwrap();
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(source.join("auth.json"), b"synthetic-source-decoy-only").unwrap();
        for name in ["user", "codex", "work"] {
            let path = root.join(name);
            fs::create_dir(&path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(root.join("work/.context-lifecycle-project"), b"isolated\n").unwrap();
        fs::write(root.join("codex/config.toml"),b"model = \"gpt-6-sol\"\ncheck_for_update_on_startup = false\napproval_policy = \"never\"\nsandbox_mode = \"read-only\"\nweb_search = \"disabled\"\n[features]\nshell_tool = false\nimage_generation = false\ncode_mode_host = false\n").unwrap();
        fs::hard_link(source.join("auth.json"), proxy.join("decoy-alias")).unwrap();
        let sibling = UnixListener::bind(proxy.join("proxy.sock")).unwrap();
        let socket = proxy.join("selected.sock");
        let policy = PathBuf::from(policy);
        let mut proxy_process = StdCommand::new("/usr/bin/python3")
            .arg(&policy)
            .args([
                "--socket",
                socket.to_str().unwrap(),
                "--journal",
                temp.path().join("proxy-journal.jsonl").to_str().unwrap(),
                "--mode",
                "synthetic",
                "--test-target",
                "127.0.0.1:9",
                "--max-seconds",
                "30",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut ready = false;
        while std::time::Instant::now() < deadline {
            if let Ok(mut stream) = UnixStream::connect(&socket) {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
                if stream
                    .write_all(b"GET /__work_engine_ready__ HTTP/1.1\r\nHost: invalid\r\n\r\n")
                    .is_ok()
                {
                    let mut line = String::new();
                    if std::io::BufReader::new(stream).read_line(&mut line).is_ok()
                        && line == "HTTP/1.1 403 Forbidden\r\n"
                    {
                        ready = true;
                        break;
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(ready);
        let helper = PathBuf::from(helper);
        let selected = AccessIsolationConfig {
            helper_sha256: format!("{:x}", Sha256::digest(fs::read(&helper).unwrap())),
            helper_executable: helper,
            proxy_socket: socket,
            proxy_policy_executable: policy,
            proxy_policy_sha256: PROXY_POLICY_SHA256.into(),
            source_home: source.clone(),
            #[cfg(feature = "synthetic-access-test")]
            synthetic_http_socket: None,
            #[cfg(feature = "synthetic-access-test")]
            synthetic_http_port: None,
            #[cfg(feature = "synthetic-access-test")]
            synthetic_http_fixture_executable: None,
            #[cfg(feature = "synthetic-access-test")]
            synthetic_http_fixture_sha256: None,
        };
        let mut bad = selected.clone();
        bad.helper_sha256 = "0".repeat(64);
        assert!(matches!(
            spawn(
                Path::new(&native),
                &root,
                &root.join("work"),
                &root.join("user"),
                &root.join("codex"),
                "http://127.0.0.1:3128",
                &bad,
                ProxyAdmission::SyntheticTest,
            ),
            Err(NativeHarnessError::IsolationFailure(_))
        ));
        fs::hard_link(source.join("auth.json"), root.join("work/alias")).unwrap();
        assert!(matches!(
            spawn(
                Path::new(&native),
                &root,
                &root.join("work"),
                &root.join("user"),
                &root.join("codex"),
                "http://127.0.0.1:3128",
                &selected,
                ProxyAdmission::SyntheticTest,
            ),
            Err(NativeHarnessError::IsolationFailure(_))
        ));
        fs::remove_file(root.join("work/alias")).unwrap();
        let (mut pipe, mut isolated) = spawn(
            Path::new(&native),
            &root,
            &root.join("work"),
            &root.join("user"),
            &root.join("codex"),
            "http://127.0.0.1:3128",
            &selected,
            ProxyAdmission::SyntheticTest,
        )
        .unwrap();
        assert_ne!(pipe.pid(), Some(isolated.native_pid));
        assert!(nspid(isolated.native_pid).unwrap().len() >= 2);
        let child_proxy = PathBuf::from(format!("/proc/{}/root/proxy", isolated.native_pid));
        assert!(!child_proxy.join("decoy-alias").exists());
        assert!(!child_proxy.join("selected.sock").exists());
        assert!(
            fs::symlink_metadata(child_proxy.join("proxy.sock"))
                .unwrap()
                .file_type()
                .is_socket()
        );
        assert_eq!(
            fs::symlink_metadata(child_proxy.join("proxy.sock"))
                .unwrap()
                .ino(),
            fs::symlink_metadata(&selected.proxy_socket).unwrap().ino()
        );
        let environment = fs::read(format!("/proc/{}/environ", isolated.native_pid)).unwrap();
        let entries = environment.split(|byte| *byte == 0).collect::<Vec<_>>();
        assert!(entries.contains(&b"HTTPS_PROXY=http://127.0.0.1:3128".as_slice()));
        assert!(entries.contains(&b"NO_PROXY=".as_slice()));
        for forbidden in [
            b"OPENAI_API_KEY=".as_slice(),
            b"ALL_PROXY=".as_slice(),
            b"HTTP_PROXY=".as_slice(),
        ] {
            assert!(!entries.iter().any(|entry| entry.starts_with(forbidden)));
        }
        pipe.send(br#"{"id":1,"method":"initialize","params":{"clientInfo":{"name":"work_engine_isolation_test","version":"0.1.0"},"capabilities":{"experimentalApi":true}}}"#).await.unwrap();
        let (_, reply) = pipe.receive().await.unwrap().unwrap();
        assert!(String::from_utf8_lossy(&reply).contains("work_engine_isolation_test"));
        assert!(isolated.route_current());
        proxy_process.kill().unwrap();
        proxy_process.wait().unwrap();
        assert!(!isolated.route_current());
        let exit = isolated.terminate_and_reap(&mut pipe).await.unwrap();
        assert_eq!(exit.pid, Some(isolated.native_pid));
        assert!(source.join("auth.json").exists());
        drop(sibling);
    }
}
