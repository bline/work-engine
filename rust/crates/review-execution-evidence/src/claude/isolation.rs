use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::profile::sha256;
use super::{Rp0Error, TrustedClaudeProfile, TrustedRp0Profile};

const MAX_PROCESS_OUTPUT: usize = 1024 * 1024;
const REVIEW_ROOT: &str = "/repo/candidate";

#[derive(Clone, Debug)]
pub struct CandidateIndex {
    project: String,
    candidate_tree_sha256: String,
    store_sha256: String,
    coverage_sha256: String,
    coverage_complete: bool,
    config_db_sha256: String,
    mcp_config_sha256: String,
    indexed_at: String,
    nodes: u64,
    edges: u64,
}

impl CandidateIndex {
    pub fn project(&self) -> &str {
        &self.project
    }
    pub fn candidate_tree_sha256(&self) -> &str {
        &self.candidate_tree_sha256
    }
    pub fn store_sha256(&self) -> &str {
        &self.store_sha256
    }
    pub fn coverage_sha256(&self) -> &str {
        &self.coverage_sha256
    }
    pub fn coverage_complete(&self) -> bool {
        self.coverage_complete
    }
    pub fn indexed_at(&self) -> &str {
        &self.indexed_at
    }
    pub fn nodes(&self) -> u64 {
        self.nodes
    }
    pub fn edges(&self) -> u64 {
        self.edges
    }
}

pub struct HostileProbeResult {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub cbm_observation: Value,
}

pub(crate) struct QualifiedLaunch {
    pub command: tokio::process::Command,
    pub ready_line: String,
    pub relay: QualifiedRelay,
}

pub(crate) struct QualifiedRelay {
    cancel: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<Result<Value, Rp0Error>>>,
    socket_path: PathBuf,
    cgroup: Option<PathBuf>,
}

impl QualifiedRelay {
    pub fn finish(mut self) -> Result<Value, Rp0Error> {
        self.cancel.store(true, Ordering::SeqCst);
        let thread = self.thread.take().ok_or(Rp0Error::IsolationFailed)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while !thread.is_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        if !thread.is_finished() {
            return Err(Rp0Error::IsolationFailed);
        }
        let result = thread.join().map_err(|_| Rp0Error::IsolationFailed)?;
        let _ = fs::remove_file(&self.socket_path);
        self.release_cgroup();
        result
    }
}

impl Drop for QualifiedRelay {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !thread.is_finished() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            if thread.is_finished() {
                let _ = thread.join();
            }
        }
        let _ = fs::remove_file(&self.socket_path);
        self.release_cgroup();
    }
}

