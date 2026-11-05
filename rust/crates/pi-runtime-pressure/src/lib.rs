//! Caller-owned composition of existing context evidence and opaque budget output.
pub mod stop_cases;
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};
use work_engine_manual_host::Artifact;
use work_engine_manual_host::pi_docker::coding::{
    PRESSURE_PROFILE, PressurePacket, PressureRelease,
};

pub fn compose(
    packet: &PressurePacket,
    barrier: &Value,
    budget: &str,
    observer: &Artifact,
) -> Result<PressureRelease, String> {
    let snapshot = &barrier["snapshot"];
    if snapshot["idle"] != true {
        return Err("recipient not idle".into());
    }
    let identity = json!({"source":packet.source,"session":barrier["sessionId"],"generation":packet.generation,"model":snapshot["model"]["id"],"window":snapshot["model"]["contextWindow"]});
    if !identity["session"].is_string()
        || !identity["model"].is_string()
        || identity["window"].as_u64().unwrap_or(0) == 0
    {
        return Err("missing identity/model/window".into());
    }
    let observed_ms = snapshot["observedMs"].as_u64().ok_or("clock missing")?;
    let usage = &snapshot["usage"];
    let tokens = match usage["availability"].as_str() {
        Some("undefined") => None,
        Some("estimate") if usage["value"]["tokens"].is_null() => None,
        Some("estimate") => Some(
            usage["value"]["tokens"]
                .as_u64()
                .ok_or("invalid estimate")?,
        ),
        _ => return Err("usage availability missing".into()),
    };
    let policy = json!({"schemaVersion":1,"unit":"basis_points","approaching":{"exit":4000,"enter":5000},"replacementCandidate":{"exit":6000,"enter":7000},"critical":{"exit":8000,"enter":9000}});
    let stamp =
        time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(observed_ms) * 1_000_000)
            .map_err(|e| e.to_string())?
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| e.to_string())?;
    let input = json!({"identity":identity,"observation_id":format!("{}:1",barrier["input_digest"].as_str().ok_or("input digest missing")?),"sequence":1,"observed_at":stamp,"observed_ms":observed_ms,"tokens":tokens,"provenance":"actual Pi1.5 getContextUsage SDK estimate; fake provider qualification"});
    let request = json!({"identity":identity,"max_age_ms":packet.max_age_ms,"now_ms":observed_ms,"policy":policy,"initial_disposition":"comfortable","records":[{"kind":"pi","payload":input}]});
    let metadata = std::fs::metadata(&observer.path).map_err(|e| e.to_string())?;
    if !observer.path.is_absolute()
        || !metadata.is_file()
        || metadata.len() > 128 * 1024 * 1024
        || work_engine_manual_host::digest(
            &std::fs::read(&observer.path).map_err(|e| e.to_string())?,
        ) != observer.sha256
    {
        return Err("observer executable identity/bound".into());
    }
    let mut child = Command::new(&observer.path)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let observer_pid = child.id();
    let write = child
        .stdin
        .take()
        .ok_or("observer stdin missing")?
        .write_all(
            serde_json::to_string(&request)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        );
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    write.map_err(|e| e.to_string())?;
    if !output.status.success() || output.stdout.len() > 24000 {
        return Err("accepted observer execution failed/bound".into());
    }
    let observed: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let context = observed[0]["result"].clone();
    if context.is_null() {
        return Err(format!("observer held: {}", observed[0]));
    }
    let payload = json!({"schema":PRESSURE_PROFILE,"authority":"observation_only","context":context,"observer_execution":{"pid":observer_pid,"exit_code":output.status.code(),"executable_sha256":observer.sha256,"stdout_sha256":work_engine_manual_host::digest(&output.stdout)},"observer_output_raw":String::from_utf8(output.stdout).map_err(|e|e.to_string())?,"sdk_usage":usage,"budget_control":{"producer":"metrics/scripts/usage_ledger.py control","raw":budget}});
    Ok(PressureRelease {
        schema: PRESSURE_PROFILE.into(),
        input_digest: barrier["input_digest"]
            .as_str()
            .ok_or("input missing")?
            .into(),
        container_id: barrier["container_id"]
            .as_str()
            .ok_or("container missing")?
            .into(),
        session_id: barrier["sessionId"]
            .as_str()
            .ok_or("session missing")?
            .into(),
        source: packet.source.clone(),
        generation: packet.generation.clone(),
        sequence: 1,
        observed_ms,
        pressure: serde_json::to_string(&payload).map_err(|e| e.to_string())?,
    })
}
