//! Installed selection is authority only at this protected owner boundary.
//! Policy structs are proposal data; only a checked live handle can execute.
use crate::{AuthorityMode, Error, Result, WorkspaceConfig, codec::raw_sha256};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

pub const INSTALLED_SELECTION: &str = "/etc/work-engine/workspace-publication/custody.json";
pub const BWRAP_SHA256: &str = "523da3e7399044be5163aee6f57a77a6bef7454376e28f0a0627920bae1b76b6";
pub const GIT_SHA256: &str = "ed8b2df973b28c0fb773b9c0dc0816d3ee2b3835bb932d9153c21417bb51b362";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
    pub uid: u32,
    pub mode: u32,
}
impl FileIdentity {
    /// Observation for an inert installation proposal, never an authority grant.
    pub fn observe(path: &Path) -> Result<Self> {
        let m = fs::symlink_metadata(path).map_err(io)?;
        if m.file_type().is_symlink() {
            return Err(invalid("symlink in custody identity"));
        }
        Ok(Self {
            device: m.dev(),
            inode: m.ino(),
            uid: m.uid(),
            mode: m.mode(),
        })
    }
    pub(crate) fn from_file(file: &File) -> Result<Self> {
        let m = file.metadata().map_err(io)?;
        Ok(Self {
            device: m.dev(),
            inode: m.ino(),
            uid: m.uid(),
            mode: m.mode(),
        })
    }
    fn verify(&self, path: &Path) -> Result<()> {
        if *self != Self::observe(path)? {
            return Err(invalid("installed filesystem identity changed"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustodySelection {
    pub schema_version: u32,
    pub epoch: String,
    /// `initialize` allocates owner state only; `active` must seal the results.
    pub phase: String,
    pub runtime_uid: u32,
    pub runtime_sha256: String,
    pub root: PathBuf,
    pub repository: PathBuf,
    pub root_identity: FileIdentity,
    pub repository_identity: FileIdentity,
    pub lock_identity: FileIdentity,
    pub marker_identity: Option<FileIdentity>,
    pub ledger_identity: Option<FileIdentity>,
    pub canonical_refs: Vec<String>,
    pub validation_program: Vec<String>,
    pub validation_program_sha256: String,
    pub validation_policy_digest: String,
    pub timeout_seconds: u64,
}

/// Not serializable or publicly constructible. A saved selection is only data.
pub struct ManagedCustody {
    path: PathBuf,
    selection_identity: FileIdentity,
    selection_digest: String,
    selection: CustodySelection,
    fixture: bool,
    runtime_path: PathBuf,
    runtime_identity: FileIdentity,
}
fn io(e: std::io::Error) -> Error {
    Error::Io(e.to_string())
}
fn invalid(s: &str) -> Error {
    Error::Integrity(s.into())
}
fn canonical(path: &Path) -> Result<()> {
    if !path.is_absolute() || fs::canonicalize(path).map_err(io)? != path {
        return Err(invalid(
            "custody path must be absolute, canonical and nonsymlinked",
        ));
    }
    for p in path.ancestors() {
        if fs::symlink_metadata(p)
            .map_err(io)?
            .file_type()
            .is_symlink()
        {
            return Err(invalid("symlink in custody ancestry"));
        }
    }
    Ok(())
}
fn protected(path: &Path) -> Result<()> {
    canonical(path)?;
    for p in path.ancestors() {
        let m = fs::symlink_metadata(p).map_err(io)?;
        if m.uid() != 0 || m.mode() & 0o022 != 0 {
            return Err(invalid(
                "installation ancestry is not independently protected",
            ));
        }
    }
    Ok(())
}
fn read_selection(path: &Path, fixture: bool) -> Result<(FileIdentity, Vec<u8>)> {
    if fixture {
        canonical(path)?;
    } else {
        protected(path)?;
    }
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(io)?;
    let id = FileIdentity::from_file(&f)?;
    if id.mode & libc::S_IFMT != libc::S_IFREG || id.mode & 0o022 != 0 {
        return Err(invalid(
            "selection must be a non-writable regular installed file",
        ));
    }
    let mut bytes = Vec::new();
    (&mut f).take(65537).read_to_end(&mut bytes).map_err(io)?;
    if bytes.len() > 65536 {
        return Err(invalid("selection too large"));
    }
    id.verify(path)?;
    Ok((id, bytes))
}
pub(crate) fn trusted_program(path: &str, expected: &str) -> Result<()> {
    let p = Path::new(path);
    // System symlinks may select a root-owned immutable implementation.
    let resolved = fs::canonicalize(p).map_err(io)?;
    protected(&resolved)?;
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&resolved)
        .map_err(io)?;
    let before = FileIdentity::from_file(&f)?;
    if before.mode & libc::S_IFMT != libc::S_IFREG || before.mode & 0o111 == 0 {
        return Err(invalid("trusted executable type/mode"));
    }
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).map_err(io)?;
    if raw_sha256(&bytes) != expected {
        return Err(invalid("independently selected executable changed"));
    }
    before.verify(&resolved)?;
    if fs::canonicalize(p).map_err(io)? != resolved {
        return Err(invalid("executable resolution changed"));
    }
    Ok(())
}
impl ManagedCustody {
    pub fn load_installed() -> Result<Self> {
        if cfg!(feature = "test-faults") {
            return Err(invalid("test-fault build cannot obtain production custody"));
        }
        Self::load(Path::new(INSTALLED_SELECTION), false)
    }
    #[cfg(feature = "test-faults")]
    pub fn load_fixture_selection(path: &Path) -> Result<Self> {
        Self::load(path, true)
    }
    fn load(path: &Path, fixture: bool) -> Result<Self> {
        let (identity, bytes) = read_selection(path, fixture)?;
        let selection: CustodySelection =
            serde_json::from_slice(&bytes).map_err(|e| invalid(&e.to_string()))?;
        let runtime_path = std::env::current_exe().map_err(io)?;
        if !fixture {
            protected(&runtime_path)?;
        }
        let runtime_identity = FileIdentity::observe(&runtime_path)?;
        if raw_sha256(&fs::read(&runtime_path).map_err(io)?) != selection.runtime_sha256 {
            return Err(invalid("current runtime differs from installed selection"));
        }
        let value = Self {
            runtime_path,
            runtime_identity,
            path: path.into(),
            selection_identity: identity,
            selection_digest: raw_sha256(&bytes),
            selection,
            fixture,
        };
        value.check()?;
        Ok(value)
    }
    pub(crate) fn binding(&self) -> serde_json::Value {
        serde_json::json!({"epoch":self.selection.epoch,"selectionDigest":self.selection_digest,"selectionIdentity":self.selection_identity,"standing":self.standing()})
    }
    pub fn epoch(&self) -> &str {
        &self.selection.epoch
    }
    pub fn standing(&self) -> &'static str {
        if self.fixture {
            "fixture_managed_custody"
        } else {
            "production_managed_custody"
        }
    }
    pub(crate) fn config(&self) -> WorkspaceConfig {
        let s = &self.selection;
        WorkspaceConfig {
            root: s.root.clone(),
            repository: s.repository.clone(),
            canonical_refs: s.canonical_refs.clone(),
            validation_program: s.validation_program.clone(),
            validation_policy_digest: s.validation_policy_digest.clone(),
            authority_mode: if self.fixture {
                AuthorityMode::FixtureManagedCustody
            } else {
                AuthorityMode::Production
            },
        }
    }
    pub(crate) fn check(&self) -> Result<()> {
        let s = &self.selection;
        let (id, bytes) = read_selection(&self.path, self.fixture)?;
        if id != self.selection_identity || raw_sha256(&bytes) != self.selection_digest {
            return Err(invalid("installed selection replaced or revoked"));
        }
        if s.schema_version != 1
            || s.epoch.trim().is_empty()
            || !["initialize", "active"].contains(&s.phase.as_str())
            || s.runtime_uid == 0
            || s.runtime_uid != unsafe { libc::geteuid() }
            || !(1..=300).contains(&s.timeout_seconds)
            || s.canonical_refs.is_empty()
            || s.validation_policy_digest.is_empty()
        {
            return Err(invalid("unsupported installed custody selection"));
        }
        // Linux prohibits writing the executing image. Recheck its installed
        // name/inode instead of repeatedly hashing a large running binary.
        if std::env::current_exe().map_err(io)? != self.runtime_path {
            return Err(invalid("runtime path changed"));
        }
        self.runtime_identity.verify(&self.runtime_path)?;
        if !self.fixture {
            protected(&self.runtime_path)?;
        }
        for p in [&s.root, &s.repository] {
            canonical(p)?;
            if p.starts_with("/usr") || p == Path::new("/") {
                return Err(invalid("protected state overlaps exposed system mount"));
            }
        }
        if s.root.starts_with(&s.repository) || s.repository.starts_with(&s.root) {
            return Err(invalid("owner roots overlap"));
        }
        s.root_identity.verify(&s.root)?;
        s.repository_identity.verify(&s.repository)?;
        for (id, p) in [
            (&s.root_identity, &s.root),
            (&s.repository_identity, &s.repository),
        ] {
            if id.uid != s.runtime_uid
                || id.mode & libc::S_IFMT != libc::S_IFDIR
                || id.mode & 0o022 != 0
                || !p.is_dir()
            {
                return Err(invalid("owner directory custody mode/uid"));
            }
        }
        if s.root_identity.mode & 0o077 != 0 || s.repository_identity.mode & 0o077 != 0 {
            return Err(invalid("owner ledger root must be private"));
        }
        let lock = s.repository.join(".work-engine-publication.lock");
        s.lock_identity.verify(&lock)?;
        if s.lock_identity.mode & libc::S_IFMT != libc::S_IFREG
            || s.lock_identity.uid != s.runtime_uid
            || s.lock_identity.mode & 0o077 != 0
        {
            return Err(invalid("permanent lock custody"));
        }
        // This realization supports a bare canonical repository only.
        if s.repository.join(".git").exists() || !s.repository.join("objects").is_dir() {
            return Err(invalid("managed canonical repository must be bare"));
        }
        // A selected canonical root cannot delegate writes through symlinked,
        // hard-linked or mounted topology outside that root.
        let mut directories = vec![s.repository.clone()];
        while let Some(dir) = directories.pop() {
            for entry in fs::read_dir(dir).map_err(io)? {
                let path = entry.map_err(io)?.path();
                let metadata = fs::symlink_metadata(&path).map_err(io)?;
                if metadata.dev() != s.repository_identity.device
                    || metadata.uid() != s.runtime_uid
                    || metadata.file_type().is_symlink()
                {
                    return Err(invalid("canonical Git topology escapes installed custody"));
                }
                if metadata.is_dir() {
                    directories.push(path);
                } else if !metadata.is_file() || metadata.nlink() != 1 {
                    return Err(invalid("canonical Git special file/hard link refused"));
                }
            }
        }
        for path in [
            "objects/info/alternates",
            "objects/info/http-alternates",
            "info/grafts",
            "shallow",
        ] {
            if s.repository.join(path).exists() {
                return Err(invalid("external/partial object topology is not supported"));
            }
        }
        match (&*s.phase, &s.marker_identity, &s.ledger_identity) {
            ("active", Some(marker), Some(ledger)) => {
                marker.verify(&s.root.join(".workspace-publication-v1"))?;
                ledger.verify(&s.root.join("workspace-publication.sqlite"))?;
                for id in [marker, ledger] {
                    if id.uid != s.runtime_uid
                        || id.mode & libc::S_IFMT != libc::S_IFREG
                        || id.mode & 0o077 != 0
                    {
                        return Err(invalid("sealed owner file custody mode/uid"));
                    }
                }
            }
            ("initialize", None, None) => {}
            _ => return Err(invalid("activation must seal marker and ledger identities")),
        }
        trusted_program("/usr/bin/bwrap", BWRAP_SHA256)?;
        trusted_program("/usr/bin/git", GIT_SHA256)?;
        let program = s
            .validation_program
            .first()
            .ok_or_else(|| invalid("installed validator absent"))?;
        if !Path::new(program).starts_with("/usr/") {
            return Err(invalid(
                "validator executable must be in exposed system mount",
            ));
        }
        trusted_program(program, &s.validation_program_sha256)?;
        Ok(())
    }
    pub(crate) fn check_initialize(&self) -> Result<()> {
        self.check()?;
        if self.selection.phase != "initialize" {
            return Err(invalid("initialization is not selected"));
        }
        Ok(())
    }
    pub(crate) fn check_active(&self) -> Result<()> {
        self.check()?;
        if self.selection.phase != "active" {
            return Err(invalid(
                "activation has not independently sealed owner state",
            ));
        }
        Ok(())
    }
    pub(crate) fn check_lock(&self, file: &File) -> Result<()> {
        self.check()?;
        if FileIdentity::from_file(file)? != self.selection.lock_identity {
            return Err(invalid("guard no longer holds installed permanent lock"));
        }
        Ok(())
    }
    /// Run untrusted code with only selected immutable source and private scratch.
    /// The trusted host selects this input. It cannot select the owner root itself.
    pub fn run_untrusted(&self, source: &Path, argv: &[String]) -> Result<Output> {
        self.run(source, argv, None)
    }
    pub(crate) fn run(&self, source: &Path, argv: &[String], tree: Option<&str>) -> Result<Output> {
        self.check_active()?;
        canonical(source)?;
        let s = &self.selection;
        if !source.is_dir()
            || ![s.root.join("operations"), s.root.join("inputs")]
                .iter()
                .any(|p| source.starts_with(p) && source != p)
        {
            return Err(invalid("source outside selected operation/input namespace"));
        }
        // Read-only bind mounts do not revoke socket connect authority. Reject
        // special files and hard links, rather than treating RO as a capability filter.
        let device = fs::symlink_metadata(source).map_err(io)?.dev();
        let mut pending = vec![source.to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in fs::read_dir(dir).map_err(io)? {
                let path = entry.map_err(io)?.path();
                let meta = fs::symlink_metadata(&path).map_err(io)?;
                if meta.dev() != device {
                    return Err(invalid("nested source mount refused"));
                }
                if meta.is_dir() {
                    pending.push(path);
                } else if meta.is_file() {
                    if meta.nlink() != 1 {
                        return Err(invalid("source hard link refused"));
                    }
                } else if !meta.file_type().is_symlink() {
                    return Err(invalid("source socket/device/FIFO refused"));
                }
            }
        }
        let program = argv
            .first()
            .ok_or_else(|| invalid("untrusted argv absent"))?;
        if !Path::new(program).starts_with("/usr/") {
            return Err(invalid("untrusted program outside isolated system mount"));
        }
        let mut cmd = Command::new("/usr/bin/bwrap");
        cmd.args([
            "--unshare-all",
            "--unshare-user",
            "--disable-userns",
            "--assert-userns-disabled",
            "--cap-drop",
            "ALL",
            "--die-with-parent",
            "--new-session",
            "--ro-bind",
            "/usr",
            "/usr",
            "--symlink",
            "usr/lib",
            "/lib",
            "--symlink",
            "usr/lib64",
            "/lib64",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--tmpfs",
            "/scratch",
            "--ro-bind",
        ])
        .arg(source)
        .args([
            "/candidate",
            "--chdir",
            "/candidate",
            "--clearenv",
            "--setenv",
            "PATH",
            "/usr/bin:/bin",
            "--setenv",
            "LC_ALL",
            "C",
            "--setenv",
            "HOME",
            "/scratch",
            "--setenv",
            "CARGO_TARGET_DIR",
            "/scratch/target",
        ]);
        if let Some(tree) = tree {
            cmd.args(["--setenv", "WORK_ENGINE_VALIDATED_TREE", tree]);
        }
        cmd.arg("--")
            .args(argv)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        // bubblewrap preserves non-CLOEXEC descriptors. Scrub independently of
        // Rust's usual CLOEXEC discipline, preserving only stdio and exec-error pipe.
        unsafe {
            cmd.pre_exec(|| {
                if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::syscall(
                    libc::SYS_close_range,
                    3u32,
                    u32::MAX,
                    libc::CLOSE_RANGE_CLOEXEC,
                ) != 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = cmd.spawn().map_err(io)?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| invalid("stdout absent"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| invalid("stderr absent"))?;
        let read = |mut pipe: Box<dyn Read + Send>| {
            let mut bytes = Vec::new();
            let result = pipe.by_ref().take(1048577).read_to_end(&mut bytes);
            (result, bytes)
        };
        let out = std::thread::spawn(move || read(Box::new(stdout)));
        let err = std::thread::spawn(move || read(Box::new(stderr)));
        let start = Instant::now();
        let mut timed_out = false;
        let status = loop {
            if let Some(status) = child.try_wait().map_err(io)? {
                break status;
            }
            if start.elapsed() > Duration::from_secs(s.timeout_seconds) {
                timed_out = true;
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.kill();
                break child.wait().map_err(io)?;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let (o, stdout) = out.join().map_err(|_| invalid("stdout collector failed"))?;
        o.map_err(io)?;
        let (e, stderr) = err.join().map_err(|_| invalid("stderr collector failed"))?;
        e.map_err(io)?;
        self.check_active()?;
        if timed_out || stdout.len() > 1048576 || stderr.len() > 1048576 {
            return Err(invalid("untrusted process time/output bound exceeded"));
        }
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}
