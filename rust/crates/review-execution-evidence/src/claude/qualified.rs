use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use claim_evidence::codec::{JsValue, canonical_json};
use slice_campaign::NativeReviewRequestRef;

use crate::contract::{require_sha, require_text, sha256};
use crate::{EvidenceReadError, ExecutionProvenance};
pub use review_execution_custody::claude::selection::{
    Prepared, QualifiedReaderSelection, command_arguments, command_digest, parse_stored_subject,
};

use super::isolation::{CandidateBoundary, tree_digest};
use super::session::RetainedReviewSession;
use super::{CredentialSource, TrustedClaudeProfile};

const MAX_PROMPT_BYTES: usize = 1024 * 1024;
const MAX_ATTEMPT_HORIZON: Duration = Duration::from_secs(600);
/// Untrusted selection data. Binding it checks the installed peer, candidate,
/// request, and selected identities; it never grants a launch.
#[derive(Clone)]
pub struct QualifiedSelection {
    pub root_id: String,
    pub provenance: ExecutionProvenance,
    pub selection_id: String,
    pub candidate_commit: String,
    pub candidate_tree: String,
    pub candidate_patch_identity: String,
    pub candidate_path: PathBuf,
    pub candidate_tree_sha256: String,
    pub cbm_project: String,
    pub cbm_store_sha256: String,
    pub cbm_generation: String,
    pub cbm_coverage_sha256: String,
    pub prompt: Vec<u8>,
    pub subject: JsValue,
    pub credential: Option<CredentialSource>,
    pub offline_witness: Option<PathBuf>,
    pub horizon: Duration,
}

/// Checked, private-field offline scope. A future RP2 installer may bind the
/// same mechanism to a production grant; this type has no such constructor.
#[derive(Clone)]
pub struct QualifiedScope {
    pub(crate) profile: TrustedClaudeProfile,
    pub(crate) selection: QualifiedSelection,
    pub(crate) prepared: Prepared,
    pub(crate) boundary: Option<Arc<CandidateBoundary>>,
    pub(crate) applied_physical_profile_digest: Option<String>,
    pub(crate) applied_request: Option<AppliedRequestBinding>,
}

#[derive(Clone)]
pub(crate) struct AppliedRequestBinding {
    pub(crate) request: NativeReviewRequestRef,
    pub(crate) prompt_template_sha256: String,
    pub(crate) template: Vec<u8>,
}

/// Deterministic prompt construction from a pinned static template and the
/// campaign's exact prepared request. This function carries no entry grant.
pub fn compose_applied_initial_prompt(
    template: &[u8],
    request: &NativeReviewRequestRef,
) -> Result<Vec<u8>, EvidenceReadError> {
    let request_bytes = serde_json::to_vec(request)
        .map_err(|_| EvidenceReadError::Conflicting("campaign request encoding invalid"))?;
    let mut prompt = Vec::with_capacity(template.len() + request_bytes.len() + 31);
    prompt.extend_from_slice(template);
    prompt.extend_from_slice(b"\n\n-- Bound review request --\n");
    prompt.extend_from_slice(&request_bytes);
    if prompt.len() > MAX_PROMPT_BYTES {
        return Err(EvidenceReadError::Capacity("Claude prompt oversized"));
    }
    Ok(prompt)
}

impl QualifiedScope {
    /// Offline-only adversarial mutations exercise the exact Applied entry
    /// validator after campaign dispatch. They cannot issue an entry token.
    #[cfg(feature = "test-support")]
    pub fn test_mutate_offline_applied_scope(
        &mut self,
        field: &str,
    ) -> Result<(), EvidenceReadError> {
        if self.profile.installed() || self.applied_request.is_none() {
            return Err(EvidenceReadError::Unsupported(
                "offline Applied test scope absent",
            ));
        }
        match field {
            "episode" => self.prepared.review_episode_id.push_str("-different"),
            "candidate" => self.prepared.candidate_tree.push_str("different"),
            "prompt" => {
                self.selection.prompt.extend_from_slice(b"\nchanged prompt");
                self.prepared.prompt_sha256 = sha256(&self.selection.prompt);
            }
            "horizon" => self.prepared.attempt_horizon_millis += 1,
            _ => {
                return Err(EvidenceReadError::Unsupported(
                    "unknown Applied test mutation",
                ));
            }
        }
        Ok(())
    }

