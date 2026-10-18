use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn request() -> (Value, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let bundle: Value = serde_json::from_slice(
        &std::fs::read(root.join(
            "rust/crates/work-engine-compiler/tests/fixtures/extension_v1/sealed-bundle.json",
        ))
        .unwrap(),
    )
    .unwrap();
    let revision = bundle["source_revision"].clone();
    let root_text = root.to_str().unwrap().to_owned();
    let request = json!({"schema_version":4,"request_id":"extension-test","operation":"compile_extension_bundle",
        "workspace_root":root_text.clone(),"bundle":bundle,"host":{"repository_revision":revision,
        "allowed_capabilities":["product-development.intake.read-source"],
        "allowed_adapters":["product-development.intake.read-source"],"allowed_providers":["local"]}});
    (request, root_text)
}
fn invoke(request: &Value, root: &str) -> (bool, Value) {
    invoke_bytes(request.to_string().as_bytes(), root)
}
fn invoke_bytes(request: &[u8], root: &str) -> (bool, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-compiler"))
        .env("WORK_ENGINE_COMPILER_WORKSPACE_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(request).unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.success(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

#[test]
fn selected_extension_operation_returns_correlated_explicit_output_bytes() {
    let (request, root) = request();
    let (success, response) = invoke(&request, &root);
    assert!(success, "{response}");
    assert_eq!(response["schema_version"], 4);
    assert_eq!(response["request_id"], "extension-test");
    assert_eq!(
        response["producer"],
        "work-engine.extension-compiler.rust-v1"
    );
    assert_eq!(response["attachment"]["schema_version"], 2);
    assert_eq!(
        response["attachment"]["sha256"],
        "f8755809381fa4a79dfb75c6f2de40d118cd353ee140960d410752901b182b05"
    );
    assert_eq!(
        response["attachment"]["profile"],
        "work-engine.run-extension.rust-v1"
    );
    assert!(
        response["attachment"]["descriptor"]["compiled_skills"][0]["output_bytes"]
            .as_array()
            .unwrap()
            .len()
            > 4000
    );
    let stored: Value =
        serde_json::from_slice(&serde_json::to_vec(&response["attachment"]).unwrap()).unwrap();
    let verify_request = json!({"schema_version":4,"request_id":"fresh-reader","operation":"verify_extension_attachment",
        "attachment":stored,"host":request["host"]});
    let (verified, readback) = invoke(&verify_request, &root);
    assert!(verified, "{readback}");
    assert_eq!(
        readback["attachment_sha256"],
        response["attachment"]["sha256"]
    );
    assert_eq!(readback["bundle_id"], "fixture-research");
}

#[test]
fn fresh_reader_rejects_tampered_and_unadmitted_serialized_attachment() {
    let (request, root) = request();
    let (ok, response) = invoke(&request, &root);
    assert!(ok);
    let mut attachment = response["attachment"].clone();
    attachment["descriptor"]["compiled_skills"][0]["output_bytes"][0] = json!(255);
    let verify_request = json!({"schema_version":4,"request_id":"tampered","operation":"verify_extension_attachment",
        "attachment":attachment,"host":request["host"]});
    let (ok, result) = invoke(&verify_request, &root);
    assert!(!ok);
    assert_eq!(result["error"]["code"], "attachment_mismatch");
    let mut denied = request["host"].clone();
    denied["allowed_capabilities"] = json!([]);
    let verify_request = json!({"schema_version":4,"request_id":"denied","operation":"verify_extension_attachment",
        "attachment":response["attachment"],"host":denied});
    let (ok, result) = invoke(&verify_request, &root);
    assert!(!ok);
    assert_eq!(result["error"]["code"], "invalid_extension");
}

#[test]
fn extension_operation_refuses_stale_input_and_host_admission() {
    let (mut stale_request, root) = request();
    stale_request["bundle"]["skills"][0]["interface_sha256"] = json!("0".repeat(64));
    let (success, response) = invoke(&stale_request, &root);
    assert!(!success);
    assert_eq!(response["error"]["code"], "source_mismatch");
    let (mut denied_request, root) = request();
    denied_request["host"]["allowed_providers"] = json!([]);
    let (success, response) = invoke(&denied_request, &root);
    assert!(!success);
    assert_eq!(response["error"]["code"], "invalid_extension");

    let (mut revision_request, root) = request();
    revision_request["bundle"]["source_revision"] = json!("0".repeat(40));
    revision_request["bundle"]["skills"][0]["structure"] = json!("/missing/source.yaml");
    let (success, response) = invoke(&revision_request, &root);
    assert!(!success);
    assert_eq!(response["error"]["code"], "source_mismatch");
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("admitted repository revision")
    );
}

#[test]
fn repeated_paths_cannot_multiply_materialized_source_without_bound() {
    let (mut request, _) = request();
    let temp = tempfile::TempDir::new().unwrap();
    let source = temp.path().join("large-source.yaml");
    std::fs::write(&source, vec![b'x'; 4 * 1024 * 1024]).unwrap();
    request["workspace_root"] = json!(temp.path().to_str().unwrap());
    let one = request["bundle"]["skills"][0].clone();
    let repeated = (0..9)
        .map(|_| {
            let mut skill = one.clone();
            skill["structure"] = json!(source.to_str().unwrap());
            skill["interface"] = json!(source.to_str().unwrap());
            skill
        })
        .collect::<Vec<_>>();
    request["bundle"]["skills"] = json!(repeated);
    let (success, response) = invoke(&request, temp.path().to_str().unwrap());
    assert!(!success);
    assert_eq!(response["error"]["code"], "resource_limit");
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("materialized extension sources")
    );
}

