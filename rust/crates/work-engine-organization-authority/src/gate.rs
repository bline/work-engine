//! Gate O subset (O3, O4, O5, O8, O9), Gate R (R1-R4), effect admission and its states.
//!
//! Every check is evaluated over one sealed observation, without short circuit; each admission
//! decision carries one outcome per [`CheckId`]. Gate O and Gate R are separate functions with
//! separate inputs; [`admit`] composes them so that the outcome vector is complete even when
//! Gate O refuses.

use std::collections::{BTreeMap, BTreeSet};

use work_engine_organization::{
    Digest, EnforceStatus, OrganizationCandidate, OrganizationalRequirements, RealizationClass,
    RealizationConstraint, UnresolvedKind, derive, enforceability, view,
};

use crate::ids::{
    BodyDigest, Boundary, ExecutableDigest, IssuanceId, ObservationRevision, RealizationDigest,
};
use crate::observation::{
    AuthorityState, IssuanceStatus, Reservation, Resolution, SealedObservation, Unobserved,
};
use crate::records::{
    ActorAnchor, Principal, ProblemScopeRef, RealizationProperties, Record, VantageActorBinding,
};

// ---------------------------------------------------------------------------------------------
// Check identities and outcomes
// ---------------------------------------------------------------------------------------------

/// The checks this crate decides. A host derives blocker discharges from these ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CheckId {
    /// Recomputed derivation yields exactly one candidate whose digest is the request's. Both the
    /// requirements and the digest come from the caller: this shows the request is
    /// self-consistent, not that it matches anything the owner committed (no committed admission
    /// anchor exists in this slice).
    O3,
    /// Authority requirements covered by valid grants in decision scope; component bindings.
    O4,
    /// The enforceability report has no `none` row.
    O5,
    /// Separation: acceptor and each dependent vantage resolve to different actor anchors.
    O8,
    /// `view(candidate)` is `Ok` and the derivation reports no internal error.
    O9,
    /// For each condition the vantage serves, the accepted realization satisfies its constraints.
    R1,
    /// The vantage's effective requirements hold for the accepted realization.
    R2,
    /// The actual actor (and installed components) are the bound ones.
    R3,
    /// The realization digest is accepted by exactly one committed `RealizationAcceptance` whose
    /// profile is the candidate's.
    R4,
}

impl CheckId {
    pub const ALL: [CheckId; 9] = [
        CheckId::O3,
        CheckId::O4,
        CheckId::O5,
        CheckId::O8,
        CheckId::O9,
        CheckId::R1,
        CheckId::R2,
        CheckId::R3,
        CheckId::R4,
    ];
}

/// A refusal reason code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReasonCode {
    CandidateUnderived,
    CandidateDigestMismatch,
    CandidateUnavailable,
    AuthorityMissing,
    AuthorityRevoked,
    AuthoritySuperseded,
    AuthorityExpired,
    AuthorityScopeUnresolved,
    AuthorityInsufficientProblemScope,
    AuthorityInsufficientActions,
    AuthorityInsufficientBoundaries,
    PrincipalNotCurrent,
    /// More than one committed principal carries the id an authority requirement names.
    PrincipalAmbiguous,
    ComponentUnbound,
    ComponentBindingAmbiguous,
    BindingContractUnqualified,
    MechanismNone,
    SeparationUnresolved,
    SameActorAnchor,
    ViewInvalid,
    OrganizationIntegrity,
    VantageUnknown,
    VantageUnbound,
    VantageBindingAmbiguous,
    RealizationUnestablished,
    RealizationIncompatible,
    ActorMismatch,
    RealizationDigestMismatch,
    ComponentExecutableMismatch,
    RealizationNotAccepted,
    RealizationAcceptanceAmbiguous,
    /// The committed acceptance of this digest is for a profile other than the candidate's.
    RealizationProfileMismatch,
}

/// One refusal reason: a code and the subject it concerns.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reason {
    pub code: ReasonCode,
    pub subject: String,
}

fn reason(code: ReasonCode, subject: impl Into<String>) -> Reason {
    Reason {
        code,
        subject: subject.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    Passed,
    /// Sorted, deduplicated reasons; never empty.
    Refused(Vec<Reason>),
}

impl CheckOutcome {
    fn from_reasons(reasons: Vec<Reason>) -> Self {
        let set: BTreeSet<Reason> = reasons.into_iter().collect();
        if set.is_empty() {
            CheckOutcome::Passed
        } else {
            CheckOutcome::Refused(set.into_iter().collect())
        }
    }
}

/// One outcome for every [`CheckId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcomes(BTreeMap<CheckId, CheckOutcome>);

impl Outcomes {
    pub fn get(&self, id: CheckId) -> &CheckOutcome {
        &self.0[&id]
    }
    pub fn passed(&self, id: CheckId) -> bool {
        matches!(self.0[&id], CheckOutcome::Passed)
    }
    pub fn all_passed(&self) -> bool {
        self.0.values().all(|o| matches!(o, CheckOutcome::Passed))
    }
    /// The refused check ids, in order.
    pub fn refused(&self) -> Vec<CheckId> {
        self.0
            .iter()
            .filter(|(_, o)| !matches!(o, CheckOutcome::Passed))
            .map(|(id, _)| *id)
            .collect()
    }
    pub fn iter(&self) -> impl Iterator<Item = (CheckId, &CheckOutcome)> {
        self.0.iter().map(|(id, o)| (*id, o))
    }
}

// ---------------------------------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------------------------------

/// The decision domain. Only production exists without the `fixture` feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Production,
    #[cfg(feature = "fixture")]
    Fixture,
}

