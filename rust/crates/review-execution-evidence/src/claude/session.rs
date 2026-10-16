use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::EvidenceReadError;
use crate::contract::{require_sha, require_text, sha256};
use crate::store::{self, AnchorIds};

pub(crate) const SESSION_LIMIT: u64 = 64 * 1024 * 1024;
pub(crate) const ACTIVE_SESSION_LIMIT: u64 = 16 * 1024 * 1024;
const MAX_FILES: usize = 4096;
const ENTRY_CHARGE: u64 = 4096;
const GUARD_MAGIC: &[u8] = b"WE-RP1-SESSION-V2\0";

pub(crate) struct GuardExport {
    pub result: Vec<u8>,
    pub files: Vec<(String, Vec<u8>)>,
}

pub(crate) fn decode_guard_export(raw: &[u8]) -> Result<GuardExport, EvidenceReadError> {
    let mut rest = raw
        .strip_prefix(GUARD_MAGIC)
        .ok_or(EvidenceReadError::Conflicting(
            "guard session frame invalid",
        ))?;
    fn take<'a>(rest: &mut &'a [u8], len: usize) -> Result<&'a [u8], EvidenceReadError> {
        if len > rest.len() {
            return Err(EvidenceReadError::Conflicting(
                "guard session frame truncated",
            ));
        }
        let (head, tail) = rest.split_at(len);
        *rest = tail;
        Ok(head)
    }
    let len = u32::from_be_bytes(take(&mut rest, 4)?.try_into().unwrap()) as usize;
    if len > crate::STREAM_MAX_BYTES {
        return Err(EvidenceReadError::Capacity(
            "guard result exceeds stream limit",
        ));
    }
    let result = take(&mut rest, len)?.to_vec();
    let count = u32::from_be_bytes(take(&mut rest, 4)?.try_into().unwrap()) as usize;
    if count > MAX_FILES {
        return Err(EvidenceReadError::Capacity("guard session entry count"));
    }
    let mut files = Vec::with_capacity(count);
    let mut names = std::collections::BTreeSet::new();
    let mut total = 0u64;
    for _ in 0..count {
        let name_len = u16::from_be_bytes(take(&mut rest, 2)?.try_into().unwrap()) as usize;
        if name_len == 0 || name_len > 4096 {
            return Err(EvidenceReadError::Conflicting("guard session path length"));
        }
        let name = std::str::from_utf8(take(&mut rest, name_len)?)
            .map_err(|_| EvidenceReadError::Conflicting("guard session path encoding"))?;
        if !valid_relative(name) || !names.insert(name.to_owned()) {
            return Err(EvidenceReadError::Conflicting("guard session path invalid"));
        }
        let file_len = u64::from_be_bytes(take(&mut rest, 8)?.try_into().unwrap());
        total = total
            .checked_add(file_len)
            .ok_or(EvidenceReadError::Capacity("guard session size overflow"))?;
        if total > ACTIVE_SESSION_LIMIT {
            return Err(EvidenceReadError::Capacity("guard session size limit"));
        }
        let bytes = take(
            &mut rest,
            usize::try_from(file_len)
                .map_err(|_| EvidenceReadError::Capacity("guard session file size"))?,
        )?
        .to_vec();
        files.push((name.to_owned(), bytes));
    }
    if !rest.is_empty() {
        return Err(EvidenceReadError::Conflicting(
            "guard session trailing bytes",
        ));
    }
    Ok(GuardExport { result, files })
}

