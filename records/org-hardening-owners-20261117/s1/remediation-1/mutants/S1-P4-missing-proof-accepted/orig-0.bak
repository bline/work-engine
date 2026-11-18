//! Fixture-only consumer (exists only with the non-default `fixture` feature).
//!
//! Everything a caller supplies here is a **synthetic test assumption**: the declaration that
//! connects a candidate's obligation and conditions to a subject, an episode and an evidence
//! consumption boundary is not evidence, establishes no mapping in production and cannot satisfy
//! or alter [`crate::prepare`]. This module checks that the supplied admissions are complete and
//! consistent *relative to those assumptions*. It does not establish that the assumptions are
//! lawful, that referenced evidence is authentic, or that any principal is separated from a
//! vantage.
//!
//! After that mechanical gate, the existing episode transition (`RecordResult`) is called
//! unchanged, so revision, writer authority, phase, exact subject and lineage remain the episode
//! owner's checks. A successful result is a fixture observation only.
//!
//! No consumption without a gate-issued proof: immediately before the transition the consumer
//! requires a [`ConsumptionProof`], an organizational admission (S-A,
//! `work-engine-organization-authority`) for a consumption boundary plus the consuming host's
//! fresh observation for it. It compares the admission's request identity with its own inputs
//! (candidate content digest, the declaration's subject, the obligation's serving vantage) and
//! the boundary kind, and runs S-A's point check itself; only `Current` lets the episode advance.
//! Only S-A can produce either value, and the S-A production policy admits nothing, so in a build
//! without S-A's `fixture` feature this consumer cannot advance an episode at all.
//!
//! What the proof does not establish (fixture standing): with S-A's `fixture` feature on, any caller
//! can mint an admission and an observation and consume outside review-delivery's gate; the
//! consumer checks that S-A found *an* admission current for this candidate, subject and vantage,
//! not whose admission it is. Only the boundary **kind** is compared: the boundary's operation
//! and epoch are not (class O10; a consumption admission for another operation passes), and the
//! proof is reusable (the episode owner refuses a second use through its revision check). The
//! observation's freshness is a caller assertion ([`GATE_OBSERVATION_FRESHNESS_ASSERTED_LABEL`]). The
//! consumer takes no proof argument it could skip:
//!
//! ```compile_fail,E0061
//! use work_engine_review_preparation::fixture::consume_fixture_result;
//! fn skip(
//!     c: &work_engine_organization::OrganizationCandidate,
//!     d: &work_engine_review_preparation::fixture::FixtureDeclaration,
//!     s: &review_episode_core::state::ReviewEpisodeState,
//!     a: &review_episode_core::identity::Authority,
//!     r: review_episode_core::identity::Revision,
//! ) {
//!     let _ = consume_fixture_result(c, d, s, a, r, "t", "{}", &[], "[]");
//! }
//! ```
//!
//! (Without the feature none of this exists; the crate-level documentation checks that.)
//!
//! The production preparation takes no declaration, and the fixture coverage converts into
//! nothing the preparation returns:
//!
//! ```compile_fail,E0277
//! use work_engine_review_preparation::{ReviewPreparation, fixture::FixtureCoverage};
//! fn upgrade(c: FixtureCoverage) -> ReviewPreparation { c.into() }
//! ```

use std::collections::BTreeSet;

use review_episode_core::EpisodeError;
use review_episode_core::codec::{JsValue, digest, field, parse_json};
use review_episode_core::command::{Action, TransitionCommand};
use review_episode_core::identity::{Authority, Revision};
use review_episode_core::implementation_review_v1::validate_subject;
use review_episode_core::reducer::{TransitionOutcome, transition};
use review_episode_core::state::{ReviewEpisodeState, validate_admission};
use serde::{Deserialize, Serialize};
use work_engine_organization::{EnforceStatus, EvidenceClass, OrganizationCandidate};
use work_engine_organization_authority::{
    AdmissionState, Boundary, BoundaryKind, EffectAdmission, ObservationRevision,
    SealedObservation, Unobserved,
};

use crate::prepare::Subject;
use crate::select::{PreparationRefusal, Selection};

