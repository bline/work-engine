use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};

fn invoke(request: &[u8]) -> (std::process::Output, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-compiler"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(request).unwrap();
    let output = child.wait_with_output().unwrap();
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    (output, envelope)
}

#[test]
fn one_shot_success_and_errors_are_bounded_and_separated() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../work-engine-compiler/tests/fixtures/v1/skills/p-rep");
    let structure = std::fs::read(root.join("structure.yaml")).unwrap();
    let interface = std::fs::read(root.join("interface.yaml")).unwrap();
    let request = json!({"schema_version":1,"operation":"compile_skill_unverified","structure_source_base64":STANDARD.encode(structure),"interface_source_base64":STANDARD.encode(interface)});
    let (result, envelope) = invoke(&serde_json::to_vec(&request).unwrap());
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert_eq!(result.stdout.last(), Some(&b'\n'));
    assert_eq!(envelope["status"], "ok");
    assert_eq!(
        envelope["ir"]["runtime_requirements"]["verified_sources"],
        false
    );
    assert_eq!(
        STANDARD
            .decode(envelope["output_base64"].as_str().unwrap())
            .unwrap(),
        std::fs::read(root.join("expected.md")).unwrap()
    );
    let cases = [
        (
            json!({"schema_version":2,"operation":"compile_skill_unverified"}),
            "unsupported_version",
        ),
        (
            json!({"schema_version":1,"operation":"verify_skill"}),
            "unsupported_operation",
        ),
        (
            json!({"schema_version":1,"operation":"compile_skill_unverified","verified_sources":true}),
            "invalid_request",
        ),
        (
            json!({"schema_version":1,"operation":"compile_skill_unverified","structure_source_base64":"!"}),
            "invalid_encoding",
        ),
    ];
    for (request, code) in cases {
        let (result, envelope) = invoke(&serde_json::to_vec(&request).unwrap());
        assert_eq!(result.status.code(), Some(2));
        assert!(!result.stderr.is_empty());
        assert_eq!(envelope["status"], "error");
        assert_eq!(envelope["error"]["code"], code);
        assert!(envelope.get("ir").is_none());
    }
}

#[test]
fn oversized_request_is_resource_limit() {
    let request = vec![b' '; 16 * 1024 * 1024 + 1];
    let (result, envelope) = invoke(&request);
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(envelope["error"]["code"], "resource_limit");
}

#[test]
fn near_limit_unknown_key_has_a_bounded_error_envelope() {
    let key = "x".repeat(16 * 1024 * 1024 - 32);
    let request = format!("{{\"{key}\":0}}");
    assert!(request.len() < 16 * 1024 * 1024);
    let (result, envelope) = invoke(request.as_bytes());
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.len() <= 16 * 1024 * 1024);
    assert_eq!(envelope["status"], "error");
    assert_eq!(envelope["error"]["code"], "invalid_request");
    assert!(envelope["error"]["path"].is_null());
}
