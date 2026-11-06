//! Exact job admission derived from deployment-owned policy. Private bytes are
//! data until rederived from the currently installed policy. No execution ledger.
use crate::codec::raw_sha256;
use crate::installed_authority::{AcceptanceRecord, read_protected};
use crate::{CampaignError, CampaignIdentity, InstalledAcceptancePin, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const POLICY_PATH: &str = "/etc/work-engine/review-host/job-policy.json";
const LIMIT: usize = 64 * 1024;

/// Installed delegation, not a job request. Fixed execution and claim fields
/// use the historical acceptance codec; subject fields are replaced by the owner.
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobReviewPolicyDocument {
    pub schema_version: u32,
    pub policy_id: String,
    pub revision: String,
    pub active: bool,
    pub expires_at_millis: u64,
    pub admitted_uid: u32,
    pub repository: PathBuf,
    pub jobs_root: PathBuf,
    pub execution_template: serde_json::Value,
    pub review_selection_template: serde_json::Value,
    pub startup_authority: JobStartupAuthority,
}

/// Exact claims bootstrap selected by the deployment owner. Source/configuration
/// hashes bind source identity, root, both grants and their decision scopes.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobStartupAuthority {
    pub claims_source_sha256: String,
    pub claims_configuration_sha256: String,
    pub completion_consumer: String,
}

/// No policy path, approved flag, executable or model can be supplied here.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobReviewRequest {
    pub identity: CampaignIdentity,
    pub candidate: crate::CandidateRef,
    pub physical_profile_digest: String,
    pub cbm_store_sha256: String,
    pub session_id: String,
}

impl JobReviewRequest {
    /// One deterministic campaign location per attempt identity. Changing a
    /// caller label cannot mint a second root for an already dispatched job.
    pub fn directory_name(&self) -> Result<String> {
        self.identity.validate()?;
        Ok(raw_sha256(
            &serde_json::to_vec(
                &serde_json::json!({"profile":"job-review-directory-v1","identity":self.identity}),
            )
            .map_err(|e| CampaignError::Contract(e.to_string()))?,
        ))
    }
}

