use serde_json::{Value, json};
use std::path::Path;
use work_engine_compiler::{
    EXTENSION_ATTACHMENT_PROFILE, ExtensionSkillSource, compile_extension_bundle,
    extension_canonical_json, sha256_hex, verify_extension_attachment,
};

fn fixture() -> (Value, Value, Vec<ExtensionSkillSource>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let bundle: Value = serde_json::from_slice(
        &std::fs::read(root.join(
            "rust/crates/work-engine-compiler/tests/fixtures/extension_v1/sealed-bundle.json",
        ))
        .unwrap(),
    )
    .unwrap();
    let skills = bundle["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|skill| ExtensionSkillSource {
            structure: std::fs::read(root.join(skill["structure"].as_str().unwrap())).unwrap(),
            interface: std::fs::read(root.join(skill["interface"].as_str().unwrap())).unwrap(),
        })
        .collect();
    let host = json!({"repository_revision":bundle["source_revision"],
        "allowed_capabilities":["product-development.intake.read-source"],
        "allowed_adapters":["product-development.intake.read-source"],"allowed_providers":["local"]});
    (bundle, host, skills)
}

#[test]
fn rust_attachment_serializes_and_rechecks_without_authority() {
    let (bundle, host, sources) = fixture();
    let compiled = compile_extension_bundle(&bundle, &host, &sources).unwrap();
    assert_eq!(compiled.descriptor["schema_version"], 2);
    assert_eq!(
        compiled.sha256,
        "f8755809381fa4a79dfb75c6f2de40d118cd353ee140960d410752901b182b05"
    );
    assert_eq!(
        sha256_hex(compiled.canonical_json.as_bytes()),
        compiled.sha256
    );
    assert_eq!(compiled.descriptor["precedence"], "below_core");
    assert_eq!(
        compiled.descriptor["compiled_skills"][0]["runtime_requirements"]["verified_sources"],
        false
    );
    assert_eq!(
        compiled.descriptor["compiled_skills"][0]["output_sha256"],
        "cf72505d1850e757f6cc6ef3331d0f66ee4c5bf51b52a527c27fe8b78c17a949"
    );
    assert_eq!(
        compiled.descriptor["compiled_skills"][0]["output_bytes"]
            .as_array()
            .unwrap()
            .len(),
        4344
    );
    let attachment = compiled.into_attachment();
    assert_eq!(attachment["schema_version"], 2);
    assert_eq!(attachment["profile"], EXTENSION_ATTACHMENT_PROFILE);
    let serialized = serde_json::to_vec(&attachment).unwrap();
    let parsed: Value = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(
        verify_extension_attachment(&parsed, &host).unwrap(),
        &attachment["descriptor"]
    );
}

#[test]
fn host_admission_and_sealed_constraints_refuse_invalid_bundles() {
    let (bundle, host, sources) = fixture();
    type Mutation = Box<dyn Fn(&mut Value, &mut Value)>;
    let cases: &[(&str, Mutation)] = &[
        (
            "source revision",
            Box::new(|b, _| b["source_revision"] = json!("0".repeat(40))),
        ),
        (
            "run repository",
            Box::new(|b, _| b["run"]["checkout"] = json!("detached:other")),
        ),
        (
            "network",
            Box::new(|b, _| b["run"]["network"] = json!("open")),
        ),
        (
            "capability",
            Box::new(|_, h| h["allowed_capabilities"] = json!([])),
        ),
        (
            "authority",
            Box::new(|b, _| b["authority_requests"] = json!(["production"])),
        ),
        (
            "effect",
            Box::new(|b, _| b["effects"] = json!(["publication"])),
        ),
        (
            "adapter",
            Box::new(|_, h| h["allowed_adapters"] = json!([])),
        ),
        (
            "provider",
            Box::new(|_, h| h["allowed_providers"] = json!([])),
        ),
        (
            "dependency",
            Box::new(|b, _| b["dependencies"][0]["mode"] = json!("network")),
        ),
        (
            "registry admission",
            Box::new(|b, _| b["registry"][0]["capability"] = json!("other")),
        ),
        (
            "registry conflict",
            Box::new(|b, _| {
                let entry = b["registry"][0].clone();
                b["registry"].as_array_mut().unwrap().push(entry);
            }),
        ),
        (
            "stale source",
            Box::new(|b, _| b["skills"][0]["structure_sha256"] = json!("0".repeat(64))),
        ),
    ];
    for (label, change) in cases {
        let (mut b, mut h) = (bundle.clone(), host.clone());
        change(&mut b, &mut h);
        assert!(
            compile_extension_bundle(&b, &h, &sources).is_err(),
            "{label}"
        );
    }
}