/// The label every declaration must carry. A declaration without it is refused.
pub const FIXTURE_LABEL: &str = "synthetic_test_assumption_not_evidence";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredProblemScope {
    pub run_id: String,
    pub plan_lineage: String,
    pub slice_number: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredEpisode {
    pub run_id: String,
    pub slice_number: u64,
    pub attempt_id: String,
    pub plan_version: String,
    pub review_obligation_id: String,
    pub review_episode_id: String,
}

/// The references an admission must carry for one condition, as assumed by the fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedRefs {
    pub claim_revision: serde_json::Value,
    pub establishment: serde_json::Value,
    pub observation: Option<serde_json::Value>,
    pub consumption: serde_json::Value,
}

/// One declared connection: a condition of the candidate, the participant that consumes it, and
/// the admission boundary, consumer and references the fixture assumes correspond to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionBinding {
    pub condition_id: String,
    /// Restates the candidate's condition boundary; checked for consistency, not a mapping.
    pub boundary: String,
    /// Restates the candidate's consumer participant; checked for consistency, not a mapping.
    pub consumer_participant: String,
    /// The consumer string the fixture assumes the admission carries.
    pub admission_consumer: String,
    pub expected: ExpectedRefs,
}

/// A synthetic declaration connecting one candidate and obligation to a subject and an episode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureDeclaration {
    pub assumption_label: String,
    pub candidate_content_digest: String,
    pub obligation_id: String,
    pub problem_scope: DeclaredProblemScope,
    pub subject: Subject,
    pub episode: DeclaredEpisode,
    pub bindings: Vec<ConditionBinding>,
}

/// A refusal before or by the consumer. Gate refusals precede the episode transition and leave
/// every input untouched; `Episode` carries the episode owner's error unchanged.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConsumeRefusal {
    #[error("selection refused: {0}")]
    Selection(#[from] PreparationRefusal),
    #[error("the declaration is not labelled `{FIXTURE_LABEL}`")]
    UnlabelledAssumption,
    #[error("declaration {field} `{declared}` conflicts with `{actual}`")]
    DeclarationConflict {
        field: &'static str,
        declared: String,
        actual: String,
    },
    #[error("required condition `{condition}` has no declared binding")]
    RequiredConditionUnmapped { condition: String },
    #[error(
        "declared binding for `{condition}`, which is not a required condition of the obligation"
    )]
    BindingNotRequired { condition: String },
    #[error("more than one declared binding for condition `{condition}`")]
    DuplicateBinding { condition: String },
    #[error(
        "boundary `{boundary}` and consumer `{consumer}` are declared for more than one condition"
    )]
    AmbiguousBindingKey { boundary: String, consumer: String },
    #[error("condition `{condition}` requires an observation reference but none is declared")]
    ObservationNotDeclared { condition: String },
    #[error("{what} is not valid input: {error}")]
    Input {
        what: &'static str,
        error: EpisodeError,
    },
    #[error("no admission for required condition `{condition}`")]
    AdmissionMissing { condition: String },
    #[error(
        "admission {index} (boundary `{boundary}`, consumer `{consumer}`) matches no declared binding"
    )]
    AdmissionUnbound {
        index: usize,
        boundary: String,
        consumer: String,
    },
    #[error("more than one admission for condition `{condition}`")]
    AdmissionDuplicate { condition: String },
    #[error("admission for `{condition}` carries a {reference} that differs from the declared one")]
    AdmissionReferenceMismatch {
        condition: String,
        reference: &'static str,
    },
    #[error("admission for `{condition}` has status `{status}`, not `established`")]
    EvidenceNotEstablished { condition: String, status: String },
    #[error("admission for `{condition}` carries no observation reference where one is required")]
    ObservationMissing { condition: String },
    /// The consumption proof's admission was issued for another boundary kind, candidate,
    /// subject or vantage than this consumption's; `field` is the first differing one.
    #[error("the consumption proof's {field} differs from this consumption's")]
    GateProofMismatch { field: &'static str },
    /// S-A's point check of the proof's admission against the host's regate observation did not
    /// find it current (Unknown, including no observation, fails closed).
    #[error("the consumption proof's admission is not current: {0:?}")]
    GateProofNotCurrent(AdmissionState),
    #[error("episode transition refused: {0}")]
    Episode(EpisodeError),
}