impl QualifiedRelay {
    fn release_cgroup(&mut self) {
        if let Some(path) = self.cgroup.take() {
            // Kill only this attempt's private child group. If the kernel has
            // not reaped it yet, leave the hard quota in force.
            let _ = fs::write(path.join("cgroup.kill"), b"1\n");
            let deadline = Instant::now() + Duration::from_secs(2);
            while fs::remove_dir(&path).is_err() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn active_cgroup(
    nonce: &str,
    budget: u64,
    live_session: &Path,
) -> Result<(PathBuf, String), Rp0Error> {
    let membership = fs::read_to_string("/proc/self/cgroup")?;
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .ok_or(Rp0Error::IsolationFailed)?;
    let current = Path::new(relative);
    let parent = current.parent().ok_or(Rp0Error::IsolationFailed)?;
    if !current.is_absolute()
        || current
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(Rp0Error::IsolationFailed);
    }
    // A nonce identifies one attempt within an evidence root; independent
    // roots can use the same profile and attempt number. Bind the private
    // group name to its canonical live root so a delayed cleanup cannot
    // cause another root's launch to collide with this group's pathname.
    let live_digest = sha256(live_session.as_os_str().as_encoded_bytes());
    let group_name = format!("rp1-{}-{}", nonce, &live_digest[..16]);
    let group_relative = parent.join(group_name);
    let path = Path::new("/sys/fs/cgroup").join(
        group_relative
            .strip_prefix("/")
            .map_err(|_| Rp0Error::IsolationFailed)?,
    );
    fs::create_dir(&path)?;
    let setup = (|| -> Result<(), Rp0Error> {
        // memory.max excludes swapped pages. This empty private group must
        // prohibit swap before any child enters it so resident and swapped
        // charges cannot exceed the remaining aggregate session budget.
        fs::write(path.join("memory.swap.max"), b"0\n")?;
        if fs::read_to_string(path.join("memory.swap.max"))?.trim() != "0"
            || fs::read_to_string(path.join("memory.swap.current"))?.trim() != "0"
        {
            return Err(Rp0Error::IsolationFailed);
        }
        fs::write(path.join("memory.max"), format!("{budget}\n"))?;
        if fs::read_to_string(path.join("memory.max"))?.trim() != budget.to_string() {
            return Err(Rp0Error::IsolationFailed);
        }
        Ok(())
    })();
    if let Err(error) = setup {
        let _ = fs::remove_dir(&path);
        return Err(error);
    }
    Ok((path, group_relative.to_string_lossy().into_owned()))
}

fn enter_cgroup_before_exec(command: &mut Command, group: &Path) -> Result<(), Rp0Error> {
    let procs = CString::new(group.join("cgroup.procs").as_os_str().as_encoded_bytes())
        .map_err(|_| Rp0Error::IsolationFailed)?;
    // SAFETY: this hook runs between fork and exec. It uses only async-signal-
    // safe libc calls and writes this child PID into this attempt's cgroup.
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(move || {
            let fd = libc::open(procs.as_ptr(), libc::O_WRONLY | libc::O_CLOEXEC);
            if fd < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let mut pid = libc::getpid() as u32;
            let mut digits = [0u8; 16];
            let mut cursor = digits.len() - 1;
            digits[cursor] = b'\n';
            loop {
                cursor -= 1;
                digits[cursor] = b'0' + (pid % 10) as u8;
                pid /= 10;
                if pid == 0 {
                    break;
                }
            }
            let bytes = &digits[cursor..];
            let written = libc::write(fd, bytes.as_ptr().cast(), bytes.len());
            let error = std::io::Error::last_os_error();
            libc::close(fd);
            if written != bytes.len() as isize {
                return Err(error);
            }
            Ok(())
        });
    }
    Ok(())
}

/// RP0 owns this private build, sealed query copy, and realized reviewer mount.
/// It neither constructs E1 qualified evidence nor launches a provider.
pub struct CandidateBoundary {
    profile: TrustedRp0Profile,
    candidate: PathBuf,
    private_root: PathBuf,
    index: CandidateIndex,
}

impl CandidateBoundary {
    pub(crate) fn guarded_launch(
        &self,
        claude: &TrustedClaudeProfile,
        live_session: &Path,
        nonce: &str,
        args: &[String],
    ) -> Result<QualifiedLaunch, Rp0Error> {
        self.verify_frozen()?;
        claude.verify()?;
        if nonce.len() != 64 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Rp0Error::IsolationFailed);
        }
        let guard = claude.guard().ok_or(Rp0Error::IsolationFailed)?;
        let live = fs::canonicalize(live_session)?;
        if live != live_session || !live.is_dir() {
            return Err(Rp0Error::IsolationFailed);
        }
        let socket_path = self.private_root.join("proxy/analysis.sock");
        let listener = UnixListener::bind(&socket_path)?;
        let cancel = Arc::new(AtomicBool::new(false));
        let server_cancel = cancel.clone();
        let profile = self.profile.clone();
        let candidate = self.candidate.clone();
        let private_root = self.private_root.clone();
        let project = self.index.project.clone();
        let server = thread::spawn(move || {
            serve_one(
                listener,
                &profile,
                &candidate,
                &private_root,
                &project,
                Some(server_cancel),
            )
        });
        let relay = QualifiedRelay {
            cancel,
            thread: Some(server),
            socket_path,
            cgroup: None,
        };
        let mut command =
            reviewer_command(&self.profile, &self.candidate, &self.private_root, false)?;
        // The reviewer sees only a size-limited private tmpfs. The pinned
        // guard exports its completed session through a bounded frame after
        // the reviewer exits; no writable host custody path is mounted.
        let budget = super::session::active_cgroup_budget(
            live_session.parent().ok_or(Rp0Error::IsolationFailed)?,
        )
        .map_err(|_| Rp0Error::IsolationFailed)?;
        let (quota_path, quota_relative) = active_cgroup(nonce, budget, &live)?;
        let mut relay = relay;
        relay.cgroup = Some(quota_path.clone());
        // The pre-exec hook places Bubblewrap and every descendant in the
        // kernel hard limit before the first instruction of the sandbox.
        enter_cgroup_before_exec(&mut command, &quota_path)?;
        for (path, size) in [
            ("/cache", 4 * 1024 * 1024),
            ("/runtime", 4 * 1024 * 1024),
            ("/scratch", 4 * 1024 * 1024),
            ("/tmp", 4 * 1024 * 1024),
            ("/session", super::session::ACTIVE_SESSION_LIMIT),
        ] {
            command
                .arg("--size")
                .arg(size.to_string())
                .arg("--tmpfs")
                .arg(path);
        }
        command.arg("--dir").arg("/cbm");
        command.arg("--proc").arg("/proc");
        command.arg("--ro-bind").arg(&quota_path).arg("/quota");
        command
            .arg("--ro-bind")
            .arg(self.private_root.join("proxy"))
            .arg("/cbm/socket");
        command
            .arg("--ro-bind")
            .arg(self.profile.cbm_client.path())
            .arg("/cbm/client");
        command
            .arg("--ro-bind")
            .arg(self.private_root.join("session/mcp.json"))
            .arg("/cbm/mcp.json");
        command.arg("--dir").arg("/claude");
        command
            .arg("--ro-bind")
            .arg(claude.program().path())
            .arg("/claude/bin");
        command
            .arg("--ro-bind")
            .arg(guard.path())
            .arg("/claude/guard");
        // Bubblewrap's implicit root and its fresh /dev otherwise admit
        // unbounded regular files outside the charged /session tmpfs.
        // Remount after every path has been installed; /session and the small
        // scratch tmpfs mounts remain separate writable child mounts.
        command.args([
            "--remount-ro",
            "/dev",
            "--remount-ro",
            "/proc",
            "--remount-ro",
            "/",
        ]);
        command.args([
            "--setenv",
            "WORK_ENGINE_QUOTA_REL",
            &quota_relative,
            "--setenv",
            "WORK_ENGINE_QUOTA_BYTES",
            &budget.to_string(),
        ]);
        command.args([
            "--setenv",
            "HOME",
            "/session",
            "--setenv",
            "CLAUDE_CONFIG_DIR",
            "/session",
            "--setenv",
            "PATH",
            "/usr/bin:/bin",
            "--",
            "/claude/guard",
            nonce,
            "/claude/bin",
            if claude.installed() {
                "installed"
            } else {
                "offline"
            },
            self.index.project.as_str(),
        ]);
        command.args(args);
        Ok(QualifiedLaunch {
            command: tokio::process::Command::from(command),
            ready_line: format!("READY:{nonce}:bwrap-candidate-ro-v2\n"),
            relay,
        })
    }
    pub fn prepare(
        profile: TrustedRp0Profile,
        candidate: impl AsRef<Path>,
        private_root: impl AsRef<Path>,
        project: &str,
    ) -> Result<Self, Rp0Error> {
        profile.verify()?;
        if project.is_empty()
            || project.len() > 64
            || !project
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(Rp0Error::IndexUnavailable);
        }
        let candidate = fs::canonicalize(candidate)?;
        if !candidate.is_dir() {
            return Err(Rp0Error::CandidateDrift);
        }
        let candidate_tree_sha256 = tree_digest(&candidate)?;
        let private_root = private_root.as_ref();
        if private_root.exists() {
            return Err(Rp0Error::UnsafePrivateRoot);
        }
        let private_parent =
            fs::canonicalize(private_root.parent().ok_or(Rp0Error::UnsafePrivateRoot)?)?;
        if private_parent.starts_with(&candidate) {
            return Err(Rp0Error::UnsafePrivateRoot);
        }
        fs::create_dir(private_root)?;
        fs::set_permissions(private_root, fs::Permissions::from_mode(0o700))?;
        let private_root = fs::canonicalize(private_root)?;
        for name in [
            "build-cache",
            "build-runtime",
            "review-cache",
            "review-runtime",
            "session",
            "scratch",
            "proxy",
        ] {
            let path = private_root.join(name);
            fs::create_dir(&path)?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
        }
        let config = b"{\"ui_enabled\":false,\"ui_port\":9749}\n";
        write_private(&private_root.join("build-cache/config.json"), config)?;
        for setting in ["auto_index", "auto_watch", "watcher_enabled"] {
            let mut command = base_command(
                &profile,
                &candidate,
                &private_root,
                "build-cache",
                "build-runtime",
                true,
            )?;
            command.args(["--", "/cbm/bin", "config", "set", setting, "false"]);
            if !bounded_output(command, Duration::from_secs(30))?
                .status
                .success()
            {
                return Err(Rp0Error::IndexUnavailable);
            }
        }
        let mut build = base_command(
            &profile,
            &candidate,
            &private_root,
            "build-cache",
            "build-runtime",
            true,
        )?;
        build.args([
            "--",
            "/cbm/bin",
            "cli",
            "index_repository",
            "--repo-path",
            REVIEW_ROOT,
            "--name",
            project,
            "--mode",
            "full",
            "--persistence",
            "false",
        ]);
        let output = bounded_output(build, Duration::from_secs(180))?;
        if !output.status.success() {
            return Err(Rp0Error::IndexUnavailable);
        }
        let result: Value =
            serde_json::from_slice(&output.stdout).map_err(|_| Rp0Error::IndexUnavailable)?;
        if result.get("project").and_then(Value::as_str) != Some(project)
            || result.get("status").and_then(Value::as_str) != Some("indexed")
        {
            return Err(Rp0Error::IndexUnavailable);
        }
        if candidate_tree_sha256 != tree_digest(&candidate)? {
            return Err(Rp0Error::CandidateDrift);
        }

        let store_name = format!("{project}.db");
        let build_store = private_root.join("build-cache").join(&store_name);
        reject_sidecars(&private_root.join("build-cache"), &store_name)?;
        verify_store(&build_store)?;
        for name in [&store_name[..], "config.json", "_config.db"] {
            let source = private_root.join("build-cache").join(name);
            let target = private_root.join("review-cache").join(name);
            if !source.is_file() {
                return Err(Rp0Error::IncompleteStore);
            }
            fs::copy(source, &target)?;
            fs::set_permissions(&target, fs::Permissions::from_mode(0o600))?;
        }
        fs::create_dir(private_root.join("review-cache/logs"))?;
        let review_store = private_root.join("review-cache").join(&store_name);
        verify_store(&review_store)?;
        let store_sha256 = sha256(&fs::read(&review_store)?);
        if store_sha256 != sha256(&fs::read(&build_store)?) {
            return Err(Rp0Error::IncompleteStore);
        }

        let mut readback = base_command(
            &profile,
            &candidate,
            &private_root,
            "review-cache",
            "review-runtime",
            true,
        )?;
        seal_store_mounts(&mut readback, &private_root, project);
        readback.args([
            "--",
            "/cbm/bin",
            "cli",
            "index_status",
            "--project",
            project,
            "--diagnostics",
            "none",
            "--format",
            "json",
        ]);
        let output = bounded_output(readback, Duration::from_secs(30))?;
        if !output.status.success() {
            return Err(Rp0Error::IncompleteStore);
        }
        let status: Value =
            serde_json::from_slice(&output.stdout).map_err(|_| Rp0Error::IncompleteStore)?;
        if status.get("project").and_then(Value::as_str) != Some(project)
            || status.get("status").and_then(Value::as_str) != Some("ready")
            || status.get("root_path").and_then(Value::as_str) != Some(REVIEW_ROOT)
            || status.pointer("/watch/watched").and_then(Value::as_bool) != Some(false)
        {
            return Err(Rp0Error::IncompleteStore);
        }
        let indexed_at = status
            .get("indexed_at")
            .and_then(Value::as_str)
            .ok_or(Rp0Error::IncompleteStore)?
            .to_owned();
        let nodes = status
            .get("nodes")
            .and_then(Value::as_u64)
            .ok_or(Rp0Error::IncompleteStore)?;
        let edges = status
            .get("edges")
            .and_then(Value::as_u64)
            .ok_or(Rp0Error::IncompleteStore)?;
        if store_sha256 != sha256(&fs::read(&review_store)?)
            || candidate_tree_sha256 != tree_digest(&candidate)?
        {
            return Err(Rp0Error::IncompleteStore);
        }
        let mut coverage_batches = Vec::new();
        let mut coverage_complete = true;
        for paths in candidate_relative_paths(&candidate)?.chunks(20) {
            let paths_json =
                serde_json::to_string(paths).map_err(|_| Rp0Error::IndexUnavailable)?;
            let mut check = base_command(
                &profile,
                &candidate,
                &private_root,
                "review-cache",
                "review-runtime",
                true,
            )?;
            seal_store_mounts(&mut check, &private_root, project);
            check.args([
                "--",
                "/cbm/bin",
                "cli",
                "check_index_coverage",
                "--project",
                project,
                "--paths",
                &paths_json,
                "--path-limit",
                "20",
                "--diagnostics",
                "none",
                "--format",
                "json",
            ]);
            let output = bounded_output(check, Duration::from_secs(30))?;
            if !output.status.success() {
                return Err(Rp0Error::IndexUnavailable);
            }
            let batch: Value =
                serde_json::from_slice(&output.stdout).map_err(|_| Rp0Error::IndexUnavailable)?;
            if batch.get("project").and_then(Value::as_str) != Some(project)
                || batch.get("path_total").and_then(Value::as_u64) != Some(paths.len() as u64)
                || batch.get("path_returned").and_then(Value::as_u64) != Some(paths.len() as u64)
                || batch
                    .pointer("/metadata/generation_matches")
                    .and_then(Value::as_bool)
                    != Some(true)
            {
                return Err(Rp0Error::IndexUnavailable);
            }
            let records = batch
                .get("paths")
                .and_then(Value::as_array)
                .ok_or(Rp0Error::IndexUnavailable)?;
            coverage_complete &= records.iter().all(|entry| {
                entry.get("status").and_then(Value::as_str) == Some("no_recorded_issue")
                    && entry.get("content_verified").and_then(Value::as_bool) == Some(true)
            });
            coverage_batches.push(batch);
        }
        let coverage_manifest = json!({"project": project, "candidate_tree_sha256":candidate_tree_sha256,
            "store_sha256":store_sha256,"coverage_complete":coverage_complete,
            "batches":coverage_batches});
        let coverage_bytes =
            serde_json::to_vec(&coverage_manifest).map_err(|_| Rp0Error::IndexUnavailable)?;
        write_private(&private_root.join("session/coverage.json"), &coverage_bytes)?;
        let coverage_sha256 = sha256(&coverage_bytes);
        let mcp_config = json!({"mcpServers": {"codebase-memory-mcp": {
            "type": "stdio", "command": "/cbm/client", "args": []
        }}});
        write_private(
            &private_root.join("session/mcp.json"),
            serde_json::to_string(&mcp_config)
                .map_err(|_| Rp0Error::IsolationFailed)?
                .as_bytes(),
        )?;
        let config_db_sha256 = sha256(&fs::read(private_root.join("review-cache/_config.db"))?);
        let mcp_config_sha256 = sha256(&fs::read(private_root.join("session/mcp.json"))?);
        Ok(Self {
            profile,
            candidate,
            private_root,
            index: CandidateIndex {
                project: project.to_owned(),
                candidate_tree_sha256,
                store_sha256,
                coverage_sha256,
                coverage_complete,
                config_db_sha256,
                mcp_config_sha256,
                indexed_at,
                nodes,
                edges,
            },
        })
    }

    pub fn index(&self) -> &CandidateIndex {
        &self.index
    }

    pub(crate) fn candidate_path(&self) -> &Path {
        &self.candidate
    }

    pub fn mcp_config_path(&self) -> PathBuf {
        self.private_root.join("session/mcp.json")
    }

    /// Real-process RP0 qualification only. The selected probe is mounted as a
    /// single read-only file; no host repository or owner root is mounted.
    pub fn run_hostile_probe(
        &self,
        peer: impl AsRef<Path>,
    ) -> Result<HostileProbeResult, Rp0Error> {
        self.verify_frozen()?;
        let peer = fs::canonicalize(peer)?;
        if !peer.is_file() {
            return Err(Rp0Error::IsolationFailed);
        }
        let listener_path = self.private_root.join("proxy/analysis.sock");
        let listener = UnixListener::bind(&listener_path)?;
        let profile = self.profile.clone();
        let candidate = self.candidate.clone();
        let private_root = self.private_root.clone();
        let project = self.index.project.clone();
        let server = thread::spawn(move || {
            serve_one(
                listener,
                &profile,
                &candidate,
                &private_root,
                &project,
                None,
            )
        });
        let mut command =
            reviewer_command(&self.profile, &self.candidate, &self.private_root, true)?;
        command
            .arg("--bind")
            .arg(self.private_root.join("session"))
            .arg("/session");
        command
            .arg("--ro-bind")
            .arg(self.private_root.join("session/mcp.json"))
            .arg("/session/mcp.json");
        command
            .arg("--bind")
            .arg(self.private_root.join("scratch"))
            .arg("/scratch");
        command.arg("--dir").arg("/cbm");
        command
            .arg("--ro-bind")
            .arg(self.private_root.join("proxy"))
            .arg("/cbm/socket");
        command
            .arg("--ro-bind")
            .arg(self.profile.cbm_client.path())
            .arg("/cbm/client");
        command.arg("--ro-bind").arg(peer).arg("/probe.py");
        command.args([
            "--setenv",
            "HOME",
            "/session",
            "--",
            "/usr/bin/python3",
            "/probe.py",
        ]);
        let output = bounded_output(command, Duration::from_secs(60));
        let server_result = server.join().map_err(|_| Rp0Error::IsolationFailed)?;
        let _ = fs::remove_file(&listener_path);
        let output = output?;
        let cbm_observation = server_result?;
        self.verify_frozen()?;
        Ok(HostileProbeResult {
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
            cbm_observation,
        })
    }

    pub fn verify_frozen(&self) -> Result<(), Rp0Error> {
        self.profile.verify()?;
        if tree_digest(&self.candidate)? != self.index.candidate_tree_sha256 {
            return Err(Rp0Error::CandidateDrift);
        }
        let store = self
            .private_root
            .join("review-cache")
            .join(format!("{}.db", self.index.project));
        reject_sidecars(
            &self.private_root.join("review-cache"),
            &format!("{}.db", self.index.project),
        )?;
        if sha256(&fs::read(store)?) != self.index.store_sha256 {
            return Err(Rp0Error::IncompleteStore);
        }
        if sha256(&fs::read(self.private_root.join("session/coverage.json"))?)
            != self.index.coverage_sha256
        {
            return Err(Rp0Error::IncompleteStore);
        }
        if sha256(&fs::read(
            self.private_root.join("review-cache/_config.db"),
        )?) != self.index.config_db_sha256
        {
            return Err(Rp0Error::IncompleteStore);
        }
        if sha256(&fs::read(self.private_root.join("session/mcp.json"))?)
            != self.index.mcp_config_sha256
        {
            return Err(Rp0Error::IsolationFailed);
        }
        let dbs = fs::read_dir(self.private_root.join("review-cache"))?
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
            .filter(|name| name.ends_with(".db"))
            .collect::<Vec<_>>();
        let expected = format!("{}.db", self.index.project);
        if dbs.len() != 2 || !dbs.contains(&expected) || !dbs.contains(&"_config.db".to_owned()) {
            return Err(Rp0Error::IncompleteStore);
        }
        let config: Value = serde_json::from_slice(&fs::read(
            self.private_root.join("review-cache/config.json"),
        )?)
        .map_err(|_| Rp0Error::IsolationFailed)?;
        if config.get("ui_enabled").and_then(Value::as_bool) != Some(false) {
            return Err(Rp0Error::IsolationFailed);
        }
        Ok(())
    }
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), Rp0Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn bounded_output(mut command: Command, horizon: Duration) -> Result<Output, Rp0Error> {
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut child = command.spawn().map_err(|_| Rp0Error::IsolationFailed)?;
    let oversized = Arc::new(AtomicBool::new(false));
    let stdout = child.stdout.take().ok_or(Rp0Error::IsolationFailed)?;
    let stderr = child.stderr.take().ok_or(Rp0Error::IsolationFailed)?;
    let capture = |mut pipe: Box<dyn Read + Send>, signal: Arc<AtomicBool>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = (&mut pipe)
                .take((MAX_PROCESS_OUTPUT + 1) as u64)
                .read_to_end(&mut bytes);
            if bytes.len() > MAX_PROCESS_OUTPUT {
                signal.store(true, Ordering::SeqCst);
            }
            bytes
        })
    };
    let out = capture(Box::new(stdout), oversized.clone());
    let err = capture(Box::new(stderr), oversized.clone());
    let deadline = Instant::now() + horizon;
    let status = loop {
        if oversized.load(Ordering::SeqCst) || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out.join();
            let _ = err.join();
            return Err(if oversized.load(Ordering::SeqCst) {
                Rp0Error::OutputLimit
            } else {
                Rp0Error::IsolationFailed
            });
        }
        if let Some(status) = child.try_wait().map_err(|_| Rp0Error::IsolationFailed)? {
            break status;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let output = Output {
        status,
        stdout: out.join().map_err(|_| Rp0Error::IsolationFailed)?,
        stderr: err.join().map_err(|_| Rp0Error::IsolationFailed)?,
    };
    if output.stdout.len() > MAX_PROCESS_OUTPUT || output.stderr.len() > MAX_PROCESS_OUTPUT {
        return Err(Rp0Error::OutputLimit);
    }
    Ok(output)
}

