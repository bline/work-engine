//! Versioned, case-scoped contract-repair relation over the admitted S5 graph.
//! Edge provenance is tabled on the wire to retain the fixed frame bound.
use crate::graph::{self, Edge, EvidenceClass, Node, RegistryEntry};
use crate::{
    ContractRepairRead, MAX_FRAME, ORIGINAL_REPORTED_REVISION, ORIGINAL_REVIEW_KEY, ObservationV2,
    REPAIR_COMMIT_OID, SourceRead,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const OPERATION: &str = "historical_s5.graph.v2";
pub const INTERPRETER: &str = "s5-contract-applicability-interpreter";
pub const RULE_REF: &str = "interpretation-rule:s5-reported-active-continuation";
pub const RULE_REVISION: &str = "v1@historical_s5.graph.v2";
const INTERPRETATION_LIMIT: &str = "Contract applicability interpretation; no S5 adoption, remediation event, cause of replacement, or savings proof";

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
            return Err("unsupported graph v2 operation");
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
#[serde(deny_unknown_fields)]
pub struct EdgeSource {
    pub owner: String,
    pub source_ref: String,
    pub source_revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterpretationSupport {
    pub repair_commit_oid: String,
    pub repair_script_blob_oid: String,
    pub repair_skill_blob_oid: String,
    pub review_episode_key: String,
    pub review_revision: String,
}
impl InterpretationSupport {
    fn fixed(repair: &ContractRepairRead) -> Self {
        Self {
            repair_commit_oid: repair.commit_oid.clone(),
            repair_script_blob_oid: repair.script_after_blob_oid.clone(),
            repair_skill_blob_oid: repair.skill_after_blob_oid.clone(),
            review_episode_key: ORIGINAL_REVIEW_KEY.into(),
            review_revision: ORIGINAL_REPORTED_REVISION.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactEdge {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub evidence_class: EvidenceClass,
    pub edge_source_index: usize,
    pub limit: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub support: Option<InterpretationSupport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterpreterRule {
    pub owner: String,
    pub source_ref: String,
    pub source_revision: String,
    pub evidence_class: EvidenceClass,
    pub legal_edge: String,
    pub origin_kind: String,
    pub target_kind: String,
    pub confidentiality: String,
    pub limit: String,
}
impl InterpreterRule {
    fn fixed() -> Self {
        Self {
            owner: INTERPRETER.into(),
            source_ref: RULE_REF.into(),
            source_revision: RULE_REVISION.into(),
            evidence_class: EvidenceClass::SemanticInterpretation,
            legal_edge: "clarifies_reported_continuation_for".into(),
            origin_kind: "contract_revision".into(),
            target_kind: "review_revision".into(),
            confidentiality: "private_local".into(),
            limit: INTERPRETATION_LIMIT.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub protocol_version: u8,
    pub projection_version: u8,
    pub operation: String,
    pub registry_version: u8,
    pub base: ObservationV2,
    pub repair: ContractRepairRead,
    pub repair_source: SourceRead,
    pub interpreter_rule: InterpreterRule,
    pub registry: Vec<RegistryEntry>,
    pub nodes: Vec<Node>,
    pub edges: Vec<CompactEdge>,
    pub edge_sources: Vec<EdgeSource>,
    pub gaps: Vec<String>,
}

fn repair_source(repair: &ContractRepairRead) -> SourceRead {
    SourceRead {
        owner: repair.source_owner.clone(),
        identity: format!(
            "{}:{}@{}:{}@{}",
            repair.commit_oid,
            repair.script_path,
            repair.script_after_blob_oid,
            repair.skill_path,
            repair.skill_after_blob_oid
        ),
        integrity: "raw_git_commit_tree_blob_and_sha256_verified".into(),
        completeness: repair.completeness.clone(),
        confidentiality: repair.confidentiality.clone(),
        limit: repair.source_limit.clone(),
    }
}

fn edge(
    kind: &str,
    from: &str,
    to: &str,
    class: EvidenceClass,
    owner: &str,
    source_ref: &str,
    revision: &str,
    limit: &str,
) -> Edge {
    Edge {
        id: format!("edge|{kind}|{from}|{to}"),
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        evidence_class: class,
        owner: owner.into(),
        source_ref: source_ref.into(),
        source_revision: revision.into(),
        limit: limit.into(),
    }
}

fn interpretation_edge_id(kind: &str, from: &str, to: &str, rule_revision: &str) -> String {
    format!("edge|{kind}|{from}|{to}|rule@{rule_revision}")
}

impl Observation {
    pub fn from_base(
        base: ObservationV2,
        repair: ContractRepairRead,
    ) -> Result<Self, &'static str> {
        repair.validate()?;
        let old = graph::Observation::from_base(base.clone())?;
        let original = base.base.reviews.first().ok_or("original episode absent")?;
        if original.key != ORIGINAL_REVIEW_KEY
            || original.head != ORIGINAL_REPORTED_REVISION
            || !original.revisions.iter().any(|r| {
                r.revision == ORIGINAL_REPORTED_REVISION
                    && r.status == "active"
                    && r.phase == "reported"
            })
        {
            return Err("reported revision does not support contract interpretation");
        }
        let source_index = base.source_reads.len();
        let before = format!("contract@{}", repair.script_before_blob_oid);
        let after = format!("contract@{}", repair.script_after_blob_oid);
        let reported = format!("rog@{}", ORIGINAL_REPORTED_REVISION);
        if !old
            .nodes
            .iter()
            .any(|n| n.id == reported && n.kind == "review_revision")
        {
            return Err("reported revision graph node absent");
        }
        let mut registry = old.registry;
        registry.push(RegistryEntry {
            owner: "independent-review-state".into(),
            source_class: repair.source_class.clone(),
            kind: "contract_revision".into(),
            confidentiality: "private_local".into(),
            completeness: repair.completeness.clone(),
            limit: repair.source_limit.clone(),
            legal_outgoing: vec!["revises_contract".into()],
        });
        registry.sort_by(|a, b| {
            (&a.owner, &a.source_class, &a.kind).cmp(&(&b.owner, &b.source_class, &b.kind))
        });
        let mut nodes = old.nodes;
        for (id, label, revision, local_id) in [
            (
                &before,
                "parent rule · remediation phase only",
                &repair.script_before_blob_oid,
                "before",
            ),
            (
                &after,
                "repair rule · active reported or remediation",
                &repair.script_after_blob_oid,
                "after",
            ),
        ] {
            nodes.push(Node {
                id: id.clone(),
                kind: "contract_revision".into(),
                evidence_class: EvidenceClass::Observed,
                owner: "independent-review-state".into(),
                source_class: repair.source_class.clone(),
                source_ref: repair.script_path.clone(),
                source_revision: revision.clone(),
                source_index,
                local_id: local_id.into(),
                label: label.into(),
                observed_time: None,
                effective_time: None,
                measurement: None,
            });
        }
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        let mut full_edges = old.edges;
        full_edges.push(edge(
            "revises_contract",
            &before,
            &after,
            EvidenceClass::Observed,
            "independent-review-state",
            &format!("git:{}", REPAIR_COMMIT_OID),
            &repair.commit_oid,
            "Exact parent/child source contract revision; no S5 adoption proof",
        ));
        let mut interpretation_edge = edge(
            "clarifies_reported_continuation_for",
            &after,
            &reported,
            EvidenceClass::SemanticInterpretation,
            INTERPRETER,
            RULE_REF,
            RULE_REVISION,
            INTERPRETATION_LIMIT,
        );
        interpretation_edge.id = interpretation_edge_id(
            &interpretation_edge.kind,
            &interpretation_edge.from,
            &interpretation_edge.to,
            RULE_REVISION,
        );
        full_edges.push(interpretation_edge);
        full_edges.sort_by(|a, b| a.id.cmp(&b.id));
        let mut edge_sources = Vec::<EdgeSource>::new();
        let mut indexes = HashMap::<(String, String, String), usize>::new();
        let mut edges = Vec::with_capacity(full_edges.len());
        for e in full_edges {
            let key = (
                e.owner.clone(),
                e.source_ref.clone(),
                e.source_revision.clone(),
            );
            let index = if let Some(index) = indexes.get(&key) {
                *index
            } else {
                let index = edge_sources.len();
                edge_sources.push(EdgeSource {
                    owner: key.0.clone(),
                    source_ref: key.1.clone(),
                    source_revision: key.2.clone(),
                });
                indexes.insert(key, index);
                index
            };
            let support = if e.kind == "clarifies_reported_continuation_for" {
                Some(InterpretationSupport::fixed(&repair))
            } else {
                None
            };
            edges.push(CompactEdge {
                id: e.id,
                kind: e.kind,
                from: e.from,
                to: e.to,
                evidence_class: e.evidence_class,
                edge_source_index: index,
                limit: e.limit,
                support,
            });
        }
        let mut gaps = old.gaps;
        gaps.retain(|g| {
            !g.starts_with("No admitted contract-repair relation")
                && !g.starts_with("Contract-repair owner and full scheduler")
        });
        gaps.push("Admitted contract source revision is not evidence of S5 deployment, use, replacement cause or savings".into());
        gaps.push("Full scheduler mutation ledger remains outside the admitted registry".into());
        Ok(Self {
            protocol_version: 4,
            projection_version: 2,
            operation: OPERATION.into(),
            registry_version: 2,
            base,
            repair_source: repair_source(&repair),
            repair,
            interpreter_rule: InterpreterRule::fixed(),
            registry,
            nodes,
            edges,
            edge_sources,
            gaps,
        })
    }

    pub fn expanded_edges(&self) -> Result<Vec<Edge>, &'static str> {
        self.edges
            .iter()
            .map(|e| {
                let source = self
                    .edge_sources
                    .get(e.edge_source_index)
                    .ok_or("dangling edge source")?;
                Ok(Edge {
                    id: e.id.clone(),
                    kind: e.kind.clone(),
                    from: e.from.clone(),
                    to: e.to.clone(),
                    evidence_class: e.evidence_class.clone(),
                    owner: source.owner.clone(),
                    source_ref: source.source_ref.clone(),
                    source_revision: source.source_revision.clone(),
                    limit: e.limit.clone(),
                })
            })
            .collect()
    }

    pub fn source(&self, index: usize) -> Option<&SourceRead> {
        self.base.source_reads.get(index).or_else(|| {
            if index == self.base.source_reads.len() {
                Some(&self.repair_source)
            } else {
                None
            }
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.protocol_version != 4
            || self.projection_version != 2
            || self.operation != OPERATION
            || self.registry_version != 2
            || self.nodes.len() > 128
            || self.edges.len() > 192
            || self.edge_sources.len() > 64
        {
            return Err("graph v2 version or inventory invalid");
        }
        let expected = Self::from_base(self.base.clone(), self.repair.clone())?;
        if self.repair_source.owner != expected.repair_source.owner
            || self.repair_source.identity != expected.repair_source.identity
            || self.repair_source.integrity != expected.repair_source.integrity
            || self.repair_source.completeness != expected.repair_source.completeness
            || self.repair_source.confidentiality != expected.repair_source.confidentiality
            || self.repair_source.limit != expected.repair_source.limit
            || self.interpreter_rule != expected.interpreter_rule
            || self.registry != expected.registry
            || self.nodes != expected.nodes
            || self.edges != expected.edges
            || self.edge_sources != expected.edge_sources
            || self.gaps != expected.gaps
        {
            return Err("graph v2 differs from exact owner facts or interpretation rule");
        }
        let size = serde_json::to_vec(&Reply::Observed {
            observation: self.clone(),
        })
        .map_err(|_| "graph v2 encoding failed")?
        .len();
        if size > MAX_FRAME {
            return Err("graph v2 exceeds response frame bound");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_rule_revision_is_part_of_interpretation_identity() {
        let kind = "clarifies_reported_continuation_for";
        let from = "contract@same-immutable-source";
        let to = "review@same-reported-revision";
        let first = interpretation_edge_id(kind, from, to, RULE_REVISION);
        assert_eq!(first, interpretation_edge_id(kind, from, to, RULE_REVISION));
        assert_ne!(
            first,
            interpretation_edge_id(kind, from, to, "v2@historical_s5.graph.v2")
        );
        let observed = edge(
            "revises_contract",
            from,
            to,
            EvidenceClass::Observed,
            "independent-review-state",
            "git:fixed",
            "fixed",
            "fixed",
        );
        assert_eq!(observed.id, format!("edge|revises_contract|{from}|{to}"));
    }
}