/// The label on the regate time a [`ConsumptionProof`] carries: the consuming host sampled it, and
/// the consumer cannot check it (fixture standing: a synthetic stand-in for a host clock).
pub const GATE_TIME_ASSERTED_LABEL: &str = "caller_asserted_regate_time";

/// The label on the freshness of the regate observation a [`ConsumptionProof`] carries: the
/// consumer cannot tell a fresh observation from an earlier one for the same reservation, so that
/// it is fresh is the caller's assertion, not something this crate checked.
pub const GATE_OBSERVATION_FRESHNESS_ASSERTED_LABEL: &str =
    "caller_asserted_regate_observation_freshness";

/// What the consumer requires before it advances an episode: a proof that the consumption-boundary
/// gate finds an organizational admission current for this candidate, subject and vantage.
///
/// - `admission` (S-A-issued): an [`EffectAdmission`] for a [`BoundaryKind::Consumption`]
///   boundary. Only S-A's `admit` produces one (no public constructor, not `Clone`), and S-A's
///   production policy admits nothing. The consumer compares its request identity with its own
///   inputs, so an admission issued for another candidate, subject, vantage or boundary **kind**
///   is refused ([`ConsumeRefusal::GateProofMismatch`]). The boundary's operation and epoch are
///   not compared (O10 class): a consumption admission for another operation passes. Nor can the
///   consumer tell whose admission it is; with S-A's `fixture` feature on, any caller can mint one.
/// - `regate_observation` (S-A-sealed): the consuming host's observation for the admission's
///   reservation. Only S-A's `observe_for` seals one (not `Clone`); S-A refuses one sealed to
///   another boundary. The consumer runs S-A's point check over it itself. It cannot tell a fresh
///   observation from an earlier one for the same reservation: freshness is a caller assertion
///   ([`GATE_OBSERVATION_FRESHNESS_ASSERTED_LABEL`], on [`GateCheck`]) and the host's obligation
///   (review-delivery's `HeldAdmission::regate` observes on every call).
/// - `regate_now_millis` (labelled caller assertion, [`GATE_TIME_ASSERTED_LABEL`]): the time the
///   host sampled for that regate; it decides expiry (O4) in the point check.
///
/// The wrapper is a parameter bundle of borrowed values: the unforgeable parts are the two S-A
/// values. It does not carry review-delivery's finality of a settled regate (S-A F-3); a holder of
/// an admission can regate it again.
#[derive(Debug, Clone, Copy)]
pub struct ConsumptionProof<'a> {
    pub admission: &'a EffectAdmission,
    pub regate_observation: &'a Result<SealedObservation, Unobserved>,
    pub regate_now_millis: u64,
}

/// What the consumer's proof check found, carried on success.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GateCheck {
    /// The admission's reserved consumption boundary.
    pub boundary: Boundary,
    /// The revision the admission was decided at.
    pub admission_revision: ObservationRevision,
    /// The revision of the regate observation the point check found current. That the
    /// observation is fresh is a caller assertion, labelled by
    /// [`GateCheck::caller_asserted_regate_observation_freshness`].
    pub regate_revision: ObservationRevision,
    /// The host-sampled regate time, under [`GATE_TIME_ASSERTED_LABEL`].
    pub regate_now_millis: u64,
}

impl GateCheck {
    pub fn time_label(&self) -> &'static str {
        GATE_TIME_ASSERTED_LABEL
    }

    /// The standing of [`GateCheck::regate_revision`]'s observation: a point check found it
    /// current, but that it is the host's *fresh* regate observation is the caller's assertion.
    pub fn caller_asserted_regate_observation_freshness(&self) -> &'static str {
        GATE_OBSERVATION_FRESHNESS_ASSERTED_LABEL
    }
}

