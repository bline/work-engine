//! R2-S refusal seam with **assumed** discharges (feature `fixture`). Mints no authority.
//!
//! The delivery host evaluates the canonical preparation's blockers at two effect boundaries:
//! before any native child is spawned ([`Boundary::BeforeSpawn`]) and immediately before the
//! existing consumer runs ([`Boundary::BeforeConsumption`]). Each evaluation refuses
//!
//! 1. a blocker whose `kind` is not the kind its id family names (an unknown or mislabelled
//!    blocker: [`GateRefusal::KindMismatch`]);
//! 2. any [`BlockerClass::KnownIncompatibility`], whatever its kind;
//! 3. any blocker whose kind has no discharge ([`Discharge::Undischarged`]).
//!
//! [`discharge`] is the host's **total** function from [`BlockerKind`] to a discharge. It is
//! derived from the kind alone: no caller supplies, extends or overrides it, and the only
//! discharge it yields in this slice is [`Discharge::AssumedDischarge`], a labelled fixture
//! assumption with the standing of a `FixtureDeclaration` ([`ASSUMED_DISCHARGE_LABEL`]). An
//! assumed discharge is not an admission, establishes no mapping and is not evidence; passing this
//! gate means only that every remaining blocker is of a kind this fixture vertical assumes away.
//! Realization check blockers (`realization:<condition>:<property>`) are never assumable.
//!
//! Production admission is not reachable through this module: it exists only with the `fixture`
//! feature and grants nothing.

use work_engine_review_preparation::{Blocker, BlockerClass, BlockerKind, ReviewPreparation};

/// The label every assumed discharge carries. Fixture standing only; never an admission.
pub const ASSUMED_DISCHARGE_LABEL: &str = "fixture_assumed_discharge_not_admission";

/// What discharges one blocker kind on the fixture vertical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discharge {
    /// A labelled fixture assumption (standing of `FixtureDeclaration`): the kind is assumed
    /// discharged for the synthetic vertical only. Nothing is established or admitted.
    AssumedDischarge,
    /// Nothing discharges the kind; a blocker of this kind always refuses.
    Undischarged,
}

/// The host-owned total discharge function. Root ruling for R2-S: the three mapping rules,
/// separation, observer identity, component binding, authority requirement and realization
/// admission are assumable; a realization check is never assumable.
pub fn discharge(kind: BlockerKind) -> Discharge {
    match kind {
        BlockerKind::ScopeDigestToSubject
        | BlockerKind::ProblemScopeToEpisodeIdentity
        | BlockerKind::ConsumptionEvidenceMapping
        | BlockerKind::Separation
        | BlockerKind::ObserverIdentity
        | BlockerKind::ComponentBinding
        | BlockerKind::AuthorityRequirement
        | BlockerKind::RealizationAdmission => Discharge::AssumedDischarge,
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
}

/// A passing evaluation: which blockers were assumed discharged, at which boundary. Constructible
/// only by [`assess`]. It is not an admission and authorizes nothing outside this crate's fixture
/// runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssumedDischarges {
    boundary: Boundary,
    assumed: Vec<(String, BlockerKind)>,
}

impl AssumedDischarges {
    pub fn label(&self) -> &'static str {
        ASSUMED_DISCHARGE_LABEL
    }
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    /// Every blocker id assumed discharged, with its kind, in preparation order.
    pub fn assumed(&self) -> &[(String, BlockerKind)] {
        &self.assumed
    }
}

/// The pure evaluation over one blocker list. Refuses in the order of the module docs.
pub fn assess(boundary: Boundary, blockers: &[Blocker]) -> Result<AssumedDischarges, GateRefusal> {
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
    Ok(AssumedDischarges {
        boundary,
        assumed: blockers.iter().map(|b| (b.id.clone(), b.kind)).collect(),
    })
}

/// [`assess`] over a whole preparation's blockers.
pub fn assess_preparation(
    boundary: Boundary,
    preparation: &ReviewPreparation,
) -> Result<AssumedDischarges, GateRefusal> {
    assess(boundary, &preparation.blockers)
}
