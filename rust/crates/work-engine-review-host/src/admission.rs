//! Host-selected startup composition. Requests remain data; production policy
//! selection belongs to the deployment owner at the fixed installed path.
use review_workflow::startup::TrustedStartup;
use slice_campaign::job_admission::{JobAdmissionEffect, JobReviewPolicy, JobReviewRequest};
use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::PathBuf,
};

#[derive(Clone)]
pub struct StartupInput {
    path: PathBuf,
    request: Option<PathBuf>,
    #[cfg(feature = "test-support")]
    fixture_policy: Option<PathBuf>,
}
impl StartupInput {
    pub fn parse(args: &[OsString]) -> Result<(bool, Self), String> {
        let initialize = match args.first().and_then(|a| a.to_str()) {
            Some("initialize") => true,
            Some("serve") => false,
            _ => return Err("usage: work-engine-review-host initialize|serve --startup <absolute-file> [--job-request <absolute-file>]".into()),
        };
        let mut path = None;
        let mut request = None;
        #[cfg(feature = "test-support")]
        let mut fixture_policy = None;
        let mut pairs = args[1..].chunks_exact(2);
        for pair in &mut pairs {
            let selected = match pair[0].to_str() {
                Some("--startup") => &mut path,
                Some("--job-request") => &mut request,
                #[cfg(feature = "test-support")]
                Some("--fixture-job-policy") => &mut fixture_policy,
                _ => return Err("unsupported host startup option".into()),
            };
            let value = PathBuf::from(&pair[1]);
            if selected.is_some() || !value.is_absolute() {
                return Err("startup options must be unique absolute paths".into());
            }
            *selected = Some(value);
        }
        if !pairs.remainder().is_empty() {
            return Err("startup option has no value".into());
        }
        let path = path.ok_or("startup path absent")?;
        #[cfg(feature = "test-support")]
        if fixture_policy.is_some() && request.is_none() {
            return Err("fixture job policy requires a job request".into());
        }
        Ok((
            initialize,
            Self {
                path,
                request,
                #[cfg(feature = "test-support")]
                fixture_policy,
            },
        ))
    }
    pub fn load(&self, initialize: bool) -> review_workflow::Result<TrustedStartup> {
        let Some(request_path) = &self.request else {
            return TrustedStartup::load(&self.path);
        };
        let request = read_request(request_path).map_err(|message| review_workflow::Error {
            kind: "startup",
            message,
            owner: None,
        })?;
        #[cfg(feature = "test-support")]
        let policy = if let Some(path) = &self.fixture_policy {
            JobReviewPolicy::open_fixture(path)
        } else {
            JobReviewPolicy::open_installed()
        };
        #[cfg(not(feature = "test-support"))]
        let policy = JobReviewPolicy::open_installed();
        let policy = policy?;
        let pin = if initialize {
            match policy.admit(&request)? {
                JobAdmissionEffect::Committed(pin) | JobAdmissionEffect::Replayed(pin) => pin,
                JobAdmissionEffect::OutcomeUnknown { admission_path } => {
                    return Err(review_workflow::Error {
                        kind: "admission_unknown",
                        message: format!(
                            "job admission outcome unknown at {}; reconcile this exact job before retry",
                            admission_path.display()
                        ),
                        owner: Some(
                            serde_json::json!({"owner":"slice-campaign","admissionPath":admission_path}),
                        ),
                    });
                }
            }
        } else {
            policy.reopen(&request)?
        };
        TrustedStartup::load_for_job(&self.path, pin)
    }
}
fn read_request(path: &PathBuf) -> Result<JobReviewRequest, String> {
    const LIMIT: usize = 64 * 1024;
    if fs::canonicalize(path).map_err(|e| e.to_string())? != *path {
        return Err("job request path must be canonical".into());
    }
    let before = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !before.is_file()
        || before.uid() != rustix::process::geteuid().as_raw()
        || before.mode() & 0o077 != 0
        || before.len() > LIMIT as u64
    {
        return Err("job request must be a uid-private regular file of at most64KiB".into());
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(|e| e.to_string())?;
    let opened = file.metadata().map_err(|e| e.to_string())?;
    if (before.dev(), before.ino()) != (opened.dev(), opened.ino()) {
        return Err("job request changed while opening".into());
    }
    let mut bytes = Vec::new();
    file.take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > LIMIT {
        return Err("job request exceeds64KiB".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("invalid job request: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }
    #[test]
    fn historical_and_job_routes_require_unique_absolute_inputs() {
        assert!(StartupInput::parse(&args(&["initialize", "--startup", "/startup"])).is_ok());
        assert!(
            StartupInput::parse(&args(&[
                "serve",
                "--job-request",
                "/request",
                "--startup",
                "/startup"
            ]))
            .is_ok()
        );
        for input in [
            vec!["serve", "--startup", "relative"],
            vec!["serve", "--startup", "/one", "--startup", "/two"],
            vec!["serve", "--startup"],
            vec!["serve", "--policy", "/authority"],
            vec!["unknown", "--startup", "/startup"],
        ] {
            assert!(StartupInput::parse(&args(&input)).is_err());
        }
    }
    #[cfg(not(feature = "test-support"))]
    #[test]
    fn production_has_no_fixture_policy_selector() {
        assert!(
            StartupInput::parse(&args(&[
                "initialize",
                "--startup",
                "/startup",
                "--job-request",
                "/request",
                "--fixture-job-policy",
                "/policy"
            ]))
            .is_err()
        );
    }
}
