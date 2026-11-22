//! Fixed public staging init. Nothing reads ingress or executes a successor.

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    const USER: libc::uid_t = 1000;
    const GROUP: libc::gid_t = 1000;
    let mut supplementary = 0;
    // Fail closed if not already reduced by the container. This program
    // doesn't accept privileges so that it can drop them for a later stage.
    let reduced = unsafe {
        let groups = libc::getgroups(1, &mut supplementary);
        libc::getpid() == 1
            && libc::getuid() == USER
            && libc::geteuid() == USER
            && libc::getgid() == GROUP
            && libc::getegid() == GROUP
            && (groups == 0 || (groups == 1 && supplementary == GROUP))
            && libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) == 1
            && libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) == 0
    };
    if !reduced {
        return std::process::ExitCode::FAILURE;
    }
    unsafe { libc::umask(0o077) };
    // A lost Host cannot turn the hold into an unbounded running init.
    std::thread::sleep(std::time::Duration::from_secs(30));
    std::process::ExitCode::SUCCESS
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    std::process::ExitCode::FAILURE
}
