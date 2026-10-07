use crate::protocol::{ProtocolError, strict_json};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tempfile::Builder;
use work_engine_compiler::sha256_hex;

const MAX_PIPE: usize = 2 * 1024 * 1024;
const RUN_TIMEOUT: Duration = Duration::from_secs(30);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
fn bounded_test_limit(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0 && *value <= default)
        .unwrap_or(default)
}

fn io_error(code: &'static str, issue: impl std::fmt::Display) -> ProtocolError {
    ProtocolError::source(code, issue.to_string())
}
fn nonblock<F: std::os::fd::AsFd>(fd: &F) -> Result<(), ProtocolError> {
    let flags = fcntl_getfl(fd).map_err(|e| io_error("aeg_unavailable", e))?;
    fcntl_setfl(fd, flags | OFlags::NONBLOCK).map_err(|e| io_error("aeg_unavailable", e))
}
fn staged(path: &Path, bytes: &[u8]) -> Result<(), ProtocolError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| io_error("aeg_unavailable", e))?;
    file.write_all(bytes)
        .map_err(|e| io_error("aeg_unavailable", e))?;
    file.sync_all()
        .map_err(|e| io_error("aeg_unavailable", e))?;
    let check = std::fs::read(path).map_err(|e| io_error("aeg_unavailable", e))?;
    if check != bytes {
        return Err(ProtocolError::source(
            "source_mismatch",
            "staged AEG input digest differs",
        ));
    }
    Ok(())
}

pub struct AegResult {
    pub envelope: Value,
    pub observation: Value,
}