mod sealed {
    pub trait Sealed {}
}

/// The gate policy. Sealed: [`Production`] and, with the `fixture` feature, [`Fixture`].
pub trait Policy: sealed::Sealed {
    const DOMAIN: Domain;
}

/// Production policy: O4 component bindings, O8 and R3 refuse `binding_contract_unqualified`, and
/// O5 refuses every `none` row (the v1 separation row is `none`). No production admission is
/// reachable.
#[derive(Debug)]
pub enum Production {}
impl sealed::Sealed for Production {}
impl Policy for Production {
    const DOMAIN: Domain = Domain::Production;
}

/// Fixture policy: owner-attested fixture records decide O4 component bindings, O8 and R3
/// positively; the separation `none` row is accepted under the labelled fixture separation
/// attestation (the fixture records O8 decides from). R3 compares declared actor values.
#[cfg(feature = "fixture")]
#[derive(Debug)]
pub enum Fixture {}
#[cfg(feature = "fixture")]
impl sealed::Sealed for Fixture {}
#[cfg(feature = "fixture")]
impl Policy for Fixture {
    const DOMAIN: Domain = Domain::Fixture;
}

// ---------------------------------------------------------------------------------------------
// Request and realization inputs
// ---------------------------------------------------------------------------------------------

/// The exact review subject `{ commit, tree, patchIdentity }`. Bound into the admission; no check
/// in this crate relates it to the problem scope (that mapping rule has no owner, O7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectRef {
    pub commit: String,
    pub tree: String,
    pub patch_identity: String,
}

/// One admission request. The boundary is the reservation's, not the request's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRequest {
    organization_id: String,
    requirements: OrganizationalRequirements,
    candidate_digest: Digest,
    vantage: String,
    subject: SubjectRef,
}

impl OperationRequest {
    /// `candidate_digest` is the caller's claim of the candidate's content digest; O3 recomputes
    /// it from `requirements` (self-consistency only, see [`CheckId::O3`]).
    pub fn new(
        organization_id: impl Into<String>,
        requirements: OrganizationalRequirements,
        candidate_digest: Digest,
        vantage: impl Into<String>,
        subject: SubjectRef,
    ) -> Self {
        OperationRequest {
            organization_id: organization_id.into(),
            requirements,
            candidate_digest,
            vantage: vantage.into(),
            subject,
        }
    }
    pub fn organization_id(&self) -> &str {
        &self.organization_id
    }
    pub fn vantage(&self) -> &str {
        &self.vantage
    }
    pub fn subject(&self) -> &SubjectRef {
        &self.subject
    }
    pub fn requirements_digest(&self) -> Digest {
        self.requirements.requirements_digest()
    }
    pub fn candidate_digest(&self) -> Digest {
        self.candidate_digest
    }
}

/// The realization the host will run, with its **declared** actor identity and installed
/// component digests. Under the fixture policy R3 compares these declared values with the bound
/// ones (owner decision O8: production R3 needs host observation of the spawned process, which
/// does not exist here). Values, not authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredRealization {
    realization_digest: RealizationDigest,
    actor_identity: String,
    installed: BTreeMap<String, ExecutableDigest>,
}