/// The consumer's own check of a proof against its inputs, then S-A's point check.
fn check_proof(
    candidate: &OrganizationCandidate,
    declaration: &FixtureDeclaration,
    proof: &ConsumptionProof<'_>,
) -> Result<GateCheck, ConsumeRefusal> {
    let admission = proof.admission;
    let mismatch = |field| Err(ConsumeRefusal::GateProofMismatch { field });
    if admission.boundary().kind() != BoundaryKind::Consumption {
        return mismatch("boundary kind");
    }
    let request = admission.request();
    // S-A O3 decided that the request's requirements derive its candidate digest, so equal
    // candidate digests also bind the requirements.
    if request.candidate_digest() != candidate.content_digest() {
        return mismatch("candidate digest");
    }
    let (subject, declared) = (request.subject(), &declaration.subject);
    if subject.commit != declared.commit
        || subject.tree != declared.tree
        || subject.patch_identity != declared.patch_identity
    {
        return mismatch("subject");
    }
    let selection = Selection::new(candidate, &declaration.obligation_id)?;
    if request.vantage() != selection.served_by {
        return mismatch("vantage");
    }
    match admission.admission_state(proof.regate_observation, proof.regate_now_millis) {
        AdmissionState::Current { revision } => Ok(GateCheck {
            boundary: admission.boundary().clone(),
            admission_revision: admission.revision(),
            regate_revision: revision,
            regate_now_millis: proof.regate_now_millis,
        }),
        other => Err(ConsumeRefusal::GateProofNotCurrent(other)),
    }
}

/// A condition whose evidence the fixture must see covered, derived from the obligation's
/// enforceability rows and the declaration. It is a fixture value: it converts into nothing
/// that production preparation returns.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CoveredCondition {
    pub condition: String,
    pub boundary: String,
    pub admission_consumer: String,
    pub requires_observation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixtureCoverage {
    pub label: &'static str,
    pub conditions: Vec<CoveredCondition>,
}

/// Derives the required condition coverage from the selected obligation of the candidate and
/// validates the declaration against it. The required set comes from the owner's
/// `evidenceCustody` rows for the obligation's conditions; the declaration must bind exactly
/// that set.
pub fn fixture_coverage(
    candidate: &OrganizationCandidate,
    declaration: &FixtureDeclaration,
) -> Result<FixtureCoverage, ConsumeRefusal> {
    if declaration.assumption_label != FIXTURE_LABEL {
        return Err(ConsumeRefusal::UnlabelledAssumption);
    }
    let selection = Selection::new(candidate, &declaration.obligation_id)?;

    let conflict = |field: &'static str, declared: String, actual: String| {
        ConsumeRefusal::DeclarationConflict {
            field,
            declared,
            actual,
        }
    };
    let actual_digest = candidate.content_digest().to_hex();
    if declaration.candidate_content_digest != actual_digest {
        return Err(conflict(
            "candidate_content_digest",
            declaration.candidate_content_digest.clone(),
            actual_digest,
        ));
    }
    let scope = candidate.problem_scope();
    let declared = &declaration.problem_scope;
    if declared.run_id != scope.run_id.as_str() {
        return Err(conflict(
            "problem_scope.run_id",
            declared.run_id.clone(),
            scope.run_id.to_string(),
        ));
    }
    if declared.plan_lineage != scope.plan_lineage.as_str() {
        return Err(conflict(
            "problem_scope.plan_lineage",
            declared.plan_lineage.clone(),
            scope.plan_lineage.to_string(),
        ));
    }
    if declared.slice_number != scope.slice_number {
        return Err(conflict(
            "problem_scope.slice_number",
            declared.slice_number.to_string(),
            scope.slice_number.to_string(),
        ));
    }

    // Required coverage: every condition of the selected obligation for which the owner states
    // an `evidenceCustody` row with a mechanism, with that row's evidence class.
    let mut required: Vec<(String, bool)> = Vec::new();
    for condition in &selection.conditions {
        let row = selection.rows().find(|row| {
            row.scope == condition.id.as_str()
                && row.property == "evidenceCustody"
                && row.status != EnforceStatus::None
        });
        if let Some(row) = row {
            required.push((
                condition.id.as_str().to_owned(),
                row.evidence_class == EvidenceClass::Observation,
            ));
        }
    }

    let mut seen = BTreeSet::new();
    for binding in &declaration.bindings {
        if !seen.insert(binding.condition_id.as_str()) {
            return Err(ConsumeRefusal::DuplicateBinding {
                condition: binding.condition_id.clone(),
            });
        }
        if !required.iter().any(|(id, _)| *id == binding.condition_id) {
            return Err(ConsumeRefusal::BindingNotRequired {
                condition: binding.condition_id.clone(),
            });
        }
    }
    let mut keys = BTreeSet::new();
    for binding in &declaration.bindings {
        if !keys.insert((
            binding.boundary.as_str(),
            binding.admission_consumer.as_str(),
        )) {
            return Err(ConsumeRefusal::AmbiguousBindingKey {
                boundary: binding.boundary.clone(),
                consumer: binding.admission_consumer.clone(),
            });
        }
    }

    let mut conditions = Vec::new();
    for (id, requires_observation) in &required {
        let binding = declaration
            .bindings
            .iter()
            .find(|b| b.condition_id == *id)
            .ok_or_else(|| ConsumeRefusal::RequiredConditionUnmapped {
                condition: id.clone(),
            })?;
        let condition = selection
            .conditions
            .iter()
            .find(|c| c.id.as_str() == id)
            .expect("required conditions come from the selection");
        if binding.boundary != condition.boundary.as_str() {
            return Err(conflict(
                "binding.boundary",
                binding.boundary.clone(),
                condition.boundary.to_string(),
            ));
        }
        if binding.consumer_participant != condition.consumer {
            return Err(conflict(
                "binding.consumer_participant",
                binding.consumer_participant.clone(),
                condition.consumer.clone(),
            ));
        }
        if *requires_observation && binding.expected.observation.is_none() {
            return Err(ConsumeRefusal::ObservationNotDeclared {
                condition: id.clone(),
            });
        }
        conditions.push(CoveredCondition {
            condition: id.clone(),
            boundary: binding.boundary.clone(),
            admission_consumer: binding.admission_consumer.clone(),
            requires_observation: *requires_observation,
        });
    }
    Ok(FixtureCoverage {
        label: FIXTURE_LABEL,
        conditions,
    })
}