fn base_command(
    profile: &TrustedRp0Profile,
    candidate: &Path,
    private_root: &Path,
    cache: &str,
    runtime: &str,
    network_denied: bool,
) -> Result<Command, Rp0Error> {
    profile.verify()?;
    let mut command = Command::new(profile.bwrap.path());
    command.env_clear();
    command.args([
        "--die-with-parent",
        "--unshare-user",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--new-session",
    ]);
    if network_denied {
        command.arg("--unshare-net");
    }
    for source in ["/usr", "/lib", "/lib64", "/etc/ssl"] {
        command.arg("--ro-bind").arg(source).arg(source);
    }
    command.arg("--dir").arg("/repo");
    command.arg("--ro-bind").arg(candidate).arg(REVIEW_ROOT);
    command
        .arg("--bind")
        .arg(private_root.join(cache))
        .arg("/cache");
    command
        .arg("--bind")
        .arg(private_root.join(runtime))
        .arg("/runtime");
    command.arg("--dir").arg("/cbm");
    command
        .arg("--ro-bind")
        .arg(profile.cbm.path())
        .arg("/cbm/bin");
    command.args([
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--clearenv",
        "--setenv",
        "HOME",
        "/",
        "--setenv",
        "PATH",
        "/usr/bin:/bin",
        "--setenv",
        "CBM_CACHE_DIR",
        "/cache",
        "--setenv",
        "CBM_RUNTIME_DIR",
        "/runtime",
        "--setenv",
        "CBM_LOG_LEVEL",
        "error",
        "--chdir",
        REVIEW_ROOT,
    ]);
    Ok(command)
}

