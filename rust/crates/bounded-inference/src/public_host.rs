//! A bounded, operator driven SIWC host. Grants are attributable inputs, not observations.
use crate::journal::{CONTENT_TYPE_VALUE_LIMIT, ContentTypeCapture, ContentTypeMeta};
use crate::{
    AttemptJournal, AttemptRecord, AttemptState, AuthStore, Bounds, HttpErrorShape, HttpEvidence,
    ObservedTerminal, PublicHttps, PublicHttpsError, Readiness, RegistrationRef, StreamError,
    StreamObserver, ToolFreeProfile, assemble_request,
};
use crate::{AssembledRequest, DirectEndpoint, DirectExchange};
use lifecycle_semantic::{
    DirectEvidenceClass, DirectPreparedCall, DirectProductionProfile, DirectReleaseState,
    DirectRole, RealDiagnosticPrepared,
};
use reqwest::header::{CONTENT_TYPE, HeaderValue};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    net::{IpAddr, TcpListener},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SMOKE_INPUT: &str = r#"Return exactly "B2_TRANSPORT_OK." including the final period. Do not include quotation marks or any other text."#;
const USAGE_LINK: &str = "https://chatgpt.com/settings/usage";
const MODEL_OBSERVATION_LIMIT: usize = 256 * 1024;

#[derive(Debug, Error)]
pub enum HostError {
    #[error("host policy or grant binding failed")]
    Policy,
    #[error("host I/O failed")]
    Io,
    #[error("authentication failed")]
    Auth,
    #[error(
        "refresh client configuration rejected; repair configuration before another admitted attempt"
    )]
    RefreshConfiguration,
    #[error("model action deadline exceeded")]
    ModelDeadline,
    #[error("model observation metadata exceeded bound")]
    ModelObservationBound,
    #[error("public HTTPS failed: {0}")]
    Https(#[from] PublicHttpsError),
    #[error("one-shot evidence failed; possible remote entry remains unresolved")]
    Evidence,
    #[error("transport failed after durable intent; remote outcome is unknown")]
    Entered,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CredentialCustody {
    owner: String,
    custodian: String,
    contract_ref: String,
    retention_purpose: String,
}

/// Human-supplied action authorization. Its fields are attributable assertions;
/// they do not independently prove deployment, consent, funding or account state.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionGrant {
    pub release_id: String,
    pub actor: String,
    pub custody_owner: String,
    pub custody_custodian: String,
    pub custody_contract_ref: String,
    pub retention_purpose: String,
    pub cleanup_disposition: String,
    pub action: String,
    pub credential_dir: PathBuf,
    pub host_id: String,
    pub work_dir: Option<PathBuf>,
    pub smoke_grant_sha256: Option<String>,
    #[serde(default)]
    pub refresh_registration: Option<RegistrationRef>,
    pub executable_sha256: String,
    pub source_sha256: String,
    pub issued_at_unix: u64,
    pub valid_until_unix: u64,
}

impl ActionGrant {
    pub fn source_sha256() -> String {
        let mut digest = Sha256::new();
        digest.update(include_bytes!("public_host.rs"));
        digest.update(include_bytes!("public_https.rs"));
        digest.update(include_bytes!("auth.rs"));
        digest.update(include_bytes!("request.rs"));
        digest.update(include_bytes!("stream.rs"));
        digest.update(include_bytes!("journal.rs"));
        digest.update(include_bytes!(
            "../../lifecycle-semantic/src/real_diagnostic.rs"
        ));
        digest.update(include_bytes!(
            "../../lifecycle-semantic/src/invocation_v2.rs"
        ));
        digest.update(include_bytes!("../../lifecycle-semantic/src/assessment.rs"));
        digest.update(include_bytes!(
            "../../lifecycle-semantic/src/continuation.rs"
        ));
        digest.update(include_bytes!("../../lifecycle-semantic/src/material.rs"));
        digest.update(include_bytes!("../../lifecycle-semantic/src/policy.rs"));
        digest.update(include_bytes!("bin/b2-public-host.rs"));
        digest.update(include_bytes!("direct.rs"));
        digest.update(include_bytes!(
            "../../lifecycle-semantic/src/direct_control.rs"
        ));
        digest.update(include_bytes!("../Cargo.toml"));
        format!("{:x}", digest.finalize())
    }

    pub fn executable_sha256() -> Result<String, HostError> {
        let exe = std::env::current_exe().map_err(|_| HostError::Policy)?;
        let bytes = fs::read(exe).map_err(|_| HostError::Policy)?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    pub fn validate(
        &self,
        action: &str,
        credential_dir: &Path,
        host_id: &str,
        work_dir: Option<&Path>,
        smoke_grant_bytes: Option<&[u8]>,
    ) -> Result<(), HostError> {
        let now = now();
        let max_secs = match action {
            "sign-in" => 630,
            "models" => 15,
            "refresh" => 20,
            "disconnect" => 15,
            "smoke" | "diagnostic" | "direct" => 45,
            _ => return Err(HostError::Policy),
        };
        if self.action != action
            || self.credential_dir != credential_dir
            || self.host_id != host_id
            || self.work_dir.as_deref() != work_dir
            || self.release_id.is_empty()
            || safe_id(&self.release_id).is_err()
            || self.actor.is_empty()
            || self.actor.len() > 256
            || self.actor.chars().any(char::is_control)
            || [
                &self.custody_owner,
                &self.custody_custodian,
                &self.custody_contract_ref,
                &self.retention_purpose,
            ]
            .iter()
            .any(|v| v.is_empty() || v.len() > 256 || v.chars().any(char::is_control))
            || self.cleanup_disposition
                != if action == "disconnect" {
                    "local-delete-and-remote-revocation-attempt"
                } else {
                    "retain-until-owner-disconnect"
                }
            || self.issued_at_unix > now
            || self.valid_until_unix <= now
            || now.saturating_sub(self.issued_at_unix) > 300
            || self.valid_until_unix > self.issued_at_unix.saturating_add(max_secs)
            || (action == "refresh") != self.refresh_registration.is_some()
            || self.source_sha256 != Self::source_sha256()
            || self.executable_sha256 != Self::executable_sha256()?
            || !valid_absolute_path(credential_dir)
            || work_dir.is_some_and(|p| !valid_absolute_path(p))
        {
            return Err(HostError::Policy);
        }
        match (smoke_grant_bytes, &self.smoke_grant_sha256) {
            (None, None) if !matches!(action, "smoke" | "diagnostic" | "direct") => (),
            (Some(bytes), Some(digest))
                if matches!(action, "smoke" | "diagnostic" | "direct")
                    && *digest == format!("{:x}", Sha256::digest(bytes)) =>
            {
                ()
            }
            _ => return Err(HostError::Policy),
        }
        Ok(())
    }

    /// Read-only custody admission for actions that must not initialize a store.
    fn verify_existing_custody(&self) -> Result<(), HostError> {
        let custody = CredentialCustody {
            owner: self.custody_owner.clone(),
            custodian: self.custody_custodian.clone(),
            contract_ref: self.custody_contract_ref.clone(),
            retention_purpose: self.retention_purpose.clone(),
        };
        let custody_path = self.credential_dir.join("credential-custody-contract.json");
        match fs::read(&custody_path) {
            Ok(bytes) => {
                let prior: CredentialCustody =
                    serde_json::from_slice(&bytes).map_err(|_| HostError::Policy)?;
                if prior != custody {
                    return Err(HostError::Policy);
                }
            }
            Err(_) => return Err(HostError::Policy),
        }
        Ok(())
    }

    fn action_path(&self) -> Result<PathBuf, HostError> {
        Ok(self
            .credential_dir
            .join(format!("action-{}.json", safe_id(&self.release_id)?)))
    }

    /// Read-only replay rejection before an ordinary store open can recover files.
    /// The create_new claim in consume remains authoritative for races.
    fn verify_unspent(&self) -> Result<(), HostError> {
        match fs::symlink_metadata(self.action_path()?) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            _ => Err(HostError::Policy),
        }
    }

    /// A durable, single-use action claim in the credential custody directory.
    fn consume(&self) -> Result<(), HostError> {
        if self.action == "sign-in" {
            let custody = CredentialCustody {
                owner: self.custody_owner.clone(),
                custodian: self.custody_custodian.clone(),
                contract_ref: self.custody_contract_ref.clone(),
                retention_purpose: self.retention_purpose.clone(),
            };
            let custody_path = self.credential_dir.join("credential-custody-contract.json");
            match fs::read(&custody_path) {
                Ok(bytes) => {
                    let prior: CredentialCustody =
                        serde_json::from_slice(&bytes).map_err(|_| HostError::Policy)?;
                    if prior != custody {
                        return Err(HostError::Policy);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    persist_new(
                        &custody_path,
                        &serde_json::to_vec(&custody).map_err(|_| HostError::Policy)?,
                    )?;
                }
                _ => return Err(HostError::Policy),
            }
        } else {
            self.verify_existing_custody()?;
        }
        self.claim()
    }

    /// The durable single-use claim itself. Every check `consume` makes comes
    /// before it, so an error before this point consumed nothing.
    fn claim(&self) -> Result<(), HostError> {
        let bytes = serde_json::to_vec(self).map_err(|_| HostError::Policy)?;
        persist_new(&self.action_path()?, &bytes)
    }

    pub fn remaining(&self) -> Result<Duration, HostError> {
        (UNIX_EPOCH + Duration::from_secs(self.valid_until_unix))
            .duration_since(SystemTime::now())
            .ok()
            .filter(|d| !d.is_zero())
            .ok_or(HostError::Policy)
    }
}

fn valid_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && !path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
}

/// The owner's signed-off facts are asserted by a named actor. The host checks
/// internal consistency and freshness; it cannot see account funding controls.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SmokeGrant {
    pub release_id: String,
    pub operation_id: String,
    pub attempt_id: String,
    pub client_id: String,
    pub subject: String,
    pub credential_revision: u64,
    pub selected_model: String,
    pub model_observation_sha256: String,
    pub funding_assertion_ref: String,
    pub funding_actor: String,
    pub funding_custodian: String,
    pub funding_setting_scope: String,
    pub funding_observed_at_unix: u64,
    pub credit_use_disabled: bool,
    pub included_plan_enabled: bool,
    /// Freshness of the asserted saved-setting observation, not funding closure.
    pub funding_observation_valid_until_unix: u64,
    pub account_owner_accepts_open_custody: bool,
    pub accepted_closure_basis: String,
    pub release_actor: String,
}

impl SmokeGrant {
    fn validate(&self, selected: &RegistrationRef, model: &str, now: u64) -> Result<(), HostError> {
        let ids = [
            &self.release_id,
            &self.operation_id,
            &self.attempt_id,
            &self.funding_assertion_ref,
            &self.funding_actor,
            &self.funding_custodian,
            &self.funding_setting_scope,
            &self.accepted_closure_basis,
            &self.release_actor,
        ];
        if ids
            .iter()
            .any(|v| v.is_empty() || v.len() > 256 || v.chars().any(char::is_control))
            || !self.credit_use_disabled
            || !self.included_plan_enabled
            || !self.account_owner_accepts_open_custody
            || self.funding_observation_valid_until_unix <= now
            || self.funding_observation_valid_until_unix > now.saturating_add(3600)
            || self.funding_observed_at_unix > now
            || now.saturating_sub(self.funding_observed_at_unix) > 300
            || self.client_id != selected.client_id
            || self.subject != selected.subject
            || self.credential_revision != selected.credential_revision
            || self.selected_model != model
            || self.model_observation_sha256.len() != 64
            || !self
                .model_observation_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err(HostError::Policy);
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct FundingObligation<'a> {
    release_id: &'a str,
    attempt_id: &'a str,
    assertion_ref: &'a str,
    actor: &'a str,
    custodian: &'a str,
    setting_scope: &'a str,
    observed_at_unix: u64,
    accepted_closure_basis: &'a str,
    status: &'static str,
    next_review: &'static str,
}

#[derive(Debug, Serialize)]
pub struct SmokeProjection {
    pub release_id: String,
    pub attempt_id: String,
    pub state: AttemptState,
    pub terminal: Option<ObservedTerminal>,
    pub output_matches_fixture: Option<bool>,
    pub response_sha256: String,
    pub response_bytes: usize,
    pub received_bytes: usize,
    pub full_capture_complete: bool,
    pub http_status: Option<u16>,
    pub content_type_meta: Option<ContentTypeMeta>,
    pub failure_category: Option<String>,
    pub exact_capture: String,
    pub quarantined: bool,
    pub funding_obligation_open: bool,
    pub local_transport_closed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticGrant {
    pub smoke: SmokeGrant,
    pub prepared_binding_sha256: String,
    pub prepared_wire_sha256: String,
    pub semantic_prompt_sha256: String,
    pub source_basis_sha256: String,
    pub candidate_sha256: String,
    pub exposure_id: String,
    pub profile_revision: String,
}

/// Non-secret admission evidence retained before the diagnostic send intent.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticAdmission {
    pub action: ActionGrant,
    pub grant_bytes: Vec<u8>,
    pub registration: RegistrationRef,
    pub action_claim_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DiagnosticProjection {
    pub release_id: String,
    pub operation_id: String,
    pub attempt_id: String,
    pub state: AttemptState,
    pub terminal: Option<ObservedTerminal>,
    pub prepared_binding_sha256: String,
    pub prepared_wire_sha256: String,
    pub admission_sha256: String,
    pub response_sha256: String,
    pub response_bytes: usize,
    pub received_bytes: usize,
    pub full_capture_complete: bool,
    pub http_status: Option<u16>,
    pub content_type_meta: Option<ContentTypeMeta>,
    pub failure_category: Option<String>,
    pub exact_capture: String,
    pub quarantined: bool,
    pub funding_obligation_open: bool,
    pub local_transport_closed: bool,
}

/// Owner release for one public Direct send of an exact prepared call.
///
/// `smoke` carries the existing registration, model-catalog observation,
/// funding-setting assertion and custody acceptance; its operation and attempt
/// are the lifecycle binding's. The remaining fields bind the prepared call the
/// owner released. Like every grant here these are attributable assertions,
/// checked for consistency and freshness, not observations of account state.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectSendGrant {
    pub smoke: SmokeGrant,
    pub role: DirectRole,
    pub prepared_binding_sha256: String,
    pub prepared_wire_sha256: String,
    pub source_basis_sha256: String,
    pub candidate_sha256: Option<String>,
    pub compile_binding_sha256: Option<String>,
    pub exposure_id: String,
    pub profile_revision: String,
    /// The startup-verified source-manifest digest of the composing host. The
    /// action grant binds the actual executable and this library's source;
    /// this binds the full host source the owner released the call to.
    pub host_source_sha256: String,
}

/// What a request for one exact payload carries into the shared send path.
#[derive(Clone, Copy)]
enum SendPayload<'a> {
    Smoke,
    Diagnostic(&'a RealDiagnosticPrepared),
    Direct {
        call: &'a DirectPreparedCall,
        journal_path: &'a Path,
    },
}

/// File-name key for a lifecycle operation or attempt id in the shared
/// custody claim namespace. Lifecycle ids may contain `:`; the digest keeps
/// them distinct from each other and from B2's plain identifiers.
pub fn direct_identity_key(id: &str) -> String {
    format!("direct-{:x}", Sha256::digest(id.as_bytes()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelObservation {
    #[serde(skip)]
    pub observation_id: String,
    pub event_nonce: String,
    pub response_sha256: String,
    pub response_bytes: usize,
    pub observed_at_unix: u64,
    pub registration: RegistrationRef,
    pub models: Vec<crate::DiscoveredModel>,
}

/// Local readiness only. Registration identifiers remain inside AuthStore.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalReadiness {
    SignedOut,
    Ready,
    Expired,
    RefreshAmbiguous,
    ReauthorizationRequired,
}

impl From<Readiness> for LocalReadiness {
    fn from(readiness: Readiness) -> Self {
        match readiness {
            Readiness::SignedOut => Self::SignedOut,
            Readiness::Ready(_) => Self::Ready,
            Readiness::Expired(_) => Self::Expired,
            Readiness::RefreshAmbiguous(_) => Self::RefreshAmbiguous,
            Readiness::ReauthorizationRequired(_) => Self::ReauthorizationRequired,
        }
    }
}

pub struct PublicHost {
    https: PublicHttps,
    store: AuthStore,
    credential_dir: PathBuf,
    host_id: String,
}

impl PublicHost {
    pub async fn open(credential_dir: &Path, host_id: &str) -> Result<Self, HostError> {
        let https = PublicHttps::new()?;
        let store = AuthStore::open(
            credential_dir,
            host_id,
            crate::public_https::local_issuer()?,
        )
        .map_err(|_| HostError::Auth)?;
        Ok(Self {
            https,
            store,
            credential_dir: credential_dir.to_path_buf(),
            host_id: host_id.into(),
        })
    }

    #[cfg(test)]
    fn synthetic(
        https: PublicHttps,
        credential_dir: &Path,
        host_id: &str,
    ) -> Result<Self, HostError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| HostError::Io)?;
        let _trusted = runtime.block_on(https.discover())?;
        Self::synthetic_local(https, credential_dir, host_id)
    }

    #[cfg(test)]
    fn synthetic_local(
        https: PublicHttps,
        credential_dir: &Path,
        host_id: &str,
    ) -> Result<Self, HostError> {
        let store = AuthStore::open(
            credential_dir,
            host_id,
            crate::public_https::local_issuer()?,
        )
        .map_err(|_| HostError::Auth)?;
        Ok(Self {
            https,
            store,
            credential_dir: credential_dir.to_path_buf(),
            host_id: host_id.into(),
        })
    }

    pub async fn sign_in_admitted<F>(
        &self,
        grant: &ActionGrant,
        app_name: &str,
        show_url: F,
    ) -> Result<RegistrationRef, HostError>
    where
        F: FnMut(&str) -> Result<(), HostError>,
    {
        grant.validate("sign-in", &self.credential_dir, &self.host_id, None, None)?;
        grant.consume()?;
        tokio::time::timeout(
            grant.remaining()?,
            self.sign_in_new_inner(app_name, show_url, Some(grant)),
        )
        .await
        .map_err(|_| HostError::Auth)?
    }

    /// A consumed action allows one refresh for exactly the selected revision.
    /// Discovery precedes the store's in-flight marker; the token POST is made
    /// only inside AuthStore's exclusive, durable refresh transaction.
    pub async fn refresh_admitted(
        &self,
        action: &ActionGrant,
    ) -> Result<RegistrationRef, HostError> {
        let expected = Self::validate_refresh_preopen(action, &self.credential_dir, &self.host_id)?;
        action.consume()?;
        tokio::time::timeout(action.remaining()?, self.refresh_inner(expected, action))
            .await
            .map_err(|_| HostError::Entered)?
    }

    async fn refresh_inner(
        &self,
        expected: RegistrationRef,
        action: &ActionGrant,
    ) -> Result<RegistrationRef, HostError> {
        action.remaining()?;
        let trusted = self.https.discover().await?;
        action.remaining()?;
        let https = self.https.clone();
        let root = self.credential_dir.clone();
        let host_id = self.host_id.clone();
        let valid_until = action.valid_until_unix;
        tokio::task::spawn_blocking(move || {
            let store = AuthStore::open(
                &root,
                &host_id,
                crate::public_https::local_issuer().map_err(|_| HostError::Auth)?,
            )
            .map_err(|_| HostError::Auth)?;
            if now() >= valid_until {
                return Err(HostError::Policy);
            }
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| HostError::Auth)?;
            store
                .refresh_with(&expected, now(), &trusted.jwks, |grant| {
                    if now() >= valid_until {
                        return Err(crate::RefreshFailure::Temporary);
                    }
                    runtime.block_on(https.refresh(grant))
                })
                .map_err(|error| match error {
                    crate::AuthError::RefreshConfiguration => HostError::RefreshConfiguration,
                    _ => HostError::Auth,
                })
        })
        .await
        .map_err(|_| HostError::Entered)?
    }

    pub async fn models_admitted(
        &self,
        grant: &ActionGrant,
    ) -> Result<ModelObservation, HostError> {
        grant.validate("models", &self.credential_dir, &self.host_id, None, None)?;
        grant.consume()?;
        tokio::time::timeout(grant.remaining()?, self.list_models_inner(Some(grant)))
            .await
            .map_err(|_| HostError::ModelDeadline)?
    }

    pub async fn disconnect_admitted(&self, grant: &ActionGrant) -> Result<bool, HostError> {
        grant.validate(
            "disconnect",
            &self.credential_dir,
            &self.host_id,
            None,
            None,
        )?;
        grant.consume()?;
        tokio::time::timeout(grant.remaining()?, self.disconnect_inner(Some(grant)))
            .await
            .map_err(|_| HostError::Policy)?
    }

    pub async fn smoke_admitted(
        &self,
        action: &ActionGrant,
        grant: &SmokeGrant,
        grant_bytes: &[u8],
        work_dir: &Path,
    ) -> Result<SmokeProjection, HostError> {
        action.validate(
            "smoke",
            &self.credential_dir,
            &self.host_id,
            Some(work_dir),
            Some(grant_bytes),
        )?;
        let bound_grant: SmokeGrant =
            serde_json::from_slice(grant_bytes).map_err(|_| HostError::Policy)?;
        if serde_json::to_value(&bound_grant).map_err(|_| HostError::Policy)?
            != serde_json::to_value(grant).map_err(|_| HostError::Policy)?
        {
            return Err(HostError::Policy);
        }
        action.consume()?;
        tokio::time::timeout(
            action.remaining()?,
            self.smoke_inner(&bound_grant, work_dir, Some(action), SendPayload::Smoke, None),
        )
        .await
        .map_err(|_| HostError::Entered)?
    }

    /// Each admission consumes one action and one operation/attempt identity.
    /// The prepared bytes are reconstructed and checked before any send intent.
    pub async fn diagnostic_admitted(
        &self,
        action: &ActionGrant,
        grant_bytes: &[u8],
        prepared: &RealDiagnosticPrepared,
        work_dir: &Path,
    ) -> Result<DiagnosticProjection, HostError> {
        action.validate(
            "diagnostic",
            &self.credential_dir,
            &self.host_id,
            Some(work_dir),
            Some(grant_bytes),
        )?;
        action.verify_unspent()?;
        let grant: DiagnosticGrant =
            serde_json::from_slice(grant_bytes).map_err(|_| HostError::Policy)?;
        prepared.validate().map_err(|_| HostError::Policy)?;
        let binding = &prepared.request.binding;
        let profile = ToolFreeProfile {
            revision: StreamObserver::SIWC_PROFILE_REVISION.into(),
            model: grant.smoke.selected_model.clone(),
            bounds: Bounds {
                request_bytes: 65_536,
                stream_bytes: 1_048_576,
                frame_bytes: 131_072,
                output_bytes: 16_384,
                events: 4_096,
                local_deadline_ms: 30_000,
            },
        };
        let assembled = assemble_request(
            &prepared.instructions,
            &prepared.request.semantic_prompt,
            &profile,
        )
        .map_err(|_| HostError::Policy)?;
        if prepared.wire_bytes != assembled.bytes
            || prepared.wire_sha256 != assembled.sha256
            || prepared.profile_revision != assembled.profile_revision
            || prepared.requested_provider != "openai-siwc"
            || prepared.requested_model != assembled.selected_model
            || grant.smoke.operation_id != binding.operation_id
            || grant.smoke.attempt_id != binding.attempt_id
            || grant.prepared_binding_sha256 != prepared.binding_sha256
            || grant.prepared_wire_sha256 != prepared.wire_sha256
            || grant.semantic_prompt_sha256 != prepared.request.semantic_prompt_sha256
            || grant.source_basis_sha256 != prepared.request.source_basis_sha256
            || grant.candidate_sha256 != prepared.request.candidate_sha256
            || grant.exposure_id != binding.exposure_id
            || grant.profile_revision != prepared.profile_revision
        {
            return Err(HostError::Policy);
        }
        action.consume()?;
        prepare_private_dir(work_dir)?;
        let action_claim = fs::read(action.action_path()?).map_err(|_| HostError::Evidence)?;
        let admission = DiagnosticAdmission {
            action: action.clone(),
            grant_bytes: grant_bytes.to_vec(),
            registration: self.selected()?,
            action_claim_sha256: format!("{:x}", Sha256::digest(&action_claim)),
        };
        let admission_bytes = serde_json::to_vec(&admission).map_err(|_| HostError::Evidence)?;
        persist_new(
            &work_dir.join(format!("diagnostic-admission-{}.json", binding.attempt_id)),
            &admission_bytes,
        )?;
        let admission_sha256 = format!("{:x}", Sha256::digest(&admission_bytes));
        let smoke = tokio::time::timeout(
            action.remaining()?,
            self.smoke_inner(
                &grant.smoke,
                work_dir,
                Some(action),
                SendPayload::Diagnostic(prepared),
                Some(&admission_sha256),
            ),
        )
        .await
        .map_err(|_| HostError::Entered)??;
        Ok(prepared_projection(
            smoke,
            grant.smoke.operation_id,
            &prepared.binding_sha256,
            &prepared.wire_sha256,
            admission_sha256,
        ))
    }

    /// One owner-admitted public Direct send for an exact prepared call.
    ///
    /// Admission reuses the diagnostic owners: the action grant (source,
    /// executable, credential, host and work-dir binding, 45-second validity,
    /// durable single-use claim) and the smoke grant (selected registration,
    /// fresh catalog observation listing the exact model, fresh funding-setting
    /// assertion, open-custody acceptance). The release, operation and attempt
    /// claims are created in the credential custody directory before intent,
    /// so no other work directory, process or restart can resend them. The
    /// funding obligation is durable before any possible send and only its
    /// owner can close it. The response is parsed with the qualified
    /// `b2-public-siwc-stream-v2` profile and the raw body is retained.
    pub async fn direct_admitted(
        &self,
        action: &ActionGrant,
        grant_bytes: &[u8],
        call: &DirectPreparedCall,
        work_dir: &Path,
        journal_path: &Path,
        host_source_sha256: &str,
    ) -> Result<DiagnosticProjection, HostError> {
        self.direct_admitted_staged(action, grant_bytes, call, work_dir, journal_path, host_source_sha256)
            .await
            .map_err(DirectAdmitError::into_host)
    }

