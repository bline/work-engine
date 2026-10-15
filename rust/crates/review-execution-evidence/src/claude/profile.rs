use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Rp0Error {
    #[error("RP0 source or boundary I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("RP0 pinned program is not an owner-controlled regular executable")]
    UnsafeProgram,
    #[error("RP0 pinned program digest changed")]
    ProgramDrift,
    #[error("RP0 candidate contains an unsupported path or changed during freeze")]
    CandidateDrift,
    #[error("RP0 private directory is not empty or has unsafe ancestry")]
    UnsafePrivateRoot,
    #[error("RP0 private index build or readback failed")]
    IndexUnavailable,
    #[error("RP0 private index is not a complete self-contained store")]
    IncompleteStore,
    #[error("RP0 sandbox realization failed")]
    IsolationFailed,
    #[error("RP0 bounded peer output exceeded its limit")]
    OutputLimit,
    #[error("RP0 credential source is unsafe or unavailable")]
    UnsafeCredential,
}

/// A trusted installed path and its independently selected SHA-256. A serialized
/// path or digest never creates this value without checking the current bytes.
#[derive(Clone, Debug)]
pub struct PinnedProgram {
    path: PathBuf,
    sha256: String,
}

impl PinnedProgram {
    pub fn load(path: impl AsRef<Path>, expected_sha256: &str) -> Result<Self, Rp0Error> {
        if expected_sha256.len() != 64
            || !expected_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Rp0Error::ProgramDrift);
        }
        let path = path.as_ref();
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.file_type().is_file() {
            return Err(Rp0Error::UnsafeProgram);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            if metadata.uid() != unsafe { getuid() } && metadata.uid() != 0
                || metadata.permissions().mode() & 0o022 != 0
                || metadata.permissions().mode() & 0o111 == 0
            {
                return Err(Rp0Error::UnsafeProgram);
            }
        }
        let canonical = fs::canonicalize(path)?;
        let actual = sha256(&fs::read(&canonical)?);
        if actual != expected_sha256.to_ascii_lowercase() {
            return Err(Rp0Error::ProgramDrift);
        }
        Ok(Self {
            path: canonical,
            sha256: actual,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub(crate) fn verify(&self) -> Result<(), Rp0Error> {
        if sha256(&fs::read(&self.path)?) == self.sha256 {
            Ok(())
        } else {
            Err(Rp0Error::ProgramDrift)
        }
    }
}

unsafe extern "C" {
    fn getuid() -> u32;
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// RP0 pins exactly one installed Linux launcher, CBM build and restrictive
/// CBM child executable. It is not a provider selection or review grant.
#[derive(Clone, Debug)]
pub struct TrustedRp0Profile {
    pub(crate) bwrap: PinnedProgram,
    pub(crate) cbm: PinnedProgram,
    pub(crate) cbm_launcher: PinnedProgram,
    pub(crate) cbm_client: PinnedProgram,
}

impl TrustedRp0Profile {
    pub fn load(
        bwrap: PinnedProgram,
        cbm: PinnedProgram,
        cbm_launcher: PinnedProgram,
        cbm_client: PinnedProgram,
    ) -> Result<Self, Rp0Error> {
        let profile = Self {
            bwrap,
            cbm,
            cbm_launcher,
            cbm_client,
        };
        profile.verify()?;
        Ok(profile)
    }

    pub(crate) fn verify(&self) -> Result<(), Rp0Error> {
        self.bwrap.verify()?;
        self.cbm.verify()?;
        self.cbm_launcher.verify()?;
        self.cbm_client.verify()
    }
}