impl DeclaredRealization {
    pub fn declared(
        realization_digest: RealizationDigest,
        actor_identity: impl Into<String>,
        installed: impl IntoIterator<Item = (String, ExecutableDigest)>,
    ) -> Self {
        DeclaredRealization {
            realization_digest,
            actor_identity: actor_identity.into(),
            installed: installed.into_iter().collect(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Pins and the admission
// ---------------------------------------------------------------------------------------------

/// The issuances an admission depends on, all from its one observation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuthorityPins {
    pub principals: BTreeSet<IssuanceId>,
    pub vantage_bindings: BTreeSet<IssuanceId>,
    pub component_bindings: BTreeSet<IssuanceId>,
    pub grants: BTreeSet<IssuanceId>,
    pub decision_scopes: BTreeSet<IssuanceId>,
    pub realization_acceptance: BTreeSet<IssuanceId>,
}

impl AuthorityPins {
    fn all(&self) -> impl Iterator<Item = &IssuanceId> {
        self.principals
            .iter()
            .chain(&self.vantage_bindings)
            .chain(&self.component_bindings)
            .chain(&self.grants)
            .chain(&self.decision_scopes)
            .chain(&self.realization_acceptance)
    }
}

/// The only value that can permit an effect, and only when the consuming host's own regate
/// ([`EffectAdmission::admission_state`] against a fresh observation) also passes: the host still
/// allocates the boundary, records its single use and commits the effect. Binds principal, actor,
/// subject, organization, operation boundary, authority revision with pins, and realization. Not
/// `Clone`, no public constructor; produced only by [`admit`], which no production policy can
/// pass.
#[derive(Debug)]
pub struct EffectAdmission {
    domain: Domain,
    boundary: Boundary,
    revision: ObservationRevision,
    request: OperationRequest,
    realization: DeclaredRealization,
    principal: IssuanceId,
    principal_body: BodyDigest,
    anchor: ActorAnchor,
    realization_profile: String,
    pins: AuthorityPins,
    outcomes: Outcomes,
}

impl EffectAdmission {
    pub fn domain(&self) -> Domain {
        self.domain
    }
    pub fn boundary(&self) -> &Boundary {
        &self.boundary
    }
    pub fn revision(&self) -> ObservationRevision {
        self.revision
    }
    pub fn request(&self) -> &OperationRequest {
        &self.request
    }
    /// The acting vantage's principal issuance.
    pub fn principal(&self) -> IssuanceId {
        self.principal
    }
    pub fn principal_body(&self) -> BodyDigest {
        self.principal_body
    }
    pub fn actor_anchor(&self) -> &ActorAnchor {
        &self.anchor
    }
    pub fn actor_identity(&self) -> &str {
        &self.realization.actor_identity
    }
    pub fn realization_digest(&self) -> &RealizationDigest {
        &self.realization.realization_digest
    }
    pub fn realization_profile(&self) -> &str {
        &self.realization_profile
    }
    pub fn pins(&self) -> &AuthorityPins {
        &self.pins
    }
    /// Every check passed; exposed by id so a host can derive blocker discharges.
    pub fn outcomes(&self) -> &Outcomes {
        &self.outcomes
    }
}

/// Why no admission was issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionRefusal {
    /// The observation is sealed to another boundary: it is not an observation for this
    /// decision, so no check outcome exists (6.1 B12b).
    ObservationBoundaryMismatch { sealed: Boundary, decided: Boundary },
    /// At least one check refused; every outcome is recorded.
    Checks(Outcomes),
}

// ---------------------------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------------------------

struct Evaluation {
    outcomes: Outcomes,
    pins: AuthorityPins,
    acting: Option<(IssuanceId, ActorAnchor)>,
    profile: Option<String>,
}

/// Decide one boundary: Gate O then Gate R over the observation sealed to `reservation`.
///
/// The observation is taken by value: one observation decides at most one admission, and a later
/// point check ([`EffectAdmission::admission_state`]) needs another [`crate::observe_for`].
pub fn admit<P: Policy>(
    reservation: &Reservation,
    observation: SealedObservation,
    request: &OperationRequest,
    realization: &DeclaredRealization,
    now_millis: u64,
) -> Result<EffectAdmission, AdmissionRefusal> {
    if observation.boundary() != reservation.boundary() {
        return Err(AdmissionRefusal::ObservationBoundaryMismatch {
            sealed: observation.boundary().clone(),
            decided: reservation.boundary().clone(),
        });
    }
    let state = observation.state();
    let evaluation = evaluate(P::DOMAIN, state, request, realization, now_millis);
    match (
        evaluation.outcomes.all_passed(),
        evaluation.acting,
        evaluation.profile,
    ) {
        (true, Some((principal, anchor)), Some(profile)) => Ok(EffectAdmission {
            domain: P::DOMAIN,
            boundary: reservation.boundary().clone(),
            revision: state.revision(),
            request: request.clone(),
            realization: realization.clone(),
            principal,
            principal_body: state
                .body_digest(&principal)
                .expect("acting principal resolved in this state"),
            anchor,
            realization_profile: profile,
            pins: evaluation.pins,
            outcomes: evaluation.outcomes,
        }),
        _ => Err(AdmissionRefusal::Checks(evaluation.outcomes)),
    }
}

fn evaluate(
    domain: Domain,
    state: &AuthorityState,
    request: &OperationRequest,
    realization: &DeclaredRealization,
    now: u64,
) -> Evaluation {
    let mut pins = AuthorityPins::default();
    let mut out: BTreeMap<CheckId, Vec<Reason>> = BTreeMap::new();

    // O3: recompute the derivation; exactly one candidate with the admitted digest.
    let derivation = derive(&request.requirements, request.requirements.profile());
    let candidate: Option<&OrganizationCandidate> = match derivation.candidates.as_slice() {
        [one] => Some(one),
        _ => None,
    };
    out.insert(
        CheckId::O3,
        match candidate {
            None => vec![reason(
                ReasonCode::CandidateUnderived,
                format!("{} unresolved", derivation.unresolved.len()),
            )],
            Some(c) if c.content_digest() != request.candidate_digest => {
                vec![reason(
                    ReasonCode::CandidateDigestMismatch,
                    c.content_digest().to_hex(),
                )]
            }
            Some(_) => vec![],
        },
    );

    // O9: organization integrity.
    let internal: Vec<String> = derivation
        .unresolved
        .iter()
        .filter(|u| u.kind == UnresolvedKind::InternalError)
        .map(|u| u.subject.clone())
        .collect();
    let view_error = candidate.and_then(|c| view(c).err().map(|e| e.to_string()));
    out.insert(
        CheckId::O9,
        o9_reasons(candidate.is_some(), view_error, internal),
    );

    let unavailable = || vec![reason(ReasonCode::CandidateUnavailable, "")];
    let mut components: BTreeMap<String, ExecutableDigest> = BTreeMap::new();
    let mut o8_bindings: BTreeMap<String, IssuanceId> = BTreeMap::new();

    match candidate {
        None => {
            for id in [
                CheckId::O4,
                CheckId::O5,
                CheckId::O8,
                CheckId::R1,
                CheckId::R2,
            ] {
                out.insert(id, unavailable());
            }
        }
        Some(c) => {
            let scope = ProblemScopeRef::of(c.problem_scope());
            let mut o4 = o4_authority(state, c, &scope, now, &mut pins);
            o4.extend(o4_components(
                domain,
                state,
                c,
                &scope,
                now,
                &mut pins,
                &mut components,
            ));
            out.insert(CheckId::O4, o4);
            out.insert(CheckId::O5, o5(domain, c));
            out.insert(
                CheckId::O8,
                o8(
                    domain,
                    state,
                    c,
                    &request.organization_id,
                    &mut pins,
                    &mut o8_bindings,
                ),
            );
        }
    }

    // R4: the realization is accepted in this observation.
    let accepted = r4(
        state,
        &realization.realization_digest,
        request.requirements.profile().as_str(),
        &mut pins,
    );
    let (r4_reasons, provides, profile) = match accepted {
        Ok((provides, profile)) => (vec![], Some(provides), Some(profile)),
        Err(r) => (r, None, None),
    };
    out.insert(CheckId::R4, r4_reasons);

    if let Some(c) = candidate {
        let (r1, r2) = r1_r2(c, &request.vantage, provides);
        out.insert(CheckId::R1, r1);
        out.insert(CheckId::R2, r2);
    }

    let (r3, acting) = r3(
        domain,
        state,
        request,
        realization,
        &o8_bindings,
        &components,
        &mut pins,
    );
    out.insert(CheckId::R3, r3);

    let outcomes = Outcomes(
        CheckId::ALL
            .iter()
            .map(|id| {
                let reasons = out.remove(id).expect("every check evaluated");
                (*id, CheckOutcome::from_reasons(reasons))
            })
            .collect(),
    );
    Evaluation {
        outcomes,
        pins,
        acting,
        profile,
    }
}

pub(crate) fn o9_reasons(
    have_candidate: bool,
    view_error: Option<String>,
    internal_errors: Vec<String>,
) -> Vec<Reason> {
    let mut reasons: Vec<Reason> = internal_errors
        .into_iter()
        .map(|subject| reason(ReasonCode::OrganizationIntegrity, subject))
        .collect();
    if let Some(error) = view_error {
        reasons.push(reason(ReasonCode::ViewInvalid, error));
    }
    if !have_candidate && reasons.is_empty() {
        reasons.push(reason(ReasonCode::CandidateUnavailable, ""));
    }
    reasons
}

// --- record lookups --------------------------------------------------------------------------

fn not_current(subject: &str, resolution: &Resolution<'_>) -> Reason {
    let detail = match resolution {
        Resolution::Revoked => "revoked",
        Resolution::Superseded { .. } => "superseded",
        Resolution::Missing => "missing",
        Resolution::Committed(_) => "wrong record kind",
    };
    reason(
        ReasonCode::PrincipalNotCurrent,
        format!("{subject}: {detail}"),
    )
}

#[cfg_attr(not(feature = "fixture"), allow(dead_code))]
fn committed_principal<'s>(
    state: &'s AuthorityState,
    id: &IssuanceId,
) -> Result<&'s Principal, Resolution<'s>> {
    match state.resolve(id) {
        Resolution::Committed(Record::Principal(p)) => Ok(p),
        other => Err(other),
    }
}

#[cfg_attr(not(feature = "fixture"), allow(dead_code))]
/// Exactly one committed principal with this principal id.
fn principal_named<'s>(
    state: &'s AuthorityState,
    name: &str,
) -> Option<(IssuanceId, &'s Principal)> {
    let found: Vec<_> = state
        .entries()
        .filter(|(_, e)| e.status == IssuanceStatus::Committed)
        .filter_map(|(id, e)| match &e.record {
            Record::Principal(p) if p.principal_id == name => Some((*id, p)),
            _ => None,
        })
        .collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

#[cfg_attr(not(feature = "fixture"), allow(dead_code))]
/// The committed vantage-actor bindings for `(organization, vantage)`.
fn vantage_bindings<'s>(
    state: &'s AuthorityState,
    organization: &str,
    vantage: &str,
) -> Vec<(IssuanceId, &'s VantageActorBinding)> {
    state
        .entries()
        .filter(|(_, e)| e.status == IssuanceStatus::Committed)
        .filter_map(|(id, e)| match &e.record {
            Record::VantageActorBinding(b)
                if b.organization_id == organization && b.vantage == vantage =>
            {
                Some((*id, b))
            }
            _ => None,
        })
        .collect()
}