pub(crate) fn import_guard_export(
    root: &Path,
    ready: &SessionReady,
    files: &[(String, Vec<u8>)],
) -> Result<(), EvidenceReadError> {
    verify_live(root, ready)?;
    let live = root.join("session-live");
    if fs::read_dir(&live)
        .map_err(|_| EvidenceReadError::Unresolved("session live enumeration failed"))?
        .next()
        .is_some()
    {
        return Err(EvidenceReadError::Conflicting(
            "initial session live not empty",
        ));
    }
    let mut projected = storage_charge(root)?.0;
    let mut dirs = std::collections::BTreeSet::new();
    for (name, bytes) in files {
        if !valid_relative(name) || name.len() > 4096 {
            return Err(EvidenceReadError::Conflicting("guard session path invalid"));
        }
        projected = projected
            .checked_add(
                ENTRY_CHARGE
                    + name.len() as u64
                    + (bytes.len() as u64)
                        .checked_add(4095)
                        .ok_or(EvidenceReadError::Capacity("guard session size overflow"))?
                        / 4096
                        * 4096,
            )
            .ok_or(EvidenceReadError::Capacity(
                "guard session reservation overflow",
            ))?;
        let mut parent = Path::new(name).parent();
        while let Some(path) = parent {
            if path.as_os_str().is_empty() {
                break;
            }
            dirs.insert(path.to_path_buf());
            parent = path.parent();
        }
    }
    projected = projected
        .checked_add((dirs.len() as u64).saturating_mul(ENTRY_CHARGE + 4096))
        .ok_or(EvidenceReadError::Capacity(
            "guard session reservation overflow",
        ))?;
    if projected > SESSION_LIMIT {
        return Err(EvidenceReadError::Capacity(
            "guard session aggregate reservation",
        ));
    }
    for dir in dirs {
        let target = live.join(dir);
        fs::create_dir_all(&target).map_err(|_| {
            EvidenceReadError::Unresolved("guard session directory creation failed")
        })?;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).map_err(|_| {
            EvidenceReadError::Unresolved("guard session directory permissions failed")
        })?;
    }
    for (name, bytes) in files {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(live.join(name))
            .map_err(|_| EvidenceReadError::Unresolved("guard session import failed"))?;
        output
            .write_all(bytes)
            .and_then(|_| output.sync_all())
            .map_err(|_| EvidenceReadError::Unresolved("guard session import sync failed"))?;
    }
    storage_charge(root)?;
    Ok(())
}

fn sync_directory(path: &Path, stage: &'static str) -> Result<(), EvidenceReadError> {
    #[cfg(feature = "test-faults")]
    if std::env::var("WORK_ENGINE_CLAUDE_SYNC_FAIL")
        .ok()
        .as_deref()
        == Some(stage)
    {
        return Err(EvidenceReadError::Unresolved(
            "session directory sync fault",
        ));
    }
    File::open(path)
        .and_then(|dir| dir.sync_all())
        .map_err(|_| EvidenceReadError::Unresolved("session directory sync failed"))?;
    #[cfg(feature = "test-faults")]
    if let Ok(trace) = std::env::var("WORK_ENGINE_CLAUDE_SYNC_TRACE") {
        let mut output = OpenOptions::new()
            .create(true)
            .append(true)
            .open(trace)
            .map_err(|_| EvidenceReadError::Unresolved("session sync trace unavailable"))?;
        writeln!(output, "{stage}")
            .map_err(|_| EvidenceReadError::Unresolved("session sync trace failed"))?;
    }
    Ok(())
}

fn charged_file_len(meta: &fs::Metadata) -> Result<u64, EvidenceReadError> {
    let allocated = meta
        .blocks()
        .checked_mul(512)
        .ok_or(EvidenceReadError::Capacity("session allocation overflow"))?;
    let logical = meta
        .len()
        .checked_add(4095)
        .ok_or(EvidenceReadError::Capacity("session length overflow"))?
        / 4096
        * 4096;
    Ok(logical.max(allocated))
}