/// Owns the direct Python child through its observed exit, including all normal error paths.
pub fn run(
    python: &Path,
    script: &[u8],
    invariants: &[u8],
    environment: &[u8],
    request: &Value,
    cancelled: &AtomicBool,
) -> Result<AegResult, ProtocolError> {
    let run_timeout = Duration::from_millis(bounded_test_limit(
        "WORK_ENGINE_COMPILER_AEG_TIMEOUT_MS",
        RUN_TIMEOUT.as_millis() as usize,
    ) as u64);
    let max_pipe = bounded_test_limit("WORK_ENGINE_COMPILER_AEG_PIPE_LIMIT", MAX_PIPE);
    let stage = Builder::new()
        .prefix("work-engine-aeg-")
        .tempdir()
        .map_err(|e| io_error("aeg_unavailable", e))?;
    let script_path = stage.path().join("owner.py");
    let invariants_path = stage.path().join("invariants.md");
    let environment_path = stage.path().join("environment.yaml");
    staged(&script_path, script)?;
    staged(&invariants_path, invariants)?;
    staged(&environment_path, environment)?;
    let input = serde_json::to_vec(request).map_err(|e| io_error("aeg_protocol", e))?;
    let mut child = Command::new(python)
        .arg("-I")
        .arg("-B")
        .arg(&script_path)
        .arg("project-role-machine")
        .arg("--invariants")
        .arg(&invariants_path)
        .arg("--environments")
        .arg(&environment_path)
        .env_clear()
        .env("PYTHONUTF8", "1")
        .env("LANG", "C.UTF-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| io_error("aeg_unavailable", e))?;
    let mut stdin = Some(child.stdin.take().expect("piped stdin"));
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    if let Err(error) = nonblock(stdin.as_ref().unwrap())
        .and_then(|_| nonblock(&stdout))
        .and_then(|_| nonblock(&stderr))
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let mut sent = 0usize;
    let mut input_open = true;
    let mut out_open = true;
    let mut err_open = true;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let start = Instant::now();
    let mut failure: Option<ProtocolError> = None;
    let mut cleanup_started: Option<Instant> = None;
    let mut exit = None;
    loop {
        if failure.is_none() && cancelled.load(Ordering::Relaxed) {
            failure = Some(ProtocolError::source(
                "cancelled",
                "compiler received cancellation",
            ));
        }
        if failure.is_none() && start.elapsed() >= run_timeout {
            failure = Some(ProtocolError::source(
                "timeout",
                "Python projection timed out",
            ));
        }
        if failure.is_some() && cleanup_started.is_none() {
            cleanup_started = Some(Instant::now());
            input_open = false;
            let _ = child.kill();
        }
        if exit.is_none() {
            exit = child
                .try_wait()
                .map_err(|e| io_error("cleanup_unconfirmed", e))?;
        }
        if let Some(cleanup_start) = cleanup_started {
            if exit.is_some() {
                break;
            }
            if cleanup_start.elapsed() >= CLEANUP_TIMEOUT {
                return Err(ProtocolError::source(
                    "cleanup_unconfirmed",
                    "Python direct-child exit unobserved",
                ));
            }
        } else if exit.is_some() && !out_open && !err_open {
            break;
        }
        let (ready_in, ready_out, ready_err) = {
            let mut slots = Vec::new();
            let mut input_index = None;
            let mut output_index = None;
            let mut error_index = None;
            if input_open {
                input_index = Some(slots.len());
                slots.push(PollFd::new(stdin.as_ref().unwrap(), PollFlags::OUT));
            }
            if out_open {
                output_index = Some(slots.len());
                slots.push(PollFd::new(&stdout, PollFlags::IN));
            }
            if err_open {
                error_index = Some(slots.len());
                slots.push(PollFd::new(&stderr, PollFlags::IN));
            }
            let timeout = Timespec {
                tv_sec: 0,
                tv_nsec: 100_000_000,
            };
            if let Err(e) = poll(&mut slots, Some(&timeout))
                && e != rustix::io::Errno::INTR
                && failure.is_none()
            {
                failure = Some(io_error("aeg_failed", e));
            }
            (
                input_index.is_some_and(|i| !slots[i].revents().is_empty()),
                output_index.is_some_and(|i| !slots[i].revents().is_empty()),
                error_index.is_some_and(|i| !slots[i].revents().is_empty()),
            )
        };
        if ready_in && input_open {
            if sent == input.len() {
                input_open = false;
            } else {
                match stdin.as_mut().unwrap().write(&input[sent..]) {
                    Ok(0) => {
                        failure.get_or_insert_with(|| {
                            ProtocolError::source("aeg_failed", "Python stdin closed early")
                        });
                        input_open = false;
                    }
                    Ok(count) => {
                        sent += count;
                        if sent == input.len() {
                            input_open = false;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => {
                        failure.get_or_insert_with(|| io_error("aeg_failed", e));
                        input_open = false;
                    }
                }
            }
        }
        if !input_open {
            stdin.take();
        }
        if ready_out && out_open {
            let mut chunk = [0u8; 8192];
            match stdout.read(&mut chunk) {
                Ok(0) => out_open = false,
                Ok(count) => {
                    if out.len() + count > max_pipe {
                        failure.get_or_insert_with(|| {
                            ProtocolError::source("resource_limit", "Python stdout exceeds limit")
                        });
                    } else {
                        out.extend_from_slice(&chunk[..count]);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    failure.get_or_insert_with(|| io_error("aeg_failed", e));
                    out_open = false;
                }
            }
        }
        if ready_err && err_open {
            let mut chunk = [0u8; 8192];
            match stderr.read(&mut chunk) {
                Ok(0) => err_open = false,
                Ok(count) => {
                    if err.len() + count > max_pipe {
                        failure.get_or_insert_with(|| {
                            ProtocolError::source("resource_limit", "Python stderr exceeds limit")
                        });
                    } else {
                        err.extend_from_slice(&chunk[..count]);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    failure.get_or_insert_with(|| io_error("aeg_failed", e));
                    err_open = false;
                }
            }
        }
    }
    let exit = match exit {
        Some(status) => status,
        None => child
            .wait()
            .map_err(|e| io_error("cleanup_unconfirmed", e))?,
    };
    if let Some(issue) = failure {
        return Err(issue);
    }
    if sent != input.len() {
        return Err(ProtocolError::source(
            "aeg_failed",
            "Python did not consume complete request",
        ));
    }
    if !exit.success() {
        return Err(ProtocolError::source(
            "aeg_failed",
            format!("Python exit {:?}", exit.code()),
        ));
    }
    if !err.is_empty() {
        return Err(ProtocolError::source(
            "aeg_failed",
            "Python wrote stderr on success",
        ));
    }
    let envelope = strict_json(&out).map_err(|_| {
        ProtocolError::source(
            "aeg_protocol",
            "Python returned malformed or duplicate-key JSON",
        )
    })?;
    Ok(AegResult {
        observation: serde_json::json!({"direct_child_reaped":true,"exit_code":exit.code(),"script_sha256":sha256_hex(script)}),
        envelope,
    })
}
