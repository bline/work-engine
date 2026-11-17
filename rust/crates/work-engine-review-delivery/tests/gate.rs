//! Delivery gate, pure part (feature `fixture`, no native child): the closed blocker kinds, the
//! host's total discharge function (admitted kinds keyed on S-A check ids, labelled assumed
//! mapping kinds, never-dischargeable realization checks), the screening refusals, and the
//! host-held admission's regate (fresh observation, finality, Unknown). The native boundaries
//! (spawn, consumption) are exercised in native_vertical.rs.
mod common;

use common::authority::{
    ACTOR, FIXED, ORG, VANTAGE, authority_world, boundary, declared, host_digest, operation_request,
};
use common::world::host_request;
use common::*;
use work_engine_organization_authority::fixture::FixtureSource;
use work_engine_organization_authority::records::Record;
use work_engine_organization_authority::{
    AdmissionState, BoundaryKind, CheckId, Domain, StaleReason, UnknownReason, observe_for, reserve,
};
use work_engine_review_delivery::gate::{
    ASSUMED_DISCHARGE_LABEL, Discharge, GateRefusal, HeldAdmission, discharge, screen,
};
use work_engine_review_preparation::{
    Blocker, BlockerClass, BlockerKind, MutationClaim, RealizationClaim, prepare,
};

/// Invariant 1: discharge is total over the closed kinds; the admitted kinds name exactly the S-B
/// check ids; the three mapping kinds stay labelled assumptions; realization checks are never
/// dischargeable.
#[test]
fn discharge_is_total_admitted_kinds_name_their_check_ids() {
    let table: Vec<(BlockerKind, Discharge)> = BlockerKind::ALL
        .iter()
        .map(|k| (*k, discharge(*k)))
        .collect();
    use CheckId::*;
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
            (BlockerKind::Separation, Discharge::Admitted(&[O8, R3])),
            (
                BlockerKind::ObserverIdentity,
                Discharge::Admitted(&[R2, R3])
            ),
            (
                BlockerKind::RealizationAdmission,
                Discharge::Admitted(&[R1, R2, R4])
            ),
            (
                BlockerKind::ComponentBinding,
                Discharge::Admitted(&[O4, R3])
            ),
            (
                BlockerKind::AuthorityRequirement,
                Discharge::Admitted(&[O4])
            ),
            (BlockerKind::RealizationCheck, Discharge::Undischarged),
        ]
    );
    assert_eq!(
        ASSUMED_DISCHARGE_LABEL,
        "fixture_assumed_discharge_not_admission"
    );
}

/// The assumption world under the host's description: no realization check blocker remains;
/// every blocker is either admitted by named checks or a labelled mapping assumption, and both
/// families occur.
#[test]
fn the_assumption_world_screens_into_admitted_and_assumed_mapping_kinds() {
    let prep = prepare(&assumption_candidate(), &host_request()).unwrap();
    // Restored S3 assertion: the host-derived evaluation has no `realization:`-prefixed check row.
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
    let classified = screen(&prep.blockers).expect("screened");
    assert_eq!(classified.len(), prep.blockers.len());
    let mapping = [
        BlockerKind::ScopeDigestToSubject,
        BlockerKind::ProblemScopeToEpisodeIdentity,
        BlockerKind::ConsumptionEvidenceMapping,
    ];
    for (id, kind, d) in &classified {
        match d {
            Discharge::AssumedDischarge => assert!(mapping.contains(kind), "{id}"),
            Discharge::Admitted(_) => assert!(!mapping.contains(kind), "{id}"),
            Discharge::Undischarged => panic!("{id} undischarged"),
        }
    }
    assert!(
        classified
            .iter()
            .any(|(_, _, d)| matches!(d, Discharge::Admitted(_)))
    );
    assert!(
        classified
            .iter()
            .any(|(_, _, d)| *d == Discharge::AssumedDischarge)
    );
}

/// The frozen world under the host's description: four known incompatibilities, refused.
#[test]
fn the_frozen_world_is_a_known_incompatibility() {
    let prep = prepare(&candidate(), &host_request()).unwrap();
    match screen(&prep.blockers) {
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
    match screen(&prep.blockers) {
        Err(GateRefusal::Undischarged { ids }) => assert_eq!(
            ids,
            [
                format!("realization:{COND_BUILDER}:mutation"),
                format!("realization:{COND_CAMPAIGN}:mutation"),
            ]
        ),
        other => panic!("expected undischarged, got {other:?}"),
    }
}

/// A blocker whose kind is not the one its id names refuses: an unknown id family, or a
/// realization check relabelled with a dischargeable kind.
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
            screen(&blockers),
            Err(GateRefusal::KindMismatch { id, kind, named })
        );
    }
}

