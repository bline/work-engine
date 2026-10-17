use crate::claude::profile::TrustedClaudeProfile;
use crate::contract::{record_digest, sha256};
use crate::store_read::{CLAUDE_PROFILE, CLAUDE_TEST_PROFILE};
use crate::{EvidenceReadError, ExecutionProvenance, REQUEST_MAX_BYTES, ReaderPin};
use claim_evidence::codec::{JsValue, parse_json};
use serde::{Deserialize, Serialize};

const CLAUDE_TOOLS: &str = "Read,Glob,Grep,mcp__codebase-memory-mcp__search_graph,mcp__codebase-memory-mcp__trace_path,mcp__codebase-memory-mcp__get_code_snippet,mcp__codebase-memory-mcp__check_index_coverage,mcp__codebase-memory-mcp__index_status,mcp__codebase-memory-mcp__query_graph,mcp__codebase-memory-mcp__get_architecture";
// The CLI schema guides generation; ReviewResult::parse remains the authority
// for cross-field rules, unique finding IDs, and exact result admission.
pub const REVIEW_SCHEMA: &str = r##"{
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
pub struct QualifiedReaderSelection {
    pub pin: ReaderPin,
    pub profile: TrustedClaudeProfile,
    pub session_id: String,
    pub attempt_id: String,
}

impl QualifiedReaderSelection {
    pub fn reader_pin(&self) -> &ReaderPin {
        &self.pin
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prepared {
    pub schema_version: u32,
    pub root_id: String,
    pub campaign_root_id: String,
    pub obligation_id: String,
    pub candidate_digest: String,
    pub selection_digest: String,
    pub profile_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_profile_digest: Option<String>,
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
    pub fn execution_profile_digest(&self) -> &str {
        self.execution_profile_digest
            .as_deref()
            .unwrap_or(&self.profile_digest)
    }

    pub fn provenance(&self) -> ExecutionProvenance {
        use crate::ExecutionBinding;
        ExecutionProvenance {
            binding: ExecutionBinding {
                campaign_root_id: self.campaign_root_id.clone(),
                obligation_id: self.obligation_id.clone(),
                candidate_digest: self.candidate_digest.clone(),
                selection_digest: self.selection_digest.clone(),
                profile_digest: self.profile_digest.clone(),
                execution_profile_digest: self.execution_profile_digest.clone(),
                prepared_request_sha256: self.prepared_request_sha256.clone(),
                review_episode_id: self.review_episode_id.clone(),
                attempt_id: self.attempt_id.clone(),
            },
            dispatch_operation_id: self.dispatch_operation_id.clone(),
            dispatch_revision: self.dispatch_revision.clone(),
            execution_session_id: Some(self.requested_session_id.clone()),
        }
    }

    pub fn pin(&self) -> Result<ReaderPin, EvidenceReadError> {
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

pub fn command_digest(profile: &TrustedClaudeProfile, session_id: &str, guarded: bool) -> String {
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

pub fn command_arguments(
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
pub fn parse_stored_subject(raw: &[u8]) -> Result<JsValue, EvidenceReadError> {
    let text = std::str::from_utf8(raw)
        .map_err(|_| EvidenceReadError::Conflicting("stored subject text invalid"))?;
    parse_json(text).map_err(|_| EvidenceReadError::Conflicting("stored subject JSON invalid"))
}

pub fn validate_stored_prompt(raw: &[u8], prepared: &Prepared) -> Result<(), EvidenceReadError> {
    if raw.len() > REQUEST_MAX_BYTES || sha256(raw) != prepared.prompt_sha256 {
        return Err(EvidenceReadError::Conflicting(
            "stored prompt binding mismatch",
        ));
    }
    Ok(())
}
