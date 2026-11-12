//! `OrganizationView` (contract 8.1, amendment section 7): a pure projection of one candidate.
//! It permits nothing and has no status fields.

use std::collections::{BTreeMap, BTreeSet};

use crate::derive::{
    AuthorityRequirement, CandidateCondition, ComponentBindingRequirement, OrganizationCandidate,
    RelationshipKind, SeparationRequirement,
};
use crate::digest::Digest;
use crate::ids::Id;
use crate::schema::{Normative, ProblemScope, RealizationConstraint, ResultContract};

/// An obligation as the candidate carries it, and the vantage that serves it (from `serves`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ViewObligation {
    pub id: Id,
    /// The literal `candidate_review`.
    pub kind: &'static str,
    /// The literal `campaign_candidate`.
    pub subject: &'static str,
    pub result_contract: ResultContract,
    /// `scope.digest`.
    pub scope_digest: Digest,
    /// The reviewer vantage in this obligation's `serves` relationship. `None` only when the
    /// candidate has no such relationship, which `derive` never produces; the absence is shown,
    /// not filled and not dropped. Two such relationships, or one with no target or an unknown
    /// obligation, are refused by [`view`], never reduced to one.
    pub served_by: Option<String>,
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
    pub problem_scope: ProblemScope,
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

/// A candidate that [`view`] refuses to project. `derive` cannot produce any of these; each
/// names a relationship or requirement row that a projection would otherwise omit or reduce to
/// one of several, so the projection fails instead of showing a smaller organization than the
/// candidate states. The identifiers are those of the rows involved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ViewError {
    #[error("hosts_execution relationship from `{from}` names no target vantage")]
    HostsExecutionWithoutTarget { from: String },
    #[error("writes_episode relationship from `{from}` names no target vantage")]
    WritesEpisodeWithoutTarget { from: String },
    #[error("reads_episode relationship from `{from}` names no target vantage")]
    ReadsEpisodeWithoutTarget { from: String },
    #[error(
        "reads_episode relationship from `{from}` reads vantage `{vantage}`, which has no writes_episode relationship"
    )]
    ReadsUnwrittenEpisode { from: String, vantage: String },
    #[error(
        "vantage `{vantage}` has more than one writes_episode relationship (writers {writers:?})"
    )]
    DuplicateEpisodeWriter {
        vantage: String,
        writers: Vec<String>,
    },
    #[error("more than one terminalizes relationship (components {components:?})")]
    DuplicateTerminalization { components: Vec<String> },
    #[error("serves relationship from `{from}` names no obligation")]
    ServesWithoutTarget { from: String },
    #[error(
        "serves relationship from `{from}` names obligation `{obligation}`, which the candidate does not carry"
    )]
    ServesUnknownObligation { from: String, obligation: String },
    #[error(
        "obligation `{obligation}` has more than one serves relationship (vantages {vantages:?})"
    )]
    DuplicateServes {
        obligation: String,
        vantages: Vec<String>,
    },
    #[error("more than one vantage requirement for vantage `{vantage}`")]
    DuplicateVantageRequirement { vantage: String },
    #[error(
        "vantage requirement for `{vantage}`, which is not a vantage participant of the candidate"
    )]
    VantageRequirementWithoutVantage { vantage: String },
}