// --- the host-held admission -----------------------------------------------------------------

fn held(src: &FixtureSource) -> HeldAdmission<'_> {
    HeldAdmission::admit(
        src,
        &FIXED,
        boundary(BoundaryKind::Consumption),
        &operation_request(),
        &declared(),
    )
    .expect("the fixture authority world admits the assumption world's request")
}

/// Invariant 6: the fixture authority world admits the assumption world's request in
/// `Domain::Fixture` with every check passed; an empty world refuses (no observation is enough).
#[test]
fn the_fixture_authority_world_admits_and_an_empty_world_refuses() {
    let w = authority_world();
    let admission = held(&w.src);
    assert_eq!(admission.admission().domain(), Domain::Fixture);
    assert!(admission.admission().outcomes().all_passed());
    assert!(
        admission
            .admission()
            .pins()
            .grants
            .contains(&w.ids.owner_grant.unwrap())
    );
    let empty = FixtureSource::new();
    match HeldAdmission::admit(
        &empty,
        &FIXED,
        boundary(BoundaryKind::Delivery),
        &operation_request(),
        &declared(),
    ) {
        Err(GateRefusal::Admission(_)) => {}
        other => panic!("expected an admission refusal, got {other:?}"),
    }
    empty.inject_unknown("torn record");
    match HeldAdmission::admit(
        &empty,
        &FIXED,
        boundary(BoundaryKind::Delivery),
        &operation_request(),
        &declared(),
    ) {
        Err(GateRefusal::NoObservation(detail)) => assert_eq!(detail, "torn record"),
        other => panic!("expected no observation, got {other:?}"),
    }
}

/// S-A F-2: every regate reads a fresh observation; a revocation after admission is seen by the
/// next regate.
#[test]
fn regate_reads_a_fresh_observation() {
    let w = authority_world();
    let mut admission = held(&w.src);
    assert!(admission.regate().permits_effect());
    w.src.revoke(&w.ids.reviewer_binding);
    assert_eq!(
        admission.regate(),
        AdmissionState::Revoked(vec![w.ids.reviewer_binding])
    );
}

/// S-A F-3: the first non-current state other than Unknown is final for the admission. A refusal
/// caused by an ambiguous decision scope stays final after the ambiguity is revoked, although a
/// point check by S-A alone would find the admission current again.
#[test]
fn the_first_non_current_state_other_than_unknown_is_final() {
    let w = authority_world();
    let mut admission = held(&w.src);
    let extra = w.src.issue(Record::fixture_decision_scope(
        "ds-1",
        1,
        common::authority::problem_scope(),
        &["accept_claim"],
        None,
    ));
    let refused = admission.regate();
    assert!(matches!(refused, AdmissionState::Refused(_)), "{refused:?}");
    w.src.revoke(&extra);
    let reservation = reserve(boundary(BoundaryKind::Consumption));
    assert!(
        admission
            .admission()
            .admission_state(&observe_for(&w.src, &reservation), common::authority::NOW)
            .permits_effect(),
        "S-A alone finds the admission current again"
    );
    assert_eq!(admission.regate(), refused, "final");
    // Supersession: a superseded pin stays superseded in `FixtureSource`, so S-A alone returns the
    // same `Stale` at every regate. This half therefore does NOT separate finality from
    // recomputation (the `Refused` half above does); it pins only that the state is stable.
    let w = authority_world();
    let mut admission = held(&w.src);
    let binding = w.ids.reviewer_binding;
    let _ = w.src.supersede(
        &binding,
        Record::fixture_vantage_binding(ORG, VANTAGE, w.ids.reviewer, host_digest(), ACTOR),
    );
    let stale = admission.regate();
    assert_eq!(
        stale,
        AdmissionState::Stale(StaleReason::IssuanceSuperseded(vec![binding]))
    );
    assert_eq!(admission.regate(), stale);
}

/// Unknown fails closed and is not final: it never permits the effect, and a later regate with an
/// observation decides normally.
#[test]
fn unknown_does_not_permit_and_is_not_final() {
    let w = authority_world();
    let mut admission = held(&w.src);
    w.src.inject_unknown("read failed");
    let unknown = admission.regate();
    assert_eq!(
        unknown,
        AdmissionState::Unknown(UnknownReason::NoObservation("read failed".into()))
    );
    assert!(!unknown.permits_effect());
    w.src.clear_unknown();
    assert!(admission.regate().permits_effect());
}