#[cfg_attr(not(feature = "fixture"), allow(dead_code))]
/// Which principal a grant must name.
enum PrincipalMatch<'a> {
    /// A principal record with this principal id (an authority requirement names principals by id).
    Named(&'a str),
    /// Exactly this principal issuance (a component binding's principal).
    Issuance(IssuanceId),
}

/// Grants covering `permission` for the principal, per rev6 §3.3/§18.4 with 6.1 F4. On success
/// pins the covering grants, their decision scopes and principals.
#[allow(clippy::too_many_arguments)]
fn covered(
    state: &AuthorityState,
    who: PrincipalMatch<'_>,
    permission: &str,
    boundaries: Option<&BTreeSet<String>>,
    scope: &ProblemScopeRef,
    now: u64,
    subject: &str,
    pins: &mut AuthorityPins,
) -> Vec<Reason> {
    // Same resolution as O8's `principal_named`: a name that matches more than one committed
    // principal resolves to none, so no grant to either covers it.
    if let PrincipalMatch::Named(name) = &who
        && state
            .entries()
            .filter(|(_, e)| e.status == IssuanceStatus::Committed)
            .filter(|(_, e)| matches!(&e.record, Record::Principal(p) if p.principal_id == *name))
            .count()
            > 1
    {
        return vec![reason(ReasonCode::PrincipalAmbiguous, subject)];
    }
    let mut reasons = Vec::new();
    let mut covering = Vec::new();
    let mut considered = false;
    for (gid, entry) in state.entries() {
        let Record::Grant(g) = &entry.record else {
            continue;
        };
        if !g.permissions.contains(permission) {
            continue;
        }
        let principal = state.resolve(&g.principal);
        let principal_matches = match (&who, &principal) {
            (PrincipalMatch::Issuance(id), _) => *id == g.principal,
            (PrincipalMatch::Named(name), _) => state.entries().any(|(pid, e)| {
                *pid == g.principal
                    && matches!(&e.record, Record::Principal(p) if p.principal_id == *name)
            }),
        };
        if !principal_matches {
            continue;
        }
        considered = true;
        match entry.status {
            IssuanceStatus::Revoked => {
                reasons.push(reason(ReasonCode::AuthorityRevoked, subject));
                continue;
            }
            IssuanceStatus::Superseded { .. } => {
                reasons.push(reason(ReasonCode::AuthoritySuperseded, subject));
                continue;
            }
            IssuanceStatus::Committed => {}
        }
        if !matches!(principal, Resolution::Committed(Record::Principal(_))) {
            reasons.push(not_current(subject, &principal));
            continue;
        }
        let mut grant_reasons = Vec::new();
        if g.expires_at_millis.is_some_and(|at| at <= now) {
            grant_reasons.push(reason(ReasonCode::AuthorityExpired, subject));
        }
        if &g.problem_scope != scope {
            grant_reasons.push(reason(
                ReasonCode::AuthorityInsufficientProblemScope,
                subject,
            ));
        }
        let scopes: Vec<_> = state
            .entries()
            .filter(|(_, e)| e.status == IssuanceStatus::Committed)
            .filter_map(|(id, e)| match &e.record {
                Record::DecisionScope(d)
                    if (d.id.as_str(), d.revision)
                        == (g.decision_scope.0.as_str(), g.decision_scope.1) =>
                {
                    Some((*id, d))
                }
                _ => None,
            })
            .collect();
        let ds = match scopes.as_slice() {
            [one] => Some(*one),
            _ => {
                grant_reasons.push(reason(ReasonCode::AuthorityScopeUnresolved, subject));
                None
            }
        };
        if let Some((_, d)) = ds {
            if &d.problem_scope != scope {
                grant_reasons.push(reason(
                    ReasonCode::AuthorityInsufficientProblemScope,
                    subject,
                ));
            }
            if !d.actions.contains(permission) {
                grant_reasons.push(reason(ReasonCode::AuthorityInsufficientActions, subject));
            }
            if let (Some(required), Some(allowed)) = (boundaries, &d.boundaries)
                && !required.is_subset(allowed)
            {
                grant_reasons.push(reason(ReasonCode::AuthorityInsufficientBoundaries, subject));
            }
        }
        if grant_reasons.is_empty() {
            let (dsid, _) = ds.expect("resolved");
            covering.push((*gid, dsid, g.principal));
        } else {
            reasons.extend(grant_reasons);
        }
    }
    if !covering.is_empty() {
        for (gid, dsid, pid) in covering {
            pins.grants.insert(gid);
            pins.decision_scopes.insert(dsid);
            pins.principals.insert(pid);
        }
        return vec![];
    }
    if !considered {
        return vec![reason(ReasonCode::AuthorityMissing, subject)];
    }
    reasons
}

