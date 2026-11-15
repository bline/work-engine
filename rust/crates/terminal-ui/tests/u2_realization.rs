#![recursion_limit = "256"]
//! Reuse the accepted S8 controlled fixture constructor; no lifecycle/host semantics added.
#[allow(dead_code)]
#[path = "../../lifecycle-proof/tests/common/mod.rs"]
mod common;
#[test]
#[ignore = "requires U2_REALIZATION_DIR, S8_INSTALLED_HOST, TMPDIR and CARGO_TARGET_DIR"]
fn bind_current_controlled_template() {
    let root = std::path::PathBuf::from(std::env::var_os("U2_REALIZATION_DIR").unwrap());
    let build = common::preflight_installed_host();
    let fixture = common::create_fixture("http://127.0.0.1:9/v1", build.clone());
    let startup = fixture.startup_path.clone();
    let retained = fixture.root.keep();
    std::fs::write(root.join("template-binding.json"),serde_json::to_vec_pretty(&serde_json::json!({"startup":startup,"retained":retained,"build":build,"manifest":fixture.source_manifest,"sources":fixture.sources})).unwrap()).unwrap();
    println!("Template retained {}", startup.display());
}
