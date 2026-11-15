//! Evaluation of a supplied realization descriptor against a candidate's per-condition
//! requirements.
//!
//! A descriptor is a **claim** by its supplier. It is not admission evidence: nothing in this
//! module can turn a claim into an established fact, and a claim that matches every requirement
//! is reported as `ClaimedCompatible`, never as met. The crate classifies no tools: tool names
//! are carried verbatim, and the owner of a tool-to-capability or tool-to-mutation mapping does
//! not exist yet, so a tool list can only leave a requirement unestablished. A known
//! incompatibility is reported only where the descriptor itself states a conflicting value.

use serde::{Deserialize, Serialize};
use work_engine_organization::{CandidateCondition, Dimension};

/// What the supplier states about mutation. The supplier's word, in both directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationClaim {
    /// The supplier states that the realization cannot mutate.
    ReadOnlyClaimed,
    /// The supplier states that the realization may write.
    WritesPossible,
    /// The supplier makes no statement.
    NotEstablished,
}

/// A caller-supplied realization descriptor. Every field is a claim; absent means unstated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizationClaim {
    pub provider_family: Option<String>,
    pub execution_confinement: Option<String>,
    pub mutation: MutationClaim,
    /// Tool names, verbatim. Never classified by this crate.
    pub granted_tools: Vec<String>,
    pub continuity: Option<String>,
}

/// The standing of one requirement against the supplied claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// The claim states a value that the requirement allows. Still only a claim.
    ClaimedCompatible,
    /// The claim states a value that the requirement does not allow.
    KnownIncompatible,
    /// The claim is absent or cannot be related to the requirement by any owner.
    Unestablished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizationCheck {
    pub property: String,
    pub required: String,
    pub claimed: String,
    pub disposition: Disposition,
    pub basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionRealization {
    pub condition: String,
    pub checks: Vec<RealizationCheck>,
}

/// The evaluation of one descriptor. Its `standing` is fixed: the descriptor is a claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizationEvaluation {
    pub standing: String,
    pub conditions: Vec<ConditionRealization>,
}

pub(crate) const CLAIM_STANDING: &str = "supplied_claim_not_admission_evidence";

fn text(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "<unstated>".to_owned())
}

fn against_allowed(
    property: &str,
    allowed: &[String],
    claimed: &Option<String>,
) -> RealizationCheck {
    let required = allowed.join(",");
    let (disposition, basis) = match claimed {
        None => (
            Disposition::Unestablished,
            "the descriptor states no value".to_owned(),
        ),
        Some(value) if allowed.iter().any(|a| a == value) => (
            Disposition::ClaimedCompatible,
            "the descriptor claims an allowed value; the claim is not admission evidence"
                .to_owned(),
        ),
        Some(value) => (
            Disposition::KnownIncompatible,
            format!("the descriptor states `{value}`, which the requirement does not allow"),
        ),
    };
    RealizationCheck {
        property: property.to_owned(),
        required,
        claimed: text(claimed),
        disposition,
        basis,
    }
}

fn evaluate_condition(
    condition: &CandidateCondition,
    claim: &RealizationClaim,
) -> ConditionRealization {
    let mut checks = Vec::new();

    let continuity_required = condition.normative.continuity.as_str().to_owned();
    checks.push(against_allowed(
        "continuity",
        std::slice::from_ref(&continuity_required),
        &claim.continuity,
    ));

    // `normative.mutation` has the single value `none`: any state that the realization may
    // write is incompatible, and the supplier's silence is unestablished. A read-only claim is
    // compatible only as a claim, and a tool list beside it cannot be reconciled with it here.
    let mutation_required = condition.normative.mutation.as_str().to_owned();
    let (mutation_disposition, mutation_basis) = match claim.mutation {
        MutationClaim::WritesPossible => (
            Disposition::KnownIncompatible,
            "the descriptor states that the realization may write; the requirement is no mutation"
                .to_owned(),
        ),
        MutationClaim::NotEstablished => (
            Disposition::Unestablished,
            "the descriptor makes no statement about mutation".to_owned(),
        ),
        MutationClaim::ReadOnlyClaimed if !claim.granted_tools.is_empty() => (
            Disposition::Unestablished,
            "the descriptor claims read-only but names tools; no owner classifies tools, so the claim cannot be checked against them"
                .to_owned(),
        ),
        MutationClaim::ReadOnlyClaimed => (
            Disposition::ClaimedCompatible,
            "the descriptor claims read-only with no tools; the claim is not admission evidence"
                .to_owned(),
        ),
    };
    checks.push(RealizationCheck {
        property: "mutation".to_owned(),
        required: mutation_required,
        claimed: format!("{:?}", claim.mutation),
        disposition: mutation_disposition,
        basis: mutation_basis,
    });

    // `normative.capabilities` is empty for every candidate that preparation accepts. Tool names
    // cannot be related to a capability set by any owner, so named tools are never "within" it.
    let capabilities_required = if condition.normative.capabilities.is_empty() {
        "none".to_owned()
    } else {
        condition
            .normative
            .capabilities
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(",")
    };
    let (capabilities_disposition, capabilities_basis) = if claim.granted_tools.is_empty() {
        (
            Disposition::ClaimedCompatible,
            "the descriptor names no tools; the claim is not admission evidence".to_owned(),
        )
    } else {
        (
            Disposition::Unestablished,
            "the descriptor names tools; no owner maps tools to capabilities".to_owned(),
        )
    };
    checks.push(RealizationCheck {
        property: "capabilities".to_owned(),
        required: capabilities_required,
        claimed: if claim.granted_tools.is_empty() {
            "none".to_owned()
        } else {
            claim.granted_tools.join(",")
        },
        disposition: capabilities_disposition,
        basis: capabilities_basis,
    });

    for constraint in &condition.realization_constraints {
        let allowed: Vec<String> = constraint
            .allowed
            .iter()
            .map(|class| class.as_str().to_owned())
            .collect();
        let (property, claimed) = match constraint.dimension {
            Dimension::ProviderFamily => ("realization:provider_family", &claim.provider_family),
            Dimension::ExecutionConfinement => (
                "realization:execution_confinement",
                &claim.execution_confinement,
            ),
        };
        checks.push(against_allowed(property, &allowed, claimed));
    }

    ConditionRealization {
        condition: condition.id.as_str().to_owned(),
        checks,
    }
}

pub(crate) fn evaluate(
    conditions: &[&CandidateCondition],
    claim: &RealizationClaim,
) -> RealizationEvaluation {
    RealizationEvaluation {
        standing: CLAIM_STANDING.to_owned(),
        conditions: conditions
            .iter()
            .map(|condition| evaluate_condition(condition, claim))
            .collect(),
    }
}