// --- Gate O ----------------------------------------------------------------------------------

fn o4_authority(
    state: &AuthorityState,
    candidate: &OrganizationCandidate,
    scope: &ProblemScopeRef,
    now: u64,
    pins: &mut AuthorityPins,
) -> Vec<Reason> {
    let mut reasons = Vec::new();
    for requirement in candidate.authority_requirements() {
        let boundaries: Option<BTreeSet<String>> = requirement
            .boundaries
            .as_ref()
            .map(|b| b.iter().map(|id| id.as_str().to_owned()).collect());
        reasons.extend(covered(
            state,
            PrincipalMatch::Named(requirement.principal.as_str()),
            requirement.permission.as_str(),
            boundaries.as_ref(),
            scope,
            now,
            &format!("authority:{}", requirement.id.as_str()),
            pins,
        ));
    }
    reasons
}

#[cfg_attr(not(feature = "fixture"), allow(unused_variables))]
fn o4_components(
    domain: Domain,
    state: &AuthorityState,
    candidate: &OrganizationCandidate,
    scope: &ProblemScopeRef,
    now: u64,
    pins: &mut AuthorityPins,
    components: &mut BTreeMap<String, ExecutableDigest>,
) -> Vec<Reason> {
    let requirements = candidate.component_binding_requirements();
    match domain {
        Domain::Production => requirements
            .iter()
            .map(|r| {
                reason(
                    ReasonCode::BindingContractUnqualified,
                    format!("component:{}", r.component),
                )
            })
            .collect(),
        #[cfg(feature = "fixture")]
        Domain::Fixture => {
            let mut reasons = Vec::new();
            for requirement in requirements {
                let subject = format!("component:{}", requirement.component);
                let bindings: Vec<_> = state
                    .entries()
                    .filter(|(_, e)| e.status == IssuanceStatus::Committed)
                    .filter_map(|(id, e)| match &e.record {
                        Record::ComponentBinding(b)
                            if b.component == requirement.component
                                && &b.problem_scope == scope =>
                        {
                            Some((*id, b))
                        }
                        _ => None,
                    })
                    .collect();
                let (bid, binding) = match bindings.as_slice() {
                    [] => {
                        reasons.push(reason(ReasonCode::ComponentUnbound, subject));
                        continue;
                    }
                    [one] => *one,
                    _ => {
                        reasons.push(reason(ReasonCode::ComponentBindingAmbiguous, subject));
                        continue;
                    }
                };
                if let Err(resolution) = committed_principal(state, &binding.principal) {
                    reasons.push(not_current(&subject, &resolution));
                    continue;
                }
                let mut binding_reasons = Vec::new();
                for permission in &requirement.permissions {
                    binding_reasons.extend(covered(
                        state,
                        PrincipalMatch::Issuance(binding.principal),
                        permission,
                        None,
                        scope,
                        now,
                        &format!("{subject}/{permission}"),
                        pins,
                    ));
                }
                if binding_reasons.is_empty() {
                    pins.component_bindings.insert(bid);
                    pins.principals.insert(binding.principal);
                    components.insert(binding.component.clone(), binding.installed_digest.clone());
                }
                reasons.extend(binding_reasons);
            }
            reasons
        }
    }
}

