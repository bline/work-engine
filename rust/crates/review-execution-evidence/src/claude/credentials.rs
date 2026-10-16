use std::ffi::CString;
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use super::Rp0Error;

const O_NOFOLLOW: i32 = 0o400000;
const O_DIRECTORY: i32 = 0o200000;
const O_CLOEXEC: i32 = 0o2000000;
const O_NONBLOCK: i32 = 0o4000;
const O_PATH: i32 = 0o10000000;
const O_RDONLY: i32 = 0;
const MAX_CREDENTIAL_BYTES: u64 = 64 * 1024;

/// The canonical credential owner installs this path. RP0 tests use only an
/// isolated synthetic canary file; reading a real source needs later authority.
#[derive(Clone)]
pub struct CredentialSource {
    path: PathBuf,
    parent_dev: u64,
    parent_ino: u64,
    source_dev: u64,
    source_ino: u64,
}

impl CredentialSource {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Rp0Error> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(Rp0Error::UnsafeCredential);
        }
        let (parent, basename) = open_parent_no_symlinks(path)?;
        // The final component has not been type-checked yet. O_NONBLOCK keeps
        // an owner-replaced FIFO from suspending this synchronous constructor.
        let file = open_relative(
            &parent,
            &basename,
            O_RDONLY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK,
        )?;
        let source_meta = file.metadata()?;
        validate_source_file(&source_meta)?;
        let metadata = parent.metadata()?;
        Ok(Self {
            path: path.into(),
            parent_dev: metadata.dev(),
            parent_ino: metadata.ino(),
            source_dev: source_meta.dev(),
            source_ino: source_meta.ino(),
        })
    }

    pub fn read_access_only(
        &self,
        now_millis: i64,
        required_horizon_millis: i64,
    ) -> Result<AccessProjection, Rp0Error> {
        self.read_access_only_with_identity(now_millis, required_horizon_millis, false)
    }

    /// RP1 binds the selected file identity for one initial attempt. RP0's
    /// established access-only projection still permits safe file rotation.
    pub(crate) fn read_access_only_exact(
        &self,
        now_millis: i64,
        required_horizon_millis: i64,
    ) -> Result<AccessProjection, Rp0Error> {
        self.read_access_only_with_identity(now_millis, required_horizon_millis, true)
    }

    fn read_access_only_with_identity(
        &self,
        now_millis: i64,
        required_horizon_millis: i64,
        exact_file: bool,
    ) -> Result<AccessProjection, Rp0Error> {
        if required_horizon_millis <= 0 {
            return Err(Rp0Error::UnsafeCredential);
        }
        let (parent, basename) = open_parent_no_symlinks(&self.path)?;
        let parent_meta = parent.metadata()?;
        if parent_meta.dev() != self.parent_dev || parent_meta.ino() != self.parent_ino {
            return Err(Rp0Error::UnsafeCredential);
        }
        let mut file = open_relative(
            &parent,
            &basename,
            O_RDONLY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK,
        )?;
        let source_meta = file.metadata()?;
        validate_source_file(&source_meta)?;
        if exact_file
            && (source_meta.dev() != self.source_dev || source_meta.ino() != self.source_ino)
        {
            return Err(Rp0Error::UnsafeCredential);
        }
        let mut raw = Vec::new();
        (&mut file)
            .take(MAX_CREDENTIAL_BYTES + 1)
            .read_to_end(&mut raw)?;
        if raw.len() as u64 > MAX_CREDENTIAL_BYTES {
            return Err(Rp0Error::UnsafeCredential);
        }
        let value: Value = serde_json::from_slice(&raw).map_err(|_| Rp0Error::UnsafeCredential)?;
        let oauth = value
            .get("claudeAiOauth")
            .ok_or(Rp0Error::UnsafeCredential)?;
        let token = oauth
            .get("accessToken")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty() && !token.chars().any(char::is_whitespace))
            .ok_or(Rp0Error::UnsafeCredential)?;
        let expiry = oauth
            .get("expiresAt")
            .and_then(Value::as_i64)
            .ok_or(Rp0Error::UnsafeCredential)?;
        if expiry <= now_millis.saturating_add(required_horizon_millis) {
            return Err(Rp0Error::UnsafeCredential);
        }
        Ok(AccessProjection {
            token: token.to_owned(),
            expires_at_millis: expiry,
        })
    }
}

unsafe extern "C" {
    fn getuid() -> u32;
    fn openat(dirfd: i32, pathname: *const std::ffi::c_char, flags: i32, mode: i32) -> i32;
}

fn open_relative(parent: &File, name: &CString, flags: i32) -> Result<File, Rp0Error> {
    // SAFETY: name is NUL-terminated, parent remains open, and the returned fd
    // is uniquely owned by File. No path component is resolved through a link.
    let fd = unsafe { openat(parent.as_raw_fd(), name.as_ptr(), flags, 0) };
    if fd < 0 {
        return Err(Rp0Error::UnsafeCredential);
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn open_parent_no_symlinks(path: &Path) -> Result<(File, CString), Rp0Error> {
    if !path.is_absolute() {
        return Err(Rp0Error::UnsafeCredential);
    }
    let names = path
        .components()
        .map(|component| match component {
            Component::RootDir => Ok(None),
            Component::Normal(name) => CString::new(name.as_bytes())
                .map(Some)
                .map_err(|_| Rp0Error::UnsafeCredential),
            _ => Err(Rp0Error::UnsafeCredential),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut names = names.into_iter().flatten().collect::<Vec<_>>();
    let basename = names.pop().ok_or(Rp0Error::UnsafeCredential)?;
    let mut parent = File::open("/")?;
    validate_ancestor(&parent.metadata()?)?;
    for name in names {
        parent = open_relative(
            &parent,
            &name,
            O_PATH | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC,
        )?;
        validate_ancestor(&parent.metadata()?)?;
    }
    Ok((parent, basename))
}

fn validate_ancestor(metadata: &fs::Metadata) -> Result<(), Rp0Error> {
    if !metadata.is_dir()
        || (metadata.uid() != unsafe { getuid() } && metadata.uid() != 0)
        || metadata.mode() & 0o002 != 0
    {
        return Err(Rp0Error::UnsafeCredential);
    }
    Ok(())
}

fn validate_source_file(metadata: &fs::Metadata) -> Result<(), Rp0Error> {
    if !metadata.is_file()
        || metadata.uid() != unsafe { getuid() }
        || metadata.mode() & 0o077 != 0
        || metadata.len() > MAX_CREDENTIAL_BYTES
    {
        return Err(Rp0Error::UnsafeCredential);
    }
    Ok(())
}

/// Secret bytes are private and deliberately absent from Debug, serialization,
/// hashes, receipts and the CBM child environment.
pub struct AccessProjection {
    token: String,
    expires_at_millis: i64,
}

impl AccessProjection {
    pub fn expires_at_millis(&self) -> i64 {
        self.expires_at_millis
    }

    /// Only the later provider owner should use this access-only projection.
    pub fn with_token<T>(&self, apply: impl FnOnce(&str) -> T) -> T {
        apply(&self.token)
    }
}
