//! A finite, disposable event/owner projection of the admitted S5 read.
//! Historical node identities never include the grant or executable-byte anchor.
use crate::{MAX_FRAME, ObservationV2, RUN_ID, SourceRead};
use serde::{Deserialize, Serialize};

pub const OPERATION: &str = "historical_s5.graph.v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub operation: String,
}
impl Request {
    pub fn all() -> Self {
        Self {
            operation: OPERATION.into(),
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.operation != OPERATION {
            return Err("unsupported graph operation");
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Observed,
    MechanicallyDerived,
    SemanticInterpretation,
    CounterfactualEstimate,
    Recommendation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryEntry {
    pub owner: String,
    pub source_class: String,
    pub kind: String,
    pub confidentiality: String,
    pub completeness: String,
    pub limit: String,
    pub legal_outgoing: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeValue {
    pub meaning: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub name: String,
    pub value: String,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub evidence_class: EvidenceClass,
    pub owner: String,
    pub source_class: String,
    pub source_ref: String,
    pub source_revision: String,
    pub source_index: usize,
    pub local_id: String,
    pub label: String,
    pub observed_time: Option<TimeValue>,
    pub effective_time: Option<TimeValue>,
    pub measurement: Option<Measurement>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub evidence_class: EvidenceClass,
    pub owner: String,
    pub source_ref: String,
    pub source_revision: String,
    pub limit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub protocol_version: u8,
    pub projection_version: u8,
    pub operation: String,
    pub registry_version: u8,
    /// Includes the admitted source anchor and the pre-existing bounded summaries.
    pub base: ObservationV2,
    pub registry: Vec<RegistryEntry>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub gaps: Vec<String>,
}

fn registry() -> Vec<RegistryEntry> {
    [
        (
            "slice-checkpoint",
            "immutable_checkpoint_artifact",
            "checkpoint",
            "selected immutable refs",
            "digest-bound, not execution attestation",
            &["has_gate_digest"][..],
        ),
        (
            "slice-builder",
            "slice_builder_runner_output",
            "gate_capture",
            "five selected captures",
            "reported runner checks only",
            &[][..],
        ),
        (
            "slice-builder",
            "coordinator_constructed_receipt",
            "gate_capture",
            "candidate-five capture",
            "coordinator checks, not historical stage count",
            &[][..],
        ),
        (
            "independent-review-state",
            "linked_revision",
            "review_revision",
            "three selected episodes",
            "revision chain, not independent acceptance",
            &[
                "reviews_exact_subject",
                "next_recorded_revision",
                "reports_finding_state",
                "replaces_reported_episode",
            ][..],
        ),
        (
            "independent-review-state",
            "finding_state_at_revision",
            "finding_state",
            "reported revision states",
            "state does not prove remediation cause",
            &[][..],
        ),
        (
            "slice-builder captured provider output",
            "captured_cli_result",
            "provider_result",
            "eight selected CLI results",
            "captured output, not billing authority",
            &[][..],
        ),
        (
            "role-scheduler",
            "current_retained_row",
            "scheduler_current_row",
            "one current row",
            "no mutation or presentation history",
            &["refers_to_review_result"][..],
        ),
        (
            "slice-supervisor",
            "frozen_metrics_row",
            "frozen_metrics_row",
            "one optional frozen row",
            "stage semantics unresolved",
            &[][..],
        ),
        (
            "s5-projection",
            "eight_distinct_captured_results",
            "provider_cost_sum",
            "eight distinct UUIDs",
            "literal sum only",
            &["includes_distinct_result"][..],
        ),
        (
            "s5-projection",
            "receipt_metric_comparison",
            "measurement_discrepancy",
            "one comparison",
            "different measurement units",
            &["compares_receipt", "compares_frozen_row"][..],
        ),
    ]
    .into_iter()
    .map(
        |(owner, source_class, kind, completeness, limit, legal_outgoing)| RegistryEntry {
            owner: owner.into(),
            source_class: source_class.into(),
            kind: kind.into(),
            confidentiality: "private_local".into(),
            completeness: completeness.into(),
            limit: limit.into(),
            legal_outgoing: legal_outgoing.iter().map(|v| (*v).into()).collect(),
        },
    )
    .collect()
}

fn source_index(base: &ObservationV2, owner: &str, identity: &str) -> Result<usize, &'static str> {
    base.source_reads
        .iter()
        .position(|s| s.owner == owner && s.identity == identity)
        .ok_or("graph source reference has no admitted read")
}

fn sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn git_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn bounded(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

fn source_matches(
    source: &SourceRead,
    owner: &str,
    identity: &str,
    integrity: &str,
    completeness: &str,
    confidentiality: &str,
    limit: &str,
) -> bool {
    source.owner == owner
        && source.identity == identity
        && source.integrity == integrity
        && source.completeness == completeness
        && source.confidentiality == confidentiality
        && source.limit == limit
}

// This finite graph operation promises a complete, private S5 case inventory.
// The host still admits and verifies each owner source; this check prevents a
// coherent but weakened wire frame from being reconstructed as that promise.
fn validate_graph_base_contract(base: &ObservationV2) -> Result<(), &'static str> {
    let case = &base.base;
    if case.candidates.len() != 6
        || case.reviews.len() != 3
        || case
            .reviews
            .iter()
            .zip([4, 4, 6])
            .any(|(episode, expected)| episode.revisions.len() != expected)
        || case.replacement_links.len() != 1
        || case.source_reads.len() != 14
        || base.source_reads.len() != 17
        || !bounded(&case.grant_id, 128)
        || !bounded(&case.cutoff, 64)
        || !bounded(&case.read_at, 64)
        || !sha256(&case.source_identity_sha256)
        || !sha256(&base.source_identity_sha256)
        || case
            .source_reads
            .iter()
            .zip(&base.source_reads)
            .any(|(a, b)| {
                a.owner != b.owner
                    || a.identity != b.identity
                    || a.integrity != b.integrity
                    || a.completeness != b.completeness
                    || a.confidentiality != b.confidentiality
                    || a.limit != b.limit
            })
    {
        return Err("graph base inventory or read identity invalid");
    }
    let checkpoint_prefix = format!("refs/work-engine/checkpoints/{RUN_ID}/slice-6/");
    for (i, candidate) in case.candidates.iter().enumerate() {
        let expected_kind = if i < 5 {
            format!("candidate-{}", i + 1)
        } else {
            "accepted".into()
        };
        if candidate.kind != expected_kind
            || candidate.attempt != (i.min(4) + 1) as u8
            || candidate.checkpoint_ref != format!("{checkpoint_prefix}{expected_kind}")
            || !git_oid(&candidate.commit_oid)
            || !git_oid(&candidate.tree_oid)
            || !git_oid(&candidate.parent_oid)
            || candidate.commit_oid.len() != candidate.tree_oid.len()
            || candidate.commit_oid.len() != candidate.parent_oid.len()
            || !sha256(&candidate.gate_digest)
            || candidate.integrity != "validated_commit_ref_and_metadata"
        {
            return Err("graph checkpoint identity invalid");
        }
        if i > 0 && candidate.parent_oid != case.candidates[i - 1].commit_oid {
            return Err("graph checkpoint parent chain invalid");
        }
        let checkpoint_source = &case.source_reads[if i < 5 { 2 * i + 1 } else { 10 }];
        if !source_matches(
            checkpoint_source,
            "slice-checkpoint",
            &format!("{}@{}", candidate.checkpoint_ref, candidate.commit_oid),
            "validated_commit_ref_and_metadata",
            "complete_for_exact_checkpoint",
            "allowlisted_metadata",
            "checkpoint is not gate execution attestation",
        ) {
            return Err("graph checkpoint source policy invalid");
        }
        if i == 5 {
            if candidate.gate.is_some()
                || candidate.gate_digest != case.candidates[4].gate_digest
                || candidate.tree_oid != case.candidates[4].tree_oid
            {
                return Err("graph accepted checkpoint identity invalid");
            }
            continue;
        }
        let gate = candidate
            .gate
            .as_ref()
            .ok_or("graph gate inventory incomplete")?;
        if gate.receipt_sha256 != candidate.gate_digest
            || !sha256(&gate.capture_sha256)
            || gate
                .corroborating_output_sha256
                .as_ref()
                .is_some_and(|digest| !sha256(digest))
            || (i < 4) != gate.corroborating_output_sha256.is_none()
            || gate.status
                != if i < 4 {
                    "passed"
                } else {
                    "all_reported_checks_passed"
                }
            || gate.check_count == 0
            || gate.check_count > 16
            || gate.check_count != gate.passed_count
            || gate
                .checks
                .iter()
                .any(|check| !bounded(&check.identity, 96) || check.outcome != "passed")
            || gate
                .checks
                .iter()
                .map(|check| check.identity.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len()
                != gate.checks.len()
            || gate.limit != "historical producer capture; no independent execution attestation"
            || !source_matches(
                &case.source_reads[2 * i],
                "slice-builder",
                &format!("{}@{}", candidate.checkpoint_ref, gate.receipt_sha256),
                "capture_and_checkpoint_digest_verified",
                "complete_for_trusted_capture",
                "allowlisted_summary",
                &gate.limit,
            )
        {
            return Err("graph gate capture or source policy invalid");
        }
    }
    for (i, episode) in case.reviews.iter().enumerate() {
        let expected_label = ["original-generic", "instruction", "replacement-generic"][i];
        if episode.label != expected_label
            || !episode.key.starts_with("independent-review-episode-v1/")
            || !sha256(&episode.key["independent-review-episode-v1/".len()..])
            || !git_oid(&episode.head)
            || !source_matches(
                &case.source_reads[11 + i],
                "independent-review-state",
                &format!("{}@{}", episode.key, episode.head),
                "envelope_history_and_state_validated",
                "linked_history_to_null_predecessor",
                "allowlisted_review_summary",
                "historical writer manifests are not this read grant; reported is not retired",
            )
        {
            return Err("graph review identity or source policy invalid");
        }
        for revision in &episode.revisions {
            if !git_oid(&revision.revision)
                || !sha256(&revision.payload_sha256)
                || !sha256(&revision.authority_manifest_sha256)
                || revision
                    .predecessor_revision
                    .as_ref()
                    .is_some_and(|oid| !git_oid(oid))
                || !bounded(&revision.authority_reference, 300)
                || revision.authority_reference != episode.revisions[0].authority_reference
                || revision.authority_manifest_sha256
                    != episode.revisions[0].authority_manifest_sha256
                || !bounded(&revision.historical_grant_id, 160)
                || revision.historical_grant_id != episode.revisions[0].historical_grant_id
                || revision.status != "active"
                || !matches!(
                    revision.phase.as_str(),
                    "initial_review" | "re_evaluation" | "remediation" | "reported"
                )
                || revision.findings.len() > 8
                || revision.findings.iter().any(|finding| {
                    !bounded(&finding.id, 128)
                        || !matches!(
                            finding.status.as_str(),
                            "open" | "verified_resolved" | "withdrawn"
                        )
                        || !matches!(
                            finding.severity.as_str(),
                            "low" | "medium" | "high" | "critical"
                        )
                })
                || revision
                    .findings
                    .iter()
                    .map(|finding| finding.id.as_str())
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != revision.findings.len()
            {
                return Err("graph review revision form invalid");
            }
        }
        let authority = &episode.revisions[0].authority_reference;
        if i < 2 {
            if !authority
                .strip_prefix("review-selection:sha256:")
                .is_some_and(sha256)
            {
                return Err("graph review selection reference invalid");
            }
        } else {
            let Some(reference) = authority.strip_prefix(
                "replacement-authority:reported_episode_cannot_bind_later_candidate:",
            ) else {
                return Err("graph replacement reference invalid");
            };
            let Some((key, revision)) = reference.split_once('@') else {
                return Err("graph replacement reference invalid");
            };
            if !key
                .strip_prefix("refs/work-engine/independent-review-state/")
                .is_some_and(sha256)
                || revision != case.reviews[0].head
            {
                return Err("graph replacement reference invalid");
            }
        }
    }
    let link = &case.replacement_links[0];
    if link.replacement_key != case.reviews[2].key
        || link.replacement_revision != case.reviews[2].head
        || link.original_key != case.reviews[0].key
        || link.original_reported_revision != case.reviews[0].head
        || link.authority_reference != case.reviews[2].revisions[0].authority_reference
        || !link
            .authority_reference
            .starts_with("replacement-authority:reported_episode_cannot_bind_later_candidate:")
    {
        return Err("graph replacement authority inventory invalid");
    }
    let provider = &base.source_reads[14];
    if !source_matches(
        provider,
        "slice-builder captured provider output",
        &format!("8 result UUIDs / {}", base.provider.cost_literal_sum_usd),
        "full_file_line_command_and_stdout_digests_verified",
        "eight_trusted_capture_descriptors",
        "allowlisted_numeric_result_summary",
        "CLI result envelopes are captured output, not canonical provider billing or a complete workflow event stream",
    ) {
        return Err("graph provider source policy invalid");
    }
    let scheduler = &base.scheduler;
    if !source_matches(
        &base.source_reads[15],
        "role-scheduler",
        &format!(
            "{}@{}:{}",
            scheduler.item_id, scheduler.store_revision, scheduler.row_sha256
        ),
        "read_only_snapshot_and_selected_row_digest_verified",
        "one_current_row_no_mutation_history",
        "allowlisted_item_summary",
        "One current acknowledged row; no mutation log, UUID causal edge, or presentation event",
    ) {
        return Err("graph scheduler source policy invalid");
    }
    let metric_source = &base.source_reads[16];
    if let Some(metrics) = &base.metrics {
        if !source_matches(
            metric_source,
            "slice-supervisor",
            &format!(
                "{}:docs/ai-workflow-metrics.jsonl:{}@{}",
                metrics.commit_oid, metrics.row_number, metrics.row_sha256
            ),
            "raw_git_object_and_row_digest_verified",
            "one_frozen_metrics_row",
            "allowlisted_metric_summary",
            "Frozen reported metrics are not provider billing or a definition of gate stage semantics",
        ) {
            return Err("graph metrics source policy invalid");
        }
    } else {
        let limit = base
            .metrics_unavailable
            .as_deref()
            .ok_or("graph metrics absence has no reason")?;
        if !bounded(limit, 160)
            || !source_matches(
                metric_source,
                "slice-supervisor",
                "frozen metrics row unavailable under this read",
                "unavailable",
                "no_frozen_metrics_comparison",
                "no_metric_values_projected",
                limit,
            )
        {
            return Err("graph metrics absence source policy invalid");
        }
    }
    Ok(())
}

fn push_edge(
    edges: &mut Vec<Edge>,
    kind: &str,
    from: &str,
    to: &str,
    evidence_class: EvidenceClass,
    owner: &str,
    source_ref: &str,
    source_revision: &str,
    limit: &str,
) {
    edges.push(Edge {
        id: format!("edge|{kind}|{from}|{to}"),
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        evidence_class,
        owner: owner.into(),
        source_ref: source_ref.into(),
        source_revision: source_revision.into(),
        limit: limit.into(),
    });
}

impl Observation {
    pub fn from_base(base: ObservationV2) -> Result<Self, &'static str> {
        base.validate()?;
        if base.base.run_id != RUN_ID
            || base.base.candidates.len() != 6
            || base.base.reviews.len() != 3
        {
            return Err("graph case identity invalid");
        }
        validate_graph_base_contract(&base)?;
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut checkpoint_ids = Vec::new();
        let mut gate_ids = Vec::new();
        for (candidate_index, candidate) in base.base.candidates.iter().enumerate() {
            let id = format!("c{}@{}", candidate_index + 1, candidate.commit_oid);
            let source = source_index(
                &base,
                "slice-checkpoint",
                &format!("{}@{}", candidate.checkpoint_ref, candidate.commit_oid),
            )?;
            nodes.push(Node {
                id: id.clone(),
                kind: "checkpoint".into(),
                evidence_class: EvidenceClass::Observed,
                owner: "slice-checkpoint".into(),
                source_class: "immutable_checkpoint_artifact".into(),
                source_ref: candidate.checkpoint_ref.clone(),
                source_revision: candidate.commit_oid.clone(),
                source_index: source,
                local_id: candidate.kind.clone(),
                label: format!("{} · {}", candidate.kind, candidate.commit_oid),
                observed_time: None,
                effective_time: None,
                measurement: None,
            });
            checkpoint_ids.push(id.clone());
            if let Some(gate) = &candidate.gate {
                if gate.receipt_sha256 != candidate.gate_digest
                    || gate.check_count != gate.checks.len()
                    || gate.passed_count
                        != gate.checks.iter().filter(|c| c.outcome == "passed").count()
                    || gate.source_kind
                        != if candidate_index < 4 {
                            "slice_builder_runner_output"
                        } else {
                            "coordinator_constructed_receipt"
                        }
                {
                    return Err("gate source does not bind checkpoint");
                }
                let gate_id = format!("g{}@{}", candidate_index + 1, gate.receipt_sha256);
                let source = source_index(
                    &base,
                    "slice-builder",
                    &format!("{}@{}", candidate.checkpoint_ref, gate.receipt_sha256),
                )?;
                nodes.push(Node {
                    id: gate_id.clone(),
                    kind: "gate_capture".into(),
                    evidence_class: EvidenceClass::Observed,
                    owner: "slice-builder".into(),
                    source_class: gate.source_kind.clone(),
                    source_ref: candidate.checkpoint_ref.clone(),
                    source_revision: gate.receipt_sha256.clone(),
                    source_index: source,
                    local_id: candidate.kind.clone(),
                    label: format!(
                        "{} · {}/{} reported checks",
                        candidate.kind, gate.passed_count, gate.check_count
                    ),
                    observed_time: None,
                    effective_time: None,
                    measurement: Some(Measurement {
                        name: "receipt_check_count".into(),
                        value: gate.check_count.to_string(),
                        unit: "top_level_checks".into(),
                    }),
                });
                push_edge(
                    &mut edges,
                    "has_gate_digest",
                    &id,
                    &gate_id,
                    EvidenceClass::Observed,
                    "slice-checkpoint",
                    &candidate.checkpoint_ref,
                    &candidate.commit_oid,
                    "Digest binding is not independent gate execution attestation",
                );
                gate_ids.push(gate_id);
            }
        }
        if gate_ids.len() != 5 {
            return Err("five bound gate captures required");
        }

        let mut review_ids = std::collections::HashMap::new();
        let mut episode_labels = std::collections::HashSet::new();
        for episode in &base.base.reviews {
            let (episode_tag, expected_revisions) = match episode.label.as_str() {
                "original-generic" => ("og", 4),
                "instruction" => ("in", 4),
                "replacement-generic" => ("rg", 6),
                _ => return Err("review episode label outside the case registry"),
            };
            if !episode_labels.insert(episode.label.as_str())
                || episode.revisions.len() != expected_revisions
                || episode
                    .revisions
                    .first()
                    .is_none_or(|r| r.revision != episode.head)
                || episode.revisions.iter().enumerate().any(|(i, revision)| {
                    revision.writer_generation == 0
                        || revision.writer_generation != episode.revisions[0].writer_generation
                        || revision.subjects.len() != 1
                        || revision.subjects[0].owner != "slice-checkpoint"
                        || revision.predecessor_revision.as_deref()
                            != episode
                                .revisions
                                .get(i + 1)
                                .map(|next| next.revision.as_str())
                })
            {
                return Err("review episode chain, generation or subject scope invalid");
            }
            let source = source_index(
                &base,
                "independent-review-state",
                &format!("{}@{}", episode.key, episode.head),
            )?;
            for revision in &episode.revisions {
                let id = format!("r{}@{}", episode_tag, revision.revision);
                if review_ids
                    .insert((episode.key.clone(), revision.revision.clone()), id.clone())
                    .is_some()
                {
                    return Err("duplicate review revision identity");
                }
                nodes.push(Node {
                    id: id.clone(),
                    kind: "review_revision".into(),
                    evidence_class: EvidenceClass::Observed,
                    owner: "independent-review-state".into(),
                    source_class: "linked_revision".into(),
                    source_ref: episode.key.clone(),
                    source_revision: revision.revision.clone(),
                    source_index: source,
                    local_id: format!("{}:{}", episode.label, revision.writer_generation),
                    label: format!(
                        "{} · {} · {}",
                        episode.label, revision.phase, revision.revision
                    ),
                    observed_time: None,
                    effective_time: None,
                    measurement: None,
                });
                for subject in &revision.subjects {
                    if subject.owner != "slice-checkpoint" {
                        continue;
                    }
                    let cp_id = base
                        .base
                        .candidates
                        .iter()
                        .enumerate()
                        .find(|(_, c)| {
                            c.checkpoint_ref == subject.reference
                                && c.commit_oid == subject.revision
                        })
                        .map(|(i, c)| format!("c{}@{}", i + 1, c.commit_oid))
                        .ok_or("review subject is not an admitted checkpoint")?;
                    if !checkpoint_ids.contains(&cp_id) {
                        return Err("review subject is not an admitted checkpoint");
                    }
                    push_edge(
                        &mut edges,
                        "reviews_exact_subject",
                        &id,
                        &cp_id,
                        EvidenceClass::Observed,
                        "independent-review-state",
                        &episode.key,
                        &revision.revision,
                        "Exact reviewed subject; not supervisor selection or acceptance",
                    );
                }
                for finding in &revision.findings {
                    let finding_id =
                        format!("f{}@{}|{}", episode_tag, revision.revision, finding.id);
                    nodes.push(Node {
                        id: finding_id.clone(),
                        kind: "finding_state".into(),
                        evidence_class: EvidenceClass::Observed,
                        owner: "independent-review-state".into(),
                        source_class: "finding_state_at_revision".into(),
                        source_ref: episode.key.clone(),
                        source_revision: revision.revision.clone(),
                        source_index: source,
                        local_id: finding.id.clone(),
                        label: format!(
                            "{} · {} · {}",
                            finding.id, finding.status, finding.severity
                        ),
                        observed_time: None,
                        effective_time: None,
                        measurement: None,
                    });
                    push_edge(
                        &mut edges,
                        "reports_finding_state",
                        &id,
                        &finding_id,
                        EvidenceClass::Observed,
                        "independent-review-state",
                        &episode.key,
                        &revision.revision,
                        "Finding state is not acceptance or remediation validity",
                    );
                }
            }
        }
        for episode in &base.base.reviews {
            for revision in &episode.revisions {
                if let Some(predecessor) = &revision.predecessor_revision {
                    let from = review_ids
                        .get(&(episode.key.clone(), predecessor.clone()))
                        .ok_or("missing review predecessor")?;
                    let to = review_ids
                        .get(&(episode.key.clone(), revision.revision.clone()))
                        .ok_or("missing review successor")?;
                    push_edge(
                        &mut edges,
                        "next_recorded_revision",
                        from,
                        to,
                        EvidenceClass::Observed,
                        "independent-review-state",
                        &episode.key,
                        &revision.revision,
                        "Revision succession does not prove code-change cause",
                    );
                }
            }
        }
        for link in &base.base.replacement_links {
            let replacement = base
                .base
                .reviews
                .iter()
                .find(|r| r.key == link.replacement_key)
                .ok_or("replacement episode missing")?;
            if replacement.head != link.replacement_revision {
                return Err("replacement authority must bind the episode head");
            }
            let head = replacement
                .revisions
                .iter()
                .find(|r| r.revision == replacement.head)
                .ok_or("replacement head missing")?;
            if !head.subjects.iter().any(|s| {
                s.owner == "slice-checkpoint"
                    && s.reference == base.base.candidates[4].checkpoint_ref
                    && s.revision == base.base.candidates[4].commit_oid
            }) {
                return Err("replacement authority head must have candidate-five subject");
            }
            let original = base
                .base
                .reviews
                .iter()
                .find(|r| r.key == link.original_key)
                .ok_or("original episode missing")?;
            if !original
                .revisions
                .iter()
                .any(|r| r.revision == link.original_reported_revision && r.phase == "reported")
            {
                return Err("original reported revision absent");
            }
            let from = review_ids
                .get(&(
                    link.replacement_key.clone(),
                    link.replacement_revision.clone(),
                ))
                .ok_or("replacement revision missing")?;
            let to = review_ids
                .get(&(
                    link.original_key.clone(),
                    link.original_reported_revision.clone(),
                ))
                .ok_or("original revision missing")?;
            push_edge(
                &mut edges,
                "replaces_reported_episode",
                from,
                to,
                EvidenceClass::Observed,
                "independent-review-state",
                &link.replacement_key,
                &link.replacement_revision,
                "Head-bound authority reference; not candidate-three direct authority",
            );
        }

        let provider_source = base.base.source_reads.len();
        let schedule_source = provider_source + 1;
        let metric_source = provider_source + 2;
        if base
            .source_reads
            .get(provider_source)
            .is_none_or(|s| s.owner != "slice-builder captured provider output")
            || base
                .source_reads
                .get(schedule_source)
                .is_none_or(|s| s.owner != "role-scheduler")
            || base
                .source_reads
                .get(metric_source)
                .is_none_or(|s| s.owner != "slice-supervisor")
        {
            return Err("extended source inventory invalid");
        }
        let mut result_ids = Vec::new();
        for attempt in &base.provider.attempts {
            let id = format!("p@{}:{}", attempt.result_uuid, attempt.source_line_sha256);
            nodes.push(Node {
                id: id.clone(),
                kind: "provider_result".into(),
                evidence_class: EvidenceClass::Observed,
                owner: "slice-builder captured provider output".into(),
                source_class: "captured_cli_result".into(),
                source_ref: format!("{}:{}", attempt.source_file_sha256, attempt.source_line),
                source_revision: attempt.source_line_sha256.clone(),
                source_index: provider_source,
                local_id: attempt.result_uuid.clone(),
                label: format!(
                    "captured {} · {} · {}",
                    attempt.result_uuid, attempt.review_label, attempt.terminal_reason
                ),
                observed_time: None,
                effective_time: None,
                measurement: Some(Measurement {
                    name: "cli_reported_cost".into(),
                    value: attempt.cost_usd.clone(),
                    unit: "USD_literal".into(),
                }),
            });
            result_ids.push(id);
        }
        result_ids.sort();
        let mut capture_hashes = base
            .provider
            .attempts
            .iter()
            .map(|a| a.source_file_sha256.as_str())
            .collect::<Vec<_>>();
        capture_hashes.sort_unstable();
        capture_hashes.dedup();
        if capture_hashes.len() != 2 {
            return Err("provider capture inventory invalid");
        }
        let aggregate_id = format!("cost@{}:{}", capture_hashes[0], capture_hashes[1]);
        nodes.push(Node {
            id: aggregate_id.clone(),
            kind: "provider_cost_sum".into(),
            evidence_class: EvidenceClass::MechanicallyDerived,
            owner: "s5-projection".into(),
            source_class: "eight_distinct_captured_results".into(),
            source_ref: "eight_admitted_result_uuids".into(),
            source_revision: aggregate_id.clone(),
            source_index: provider_source,
            local_id: "captured_cli_cost_sum".into(),
            label: "sum of eight distinct captured CLI result literals".into(),
            observed_time: None,
            effective_time: None,
            measurement: Some(Measurement {
                name: "captured_cli_cost_sum".into(),
                value: base.provider.cost_literal_sum_usd.clone(),
                unit: "USD_literal".into(),
            }),
        });
        for id in result_ids {
            push_edge(
                &mut edges,
                "includes_distinct_result",
                &aggregate_id,
                &id,
                EvidenceClass::MechanicallyDerived,
                "s5-projection",
                "eight_admitted_result_uuids",
                &aggregate_id,
                "Unique captured UUIDs; not billing authority or multi-dimension cost attribution",
            );
        }

        let scheduler = &base.scheduler;
        let scheduler_id = format!("s@{}:{}", scheduler.store_revision, scheduler.row_sha256);
        nodes.push(Node {
            id: scheduler_id.clone(),
            kind: "scheduler_current_row".into(),
            evidence_class: EvidenceClass::Observed,
            owner: "role-scheduler".into(),
            source_class: "current_retained_row".into(),
            source_ref: scheduler.item_id.clone(),
            source_revision: format!("{}:{}", scheduler.store_revision, scheduler.row_sha256),
            source_index: schedule_source,
            local_id: scheduler.item_id.clone(),
            label: format!("current {} row · {}", scheduler.status, scheduler.item_id),
            observed_time: Some(TimeValue {
                meaning: "current_row_updated_at".into(),
                value: scheduler.updated_at.clone(),
            }),
            effective_time: Some(TimeValue {
                meaning: "current_row_due_at".into(),
                value: scheduler.due_at.clone(),
            }),
            measurement: None,
        });
        let (_, result_revision) = scheduler
            .result_ref
            .rsplit_once('@')
            .ok_or("scheduler result reference invalid")?;
        let result = review_ids
            .get(&(crate::REPLACEMENT_REVIEW_KEY.into(), result_revision.into()))
            .ok_or("scheduler result revision absent")?;
        push_edge(
            &mut edges,
            "refers_to_review_result",
            &scheduler_id,
            result,
            EvidenceClass::Observed,
            "role-scheduler",
            &scheduler.item_id,
            &scheduler.row_sha256,
            "Current acknowledged row only; no schedule transition or provider-result cause",
        );

        if let Some(metrics) = &base.metrics {
            let metric_id = format!(
                "m@{}:{}:{}",
                metrics.commit_oid, metrics.row_number, metrics.row_sha256
            );
            nodes.push(Node {
                id: metric_id.clone(),
                kind: "frozen_metrics_row".into(),
                evidence_class: EvidenceClass::Observed,
                owner: "slice-supervisor".into(),
                source_class: "frozen_metrics_row".into(),
                source_ref: format!(
                    "{}:docs/ai-workflow-metrics.jsonl:{}",
                    metrics.commit_oid, metrics.row_number
                ),
                source_revision: metrics.row_sha256.clone(),
                source_index: metric_source,
                local_id: "stage_total".into(),
                label: format!(
                    "frozen row · stage_total {} (meaning unresolved)",
                    metrics.stage_total
                ),
                observed_time: None,
                effective_time: None,
                measurement: Some(Measurement {
                    name: "historical_stage_total".into(),
                    value: metrics.stage_total.to_string(),
                    unit: "stages_unestablished".into(),
                }),
            });
            let gate = &base.base.candidates[4]
                .gate
                .as_ref()
                .ok_or("candidate-five gate absent")?;
            if metrics.gate_digest != gate.receipt_sha256 {
                return Err("frozen metric gate digest mismatch");
            }
            let discrepancy_id = format!("d@{}:{}", gate.receipt_sha256, metric_id);
            nodes.push(Node {
                id: discrepancy_id.clone(),
                kind: "measurement_discrepancy".into(),
                evidence_class: EvidenceClass::MechanicallyDerived,
                owner: "s5-projection".into(),
                source_class: "receipt_metric_comparison".into(),
                source_ref: gate.receipt_sha256.clone(),
                source_revision: metrics.row_sha256.clone(),
                source_index: metric_source,
                local_id: "gate_checks_versus_stage_total".into(),
                label: format!(
                    "{} gate checks / {} metric stages · meanings differ",
                    gate.check_count, metrics.stage_total
                ),
                observed_time: None,
                effective_time: None,
                measurement: None,
            });
            push_edge(
                &mut edges,
                "compares_receipt",
                &discrepancy_id,
                &gate_ids[4],
                EvidenceClass::MechanicallyDerived,
                "s5-projection",
                &gate.receipt_sha256,
                &metrics.row_sha256,
                "Different units; not an eight-check gate",
            );
            push_edge(
                &mut edges,
                "compares_frozen_row",
                &discrepancy_id,
                &metric_id,
                EvidenceClass::MechanicallyDerived,
                "s5-projection",
                &metrics.commit_oid,
                &metrics.row_sha256,
                "Historical stage semantics unestablished",
            );
        }
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        edges.sort_by(|a, b| a.id.cmp(&b.id));
        let mut gaps = vec![
            "No admitted contract-repair relation or realized-savings proof".into(),
            "Remediation-phase revisions and finding states are present; code-change cause and validity are not established".into(),
            "Captured HTTP 429 and current scheduler row are adjacent case facts, not a UUID-to-schedule causal edge".into(),
            "Current scheduler row has no mutation/presentation history".into(),
            "Selected-perspective and completion authority consequences are not projected".into(),
        ];
        if base.metrics.is_none() {
            gaps.push("Optional frozen metrics row unavailable under this read; no stage comparison projected".into());
        }
        gaps.push("Contract-repair owner and full scheduler mutation ledger are outside the admitted registry".into());
        let graph = Self {
            protocol_version: 3,
            projection_version: 1,
            operation: OPERATION.into(),
            registry_version: 1,
            base,
            registry: registry(),
            nodes,
            edges,
            gaps,
        };
        if graph.nodes.len() > 128 || graph.edges.len() > 192 || graph.registry.len() > 16 {
            return Err("graph inventory exceeds fixed bounds");
        }
        let mut node_ids = std::collections::HashSet::new();
        for node in &graph.nodes {
            if !node_ids.insert(node.id.as_str())
                || !graph.registry.iter().any(|entry| {
                    entry.owner == node.owner
                        && entry.source_class == node.source_class
                        && entry.kind == node.kind
                })
            {
                return Err("duplicate or unregistered graph node");
            }
        }
        let mut edge_ids = std::collections::HashSet::new();
        for edge in &graph.edges {
            let origin = graph
                .nodes
                .iter()
                .find(|node| node.id == edge.from)
                .ok_or("graph edge origin missing")?;
            if !edge_ids.insert(edge.id.as_str())
                || !node_ids.contains(edge.to.as_str())
                || !graph.registry.iter().any(|entry| {
                    entry.kind == origin.kind
                        && entry.owner == origin.owner
                        && entry.source_class == origin.source_class
                        && entry.legal_outgoing.iter().any(|kind| kind == &edge.kind)
                })
            {
                return Err("duplicate, dangling or unregistered graph edge");
            }
        }
        Ok(graph)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.protocol_version != 3
            || self.projection_version != 1
            || self.operation != OPERATION
            || self.registry_version != 1
            || self.nodes.len() > 128
            || self.edges.len() > 192
        {
            return Err("graph version or inventory invalid");
        }
        let expected = Self::from_base(self.base.clone())?;
        if self.registry != expected.registry
            || self.nodes != expected.nodes
            || self.edges != expected.edges
            || self.gaps != expected.gaps
        {
            return Err("graph does not match admitted owner facts and legal joins");
        }
        let len = serde_json::to_vec(&Reply::Observed {
            observation: self.clone(),
        })
        .map_err(|_| "graph encoding failed")?
        .len();
        if len > MAX_FRAME {
            return Err("graph exceeds response frame bound");
        }
        Ok(())
    }
}