    pub fn with_applied_request(
        mut self,
        template: &[u8],
        request: NativeReviewRequestRef,
    ) -> Result<Self, EvidenceReadError> {
        let b = &self.selection.provenance.binding;
        if self.selection.prompt != compose_applied_initial_prompt(template, &request)?
            || b.campaign_root_id != request.root_id
            || b.obligation_id != request.obligation_id
            || b.candidate_digest != request.candidate.receipt_sha256
            || b.selection_digest != request.selection.campaign_digest
            || b.profile_digest != request.profile_digest
            || b.prepared_request_sha256 != request.request_digest
            || self.selection.candidate_commit != request.candidate.commit
            || self.selection.candidate_tree != request.candidate.tree
            || self.selection.candidate_patch_identity != request.candidate.patch_identity
            || self.selection.selection_id != request.selection.selection_id
        {
            return Err(EvidenceReadError::Conflicting(
                "campaign request differs from qualified prompt",
            ));
        }
        self.applied_request = Some(AppliedRequestBinding {
            request,
            prompt_template_sha256: sha256(template),
            template: template.to_vec(),
        });
        Ok(self)
    }

    /// Descriptive campaign physical-profile binding used only by RP2's
    /// Applied entry check. Setting it cannot construct an entry permit.
    pub fn with_applied_physical_profile(
        mut self,
        digest: &str,
    ) -> Result<Self, EvidenceReadError> {
        require_sha(digest)?;
        self.applied_physical_profile_digest = Some(digest.into());
        Ok(self)
    }
    #[cfg(feature = "test-support")]
    pub fn bind_offline_peer(
        profile: TrustedClaudeProfile,
        selection: QualifiedSelection,
    ) -> Result<Self, EvidenceReadError> {
        Self::bind_checked(profile, selection, None)
    }

    #[cfg(feature = "test-support")]
    pub fn bind_offline_guarded_peer(
        profile: TrustedClaudeProfile,
        selection: QualifiedSelection,
        boundary: Arc<CandidateBoundary>,
    ) -> Result<Self, EvidenceReadError> {
        if profile.installed() {
            return Err(EvidenceReadError::Unsupported(
                "installed profile is not offline",
            ));
        }
        Self::bind_checked(profile, selection, Some(boundary))
    }

    #[cfg(feature = "test-support")]
    pub fn rebind_offline_guarded_peer(
        self,
        profile: TrustedClaudeProfile,
        boundary: Arc<CandidateBoundary>,
    ) -> Result<Self, EvidenceReadError> {
        let mut selection = self.selection;
        selection.provenance.binding.execution_profile_digest =
            Some(profile.profile_digest().into());
        if self.applied_physical_profile_digest.is_none() {
            selection.provenance.binding.profile_digest = profile.profile_digest().into();
        }
        selection.cbm_project = boundary.index().project().into();
        selection.cbm_store_sha256 = boundary.index().store_sha256().into();
        selection.cbm_generation = boundary.index().indexed_at().into();
        selection.cbm_coverage_sha256 = boundary.index().coverage_sha256().into();
        Self::bind_offline_guarded_peer(profile, selection, boundary)
    }

    pub fn bind_installed(
        profile: TrustedClaudeProfile,
        selection: QualifiedSelection,
        boundary: Arc<CandidateBoundary>,
    ) -> Result<Self, EvidenceReadError> {
        if !profile.installed() {
            return Err(EvidenceReadError::Unsupported(
                "offline profile is not installed",
            ));
        }
        Self::bind_checked(profile, selection, Some(boundary))
    }

