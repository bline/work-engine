use std::fs;
use std::path::PathBuf;
use std::process::Command;

use lifecycle_wire::{
    AdmissionV1, CommandOperationV1, EffectSettlementV1, ExecutionOutcomeV1, FieldV1, WaitTargetV1,
    WireError, WireErrorCode, negotiate_version, parse_command, parse_snapshot, schema_bundle_v1,
};
use serde_json::{Value, json};
use work_engine_types::CodecContract;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn fixture(name: &str) -> Vec<u8> {
    fs::read(root().join("tests/fixtures").join(name)).unwrap()
}
fn value(name: &str) -> Value {
    serde_json::from_slice(&fixture(name)).unwrap()
}

#[test]
fn generated_draft_and_committed_bytes_are_identical_and_validate_fixtures() {
    let bundle = schema_bundle_v1();
    for (name, schema) in &bundle {
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        let mut expected = serde_json::to_vec_pretty(schema).unwrap();
        expected.push(b'\n');
        assert_eq!(
            fs::read(root().join("schemas/v1").join(name)).unwrap(),
            expected,
            "drift in {name}"
        );
        jsonschema::draft202012::new(schema).unwrap();
    }
    let valid = [
        ("command.schema.json", "command-v1.json"),
        ("snapshot.schema.json", "failed-unsettled-v1.json"),
        (
            "snapshot.schema.json",
            "historical-complete-current-blocked-v1.json",
        ),
        ("wait-result.schema.json", "observation-unavailable-v1.json"),
    ];
    for (schema, document) in valid {
        assert!(
            jsonschema::draft202012::is_valid(&bundle[schema], &value(document)),
            "{document} invalid under {schema}"
        );
    }
    let mut extra = value("command-v1.json");
    extra["extra_authority"] = json!(true);
    assert!(!jsonschema::draft202012::is_valid(
        &bundle["command.schema.json"],
        &extra
    ));
    extra = value("command-v1.json");
    extra["payload"]["override"] = json!(true);
    assert!(!jsonschema::draft202012::is_valid(
        &bundle["command.schema.json"],
        &extra
    ));
    assert!(!jsonschema::draft202012::is_valid(
        &bundle["snapshot.schema.json"],
        &value("unsupported-observation-v2.json")
    ));
}