/// Projects a candidate. Pure: the same candidate gives the same view. A relationship or
/// requirement row that the projection would omit, or reduce to one of several, is a
/// [`ViewError`]; what stays absent is absent in the candidate and is shown as absent.
pub fn view(candidate: &OrganizationCandidate) -> Result<OrganizationView, ViewError> {
    let relationships = candidate.relationships();
    let of_kind = |kind: RelationshipKind| relationships.iter().filter(move |r| r.kind == kind);

    // `serves`: each row names one carried obligation, and each obligation has at most one.
    let mut serving: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for row in of_kind(RelationshipKind::Serves) {
        let obligation = row
            .to
            .as_deref()
            .ok_or_else(|| ViewError::ServesWithoutTarget {
                from: row.from.clone(),
            })?;
        if !candidate
            .obligations()
            .iter()
            .any(|o| o.id.as_str() == obligation)
        {
            return Err(ViewError::ServesUnknownObligation {
                from: row.from.clone(),
                obligation: obligation.to_owned(),
            });
        }
        serving.entry(obligation).or_default().push(&row.from);
    }
    for (obligation, vantages) in &serving {
        if vantages.len() > 1 {
            return Err(ViewError::DuplicateServes {
                obligation: (*obligation).to_owned(),
                vantages: vantages.iter().map(|v| (*v).to_owned()).collect(),
            });
        }
    }

    // Every obligation of the candidate is projected, whether or not its `serves` row is found.
    let obligations = candidate
        .obligations()
        .iter()
        .map(|obligation| ViewObligation {
            id: obligation.id.clone(),
            kind: obligation.kind(),
            subject: obligation.subject(),
            result_contract: obligation.result_contract.clone(),
            scope_digest: obligation.scope_digest,
            served_by: serving
                .get(obligation.id.as_str())
                .and_then(|vantages| vantages.first())
                .map(|vantage| (*vantage).to_owned()),
        })
        .collect();

    // Vantage requirements: one per vantage participant at most, none for a non-vantage. A
    // vantage with none (the builder vantage) keeps `effective: None` and empty lists.
    let mut required: BTreeSet<&str> = BTreeSet::new();
    for requirement in candidate.vantage_requirements() {
        if !candidate.participants().iter().any(|p| {
            p.kind == crate::derive::ParticipantKind::Vantage && p.id == requirement.vantage
        }) {
            return Err(ViewError::VantageRequirementWithoutVantage {
                vantage: requirement.vantage.clone(),
            });
        }
        if !required.insert(requirement.vantage.as_str()) {
            return Err(ViewError::DuplicateVantageRequirement {
                vantage: requirement.vantage.clone(),
            });
        }
    }
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

    let mut hosting = Vec::new();
    for row in of_kind(RelationshipKind::HostsExecution) {
        let vantage = row
            .to
            .clone()
            .ok_or_else(|| ViewError::HostsExecutionWithoutTarget {
                from: row.from.clone(),
            })?;
        hosting.push(Hosting {
            component: row.from.clone(),
            vantage,
        });
    }

    // Episode access: one writer per vantage, and every reader reads a written vantage.
    let mut writers: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut write_rows: Vec<(&str, &str)> = Vec::new();
    for row in of_kind(RelationshipKind::WritesEpisode) {
        let vantage = row
            .to
            .as_deref()
            .ok_or_else(|| ViewError::WritesEpisodeWithoutTarget {
                from: row.from.clone(),
            })?;
        writers.entry(vantage).or_default().push(&row.from);
        write_rows.push((vantage, &row.from));
    }
    for (vantage, written_by) in &writers {
        if written_by.len() > 1 {
            return Err(ViewError::DuplicateEpisodeWriter {
                vantage: (*vantage).to_owned(),
                writers: written_by.iter().map(|w| (*w).to_owned()).collect(),
            });
        }
    }
    for row in of_kind(RelationshipKind::ReadsEpisode) {
        let vantage = row
            .to
            .as_deref()
            .ok_or_else(|| ViewError::ReadsEpisodeWithoutTarget {
                from: row.from.clone(),
            })?;
        if !writers.contains_key(vantage) {
            return Err(ViewError::ReadsUnwrittenEpisode {
                from: row.from.clone(),
                vantage: vantage.to_owned(),
            });
        }
    }
    let episode_access = write_rows
        .iter()
        .map(|(vantage, writer)| EpisodeAccess {
            vantage: (*vantage).to_owned(),
            writer: (*writer).to_owned(),
            readers: of_kind(RelationshipKind::ReadsEpisode)
                .filter(|reads| reads.to.as_deref() == Some(*vantage))
                .map(|reads| reads.from.clone())
                .collect(),
        })
        .collect();

    let components: Vec<&str> = of_kind(RelationshipKind::Terminalizes)
        .map(|r| r.from.as_str())
        .collect();
    if components.len() > 1 {
        return Err(ViewError::DuplicateTerminalization {
            components: components.iter().map(|c| (*c).to_owned()).collect(),
        });
    }
    let terminalization = components.first().map(|component| Terminalization {
        component: (*component).to_owned(),
    });

    Ok(OrganizationView {
        problem_scope: candidate.problem_scope().clone(),
        obligations,
        vantages,
        conditions: candidate.conditions().to_vec(),
        hosting,
        episode_access,
        terminalization,
        separation_requirements: candidate.separation_requirements().to_vec(),
        authority_requirements: candidate.authority_requirements().to_vec(),
        component_binding_requirements: candidate.component_binding_requirements().to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derive::derive;
    use crate::schema::{OrganizationalRequirements, Profile};
    use serde_json::json;

    /// The same minimal requirements the derive tests use: one obligation, one condition.
    fn derived() -> OrganizationCandidate {
        let doc = json!({
            "problemScope": {"runId": "run-1", "planLineage": "plan-a", "sliceNumber": 1},
            "obligations": [{
                "id": "ob-a", "kind": "candidate_review", "subject": "campaign_candidate",
                "resultContract": {"id": "implementation-review", "version": 1},
                "scope": {"digest": "a".repeat(64)}
            }],
            "acceptanceConditions": [{
                "id": "cond-1", "obligation": "ob-a",
                "consumptionBoundary": "campaign_terminalization", "consumerClass": "campaign",
                "proposition": "p", "coveredState": "review-result",
                "normative": {
                    "continuity": "fresh_initial", "mutation": "none", "capabilities": [],
                    "integrity": "verified_artifacts",
                    "evidenceCustody": "execution_evidence_custody",
                    "observerClass": "independent_execution_evidence"
                },
                "acceptor": "owner-a",
                "unestablishedRoute": "hold-consumption-for-owner-disposition"
            }],
            "requiredAuthorities": [
                {"id": "auth-1", "principal": "owner-a", "permission": "accept_claim", "scope": {}}
            ],
            "organizationPolicy": {"mode": "fixed", "profile": "fixed-slice-review-v1"}
        });
        let requirements =
            OrganizationalRequirements::from_json_bytes(&serde_json::to_vec(&doc).unwrap())
                .unwrap();
        derive(&requirements, Profile::FixedSliceReviewV1)
            .candidates
            .remove(0)
    }

    fn first_of(candidate: &mut OrganizationCandidate, kind: RelationshipKind) -> usize {
        candidate
            .relationships()
            .iter()
            .position(|r| r.kind == kind)
            .unwrap_or_else(|| panic!("no {kind:?} row in the derived candidate"))
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_derived_candidate_projects() {
        let candidate = derived();
        let projected = view(&candidate).unwrap();
        assert_eq!(
            projected.obligations[0].served_by.as_deref(),
            Some("vantage:ob-a")
        );
        assert_eq!(projected.hosting.len(), 1);
        assert_eq!(projected.episode_access.len(), 1);
        assert!(projected.terminalization.is_some());
        assert_eq!(projected, view(&candidate).unwrap());
    }

    #[test]
    fn a_hosts_execution_row_with_no_target_is_refused() {
        let mut candidate = derived();
        let at = first_of(&mut candidate, RelationshipKind::HostsExecution);
        candidate.relationships_mut()[at].to = None;
        assert_eq!(
            view(&candidate),
            Err(ViewError::HostsExecutionWithoutTarget {
                from: "component:campaign".to_owned()
            })
        );
    }

    #[test]
    fn a_writes_episode_row_with_no_target_is_refused() {
        let mut candidate = derived();
        let at = first_of(&mut candidate, RelationshipKind::WritesEpisode);
        candidate.relationships_mut()[at].to = None;
        assert_eq!(
            view(&candidate),
            Err(ViewError::WritesEpisodeWithoutTarget {
                from: "component:episode_host".to_owned()
            })
        );
    }

    #[test]
    fn a_reads_episode_row_with_no_target_is_refused() {
        let mut candidate = derived();
        let at = first_of(&mut candidate, RelationshipKind::ReadsEpisode);
        candidate.relationships_mut()[at].to = None;
        assert_eq!(
            view(&candidate),
            Err(ViewError::ReadsEpisodeWithoutTarget {
                from: "component:episode_host".to_owned()
            })
        );
    }

    #[test]
    fn a_reads_episode_row_for_a_vantage_nobody_writes_is_refused() {
        let mut candidate = derived();
        let at = first_of(&mut candidate, RelationshipKind::ReadsEpisode);
        candidate.relationships_mut()[at].to = Some("vantage-builder".to_owned());
        assert_eq!(
            view(&candidate),
            Err(ViewError::ReadsUnwrittenEpisode {
                from: "component:episode_host".to_owned(),
                vantage: "vantage-builder".to_owned()
            })
        );
    }

    #[test]
    fn two_writes_episode_rows_for_one_vantage_are_refused() {
        let mut candidate = derived();
        let at = first_of(&mut candidate, RelationshipKind::WritesEpisode);
        let mut second = candidate.relationships()[at].clone();
        second.from = "component:campaign".to_owned();
        candidate.relationships_mut().push(second);
        assert_eq!(
            view(&candidate),
            Err(ViewError::DuplicateEpisodeWriter {
                vantage: "vantage:ob-a".to_owned(),
                writers: strings(&["component:episode_host", "component:campaign"])
            })
        );
    }

    #[test]
    fn two_terminalizes_rows_are_refused() {
        let mut candidate = derived();
        let at = first_of(&mut candidate, RelationshipKind::Terminalizes);
        let mut second = candidate.relationships()[at].clone();
        second.from = "component:episode_host".to_owned();
        candidate.relationships_mut().push(second);
        let Err(ViewError::DuplicateTerminalization { components }) = view(&candidate) else {
            panic!("expected DuplicateTerminalization");
        };
        let mut sorted = components.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            strings(&["component:campaign", "component:episode_host"])
        );
    }

    #[test]
    fn a_serves_row_with_no_target_is_refused() {
        let mut candidate = derived();
        let at = first_of(&mut candidate, RelationshipKind::Serves);
        candidate.relationships_mut()[at].to = None;
        assert_eq!(
            view(&candidate),
            Err(ViewError::ServesWithoutTarget {
                from: "vantage:ob-a".to_owned()
            })
        );
    }

    #[test]
    fn a_serves_row_for_an_obligation_the_candidate_lacks_is_refused() {
        let mut candidate = derived();
        let at = first_of(&mut candidate, RelationshipKind::Serves);
        candidate.relationships_mut()[at].to = Some("ob-ghost".to_owned());
        assert_eq!(
            view(&candidate),
            Err(ViewError::ServesUnknownObligation {
                from: "vantage:ob-a".to_owned(),
                obligation: "ob-ghost".to_owned()
            })
        );
    }

    #[test]
    fn two_serves_rows_for_one_obligation_are_refused() {
        let mut candidate = derived();
        let at = first_of(&mut candidate, RelationshipKind::Serves);
        let mut second = candidate.relationships()[at].clone();
        second.from = "vantage:other".to_owned();
        candidate.relationships_mut().push(second);
        assert_eq!(
            view(&candidate),
            Err(ViewError::DuplicateServes {
                obligation: "ob-a".to_owned(),
                vantages: strings(&["vantage:ob-a", "vantage:other"])
            })
        );
    }

    #[test]
    fn two_vantage_requirements_for_one_vantage_are_refused() {
        let mut candidate = derived();
        let second = candidate.vantage_requirements()[0].clone();
        candidate.vantage_requirements_mut().push(second);
        assert_eq!(
            view(&candidate),
            Err(ViewError::DuplicateVantageRequirement {
                vantage: "vantage:ob-a".to_owned()
            })
        );
    }

    #[test]
    fn a_vantage_requirement_for_no_vantage_participant_is_refused() {
        let mut candidate = derived();
        let mut ghost = candidate.vantage_requirements()[0].clone();
        ghost.vantage = "vantage:ghost".to_owned();
        candidate.vantage_requirements_mut().push(ghost);
        assert_eq!(
            view(&candidate),
            Err(ViewError::VantageRequirementWithoutVantage {
                vantage: "vantage:ghost".to_owned()
            })
        );
    }

    /// Absences the candidate states stay absent and visible: no `serves` row, no terminalizes
    /// row, and a vantage with no requirement (as the builder vantage has none).
    #[test]
    fn a_stated_absence_is_shown_not_refused() {
        let mut candidate = derived();
        candidate.relationships_mut().retain(|r| {
            r.kind != RelationshipKind::Serves && r.kind != RelationshipKind::Terminalizes
        });
        candidate.vantage_requirements_mut().clear();
        let projected = view(&candidate).unwrap();
        assert_eq!(projected.obligations[0].served_by, None);
        assert_eq!(projected.terminalization, None);
        let vantage = &projected.vantages[0];
        assert_eq!(vantage.id, "vantage:ob-a");
        assert!(vantage.effective.is_none());
        assert!(vantage.effective_constraints.is_empty());
        assert!(vantage.conditions.is_empty());
    }
}