/// The standing of a fixture consumption: one fixture observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureLabel {
    FixtureObservationOnly,
}

/// What was checked about the admissions' evidence references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStanding {
    /// Syntax and consistency with the fixture's declared references only. Not authenticity.
    ConsistentWithFixtureAssumptionsOnly,
}

/// The production consequence of the observation. It has one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionConsequence {
    Unestablished,
}

/// The acceptance of one condition. It has one value: this crate cannot accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acceptance {
    NotEstablished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FixtureCondition {
    pub condition: String,
    pub acceptor: String,
    pub unestablished_route: String,
    pub acceptance: Acceptance,
}

/// A fixture observation of a transition the episode core applied. It grants, publishes and
/// accepts nothing, and releases no production consequence.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FixtureConsumption {
    pub label: FixtureLabel,
    pub evidence: EvidenceStanding,
    pub production_consequence: ProductionConsequence,
    pub conditions: Vec<FixtureCondition>,
    /// What the consumer's check of its [`ConsumptionProof`] found.
    pub gate: GateCheck,
    outcome: TransitionOutcome,
}

impl FixtureConsumption {
    pub fn outcome(&self) -> &TransitionOutcome {
        &self.outcome
    }
    pub fn state(&self) -> &ReviewEpisodeState {
        self.outcome.state()
    }
}

fn input(what: &'static str, error: EpisodeError) -> ConsumeRefusal {
    ConsumeRefusal::Input { what, error }
}

fn text_of(value: &JsValue, what: &'static str) -> Result<String, ConsumeRefusal> {
    value
        .as_text()
        .and_then(|t| t.to_string_checked())
        .map_err(|e| input(what, e))
}

/// The canonical digest of a declared reference, through the episode codec.
fn declared_digest(
    value: &serde_json::Value,
    what: &'static str,
) -> Result<String, ConsumeRefusal> {
    let text =
        serde_json::to_string(value).map_err(|e| input(what, EpisodeError::new(e.to_string())))?;
    Ok(digest(&parse_json(&text).map_err(|e| input(what, e))?))
}

