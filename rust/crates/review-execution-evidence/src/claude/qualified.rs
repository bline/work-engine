use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use claim_evidence::codec::{JsValue, canonical_json, parse_json};
use serde::{Deserialize, Serialize};

use crate::contract::{record_digest, require_sha, require_text, sha256};
use crate::store::{CLAUDE_PROFILE, CLAUDE_TEST_PROFILE, ReaderPin};
use crate::{EvidenceReadError, ExecutionProvenance, REQUEST_MAX_BYTES};

use super::isolation::{CandidateBoundary, tree_digest};
use super::session::RetainedReviewSession;
use super::{CredentialSource, TrustedClaudeProfile};

const MAX_PROMPT_BYTES: usize = 1024 * 1024;
const MAX_ATTEMPT_HORIZON: Duration = Duration::from_secs(600);
const CLAUDE_TOOLS: &str = "Read,Glob,Grep,mcp__codebase-memory-mcp__search_graph,mcp__codebase-memory-mcp__trace_path,mcp__codebase-memory-mcp__get_code_snippet,mcp__codebase-memory-mcp__check_index_coverage,mcp__codebase-memory-mcp__index_status,mcp__codebase-memory-mcp__query_graph,mcp__codebase-memory-mcp__get_architecture";
// The CLI schema guides generation; ReviewResult::parse remains the authority
// for cross-field rules, unique finding IDs, and exact result admission.
const REVIEW_SCHEMA: &str = r##"{
  "type":"object",
  "properties":{
    "schemaVersion":{"type":"integer","const":1},
    "subject":{"type":"object","properties":{
      "commit":{"type":"string","minLength":1},
      "tree":{"type":"string","minLength":1},
      "patchIdentity":{"type":"string","minLength":1}},
      "required":["commit","tree","patchIdentity"],"additionalProperties":false},
    "verdict":{"type":"string","enum":["acceptable_as_is","remediation_required","incomplete"]},
    "findings":{"type":"array","items":{"type":"object","properties":{
      "id":{"type":"string","minLength":1},
      "severity":{"type":"string","enum":["blocker","high","medium","low","info"]},
      "title":{"type":"string","minLength":1},
      "evidence":{"$ref":"#/$defs/evidence"},
      "observed":{"type":"string","minLength":1},
      "violatedExpectation":{"type":"string","minLength":1},
      "consequence":{"type":"string","minLength":1},
      "basis":{"type":"string","enum":["reproduced","inferred"]},
      "confidence":{"type":"string","enum":["high","medium","low"]},
      "recommendedRemediation":{"type":"string","minLength":1},
      "status":{"type":"string","enum":["open","remediation_presented","verified_resolved","withdrawn","unresolved"]},
      "remediationEvidence":{"type":"array","items":{"$ref":"#/$defs/evidenceItem"}}},
      "required":["id","severity","title","evidence","observed","violatedExpectation",
        "consequence","basis","confidence","recommendedRemediation","status","remediationEvidence"],
      "additionalProperties":false}},
    "decisiveEvidence":{"type":"array","items":{"$ref":"#/$defs/evidenceItem"}},
    "limitations":{"type":"array","items":{"type":"string","minLength":1}}
  },
  "$defs":{
    "evidence":{"type":"array","minItems":1,"items":{"$ref":"#/$defs/evidenceItem"}},
    "evidenceItem":{"type":"object","properties":{
      "path":{"type":"string","minLength":1},
      "startLine":{"type":"integer","minimum":1},
      "endLine":{"type":"integer","minimum":1},
      "sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}},
      "required":["path","startLine","endLine","sha256"],"additionalProperties":false}
  },
  "required":["schemaVersion","subject","verdict","findings","decisiveEvidence","limitations"],
  "additionalProperties":false
}"##;

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
}

#[derive(Clone)]
pub struct QualifiedReaderSelection {
    pub(crate) pin: ReaderPin,
    pub(crate) profile: TrustedClaudeProfile,
    pub(crate) session_id: String,
    pub(crate) attempt_id: String,
}