#[cfg_attr(not(feature = "fixture"), allow(unused_variables))]
fn o5(domain: Domain, candidate: &OrganizationCandidate) -> Vec<Reason> {
    enforceability(candidate, candidate.profile())
        .rows
        .iter()
        .filter(|row| row.status == EnforceStatus::None)
        .filter(|row| match domain {
            Domain::Production => true,
            // Fixture separation attestation: the separation row is decided by O8 over fixture
            // records instead of the production `none` row. No other row is attested.
            #[cfg(feature = "fixture")]
            Domain::Fixture => row.property != "separation",
        })
        .map(|row| {
            reason(
                ReasonCode::MechanismNone,
                format!("{}/{}", row.scope, row.property),
            )
        })
        .collect()
}

#[cfg_attr(not(feature = "fixture"), allow(unused_variables))]
fn o8(
    domain: Domain,
    state: &AuthorityState,
    candidate: &OrganizationCandidate,
    organization: &str,
    pins: &mut AuthorityPins,
    resolved: &mut BTreeMap<String, IssuanceId>,
) -> Vec<Reason> {
    let requirements = candidate.separation_requirements();
    match domain {
        Domain::Production => requirements
            .iter()
            .map(|r| {
                reason(
                    ReasonCode::BindingContractUnqualified,
                    format!("separation:{}", r.condition.as_str()),
                )
            })
            .collect(),
        #[cfg(feature = "fixture")]
        Domain::Fixture => {
            let mut reasons = Vec::new();
            for requirement in requirements {
                let condition = requirement.condition.as_str();
                let acceptor = requirement
                    .acceptor
                    .strip_prefix("principal:")
                    .and_then(|name| principal_named(state, name));
                let Some((acceptor_id, acceptor)) = acceptor else {
                    reasons.push(reason(
                        ReasonCode::SeparationUnresolved,
                        format!("{condition}/{}", requirement.acceptor),
                    ));
                    continue;
                };
                let mut separation_reasons = Vec::new();
                let mut separation_pins = Vec::new();
                for vantage in &requirement.vantages {
                    let subject = format!("{condition}/{vantage}");
                    let bindings = vantage_bindings(state, organization, vantage);
                    let [(bid, binding)] = bindings.as_slice() else {
                        separation_reasons.push(reason(ReasonCode::SeparationUnresolved, subject));
                        continue;
                    };
                    let Ok(principal) = committed_principal(state, &binding.principal) else {
                        separation_reasons.push(reason(ReasonCode::SeparationUnresolved, subject));
                        continue;
                    };
                    if principal.anchor == acceptor.anchor {
                        separation_reasons.push(reason(ReasonCode::SameActorAnchor, subject));
                        continue;
                    }
                    separation_pins.push((vantage.clone(), *bid, binding.principal));
                }
                if separation_reasons.is_empty() {
                    pins.principals.insert(acceptor_id);
                    for (vantage, bid, pid) in separation_pins {
                        pins.vantage_bindings.insert(bid);
                        pins.principals.insert(pid);
                        resolved.insert(vantage, bid);
                    }
                }
                reasons.extend(separation_reasons);
            }
            reasons
        }
    }
}

// --- Gate R ----------------------------------------------------------------------------------

/// R4: exactly one committed acceptance of this digest, stated for the candidate's profile.
fn r4(
    state: &AuthorityState,
    digest: &RealizationDigest,
    profile: &str,
    pins: &mut AuthorityPins,
) -> Result<(RealizationProperties, String), Vec<Reason>> {
    let accepted: Vec<_> = state
        .entries()
        .filter(|(_, e)| e.status == IssuanceStatus::Committed)
        .filter_map(|(id, e)| match &e.record {
            Record::RealizationAcceptance(r) if &r.realization_digest == digest => Some((*id, r)),
            _ => None,
        })
        .collect();
    match accepted.as_slice() {
        [] => Err(vec![reason(
            ReasonCode::RealizationNotAccepted,
            digest.as_str(),
        )]),
        [(_, record)] if record.profile != profile => Err(vec![reason(
            ReasonCode::RealizationProfileMismatch,
            digest.as_str(),
        )]),
        [(id, record)] => {
            pins.realization_acceptance.insert(*id);
            Ok((record.provides.clone(), record.profile.clone()))
        }
        _ => Err(vec![reason(
            ReasonCode::RealizationAcceptanceAmbiguous,
            digest.as_str(),
        )]),
    }
}