    #[cfg_attr(not(feature = "test-support"), allow(dead_code))]
    fn bind_checked(
        profile: TrustedClaudeProfile,
        selection: QualifiedSelection,
        boundary: Option<Arc<CandidateBoundary>>,
    ) -> Result<Self, EvidenceReadError> {
        profile
            .verify()
            .map_err(|_| EvidenceReadError::Conflicting("Claude executable drift"))?;
        selection.provenance.validate()?;
        for value in [
            &selection.root_id,
            &selection.selection_id,
            &selection.candidate_commit,
            &selection.candidate_tree,
            &selection.candidate_patch_identity,
            &selection.cbm_project,
            &selection.cbm_generation,
        ] {
            require_text(value)?;
        }
        for digest in [
            &selection.candidate_tree_sha256,
            &selection.cbm_store_sha256,
            &selection.cbm_coverage_sha256,
        ] {
            require_sha(digest)?;
        }
        if selection
            .provenance
            .binding
            .execution_profile_digest
            .as_deref()
            .unwrap_or(&selection.provenance.binding.profile_digest)
            != profile.profile_digest()
        {
            return Err(EvidenceReadError::Conflicting(
                "Claude profile binding mismatch",
            ));
        }
        if profile.installed() && boundary.is_none() {
            return Err(EvidenceReadError::Unsupported(
                "installed Claude boundary absent",
            ));
        }
        if profile.installed() && selection.credential.is_none() {
            return Err(EvidenceReadError::Unsupported(
                "installed Claude access source absent",
            ));
        }
        if boundary.is_some() && profile.guard().is_none() {
            return Err(EvidenceReadError::Unsupported(
                "guarded Claude profile lacks pinned guard",
            ));
        }
        if let Some(boundary) = &boundary {
            boundary
                .verify_frozen()
                .map_err(|_| EvidenceReadError::Conflicting("Claude boundary drift"))?;
            let index = boundary.index();
            if boundary.candidate_path() != selection.candidate_path
                || index.project() != selection.cbm_project
                || index.candidate_tree_sha256() != selection.candidate_tree_sha256
                || index.store_sha256() != selection.cbm_store_sha256
                || index.coverage_sha256() != selection.cbm_coverage_sha256
                || index.indexed_at() != selection.cbm_generation
                || !index.coverage_complete()
            {
                return Err(EvidenceReadError::Conflicting(
                    "Claude CBM/candidate binding mismatch",
                ));
            }
        }
        let session_id = selection.provenance.execution_session_id.as_deref().ok_or(
            EvidenceReadError::Conflicting("Claude selected session absent"),
        )?;
        require_text(session_id)?;
        if selection.prompt.is_empty()
            || selection.prompt.len() > MAX_PROMPT_BYTES
            || std::str::from_utf8(&selection.prompt).is_err()
        {
            return Err(EvidenceReadError::Capacity(
                "Claude prompt invalid or oversized",
            ));
        }
        if selection.horizon.is_zero() || selection.horizon > MAX_ATTEMPT_HORIZON {
            return Err(EvidenceReadError::Unsupported("Claude attempt horizon"));
        }
        if selection
            .offline_witness
            .as_ref()
            .is_some_and(|path| !path.is_absolute())
        {
            return Err(EvidenceReadError::Unsupported(
                "offline witness path must be absolute",
            ));
        }
        if !selection.candidate_path.is_absolute()
            || selection
                .candidate_path
                .canonicalize()
                .map_err(|_| EvidenceReadError::Unresolved("candidate inaccessible"))?
                != selection.candidate_path
            || !selection.candidate_path.is_dir()
            || tree_digest(&selection.candidate_path)
                .map_err(|_| EvidenceReadError::Unresolved("candidate unreadable"))?
                != selection.candidate_tree_sha256
        {
            return Err(EvidenceReadError::Conflicting("candidate tree drift"));
        }
        let subject_bytes = canonical_json(&selection.subject)
            .map_err(|_| EvidenceReadError::Conflicting("Claude subject invalid"))?;
        if selection.subject.get("commit") != Some(&JsValue::text(&selection.candidate_commit))
            || selection.subject.get("tree") != Some(&JsValue::text(&selection.candidate_tree))
            || selection.subject.get("patchIdentity")
                != Some(&JsValue::text(&selection.candidate_patch_identity))
        {
            return Err(EvidenceReadError::Conflicting(
                "Claude subject binding mismatch",
            ));
        }
        review_episode_core::implementation_review_v1::validate_subject(
            &review_episode_core::codec::parse_json(&subject_bytes)
                .map_err(|_| EvidenceReadError::Conflicting("episode subject JSON invalid"))?,
        )
        .map_err(|_| EvidenceReadError::Conflicting("episode subject invalid"))?;
        let command_sha256 = command_digest(&profile, session_id, boundary.is_some());
        let b = &selection.provenance.binding;
        let prepared = Prepared {
            schema_version: 1,
            root_id: selection.root_id.clone(),
            campaign_root_id: b.campaign_root_id.clone(),
            obligation_id: b.obligation_id.clone(),
            candidate_digest: b.candidate_digest.clone(),
            selection_digest: b.selection_digest.clone(),
            profile_digest: b.profile_digest.clone(),
            execution_profile_digest: b.execution_profile_digest.clone(),
            prepared_request_sha256: b.prepared_request_sha256.clone(),
            review_episode_id: b.review_episode_id.clone(),
            attempt_id: b.attempt_id.clone(),
            dispatch_operation_id: selection.provenance.dispatch_operation_id.clone(),
            dispatch_revision: selection.provenance.dispatch_revision.clone(),
            selection_id: selection.selection_id.clone(),
            candidate_commit: selection.candidate_commit.clone(),
            candidate_tree: selection.candidate_tree.clone(),
            candidate_patch_identity: selection.candidate_patch_identity.clone(),
            candidate_tree_sha256: selection.candidate_tree_sha256.clone(),
            cbm_project: selection.cbm_project.clone(),
            cbm_store_sha256: selection.cbm_store_sha256.clone(),
            cbm_generation: selection.cbm_generation.clone(),
            cbm_coverage_sha256: selection.cbm_coverage_sha256.clone(),
            command_sha256,
            executable_sha256: profile.program().sha256().into(),
            guard_sha256: profile.guard().map(|guard| guard.sha256().into()),
            catalog_sha256: profile.catalog_sha256().into(),
            prompt_sha256: sha256(&selection.prompt),
            subject_sha256: sha256(subject_bytes.as_bytes()),
            requested_model: profile.requested_model().into(),
            requested_effort: profile.requested_effort().into(),
            requested_session_id: session_id.into(),
            attempt_horizon_millis: selection.horizon.as_millis() as u64,
            provenance_class: if profile.installed() {
                "qualified_provider".into()
            } else {
                "offline_peer".into()
            },
            boundary_kind: if boundary.is_some() {
                "rp0-guarded-v1".into()
            } else {
                "offline-direct-v1".into()
            },
        };
        Ok(Self {
            profile,
            selection,
            prepared,
            boundary,
            applied_physical_profile_digest: None,
            applied_request: None,
        })
    }