/// Count live, every retained snapshot and every orphan. A fixed per-entry
/// charge conservatively includes bounded directory and name metadata.
fn storage_charge(root: &Path) -> Result<(u64, u64), EvidenceReadError> {
    let mut total = 0u64;
    let mut live_total = 0u64;
    let mut entries = 0usize;
    for (name, is_live) in [("session-live", true), ("session-snapshots", false)] {
        let top = root.join(name);
        if !top.exists() {
            continue;
        }
        let mut stack = vec![(top, 0usize)];
        while let Some((path, depth)) = stack.pop() {
            if depth > 16 || entries >= MAX_FILES {
                return Err(EvidenceReadError::Capacity("session entry count or depth"));
            }
            entries += 1;
            let meta = fs::symlink_metadata(&path)
                .map_err(|_| EvidenceReadError::Unresolved("session storage inaccessible"))?;
            if meta.file_type().is_symlink() {
                return Err(EvidenceReadError::Conflicting("session storage symlink"));
            }
            let mut charge = ENTRY_CHARGE
                .checked_add(path.as_os_str().len() as u64)
                .ok_or(EvidenceReadError::Capacity("session metadata overflow"))?;
            if meta.is_dir() {
                for entry in fs::read_dir(&path).map_err(|_| {
                    EvidenceReadError::Unresolved("session storage enumeration failed")
                })? {
                    stack.push((
                        entry
                            .map_err(|_| {
                                EvidenceReadError::Unresolved("session storage enumeration failed")
                            })?
                            .path(),
                        depth + 1,
                    ));
                }
            } else if meta.is_file() {
                if meta.nlink() != 1 {
                    return Err(EvidenceReadError::Conflicting("session storage hardlink"));
                }
                charge = charge
                    .checked_add(charged_file_len(&meta)?)
                    .ok_or(EvidenceReadError::Capacity("session storage overflow"))?;
            } else {
                return Err(EvidenceReadError::Conflicting(
                    "session storage special file",
                ));
            }
            total = total
                .checked_add(charge)
                .ok_or(EvidenceReadError::Capacity("session storage overflow"))?;
            if is_live {
                live_total = live_total
                    .checked_add(charge)
                    .ok_or(EvidenceReadError::Capacity("session storage overflow"))?;
            }
            if total > SESSION_LIMIT {
                return Err(EvidenceReadError::Capacity("session storage exceeds limit"));
            }
        }
    }
    Ok((total, live_total))
}

/// The active sandbox's cgroup charges its processes, tmpfs data, and kernel
/// inode/dentry metadata against the room left by all retained host custody.
/// Process memory makes this deliberately conservative.
pub(crate) fn active_cgroup_budget(root: &Path) -> Result<u64, EvidenceReadError> {
    let current = storage_charge(root)?.0;
    let remaining = SESSION_LIMIT
        .checked_sub(current)
        .ok_or(EvidenceReadError::Capacity(
            "session active budget exhausted",
        ))?;
    // cgroup memory.max is page-granular; round down before writing and
    // verifying the realized kernel value.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page <= 0 {
        return Err(EvidenceReadError::Unresolved(
            "memory page size unavailable",
        ));
    }
    let budget = remaining / page as u64 * page as u64;
    if budget == 0 {
        return Err(EvidenceReadError::Capacity(
            "session active budget exhausted",
        ));
    }
    Ok(budget)
}

