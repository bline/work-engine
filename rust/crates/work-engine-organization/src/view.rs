//! `OrganizationView` (contract 8.1, amendment section 7): a pure projection of one candidate.
//! It permits nothing and has no status fields.

use crate::derive::{
    AuthorityRequirement, CandidateCondition, ComponentBindingRequirement, OrganizationCandidate,
    RelationshipKind, SeparationRequirement,
};
use crate::ids::Id;
use crate::schema::{Normative, RealizationConstraint};

/// An obligation and the vantage that serves it (from `serves`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ViewObligation {
    pub id: Id,
    pub served_by: String,
}

/// A vantage. The builder vantage has no effective requirements and no conditions of its own: it
/// consumes, it is not reviewed against conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ViewVantage {
    pub id: String,
    pub effective: Option<Normative>,
    pub effective_constraints: Vec<RealizationConstraint>,
    pub conditions: Vec<Id>,
}

/// `hosts_execution`: a profile-fixed assignment, not a standing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Hosting {
    pub component: String,
    pub vantage: String,
}

/// Episode access for one reviewer vantage, from `writes_episode` and `reads_episode`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EpisodeAccess {
    pub vantage: String,
    pub writer: String,
    pub readers: Vec<String>,
}

/// `terminalizes`: names the component only. It is not evidence that terminalization happened.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Terminalization {
    pub component: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OrganizationView {
    pub obligations: Vec<ViewObligation>,
    pub vantages: Vec<ViewVantage>,
    pub conditions: Vec<CandidateCondition>,
    pub hosting: Vec<Hosting>,
    pub episode_access: Vec<EpisodeAccess>,
    pub terminalization: Option<Terminalization>,
    pub separation_requirements: Vec<SeparationRequirement>,
    pub authority_requirements: Vec<AuthorityRequirement>,
    pub component_binding_requirements: Vec<ComponentBindingRequirement>,
}

/// Projects a candidate. Pure: the same candidate gives the same view.
pub fn view(candidate: &OrganizationCandidate) -> OrganizationView {
    let relationships = candidate.relationships();
    let of_kind = |kind: RelationshipKind| relationships.iter().filter(move |r| r.kind == kind);

    let obligations = of_kind(RelationshipKind::Serves)
        .filter_map(|r| {
            let id = Id::parse(r.to.as_deref()?).ok()?;
            Some(ViewObligation {
                id,
                served_by: r.from.clone(),
            })
        })
        .collect();

    let vantages = candidate
        .participants()
        .iter()
        .filter(|p| p.kind == crate::derive::ParticipantKind::Vantage)
        .map(|p| {
            let requirement = candidate
                .vantage_requirements()
                .iter()
                .find(|v| v.vantage == p.id);
            ViewVantage {
                id: p.id.clone(),
                effective: requirement.map(|v| v.effective.clone()),
                effective_constraints: requirement
                    .map(|v| v.effective_constraints.clone())
                    .unwrap_or_default(),
                conditions: requirement
                    .map(|v| v.derived_from.clone())
                    .unwrap_or_default(),
            }
        })
        .collect();

    let hosting = of_kind(RelationshipKind::HostsExecution)
        .filter_map(|r| {
            Some(Hosting {
                component: r.from.clone(),
                vantage: r.to.clone()?,
            })
        })
        .collect();

    let episode_access = of_kind(RelationshipKind::WritesEpisode)
        .filter_map(|writes| {
            let vantage = writes.to.clone()?;
            let readers = of_kind(RelationshipKind::ReadsEpisode)
                .filter(|reads| reads.to.as_deref() == Some(vantage.as_str()))
                .map(|reads| reads.from.clone())
                .collect();
            Some(EpisodeAccess {
                vantage,
                writer: writes.from.clone(),
                readers,
            })
        })
        .collect();

    let terminalization = of_kind(RelationshipKind::Terminalizes)
        .next()
        .map(|r| Terminalization {
            component: r.from.clone(),
        });

    OrganizationView {
        obligations,
        vantages,
        conditions: candidate.conditions().to_vec(),
        hosting,
        episode_access,
        terminalization,
        separation_requirements: candidate.separation_requirements().to_vec(),
        authority_requirements: candidate.authority_requirements().to_vec(),
        component_binding_requirements: candidate.component_binding_requirements().to_vec(),
    }
}