/// Class-level satisfaction of one constraint by the accepted classes (no mechanism lookup: a
/// `none` mechanism row is O5's refusal).
fn constraint_reasons(
    subject: &str,
    constraint: &RealizationConstraint,
    provides: &RealizationProperties,
) -> Option<Reason> {
    let dimension = constraint.dimension;
    let class: Option<&RealizationClass> =
        provides.classes.iter().find(|c| c.dimension() == dimension);
    let subject = format!("{subject}/{}", dimension.as_str());
    match class {
        None => Some(reason(ReasonCode::RealizationUnestablished, subject)),
        Some(c) if !constraint.allowed.contains(c) => {
            Some(reason(ReasonCode::RealizationIncompatible, subject))
        }
        Some(_) => None,
    }
}

fn r1_r2(
    candidate: &OrganizationCandidate,
    vantage: &str,
    provides: Option<RealizationProperties>,
) -> (Vec<Reason>, Vec<Reason>) {
    let Some(provides) = provides else {
        let none = || {
            vec![reason(
                ReasonCode::RealizationUnestablished,
                "no accepted realization",
            )]
        };
        return (none(), none());
    };
    let r1 = candidate
        .conditions()
        .iter()
        .filter(|c| c.vantage == vantage)
        .flat_map(|c| {
            c.realization_constraints
                .iter()
                .filter_map(|k| constraint_reasons(c.id.as_str(), k, &provides))
                .collect::<Vec<_>>()
        })
        .collect();
    let Some(requirement) = candidate
        .vantage_requirements()
        .iter()
        .find(|v| v.vantage == vantage)
    else {
        return (r1, vec![reason(ReasonCode::VantageUnknown, vantage)]);
    };
    let mut r2: Vec<Reason> = requirement
        .effective_constraints
        .iter()
        .filter_map(|k| constraint_reasons(vantage, k, &provides))
        .collect();
    let effective = &requirement.effective;
    let properties: [(&str, &str, Option<&str>); 5] = [
        (
            "continuity",
            effective.continuity.as_str(),
            provides.continuity.map(|v| v.as_str()),
        ),
        (
            "mutation",
            effective.mutation.as_str(),
            provides.mutation.map(|v| v.as_str()),
        ),
        (
            "integrity",
            effective.integrity.as_str(),
            provides.integrity.map(|v| v.as_str()),
        ),
        (
            "evidenceCustody",
            effective.evidence_custody.as_str(),
            provides.evidence_custody.map(|v| v.as_str()),
        ),
        (
            "observerClass",
            effective.observer_class.as_str(),
            provides.observer_class.map(|v| v.as_str()),
        ),
    ];
    for (property, required, provided) in properties {
        let subject = format!("{vantage}/{property}");
        match provided {
            None => r2.push(reason(ReasonCode::RealizationUnestablished, subject)),
            Some(value) if value != required => {
                r2.push(reason(ReasonCode::RealizationIncompatible, subject))
            }
            Some(_) => {}
        }
    }
    (r1, r2)
}

