//! Delivery gate (feature `fixture`): real discharges derived from an organizational admission
//! (`work-engine-organization-authority`, S-A) plus the labelled assumed discharges that remain.
//! Mints no authority.
//!
//! The delivery host evaluates the canonical preparation's blockers at two effect boundaries:
//! before any native child is spawned ([`Boundary::BeforeSpawn`]) and immediately before the
//! existing consumer runs ([`Boundary::BeforeConsumption`]). Each evaluation refuses
//!
//! 1. a blocker whose `kind` is not the kind its id family names (an unknown or mislabelled
//!    blocker: [`GateRefusal::KindMismatch`]);
//! 2. any [`BlockerClass::KnownIncompatibility`], whatever its kind;
//! 3. any blocker whose kind has no discharge ([`Discharge::Undischarged`]);
//! 4. any blocker whose kind is discharged by admission ([`Discharge::Admitted`]) when one of the
//!    named S-A checks did not pass in the host's own [`EffectAdmission`]
//!    ([`GateRefusal::NotAdmitted`]), or when that admission is not in [`Domain::Fixture`].
//!
//! [`discharge`] is the host's **total** function from [`BlockerKind`] to a discharge. It is
//! derived from the kind alone: no caller supplies, extends or overrides it. Admitted kinds name
//! S-A [`CheckId`]s and are decided by those ids alone (never by reason subjects, which are
//! diagnostics). `scope-digest-to-subject` is admitted by S-A check [`CheckId::S1`] (the served
//! obligations' `scope.digest` is the exact-subject digest of the admitted subject). The other two
//! mapping kinds stay labelled fixture assumptions ([`ASSUMED_DISCHARGE_LABEL`]): their mapping
//! rules have no owner (O7), so nothing admits them.
//! Realization check blockers (`realization:<condition>:<property>`) are never dischargeable.
//!
//! Every input of the admission the evaluation reads is host-derived, digest-bound or a named
//! labelled assertion ([`CALLER_ASSERTED_LABEL`]); no caller supplies an operation request or an
//! organization (S-A derives the organization from the committed admission of the candidate,
//! [`CheckId::OA`]). The summary on success ([`AdmissionSummary`]) carries the request identity
//! the host constructed, the organization S-A derived and the asserted inputs, so a label never
//! stays behind the gate.
//!
//! The evaluation that yields [`AdmittedDischarges`] and [`AssumedDischarges`] is crate-private
//! and takes the binding and an admission the host obtained itself, so neither value can be
//! minted by a caller. [`screen`] is the pure, token-free classification for tests.
//!
//! An assumed discharge is never an admission:
//!
//! ```compile_fail,E0603
//! use work_engine_review_delivery::gate::assess;
//! ```
//!
//! ```compile_fail,E0451
//! use work_engine_review_delivery::gate::{AssumedDischarges, Boundary};
//! fn mint() -> AssumedDischarges {
//!     AssumedDischarges { boundary: Boundary::BeforeSpawn, bound_to: String::new(), assumed: vec![] }
//! }
//! ```
//!
//! Production admission is not reachable through this module: it exists only with the `fixture`
//! feature, admits only under the S-A fixture policy, and grants nothing outside this crate's
//! fixture runner.

use work_engine_organization::Digest;
use work_engine_organization_authority::{
    AdmissionRefusal, AdmissionState, AdmittedOrganization, AuthorityObservation,
    Boundary as AuthorityBoundary, CheckId, DeclaredRealization, Domain, EffectAdmission, Fixture,
    ObservationRevision, OperationRequest, Outcomes, Reservation, SealedObservation, SubjectRef,
    Unobserved, admit, observe_for, reserve,
};
use work_engine_review_preparation::{Blocker, BlockerClass, BlockerKind};

use crate::binding::ExperimentBinding;

/// The label every assumed discharge carries. Fixture standing only; never an admission.
pub const ASSUMED_DISCHARGE_LABEL: &str = "fixture_assumed_discharge_not_admission";

