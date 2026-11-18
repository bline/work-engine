//! Result check and the non-consumable envelope (design review P2-2, P1-2(c), P3-1, P3-3; H1
//! review P2-2, P2-3, P3-1, P3-2, P3-7).
//!
//! The answer is kept only as a string inside an [`Envelope`] whose standing fields are fixed:
//! `owner_pending`, no organizational admission, not consumable, route qualification only. No
//! check here establishes a condition; a passing envelope is still not a review result anyone may
//! consume.

use review_episode_core::codec::{JsValue, digest, parse_json};
use review_episode_core::implementation_review_v1::ReviewResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

use crate::inputs::JobInputs;
use crate::revision::{self, Profile, Wire};
use crate::subject::REVIEW_PATCH;
use crate::{Refusal, sha256_hex, task};

/// v3: the envelope names its provider `profile`; on the Messages wire `host_reported` carries the
/// request insertions' digests; the frozen world may be `prepared_conditions_unmet`.
pub const ENVELOPE_SCHEMA: &str = "work-engine.review-hosted-envelope.v3";

/// One retained provider request: the body the Host retained create-only
/// (`coding-hosted-{request}.body.json`, `work-engine-manual-host/src/pi_coding_hosted.rs:521-539`)
/// and every attempt's recorded `request_body_sha256` (`pi_coding_provider.rs:110-114`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedRequest {
    pub request: u64,
    pub body: Option<Vec<u8>>,
    pub attempt_digests: Vec<Option<String>>,
}

/// `analysis-result.json` (`pi-pressure-context-host/src/analysis.rs:293-302`): the answer and its
/// digest, plus the Host-reported members carried into the envelope as reported (P3-1).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AnalysisResult {
    pub answer: String,
    pub answer_sha256: String,
    #[serde(default)]
    pub settlement: Option<Value>,
    #[serde(default)]
    pub provider_requests: Option<Value>,
    #[serde(default)]
    pub snapshot_deliveries: Option<Value>,
    #[serde(default)]
    pub hosted_attempts: Option<Value>,
}

impl AnalysisResult {
    pub fn new(answer: String, answer_sha256: String) -> Self {
        Self {
            answer,
            answer_sha256,
            settlement: None,
            provider_requests: None,
            snapshot_deliveries: None,
            hosted_attempts: None,
        }
    }
}

/// The run's own job artifacts, as `analyze.py` writes them under `<base>/`: `inputs/snapshot-manifest.json`
/// (`analyze.py:64`, pinned by `job.json` `workspace.manifest` at `:102`), `inputs/packet.json`
/// (`:96-97`), `inputs/authority.json` (`:90-95`) and `job.json` (`:98-111`, `job_id` from the
/// run directory name at `:88`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunArtifacts {
    pub manifest_bytes: Vec<u8>,
    pub packet: Value,
    pub authority: Value,
    pub job: Value,
}