#[cfg_attr(not(feature = "fixture"), allow(unused_variables))]
#[allow(clippy::too_many_arguments)]
fn r3(
    domain: Domain,
    state: &AuthorityState,
    request: &OperationRequest,
    realization: &DeclaredRealization,
    o8_bindings: &BTreeMap<String, IssuanceId>,
    components: &BTreeMap<String, ExecutableDigest>,
    pins: &mut AuthorityPins,
) -> (Vec<Reason>, Option<(IssuanceId, ActorAnchor)>) {
    let vantage = request.vantage.as_str();
    match domain {
        // Owner decision O8: production R3 compares a host observation of the spawned process
        // with the attested binding. No such mechanism exists, so production refuses.
        Domain::Production => (
            vec![reason(
                ReasonCode::BindingContractUnqualified,
                format!("actor:{vantage}"),
            )],
            None,
        ),
        #[cfg(feature = "fixture")]
        Domain::Fixture => {
            let mut reasons = Vec::new();
            // The binding O8 checked for this vantage, if any; otherwise the one committed binding.
            let found = vantage_bindings(state, &request.organization_id, vantage);
            let chosen = match o8_bindings.get(vantage) {
                Some(id) => found.iter().find(|(bid, _)| bid == id).copied(),
                None => match found.as_slice() {
                    [one] => Some(*one),
                    [] => None,
                    _ => {
                        reasons.push(reason(ReasonCode::VantageBindingAmbiguous, vantage));
                        return (reasons, None);
                    }
                },
            };
            let Some((bid, binding)) = chosen else {
                reasons.push(reason(ReasonCode::VantageUnbound, vantage));
                return (reasons, None);
            };
            let principal = committed_principal(state, &binding.principal);
            if let Err(resolution) = &principal {
                reasons.push(not_current(vantage, resolution));
            }
            if binding.actor.actor_identity != realization.actor_identity {
                reasons.push(reason(ReasonCode::ActorMismatch, vantage));
            }
            if binding.actor.realization_digest != realization.realization_digest {
                reasons.push(reason(ReasonCode::RealizationDigestMismatch, vantage));
            }
            for (component, installed) in components {
                if realization.installed.get(component) != Some(installed) {
                    reasons.push(reason(
                        ReasonCode::ComponentExecutableMismatch,
                        component.clone(),
                    ));
                }
            }
            match principal {
                Ok(p) if reasons.is_empty() => {
                    pins.vantage_bindings.insert(bid);
                    pins.principals.insert(binding.principal);
                    (reasons, Some((binding.principal, p.anchor.clone())))
                }
                _ => (reasons, None),
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Admission states
// ---------------------------------------------------------------------------------------------

/// Why an admission is stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaleReason {
    /// A later epoch of the same operation and boundary kind was reserved.
    BoundarySuperseded { by: Boundary },
    /// The observation is sealed to another boundary.
    ObservationBoundaryMismatch { sealed: Boundary },
    /// A pinned issuance was superseded by a later issuance.
    IssuanceSuperseded(Vec<IssuanceId>),
}

/// Why the state is unknown. Fail closed: no advance, no consumption, no retry by uncertainty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownReason {
    /// No observation.
    NoObservation(String),
    /// The source reported a lower revision than the admission's (obligation 2 violated).
    RevisionRegressed,
    /// The source reported a different state at the admission's revision (obligation 4 violated).
    RevisionConflict,
    /// A pinned issuance is missing at a later revision (obligation 2 violated).
    IssuanceRetracted(Vec<IssuanceId>),
}

/// The state of an admission at a point check against a new observation.
///
/// | State | Consequence |
/// |---|---|
/// | `Current` | Exactly one effect at the admission's boundary may proceed; the host records its single use. |
/// | `Stale` | No new effect. An already-started step may finish, but its outcome is held, not consumed. A new boundary needs a new admission. |
/// | `Revoked` | As stale, and no alternative is selected: failure of an authorized candidate does not authorize a previously unauthorized alternative. |
/// | `Unknown` | Fail closed: no advance, no consumption, no retry by uncertainty; reconcile the same boundary identity, never mint another. |
/// | `Refused` | Regating the same request against the new observation refused; terminal for this request; every outcome recorded. |
///
/// Unknown, revoked and refused are not negative results about the work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionState {
    Current { revision: ObservationRevision },
    Stale(StaleReason),
    Revoked(Vec<IssuanceId>),
    Unknown(UnknownReason),
    Refused(Outcomes),
}

impl AdmissionState {
    /// Only a current admission permits its effect.
    pub fn permits_effect(&self) -> bool {
        matches!(self, AdmissionState::Current { .. })
    }
}

impl EffectAdmission {
    /// Point check against a new observation (6.1 F3 step 4). [`admit`] consumes the observation
    /// it decides on, so the observation passed here comes from another [`crate::observe_for`]. It
    /// must be sealed to this admission's boundary; every pin must still be committed; if the revision moved or time
    /// passed, Gate O and Gate R re-run with the same request and must pass.
    pub fn admission_state(
        &self,
        observation: &Result<SealedObservation, Unobserved>,
        now_millis: u64,
    ) -> AdmissionState {
        let observation = match observation {
            Err(unobserved) => {
                return AdmissionState::Unknown(UnknownReason::NoObservation(
                    unobserved.detail.clone(),
                ));
            }
            Ok(o) => o,
        };
        if observation.boundary() != &self.boundary {
            return AdmissionState::Stale(if observation.boundary().supersedes(&self.boundary) {
                StaleReason::BoundarySuperseded {
                    by: observation.boundary().clone(),
                }
            } else {
                StaleReason::ObservationBoundaryMismatch {
                    sealed: observation.boundary().clone(),
                }
            });
        }
        let state = observation.state();
        let revision = state.revision();
        if revision.sequence() < self.revision.sequence() {
            return AdmissionState::Unknown(UnknownReason::RevisionRegressed);
        }
        if revision.sequence() == self.revision.sequence() && revision != self.revision {
            return AdmissionState::Unknown(UnknownReason::RevisionConflict);
        }
        let (mut retracted, mut revoked, mut superseded) = (Vec::new(), Vec::new(), Vec::new());
        for id in self.pins.all() {
            match state.resolve(id) {
                Resolution::Committed(_) => {}
                Resolution::Missing => retracted.push(*id),
                Resolution::Revoked => revoked.push(*id),
                Resolution::Superseded { .. } => superseded.push(*id),
            }
        }
        if !retracted.is_empty() {
            return AdmissionState::Unknown(UnknownReason::IssuanceRetracted(retracted));
        }
        if !revoked.is_empty() {
            return AdmissionState::Revoked(revoked);
        }
        if !superseded.is_empty() {
            return AdmissionState::Stale(StaleReason::IssuanceSuperseded(superseded));
        }
        let regate = evaluate(
            self.domain,
            state,
            &self.request,
            &self.realization,
            now_millis,
        );
        if regate.outcomes.all_passed() {
            AdmissionState::Current { revision }
        } else {
            AdmissionState::Refused(regate.outcomes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o9_refuses_internal_errors_and_view_errors_as_integrity() {
        // O9's negatives are unreachable from valid requirements (derive and view only fail on
        // their own defects), so the check function is exercised directly. Oracle G1 at the
        // function level.
        assert!(o9_reasons(true, None, vec![]).is_empty());
        assert_eq!(
            o9_reasons(true, None, vec!["consumerClass".into()]),
            vec![reason(ReasonCode::OrganizationIntegrity, "consumerClass")]
        );
        assert_eq!(
            o9_reasons(true, Some("bad view".into()), vec![]),
            vec![reason(ReasonCode::ViewInvalid, "bad view")]
        );
        assert_eq!(
            o9_reasons(false, None, vec!["x".into()]),
            vec![reason(ReasonCode::OrganizationIntegrity, "x")]
        );
        assert_eq!(
            o9_reasons(false, None, vec![]),
            vec![reason(ReasonCode::CandidateUnavailable, "")]
        );
    }
}