#[test]
fn large_valid_attachment_roundtrips_through_fresh_verifier() {
    let (mut request, root) = request();
    let skill = request["bundle"]["skills"][0].clone();
    request["bundle"]["skills"] = Value::Array(vec![skill; 120]);
    request["bundle"]["run"]["opaque"] = json!("x".repeat(3 * 1024 * 1024));
    assert!(request.to_string().len() < 4 * 1024 * 1024);
    let (compiled, response) = invoke(&request, &root);
    assert!(compiled, "{response}");
    let serialized = serde_json::to_vec(&response["attachment"]).unwrap();
    assert!(serialized.len() > 4 * 1024 * 1024);
    let attachment: Value = serde_json::from_slice(&serialized).unwrap();
    let verify_request = json!({"schema_version":4,"request_id":"large-reader",
        "operation":"verify_extension_attachment","attachment":attachment,"host":request["host"]});
    let (verified, readback) = invoke(&verify_request, &root);
    assert!(verified, "{readback}");
    assert_eq!(readback["bundle_id"], "fixture-research");
}

#[test]
fn compile_refuses_host_that_expands_beyond_fresh_verifier_envelope() {
    let (mut request, root) = request();
    let skill = request["bundle"]["skills"][0].clone();
    request["bundle"]["skills"] = Value::Array(vec![skill; 120]);
    request["host"]["opaque"] = json!("numeric-marker");
    let serialized = request.to_string();
    let numbers = vec!["1e15"; 820_000].join(",");
    let raw = serialized.replace(
        "\"opaque\":\"numeric-marker\"",
        &format!("\"opaque\":[{numbers}]"),
    );
    assert!(raw.len() < 4 * 1024 * 1024);
    let parsed: Value = serde_json::from_str(&raw).unwrap();
    assert!(serde_json::to_vec(&parsed["host"]).unwrap().len() > 14 * 1024 * 1024);
    let (success, response) = invoke_bytes(raw.as_bytes(), &root);
    assert!(!success, "compile unexpectedly accepted expanding host");
    assert_eq!(response["error"]["code"], "resource_limit");
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("fresh verifier")
    );
}
