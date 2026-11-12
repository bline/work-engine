use research_register::read_host::*;
use std::{
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
static CANCEL: AtomicBool = AtomicBool::new(false);
extern "C" fn cancel(_: i32) {
    CANCEL.store(true, Ordering::SeqCst);
}
fn main() {
    let result = execute();
    match result {
        Ok(bytes) => {
            if std::io::stdout().write_all(&bytes).is_err() {
                std::process::exit(4);
            }
        }
        Err(reason) => {
            let refusal = serde_json::json!({"schema":"research-read-v1","outcome":"refused","reason":reason});
            println!("{refusal}");
            std::process::exit(4);
        }
    }
}
fn execute() -> Result<Vec<u8>, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--private-worker-v1"] {
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(16385)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 16384 {
            return Err("ControlInputLimit".into());
        }
        let input: WorkerInput = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let producer = input.producer.clone();
        return frame(&match worker(input) {
            Ok(report) => report,
            Err(reason) => {
                serde_json::json!({"outcome":"refused","producer":producer,"reason":reason})
            }
        });
    }
    if args.len() != 2 {
        return Err("usage: work-engine-research-read CGROUP_ROOT TRUSTED_PROFILE_JSON; request JSON on stdin".into());
    }
    let mut profile_bytes = Vec::new();
    std::fs::File::open(&args[1])
        .map_err(|e| e.to_string())?
        .take(8193)
        .read_to_end(&mut profile_bytes)
        .map_err(|e| e.to_string())?;
    if profile_bytes.len() > 8192 {
        return Err("ProfileInputLimit".into());
    }
    let profile: HostProfile = serde_json::from_slice(&profile_bytes).map_err(|e| e.to_string())?;
    profile.validate()?;
    #[cfg(target_os = "linux")]
    unsafe {
        libc::signal(libc::SIGINT, cancel as libc::sighandler_t);
        libc::signal(libc::SIGTERM, cancel as libc::sighandler_t);
    }
    let start = std::time::Instant::now();
    let bytes = acquire_input(start, profile.wall_millis)?;
    let request = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let input = WorkerInput {
        request: Some(request),
        profile,
        producer: producer(&exe)?,
        probe: None,
    };
    serde_json::to_vec(&run_started(
        &exe,
        Path::new(&args[0]),
        input,
        &CANCEL,
        start,
    )?)
    .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn acquire_input(start: std::time::Instant, wall_millis: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    loop {
        if CANCEL.load(Ordering::SeqCst) {
            return Err("Cancelled".into());
        }
        if start.elapsed() >= std::time::Duration::from_millis(wall_millis) {
            return Err("WallLimit".into());
        }
        let mut fd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut fd, 1, 5) };
        if ready < 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e.to_string());
        }
        if ready == 0 {
            continue;
        }
        let mut buf = [0u8; 4096];
        let n = unsafe { libc::read(libc::STDIN_FILENO, buf.as_mut_ptr().cast(), buf.len()) };
        if n < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        if n == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&buf[..n as usize]);
        if bytes.len() > 16384 {
            return Err("ControlInputLimit".into());
        }
    }
}
#[cfg(not(target_os = "linux"))]
fn acquire_input(_: std::time::Instant, _: u64) -> Result<Vec<u8>, String> {
    Err("ControlsUnavailable".into())
}