/// The label every admission summary carries: an S-A admission in `Domain::Fixture`, decided over
/// synthetic fixture records. Not a production admission.
pub const FIXTURE_ADMISSION_LABEL: &str = "fixture_domain_admission_not_production";

/// The label on the input of an admission that a caller supplied and no host-owned fact anchors:
/// the declared actor identity (no host observation of the spawned process). S-A R3 is decided
/// relative to it.
pub const CALLER_ASSERTED_LABEL: &str = "caller_asserted_unanchored";

/// The time at which the host reaches a boundary, supplied as a **source** the host samples at
/// each evaluation (admission and regate), never as one value. Fixture standing: a synthetic
/// stand-in for a host clock, like the authority source.
pub trait TimeSource {
    fn now_millis(&self) -> u64;
}

/// What discharges one blocker kind on the fixture vertical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discharge {
    /// Discharged only when every named check passed in the [`EffectAdmission`] the host obtained
    /// itself for the boundary.
    Admitted(&'static [CheckId]),
    /// A labelled fixture assumption (standing of `FixtureDeclaration`): the kind is assumed
    /// discharged for the synthetic vertical only. Nothing is established or admitted.
    AssumedDischarge,
    /// Nothing discharges the kind; a blocker of this kind always refuses.
    Undischarged,
}

/// The host-owned total discharge function (S-B mapping).
pub fn discharge(kind: BlockerKind) -> Discharge {
    match kind {
        BlockerKind::Separation => Discharge::Admitted(&[CheckId::OA, CheckId::O8, CheckId::R3]),
        BlockerKind::ComponentBinding => {
            Discharge::Admitted(&[CheckId::OA, CheckId::O4, CheckId::R3])
        }
        BlockerKind::AuthorityRequirement => Discharge::Admitted(&[CheckId::O4]),
        BlockerKind::RealizationAdmission => {
            Discharge::Admitted(&[CheckId::R1, CheckId::R2, CheckId::R4])
        }
        BlockerKind::ObserverIdentity => {
            Discharge::Admitted(&[CheckId::OA, CheckId::R2, CheckId::R3])
        }
        BlockerKind::ScopeDigestToSubject => Discharge::Admitted(&[CheckId::S1]),
        BlockerKind::ProblemScopeToEpisodeIdentity | BlockerKind::ConsumptionEvidenceMapping => {
            Discharge::AssumedDischarge
        }
        BlockerKind::RealizationCheck => Discharge::Undischarged,
    }
}

/// The effect boundary an evaluation guards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boundary {
    /// Before a native child is spawned and given the contract.
    BeforeSpawn,
    /// After a verified native attempt, immediately before the existing consumer runs.
    BeforeConsumption,
}

/// Why an evaluation refused. Each list names every offending blocker id, in preparation order.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GateRefusal {
    #[error("blocker `{id}` carries kind {kind:?}, but its id names {named:?}")]
    KindMismatch {
        id: String,
        kind: BlockerKind,
        named: Option<BlockerKind>,
    },
    #[error("known realization incompatibility: {ids:?}")]
    KnownIncompatibility { ids: Vec<String> },
    #[error("no discharge exists for blocker(s): {ids:?}")]
    Undischarged { ids: Vec<String> },
    #[error("the host-derived realization description differs from the one the binding prepared")]
    RealizationDiffers,
    #[error("the owner refused to recompute the preparation at the boundary: {0}")]
    Preparation(String),
    #[error("the preparation recomputed at the boundary differs from the delivered preparation")]
    PreparationChanged,
    /// No authority observation exists for the reserved boundary (fail closed).
    #[error("no authority observation: {0}")]
    NoObservation(String),
    /// The S-A admission refused; every check outcome is carried.
    #[error("organizational admission refused: {0:?}")]
    Admission(AdmissionRefusal),
    /// The admission does not describe the binding (S-A F-7): `field` is the first differing one.
    #[error("the admission's {field} differs from the binding's")]
    AdmissionMismatch { field: &'static str },
    /// The admission is not in `Domain::Fixture` (S-A F-8).
    #[error("the admission is not in the fixture domain")]
    NotFixtureDomain,
    /// Admitted kinds whose named checks did not all pass.
    #[error("no admitted discharge for blocker(s): {ids:?}")]
    NotAdmitted { ids: Vec<String> },
    /// The regate before consumption did not find the admission current (Unknown fails closed).
    #[error("the admission is not current at the regate: {0:?}")]
    NotCurrent(AdmissionState),
}

