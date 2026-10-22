//! Unit 2: the token-usage projector.

use context_pressure::{
    ErrorKind, PressureProjection, TokenUsagePressureProjector, UnavailableReason,
};
use serde_json::{Value, json};

fn profile() -> Value {
    json!({
        "schemaVersion": 1,
        "usageField": "last.totalTokens",
        "windowField": "modelContextWindow",
        "rounding": "floor",
        "saturation": "clamp_10000",
    })
}

fn snapshot(total: Value, input: Value, window: Value) -> Value {
    json!({
        "schemaVersion": 1,
        "threadId": "thread-1",
        "latestTokenUsage": {
            "sequence": 7,
            "observedAt": "2026-08-25T22:01:00.000Z",
            "details": {
                "last": { "totalTokens": total, "inputTokens": input },
                "modelContextWindow": window,
            },
        },
    })
}

fn projector() -> TokenUsagePressureProjector {
    TokenUsagePressureProjector::new(&profile()).unwrap()
}

fn error_kind(value: &Value) -> ErrorKind {
    projector().project(value).unwrap_err().kind()
}

#[test]
fn projects_floor_of_ratio_in_basis_points() {
    let PressureProjection::Projected(projected) = projector()
        .project(&snapshot(json!(6_500), json!(6_000), json!(10_000)))
        .unwrap()
    else {
        panic!("expected a projection");
    };
    let observation = projected.observation();
    assert_eq!(observation.pressure_basis_points(), 6_500);
    assert_eq!(observation.observation_id(), "token-usage:thread-1:7");
    assert_eq!(observation.sequence(), 7);
    assert_eq!(
        observation.source(),
        "codex_app_server:last.totalTokens/modelContextWindow"
    );
    assert_eq!(projected.measured_tokens(), 6_500);
    assert_eq!(projected.reported_input_tokens(), 6_000);
    assert_eq!(projected.context_window(), 10_000);
    assert_eq!(
        projected.profile_revision(),
        projector().profile().profile_revision()
    );
    assert_eq!(observation.source_revision().len(), 71);

    let wire = serde_json::to_value(PressureProjection::Projected(projected)).unwrap();
    assert_eq!(wire["status"], "projected");
    assert_eq!(wire["observation"]["schemaVersion"], 1);
}

#[test]
fn floors_and_saturates() {
    let pressure = |total: u64, window: u64| match projector()
        .project(&snapshot(json!(total), json!(0), json!(window)))
        .unwrap()
    {
        PressureProjection::Projected(p) => p.observation().pressure_basis_points(),
        other => panic!("unexpected {other:?}"),
    };
    assert_eq!(pressure(1, 3), 3_333);
    assert_eq!(pressure(2, 3), 6_666);
    assert_eq!(pressure(0, 100), 0);
    assert_eq!(pressure(100, 100), 10_000);
    assert_eq!(pressure(250, 100), 10_000);
}

#[test]
fn unavailable_evidence_is_distinct_from_error() {
    let missing = json!({ "schemaVersion": 1, "threadId": "t", "latestTokenUsage": null });
    assert_eq!(
        projector().project(&missing).unwrap(),
        PressureProjection::Unavailable {
            reason: UnavailableReason::TokenUsageMissing
        }
    );
    for window in [json!(0), json!(-5), json!(1.5), json!(null), json!("9")] {
        assert_eq!(
            projector()
                .project(&snapshot(json!(10), json!(10), window))
                .unwrap(),
            PressureProjection::Unavailable {
                reason: UnavailableReason::ContextWindowMissing
            }
        );
    }
    let wire = serde_json::to_value(PressureProjection::Unavailable {
        reason: UnavailableReason::ContextWindowMissing,
    })
    .unwrap();
    assert_eq!(
        wire,
        json!({ "status": "unavailable", "reason": "context_window_missing" })
    );
}

#[test]
fn rejects_malformed_usage_in_js_order() {
    assert_eq!(error_kind(&json!(null)), ErrorKind::NotObject);
    assert_eq!(
        error_kind(&json!({ "schemaVersion": 1, "threadId": "t" })),
        ErrorKind::NotObject
    );
    assert_eq!(
        error_kind(&snapshot(json!(-1), json!(1), json!(10))),
        ErrorKind::InvalidTokenCount
    );
    assert_eq!(
        error_kind(&snapshot(json!(1.5), json!(1), json!(10))),
        ErrorKind::InvalidTokenCount
    );
    assert_eq!(
        error_kind(&snapshot(json!(1), json!("x"), json!(10))),
        ErrorKind::InvalidTokenCount
    );
    // Token counts are checked before the window: a bad count beats a missing window.
    assert_eq!(
        error_kind(&snapshot(json!(-1), json!(1), json!(null))),
        ErrorKind::InvalidTokenCount
    );
    let mut bad_sequence = snapshot(json!(1), json!(1), json!(10));
    bad_sequence["latestTokenUsage"]["sequence"] = json!(0);
    assert_eq!(error_kind(&bad_sequence), ErrorKind::InvalidUsageSequence);
    let mut bad_time = snapshot(json!(1), json!(1), json!(10));
    bad_time["latestTokenUsage"]["observedAt"] = json!("not a time");
    assert_eq!(error_kind(&bad_time), ErrorKind::InvalidUsageTimestamp);
}

#[test]
fn source_revision_binds_the_whole_usage_observation() {
    let revision = |extra: Value| {
        let mut value = snapshot(json!(10), json!(10), json!(100));
        value["latestTokenUsage"]["extra"] = extra;
        match projector().project(&value).unwrap() {
            PressureProjection::Projected(p) => p.observation().source_revision().to_owned(),
            other => panic!("unexpected {other:?}"),
        }
    };
    assert_eq!(revision(json!(1)), revision(json!(1.0)));
    assert_ne!(revision(json!(1)), revision(json!(2)));
}

#[test]
fn rust_only_strictness_on_thread_identity() {
    let mut value = snapshot(json!(1), json!(1), json!(10));
    value["threadId"] = json!(5);
    assert_eq!(error_kind(&value), ErrorKind::InvalidSnapshotField);
    let mut value = snapshot(json!(1), json!(1), json!(10));
    value.as_object_mut().unwrap().remove("schemaVersion");
    assert_eq!(error_kind(&value), ErrorKind::InvalidSnapshotField);
}

#[test]
fn rejects_bad_profile() {
    let bad = json!({ "schemaVersion": 2 });
    assert_eq!(
        TokenUsagePressureProjector::new(&bad).unwrap_err().kind(),
        ErrorKind::UnsupportedSchemaVersion
    );
}
