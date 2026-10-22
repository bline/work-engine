//! Unit 1: policy and profile validation, revision hashing.

use context_pressure::{
    ContextPressurePolicy, ErrorKind, PolicyUnit, Rounding, Saturation, TokenUsagePressureProfile,
    UsageField, WindowField,
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

fn with(mut base: Value, key: &str, value: Value) -> Value {
    base[key] = value;
    base
}

fn profile() -> Value {
    json!({
        "schemaVersion": 1,
        "usageField": "last.totalTokens",
        "windowField": "modelContextWindow",
        "rounding": "floor",
        "saturation": "clamp_10000",
    })
}

#[test]
fn policy_is_explicit_revision_bound_and_strict() {
    let first = ContextPressurePolicy::from_value(&policy()).unwrap();
    let second = ContextPressurePolicy::from_value(&policy()).unwrap();
    assert_eq!(first.policy_revision(), second.policy_revision());
    assert!(first.policy_revision().starts_with("sha256:"));
    assert_eq!(first.policy_revision().len(), 71);
    assert_eq!(first.unit(), PolicyUnit::BasisPoints);

    let unknown = ContextPressurePolicy::from_value(&with(policy(), "sampleEveryTurns", json!(1)))
        .unwrap_err();
    assert_eq!(unknown.kind(), ErrorKind::UnsupportedFields);
    assert_eq!(
        unknown.message(),
        "context pressure policy contains unsupported fields: sampleEveryTurns"
    );

    let equal_band = with(
        policy(),
        "approaching",
        json!({ "enter": 6500, "exit": 6500 }),
    );
    let error = ContextPressurePolicy::from_value(&equal_band).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::BandExitNotBelowEnter);
    assert!(error.message().contains("exit must be lower"));

    let unordered = with(
        policy(),
        "replacementCandidate",
        json!({ "enter": 7800, "exit": 6000 }),
    );
    let error = ContextPressurePolicy::from_value(&unordered).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ThresholdsNotOrdered);
}

#[test]
fn policy_rejects_each_malformed_field_by_kind() {
    let kind = |value: Value| {
        ContextPressurePolicy::from_value(&value)
            .unwrap_err()
            .kind()
    };
    assert_eq!(kind(json!([])), ErrorKind::NotObject);
    assert_eq!(kind(json!(null)), ErrorKind::NotObject);
    assert_eq!(
        kind(with(policy(), "schemaVersion", json!(2))),
        ErrorKind::UnsupportedSchemaVersion
    );
    assert_eq!(
        kind(with(policy(), "unit", json!("tokens"))),
        ErrorKind::UnsupportedUnit
    );
    assert_eq!(
        kind(with(
            policy(),
            "critical",
            json!({ "enter": 10001, "exit": 8000 })
        )),
        ErrorKind::NotBasisPoints
    );
    assert_eq!(
        kind(with(
            policy(),
            "critical",
            json!({ "enter": 9000.5, "exit": 8000 })
        )),
        ErrorKind::NotBasisPoints
    );
    assert_eq!(
        kind(with(
            policy(),
            "critical",
            json!({ "enter": 9000, "exit": 8000, "x": 1 })
        )),
        ErrorKind::UnsupportedFields
    );
    assert_eq!(
        kind(with(policy(), "critical", json!("nope"))),
        ErrorKind::NotObject
    );
}

#[test]
fn policy_revision_ignores_key_order_and_integral_float_spelling() {
    let reordered: Value = serde_json::from_str(
        r#"{"critical":{"exit":8000.0,"enter":9000},"unit":"basis_points","schemaVersion":1.0,
            "replacementCandidate":{"exit":6800,"enter":7800},"approaching":{"exit":5500,"enter":6500}}"#,
    )
    .unwrap();
    assert_eq!(
        ContextPressurePolicy::from_value(&reordered)
            .unwrap()
            .policy_revision(),
        ContextPressurePolicy::from_value(&policy())
            .unwrap()
            .policy_revision()
    );
}

#[test]
fn policy_serializes_to_the_js_wire_form() {
    let wire = serde_json::to_value(ContextPressurePolicy::from_value(&policy()).unwrap()).unwrap();
    let mut expected = policy();
    expected["policyRevision"] = wire["policyRevision"].clone();
    assert_eq!(wire, expected);
}

#[test]
fn profile_validates_and_binds_a_revision() {
    let first = TokenUsagePressureProfile::from_value(&profile()).unwrap();
    assert_eq!(first.usage_field(), UsageField::LastTotalTokens);
    assert_eq!(first.window_field(), WindowField::ModelContextWindow);
    assert_eq!(first.rounding(), Rounding::Floor);
    assert_eq!(first.saturation(), Saturation::Clamp10000);
    let wire = serde_json::to_value(&first).unwrap();
    let mut expected = profile();
    expected["profileRevision"] = json!(first.profile_revision());
    assert_eq!(wire, expected);

    let kind = |value: Value| {
        TokenUsagePressureProfile::from_value(&value)
            .unwrap_err()
            .kind()
    };
    assert_eq!(kind(json!(1)), ErrorKind::NotObject);
    assert_eq!(
        kind(with(profile(), "extra", json!(true))),
        ErrorKind::UnsupportedFields
    );
    assert_eq!(
        kind(with(profile(), "schemaVersion", json!(2))),
        ErrorKind::UnsupportedSchemaVersion
    );
    for (field, value) in [
        ("usageField", "last.inputTokens"),
        ("windowField", "other"),
        ("rounding", "ceil"),
        ("saturation", "none"),
    ] {
        assert_eq!(
            kind(with(profile(), field, json!(value))),
            ErrorKind::UnsupportedProfile
        );
    }
}