fn reviewer_command(
    profile: &TrustedRp0Profile,
    candidate: &Path,
    private_root: &Path,
    mount_proc: bool,
) -> Result<Command, Rp0Error> {
    profile.verify()?;
    let mut command = Command::new(profile.bwrap.path());
    command.env_clear();
    command.args([
        "--die-with-parent",
        "--unshare-user",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--new-session",
    ]);
    for source in ["/usr", "/lib", "/lib64", "/etc/ssl"] {
        command.arg("--ro-bind").arg(source).arg(source);
    }
    command.arg("--dir").arg("/repo");
    command.arg("--ro-bind").arg(candidate).arg(REVIEW_ROOT);
    if mount_proc {
        command.args(["--proc", "/proc"]);
    }
    command.args([
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--clearenv",
        "--setenv",
        "HOME",
        "/",
        "--setenv",
        "PATH",
        "/usr/bin:/bin",
        "--chdir",
        REVIEW_ROOT,
    ]);
    let _ = private_root;
    Ok(command)
}

const MAX_MCP_LINE: usize = 64 * 1024;
const MAX_MCP_RESULT: usize = 1024 * 1024;
const MAX_MCP_REQUESTS: usize = 4096;
const ALLOWED_GRAPH_TOOLS: &[&str] = &[
    "search_graph",
    "trace_path",
    "get_code_snippet",
    "check_index_coverage",
    "index_status",
    "query_graph",
    "get_architecture",
];