/// Pure classification of a blocker list: refusals 1-3 of the module docs, then each blocker's
/// discharge. Plain data, no token: it authorizes nothing.
#[allow(clippy::type_complexity)]
pub fn screen(blockers: &[Blocker]) -> Result<Vec<(String, BlockerKind, Discharge)>, GateRefusal> {
    for blocker in blockers {
        let named = BlockerKind::of_id(&blocker.id);
        if named != Some(blocker.kind) {
            return Err(GateRefusal::KindMismatch {
                id: blocker.id.clone(),
                kind: blocker.kind,
                named,
            });
        }
    }
    let ids = |keep: &dyn Fn(&Blocker) -> bool| -> Vec<String> {
        blockers
            .iter()
            .filter(|b| keep(b))
            .map(|b| b.id.clone())
            .collect()
    };
    let incompatible = ids(&|b| b.class == BlockerClass::KnownIncompatibility);
    if !incompatible.is_empty() {
        return Err(GateRefusal::KnownIncompatibility { ids: incompatible });
    }
    let undischarged = ids(&|b| discharge(b.kind) == Discharge::Undischarged);
    if !undischarged.is_empty() {
        return Err(GateRefusal::Undischarged { ids: undischarged });
    }
    Ok(blockers
        .iter()
        .map(|b| (b.id.clone(), b.kind, discharge(b.kind)))
        .collect())
}

/// The request identity the host constructed for the admission (from the binding, never from a
/// caller): the vantage of the preparation, the binding's subject and the binding's candidate
/// content digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestIdentity {
    pub vantage: String,
    pub subject: SubjectRef,
    pub candidate_digest: Digest,
}

/// The admission input a caller asserted that no host-owned fact anchors, under
/// [`CALLER_ASSERTED_LABEL`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerAsserted {
    pub actor_identity: String,
}

impl CallerAsserted {
    pub fn label(&self) -> &'static str {
        CALLER_ASSERTED_LABEL
    }
}

/// What the host's admission at one boundary decided: domain (always `Domain::Fixture` here),
/// reserved boundary, observation revision, every check outcome by id, the request identity the
/// host constructed, the organization S-A derived, the caller-asserted input it was decided over, and the revision of the
/// fresh regate observation that found it current (the delivery regate immediately before the
/// spawn; the consumption regate immediately before the consumer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionSummary {
    domain: Domain,
    boundary: AuthorityBoundary,
    revision: ObservationRevision,
    outcomes: Outcomes,
    request: RequestIdentity,
    organization: AdmittedOrganization,
    asserted: CallerAsserted,
    regate_revision: Option<ObservationRevision>,
}

impl AdmissionSummary {
    pub fn label(&self) -> &'static str {
        FIXTURE_ADMISSION_LABEL
    }
    /// The request identity the admission was decided for (host-constructed).
    pub fn request(&self) -> &RequestIdentity {
        &self.request
    }
    /// The organization S-A derived from the committed admission of the candidate (unlabelled:
    /// no caller supplied it).
    pub fn organization(&self) -> &AdmittedOrganization {
        &self.organization
    }
    /// The caller-asserted, unanchored input the admission was decided over.
    pub fn asserted(&self) -> &CallerAsserted {
        &self.asserted
    }
    pub fn domain(&self) -> Domain {
        self.domain
    }
    pub fn boundary(&self) -> &AuthorityBoundary {
        &self.boundary
    }
    pub fn revision(&self) -> ObservationRevision {
        self.revision
    }
    pub fn outcomes(&self) -> &Outcomes {
        &self.outcomes
    }
    /// The revision of the fresh observation the boundary's regate found current. `Some` on every
    /// summary a [`crate::native::Delivered`] carries; `None` only before that regate ran.
    pub fn regate_revision(&self) -> Option<ObservationRevision> {
        self.regate_revision
    }
}

