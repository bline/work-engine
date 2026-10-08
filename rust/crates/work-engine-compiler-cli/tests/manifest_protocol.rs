use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn invoke(bytes: &[u8]) -> (std::process::Output, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_work-engine-compiler"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let out = child.wait_with_output().unwrap();
    let response = serde_json::from_slice(&out.stdout).unwrap();
    (out, response)
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../work-engine-compiler/tests/fixtures/v1/manifest/projection-oracle.json"
    ))
    .unwrap()
}
#[test]
fn protocol_v3_projects_and_satisfies_without_changing_v1_v2() {
    let f = fixture();
    let request = json!({"schema_version":3,"request_id":"c3-one","operation":"project_runtime_manifest","document":f["document"],"options":f["wireOptions"]});
    let (out, r) = invoke(&serde_json::to_vec(&request).unwrap());
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    assert_eq!(out.stdout.last(), Some(&b'\n'));
    assert_eq!(r["producer"], "work-engine.manifest-compiler.rust-v1");
    assert_eq!(r["result"], f["oracle"]);
    for (i, c) in f["satisfactionCases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let req = json!({"schema_version":3,"request_id":format!("receipt-{i}"),"operation":"satisfy_runtime_requirements","manifest":f["oracle"],"role_id":c["roleId"],"requirements":c["requirements"],"skill_name":c["skillName"]});
        let (out, r) = invoke(&serde_json::to_vec(&req).unwrap());
        assert!(out.status.success());
        assert_eq!(r["result"], c["receipt"]);
    }
}
#[test]
fn v3_strict_refusals_have_correlated_error_and_no_result() {
    let f = fixture();
    let base = json!({"schema_version":3,"request_id":"bad","operation":"project_runtime_manifest","document":f["document"],"options":f["wireOptions"]});
    let mut unknown = base.clone();
    unknown["extra"] = json!(0);
    let mut invalid = base.clone();
    invalid["document"]["roles"]["slice-supervisor"]["contract"] = json!("wrong");
    let mut deep = base.clone();
    let mut nest = json!(null);
    for _ in 0..65 {
        nest = json!([nest])
    }
    deep["document"]["roles"]["slice-supervisor"]["unexpected"] = nest;
    for (req, code) in [
        (unknown, "invalid_request"),
        (invalid, "invalid_manifest"),
        (deep, "resource_limit"),
    ] {
        let (out, r) = invoke(&serde_json::to_vec(&req).unwrap());
        assert_eq!(out.status.code(), Some(2));
        assert_eq!(r["schema_version"], 3);
        assert_eq!(r["request_id"], "bad");
        assert_eq!(r["operation"], "project_runtime_manifest");
        assert_eq!(r["error"]["code"], code);
        assert!(r.get("result").is_none());
    }
    let duplicate=b"{\"schema_version\":3,\"request_id\":\"bad\",\"operation\":\"project_runtime_manifest\",\"document\":{},\"options\":{},\"options\":{}}";
    let (out, r) = invoke(duplicate);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(r["error"]["code"], "invalid_request");
    let (out, r) = invoke(b"{\"schema_version\":3}{}");
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(r["error"]["code"], "invalid_request");
}
