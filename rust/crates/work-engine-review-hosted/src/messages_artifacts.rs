//! Read-only Messages artifact mechanics for a new, unselected revision.
//! No launch, provider, credential, or owner-result API. A comparison does not
//! establish who produced the artifacts, what ran, or what was served.

use std::{collections::BTreeMap, path::Path};

use review_episode_core::{codec::parse_json, implementation_review_v1::ReviewResult};
use serde::Serialize;
use work_engine_review_delivery::wire;
use work_engine_review_preparation::Subject;

use crate::{
    Refusal,
    check::{AnalysisResult, RequestInsertions, RetainedRequest, messages_body},
    revision::{CLAUDE_TOOLS, Profile},
    sha256_hex,
    subject::{self, Snapshot},
    task::{self, Limits},
};

pub const REVISION: &str = "milestone-a-messages-artifacts-v1";

/// Locally prepared bytes. Caller-supplied contract and expectation are not
/// verified owner inputs; the caller must bind these independently of the return.
#[derive(Debug)]
pub struct MessagesMaterialization {
    subject: Subject,
    subject_digest: String,
    requested_model: String,
    task: String,
    manifest: Vec<u8>,
    snapshot: Snapshot,
    limits: Limits,
}

#[derive(Serialize)]
struct ManifestMember {
    kind: &'static str,
    sha256: String,
}

impl MessagesMaterialization {
    /// Read Git objects only, from the selected repository. No artifact is
    /// published. The writer can pin the exact task and manifest bytes before use.
    pub fn prepare(
        repo: &Path,
        expected_baseline: &str,
        expected_subject: &Subject,
        expected_scope_digest: &str,
        contract: &str,
        requested_model: &str,
        limits: Limits,
    ) -> Result<Self, Refusal> {
        if requested_model.trim().is_empty() {
            return Err(Refusal::Artifact("requested model missing".into()));
        }
        let task = task::render_for(Profile::ClaudeSdk, contract, expected_subject, limits)?;
        let snapshot = subject::materialize(repo, expected_subject, expected_scope_digest)?;
        if snapshot.parent != expected_baseline {
            return Err(Refusal::BaselineMismatch {
                expected: expected_baseline.into(),
                actual: snapshot.parent,
            });
        }
        subject::check_context_fit(snapshot.bytes(), task.len())?;
        subject::check_read_budget(&snapshot, limits.max_reads)?;
        let members: BTreeMap<_, _> = snapshot
            .manifest()
            .into_iter()
            .map(|(path, sha256)| {
                (
                    path,
                    ManifestMember {
                        kind: "file",
                        sha256,
                    },
                )
            })
            .collect();
        let manifest = serde_json::to_vec(&members).map_err(|e| Refusal::Io(e.to_string()))?;
        Ok(Self {
            subject: expected_subject.clone(),
            subject_digest: expected_scope_digest.into(),
            requested_model: requested_model.into(),
            task,
            manifest,
            snapshot,
            limits,
        })
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn task(&self) -> &str {
        &self.task
    }

    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest
    }

    /// Compare candidate artifacts against the separately materialized bytes.
    /// Attempt digests and the answer digest are supplied artifacts (not Host
    /// observations). An independent custody reader must bind them to the exact
    /// campaign/transport and actual execution before relying on the comparison.
    pub fn compare(
        &self,
        retained_manifest: &[u8],
        requests: &[RetainedRequest],
        result: &AnalysisResult,
    ) -> Result<MessagesArtifactComparison, Refusal> {
        if retained_manifest != self.manifest {
            return Err(Refusal::Artifact("manifest bytes differ".into()));
        }
        if requests.is_empty() || requests.len() > self.limits.max_requests as usize {
            return Err(Refusal::Artifact("request count out of bounds".into()));
        }
        let tools: Vec<_> = CLAUDE_TOOLS.into_iter().map(str::to_owned).collect();
        let mut request_insertions = Vec::with_capacity(requests.len());
        for (index, request) in requests.iter().enumerate() {
            let n = index as u64 + 1;
            let body = request
                .body
                .as_deref()
                .ok_or_else(|| Refusal::Artifact(format!("request {n}: body missing")))?;
            let body_sha = sha256_hex(body);
            if request.request != n
                || !matches!(request.attempt_digests.as_slice(), [Some(sha)] if sha == &body_sha)
            {
                return Err(Refusal::Artifact(format!(
                    "request {n}: numbering or single attempt body digest differs"
                )));
            }
            let parsed = wire::parse(body).map_err(|_| {
                Refusal::Artifact(format!("request {n}: ambiguous or invalid JSON"))
            })?;
            request_insertions.push(
                messages_body(n, &parsed, &self.requested_model, &tools, self.task())
                    .map_err(Refusal::Artifact)?,
            );
        }
        let answer_sha256 = sha256_hex(result.answer.as_bytes());
        if answer_sha256 != result.answer_sha256 {
            return Err(Refusal::Artifact("answer digest differs".into()));
        }
        // The episode codec owns subject and result semantics. Reject ambiguous
        // duplicate keys before using it; it alone would normalize duplicate keys.
        wire::parse(result.answer.as_bytes())
            .map_err(|_| Refusal::Artifact("answer is not one unambiguous JSON value".into()))?;
        let answer = parse_json(&result.answer)
            .and_then(ReviewResult::parse)
            .map_err(|_| {
                Refusal::Artifact("answer differs from the review result contract".into())
            })?;
        if answer.value.get("subject") != Some(&self.subject.to_js_value()) {
            return Err(Refusal::Artifact("answer subject differs".into()));
        }
        Ok(MessagesArtifactComparison {
            revision: REVISION,
            subject_digest: self.subject_digest.clone(),
            task_sha256: sha256_hex(self.task.as_bytes()),
            manifest_sha256: sha256_hex(retained_manifest),
            answer_sha256,
            request_insertions,
        })
    }
}

/// Byte/shape comparison only. Not an admission, execution result or claim receipt.
#[derive(Debug, Serialize)]
pub struct MessagesArtifactComparison {
    pub revision: &'static str,
    pub subject_digest: String,
    pub task_sha256: String,
    pub manifest_sha256: String,
    pub answer_sha256: String,
    pub request_insertions: Vec<RequestInsertions>,
}
