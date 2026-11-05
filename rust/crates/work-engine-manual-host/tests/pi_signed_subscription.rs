//! Public admission boundary, no native source or provider access.
use work_engine_manual_host::pi_docker::{Host, Job};
#[test]
fn public_job_json_cannot_supply_a_signed_owner() {
    // Even completely caller-chosen JSON cannot deserialize the opaque owner.
    // An ordinary Host request with the signed schema fails before opening paths.
    let artifact = serde_json::json!({"path":"/does-not-exist","sha256":"a".repeat(64)});
    let tree = serde_json::json!({"root":"/does-not-exist","manifest":artifact});
    let job:Job=serde_json::from_value(serde_json::json!({
        "schema":"work-engine.pi-docker-custody.v1","job_id":"untrusted-signed-job","state_dir":"/does-not-exist",
        "authority":artifact,"packet":artifact,"docker":artifact,"image":"untrusted","runtime":tree,"workspace":tree,
        "deadline_unix_seconds":1,"command_timeout_seconds":1,"idle_seconds":0,
        "coding":{"schema":"work-engine.pi-coding-signed-subscription.v1","shim":artifact,"toolchain":tree,"acceptance":artifact,"fixture_command":"","max_tool_calls":1,"workspace_write_boundary":"src_lib_rs"}
    })).unwrap();
    assert!(Host::admit(job).is_err());
}