/// The blockers discharged by the host's admission at one boundary, each with the check ids that
/// discharged it. Constructible only by the crate-private evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedDischarges {
    boundary: Boundary,
    bound_to: String,
    admission: AdmissionSummary,
    admitted: Vec<(String, BlockerKind, Vec<CheckId>)>,
}

impl AdmittedDischarges {
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    /// The frozen contract digest of the binding this evaluation was derived from.
    pub fn bound_to(&self) -> &str {
        &self.bound_to
    }
    pub fn admission(&self) -> &AdmissionSummary {
        &self.admission
    }
    /// Every admitted blocker id with its kind and discharging check ids, in preparation order.
    pub fn admitted(&self) -> &[(String, BlockerKind, Vec<CheckId>)] {
        &self.admitted
    }
}

/// The blockers that passed only by labelled assumed discharge (the unowned mapping kinds), at one
/// boundary, bound to the binding they were derived from. Constructible only by the crate-private
/// evaluation. It is not an admission and authorizes nothing outside this crate's fixture runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssumedDischarges {
    boundary: Boundary,
    bound_to: String,
    assumed: Vec<(String, BlockerKind)>,
}

impl AssumedDischarges {
    pub fn label(&self) -> &'static str {
        ASSUMED_DISCHARGE_LABEL
    }
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    /// The frozen contract digest of the binding this evaluation was derived from.
    pub fn bound_to(&self) -> &str {
        &self.bound_to
    }
    /// Every blocker id assumed discharged, with its kind, in preparation order.
    pub fn assumed(&self) -> &[(String, BlockerKind)] {
        &self.assumed
    }
}

/// The evaluation at one boundary over the binding's blockers and the host's own admission.
pub(crate) fn assess(
    boundary: Boundary,
    binding: &ExperimentBinding,
    blockers: &[Blocker],
    admission: &EffectAdmission,
) -> Result<(AdmittedDischarges, AssumedDischarges), GateRefusal> {
    let classified = screen(blockers)?;
    if admission.domain() != Domain::Fixture {
        return Err(GateRefusal::NotFixtureDomain);
    }
    let outcomes = admission.outcomes();
    let not_admitted: Vec<String> = classified
        .iter()
        .filter(|(_, _, d)| match d {
            Discharge::Admitted(checks) => !checks.iter().all(|c| outcomes.passed(*c)),
            _ => false,
        })
        .map(|(id, _, _)| id.clone())
        .collect();
    if !not_admitted.is_empty() {
        return Err(GateRefusal::NotAdmitted { ids: not_admitted });
    }
    let bound_to = binding.frozen().contract_sha256().to_owned();
    let request = admission.request();
    let mut admitted = Vec::new();
    let mut assumed = Vec::new();
    for (id, kind, d) in classified {
        match d {
            Discharge::Admitted(checks) => admitted.push((id, kind, checks.to_vec())),
            Discharge::AssumedDischarge => assumed.push((id, kind)),
            Discharge::Undischarged => unreachable!("screen refuses undischarged kinds"),
        }
    }
    Ok((
        AdmittedDischarges {
            boundary,
            bound_to: bound_to.clone(),
            admission: AdmissionSummary {
                domain: admission.domain(),
                boundary: admission.boundary().clone(),
                revision: admission.revision(),
                outcomes: outcomes.clone(),
                request: RequestIdentity {
                    vantage: request.vantage().to_owned(),
                    subject: request.subject().clone(),
                    candidate_digest: request.candidate_digest(),
                },
                organization: admission.organization().clone(),
                asserted: CallerAsserted {
                    actor_identity: admission.actor_identity().to_owned(),
                },
                regate_revision: None,
            },
            admitted,
        },
        AssumedDischarges {
            boundary,
            bound_to,
            assumed,
        },
    ))
}

