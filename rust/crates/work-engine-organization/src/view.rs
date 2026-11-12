//! `OrganizationView` (contract 8.1, amendment section 7): a pure projection of one candidate.
//! It permits nothing and has no status fields.

use std::collections::{BTreeMap, BTreeSet};

use crate::derive::{
    AuthorityRequirement, CandidateCondition, ComponentBindingRequirement, OrganizationCandidate,
    ParticipantKind, Relationship, RelationshipKind, SeparationRequirement,
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
    #[error(
        "{kind} relationship from `{from}`: the source is not a {required} participant of the candidate"
    )]
    RelationshipSourceInvalid {
        kind: &'static str,
        from: String,
        required: &'static str,
    },
    #[error("{kind} relationship from `{from}` names no target")]
    RelationshipWithoutTarget { kind: &'static str, from: String },
    #[error(
        "{kind} relationship from `{from}` names target `{to}`, which is not a {required} of the candidate"
    )]
    RelationshipTargetInvalid {
        kind: &'static str,
        from: String,
        to: String,
        required: &'static str,
    },
    #[error("{kind} relationship from `{from}` names target `{to}`, where none belongs")]
    RelationshipTargetUnexpected {
        kind: &'static str,
        from: String,
        to: String,
    },
    #[error("{kind} relationship from `{from}` names no condition")]
    RelationshipWithoutCondition { kind: &'static str, from: String },
    #[error("{kind} relationship from `{from}` names condition `{condition}`, where none belongs")]
    RelationshipConditionUnexpected {
        kind: &'static str,
        from: String,
        condition: String,
    },
    #[error(
        "{kind} relationship from `{from}` names condition `{condition}`, which the candidate does not carry"
    )]
    RelationshipUnknownCondition {
        kind: &'static str,
        from: String,
        condition: String,
    },
    #[error(
        "{kind} relationship from `{from}` for condition `{condition}` targets `{to}`, but the vantage serving the condition's obligation is `{expected}`"
    )]
    RelationshipWrongVantage {
        kind: &'static str,
        from: String,
        to: String,
        condition: String,
        expected: String,
    },
    #[error(
        "consumes_at relationship from `{from}` for condition `{condition}`, but the condition's consumer is `{expected}`"
    )]
    ConsumesAtWrongConsumer {
        from: String,
        condition: String,
        expected: String,
    },
    #[error(
        "accepts_claim relationship from `{from}` for condition `{condition}`, but the condition's acceptor is `{expected}`"
    )]
    AcceptsClaimWrongAcceptor {
        from: String,
        condition: String,
        expected: String,
    },
    #[error("condition `{condition}` has more than one {kind} relationship (sources {sources:?})")]
    DuplicateConditionRelationship {
        kind: &'static str,
        condition: String,
        sources: Vec<String>,
    },
    #[error("duplicate {kind} relationship from `{from}` (target {to:?}, condition {condition:?})")]
    DuplicateRelationship {
        kind: &'static str,
        from: String,
        to: Option<String>,
        condition: Option<String>,
    },
}