    pub fn provenance(&self) -> &ExecutionProvenance {
        &self.selection.provenance
    }

    pub fn reader_selection(&self) -> Result<QualifiedReaderSelection, EvidenceReadError> {
        Ok(QualifiedReaderSelection {
            pin: self.prepared.pin()?,
            profile: self.profile.clone(),
            session_id: self.prepared.requested_session_id.clone(),
            attempt_id: self.prepared.attempt_id.clone(),
        })
    }

    pub(crate) fn preflight(&self) -> Result<(), EvidenceReadError> {
        self.profile
            .verify()
            .map_err(|_| EvidenceReadError::Conflicting("Claude executable drift"))?;
        if self.applied_request.is_some() {
            slice_campaign::verify_candidate_snapshot(
                &self.selection.candidate_path,
                &self.selection.candidate_tree,
            )
            .map_err(|_| EvidenceReadError::Conflicting("selected Git snapshot drift"))?;
        }
        if tree_digest(&self.selection.candidate_path)
            .map_err(|_| EvidenceReadError::Unresolved("candidate unreadable"))?
            != self.prepared.candidate_tree_sha256
        {
            return Err(EvidenceReadError::Conflicting("candidate tree drift"));
        }
        if sha256(&self.selection.prompt) != self.prepared.prompt_sha256 {
            return Err(EvidenceReadError::Conflicting("Claude prompt drift"));
        }
        if let Some(boundary) = &self.boundary {
            boundary
                .verify_frozen()
                .map_err(|_| EvidenceReadError::Conflicting("Claude boundary drift"))?;
        }
        Ok(())
    }