#[test]
fn opaque_admitted_fields_survive_attachment_digest() {
    let (mut bundle, host, sources) = fixture();
    bundle["run"]["opaque"] = json!({"z": 2});
    bundle["dependencies"][0]["opaque"] = json!([1, 2]);
    bundle["registry"][0]["opaque"] = json!({"any": true});
    let result = compile_extension_bundle(&bundle, &host, &sources).unwrap();
    assert_eq!(result.descriptor["run"]["opaque"], json!({"z":2}));
    assert_eq!(
        result.descriptor["dependencies"][0]["opaque"],
        json!([1, 2])
    );
    assert_eq!(
        result.descriptor["registry"][0]["opaque"],
        json!({"any":true})
    );
}

#[test]
fn nontext_admission_values_are_rejected_by_the_rust_profile() {
    let (mut bundle, mut host, sources) = fixture();
    bundle["capabilities"] = json!(["product-development.intake.read-source", 2, 2, false]);
    host["allowed_capabilities"] = json!([false, 2, "product-development.intake.read-source"]);
    let issue = compile_extension_bundle(&bundle, &host, &sources)
        .err()
        .unwrap();
    assert_eq!(issue.code, "invalid_extension");
}

#[test]
fn repeated_valid_skills_reject_cumulative_output_before_projection() {
    let (mut bundle, host, sources) = fixture();
    let skill = bundle["skills"][0].clone();
    bundle["skills"] = Value::Array(vec![skill; 121]);
    let source = &sources[0];
    let repeated = (0..121)
        .map(|_| ExtensionSkillSource {
            structure: source.structure.clone(),
            interface: source.interface.clone(),
        })
        .collect::<Vec<_>>();
    let issue = compile_extension_bundle(&bundle, &host, &repeated)
        .err()
        .unwrap();
    assert_eq!(issue.code, "resource_limit");
    assert!(issue.message.contains("compiled output"));
}

#[test]
fn serialized_reader_rejects_tamper_and_rehashed_forbidden_changes() {
    let (bundle, host, sources) = fixture();
    let original = compile_extension_bundle(&bundle, &host, &sources)
        .unwrap()
        .into_attachment();
    let mut changed = original.clone();
    changed["descriptor"]["compiled_skills"][0]["output_bytes"][0] = json!(255);
    assert_eq!(
        verify_extension_attachment(&changed, &host)
            .unwrap_err()
            .code,
        "attachment_mismatch"
    );
    changed["sha256"] = json!(sha256_hex(extension_canonical_json(&changed["descriptor"])));
    assert_eq!(
        verify_extension_attachment(&changed, &host)
            .unwrap_err()
            .code,
        "attachment_mismatch"
    );
    let mut changed = original.clone();
    changed["descriptor"]["effects"] = json!(["publication"]);
    changed["sha256"] = json!(sha256_hex(extension_canonical_json(&changed["descriptor"])));
    assert_eq!(
        verify_extension_attachment(&changed, &host)
            .unwrap_err()
            .code,
        "invalid_extension"
    );
    let mut changed = original.clone();
    changed["profile"] = json!("other");
    assert_eq!(
        verify_extension_attachment(&changed, &host)
            .unwrap_err()
            .code,
        "invalid_extension"
    );
    let mut denied = host.clone();
    denied["allowed_adapters"] = json!([]);
    assert_eq!(
        verify_extension_attachment(&original, &denied)
            .unwrap_err()
            .code,
        "invalid_extension"
    );
}

#[test]
fn opaque_large_integer_changes_require_a_new_attachment_digest() {
    let (mut bundle, host, sources) = fixture();
    bundle["run"]["opaque"] = json!(9_007_199_254_740_992_u64);
    let original = compile_extension_bundle(&bundle, &host, &sources)
        .unwrap()
        .into_attachment();
    let mut changed = original.clone();
    changed["descriptor"]["run"]["opaque"] = json!(9_007_199_254_740_993_u64);
    assert_ne!(changed["descriptor"], original["descriptor"]);
    assert!(verify_extension_attachment(&original, &host).is_ok());
    assert_eq!(
        verify_extension_attachment(&changed, &host)
            .unwrap_err()
            .code,
        "attachment_mismatch"
    );
    bundle["run"]["opaque"] = json!(9_007_199_254_740_993_u64);
    let second = compile_extension_bundle(&bundle, &host, &sources)
        .unwrap()
        .into_attachment();
    assert_ne!(original["sha256"], second["sha256"]);
    assert!(verify_extension_attachment(&second, &host).is_ok());
}
