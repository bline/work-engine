//! The only opt-in task-free natural-exit executable. Intentionally not a
//! Server route, daemon, recovery command, or issuer.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn main() -> std::process::ExitCode {
    use work_engine_manual_host::pi_docker::init_only_ready::run_natural_exit_foreground;

    match run_natural_exit_foreground(std::env::args_os().skip(1)) {
        Ok(verdict) if verdict.qualified => {
            println!("qualified: original natural exit and terminal settlement");
            std::process::ExitCode::SUCCESS
        }
        Ok(_) | Err(_) => {
            // Do not export raw approval/evidence/error content out of custody,
            // or imply whether a durable spend/effect occurred on failed return.
            eprintln!(
                "not qualified; inspect protected Campaign and capture history; do not retry"
            );
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
fn main() -> std::process::ExitCode {
    eprintln!("private natural exit is unavailable on this platform");
    std::process::ExitCode::FAILURE
}