fn serve_one(
    listener: UnixListener,
    profile: &TrustedRp0Profile,
    candidate: &Path,
    private_root: &Path,
    project: &str,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<Value, Rp0Error> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + Duration::from_secs(35);
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if cancel
                    .as_ref()
                    .is_some_and(|flag| flag.load(Ordering::SeqCst))
                {
                    return Err(Rp0Error::IsolationFailed);
                }
                if Instant::now() >= deadline {
                    return Err(Rp0Error::IsolationFailed);
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.into()),
        }
    };
    stream.set_read_timeout(Some(if cancel.is_some() {
        Duration::from_secs(1)
    } else {
        Duration::from_secs(25)
    }))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut command = base_command(
        profile,
        candidate,
        private_root,
        "review-cache",
        "review-runtime",
        true,
    )?;
    seal_store_mounts(&mut command, private_root, project);
    command
        .arg("--ro-bind")
        .arg(profile.cbm_launcher.path())
        .arg("/cbm/launcher");
    command.args(["--", "/cbm/launcher"]);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn().map_err(|_| Rp0Error::IsolationFailed)?;
    let cbm_parent_pid = child.id();
    let mut child_stdin = child.stdin.take().ok_or(Rp0Error::IsolationFailed)?;
    set_nonblocking(child_stdin.as_raw_fd())?;
    let child_stdout = child.stdout.take().ok_or(Rp0Error::IsolationFailed)?;
    let (sender, receiver) = mpsc::sync_channel::<Result<Vec<u8>, std::io::Error>>(1);
    let output_reader = thread::spawn(move || {
        let mut output = BufReader::new(child_stdout);
        loop {
            let mut line = Vec::new();
            let result = output
                .by_ref()
                .take((MAX_MCP_RESULT + 1) as u64)
                .read_until(b'\n', &mut line)
                .map(|_| line);
            let done = match &result {
                Ok(bytes) => bytes.is_empty(),
                Err(_) => true,
            };
            if sender.send(result).is_err() || done {
                break;
            }
        }
    });
    let mut observation = None;
    let mut forwarded = Vec::new();
    let mut request_count = 0;
    let result = (|| -> Result<(), Rp0Error> {
        loop {
            let mut line = Vec::new();
            let length = match reader
                .by_ref()
                .take((MAX_MCP_LINE + 1) as u64)
                .read_until(b'\n', &mut line)
            {
                Ok(length) => length,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) && cancel.is_some() =>
                {
                    if cancel
                        .as_ref()
                        .is_some_and(|flag| flag.load(Ordering::SeqCst))
                    {
                        break Ok(());
                    }
                    continue;
                }
                Err(error) => break Err(error.into()),
            };
            if length == 0 {
                break Ok(());
            }
            if length > MAX_MCP_LINE || line.last() != Some(&b'\n') {
                break Err(Rp0Error::OutputLimit);
            }
            let request: Value = match serde_json::from_slice(&line) {
                Ok(value) => value,
                Err(_) => break Err(Rp0Error::IsolationFailed),
            };
            let id = request.get("id").cloned();
            request_count += 1;
            if request_count > MAX_MCP_REQUESTS {
                if let Some(id) = id {
                    let response = json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,
                        "message":"RP0 explicit MCP request budget exceeded"}});
                    stream.write_all(response.to_string().as_bytes())?;
                    stream.write_all(b"\n")?;
                }
                break Err(Rp0Error::OutputLimit);
            }
            if !allowed_mcp_request(&request, project) {
                if let Some(id) = id {
                    let response = json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,
                    "message":"RP0 tool or candidate scope unavailable"}});
                    stream.write_all(response.to_string().as_bytes())?;
                    stream.write_all(b"\n")?;
                }
                continue;
            }
            if let Some(name) = request.pointer("/params/name").and_then(Value::as_str) {
                forwarded.push(json!({"id":id,"tool":name}));
            }
            write_with_deadline(&mut child_stdin, &line, Duration::from_secs(5))?;
            if id.is_none() {
                continue;
            }
            let response = receiver
                .recv_timeout(Duration::from_secs(20))
                .map_err(|_| Rp0Error::IsolationFailed)??;
            if response.is_empty()
                || response.len() > MAX_MCP_RESULT
                || response.last() != Some(&b'\n')
            {
                break Err(Rp0Error::OutputLimit);
            }
            let backend: Value =
                serde_json::from_slice(&response).map_err(|_| Rp0Error::IsolationFailed)?;
            if backend.get("id") != id.as_ref() {
                break Err(Rp0Error::IsolationFailed);
            }
            if request.get("method").and_then(Value::as_str) == Some("tools/list") {
                let mut listed: Value =
                    serde_json::from_slice(&response).map_err(|_| Rp0Error::IsolationFailed)?;
                let tools = listed
                    .pointer_mut("/result/tools")
                    .and_then(Value::as_array_mut)
                    .ok_or(Rp0Error::IsolationFailed)?;
                tools.retain(|tool| {
                    tool.get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|name| ALLOWED_GRAPH_TOOLS.contains(&name))
                });
                let filtered =
                    serde_json::to_vec(&listed).map_err(|_| Rp0Error::IsolationFailed)?;
                stream.write_all(&filtered)?;
                stream.write_all(b"\n")?;
            } else {
                stream.write_all(&response)?;
            }
            if request.pointer("/params/name").and_then(Value::as_str) == Some("index_status") {
                observation = Some(observe_cbm_descendants(cbm_parent_pid, project)?);
            }
        }
    })();
    let final_observation = observe_cbm_descendants(cbm_parent_pid, project);
    let _ = child.kill();
    let _ = child.wait();
    drop(receiver);
    let join_deadline = Instant::now() + Duration::from_secs(5);
    while !output_reader.is_finished() && Instant::now() < join_deadline {
        thread::sleep(Duration::from_millis(10));
    }
    if !output_reader.is_finished() {
        return Err(Rp0Error::IsolationFailed);
    }
    output_reader
        .join()
        .map_err(|_| Rp0Error::IsolationFailed)?;
    result?;
    let initial = observation.ok_or(Rp0Error::IsolationFailed)?;
    let final_observation = final_observation?;
    Ok(
        json!({"after_index_status": initial, "after_reload_interval": final_observation,
        "backend_forwarded":forwarded,"mcp_request_count":request_count,
        "mcp_request_budget":MAX_MCP_REQUESTS}),
    )
}

