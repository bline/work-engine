//! Closed serde types for the `portfolio-decision-v1` record (contract 4.1).
//!
//! The record is authored; this crate only checks it, so every type derives both `Serialize`
//! and `Deserialize` for fixtures and for round-trip tests. Every struct carries
//! `deny_unknown_fields`; collections are `Vec`; there is no map, no float and no untyped value
//! (contract S3, S4). A nullable field is a required key whose value may be `null`.
//!
//! The vocabulary is deliberately closed and has no way to say "reject": a proposal that is
//! not selected is `defer` or `exclude_from_current_portfolio`.

use proposal_evaluation::model::FileRef;
use serde::{Deserialize, Deserializer, Serialize};

pub const PORTFOLIO_SCHEMA: &str = "portfolio-decision-v1";

fn required<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortfolioDecision {
    pub schema: String,
    pub portfolio_id: String,
    pub revision: u64,
    #[serde(deserialize_with = "required")]
    pub supersedes: Option<FileRef>,
    pub decided_at: String,
    pub authority: Authority,
    pub strategic_basis: StrategicBasis,
    pub proposal_set: Vec<SetMember>,
    pub relationship_graph: RelationshipGraph,
    pub evaluation_basis: Vec<EvaluationRef>,
    pub readiness_basis: Vec<ReadinessRef>,
    pub cross_proposal_analysis: CrossProposalAnalysis,
    pub dispositions: Vec<Disposition>,
    pub non_effects: NonEffects,
    pub limitations: Vec<String>,
}

/// `{decision_owner, exercised_by, evidence}` (the proposal-decision authority shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub decision_owner: String,
    pub exercised_by: String,
    pub evidence: String,
}

/// Digest-only (UD4): the handoff is re-hashed, never parsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategicBasis {
    pub handoff: FileRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetMember {
    pub proposal_id: String,
    pub packet: FileRef,
    #[serde(deserialize_with = "required")]
    pub decision: Option<FileRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipSource {
    PacketRelationships,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipGraph {
    pub repository_revision: String,
    pub source: RelationshipSource,
}

/// Which evaluation record a basis entry names (Erratum 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationKind {
    Comparison,
    EstimateSet,
}

/// `{kind, path, sha256}`: a `FileRef` tagged with the class it must have.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationRef {
    pub kind: EvaluationKind,
    pub path: String,
    pub sha256: String,
}

impl EvaluationRef {
    pub fn file_ref(&self) -> FileRef {
        FileRef {
            path: self.path.clone(),
            sha256: self.sha256.clone(),
        }
    }
}

/// `{proposal_id, decision_id, assessment}` (Erratum 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessRef {
    pub proposal_id: String,
    pub decision_id: String,
    pub assessment: FileRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossProposalAnalysis {
    pub unlock: Vec<Unlock>,
    pub mutually_exclusive_under_basis: Vec<MutualExclusion>,
}

/// One relationship edge an unlock relies on, as found in the packet bytes of `source`: the
/// relationship `{type, target_id: target}`. `type` is a string so an edge outside the
/// permitted types can be written down and then refused (`unlock_type_invalid`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipEdge {
    pub source: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unlock {
    pub from: String,
    pub to: String,
    pub relationship_basis: Vec<RelationshipEdge>,
    pub judgment: String,
    pub attributed_to: String,
}

/// Why a set cannot be selected together: a decision consequence, never graph truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BasisKind {
    Capacity,
    Policy,
    Authority,
    Timing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutualExclusion {
    pub proposals: Vec<String>,
    pub basis_kind: BasisKind,
    pub reason: String,
    pub attributed_to: String,
}

/// One disposition per proposal, internally tagged by `disposition` (contract 4.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
pub enum Disposition {
    Prioritize {
        proposal_id: String,
        rationale: String,
        rank: u32,
    },
    Sequence {
        proposal_id: String,
        rationale: String,
        after: Vec<String>,
    },
    Defer {
        proposal_id: String,
        rationale: String,
        revisit_when: Vec<String>,
    },
    ExcludeFromCurrentPortfolio {
        proposal_id: String,
        rationale: String,
        reason: String,
    },
    SelectForCampaign {
        proposal_id: String,
        rationale: String,
        campaign_ref: String,
    },
}

impl Disposition {
    pub fn proposal_id(&self) -> &str {
        match self {
            Disposition::Prioritize { proposal_id, .. }
            | Disposition::Sequence { proposal_id, .. }
            | Disposition::Defer { proposal_id, .. }
            | Disposition::ExcludeFromCurrentPortfolio { proposal_id, .. }
            | Disposition::SelectForCampaign { proposal_id, .. } => proposal_id,
        }
    }

    pub fn rationale(&self) -> &str {
        match self {
            Disposition::Prioritize { rationale, .. }
            | Disposition::Sequence { rationale, .. }
            | Disposition::Defer { rationale, .. }
            | Disposition::ExcludeFromCurrentPortfolio { rationale, .. }
            | Disposition::SelectForCampaign { rationale, .. } => rationale,
        }
    }

    /// The serialised `disposition` tag.
    pub fn tag(&self) -> &'static str {
        match self {
            Disposition::Prioritize { .. } => "prioritize",
            Disposition::Sequence { .. } => "sequence",
            Disposition::Defer { .. } => "defer",
            Disposition::ExcludeFromCurrentPortfolio { .. } => "exclude_from_current_portfolio",
            Disposition::SelectForCampaign { .. } => "select_for_campaign",
        }
    }
}

/// The serialised disposition tags, in declaration order. A test keeps this in step with
/// [`Disposition`] and asserts it is disjoint from the proposal-decision vocabulary.
pub const DISPOSITION_TAGS: [&str; 5] = [
    "prioritize",
    "sequence",
    "defer",
    "exclude_from_current_portfolio",
    "select_for_campaign",
];

/// Constant all-false block (contract S10). A portfolio decision informs; it changes no
/// lifecycle, rejects nothing, mutates no graph, authorises no implementation, executes no schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonEffects {
    pub proposal_lifecycle_mutated: bool,
    pub proposal_rejected: bool,
    pub relationship_graph_mutated: bool,
    pub implementation_authorized: bool,
    pub schedule_executed: bool,
}

impl NonEffects {
    pub fn fields(&self) -> [(&'static str, bool); 5] {
        [
            (
                "proposal_lifecycle_mutated",
                self.proposal_lifecycle_mutated,
            ),
            ("proposal_rejected", self.proposal_rejected),
            (
                "relationship_graph_mutated",
                self.relationship_graph_mutated,
            ),
            ("implementation_authorized", self.implementation_authorized),
            ("schedule_executed", self.schedule_executed),
        ]
    }
}
