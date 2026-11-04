fn observer() -> Artifact {
    serde_json::from_slice(&std::fs::read(std::env::var("PRESSURE_OBSERVER_PIN").unwrap()).unwrap())
        .unwrap()
}
use serde_json::{Value, json};
use work_engine_manual_host::{
    Artifact,
    pi_docker::coding::{PRESSURE_PROFILE, PressurePacket, validate_pressure_release},
};
fn fixture() -> (PressurePacket, Value) {
    let p = PressurePacket {
        schema: PRESSURE_PROFILE.into(),
        source: "sdk".into(),
        generation: "g1".into(),
        budget: Artifact {
            path: "/unused-in-pure-test".into(),
            sha256: "unused".into(),
        },
        max_age_ms: 1000,
    };
    let ready = json!({"input_digest":"bound-input","container_id":"bound-container","sessionId":"physical-session","snapshot":{"sessionId":"physical-session","idle":true,"observedMs":1767225600000u64,"model":{"id":"catalog-model","provider":"fake","contextWindow":1000},"usage":{"availability":"estimate","value":{"tokens":750,"contextWindow":1000,"percent":75}}}});
    (p, ready)
}
#[test]
fn exact_composition_preserves_opaque_budget_and_integer_pressure() {
    let (p, b) = fixture();
    let raw = "{\"owner\":\"budget\",\"cursor\":\"opaque1\",\"money\":\"do not reinterpret\"}\n";
    let r = pi_runtime_pressure::compose(&p, &b, raw, &observer()).unwrap();
    validate_pressure_release(&p, &r, &b, raw, r.observed_ms, false).unwrap();
    let v: Value = serde_json::from_str(&r.pressure).unwrap();
    assert_eq!(v["budget_control"]["raw"], raw);
    assert_eq!(
        v["context"]["pressure"]["disposition"],
        "replacement_candidate"
    );
    assert_eq!(v["authority"], "observation_only");
}
#[test]
fn addressed_freshness_sequence_and_fence_negatives() {
    let (p, b) = fixture();
    let r = pi_runtime_pressure::compose(&p, &b, "opaque", &observer()).unwrap();
    for field in 0..7 {
        let mut bad = r.clone();
        match field {
            0 => bad.session_id = "other".into(),
            1 => bad.container_id = "other".into(),
            2 => bad.input_digest = "other".into(),
            3 => bad.generation = "g2".into(),
            4 => bad.source = "other".into(),
            5 => bad.sequence = 2,
            _ => bad.observed_ms += 1,
        };
        assert!(
            validate_pressure_release(&p, &bad, &b, "opaque", r.observed_ms + 1, false).is_err(),
            "field {field}"
        );
    }
    assert!(validate_pressure_release(&p, &r, &b, "opaque", r.observed_ms + 1001, false).is_err());
    assert!(validate_pressure_release(&p, &r, &b, "opaque", r.observed_ms - 1, false).is_err());
    assert!(validate_pressure_release(&p, &r, &b, "opaque", r.observed_ms, true).is_err());
    assert!(validate_pressure_release(&p, &r, &b, "changed-budget", r.observed_ms, false).is_err());
}
#[test]
fn wrong_model_and_sdk_estimate_are_not_rebound() {
    let (p, b) = fixture();
    let mut r = pi_runtime_pressure::compose(&p, &b, "opaque", &observer()).unwrap();
    let mut v: Value = serde_json::from_str(&r.pressure).unwrap();
    v["context"]["input"]["identity"]["model"] = json!("other");
    r.pressure = v.to_string();
    assert!(validate_pressure_release(&p, &r, &b, "opaque", r.observed_ms, false).is_err());
}
#[test]
fn undefined_and_post_compaction_null_stay_distinct() {
    let (p, mut b) = fixture();
    for usage in [
        json!({"availability":"undefined"}),
        json!({"availability":"estimate","value":{"tokens":null,"contextWindow":1000,"percent":null}}),
    ] {
        b["snapshot"]["usage"] = usage.clone();
        let r = pi_runtime_pressure::compose(&p, &b, "opaque", &observer()).unwrap();
        let v: Value = serde_json::from_str(&r.pressure).unwrap();
        assert_eq!(v["context"]["availability"], "unavailable");
        assert_eq!(v["sdk_usage"], usage);
        validate_pressure_release(&p, &r, &b, "opaque", r.observed_ms, false).unwrap();
    }
}
#[test]
fn missing_busy_and_invalid_usage_hold() {
    let (p, mut b) = fixture();
    b["snapshot"]["idle"] = json!(false);
    assert!(pi_runtime_pressure::compose(&p, &b, "opaque", &observer()).is_err());
    b["snapshot"]["idle"] = json!(true);
    b["snapshot"]["usage"] = json!({});
    assert!(pi_runtime_pressure::compose(&p, &b, "opaque", &observer()).is_err());
    b["snapshot"]["model"]["contextWindow"] = json!(0);
    assert!(pi_runtime_pressure::compose(&p, &b, "opaque", &observer()).is_err());
}
#[test]
fn strict_wire_rejects_unknown_fields_and_pressure_bounds() {
    let (p, b) = fixture();
    let r = pi_runtime_pressure::compose(&p, &b, "opaque", &observer()).unwrap();
    let mut v = serde_json::to_value(&r).unwrap();
    v["allowReset"] = json!(true);
    assert!(
        serde_json::from_value::<work_engine_manual_host::pi_docker::coding::PressureRelease>(v)
            .is_err()
    );
    let mut bad = r.clone();
    bad.pressure = "x".repeat(24001);
    assert!(validate_pressure_release(&p, &bad, &b, "opaque", r.observed_ms, false).is_err());
}
