//! Trusted out-of-band composition. Authenticate launcher and startup source
//! custody in the deployment; a digest or owner string alone is not authority.
use claim_evidence::{
    ClaimError, ClaimResult, OpenFiles, RepositoryAdmissionBinding, RepositoryAdmissionPort,
    RepositoryLease, RepositoryProducerStartup, RepositoryReadStartup, sha256_bytes,
};
pub mod reassessment;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    os::fd::AsRawFd,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Startup {
    pub schema_version: u8,
    pub root: PathBuf,
    pub config: PathBuf,
    pub source: PathBuf,
    pub expected_config_sha256: String,
    pub expected_source_sha256: String,
    pub repository_id: String,
    pub decision_scope: String,
    pub actor: String,
    pub grant_id: String,
    pub allowed_actions: Vec<String>,
}
impl Startup {
    pub fn producer(&self) -> ClaimResult<RepositoryProducerStartup> {
        if self.schema_version != 1
            || self.actor.is_empty()
            || self.repository_id.is_empty()
            || self.decision_scope.is_empty()
            || self.grant_id.is_empty()
        {
            return Err(ClaimError::new("invalid trusted startup"));
        }
        Ok(RepositoryProducerStartup {
            read: RepositoryReadStartup {
                files: OpenFiles {
                    root: self.root.clone(),
                    config: self.config.clone(),
                    source: self.source.clone(),
                    expected_config_sha256: self.expected_config_sha256.clone(),
                    expected_source_sha256: self.expected_source_sha256.clone(),
                },
                repository_id: self.repository_id.clone(),
                decision_scope: self.decision_scope.clone(),
            },
            actor: self.actor.clone(),
            grant_id: self.grant_id.clone(),
            allowed_actions: self.allowed_actions.clone(),
        })
    }
}
/// Retains exact trusted startup/config/source bytes through the operation and
/// serializes producer admission through the canonical root's existing lock.
pub struct TrustedAdmission {
    path: PathBuf,
    sha: String,
    startup: Startup,
}
impl TrustedAdmission {
    pub fn load(path: &Path, expected_sha: &str) -> ClaimResult<Self> {
        let bytes = fs::read(path).map_err(|e| ClaimError::new(e.to_string()))?;
        if sha256_bytes(&bytes) != expected_sha {
            return Err(ClaimError::new("startup digest mismatch"));
        }
        let startup: Startup =
            serde_json::from_slice(&bytes).map_err(|e| ClaimError::new(e.to_string()))?;
        startup.producer()?;
        let this = Self {
            path: path.to_path_buf(),
            sha: expected_sha.into(),
            startup,
        };
        this.check()?;
        Ok(this)
    }
    pub fn startup(&self) -> &Startup {
        &self.startup
    }
    fn check(&self) -> ClaimResult<()> {
        for (path, sha) in [
            (&self.path, &self.sha),
            (&self.startup.config, &self.startup.expected_config_sha256),
            (&self.startup.source, &self.startup.expected_source_sha256),
        ] {
            let bytes = fs::read(path).map_err(|e| ClaimError::new(e.to_string()))?;
            if sha256_bytes(&bytes) != *sha {
                return Err(ClaimError::new("trusted startup/source changed"));
            }
        }
        Ok(())
    }
    fn check_binding(&self, b: &RepositoryAdmissionBinding) -> ClaimResult<()> {
        use claim_evidence::codec::{digest, field, parse_json};
        self.check()?;
        if b.repository_id != self.startup.repository_id
            || b.producer != self.startup.actor
            || b.decision_scope != self.startup.decision_scope
            || b.operation_id.is_empty()
            || b.request_sha256.len() != 64
            || !b
                .request_sha256
                .bytes()
                .all(|x| x.is_ascii_hexdigit() && !x.is_ascii_uppercase())
        {
            return Err(ClaimError::new("repository admission mismatch"));
        }
        let config = parse_json(
            &fs::read_to_string(&self.startup.config)
                .map_err(|e| ClaimError::new(e.to_string()))?,
        )?;
        if field(&config, "root_id")?.as_text()?.to_string_checked()? != b.root_id {
            return Err(ClaimError::new("admission root mismatch"));
        }
        let grant = field(&config, "grants")?
            .as_array()?
            .iter()
            .find(|g| {
                field(g, "grant_id")
                    .ok()
                    .and_then(|v| v.as_text().ok())
                    .and_then(|v| v.to_string_checked().ok())
                    .as_deref()
                    == Some(&self.startup.grant_id)
            })
            .ok_or_else(|| ClaimError::new("admission grant absent"))?;
        if digest(grant)? != b.authority_ref_sha256 {
            return Err(ClaimError::new("admission grant digest mismatch"));
        }
        Ok(())
    }
}
struct Lease<'a> {
    _file: File,
    admission: &'a TrustedAdmission,
    binding: RepositoryAdmissionBinding,
}
impl RepositoryLease for Lease<'_> {
    fn validate(&self) -> ClaimResult<()> {
        self.admission.check_binding(&self.binding)
    }
}
impl RepositoryAdmissionPort for TrustedAdmission {
    fn acquire<'a>(
        &'a self,
        b: &RepositoryAdmissionBinding,
    ) -> ClaimResult<Box<dyn RepositoryLease + 'a>> {
        self.check_binding(b)?;
        // Root marker is already pinned by the canonical owner; no client paths.
        let path = self.startup.root.join("repository-admission.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|e| ClaimError::new(e.to_string()))?;
        // Blocking is bounded by a busy result; the caller may inspect an entered operation.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(ClaimError::new("repository admission busy"));
        }
        self.check()?;
        Ok(Box::new(Lease {
            _file: file,
            admission: self,
            binding: b.clone(),
        }))
    }
}