/// A policy document or request cannot deserialize this capability.
/// ```compile_fail
/// let _: slice_campaign::job_admission::JobReviewPolicy = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone)]
pub struct JobReviewPolicy {
    path: PathBuf,
    digest: String,
    fixture: bool,
    document: JobReviewPolicyDocument,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AdmissionDocument {
    schema_version: u32,
    standing: String,
    policy_id: String,
    policy_revision: String,
    policy_sha256: String,
    repository: PathBuf,
    campaign_root: PathBuf,
    request: JobReviewRequest,
    acceptance: serde_json::Value,
    review_selection: serde_json::Value,
}

/// Retained trusted origin, never deserializable from an admission document.
#[derive(Clone)]
pub struct JobAdmissionRef {
    policy: JobReviewPolicy,
    request: JobReviewRequest,
    path: PathBuf,
}

pub enum JobAdmissionEffect {
    Committed(InstalledAcceptancePin),
    Replayed(InstalledAcceptancePin),
    /// The caller must reconcile the same path, never create another job.
    OutcomeUnknown {
        admission_path: PathBuf,
    },
}

fn now() -> Result<u64> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| CampaignError::Root(e.to_string()))?
            .as_millis(),
    )
    .map_err(|_| CampaignError::Root("clock exceeds job policy range".into()))
}
fn io(e: std::io::Error) -> CampaignError {
    CampaignError::Io(e.to_string())
}
fn canonical(path: &Path) -> Result<()> {
    if !path.is_absolute() || fs::canonicalize(path).ok().as_deref() != Some(path) {
        return Err(CampaignError::Root(
            "job policy path is not canonical".into(),
        ));
    }
    Ok(())
}
fn private_directory(path: &Path) -> Result<()> {
    canonical(path)?;
    let m = fs::symlink_metadata(path).map_err(io)?;
    if !m.is_dir() || m.uid() != rustix::process::geteuid().as_raw() || m.mode() & 0o077 != 0 {
        return Err(CampaignError::Root(
            "job directory must be uid-private".into(),
        ));
    }
    Ok(())
}
fn private_bytes(path: &Path) -> Result<Vec<u8>> {
    canonical(path)?;
    let before = fs::symlink_metadata(path).map_err(io)?;
    if !before.is_file()
        || before.uid() != rustix::process::geteuid().as_raw()
        || before.mode() & 0o077 != 0
        || before.len() > LIMIT as u64
    {
        return Err(CampaignError::Root("unsafe job admission file".into()));
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(io)?;
    let opened = file.metadata().map_err(io)?;
    if (opened.dev(), opened.ino()) != (before.dev(), before.ino()) {
        return Err(CampaignError::Root("job admission changed".into()));
    }
    use std::io::Read;
    let mut raw = Vec::new();
    file.take((LIMIT + 1) as u64)
        .read_to_end(&mut raw)
        .map_err(io)?;
    if raw.len() > LIMIT {
        return Err(CampaignError::Contract(
            "job admission exceeds limit".into(),
        ));
    }
    Ok(raw)
}
impl JobReviewPolicy {
    /// Only the deployment-selected fixed path can create production policy.
    pub fn open_installed() -> Result<Self> {
        Self::load(Path::new(POLICY_PATH), false)
    }
    #[cfg(feature = "test-support")]
    /// Explicit fixture authority. It can qualify admission/consumer checks but
    /// the execution consumer refuses to launch it as qualified provider work.
    pub fn open_fixture(path: &Path) -> Result<Self> {
        Self::load(path, true)
    }
    fn load(path: &Path, fixture: bool) -> Result<Self> {
        let raw = if fixture {
            private_bytes(path)?
        } else {
            read_protected(path)?
        };
        let document: JobReviewPolicyDocument =
            serde_json::from_slice(&raw).map_err(|e| CampaignError::Contract(e.to_string()))?;
        if document.schema_version != 1
            || !document.active
            || document.expires_at_millis <= now()?
            || document.admitted_uid != rustix::process::geteuid().as_raw()
        {
            return Err(CampaignError::Unsupported(
                "job policy inactive, expired or wrong uid".into(),
            ));
        }
        crate::require_text(&document.policy_id, "job policy id")?;
        crate::require_text(&document.revision, "job policy revision")?;
        crate::require_sha(
            &document.startup_authority.claims_source_sha256,
            "job claims source",
        )?;
        crate::require_sha(
            &document.startup_authority.claims_configuration_sha256,
            "job claims configuration",
        )?;
        crate::require_text(
            &document.startup_authority.completion_consumer,
            "job completion consumer",
        )?;
        canonical(&document.repository)?;
        private_directory(&document.jobs_root)?;
        let template = AcceptanceRecord::parse(
            &serde_json::to_vec(&document.execution_template)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
        )?;
        let text = |key: &str| -> Result<String> {
            document.review_selection_template["subject"][key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| CampaignError::Contract("policy selection subject absent".into()))
        };
        let candidate = crate::CandidateRef {
            commit: text("commit")?,
            tree: text("tree")?,
            patch_identity: text("patchIdentity")?,
            receipt_sha256: String::new(),
            physical_profile_sha256: String::new(),
        };
        crate::contract::validate_selection(
            &document.review_selection_template,
            &template.identity,
            &candidate,
        )?;
        if document.review_selection_template["specialists"]
            .as_array()
            .map(Vec::len)
            != Some(1)
            || document.review_selection_template["specialists"][0]["obligationId"]
                != template.obligation_id
        {
            return Err(CampaignError::Contract(
                "policy selection must own one exact obligation".into(),
            ));
        }
        Ok(Self {
            path: path.into(),
            digest: raw_sha256(&raw),
            fixture,
            document,
        })
    }
    fn current(&self) -> Result<Self> {
        let current = Self::load(&self.path, self.fixture)?;
        if current.digest != self.digest {
            return Err(CampaignError::Conflict(
                "installed job policy changed".into(),
            ));
        }
        Ok(current)
    }
    /// Preserve the installed claim contracts, changing only their exact
    /// subject/episode/consumer identities through the owning selection codec.
    pub fn review_selection(&self, request: &JobReviewRequest) -> Result<serde_json::Value> {
        self.current()?;
        let template = AcceptanceRecord::parse(
            &serde_json::to_vec(&self.document.execution_template)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
        )?;
        let mut selection = self.document.review_selection_template.clone();
        let subject = serde_json::json!({"commit":request.candidate.commit,"tree":request.candidate.tree,"patchIdentity":request.candidate.patch_identity});
        selection["subject"] = subject.clone();
        let specialists = selection["specialists"]
            .as_array_mut()
            .ok_or_else(|| CampaignError::Contract("job policy specialists absent".into()))?;
        if specialists.len() != 1
            || specialists[0]["obligationId"] != template.obligation_id
            || specialists[0]["selection"] != "selected"
        {
            return Err(CampaignError::Contract(
                "job policy requires one exact selected obligation".into(),
            ));
        }
        let episode = crate::contract::episode_digest(
            &serde_json::json!({"identity":request.identity,"obligationId":template.obligation_id}),
        )?;
        let claims = specialists[0]["requiredClaims"]
            .as_array_mut()
            .ok_or_else(|| CampaignError::Contract("job policy claim contracts absent".into()))?;
        for claim in claims {
            claim["subject"] =
                serde_json::json!({"candidate":subject,"reviewEpisodeId":episode[..32]});
            let consumer = match claim["consumptionBoundary"].as_str() {
                Some("builder_projection") => format!("slice-builder:{}", request.identity.key()),
                Some("campaign_terminalization") => {
                    format!("slice-campaign:{}", request.identity.key())
                }
                _ => {
                    return Err(CampaignError::Contract(
                        "job policy claim boundary unsupported".into(),
                    ));
                }
            };
            claim["consumer"] = serde_json::json!(consumer);
            let id_body = serde_json::json!({"proposition":claim["proposition"],"subject":claim["subject"],"coveredState":claim["coveredState"],"consumptionBoundary":claim["consumptionBoundary"],"consumer":claim["consumer"]});
            claim["claimId"] = serde_json::json!(format!(
                "production-path-claim-v1@{}",
                crate::contract::claim_digest(&id_body)?
            ));
            let mut revision_body = claim.clone();
            revision_body
                .as_object_mut()
                .ok_or_else(|| CampaignError::Contract("job policy claim malformed".into()))?
                .remove("revision");
            claim["revision"] = serde_json::json!(format!(
                "production-path-claim-revision-v1@{}",
                crate::contract::claim_digest(&revision_body)?
            ));
        }
        crate::contract::validate_selection(&selection, &request.identity, &request.candidate)?;
        Ok(selection)
    }
    fn derive(&self, request: &JobReviewRequest) -> Result<(PathBuf, Vec<u8>)> {
        request.identity.validate()?;
        let template = AcceptanceRecord::parse(
            &serde_json::to_vec(&self.document.execution_template)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
        )?;
        if request.identity.run_id != template.identity.run_id
            || request.identity.plan_version != template.identity.plan_version
            || request.identity.slice_number != template.identity.slice_number
        {
            return Err(CampaignError::Conflict(
                "job outside delegated run/plan/slice".into(),
            ));
        }
        let dir = self.document.jobs_root.join(request.directory_name()?);
        private_directory(&dir)?;
        let path = dir.join("review-admission.json");
        let mut acceptance = self.document.execution_template.clone();
        acceptance["reference"] = serde_json::json!(path);
        acceptance["revision"] = serde_json::json!(format!(
            "job-review-v1:{}:{}",
            self.document.revision,
            request.directory_name()?
        ));
        acceptance["identity"] = serde_json::to_value(&request.identity)
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        acceptance["attemptId"] = serde_json::json!(request.identity.attempt_id);
        let review_selection = self.review_selection(request)?;
        let selected = crate::contract::validate_selection(
            &review_selection,
            &request.identity,
            &request.candidate,
        )?;
        acceptance["candidateTree"] = serde_json::json!(request.candidate.tree);
        acceptance["selectionDigest"] = serde_json::json!(selected.campaign_digest);
        for claim in review_selection["specialists"][0]["requiredClaims"]
            .as_array()
            .unwrap()
        {
            let field = if claim["consumptionBoundary"] == "builder_projection" {
                "builderClaimId"
            } else {
                "campaignClaimId"
            };
            acceptance[field] = claim["claimId"].clone();
        }
        acceptance["physicalProfileDigest"] = serde_json::json!(request.physical_profile_digest);
        acceptance["cbmStoreSha256"] = serde_json::json!(request.cbm_store_sha256);
        acceptance["sessionId"] = serde_json::json!(request.session_id);
        let parsed = AcceptanceRecord::parse(
            &serde_json::to_vec(&acceptance).map_err(|e| CampaignError::Contract(e.to_string()))?,
        )?;
        if parsed.identity.attempt_id != parsed.attempt_id {
            return Err(CampaignError::Contract("job attempt differs".into()));
        }
        let doc = AdmissionDocument {
            schema_version: 1,
            standing: if self.fixture {
                "fixture"
            } else {
                "installed-policy"
            }
            .into(),
            policy_id: self.document.policy_id.clone(),
            policy_revision: self.document.revision.clone(),
            policy_sha256: self.digest.clone(),
            repository: self.document.repository.clone(),
            campaign_root: dir.join("campaign"),
            request: request.clone(),
            acceptance,
            review_selection,
        };
        let raw = serde_json::to_vec(&doc).map_err(|e| CampaignError::Contract(e.to_string()))?;
        if raw.len() > LIMIT {
            return Err(CampaignError::Contract(
                "job admission exceeds limit".into(),
            ));
        }
        Ok((path, raw))
    }
    pub fn reopen(&self, request: &JobReviewRequest) -> Result<InstalledAcceptancePin> {
        let current = self.current()?;
        let (path, expected) = current.derive(request)?;
        let raw = private_bytes(&path)?;
        if raw != expected {
            return Err(CampaignError::Conflict(
                "job admission is not exact policy derivation".into(),
            ));
        }
        let document: AdmissionDocument =
            serde_json::from_slice(&raw).map_err(|e| CampaignError::Contract(e.to_string()))?;
        let record = AcceptanceRecord::parse(
            &serde_json::to_vec(&document.acceptance)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
        )?;
        Ok(InstalledAcceptancePin {
            digest: raw_sha256(&raw),
            record,
            job: Some(Box::new(JobAdmissionRef {
                policy: current,
                request: request.clone(),
                path,
            })),
        })
    }
    /// Publication is exclusive and durable. An orphan staging file is uncertain,
    /// not permission to overwrite it or mint replacement execution authority.
    pub fn admit(&self, request: &JobReviewRequest) -> Result<JobAdmissionEffect> {
        let current = self.current()?;
        let (path, raw) = current.derive(request)?;
        if path.exists() {
            return Ok(JobAdmissionEffect::Replayed(current.reopen(request)?));
        }
        let staging = path.with_extension("pending");
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&staging)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Ok(JobAdmissionEffect::OutcomeUnknown {
                    admission_path: path,
                });
            }
            Err(e) => return Err(io(e)),
        };
        if file.write_all(&raw).and_then(|_| file.sync_all()).is_err() {
            return Ok(JobAdmissionEffect::OutcomeUnknown {
                admission_path: path,
            });
        }
        // Recheck revocation before publishing. Staging is retained on refusal.
        current.current()?;
        if fs::hard_link(&staging, &path).is_err() {
            return Ok(JobAdmissionEffect::OutcomeUnknown {
                admission_path: path,
            });
        }
        if File::open(path.parent().unwrap())
            .and_then(|f| f.sync_all())
            .and_then(|_| File::open(&current.document.jobs_root))
            .and_then(|f| f.sync_all())
            .is_err()
        {
            return Ok(JobAdmissionEffect::OutcomeUnknown {
                admission_path: path,
            });
        }
        // Retain staging as immutable custody of this publication; it is not a
        // retry token. Reopening always verifies final bytes against policy.
        Ok(JobAdmissionEffect::Committed(current.reopen(request)?))
    }
}
impl JobAdmissionRef {
    pub(crate) fn validate_candidate(&self, candidate: &crate::CandidateRef) -> Result<()> {
        if candidate != &self.request.candidate {
            return Err(CampaignError::Conflict(
                "job candidate artifacts differ from admission".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn validate_startup_authority(&self, selected: &JobStartupAuthority) -> Result<()> {
        self.reopen()?;
        if selected != &self.policy.document.startup_authority {
            return Err(CampaignError::Conflict(
                "job startup authority differs from policy".into(),
            ));
        }
        Ok(())
    }
    pub fn reopen(&self) -> Result<InstalledAcceptancePin> {
        self.policy.reopen(&self.request)
    }
    pub(crate) fn verify(&self, digest: &str) -> Result<()> {
        let pin = self.reopen()?;
        if pin.digest() != digest {
            return Err(CampaignError::Conflict(
                "job admission digest changed".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn fixture(&self) -> bool {
        self.policy.fixture
    }
    pub(crate) fn validate_child(&self, path: &Path, child: &str) -> Result<()> {
        if ![
            "startup.json",
            "workspace",
            "campaign",
            "episode",
            "claims",
            "evidence",
        ]
        .contains(&child)
            || path != self.path.parent().unwrap().join(child)
        {
            return Err(CampaignError::Conflict(
                "job owner root outside admitted layout".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn validate_paths(&self, repository: &Path, campaign: &Path) -> Result<()> {
        if repository != self.policy.document.repository
            || campaign != self.path.parent().unwrap().join("campaign")
        {
            return Err(CampaignError::Conflict(
                "job repository/campaign root outside policy".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    fn fixture() -> (
        tempfile::TempDir,
        JobReviewPolicyDocument,
        JobReviewPolicy,
        JobReviewRequest,
    ) {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let repo = temp.path().join("repo");
        fs::create_dir(&repo).unwrap();
        let identity = CampaignIdentity {
            run_id: "fixture-run".into(),
            plan_version: "fixture-plan".into(),
            slice_number: 1,
            attempt_id: "fresh-attempt".into(),
        };
        let template = json!({"schemaVersion":1,"decision":"accepted","reference":"fixture-template","revision":"v1","identity":identity,"obligationId":"review","candidateTree":"1".repeat(40),"builderClaimId":"builder","campaignClaimId":"campaign","selectionDigest":"1".repeat(64),"physicalProfileDigest":"2".repeat(64),"claudeProfileDigest":"3".repeat(64),"programSha256":"4".repeat(64),"guardSha256":"5".repeat(64),"cbmStoreSha256":"6".repeat(64),"catalogSha256":"7".repeat(64),"requestedModel":"fixture-model","credentialSourceMetadataSha256":"8".repeat(64),"resourcePolicy":"strict_aggregate_session_64mib_v1","sessionId":"template-session","attemptId":"fresh-attempt","acceptedBoundarySha256":"9".repeat(64),"promptTemplateSha256":"a".repeat(64),"attemptHorizonMillis":1000});
        let doc = JobReviewPolicyDocument {
            schema_version: 1,
            policy_id: "fixture-policy".into(),
            revision: "r1".into(),
            active: true,
            expires_at_millis: u64::MAX,
            admitted_uid: rustix::process::geteuid().as_raw(),
            repository: repo,
            jobs_root: temp.path().into(),
            startup_authority: JobStartupAuthority {
                claims_source_sha256: "b".repeat(64),
                claims_configuration_sha256: "c".repeat(64),
                completion_consumer: "fixture-completion".into(),
            },
            execution_template: template,
            review_selection_template: {
                let subject = json!({"commit":"1".repeat(40),"tree":"1".repeat(40),"patchIdentity":"1".repeat(64)});
                json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"policy-selection","subject":subject,"specialists":[{"obligationId":"review","skill":"implementation-review","selection":"selected","requiredClaims":[crate::test_support::claim(&identity,"review",&subject,"builder_projection"),crate::test_support::claim(&identity,"review",&subject,"campaign_terminalization")]}]})
            },
        };
        let path = temp.path().join("policy.json");
        fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let policy = JobReviewPolicy::open_fixture(&path).unwrap();
        let request = JobReviewRequest {
            identity,
            candidate: crate::CandidateRef {
                commit: "b".repeat(40),
                tree: "b".repeat(40),
                patch_identity: "b".repeat(64),
                receipt_sha256: "b".repeat(64),
                physical_profile_sha256: "b".repeat(64),
            },
            physical_profile_digest: "c".repeat(64),
            cbm_store_sha256: "d".repeat(64),
            session_id: "fresh-session".into(),
        };
        let job = temp.path().join(request.directory_name().unwrap());
        fs::create_dir(&job).unwrap();
        fs::set_permissions(&job, fs::Permissions::from_mode(0o700)).unwrap();
        (temp, doc, policy, request)
    }
    #[test]
    fn exact_admission_reopens_and_changed_subject_cannot_replay() {
        let (_temp, _doc, policy, request) = fixture();
        let JobAdmissionEffect::Committed(pin) = policy.admit(&request).unwrap() else {
            panic!("fresh")
        };
        assert_eq!(pin.digest(), policy.reopen(&request).unwrap().digest());
        assert!(matches!(
            policy.admit(&request).unwrap(),
            JobAdmissionEffect::Replayed(_)
        ));
        for field in 0..7 {
            let mut changed = request.clone();
            match field {
                0 => changed.candidate.tree = "e".repeat(40),
                1 => changed.session_id = "different".into(),
                2 => changed.identity.attempt_id = "different".into(),
                3 => changed.candidate.patch_identity = "e".repeat(64),
                4 => changed.physical_profile_digest = "f".repeat(64),
                5 => changed.cbm_store_sha256 = "f".repeat(64),
                _ => changed.identity.run_id = "wrong-run".into(),
            }
            assert!(policy.admit(&changed).is_err(), "changed field {field}");
            assert!(policy.reopen(&changed).is_err(), "changed field {field}");
        }
        assert!(
            pin.validate_job_child(&policy.document.jobs_root.join("foreign"), "evidence")
                .is_err()
        );
        assert!(
            pin.validate_job_paths(
                &policy.document.repository,
                &policy
                    .document
                    .jobs_root
                    .join(request.directory_name().unwrap())
                    .join("campaign")
            )
            .is_ok()
        );
        assert!(
            pin.validate_job_paths(
                &policy.document.repository,
                &policy.document.jobs_root.join("other/campaign")
            )
            .is_err()
        );
    }
    #[test]
    fn new_subject_derives_new_claim_identities_without_replacing_policy() {
        let (temp, _doc, policy, request) = fixture();
        let original = policy.review_selection(&request).unwrap();
        let mut fresh = request.clone();
        fresh.identity.attempt_id = "new-attempt".into();
        fresh.candidate.tree = "e".repeat(40);
        fresh.candidate.commit = "f".repeat(40);
        fresh.candidate.patch_identity = "e".repeat(64);
        fresh.session_id = "new-session".into();
        let job = temp.path().join(fresh.directory_name().unwrap());
        fs::create_dir(&job).unwrap();
        fs::set_permissions(&job, fs::Permissions::from_mode(0o700)).unwrap();
        let current = policy.review_selection(&fresh).unwrap();
        for index in 0..2 {
            let a = &original["specialists"][0]["requiredClaims"][index];
            let b = &current["specialists"][0]["requiredClaims"][index];
            assert_ne!(a["claimId"], b["claimId"]);
            assert_eq!(a["profile"], b["profile"]);
            assert_eq!(a["acceptance"], b["acceptance"]);
            assert_eq!(a["coveredState"], b["coveredState"]);
        }
        assert!(matches!(
            policy.admit(&fresh).unwrap(),
            JobAdmissionEffect::Committed(_)
        ));
        assert!(policy.reopen(&fresh).is_ok());
    }

    #[test]
    fn private_acceptance_is_not_authority_and_policy_must_stay_current() {
        let (_temp, doc, policy, request) = fixture();
        let JobAdmissionEffect::Committed(pin) = policy.admit(&request).unwrap() else {
            panic!("fresh")
        };
        let (path, original) = policy.derive(&request).unwrap();
        let mut forged: serde_json::Value = serde_json::from_slice(&original).unwrap();
        forged["acceptance"]["requestedModel"] = json!("caller-model");
        fs::write(&path, serde_json::to_vec(&forged).unwrap()).unwrap();
        assert!(pin.revalidate().is_err());
        assert!(policy.reopen(&request).is_err());
        fs::write(&path, &original).unwrap();
        for kind in 0..4 {
            let mut changed = doc.clone();
            match kind {
                0 => changed.active = false,
                1 => changed.revision = "replacement".into(),
                2 => changed.expires_at_millis = 1,
                _ => changed.admitted_uid = changed.admitted_uid.wrapping_add(1),
            }
            fs::write(&policy.path, serde_json::to_vec(&changed).unwrap()).unwrap();
            assert!(pin.revalidate().is_err(), "policy mutation {kind}");
            assert!(policy.reopen(&request).is_err());
        }
    }
    #[test]
    fn interrupted_publication_remains_uncertain_until_exact_final_readback() {
        let (_temp, _doc, policy, request) = fixture();
        let (path, raw) = policy.derive(&request).unwrap();
        let pending = path.with_extension("pending");
        fs::write(&pending, &raw).unwrap();
        fs::set_permissions(&pending, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(matches!(
            policy.admit(&request).unwrap(),
            JobAdmissionEffect::OutcomeUnknown { .. }
        ));
        assert!(policy.reopen(&request).is_err());
        // Simulated crash after exclusive publication, before returning outcome.
        fs::hard_link(&pending, &path).unwrap();
        let pin = policy.reopen(&request).unwrap();
        assert!(pin.job_admission().is_some());
        assert!(matches!(
            policy.admit(&request).unwrap(),
            JobAdmissionEffect::Replayed(_)
        ));
    }
    #[test]
    fn request_cannot_select_authority_or_escape_policy_scope() {
        let (_temp, _doc, policy, request) = fixture();
        for key in [
            "approved",
            "policyPath",
            "requestedModel",
            "executionTemplate",
            "jobId",
        ] {
            let mut forged = serde_json::to_value(&request).unwrap();
            forged[key] = json!(true);
            assert!(serde_json::from_value::<JobReviewRequest>(forged).is_err());
        }
        let mut other = request;
        other.identity.plan_version = "wrong-plan".into();
        assert!(policy.admit(&other).is_err());
    }
}
