use crate::EvidenceReadError;
use crate::contract::{require_sha, require_text, sha256};
use crate::store_read::{self, AnchorIds};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
const SESSION_LIMIT: u64 = 64 * 1024 * 1024;
const MAX_FILES: usize = 4096;
const ENTRY_CHARGE: u64 = 4096;

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
/// ```
/// use review_execution_custody::claude::RetainedReviewSession;
/// fn observe(_session: Option<&RetainedReviewSession>) {}
/// observe(None);
/// ```
///
/// ```compile_fail
/// use review_execution_custody::claude::RetainedReviewSession;
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
        if store_read::anchor_ids(&root)? != anchors {
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

    pub fn verify_predecessor(&self) -> Result<(), EvidenceReadError> {
        if store_read::anchor_ids(&self.root)? != self.anchors {
            return Err(EvidenceReadError::Conflicting(
                "retained session root anchors changed",
            ));
        }
        verify_live(&self.root, &self.ready())?;
        verify_chain(&self.root, &self.chain)
    }

    pub fn chain(&self) -> &[SessionManifest] {
        &self.chain
    }
    pub fn ready(&self) -> SessionReady {
        SessionReady {
            dev: self.live_dev,
            ino: self.live_ino,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionReady {
    pub dev: u64,
    pub ino: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionFile {
    pub path: String,
    pub len: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionManifest {
    pub schema_version: u32,
    pub attempt_id: String,
    pub session_id: String,
    pub predecessor_sha256: Option<String>,
    pub predecessor_attempt_id: Option<String>,
    pub snapshot_dir: String,
    pub files: Vec<SessionFile>,
}

pub fn verify_live(root: &Path, ready: &SessionReady) -> Result<(), EvidenceReadError> {
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
pub fn verify_snapshot(root: &Path, manifest: &SessionManifest) -> Result<(), EvidenceReadError> {
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

pub fn manifest_digest(manifest: &SessionManifest) -> Result<String, EvidenceReadError> {
    Ok(sha256(&serde_json::to_vec(manifest).map_err(|_| {
        EvidenceReadError::Conflicting("session manifest serialization failed")
    })?))
}

pub fn verify_chain(root: &Path, chain: &[SessionManifest]) -> Result<(), EvidenceReadError> {
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

pub fn verify_live_manifest(
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
