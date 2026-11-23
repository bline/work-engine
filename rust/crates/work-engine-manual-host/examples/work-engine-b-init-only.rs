//! Opt-in Linux init-only subject. Do not run on the development host.

#[path = "../src/pi_init_only_ready/artifact_contract.rs"]
mod contract;

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    use contract::*;

    fn run() -> bool {
        let mut uid = [0; 3];
        let mut gid = [0; 3];
        let mut groups = [0];
        // SAFETY: all kernel out pointers reference live, correctly sized
        // storage; prctl options below take only integer arguments.
        unsafe {
            if libc::getpid() != 1
                || libc::getresuid(&mut uid[0], &mut uid[1], &mut uid[2]) != 0
                || uid != [USER; 3]
                || libc::getresgid(&mut gid[0], &mut gid[1], &mut gid[2]) != 0
                || gid != [GROUP; 3]
            {
                return false;
            }
            let n = libc::getgroups(1, groups.as_mut_ptr());
            if !(0..=1).contains(&n)
                || !reduced_groups(&groups[..n as usize])
                || libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) != 1
            {
                return false;
            }
            let dumpable = libc::prctl(libc::PR_GET_DUMPABLE, 0, 0, 0, 0);
            if dumpable < 0
                || (dumpable != 0 && libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) != 0)
                || libc::prctl(libc::PR_GET_DUMPABLE, 0, 0, 0, 0) != 0
            {
                return false;
            }
            let mask = libc::mode_t::from(PRIVATE_UMASK);
            libc::umask(mask);
            if libc::umask(mask) != mask {
                return false;
            }

            // One short nonblocking write, with no retry on error/backpressure.
            // The closed OCI launch connects fd 1 to the exclusive stdout pipe.
            let fd = libc::STDOUT_FILENO;
            let flags = libc::fcntl(fd, libc::F_GETFL);
            if flags < 0 || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) != 0 {
                return false;
            }
            let mut deadline = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            if libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut deadline) != 0 {
                return false;
            }
            let Some(stop) = deadline.tv_sec.checked_add(HOLD_SECONDS as libc::time_t) else {
                return false;
            };
            deadline.tv_sec = stop;
            if libc::write(fd, CHECKPOINT.as_ptr().cast(), CHECKPOINT.len())
                != CHECKPOINT.len() as libc::ssize_t
            {
                return false;
            }
            loop {
                match libc::clock_nanosleep(
                    libc::CLOCK_MONOTONIC,
                    libc::TIMER_ABSTIME,
                    &deadline,
                    std::ptr::null_mut(),
                ) {
                    0 => return true,
                    libc::EINTR => continue,
                    _ => return false,
                }
            }
        }
    }

    // A fixed argv is checked at materialization, not parsed at runtime.
    const _: () = assert!(ENTRYPOINT.as_bytes()[0] == b'/');
    if run() {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    std::process::ExitCode::FAILURE
}
