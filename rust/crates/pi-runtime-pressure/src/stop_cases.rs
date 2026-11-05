//! Qualification observations only; Host retains every effect and deadline.
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use work_engine_manual_host::{digest, pi_docker::request_cancel};
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn boot_seconds() -> Result<f64, String> {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) } != 0 {
        return Err("boot clock unavailable".into());
    }
    Ok(t.tv_sec as f64 + t.tv_nsec as f64 / 1e9)
}
pub fn tool_age(process: &Value) -> Result<f64, String> {
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if hz <= 0 {
        return Err("clock ticks unavailable".into());
    }
    let ticks = process["start"]
        .as_str()
        .ok_or("tool start missing")?
        .parse::<u64>()
        .map_err(|e| e.to_string())?;
    Ok(boot_seconds()? - ticks as f64 / hz as f64)
}
pub fn case_name(mode: Option<&str>) -> Option<&'static str> {
    match mode {
        Some("--cancel-after-pressure") => Some("cancel"),
        Some("--deadline-after-pressure") => Some("deadline"),
        _ => None,
    }
}
pub fn release_due(case: &str, deadline: u64, now: u64) -> bool {
    case != "deadline" || deadline.saturating_mul(1000).saturating_sub(now) <= 25000
}
pub fn classify(case: &str, j: &Value, r: &Value, w: &Value) -> Result<(), String> {
    if r["receipt"]["status"] != "not_established"
        || j["phase"] != "collected"
        || j["removed"] != true
        || j["coding"]["tool_removed"] != true
        || !j["coding"]["final_acceptance"].is_null()
    {
        return Err("stop case completed or uncollected".into());
    }
    if w["native_request_sha256"].as_str().unwrap_or("").len() != 64
        || w["native_message_sha256"].as_str().unwrap_or("").len() != 64
        || w["active_tool_stdout_sha256"].as_str().unwrap_or("").len() != 64
    {
        return Err("pressure/tool proof missing".into());
    }
    let observed = w["observed_ms"].as_u64().ok_or("witness clock")?;
    let deadline = j["job"]["deadline_unix_seconds"]
        .as_u64()
        .ok_or("deadline")?
        * 1000;
    if observed >= deadline {
        return Err("activity was already expired".into());
    }
    if case == "cancel" {
        let cancelled = w["cancel_requested_ms"].as_u64().ok_or("cancel absent")?;
        if cancelled < observed || cancelled >= deadline
            || w["cancel_ack"]["status"] != "cancellation_requested"
            || !w["cancel_tool_age_seconds"].as_f64().is_some_and(|age| (0.0..30.0).contains(&age))
            || j["processes"][w["tool_process_index"].as_u64().ok_or("tool index")? as usize]["timed_out"] != false
            || !j["failures"].as_array().is_some_and(|f| f.iter().any(|v| v.as_str() == Some("cancellation requested")))
        {
            return Err("cancel order".into());
        }
    } else if case == "deadline" {
        let finished = w["tool_finished_observed_ms"]
            .as_u64()
            .ok_or("tool exit absent")?;
        let age = w["tool_exit_age_seconds"]
            .as_f64()
            .ok_or("tool age absent")?;
        if finished < deadline || age >= 30.0 || age < 0.0 || !w["cancel_requested_ms"].is_null() {
            return Err("job deadline not distinguished from command timeout".into());
        }
    } else {
        return Err("unknown case".into());
    }
    Ok(())
}
pub fn observe_stop(
    case: &str,
    dir: &Path,
    release: &str,
    session: &str,
    deadline: u64,
    done: &std::sync::atomic::AtomicBool,
) -> Result<Value, String> {
    let until = Instant::now() + Duration::from_secs(100);
    loop {
        if done.load(std::sync::atomic::Ordering::Acquire) || Instant::now() >= until {
            return Err("active post-pressure proof unavailable".into());
        }
        let request = fs::read(dir.join("coding-pressure-request.json"));
        let message = fs::read(dir.join("coding-pressure-message.json"));
        if let (Ok(request), Ok(message)) = (request, message) {
            let p: Value = serde_json::from_slice(&request).map_err(|e| e.to_string())?;
            let m: Value = serde_json::from_slice(&message).map_err(|e| e.to_string())?;
            let contains = |v: &Value| {
                v.as_array().is_some_and(|messages| {
                    messages.iter().any(|m| {
                        m["content"]
                            .as_array()
                            .is_some_and(|c| c.iter().any(|c| c["text"] == release))
                    })
                })
            };
            if p["sessionId"] != session
                || m["sessionId"] != session
                || !contains(&p["context"]["messages"])
                || !contains(&m["messages"])
            {
                return Err("native pressure identity/content conflict".into());
            }
            let j: Value = serde_json::from_slice(
                &fs::read(dir.join("pi-state.json")).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if j["phase"] == "collected" || j["coding"]["completed"] == true {
                return Err("already completed before stop".into());
            }
            if let Some((index, process)) = j["processes"].as_array().and_then(|p| {
                p.iter().enumerate().find(|(_, p)| {
                    p["argv"]
                        .as_array()
                        .is_some_and(|a| a.get(1).is_some_and(|s| s == "exec"))
                })
            }) {
                let stdout = fs::read(process["stdout"].as_str().ok_or("tool stdout")?)
                    .map_err(|e| e.to_string())?;
                if String::from_utf8_lossy(&stdout).contains("PRESSURE_STOP_TOOL_ACTIVE")
                    && process["status"].is_null()
                    && process["stdout_sha256"].is_null()
                {
                    let observed = now_ms();
                    if observed >= deadline * 1000 {
                        return Err("pressure activity too late".into());
                    }
                    let mut witness = json!({"case":case,"observed_ms":observed,"session_id":session,"native_request_sha256":digest(&request),"native_message_sha256":digest(&message),"active_tool_stdout_sha256":digest(&stdout),"tool_process_index":index,"tool_process_at_observation":process,"deadline_unix_seconds":deadline});
                    fs::write(
                        dir.join("stop-witness-initial.json"),
                        serde_json::to_vec_pretty(&witness).unwrap(),
                    )
                    .map_err(|e| e.to_string())?;
                    if case == "cancel" {
                        witness["cancel_ack"] = request_cancel(dir).map_err(|e| e.to_string())?;
                        witness["cancel_requested_ms"] = json!(now_ms());
                        witness["cancel_tool_age_seconds"] = json!(tool_age(process)?);
                    } else {
                        loop {
                            if Instant::now() >= until {
                                return Err("tool exit not observed".into());
                            }
                            let latest: Value = serde_json::from_slice(
                                &fs::read(dir.join("pi-state.json")).map_err(|e| e.to_string())?,
                            )
                            .map_err(|e| e.to_string())?;
                            let current = &latest["processes"][index];
                            if !current["stdout_sha256"].is_null() {
                                witness["tool_finished_observed_ms"] = json!(now_ms());
                                witness["tool_exit_age_seconds"] = json!(tool_age(current)?);
                                witness["tool_status"] = current["status"].clone();
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }
                    fs::write(
                        dir.join("stop-witness.json"),
                        serde_json::to_vec_pretty(&witness).unwrap(),
                    )
                    .map_err(|e| e.to_string())?;
                    return Ok(witness);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