impl QualifiedReaderSelection {
    pub fn reader_pin(&self) -> &ReaderPin {
        &self.pin
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Prepared {
    pub schema_version: u32,
    pub root_id: String,
    pub campaign_root_id: String,
    pub obligation_id: String,
    pub candidate_digest: String,
    pub selection_digest: String,
    pub profile_digest: String,
    pub prepared_request_sha256: String,
    pub review_episode_id: String,
    pub attempt_id: String,
    pub dispatch_operation_id: String,
    pub dispatch_revision: String,
    pub selection_id: String,
    pub candidate_commit: String,
    pub candidate_tree: String,
    pub candidate_patch_identity: String,
    pub candidate_tree_sha256: String,
    pub cbm_project: String,
    pub cbm_store_sha256: String,
    pub cbm_generation: String,
    pub cbm_coverage_sha256: String,
    pub command_sha256: String,
    pub executable_sha256: String,
    pub guard_sha256: Option<String>,
    pub catalog_sha256: String,
    pub prompt_sha256: String,
    pub subject_sha256: String,
    pub requested_model: String,
    pub requested_effort: String,
    pub requested_session_id: String,
    pub attempt_horizon_millis: u64,
    pub provenance_class: String,
    pub boundary_kind: String,
}

impl Prepared {
    pub(crate) fn provenance(&self) -> ExecutionProvenance {
        use crate::ExecutionBinding;
        ExecutionProvenance {
            binding: ExecutionBinding {
                campaign_root_id: self.campaign_root_id.clone(),
                obligation_id: self.obligation_id.clone(),
                candidate_digest: self.candidate_digest.clone(),
                selection_digest: self.selection_digest.clone(),
                profile_digest: self.profile_digest.clone(),
                prepared_request_sha256: self.prepared_request_sha256.clone(),
                review_episode_id: self.review_episode_id.clone(),
                attempt_id: self.attempt_id.clone(),
            },
            dispatch_operation_id: self.dispatch_operation_id.clone(),
            dispatch_revision: self.dispatch_revision.clone(),
            execution_session_id: Some(self.requested_session_id.clone()),
        }
    }

    pub(crate) fn pin(&self) -> Result<ReaderPin, EvidenceReadError> {
        let bytes = serde_json::to_vec(self)
            .map_err(|_| EvidenceReadError::Conflicting("Claude prepared record invalid"))?;
        Ok(ReaderPin {
            root_id: self.root_id.clone(),
            configuration_sha256: record_digest(&[
                if self.provenance_class == "qualified_provider" {
                    b"claude-qualified-scope-v1"
                } else {
                    b"claude-offline-scope-v1"
                },
                &bytes,
            ]),
            executable_sha256: self.executable_sha256.clone(),
            source_sha256: self.candidate_tree_sha256.clone(),
            profile: if self.provenance_class == "qualified_provider" {
                CLAUDE_PROFILE.into()
            } else if self.provenance_class == "offline_peer" {
                CLAUDE_TEST_PROFILE.into()
            } else {
                return Err(EvidenceReadError::Unsupported("Claude provenance class"));
            },
        })
    }
}

impl QualifiedScope {
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
        selection.provenance.binding.profile_digest = profile.profile_digest().into();
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
        if selection.provenance.binding.profile_digest != profile.profile_digest() {
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
        })
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

pub(crate) fn command_digest(
    profile: &TrustedClaudeProfile,
    session_id: &str,
    guarded: bool,
) -> String {
    let arguments = command_arguments(profile, session_id, guarded);
    record_digest(&[
        b"claude-command-v1",
        profile.program().sha256().as_bytes(),
        profile
            .guard()
            .map_or("", |guard| guard.sha256())
            .as_bytes(),
        arguments.join("\0").as_bytes(),
    ])
}

fn command_arguments(
    profile: &TrustedClaudeProfile,
    session_id: &str,
    guarded: bool,
) -> Vec<String> {
    let mut args = vec![
        "-p".into(),
        "--effort".into(),
        profile.requested_effort().into(),
        "--model".into(),
        profile.requested_model().into(),
        "--session-id".into(),
        session_id.into(),
    ];
    if guarded {
        args.extend(
            [
                "--strict-mcp-config",
                "--mcp-config",
                "/cbm/mcp.json",
                "--tools",
                CLAUDE_TOOLS,
                "--json-schema",
                REVIEW_SCHEMA,
                "--dangerously-skip-permissions",
            ]
            .into_iter()
            .map(str::to_owned),
        );
    }
    args.extend(["--output-format".into(), "json".into()]);
    args
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
            profile_digest: scope.prepared.profile_digest.clone(),
            offline_only: true,
        }
    }

    pub(crate) fn consume(self, scope: &QualifiedScope) -> Result<(), EvidenceReadError> {
        if self.attempt_id != scope.prepared.attempt_id
            || self.profile_digest != scope.prepared.profile_digest
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
            profile_digest: scope.prepared.profile_digest.clone(),
            offline_only: true,
        })
    }

    pub(crate) fn consume(
        self,
        scope: &QualifiedScope,
        prior: &RetainedReviewSession,
    ) -> Result<String, EvidenceReadError> {
        if self.profile_digest != scope.prepared.profile_digest
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

pub(crate) fn parse_stored_subject(raw: &[u8]) -> Result<JsValue, EvidenceReadError> {
    let text = std::str::from_utf8(raw)
        .map_err(|_| EvidenceReadError::Conflicting("stored subject text invalid"))?;
    parse_json(text).map_err(|_| EvidenceReadError::Conflicting("stored subject JSON invalid"))
}

pub(crate) fn validate_stored_prompt(
    raw: &[u8],
    prepared: &Prepared,
) -> Result<(), EvidenceReadError> {
    if raw.len() > REQUEST_MAX_BYTES || sha256(raw) != prepared.prompt_sha256 {
        return Err(EvidenceReadError::Conflicting(
            "stored prompt binding mismatch",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::REVIEW_SCHEMA;

    #[test]
    fn review_schema_has_declared_required_fields() {
        let schema: serde_json::Value = serde_json::from_str(REVIEW_SCHEMA).unwrap();
        for field in schema["required"].as_array().unwrap() {
            let name = field.as_str().unwrap();
            assert!(schema["properties"].get(name).is_some(), "{name}");
        }
    }
}
