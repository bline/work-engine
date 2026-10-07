use serde_json::{Value, json};
use work_engine_types::CodecContract;

#[test]
fn new_jcs_bytes_and_digest_match_cross_language_vector() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/codec-v1.json")).unwrap();
    let payload = &fixture["unicode_payload"];
    let bytes = CodecContract::LifecycleCommandV1
        .canonical_json(payload)
        .unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        fixture["new_canonical_utf8"].as_str().unwrap()
    );
    assert_eq!(
        CodecContract::LifecycleCommandV1
            .digest_json(payload)
            .unwrap()
            .hex(),
        fixture["new_sha256"].as_str().unwrap()
    );
    assert_ne!(
        fixture["legacy_claim_evidence"]["sha256"],
        fixture["new_sha256"]
    );
    assert_ne!(
        fixture["legacy_workspace_coordination"]["sha256"],
        fixture["new_sha256"]
    );
    assert_ne!(
        fixture["legacy_claim_evidence"]["canonical_utf8_hex"],
        fixture["legacy_workspace_coordination"]["canonical_utf8_hex"]
    );
}

#[test]
fn digest_contract_binds_kind_and_exact_json_distinctions() {
    let command = CodecContract::LifecycleCommandV1;
    let snapshot = CodecContract::LifecycleSnapshotV1;
    let absent = json!({});
    let explicit_null = json!({"value": null});
    assert_ne!(
        command.digest_json(&absent).unwrap(),
        command.digest_json(&explicit_null).unwrap()
    );
    assert_ne!(
        command.digest_json(&absent).unwrap(),
        snapshot.digest_json(&absent).unwrap()
    );
    assert!(command.digest_binary(b"bytes").is_err());
    assert!(
        CodecContract::BinaryArtifactV1
            .digest_json(&absent)
            .is_err()
    );
    assert!(command.digest_json(&json!({"unsafe": u64::MAX})).is_err());
    assert!(
        command
            .verify_declaration("work-engine.lifecycle", "command", 1, "jcs-rfc8785-v1")
            .is_ok()
    );
    assert!(
        command
            .verify_declaration("work-engine.lifecycle", "command", 1, "legacy-json")
            .is_err()
    );
    assert!(
        command
            .verify_declaration("other.domain", "command", 1, "jcs-rfc8785-v1")
            .is_err()
    );
    assert_eq!(
        String::from_utf8(
            command
                .canonical_json(&json!({"minus_zero": -0.0}))
                .unwrap()
        )
        .unwrap(),
        "{\"codec\":\"jcs-rfc8785-v1\",\"domain\":\"work-engine.lifecycle\",\"kind\":\"command\",\"payload\":{\"minus_zero\":0},\"schema\":1}"
    );
}

#[test]
fn escaped_numeric_and_exact_binary_vectors_match_literal_bytes() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/codec-v1.json")).unwrap();
    let payload = &fixture["escaped_numeric_payload"];
    let bytes = CodecContract::LifecycleCommandV1
        .canonical_json(payload)
        .unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        fixture["escaped_numeric_canonical_utf8"].as_str().unwrap()
    );
    assert_eq!(
        CodecContract::LifecycleCommandV1
            .digest_json(payload)
            .unwrap()
            .hex(),
        fixture["escaped_numeric_sha256"].as_str().unwrap()
    );

    let binary = [0x00, 0xff, 0x01, 0x00, 0x2a];
    assert_eq!(
        CodecContract::BinaryArtifactV1
            .digest_binary(&binary)
            .unwrap()
            .hex(),
        fixture["binary_sha256"].as_str().unwrap()
    );
    let binary_hex = binary
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(binary_hex, fixture["binary_payload_hex"].as_str().unwrap());
    let text = fixture["text_input_utf8"].as_str().unwrap();
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(text.as_bytes())
        .unwrap();
    assert_eq!(digest.hex(), fixture["text_input_sha256"].as_str().unwrap());
    assert!(
        digest
            .verify_binary(CodecContract::LifecycleTextInputV1, text.as_bytes())
            .is_ok()
    );
    assert!(
        digest
            .verify_binary(CodecContract::LifecycleTextInputV1, b"changed")
            .is_err()
    );
}
