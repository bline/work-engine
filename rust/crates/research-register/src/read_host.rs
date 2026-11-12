//! Linux enclosing host. Only complete, pinned worker frames become visible results.
use crate::{
    GitObjects, Outcome, PinnedSnapshot,
    bounded_read::ReadProfile,
    use_bundle::{BlobPin, validate_use_bundle},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadRequest {
    pub repository: PathBuf,
    pub commit: String,
    pub register_path: String,
    pub bundle: BlobPin,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostProfile {
    pub repository_scope: Option<PathBuf>,
    pub read: ReadProfile,
    pub cpu_seconds: u64,
    pub wall_millis: u64,
    pub memory_bytes: u64,
    pub output_bytes: usize,
}
impl Default for HostProfile {
    fn default() -> Self {
        Self {
            repository_scope: None,
            read: ReadProfile::default(),
            cpu_seconds: 60,
            wall_millis: 60000,
            memory_bytes: 1 << 30,
            output_bytes: 8 << 20,
        }
    }
}
impl HostProfile {
    pub fn validate(&self) -> Result<(), String> {
        self.read.validate().map_err(|e| e.to_string())?;
        if self.cpu_seconds == 0
            || self.cpu_seconds > 60
            || self.wall_millis == 0
            || self.wall_millis > 60000
            || self.memory_bytes == 0
            || self.memory_bytes > 1 << 30
            || self.output_bytes == 0
            || self.output_bytes > 8 << 20
        {
            return Err("invalid restrictive host profile".into());
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerInput {
    pub request: Option<ReadRequest>,
    pub profile: HostProfile,
    pub producer: String,
    pub probe: Option<String>,
}
pub fn producer(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn worker(input: WorkerInput) -> Result<serde_json::Value, String> {
    input.profile.validate()?;
    #[cfg(target_os = "linux")]
    {
        let cg = std::fs::read_to_string("/proc/self/cgroup").map_err(|e| e.to_string())?;
        let rel = cg
            .lines()
            .find_map(|l| l.strip_prefix("0::"))
            .ok_or("ControlsUnavailable")?;
        let path = Path::new("/sys/fs/cgroup").join(rel.trim_start_matches('/'));
        let mem: u64 = std::fs::read_to_string(path.join("memory.max"))
            .map_err(|e| e.to_string())?
            .trim()
            .parse()
            .map_err(|_| "ControlsUnavailable")?;
        let mut cpu = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        let ok = unsafe { libc::getrlimit(libc::RLIMIT_CPU, &mut cpu) };
        if mem > input.profile.memory_bytes || ok != 0 || cpu.rlim_max > input.profile.cpu_seconds {
            return Err("ControlsUnavailable".into());
        }
    }
    // The parent executes its hash-verified sealed image; echo that bound identity.

    let register_only = input.probe.as_deref() == Some("register-only");
    if let Some(probe) = input.probe
        && !register_only
    {
        match probe.as_str() {
            "cpu" => loop {
                std::hint::black_box(17u64.wrapping_mul(19));
            },
            "wall" => loop {
                std::thread::sleep(Duration::from_secs(1));
            },
            "memory" => {
                let mut blocks = Vec::new();
                loop {
                    let mut block = vec![0u8; 1 << 20];
                    for b in &mut block {
                        *b = std::hint::black_box(1);
                    }
                    blocks.push(block);
                    std::hint::black_box(&blocks);
                }
            }
            "output" => loop {
                std::io::stdout()
                    .write_all(&[b'x'; 8192])
                    .map_err(|e| e.to_string())?;
            },
            "crash" => std::process::abort(),
            "truncated" => {
                std::io::stdout()
                    .write_all(b"RREAD1\n100\n{}")
                    .map_err(|e| e.to_string())?;
                std::process::exit(0);
            }
            "success" => {
                return Ok(
                    serde_json::json!({"probe":"success","producer":input.producer,"outcome":"complete"}),
                );
            }
            _ => return Err("unknown qualification probe".into()),
        }
    }
    let r = input.request.ok_or("missing read request")?;
    if r.commit.len() != 40
        || !r
            .commit
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("exact lowercase commit required".into());
    }
    let scope = input
        .profile
        .repository_scope
        .as_ref()
        .ok_or("ScopeUnavailable")?;
    let admitted = admit_repository(scope, &r.repository)?;
    let objects = open_admitted_repository(&admitted, input.profile.read.clone())?;
    let snapshot =
        PinnedSnapshot::build(objects, &r.commit, &r.register_path).map_err(|e| e.to_string())?;
    let valid = match crate::validate(&snapshot).map_err(|e| e.to_string())? {
        Outcome::Valid(v) => v,
        Outcome::Invalid(_) => return Err("InvalidRegister".into()),
    };
    if valid.records().decisions.len() > input.profile.read.decisions {
        return Err("DecisionLimit".into());
    }
    if register_only {
        let anchors = crate::anchors(&valid, &snapshot).map_err(|e| e.to_string())?;
        return Ok(
            serde_json::json!({"schema":"research-read-qualification-v1","outcome":"control_complete","probe":"register-only","m1_bundle":"NOT_EXERCISED","producer":input.producer,"commit":snapshot.commit().to_string(),"decisions":valid.records().decisions.len(),"accounting":snapshot.objects().read_accounting(),"profile_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&input.profile).map_err(|e|e.to_string())?)),"anchors":anchors.to_json()}),
        );
    }
    let bundle =
        validate_use_bundle(&snapshot, &valid, r.bundle.clone()).map_err(|e| e.to_string())?;
    let anchors = crate::anchors(&valid, &snapshot).map_err(|e| e.to_string())?;
    Ok(
        serde_json::json!({"schema":"research-read-v1","outcome":"complete","producer":input.producer,"commit":snapshot.commit().to_string(),"register_tree":snapshot.register_tree().to_string(),"bundle":bundle.manifest_pin(),"anchors":anchors.to_json(),"accounting":snapshot.objects().read_accounting(),"profile_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&input.profile).map_err(|e|e.to_string())?))}),
    )
}
fn admit_repository(scope: &Path, requested: &Path) -> Result<PathBuf, String> {
    let admitted = std::fs::canonicalize(requested).map_err(|e| e.to_string())?;
    if std::fs::canonicalize(scope).map_err(|e| e.to_string())? != admitted {
        return Err("ScopeUnavailable".into());
    }
    Ok(admitted)
}
fn open_admitted_repository(admitted: &Path, profile: ReadProfile) -> Result<GitObjects, String> {
    GitObjects::open_bounded(admitted, profile).map_err(|e| e.to_string())
}
pub fn frame(value: &serde_json::Value) -> Result<Vec<u8>, String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let mut out = format!("RREAD1\n{}\n", bytes.len()).into_bytes();
    out.extend(bytes);
    Ok(out)
}
fn unframe(bytes: &[u8]) -> Result<serde_json::Value, String> {
    let rest = bytes.strip_prefix(b"RREAD1\n").ok_or("Frame")?;
    let end = rest.iter().position(|b| *b == b'\n').ok_or("Frame")?;
    if end > 12 {
        return Err("Frame".into());
    }
    let n: usize = std::str::from_utf8(&rest[..end])
        .map_err(|_| "Frame")?
        .parse()
        .map_err(|_| "Frame")?;
    let payload = &rest[end + 1..];
    if payload.len() != n {
        return Err("Frame".into());
    }
    serde_json::from_slice(payload).map_err(|_| "Frame".into())
}
#[cfg(target_os = "linux")]
pub fn run(
    executable: &Path,
    cgroup_root: &Path,
    input: WorkerInput,
    cancel: &AtomicBool,
) -> Result<serde_json::Value, String> {
    run_started(executable, cgroup_root, input, cancel, Instant::now())
}
#[cfg(target_os = "linux")]
pub fn run_started(
    executable: &Path,
    cgroup_root: &Path,
    input: WorkerInput,
    cancel: &AtomicBool,
    start: Instant,
) -> Result<serde_json::Value, String> {
    use std::os::unix::process::CommandExt;
    input.profile.validate()?;
    if cancel.load(Ordering::SeqCst) {
        return Err("Cancelled".into());
    }
    if start.elapsed() >= Duration::from_millis(input.profile.wall_millis) {
        return Err("WallLimit".into());
    }
    // Execute a sealed private copy: a pathname replacement cannot switch the admitted image.
    use std::os::fd::{AsRawFd, FromRawFd};
    let name = std::ffi::CString::new("research-read-image").unwrap();
    let fd =
        unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    if fd < 0 {
        return Err(format!(
            "ControlsUnavailable: {}",
            std::io::Error::last_os_error()
        ));
    }
    let mut image = unsafe { std::fs::File::from_raw_fd(fd) };
    let mut source = std::fs::File::open(executable).map_err(|e| e.to_string())?;
    let n = std::io::copy(
        &mut std::io::Read::by_ref(&mut source).take((128 << 20) + 1),
        &mut image,
    )
    .map_err(|e| e.to_string())?;
    if n > 128 << 20 {
        return Err("ExecutableLimit".into());
    }
    if unsafe {
        libc::fcntl(
            image.as_raw_fd(),
            libc::F_ADD_SEALS,
            libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL,
        )
    } < 0
    {
        return Err("ControlsUnavailable: image seal".into());
    }
    let image_path = PathBuf::from(format!("/proc/self/fd/{}", image.as_raw_fd()));
    let expected = producer(&image_path)?;
    if expected != input.producer {
        return Err("ProducerMismatch".into());
    }
    let encoded = serde_json::to_vec(&input).map_err(|e| e.to_string())?;
    if encoded.len() > 16384 {
        return Err("ControlInputLimit".into());
    }
    let group = cgroup_root.join(format!(
        "research-read-{}-{}",
        std::process::id(),
        SystemNonce::next()
    ));
    std::fs::create_dir(&group).map_err(|e| format!("ControlsUnavailable: {e}"))?;
    struct Group(PathBuf);
    impl Drop for Group {
        fn drop(&mut self) {
            let _ = std::fs::write(self.0.join("cgroup.kill"), "1");
            let _ = std::fs::remove_dir(&self.0);
        }
    }
    let guard = Group(group.clone());
    for (name, value) in [
        ("memory.max", input.profile.memory_bytes.to_string()),
        ("memory.swap.max", "0".into()),
        ("memory.oom.group", "1".into()),
        ("pids.max", "16".into()),
    ] {
        std::fs::write(group.join(name), value).map_err(|e| format!("ControlsUnavailable: {e}"))?;
    }
    if std::fs::read_to_string(group.join("memory.max"))
        .map_err(|e| e.to_string())?
        .trim()
        != input.profile.memory_bytes.to_string()
    {
        return Err("ControlsUnavailable".into());
    }
    let procs = std::ffi::CString::new(group.join("cgroup.procs").as_os_str().as_encoded_bytes())
        .map_err(|e| e.to_string())?;
    let cpu = input.profile.cpu_seconds;
    let mut command = std::process::Command::new(&image_path);
    command
        .arg("--private-worker-v1")
        .env_clear()
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // Only async-signal-safe syscalls occur between fork and exec. Object opening is worker-only.
    unsafe {
        command.pre_exec(move || {
            let lim = libc::rlimit {
                rlim_cur: cpu,
                rlim_max: cpu,
            };
            let core = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::setrlimit(libc::RLIMIT_CORE, &core) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::setrlimit(libc::RLIMIT_CPU, &lim) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            let fd = libc::open(procs.as_ptr(), libc::O_WRONLY | libc::O_CLOEXEC);
            if fd < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let n = libc::write(fd, b"0".as_ptr().cast(), 1);
            libc::close(fd);
            if n != 1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("ControlsUnavailable: {e}"))?;
    let total = Arc::new(AtomicUsize::new(0));
    let overflow = Arc::new(AtomicBool::new(false));
    fn drain(
        mut rd: impl Read + Send + 'static,
        total: Arc<AtomicUsize>,
        overflow: Arc<AtomicBool>,
        cap: usize,
    ) -> std::thread::JoinHandle<Result<Vec<u8>, String>> {
        std::thread::spawn(move || {
            let mut out = Vec::new();
            let mut buf = [0; 8192];
            loop {
                let n = rd.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                let before = total.fetch_add(n, Ordering::SeqCst);
                if before.saturating_add(n) > cap {
                    overflow.store(true, Ordering::SeqCst);
                } else {
                    out.extend_from_slice(&buf[..n]);
                }
            }
            Ok(out)
        })
    }
    let stdout = drain(
        child.stdout.take().unwrap(),
        total.clone(),
        overflow.clone(),
        input.profile.output_bytes,
    );
    let stderr = drain(
        child.stderr.take().unwrap(),
        total,
        overflow.clone(),
        input.profile.output_bytes,
    );
    let mut stdin = child.stdin.take().unwrap();
    let sender = std::thread::spawn(move || stdin.write_all(&encoded));
    let mut refused = None;
    let status = loop {
        if cancel.load(Ordering::SeqCst) {
            refused = Some("Cancelled".into());
        } else if overflow.load(Ordering::SeqCst) {
            refused = Some("OutputLimit".into());
        } else if start.elapsed() > Duration::from_millis(input.profile.wall_millis) {
            refused = Some("WallLimit".into());
        }
        if refused.is_some() {
            let _ = std::fs::write(group.join("cgroup.kill"), "1");
            let _ = child.kill();
            break child.wait().map_err(|e| e.to_string())?;
        }
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(s) => break s,
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    // Kill/drain owned descendants even if the main process exits before them.
    let _ = std::fs::write(group.join("cgroup.kill"), "1");
    let sent = sender.join().map_err(|_| "ControlDelivery")?;
    let out = stdout.join().map_err(|_| "Drain")??;
    let _err = stderr.join().map_err(|_| "Drain")??;
    let oom = std::fs::read_to_string(group.join("memory.events"))
        .unwrap_or_default()
        .lines()
        .any(|line| line.strip_prefix("oom_kill ").is_some_and(|n| n != "0"));
    if cancel.load(Ordering::SeqCst) {
        return Err("Cancelled".into());
    }
    if overflow.load(Ordering::SeqCst) {
        return Err("OutputLimit".into());
    }
    if let Some(e) = refused {
        return Err(e);
    }
    if oom {
        return Err("MemoryLimit".into());
    }
    if let Err(error) = sent {
        return Err(format!("ControlDelivery: {error}"));
    }
    if !status.success() {
        return Err(format!("WorkerExit: {status}"));
    }
    let report = unframe(&out)?;
    if report.get("producer").and_then(|v| v.as_str()) == Some(expected.as_str())
        && report.get("outcome").and_then(|v| v.as_str()) == Some("refused")
    {
        return Err(format!(
            "ReaderRefusal: {}",
            report
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
        ));
    }
    let expected_outcome = if input.probe.as_deref() == Some("register-only") {
        "control_complete"
    } else {
        "complete"
    };
    if report.get("producer").and_then(|v| v.as_str()) != Some(expected.as_str())
        || report.get("outcome").and_then(|v| v.as_str()) != Some(expected_outcome)
    {
        return Err("OutcomeOrProducerMismatch".into());
    }
    if let Some(r) = input.request {
        let profile_pin = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&input.profile).map_err(|e| e.to_string())?)
        );
        if report.get("profile_sha256").and_then(|v| v.as_str()) != Some(profile_pin.as_str()) {
            return Err("ProfileMismatch".into());
        }
        if report.get("commit").and_then(|v| v.as_str()) != Some(r.commit.as_str())
            || (input.probe.as_deref() != Some("register-only")
                && report.get("bundle")
                    != Some(&serde_json::to_value(r.bundle).map_err(|e| e.to_string())?))
        {
            return Err("PinMismatch".into());
        }
    }
    drop(guard);
    Ok(report)
}
struct SystemNonce;
impl SystemNonce {
    fn next() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    }
}
#[cfg(not(target_os = "linux"))]
pub fn run(
    _: &Path,
    _: &Path,
    _: WorkerInput,
    _: &AtomicBool,
) -> Result<serde_json::Value, String> {
    Err("UnsupportedPlatform".into())
}

#[cfg(not(target_os = "linux"))]
pub fn run_started(
    _: &Path,
    _: &Path,
    _: WorkerInput,
    _: &AtomicBool,
    _: Instant,
) -> Result<serde_json::Value, String> {
    Err("UnsupportedPlatform".into())
}
#[cfg(all(test, unix))]
mod scope_tests {
    use super::*;
    #[test]
    fn retargeting_request_symlink_preserves_admitted_repository() {
        let root = std::env::temp_dir().join(format!(
            "research-scope-{}-{}",
            std::process::id(),
            SystemNonce::next()
        ));
        let a = root.join("a");
        let b = root.join("b");
        let link = root.join("request");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        for dir in [&a, &b] {
            assert!(
                std::process::Command::new("git")
                    .args(["init", "-q"])
                    .arg(dir)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        std::os::unix::fs::symlink(&a, &link).unwrap();
        let admitted = admit_repository(&a, &link).unwrap();
        assert_eq!(
            open_admitted_repository(&admitted, ReadProfile::default())
                .unwrap()
                .path(),
            a.to_str().unwrap()
        );
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&b, &link).unwrap();
        assert_eq!(
            open_admitted_repository(&admitted, ReadProfile::default())
                .unwrap()
                .path(),
            a.to_str().unwrap()
        );
        assert_eq!(
            open_admitted_repository(&link, ReadProfile::default())
                .unwrap()
                .path(),
            b.to_str().unwrap()
        ); // old reopening route follows unauthorized target
        assert!(admit_repository(&a, &link).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