#[test]
fn command_shape_digest_and_semantics_stay_separate() {
    let command = parse_command(&fixture("command-v1.json")).unwrap();
    assert!(matches!(
        command.operation,
        CommandOperationV1::RequestReplacement(_)
    ));
    let mut altered = value("command-v1.json");
    altered["grant_ref"] = json!("grant-2");
    assert!(jsonschema::draft202012::is_valid(
        &schema_bundle_v1()["command.schema.json"],
        &altered
    ));
    let bytes = serde_json::to_vec(&altered).unwrap();
    assert!(matches!(
        parse_command(&bytes),
        Err(WireError::SemanticInvalidity)
    ));
    altered = value("command-v1.json");
    altered["kind"] = json!("authorize_recovery");
    assert!(matches!(
        parse_command(&serde_json::to_vec(&altered).unwrap()),
        Err(WireError::UnsupportedCapability)
    ));
    altered = value("command-v1.json");
    altered["payload"]["unexpected"] = json!(true);
    assert!(parse_command(&serde_json::to_vec(&altered).unwrap()).is_err());
    altered = value("command-v1.json");
    altered["expected_revision"] = json!("18446744073709551616");
    assert!(matches!(
        parse_command(&serde_json::to_vec(&altered).unwrap()),
        Err(WireError::SemanticInvalidity)
    ));
    assert!(parse_command(br#"{"x":1,"x":2}"#).is_err());
    assert!(parse_command(br#"{"payload":{"x":9007199254740993}}"#).is_err());
}

#[test]
fn read_compatibility_keeps_dimensions_and_rejects_unknown_variants() {
    let failed = parse_snapshot(&fixture("failed-unsettled-v1.json"), "subject-1").unwrap();
    assert!(
        matches!(failed.delivery, FieldV1::Known(ref delivery) if delivery.outcome == ExecutionOutcomeV1::Failed && delivery.settlement == EffectSettlementV1::Unresolved)
    );
    assert!(matches!(
        failed.current_admission,
        AdmissionV1::Fenced { .. }
    ));
    assert!(
        WaitTargetV1::DeliveryOutcome {
            delivery_id: "delivery-1".into(),
            outcome: ExecutionOutcomeV1::Failed,
        }
        .is_observed(&failed)
    );
    assert!(
        !WaitTargetV1::DeliveryOutcome {
            delivery_id: "delivery-other".into(),
            outcome: ExecutionOutcomeV1::Failed,
        }
        .is_observed(&failed)
    );
    let history = parse_snapshot(
        &fixture("historical-complete-current-blocked-v1.json"),
        "subject-1",
    )
    .unwrap();
    assert!(matches!(history.transition, FieldV1::Known(ref transition) if transition.completed));
    assert!(matches!(
        history.current_admission,
        AdmissionV1::Transition { .. }
    ));
    assert!(matches!(
        parse_snapshot(&fixture("unsupported-observation-v2.json"), "subject-1"),
        Err(WireError::UnsupportedObservationVariant)
    ));
    let mut unsupported_stage = value("historical-complete-current-blocked-v1.json");
    unsupported_stage["transition"]["value"]["stage"] = json!("future_stage");
    assert!(matches!(
        parse_snapshot(
            &serde_json::to_vec(&unsupported_stage).unwrap(),
            "subject-1"
        ),
        Err(WireError::UnsupportedObservationVariant)
    ));
    let mut unsupported_provenance = value("failed-unsettled-v1.json");
    unsupported_provenance["delivery"]["value"]["provenance"]["state"] = json!("future_provenance");
    assert!(matches!(
        parse_snapshot(
            &serde_json::to_vec(&unsupported_provenance).unwrap(),
            "subject-1"
        ),
        Err(WireError::UnsupportedObservationVariant)
    ));
    let mut additive = value("failed-unsettled-v1.json");
    additive["future_read_metadata"] = json!("allowed");
    assert!(parse_snapshot(&serde_json::to_vec(&additive).unwrap(), "subject-1").is_ok());
    assert!(matches!(
        parse_snapshot(&fixture("failed-unsettled-v1.json"), "subject-2"),
        Err(WireError::SemanticInvalidity)
    ));
    assert_eq!(negotiate_version(&[2, 1]).unwrap(), 1);
    assert!(matches!(
        negotiate_version(&[2]),
        Err(WireError::UnsupportedVersion)
    ));
    assert_eq!(
        serde_json::to_value(WireErrorCode::UnsupportedObservationVariant).unwrap(),
        "unsupported_observation_variant"
    );
}

#[test]
fn command_codec_bytes_match_independent_vector_and_legacy_bytes_are_not_rehashed() {
    let vector = value("compatibility-v1.json");
    let command = parse_command(&fixture("command-v1.json")).unwrap();
    let basis = command.digest_basis();
    let canonical = CodecContract::LifecycleCommandV1
        .canonical_json(&basis)
        .unwrap();
    assert_eq!(
        String::from_utf8(canonical).unwrap(),
        vector["command_canonical_utf8"]
    );
    assert_eq!(
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap()
            .hex(),
        vector["command_digest"]
    );
    let snapshot = value("failed-unsettled-v1.json");
    assert!(parse_snapshot(&fixture("failed-unsettled-v1.json"), "subject-1").is_ok());
    assert_eq!(
        String::from_utf8(
            CodecContract::LifecycleSnapshotV1
                .canonical_json(&snapshot)
                .unwrap()
        )
        .unwrap(),
        vector["snapshot_canonical_utf8"]
    );
    assert_eq!(
        CodecContract::LifecycleSnapshotV1
            .digest_json(&snapshot)
            .unwrap()
            .hex(),
        vector["snapshot_digest"]
    );
    let historical: Value = serde_json::from_slice(
        &fs::read(root().join("../work-engine-types/tests/fixtures/codec-v1.json")).unwrap(),
    )
    .unwrap();
    assert_ne!(
        historical["legacy_claim_evidence"]["sha256"],
        historical["legacy_workspace_coordination"]["sha256"]
    );

    let js = r#"
const fs = require('fs'); const crypto = require('crypto');
const command = JSON.parse(fs.readFileSync(process.argv[1], 'utf8'));
const snapshot = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
delete command.request_digest;
function sorted(value) {
  if (Array.isArray(value)) return value.map(sorted);
  if (value && typeof value === 'object') return Object.fromEntries(Object.keys(value).sort().map(key => [key, sorted(value[key])]));
  return value;
}
function encode(kind, payload) {
  const envelope = {codec:'jcs-rfc8785-v1',domain:'work-engine.lifecycle',kind,payload,schema:1};
  const canonical = JSON.stringify(sorted(envelope));
  return {canonical, digest:crypto.createHash('sha256').update(canonical).digest('hex')};
}
process.stdout.write(JSON.stringify({command:encode('command', command),snapshot:encode('snapshot', snapshot)}));
"#;
    let result = Command::new("node")
        .args(["-e", js, "--"])
        .arg(root().join("tests/fixtures/command-v1.json"))
        .arg(root().join("tests/fixtures/failed-unsettled-v1.json"))
        .output()
        .expect("Node.js available for JavaScript compatibility vector");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let js_vector: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        js_vector["command"]["canonical"],
        vector["command_canonical_utf8"]
    );
    assert_eq!(js_vector["command"]["digest"], vector["command_digest"]);
    assert_eq!(
        js_vector["snapshot"]["canonical"],
        vector["snapshot_canonical_utf8"]
    );
    assert_eq!(js_vector["snapshot"]["digest"], vector["snapshot_digest"]);

    for case in vector["negotiation"].as_array().unwrap() {
        let offered: Vec<u16> = case["offered"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_u64().unwrap() as u16)
            .collect();
        match case.get("selected") {
            Some(selected) => assert_eq!(
                negotiate_version(&offered).unwrap(),
                selected.as_u64().unwrap() as u16
            ),
            None => assert!(matches!(
                negotiate_version(&offered),
                Err(WireError::UnsupportedVersion)
            )),
        }
    }
}

#[test]
fn u0_manifest_hashes_bind_schema_fixture_source_and_client_api() {
    let manifest: Value =
        serde_json::from_slice(&fs::read(root().join("schemas/v1/manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["protocol_version"], 1);
    assert_eq!(manifest["generator"]["settings"], "draft2020_12");
    let schema_hashes = manifest["schema_sha256"].as_object().unwrap();
    let bundle_hash = CodecContract::BinaryArtifactV1
        .digest_binary(&serde_json::to_vec(schema_hashes).unwrap())
        .unwrap()
        .hex();
    assert_eq!(manifest["schema_bundle_sha256"], bundle_hash);
    assert_eq!(
        manifest["unsupported_observation_variant"]["code"],
        "unsupported_observation_variant"
    );
    for (group, prefix) in [
        ("schema_sha256", "schemas/v1"),
        ("fixture_sha256", "tests/fixtures"),
        ("source_sha256", ""),
    ] {
        for (name, expected) in manifest[group].as_object().unwrap() {
            let bytes = fs::read(root().join(prefix).join(name)).unwrap();
            let actual = CodecContract::BinaryArtifactV1
                .digest_binary(&bytes)
                .unwrap()
                .hex();
            assert_eq!(
                actual,
                expected.as_str().unwrap(),
                "hash drift: {group}/{name}"
            );
        }
    }
}