impl AdmittedDischarges {
    pub(crate) fn with_regate(mut self, revision: ObservationRevision) -> Self {
        self.admission.regate_revision = Some(revision);
        self
    }
}

/// One admission the host obtained itself at one reserved boundary, held for its regate.
///
/// [`HeldAdmission::admit`] reserves the boundary, takes one observation for it and admits under
/// the fixture policy. [`HeldAdmission::regate`] takes a **fresh** observation from the source for
/// the same reservation on every call (S-A F-2: it accepts no observation, so the admission-time
/// one cannot be replayed) and returns the admission's state. The first non-current state other
/// than `Unknown` is final for this admission (S-A F-3): later regates return it unchanged.
/// `Unknown` is not final and does not permit the effect (fail closed). Single use of the
/// boundary is not recorded here (O10 gap).
///
/// The admission holds the authority source and the time source it was admitted with, so a regate
/// takes no argument that could differ from them; the time source is sampled at the admission and
/// at every regate (never reused from the admission).
pub struct HeldAdmission<'a> {
    source: &'a dyn AuthorityObservation,
    clock: &'a dyn TimeSource,
    reservation: Reservation,
    admission: EffectAdmission,
    settled: Option<AdmissionState>,
}

impl std::fmt::Debug for HeldAdmission<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeldAdmission")
            .field("reservation", &self.reservation)
            .field("admission", &self.admission)
            .field("settled", &self.settled)
            .finish_non_exhaustive()
    }
}

impl<'a> HeldAdmission<'a> {
    pub fn admit(
        source: &'a dyn AuthorityObservation,
        clock: &'a dyn TimeSource,
        boundary: AuthorityBoundary,
        request: &OperationRequest,
        realization: &DeclaredRealization,
    ) -> Result<Self, GateRefusal> {
        let reservation = reserve(boundary);
        let observation = observe_for(source, &reservation)
            .map_err(|unobserved| GateRefusal::NoObservation(unobserved.detail))?;
        let admission = admit::<Fixture>(
            &reservation,
            observation,
            request,
            realization,
            clock.now_millis(),
        )
        .map_err(GateRefusal::Admission)?;
        Ok(Self {
            source,
            clock,
            reservation,
            admission,
            settled: None,
        })
    }

    pub fn admission(&self) -> &EffectAdmission {
        &self.admission
    }

    /// Point check against a fresh observation of the held source for this admission's
    /// reservation, at a freshly sampled time.
    pub fn regate(&mut self) -> AdmissionState {
        self.regate_observed().0
    }

    /// [`HeldAdmission::regate`], also returning the fresh observation and the time sample it was
    /// decided over (none once the state is settled: no observation is taken then). The consumer
    /// re-runs S-A's point check over exactly these inputs before it advances an episode.
    pub(crate) fn regate_observed(&mut self) -> (AdmissionState, Option<RegateInputs>) {
        if let Some(settled) = &self.settled {
            return (settled.clone(), None);
        }
        let fresh = observe_for(self.source, &self.reservation);
        let now_millis = self.clock.now_millis();
        let state = self.admission.admission_state(&fresh, now_millis);
        if !matches!(
            state,
            AdmissionState::Current { .. } | AdmissionState::Unknown(_)
        ) {
            self.settled = Some(state.clone());
        }
        (
            state,
            Some(RegateInputs {
                observation: fresh,
                now_millis,
            }),
        )
    }
}

/// The inputs of one regate that took a fresh observation.
pub(crate) struct RegateInputs {
    pub(crate) observation: Result<SealedObservation, Unobserved>,
    pub(crate) now_millis: u64,
}
