//! Unit 3: the hysteresis controller (the cases of the JS controller test file, plus edges).

use context_pressure::{
    ContextPressureController, ContextPressurePolicy, Disposition, ErrorKind, ObserveStatus,
    TransitionReason,
};
use serde_json::{Value, json};

fn policy() -> Value {
    json!({
        "schemaVersion": 1,
        "unit": "basis_points",
        "approaching": { "enter": 6500, "exit": 5500 },
        "replacementCandidate": { "enter": 7800, "exit": 6800 },
        "critical": { "enter": 9000, "exit": 8000 },
    })
}

fn observation(index: u64, pressure: u64) -> Value {
    json!({
        "schemaVersion": 1,
        "observationId": format!("observation-{index}"),
        "sequence": index,
        "observedAt": format!("2026-08-25T22:{index:02}:00.000Z"),
        "pressureBasisPoints": pressure,
        "source": "test-host-projection",
        "sourceRevision": format!("sha256:{}", "a".repeat(64)),
    })
}

fn controller() -> ContextPressureController {
    ContextPressureController::from_options(&json!({ "policy": policy() })).unwrap()
}

fn step(
    c: &mut ContextPressureController,
    index: u64,
    pressure: u64,
) -> (Disposition, TransitionReason) {
    let result = c.observe(&observation(index, pressure)).unwrap();
    assert_eq!(result.status, ObserveStatus::Observed);
    (result.transition.disposition(), result.transition.reason())
}

#[test]
fn advances_through_configured_dispositions() {
    let mut c = controller();
    let first = c.observe(&observation(1, 6000)).unwrap();
    assert_eq!(first.transition.disposition(), Disposition::Comfortable);
    assert!(!first.transition.changed());
    assert_eq!(
        first.transition.reason(),
        TransitionReason::BelowEntryThreshold
    );

    let second = c.observe(&observation(2, 6500)).unwrap();
    assert_eq!(second.transition.disposition(), Disposition::Approaching);
    assert!(second.transition.changed());
    assert_eq!(
        second.transition.reason(),
        TransitionReason::ThresholdCrossed
    );

    assert_eq!(step(&mut c, 3, 7800).0, Disposition::ReplacementCandidate);
    assert_eq!(step(&mut c, 4, 9000).0, Disposition::Critical);
    assert_eq!(c.snapshot().disposition(), Disposition::Critical);
}

#[test]
fn hysteresis_holds_until_each_exit_band() {
    let mut c = controller();
    assert_eq!(step(&mut c, 1, 9500).0, Disposition::Critical);
    assert_eq!(
        step(&mut c, 2, 8500),
        (Disposition::Critical, TransitionReason::HysteresisHeld)
    );
    assert_eq!(step(&mut c, 3, 7500).0, Disposition::ReplacementCandidate);
    assert_eq!(
        step(&mut c, 4, 7000),
        (
            Disposition::ReplacementCandidate,
            TransitionReason::HysteresisHeld
        )
    );
    assert_eq!(step(&mut c, 5, 6500).0, Disposition::Approaching);
    assert_eq!(
        step(&mut c, 6, 6000),
        (Disposition::Approaching, TransitionReason::HysteresisHeld)
    );
    assert_eq!(step(&mut c, 7, 5000).0, Disposition::Comfortable);
}

#[test]
fn within_disposition_and_stage_skipping() {
    let mut c = controller();
    let jump = c.observe(&observation(1, 9400)).unwrap();
    assert_eq!(
        jump.transition.previous_disposition(),
        Disposition::Comfortable
    );
    assert_eq!(jump.transition.disposition(), Disposition::Critical);
    assert_eq!(
        step(&mut c, 2, 9100),
        (Disposition::Critical, TransitionReason::WithinDisposition)
    );
    let wire = serde_json::to_value(c.snapshot()).unwrap();
    let mut keys: Vec<&str> = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "disposition",
            "lastObservation",
            "lastTransition",
            "minimumSequence",
            "policyRevision",
            "schemaVersion"
        ]
    );
}

#[test]
fn exact_replay_is_idempotent_and_conflicts_fail() {
    let mut c = controller();
    let source = observation(1, 7900);
    let first = c.observe(&source).unwrap();
    let replay = c.observe(&source.clone()).unwrap();
    assert_eq!(replay.status, ObserveStatus::Replayed);
    assert_eq!(replay.transition, first.transition);

    let mut conflicting = source.clone();
    conflicting["pressureBasisPoints"] = json!(9500);
    assert_eq!(
        c.observe(&conflicting).unwrap_err().kind(),
        ErrorKind::ObservationIdReused
    );

    let mut stale = observation(2, 8000);
    stale["observationId"] = json!("stale-observation");
    stale["sequence"] = json!(1);
    assert_eq!(
        c.observe(&stale).unwrap_err().kind(),
        ErrorKind::SequenceNotMonotonic
    );
    assert_eq!(
        c.snapshot().last_observation().unwrap().observation_id(),
        "observation-1"
    );
}

