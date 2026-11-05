use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    os::unix::{io::AsRawFd, process::CommandExt},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
const OWNER_SHA: &str = "470a625b76d17a62a68659709d99d2767c944441be4cd2ddbff96bc6be12a12d";
const OWNER_LIMIT: usize = 512 * 1024;
const OUTPUT_LIMIT: usize = 1024 * 1024;
const HELPER: &str = include_str!("../command_owner.py");
pub fn submit(profile: &session_query::Profile, args: Value) -> Result<Value, &'static str> {
    profile.validate()?;
    if profile.bridge.is_none() {
        return Err("bridge_unavailable");
    }
    let owner = session_query::read_file(&profile.owner, OWNER_LIMIT, false)
        .map_err(|_| "owner_unavailable")?;
    if format!("{:x}", Sha256::digest(&owner)) != OWNER_SHA {
        return Err("owner_pin_mismatch");
    }
    let source = String::from_utf8(owner).map_err(|_| "owner_unavailable")?;
    let payload = serde_json::to_vec(&json!({"profile":profile,"name":"submit_session_request","arguments":args,"owner_source":source})).map_err(|_| "invalid_arguments")?;
    if payload.len() > OWNER_LIMIT + session_query::INPUT_LIMIT {
        return Err("input_too_large");
    }
    transport(&payload, Duration::from_secs(35), HELPER)
}
fn transport(payload: &[u8], timeout: Duration, program: &str) -> Result<Value, &'static str> {
    let mut child = Command::new("/usr/bin/python3")
        .args(["-I", "-c", program])
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "owner_unavailable")?;
    let mut stdin = child.stdin.take();
    let mut stdout = child.stdout.take().unwrap();
    let set_nonblock = |fd| unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) } == 0;
    if !set_nonblock(stdin.as_ref().unwrap().as_raw_fd()) || !set_nonblock(stdout.as_raw_fd()) {
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let _ = child.wait();
        return Err("owner_unavailable");
    }
    let deadline = Instant::now() + timeout;
    let mut sent = 0;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 16384];
    let mut drain = |stdout: &mut std::process::ChildStdout| -> Result<(), &'static str> {
        loop {
            match stdout.read(&mut chunk) {
                Ok(0) => return Ok(()),
                Ok(n) => {
                    bytes.extend_from_slice(&chunk[..n]);
                    if bytes.len() > OUTPUT_LIMIT {
                        return Err("output_too_large");
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err("owner_unavailable"),
            }
        }
    };
    let status = loop {
        if let Err(e) = drain(&mut stdout) {
            break Err(e);
        }
        if let Some(pipe) = stdin.as_mut() {
            match pipe.write(&payload[sent..]) {
                Ok(n) => {
                    sent += n;
                    if sent == payload.len() {
                        stdin.take();
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(_) => break Err("owner_unavailable"),
            }
        }
        // Observe exit without reaping: the process-group id cannot be reused before cleanup.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let observed = unsafe {
            libc::waitid(
                libc::P_PID,
                child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if observed != 0 {
            break Err("owner_unavailable");
        }
        if unsafe { info.si_pid() } != 0 {
            break if sent == payload.len()
                && info.si_code == libc::CLD_EXITED
                && unsafe { info.si_status() } == 0
            {
                Ok(())
            } else {
                Err("owner_unavailable")
            };
        }
        if Instant::now() >= deadline {
            break Err("timeout");
        }
        thread::sleep(Duration::from_millis(10));
    };
    // Kill the owned group while its leader is still unreaped. No background I/O workers exist.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
    let final_drain = drain(&mut stdout);
    status?;
    final_drain?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "owner_unavailable")?;
    if let Some(reason) = value.get("error").and_then(Value::as_str) {
        return Err(match reason {
            "query_rejected" => "query_rejected",
            "unauthorized" => "unauthorized",
            "history_unavailable" => "history_unavailable",
            "bridge_unavailable" => "bridge_unavailable",
            "identity_conflict" => "identity_conflict",
            "expired" => "expired",
            "target_mismatch" => "target_mismatch",
            "payload_conflict" => "payload_conflict",
            "session_busy" => "session_busy",
            "entry_uncertain" => "entry_uncertain",
            "custody_lost" => "custody_lost",
            _ => "owner_unavailable",
        });
    }
    value.get("result").cloned().ok_or("owner_unavailable")
}