    /// `direct_admitted` with its failures split at the durable action claim:
    /// `Withheld` failures happened before it, so no release, custody or action
    /// identity was consumed and nothing was sent.
    pub(crate) async fn direct_admitted_staged(
        &self,
        action: &ActionGrant,
        grant_bytes: &[u8],
        call: &DirectPreparedCall,
        work_dir: &Path,
        journal_path: &Path,
        host_source_sha256: &str,
    ) -> Result<DiagnosticProjection, DirectAdmitError> {
        let grant = (|| -> Result<(DirectSendGrant, RegistrationRef), HostError> {
            action.validate(
                "direct",
                &self.credential_dir,
                &self.host_id,
                Some(work_dir),
                Some(grant_bytes),
            )?;
            action.verify_unspent()?;
            let grant: DirectSendGrant =
                serde_json::from_slice(grant_bytes).map_err(|_| HostError::Policy)?;
            call.validate().map_err(|_| HostError::Policy)?;
            let selected = self.selected()?;
            grant
                .smoke
                .validate(&selected, &call.wire.requested_model, now())?;
            if journal_path.parent() != Some(work_dir)
                || !direct_grant_binds(&grant, call, host_source_sha256)
            {
                return Err(HostError::Policy);
            }
            action.verify_existing_custody()?;
            action.action_path()?;
            // The send boundary's time-bound currentness, checked once more
            // before the claim, so a lapse while queued after `Ready` is
            // withheld rather than consumed. Only a lapse between the claim
            // and the send boundary remains a consumed, fenced `NotSent`.
            let at = now();
            current_model_observation(&self.credential_dir, &grant.smoke, &selected, at)?;
            if action.remaining()? < Duration::from_secs(30)
                || grant.smoke.funding_observation_valid_until_unix < at.saturating_add(30)
            {
                return Err(HostError::Policy);
            }
            match AuthStore::inspect_readiness(
                &self.credential_dir,
                &self.host_id,
                crate::public_https::local_issuer().map_err(|_| HostError::Policy)?,
                at.saturating_add(30),
            ) {
                Ok(Readiness::Ready(later)) if later == selected => {}
                _ => return Err(HostError::Policy),
            }
            Ok((grant, selected))
        })()
        .map_err(DirectAdmitError::Withheld)?;
        self.direct_claimed(action, grant_bytes, grant, call, work_dir, journal_path)
            .await
            .map_err(DirectAdmitError::Claimed)
    }

    async fn direct_claimed(
        &self,
        action: &ActionGrant,
        grant_bytes: &[u8],
        (grant, selected): (DirectSendGrant, RegistrationRef),
        call: &DirectPreparedCall,
        work_dir: &Path,
        journal_path: &Path,
    ) -> Result<DiagnosticProjection, HostError> {
        let binding = &call.binding;
        action.claim()?;
        prepare_private_dir(work_dir)?;
        let action_claim = fs::read(action.action_path()?).map_err(|_| HostError::Evidence)?;
        let admission = DiagnosticAdmission {
            action: action.clone(),
            grant_bytes: grant_bytes.to_vec(),
            registration: selected,
            action_claim_sha256: format!("{:x}", Sha256::digest(&action_claim)),
        };
        let admission_bytes = serde_json::to_vec(&admission).map_err(|_| HostError::Evidence)?;
        persist_new(
            &work_dir.join(format!(
                "direct-admission-{}.json",
                direct_identity_key(&binding.attempt_id)
            )),
            &admission_bytes,
        )?;
        let admission_sha256 = format!("{:x}", Sha256::digest(&admission_bytes));
        let smoke = tokio::time::timeout(
            action.remaining()?,
            self.smoke_inner(
                &grant.smoke,
                work_dir,
                Some(action),
                SendPayload::Direct { call, journal_path },
                Some(&admission_sha256),
            ),
        )
        .await
        .map_err(|_| HostError::Entered)??;
        Ok(prepared_projection(
            smoke,
            grant.smoke.operation_id,
            &call.binding_sha256,
            &call.wire_sha256,
            admission_sha256,
        ))
    }

    /// Bind first, then authorize. This command returns only after a real loopback
    /// callback or timeout; the browser URL is shown by the operator UI alone.
    #[cfg(test)]
    async fn sign_in_new<F>(
        &self,
        app_name: &str,
        show_url: F,
    ) -> Result<RegistrationRef, HostError>
    where
        F: FnMut(&str) -> Result<(), HostError>,
    {
        self.sign_in_new_inner(app_name, show_url, None).await
    }
    async fn sign_in_new_inner<F>(
        &self,
        app_name: &str,
        mut show_url: F,
        admitted: Option<&ActionGrant>,
    ) -> Result<RegistrationRef, HostError>
    where
        F: FnMut(&str) -> Result<(), HostError>,
    {
        let action = async {
            if let Some(grant) = admitted {
                grant.remaining()?;
            }
            let trusted = self.https.discover().await?;
            let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| HostError::Io)?;
            listener.set_nonblocking(true).map_err(|_| HostError::Io)?;
            let redirect = format!(
                "http://127.0.0.1:{}/auth/callback",
                listener.local_addr().map_err(|_| HostError::Io)?.port()
            );
            let start = self
                .store
                .begin_registration(app_name, &redirect, now())
                .map_err(|_| HostError::Auth)?;
            show_url(start.authorize_url())?;
            let callback =
                receive_callback_async(listener, &redirect, Duration::from_secs(600)).await?;
            let exchange = self
                .store
                .accept_callback(&callback, now())
                .map_err(|_| HostError::Auth)?;
            if let Some(grant) = admitted {
                grant.remaining()?;
            }
            let token = self.https.exchange(&exchange).await?;
            self.store
                .finish_exchange(&exchange, token, &trusted.jwks, now())
                .map_err(|_| HostError::Auth)
        };
        tokio::time::timeout(Duration::from_secs(630), action)
            .await
            .map_err(|_| HostError::Auth)?
    }

    pub fn usage_url(&self) -> &'static str {
        USAGE_LINK
    }
    pub fn inspect_local_readiness(
        credential_dir: &Path,
        host_id: &str,
    ) -> Result<LocalReadiness, crate::AuthError> {
        let issuer =
            crate::public_https::local_issuer().map_err(|_| crate::AuthError::InvalidInput)?;
        AuthStore::inspect_readiness(credential_dir, host_id, issuer, now())
            .map(LocalReadiness::from)
    }

    /// Read-only target evidence for constructing an exact refresh admission.
    /// This inspects the existing store without initializing or changing it.
    pub fn inspect_local_renewal_target(
        credential_dir: &Path,
        host_id: &str,
    ) -> Result<RegistrationRef, crate::AuthError> {
        let issuer =
            crate::public_https::local_issuer().map_err(|_| crate::AuthError::InvalidInput)?;
        AuthStore::inspect_renewal_target(credential_dir, host_id, issuer)
    }

    /// The CLI calls this before its ordinary, mutating AuthStore open.
    pub fn validate_refresh_preopen(
        action: &ActionGrant,
        credential_dir: &Path,
        host_id: &str,
    ) -> Result<RegistrationRef, HostError> {
        action.validate("refresh", credential_dir, host_id, None, None)?;
        action.verify_existing_custody()?;
        action.verify_unspent()?;
        let expected = action
            .refresh_registration
            .as_ref()
            .ok_or(HostError::Policy)?;
        let actual = Self::inspect_local_renewal_target(credential_dir, host_id)
            .map_err(|_| HostError::Auth)?;
        if actual != *expected {
            return Err(HostError::Auth);
        }
        Ok(actual)
    }

    pub fn selected(&self) -> Result<RegistrationRef, HostError> {
        match self.store.readiness(now()).map_err(|_| HostError::Auth)? {
            Readiness::Ready(reference) => Ok(reference),
            _ => Err(HostError::Auth),
        }
    }

    #[cfg(test)]
    async fn list_models(&self) -> Result<ModelObservation, HostError> {
        self.list_models_inner(None).await
    }
    async fn list_models_inner(
        &self,
        admitted: Option<&ActionGrant>,
    ) -> Result<ModelObservation, HostError> {
        let selected = self.selected()?;
        let bearer = self
            .store
            .bearer(&selected, now())
            .map_err(|_| HostError::Auth)?;
        if bearer.registration != selected {
            return Err(HostError::Policy);
        }
        if let Some(grant) = admitted {
            grant.remaining()?;
        }
        let response = self.https.models(&bearer).await?;
        let mut observation = ModelObservation {
            observation_id: String::new(),
            event_nonce: String::new(),
            response_sha256: response.response_sha256,
            response_bytes: response.response_bytes,
            observed_at_unix: now(),
            registration: selected,
            models: response.models,
        };
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| HostError::Evidence)?;
        observation.event_nonce = nonce.iter().map(|b| format!("{b:02x}")).collect();
        let observation_bytes =
            serde_json::to_vec(&observation).map_err(|_| HostError::Evidence)?;
        if observation_bytes.len() > MODEL_OBSERVATION_LIMIT {
            return Err(HostError::ModelObservationBound);
        }
        let raw_path = self.credential_dir.join(format!(
            "model-response-{}.bin",
            observation.response_sha256
        ));
        match fs::read(&raw_path) {
            Ok(prior)
                if prior == response.raw_body
                    && format!("{:x}", Sha256::digest(&prior)) == observation.response_sha256 =>
            {
                ()
            }
            Ok(_) => return Err(HostError::Evidence),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                persist_new(&raw_path, &response.raw_body)?
            }
            Err(_) => return Err(HostError::Evidence),
        }
        let observation_id = format!("{:x}", Sha256::digest(&observation_bytes));
        let path = self.observation_path(&observation_id)?;
        persist_new(&path, &observation_bytes)?;
        Ok(ModelObservation {
            observation_id,
            ..observation
        })
    }

    #[cfg(test)]
    async fn disconnect(&self) -> Result<bool, HostError> {
        self.disconnect_inner(None).await
    }
    async fn disconnect_inner(&self, admitted: Option<&ActionGrant>) -> Result<bool, HostError> {
        let selected = match self.store.readiness(now()).map_err(|_| HostError::Auth)? {
            Readiness::SignedOut => return Err(HostError::Auth),
            Readiness::Ready(reference)
            | Readiness::Expired(reference)
            | Readiness::RefreshAmbiguous(reference)
            | Readiness::ReauthorizationRequired(reference) => reference,
        };
        let grant = self
            .store
            .sign_out_local(&selected)
            .map_err(|_| HostError::Auth)?;
        match grant {
            Some(grant) => {
                if admitted.is_some_and(|action| action.remaining().is_err()) {
                    return Ok(false);
                }
                Ok(self
                    .https
                    .revoke(&grant.endpoint, &grant)
                    .await
                    .unwrap_or(false))
            }
            None => Ok(false),
        }
    }

    fn observation_path(&self, digest: &str) -> Result<PathBuf, HostError> {
        observation_file(&self.credential_dir, digest)
    }

    #[cfg(test)]
    async fn smoke(
        &self,
        grant: &SmokeGrant,
        work_dir: &Path,
    ) -> Result<SmokeProjection, HostError> {
        self.smoke_inner(grant, work_dir, None, SendPayload::Smoke, None).await
    }
    async fn smoke_inner(
        &self,
        grant: &SmokeGrant,
        work_dir: &Path,
        admitted: Option<&ActionGrant>,
        payload: SendPayload<'_>,
        admission_sha256: Option<&str>,
    ) -> Result<SmokeProjection, HostError> {
        let selected = self.selected()?;
        let observed_now = now();
        grant.validate(&selected, &grant.selected_model, observed_now)?;
        let observation = current_model_observation(&self.credential_dir, grant, &selected, observed_now)?;
        let bounds = Bounds {
            request_bytes: if matches!(payload, SendPayload::Smoke) { 4096 } else { 65_536 },
            stream_bytes: 1024 * 1024,
            frame_bytes: 128 * 1024,
            output_bytes: 16 * 1024,
            events: 4096,
            local_deadline_ms: 30_000,
        };
        let profile = ToolFreeProfile {
            revision: StreamObserver::SIWC_PROFILE_REVISION.into(),
            model: grant.selected_model.clone(),
            bounds,
        };
        let request = match payload {
            SendPayload::Diagnostic(prepared) => {
                let assembled = assemble_request(
                    &prepared.instructions,
                    &prepared.request.semantic_prompt,
                    &profile,
                )
                .map_err(|_| HostError::Policy)?;
                if prepared.wire_bytes != assembled.bytes
                    || prepared.wire_sha256 != assembled.sha256
                {
                    return Err(HostError::Policy);
                }
                assembled
            }
            SendPayload::Direct { call, .. } => {
                let assembled = assemble_request(&call.wire.instructions, &call.input, &profile)
                    .map_err(|_| HostError::Policy)?;
                if call.wire.wire_bytes != assembled.bytes || call.wire_sha256 != assembled.sha256 {
                    return Err(HostError::Policy);
                }
                assembled
            }
            SendPayload::Smoke => {
                assemble_request("", SMOKE_INPUT, &profile).map_err(|_| HostError::Policy)?
            }
        };
        prepare_private_dir(work_dir)?;
        let release_id = safe_id(&grant.release_id)?;
        // Direct operation identities are lifecycle ids, which may contain
        // characters unsafe in file names; their claims use a digest key.
        let (operation_key, attempt_key) = match payload {
            SendPayload::Direct { .. } => {
                (direct_identity_key(&grant.operation_id), direct_identity_key(&grant.attempt_id))
            }
            _ => (
                safe_id(&grant.operation_id)?.to_owned(),
                safe_id(&grant.attempt_id)?.to_owned(),
            ),
        };
        let (operation_id, attempt_id) = (operation_key.as_str(), attempt_key.as_str());
        let _closure = LocalClosureGuard(work_dir.join(format!("local-closure-{attempt_id}.json")));
        // This custody directory is fixed by the selected registration. Creating
        // all three identities before intent prevents alternate work dirs and
        // fresh action grants from replaying a release, operation, or attempt.
        for (kind, id) in [
            ("release", release_id),
            ("operation", operation_id),
            ("attempt", attempt_id),
        ] {
            persist_new(
                &self.credential_dir.join(format!("smoke-{kind}-{id}.json")),
                &serde_json::to_vec(grant).map_err(|_| HostError::Policy)?,
            )?;
        }
        let obligation = work_dir.join(format!("funding-{release_id}.json"));
        let journal_path = match payload {
            SendPayload::Direct { journal_path, .. } => journal_path.to_path_buf(),
            _ => work_dir.join(format!("attempt-{attempt_id}.json")),
        };
        let open = FundingObligation {
            release_id: &grant.release_id,
            attempt_id: &grant.attempt_id,
            assertion_ref: &grant.funding_assertion_ref,
            actor: &grant.funding_actor,
            custodian: &grant.funding_custodian,
            setting_scope: &grant.funding_setting_scope,
            observed_at_unix: grant.funding_observed_at_unix,
            accepted_closure_basis: &grant.accepted_closure_basis,
            status: "open",
            next_review: "custodian must establish accepted remote/funding closure or retain unresolved obligation",
        };
        persist_new(
            &obligation,
            &serde_json::to_vec(&open).map_err(|_| HostError::Evidence)?,
        )?;
        let record = AttemptRecord::prepared_bound(
            &grant.operation_id,
            &grant.attempt_id,
            &request.sha256,
            &request.profile_revision,
            &request.selected_model,
            request.bounds.clone(),
            selected.clone(),
        )
        .map_err(|_| HostError::Evidence)?;
        let mut journal =
            AttemptJournal::create(&journal_path, record).map_err(|_| HostError::Evidence)?;
        journal
            .mark_send_intent()
            .map_err(|_| HostError::Evidence)?;
        let quarantine = work_dir.join(format!("quarantine-{attempt_id}.bin"));
        let mut capture = private_new(&quarantine)?;
        let mut capture_status = CaptureStatusGuard {
            path: work_dir.join(format!("capture-status-{attempt_id}.json")),
            received_bytes: 0,
            retained_bytes: Some(0),
            sha256: None,
            full_capture_complete: false,
        };
        // Reacquire and check at the last possible-send boundary, after durable
        // writes and model evidence. An earlier timestamp cannot fund this POST.
        #[cfg(test)]
        if grant.release_id == "test-expiring-funding" {
            tokio::time::sleep(Duration::from_millis(1200)).await;
        }
        // The selected registration is signed out after the claim and durable
        // intent, so the send boundary's bearer read fails without a time margin.
        #[cfg(test)]
        if grant.release_id == "test-postclaim-signout" {
            self.store.sign_out_local(&selected).map_err(|_| HostError::Auth)?;
        }
        let send_now = now();
        let current_bearer: Result<_, HostError> = (|| {
            grant.validate(&selected, &grant.selected_model, send_now)?;
            if observation.observed_at_unix > send_now
                || send_now.saturating_sub(observation.observed_at_unix) > 300
                || grant.funding_observation_valid_until_unix < send_now.saturating_add(30)
            {
                return Err(HostError::Policy);
            }
            let bearer = self
                .store
                .bearer(&selected, send_now)
                .map_err(|_| HostError::Auth)?;
            if bearer.registration != selected || bearer.expires_at < send_now.saturating_add(30) {
                return Err(HostError::Policy);
            }
            if let Some(action) = admitted {
                if action.remaining()? < Duration::from_secs(30) {
                    return Err(HostError::Policy);
                }
            }
            Ok(bearer)
        })();
        let bearer = match current_bearer {
            Ok(bearer) => bearer,
            Err(error) => {
                let path = work_dir.join(format!("pre-send-fence-{attempt_id}.json"));
                persist_new(&path, br#"{"post_dispatched":false,"reason":"final_currentness_failed","release_consumed":true,"funding_obligation_open":true}"#)?;
                return Err(error);
            }
        };
        let token = bearer.authorization_value();
        let raw_token = token.strip_prefix("Bearer ").ok_or(HostError::Policy)?;
        let deadline = Instant::now() + Duration::from_secs(30);
        let response_result = tokio::time::timeout(
            Duration::from_secs(30),
            self.https.send_response(&bearer, request.bytes),
        )
        .await;
        let mut response = match response_result {
            Ok(Ok(response)) => Some(response),
            _ => None,
        };
        let status = response.as_ref().map(|r| r.status().as_u16());
        let (content_type, content_type_meta) = match response.as_ref() {
            Some(r) => {
                let (value, meta) = observe_content_type(r.headers().get(CONTENT_TYPE), raw_token);
                (value, Some(meta))
            }
            None => (None, None),
        };
        // A missing media type permits bounded parsing, never success by itself.
        // Explicit non-SSE values still fail at the HTTP boundary. Both eligible
        // paths must satisfy the same strict stream/identity/terminal checks.
        let stream_eligible = status == Some(200)
            && content_type_meta
                .as_ref()
                .is_some_and(|m| m.event_stream_prefix || m.capture == ContentTypeCapture::Missing);
        let mut observer = StreamObserver::siwc(profile.bounds).map_err(|_| HostError::Policy)?;
        let mut digest = Sha256::new();
        let mut received = 0usize;
        let mut retained = 0usize;
        let mut suspect = false;
        let mut scan_tail = Vec::<u8>::new();
        let mut decode_scan = Vec::<u8>::new();
        let mut failure = if response.is_none() {
            Some(StreamError::TransportFailure)
        } else {
            None
        };
        let mut complete = false;
        while let Some(ref mut body) = response {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                failure = Some(StreamError::Timeout);
                break;
            }
            match tokio::time::timeout(remaining, body.chunk()).await {
                Ok(Ok(Some(chunk))) => {
                    received = received.saturating_add(chunk.len());
                    capture_status.received_bytes = received;
                    let limit: usize = if stream_eligible {
                        1024 * 1024
                    } else {
                        64 * 1024
                    };
                    let keep_len = chunk.len().min(limit.saturating_sub(retained));
                    let keep = &chunk[..keep_len];
                    if !keep.is_empty() {
                        // Capture first into a protected quarantine path. On any
                        // subsequent failure it stays there, never at a replay path.
                        #[cfg(test)]
                        if grant.release_id == "test-capture-write-failure" {
                            let _ = capture.write_all(&keep[..keep.len() / 2]);
                            capture_status.retained_bytes = None;
                            return Err(HostError::Evidence);
                        }
                        if capture.write_all(keep).is_err() {
                            capture_status.retained_bytes = None;
                            return Err(HostError::Evidence);
                        }
                        digest.update(keep);
                        retained += keep_len;
                        capture_status.retained_bytes = Some(retained);
                        decode_scan.extend_from_slice(keep);
                        let mut scan = scan_tail.clone();
                        scan.extend_from_slice(keep);
                        suspect |= contains_known_secret(&scan, raw_token);
                        let tail_len = raw_token.len().max(7).min(4096).saturating_sub(1);
                        scan_tail = scan[scan.len().saturating_sub(tail_len)..].to_vec();
                    }
                    if keep_len < chunk.len() {
                        failure = Some(StreamError::StreamTooLarge);
                        break;
                    }
                    if stream_eligible {
                        if let Err(e) = observer.push(keep) {
                            failure = Some(e);
                            break;
                        }
                    }
                }
                Ok(Ok(None)) => {
                    complete = true;
                    break;
                }
                Ok(Err(error)) => {
                    failure = Some(if error.is_timeout() { StreamError::Timeout } else { StreamError::TransportFailure });
                    break;
                }
                Err(_) => {
                    failure = Some(StreamError::Timeout);
                    break;
                }
            }
        }
        capture.sync_all().map_err(|_| HostError::Evidence)?;
        drop(capture);
        drop(response);
        sync_dir(work_dir)?;
        suspect |= decoded_secret_suspected(&decode_scan, raw_token);
        let body_sha = format!("{:x}", digest.finalize());
        capture_status.sha256 = Some(body_sha.clone());
        capture_status.full_capture_complete =
            complete && retained == received && failure.is_none();
        if status.is_some() {
            let error_shape = if stream_eligible {
                None
            } else {
                Some(http_error_shape(&decode_scan, failure.as_ref()))
            };
            journal
                .mark_http(HttpEvidence {
                    status,
                    request_id: None,
                    content_type,
                    content_type_meta: content_type_meta.clone(),
                    error_shape,
                    error_code: None,
                    error_param: None,
                    body_bytes: retained as u64,
                    body_sha256: Some(body_sha.clone()),
                    body_complete: complete && failure.is_none(),
                })
                .map_err(|_| HostError::Evidence)?;
        }
        let mut observation = if !stream_eligible {
            observer.failure_evidence(failure.clone().unwrap_or(StreamError::HttpFailure))
        } else if let Some(e) = failure.clone() {
            observer.failure_evidence(e)
        } else {
            observer.finish_evidence()
        };
        suspect |= serde_json::to_vec(&observation).ok().is_some_and(|bytes| {
            contains_known_secret(&bytes, raw_token) || decoded_secret_suspected(&bytes, raw_token)
        });
        if suspect {
            observation = crate::StreamObservation {
                response_id: None,
                observed_model: None,
                terminal: None,
                output: None,
                usage: Vec::new(),
                unique_events: observation.unique_events,
                local_failure: Some(StreamError::CredentialEcho),
            };
        }
        if observation.local_failure.is_none()
            && observation.observed_model.as_deref() != Some(grant.selected_model.as_str())
        {
            observation.local_failure = Some(StreamError::ConflictingIdentity);
            observation.output = None;
        }
        let terminal = safe_terminal(observation.terminal.as_ref());
        let output_matches_fixture = observation.output.as_ref().and_then(|s| {
            matches!(payload, SendPayload::Smoke).then(|| s.trim() == "B2_TRANSPORT_OK.")
        });
        let failure_category = observation.local_failure.as_ref().map(|e| format!("{e:?}"));
        let quarantined = suspect
            || !complete
            || failure.is_some()
            || !stream_eligible
            || observation.local_failure.is_some();
        if observation.local_failure.is_some() {
            journal
                .mark_unknown(observation)
                .map_err(|_| HostError::Evidence)?;
        } else {
            journal
                .mark_terminal(observation)
                .map_err(|_| HostError::Evidence)?;
        }
        let final_capture = if quarantined {
            quarantine
        } else {
            let path = work_dir.join(format!("capture-{attempt_id}.bin"));
            fs::rename(&quarantine, &path).map_err(|_| HostError::Evidence)?;
            sync_dir(work_dir)?;
            path
        };
        let projection = SmokeProjection {
            release_id: grant.release_id.clone(),
            attempt_id: grant.attempt_id.clone(),
            state: journal.record().state,
            terminal,
            output_matches_fixture,
            response_sha256: body_sha,
            response_bytes: retained,
            received_bytes: received,
            full_capture_complete: complete && retained == received && failure.is_none(),
            http_status: status,
            content_type_meta,
            failure_category,
            exact_capture: final_capture.to_string_lossy().into(),
            quarantined,
            funding_obligation_open: true,
            local_transport_closed: true,
        };
        let projection_path = work_dir.join(format!(
            "{}projection-{attempt_id}.json",
            match payload {
                SendPayload::Smoke => "",
                SendPayload::Diagnostic(_) => "diagnostic-",
                SendPayload::Direct { .. } => "direct-",
            }
        ));
        let prepared_digests = match payload {
            SendPayload::Smoke => None,
            SendPayload::Diagnostic(prepared) => {
                Some((&prepared.binding_sha256, &prepared.wire_sha256))
            }
            SendPayload::Direct { call, .. } => Some((&call.binding_sha256, &call.wire_sha256)),
        };
        let projected_bytes = if let Some((binding_sha256, wire_sha256)) = prepared_digests {
            let mut value = serde_json::to_value(&projection).map_err(|_| HostError::Evidence)?;
            let map = value.as_object_mut().ok_or(HostError::Evidence)?;
            map.remove("output_matches_fixture");
            map.insert("operation_id".into(), serde_json::json!(grant.operation_id));
            map.insert("prepared_binding_sha256".into(), serde_json::json!(binding_sha256));
            map.insert("prepared_wire_sha256".into(), serde_json::json!(wire_sha256));
            map.insert("admission_sha256".into(), serde_json::json!(admission_sha256.ok_or(HostError::Evidence)?));
            serde_json::to_vec(&value).map_err(|_| HostError::Evidence)?
        } else {
            serde_json::to_vec(&projection).map_err(|_| HostError::Evidence)?
        };
        persist_new(&projection_path, &projected_bytes)?;
        Ok(projection)
    }
}

fn observation_file(credential_dir: &Path, digest: &str) -> Result<PathBuf, HostError> {
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(HostError::Policy);
    }
    Ok(credential_dir.join(format!("model-observation-{digest}.json")))
}

/// The saved catalog observation a smoke grant names, checked against its
/// raw response, the selected registration, the exact model and its 300 s
/// freshness at `at`. Shared by the send boundary and release pre-validation.
fn current_model_observation(
    credential_dir: &Path,
    grant: &SmokeGrant,
    selected: &RegistrationRef,
    at: u64,
) -> Result<ModelObservation, HostError> {
    let observation_path = observation_file(credential_dir, &grant.model_observation_sha256)?;
    let observation_bytes = fs::read(&observation_path).map_err(|_| HostError::Policy)?;
    if observation_bytes.len() > MODEL_OBSERVATION_LIMIT {
        return Err(HostError::Policy);
    }
    let observation: ModelObservation =
        serde_json::from_slice(&observation_bytes).map_err(|_| HostError::Policy)?;
    if observation.registration != *selected
        || format!("{:x}", Sha256::digest(&observation_bytes)) != grant.model_observation_sha256
        || observation.observed_at_unix > at
        || at.saturating_sub(observation.observed_at_unix) > 300
    {
        return Err(HostError::Policy);
    }
    let raw_path = credential_dir.join(format!(
        "model-response-{}.bin",
        observation.response_sha256
    ));
    let raw = fs::read(raw_path).map_err(|_| HostError::Policy)?;
    if raw.len() != observation.response_bytes
        || raw.len() > crate::public_https::MODEL_BODY_LIMIT
        || format!("{:x}", Sha256::digest(&raw)) != observation.response_sha256
    {
        return Err(HostError::Policy);
    }
    let parsed: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|_| HostError::Policy)?;
    let parsed_models = parsed
        .get("models")
        .and_then(|v| v.as_array())
        .ok_or(HostError::Policy)?;
    if parsed_models.len() != observation.models.len()
        || !parsed_models
            .iter()
            .zip(&observation.models)
            .all(|(raw, saved)| {
                raw.get("slug").and_then(|v| v.as_str()) == Some(saved.slug.as_str())
                    && raw.get("visibility").and_then(|v| v.as_str())
                        == Some(saved.visibility.as_str())
            })
        || !observation
            .models
            .iter()
            .any(|m| m.slug == grant.selected_model && m.visibility == "list")
    {
        return Err(HostError::Policy);
    }
    Ok(observation)
}

