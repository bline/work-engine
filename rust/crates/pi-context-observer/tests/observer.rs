use context_pressure::Disposition;
use pi_context_observer::{Identity, Observation, Observer};
use serde_json::json;
fn identity() -> Identity {
    Identity {
        source: "pi:test".into(),
        session: "sdk-session".into(),
        generation: "g1".into(),
        model: "model".into(),
        window: 1000,
    }
}
fn policy() -> serde_json::Value {
    json!({"schemaVersion":1,"unit":"basis_points","approaching":{"exit":4000,"enter":5000},"replacementCandidate":{"exit":6000,"enter":7000},"critical":{"exit":8000,"enter":9000}})
}
fn observation(sequence: u64, tokens: Option<u64>) -> Observation {
    Observation {
        identity: identity(),
        observation_id: format!("o{sequence}"),
        sequence,
        observed_at: "2026-01-01T00:00:00.000Z".into(),
        observed_ms: 1767225600000,
        tokens,
        provenance: "constructed SDK estimate".into(),
    }
}
#[test]
fn evidence_and_ordering() {
    let mut o = Observer::new(identity(), 1000, &policy(), Disposition::Comfortable).unwrap();
    let now = 1767225600000;
    let first = observation(1, Some(750));
    let result = o.observe(first.clone(), now).unwrap();
    assert_eq!(result["pressure"]["disposition"], "replacement_candidate");
    assert_eq!(o.observe(first, now).unwrap(), result);
    assert!(o.observe(observation(1, Some(2)), now).is_err());
    let mut mismatch = observation(2, Some(1));
    mismatch.identity.model = "other".into();
    assert!(o.observe(mismatch, now).is_err());
    let unavailable = o.observe(observation(2, None), now).unwrap();
    assert_eq!(unavailable["availability"], "unavailable");
    assert!(o.observe(observation(3, Some(1)), now + 1001).is_err());
    let resumed = o.observe(observation(3, Some(850)), now).unwrap();
    assert_eq!(resumed["pressure"]["disposition"], "replacement_candidate");
    let high = o.observe(observation(4, Some(950)), now).unwrap();
    assert_eq!(high["pressure"]["disposition"], "critical");
    let low = o.observe(observation(5, Some(0)), now).unwrap();
    assert_eq!(low["pressure"]["disposition"], "comfortable");
}
#[test]
fn invalid_identity_and_timestamp() {
    let mut id = identity();
    id.window = 0;
    assert!(Observer::new(id, 1, &policy(), Disposition::Comfortable).is_err());
    for field in 0..3 {
        let mut o = Observer::new(identity(), 1, &policy(), Disposition::Comfortable).unwrap();
        let mut v = observation(1, Some(1));
        match field {
            0 => v.identity.session = "other".into(),
            1 => v.identity.generation = "other".into(),
            _ => v.observed_ms += 1,
        };
        assert!(o.observe(v, 1767225600001).is_err());
    }
}
#[test]
fn executable_preserves_foreign_control_record() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let text = include_str!("../fixtures/mixed.json");
    let input: serde_json::Value = serde_json::from_str(text).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_pi-context-observer"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        result[0]["result"]["pressure"]["disposition"],
        "replacement_candidate"
    );
    for index in [1, 2] {
        assert_eq!(result[index], input["records"][index]);
        assert!(result[index]["payload"]["next_budget_cursor"].is_string());
        assert_eq!(
            result[index]["payload"]["snapshot"]["measurements"]["labels"]["quota_usd"],
            "API-equivalent dollars adjusted for subscription speed"
        );
    }
    assert_ne!(
        result[1]["payload"]["next_budget_cursor"],
        result[2]["payload"]["next_budget_cursor"]
    );
    assert_eq!(result[1]["payload"]["snapshot"]["status"], "complete");
    assert_eq!(result[1]["payload"]["events"][0]["pool"], "codex");
    assert_eq!(result[1]["payload"]["events"][1]["summary"], "elapsed hours");
    assert_eq!(result[2]["payload"]["snapshot"]["status"], "not_established");
    assert!(result[2]["payload"]["snapshot"]["measurements"]["account_quota"]["used_percent"].is_null());
    assert_eq!(result[2]["payload"]["snapshot"]["cost_controls"], json!([]));
}
#[test]
fn identity_change_does_not_imply_recovery() {
    let mut next = identity();
    next.generation = "g2".into();
    let mut o = Observer::new(next.clone(), 1000, &policy(), Disposition::Critical).unwrap();
    let mut unavailable = observation(1, None);
    unavailable.identity = next.clone();
    assert_eq!(
        o.observe(unavailable, 1767225600000).unwrap()["availability"],
        "unavailable"
    );
    let mut reading = observation(2, Some(850));
    reading.identity = next;
    assert_eq!(
        o.observe(reading, 1767225600000).unwrap()["pressure"]["disposition"],
        "critical"
    );
}

#[test]
fn exact_basis_point_boundaries() {
    let max = 9_007_199_254_740_991_u64;
    let boundary_policy = json!({"schemaVersion":1,"unit":"basis_points","approaching":{"exit":4000,"enter":5700},"replacementCandidate":{"exit":6000,"enter":7000},"critical":{"exit":8000,"enter":9000}});
    for (tokens, window, expected) in [
        (57, 100, 5700),
        (0, 100, 0),
        (1, 3, 3333),
        (101, 100, 10000),
        (max, max, 10000),
        (max - 1, max, 9999),
        (max, 1, 10000),
        (1, max, 0),
    ] {
        let mut id = identity();
        id.window = window;
        let mut o = Observer::new(id.clone(), 1000, &boundary_policy, Disposition::Comfortable).unwrap();
        let mut reading = observation(1, Some(tokens));
        reading.identity = id;
        let result = o.observe(reading, 1767225600000).unwrap();
        assert_eq!(result["pressure"]["observation"]["pressureBasisPoints"], expected);
        if tokens == 57 && window == 100 {
            assert_eq!(result["pressure"]["disposition"], "approaching");
        }
    }
}
