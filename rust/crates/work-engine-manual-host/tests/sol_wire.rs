//! Opt-in use of existing pinned Pi assets; never reads native credential files.
#[test]
#[ignore = "requires explicit WORK_ENGINE_PI_ROOT pointing to the pinned existing Pi1.5.0 tree"]
fn actual_sol_high_adapter_serializes_model_effort_and_payer() {
    let root = std::env::var("WORK_ENGINE_PI_ROOT").expect("explicit pinned Pi root");
    let out = std::process::Command::new("node")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/sol-wire.mjs"
        ))
        .arg(root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("node capture");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let capture: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(capture["provider_execution"], false);
    assert_eq!(capture["request"]["body"]["reasoning"]["effort"], "high");
    assert!(capture["request"]["headers"].get("authorization").is_none());
    println!("{}", String::from_utf8_lossy(&out.stdout));
}