unsafe extern "C" {
    fn fcntl(fd: i32, command: i32, ...) -> i32;
}

fn set_nonblocking(fd: i32) -> Result<(), Rp0Error> {
    const F_GETFL: i32 = 3;
    const F_SETFL: i32 = 4;
    const O_NONBLOCK: i32 = 0o4000;
    let flags = unsafe { fcntl(fd, F_GETFL) };
    if flags < 0 || unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) } < 0 {
        return Err(Rp0Error::IsolationFailed);
    }
    Ok(())
}

fn write_with_deadline(
    writer: &mut impl Write,
    mut bytes: &[u8],
    limit: Duration,
) -> Result<(), Rp0Error> {
    let deadline = Instant::now() + limit;
    while !bytes.is_empty() {
        match writer.write(bytes) {
            Ok(0) => return Err(Rp0Error::IsolationFailed),
            Ok(written) => bytes = &bytes[written..],
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(Rp0Error::IsolationFailed);
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn observe_cbm_descendants(parent: u32, project: &str) -> Result<Value, Rp0Error> {
    let mut processes = Vec::new();
    let mut all = Vec::new();
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(status) = fs::read_to_string(entry.path().join("status")) else {
            continue;
        };
        let ppid = status.lines().find_map(|line| {
            line.strip_prefix("PPid:")
                .and_then(|value| value.trim().parse::<u32>().ok())
        });
        if let Some(ppid) = ppid {
            all.push((pid, ppid, status));
        }
    }
    let mut descendants = vec![parent];
    loop {
        let before = descendants.len();
        for (pid, ppid, _) in &all {
            if descendants.contains(ppid) && !descendants.contains(pid) {
                descendants.push(*pid);
            }
        }
        if descendants.len() == before {
            break;
        }
    }
    let host_net = fs::read_link("/proc/self/ns/net")?;
    for (pid, _, status) in all {
        if !descendants.contains(&pid) {
            continue;
        }
        let Ok(exe) = fs::read_link(format!("/proc/{pid}/exe")) else {
            continue;
        };
        if exe != Path::new("/cbm/bin") {
            continue;
        }
        let field = |name: &str| {
            status
                .lines()
                .find_map(|line| line.strip_prefix(name).map(str::trim).map(str::to_owned))
        };
        let net = fs::read_link(format!("/proc/{pid}/ns/net"))?;
        let tcp = fs::read_to_string(format!("/proc/{pid}/net/tcp"))?;
        let tcp6 = fs::read_to_string(format!("/proc/{pid}/net/tcp6"))?;
        let listeners = tcp
            .lines()
            .skip(1)
            .chain(tcp6.lines().skip(1))
            .filter(|line| line.split_whitespace().nth(3) == Some("0A"))
            .count();
        let devices = fs::read_to_string(format!("/proc/{pid}/net/dev"))?;
        let only_loopback = devices.lines().skip(2).all(|line| {
            line.split(':')
                .next()
                .is_some_and(|name| name.trim() == "lo")
        });
        let mounts = fs::read_to_string(format!("/proc/{pid}/mountinfo"))?;
        let mut protected_mounts = serde_json::Map::new();
        for name in [
            format!("/cache/{project}.db"),
            "/cache/config.json".into(),
            "/cache/_config.db".into(),
            REVIEW_ROOT.into(),
        ] {
            let observed = mounts.lines().find_map(|line| {
                let fields = line.split_whitespace().collect::<Vec<_>>();
                (fields.len() > 5 && fields[4] == name)
                    .then_some(fields[5].split(',').any(|o| o == "ro"))
            });
            let write_path = format!("/proc/{pid}/root{name}");
            let write_errno = OpenOptions::new()
                .write(true)
                .open(&write_path)
                .err()
                .and_then(|error| error.raw_os_error());
            protected_mounts.insert(
                name,
                json!({"mount_readonly": observed,
                "write_open_errno": write_errno}),
            );
        }
        processes.push(json!({"pid":pid,"no_new_privs":field("NoNewPrivs:"),
            "seccomp":field("Seccomp:"),"separate_net_namespace":net != host_net,
            "only_loopback":only_loopback,"tcp_listeners":listeners,
            "protected_mounts":protected_mounts}));
    }
    if processes.len() < 2
        || processes.iter().any(|entry| {
            entry["no_new_privs"] != "1"
                || entry["seccomp"] != "2"
                || entry["separate_net_namespace"] != true
                || entry["only_loopback"] != true
                || entry["tcp_listeners"] != 0
                || entry["protected_mounts"].as_object().is_none_or(|mounts| {
                    mounts.len() != 4
                        || mounts.iter().any(|(name, mount)| {
                            mount["mount_readonly"] != true
                                || (name != REVIEW_ROOT && mount["write_open_errno"] != 30)
                        })
                })
        })
    {
        return Err(Rp0Error::IsolationFailed);
    }
    Ok(Value::Array(processes))
}

fn allowed_mcp_request(request: &Value, project: &str) -> bool {
    let Some(object) = request.as_object() else {
        return false;
    };
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return false;
    };
    match method {
        "initialize" => object.get("id").is_some(),
        "notifications/initialized" => object.get("id").is_none(),
        "tools/list" => object.get("id").is_some(),
        "tools/call" => {
            if object.get("id").is_none() {
                return false;
            }
            let Some(params) = object.get("params").and_then(Value::as_object) else {
                return false;
            };
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return false;
            };
            if !ALLOWED_GRAPH_TOOLS.contains(&name) {
                return false;
            }
            let Some(arguments) = params.get("arguments").and_then(Value::as_object) else {
                return false;
            };
            arguments
                .get("project")
                .is_none_or(|value| value.as_str() == Some(project))
        }
        _ => false,
    }
}

