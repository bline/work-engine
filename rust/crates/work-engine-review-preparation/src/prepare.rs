//! Production preparation: an inert projection of one canonical candidate, with blockers.
//!
//! `prepare` derives the view and the enforceability report from the candidate itself and
//! projects the selected obligation. It carries meaning owned elsewhere (result contract,
//! conditions, requirements, constraints, provenance) and states which mandatory mappings are
//! not established. The result is a plain value: it holds no handle, grants nothing, allocates
//! no session and is not an execution or admission token. The unconditional blockers are built
//! from the candidate alone, so no request field, claim or fixture declaration can remove them.

use review_episode_core::codec::JsValue;
use review_episode_core::implementation_review_v1::validate_subject;
use serde::{Deserialize, Serialize};
use work_engine_organization::{
    EnforceStatus, EvidenceClass, Normative, OrganizationCandidate, RealizationConstraint,
};

use crate::realization::{self, Disposition, RealizationClaim, RealizationEvaluation};
use crate::select::{PreparationRefusal, Selection};

/// The exact review subject, as the supplier states it. Validated by the episode owner's
/// `validate_subject`; its relation to the obligation's `scope.digest` is not established here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    pub commit: String,
    pub tree: String,
    #[serde(rename = "patchIdentity")]
    pub patch_identity: String,
}

impl Subject {
    pub fn to_js_value(&self) -> JsValue {
        JsValue::object([
            ("commit", JsValue::text(&self.commit)),
            ("tree", JsValue::text(&self.tree)),
            ("patchIdentity", JsValue::text(&self.patch_identity)),
        ])
    }
}

