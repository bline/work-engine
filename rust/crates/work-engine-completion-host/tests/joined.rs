use serde_json::json;
use work_engine_completion_host::Request;

#[test]
fn wire_cannot_supply_an_operator_principal_or_approval_boolean() {
    let base = json!({"command":"create","offerRef":{"kind":"completion-offer","schemaVersion":1,
        "rootId":"fixture","recordId":"offer","sha256":"a".repeat(64)},
        "displayedDigest":"b".repeat(64),"requestId":"create-1"});
    assert!(serde_json::from_value::<Request>(base.clone()).is_ok());
    let mut forged = base.clone();
    forged["actor"] = json!("human");
    assert!(serde_json::from_value::<Request>(forged).is_err());
    let mut forged = base.clone();
    forged["approve"] = json!(true);
    assert!(serde_json::from_value::<Request>(forged).is_err());
    let mut missing = base;
    missing.as_object_mut().unwrap().remove("displayedDigest");
    assert!(serde_json::from_value::<Request>(missing).is_err());
}