fn seal_store_mounts(command: &mut Command, private_root: &Path, project: &str) {
    for name in [
        format!("{project}.db"),
        "config.json".into(),
        "_config.db".into(),
    ] {
        command
            .arg("--ro-bind")
            .arg(private_root.join("review-cache").join(&name))
            .arg(Path::new("/cache").join(&name));
    }
}

fn verify_store(path: &Path) -> Result<(), Rp0Error> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| Rp0Error::IncompleteStore)?;
    let result: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|_| Rp0Error::IncompleteStore)?;
    if result == "ok" {
        Ok(())
    } else {
        Err(Rp0Error::IncompleteStore)
    }
}

fn reject_sidecars(dir: &Path, store_name: &str) -> Result<(), Rp0Error> {
    for suffix in ["-wal", "-shm", "-journal"] {
        if dir.join(format!("{store_name}{suffix}")).exists() {
            return Err(Rp0Error::IncompleteStore);
        }
    }
    Ok(())
}

pub(crate) fn tree_digest(root: &Path) -> Result<String, Rp0Error> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_owned()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() {
                files.push(path);
            } else {
                return Err(Rp0Error::CandidateDrift);
            }
        }
    }
    files.sort();
    let mut hash = Sha256::new();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| Rp0Error::CandidateDrift)?;
        let name = relative.to_str().ok_or(Rp0Error::CandidateDrift)?;
        let bytes = fs::read(&path)?;
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(&bytes);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn candidate_relative_paths(root: &Path) -> Result<Vec<String>, Rp0Error> {
    let mut paths = Vec::new();
    let mut stack = vec![root.to_owned()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| Rp0Error::CandidateDrift)?;
                paths.push(
                    relative
                        .to_str()
                        .ok_or(Rp0Error::CandidateDrift)?
                        .to_owned(),
                );
            } else {
                return Err(Rp0Error::CandidateDrift);
            }
        }
    }
    paths.sort();
    Ok(paths)
}
