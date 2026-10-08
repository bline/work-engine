//! Offline fixture admission. A request cannot construct a permit or select its registry.
//!
//! ```compile_fail
//! use review_episode::admission::AdmittedResult;
//! let _forged = AdmittedResult {};
//! ```
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use review_episode_core::codec::{JsValue, digest, field, parse_json};

use crate::protocol::Request;
use crate::{AppError, AppResult};

pub struct AdmittedResult {
    _private: (),
}

impl AdmittedResult {
    pub(crate) fn checked() -> Self {
        Self { _private: () }
    }
}

pub trait HostAdmissionPort {
    fn request_version(&self) -> u8;

    fn precheck(&self, _request: &Request) -> AppResult<()> {
        Ok(())
    }

    fn admit(
        &self,
        request: &Request,
        observed_revision: Option<&str>,
    ) -> AppResult<AdmittedResult>;
}

pub struct FixtureAdmission {
    registry: JsValue,
}

impl FixtureAdmission {
    pub fn load(path: &Path, root: &Path) -> AppResult<Self> {
        #[cfg(not(unix))]
        {
            let _ = (path, root);
            return Err(AppError::Admission(
                "offline fixture profile requires Unix".into(),
            ));
        }
        #[cfg(unix)]
        {
            use rustix::fs::{Mode, OFlags};
            use std::os::unix::fs::MetadataExt;
            let fd = rustix::fs::open(path, OFlags::RDONLY | OFlags::NOFOLLOW, Mode::empty())
                .map_err(|_| AppError::Admission("fixture registry cannot be opened".into()))?;
            let file = File::from(fd);
            let meta = file
                .metadata()
                .map_err(|_| AppError::Admission("fixture metadata unavailable".into()))?;
            let named = fs::symlink_metadata(path)
                .map_err(|_| AppError::Admission("fixture path unavailable".into()))?;
            if !meta.is_file()
                || !named.is_file()
                || named.file_type().is_symlink()
                || meta.dev() != named.dev()
                || meta.ino() != named.ino()
                || meta.nlink() != 1
                || meta.mode() & 0o077 != 0
            {
                return Err(AppError::Admission(
                    "fixture registry path is unsafe".into(),
                ));
            }
            let mut bytes = Vec::new();
            file.take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| AppError::Admission("fixture registry unreadable".into()))?;
            if bytes.len() > 8 * 1024 * 1024 {
                return Err(AppError::Admission("fixture registry exceeds limit".into()));
            }
            let text = std::str::from_utf8(&bytes)
                .map_err(|_| AppError::Admission("fixture registry is not UTF-8".into()))?;
            let registry = parse_json(text)
                .map_err(|_| AppError::Admission("fixture registry JSON is invalid".into()))?;
            let declared = field(&registry, "root")
                .map_err(|_| AppError::Admission("fixture root missing".into()))?
                .as_text()
                .map_err(|_| AppError::Admission("fixture root invalid".into()))?
                .to_string_checked()
                .map_err(|_| AppError::Admission("fixture root invalid".into()))?;
            let actual = root
                .canonicalize()
                .map_err(|_| AppError::Admission("offline root unavailable".into()))?;
            if declared != actual.to_string_lossy() {
                return Err(AppError::Admission("fixture root differs".into()));
            }
            field(&registry, "grants")
                .map_err(|_| AppError::Admission("fixture grants missing".into()))?
                .as_array()
                .map_err(|_| AppError::Admission("fixture grants invalid".into()))?;
            Ok(Self { registry })
        }
    }
}

impl HostAdmissionPort for FixtureAdmission {
    fn request_version(&self) -> u8 {
        1
    }

    fn admit(
        &self,
        request: &Request,
        observed_revision: Option<&str>,
    ) -> AppResult<AdmittedResult> {
        let grants = field(&self.registry, "grants")?.as_array()?;
        let expected_digest = digest(&request.envelope);
        let mut matches = 0;
        for grant in grants {
            let id = field(grant, "grantId")?.as_text()?.to_string_checked()?;
            if id != request.grant_id {
                continue;
            }
            matches += 1;
            let request_digest = field(grant, "requestDigest")?
                .as_text()?
                .to_string_checked()?;
            let selection_revision = field(grant, "selectionRevision")?
                .as_text()?
                .to_string_checked()?;
            let observed = field(grant, "observedRevision")?;
            let correct_observation = match (observed_revision, observed) {
                (None, JsValue::Null) => true,
                (Some(actual), JsValue::String(declared)) => {
                    declared.to_string_checked()? == actual
                }
                _ => false,
            };
            if request_digest != expected_digest
                || selection_revision != request.selection_revision.0
                || !correct_observation
            {
                return Err(AppError::Admission(
                    "fixture request or observed revision differs".into(),
                ));
            }
        }
        if matches != 1 {
            return Err(AppError::Admission(
                "exactly one offline fixture grant is required".into(),
            ));
        }
        Ok(AdmittedResult::checked())
    }
}