/// Projects a candidate. Pure: the same candidate gives the same view. A relationship or
/// requirement row that the projection would omit, or reduce to one of several, is a
/// [`ViewError`]; what stays absent is absent in the candidate and is shown as absent. Every
/// relationship row is also validated whether or not the view presents it (`consumes_at` and
/// `accepts_claim` are not projected): see [`validate_relationships`].
pub fn view(candidate: &OrganizationCandidate) -> Result<OrganizationView, ViewError> {
    let relationships = candidate.relationships();
    let of_kind = |kind: RelationshipKind| relationships.iter().filter(move |r| r.kind == kind);

    // `serves`: each row names one carried obligation, and each obligation has at most one.
    let mut serving: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for row in of_kind(RelationshipKind::Serves) {
        let Some(obligation) = row.to.as_deref() else {
            return Err(ViewError::ServesWithoutTarget {
                from: row.from.clone(),
            });
        };
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
        let Some(vantage) = row.to.clone() else {
            return Err(ViewError::HostsExecutionWithoutTarget {
                from: row.from.clone(),
            });
        };
        hosting.push(Hosting {
            component: row.from.clone(),
            vantage,
        });
    }

    // Episode access: one writer per vantage, and every reader reads a written vantage.
    let mut writers: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut write_rows: Vec<(&str, &str)> = Vec::new();
    for row in of_kind(RelationshipKind::WritesEpisode) {
        let Some(vantage) = row.to.as_deref() else {
            return Err(ViewError::WritesEpisodeWithoutTarget {
                from: row.from.clone(),
            });
        };
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
        let Some(vantage) = row.to.as_deref() else {
            return Err(ViewError::ReadsEpisodeWithoutTarget {
                from: row.from.clone(),
            });
        };
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

    validate_relationships(candidate)?;

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

/// The participant kinds a relationship kind's `from` may name. `consumes_at` is from the
/// condition's consumer, which is the campaign component or the builder vantage.
fn source_kinds(kind: RelationshipKind) -> &'static [ParticipantKind] {
    match kind {
        RelationshipKind::Serves => &[ParticipantKind::Vantage],
        RelationshipKind::ConsumesAt => &[ParticipantKind::Vantage, ParticipantKind::Component],
        RelationshipKind::AcceptsClaim => &[ParticipantKind::Principal],
        RelationshipKind::HostsExecution
        | RelationshipKind::WritesEpisode
        | RelationshipKind::ReadsEpisode
        | RelationshipKind::Terminalizes => &[ParticipantKind::Component],
    }
}

fn kinds_name(kinds: &[ParticipantKind]) -> &'static str {
    match kinds {
        [ParticipantKind::Vantage] => "vantage",
        [ParticipantKind::Component] => "component",
        [ParticipantKind::Principal] => "principal",
        _ => "vantage or component",
    }
}

/// Validates every relationship row of the candidate, of every kind, against what `derive`
/// emits (amendment 5.1-A1 section 3), whether or not the view presents the kind. Rows are
/// checked in candidate order; the first failure is returned. Required rows that are absent are
/// not checked here: that is a separate question.
fn validate_relationships(candidate: &OrganizationCandidate) -> Result<(), ViewError> {
    let participants: BTreeMap<&str, ParticipantKind> = candidate
        .participants()
        .iter()
        .map(|p| (p.id.as_str(), p.kind))
        .collect();
    let conditions: BTreeMap<&str, &CandidateCondition> = candidate
        .conditions()
        .iter()
        .map(|c| (c.id.as_str(), c))
        .collect();
    let mut per_condition: BTreeMap<(RelationshipKind, &str), &str> = BTreeMap::new();
    let mut seen: BTreeSet<(RelationshipKind, &str, Option<&str>, Option<&str>)> = BTreeSet::new();

    for row in candidate.relationships() {
        let kind = row.kind.as_str();
        let from = row.from.as_str();

        // 1. `from` is a participant of the required kind.
        let required = source_kinds(row.kind);
        if !participants.get(from).is_some_and(|k| required.contains(k)) {
            return Err(ViewError::RelationshipSourceInvalid {
                kind,
                from: row.from.clone(),
                required: kinds_name(required),
            });
        }

        // 2. `to` is present exactly when the kind has a target, and names the right entity.
        match (row.kind, row.to.as_deref()) {
            (RelationshipKind::Terminalizes, Some(to)) => {
                return Err(ViewError::RelationshipTargetUnexpected {
                    kind,
                    from: row.from.clone(),
                    to: to.to_owned(),
                });
            }
            (RelationshipKind::Terminalizes, None) => {}
            (_, None) => {
                return Err(ViewError::RelationshipWithoutTarget {
                    kind,
                    from: row.from.clone(),
                });
            }
            // The `serves` target is an obligation, checked against the candidate's obligations
            // before the projection; the other targets are vantage participants.
            (RelationshipKind::Serves, Some(_)) => {}
            (_, Some(to)) => {
                if participants.get(to) != Some(&ParticipantKind::Vantage) {
                    return Err(ViewError::RelationshipTargetInvalid {
                        kind,
                        from: row.from.clone(),
                        to: to.to_owned(),
                        required: "vantage participant",
                    });
                }
            }
        }

        // 3 and 4. `condition` is present exactly for `consumes_at` and `accepts_claim`, names a
        // carried condition, and the row agrees with that condition.
        match (row.kind, row.condition.as_ref()) {
            (RelationshipKind::ConsumesAt | RelationshipKind::AcceptsClaim, None) => {
                return Err(ViewError::RelationshipWithoutCondition {
                    kind,
                    from: row.from.clone(),
                });
            }
            (RelationshipKind::ConsumesAt | RelationshipKind::AcceptsClaim, Some(id)) => {
                check_condition_row(row, id.as_str(), &conditions)?;
                if let Some(first) = per_condition.insert((row.kind, id.as_str()), from) {
                    return Err(ViewError::DuplicateConditionRelationship {
                        kind,
                        condition: id.to_string(),
                        sources: vec![first.to_owned(), row.from.clone()],
                    });
                }
            }
            (_, Some(id)) => {
                return Err(ViewError::RelationshipConditionUnexpected {
                    kind,
                    from: row.from.clone(),
                    condition: id.to_string(),
                });
            }
            (_, None) => {}
        }

        // 5. No duplicate row under the set key.
        let key = (
            row.kind,
            from,
            row.to.as_deref(),
            row.condition.as_ref().map(Id::as_str),
        );
        if !seen.insert(key) {
            return Err(ViewError::DuplicateRelationship {
                kind,
                from: row.from.clone(),
                to: row.to.clone(),
                condition: row.condition.as_ref().map(|c| c.to_string()),
            });
        }
    }
    Ok(())
}

