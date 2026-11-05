//! Mechanical identity/effect-bound gate, not semantic acceptance.
use serde_json::{Value, json};
use work_engine_types::CodecContract;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("manifest required")?;
    let m: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let files = m["files"].as_array().ok_or("files required")?;
    if files.is_empty() {
        return Err("empty closure".into());
    }
    for f in files {
        let p = f["path"].as_str().ok_or("path")?;
        if f["sha256"]
            != CodecContract::BinaryArtifactV1
                .digest_binary(&std::fs::read(p)?)?
                .hex()
        {
            return Err(format!("changed input {p}").into());
        }
    }
    let j = &m["composition_job"];
    if j["image"] != "node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da"
        || j["network"] != "none"
        || j["provider"] != "disabled"
        || j["tools"] != json!([])
        || j["container_limit"] != 1
        || j["sdk_entries"] != 3
        || j["provider_fixture_calls"] != 5
        || j["actuator_limit"] != 3
        || j["profile"] != "pi-1.5.0-runtime-anchored-turns-v2"
        || j["host_placement"] != "outside_docker"
    {
        return Err("composition job refused".into());
    }
    println!(
        "{}",
        json!({"gate":"pass","files":files.len(),"composition_job":j})
    );
    Ok(())
}