fn prepared_projection(
    smoke: SmokeProjection,
    operation_id: String,
    binding_sha256: &str,
    wire_sha256: &str,
    admission_sha256: String,
) -> DiagnosticProjection {
    DiagnosticProjection {
        release_id: smoke.release_id,
        operation_id,
        attempt_id: smoke.attempt_id,
        state: smoke.state,
        terminal: smoke.terminal,
        prepared_binding_sha256: binding_sha256.into(),
        prepared_wire_sha256: wire_sha256.into(),
        admission_sha256,
        response_sha256: smoke.response_sha256,
        response_bytes: smoke.response_bytes,
        received_bytes: smoke.received_bytes,
        full_capture_complete: smoke.full_capture_complete,
        http_status: smoke.http_status,
        content_type_meta: smoke.content_type_meta,
        failure_category: smoke.failure_category,
        exact_capture: smoke.exact_capture,
        quarantined: smoke.quarantined,
        funding_obligation_open: smoke.funding_obligation_open,
        local_transport_closed: smoke.local_transport_closed,
    }
}

/// Every `DirectSendGrant` field that names the prepared call, the fixed
/// public profile and the composing host's source equals the current value.
fn direct_grant_binds(
    grant: &DirectSendGrant,
    call: &DirectPreparedCall,
    host_source_sha256: &str,
) -> bool {
    let binding = &call.binding;
    grant.profile_revision == PublicHostEndpoint::PROFILE_REVISION
        && call.wire.profile_revision == grant.profile_revision
        && call.wire.requested_provider == PublicHostEndpoint::PROVIDER
        && grant.smoke.operation_id == binding.operation_id
        && grant.smoke.attempt_id == binding.attempt_id
        && grant.role == binding.role
        && grant.exposure_id == binding.exposure_id
        && grant.prepared_binding_sha256 == call.binding_sha256
        && grant.prepared_wire_sha256 == call.wire_sha256
        && grant.source_basis_sha256 == call.source_basis_sha256
        && grant.candidate_sha256 == call.candidate_sha256
        && grant.compile_binding_sha256 == call.compile_binding_sha256
        && valid_sha256(host_source_sha256)
        && grant.host_source_sha256 == host_source_sha256
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// What the release owner needs to issue the action grant and the
/// `DirectSendGrant` for one exact prepared call. It carries current values
/// only, never a bearer, token or other credential material.
#[derive(Serialize)]
struct DirectReleaseRequest<'a> {
    schema: &'static str,
    call: &'a DirectPreparedCall,
    provider: &'static str,
    profile_revision: &'static str,
    model: &'a str,
    bounds: Bounds,
    release_dir: &'a Path,
    action_input: String,
    grant_input: String,
    /// How a release that the check reports `ready` is used.
    ready_release_use: &'static str,
    action: DirectReleaseAction<'a>,
    grant: DirectReleaseGrant<'a>,
}

#[derive(Serialize)]
struct DirectReleaseAction<'a> {
    action: &'static str,
    credential_dir: &'a Path,
    host_id: &'a str,
    work_dir: &'a Path,
    source_sha256: String,
    executable_sha256: &'a str,
    cleanup_disposition: &'static str,
    max_validity_secs: u64,
    min_remaining_secs_at_check: u64,
    smoke_grant_sha256: &'static str,
}

#[derive(Serialize)]
struct DirectReleaseGrant<'a> {
    role: DirectRole,
    smoke_operation_id: &'a str,
    smoke_attempt_id: &'a str,
    smoke_selected_model: &'a str,
    prepared_binding_sha256: &'a str,
    prepared_wire_sha256: &'a str,
    source_basis_sha256: &'a str,
    candidate_sha256: Option<&'a str>,
    compile_binding_sha256: Option<&'a str>,
    exposure_id: &'a str,
    profile_revision: &'static str,
    host_source_sha256: &'a str,
}

/// The latest pre-validation outcome for one prepared call.
#[derive(Serialize)]
struct DirectReleaseCheck<'a> {
    schema: &'static str,
    operation_id: &'a str,
    prepared_binding_sha256: &'a str,
    status: &'static str,
    reason: Option<&'a str>,
    checked_at_unix: u64,
}

/// A `direct_admitted` failure, split at the durable action claim.
#[derive(Debug)]
pub(crate) enum DirectAdmitError {
    /// Before the claim: nothing consumed, nothing sent.
    Withheld(HostError),
    /// At or after the claim: the action is spent; the journal decides.
    Claimed(HostError),
}

impl DirectAdmitError {
    fn into_host(self) -> HostError {
        match self {
            Self::Withheld(error) | Self::Claimed(error) => error,
        }
    }
}

/// The gap between a `Ready` offer and the send boundary that the offer's
/// time-bound pre-validation anticipates. It only lowers how often inputs
/// lapse in that gap; it guarantees nothing. A send-boundary refusal before
/// the action claim is `Withheld`, so the lifecycle owner withdraws its
/// intent and the call stays offerable.
const DIRECT_RELEASE_SEND_GAP_SECS: u64 = 3;

/// Remaining validity required at offer for the action grant, the funding
/// observation and the credential: the send boundary's 30-second requirement
/// plus the offer-to-send gap.
const DIRECT_RELEASE_MIN_REMAINING_SECS: u64 = 30 + DIRECT_RELEASE_SEND_GAP_SECS;

/// The owner inputs one `Ready` offer validated, for the next send of the
/// same prepared call.
struct CheckedRelease {
    prepared_binding_sha256: String,
    action: ActionGrant,
    grant_bytes: Vec<u8>,
}

/// Whether the owner inputs for one prepared call can be consumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectAdmissionStatus {
    /// No owner release is present; a submission now would be `NotSent`.
    InputsMissing,
    /// Owner release files are present. Validity is decided only at the send
    /// boundary; presence is not an admission.
    InputsPresent,
    /// The operation or attempt identity is already claimed in custody; it can
    /// never be submitted again.
    IdentityClaimed,
}

const RELEASE_INPUT_LIMIT: u64 = 64 * 1024;

/// Production public Direct endpoint. Every send is one
/// `PublicHost::direct_admitted` call for the exact prepared call, consuming
/// owner-supplied action and Direct send grants from a private release
/// directory. Before any lifecycle send intent, `offer_release` projects the
/// exact call to the owner and reports a missing or inconsistent release as
/// `Pending`, consuming nothing. A `Ready` offer pre-validates every check the
/// send boundary makes short of consuming, time-bound ones for the
/// offer-to-send gap, and the next send of that call consumes exactly the
/// inputs it validated; later changes to the release files neither revoke nor
/// replace them. Missing, invalid or lapsed owner inputs found at the send
/// boundary before the action claim yield `Withheld`; a refusal after the
/// claim is a terminal `NotSent`; an identity already claimed in custody, or
/// an existing journal, yields `OutcomeUnknown` and never a resend.
pub struct PublicHostEndpoint {
    credential_dir: PathBuf,
    host_id: String,
    release_dir: PathBuf,
    /// Startup-verified source-manifest digest every send grant must name.
    host_source_sha256: String,
    /// Digest of the running executable, checked equal to the startup pin.
    executable_sha256: String,
    /// Per operation, the inputs the latest offer found `Ready`. Any other
    /// offer outcome clears the entry; a send takes it.
    checked: std::sync::Mutex<std::collections::HashMap<String, CheckedRelease>>,
    #[cfg(test)]
    synthetic: Option<Box<dyn Fn() -> Result<PublicHttps, PublicHttpsError> + Send + Sync>>,
}

impl PublicHostEndpoint {
    pub const PROVIDER: &'static str = "openai_responses";
    /// The qualified public SIWC observation profile. The wire is the same
    /// tool-free request as `selected-reasoning-v1`; the revision names the
    /// parser and is recorded in the prepared wire and the journal.
    pub const PROFILE_REVISION: &'static str = StreamObserver::SIWC_PROFILE_REVISION;

    /// The fixed bounds of the public route. A profile with other bounds is
    /// refused before any claim.
    pub fn bounds() -> Bounds {
        Bounds {
            request_bytes: 65_536,
            stream_bytes: 1024 * 1024,
            frame_bytes: 128 * 1024,
            output_bytes: 16 * 1024,
            events: 4096,
            local_deadline_ms: 30_000,
        }
    }

    /// Bind an existing `Ready` credential store, an existing owner-only
    /// release directory, the composing host's startup-verified source
    /// manifest digest and its pinned executable digest (which must be the
    /// running executable). Read-only: nothing is created, refreshed or
    /// requested. Readiness here is not a current observation; the send
    /// boundary re-reads it.
    pub fn open(
        credential_dir: &Path,
        host_id: &str,
        release_dir: &Path,
        host_source_sha256: &str,
        executable_sha256: &str,
    ) -> Result<Self, HostError> {
        if !valid_absolute_path(credential_dir)
            || !valid_absolute_path(release_dir)
            || release_dir.starts_with(credential_dir)
            || credential_dir.starts_with(release_dir)
            || !valid_sha256(host_source_sha256)
            || executable_sha256 != ActionGrant::executable_sha256()?
        {
            return Err(HostError::Policy);
        }
        owner_only_existing_dir(release_dir)?;
        match AuthStore::inspect_readiness(
            credential_dir,
            host_id,
            crate::public_https::local_issuer()?,
            now(),
        ) {
            Ok(Readiness::Ready(_)) => {}
            _ => return Err(HostError::Auth),
        }
        Ok(Self {
            credential_dir: credential_dir.to_path_buf(),
            host_id: host_id.into(),
            release_dir: release_dir.to_path_buf(),
            host_source_sha256: host_source_sha256.into(),
            executable_sha256: executable_sha256.into(),
            checked: Default::default(),
            #[cfg(test)]
            synthetic: None,
        })
    }

    #[cfg(test)]
    fn synthetic(
        https: impl Fn() -> Result<PublicHttps, PublicHttpsError> + Send + Sync + 'static,
        credential_dir: &Path,
        host_id: &str,
        release_dir: &Path,
    ) -> Self {
        Self {
            credential_dir: credential_dir.to_path_buf(),
            host_id: host_id.into(),
            release_dir: release_dir.to_path_buf(),
            host_source_sha256: tests::HOST_SOURCE_SHA256.into(),
            executable_sha256: ActionGrant::executable_sha256().unwrap(),
            checked: Default::default(),
            synthetic: Some(Box::new(https)),
        }
    }

    /// The host source digest every `DirectSendGrant` must name.
    pub fn host_source_sha256(&self) -> &str {
        &self.host_source_sha256
    }

    fn release_file(&self, kind: &str, operation_id: &str) -> PathBuf {
        self.release_dir
            .join(format!("direct-{kind}-{}.json", direct_identity_key(operation_id)))
    }

    /// The request projection and latest release check for one operation.
    pub fn release_projection_paths(&self, operation_id: &str) -> (PathBuf, PathBuf) {
        (
            self.release_file("request", operation_id),
            self.release_file("release-check", operation_id),
        )
    }

    fn release_request<'a>(
        &'a self,
        call: &'a DirectPreparedCall,
        work_dir: &'a Path,
    ) -> DirectReleaseRequest<'a> {
        let (action_input, grant_input) = self.release_input_paths(&call.binding.operation_id);
        let name = |p: PathBuf| {
            p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        DirectReleaseRequest {
            schema: "b3-direct-release-request-v1",
            call,
            provider: Self::PROVIDER,
            profile_revision: Self::PROFILE_REVISION,
            model: &call.wire.requested_model,
            bounds: Self::bounds(),
            release_dir: &self.release_dir,
            action_input: name(action_input),
            grant_input: name(grant_input),
            ready_release_use: "the next send of this exact call consumes the inputs the ready check validated; later changes to the input files do not revoke or replace them",
            action: DirectReleaseAction {
                action: "direct",
                credential_dir: &self.credential_dir,
                host_id: &self.host_id,
                work_dir,
                source_sha256: ActionGrant::source_sha256(),
                executable_sha256: &self.executable_sha256,
                cleanup_disposition: "retain-until-owner-disconnect",
                max_validity_secs: 45,
                min_remaining_secs_at_check: DIRECT_RELEASE_MIN_REMAINING_SECS,
                smoke_grant_sha256: "sha256 of the direct-grant input bytes",
            },
            grant: DirectReleaseGrant {
                role: call.binding.role,
                smoke_operation_id: &call.binding.operation_id,
                smoke_attempt_id: &call.binding.attempt_id,
                smoke_selected_model: &call.wire.requested_model,
                prepared_binding_sha256: &call.binding_sha256,
                prepared_wire_sha256: &call.wire_sha256,
                source_basis_sha256: &call.source_basis_sha256,
                candidate_sha256: call.candidate_sha256.as_deref(),
                compile_binding_sha256: call.compile_binding_sha256.as_deref(),
                exposure_id: &call.binding.exposure_id,
                profile_revision: Self::PROFILE_REVISION,
                host_source_sha256: &self.host_source_sha256,
            },
        }
    }

    /// Pure pre-validation of the owner's release for `call`: every check the
    /// send boundary makes that can be made without consuming or claiming
    /// anything, with time-bound ones still holding after the offer-to-send
    /// gap. Passing it is not an admission; it returns the inputs it checked.
    fn prevalidate_release(
        &self,
        call: &DirectPreparedCall,
        work_dir: &Path,
    ) -> Result<(ActionGrant, Vec<u8>), &'static str> {
        call.validate().map_err(|_| "prepared call invalid")?;
        let (action, grant_bytes) = self
            .release_inputs(&call.binding.operation_id)
            .map_err(|_| "release inputs missing or not owner-only regular files")?;
        action
            .validate(
                "direct",
                &self.credential_dir,
                &self.host_id,
                Some(work_dir),
                Some(&grant_bytes),
            )
            .map_err(|_| "action grant fields, source, executable, window or grant digest")?;
        if action.executable_sha256 != self.executable_sha256 {
            return Err("action grant executable is not the startup-pinned executable");
        }
        if action
            .remaining()
            .map_or(true, |d| d < Duration::from_secs(DIRECT_RELEASE_MIN_REMAINING_SECS))
        {
            return Err("action grant window too short for a bounded send");
        }
        action.verify_unspent().map_err(|_| "action release already spent")?;
        action
            .action_path()
            .map_err(|_| "action release id is not a safe identifier")?;
        action
            .verify_existing_custody()
            .map_err(|_| "credential custody contract missing or not the action's")?;
        let grant: DirectSendGrant =
            serde_json::from_slice(&grant_bytes).map_err(|_| "direct send grant unparsable")?;
        if !direct_grant_binds(&grant, call, &self.host_source_sha256) {
            return Err("direct send grant does not bind this prepared call and host source");
        }
        let readiness = |at: u64| -> Result<Readiness, &'static str> {
            AuthStore::inspect_readiness(
                &self.credential_dir,
                &self.host_id,
                crate::public_https::local_issuer().map_err(|_| "issuer unavailable")?,
                at,
            )
            .map_err(|_| "credential store not ready")
        };
        let checked_at = now();
        let selected = match readiness(checked_at)? {
            Readiness::Ready(selected) => selected,
            _ => return Err("credential store not ready"),
        };
        // The send boundary needs a bearer valid 30 s past it: the credential
        // must still be ready, for the same registration, after the gap.
        match readiness(checked_at.saturating_add(DIRECT_RELEASE_MIN_REMAINING_SECS))? {
            Readiness::Ready(later) if later == selected => {}
            _ => return Err("credential expires before the send boundary"),
        }
        let send_at = checked_at.saturating_add(DIRECT_RELEASE_SEND_GAP_SECS);
        for at in [checked_at, send_at] {
            grant
                .smoke
                .validate(&selected, &call.wire.requested_model, at)
                .map_err(|_| "smoke grant registration, model, funding or custody fields")?;
            current_model_observation(&self.credential_dir, &grant.smoke, &selected, at)
                .map_err(|_| "model catalog observation missing, inconsistent or stale")?;
        }
        if grant.smoke.funding_observation_valid_until_unix
            < checked_at.saturating_add(DIRECT_RELEASE_MIN_REMAINING_SECS)
        {
            return Err("funding observation expires before the send boundary");
        }
        Ok((action, grant_bytes))
    }

    /// Where the owner places the action grant and the `DirectSendGrant` bytes
    /// for one lifecycle operation.
    pub fn release_input_paths(&self, operation_id: &str) -> (PathBuf, PathBuf) {
        let key = direct_identity_key(operation_id);
        (
            self.release_dir.join(format!("direct-action-{key}.json")),
            self.release_dir.join(format!("direct-grant-{key}.json")),
        )
    }

    /// Read-only status a host can consult before beginning a lifecycle
    /// attempt, so a missing owner release does not spend the operation.
    pub fn admission_status(&self, call: &DirectPreparedCall) -> DirectAdmissionStatus {
        if self.identity_claimed(call) {
            return DirectAdmissionStatus::IdentityClaimed;
        }
        let (action, grant) = self.release_input_paths(&call.binding.operation_id);
        if read_owner_input(&action).is_ok() && read_owner_input(&grant).is_ok() {
            DirectAdmissionStatus::InputsPresent
        } else {
            DirectAdmissionStatus::InputsMissing
        }
    }

    fn identity_claimed(&self, call: &DirectPreparedCall) -> bool {
        [
            ("operation", &call.binding.operation_id),
            ("attempt", &call.binding.attempt_id),
        ]
        .iter()
        .any(|(kind, id)| {
            let path = self
                .credential_dir
                .join(format!("smoke-{kind}-{}.json", direct_identity_key(id)));
            !matches!(fs::symlink_metadata(path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
        })
    }

    fn release_inputs(&self, operation_id: &str) -> Result<(ActionGrant, Vec<u8>), HostError> {
        // The directory's ownership and mode are rechecked on every read.
        owner_only_existing_dir(&self.release_dir)?;
        let (action_path, grant_path) = self.release_input_paths(operation_id);
        let action: ActionGrant = serde_json::from_slice(&read_owner_input(&action_path)?)
            .map_err(|_| HostError::Policy)?;
        Ok((action, read_owner_input(&grant_path)?))
    }

    async fn host(&self) -> Result<PublicHost, HostError> {
        #[cfg(test)]
        if let Some(https) = &self.synthetic {
            return PublicHost::synthetic_local(https()?, &self.credential_dir, &self.host_id);
        }
        PublicHost::open(&self.credential_dir, &self.host_id).await
    }

    fn classify(
        journal_path: &Path,
        work_dir: &Path,
        call: &DirectPreparedCall,
        admitted: bool,
    ) -> DirectExchange {
        match AttemptJournal::open(journal_path) {
            Ok(journal) => {
                let record = journal.record().clone();
                let fenced = work_dir
                    .join(format!(
                        "pre-send-fence-{}.json",
                        direct_identity_key(&call.binding.attempt_id)
                    ))
                    .is_file();
                match record.state {
                    AttemptState::Prepared => DirectExchange::NotSent,
                    // Durable intent, then final currentness failed: the
                    // fence records that no POST was dispatched.
                    AttemptState::PossiblyEntered if fenced && record.http.is_none() => {
                        DirectExchange::NotSent
                    }
                    AttemptState::TerminalObserved if admitted => {
                        DirectExchange::Observed(Box::new(record))
                    }
                    _ => DirectExchange::OutcomeUnknown,
                }
            }
            Err(_) => match fs::symlink_metadata(journal_path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => DirectExchange::NotSent,
                _ => DirectExchange::OutcomeUnknown,
            },
        }
    }
}

impl DirectEndpoint for PublicHostEndpoint {
    fn provider(&self) -> &str {
        Self::PROVIDER
    }

    fn evidence(&self) -> DirectEvidenceClass {
        DirectProductionProfile::public_api()
    }

    fn profile_revision(&self) -> &str {
        Self::PROFILE_REVISION
    }

    /// Admission binds the prepared call; an exchange without it never sends.
    fn exchange(
        &self,
        _journal_path: &Path,
        _operation_id: &str,
        _attempt_id: &str,
        _request: &AssembledRequest,
    ) -> DirectExchange {
        DirectExchange::NotSent
    }

    fn exchange_prepared(
        &self,
        journal_path: &Path,
        call: &DirectPreparedCall,
        request: &AssembledRequest,
    ) -> DirectExchange {
        if fs::symlink_metadata(journal_path).is_ok() || self.identity_claimed(call) {
            return DirectExchange::OutcomeUnknown;
        }
        let Some(work_dir) = journal_path.parent() else {
            return DirectExchange::Withheld;
        };
        // An internal request/profile mismatch is a defect no owner release
        // can repair, so it stays a terminal `NotSent`.
        if request.bytes != call.wire.wire_bytes
            || request.sha256 != call.wire_sha256
            || request.profile_revision != Self::PROFILE_REVISION
            || request.bounds != Self::bounds()
        {
            return DirectExchange::NotSent;
        }
        // A `Ready` offer for this exact call fixed the inputs this send
        // consumes; they are re-validated in full below. Without one, the
        // release files are read now.
        let checked = self
            .checked
            .lock()
            .ok()
            .and_then(|mut checked| checked.remove(&call.binding.operation_id))
            .filter(|release| release.prepared_binding_sha256 == call.binding_sha256);
        let (action, grant_bytes) = match checked {
            Some(release) => (release.action, release.grant_bytes),
            None => match self.release_inputs(&call.binding.operation_id) {
                Ok(inputs) => inputs,
                Err(_) => return DirectExchange::Withheld,
            },
        };
        if action.executable_sha256 != self.executable_sha256 {
            return DirectExchange::Withheld;
        }
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return DirectExchange::Withheld;
        };
        let result = runtime.block_on(async {
            let host = self.host().await.map_err(DirectAdmitError::Withheld)?;
            host.direct_admitted_staged(
                &action,
                &grant_bytes,
                call,
                work_dir,
                journal_path,
                &self.host_source_sha256,
            )
            .await
        });
        drop(runtime);
        // Withheld only with positive local evidence too: no journal, and
        // neither identity claimed in custody.
        if matches!(result, Err(DirectAdmitError::Withheld(_)))
            && matches!(fs::symlink_metadata(journal_path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
            && !self.identity_claimed(call)
        {
            return DirectExchange::Withheld;
        }
        Self::classify(journal_path, work_dir, call, result.is_ok())
    }

    /// Custody claims span work directories: another one may have sent this
    /// call although no journal exists here.
    fn identity_may_be_consumed(&self, call: &DirectPreparedCall) -> bool {
        self.identity_claimed(call)
    }

    /// The complete raw HTTP body is retained at the capture path, so its
    /// digest is recorded rather than a normalized observation digest.
    fn capture_digest(&self, record: &AttemptRecord) -> Option<String> {
        let http = record.http.as_ref()?;
        if http.status != Some(200) || !http.body_complete {
            return None;
        }
        http.body_sha256.clone()
    }

    /// Project the exact call to its release owner, then pre-validate the
    /// owner's inputs without consuming them. A claimed identity is `Ready`:
    /// the send path then reports `OutcomeUnknown`, never a resend, so a
    /// prior possible send is never hidden behind `Pending`. Only a `Ready`
    /// pre-validation leaves checked inputs for the next send of this call.
    fn offer_release(&self, journal_path: &Path, call: &DirectPreparedCall) -> DirectReleaseState {
        let operation = &call.binding.operation_id;
        if let Ok(mut checked) = self.checked.lock() {
            checked.remove(operation);
        }
        if self.identity_claimed(call) {
            return DirectReleaseState::Ready;
        }
        let pending = |reason: &str| DirectReleaseState::Pending {
            reason: reason.into(),
        };
        let Some(work_dir) = journal_path.parent() else {
            return pending("journal path has no work directory");
        };
        if owner_only_existing_dir(&self.release_dir).is_err() {
            return pending("release directory is not an owner-only directory");
        }
        let (request_path, check_path) = self.release_projection_paths(operation);
        let Ok(request) = serde_json::to_vec_pretty(&self.release_request(call, work_dir)) else {
            return pending("release request unserializable");
        };
        if replace_private(&self.release_dir, &request_path, &request).is_err() {
            return pending("release request projection could not be written");
        }
        let outcome = self.prevalidate_release(call, work_dir);
        let check = DirectReleaseCheck {
            schema: "b3-direct-release-check-v1",
            operation_id: operation,
            prepared_binding_sha256: &call.binding_sha256,
            status: if outcome.is_ok() { "ready" } else { "pending" },
            reason: outcome.as_ref().err().copied(),
            checked_at_unix: now(),
        };
        let written = serde_json::to_vec_pretty(&check)
            .ok()
            .is_some_and(|bytes| replace_private(&self.release_dir, &check_path, &bytes).is_ok());
        match outcome {
            Ok((action, grant_bytes)) if written => match self.checked.lock() {
                Ok(mut checked) => {
                    checked.insert(
                        operation.clone(),
                        CheckedRelease {
                            prepared_binding_sha256: call.binding_sha256.clone(),
                            action,
                            grant_bytes,
                        },
                    );
                    DirectReleaseState::Ready
                }
                Err(_) => pending("checked release could not be retained"),
            },
            Ok(_) => pending("release check could not be recorded"),
            Err(reason) => pending(reason),
        }
    }
}

/// Atomically replace one owner-only projection file inside `dir`.
fn replace_private(dir: &Path, path: &Path, bytes: &[u8]) -> Result<(), HostError> {
    if path.parent() != Some(dir) {
        return Err(HostError::Policy);
    }
    let name = path.file_name().ok_or(HostError::Policy)?;
    let temp = dir.join(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let _ = fs::remove_file(&temp);
    let mut file = private_new(&temp)?;
    let written = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| HostError::Io);
    drop(file);
    if written.is_err() || fs::rename(&temp, path).is_err() {
        let _ = fs::remove_file(&temp);
        return Err(HostError::Io);
    }
    sync_dir(dir)
}

fn owner_only_existing_dir(path: &Path) -> Result<(), HostError> {
    let meta = fs::symlink_metadata(path).map_err(|_| HostError::Policy)?;
    if !meta.file_type().is_dir()
        || meta.permissions().mode() & 0o077 != 0
        || meta.uid() != rustix::process::getuid().as_raw()
    {
        return Err(HostError::Policy);
    }
    Ok(())
}

/// An owner input is a bounded regular file owned by this account and not
/// writable by others. The final component is opened with `O_NOFOLLOW` (a
/// symlink fails the open) and `O_NONBLOCK` (a FIFO cannot stall the host),
/// and every check is made on the opened descriptor, so a swap between check
/// and read cannot substitute another file.
fn read_owner_input(path: &Path) -> Result<Vec<u8>, HostError> {
    use std::io::Read;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(path)
        .map_err(|_| HostError::Policy)?;
    let meta = file.metadata().map_err(|_| HostError::Policy)?;
    if !meta.file_type().is_file()
        || meta.len() > RELEASE_INPUT_LIMIT
        || meta.permissions().mode() & 0o022 != 0
        || meta.uid() != rustix::process::getuid().as_raw()
    {
        return Err(HostError::Policy);
    }
    let mut bytes = Vec::new();
    file.take(RELEASE_INPUT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| HostError::Policy)?;
    if bytes.len() as u64 > RELEASE_INPUT_LIMIT {
        return Err(HostError::Policy);
    }
    Ok(bytes)
}

struct CaptureStatusGuard {
    path: PathBuf,
    received_bytes: usize,
    retained_bytes: Option<usize>,
    sha256: Option<String>,
    full_capture_complete: bool,
}

impl Drop for CaptureStatusGuard {
    fn drop(&mut self) {
        let status = serde_json::json!({
            "received_bytes": self.received_bytes,
            "retained_bytes": self.retained_bytes,
            "retained_sha256": self.sha256,
            "full_capture_complete": self.full_capture_complete,
            "unknown_retained_prefix": self.retained_bytes.is_none(),
        });
        if let Ok(bytes) = serde_json::to_vec(&status) {
            let _ = persist_new(&self.path, &bytes);
        }
    }
}

struct LocalClosureGuard(PathBuf);

impl Drop for LocalClosureGuard {
    fn drop(&mut self) {
        let _ = persist_new(&self.0, br#"{"stream_handle_closed":true,"owned_listener":false,"remote_settlement":"unknown_from_local_closure"}"#);
    }
}

fn http_error_shape(bytes: &[u8], failure: Option<&StreamError>) -> HttpErrorShape {
    if matches!(failure, Some(StreamError::StreamTooLarge)) {
        return HttpErrorShape::TooLarge;
    }
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(value) => match (value.get("detail").is_some(), value.get("error").is_some()) {
            (true, true) => HttpErrorShape::DetailAndStructured,
            (true, false) => HttpErrorShape::Detail,
            (false, true) => HttpErrorShape::Structured,
            (false, false) => HttpErrorShape::OtherJson,
        },
        Err(_) => HttpErrorShape::NonJson,
    }
}

fn observe_content_type(
    header: Option<&HeaderValue>,
    raw_token: &str,
) -> (Option<String>, ContentTypeMeta) {
    let prefix = header
        .and_then(|h| h.to_str().ok())
        .is_some_and(|v| v.starts_with("text/event-stream"));
    let Some(header) = header else {
        return (
            None,
            ContentTypeMeta {
                capture: ContentTypeCapture::Missing,
                bytes: Some(0),
                event_stream_prefix: prefix,
            },
        );
    };
    let raw = header.as_bytes();
    // Screen before retaining value, length or a digest. This is diagnostic
    // suppression only: it does not alter stream acceptance or body quarantine.
    if contains_known_secret(raw, raw_token) || decoded_secret_suspected(raw, raw_token) {
        return (
            None,
            ContentTypeMeta {
                capture: ContentTypeCapture::Redacted,
                bytes: None,
                event_stream_prefix: prefix,
            },
        );
    }
    let (value, capture) = if raw.len() > CONTENT_TYPE_VALUE_LIMIT {
        (None, ContentTypeCapture::Oversized)
    } else if !raw.iter().all(|b| (0x20..=0x7e).contains(b)) {
        (None, ContentTypeCapture::NonText)
    } else {
        match header.to_str() {
            Ok(value) => (Some(value.to_owned()), ContentTypeCapture::Captured),
            Err(_) => (None, ContentTypeCapture::NonText),
        }
    };
    (
        value,
        ContentTypeMeta {
            capture,
            bytes: Some(raw.len()),
            event_stream_prefix: prefix,
        },
    )
}

fn contains_known_secret(bytes: &[u8], raw_token: &str) -> bool {
    (!raw_token.is_empty()
        && bytes
            .windows(raw_token.len())
            .any(|w| w == raw_token.as_bytes()))
        || bytes.windows(7).any(|w| w.eq_ignore_ascii_case(b"Bearer "))
        || bytes.windows(3).any(|w| w == b"sk-")
}

fn decoded_secret_suspected(bytes: &[u8], raw_token: &str) -> bool {
    fn walk(value: &serde_json::Value, raw_token: &str) -> bool {
        match value {
            serde_json::Value::String(s) => contains_known_secret(s.as_bytes(), raw_token),
            serde_json::Value::Array(xs) => xs.iter().any(|x| walk(x, raw_token)),
            serde_json::Value::Object(xs) => xs
                .iter()
                .any(|(k, v)| contains_known_secret(k.as_bytes(), raw_token) || walk(v, raw_token)),
            _ => false,
        }
    }
    if serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .is_some_and(|v| walk(&v, raw_token))
    {
        return true;
    }
    // Match the observer's SSE data-line assembly before JSON decoding.
    for frame in bytes
        .split(|b| *b == b'\n')
        .collect::<Vec<_>>()
        .split(|line| line.is_empty() || *line == [b'\r'])
    {
        let mut data = Vec::new();
        for line in frame {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if let Some(value) = line.strip_prefix(b"data:") {
                if !data.is_empty() {
                    data.push(b'\n');
                }
                data.extend_from_slice(value.strip_prefix(b" ").unwrap_or(value));
            }
        }
        if serde_json::from_slice::<serde_json::Value>(&data)
            .ok()
            .is_some_and(|v| walk(&v, raw_token))
        {
            return true;
        }
    }
    false
}

fn safe_terminal(terminal: Option<&ObservedTerminal>) -> Option<ObservedTerminal> {
    terminal.map(|t| match t {
        ObservedTerminal::Completed => ObservedTerminal::Completed,
        ObservedTerminal::Failed { .. } => ObservedTerminal::Failed { code: None },
        ObservedTerminal::Incomplete { .. } => ObservedTerminal::Incomplete { reason: None },
    })
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
fn safe_id(id: &str) -> Result<&str, HostError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        Err(HostError::Policy)
    } else {
        Ok(id)
    }
}
fn prepare_private_dir(path: &Path) -> Result<(), HostError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(HostError::Policy);
    }
    let mut cursor = std::path::PathBuf::new();
    for component in path.components() {
        cursor.push(component);
        if let Ok(meta) = fs::symlink_metadata(&cursor) {
            if meta.file_type().is_symlink() {
                return Err(HostError::Policy);
            }
        }
    }
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    match builder.create(path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(_) => return Err(HostError::Io),
    }
    let meta = fs::symlink_metadata(path).map_err(|_| HostError::Io)?;
    if !meta.file_type().is_dir()
        || meta.permissions().mode() & 0o077 != 0
        || meta.uid() != rustix::process::getuid().as_raw()
    {
        return Err(HostError::Policy);
    }
    Ok(())
}
fn private_new(path: &Path) -> Result<File, HostError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| HostError::Evidence)
}
fn persist_new(path: &Path, bytes: &[u8]) -> Result<(), HostError> {
    let mut file = private_new(path)?;
    file.write_all(bytes).map_err(|_| HostError::Evidence)?;
    file.sync_all().map_err(|_| HostError::Evidence)?;
    sync_dir(path.parent().ok_or(HostError::Evidence)?)
}
fn sync_dir(path: &Path) -> Result<(), HostError> {
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|_| HostError::Evidence)
}

