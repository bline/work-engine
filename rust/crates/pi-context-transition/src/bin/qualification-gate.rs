use serde_json::Value;
use std::{fs, path::Path};
use work_engine_types::CodecContract;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("manifest required")?;
    let manifest: Value = serde_json::from_slice(&fs::read(path)?)?;
    let files = manifest["files"].as_array().ok_or("files required")?;
    for item in files {
        let p = Path::new(item["path"].as_str().ok_or("path required")?);
        let d = CodecContract::BinaryArtifactV1
            .digest_binary(&fs::read(p)?)?
            .hex();
        if item["sha256"] != d {
            return Err(format!("digest mismatch: {}", p.display()).into());
        }
    }
    let job = &manifest["job"];
    if job["image"]
        != "node@sha256:4b150b48b4116e2a22f2c39cc27025fbf4f153e2078fd3b14d159fdb5c7279da"
        || job["network"] != "none"
        || job["provider"] != "disabled"
        || job["tools"] != serde_json::json!([])
        || job["actuator_limit"] != 1
        || job["provider_fixture_calls"] != 2
        || job["profile"] != "pi-1.5.0-runtime-fresh-session-v1"
    {
        return Err("job binding rejected".into());
    }
    println!(
        "{}",
        serde_json::json!({"gate":"pass","files":files.len(),"job":job})
    );
    Ok(())
}