pub(crate) fn reserve_snapshot(root: &Path) -> Result<(), EvidenceReadError> {
    let (current, live) = storage_charge(root)?;
    // The second live charge covers the longer archive path and directory
    // metadata for every cloned entry, not merely file contents.
    let projected = current
        .checked_add(live)
        .and_then(|value| value.checked_add(live))
        .and_then(|value| value.checked_add(ENTRY_CHARGE * 2))
        .ok_or(EvidenceReadError::Capacity("session reservation overflow"))?;
    if projected > SESSION_LIMIT {
        return Err(EvidenceReadError::Capacity(
            "session aggregate reservation exceeds limit",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionLocator {
    pub schema_version: u32,
    pub root_id: String,
    pub profile: String,
    pub attempt_id: String,
    pub session_id: String,
    pub manifest_sha256: String,
}

impl SessionLocator {
    pub fn validate(&self) -> Result<(), EvidenceReadError> {
        if self.schema_version != 1 {
            return Err(EvidenceReadError::Unsupported("session locator schema"));
        }
        for text in [
            &self.root_id,
            &self.profile,
            &self.attempt_id,
            &self.session_id,
        ] {
            require_text(text)?;
        }
        require_sha(&self.manifest_sha256)
    }
}

/// Issued only by a checked reader after validating the current live session
/// and the immutable attempt snapshot. No Clone or serialization authority.
///
/// ```compile_fail
/// use review_execution_evidence::claude::RetainedReviewSession;
/// fn requires_clone<T: Clone>() {}
/// requires_clone::<RetainedReviewSession>();
/// ```
pub struct RetainedReviewSession {
    locator: SessionLocator,
    root: PathBuf,
    anchors: AnchorIds,
    live_dev: u64,
    live_ino: u64,
    chain: Vec<SessionManifest>,
}

impl RetainedReviewSession {
    pub fn locator(&self) -> SessionLocator {
        self.locator.clone()
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub(crate) fn checked(
        locator: SessionLocator,
        root: PathBuf,
        ready: &SessionReady,
        chain: Vec<SessionManifest>,
        anchors: AnchorIds,
    ) -> Result<Self, EvidenceReadError> {
        locator.validate()?;
        if store::anchor_ids(&root)? != anchors {
            return Err(EvidenceReadError::Conflicting(
                "retained session root anchors changed",
            ));
        }
        verify_live(&root, ready)?;
        verify_chain(&root, &chain)?;
        let latest = chain
            .last()
            .ok_or(EvidenceReadError::Conflicting("session chain empty"))?;
        if locator.attempt_id != latest.attempt_id
            || locator.session_id != latest.session_id
            || locator.manifest_sha256 != manifest_digest(latest)?
        {
            return Err(EvidenceReadError::Conflicting(
                "session locator/chain mismatch",
            ));
        }
        verify_live_manifest(&root, latest)?;
        Ok(Self {
            locator,
            root,
            anchors,
            live_dev: ready.dev,
            live_ino: ready.ino,
            chain,
        })
    }

    pub fn verify_current(&self) -> Result<(), EvidenceReadError> {
        self.verify_predecessor()?;
        verify_live_manifest(&self.root, self.chain.last().unwrap())
    }

    pub(crate) fn verify_predecessor(&self) -> Result<(), EvidenceReadError> {
        if store::anchor_ids(&self.root)? != self.anchors {
            return Err(EvidenceReadError::Conflicting(
                "retained session root anchors changed",
            ));
        }
        verify_live(&self.root, &self.ready())?;
        verify_chain(&self.root, &self.chain)
    }

    pub(crate) fn chain(&self) -> &[SessionManifest] {
        &self.chain
    }
    pub(crate) fn ready(&self) -> SessionReady {
        SessionReady {
            dev: self.live_dev,
            ino: self.live_ino,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SessionReady {
    pub dev: u64,
    pub ino: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SessionFile {
    pub path: String,
    pub len: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SessionManifest {
    pub schema_version: u32,
    pub attempt_id: String,
    pub session_id: String,
    pub predecessor_sha256: Option<String>,
    pub predecessor_attempt_id: Option<String>,
    pub snapshot_dir: String,
    pub files: Vec<SessionFile>,
}

pub(crate) fn create_live(root: &Path) -> Result<SessionReady, EvidenceReadError> {
    let path = root.join("session-live");
    fs::create_dir(&path)
        .map_err(|_| EvidenceReadError::Unresolved("session directory creation failed"))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
        .map_err(|_| EvidenceReadError::Unresolved("session permissions failed"))?;
    sync_directory(&path, "live_directory")?;
    sync_directory(root, "live_parent")?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| EvidenceReadError::Unresolved("session identity unavailable"))?;
    Ok(SessionReady {
        dev: metadata.dev(),
        ino: metadata.ino(),
    })
}

pub(crate) fn verify_live(root: &Path, ready: &SessionReady) -> Result<(), EvidenceReadError> {
    let path = root.join("session-live");
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| EvidenceReadError::Unresolved("retained session lost"))?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.dev() != ready.dev
        || metadata.ino() != ready.ino
        || metadata.mode() & 0o077 != 0
    {
        return Err(EvidenceReadError::Conflicting(
            "retained session identity changed",
        ));
    }
    Ok(())
}

fn valid_relative(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        && Path::new(path).components().count() <= 16
}

fn regular(path: &Path) -> Result<fs::Metadata, EvidenceReadError> {
    let meta = fs::symlink_metadata(path)
        .map_err(|_| EvidenceReadError::Unresolved("session file inaccessible"))?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.nlink() != 1 {
        return Err(EvidenceReadError::Conflicting(
            "unsafe session file or hardlink",
        ));
    }
    Ok(meta)
}

/// Resolve every component below an already opened session directory. The
/// reviewer can mutate live paths, so a prior symlink check is insufficient.
fn open_beneath(base: &Path, relative: &Path) -> Result<File, EvidenceReadError> {
    let name = relative
        .to_str()
        .ok_or(EvidenceReadError::Conflicting("session path not UTF-8"))?;
    if !valid_relative(name) {
        return Err(EvidenceReadError::Conflicting("session path invalid"));
    }
    let dir = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(base)
        .map_err(|_| EvidenceReadError::Unresolved("session directory unavailable"))?;
    let name =
        CString::new(name).map_err(|_| EvidenceReadError::Conflicting("session path invalid"))?;
    let mut how: libc::open_how = unsafe { std::mem::zeroed() };
    how.flags = (libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW) as u64;
    how.resolve = libc::RESOLVE_BENEATH | libc::RESOLVE_NO_SYMLINKS | libc::RESOLVE_NO_MAGICLINKS;
    // SAFETY: the fd is owned by `dir`; `name` and `how` remain alive for the
    // syscall; a successful new descriptor is uniquely transferred to File.
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            dir.as_raw_fd(),
            name.as_ptr(),
            &how,
            std::mem::size_of::<libc::open_how>(),
        )
    };
    if fd < 0 {
        return Err(EvidenceReadError::Conflicting(
            "session path escaped or changed",
        ));
    }
    Ok(unsafe { File::from_raw_fd(fd as i32) })
}

/// Copy bounded live bytes into a new immutable per-attempt snapshot before the
/// owner records initial capture or a later evolution. A crash before that
/// record leaves only unresolved orphan custody, never an issued capability.
pub(crate) fn capture_snapshot(
    root: &Path,
    ready: &SessionReady,
    attempt: &str,
    session_id: &str,
    forbidden_token: Option<&str>,
    predecessor: Option<&SessionManifest>,
) -> Result<SessionManifest, EvidenceReadError> {
    verify_live(root, ready)?;
    require_text(attempt)?;
    require_text(session_id)?;
    reserve_snapshot(root)?;
    let snapshot_dir = sha256(attempt.as_bytes());
    let archive_parent = root.join("session-snapshots");
    if !archive_parent.exists() {
        fs::create_dir(&archive_parent)
            .map_err(|_| EvidenceReadError::Unresolved("session archive creation failed"))?;
        fs::set_permissions(&archive_parent, fs::Permissions::from_mode(0o700))
            .map_err(|_| EvidenceReadError::Unresolved("session archive permissions failed"))?;
    }
    let archive = archive_parent.join(&snapshot_dir);
    fs::create_dir(&archive)
        .map_err(|_| EvidenceReadError::Conflicting("session snapshot already exists"))?;
    fs::set_permissions(&archive, fs::Permissions::from_mode(0o700))
        .map_err(|_| EvidenceReadError::Unresolved("session snapshot permissions failed"))?;
    let live = root.join("session-live");
    let mut paths = Vec::new();
    let mut stack = vec![live.clone()];
    while let Some(dir) = stack.pop() {
        for item in fs::read_dir(&dir)
            .map_err(|_| EvidenceReadError::Unresolved("session enumeration failed"))?
        {
            let item =
                item.map_err(|_| EvidenceReadError::Unresolved("session enumeration failed"))?;
            let path = item.path();
            let meta = fs::symlink_metadata(&path)
                .map_err(|_| EvidenceReadError::Unresolved("session entry inaccessible"))?;
            if meta.file_type().is_symlink() {
                return Err(EvidenceReadError::Conflicting("session symlink"));
            }
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                paths.push(path);
            } else {
                return Err(EvidenceReadError::Conflicting("session special file"));
            }
            if paths.len() > MAX_FILES {
                return Err(EvidenceReadError::Capacity("session file count"));
            }
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err(EvidenceReadError::Unresolved(
            "retained session has no files",
        ));
    }
    let mut total = 0u64;
    let mut files = Vec::new();
    let mut nested_dirs = std::collections::BTreeSet::new();
    let mut live_nested_dirs = std::collections::BTreeSet::new();
    for source in paths {
        let relative = source
            .strip_prefix(&live)
            .map_err(|_| EvidenceReadError::Conflicting("session path escaped"))?;
        let name = relative
            .to_str()
            .ok_or(EvidenceReadError::Conflicting("session path not UTF-8"))?;
        if !valid_relative(name) {
            return Err(EvidenceReadError::Conflicting("session path invalid"));
        }
        let before = regular(&source)?;
        total = total
            .checked_add(before.len())
            .ok_or(EvidenceReadError::Capacity("session storage overflow"))?;
        if total > SESSION_LIMIT {
            return Err(EvidenceReadError::Capacity("session storage exceeds limit"));
        }
        let opened = open_beneath(&live, relative)?;
        opened
            .sync_all()
            .map_err(|_| EvidenceReadError::Unresolved("live session file sync failed"))?;
        let opened_meta = opened
            .metadata()
            .map_err(|_| EvidenceReadError::Unresolved("session file identity failed"))?;
        if opened_meta.dev() != before.dev() || opened_meta.ino() != before.ino() {
            return Err(EvidenceReadError::Conflicting("session file replaced"));
        }
        let mut bytes = Vec::new();
        opened
            .take(before.len() + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| EvidenceReadError::Unresolved("session file read failed"))?;
        if bytes.len() as u64 != before.len() || regular(&source)?.ino() != before.ino() {
            return Err(EvidenceReadError::Conflicting(
                "session file changed during capture",
            ));
        }
        if forbidden_token.is_some_and(|token| {
            !token.is_empty()
                && bytes
                    .windows(token.len())
                    .any(|part| part == token.as_bytes())
        }) {
            return Err(EvidenceReadError::Conflicting(
                "session contains credential material",
            ));
        }
        let target = archive.join(relative);
        if let Some(parent) = source.parent() {
            let mut nested = parent;
            while nested != live {
                live_nested_dirs.insert(nested.to_path_buf());
                nested = nested
                    .parent()
                    .ok_or(EvidenceReadError::Conflicting("live session path escaped"))?;
            }
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| EvidenceReadError::Unresolved("session snapshot directory failed"))?;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|_| {
                EvidenceReadError::Unresolved("session snapshot permissions failed")
            })?;
            let mut nested = parent;
            while nested != archive {
                nested_dirs.insert(nested.to_path_buf());
                nested = nested.parent().ok_or(EvidenceReadError::Conflicting(
                    "session snapshot path escaped",
                ))?;
            }
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&target)
            .map_err(|_| EvidenceReadError::Unresolved("session snapshot write failed"))?;
        output
            .write_all(&bytes)
            .and_then(|_| output.sync_all())
            .map_err(|_| EvidenceReadError::Unresolved("session snapshot sync failed"))?;
        files.push(SessionFile {
            path: name.into(),
            len: before.len(),
            sha256: sha256(&bytes),
        });
    }
    let mut nested_dirs: Vec<_> = nested_dirs.into_iter().collect();
    nested_dirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for dir in nested_dirs {
        sync_directory(&dir, "snapshot_nested")?;
    }
    let mut live_nested_dirs: Vec<_> = live_nested_dirs.into_iter().collect();
    live_nested_dirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for dir in live_nested_dirs {
        sync_directory(&dir, "live_nested")?;
    }
    sync_directory(&live, "live_directory_before_capture")?;
    sync_directory(&archive, "snapshot_directory")?;
    sync_directory(&archive_parent, "snapshots_parent")?;
    sync_directory(root, "snapshot_owner_root")?;
    if storage_charge(root)?.0 > SESSION_LIMIT {
        return Err(EvidenceReadError::Capacity("session storage exceeds limit"));
    }
    verify_live(root, ready)?;
    Ok(SessionManifest {
        schema_version: 1,
        attempt_id: attempt.into(),
        session_id: session_id.into(),
        predecessor_sha256: predecessor.map(manifest_digest).transpose()?,
        predecessor_attempt_id: predecessor.map(|prior| prior.attempt_id.clone()),
        snapshot_dir,
        files,
    })
}

pub(crate) fn verify_snapshot(
    root: &Path,
    manifest: &SessionManifest,
) -> Result<(), EvidenceReadError> {
    if manifest.schema_version != 1
        || manifest.predecessor_sha256.is_some() != manifest.predecessor_attempt_id.is_some()
        || manifest.snapshot_dir != sha256(manifest.attempt_id.as_bytes())
    {
        return Err(EvidenceReadError::Conflicting(
            "session manifest identity invalid",
        ));
    }
    let archive = root.join("session-snapshots").join(&manifest.snapshot_dir);
    let meta = fs::symlink_metadata(&archive)
        .map_err(|_| EvidenceReadError::Unresolved("session snapshot missing"))?;
    if !meta.is_dir() || meta.file_type().is_symlink() || meta.mode() & 0o077 != 0 {
        return Err(EvidenceReadError::Conflicting("unsafe session snapshot"));
    }
    let mut total = 0u64;
    let mut expected = std::collections::BTreeSet::new();
    for item in &manifest.files {
        if !valid_relative(&item.path) || !expected.insert(item.path.clone()) {
            return Err(EvidenceReadError::Conflicting(
                "session manifest path invalid",
            ));
        }
        require_sha(&item.sha256)?;
        total = total
            .checked_add(item.len)
            .ok_or(EvidenceReadError::Capacity(
                "session manifest length overflow",
            ))?;
        if total > SESSION_LIMIT {
            return Err(EvidenceReadError::Capacity(
                "session manifest exceeds limit",
            ));
        }
        let path = archive.join(&item.path);
        let meta = regular(&path)?;
        if meta.len() != item.len || meta.mode() & 0o077 != 0 {
            return Err(EvidenceReadError::Conflicting(
                "session snapshot length changed",
            ));
        }
        let mut bytes = Vec::new();
        let opened = open_beneath(&archive, Path::new(&item.path))?;
        let opened_meta = opened
            .metadata()
            .map_err(|_| EvidenceReadError::Unresolved("session snapshot identity unavailable"))?;
        if opened_meta.dev() != meta.dev() || opened_meta.ino() != meta.ino() {
            return Err(EvidenceReadError::Conflicting(
                "session snapshot path changed",
            ));
        }
        opened
            .take(item.len + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| EvidenceReadError::Unresolved("session snapshot unreadable"))?;
        if sha256(&bytes) != item.sha256 {
            return Err(EvidenceReadError::Conflicting(
                "session snapshot digest changed",
            ));
        }
    }
    let mut actual = std::collections::BTreeSet::new();
    let mut stack = vec![archive.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir)
            .map_err(|_| EvidenceReadError::Unresolved("session snapshot enumeration failed"))?
        {
            let entry = entry.map_err(|_| {
                EvidenceReadError::Unresolved("session snapshot enumeration failed")
            })?;
            let path = entry.path();
            let meta = fs::symlink_metadata(&path)
                .map_err(|_| EvidenceReadError::Unresolved("session snapshot entry failed"))?;
            if meta.file_type().is_symlink() {
                return Err(EvidenceReadError::Conflicting("session snapshot symlink"));
            }
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                let relative = path
                    .strip_prefix(&archive)
                    .map_err(|_| EvidenceReadError::Conflicting("session snapshot path escaped"))?;
                actual.insert(relative.to_string_lossy().into_owned());
            } else {
                return Err(EvidenceReadError::Conflicting(
                    "session snapshot special file",
                ));
            }
        }
    }
    if actual != expected {
        return Err(EvidenceReadError::Conflicting(
            "session snapshot inventory changed",
        ));
    }
    Ok(())
}

pub(crate) fn manifest_digest(manifest: &SessionManifest) -> Result<String, EvidenceReadError> {
    Ok(sha256(&serde_json::to_vec(manifest).map_err(|_| {
        EvidenceReadError::Conflicting("session manifest serialization failed")
    })?))
}

pub(crate) fn verify_chain(
    root: &Path,
    chain: &[SessionManifest],
) -> Result<(), EvidenceReadError> {
    storage_charge(root)?;
    if chain.is_empty() || chain.len() > 64 {
        return Err(EvidenceReadError::Capacity("session chain length invalid"));
    }
    let mut attempts = std::collections::BTreeSet::new();
    for (position, manifest) in chain.iter().enumerate() {
        if !attempts.insert(&manifest.attempt_id) || manifest.session_id != chain[0].session_id {
            return Err(EvidenceReadError::Conflicting(
                "session chain identity invalid",
            ));
        }
        let predecessor = position.checked_sub(1).map(|index| &chain[index]);
        if manifest.predecessor_attempt_id.as_deref()
            != predecessor.map(|prior| prior.attempt_id.as_str())
            || manifest.predecessor_sha256 != predecessor.map(manifest_digest).transpose()?
        {
            return Err(EvidenceReadError::Conflicting(
                "session predecessor link invalid",
            ));
        }
        verify_snapshot(root, manifest)?;
    }
    Ok(())
}

pub(crate) fn verify_live_manifest(
    root: &Path,
    manifest: &SessionManifest,
) -> Result<(), EvidenceReadError> {
    storage_charge(root)?;
    let live = root.join("session-live");
    let mut actual = std::collections::BTreeSet::new();
    let mut stack = vec![live.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir)
            .map_err(|_| EvidenceReadError::Unresolved("live session enumeration failed"))?
        {
            let entry = entry
                .map_err(|_| EvidenceReadError::Unresolved("live session enumeration failed"))?;
            let path = entry.path();
            let meta = fs::symlink_metadata(&path)
                .map_err(|_| EvidenceReadError::Unresolved("live session entry unavailable"))?;
            if meta.file_type().is_symlink() {
                return Err(EvidenceReadError::Conflicting("live session symlink"));
            }
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                let name = path
                    .strip_prefix(&live)
                    .map_err(|_| EvidenceReadError::Conflicting("live session path escaped"))?
                    .to_str()
                    .ok_or(EvidenceReadError::Conflicting("live session path invalid"))?
                    .to_owned();
                actual.insert(name);
            } else {
                return Err(EvidenceReadError::Conflicting("live session special file"));
            }
        }
    }
    let expected: std::collections::BTreeSet<_> = manifest
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect();
    if actual != expected {
        return Err(EvidenceReadError::Conflicting(
            "live session inventory changed",
        ));
    }
    for item in &manifest.files {
        let path = live.join(&item.path);
        let meta = regular(&path)?;
        let mut bytes = Vec::new();
        let opened = open_beneath(&live, Path::new(&item.path))?;
        let opened_meta = opened
            .metadata()
            .map_err(|_| EvidenceReadError::Unresolved("live session identity unavailable"))?;
        opened
            .take(item.len + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| EvidenceReadError::Unresolved("live session file unreadable"))?;
        if meta.len() != item.len
            || opened_meta.dev() != meta.dev()
            || opened_meta.ino() != meta.ino()
            || sha256(&bytes) != item.sha256
        {
            return Err(EvidenceReadError::Conflicting("live session file changed"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::open_beneath;
    use std::fs;
    use std::io::Read;
    use std::os::unix::fs::symlink;
    use std::path::Path;

    #[test]
    fn descriptor_relative_open_refuses_directory_link_escape() {
        let temp = tempfile::tempdir().unwrap();
        let live = temp.path().join("live");
        let outside = temp.path().join("outside");
        fs::create_dir(&live).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret"), b"outside").unwrap();
        fs::write(live.join("valid"), b"inside").unwrap();
        symlink(&outside, live.join("escape")).unwrap();
        assert!(open_beneath(&live, Path::new("escape/secret")).is_err());
        let mut opened = open_beneath(&live, Path::new("valid")).unwrap();
        let mut bytes = Vec::new();
        opened.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"inside");
    }
}
