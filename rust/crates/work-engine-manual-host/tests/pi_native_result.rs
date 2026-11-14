use review_execution_evidence::pi::{Binding, LiveStore};
use serde_json::{Value, json};
use work_engine_manual_host::pi_native::review_result;
fn result() -> Value {
    json!({"schemaVersion":1,"subject":{"commit":"c","tree":"t","patchIdentity":"p"},"verdict":"acceptable_as_is","findings":[],"decisiveEvidence":[{"path":"dist.mjs","startLine":1,"endLine":2,"sha256":"a".repeat(64)}],"limitations":["offline only"]})
}
#[test]
fn native_result_consumer_preserves_c4() {
    let t = tempfile::tempdir().unwrap();
    let store = LiveStore::create(
        &t.path().join("store"),
        b"original_synthetic_access".to_vec(),
        Binding::native(
            "11111111-1111-4111-8111-111111111111".into(),
            "attempt".into(),
            1,
        ),
        u64::MAX,
    )
    .unwrap();
    assert!(review_result(&result().to_string(), &store).is_ok());
    let mut r = result();
    r["decisiveEvidence"] = json!([]);
    assert!(review_result(&r.to_string(), &store).is_err());
    for severity in ["info", "low", "medium", "high", "blocker"] {
        for status in ["open", "remediation_presented", "unresolved"] {
            let mut r = result();
            r["findings"] = json!([{"id":"F1","severity":severity,"title":"bounded finding","evidence":[{"path":"dist.mjs","startLine":1,"endLine":2,"sha256":"b".repeat(64)}],"observed":"observation","violatedExpectation":"expectation","consequence":"consequence","basis":"inferred","confidence":"medium","recommendedRemediation":"remediation","status":status,"remediationEvidence":if status=="remediation_presented"{json!([{"path":"dist.mjs","startLine":1,"endLine":2,"sha256":"c".repeat(64)}])}else{json!([])}}]);
            assert_eq!(
                review_result(&r.to_string(), &store).is_ok(),
                severity == "info",
                "{severity}/{status}"
            );
        }
    }
    assert!(review_result("{\"schemaVersion\":1,\"schemaVersion\":2}", &store).is_err());
}

#[test]
fn native_request_projects_pinned_additional_tools_and_refuses_unknown_consumers() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let t = tempfile::tempdir().unwrap();
    let store = LiveStore::create(
        &t.path().join("store"),
        b"original_synthetic_access".to_vec(),
        Binding::native(
            "11111111-1111-4111-8111-111111111111".into(),
            "attempt".into(),
            1,
        ),
        u64::MAX,
    )
    .unwrap();
    let body = json!({"model":"gpt-6-luna","stream":true,"input":[{"type":"additional_tools","role":"developer","tools":[{"type":"function","name":"write","description":"write module","parameters":{"type":"object"},"strict":true}]},{"role":"user","content":[{"type":"input_text","text":"develop module"}]}]});
    let frame = |b: &Value| json!({"method":"POST","headers":{},"body":STANDARD.encode(serde_json::to_vec(b).unwrap())});
    let projected = work_engine_manual_host::pi_native::request(&frame(&body), &store).unwrap();
    let projected: Value = serde_json::from_slice(&projected).unwrap();
    assert_eq!(projected["input"], body["input"]);
    assert_eq!(projected["store"], false);
    for (key, value) in [
        ("name", json!("ungranted-tool")),
        ("parameters", json!("invalid")),
        ("strict", json!(1)),
    ] {
        let mut bad = body.clone();
        bad["input"][0]["tools"][0][key] = value;
        assert!(work_engine_manual_host::pi_native::request(&frame(&bad), &store).is_err());
    }
    let mut bad = body.clone();
    bad["input"][0]["role"] = json!("user");
    assert!(work_engine_manual_host::pi_native::request(&frame(&bad), &store).is_err());
}
