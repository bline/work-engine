//! R2-S refusal seam, pure part (feature `fixture`, no native child): the closed blocker kinds,
//! the host's total discharge function with its labelled assumption set, and the evaluation's
//! refusals. The native boundaries (spawn, consumption) are exercised in native_vertical.rs.
mod common;

use common::world::host_request;
use common::*;
use work_engine_review_delivery::gate::{
    ASSUMED_DISCHARGE_LABEL, Boundary, Discharge, GateRefusal, assess, assess_preparation,
    discharge,
};
use work_engine_review_preparation::{
    Blocker, BlockerClass, BlockerKind, MutationClaim, RealizationClaim, prepare,
};

/// Invariant 4: discharge is total over the closed kinds; the assumption set is exactly the root
/// ruling; realization checks are never assumable.
#[test]
fn discharge_is_total_and_realization_checks_are_never_assumable() {
    let table: Vec<(BlockerKind, Discharge)> = BlockerKind::ALL
        .iter()
        .map(|k| (*k, discharge(*k)))
        .collect();
    assert_eq!(
        table,
        [
            (
                BlockerKind::ScopeDigestToSubject,
                Discharge::AssumedDischarge
            ),
            (
                BlockerKind::ProblemScopeToEpisodeIdentity,
                Discharge::AssumedDischarge
            ),
            (
                BlockerKind::ConsumptionEvidenceMapping,
                Discharge::AssumedDischarge
            ),
            (BlockerKind::Separation, Discharge::AssumedDischarge),
            (BlockerKind::ObserverIdentity, Discharge::AssumedDischarge),
            (
                BlockerKind::RealizationAdmission,
                Discharge::AssumedDischarge
            ),
            (BlockerKind::ComponentBinding, Discharge::AssumedDischarge),
            (
                BlockerKind::AuthorityRequirement,
                Discharge::AssumedDischarge
            ),
            (BlockerKind::RealizationCheck, Discharge::Undischarged),
        ]
    );
    assert_eq!(
        ASSUMED_DISCHARGE_LABEL,
        "fixture_assumed_discharge_not_admission"
    );
}

/// The assumption world under the host's description: no realization check blocker remains, and
/// every blocker passes only as a labelled assumption.
#[test]
fn the_assumption_world_passes_only_by_labelled_assumed_discharges() {
    let prep = prepare(&assumption_candidate(), &host_request()).unwrap();
    let evaluation = prep.realization_evaluation.as_ref().expect("host-derived");
    assert!(evaluation.conditions.iter().all(|c| {
        c.checks
            .iter()
            .all(|k| !k.property.starts_with("realization:"))
    }));
    assert!(
        prep.blockers
            .iter()
            .all(|b| b.kind != BlockerKind::RealizationCheck)
    );
    let passed = assess_preparation(Boundary::BeforeSpawn, &prep).expect("assumed away");
    assert_eq!(passed.label(), ASSUMED_DISCHARGE_LABEL);
    assert_eq!(passed.boundary(), Boundary::BeforeSpawn);
    assert_eq!(passed.assumed().len(), prep.blockers.len());
    assert!(!passed.assumed().is_empty());
}

/// The frozen world under the host's description: four known incompatibilities, refused.
#[test]
fn the_frozen_world_is_a_known_incompatibility() {
    let prep = prepare(&candidate(), &host_request()).unwrap();
    match assess_preparation(Boundary::BeforeSpawn, &prep) {
        Err(GateRefusal::KnownIncompatibility { ids }) => assert_eq!(
            ids,
            [
                format!("realization:{COND_BUILDER}:realization:execution_confinement"),
                format!("realization:{COND_BUILDER}:realization:provider_family"),
                format!("realization:{COND_CAMPAIGN}:realization:execution_confinement"),
                format!("realization:{COND_CAMPAIGN}:realization:provider_family"),
            ]
        ),
        other => panic!("expected a known incompatibility, got {other:?}"),
    }
}

/// An unestablished realization check (no incompatibility anywhere) still refuses: its kind has no
/// discharge.
#[test]
fn an_unestablished_realization_check_has_no_discharge_and_refuses() {
    let unstated = RealizationClaim {
        mutation: MutationClaim::NotEstablished,
        ..host_request().realization.unwrap()
    };
    let prep = prepare(&assumption_candidate(), &request(Some(unstated))).unwrap();
    assert!(
        prep.blockers
            .iter()
            .all(|b| b.class == BlockerClass::Unestablished)
    );
    for boundary in [Boundary::BeforeSpawn, Boundary::BeforeConsumption] {
        match assess_preparation(boundary, &prep) {
            Err(GateRefusal::Undischarged { ids }) => assert_eq!(
                ids,
                [
                    format!("realization:{COND_BUILDER}:mutation"),
                    format!("realization:{COND_CAMPAIGN}:mutation"),
                ]
            ),
            other => panic!("{boundary:?}: expected undischarged, got {other:?}"),
        }
    }
}

/// A blocker whose kind is not the one its id names refuses: an unknown id family, or a
/// realization check relabelled with an assumable kind.
#[test]
fn an_unknown_or_mislabelled_blocker_kind_refuses() {
    let base = prepare(&assumption_candidate(), &host_request()).unwrap();
    let unknown = Blocker {
        id: "novel-family:x".into(),
        kind: BlockerKind::ScopeDigestToSubject,
        class: BlockerClass::Unestablished,
        scope: "x".into(),
        requirement: "x".into(),
        basis: "x".into(),
    };
    let relabelled = Blocker {
        id: format!("realization:{COND_BUILDER}:mutation"),
        kind: BlockerKind::Separation,
        ..unknown.clone()
    };
    for (extra, named) in [
        (unknown, None),
        (relabelled, Some(BlockerKind::RealizationCheck)),
    ] {
        let mut blockers = base.blockers.clone();
        let id = extra.id.clone();
        let kind = extra.kind;
        blockers.push(extra);
        assert_eq!(
            assess(Boundary::BeforeSpawn, &blockers),
            Err(GateRefusal::KindMismatch { id, kind, named })
        );
    }
}