/// A `consumes_at` or `accepts_claim` row names a carried condition, targets the vantage that
/// serves the condition's obligation, and (for `consumes_at`) comes from the condition's
/// consumer or (for `accepts_claim`) from the condition's acceptor.
fn check_condition_row(
    row: &Relationship,
    condition_id: &str,
    conditions: &BTreeMap<&str, &CandidateCondition>,
) -> Result<(), ViewError> {
    let kind = row.kind.as_str();
    let Some(condition) = conditions.get(condition_id) else {
        return Err(ViewError::RelationshipUnknownCondition {
            kind,
            from: row.from.clone(),
            condition: condition_id.to_owned(),
        });
    };
    // A target is always present here: step 2 refused a missing one.
    if let Some(to) = row.to.as_deref()
        && to != condition.vantage
    {
        return Err(ViewError::RelationshipWrongVantage {
            kind,
            from: row.from.clone(),
            to: to.to_owned(),
            condition: condition_id.to_owned(),
            expected: condition.vantage.clone(),
        });
    }
    if row.kind == RelationshipKind::ConsumesAt {
        if row.from != condition.consumer {
            return Err(ViewError::ConsumesAtWrongConsumer {
                from: row.from.clone(),
                condition: condition_id.to_owned(),
                expected: condition.consumer.clone(),
            });
        }
    } else {
        let expected = format!("principal:{}", condition.acceptor);
        if row.from != expected {
            return Err(ViewError::AcceptsClaimWrongAcceptor {
                from: row.from.clone(),
                condition: condition_id.to_owned(),
                expected,
            });
        }
    }
    Ok(())
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

    // --- Every relationship row is validated, whether or not the view presents its kind. ---

    /// Two obligations with one condition each (`cond-1` on `ob-a`, consumed by the campaign;
    /// `cond-2` on `ob-b`, consumed by the builder), accepted by `owner-a` and `owner-b`.
    fn derived_two() -> OrganizationCandidate {
        let condition = |id: &str, obligation: &str, class: &str, acceptor: &str| {
            json!({
                "id": id, "obligation": obligation,
                "consumptionBoundary": "campaign_terminalization", "consumerClass": class,
                "proposition": "p", "coveredState": "review-result",
                "normative": {
                    "continuity": "fresh_initial", "mutation": "none", "capabilities": [],
                    "integrity": "verified_artifacts",
                    "evidenceCustody": "execution_evidence_custody",
                    "observerClass": "independent_execution_evidence"
                },
                "acceptor": acceptor,
                "unestablishedRoute": "hold-consumption-for-owner-disposition"
            })
        };
        let obligation = |id: &str, digest: &str| {
            json!({
                "id": id, "kind": "candidate_review", "subject": "campaign_candidate",
                "resultContract": {"id": "implementation-review", "version": 1},
                "scope": {"digest": digest.repeat(64)}
            })
        };
        let doc = json!({
            "problemScope": {"runId": "run-1", "planLineage": "plan-a", "sliceNumber": 1},
            "obligations": [obligation("ob-a", "a"), obligation("ob-b", "b")],
            "acceptanceConditions": [
                condition("cond-1", "ob-a", "campaign", "owner-a"),
                condition("cond-2", "ob-b", "builder", "owner-b"),
            ],
            "requiredAuthorities": [
                {"id": "auth-1", "principal": "owner-a", "permission": "accept_claim", "scope": {}},
                {"id": "auth-2", "principal": "owner-b", "permission": "accept_claim", "scope": {}}
            ],
            "organizationPolicy": {"mode": "fixed", "profile": "fixed-slice-review-v1"}
        });
        let requirements =
            OrganizationalRequirements::from_json_bytes(&serde_json::to_vec(&doc).unwrap())
                .unwrap();
        let mut result = derive(&requirements, Profile::FixedSliceReviewV1);
        assert!(result.unresolved.is_empty(), "{:?}", result.unresolved);
        result.candidates.remove(0)
    }

    fn row_for(
        candidate: &OrganizationCandidate,
        kind: RelationshipKind,
        condition: Option<&str>,
    ) -> usize {
        candidate
            .relationships()
            .iter()
            .position(|r| r.kind == kind && r.condition.as_ref().map(Id::as_str) == condition)
            .unwrap_or_else(|| panic!("no {kind:?} row for {condition:?}"))
    }

    fn edit(
        candidate: &mut OrganizationCandidate,
        kind: RelationshipKind,
        condition: Option<&str>,
        change: impl FnOnce(&mut Relationship),
    ) {
        let at = row_for(candidate, kind, condition);
        change(&mut candidate.relationships_mut()[at]);
    }

    fn some(text: &str) -> Option<String> {
        Some(text.to_owned())
    }

    fn id(text: &str) -> Option<Id> {
        Some(Id::parse(text).unwrap())
    }

    #[test]
    fn a_derived_two_obligation_candidate_with_a_builder_consumer_projects() {
        let candidate = derived_two();
        let kinds: Vec<_> = candidate.relationships().iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RelationshipKind::ConsumesAt));
        let builder_consumes = candidate
            .relationships()
            .iter()
            .any(|r| r.kind == RelationshipKind::ConsumesAt && r.from == "vantage-builder");
        assert!(builder_consumes);
        assert!(view(&candidate).is_ok());
    }

    // The reviewer's four probes.

    #[test]
    fn probe_an_extra_consumes_at_row_with_ghost_references_is_refused() {
        let mut candidate = derived();
        candidate.relationships_mut().push(Relationship {
            kind: RelationshipKind::ConsumesAt,
            from: "component:ghost".to_owned(),
            to: some("vantage:ghost"),
            condition: id("ghost"),
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipSourceInvalid {
                kind: "consumes_at",
                from: "component:ghost".to_owned(),
                required: "vantage or component"
            })
        );
    }

    #[test]
    fn probe_a_consumes_at_row_with_no_target_and_no_condition_is_refused() {
        let mut candidate = derived();
        candidate.relationships_mut().push(Relationship {
            kind: RelationshipKind::ConsumesAt,
            from: "component:campaign".to_owned(),
            to: None,
            condition: None,
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipWithoutTarget {
                kind: "consumes_at",
                from: "component:campaign".to_owned()
            })
        );
    }

    #[test]
    fn probe_an_extra_accepts_claim_row_with_ghost_references_is_refused() {
        let mut candidate = derived();
        candidate.relationships_mut().push(Relationship {
            kind: RelationshipKind::AcceptsClaim,
            from: "principal:ghost".to_owned(),
            to: some("vantage:ghost"),
            condition: id("ghost"),
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipSourceInvalid {
                kind: "accepts_claim",
                from: "principal:ghost".to_owned(),
                required: "principal"
            })
        );
    }

    #[test]
    fn probe_an_accepts_claim_row_with_no_target_and_no_condition_is_refused() {
        let mut candidate = derived();
        candidate.relationships_mut().push(Relationship {
            kind: RelationshipKind::AcceptsClaim,
            from: "principal:owner-a".to_owned(),
            to: None,
            condition: None,
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipWithoutTarget {
                kind: "accepts_claim",
                from: "principal:owner-a".to_owned()
            })
        );
    }

    // `consumes_at`.

    #[test]
    fn consumes_at_from_a_principal_is_refused() {
        let mut candidate = derived();
        edit(
            &mut candidate,
            RelationshipKind::ConsumesAt,
            Some("cond-1"),
            |r| {
                r.from = "principal:owner-a".to_owned();
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipSourceInvalid {
                kind: "consumes_at",
                from: "principal:owner-a".to_owned(),
                required: "vantage or component"
            })
        );
    }

    #[test]
    fn consumes_at_to_a_non_vantage_is_refused() {
        let mut candidate = derived();
        edit(
            &mut candidate,
            RelationshipKind::ConsumesAt,
            Some("cond-1"),
            |r| {
                r.to = some("component:episode_host");
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipTargetInvalid {
                kind: "consumes_at",
                from: "component:campaign".to_owned(),
                to: "component:episode_host".to_owned(),
                required: "vantage participant"
            })
        );
    }

    #[test]
    fn consumes_at_with_no_condition_is_refused() {
        let mut candidate = derived();
        edit(
            &mut candidate,
            RelationshipKind::ConsumesAt,
            Some("cond-1"),
            |r| {
                r.condition = None;
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipWithoutCondition {
                kind: "consumes_at",
                from: "component:campaign".to_owned()
            })
        );
    }

    #[test]
    fn consumes_at_for_a_condition_the_candidate_lacks_is_refused() {
        let mut candidate = derived();
        edit(
            &mut candidate,
            RelationshipKind::ConsumesAt,
            Some("cond-1"),
            |r| {
                r.condition = id("cond-ghost");
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipUnknownCondition {
                kind: "consumes_at",
                from: "component:campaign".to_owned(),
                condition: "cond-ghost".to_owned()
            })
        );
    }

    #[test]
    fn consumes_at_targeting_another_conditions_vantage_is_refused() {
        let mut candidate = derived_two();
        edit(
            &mut candidate,
            RelationshipKind::ConsumesAt,
            Some("cond-1"),
            |r| {
                r.to = some("vantage:ob-b");
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipWrongVantage {
                kind: "consumes_at",
                from: "component:campaign".to_owned(),
                to: "vantage:ob-b".to_owned(),
                condition: "cond-1".to_owned(),
                expected: "vantage:ob-a".to_owned()
            })
        );
    }

    #[test]
    fn consumes_at_from_a_participant_that_is_not_the_conditions_consumer_is_refused() {
        let mut candidate = derived_two();
        edit(
            &mut candidate,
            RelationshipKind::ConsumesAt,
            Some("cond-2"),
            |r| {
                r.from = "component:campaign".to_owned();
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::ConsumesAtWrongConsumer {
                from: "component:campaign".to_owned(),
                condition: "cond-2".to_owned(),
                expected: "vantage-builder".to_owned()
            })
        );
    }

    #[test]
    fn two_consumes_at_rows_for_one_condition_are_refused() {
        let mut candidate = derived();
        let at = row_for(&candidate, RelationshipKind::ConsumesAt, Some("cond-1"));
        let second = candidate.relationships()[at].clone();
        candidate.relationships_mut().push(second);
        assert_eq!(
            view(&candidate),
            Err(ViewError::DuplicateConditionRelationship {
                kind: "consumes_at",
                condition: "cond-1".to_owned(),
                sources: strings(&["component:campaign", "component:campaign"])
            })
        );
    }

    // `accepts_claim`.

    #[test]
    fn accepts_claim_from_a_component_is_refused() {
        let mut candidate = derived();
        edit(
            &mut candidate,
            RelationshipKind::AcceptsClaim,
            Some("cond-1"),
            |r| {
                r.from = "component:campaign".to_owned();
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipSourceInvalid {
                kind: "accepts_claim",
                from: "component:campaign".to_owned(),
                required: "principal"
            })
        );
    }

    #[test]
    fn accepts_claim_to_a_non_vantage_is_refused() {
        let mut candidate = derived();
        edit(
            &mut candidate,
            RelationshipKind::AcceptsClaim,
            Some("cond-1"),
            |r| {
                r.to = some("principal:owner-a");
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipTargetInvalid {
                kind: "accepts_claim",
                from: "principal:owner-a".to_owned(),
                to: "principal:owner-a".to_owned(),
                required: "vantage participant"
            })
        );
    }

    #[test]
    fn accepts_claim_with_no_condition_is_refused() {
        let mut candidate = derived();
        edit(
            &mut candidate,
            RelationshipKind::AcceptsClaim,
            Some("cond-1"),
            |r| {
                r.condition = None;
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipWithoutCondition {
                kind: "accepts_claim",
                from: "principal:owner-a".to_owned()
            })
        );
    }

    #[test]
    fn accepts_claim_for_a_condition_the_candidate_lacks_is_refused() {
        let mut candidate = derived();
        edit(
            &mut candidate,
            RelationshipKind::AcceptsClaim,
            Some("cond-1"),
            |r| {
                r.condition = id("cond-ghost");
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipUnknownCondition {
                kind: "accepts_claim",
                from: "principal:owner-a".to_owned(),
                condition: "cond-ghost".to_owned()
            })
        );
    }

    #[test]
    fn accepts_claim_targeting_another_conditions_vantage_is_refused() {
        let mut candidate = derived_two();
        edit(
            &mut candidate,
            RelationshipKind::AcceptsClaim,
            Some("cond-1"),
            |r| {
                r.to = some("vantage:ob-b");
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipWrongVantage {
                kind: "accepts_claim",
                from: "principal:owner-a".to_owned(),
                to: "vantage:ob-b".to_owned(),
                condition: "cond-1".to_owned(),
                expected: "vantage:ob-a".to_owned()
            })
        );
    }

    #[test]
    fn accepts_claim_from_a_principal_that_is_not_the_conditions_acceptor_is_refused() {
        let mut candidate = derived_two();
        edit(
            &mut candidate,
            RelationshipKind::AcceptsClaim,
            Some("cond-1"),
            |r| {
                r.from = "principal:owner-b".to_owned();
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::AcceptsClaimWrongAcceptor {
                from: "principal:owner-b".to_owned(),
                condition: "cond-1".to_owned(),
                expected: "principal:owner-a".to_owned()
            })
        );
    }

    #[test]
    fn two_accepts_claim_rows_for_one_condition_are_refused() {
        let mut candidate = derived();
        let at = row_for(&candidate, RelationshipKind::AcceptsClaim, Some("cond-1"));
        let second = candidate.relationships()[at].clone();
        candidate.relationships_mut().push(second);
        assert_eq!(
            view(&candidate),
            Err(ViewError::DuplicateConditionRelationship {
                kind: "accepts_claim",
                condition: "cond-1".to_owned(),
                sources: strings(&["principal:owner-a", "principal:owner-a"])
            })
        );
    }

    // `serves`, `hosts_execution`, `writes_episode`, `reads_episode`, `terminalizes`: source,
    // target and condition on every row, and exact duplicates.

    #[test]
    fn serves_from_a_non_vantage_is_refused() {
        let mut candidate = derived();
        edit(&mut candidate, RelationshipKind::Serves, None, |r| {
            r.from = "component:campaign".to_owned();
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipSourceInvalid {
                kind: "serves",
                from: "component:campaign".to_owned(),
                required: "vantage"
            })
        );
    }

    #[test]
    fn serves_with_a_condition_is_refused() {
        let mut candidate = derived();
        edit(&mut candidate, RelationshipKind::Serves, None, |r| {
            r.condition = id("cond-1");
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipConditionUnexpected {
                kind: "serves",
                from: "vantage:ob-a".to_owned(),
                condition: "cond-1".to_owned()
            })
        );
    }

    #[test]
    fn hosts_execution_from_a_non_component_is_refused() {
        let mut candidate = derived();
        edit(
            &mut candidate,
            RelationshipKind::HostsExecution,
            None,
            |r| {
                r.from = "principal:owner-a".to_owned();
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipSourceInvalid {
                kind: "hosts_execution",
                from: "principal:owner-a".to_owned(),
                required: "component"
            })
        );
    }

    #[test]
    fn hosts_execution_to_a_non_vantage_is_refused() {
        for target in ["component:campaign", "vantage:ghost"] {
            let mut candidate = derived();
            edit(
                &mut candidate,
                RelationshipKind::HostsExecution,
                None,
                |r| {
                    r.to = some(target);
                },
            );
            assert_eq!(
                view(&candidate),
                Err(ViewError::RelationshipTargetInvalid {
                    kind: "hosts_execution",
                    from: "component:campaign".to_owned(),
                    to: target.to_owned(),
                    required: "vantage participant"
                })
            );
        }
    }

    #[test]
    fn hosts_execution_with_a_condition_is_refused() {
        let mut candidate = derived();
        edit(
            &mut candidate,
            RelationshipKind::HostsExecution,
            None,
            |r| {
                r.condition = id("cond-1");
            },
        );
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipConditionUnexpected {
                kind: "hosts_execution",
                from: "component:campaign".to_owned(),
                condition: "cond-1".to_owned()
            })
        );
    }

    #[test]
    fn a_duplicate_hosts_execution_row_is_refused() {
        let mut candidate = derived();
        let at = row_for(&candidate, RelationshipKind::HostsExecution, None);
        let second = candidate.relationships()[at].clone();
        candidate.relationships_mut().push(second);
        assert_eq!(
            view(&candidate),
            Err(ViewError::DuplicateRelationship {
                kind: "hosts_execution",
                from: "component:campaign".to_owned(),
                to: some("vantage:ob-a"),
                condition: None
            })
        );
    }

    #[test]
    fn writes_episode_from_a_non_component_is_refused() {
        let mut candidate = derived();
        edit(&mut candidate, RelationshipKind::WritesEpisode, None, |r| {
            r.from = "vantage:ob-a".to_owned();
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipSourceInvalid {
                kind: "writes_episode",
                from: "vantage:ob-a".to_owned(),
                required: "component"
            })
        );
    }

    #[test]
    fn writes_episode_to_a_non_vantage_is_refused() {
        // The reader row must name a written vantage to pass the episode-access checks, so both
        // rows move to the same non-vantage; the reader row comes first in candidate order.
        let mut candidate = derived();
        for kind in [
            RelationshipKind::WritesEpisode,
            RelationshipKind::ReadsEpisode,
        ] {
            edit(&mut candidate, kind, None, |r| r.to = some("vantage:ghost"));
        }
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipTargetInvalid {
                kind: "reads_episode",
                from: "component:episode_host".to_owned(),
                to: "vantage:ghost".to_owned(),
                required: "vantage participant"
            })
        );
        let mut candidate = derived();
        edit(&mut candidate, RelationshipKind::WritesEpisode, None, |r| {
            r.to = some("vantage:ghost");
        });
        edit(&mut candidate, RelationshipKind::ReadsEpisode, None, |r| {
            r.to = some("vantage:ghost");
        });
        candidate
            .relationships_mut()
            .retain(|r| r.kind != RelationshipKind::ReadsEpisode);
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipTargetInvalid {
                kind: "writes_episode",
                from: "component:episode_host".to_owned(),
                to: "vantage:ghost".to_owned(),
                required: "vantage participant"
            })
        );
    }

    #[test]
    fn writes_episode_with_a_condition_is_refused() {
        let mut candidate = derived();
        edit(&mut candidate, RelationshipKind::WritesEpisode, None, |r| {
            r.condition = id("cond-1");
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipConditionUnexpected {
                kind: "writes_episode",
                from: "component:episode_host".to_owned(),
                condition: "cond-1".to_owned()
            })
        );
    }

    #[test]
    fn reads_episode_from_a_non_component_is_refused() {
        let mut candidate = derived();
        edit(&mut candidate, RelationshipKind::ReadsEpisode, None, |r| {
            r.from = "principal:owner-a".to_owned();
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipSourceInvalid {
                kind: "reads_episode",
                from: "principal:owner-a".to_owned(),
                required: "component"
            })
        );
    }

    #[test]
    fn reads_episode_with_a_condition_is_refused() {
        let mut candidate = derived();
        edit(&mut candidate, RelationshipKind::ReadsEpisode, None, |r| {
            r.condition = id("cond-1");
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipConditionUnexpected {
                kind: "reads_episode",
                from: "component:episode_host".to_owned(),
                condition: "cond-1".to_owned()
            })
        );
    }

    #[test]
    fn a_duplicate_reads_episode_row_is_refused() {
        let mut candidate = derived();
        let at = row_for(&candidate, RelationshipKind::ReadsEpisode, None);
        let second = candidate.relationships()[at].clone();
        candidate.relationships_mut().push(second);
        assert_eq!(
            view(&candidate),
            Err(ViewError::DuplicateRelationship {
                kind: "reads_episode",
                from: "component:episode_host".to_owned(),
                to: some("vantage:ob-a"),
                condition: None
            })
        );
    }

    #[test]
    fn terminalizes_from_a_non_component_is_refused() {
        let mut candidate = derived();
        edit(&mut candidate, RelationshipKind::Terminalizes, None, |r| {
            r.from = "vantage:ob-a".to_owned();
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipSourceInvalid {
                kind: "terminalizes",
                from: "vantage:ob-a".to_owned(),
                required: "component"
            })
        );
    }

    #[test]
    fn terminalizes_with_a_target_is_refused() {
        let mut candidate = derived();
        edit(&mut candidate, RelationshipKind::Terminalizes, None, |r| {
            r.to = some("vantage:ob-a");
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipTargetUnexpected {
                kind: "terminalizes",
                from: "component:campaign".to_owned(),
                to: "vantage:ob-a".to_owned()
            })
        );
    }

    #[test]
    fn terminalizes_with_a_condition_is_refused() {
        let mut candidate = derived();
        edit(&mut candidate, RelationshipKind::Terminalizes, None, |r| {
            r.condition = id("cond-1");
        });
        assert_eq!(
            view(&candidate),
            Err(ViewError::RelationshipConditionUnexpected {
                kind: "terminalizes",
                from: "component:campaign".to_owned(),
                condition: "cond-1".to_owned()
            })
        );
    }

    /// An absent required row is a separate question (not asked): no `consumes_at` row for a
    /// condition, and no `terminalizes` row, still project.
    #[test]
    fn an_absent_condition_row_is_not_refused_here() {
        let mut candidate = derived();
        candidate.relationships_mut().retain(|r| {
            r.kind != RelationshipKind::ConsumesAt && r.kind != RelationshipKind::AcceptsClaim
        });
        assert!(view(&candidate).is_ok());
    }
}
