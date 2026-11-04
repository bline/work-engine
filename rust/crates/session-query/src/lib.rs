//! Local, read-only client of the session-events MCP owner.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

unsafe extern "C" {
    fn killpg(pgrp: i32, sig: i32) -> i32;
}

const LIMIT: usize = 1024 * 1024;
const HELPER_SHA: &str = "98d62b54fa392c10a796f93aab51ecf0e7752e489dae8ceffbba7df319580e32";

/// Execute a whitelisted query. Grant material belongs in `arguments` read from a private input.
pub fn query(profile: Value, name: &str, arguments: Value) -> Result<Value, String> {
    let fields: &[&str] = match name {
        "list_sessions" => &["query", "host", "limit"],
        "read_session" => &[
            "session_id",
            "entry_id",
            "before",
            "after",
            "tail",
            "roles",
            "text_offset",
            "max_chars",
        ],
        "read_session_operation" => &["request_id", "grant_credential"],
        "read_session_result" => &["request_id", "grant_credential", "text_offset", "max_chars"],
        _ => return Err("invalid_query".into()),
    };
    let args = arguments.as_object().ok_or("invalid_arguments")?;
    if args.keys().any(|k| !fields.contains(&k.as_str())) {
        return Err("invalid_arguments".into());
    }
    let helper = profile
        .get("helper")
        .and_then(Value::as_str)
        .ok_or("invalid_profile")?;
    if !std::path::Path::new(helper).is_absolute() {
        return Err("invalid_profile".into());
    }
    let metadata = fs::symlink_metadata(helper).map_err(|_| "helper_unavailable")?;
    if !metadata.file_type().is_file() || metadata.len() > LIMIT as u64 {
        return Err("invalid_helper".into());
    }
    let bytes = fs::read(helper).map_err(|_| "helper_unavailable")?;
    if format!("{:x}", Sha256::digest(&bytes)) != HELPER_SHA {
        return Err("helper_pin_mismatch".into());
    }
    let payload = serde_json::to_vec(&json!({"profile":profile,"name":name,"arguments":arguments}))
        .map_err(|_| "invalid_request")?;
    if payload.len() > 256 * 1024 {
        return Err("input_too_large".into());
    }
    let mut child = Command::new("/usr/bin/python3")
        .arg("-I")
        .arg(helper)
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "helper_unavailable")?;
    let mut stdin = child.stdin.take().ok_or("transport_error")?;
    let mut stdout = child.stdout.take().ok_or("transport_error")?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = stdin.write_all(&payload);
    });
    thread::spawn(move || {
        let mut out = Vec::new();
        let result = stdout
            .by_ref()
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut out);
        let _ = tx.send((result, out));
    });
    let deadline = Instant::now() + Duration::from_secs(35);
    let mut output = None;
    let result = loop {
        if output.is_none() {
            if let Ok(v) = rx.try_recv() {
                output = Some(v);
            }
        }
        if let Some((_, ref data)) = output {
            if data.len() > LIMIT {
                break Err("output_too_large");
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let received = output
                    .take()
                    .or_else(|| rx.recv_timeout(Duration::from_millis(200)).ok());
                break match received {
                    Some((Ok(_), data)) if status.success() => {
                        serde_json::from_slice(&data).map_err(|_| "invalid_response")
                    }
                    _ => Err("owner_unavailable"),
                };
            }
            Err(_) => break Err("transport_error"),
            _ if Instant::now() >= deadline => break Err("timeout"),
            _ => thread::sleep(Duration::from_millis(10)),
        }
    };
    // Reap the owned helper and terminate descendants retaining its pipes.
    unsafe {
        killpg(child.id() as i32, 9);
    }
    let _ = child.kill();
    let _ = child.wait();
    result.map_err(str::to_string)
}