async fn receive_callback_async(
    listener: TcpListener,
    redirect: &str,
    deadline: Duration,
) -> Result<String, HostError> {
    let deadline = tokio::time::Instant::now() + deadline;
    let listener = tokio::net::TcpListener::from_std(listener).map_err(|_| HostError::Io)?;
    loop {
        let (mut socket, peer) = tokio::time::timeout_at(deadline, listener.accept())
            .await
            .map_err(|_| HostError::Auth)?
            .map_err(|_| HostError::Io)?;
        if peer.ip() != IpAddr::V4(std::net::Ipv4Addr::LOCALHOST) {
            continue;
        }
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            let n = tokio::time::timeout_at(deadline, socket.read(&mut chunk))
                .await
                .map_err(|_| HostError::Auth)?
                .map_err(|_| HostError::Auth)?;
            if n == 0 || bytes.len().saturating_add(n) > 8192 {
                return Err(HostError::Auth);
            }
            bytes.extend_from_slice(&chunk[..n]);
            if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let request = parse_callback_request(&bytes, redirect)?;
        tokio::time::timeout_at(deadline, socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 25\r\nConnection: close\r\n\r\nAuthorization received.\n"))
            .await.map_err(|_| HostError::Auth)?.map_err(|_| HostError::Io)?;
        return Ok(request);
    }
}
fn parse_callback_request(bytes: &[u8], redirect: &str) -> Result<String, HostError> {
    let request = std::str::from_utf8(bytes).map_err(|_| HostError::Auth)?;
    let mut lines = request.split("\r\n");
    let first = lines.next().ok_or(HostError::Auth)?;
    let path = first
        .strip_prefix("GET ")
        .and_then(|v| v.strip_suffix(" HTTP/1.1"))
        .ok_or(HostError::Auth)?;
    if !path.starts_with("/auth/callback?") || path.contains('#') {
        return Err(HostError::Auth);
    }
    let base = url::Url::parse(redirect).map_err(|_| HostError::Auth)?;
    let host = format!("127.0.0.1:{}", base.port().ok_or(HostError::Auth)?);
    let mut host_seen = false;
    for line in lines {
        if let Some(value) = line
            .strip_prefix("Host: ")
            .or_else(|| line.strip_prefix("host: "))
        {
            if host_seen || value != host {
                return Err(HostError::Auth);
            }
            host_seen = true;
        }
    }
    if !host_seen {
        return Err(HostError::Auth);
    }
    Ok(format!("http://{host}{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use serde_json::json;
    use std::{
        io::Read,
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
    };

    const JWKS: &str = include_str!("../tests/fixtures/synthetic-siwc-jwks.json");
    const KEY: &[u8] = include_bytes!("../tests/fixtures/synthetic-siwc-rsa.pkcs1.der");
    const SCOPE: &str =
        "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";

    fn temp(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "b2-public-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }
    fn reply(status: &str, kind: &str, body: &[u8]) -> Vec<u8> {
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        [head.as_bytes(), body].concat()
    }
    fn read_request(io: &mut impl Read) -> (String, Vec<u8>) {
        let mut all = Vec::new();
        let mut buf = [0u8; 4096];
        let end = loop {
            let n = io.read(&mut buf).unwrap();
            assert!(n > 0);
            all.extend_from_slice(&buf[..n]);
            if let Some(i) = all.windows(4).position(|w| w == b"\r\n\r\n") {
                break i + 4;
            }
            assert!(all.len() < 32 * 1024);
        };
        let head = String::from_utf8(all[..end].to_vec()).unwrap();
        let len: usize = head
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .and_then(|v| v.trim().parse().ok())
            })
            .unwrap_or(0);
        let mut body = all[end..].to_vec();
        while body.len() < len {
            let n = io.read(&mut buf).unwrap();
            assert!(n > 0);
            body.extend_from_slice(&buf[..n]);
        }
        (head, body)
    }
    fn tls_peer(
        metadata: serde_json::Value,
        event_body: Vec<u8>,
        expected: usize,
    ) -> (
        std::net::SocketAddr,
        Vec<u8>,
        thread::JoinHandle<Vec<String>>,
    ) {
        tls_peer_auth(metadata, event_body, expected, None)
    }
    fn tls_peer_auth(
        metadata: serde_json::Value,
        event_body: Vec<u8>,
        expected: usize,
        auth: Option<Arc<Mutex<Option<(String, String)>>>>,
    ) -> (
        std::net::SocketAddr,
        Vec<u8>,
        thread::JoinHandle<Vec<String>>,
    ) {
        let cert = rcgen::generate_simple_self_signed(vec![
            "auth.openai.com".into(),
            "api.openai.com".into(),
        ])
        .unwrap();
        let der = cert.cert.der().to_vec();
        let private = rustls::pki_types::PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
        let config = Arc::new(
            rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(
                    vec![rustls::pki_types::CertificateDer::from(der.clone())],
                    private.into(),
                )
                .unwrap(),
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let mut paths = Vec::new();
            for _ in 0..expected {
                let (socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut tls = rustls::StreamOwned::new(
                    rustls::ServerConnection::new(config.clone()).unwrap(),
                    socket,
                );
                let (head, body) = read_request(&mut tls);
                let path = head.split_whitespace().nth(1).unwrap().to_owned();
                let response = match path.as_str() {
                    "/.well-known/openid-configuration" if metadata.get("redirect") == Some(&json!(true)) =>
                        b"HTTP/1.1 302 Found\r\nLocation: https://evil.invalid/metadata\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
                    "/.well-known/openid-configuration" => reply("200 OK", "application/json", metadata.to_string().as_bytes()),
                    "/.well-known/jwks.json" => reply("200 OK", "application/json", JWKS.as_bytes()),
                    "/v1/models" => {
                        let status = metadata.get("test_models_status").and_then(|v| v.as_str()).unwrap_or("200 OK");
                        let body = metadata.get("test_models_body").and_then(|v| v.as_str())
                            .map(str::as_bytes)
                            .unwrap_or(br#"{"models":[{"slug":"fake-model","visibility":"list"},{"slug":"hidden","visibility":"hide"}]}"#);
                        let oversized;
                        let body = if metadata.get("test_models_body_large") == Some(&json!(true)) {
                            oversized = vec![b'x'; crate::public_https::MODEL_BODY_LIMIT + 1];
                            oversized.as_slice()
                        } else if metadata.get("test_models_escaped_entries") == Some(&json!(true)) {
                            oversized = json!({"models":vec![json!({"slug":"\\".repeat(256),"visibility":"\\".repeat(128)}); 512]}).to_string().into_bytes();
                            oversized.as_slice()
                        } else if let Some(size) = metadata.get("test_models_padding_bytes").and_then(|v| v.as_u64()) {
                            oversized = [body, &vec![b' '; size as usize - body.len()]].concat();
                            oversized.as_slice()
                        } else {
                            body
                        };
                        let mut response = reply(status, "application/json", body);
                        if metadata.get("test_models_large_header") == Some(&json!(true)) {
                            let at = response.windows(2).position(|w| w == b"\r\n").unwrap() + 2;
                            response.splice(at..at, [b"X-Pad: ".as_slice(), &vec![b'x'; 17 * 1024], b"\r\n"].concat());
                        }
                        response
                    },
                    "/api/accounts/oauth/revoke" => {
                        assert!(head.starts_with("POST "));
                        assert!(String::from_utf8_lossy(&body).contains("token=synthetic-refresh"));
                        reply("200 OK", "text/plain", b"")
                    },
                    "/api/accounts/oauth/token" => {
                        let form: std::collections::HashMap<String,String> =
                            url::form_urlencoded::parse(&body).into_owned().collect();
                        if form.get("grant_type").map(String::as_str) == Some("refresh_token") {
                            assert_eq!(form.get("client_id").map(String::as_str), Some("synthetic-client"));
                            assert_eq!(form.get("refresh_token").map(String::as_str), Some("synthetic-refresh"));
                            assert_eq!(form.get("resource").map(String::as_str), Some("https://api.openai.com/v1"));
                            assert!(!form.contains_key("scope"));
                            let status = metadata.get("test_refresh_status").and_then(|v| v.as_str()).unwrap_or("200 OK");
                            if metadata.get("test_refresh_disconnect") == Some(&json!(true)) {
                                Vec::new()
                            } else if status != "200 OK" {
                                let body = metadata.get("test_refresh_body").and_then(|v| v.as_str()).unwrap_or("{}");
                                reply(status, "application/json", body.as_bytes())
                            } else {
                                let mut header = Header::new(Algorithm::RS256);
                                header.kid = Some("synthetic-key-1".into());
                                let t = now();
                                let sub = metadata.get("test_refresh_sub").and_then(|v| v.as_str()).unwrap_or("synthetic-subject");
                                let id = encode(&header, &json!({"iss":"https://auth.openai.com","aud":"synthetic-client",
                                    "sub":sub,"exp":t+3600,"iat":t-1}), &EncodingKey::from_rsa_der(KEY)).unwrap();
                                let token = json!({"access_token":"replacement-access","refresh_token":"replacement-refresh",
                                    "id_token":id,"token_type":"Bearer","expires_in":3600,"scope":SCOPE});
                                reply("200 OK", "application/json", token.to_string().as_bytes())
                            }
                        } else {
                        assert_eq!(form.get("grant_type").map(String::as_str), Some("authorization_code"));
                        assert_eq!(form.get("code").map(String::as_str), Some("synthetic-code"));
                        assert_eq!(form.get("client_id").map(String::as_str), Some("synthetic-client"));
                        let (nonce, challenge) = auth.as_ref().unwrap().lock().unwrap().clone().unwrap();
                        let verifier = form.get("code_verifier").unwrap();
                        assert_eq!(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())), challenge);
                        let mut header = Header::new(Algorithm::RS256);
                        header.kid = Some("synthetic-key-1".into());
                        let t = now();
                        let id = encode(&header, &json!({"iss":"https://auth.openai.com","aud":"synthetic-client",
                            "sub":"synthetic-subject","exp":t+3600,"iat":t-1,"nonce":nonce}),
                            &EncodingKey::from_rsa_der(KEY)).unwrap();
                        let token = json!({"access_token":"synthetic-access","refresh_token":"synthetic-refresh",
                            "id_token":id,"token_type":"Bearer","expires_in":3600,"scope":SCOPE});
                        reply("200 OK", "application/json", token.to_string().as_bytes())
                        }
                    },
                    "/v1/responses" => {
                        assert!(head.to_ascii_lowercase().contains("authorization: bearer synthetic-access"));
                        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
                        let diagnostic = metadata.get("test_diagnostic") == Some(&json!(true));
                        assert_eq!(json["model"], if diagnostic { "gpt-6-astra" } else { "fake-model" });
                        assert_eq!(json["store"], false);
                        assert_eq!(json["stream"], true);
                        if diagnostic {
                            assert!(json["input"][0]["content"][0]["text"].as_str().is_some_and(|s|!s.is_empty() && s != SMOKE_INPUT));
                        } else {
                            assert_eq!(json["input"][0]["content"][0]["text"], SMOKE_INPUT);
                        }
                        let status = metadata.get("test_response_status").and_then(|v| v.as_str()).unwrap_or("200 OK");
                        let kind = metadata.get("test_response_kind").and_then(|v| v.as_str()).unwrap_or("text/event-stream");
                        if metadata.get("test_response_no_content_type") == Some(&json!(true)) {
                            [format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", event_body.len()).into_bytes(), event_body.clone()].concat()
                        } else {
                            reply(status, kind, &event_body)
                        }
                    }
                    _ => panic!("unexpected path {path}"),
                };
                paths.push(path);
                if paths.last().is_some_and(|p| p == "/v1/responses")
                    && metadata.get("test_response_stall_ms").and_then(|v|v.as_u64()).is_some()
                {
                    let header = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", event_body.len()+10);
                    tls.write_all(header.as_bytes()).unwrap();
                    tls.write_all(&event_body).unwrap();
                    tls.flush().unwrap();
                    thread::sleep(Duration::from_millis(metadata["test_response_stall_ms"].as_u64().unwrap()));
                    continue;
                }
                if paths.last().is_some_and(|p| p == "/v1/models")
                    && metadata
                        .get("test_split_models_ms")
                        .and_then(|v| v.as_u64())
                        .is_some()
                {
                    let delay = metadata["test_split_models_ms"].as_u64().unwrap();
                    let split = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
                    thread::sleep(Duration::from_millis(delay));
                    if tls.write_all(&response[..split]).is_ok() {
                        let _ = tls.flush();
                        thread::sleep(Duration::from_millis(delay));
                        let _ = tls.write_all(&response[split..]);
                        let _ = tls.flush();
                    }
                } else {
                    tls.write_all(&response).unwrap();
                    tls.flush().unwrap();
                }
            }
            if let Some(delay) = metadata.get("test_hold_open_ms").and_then(|v| v.as_u64()) {
                thread::sleep(Duration::from_millis(delay));
            }
            paths
        });
        (addr, der, handle)
    }
    fn metadata() -> serde_json::Value {
        json!({"issuer":"https://auth.openai.com",
            "authorization_endpoint":"https://auth.openai.com/api/accounts/authorize",
            "token_endpoint":"https://auth.openai.com/api/accounts/oauth/token",
            "revocation_endpoint":"https://auth.openai.com/api/accounts/oauth/revoke",
            "jwks_uri":"https://auth.openai.com/.well-known/jwks.json"})
    }
    fn stream() -> Vec<u8> {
        let fixture = include_str!("../tests/fixtures/synthetic-response-v1.sse")
            .replace("observed-fixture", "fake-model")
            .replace("REJECT", "B2_TRANSPORT_OK.");
        fixture.into_bytes()
    }
    fn seeded_host(addr: std::net::SocketAddr, der: &[u8], dir: &Path) -> PublicHost {
        fs::create_dir_all(dir.parent().unwrap()).unwrap();
        let https = PublicHttps::synthetic(der, addr);
        let host = PublicHost::synthetic(https, dir, "synthetic_host_public").unwrap();
        let t = now();
        let start = host
            .store
            .begin_registration("Synthetic", "http://127.0.0.1:45123/auth/callback", t)
            .unwrap();
        let url = url::Url::parse(start.authorize_url()).unwrap();
        let param = |name: &str| {
            url.query_pairs()
                .find(|(k, _)| k == name)
                .unwrap()
                .1
                .to_string()
        };
        let exchange = host.store.accept_callback(&format!("http://127.0.0.1:45123/auth/callback?state={}&code=synthetic&client_id=synthetic-client", param("state")), t).unwrap();
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("synthetic-key-1".into());
        let id = encode(
            &header,
            &json!({"iss":"https://auth.openai.com","aud":"synthetic-client",
            "sub":"synthetic-subject","exp":t+3600,"iat":t-1,"nonce":param("nonce")}),
            &EncodingKey::from_rsa_der(KEY),
        )
        .unwrap();
        let token = crate::TokenResponse::from_json(
            serde_json::to_string(&json!({
                "access_token":"synthetic-access","refresh_token":"synthetic-refresh",
                "id_token":id,"token_type":"Bearer","expires_in":3600,"scope":SCOPE
            }))
            .unwrap()
            .as_bytes(),
        )
        .unwrap();
        host.store
            .finish_exchange(&exchange, token, JWKS, t)
            .unwrap();
        action_for(&host, "sign-in", "seed-custody")
            .consume()
            .unwrap();
        host
    }
    fn grant(host: &PublicHost, model_sha: String) -> SmokeGrant {
        let observation_id = fs::read_dir(&host.credential_dir)
            .unwrap()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with("model-observation-")
                    && name.ends_with(".json")
                    && fs::read(e.path())
                        .ok()
                        .and_then(|bytes| serde_json::from_slice::<ModelObservation>(&bytes).ok())
                        .is_some_and(|o| o.response_sha256 == model_sha)
                {
                    Some(
                        name.trim_start_matches("model-observation-")
                            .trim_end_matches(".json")
                            .to_owned(),
                    )
                } else {
                    None
                }
            })
            .next()
            .unwrap_or(model_sha);
        let selected = host.selected().unwrap();
        SmokeGrant {
            release_id: "release-one".into(),
            operation_id: "operation-one".into(),
            attempt_id: "attempt-one".into(),
            client_id: selected.client_id,
            subject: selected.subject,
            credential_revision: selected.credential_revision,
            selected_model: "fake-model".into(),
            model_observation_sha256: observation_id,
            funding_assertion_ref: "owner-note-1".into(),
            funding_actor: "account-owner".into(),
            funding_custodian: "custodian-1".into(),
            funding_setting_scope: "this-client-workspace".into(),
            funding_observed_at_unix: now(),
            credit_use_disabled: true,
            included_plan_enabled: true,
            funding_observation_valid_until_unix: now() + 600,
            account_owner_accepts_open_custody: true,
            accepted_closure_basis: "owner-review".into(),
            release_actor: "release-owner".into(),
        }
    }
    fn model_sha() -> String {
        format!(
            "{:x}",
            Sha256::digest(br#"{"models":[{"slug":"fake-model","visibility":"list"},{"slug":"hidden","visibility":"hide"}]}"#)
        )
    }
    fn action_for(host: &PublicHost, action: &str, release_id: &str) -> ActionGrant {
        let issued = now();
        ActionGrant {
            release_id: release_id.into(),
            actor: "synthetic-owner".into(),
            custody_owner: "synthetic-account-owner".into(),
            custody_custodian: "synthetic-token-custodian".into(),
            custody_contract_ref: "synthetic-custody-contract".into(),
            retention_purpose: "bounded public host auth and model inspection".into(),
            cleanup_disposition: if action == "disconnect" {
                "local-delete-and-remote-revocation-attempt"
            } else {
                "retain-until-owner-disconnect"
            }
            .into(),
            action: action.into(),
            credential_dir: host.credential_dir.clone(),
            host_id: host.host_id.clone(),
            work_dir: None,
            smoke_grant_sha256: None,
            refresh_registration: None,
            executable_sha256: ActionGrant::executable_sha256().unwrap(),
            source_sha256: ActionGrant::source_sha256(),
            issued_at_unix: issued,
            valid_until_unix: issued + 15,
        }
    }
    #[test]
    fn admitted_refresh_rotates_once_and_checks_revision() {
        let root = temp("refresh-rotation");
        let (addr, der, server) = tls_peer(metadata(), vec![], 5);
        let host = seeded_host(addr, &der, &root.join("credentials"));
        let prior = host.selected().unwrap();
        let mut action = action_for(&host, "refresh", "refresh-once");
        action.refresh_registration = Some(prior.clone());
        let updated = runtime().block_on(host.refresh_admitted(&action)).unwrap();
        assert!(updated.credential_revision > prior.credential_revision);
        assert_eq!(
            host.store.readiness(now()).unwrap(),
            Readiness::Ready(updated)
        );
        assert!(runtime().block_on(host.refresh_admitted(&action)).is_err());
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| p.as_str() == "/api/accounts/oauth/token")
                .count(),
            1
        );
    }

    #[test]
    fn rejected_refresh_identity_stays_fenced() {
        let root = temp("refresh-identity");
        let mut config = metadata();
        config["test_refresh_sub"] = json!("wrong-subject");
        let (addr, der, server) = tls_peer(config, vec![], 5);
        let host = seeded_host(addr, &der, &root.join("credentials"));
        let mut action = action_for(&host, "refresh", "refresh-identity");
        action.refresh_registration = Some(host.selected().unwrap());
        assert!(runtime().block_on(host.refresh_admitted(&action)).is_err());
        assert!(matches!(
            host.store.readiness(now()).unwrap(),
            Readiness::RefreshAmbiguous(_)
        ));
        assert_eq!(server.join().unwrap().len(), 5);
    }

    #[test]
    fn uncertain_refresh_response_fences_selected_credentials() {
        let root = temp("refresh-uncertain");
        let mut config = metadata();
        config["test_refresh_status"] = json!("503 Service Unavailable");
        config["test_refresh_body"] = json!(r#"{"error":"temporarily_unavailable"}"#);
        let (addr, der, server) = tls_peer(config, vec![], 5);
        let host = seeded_host(addr, &der, &root.join("credentials"));
        let mut action = action_for(&host, "refresh", "refresh-uncertain");
        action.refresh_registration = Some(host.selected().unwrap());
        assert!(runtime().block_on(host.refresh_admitted(&action)).is_err());
        assert!(matches!(
            host.store.readiness(now()).unwrap(),
            Readiness::RefreshAmbiguous(_)
        ));
        assert!(runtime().block_on(host.refresh_admitted(&action)).is_err());
        assert_eq!(server.join().unwrap().len(), 5);
    }

    #[test]
    fn lost_refresh_response_fences_selected_credentials() {
        let root = temp("refresh-disconnect");
        let mut config = metadata();
        config["test_refresh_disconnect"] = json!(true);
        let (addr, der, server) = tls_peer(config, vec![], 5);
        let host = seeded_host(addr, &der, &root.join("credentials"));
        let mut action = action_for(&host, "refresh", "refresh-disconnect");
        action.refresh_registration = Some(host.selected().unwrap());
        assert!(runtime().block_on(host.refresh_admitted(&action)).is_err());
        assert!(matches!(
            host.store.readiness(now()).unwrap(),
            Readiness::RefreshAmbiguous(_)
        ));
        assert_eq!(server.join().unwrap().len(), 5);
    }

    #[test]
    fn renewal_target_inspection_is_read_only() {
        let root = temp("renewal-target");
        let (addr, der, server) = tls_peer(metadata(), vec![], 2);
        let host = seeded_host(addr, &der, &root.join("credentials"));
        let prior = fs::read(host.credential_dir.join("state.json")).unwrap();
        let target =
            PublicHost::inspect_local_renewal_target(&host.credential_dir, &host.host_id).unwrap();
        assert_eq!(target, host.selected().unwrap());
        assert_eq!(
            prior,
            fs::read(host.credential_dir.join("state.json")).unwrap()
        );
        assert_eq!(server.join().unwrap().len(), 2);
    }

    #[test]
    fn missing_or_empty_refresh_credential_is_not_an_admissible_target() {
        let root = temp("renewal-missing-credential");
        let (addr, der, server) = tls_peer(metadata(), vec![], 2);
        let host = seeded_host(addr, &der, &root.join("credentials"));
        let path = host.credential_dir.join("state.json");
        let original: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for credential in [serde_json::Value::Null, {
            let mut empty = original["registrations"][0]["credential"].clone();
            empty["refresh"] = json!("");
            empty
        }] {
            let mut state = original.clone();
            state["registrations"][0]["credential"] = credential;
            fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
            let before = fs::read(&path).unwrap();
            assert!(
                PublicHost::inspect_local_renewal_target(&host.credential_dir, &host.host_id)
                    .is_err()
            );
            assert!(
                !host
                    .credential_dir
                    .join("action-missing-credential.json")
                    .exists()
            );
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        assert_eq!(server.join().unwrap().len(), 2);
    }

    #[test]
    fn invalid_client_preserves_renewal_eligibility_and_reports_configuration() {
        let root = temp("refresh-invalid-client");
        let mut config = metadata();
        config["test_refresh_status"] = json!("400 Bad Request");
        config["test_refresh_body"] = json!(r#"{"error":"invalid_client"}"#);
        let (addr, der, server) = tls_peer(config, vec![], 5);
        let host = seeded_host(addr, &der, &root.join("credentials"));
        let prior = host.selected().unwrap();
        let mut action = action_for(&host, "refresh", "refresh-invalid-client");
        action.refresh_registration = Some(prior.clone());
        assert!(matches!(
            runtime().block_on(host.refresh_admitted(&action)),
            Err(HostError::RefreshConfiguration)
        ));
        assert_eq!(
            PublicHost::inspect_local_renewal_target(&host.credential_dir, &host.host_id).unwrap(),
            prior
        );
        assert!(matches!(
            host.store.readiness(now()).unwrap(),
            Readiness::Ready(_)
        ));
        assert!(
            host.credential_dir
                .join("action-refresh-invalid-client.json")
                .exists()
        );
        assert!(runtime().block_on(host.refresh_admitted(&action)).is_err());
        assert_eq!(server.join().unwrap().len(), 5);
    }

    #[test]
    fn invalid_refresh_action_has_no_remote_or_store_effect() {
        let root = temp("refresh-invalid");
        let (addr, der, server) = tls_peer(metadata(), vec![], 2);
        let host = seeded_host(addr, &der, &root.join("credentials"));
        let prior = host.selected().unwrap();
        let action = action_for(&host, "refresh", "refresh-invalid");
        assert!(runtime().block_on(host.refresh_admitted(&action)).is_err());
        assert_eq!(host.selected().unwrap(), prior);
        assert!(
            !host
                .credential_dir
                .join("action-refresh-invalid.json")
                .exists()
        );
        assert_eq!(server.join().unwrap().len(), 2);
    }

    #[test]
    fn terminal_refresh_and_invalid_revision_do_not_replay() {
        let root = temp("refresh-terminal");
        let mut config = metadata();
        config["test_refresh_status"] = json!("400 Bad Request");
        config["test_refresh_body"] = json!(r#"{"error":"invalid_grant"}"#);
        let (addr, der, server) = tls_peer(config, vec![], 5);
        let host = seeded_host(addr, &der, &root.join("credentials"));
        let mut action = action_for(&host, "refresh", "refresh-terminal");
        let mut wrong = host.selected().unwrap();
        wrong.credential_revision += 1;
        action.refresh_registration = Some(wrong);
        assert!(runtime().block_on(host.refresh_admitted(&action)).is_err());
        assert!(
            !host
                .credential_dir
                .join("action-refresh-terminal.json")
                .exists()
        );
        action.refresh_registration = Some(host.selected().unwrap());
        assert!(runtime().block_on(host.refresh_admitted(&action)).is_err());
        assert!(matches!(
            host.store.readiness(now()).unwrap(),
            Readiness::ReauthorizationRequired(_)
        ));
        assert_eq!(server.join().unwrap().len(), 5);
    }

    fn event(value: serde_json::Value) -> String {
        format!(
            "event: {}\ndata: {}\n\n",
            value["type"].as_str().unwrap(),
            value
        )
    }

    #[test]
    fn local_readiness_projection_contains_no_registration_identity() {
        let reference = crate::RegistrationRef {
            client_id: "private-client-marker".into(),
            subject: "private-subject-marker".into(),
            credential_revision: 17,
        };
        let cases = [
            (Readiness::SignedOut, "signed_out"),
            (Readiness::Ready(reference.clone()), "ready"),
            (Readiness::Expired(reference.clone()), "expired"),
            (
                Readiness::RefreshAmbiguous(reference.clone()),
                "refresh_ambiguous",
            ),
            (
                Readiness::ReauthorizationRequired(reference),
                "reauthorization_required",
            ),
        ];
        for (source, expected) in cases {
            let projected = LocalReadiness::from(source);
            let encoded = serde_json::to_string(&projected).unwrap();
            assert_eq!(encoded, format!("\"{expected}\""));
            assert!(!format!("{projected:?} {encoded}").contains("private-"));
        }
    }

    #[test]
    fn local_readiness_inspection_uses_saved_ready_state_without_peer_or_cleanup() {
        let root = temp("local-readiness-ready");
        let (addr, der, server) = tls_peer(metadata(), vec![], 2);
        let host = seeded_host(addr, &der, &root.join("auth"));
        assert_eq!(server.join().unwrap().len(), 2);
        let state_path = host.credential_dir.join("state.json");
        let before = fs::read(&state_path).unwrap();
        let orphan = host
            .credential_dir
            .join(format!(".state-{}", "A".repeat(43)));
        fs::write(&orphan, b"preserve-orphan").unwrap();
        assert_eq!(
            PublicHost::inspect_local_readiness(&host.credential_dir, &host.host_id).unwrap(),
            LocalReadiness::Ready
        );
        assert_eq!(fs::read(&state_path).unwrap(), before);
        assert_eq!(fs::read(&orphan).unwrap(), b"preserve-orphan");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn metadata_endpoint_binding_rejects_mismatch_before_jwks() {
        let mut bad = metadata();
        bad["token_endpoint"] = json!("https://evil.invalid/token");
        let (addr, der, server) = tls_peer(bad, vec![], 1);
        let https = PublicHttps::synthetic(&der, addr);
        assert!(runtime().block_on(https.discover()).is_err());
        assert_eq!(server.join().unwrap().len(), 1);
    }

    #[test]
    fn public_endpoint_policy_rejects_redirect_and_arbitrary_destinations() {
        assert!(!super::super::public_https::trusted_auth_path(
            "https://evil.invalid/.well-known/jwks.json",
            "/.well-known/"
        ));
        assert!(!super::super::public_https::trusted_auth_path(
            "https://auth.openai.com:444/.well-known/jwks.json",
            "/.well-known/"
        ));
        assert!(!super::super::public_https::trusted_auth_path(
            "https://auth.openai.com/.well-known/jwks.json?x=1",
            "/.well-known/"
        ));
        let (addr, der, server) = tls_peer(json!({"redirect":true}), vec![], 1);
        let https = PublicHttps::synthetic(&der, addr);
        assert!(runtime().block_on(https.discover()).is_err());
        assert_eq!(server.join().unwrap().len(), 1);
    }

    #[test]
    fn public_tls_rejects_wrong_server_identity() {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let der = cert.cert.der().to_vec();
        let private = rustls::pki_types::PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der());
        let config = Arc::new(
            rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(
                    vec![rustls::pki_types::CertificateDer::from(der.clone())],
                    private.into(),
                )
                .unwrap(),
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut tls = rustls::ServerConnection::new(config).unwrap();
            while tls.is_handshaking() && tls.complete_io(&mut socket).is_ok() {}
            assert!(tls.is_handshaking());
        });
        let https = PublicHttps::synthetic(&der, addr);
        assert!(runtime().block_on(https.discover()).is_err());
        server.join().unwrap();
    }

    #[test]
    fn composed_tls_model_selection_one_shot_capture_and_open_funding() {
        let root = temp("smoke");
        let (addr, der, server) = tls_peer(metadata(), stream(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let models = runtime().block_on(host.list_models()).unwrap();
        assert_eq!(models.models[0].slug, "fake-model");
        let grant = grant(&host, model_sha());
        assert!(
            grant
                .validate(&host.selected().unwrap(), "fake-model", now())
                .is_ok()
        );
        let observed_sha = models.observation_id;
        assert_eq!(grant.model_observation_sha256, observed_sha);
        let work = root.join("work");
        let projection = runtime().block_on(host.smoke(&grant, &work)).unwrap();
        assert_eq!(projection.state, AttemptState::TerminalObserved);
        assert_eq!(projection.output_matches_fixture, Some(true));
        let record = AttemptJournal::open(work.join("attempt-attempt-one.json")).unwrap();
        assert_eq!(
            record
                .record()
                .http
                .as_ref()
                .unwrap()
                .content_type
                .as_deref(),
            Some("text/event-stream")
        );
        assert!(
            projection
                .content_type_meta
                .as_ref()
                .unwrap()
                .event_stream_prefix
        );
        assert!(!projection.quarantined);
        assert!(projection.funding_obligation_open);
        assert_eq!(fs::read(&projection.exact_capture).unwrap(), stream());
        assert_eq!(
            AttemptJournal::open(work.join("attempt-attempt-one.json"))
                .unwrap()
                .record()
                .state,
            AttemptState::TerminalObserved
        );
        assert!(runtime().block_on(host.smoke(&grant, &work)).is_err());
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| p.as_str() == "/v1/responses")
                .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn smoke_process_child() {
        let Ok(root) = std::env::var("B2_SMOKE_CHILD_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let addr = std::env::var("B2_SMOKE_CHILD_ADDR")
            .unwrap()
            .parse()
            .unwrap();
        let der = fs::read(root.join("peer.der")).unwrap();
        let https = PublicHttps::synthetic(&der, addr);
        let host = PublicHost::synthetic_local(https, &root.join("auth"), "synthetic_host_public")
            .unwrap();
        let bytes = fs::read(root.join("smoke-grant.json")).unwrap();
        let grant: SmokeGrant = serde_json::from_slice(&bytes).unwrap();
        let suffix = std::env::var("B2_SMOKE_CHILD_SUFFIX").unwrap();
        let work = root.join(format!("work-{suffix}"));
        let mut action = action_for(&host, "smoke", &format!("action-{suffix}"));
        action.work_dir = Some(work.clone());
        action.smoke_grant_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
        action.valid_until_unix = action.issued_at_unix + 45;
        if runtime()
            .block_on(host.smoke_admitted(&action, &grant, &bytes, &work))
            .is_err()
        {
            std::process::exit(17);
        }
    }

    #[test]
    fn same_smoke_release_is_fenced_across_live_peer_workdirs_and_processes() {
        let root = temp("release-processes");
        let mut settings = metadata();
        settings["test_hold_open_ms"] = json!(3000);
        let (addr, der, server) = tls_peer(settings, stream(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime().block_on(host.list_models()).unwrap();
        fs::write(root.join("peer.der"), der).unwrap();
        let grant = grant(&host, model_sha());
        fs::write(
            root.join("smoke-grant.json"),
            serde_json::to_vec(&grant).unwrap(),
        )
        .unwrap();
        let exe = std::env::current_exe().unwrap();
        let mut children = Vec::new();
        for suffix in ["one", "two"] {
            children.push(
                std::process::Command::new(&exe)
                    .arg("--exact")
                    .arg("public_host::tests::smoke_process_child")
                    .env("B2_SMOKE_CHILD_ROOT", &root)
                    .env("B2_SMOKE_CHILD_ADDR", addr.to_string())
                    .env("B2_SMOKE_CHILD_SUFFIX", suffix)
                    .spawn()
                    .unwrap(),
            );
        }
        let statuses: Vec<_> = children
            .iter_mut()
            .map(|child| child.wait().unwrap())
            .collect();
        assert_eq!(statuses.iter().filter(|s| s.success()).count(), 1);
        assert!(std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok());
        let paths = server.join().unwrap();
        assert_eq!(
            paths
                .iter()
                .filter(|p| p.as_str() == "/v1/responses")
                .count(),
            1
        );
        assert!(root.join("auth/smoke-release-release-one.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn funding_expiry_after_durable_intent_prevents_post_and_keeps_fence() {
        let root = temp("final-funding");
        let (addr, der, server) = tls_peer(metadata(), vec![], 3);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime().block_on(host.list_models()).unwrap();
        let mut grant = grant(&host, model_sha());
        grant.release_id = "test-expiring-funding".into();
        grant.funding_observation_valid_until_unix = now() + 30;
        let work = root.join("work");
        assert!(runtime().block_on(host.smoke(&grant, &work)).is_err());
        assert!(work.join("funding-test-expiring-funding.json").exists());
        assert_eq!(
            AttemptJournal::open(work.join("attempt-attempt-one.json"))
                .unwrap()
                .record()
                .state,
            AttemptState::PossiblyEntered
        );
        assert!(work.join("local-closure-attempt-one.json").exists());
        let fence: serde_json::Value = serde_json::from_slice(
            &fs::read(work.join("pre-send-fence-attempt-one.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(fence["post_dispatched"], false);
        assert!(!server.join().unwrap().iter().any(|p| p == "/v1/responses"));
        assert!(
            runtime()
                .block_on(host.smoke(&grant, &root.join("other")))
                .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn escaped_bearer_in_decoded_terminal_is_quarantined_and_publicly_sanitized() {
        let body = [
            event(json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-safe","model":"fake-model"}})),
            "event: response.failed\ndata: {\"type\":\"response.failed\",\"sequence_number\":2,\"response\":{\"id\":\"resp-safe\",\"model\":\"fake-model\",\"status\":\"failed\",\"error\":{\"code\":\"\\u0073ynthetic-access\"}}}\n\n".to_string(),
        ].concat().into_bytes();
        assert!(
            !body
                .windows(b"synthetic-access".len())
                .any(|w| w == b"synthetic-access")
        );
        let root = temp("decoded-secret");
        let (addr, der, server) = tls_peer(metadata(), body.clone(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime().block_on(host.list_models()).unwrap();
        let projection = runtime()
            .block_on(host.smoke(&grant(&host, model_sha()), &root.join("work")))
            .unwrap();
        let public = serde_json::to_string(&projection).unwrap();
        assert!(!public.contains("synthetic-access"));
        assert!(projection.quarantined);
        assert_eq!(projection.state, AttemptState::OutcomeUnknown);
        assert_eq!(fs::read(&projection.exact_capture).unwrap(), body);
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| p.as_str() == "/v1/responses")
                .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retained_capture_identity_and_original_limit_failure_are_truthful() {
        let root = temp("capture-limit");
        let body = vec![b'x'; 128 * 1024];
        let mut settings = metadata();
        settings["test_response_kind"] = json!("text/plain");
        let (addr, der, server) = tls_peer(settings, body, 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime().block_on(host.list_models()).unwrap();
        let projection = runtime()
            .block_on(host.smoke(&grant(&host, model_sha()), &root.join("work")))
            .unwrap();
        let saved = fs::read(&projection.exact_capture).unwrap();
        assert_eq!(saved.len(), projection.response_bytes);
        assert_eq!(
            projection.response_sha256,
            format!("{:x}", Sha256::digest(&saved))
        );
        assert!(projection.received_bytes > projection.response_bytes);
        assert!(!projection.full_capture_complete);
        let record = AttemptJournal::open(root.join("work/attempt-attempt-one.json")).unwrap();
        assert_eq!(
            record.record().observation.as_ref().unwrap().local_failure,
            Some(StreamError::StreamTooLarge)
        );
        assert!(root.join("work/local-closure-attempt-one.json").exists());
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn public_siwc_profile_accepts_coherent_sparse_stream_and_quarantines_missing_witness() {
        let fixture = include_str!("../tests/fixtures/synthetic-siwc-sparse-response.sse");
        for invalid in [false, true] {
            let root = temp("siwc-sparse");
            let mut settings = metadata();
            settings["test_response_no_content_type"] = json!(true);
            let body = if invalid {
                fixture
                    .split("\n\n")
                    .filter(|frame| !frame.contains("response.output_text.done"))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            } else {
                fixture.to_owned()
            };
            let (addr, der, server) = tls_peer(settings, body.into_bytes(), 4);
            let host = seeded_host(addr, &der, &root.join("auth"));
            runtime().block_on(host.list_models()).unwrap();
            let work = root.join("work");
            let result = runtime()
                .block_on(host.smoke(&grant(&host, model_sha()), &work))
                .unwrap();
            let journal = AttemptJournal::open(work.join("attempt-attempt-one.json")).unwrap();
            assert_eq!(
                journal.record().profile_revision,
                StreamObserver::SIWC_PROFILE_REVISION
            );
            assert_eq!(result.quarantined, invalid);
            assert_eq!(
                result.output_matches_fixture,
                if invalid { None } else { Some(true) }
            );
            assert_eq!(
                result.state,
                if invalid {
                    AttemptState::OutcomeUnknown
                } else {
                    AttemptState::TerminalObserved
                }
            );
            server.join().unwrap();
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn missing_content_type_requires_a_valid_bounded_stream() {
        let no_terminal = b"data: {\"type\":\"response.created\",\"sequence_number\":0,\"response\":{\"id\":\"r\",\"model\":\"fake-model\",\"status\":\"in_progress\"}}\n\n".to_vec();
        for (label, status, body, expected_failure) in [
            ("valid", "200 OK", stream(), None),
            (
                "json",
                "200 OK",
                br#"{"error":"synthetic"}"#.to_vec(),
                Some("MalformedEvent"),
            ),
            ("empty", "200 OK", Vec::new(), Some("MissingTerminal")),
            (
                "no-terminal",
                "200 OK",
                no_terminal,
                Some("MissingTerminal"),
            ),
            (
                "wrong-model",
                "200 OK",
                String::from_utf8(stream())
                    .unwrap()
                    .replace("fake-model", "other-model")
                    .into_bytes(),
                Some("ConflictingIdentity"),
            ),
            (
                "oversized-frame",
                "200 OK",
                vec![b'x'; 128 * 1024 + 1],
                Some("FrameTooLarge"),
            ),
            (
                "wrong-status",
                "429 Too Many Requests",
                stream(),
                Some("HttpFailure"),
            ),
        ] {
            let root = temp(label);
            let mut settings = metadata();
            settings["test_response_no_content_type"] = json!(true);
            settings["test_response_status"] = json!(status);
            let (addr, der, server) = tls_peer(settings, body, 4);
            let host = seeded_host(addr, &der, &root.join("auth"));
            runtime().block_on(host.list_models()).unwrap();
            let work = root.join("work");
            let projection = runtime()
                .block_on(host.smoke(&grant(&host, model_sha()), &work))
                .unwrap();
            let record = AttemptJournal::open(work.join("attempt-attempt-one.json")).unwrap();
            let http = record.record().http.as_ref().unwrap();
            assert!(http.content_type.is_none(), "{label}");
            let meta = http.content_type_meta.as_ref().unwrap();
            assert_eq!(meta.capture, ContentTypeCapture::Missing, "{label}");
            assert!(!meta.event_stream_prefix, "{label}");
            assert_eq!(projection.content_type_meta.as_ref(), Some(meta));
            assert_eq!(
                projection.failure_category.as_deref(),
                expected_failure,
                "{label}"
            );
            assert_eq!(
                projection.quarantined,
                expected_failure.is_some(),
                "{label}"
            );
            assert_eq!(
                projection.state,
                if expected_failure.is_some() {
                    AttemptState::OutcomeUnknown
                } else {
                    AttemptState::TerminalObserved
                },
                "{label}"
            );
            assert_eq!(
                projection.output_matches_fixture,
                if expected_failure.is_some() {
                    None
                } else {
                    Some(true)
                },
                "{label}"
            );
            assert!(projection.funding_obligation_open && projection.local_transport_closed);
            server.join().unwrap();
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn content_type_diagnostics_preserve_predicate_and_suppress_unsafe_values() {
        let (value, meta) = observe_content_type(None, "synthetic-access");
        assert!(value.is_none());
        assert_eq!(meta.capture, ContentTypeCapture::Missing);
        assert!(!meta.event_stream_prefix);
        for (raw, capture, prefix, retained) in [
            (
                b"Text/Event-Stream; charset=utf-8".to_vec(),
                ContentTypeCapture::Captured,
                false,
                true,
            ),
            (
                b"text/event-stream; charset=utf-8".to_vec(),
                ContentTypeCapture::Captured,
                true,
                true,
            ),
            (
                b"text/\xff".to_vec(),
                ContentTypeCapture::NonText,
                false,
                false,
            ),
            (
                b"text/event-stream; synthetic-access".to_vec(),
                ContentTypeCapture::Redacted,
                true,
                false,
            ),
            (
                br#""synthetic-\u0061ccess""#.to_vec(),
                ContentTypeCapture::Redacted,
                false,
                false,
            ),
            (
                format!("text/event-stream;{}", "x".repeat(CONTENT_TYPE_VALUE_LIMIT)).into_bytes(),
                ContentTypeCapture::Oversized,
                true,
                false,
            ),
        ] {
            let header = HeaderValue::from_bytes(&raw).unwrap();
            let (value, meta) = observe_content_type(Some(&header), "synthetic-access");
            assert_eq!(meta.capture, capture);
            assert_eq!(meta.event_stream_prefix, prefix);
            assert_eq!(value.is_some(), retained);
            if capture == ContentTypeCapture::Redacted {
                assert!(meta.bytes.is_none());
            }
            if let Some(value) = value {
                assert_eq!(value.as_bytes(), raw);
            }
        }
    }

    #[test]
    fn oversized_content_type_diagnostic_does_not_reject_an_accepted_stream() {
        let root = temp("oversized-content-type");
        let mut settings = metadata();
        settings["test_response_kind"] = json!(format!(
            "text/event-stream;{}",
            "x".repeat(CONTENT_TYPE_VALUE_LIMIT)
        ));
        let (addr, der, server) = tls_peer(settings, stream(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime().block_on(host.list_models()).unwrap();
        let projection = runtime()
            .block_on(host.smoke(&grant(&host, model_sha()), &root.join("work")))
            .unwrap();
        assert_eq!(projection.state, AttemptState::TerminalObserved);
        assert_eq!(projection.output_matches_fixture, Some(true));
        assert_eq!(
            projection.content_type_meta.as_ref().unwrap().capture,
            ContentTypeCapture::Oversized
        );
        assert!(
            projection
                .content_type_meta
                .as_ref()
                .unwrap()
                .event_stream_prefix
        );
        let record = AttemptJournal::open(root.join("work/attempt-attempt-one.json")).unwrap();
        assert!(
            record
                .record()
                .http
                .as_ref()
                .unwrap()
                .content_type
                .is_none()
        );
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejected_http_responses_preserve_status_body_identity_and_local_closure() {
        for (label, status, kind) in [
            ("401", "401 Unauthorized", "application/json"),
            ("403", "403 Forbidden", "application/json"),
            ("429", "429 Too Many Requests", "application/json"),
            ("500", "500 Internal Server Error", "application/json"),
            ("wrong-type", "200 OK", "text/plain"),
        ] {
            let root = temp(label);
            let mut settings = metadata();
            settings["test_response_status"] = json!(status);
            settings["test_response_kind"] = json!(kind);
            let body = br#"{"error":{"code":"synthetic_rejection","message":"denied"}}"#.to_vec();
            let (addr, der, server) = tls_peer(settings, body.clone(), 4);
            let host = seeded_host(addr, &der, &root.join("auth"));
            runtime().block_on(host.list_models()).unwrap();
            let projection = runtime()
                .block_on(host.smoke(&grant(&host, model_sha()), &root.join("work")))
                .unwrap();
            let record = AttemptJournal::open(root.join("work/attempt-attempt-one.json")).unwrap();
            let http = record.record().http.as_ref().unwrap();
            assert_eq!(http.status, projection.http_status);
            assert_eq!(http.content_type.as_deref(), Some(kind));
            let meta = http.content_type_meta.as_ref().unwrap();
            assert_eq!(meta.capture, ContentTypeCapture::Captured);
            assert!(!meta.event_stream_prefix);
            assert_eq!(projection.content_type_meta.as_ref(), Some(meta));
            assert!(
                serde_json::to_value(&projection)
                    .unwrap()
                    .get("content_type")
                    .is_none()
            );
            assert_eq!(http.body_bytes as usize, body.len());
            assert_eq!(
                http.body_sha256.as_deref(),
                Some(projection.response_sha256.as_str())
            );
            assert!(matches!(http.error_shape, Some(HttpErrorShape::Structured)));
            assert_eq!(fs::read(&projection.exact_capture).unwrap(), body);
            assert_eq!(projection.state, AttemptState::OutcomeUnknown);
            assert_eq!(projection.failure_category.as_deref(), Some("HttpFailure"));
            assert!(root.join("work/local-closure-attempt-one.json").exists());
            assert!(projection.funding_obligation_open);
            server.join().unwrap();
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn interrupted_capture_write_keeps_unknown_prefix_and_open_remote_obligation() {
        let root = temp("capture-write-fault");
        let (addr, der, server) = tls_peer(metadata(), stream(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime().block_on(host.list_models()).unwrap();
        let mut grant = grant(&host, model_sha());
        grant.release_id = "test-capture-write-failure".into();
        let work = root.join("work");
        assert!(runtime().block_on(host.smoke(&grant, &work)).is_err());
        let status: serde_json::Value = serde_json::from_slice(
            &fs::read(work.join("capture-status-attempt-one.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(status["full_capture_complete"], false);
        assert_eq!(status["unknown_retained_prefix"], true);
        assert!(work.join("quarantine-attempt-one.bin").exists());
        assert_eq!(
            AttemptJournal::open(work.join("attempt-attempt-one.json"))
                .unwrap()
                .record()
                .state,
            AttemptState::PossiblyEntered
        );
        assert!(work.join("local-closure-attempt-one.json").exists());
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn transport_failure_before_headers_keeps_unknown_and_local_closure() {
        let root = temp("network-failure");
        let (addr, der, server) = tls_peer(metadata(), vec![], 3);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime().block_on(host.list_models()).unwrap();
        server.join().unwrap();
        let work = root.join("work");
        let projection = runtime()
            .block_on(host.smoke(&grant(&host, model_sha()), &work))
            .unwrap();
        assert_eq!(projection.state, AttemptState::OutcomeUnknown);
        assert_eq!(projection.http_status, None);
        assert_eq!(
            projection.failure_category.as_deref(),
            Some("TransportFailure")
        );
        assert!(!projection.full_capture_complete);
        assert!(work.join("local-closure-attempt-one.json").exists());
        assert!(projection.funding_obligation_open);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn callback_host_path_and_size_are_checked_on_real_socket() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let redirect = format!("http://127.0.0.1:{}/auth/callback", addr.port());
        let handle = thread::spawn(move || {
            let mut socket = std::net::TcpStream::connect(addr).unwrap();
            socket
                .write_all(
                    b"GET /auth/callback?state=x&code=y HTTP/1.1\r\nHost: evil.invalid\r\n\r\n",
                )
                .unwrap();
        });
        assert!(
            runtime()
                .block_on(receive_callback_async(
                    listener,
                    &redirect,
                    Duration::from_millis(500)
                ))
                .is_err()
        );
        handle.join().unwrap();
    }

    #[test]
    fn slow_drip_callback_cannot_extend_absolute_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let redirect = format!("http://127.0.0.1:{}/auth/callback", addr.port());
        let writer = thread::spawn(move || {
            let mut socket = std::net::TcpStream::connect(addr).unwrap();
            for byte in b"GET /auth/callback?state=x&code=y HTTP/1.1\r\nHost: 127.0.0.1:1\r\n\r\n" {
                if socket.write_all(&[*byte]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(30));
            }
        });
        let started = Instant::now();
        assert!(
            runtime()
                .block_on(receive_callback_async(
                    listener,
                    &redirect,
                    Duration::from_millis(100)
                ))
                .is_err()
        );
        assert!(started.elapsed() < Duration::from_millis(350));
        writer.join().unwrap();
    }

    #[test]
    fn https_headers_and_body_share_one_deadline() {
        let mut settings = metadata();
        settings["test_split_models_ms"] = json!(80);
        let (addr, der, server) = tls_peer(settings, vec![], 1);
        let https = PublicHttps::synthetic(&der, addr);
        let started = Instant::now();
        let result = runtime().block_on(https.get_bounded(
            "https://api.openai.com/v1/models",
            128 * 1024,
            Duration::from_millis(120),
        ));
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_millis(220));
        assert_eq!(server.join().unwrap(), vec!["/v1/models".to_string()]);
    }

    #[test]
    fn action_admission_binds_executable_source_command_paths_and_single_use() {
        let root = temp("action-admission");
        fs::create_dir(&root).unwrap();
        let credential_dir = root.join("credentials");
        prepare_private_dir(&credential_dir).unwrap();
        let work_dir = root.join("work");
        let issued = now();
        let mut grant = ActionGrant {
            release_id: "one-action".into(),
            actor: "synthetic-owner".into(),
            custody_owner: "synthetic-account-owner".into(),
            custody_custodian: "synthetic-token-custodian".into(),
            custody_contract_ref: "synthetic-custody-contract".into(),
            retention_purpose: "bounded public host auth and model inspection".into(),
            cleanup_disposition: "retain-until-owner-disconnect".into(),
            action: "smoke".into(),
            credential_dir: credential_dir.clone(),
            host_id: "synthetic_host".into(),
            work_dir: Some(work_dir.clone()),
            smoke_grant_sha256: Some(format!("{:x}", Sha256::digest(b"synthetic-grant"))),
            refresh_registration: None,
            executable_sha256: ActionGrant::executable_sha256().unwrap(),
            source_sha256: ActionGrant::source_sha256(),
            issued_at_unix: issued,
            valid_until_unix: issued + 45,
        };
        assert!(
            grant
                .validate(
                    "smoke",
                    &credential_dir,
                    "synthetic_host",
                    Some(&work_dir),
                    Some(b"synthetic-grant")
                )
                .is_ok()
        );
        let contract = CredentialCustody {
            owner: grant.custody_owner.clone(),
            custodian: grant.custody_custodian.clone(),
            contract_ref: grant.custody_contract_ref.clone(),
            retention_purpose: grant.retention_purpose.clone(),
        };
        persist_new(
            &credential_dir.join("credential-custody-contract.json"),
            &serde_json::to_vec(&contract).unwrap(),
        )
        .unwrap();
        assert!(
            grant
                .validate("models", &credential_dir, "synthetic_host", None, None)
                .is_err()
        );
        assert!(
            grant
                .validate(
                    "smoke",
                    &credential_dir,
                    "synthetic_host",
                    Some(&work_dir),
                    Some(b"changed")
                )
                .is_err()
        );
        grant.executable_sha256 = "0".repeat(64);
        assert!(
            grant
                .validate(
                    "smoke",
                    &credential_dir,
                    "synthetic_host",
                    Some(&work_dir),
                    Some(b"synthetic-grant")
                )
                .is_err()
        );
        grant.executable_sha256 = ActionGrant::executable_sha256().unwrap();
        grant.consume().unwrap();
        assert!(grant.consume().is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_action_causes_zero_tls_calls_across_local_open_and_host_method() {
        let root = temp("zero-entry");
        fs::create_dir(&root).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec![
            "auth.openai.com".into(),
            "api.openai.com".into(),
        ])
        .unwrap();
        let https =
            PublicHttps::synthetic(cert.cert.der().as_ref(), listener.local_addr().unwrap());
        let host = PublicHost::synthetic_local(https, &root.join("auth"), "synthetic_host_public")
            .unwrap();
        let grant = action_for(&host, "disconnect", "wrong-action");
        assert!(runtime().block_on(host.models_admitted(&grant)).is_err());
        assert!(listener.accept().is_err());
        assert!(!root.join("auth/action-wrong-action.json").exists());
        let mut sign_in = action_for(&host, "sign-in", "no-custody-sign-in");
        sign_in.custody_contract_ref.clear();
        assert!(matches!(
            runtime().block_on(host.sign_in_admitted(&sign_in, "Synthetic", |_| Ok(()))),
            Err(HostError::Policy)
        ));
        assert!(!root.join("auth/action-no-custody-sign-in.json").exists());
        let mut models = action_for(&host, "models", "no-retention-models");
        models.retention_purpose.clear();
        assert!(matches!(
            runtime().block_on(host.models_admitted(&models)),
            Err(HostError::Policy)
        ));
        assert!(listener.accept().is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn funding_and_selection_fail_before_any_network_entry() {
        let root = temp("fence");
        let (addr, der, server) = tls_peer(metadata(), vec![], 2);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let mut grant = grant(&host, model_sha());
        grant.credit_use_disabled = false;
        assert!(
            runtime()
                .block_on(host.smoke(&grant, &root.join("work")))
                .is_err()
        );
        grant.credit_use_disabled = true;
        grant.credential_revision += 1;
        assert!(
            runtime()
                .block_on(host.smoke(&grant, &root.join("work")))
                .is_err()
        );
        assert_eq!(server.join().unwrap().len(), 2);
        assert!(!root.join("work").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn quota_failure_after_text_preserves_terminal_and_usage_without_output() {
        let body = [
            event(json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-quota","model":"fake-model"}})),
            event(json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"item-1","type":"message"}})),
            event(json!({"type":"response.output_text.delta","sequence_number":3,"output_index":0,"content_index":0,"item_id":"item-1","delta":"partial"})),
            event(json!({"type":"response.failed","sequence_number":4,"response":{"id":"resp-quota","model":"fake-model","status":"failed","error":{"code":"subscription_sharing_usage_limit_exceeded"},"usage":{"input_tokens":5}}})),
        ].concat().into_bytes();
        let root = temp("quota");
        let (addr, der, server) = tls_peer(metadata(), body, 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime().block_on(host.list_models()).unwrap();
        let result = runtime()
            .block_on(host.smoke(&grant(&host, model_sha()), &root.join("work")))
            .unwrap();
        assert_eq!(result.state, AttemptState::TerminalObserved);
        assert!(matches!(
            result.terminal,
            Some(ObservedTerminal::Failed { .. })
        ));
        assert_eq!(result.output_matches_fixture, None);
        let attempt = AttemptJournal::open(root.join("work/attempt-attempt-one.json")).unwrap();
        assert_eq!(
            attempt.record().observation.as_ref().unwrap().usage[0]
                .counts
                .input,
            Some(5)
        );
        assert!(root.join("work/funding-release-one.json").exists());
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_terminal_and_secret_echo_keep_unknown_and_quarantine() {
        for (label, body, secret_echo) in [
            ("missing", event(json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-missing","model":"fake-model"}})).into_bytes(), false),
            ("secret", {
                let mut bytes = stream();
                bytes.extend_from_slice(b"synthetic-access");
                bytes
            }, true),
        ] {
            let root = temp(label);
            let (addr, der, server) = tls_peer(metadata(), body, 4);
            let host = seeded_host(addr, &der, &root.join("auth"));
            runtime().block_on(host.list_models()).unwrap();
            let result = runtime().block_on(host.smoke(&grant(&host, model_sha()), &root.join("work"))).unwrap();
            assert_eq!(result.state, AttemptState::OutcomeUnknown);
            assert!(result.quarantined);
            assert!(result.funding_obligation_open);
            assert_eq!(Path::new(&result.exact_capture).exists(), true);
            let attempt = AttemptJournal::open(root.join("work/attempt-attempt-one.json")).unwrap();
            assert!(attempt.record().remote_outcome_unknown());
            assert!(result.exact_capture.contains("quarantine-"));
            let evidence = attempt.record().observation.as_ref().unwrap();
            if secret_echo {
                assert_eq!(evidence.response_id, None);
                assert_eq!(evidence.local_failure, Some(StreamError::CredentialEcho));
            } else {
                assert_eq!(evidence.response_id.as_deref(), Some("resp-missing"));
                assert_eq!(evidence.local_failure, Some(StreamError::MissingTerminal));
            }
            server.join().unwrap();
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn evidence_path_failure_prevents_response_send() {
        let root = temp("evidence");
        let (addr, der, server) = tls_peer(metadata(), vec![], 3);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime().block_on(host.list_models()).unwrap();
        let work = root.join("work");
        prepare_private_dir(&work).unwrap();
        fs::write(work.join("funding-release-one.json"), b"occupied").unwrap();
        assert!(
            runtime()
                .block_on(host.smoke(&grant(&host, model_sha()), &work))
                .is_err()
        );
        let paths = server.join().unwrap();
        assert!(!paths.iter().any(|p| p == "/v1/responses"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn model_selection_rejects_changed_protected_response_without_post() {
        let root = temp("model-binding");
        let (addr, der, server) = tls_peer(metadata(), vec![], 3);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let observation = runtime().block_on(host.list_models()).unwrap();
        assert_eq!(observation.response_sha256, model_sha());
        let raw_path = root
            .join("auth")
            .join(format!("model-response-{}.bin", model_sha()));
        fs::write(&raw_path, b"{}").unwrap();
        assert!(
            runtime()
                .block_on(host.smoke(&grant(&host, model_sha()), &root.join("work")))
                .is_err()
        );
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| p.as_str() == "/v1/responses")
                .count(),
            0
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn catalog_at_body_limit_is_retained_and_usable_for_selection() {
        let root = temp("large-model-catalog");
        let mut body = json!({"models":[{"slug":"fake-model","visibility":"list"}]}).to_string();
        let mut config = metadata();
        config["test_models_body"] = json!(body);
        config["test_models_padding_bytes"] = json!(crate::public_https::MODEL_BODY_LIMIT);
        body.extend(
            std::iter::repeat(' ').take(crate::public_https::MODEL_BODY_LIMIT - body.len()),
        );
        let expected_sha = format!("{:x}", Sha256::digest(body.as_bytes()));
        let (addr, der, server) = tls_peer(config, stream(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let action = action_for(&host, "models", "large-models-once");
        let observation = runtime().block_on(host.models_admitted(&action)).unwrap();
        assert_eq!(observation.response_bytes, 2 * 1024 * 1024);
        assert_eq!(observation.response_sha256, expected_sha);
        assert_eq!(observation.models.len(), 1);
        let retained = fs::read(
            root.join("auth")
                .join(format!("model-response-{expected_sha}.bin")),
        )
        .unwrap();
        assert_eq!(retained, body.as_bytes());
        assert!(runtime().block_on(host.models_admitted(&action)).is_err());
        let projection = runtime()
            .block_on(host.smoke(&grant(&host, expected_sha), &root.join("work")))
            .unwrap();
        assert_eq!(projection.output_matches_fixture, Some(true));
        let paths = server.join().unwrap();
        assert_eq!(
            paths.iter().filter(|p| p.as_str() == "/v1/models").count(),
            1
        );
        assert_eq!(
            paths
                .iter()
                .filter(|p| p.as_str() == "/v1/responses")
                .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn model_failure_categories_are_safe_and_do_not_create_observations() {
        let cases = [
            (
                "http",
                json!({"test_models_status":"403 Forbidden", "test_models_body":"secret-marker"}),
            ),
            ("json", json!({"test_models_body":"secret-marker{"})),
            (
                "schema",
                json!({"test_models_body":"{\"models\":[{\"slug\":\"secret-marker\"}]}"}),
            ),
            ("header", json!({"test_models_large_header":true})),
            ("body", json!({"test_models_body_large":true})),
            ("observation", json!({"test_models_escaped_entries":true})),
            (
                "count",
                json!({"test_models_body":json!({"models":vec![json!({"slug":"fake-model","visibility":"list"}); 513]}).to_string()}),
            ),
        ];
        for (label, controls) in cases {
            let root = temp(label);
            let mut config = metadata();
            for (key, value) in controls.as_object().unwrap() {
                config[key] = value.clone();
            }
            let (addr, der, server) = tls_peer(config, vec![], 3);
            let host = seeded_host(addr, &der, &root.join("auth"));
            let action = action_for(&host, "models", "model-error-once");
            let error = runtime()
                .block_on(host.models_admitted(&action))
                .unwrap_err();
            assert!(
                match label {
                    "http" => matches!(error, HostError::Https(PublicHttpsError::HttpStatus(403))),
                    "json" => matches!(error, HostError::Https(PublicHttpsError::ModelJson)),
                    "schema" => matches!(error, HostError::Https(PublicHttpsError::ModelSchema)),
                    "header" => matches!(error, HostError::Https(PublicHttpsError::HeaderBound)),
                    "body" => matches!(error, HostError::Https(PublicHttpsError::BodyBound)),
                    "count" => matches!(error, HostError::Https(PublicHttpsError::Bound)),
                    "observation" => matches!(error, HostError::ModelObservationBound),
                    _ => unreachable!(),
                },
                "{label}: {error:?}"
            );
            assert!(!format!("{error:?} {error}").contains("secret-marker"));
            assert!(fs::read_dir(root.join("auth")).unwrap().all(|entry| {
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("model-")
            }));
            assert!(runtime().block_on(host.models_admitted(&action)).is_err());
            assert_eq!(
                server
                    .join()
                    .unwrap()
                    .iter()
                    .filter(|p| p.as_str() == "/v1/models")
                    .count(),
                1
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn model_transport_and_deadline_are_distinct_without_observation() {
        let root = temp("model-transport");
        let (addr, der, server) = tls_peer(metadata(), vec![], 2);
        let host = seeded_host(addr, &der, &root.join("auth"));
        assert_eq!(server.join().unwrap().len(), 2);
        let action = action_for(&host, "models", "transport-once");
        assert!(matches!(
            runtime().block_on(host.models_admitted(&action)),
            Err(HostError::Https(PublicHttpsError::Transport))
        ));
        fs::remove_dir_all(root).unwrap();

        let root = temp("model-deadline");
        let mut config = metadata();
        config["test_split_models_ms"] = json!(2_600);
        let (addr, der, server) = tls_peer(config, vec![], 3);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let action = action_for(&host, "models", "deadline-once");
        assert!(matches!(
            runtime().block_on(host.models_admitted(&action)),
            Err(HostError::Https(PublicHttpsError::Deadline))
        ));
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| p.as_str() == "/v1/models")
                .count(),
            1
        );
        assert!(fs::read_dir(root.join("auth")).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("model-observation-")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stalled_tls_handshake_is_deadline_and_closed_peer_is_transport() {
        let root = temp("model-handshake-deadline");
        let (addr, der, discovery_peer) = tls_peer(metadata(), vec![], 2);
        let host = seeded_host(addr, &der, &root.join("auth"));
        assert_eq!(discovery_peer.join().unwrap().len(), 2);
        let selected = host.selected().unwrap();
        let bearer = host.store.bearer(&selected, now()).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let stalled_addr = listener.local_addr().unwrap();
        let stalled_peer = thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(3_400));
            drop(socket);
        });
        let https = PublicHttps::synthetic(&der, stalled_addr);
        let started = Instant::now();
        let error = runtime().block_on(https.models(&bearer)).err().unwrap();
        assert!(matches!(error, PublicHttpsError::Deadline), "{error:?}");
        assert!(started.elapsed() < Duration::from_secs(5));
        stalled_peer.join().unwrap();

        let closed = TcpListener::bind("127.0.0.1:0").unwrap();
        let closed_addr = closed.local_addr().unwrap();
        drop(closed);
        let https = PublicHttps::synthetic(&der, closed_addr);
        assert!(matches!(
            runtime().block_on(https.models(&bearer)),
            Err(PublicHttpsError::Transport)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn admitted_model_action_makes_exactly_one_get_and_consumes_release() {
        let root = temp("admitted-models");
        let (addr, der, server) = tls_peer(metadata(), vec![], 3);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let action = action_for(&host, "models", "models-once");
        let observation = runtime().block_on(host.models_admitted(&action)).unwrap();
        assert_eq!(observation.response_sha256, model_sha());
        assert_eq!(observation.registration, host.selected().unwrap());
        assert!(runtime().block_on(host.models_admitted(&action)).is_err());
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| p.as_str() == "/v1/models")
                .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disconnect_closes_local_use_and_records_remote_revocation_result() {
        let root = temp("disconnect");
        let (addr, der, server) = tls_peer(metadata(), vec![], 3);
        let host = seeded_host(addr, &der, &root.join("auth"));
        assert!(runtime().block_on(host.disconnect()).unwrap());
        assert!(host.selected().is_err());
        assert!(
            server
                .join()
                .unwrap()
                .contains(&"/api/accounts/oauth/revoke".to_owned())
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disconnect_disables_local_use_when_issuer_peer_is_gone() {
        let root = temp("disconnect-outage");
        let (addr, der, server) = tls_peer(metadata(), vec![], 2);
        let host = seeded_host(addr, &der, &root.join("auth"));
        server.join().unwrap();
        let action = action_for(&host, "disconnect", "disconnect-outage");
        assert!(
            !runtime()
                .block_on(host.disconnect_admitted(&action))
                .unwrap()
        );
        assert!(host.selected().is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loopback_callback_pkce_exchange_and_jwks_reach_auth_store() {
        let root = temp("signin");
        fs::create_dir(&root).unwrap();
        let auth = Arc::new(Mutex::new(None));
        let (addr, der, server) = tls_peer_auth(metadata(), vec![], 5, Some(auth.clone()));
        let host = PublicHost::synthetic(
            PublicHttps::synthetic(&der, addr),
            &root.join("auth"),
            "synthetic_host_signin",
        )
        .unwrap();
        let mut callback = None;
        let registration = runtime().block_on(host.sign_in_new("Synthetic", |raw| {
            let url = url::Url::parse(raw).unwrap();
            let param = |name: &str| url.query_pairs().find(|(k, _)| k == name).unwrap().1.to_string();
            let state = param("state");
            *auth.lock().unwrap() = Some((param("nonce"), param("code_challenge")));
            let redirect = url::Url::parse(&param("redirect_uri")).unwrap();
            assert_eq!(redirect.host_str(), Some("127.0.0.1"));
            let port = redirect.port().unwrap();
            callback = Some(thread::spawn(move || {
                let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
                let request = format!("GET /auth/callback?state={state}&code=synthetic-code&client_id=synthetic-client HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n");
                socket.write_all(request.as_bytes()).unwrap();
                let mut response = [0u8; 256];
                assert!(socket.read(&mut response).unwrap() > 0);
            }));
            Ok(())
        })).unwrap();
        callback.unwrap().join().unwrap();
        assert_eq!(registration.client_id, "synthetic-client");
        assert_eq!(host.selected().unwrap(), registration);
        assert_eq!(
            server.join().unwrap().last().unwrap(),
            "/api/accounts/oauth/token"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn admitted_grant_object_mismatch_fences_post() {
        let root = temp("grant-object-mismatch");
        let (addr, der, server) = tls_peer(metadata(), stream(), 3);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "models-one")))
            .unwrap();
        let mut authorized = grant(&host, model_sha());
        let mut executed = grant(&host, model_sha());
        for variant in 0..3 {
            authorized.release_id = format!("release-authorized-{variant}");
            executed.release_id = authorized.release_id.clone();
            match variant {
                0 => {
                    authorized.credit_use_disabled = false;
                    executed.credit_use_disabled = true;
                }
                1 => {
                    authorized.credit_use_disabled = true;
                    executed.selected_model = "different-model".into();
                }
                _ => {
                    executed.selected_model = authorized.selected_model.clone();
                    executed.attempt_id = "different-attempt".into();
                }
            }
            let bytes = serde_json::to_vec(&authorized).unwrap();
            let mut action = action_for(&host, "smoke", &format!("smoke-{variant}"));
            let work = root.join(format!("work-{variant}"));
            action.work_dir = Some(work.clone());
            action.smoke_grant_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
            action.valid_until_unix = action.issued_at_unix + 45;
            assert!(matches!(
                runtime().block_on(host.smoke_admitted(&action, &executed, &bytes, &work)),
                Err(HostError::Policy)
            ));
        }
        let invalid_bytes = b"{}";
        let mut invalid_action = action_for(&host, "smoke", "invalid-bound-grant");
        let invalid_work = root.join("invalid-work");
        invalid_action.work_dir = Some(invalid_work.clone());
        invalid_action.smoke_grant_sha256 = Some(format!("{:x}", Sha256::digest(invalid_bytes)));
        invalid_action.valid_until_unix = invalid_action.issued_at_unix + 45;
        assert!(matches!(
            runtime().block_on(host.smoke_admitted(
                &invalid_action,
                &executed,
                invalid_bytes,
                &invalid_work
            )),
            Err(HostError::Policy)
        ));
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| *p == "/v1/responses")
                .count(),
            0
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn diagnostic_prepared() -> RealDiagnosticPrepared {
        serde_json::from_slice(include_bytes!("../tests/fixtures/synthetic-real-diagnostic-q01-prepared.json")).unwrap()
    }

    fn diagnostic_stream() -> Vec<u8> {
        let material: serde_json::Value = serde_json::from_slice(include_bytes!("../tests/fixtures/synthetic-real-diagnostic-q01-material.json")).unwrap();
        let report = json!({"whole_response_assessed":true,"findings":[{
            "obligation_id":material["obligations"][0]["id"],"status":"satisfied",
            "sources":material["obligations"][0]["sources"],"explanation":"whole plan checked"}],
            "critical_errors":[]}).to_string();
        diagnostic_stream_with_report(report)
    }
    fn diagnostic_stream_with_report(report: String) -> Vec<u8> {
        let events = [
            json!({"type":"response.created","sequence_number":0,"response":{"id":"response-diagnostic","model":"gpt-6-astra","status":"in_progress","output":[]}}),
            json!({"type":"response.in_progress","sequence_number":1,"response":{"id":"response-diagnostic","model":"gpt-6-astra","status":"in_progress","output":[]}}),
            json!({"type":"response.output_item.added","sequence_number":2,"output_index":0,"item":{"id":"item-diagnostic","type":"message","role":"assistant","status":"in_progress","content":[]}}),
            json!({"type":"response.content_part.added","sequence_number":3,"output_index":0,"content_index":0,"item_id":"item-diagnostic","part":{"type":"output_text","text":""}}),
            json!({"type":"response.output_text.delta","sequence_number":4,"output_index":0,"content_index":0,"item_id":"item-diagnostic","delta":report}),
            json!({"type":"response.output_text.done","sequence_number":5,"output_index":0,"content_index":0,"item_id":"item-diagnostic","text":report}),
            json!({"type":"response.content_part.done","sequence_number":6,"output_index":0,"content_index":0,"item_id":"item-diagnostic","part":{"type":"output_text","text":report}}),
            json!({"type":"response.output_item.done","sequence_number":7,"output_index":0,"item":{"id":"item-diagnostic","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":report}]}}),
            json!({"type":"response.completed","sequence_number":8,"response":{"id":"response-diagnostic","model":"gpt-6-astra","status":"completed","output":[]}}),
        ];
        events.into_iter().map(|v|format!("data: {v}\n\n")).collect::<String>().into_bytes()
    }
    fn diagnostic_provider_incomplete() -> Vec<u8> {
        [json!({"type":"response.created","sequence_number":0,"response":{"id":"response-diagnostic","model":"gpt-6-astra","status":"in_progress","output":[]}}),
         json!({"type":"response.incomplete","sequence_number":1,"response":{"id":"response-diagnostic","model":"gpt-6-astra","status":"incomplete","incomplete_details":{"reason":"limit"}}})]
            .into_iter().map(|v|format!("data: {v}\n\n")).collect::<String>().into_bytes()
    }
    fn diagnostic_completed_without_output() -> Vec<u8> {
        [json!({"type":"response.created","sequence_number":0,"response":{"id":"response-diagnostic","model":"gpt-6-astra","status":"in_progress","output":[]}}),
         json!({"type":"response.completed","sequence_number":1,"response":{"id":"response-diagnostic","model":"gpt-6-astra","status":"completed","output":[]}})]
            .into_iter().map(|v|format!("data: {v}\n\n")).collect::<String>().into_bytes()
    }

    fn diagnostic_metadata() -> serde_json::Value {
        let mut meta = metadata();
        meta["test_diagnostic"] = json!(true);
        meta["test_models_body"] = json!(r#"{"models":[{"slug":"gpt-6-astra","visibility":"list"}]}"#);
        meta
    }

    fn diagnostic_grant(host: &PublicHost, prepared: &RealDiagnosticPrepared, model_sha: String) -> DiagnosticGrant {
        let mut smoke = grant(host, model_sha);
        smoke.release_id = "diagnostic-release".into();
        smoke.operation_id = prepared.request.binding.operation_id.clone();
        smoke.attempt_id = prepared.request.binding.attempt_id.clone();
        smoke.selected_model = prepared.requested_model.clone();
        DiagnosticGrant { smoke, prepared_binding_sha256:prepared.binding_sha256.clone(),
            prepared_wire_sha256:prepared.wire_sha256.clone(),
            semantic_prompt_sha256:prepared.request.semantic_prompt_sha256.clone(),
            source_basis_sha256:prepared.request.source_basis_sha256.clone(),
            candidate_sha256:prepared.request.candidate_sha256.clone(),
            exposure_id:prepared.request.binding.exposure_id.clone(),
            profile_revision:prepared.profile_revision.clone() }
    }

    #[test]
    fn diagnostic_actual_host_admission_and_capture() {
        let root = std::env::var_os("B2_DIAGNOSTIC_FIXTURE_DIR").map(PathBuf::from)
            .unwrap_or_else(||temp("diagnostic-actual"));
        let (addr, der, server) = tls_peer(diagnostic_metadata(), diagnostic_stream(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let model = runtime().block_on(host.models_admitted(&action_for(&host,"models","diagnostic-models"))).unwrap();
        let prepared = diagnostic_prepared();
        let mut grant = diagnostic_grant(&host,&prepared,model.response_sha256);
        let work = root.join("work");
        let grant_bytes = serde_json::to_vec(&grant).unwrap();
        let mut action = action_for(&host,"diagnostic","diagnostic-release");
        action.work_dir = Some(work.clone());
        action.smoke_grant_sha256 = Some(format!("{:x}",Sha256::digest(&grant_bytes)));
        action.valid_until_unix = action.issued_at_unix + 45;
        for field in 0..7 {
            let mut bad = grant.clone();
            match field {
                0=>bad.prepared_binding_sha256="0".repeat(64),
                1=>bad.prepared_wire_sha256="0".repeat(64),
                2=>bad.semantic_prompt_sha256="0".repeat(64),
                3=>bad.source_basis_sha256="0".repeat(64),
                4=>bad.candidate_sha256="0".repeat(64),
                5=>bad.exposure_id="wrong-exposure".into(),
                _=>bad.profile_revision="wrong-profile".into(),
            }
            let bytes=serde_json::to_vec(&bad).unwrap();
            let mut bad_action=action.clone();
            bad_action.smoke_grant_sha256=Some(format!("{:x}",Sha256::digest(&bytes)));
            assert!(matches!(runtime().block_on(host.diagnostic_admitted(&bad_action,&bytes,&prepared,&work)),Err(HostError::Policy)));
            assert!(!bad_action.action_path().unwrap().exists());
        }
        // Each refused call above re-validated the action, re-hashing the test
        // executable; under a parallel lib run that spent most of the 45 s
        // window, below the send boundary's 30 s margin. None was consumed, so
        // the valid action is issued as an owner issues it: at use.
        action.issued_at_unix = now();
        action.valid_until_unix = action.issued_at_unix + 45;
        let projection=runtime().block_on(host.diagnostic_admitted(&action,&grant_bytes,&prepared,&work)).unwrap();
        assert_eq!(projection.http_status,Some(200));
        assert_eq!(projection.terminal,Some(ObservedTerminal::Completed));
        assert!(projection.full_capture_complete);
        assert!(work.join(format!("diagnostic-admission-{}.json",grant.smoke.attempt_id)).exists());
        assert!(matches!(runtime().block_on(host.diagnostic_admitted(&action,&grant_bytes,&prepared,&work)),Err(HostError::Policy)));
        grant.smoke.release_id="collision-release".into();
        let collision_bytes=serde_json::to_vec(&grant).unwrap();
        let mut collision_action=action.clone();
        collision_action.release_id="collision-release".into();
        collision_action.smoke_grant_sha256=Some(format!("{:x}",Sha256::digest(&collision_bytes)));
        collision_action.work_dir=Some(root.join("collision"));
        assert!(runtime().block_on(host.diagnostic_admitted(&collision_action,&collision_bytes,&prepared,&root.join("collision"))).is_err());
        assert_eq!(server.join().unwrap().iter().filter(|p|p.as_str()=="/v1/responses").count(),1);
        if std::env::var_os("B2_DIAGNOSTIC_FIXTURE_DIR").is_none() { fs::remove_dir_all(root).unwrap(); }
    }

    #[test]
    fn diagnostic_actual_host_adverse_captures() {
        let base = std::env::var_os("B2_DIAGNOSTIC_ADVERSE_DIR").map(PathBuf::from)
            .unwrap_or_else(||temp("diagnostic-adverse"));
        let material:serde_json::Value=serde_json::from_slice(include_bytes!("../tests/fixtures/synthetic-real-diagnostic-q01-material.json")).unwrap();
        let mut bad_source=material["obligations"][0]["sources"][0].clone();
        bad_source["id"]=json!("fabricated-source");
        let bad_report=json!({"whole_response_assessed":true,"findings":[{
            "obligation_id":material["obligations"][0]["id"],"status":"satisfied",
            "sources":[bad_source],"explanation":"unsupported"}],"critical_errors":[]}).to_string();
        for (kind, body) in [
            ("http", diagnostic_stream()),
            ("incomplete", b"data: {\"type\":\"response.created\",\"sequence_number\":0,\"response\":{\"id\":\"response-diagnostic\",\"model\":\"gpt-6-astra\",\"status\":\"in_progress\",\"output\":[]}}\n\n".to_vec()),
            ("timeout", diagnostic_stream()),
            ("bad-citation", diagnostic_stream_with_report(bad_report)),
            ("malformed-report", diagnostic_stream_with_report("not json".into())),
            ("provider-incomplete", diagnostic_provider_incomplete()),
            ("completed-empty-output", diagnostic_completed_without_output()),
        ] {
            if std::env::var("B2_DIAGNOSTIC_SCENARIO").ok().is_some_and(|v|v!=kind) { continue; }
            let root = base.join(kind);
            let mut meta = diagnostic_metadata();
            if kind == "http" { meta["test_response_status"] = json!("503 Service Unavailable"); }
            if kind == "timeout" { meta["test_response_stall_ms"] = json!(35_000); }
            let (addr, der, server) = tls_peer(meta, body, 4);
            let host = seeded_host(addr, &der, &root.join("auth"));
            let model = runtime().block_on(host.models_admitted(&action_for(&host,"models","diagnostic-models"))).unwrap();
            let prepared = diagnostic_prepared();
            let grant = diagnostic_grant(&host,&prepared,model.response_sha256);
            let grant_bytes = serde_json::to_vec(&grant).unwrap();
            let work = root.join("work");
            let mut action = action_for(&host,"diagnostic","diagnostic-release");
            action.work_dir = Some(work.clone());
            action.smoke_grant_sha256 = Some(format!("{:x}",Sha256::digest(&grant_bytes)));
            action.valid_until_unix = action.issued_at_unix + 45;
            let projection = runtime().block_on(host.diagnostic_admitted(&action,&grant_bytes,&prepared,&work)).unwrap();
            match kind {
                "http" => assert_eq!(projection.http_status, Some(503)),
                "incomplete" => assert_ne!(projection.terminal, Some(ObservedTerminal::Completed)),
                "timeout" => assert_eq!(projection.failure_category.as_deref(), Some("Timeout")),
                "bad-citation" | "malformed-report" => assert_eq!(projection.terminal, Some(ObservedTerminal::Completed)),
                "provider-incomplete" => assert!(matches!(projection.terminal.as_ref(),Some(ObservedTerminal::Incomplete{..}))),
                "completed-empty-output" => assert_eq!(projection.terminal,Some(ObservedTerminal::Completed)),
                _ => unreachable!(),
            }
            assert_eq!(projection.quarantined, !matches!(kind,"bad-citation"|"malformed-report"|"provider-incomplete"|"completed-empty-output"));
            assert!(projection.response_bytes > 0);
            assert_eq!(server.join().unwrap().iter().filter(|p|p.as_str()=="/v1/responses").count(),1);
        }
        if std::env::var_os("B2_DIAGNOSTIC_ADVERSE_DIR").is_none() { fs::remove_dir_all(base).unwrap(); }
    }

    #[test]
    fn repeated_model_bytes_create_fresh_observation_ids() {
        let root = temp("repeat-observation");
        let (addr, der, server) = tls_peer(metadata(), vec![], 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let first = runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "models-first")))
            .unwrap();
        let second = runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "models-second")))
            .unwrap();
        assert_eq!(first.response_sha256, second.response_sha256);
        assert_ne!(first.observation_id, second.observation_id);
        assert!(
            host.observation_path(&first.observation_id)
                .unwrap()
                .exists()
        );
        assert!(
            host.observation_path(&second.observation_id)
                .unwrap()
                .exists()
        );
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| *p == "/v1/models")
                .count(),
            2
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn multiline_decoded_bearer_is_quarantined_before_journal() {
        let failed = json!({"type":"response.failed","sequence_number":2,"response":{"id":"resp-safe","model":"fake-model","status":"failed","error":{"code":"synthetic-access"}}});
        let pretty = serde_json::to_string_pretty(&failed)
            .unwrap()
            .replace("synthetic-access", "\\u0073ynthetic-access");
        let frame = format!(
            "event: response.failed\n{}\n\n",
            pretty
                .lines()
                .map(|l| format!("data: {l}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let body = [event(json!({"type":"response.created","sequence_number":1,"response":{"id":"resp-safe","model":"fake-model"}})), frame].concat().into_bytes();
        let root = temp("multiline-quarantine");
        let (addr, der, server) = tls_peer(metadata(), body, 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "models-one")))
            .unwrap();
        let smoke_grant = grant(&host, model_sha());
        let bytes = serde_json::to_vec(&smoke_grant).unwrap();
        let work = root.join("work");
        let mut action = action_for(&host, "smoke", "smoke-one");
        action.work_dir = Some(work.clone());
        action.smoke_grant_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
        action.valid_until_unix = action.issued_at_unix + 45;
        let projection = runtime()
            .block_on(host.smoke_admitted(&action, &smoke_grant, &bytes, &work))
            .unwrap();
        assert!(projection.quarantined);
        assert!(projection.exact_capture.contains("quarantine-"));
        assert!(
            !fs::read_to_string(work.join("attempt-attempt-one.json"))
                .unwrap()
                .contains("synthetic-access")
        );
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| *p == "/v1/responses")
                .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn custody_input_requires_attributed_retention_and_supported_cleanup() {
        let root = temp("custody-input");
        fs::create_dir(&root).unwrap();
        let dir = root.join("auth");
        prepare_private_dir(&dir).unwrap();
        let mut action = ActionGrant {
            release_id: "custody-one".into(),
            actor: "synthetic-release-owner".into(),
            custody_owner: "synthetic-account-owner".into(),
            custody_custodian: "synthetic-token-custodian".into(),
            custody_contract_ref: "synthetic-owner-acceptance".into(),
            retention_purpose: "bounded model inspection".into(),
            cleanup_disposition: "retain-until-owner-disconnect".into(),
            action: "models".into(),
            credential_dir: dir.clone(),
            host_id: "synthetic-host".into(),
            work_dir: None,
            smoke_grant_sha256: None,
            refresh_registration: None,
            executable_sha256: ActionGrant::executable_sha256().unwrap(),
            source_sha256: ActionGrant::source_sha256(),
            issued_at_unix: now(),
            valid_until_unix: now() + 15,
        };
        assert!(
            action
                .validate("models", &dir, "synthetic-host", None, None)
                .is_ok()
        );
        action.custody_contract_ref.clear();
        assert!(matches!(
            action.validate("models", &dir, "synthetic-host", None, None),
            Err(HostError::Policy)
        ));
        action.custody_contract_ref = "synthetic-owner-acceptance".into();
        action.cleanup_disposition = "delete-immediately".into();
        assert!(matches!(
            action.validate("models", &dir, "synthetic-host", None, None),
            Err(HostError::Policy)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fragmented_decoded_bearer_is_quarantined_before_journal() {
        let mut body = String::from_utf8(stream())
            .unwrap()
            .replace("B2_TRANSPORT_OK.", r"\u0073ynthetic-access")
            .replace("\"sequence_number\":4", "\"sequence_number\":5")
            .replace(
                "\"sequence_number\":5,\"response\"",
                "\"sequence_number\":6,\"response\"",
            );
        body = body.replacen(
            r#""delta":"\u0073ynthetic-access""#,
            r#""delta":"\u0073ynthetic-""#,
            1,
        );
        let split = event(
            json!({"type":"response.output_text.delta","sequence_number":4,"output_index":0,"content_index":0,"item_id":"item-fixture","delta":"access"}),
        );
        let insertion = body.find("event: response.output_text.done").unwrap();
        body.insert_str(insertion, &split);
        assert!(!body.contains("synthetic-access"));
        let root = temp("fragmented-quarantine");
        let (addr, der, server) = tls_peer(metadata(), body.into_bytes(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "models-one")))
            .unwrap();
        let smoke_grant = grant(&host, model_sha());
        let bytes = serde_json::to_vec(&smoke_grant).unwrap();
        let work = root.join("work");
        let mut action = action_for(&host, "smoke", "smoke-one");
        action.work_dir = Some(work.clone());
        action.smoke_grant_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
        action.valid_until_unix = action.issued_at_unix + 45;
        let projection = runtime()
            .block_on(host.smoke_admitted(&action, &smoke_grant, &bytes, &work))
            .unwrap();
        assert!(projection.quarantined);
        assert!(
            !fs::read_to_string(work.join("attempt-attempt-one.json"))
                .unwrap()
                .contains("synthetic-access")
        );
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| *p == "/v1/responses")
                .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn admitted_custody_change_fences_model_get() {
        let root = temp("custody-mismatch");
        let (addr, der, server) = tls_peer(metadata(), vec![], 2);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let mut action = action_for(&host, "models", "changed-custody");
        action.custody_custodian = "different-custodian".into();
        assert!(matches!(
            runtime().block_on(host.models_admitted(&action)),
            Err(HostError::Policy)
        ));
        assert!(!root.join("auth/action-changed-custody.json").exists());
        assert_eq!(
            server
                .join()
                .unwrap()
                .iter()
                .filter(|p| *p == "/v1/models")
                .count(),
            0
        );
        fs::remove_dir_all(root).unwrap();
    }

    // Direct public composition. Evidence class: synthetic. The TLS peer is a
    // throwaway `api.openai.com` identity trusted only by the test client; the
    // streams are author-built in the sparse public SIWC shape. These controls
    // exercise admission, claims, parser profile and journal classification,
    // not any live provider behavior.

    const DIRECT_MODEL: &str = "gpt-6-astra";
    /// Synthetic host source-manifest digest the test endpoints are composed with.
    pub(super) const HOST_SOURCE_SHA256: &str =
        "5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e5ca1ab1e";

    fn direct_adapter(
        host: &PublicHost,
        der: &[u8],
        addr: std::net::SocketAddr,
        release: &Path,
        journals: &Path,
    ) -> crate::DirectAdapter<PublicHostEndpoint> {
        let der = der.to_vec();
        let endpoint = PublicHostEndpoint::synthetic(
            move || Ok(PublicHttps::synthetic(&der, addr)),
            &host.credential_dir,
            &host.host_id,
            release,
        );
        crate::DirectAdapter::new(
            endpoint,
            crate::DirectProfile {
                model: DIRECT_MODEL.into(),
                bounds: PublicHostEndpoint::bounds(),
            },
            journals,
        )
    }

    fn direct_call(
        adapter: &crate::DirectAdapter<PublicHostEndpoint>,
        operation: &str,
        attempt: &str,
    ) -> DirectPreparedCall {
        use lifecycle_semantic::DirectInferencePort;
        let input = "synthetic direct compile input";
        let wire = adapter.wire(DirectRole::Compile, input).unwrap();
        let hex = |b: &[u8]| format!("{:x}", Sha256::digest(b));
        let mut call: DirectPreparedCall = serde_json::from_value(json!({
            "binding": {
                "subject": "subject-x", "transition": "transition-x", "capture_id": "capture-x",
                "role": "compile", "policy_id": "policy-x", "operation_id": operation,
                "attempt_id": attempt, "exposure_id": format!("exposure-{attempt}")
            },
            "source_basis_sha256": hex(b"basis"),
            "candidate_sha256": null,
            "compile_binding_sha256": null,
            "input": input,
            "input_sha256": hex(input.as_bytes()),
            "wire_sha256": hex(&wire.wire_bytes),
            "wire": wire,
            "max_output_bytes": 16384,
            "binding_sha256": "0".repeat(64),
        }))
        .unwrap();
        call.binding_sha256 = hex(&serde_json::to_vec(&(
            &call.binding,
            &call.source_basis_sha256,
            &call.candidate_sha256,
            &call.compile_binding_sha256,
            &call.input_sha256,
            &call.wire,
            &call.wire_sha256,
            call.max_output_bytes,
        ))
        .unwrap());
        call.validate().unwrap();
        call
    }

    fn private_dir(path: &Path) -> PathBuf {
        fs::DirBuilder::new().recursive(true).mode(0o700).create(path).unwrap();
        path.to_path_buf()
    }

    #[allow(clippy::too_many_arguments)]
    fn write_direct_release(
        host: &PublicHost,
        release: &Path,
        call: &DirectPreparedCall,
        model_sha: &str,
        release_id: &str,
        work: &Path,
        edit: impl FnOnce(&mut DirectSendGrant),
    ) {
        let mut smoke = grant(host, model_sha.to_owned());
        smoke.release_id = release_id.into();
        smoke.operation_id = call.binding.operation_id.clone();
        smoke.attempt_id = call.binding.attempt_id.clone();
        smoke.selected_model = DIRECT_MODEL.into();
        let mut direct = DirectSendGrant {
            smoke,
            role: call.binding.role,
            prepared_binding_sha256: call.binding_sha256.clone(),
            prepared_wire_sha256: call.wire_sha256.clone(),
            source_basis_sha256: call.source_basis_sha256.clone(),
            candidate_sha256: call.candidate_sha256.clone(),
            compile_binding_sha256: call.compile_binding_sha256.clone(),
            exposure_id: call.binding.exposure_id.clone(),
            profile_revision: PublicHostEndpoint::PROFILE_REVISION.into(),
            host_source_sha256: HOST_SOURCE_SHA256.into(),
        };
        edit(&mut direct);
        let bytes = serde_json::to_vec(&direct).unwrap();
        let mut action = action_for(host, "direct", release_id);
        action.work_dir = Some(work.to_path_buf());
        action.smoke_grant_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
        action.valid_until_unix = action.issued_at_unix + 45;
        let key = direct_identity_key(&call.binding.operation_id);
        for (name, content) in [
            (format!("direct-action-{key}.json"), serde_json::to_vec(&action).unwrap()),
            (format!("direct-grant-{key}.json"), bytes),
        ] {
            let path = release.join(name);
            let _ = fs::remove_file(&path);
            fs::write(&path, content).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }

    fn custody_claims(host: &PublicHost) -> usize {
        fs::read_dir(&host.credential_dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("smoke-"))
            .count()
    }

    #[test]
    fn direct_public_send_is_owner_admitted_siwc_parsed_and_never_resent() {
        use lifecycle_semantic::{DirectInferencePort, DirectSubmitError, DirectTerminal};
        let root = temp("direct-admitted");
        let raw = diagnostic_stream();
        let (addr, der, server) = tls_peer(diagnostic_metadata(), raw.clone(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let model = runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "direct-models")))
            .unwrap();
        let release = private_dir(&root.join("release"));
        let journals = root.join("journals");
        let adapter = direct_adapter(&host, &der, addr, &release, &journals);
        // Lifecycle ids carry `:`; the custody key is a digest of them.
        let call = direct_call(&adapter, "operation:b3-unit1-compile", "attempt:b3-unit1-compile-01");
        assert_eq!(call.wire.profile_revision, "b2-public-siwc-stream-v2");
        assert_eq!(call.wire.requested_provider, "openai_responses");
        let endpoint = PublicHostEndpoint::synthetic(
            || Err(PublicHttpsError::Policy),
            &host.credential_dir,
            &host.host_id,
            &release,
        );
        assert_eq!(endpoint.admission_status(&call), DirectAdmissionStatus::InputsMissing);
        let claims_before = custody_claims(&host);

        // No owner input: withheld before any claim, nothing claimed, no journal.
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::Withheld);
        assert_eq!(custody_claims(&host), claims_before);

        // Each mismatched owner binding is refused before any claim or intent.
        let model_sha = model.response_sha256.clone();
        let edits: Vec<Box<dyn Fn(&mut DirectSendGrant)>> = vec![
            Box::new(|g| g.smoke.selected_model = "other-model".into()),
            Box::new(|g| g.prepared_binding_sha256 = "0".repeat(64)),
            Box::new(|g| g.prepared_wire_sha256 = "0".repeat(64)),
            Box::new(|g| g.source_basis_sha256 = "0".repeat(64)),
            Box::new(|g| g.exposure_id = "wrong-exposure".into()),
            Box::new(|g| g.role = DirectRole::Verify),
            Box::new(|g| g.profile_revision = "selected-reasoning-v1".into()),
            Box::new(|g| g.smoke.attempt_id = "attempt:other".into()),
            Box::new(|g| g.smoke.credit_use_disabled = false),
            Box::new(|g| g.smoke.funding_observed_at_unix = now() - 301),
            Box::new(|g| g.host_source_sha256 = "1".repeat(64)),
        ];
        for edit in &edits {
            write_direct_release(&host, &release, &call, &model_sha, "direct-release", &journals, |g| edit(g));
            assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::Withheld);
            assert_eq!(custody_claims(&host), claims_before);
            assert!(!host.credential_dir.join("action-direct-release.json").exists());
            assert!(!journals.join(format!("{}.attempt.json", call.binding.operation_id)).exists());
        }

        write_direct_release(&host, &release, &call, &model_sha, "direct-release", &journals, |_| {});
        assert_eq!(endpoint.admission_status(&call), DirectAdmissionStatus::InputsPresent);
        let capture = adapter.submit(&call).unwrap();
        assert_eq!(capture.terminal, DirectTerminal::Complete);
        assert_eq!(capture.observed_model.as_deref(), Some(DIRECT_MODEL));
        assert_eq!(capture.observed_provider.as_deref(), Some("openai_responses"));
        assert_eq!(capture.response_id.as_deref(), Some("response-diagnostic"));
        assert_eq!(capture.evidence, DirectProductionProfile::public_api());
        // The capture digest is of the retained raw body, which is kept.
        assert_eq!(capture.captured_stream_sha256, format!("{:x}", Sha256::digest(&raw)));
        let key = direct_identity_key(&call.binding.attempt_id);
        assert_eq!(fs::read(journals.join(format!("capture-{key}.bin"))).unwrap(), raw);
        assert!(journals.join("funding-direct-release.json").exists());
        assert!(journals.join(format!("direct-admission-{key}.json")).exists());
        let journal = AttemptJournal::open(journals.join(format!("{}.attempt.json", call.binding.operation_id))).unwrap();
        assert_eq!(journal.record().profile_revision, "b2-public-siwc-stream-v2");
        assert_eq!(journal.record().registration.as_ref(), Some(&host.selected().unwrap()));
        assert_eq!(custody_claims(&host), claims_before + 3);
        assert_eq!(endpoint.admission_status(&call), DirectAdmissionStatus::IdentityClaimed);

        // Same journal: possible earlier send, never resent.
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        // Another work directory (restart with new config), even with a fresh
        // owner release for the same operation: the custody claim fences it.
        let journals2 = root.join("journals-2");
        let adapter2 = direct_adapter(&host, &der, addr, &release, &journals2);
        write_direct_release(&host, &release, &call, &model_sha, "direct-release-2", &journals2, |_| {});
        assert_eq!(adapter2.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert!(!host.credential_dir.join("action-direct-release-2.json").exists());
        // The legacy exchange without a prepared call never sends.
        assert!(matches!(
            endpoint.exchange(&journals2.join("x.json"), "op", "at", &crate::assemble_request("i", "x", &ToolFreeProfile {
                revision: PublicHostEndpoint::PROFILE_REVISION.into(),
                model: DIRECT_MODEL.into(),
                bounds: PublicHostEndpoint::bounds(),
            }).unwrap()),
            DirectExchange::NotSent
        ));
        assert_eq!(
            server.join().unwrap().iter().filter(|p| p.as_str() == "/v1/responses").count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn direct_public_profile_and_bounds_are_fixed_before_any_claim() {
        use lifecycle_semantic::{DirectInferencePort, DirectSubmitError};
        let root = temp("direct-profile");
        let (addr, der, server) = tls_peer(diagnostic_metadata(), diagnostic_stream(), 3);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let model = runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "direct-models")))
            .unwrap();
        let release = private_dir(&root.join("release"));
        let journals = root.join("journals");
        let claims_before = custody_claims(&host);
        let mut bounds = PublicHostEndpoint::bounds();
        bounds.output_bytes = 8 * 1024;
        let der2 = der.clone();
        let adapter = crate::DirectAdapter::new(
            PublicHostEndpoint::synthetic(
                move || Ok(PublicHttps::synthetic(&der2, addr)),
                &host.credential_dir,
                &host.host_id,
                &release,
            ),
            crate::DirectProfile { model: DIRECT_MODEL.into(), bounds },
            &journals,
        );
        let call = direct_call(&adapter, "operation:bounds", "attempt:bounds");
        write_direct_release(&host, &release, &call, &model.response_sha256, "bounds-release", &journals, |_| {});
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::NotSent);
        assert_eq!(custody_claims(&host), claims_before);
        assert!(!host.credential_dir.join("action-bounds-release.json").exists());
        // A selected-reasoning-v1 wire offered to the public endpoint is refused.
        let adapter = direct_adapter(&host, &der, addr, &release, &journals);
        let mut call = direct_call(&adapter, "operation:profile", "attempt:profile");
        call.wire.profile_revision = "selected-reasoning-v1".into();
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::NotSent);
        assert_eq!(custody_claims(&host), claims_before);
        assert_eq!(
            server.join().unwrap().iter().filter(|p| p.as_str() == "/v1/responses").count(),
            0
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn direct_public_final_currentness_failure_after_intent_is_fenced_not_sent() {
        use lifecycle_semantic::{DirectInferencePort, DirectSubmitError};
        let root = temp("direct-fence");
        let (addr, der, server) = tls_peer(diagnostic_metadata(), diagnostic_stream(), 3);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let model = runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "direct-models")))
            .unwrap();
        let release = private_dir(&root.join("release"));
        let journals = root.join("journals");
        let adapter = direct_adapter(&host, &der, addr, &release, &journals);
        let call = direct_call(&adapter, "operation:fence", "attempt:fence");
        // Every pre-claim check passes; the test hook then signs the selected
        // registration out between the claim and the send boundary.
        write_direct_release(&host, &release, &call, &model.response_sha256, "test-postclaim-signout", &journals, |_| {});
        let claims_before = custody_claims(&host);
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::NotSent);
        let key = direct_identity_key(&call.binding.attempt_id);
        // Unlike a pre-claim `Withheld`, the release and both identities were
        // consumed. The obligation was durable before the possible send and
        // stays open, so the operation cannot be resent.
        assert_eq!(custody_claims(&host), claims_before + 3);
        assert!(journals.join("funding-test-postclaim-signout.json").exists());
        let fence: serde_json::Value =
            serde_json::from_slice(&fs::read(journals.join(format!("pre-send-fence-{key}.json"))).unwrap())
                .unwrap();
        assert_eq!(fence["post_dispatched"], false);
        assert_eq!(fence["release_consumed"], true);
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert_eq!(
            server.join().unwrap().iter().filter(|p| p.as_str() == "/v1/responses").count(),
            0
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn direct_public_adverse_streams_never_yield_a_complete_capture() {
        use lifecycle_semantic::{DirectInferencePort, DirectSubmitError, DirectTerminal};
        let wrong_model = String::from_utf8(diagnostic_stream())
            .unwrap()
            .replace("gpt-6-astra", "gpt-6-other")
            .into_bytes();
        let mut missing_terminal = diagnostic_stream();
        let cut = String::from_utf8_lossy(&missing_terminal).rfind("data: ").unwrap();
        missing_terminal.truncate(cut);
        let conflicting_id = String::from_utf8(diagnostic_stream())
            .unwrap()
            .replacen("response-diagnostic", "response-other", 1)
            .into_bytes();
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("wrong-model", wrong_model),
            ("provider-incomplete", diagnostic_provider_incomplete()),
            ("completed-without-output", diagnostic_completed_without_output()),
            ("missing-terminal", missing_terminal),
            ("conflicting-response-id", conflicting_id),
            ("malformed", b"data: {not json\n\n".to_vec()),
        ];
        for (label, body) in cases {
            let root = temp(&format!("direct-adverse-{label}"));
            let (addr, der, server) = tls_peer(diagnostic_metadata(), body, 4);
            let host = seeded_host(addr, &der, &root.join("auth"));
            let model = runtime()
                .block_on(host.models_admitted(&action_for(&host, "models", "direct-models")))
                .unwrap();
            let release = private_dir(&root.join("release"));
            let journals = root.join("journals");
            let adapter = direct_adapter(&host, &der, addr, &release, &journals);
            let call = direct_call(&adapter, "operation:adverse", "attempt:adverse");
            write_direct_release(&host, &release, &call, &model.response_sha256, "adverse-release", &journals, |_| {});
            match adapter.submit(&call) {
                Err(error) => assert_eq!(error, DirectSubmitError::OutcomeUnknown, "{label}"),
                Ok(capture) => assert_ne!(capture.terminal, DirectTerminal::Complete, "{label}"),
            }
            assert!(journals.join("funding-adverse-release.json").exists(), "{label}");
            assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown, "{label}");
            assert_eq!(
                server.join().unwrap().iter().filter(|p| p.as_str() == "/v1/responses").count(),
                1,
                "{label}"
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn direct_public_endpoint_open_is_read_only_and_requires_owner_only_release_dir() {
        let root = temp("direct-open");
        let release = private_dir(&root.join("release"));
        let absent = root.join("absent-credentials");
        let exe = ActionGrant::executable_sha256().unwrap();
        let open = |cred: &Path, release: &Path| {
            PublicHostEndpoint::open(cred, "synthetic_host_public", release, HOST_SOURCE_SHA256, &exe)
        };
        assert!(matches!(open(&absent, &release), Err(HostError::Auth)));
        assert!(!absent.exists(), "opening must not initialize a store");
        assert!(matches!(open(Path::new("relative"), &release), Err(HostError::Policy)));
        let shared = root.join("shared-release");
        fs::create_dir_all(&shared).unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(open(&absent, &shared), Err(HostError::Policy)));
        assert!(matches!(open(&absent, &root.join("missing-release")), Err(HostError::Policy)));
        // The release directory may not be nested with the credential store.
        let nested = private_dir(&absent.join("release"));
        assert!(matches!(open(&absent, &nested), Err(HostError::Policy)));
        fs::remove_dir_all(&absent).unwrap();
        // A malformed host source digest or an executable digest that is not
        // the running executable is refused before readiness is inspected.
        for (source, exe) in [("not-a-digest", exe.as_str()), (HOST_SOURCE_SHA256, &"0".repeat(64))] {
            assert!(matches!(
                PublicHostEndpoint::open(&absent, "synthetic_host_public", &release, source, exe),
                Err(HostError::Policy)
            ));
        }
        assert!(!absent.exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn rewrite_action(release: &Path, call: &DirectPreparedCall, edit: impl FnOnce(&mut ActionGrant)) {
        let key = direct_identity_key(&call.binding.operation_id);
        let path = release.join(format!("direct-action-{key}.json"));
        let mut action: ActionGrant = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        edit(&mut action);
        fs::write(&path, serde_json::to_vec(&action).unwrap()).unwrap();
    }

    fn release_check(endpoint: &PublicHostEndpoint, call: &DirectPreparedCall) -> serde_json::Value {
        let (_, check) = endpoint.release_projection_paths(&call.binding.operation_id);
        serde_json::from_slice(&fs::read(check).unwrap()).unwrap()
    }

    /// Release offer before intent. Evidence class: synthetic (throwaway TLS
    /// peer, author-built stream, synthetic registration).
    #[test]
    fn direct_release_offer_projects_the_exact_call_and_prevalidates_without_consuming() {
        use lifecycle_semantic::{DirectInferencePort, DirectReleaseState, DirectSubmitError, DirectTerminal};
        let root = temp("direct-release-offer");
        let (addr, der, server) = tls_peer(diagnostic_metadata(), diagnostic_stream(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let model = runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "direct-models")))
            .unwrap();
        let release = private_dir(&root.join("release"));
        let journals = root.join("journals");
        let adapter = direct_adapter(&host, &der, addr, &release, &journals);
        let endpoint = PublicHostEndpoint::synthetic(
            || Err(PublicHttpsError::Policy),
            &host.credential_dir,
            &host.host_id,
            &release,
        );
        let call = direct_call(&adapter, "operation:release-offer", "attempt:release-offer");
        let claims_before = custody_claims(&host);
        let journal = journals.join(format!("{}.attempt.json", call.binding.operation_id));
        let pending = |state: DirectReleaseState| matches!(state, DirectReleaseState::Pending { .. });

        // Missing release: Pending, with the exact call projected for its owner.
        assert!(pending(adapter.offer_release(&call)));
        let (request_path, _) = endpoint.release_projection_paths(&call.binding.operation_id);
        let meta = fs::symlink_metadata(&request_path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        let request: serde_json::Value = serde_json::from_slice(&fs::read(&request_path).unwrap()).unwrap();
        let back: DirectPreparedCall = serde_json::from_value(request["call"].clone()).unwrap();
        assert_eq!(back, call);
        assert_eq!(request["grant"]["host_source_sha256"], HOST_SOURCE_SHA256);
        assert_eq!(request["grant"]["prepared_binding_sha256"], call.binding_sha256.as_str());
        assert_eq!(request["action"]["work_dir"], journals.to_str().unwrap());
        assert_eq!(request["action"]["source_sha256"], ActionGrant::source_sha256());
        assert_eq!(request["action"]["executable_sha256"], ActionGrant::executable_sha256().unwrap());
        assert_eq!(request["bounds"], serde_json::to_value(PublicHostEndpoint::bounds()).unwrap());
        // No credential material: the projection names the store, never its contents.
        let projection = String::from_utf8(fs::read(&request_path).unwrap()).unwrap();
        for entry in fs::read_dir(&host.credential_dir).unwrap().flatten() {
            let bytes = fs::read(entry.path()).unwrap_or_default();
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                for key in ["access_token", "refresh_token", "id_token"] {
                    if let Some(secret) = value.get(key).and_then(|v| v.as_str()) {
                        assert!(!projection.contains(secret), "{key} leaked");
                    }
                }
            }
        }
        assert_eq!(release_check(&endpoint, &call)["status"], "pending");
        assert_eq!(custody_claims(&host), claims_before);
        assert!(!journal.exists());

        // Inconsistent releases stay Pending and consume nothing.
        let model_sha = model.response_sha256.clone();
        let edits: Vec<Box<dyn Fn(&mut DirectSendGrant)>> = vec![
            Box::new(|g| g.host_source_sha256 = "1".repeat(64)),
            Box::new(|g| g.prepared_binding_sha256 = "0".repeat(64)),
            Box::new(|g| g.smoke.selected_model = "other-model".into()),
            Box::new(|g| g.smoke.operation_id = "operation:other".into()),
        ];
        for edit in &edits {
            write_direct_release(&host, &release, &call, &model_sha, "offer-release", &journals, |g| edit(g));
            assert!(pending(adapter.offer_release(&call)));
            assert_eq!(release_check(&endpoint, &call)["status"], "pending");
        }
        type ActionEdit = Box<dyn Fn(&mut ActionGrant)>;
        let action_edits: Vec<(&str, ActionEdit)> = vec![
            ("short window", Box::new(|a| a.valid_until_unix = now() + 20)),
            ("other work dir", Box::new(|a| a.work_dir = Some(PathBuf::from("/elsewhere")))),
            ("other executable", Box::new(|a| a.executable_sha256 = "0".repeat(64))),
            ("wrong action", Box::new(|a| a.action = "smoke".into())),
        ];
        for (label, edit) in &action_edits {
            write_direct_release(&host, &release, &call, &model_sha, "offer-release", &journals, |_| {});
            rewrite_action(&release, &call, |a| edit(a));
            assert!(pending(adapter.offer_release(&call)), "{label}");
        }
        assert_eq!(custody_claims(&host), claims_before);
        assert!(!host.credential_dir.join("action-offer-release.json").exists());
        assert!(!journal.exists());

        // A consistent release is Ready, and offering it (twice) consumes
        // nothing. Each offer re-hashes the executable, so the release is
        // rewritten before each one to keep its 45 s window fresh under load.
        for _ in 0..2 {
            write_direct_release(&host, &release, &call, &model_sha, "offer-release", &journals, |_| {});
            assert_eq!(adapter.offer_release(&call), DirectReleaseState::Ready);
            assert_eq!(release_check(&endpoint, &call)["status"], "ready");
            assert_eq!(custody_claims(&host), claims_before);
            assert!(!host.credential_dir.join("action-offer-release.json").exists());
            assert_eq!(endpoint.admission_status(&call), DirectAdmissionStatus::InputsPresent);
        }

        // The send boundary re-checks and sends exactly once.
        write_direct_release(&host, &release, &call, &model_sha, "offer-release", &journals, |_| {});
        let capture = adapter.submit(&call).unwrap();
        assert_eq!(capture.terminal, DirectTerminal::Complete);
        // After a send the journal decides; the owner is not consulted again.
        fs::remove_file(endpoint.release_input_paths(&call.binding.operation_id).0).unwrap();
        assert_eq!(adapter.offer_release(&call), DirectReleaseState::Ready);
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        // A different work directory sees the custody claim: Ready, then
        // OutcomeUnknown, never Pending and never a resend.
        let journals2 = root.join("journals-2");
        let adapter2 = direct_adapter(&host, &der, addr, &release, &journals2);
        assert_eq!(adapter2.offer_release(&call), DirectReleaseState::Ready);
        assert_eq!(adapter2.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert_eq!(
            server.join().unwrap().iter().filter(|p| p.as_str() == "/v1/responses").count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    /// A `Ready` offer is followed by durable lifecycle intent, after which a
    /// send-boundary refusal is a terminal `NotSent`. Evidence class:
    /// synthetic (throwaway TLS peer, author-built stream, synthetic
    /// registration and grants).
    #[test]
    fn ready_release_covers_send_currentness_and_is_consumed_as_checked() {
        use lifecycle_semantic::{DirectInferencePort, DirectReleaseState, DirectSubmitError, DirectTerminal};
        let root = temp("direct-release-checked");
        let (addr, der, server) = tls_peer(diagnostic_metadata(), diagnostic_stream(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let model = runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "direct-models")))
            .unwrap();
        let model_sha = model.response_sha256.clone();
        let release = private_dir(&root.join("release"));
        let journals = root.join("journals");
        let adapter = direct_adapter(&host, &der, addr, &release, &journals);
        let endpoint = PublicHostEndpoint::synthetic(
            || Err(PublicHttpsError::Policy),
            &host.credential_dir,
            &host.host_id,
            &release,
        );
        let claims_before = custody_claims(&host);

        // Each send-boundary check that a valid-now release could fail within
        // the offer-to-send gap keeps the offer Pending.
        let call = direct_call(&adapter, "operation:checked", "attempt:checked");
        type GrantEdit = Box<dyn Fn(&mut DirectSendGrant)>;
        let edits: Vec<(&str, GrantEdit)> = vec![
            ("model catalog", Box::new(|g| g.smoke.model_observation_sha256 = "a".repeat(64))),
            ("funding observation expires", Box::new(|g| g.smoke.funding_observation_valid_until_unix = now() + 30)),
            ("smoke grant", Box::new(|g| g.smoke.funding_observed_at_unix = now() - 299)),
        ];
        for (reason, edit) in &edits {
            write_direct_release(&host, &release, &call, &model_sha, "checked-release", &journals, |g| edit(g));
            assert!(matches!(adapter.offer_release(&call), DirectReleaseState::Pending { .. }), "{reason}");
            let check = release_check(&endpoint, &call);
            assert_eq!(check["status"], "pending", "{reason}");
            assert!(check["reason"].as_str().unwrap().starts_with(reason), "{reason}: {check}");
        }
        assert_eq!(custody_claims(&host), claims_before);
        assert!(!host.credential_dir.join("action-checked-release.json").exists());

        // A non-Ready offer discards inputs an earlier offer checked: the send
        // then reads the current (invalid) files and claims nothing.
        let other = direct_call(&adapter, "operation:rechecked", "attempt:rechecked");
        write_direct_release(&host, &release, &other, &model_sha, "rechecked-release", &journals, |_| {});
        assert_eq!(adapter.offer_release(&other), DirectReleaseState::Ready);
        write_direct_release(&host, &release, &other, &model_sha, "rechecked-release", &journals, |g| {
            g.host_source_sha256 = "1".repeat(64)
        });
        assert!(matches!(adapter.offer_release(&other), DirectReleaseState::Pending { .. }));
        // Refused before the action claim: withheld, the call stays offerable.
        assert_eq!(adapter.submit(&other).unwrap_err(), DirectSubmitError::Withheld);
        assert_eq!(custody_claims(&host), claims_before);
        assert!(!host.credential_dir.join("action-rechecked-release.json").exists());

        // Ready: the next send consumes exactly the checked inputs, even though
        // the files were removed and replaced after the check.
        write_direct_release(&host, &release, &call, &model_sha, "checked-release", &journals, |_| {});
        assert_eq!(adapter.offer_release(&call), DirectReleaseState::Ready);
        let (action_path, grant_path) = endpoint.release_input_paths(&call.binding.operation_id);
        fs::remove_file(&action_path).unwrap();
        fs::write(&grant_path, b"{}").unwrap();
        let capture = adapter.submit(&call).unwrap();
        assert_eq!(capture.terminal, DirectTerminal::Complete);
        assert!(host.credential_dir.join("action-checked-release.json").exists());
        assert_eq!(custody_claims(&host), claims_before + 3);
        // Checked inputs are taken by that send; nothing is sent again.
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert_eq!(
            server.join().unwrap().iter().filter(|p| p.as_str() == "/v1/responses").count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    /// Pre-consumption refusals through the public endpoint. Evidence class:
    /// synthetic (throwaway TLS peer, author-built stream, synthetic
    /// registration and custody). Deterministically bad custody or release
    /// identifiers stay `Pending` at offer; custody changed, the release spent
    /// elsewhere, or the action window lapsed after `Ready` are `Withheld` at
    /// the send boundary with nothing claimed, journalled or sent; a later
    /// valid release is re-offered, re-checked and sent exactly once.
    #[test]
    fn direct_release_refused_before_claim_is_withheld_and_a_valid_release_sends_once() {
        use lifecycle_semantic::{DirectInferencePort, DirectReleaseState, DirectSubmitError, DirectTerminal};
        let root = temp("direct-release-withheld");
        let (addr, der, server) = tls_peer(diagnostic_metadata(), diagnostic_stream(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let model = runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "direct-models")))
            .unwrap();
        let model_sha = model.response_sha256.clone();
        let release = private_dir(&root.join("release"));
        let journals = root.join("journals");
        let adapter = direct_adapter(&host, &der, addr, &release, &journals);
        let endpoint = PublicHostEndpoint::synthetic(
            || Err(PublicHttpsError::Policy),
            &host.credential_dir,
            &host.host_id,
            &release,
        );
        let call = direct_call(&adapter, "operation:withheld", "attempt:withheld");
        let journal = journals.join(format!("{}.attempt.json", call.binding.operation_id));
        let custody = host.credential_dir.join("credential-custody-contract.json");
        let custody_bytes = fs::read(&custody).unwrap();
        let claims_before = custody_claims(&host);
        let unconsumed = |label: &str| {
            assert_eq!(custody_claims(&host), claims_before, "{label}");
            assert!(!host.credential_dir.join("action-withheld-release.json").exists(), "{label}");
            assert!(!journal.exists(), "{label}");
        };
        let pending = |label: &str| {
            assert!(
                matches!(adapter.offer_release(&call), DirectReleaseState::Pending { .. }),
                "{label}"
            );
            assert_eq!(release_check(&endpoint, &call)["status"], "pending", "{label}");
        };

        // Deterministic bad custody or release id: Pending at offer.
        write_direct_release(&host, &release, &call, &model_sha, "withheld-release", &journals, |_| {});
        fs::remove_file(&custody).unwrap();
        pending("custody contract missing");
        let reason = release_check(&endpoint, &call)["reason"].as_str().unwrap().to_owned();
        assert!(reason.starts_with("credential custody contract"), "{reason}");
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::Withheld);
        fs::write(&custody, &custody_bytes).unwrap();
        fs::set_permissions(&custody, fs::Permissions::from_mode(0o600)).unwrap();
        rewrite_action(&release, &call, |a| a.custody_custodian = "different-custodian".into());
        pending("custody contract not the action's");
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::Withheld);
        rewrite_action(&release, &call, |a| a.release_id = "../withheld-release".into());
        pending("malformed release id");
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::Withheld);
        unconsumed("deterministic");

        // Changed after Ready: custody replaced, release spent elsewhere, or
        // the action window lapsed in the offer-to-send gap.
        type Lapse<'a> = Box<dyn Fn() + 'a>;
        let spent = host.credential_dir.join("action-withheld-release.json");
        let lapses: Vec<(&str, Lapse<'_>, Lapse<'_>)> = vec![
            (
                "custody replaced",
                Box::new(|| {
                    let mut other: serde_json::Value = serde_json::from_slice(&custody_bytes).unwrap();
                    other["custodian"] = "different-custodian".into();
                    fs::write(&custody, serde_json::to_vec(&other).unwrap()).unwrap();
                }),
                Box::new(|| fs::write(&custody, &custody_bytes).unwrap()),
            ),
            (
                "release spent elsewhere",
                Box::new(|| fs::write(&spent, b"{}").unwrap()),
                Box::new(|| fs::remove_file(&spent).unwrap()),
            ),
            (
                "action window lapsed",
                // Wait until less than the send boundary's 30 s remains.
                Box::new(|| {
                    let key = direct_identity_key(&call.binding.operation_id);
                    let action: ActionGrant = serde_json::from_slice(
                        &fs::read(release.join(format!("direct-action-{key}.json"))).unwrap(),
                    )
                    .unwrap();
                    while action.valid_until_unix.saturating_sub(now()) >= 29 {
                        std::thread::sleep(Duration::from_millis(250));
                    }
                }),
                Box::new(|| ()),
            ),
        ];
        for (label, lapse, restore) in &lapses {
            write_direct_release(&host, &release, &call, &model_sha, "withheld-release", &journals, |_| {});
            if *label == "action window lapsed" {
                rewrite_action(&release, &call, |a| a.valid_until_unix = now() + 40);
            }
            assert_eq!(adapter.offer_release(&call), DirectReleaseState::Ready, "{label}");
            lapse();
            assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::Withheld, "{label}");
            restore();
            unconsumed(label);
        }

        // A valid release: offered again, re-checked, sent once, never again.
        write_direct_release(&host, &release, &call, &model_sha, "withheld-release", &journals, |_| {});
        assert_eq!(adapter.offer_release(&call), DirectReleaseState::Ready);
        let capture = adapter.submit(&call).unwrap();
        assert_eq!(capture.terminal, DirectTerminal::Complete);
        assert!(host.credential_dir.join("action-withheld-release.json").exists());
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert_eq!(
            server.join().unwrap().iter().filter(|p| p.as_str() == "/v1/responses").count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    /// A work directory that cannot be created refuses before the endpoint.
    /// Before any claim that refusal is `Withheld` and the call is re-offered
    /// and sent once. After this call's identity is claimed in shared custody,
    /// another work directory whose journal directory fails is `Ready` at offer
    /// and `OutcomeUnknown` at send: a `Withheld` there would let the lifecycle
    /// withdraw a possibly sent call to `Pending`. Evidence class: synthetic
    /// (throwaway TLS peer, author-built stream, synthetic registration and
    /// custody).
    #[test]
    fn journal_directory_failure_after_a_claim_elsewhere_is_unknown_not_withheld() {
        use lifecycle_semantic::{DirectInferencePort, DirectReleaseState, DirectSubmitError, DirectTerminal};
        let root = temp("direct-journal-dir");
        let (addr, der, server) = tls_peer(diagnostic_metadata(), diagnostic_stream(), 4);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let model = runtime()
            .block_on(host.models_admitted(&action_for(&host, "models", "direct-models")))
            .unwrap();
        let model_sha = model.response_sha256.clone();
        let release = private_dir(&root.join("release"));
        let journals = root.join("journals");
        let adapter = direct_adapter(&host, &der, addr, &release, &journals);
        let call = direct_call(&adapter, "operation:journal-dir", "attempt:journal-dir");
        let spent = host.credential_dir.join("action-journal-dir-release.json");
        let claims_before = custody_claims(&host);

        // Before any claim: a regular file where the work directory belongs.
        fs::write(&journals, b"").unwrap();
        write_direct_release(&host, &release, &call, &model_sha, "journal-dir-release", &journals, |_| {});
        assert_eq!(adapter.submit(&call).unwrap_err(), DirectSubmitError::Withheld);
        assert_eq!(custody_claims(&host), claims_before);
        assert!(!spent.exists());
        // Repaired, the same call is offered again, re-checked and sent once.
        fs::remove_file(&journals).unwrap();
        write_direct_release(&host, &release, &call, &model_sha, "journal-dir-release", &journals, |_| {});
        assert_eq!(adapter.offer_release(&call), DirectReleaseState::Ready);
        assert_eq!(adapter.submit(&call).unwrap().terminal, DirectTerminal::Complete);
        assert!(spent.exists());
        let claims_after = custody_claims(&host);
        assert!(claims_after > claims_before);

        // Elsewhere, the work directory is absent when its adapter is composed
        // and later becomes a regular file: no local journal can exist there,
        // so only shared custody shows the claim.
        let elsewhere = root.join("journals-2");
        let adapter2 = direct_adapter(&host, &der, addr, &release, &elsewhere);
        fs::write(&elsewhere, b"").unwrap();
        assert_eq!(adapter2.offer_release(&call), DirectReleaseState::Ready);
        assert_eq!(adapter2.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert_eq!(adapter2.submit(&call).unwrap_err(), DirectSubmitError::OutcomeUnknown);
        // An inaccessible work directory elsewhere: likewise never withheld.
        let locked = private_dir(&root.join("locked"));
        let adapter3 = direct_adapter(&host, &der, addr, &release, &locked.join("journals"));
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let offered = adapter3.offer_release(&call);
        let refused = adapter3.submit(&call);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(offered, DirectReleaseState::Ready);
        assert_eq!(refused.unwrap_err(), DirectSubmitError::OutcomeUnknown);
        assert_eq!(custody_claims(&host), claims_after);
        assert_eq!(
            server.join().unwrap().iter().filter(|p| p.as_str() == "/v1/responses").count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn owner_inputs_refuse_symlinks_fifos_swaps_and_shared_release_dirs() {
        use lifecycle_semantic::{DirectInferencePort, DirectReleaseState};
        let root = temp("direct-owner-input");
        let release = private_dir(&root.join("release"));
        let outside = root.join("outside.json");
        fs::write(&outside, b"{}").unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(read_owner_input(&outside).unwrap(), b"{}");

        let link = release.join("link.json");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert!(matches!(read_owner_input(&link), Err(HostError::Policy)));

        let fifo = release.join("fifo.json");
        rustix::fs::mkfifoat(rustix::fs::CWD, &fifo, rustix::fs::Mode::from_raw_mode(0o600)).unwrap();
        assert!(matches!(read_owner_input(&fifo), Err(HostError::Policy)));

        let shared = release.join("shared.json");
        fs::write(&shared, b"{}").unwrap();
        fs::set_permissions(&shared, fs::Permissions::from_mode(0o620)).unwrap();
        assert!(matches!(read_owner_input(&shared), Err(HostError::Policy)));

        let large = release.join("large.json");
        fs::write(&large, vec![b' '; RELEASE_INPUT_LIMIT as usize + 1]).unwrap();
        fs::set_permissions(&large, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(matches!(read_owner_input(&large), Err(HostError::Policy)));

        // A regular file swapped for a symlink to a valid file is refused.
        let swapped = release.join("swapped.json");
        fs::write(&swapped, b"{}").unwrap();
        fs::set_permissions(&swapped, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(read_owner_input(&swapped).is_ok());
        fs::remove_file(&swapped).unwrap();
        std::os::unix::fs::symlink(&outside, &swapped).unwrap();
        assert!(matches!(read_owner_input(&swapped), Err(HostError::Policy)));

        // Release-directory ownership and mode are rechecked at each read.
        let (addr, der, server) = tls_peer(diagnostic_metadata(), diagnostic_stream(), 2);
        let host = seeded_host(addr, &der, &root.join("auth"));
        let journals = root.join("journals");
        let adapter = direct_adapter(&host, &der, addr, &release, &journals);
        let call = direct_call(&adapter, "operation:owner-input", "attempt:owner-input");
        let endpoint = PublicHostEndpoint::synthetic(
            || Err(PublicHttpsError::Policy),
            &host.credential_dir,
            &host.host_id,
            &release,
        );
        let (action, _) = endpoint.release_input_paths(&call.binding.operation_id);
        std::os::unix::fs::symlink(&outside, &action).unwrap();
        assert!(matches!(
            endpoint.release_inputs(&call.binding.operation_id),
            Err(HostError::Policy)
        ));
        fs::set_permissions(&release, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            endpoint.release_inputs(&call.binding.operation_id),
            Err(HostError::Policy)
        ));
        assert!(matches!(adapter.offer_release(&call), DirectReleaseState::Pending { .. }));
        fs::set_permissions(&release, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(!server.join().unwrap().iter().any(|p| p.as_str() == "/v1/responses"));
        fs::remove_dir_all(root).unwrap();
    }
}
