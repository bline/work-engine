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

    pub fn verify(&self) -> Result<(), Rp0Error> {
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

pub fn sha256(bytes: &[u8]) -> String {
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

    pub fn verify(&self) -> Result<(), Rp0Error> {
        self.bwrap.verify()?;
        self.cbm.verify()?;
        self.cbm_launcher.verify()?;
        self.cbm_client.verify()
    }
}

/// Exact installed command and catalog. This selection is descriptive until a
/// workflow owner supplies an in-process entry permit. RP1 exposes only the
/// offline fixture installer; production installation belongs to RP2.
#[derive(Clone)]
pub struct TrustedClaudeProfile {
    program: PinnedProgram,
    guard: Option<PinnedProgram>,
    installed: bool,
    model: String,
    effort: String,
    accepted_observed_models: Vec<String>,
    catalog_sha256: String,
    digest: String,
}

impl TrustedClaudeProfile {
    #[cfg(feature = "test-support")]
    pub fn load_offline_peer(
        program: PinnedProgram,
        model: &str,
        effort: &str,
        accepted_observed_models: Vec<String>,
        catalog: &[u8],
    ) -> Result<Self, Rp0Error> {
        Self::load_checked(
            program,
            None,
            false,
            model,
            effort,
            accepted_observed_models,
            catalog,
        )
    }

    #[cfg(feature = "test-support")]
    pub fn load_offline_guarded_peer(
        program: PinnedProgram,
        guard: PinnedProgram,
        model: &str,
        effort: &str,
        accepted_observed_models: Vec<String>,
        catalog: &[u8],
    ) -> Result<Self, Rp0Error> {
        Self::load_checked(
            program,
            Some(guard),
            false,
            model,
            effort,
            accepted_observed_models,
            catalog,
        )
    }

    /// A checked installed selection remains inert until an Applied owner
    /// supplies a private entry permit. RP1 does not issue that permit.
    pub fn load_installed(
        program: PinnedProgram,
        guard: PinnedProgram,
        model: &str,
        effort: &str,
        accepted_observed_models: Vec<String>,
        catalog: &[u8],
    ) -> Result<Self, Rp0Error> {
        Self::load_checked(
            program,
            Some(guard),
            true,
            model,
            effort,
            accepted_observed_models,
            catalog,
        )
    }

    #[cfg_attr(not(feature = "test-support"), allow(dead_code))]
    fn load_checked(
        program: PinnedProgram,
        guard: Option<PinnedProgram>,
        installed: bool,
        model: &str,
        effort: &str,
        accepted_observed_models: Vec<String>,
        catalog: &[u8],
    ) -> Result<Self, Rp0Error> {
        program.verify()?;
        if let Some(guard) = &guard {
            guard.verify()?;
        }
        if model.is_empty()
            || model.len() > 128
            || !model
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._".contains(&b))
            || !matches!(effort, "low" | "medium" | "high")
            || accepted_observed_models.is_empty()
            || accepted_observed_models.len() > 8
            || accepted_observed_models.iter().any(|observed| {
                observed.is_empty()
                    || observed.len() > 128
                    || !observed
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-._".contains(&b))
            })
            || catalog.is_empty()
            || catalog.len() > 64 * 1024
            || claim_evidence::codec::parse_json(
                std::str::from_utf8(catalog).map_err(|_| Rp0Error::IsolationFailed)?,
            )
            .is_err()
        {
            return Err(Rp0Error::IsolationFailed);
        }
        let catalog_sha256 = sha256(catalog);
        let digest = crate::contract::record_digest(&[
            if installed {
                b"claude-installed-profile-v1"
            } else {
                b"claude-offline-profile-v1"
            },
            program.sha256().as_bytes(),
            guard.as_ref().map_or("", PinnedProgram::sha256).as_bytes(),
            model.as_bytes(),
            effort.as_bytes(),
            catalog_sha256.as_bytes(),
            accepted_observed_models.join("\0").as_bytes(),
        ]);
        Ok(Self {
            program,
            guard,
            installed,
            model: model.into(),
            effort: effort.into(),
            accepted_observed_models,
            catalog_sha256,
            digest,
        })
    }

    pub fn profile_digest(&self) -> &str {
        &self.digest
    }
    pub fn requested_model(&self) -> &str {
        &self.model
    }
    pub fn requested_effort(&self) -> &str {
        &self.effort
    }
    pub fn catalog_sha256(&self) -> &str {
        &self.catalog_sha256
    }
    pub fn program(&self) -> &PinnedProgram {
        &self.program
    }
    pub fn guard(&self) -> Option<&PinnedProgram> {
        self.guard.as_ref()
    }
    pub fn installed(&self) -> bool {
        self.installed
    }
    pub fn accepts_observed_model(&self, value: &str) -> bool {
        self.accepted_observed_models
            .iter()
            .any(|accepted| accepted == value)
    }
    pub fn verify(&self) -> Result<(), Rp0Error> {
        self.program.verify()?;
        if let Some(guard) = &self.guard {
            guard.verify()?;
        }
        Ok(())
    }
}