impl RunArtifacts {
    /// The served manifest as path → SHA-256; `None` when it is not the inventory shape
    /// (`analyze.py:31-37`: `{path: {"kind": "file", "sha256": …}}`).
    pub fn served_manifest(&self) -> Option<BTreeMap<String, String>> {
        let value: Value = serde_json::from_slice(&self.manifest_bytes).ok()?;
        value
            .as_object()?
            .iter()
            .map(|(path, entry)| {
                (entry["kind"] == "file")
                    .then(|| {
                        entry["sha256"]
                            .as_str()
                            .map(|s| (path.clone(), s.to_owned()))
                    })
                    .flatten()
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Passed,
    Failed,
    NotRun,
    /// Informational: reported for the operator, never a pass condition.
    Reported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckOutcome {
    pub id: String,
    pub outcome: Outcome,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grounding {
    /// Evidence items (decisive, finding and remediation evidence) whose `(path, sha256)` is a
    /// member of the manifest the Host served.
    pub grounded: usize,
    pub total: usize,
    /// Indices into `decisiveEvidence` that are not grounded.
    pub ungrounded_decisive: Vec<usize>,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    pub label: String,
    /// `None` when the recomputation failed (not established).
    pub run_id: Option<String>,
    pub requirements_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenWorldOutcome {
    pub outcome: String,
    /// `None` when the frozen world could not be recomputed (not established).
    pub known_incompatibilities: Option<usize>,
    /// Unestablished execution-confinement blockers (`prepared_conditions_unmet` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unestablished_confinement: Option<usize>,
}

/// What the Host's `analysis-result.json` reported, carried verbatim (P3-1). Not checked here;
/// the serving route of each attempt (and, on `claude-sdk`, the subscription billing guard's
/// outcome) is the Host's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostReported {
    pub settlement: Option<Value>,
    pub provider_requests: Option<Value>,
    pub snapshot_deliveries: Option<Value>,
    pub hosted_attempts: Option<Value>,
    /// Messages wire only: digests of Claude Code's `system[]` blocks and environment message per
    /// retained request, recorded and not interpreted. `None` on the Responses wire or when the
    /// delivery check did not pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_insertions: Option<Vec<RequestInsertions>>,
}

/// Never a bare review file: the answer is a string member, and the standing fields are fixed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub schema: String,
    pub standing: String,
    pub org_admission: String,
    pub consumable: bool,
    pub proposition: String,
    /// The provider profile the record names; every recomputation below is the profile's.
    pub profile: Profile,
    pub world: World,
    pub requested_model: String,
    /// The provider id the Host requires in request frames, not the serving endpoint (see
    /// `host_reported.hosted_attempts`).
    pub requested_provider: String,
    pub frozen_world: FrozenWorldOutcome,
    /// `None` when the recomputation failed (not established).
    pub undischarged_blockers: Option<usize>,
    pub job_id: String,
    pub subject: Value,
    pub contract_sha256: String,
    pub task_sha256: String,
    pub answer: String,
    pub answer_sha256: String,
    pub host_reported: HostReported,
    pub checks: Vec<CheckOutcome>,
    pub grounding: Option<Grounding>,
    /// `checks_passed` only when every non-informational check passed.
    pub outcome: String,
    pub not_claimed: Vec<String>,
}

/// What no envelope claims, whatever its checks say (see `CLAIMS.md`).
pub const NOT_CLAIMED: [&str; 12] = [
    "mutation: none is unestablished; only the Host-recorded boundaries are claimed",
    "execution confinement is unestablished",
    "no acceptance condition of either world is established",
    "not consumable",
    "not organizationally admitted",
    "grants are self-issued by the operator's own process",
    "the job and its runtime admission are prepared by analyze.py, not by this crate",
    "receipt by the live provider is not independently observed",
    "the serving route, payer and any subscription billing guard are Host-reported (host_reported.hosted_attempts), not checked",
    "Claude Code's system blocks and environment message are recorded by digest only, not interpreted",
    "the review's quality and correctness are unassessed",
    "the frozen candidate's conditions remain unmet and it yields no job inputs (refused on openai-codex; execution confinement unestablished on claude-sdk)",
];

struct Checks(Vec<CheckOutcome>);

impl Checks {
    fn push(&mut self, id: &str, outcome: Outcome, detail: impl Into<String>) {
        self.0.push(CheckOutcome {
            id: id.to_owned(),
            outcome,
            detail: detail.into(),
        });
    }
    fn pass(&mut self, id: &str, ok: bool, detail: impl Into<String>) -> bool {
        self.push(
            id,
            if ok { Outcome::Passed } else { Outcome::Failed },
            detail,
        );
        ok
    }
}

fn user_texts(body: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for item in body["input"].as_array().into_iter().flatten() {
        if item["role"] != "user" || !(item["type"].is_null() || item["type"] == "message") {
            continue;
        }
        match &item["content"] {
            Value::String(text) => out.push(text.clone()),
            Value::Array(parts) => out.extend(
                parts
                    .iter()
                    .filter(|p| p["type"] == "input_text")
                    .filter_map(|p| p["text"].as_str().map(str::to_owned)),
            ),
            _ => {}
        }
    }
    out
}

/// Every tool name a body declares: top-level `tools` and `additional_tools` input items (the
/// Host's own reading, `pi_coding_hosted.rs:31-45`), sorted.
fn tool_names(body: &Value) -> Vec<String> {
    let mut names: Vec<String> = body["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(
            body["input"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|i| i["type"] == "additional_tools")
                .flat_map(|i| i["tools"].as_array().into_iter().flatten()),
        )
        .map(|t| t["name"].as_str().unwrap_or("").to_owned())
        .collect();
    names.sort();
    names
}

fn count_or_not_established(count: Option<usize>) -> String {
    count.map_or_else(|| "not established".to_owned(), |n| n.to_string())
}

/// The task text plus exactly the Host suffix: starts with the task bytes, is exactly
/// [`task::HOST_SUFFIX_BYTES`] longer, and the bytes after the task hash to
/// [`task::HOST_SUFFIX_SHA256`] (the suffix text is not copied).
fn task_with_suffix(n: u64, text: &str, task: &str, what: &str) -> Result<(), String> {
    if !text.starts_with(task) {
        return Err(format!(
            "request {n}: the {what} does not start with the task bytes"
        ));
    }
    if text.len() != task.len() + task::HOST_SUFFIX_BYTES {
        return Err(format!(
            "request {n}: the {what} is not the task plus the {}-byte Host suffix",
            task::HOST_SUFFIX_BYTES
        ));
    }
    if sha256_hex(&text.as_bytes()[task.len()..]) != task::HOST_SUFFIX_SHA256 {
        return Err(format!(
            "request {n}: the {what} has the Host suffix length but not the Host suffix bytes"
        ));
    }
    Ok(())
}

/// SHA-256 of a value's `serde_json` serialization.
fn value_sha256(value: &Value) -> String {
    sha256_hex(&serde_json::to_vec(value).unwrap_or_default())
}

/// P3-3 / H1 P2-3 / P3-7, per retained body: its digest equals every attempt's recorded digest,
/// then the profile's wire check ([`responses_body`] or [`messages_body`]).
fn delivery(
    profile: Profile,
    requests: &[RetainedRequest],
    task: &str,
) -> Result<(String, Option<Vec<RequestInsertions>>), String> {
    if requests.is_empty() {
        return Err("no retained request".into());
    }
    let tools = profile.request_tools();
    let mut insertions = Vec::new();
    for (index, r) in requests.iter().enumerate() {
        let n = r.request;
        if n != index as u64 + 1 {
            return Err(format!("requests are not numbered 1..={}", requests.len()));
        }
        let body = r
            .body
            .as_ref()
            .ok_or(format!("request {n}: body not retained"))?;
        let digest = sha256_hex(body);
        if r.attempt_digests.is_empty()
            || r.attempt_digests
                .iter()
                .any(|d| d.as_deref() != Some(&digest))
        {
            return Err(format!(
                "request {n}: an attempt's request_body_sha256 is not the retained body's"
            ));
        }
        let parsed: Value =
            serde_json::from_slice(body).map_err(|_| format!("request {n}: body is not JSON"))?;
        match profile.wire() {
            Wire::Responses => responses_body(n, &parsed, profile, &tools, task)?,
            Wire::Messages => insertions.push(messages_body(n, &parsed, profile, &tools, task)?),
        }
    }
    let model = profile.model();
    Ok(match profile.wire() {
        Wire::Responses => (
            format!(
                "{} retained request bodies: model {model}, tools {tools:?}, one user text = task + Host suffix",
                requests.len()
            ),
            None,
        ),
        Wire::Messages => (
            format!(
                "{} retained Messages request bodies: model {model}, tools {tools:?}, messages[0] one text block = task + Host suffix, later user turns tool results only; insertions recorded by digest in host_reported",
                requests.len()
            ),
            Some(insertions),
        ),
    })
}

/// Responses wire (unchanged): `model` is the requested model, the declared tools are exactly the
/// profile's, and the body holds exactly one user text item = task + Host suffix.
fn responses_body(
    n: u64,
    parsed: &Value,
    profile: Profile,
    tools: &[String],
    task: &str,
) -> Result<(), String> {
    let model = profile.model();
    if parsed["model"] != model {
        return Err(format!("request {n}: model is not {model}"));
    }
    if tool_names(parsed) != tools {
        return Err(format!(
            "request {n}: declared tools are not exactly {tools:?}"
        ));
    }
    let texts = user_texts(parsed);
    if texts.len() != 1 {
        return Err(format!(
            "request {n}: {} user text items, expected exactly one",
            texts.len()
        ));
    }
    task_with_suffix(n, &texts[0], task, "user text")
}

/// The digests of what Claude Code inserts into a Messages request, recorded as Host-reported and
/// not interpreted: each `system[]` block and the role `system` (environment) message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestInsertions {
    pub request: u64,
    /// SHA-256 of each `system[]` block's `serde_json` serialization, in order (a string `system`
    /// counts as one block).
    pub system_blocks_sha256: Vec<String>,
    /// SHA-256 of the `messages[1]` role `system` message's serialization, when present.
    pub environment_message_sha256: Option<String>,
    /// Role `system` messages Claude Code inserted after a tool-result user message (for example
    /// a `<total_tokens>` notice), by `messages[]` index and digest only, never interpreted.
    #[serde(default)]
    pub inserted_system_messages: Vec<InsertedSystemMessage>,
}

/// One tolerated role `system` message after `messages[1]`: its index and SHA-256 only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsertedSystemMessage {
    pub message_index: u64,
    pub sha256: String,
}

/// Anthropic Messages wire (`claude-sdk`). Checked: `model` is the profile model
/// (`pi_coding_hosted.rs:732`); the declared tools are exactly the profile's names (the Host's
/// `check_messages_tools`, `pi_coding_analysis.rs:114-151`, at `5af34cbf`); `messages[0]` is a
/// user message whose content is exactly one text block (or a string, Claude Code's later rewrite,
/// `pi_coding_hosted.rs:2923-2925`) = task + Host suffix; a role `system` message (string or text
/// blocks only) is tolerated only at `messages[1]` (Claude Code's environment block) or directly
/// after a user message holding only `tool_result` blocks (observed `<total_tokens>` notices),
/// recorded by digest and never interpreted; every later user message holds only `tool_result`
/// blocks; assistant messages (text, `tool_use`, `thinking`) are not read, so thinking is never
/// answer or evidence here. Tolerated and recorded by digest, never interpreted: `system[]` and the
/// environment message.
fn messages_body(
    n: u64,
    parsed: &Value,
    profile: Profile,
    tools: &[String],
    task: &str,
) -> Result<RequestInsertions, String> {
    let model = profile.model();
    if parsed["model"] != model {
        return Err(format!("request {n}: model is not {model}"));
    }
    let mut names: Vec<String> = parsed["tools"]
        .as_array()
        .ok_or(format!("request {n}: no tools array"))?
        .iter()
        .map(|t| t["name"].as_str().unwrap_or("").to_owned())
        .collect();
    names.sort();
    if names != tools {
        return Err(format!(
            "request {n}: declared tools are not exactly {tools:?}"
        ));
    }
    let messages = parsed["messages"]
        .as_array()
        .filter(|m| !m.is_empty())
        .ok_or(format!("request {n}: no messages"))?;
    let first = &messages[0];
    if first["role"] != "user" {
        return Err(format!("request {n}: messages[0] is not a user message"));
    }
    let text = match &first["content"] {
        Value::String(text) => text.as_str(),
        Value::Array(blocks) => {
            let block = blocks
                .first()
                .filter(|b| b["type"] == "text")
                .and_then(|b| b["text"].as_str())
                .ok_or(format!(
                    "request {n}: messages[0]'s first content block is not a text block"
                ))?;
            if blocks.len() != 1 {
                return Err(format!(
                    "request {n}: messages[0] holds {} content blocks, expected only the task text block",
                    blocks.len()
                ));
            }
            block
        }
        _ => return Err(format!("request {n}: messages[0] has no content")),
    };
    task_with_suffix(n, text, task, "first text block of messages[0]")?;
    let mut environment = None;
    let mut inserted_system_messages = Vec::new();
    for (i, m) in messages.iter().enumerate().skip(1) {
        match m["role"].as_str() {
            Some("system") => {
                // Closed observed shape: messages[1], or directly after a user message holding
                // only tool results. Content is a string or text blocks only. Never interpreted.
                if i != 1 && !is_tool_results_only_user(&messages[i - 1]) {
                    return Err(format!(
                        "request {n}: role system message at messages[{i}]; tolerated only at messages[1] or directly after a user message holding only tool results"
                    ));
                }
                let content_ok = match &m["content"] {
                    Value::String(_) => true,
                    Value::Array(blocks) => blocks.iter().all(|b| b["type"] == "text"),
                    _ => false,
                };
                if !content_ok {
                    return Err(format!(
                        "request {n}: role system message at messages[{i}] holds content other than a string or text blocks"
                    ));
                }
                if i == 1 {
                    environment = Some(value_sha256(m));
                } else {
                    inserted_system_messages.push(InsertedSystemMessage {
                        message_index: i as u64,
                        sha256: value_sha256(m),
                    });
                }
            }
            // Not read: an assistant turn is neither delivery, answer nor evidence here.
            Some("assistant") => {}
            Some("user") => {
                if !is_tool_results_only_user(m) {
                    return Err(format!(
                        "request {n}: messages[{i}] carries user content other than tool results"
                    ));
                }
            }
            _ => return Err(format!("request {n}: messages[{i}] has an unknown role")),
        }
    }
    let system_blocks_sha256 = match &parsed["system"] {
        Value::Null => Vec::new(),
        Value::Array(blocks) => blocks.iter().map(value_sha256).collect(),
        other => vec![value_sha256(other)],
    };
    Ok(RequestInsertions {
        request: n,
        system_blocks_sha256,
        environment_message_sha256: environment,
        inserted_system_messages,
    })
}

/// A `user` message whose content is a non-empty list of only `tool_result` blocks.
fn is_tool_results_only_user(m: &Value) -> bool {
    m["role"] == "user"
        && m["content"].as_array().is_some_and(|blocks| {
            !blocks.is_empty() && blocks.iter().all(|b| b["type"] == "tool_result")
        })
}

fn evidence_items(result: &ReviewResult) -> (Vec<&JsValue>, usize) {
    let mut all = Vec::new();
    let decisive = result
        .value
        .get("decisiveEvidence")
        .and_then(|v| v.as_array().ok())
        .unwrap_or(&[]);
    all.extend(decisive.iter());
    for finding in &result.findings {
        for key in ["evidence", "remediationEvidence"] {
            if let Some(items) = finding.get(key).and_then(|v| v.as_array().ok()) {
                all.extend(items.iter());
            }
        }
    }
    (all, decisive.len())
}

fn text_of(value: &JsValue, key: &str) -> Option<String> {
    value.get(key)?.as_text().ok()?.to_string_checked().ok()
}

/// P3-1: k of n evidence items whose `(path, sha256)` is a member of `manifest`.
pub fn grounding(result: &ReviewResult, manifest: &BTreeMap<String, String>) -> Grounding {
    let (items, decisive) = evidence_items(result);
    let is_grounded = |item: &JsValue| match (text_of(item, "path"), text_of(item, "sha256")) {
        (Some(path), Some(sha)) => manifest.get(&path) == Some(&sha),
        _ => false,
    };
    let grounded = items.iter().filter(|i| is_grounded(i)).count();
    let ungrounded_decisive: Vec<usize> = items[..decisive]
        .iter()
        .enumerate()
        .filter(|(_, i)| !is_grounded(i))
        .map(|(n, _)| n)
        .collect();
    let mut flags = Vec::new();
    if !ungrounded_decisive.is_empty() {
        flags.push("evidence_ungrounded".to_owned());
    }
    Grounding {
        grounded,
        total: items.len(),
        ungrounded_decisive,
        flags,
    }
}

/// H1 P2-2: the run's job artifacts against the record. Returns the served manifest when it
/// parses (grounding is counted against it).
fn run_binding(
    checks: &mut Checks,
    record: &JobInputs,
    run: &RunArtifacts,
    scope_digest: Option<&str>,
) -> Option<BTreeMap<String, String>> {
    let served = run.served_manifest();
    let pinned = run.job["workspace"]["manifest"]["sha256"].as_str()
        == Some(sha256_hex(&run.manifest_bytes).as_str());
    checks.pass(
        "snapshot_served",
        pinned && served.as_ref() == Some(&record.snapshot_manifest),
        format!(
            "served manifest pinned by job.json: {pinned}; equal to the vetted record: {}",
            served.as_ref() == Some(&record.snapshot_manifest)
        ),
    );
    let patch = served.as_ref().and_then(|m| m.get(REVIEW_PATCH));
    let subject = &record.subject.patch_identity;
    checks.pass(
        "subject_bound_to_scope",
        scope_digest == Some(subject.as_str()) && patch == Some(subject),
        format!(
            "patchIdentity {subject}; scope.digest {scope_digest:?}; served {REVIEW_PATCH} {patch:?}"
        ),
    );
    let job_id = run.job["job_id"].as_str();
    checks.pass(
        "job_id_bound",
        job_id == Some(record.job_id.as_str())
            && run.authority["job_id"].as_str() == Some(record.job_id.as_str()),
        format!("record {}; job.json {job_id:?}", record.job_id),
    );
    let profile = record.profile;
    let job_provider = run.job["coding"]["provider"]["provider"].as_str();
    let job_model = run.job["coding"]["provider"]["model"].as_str();
    checks.pass(
        "job_profile_bound",
        job_provider == Some(profile.provider()) && job_model == Some(profile.model()),
        format!(
            "profile {} / {}; job.json {job_provider:?} / {job_model:?}",
            profile.provider(),
            profile.model()
        ),
    );
    let limits_ok = run.packet["max_provider_requests"].as_u64()
        == Some(u64::from(record.limits.max_requests))
        && run.packet["max_snapshot_reads"].as_u64() == Some(u64::from(record.limits.max_reads))
        && run.authority["task_sha256"].as_str() == Some(record.task_sha256.as_str());
    checks.pass(
        "limits_bound",
        limits_ok,
        format!(
            "record {:?}; packet requests {} reads {}; authority task_sha256 {}",
            record.limits,
            run.packet["max_provider_requests"],
            run.packet["max_snapshot_reads"],
            run.authority["task_sha256"]
        ),
    );
    served
}

/// Runs every check and returns the envelope. Never fails: a refusal is an envelope whose
/// `outcome` is `checks_failed`.
pub fn check(
    record: &JobInputs,
    run: &RunArtifacts,
    result: &AnalysisResult,
    requests: &[RetainedRequest],
) -> Envelope {
    let mut checks = Checks(Vec::new());
    let subject = &record.subject;
    let profile = record.profile;
    let rev = profile.revision();
    let (template_id, _, template_sha256) = task::template(profile);

    // Recompute candidate -> prepare -> freeze -> render from the retained inputs (P1-2(c)), all
    // on the record's profile.
    checks.pass(
        "requirements_revision",
        record.revision == rev.id && record.requirements_sha256 == sha256_hex(rev.requirements),
        format!(
            "profile {}: revision {} sha256 {}",
            profile.provider(),
            record.revision,
            record.requirements_sha256
        ),
    );
    checks.pass(
        "realization_claim",
        record.realization_claim == profile.claim()
            && record.provider == profile.provider()
            && record.requested_model == profile.model(),
        format!(
            "the recorded claim, provider and model are profile {}'s; bodies are compared in `delivery`",
            profile.provider()
        ),
    );
    let recomputed =
        revision::prepare_world_for(profile, rev.requirements, subject, &record.job_id);
    let (blockers, run_id, scope_digest, task_bytes) = match &recomputed {
        Ok(frozen) => {
            checks.pass(
                "contract_recomputed",
                frozen.contract_sha256() == record.contract_sha256
                    && *frozen.transport() == record.transport,
                format!("recomputed contract_sha256 {}", frozen.contract_sha256()),
            );
            let task = task::render_for(profile, frozen.contract(), subject, record.limits);
            let task_ok = task
                .as_ref()
                .is_ok_and(|t| sha256_hex(t.as_bytes()) == record.task_sha256);
            checks.pass(
                "task_recomputed",
                task_ok
                    && record.template_sha256 == template_sha256
                    && record.template == template_id,
                match &task {
                    Ok(t) => format!("recomputed task_sha256 {}", sha256_hex(t.as_bytes())),
                    Err(e) => e.to_string(),
                },
            );
            let prep = frozen.preparation();
            (
                Some(prep.blockers.len()),
                Some(prep.model_context.identity.run_id.clone()),
                Some(prep.model_context.obligation.scope_digest.clone()),
                task.ok().filter(|_| task_ok),
            )
        }
        Err(e) => {
            checks.pass("contract_recomputed", false, e.to_string());
            checks.push("task_recomputed", Outcome::NotRun, "no recomputed contract");
            (None, None, None, None)
        }
    };
    let frozen_world = match revision::frozen_world_for(profile, subject) {
        Ok(w) => FrozenWorldOutcome {
            known_incompatibilities: Some(w.known_incompatibilities.len()),
            unestablished_confinement: (w.outcome == "prepared_conditions_unmet")
                .then_some(w.unestablished_confinement.len()),
            outcome: w.outcome,
        },
        Err(e) => FrozenWorldOutcome {
            outcome: format!("not_established: {e}"),
            known_incompatibilities: None,
            unestablished_confinement: None,
        },
    };
    // The frozen world stays unmet on either profile: refused by a known provider-family
    // incompatibility (openai-codex), or prepared with confinement unestablished (claude-sdk).
    let frozen_ok = match profile {
        Profile::OpenaiCodex => {
            frozen_world.outcome == "refused" && frozen_world.known_incompatibilities > Some(0)
        }
        Profile::ClaudeSdk => {
            frozen_world.outcome == "prepared_conditions_unmet"
                && frozen_world.known_incompatibilities == Some(0)
                && frozen_world.unestablished_confinement > Some(0)
        }
    };
    checks.pass(
        "frozen_world_unmet",
        frozen_ok,
        format!(
            "{}: {} known incompatibilities, {} unestablished confinement",
            frozen_world.outcome,
            count_or_not_established(frozen_world.known_incompatibilities),
            count_or_not_established(frozen_world.unestablished_confinement)
        ),
    );
    let served = run_binding(&mut checks, record, run, scope_digest.as_deref());
    let mut request_insertions = None;
    match &task_bytes {
        Some(task) => match delivery(profile, requests, task) {
            Ok((detail, insertions)) => {
                request_insertions = insertions;
                checks.pass("delivery", true, detail)
            }
            Err(detail) => checks.pass("delivery", false, detail),
        },
        None => {
            checks.push("delivery", Outcome::NotRun, "no recomputed task");
            false
        }
    };

    // The answer: digest, strict parse (no fence stripping, no repair), schema, subject.
    let answer_ok = checks.pass(
        "answer_sha256",
        sha256_hex(result.answer.as_bytes()) == result.answer_sha256,
        format!("recorded {}", result.answer_sha256),
    );
    let mut grounding_report = None;
    let parsed = if answer_ok {
        match parse_json(&result.answer) {
            Ok(v) => {
                checks.pass("strict_json", true, "the answer is exactly one JSON value");
                Some(v)
            }
            Err(e) => {
                checks.pass("strict_json", false, e.message);
                None
            }
        }
    } else {
        checks.push("strict_json", Outcome::NotRun, "answer digest mismatch");
        None
    };
    let review = match parsed {
        Some(value) => match ReviewResult::parse(value) {
            Ok(r) => {
                checks.pass("result_contract", true, "implementation-review/v1");
                Some(r)
            }
            Err(e) => {
                checks.pass("result_contract", false, e.message);
                None
            }
        },
        None => {
            checks.push("result_contract", Outcome::NotRun, "no parsed answer");
            None
        }
    };
    match &review {
        Some(r) => {
            let digest_ok = r
                .subject_digest()
                .is_ok_and(|d| d.0 == digest(&subject.to_js_value()));
            let fields_ok = r.value.get("subject").is_some_and(|s| {
                text_of(s, "commit").as_deref() == Some(subject.commit.as_str())
                    && text_of(s, "tree").as_deref() == Some(subject.tree.as_str())
                    && text_of(s, "patchIdentity").as_deref()
                        == Some(subject.patch_identity.as_str())
            });
            checks.pass(
                "subject_equal",
                digest_ok && fields_ok,
                "subject digest and its three fields",
            );
            match &served {
                Some(manifest) => {
                    let g = grounding(r, manifest);
                    checks.push(
                        "grounding",
                        Outcome::Reported,
                        format!(
                            "{} of {} evidence items grounded in the served manifest",
                            g.grounded, g.total
                        ),
                    );
                    grounding_report = Some(g);
                }
                None => checks.push("grounding", Outcome::NotRun, "served manifest unreadable"),
            }
        }
        None => {
            checks.push("subject_equal", Outcome::NotRun, "no valid result");
            checks.push("grounding", Outcome::NotRun, "no valid result");
        }
    }

    let passed = checks
        .0
        .iter()
        .all(|c| matches!(c.outcome, Outcome::Passed | Outcome::Reported));
    let subject_value = serde_json::to_value(subject).unwrap_or(Value::Null);
    Envelope {
        schema: ENVELOPE_SCHEMA.to_owned(),
        standing: "owner_pending".to_owned(),
        org_admission: "none_requested".to_owned(),
        consumable: false,
        proposition: "route_qualification_only".to_owned(),
        profile,
        world: World {
            label: rev.id.to_owned(),
            run_id,
            requirements_sha256: sha256_hex(rev.requirements),
        },
        requested_model: profile.model().to_owned(),
        requested_provider: profile.provider().to_owned(),
        frozen_world,
        undischarged_blockers: blockers,
        job_id: record.job_id.clone(),
        subject: subject_value,
        contract_sha256: record.contract_sha256.clone(),
        task_sha256: record.task_sha256.clone(),
        answer: result.answer.clone(),
        answer_sha256: result.answer_sha256.clone(),
        host_reported: HostReported {
            settlement: result.settlement.clone(),
            provider_requests: result.provider_requests.clone(),
            snapshot_deliveries: result.snapshot_deliveries.clone(),
            hosted_attempts: result.hosted_attempts.clone(),
            request_insertions,
        },
        checks: checks.0,
        grounding: grounding_report,
        outcome: if passed {
            "checks_passed"
        } else {
            "checks_failed"
        }
        .to_owned(),
        not_claimed: NOT_CLAIMED.iter().map(|s| (*s).to_owned()).collect(),
    }
}

/// Reads a run's state directory: `analysis-result.json`, the request ledger in `pi-state.json`
/// (`coding.provider_requests.<n>.attempts[].request_body_sha256`, the shape `analyze.py:139`,
/// `:146` reads) and each `coding-hosted-<n>.body.json`.
pub fn load_state(state: &Path) -> Result<(AnalysisResult, Vec<RetainedRequest>), Refusal> {
    let read = |name: &str| std::fs::read(state.join(name));
    let result: AnalysisResult = serde_json::from_slice(&read("analysis-result.json")?)
        .map_err(|e| Refusal::Io(format!("analysis-result.json: {e}")))?;
    let pi: Value = serde_json::from_slice(&read("pi-state.json")?)
        .map_err(|e| Refusal::Io(format!("pi-state.json: {e}")))?;
    let ledger = pi["coding"]["provider_requests"]
        .as_object()
        .ok_or_else(|| Refusal::Io("pi-state.json: no coding.provider_requests".into()))?;
    let mut requests = Vec::new();
    for (key, entry) in ledger {
        let request: u64 = key
            .parse()
            .map_err(|_| Refusal::Io(format!("pi-state.json: request key {key}")))?;
        let attempt_digests = entry["attempts"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|a| a["request_body_sha256"].as_str().map(str::to_owned))
            .collect();
        requests.push(RetainedRequest {
            request,
            body: read(&format!("coding-hosted-{request}.body.json")).ok(),
            attempt_digests,
        });
    }
    requests.sort_by_key(|r| r.request);
    Ok((result, requests))
}

/// Reads the run's job artifacts: `<run_inputs>/{snapshot-manifest,packet,authority}.json` and
/// `job.json`.
pub fn load_run(run_inputs: &Path, job: &Path) -> Result<RunArtifacts, Refusal> {
    let json = |path: &Path| -> Result<Value, Refusal> {
        serde_json::from_slice(&std::fs::read(path)?)
            .map_err(|e| Refusal::Io(format!("{}: {e}", path.display())))
    };
    Ok(RunArtifacts {
        manifest_bytes: std::fs::read(run_inputs.join("snapshot-manifest.json"))?,
        packet: json(&run_inputs.join("packet.json"))?,
        authority: json(&run_inputs.join("authority.json"))?,
        job: json(job)?,
    })
}