#[test]
fn recovered_floor_and_initial_disposition() {
    let options =
        json!({ "policy": policy(), "initialDisposition": "critical", "minimumSequence": 5 });
    let mut c = ContextPressureController::from_options(&options).unwrap();
    assert_eq!(
        c.observe(&observation(5, 6000)).unwrap_err().kind(),
        ErrorKind::SequenceBelowFloor
    );
    assert_eq!(
        step(&mut c, 6, 8500),
        (Disposition::Critical, TransitionReason::HysteresisHeld)
    );

    let bad = |options: Value| {
        ContextPressureController::from_options(&options)
            .unwrap_err()
            .kind()
    };
    assert_eq!(
        bad(json!({ "policy": policy(), "initialDisposition": "panic" })),
        ErrorKind::UnsupportedInitialDisposition
    );
    assert_eq!(
        bad(json!({ "policy": policy(), "initialDisposition": null })),
        ErrorKind::UnsupportedInitialDisposition
    );
    assert_eq!(
        bad(json!({ "policy": policy(), "minimumSequence": -1 })),
        ErrorKind::InvalidMinimumSequence
    );
    assert_eq!(
        bad(json!({ "policy": policy(), "minimumSequence": 1.5 })),
        ErrorKind::InvalidMinimumSequence
    );
    assert_eq!(bad(json!({})), ErrorKind::NotObject);
    assert_eq!(
        bad(json!({ "policy": { "schemaVersion": 2 } })),
        ErrorKind::UnsupportedSchemaVersion
    );
}

#[test]
fn typed_constructor_matches_options_constructor() {
    let typed = ContextPressureController::new(
        ContextPressurePolicy::from_value(&policy()).unwrap(),
        Disposition::Approaching,
        3,
    )
    .unwrap();
    let from_options = ContextPressureController::from_options(
        &json!({ "policy": policy(), "initialDisposition": "approaching", "minimumSequence": 3 }),
    )
    .unwrap();
    assert_eq!(typed.snapshot(), from_options.snapshot());
    assert_eq!(
        ContextPressureController::new(
            ContextPressurePolicy::from_value(&policy()).unwrap(),
            Disposition::Comfortable,
            u64::MAX
        )
        .unwrap_err()
        .kind(),
        ErrorKind::InvalidMinimumSequence
    );
}

#[test]
fn observation_validation_by_kind() {
    let kind = |value: Value| controller().observe(&value).unwrap_err().kind();
    assert_eq!(kind(json!(null)), ErrorKind::NotObject);
    let mut extra = observation(1, 100);
    extra["x"] = json!(1);
    assert_eq!(kind(extra), ErrorKind::UnsupportedFields);
    let mut v = observation(1, 100);
    v["schemaVersion"] = json!(2);
    assert_eq!(kind(v), ErrorKind::UnsupportedSchemaVersion);
    let mut v = observation(1, 100);
    v["observationId"] = json!(" \u{feff}\n");
    assert_eq!(kind(v), ErrorKind::NotNonEmptyString);
    let mut v = observation(1, 100);
    v["sequence"] = json!(0);
    assert_eq!(kind(v), ErrorKind::NotPositiveInteger);
    let mut v = observation(1, 100);
    v["observedAt"] = json!("yesterday");
    assert_eq!(kind(v), ErrorKind::NotIsoTimestamp);
    let mut v = observation(1, 100);
    v["pressureBasisPoints"] = json!(10_001);
    assert_eq!(kind(v), ErrorKind::NotBasisPoints);
    let mut v = observation(1, 100);
    v["sourceRevision"] = json!(format!("sha256:{}", "A".repeat(64)));
    assert_eq!(kind(v), ErrorKind::NotShaRevision);
    let mut v = observation(1, 100);
    v["sourceRevision"] = json!(format!("sha256:{}\n", "a".repeat(64)));
    assert_eq!(kind(v), ErrorKind::NotShaRevision);
}

#[test]
fn transition_wire_form_matches_js_keys() {
    let mut c = controller();
    let result = c.observe(&observation(1, 7000)).unwrap();
    let wire = serde_json::to_value(&result).unwrap();
    let mut keys: Vec<&str> = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "changed",
            "disposition",
            "observation",
            "policyRevision",
            "previousDisposition",
            "reason",
            "schemaVersion",
            "status"
        ]
    );
    assert_eq!(wire["status"], "observed");
    assert_eq!(wire["disposition"], "approaching");
}
