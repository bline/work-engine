//! Public, bounded read projection for one admitted historical S5 investigation.
pub mod graph;
use serde::{Deserialize, Serialize};

pub const MAX_FRAME: usize = 131_072;
pub const OPERATION: &str = "historical_s5.inspect.v1";
pub const RUN_ID: &str = "skills-migration-app-server-20260828";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub operation: String,
    pub candidate_attempt: Option<u8>,
    pub review_key: Option<String>,
}
impl Request {
    pub fn all() -> Self {
        Self {
            operation: OPERATION.into(),
            candidate_attempt: None,
            review_key: None,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.operation != OPERATION
            || self
                .candidate_attempt
                .is_some_and(|n| !(1..=5).contains(&n))
            || self.review_key.as_ref().is_some_and(|key| {
                !key.starts_with("independent-review-episode-v1/") || key.len() > 128
            })
        {
            return Err("request is outside the fixed historical operation");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRead {
    pub owner: String,
    pub identity: String,
    pub integrity: String,
    pub completeness: String,
    pub confidentiality: String,
    pub limit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateCheck {
    pub identity: String,
    pub outcome: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    pub source_kind: String,
    pub receipt_sha256: String,
    pub capture_sha256: String,
    pub corroborating_output_sha256: Option<String>,
    pub status: String,
    pub check_count: usize,
    pub passed_count: usize,
    pub checks: Vec<GateCheck>,
    pub limit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub kind: String,
    pub attempt: u8,
    pub checkpoint_ref: String,
    pub commit_oid: String,
    pub tree_oid: String,
    pub parent_oid: String,
    pub gate_digest: String,
    pub gate: Option<Gate>,
    pub integrity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub owner: String,
    pub reference: String,
    pub revision: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub status: String,
    pub severity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewRevision {
    pub revision: String,
    pub predecessor_revision: Option<String>,
    pub payload_sha256: String,
    pub status: String,
    pub phase: String,
    pub subjects: Vec<Subject>,
    pub findings: Vec<Finding>,
    pub authority_reference: String,
    pub authority_manifest_sha256: String,
    pub historical_grant_id: String,
    pub writer_generation: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewEpisode {
    pub label: String,
    pub key: String,
    pub head: String,
    pub revisions: Vec<ReviewRevision>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplacementLink {
    pub replacement_key: String,
    pub replacement_revision: String,
    pub original_key: String,
    pub original_reported_revision: String,
    pub authority_reference: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub protocol_version: u8,
    pub projection_version: u8,
    pub operation: String,
    pub run_id: String,
    pub slice_number: u8,
    pub grant_id: String,
    pub cutoff: String,
    pub read_at: String,
    pub source_identity_sha256: String,
    pub candidates: Vec<Candidate>,
    pub reviews: Vec<ReviewEpisode>,
    pub replacement_links: Vec<ReplacementLink>,
    pub source_reads: Vec<SourceRead>,
    pub gaps: Vec<String>,
}
impl Observation {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.protocol_version != 1
            || self.projection_version != 1
            || self.operation != OPERATION
            || self.run_id != RUN_ID
            || self.slice_number != 6
            || self.candidates.len() != 6
            || self.reviews.len() != 3
            || self.grant_id.is_empty()
            || self.source_identity_sha256.len() != 64
        {
            return Err("observation identity or version is invalid");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reply {
    Observed { observation: Observation },
    Refused { reason: String },
    Unavailable { reason: String },
}

pub const OPERATION_V2: &str = "historical_s5.inspect.v2";
pub const SCHEDULER_ITEM_ID: &str =
    "skills-migration-s5-candidate4-generic-review-quota-retry-20260829";
pub const REPLACEMENT_REVIEW_KEY: &str = "independent-review-episode-v1/8c55f4e88ef8f28cff64f97cd2c8f49b37427a6b5fbab5a75afd9135814148b6";
pub const REPLACEMENT_EPISODE_REF: &str = "refs/work-engine/independent-review-state/9a1646c857c09283aebaeec25073ea5fd0ac31ea98aa4844efe7884978cb6076";
pub const CANDIDATE4_RESULT_REVISION: &str = "034b4df528510e5f65029aa029f3fd2f78ef39d1";
pub const METRICS_COMMIT_OID: &str = "f64bc61dd5b618d9531e7937e3494fe459dc1ea0";
pub const METRICS_BLOB_OID: &str = "a8e0eac325b48f7f693bb8843476276ffd669236";
pub const METRICS_ROW_SHA256: &str =
    "2bd65a15f9ae61bbd3a6a16b7be41f46b66ca1a7b35a22e1a316d60405b6096d";

fn present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

// These types are shared by the admitting host and its bounded source helper.
// Deserializing a Value first would silently discard duplicate nested keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCaptureSource {
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderResultSource {
    #[serde(deserialize_with = "present_option")]
    pub api_error_status: Option<u64>,
    pub capture: usize,
    pub command_kind: String,
    pub command_sha256: String,
    pub event_id: String,
    pub exit_code: i64,
    pub is_error: bool,
    pub line: u64,
    pub line_sha256: String,
    pub review_label: String,
    pub session_id: String,
    pub stdout_sha256: String,
    pub uuid: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderSources {
    pub captures: Vec<ProviderCaptureSource>,
    pub results: Vec<ProviderResultSource>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulerSource {
    pub db_path: String,
    pub due_at: String,
    pub episode_ref: String,
    pub intent_sha256: String,
    pub item_id: String,
    pub logical_role: String,
    pub repository_id: String,
    pub result_ref: String,
    pub row_sha256: String,
    pub session_id: String,
    pub store_revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsSource {
    pub blob_oid: String,
    pub commit_oid: String,
    pub gate_digest: String,
    pub path: String,
    pub row_sha256: String,
}

fn hex_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn bounded_text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

pub fn validate_v2_sources(
    provider: &ProviderSources,
    scheduler: &SchedulerSource,
    metrics: &MetricsSource,
    repository: &str,
    gate_digest: &str,
) -> Result<(), &'static str> {
    use std::collections::HashSet;
    if provider.captures.len() != 2
        || provider.results.len() != 8
        || provider.captures[0].sha256
            != "bcb0280c4b4859b4a7d15e6f2bf67bd61fdaeebf696081267617a0ad59e754c7"
        || provider.captures[1].sha256
            != "993c18502d63aa7b8bfa87e1949e5b6bf5437a80c1b578e6e74527cc1db9811e"
        || provider.captures.iter().any(|c| {
            !std::path::Path::new(&c.path).is_absolute()
                || !bounded_text(&c.path, 512)
                || !hex_digest(&c.sha256)
        })
        || scheduler.item_id != SCHEDULER_ITEM_ID
        || scheduler.store_revision == 0
        || scheduler.repository_id != repository
        || scheduler.logical_role != "slice-supervisor"
        || scheduler.episode_ref != REPLACEMENT_EPISODE_REF
        || scheduler.result_ref != format!("{REPLACEMENT_EPISODE_REF}@{CANDIDATE4_RESULT_REVISION}")
        || !std::path::Path::new(&scheduler.db_path).is_absolute()
        || !bounded_text(&scheduler.db_path, 512)
        || !hex_digest(&scheduler.intent_sha256)
        || !hex_digest(&scheduler.row_sha256)
        || !uuid(&scheduler.session_id)
        || !bounded_text(&scheduler.due_at, 64)
        || metrics.commit_oid != METRICS_COMMIT_OID
        || metrics.blob_oid != METRICS_BLOB_OID
        || !oid(&metrics.commit_oid)
        || !oid(&metrics.blob_oid)
        || metrics.row_sha256 != METRICS_ROW_SHA256
        || metrics.path != "docs/ai-workflow-metrics.jsonl"
        || metrics.gate_digest != gate_digest
    {
        return Err("v2 source inventory identity invalid");
    }
    let mut uuids = HashSet::new();
    let mut error_count = 0;
    for result in &provider.results {
        if !uuid(&result.uuid)
            || !uuid(&result.session_id)
            || !uuids.insert(&result.uuid)
            || result.capture > 1
            || !(1..=10_000).contains(&result.line)
            || !hex_digest(&result.command_sha256)
            || !hex_digest(&result.line_sha256)
            || !hex_digest(&result.stdout_sha256)
            || !bounded_text(&result.event_id, 128)
            || !matches!(
                result.command_kind.as_str(),
                "direct_claude_cli" | "historical_shell_replay"
            )
            || !matches!(
                result.review_label.as_str(),
                "original-generic" | "instruction" | "replacement-generic"
            )
            || result.is_error != result.api_error_status.is_some()
            || (result.is_error && (result.api_error_status != Some(429) || result.exit_code != 1))
            || (!result.is_error && result.exit_code != 0)
        {
            return Err("v2 provider descriptor invalid");
        }
        if result.is_error {
            error_count += 1;
            if result.session_id != scheduler.session_id
                || result.review_label != "replacement-generic"
            {
                return Err("v2 provider retry binding invalid");
            }
        }
    }
    if error_count != 1 {
        return Err("v2 provider error inventory invalid");
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestV2 {
    pub operation: String,
    pub candidate_attempt: Option<u8>,
    pub review_key: Option<String>,
}
impl RequestV2 {
    pub fn all() -> Self {
        Self {
            operation: OPERATION_V2.into(),
            candidate_attempt: None,
            review_key: None,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.operation != OPERATION_V2
            || self
                .candidate_attempt
                .is_some_and(|n| !(1..=5).contains(&n))
            || self.review_key.as_ref().is_some_and(|key| {
                !key.starts_with("independent-review-episode-v1/") || key.len() > 128
            })
        {
            return Err("request is outside the fixed historical v2 operation");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProviderUsage {
    pub input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub output_tokens: u64,
    pub thinking_tokens: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAttempt {
    pub result_uuid: String,
    pub session_id: String,
    pub review_label: String,
    pub event_id: String,
    pub source_line: u64,
    pub source_file_sha256: String,
    pub source_line_sha256: String,
    pub stdout_sha256: String,
    pub command_kind: String,
    pub exit_code: i64,
    pub is_error: bool,
    #[serde(deserialize_with = "present_option")]
    pub api_error_status: Option<u64>,
    pub terminal_reason: String,
    #[serde(deserialize_with = "present_option")]
    pub model: Option<String>,
    pub duration_ms: u64,
    pub cost_usd: String,
    pub usage: ProviderUsage,
    pub source_class: String,
    pub attribution: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderRead {
    pub source_owner: String,
    pub source_class: String,
    pub source_limit: String,
    pub attempts: Vec<ProviderAttempt>,
    pub unique_result_uuids: usize,
    pub successes: u64,
    pub errors: u64,
    pub cost_literal_sum_usd: String,
    pub usage_total: ProviderUsage,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulerRead {
    pub source_owner: String,
    pub source_class: String,
    pub source_limit: String,
    pub item_id: String,
    pub store_revision: u64,
    pub row_sha256: String,
    pub repository_id: String,
    pub logical_role: String,
    pub candidate_attempt: u8,
    pub session_id: String,
    pub episode_ref: String,
    pub created_at: String,
    pub due_at: String,
    pub updated_at: String,
    #[serde(deserialize_with = "present_option")]
    pub presented_at: Option<String>,
    pub status: String,
    pub result_ref: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsRead {
    pub source_owner: String,
    pub source_class: String,
    pub source_limit: String,
    pub commit_oid: String,
    pub blob_oid: String,
    pub row_number: u64,
    pub row_sha256: String,
    pub gate_digest: String,
    pub stage_total: u64,
    pub provider_cost_usd: String,
    pub provider_successful_calls: u64,
    pub provider_failed_calls: u64,
    pub provider_cache_creation_tokens: u64,
    pub provider_cache_read_tokens: u64,
    pub provider_output_tokens: u64,
    pub provider_thinking_tokens: u64,
    pub independent_review_attempts: u64,
    pub independent_review_successful: u64,
    pub independent_review_failed: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationV2 {
    pub protocol_version: u8,
    pub projection_version: u8,
    pub operation: String,
    pub base: Observation,
    pub provider: ProviderRead,
    pub scheduler: SchedulerRead,
    #[serde(deserialize_with = "present_option")]
    pub metrics: Option<MetricsRead>,
    #[serde(deserialize_with = "present_option")]
    pub metrics_unavailable: Option<String>,
    pub source_identity_sha256: String,
    pub source_reads: Vec<SourceRead>,
    pub gaps: Vec<String>,
}
impl ObservationV2 {
    pub fn validate(&self) -> Result<(), &'static str> {
        use std::collections::HashSet;
        self.base.validate()?;
        if self.protocol_version != 2
            || self.projection_version != 2
            || self.operation != OPERATION_V2
            || self.provider.source_owner != "slice-builder captured provider output"
            || self.provider.source_class != "captured_cli_result"
            || self.provider.attempts.len() != 8
            || self.provider.unique_result_uuids != 8
            || self.provider.successes != 7
            || self.provider.errors != 1
            || self.provider.cost_literal_sum_usd != "15.7545494999999994"
            || self.provider.usage_total.thinking_tokens != 101224
            || self.scheduler.source_owner != "role-scheduler"
            || self.scheduler.source_class != "current_retained_row"
            || self.scheduler.item_id != SCHEDULER_ITEM_ID
            || self.scheduler.store_revision == 0
            || self.scheduler.logical_role != "slice-supervisor"
            || self.scheduler.repository_id != "/home/bline/code/work-engine"
            || self.scheduler.candidate_attempt != 4
            || self.scheduler.status != "acknowledged"
            || self.scheduler.presented_at.is_some()
            || !hex_digest(&self.scheduler.row_sha256)
            || !hex_digest(&self.source_identity_sha256)
            || !hex_digest(&self.base.source_identity_sha256)
            || self.source_reads.len() != self.base.source_reads.len() + 3
            || self.source_reads[self.base.source_reads.len()].owner
                != "slice-builder captured provider output"
            || self.source_reads[self.base.source_reads.len() + 1].owner != "role-scheduler"
            || self.source_reads[self.base.source_reads.len() + 2].owner != "slice-supervisor"
            || self.metrics.is_some() == self.metrics_unavailable.is_some()
            || self
                .base
                .candidates
                .iter()
                .enumerate()
                .any(|(i, c)| c.attempt != (i.min(4) + 1) as u8)
        {
            return Err("v2 observation identity or completeness is invalid");
        }
        let mut seen = HashSet::new();
        let mut successes = 0u64;
        let mut errors = 0u64;
        let mut costs = Vec::new();
        let mut totals = [0u64; 5];
        for a in &self.provider.attempts {
            if !uuid(&a.result_uuid)
                || !uuid(&a.session_id)
                || !seen.insert(&a.result_uuid)
                || a.source_class != "captured_cli_result"
                || a.attribution != "capture_descriptor_bound"
                || !matches!(
                    a.review_label.as_str(),
                    "original-generic" | "instruction" | "replacement-generic"
                )
                || !matches!(
                    a.command_kind.as_str(),
                    "direct_claude_cli" | "historical_shell_replay"
                )
                || !bounded_text(&a.event_id, 128)
                || !bounded_text(&a.terminal_reason, 64)
                || a.model.as_ref().is_some_and(|m| !bounded_text(m, 80))
                || !(1..=10_000).contains(&a.source_line)
                || !hex_digest(&a.source_file_sha256)
                || !matches!(
                    a.source_file_sha256.as_str(),
                    "bcb0280c4b4859b4a7d15e6f2bf67bd61fdaeebf696081267617a0ad59e754c7"
                        | "993c18502d63aa7b8bfa87e1949e5b6bf5437a80c1b578e6e74527cc1db9811e"
                )
                || !hex_digest(&a.source_line_sha256)
                || !hex_digest(&a.stdout_sha256)
                || a.duration_ms > 86_400_000
                || a.is_error != a.api_error_status.is_some()
                || (a.is_error && (a.api_error_status != Some(429) || a.exit_code != 1))
                || (!a.is_error && a.exit_code != 0)
            {
                return Err("v2 provider result identity invalid");
            }
            if a.is_error {
                errors += 1;
            } else {
                successes += 1;
            }
            costs.push(a.cost_usd.as_str());
            for (i, value) in [
                a.usage.input_tokens,
                a.usage.cache_creation_input_tokens,
                a.usage.cache_read_input_tokens,
                a.usage.output_tokens,
                a.usage.thinking_tokens,
            ]
            .into_iter()
            .enumerate()
            {
                if value > 1_000_000_000 {
                    return Err("v2 provider token count outside bound");
                }
                totals[i] = totals[i]
                    .checked_add(value)
                    .ok_or("v2 provider token sum overflow")?;
            }
        }
        let cost_sum = decimal_sum(&costs)?;
        if seen.len() != self.provider.unique_result_uuids
            || successes != self.provider.successes
            || errors != self.provider.errors
            || cost_sum != self.provider.cost_literal_sum_usd
            || self.provider.usage_total
                != (ProviderUsage {
                    input_tokens: totals[0],
                    cache_creation_input_tokens: totals[1],
                    cache_read_input_tokens: totals[2],
                    output_tokens: totals[3],
                    thinking_tokens: totals[4],
                })
        {
            return Err("v2 provider aggregate contradicts attempts");
        }
        if self
            .provider
            .attempts
            .iter()
            .filter(|a| a.is_error && a.api_error_status == Some(429))
            .count()
            != 1
        {
            return Err("v2 provider error set invalid");
        }
        let failed = self
            .provider
            .attempts
            .iter()
            .find(|a| a.is_error && a.api_error_status == Some(429))
            .unwrap();
        if failed.session_id != self.scheduler.session_id
            || failed.usage.thinking_tokens != 27
            || failed.review_label != "replacement-generic"
        {
            return Err("v2 failed-call/scheduler attribution differs");
        }
        let replacement = self
            .base
            .reviews
            .iter()
            .find(|r| r.label == "replacement-generic" && r.key == REPLACEMENT_REVIEW_KEY)
            .ok_or("replacement review absent")?;
        if self.scheduler.episode_ref != REPLACEMENT_EPISODE_REF
            || self.scheduler.result_ref
                != format!("{REPLACEMENT_EPISODE_REF}@{CANDIDATE4_RESULT_REVISION}")
            || !uuid(&self.scheduler.session_id)
            || !bounded_text(&self.scheduler.created_at, 64)
            || !bounded_text(&self.scheduler.due_at, 64)
            || !bounded_text(&self.scheduler.updated_at, 64)
            || !replacement.revisions.iter().any(|r| {
                r.revision == CANDIDATE4_RESULT_REVISION
                    && r.subjects.iter().any(|s| {
                        s.owner == "slice-checkpoint"
                            && s.reference == self.base.candidates[3].checkpoint_ref
                            && s.revision == self.base.candidates[3].commit_oid
                    })
            })
        {
            return Err("scheduler result is not an admitted review revision");
        }
        if let Some(metrics) = &self.metrics {
            if metrics.source_owner != "slice-supervisor"
                || metrics.source_class != "frozen_metrics_row"
                || metrics.commit_oid != METRICS_COMMIT_OID
                || metrics.blob_oid != METRICS_BLOB_OID
                || metrics.row_sha256 != METRICS_ROW_SHA256
                || metrics.stage_total != 8
                || metrics.provider_cost_usd != "15.7545495"
                || metrics.provider_thinking_tokens != 101197
                || metrics.row_number != 21
                || metrics.gate_digest != self.base.candidates[4].gate_digest
                || self.source_reads[self.base.source_reads.len() + 2].integrity
                    != "raw_git_object_and_row_digest_verified"
            {
                return Err("frozen metrics comparison invalid");
            }
        } else if self.metrics_unavailable.as_deref()
            != Some("Frozen metrics source unavailable under this read")
            || self.source_reads[self.base.source_reads.len() + 2].integrity != "unavailable"
        {
            return Err("metrics unavailable reason invalid");
        }
        Ok(())
    }
}

fn decimal_sum(literals: &[&str]) -> Result<String, &'static str> {
    let mut terms = Vec::new();
    let mut scale = 0usize;
    for literal in literals {
        if literal.is_empty()
            || literal.ends_with('.')
            || literal.len() > 48
            || literal.contains(['e', 'E', '+', '-'])
        {
            return Err("invalid provider decimal");
        }
        let (whole, fractional) = literal.split_once('.').unwrap_or((literal, ""));
        if whole.is_empty()
            || fractional.len() > 18
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fractional.bytes().all(|b| b.is_ascii_digit())
        {
            return Err("invalid provider decimal");
        }
        let number = format!("{whole}{fractional}")
            .parse::<u128>()
            .map_err(|_| "provider decimal overflow")?;
        scale = scale.max(fractional.len());
        terms.push((number, fractional.len()));
    }
    let mut total = 0u128;
    for (number, n) in terms {
        let factor = 10u128
            .checked_pow((scale - n) as u32)
            .ok_or("provider decimal overflow")?;
        total = total
            .checked_add(
                number
                    .checked_mul(factor)
                    .ok_or("provider decimal overflow")?,
            )
            .ok_or("provider decimal overflow")?;
    }
    if scale == 0 {
        return Ok(total.to_string());
    }
    let factor = 10u128
        .checked_pow(scale as u32)
        .ok_or("provider decimal overflow")?;
    Ok(format!(
        "{}.{:0width$}",
        total / factor,
        total % factor,
        width = scale
    ))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReplyV2 {
    Observed { observation: ObservationV2 },
    Refused { reason: String },
    Unavailable { reason: String },
}

#[cfg(test)]
mod v2_regression_tests {
    use super::*;

    #[test]
    fn nullable_observations_require_explicit_fields() {
        let scheduler = serde_json::json!({
            "source_owner": "role-scheduler", "source_class": "current_retained_row",
            "source_limit": "current row", "item_id": "item", "store_revision": 1,
            "row_sha256": "digest", "repository_id": "repo", "logical_role": "role",
            "candidate_attempt": 4, "session_id": "session", "episode_ref": "episode",
            "created_at": "created", "due_at": "due", "updated_at": "updated",
            "presented_at": null, "status": "acknowledged", "result_ref": "result"
        });
        assert!(serde_json::from_value::<SchedulerRead>(scheduler.clone()).is_ok());
        let mut missing = scheduler.clone();
        missing.as_object_mut().unwrap().remove("presented_at");
        assert!(serde_json::from_value::<SchedulerRead>(missing).is_err());
        let mut wrong_type = scheduler;
        wrong_type["presented_at"] = serde_json::json!(false);
        assert!(serde_json::from_value::<SchedulerRead>(wrong_type).is_err());

        let attempt = serde_json::json!({
            "result_uuid": "uuid", "session_id": "session", "review_label": "label",
            "event_id": "event", "source_line": 1, "source_file_sha256": "digest",
            "source_line_sha256": "digest", "stdout_sha256": "digest",
            "command_kind": "command", "exit_code": 0, "is_error": false,
            "api_error_status": null, "terminal_reason": "success", "model": null,
            "duration_ms": 1, "cost_usd": "0", "usage": {
                "input_tokens": 0, "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0, "output_tokens": 0, "thinking_tokens": 0
            }, "source_class": "captured_cli_result", "attribution": "source"
        });
        assert!(serde_json::from_value::<ProviderAttempt>(attempt.clone()).is_ok());
        for field in ["api_error_status", "model"] {
            let mut missing = attempt.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<ProviderAttempt>(missing).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn nested_source_descriptors_reject_unknown_duplicate_missing_and_wrong_type() {
        assert!(
            serde_json::from_str::<ProviderSources>(r#"{"captures":[],"results":[],"extra":1}"#)
                .is_err()
        );
        assert!(
            serde_json::from_str::<ProviderSources>(
                r#"{"captures":[],"captures":[],"results":[]}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<SchedulerSource>(r#"{"db_path":3}"#).is_err());
        let valid = r#"{"api_error_status":null,"capture":0,"command_kind":"direct_claude_cli","command_sha256":"x","event_id":"x","exit_code":0,"is_error":false,"line":1,"line_sha256":"x","review_label":"instruction","session_id":"x","stdout_sha256":"x","uuid":"x"}"#;
        assert!(serde_json::from_str::<ProviderResultSource>(valid).is_ok());
        assert!(
            serde_json::from_str::<ProviderResultSource>(
                &valid.replace("\"api_error_status\":null,", "")
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<ProviderResultSource>(
                &valid.replace("\"capture\":0", "\"capture\":0,\"capture\":1")
            )
            .is_err()
        );
    }

    #[test]
    fn decimal_aggregation_checks_lexemes_and_overflow() {
        assert_eq!(decimal_sum(&["1.2", "0.03"]).unwrap(), "1.23");
        for bad in [
            "NaN",
            "1e2",
            "-1",
            "1.",
            "",
            "340282366920938463463374607431768211456",
        ] {
            assert!(decimal_sum(&[bad]).is_err(), "{bad}");
        }
        assert!(decimal_sum(&["340282366920938463463374607431768211455", "1"]).is_err());
    }
}