/// Gates a result and its admissions against the fixture declaration and, only if every
/// mechanical check passes, calls the episode core's `RecordResult` transition.
///
/// `state` is borrowed and never modified; a refusal returns no state. Gate refusals come before
/// the transition, so an incomplete or inconsistent evidence set cannot advance the episode
/// through this entry point. Revision, writer authority, phase, exact subject and finding
/// lineage are checked only by the episode core, and its error is returned unchanged.
///
/// The last check before the transition is the [`ConsumptionProof`]: its admission must be for a
/// consumption boundary and for this candidate, the declaration's subject and the obligation's
/// serving vantage, and S-A's point check over the host's regate observation must find it
/// `Current`. Otherwise the episode does not advance.
#[allow(clippy::too_many_arguments)]
pub fn consume_fixture_result(
    candidate: &OrganizationCandidate,
    declaration: &FixtureDeclaration,
    state: &ReviewEpisodeState,
    authority: &Authority,
    expected_revision: Revision,
    transition_id: &str,
    result_json: &str,
    unresolved_questions: &[&str],
    admissions_json: &str,
    proof: &ConsumptionProof<'_>,
) -> Result<FixtureConsumption, ConsumeRefusal> {
    let coverage = fixture_coverage(candidate, declaration)?;

    // The declaration must restate the episode and subject the state actually has.
    let identity = state.identity();
    let episode = &declaration.episode;
    let episode_fields: [(&'static str, String, String); 6] = [
        (
            "episode.run_id",
            episode.run_id.clone(),
            identity
                .run_id
                .to_string_checked()
                .map_err(|e| input("episode identity", e))?,
        ),
        (
            "episode.slice_number",
            episode.slice_number.to_string(),
            identity.slice_number.to_string(),
        ),
        (
            "episode.attempt_id",
            episode.attempt_id.clone(),
            identity
                .attempt_id
                .to_string_checked()
                .map_err(|e| input("episode identity", e))?,
        ),
        (
            "episode.plan_version",
            episode.plan_version.clone(),
            identity
                .plan_version
                .to_string_checked()
                .map_err(|e| input("episode identity", e))?,
        ),
        (
            "episode.review_obligation_id",
            episode.review_obligation_id.clone(),
            identity
                .review_obligation_id
                .to_string_checked()
                .map_err(|e| input("episode identity", e))?,
        ),
        (
            "episode.review_episode_id",
            episode.review_episode_id.clone(),
            identity
                .review_episode_id
                .to_string_checked()
                .map_err(|e| input("episode identity", e))?,
        ),
    ];
    for (field_name, declared, actual) in episode_fields {
        if declared != actual {
            return Err(ConsumeRefusal::DeclarationConflict {
                field: field_name,
                declared,
                actual,
            });
        }
    }
    let subject = declaration.subject.to_js_value();
    validate_subject(&subject).map_err(|e| input("declared subject", e))?;
    let state_subject = text_of(
        field(state.get("subject"), "sha256").map_err(|e| input("episode subject", e))?,
        "episode subject",
    )?;
    let declared_subject = digest(&subject);
    if declared_subject != state_subject {
        return Err(ConsumeRefusal::DeclarationConflict {
            field: "subject.sha256",
            declared: declared_subject,
            actual: state_subject,
        });
    }

    let result = parse_json(result_json).map_err(|e| input("result", e))?;
    let admissions = match parse_json(admissions_json).map_err(|e| input("admissions", e))? {
        JsValue::Array(items) => items,
        _ => {
            return Err(input(
                "admissions",
                EpisodeError::new("evidence admissions must be an array"),
            ));
        }
    };

    // Match each admission to exactly one declared binding, and each required condition to
    // exactly one admission. The (boundary, consumer) key selects a binding only because the
    // declaration states it; the key alone establishes nothing.
    let mut matched: Vec<Option<usize>> = vec![None; coverage.conditions.len()];
    for (index, admission) in admissions.iter().enumerate() {
        validate_admission(admission).map_err(|e| input("admission", e))?;
        let boundary = text_of(
            field(admission, "boundary").map_err(|e| input("admission", e))?,
            "admission boundary",
        )?;
        let consumer = text_of(
            field(admission, "consumer").map_err(|e| input("admission", e))?,
            "admission consumer",
        )?;
        let slot = coverage
            .conditions
            .iter()
            .position(|c| c.boundary == boundary && c.admission_consumer == consumer)
            .ok_or(ConsumeRefusal::AdmissionUnbound {
                index,
                boundary,
                consumer,
            })?;
        if matched[slot].is_some() {
            return Err(ConsumeRefusal::AdmissionDuplicate {
                condition: coverage.conditions[slot].condition.clone(),
            });
        }
        matched[slot] = Some(index);
    }
    for (slot, covered) in coverage.conditions.iter().enumerate() {
        let Some(index) = matched[slot] else {
            return Err(ConsumeRefusal::AdmissionMissing {
                condition: covered.condition.clone(),
            });
        };
        let admission = &admissions[index];
        let binding = declaration
            .bindings
            .iter()
            .find(|b| b.condition_id == covered.condition)
            .expect("coverage comes from the declaration");
        let mismatch = |reference: &'static str| ConsumeRefusal::AdmissionReferenceMismatch {
            condition: covered.condition.clone(),
            reference,
        };
        let carried = |key: &str| field(admission, key).map_err(|e| input("admission", e));
        if digest(carried("claimRevisionRef")?)
            != declared_digest(&binding.expected.claim_revision, "declared claim revision")?
        {
            return Err(mismatch("claimRevisionRef"));
        }
        if digest(carried("establishmentRef")?)
            != declared_digest(&binding.expected.establishment, "declared establishment")?
        {
            return Err(mismatch("establishmentRef"));
        }
        if digest(carried("consumptionRef")?)
            != declared_digest(&binding.expected.consumption, "declared consumption")?
        {
            return Err(mismatch("consumptionRef"));
        }
        let observation = carried("observationRef")?;
        match (&binding.expected.observation, observation) {
            (None, JsValue::Null) => {}
            (Some(expected), carried_value) if carried_value != &JsValue::Null => {
                if digest(carried_value) != declared_digest(expected, "declared observation")? {
                    return Err(mismatch("observationRef"));
                }
            }
            (Some(_), _) => {
                return Err(ConsumeRefusal::ObservationMissing {
                    condition: covered.condition.clone(),
                });
            }
            (None, _) => return Err(mismatch("observationRef")),
        }
        if covered.requires_observation && observation == &JsValue::Null {
            return Err(ConsumeRefusal::ObservationMissing {
                condition: covered.condition.clone(),
            });
        }
        let status = text_of(carried("status")?, "admission status")?;
        if status != "established" {
            return Err(ConsumeRefusal::EvidenceNotEstablished {
                condition: covered.condition.clone(),
                status,
            });
        }
    }

    let mut payload = vec![
        ("result", result),
        (
            "unresolvedQuestions",
            JsValue::Array(
                unresolved_questions
                    .iter()
                    .map(|q| JsValue::text(q))
                    .collect(),
            ),
        ),
    ];
    if !admissions.is_empty() {
        payload.push(("evidenceAdmissions", JsValue::Array(admissions)));
    }
    // No consumption without a gate-issued proof, checked against this consumption's own inputs.
    let gate = check_proof(candidate, declaration, proof)?;
    let command = TransitionCommand::new(
        authority.clone(),
        expected_revision,
        transition_id,
        Action::RecordResult,
        JsValue::object(payload),
    )
    .map_err(ConsumeRefusal::Episode)?;
    let outcome = transition(state, &command).map_err(ConsumeRefusal::Episode)?;

    Ok(FixtureConsumption {
        label: FixtureLabel::FixtureObservationOnly,
        evidence: EvidenceStanding::ConsistentWithFixtureAssumptionsOnly,
        production_consequence: ProductionConsequence::Unestablished,
        conditions: selection_dispositions(candidate, &declaration.obligation_id, &coverage)?,
        gate,
        outcome,
    })
}

fn selection_dispositions(
    candidate: &OrganizationCandidate,
    obligation_id: &str,
    coverage: &FixtureCoverage,
) -> Result<Vec<FixtureCondition>, ConsumeRefusal> {
    let selection = Selection::new(candidate, obligation_id)?;
    Ok(selection
        .conditions
        .iter()
        .filter(|c| {
            coverage
                .conditions
                .iter()
                .any(|cc| cc.condition == c.id.as_str())
        })
        .map(|c| FixtureCondition {
            condition: c.id.as_str().to_owned(),
            acceptor: c.acceptor.as_str().to_owned(),
            unestablished_route: c.unestablished_route.as_str().to_owned(),
            acceptance: Acceptance::NotEstablished,
        })
        .collect())
}