/// What a caller supplies. The subject and the descriptor are claims; neither is evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationRequest {
    pub obligation_id: String,
    pub subject: Subject,
    pub realization: Option<RealizationClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormativeContext {
    pub continuity: String,
    pub mutation: String,
    pub independence: Option<Vec<String>>,
    pub capabilities: Vec<String>,
    pub integrity: String,
    pub evidence_custody: String,
    pub observer_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintContext {
    pub dimension: String,
    pub allowed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateIdentity {
    pub requirements_digest: String,
    pub candidate_content_digest: String,
    pub profile: String,
    pub run_id: String,
    pub plan_lineage: String,
    pub slice_number: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObligationContext {
    pub id: String,
    pub kind: String,
    pub subject_class: String,
    pub result_contract_id: String,
    pub result_contract_version: u64,
    /// `scope.digest`: carried as identity only. Its relation to `subject` is a blocker.
    pub scope_digest: String,
    pub subject: Subject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VantageContext {
    pub id: String,
    pub effective: Option<NormativeContext>,
    pub effective_constraints: Vec<ConstraintContext>,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionContext {
    pub id: String,
    pub vantage: String,
    pub consumer: String,
    pub boundary: String,
    pub proposition: String,
    pub covered_state: String,
    pub acceptor: String,
    pub unestablished_route: String,
    pub normative: NormativeContext,
    pub constraints: Vec<ConstraintContext>,
}

/// The model-facing context: derived data, not prose and not a policy. Rendering it is
/// serialization (`to_json`); it adds no meaning and no instruction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelContext {
    pub identity: CandidateIdentity,
    pub obligation: ObligationContext,
    pub vantage: VantageContext,
    pub conditions: Vec<ConditionContext>,
}

impl ModelContext {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("context serializes")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowStatus {
    Supported,
    SupportedWithRealizationMapping,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowEvidenceClass {
    Constraint,
    Recording,
    ConstraintAndRecording,
    Observation,
    NotApplicable,
}

/// One row of the owner's enforceability report, verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnforcementRow {
    pub property: String,
    pub scope: String,
    pub requirement: String,
    pub status: RowStatus,
    pub evidence_class: RowEvidenceClass,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeparationContext {
    pub condition: String,
    pub acceptor: String,
    pub vantages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityContext {
    pub id: String,
    pub principal: String,
    pub permission: String,
    pub boundaries: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentBindingContext {
    pub component: String,
    pub permissions: Vec<String>,
}

/// What the host would have to enforce or establish, constructed apart from the model context.
/// `authority` and `component_binding` are candidate-wide: the owner provides no reference that
/// ties either to one obligation, so they are not narrowed by this crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostEnforcement {
    pub rows: Vec<EnforcementRow>,
    pub separation: Vec<SeparationContext>,
    pub authority: Vec<AuthorityContext>,
    pub component_binding: Vec<ComponentBindingContext>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEndpoint {
    pub kind: String,
    pub id: String,
    pub field: String,
}

/// A candidate member and the input item it originates in, as the owner traced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub target: TraceEndpoint,
    pub source: TraceEndpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerClass {
    /// A supplied claim states a value that a requirement does not allow.
    KnownIncompatibility,
    /// A mandatory mapping or evidence has no establishing owner or mechanism.
    Unestablished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blocker {
    pub id: String,
    pub class: BlockerClass,
    pub scope: String,
    pub requirement: String,
    pub basis: String,
}

/// The standing of the whole value. It has one variant: preparation is inert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparationStatus {
    /// Not an execution grant, an admission or a session. Allocates and calls nothing.
    InertNotAnAdmissionToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewPreparation {
    pub status: PreparationStatus,
    pub model_context: ModelContext,
    pub host_enforcement: HostEnforcement,
    pub provenance: Vec<Provenance>,
    pub realization_evaluation: Option<RealizationEvaluation>,
    pub blockers: Vec<Blocker>,
}

fn normative_context(n: &Normative) -> NormativeContext {
    NormativeContext {
        continuity: n.continuity.as_str().to_owned(),
        mutation: n.mutation.as_str().to_owned(),
        independence: n
            .independence
            .as_ref()
            .map(|set| set.iter().map(|s| s.as_str().to_owned()).collect()),
        capabilities: n
            .capabilities
            .iter()
            .map(|c| c.as_str().to_owned())
            .collect(),
        integrity: n.integrity.as_str().to_owned(),
        evidence_custody: n.evidence_custody.as_str().to_owned(),
        observer_class: n.observer_class.as_str().to_owned(),
    }
}

fn constraint_context(c: &RealizationConstraint) -> ConstraintContext {
    ConstraintContext {
        dimension: c.dimension.as_str().to_owned(),
        allowed: c
            .allowed
            .iter()
            .map(|class| class.as_str().to_owned())
            .collect(),
    }
}

fn row_status(status: EnforceStatus) -> RowStatus {
    match status {
        EnforceStatus::Supported => RowStatus::Supported,
        EnforceStatus::SupportedWithRealizationMapping => {
            RowStatus::SupportedWithRealizationMapping
        }
        EnforceStatus::None => RowStatus::None,
    }
}

fn row_class(class: EvidenceClass) -> RowEvidenceClass {
    match class {
        EvidenceClass::Constraint => RowEvidenceClass::Constraint,
        EvidenceClass::Recording => RowEvidenceClass::Recording,
        EvidenceClass::ConstraintAndRecording => RowEvidenceClass::ConstraintAndRecording,
        EvidenceClass::Observation => RowEvidenceClass::Observation,
        EvidenceClass::NotApplicable => RowEvidenceClass::NotApplicable,
    }
}

fn unestablished(id: String, scope: &str, requirement: &str, basis: &str) -> Blocker {
    Blocker {
        id,
        class: BlockerClass::Unestablished,
        scope: scope.to_owned(),
        requirement: requirement.to_owned(),
        basis: basis.to_owned(),
    }
}

/// Derives a preparation from one canonical candidate.
///
/// Refuses (see [`PreparationRefusal`]) an unknown or ambiguous obligation, a candidate the
/// organization view refuses, an invalid subject, an unsupported result contract and any
/// mandatory semantics without a mechanism. Otherwise returns the projection with its blockers:
/// always the unestablished mappings listed in [`ReviewPreparation::blockers`], plus a known
/// incompatibility for each conflicting value in a supplied realization claim.
pub fn prepare(
    candidate: &OrganizationCandidate,
    request: &PreparationRequest,
) -> Result<ReviewPreparation, PreparationRefusal> {
    let selection = Selection::new(candidate, &request.obligation_id)?;
    validate_subject(&request.subject.to_js_value())
        .map_err(|error| PreparationRefusal::InvalidSubject(error.message))?;

    let obligation = selection.obligation;
    let problem_scope = candidate.problem_scope();
    let identity = CandidateIdentity {
        requirements_digest: candidate.requirements_digest().to_hex(),
        candidate_content_digest: candidate.content_digest().to_hex(),
        profile: candidate.profile().as_str().to_owned(),
        run_id: problem_scope.run_id.as_str().to_owned(),
        plan_lineage: problem_scope.plan_lineage.as_str().to_owned(),
        slice_number: problem_scope.slice_number,
    };

    let vantage = selection
        .view
        .vantages
        .iter()
        .find(|v| v.id == selection.served_by)
        .ok_or_else(|| PreparationRefusal::ServingVantage {
            obligation: obligation.id.as_str().to_owned(),
        })?;
    let vantage = VantageContext {
        id: vantage.id.clone(),
        effective: vantage.effective.as_ref().map(normative_context),
        effective_constraints: vantage
            .effective_constraints
            .iter()
            .map(constraint_context)
            .collect(),
        conditions: vantage
            .conditions
            .iter()
            .map(|c| c.as_str().to_owned())
            .collect(),
    };

    let conditions: Vec<ConditionContext> = selection
        .conditions
        .iter()
        .map(|c| ConditionContext {
            id: c.id.as_str().to_owned(),
            vantage: c.vantage.clone(),
            consumer: c.consumer.clone(),
            boundary: c.boundary.as_str().to_owned(),
            proposition: c.proposition.clone(),
            covered_state: c.covered_state.as_str().to_owned(),
            acceptor: c.acceptor.as_str().to_owned(),
            unestablished_route: c.unestablished_route.as_str().to_owned(),
            normative: normative_context(&c.normative),
            constraints: c
                .realization_constraints
                .iter()
                .map(constraint_context)
                .collect(),
        })
        .collect();

    let is_selected_condition = |id: &str| selection.conditions.iter().any(|c| c.id.as_str() == id);
    let host_enforcement = HostEnforcement {
        rows: selection
            .rows()
            .map(|row| EnforcementRow {
                property: row.property.clone(),
                scope: row.scope.clone(),
                requirement: row.requirement.clone(),
                status: row_status(row.status),
                evidence_class: row_class(row.evidence_class),
                note: row.note.to_owned(),
            })
            .collect(),
        separation: selection
            .view
            .separation_requirements
            .iter()
            .filter(|s| is_selected_condition(s.condition.as_str()))
            .map(|s| SeparationContext {
                condition: s.condition.as_str().to_owned(),
                acceptor: s.acceptor.clone(),
                vantages: s.vantages.clone(),
            })
            .collect(),
        authority: selection
            .view
            .authority_requirements
            .iter()
            .map(|a| AuthorityContext {
                id: a.id.as_str().to_owned(),
                principal: a.principal.as_str().to_owned(),
                permission: a.permission.as_str().to_owned(),
                boundaries: a
                    .boundaries
                    .as_ref()
                    .map(|set| set.iter().map(|b| b.as_str().to_owned()).collect()),
            })
            .collect(),
        component_binding: selection
            .view
            .component_binding_requirements
            .iter()
            .map(|c| ComponentBindingContext {
                component: c.component.clone(),
                permissions: c.permissions.clone(),
            })
            .collect(),
    };

    let provenance: Vec<Provenance> = candidate
        .requirement_trace()
        .iter()
        .filter(|row| match row.target.kind {
            // Ids are unique per kind, not across kinds: an id alone does not name an owner.
            "obligation" => row.target.id == obligation.id.as_str(),
            "condition" => is_selected_condition(&row.target.id),
            // The owner keys a separation requirement by its owning condition, so the id names
            // a selected condition. Its source endpoints (the condition's acceptor and
            // obligation, and every builder condition's consumer class, whichever obligation
            // owns that condition) are carried unchanged: they are the provenance of the
            // separation requirement and blocker this preparation emits.
            "separationRequirement" => is_selected_condition(&row.target.id),
            _ => false,
        })
        .map(|row| Provenance {
            target: TraceEndpoint {
                kind: row.target.kind.to_owned(),
                id: row.target.id.clone(),
                field: row.target.field.clone(),
            },
            source: TraceEndpoint {
                kind: row.source.kind.to_owned(),
                id: row.source.id.clone(),
                field: row.source.field.clone(),
            },
        })
        .collect();

    let realization_evaluation = request
        .realization
        .as_ref()
        .map(|claim| realization::evaluate(&selection.conditions, claim));

    let mut blockers = Vec::new();
    let obligation_id = obligation.id.as_str();
    blockers.push(unestablished(
        format!("scope-digest-to-subject:{obligation_id}"),
        obligation_id,
        "the obligation's scope.digest identifies the supplied exact subject",
        "no owner states the rule that relates scope.digest to the review subject; the supplied subject is carried as a claim",
    ));
    blockers.push(unestablished(
        "problem-scope-to-episode-identity".to_owned(),
        obligation_id,
        "the problem scope (run, plan lineage, slice) corresponds to a review episode identity",
        "no owner states the correspondence between problem-scope fields and episode identity fields",
    ));
    for separation in &host_enforcement.separation {
        blockers.push(unestablished(
            format!("separation:{}", separation.condition),
            &separation.condition,
            &format!(
                "{} is separate from {}",
                separation.acceptor,
                separation.vantages.join(",")
            ),
            "no qualified principal-to-vantage identity mapping exists (OC2); no mechanism can establish separation",
        ));
    }
    for condition in &selection.conditions {
        let scope = condition.id.as_str();
        blockers.push(unestablished(
            format!("consumption-evidence-mapping:{scope}"),
            scope,
            &format!(
                "boundary `{}` and consumer `{}` correspond to an evidence admission",
                condition.boundary, condition.consumer
            ),
            "no owner states how a condition's boundary and consumer participant map to an admission's boundary and consumer; matching strings are not a mapping",
        ));
        let observer = selection.rows().find(|row| {
            row.scope == scope
                && row.property == "observerClass"
                && row.status == EnforceStatus::SupportedWithRealizationMapping
        });
        if let Some(row) = observer {
            blockers.push(unestablished(
                format!("observer-identity:{scope}"),
                scope,
                &row.requirement,
                "the realization admission must state the class-to-identity mapping; none exists",
            ));
        }
        blockers.push(unestablished(
            format!("realization-admission:{scope}"),
            scope,
            "the realization is admitted with evidence of the required capabilities, mutation, provider family and confinement",
            "a supplied realization descriptor is a claim; no admission evidence exists",
        ));
    }
    for component in &host_enforcement.component_binding {
        blockers.push(unestablished(
            format!("component-binding:{}", component.component),
            &component.component,
            &format!(
                "bound to a principal holding {} over the problem scope",
                component.permissions.join(",")
            ),
            "the candidate states a requirement only; no verified mapping exists",
        ));
    }
    for authority in &host_enforcement.authority {
        blockers.push(unestablished(
            format!("authority-requirement:{}", authority.id),
            &authority.id,
            &format!("{} holds {}", authority.principal, authority.permission),
            "a required authority is a dependency, not a grant; no admitted grant covers it",
        ));
    }
    if let Some(evaluation) = &realization_evaluation {
        for condition in &evaluation.conditions {
            for check in &condition.checks {
                let class = match check.disposition {
                    Disposition::ClaimedCompatible => continue,
                    Disposition::KnownIncompatible => BlockerClass::KnownIncompatibility,
                    Disposition::Unestablished => BlockerClass::Unestablished,
                };
                blockers.push(Blocker {
                    id: format!("realization:{}:{}", condition.condition, check.property),
                    class,
                    scope: condition.condition.clone(),
                    requirement: format!("{} = {}", check.property, check.required),
                    basis: check.basis.clone(),
                });
            }
        }
    }

    let model_context = ModelContext {
        identity,
        obligation: ObligationContext {
            id: obligation_id.to_owned(),
            kind: obligation.kind().to_owned(),
            subject_class: obligation.subject().to_owned(),
            result_contract_id: obligation.result_contract.id.as_str().to_owned(),
            result_contract_version: obligation.result_contract.version,
            scope_digest: obligation.scope_digest.to_hex(),
            subject: request.subject.clone(),
        },
        vantage,
        conditions,
    };

    Ok(ReviewPreparation {
        status: PreparationStatus::InertNotAnAdmissionToken,
        model_context,
        host_enforcement,
        provenance,
        realization_evaluation,
        blockers,
    })
}
