use serde_json::{Value, json};
use work_engine_compiler::{project_runtime_manifest, satisfy_runtime_requirements};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/v1/manifest/projection-oracle.json")).unwrap()
}
#[test]
fn frozen_projection_and_all_six_satisfaction_receipts() {
    let f = fixture();
    let projected = project_runtime_manifest(&f["document"], &f["wireOptions"]).unwrap();
    assert_eq!(projected, f["oracle"]);
    for case in f["satisfactionCases"].as_array().unwrap() {
        let receipt = satisfy_runtime_requirements(
            &projected,
            case["roleId"].as_str().unwrap(),
            &case["requirements"],
            case["skillName"].as_str(),
        )
        .unwrap();
        assert_eq!(receipt, case["receipt"]);
    }
}
#[test]
fn refusal_precedes_receipt_and_identity_distinguishes_paths() {
    let f = fixture();
    let m = project_runtime_manifest(&f["document"], &f["wireOptions"]).unwrap();
    let c = &f["satisfactionCases"][0];
    let mut altered = c["requirements"].clone();
    altered["verified_sources"] = json!(false);
    assert_eq!(
        satisfy_runtime_requirements(&m, c["roleId"].as_str().unwrap(), &altered, None)
            .unwrap_err()
            .code,
        "requirements_unsatisfied"
    );
    let mut opts = f["wireOptions"].clone();
    opts["base_directory"] = json!("/fixture/another-delivery/app-server");
    let relocated = project_runtime_manifest(&f["document"], &opts).unwrap();
    assert_eq!(
        m["roles"]["slice-supervisor"]["runtimeEnvironmentRevision"],
        relocated["roles"]["slice-supervisor"]["runtimeEnvironmentRevision"]
    );
    assert_ne!(
        m["roles"]["slice-supervisor"]["roleContract"]["activatedPath"],
        relocated["roles"]["slice-supervisor"]["roleContract"]["activatedPath"]
    );
    opts["identity_base_directory"] = json!("/fixture/another-identity/app-server");
    let changed = project_runtime_manifest(&f["document"], &opts).unwrap();
    assert_ne!(
        m["roles"]["slice-supervisor"]["runtimeEnvironmentRevision"],
        changed["roles"]["slice-supervisor"]["runtimeEnvironmentRevision"]
    );
}
fn resign(v: &mut Value) {
    let mut unsigned = v.as_object().unwrap().clone();
    unsigned.remove("sha256");
    v["sha256"] = json!(work_engine_compiler::sha256_hex(
        work_engine_compiler::canonical_json(&Value::Object(unsigned)).as_bytes()
    ));
}
#[test]
fn requirement_refusals_cover_role_and_secondary_without_receipts() {
    let f = fixture();
    let m = f["oracle"].clone();
    let role = f["satisfactionCases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["roleId"] == "slice-supervisor" && v["skillName"].is_null())
        .unwrap();
    let req = &role["requirements"];
    let mut altered = req.clone();
    altered["sha256"] = json!("0".repeat(64));
    assert_eq!(
        satisfy_runtime_requirements(&m, "slice-supervisor", &altered, None)
            .unwrap_err()
            .code,
        "requirements_unsatisfied"
    );
    altered = req.clone();
    altered["verified_sources"] = json!(false);
    resign(&mut altered);
    assert_eq!(
        satisfy_runtime_requirements(&m, "slice-supervisor", &altered, None)
            .unwrap_err()
            .code,
        "requirements_unsatisfied"
    );
    altered = req.clone();
    altered["required_capabilities"] = json!(["never.granted"]);
    resign(&mut altered);
    assert_eq!(
        satisfy_runtime_requirements(&m, "slice-supervisor", &altered, None)
            .unwrap_err()
            .code,
        "requirements_unsatisfied"
    );
    altered = req.clone();
    altered["capability_ceiling"] = json!([]);
    resign(&mut altered);
    assert_eq!(
        satisfy_runtime_requirements(&m, "slice-supervisor", &altered, None)
            .unwrap_err()
            .code,
        "requirements_unsatisfied"
    );
    altered = req.clone();
    altered["continuity"] = json!("ephemeral");
    resign(&mut altered);
    assert_eq!(
        satisfy_runtime_requirements(&m, "slice-supervisor", &altered, None)
            .unwrap_err()
            .code,
        "requirements_unsatisfied"
    );
    altered = req.clone();
    altered["compiled_skill_sha256"] = json!("0".repeat(64));
    resign(&mut altered);
    assert_eq!(
        satisfy_runtime_requirements(&m, "slice-supervisor", &altered, None)
            .unwrap_err()
            .code,
        "requirements_unsatisfied"
    );
    altered = req.clone();
    altered["contract"]["kind"] = json!("skill");
    resign(&mut altered);
    assert_eq!(
        satisfy_runtime_requirements(&m, "slice-supervisor", &altered, None)
            .unwrap_err()
            .code,
        "requirements_unsatisfied"
    );
    altered = req.clone();
    altered["contract"]["path"] = json!("wrong");
    resign(&mut altered);
    assert_eq!(
        satisfy_runtime_requirements(&m, "slice-supervisor", &altered, None)
            .unwrap_err()
            .code,
        "requirements_unsatisfied"
    );
    altered = req.clone();
    altered["contract"]["must_be_activated"] = json!(false);
    resign(&mut altered);
    assert_eq!(
        satisfy_runtime_requirements(&m, "slice-supervisor", &altered, None)
            .unwrap_err()
            .code,
        "invalid_requirements"
    );
    let secondary = f["satisfactionCases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["skillName"] == "repo-search" && v["roleId"] == "slice-builder")
        .unwrap();
    let mut sm = m.clone();
    sm["roles"]["slice-builder"]["skills"]
        .as_array_mut()
        .unwrap()
        .retain(|s| s["name"] != "repo-search");
    assert_eq!(
        satisfy_runtime_requirements(
            &sm,
            "slice-builder",
            &secondary["requirements"],
            Some("repo-search")
        )
        .unwrap_err()
        .code,
        "requirements_unsatisfied"
    );
    sm = m.clone();
    let skill = sm["roles"]["slice-builder"]["skills"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["name"] == "repo-search")
        .unwrap();
    skill["capabilities"] = json!(["not.granted"]);
    assert_eq!(
        satisfy_runtime_requirements(
            &sm,
            "slice-builder",
            &secondary["requirements"],
            Some("repo-search")
        )
        .unwrap_err()
        .code,
        "requirements_unsatisfied"
    );
    sm = m.clone();
    let skill = sm["roles"]["slice-builder"]["skills"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["name"] == "repo-search")
        .unwrap();
    skill["effects"] = json!(["not.granted"]);
    assert_eq!(
        satisfy_runtime_requirements(
            &sm,
            "slice-builder",
            &secondary["requirements"],
            Some("repo-search")
        )
        .unwrap_err()
        .code,
        "requirements_unsatisfied"
    );
    let mut opaque = secondary["requirements"].clone();
    opaque["opaque_supported_field"] = json!({"\u{1f680}":2,"a":1});
    resign(&mut opaque);
    assert!(
        satisfy_runtime_requirements(&m, "slice-builder", &opaque, Some("repo-search")).is_ok()
    );
}
