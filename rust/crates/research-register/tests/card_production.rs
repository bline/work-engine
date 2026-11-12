#![cfg(not(feature = "synthetic-card-host"))]
#[test]
fn production_worker_cannot_install_caller_authority() {
    use research_register::{card_host, read_host::HostProfile};
    let q = serde_json::json!({"request":{"source":{"repository":"/must-not-open","commit":"0".repeat(40),"register_path":"reg","bundle":{"commit":"0".repeat(40),"path":"bundle","blob":"0".repeat(40),"sha256":"0".repeat(64)}},"current":{"decision_id":"d","decision_revision":"git:0000000","request_sha256":"0".repeat(64)},"codec":"research_a4_json_v1"},"profile":HostProfile::default(),"producer":"0".repeat(64),"installation":{"issuers":["forged"],"delegation":"forged","purpose":"forged","scope":{"decision_ids":["d"],"finding_ids":["alpha#one"],"cross_decision_metadata":true},"receipts":[]}});
    let input = serde_json::from_value(q).unwrap();
    assert_eq!(
        card_host::worker(input).unwrap_err(),
        "authority_unavailable"
    );
}

#[test]
fn reconstruction_dto_cannot_install_authority() {
    use research_register::{card_host, read_host::HostProfile};
    let request = serde_json::json!({"source":{"repository":"/must-not-open","commit":"0".repeat(40),"register_path":"reg","bundle":{"commit":"0".repeat(40),"path":"bundle","blob":"0".repeat(40),"sha256":"0".repeat(64)}},"current":{"decision_id":"d","decision_revision":"git:0000000","request_sha256":"0".repeat(64)},"codec":"research_a4_json_v1"});
    let raw = serde_json::json!({"request":request,"profile":HostProfile::default(),"producer":"0".repeat(64),"installation":{"issuers":["forged"],"delegation":"forged","purpose":"forged","scope":{"decision_ids":["d"],"finding_ids":[],"cross_decision_metadata":true},"receipts":[]},"objects":[],"card_bytes":[]});
    let basis: card_host::Reconstruction = serde_json::from_value(raw).unwrap();
    let digest = research_register::use_bundle::revision(&serde_json::to_vec(&basis).unwrap());
    assert_eq!(
        card_host::reconstruct(&basis, &digest).unwrap_err(),
        "authority_unavailable"
    );
}