    pub(crate) fn arguments(&self) -> Vec<String> {
        command_arguments(
            &self.profile,
            &self.prepared.requested_session_id,
            self.boundary.is_some(),
        )
    }
}

/// A non-replayable local entry token. Only the feature-gated offline owner can
/// mint one in RP1. It is separate from every locator and checked read result.
///
/// ```compile_fail
/// use review_execution_evidence::claude::{InitialEntryPermit, SessionLocator};
/// let locator = SessionLocator { schema_version: 1, root_id: "r".into(),
///     profile: "p".into(), attempt_id: "a".into(), session_id: "s".into(),
///     manifest_sha256: "0".repeat(64) };
/// let permit: InitialEntryPermit = locator.into();
/// ```
pub struct InitialEntryPermit {
    attempt_id: String,
    profile_digest: String,
    offline_only: bool,
}

impl InitialEntryPermit {
    #[cfg(feature = "test-support")]
    pub fn offline_peer(scope: &QualifiedScope) -> Self {
        assert!(
            !scope.profile.installed(),
            "offline permit cannot admit installed Claude profile"
        );
        Self {
            attempt_id: scope.prepared.attempt_id.clone(),
            profile_digest: scope.prepared.execution_profile_digest().to_owned(),
            offline_only: true,
        }
    }

    pub(crate) fn applied(scope: &QualifiedScope, offline_only: bool) -> Self {
        Self {
            attempt_id: scope.prepared.attempt_id.clone(),
            profile_digest: scope.prepared.execution_profile_digest().to_owned(),
            offline_only,
        }
    }

    pub(crate) fn consume(self, scope: &QualifiedScope) -> Result<(), EvidenceReadError> {
        if self.attempt_id != scope.prepared.attempt_id
            || self.profile_digest != scope.prepared.execution_profile_digest()
            || (self.offline_only && scope.profile.installed())
        {
            return Err(EvidenceReadError::Conflicting(
                "Claude entry permit mismatch",
            ));
        }
        Ok(())
    }
}

/// Authorizes custody of one later session snapshot, without authorizing any
/// model call. RP1 has only a feature-gated offline constructor; RP2 must bind
/// a production successor to its own Applied owner before adding one.
pub struct SessionEvolutionPermit {
    next_attempt_id: String,
    predecessor_manifest_sha256: String,
    profile_digest: String,
    offline_only: bool,
}

impl SessionEvolutionPermit {
    #[cfg(feature = "test-support")]
    pub fn offline_peer(
        scope: &QualifiedScope,
        prior: &RetainedReviewSession,
        next_attempt_id: &str,
    ) -> Result<Self, EvidenceReadError> {
        if scope.profile.installed() {
            return Err(EvidenceReadError::Unsupported(
                "installed session evolution permit unavailable",
            ));
        }
        require_text(next_attempt_id)?;
        let locator = prior.locator();
        if next_attempt_id == locator.attempt_id {
            return Err(EvidenceReadError::Conflicting(
                "successor attempt reuses predecessor",
            ));
        }
        Ok(Self {
            next_attempt_id: next_attempt_id.into(),
            predecessor_manifest_sha256: locator.manifest_sha256,
            profile_digest: scope.prepared.execution_profile_digest().to_owned(),
            offline_only: true,
        })
    }

    pub(crate) fn consume(
        self,
        scope: &QualifiedScope,
        prior: &RetainedReviewSession,
    ) -> Result<String, EvidenceReadError> {
        if self.profile_digest != scope.prepared.execution_profile_digest()
            || self.predecessor_manifest_sha256 != prior.locator().manifest_sha256
            || (self.offline_only && scope.profile.installed())
        {
            return Err(EvidenceReadError::Conflicting(
                "session evolution permit mismatch",
            ));
        }
        Ok(self.next_attempt_id)
    }
}

#[cfg(test)]
mod tests {
    use review_execution_custody::claude::selection::REVIEW_SCHEMA;

    #[test]
    fn review_schema_has_declared_required_fields() {
        let schema: serde_json::Value = serde_json::from_str(REVIEW_SCHEMA).unwrap();
        for field in schema["required"].as_array().unwrap() {
            let name = field.as_str().unwrap();
            assert!(schema["properties"].get(name).is_some(), "{name}");
        }
    }
}
