//! The only MCP executable in the RP0 Claude profile. This is a narrow child
//! launcher, not an MCP proxy: after installing a kernel filter it execs CBM.
//! The filter is inherited by CBM's detached daemon and index workers.

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
compile_error!("the qualified RP0 CBM launcher requires Linux x86_64");

use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy)]
struct SockFilter {
    code: u16,
    jt: u8,
    jf: u8,
    k: u32,
}

#[repr(C)]
struct SockFprog {
    len: u16,
    filter: *const SockFilter,
}

unsafe extern "C" {
    fn prctl(option: i32, ...) -> i32;
    fn syscall(number: i64, ...) -> i64;
}

const PR_SET_NO_NEW_PRIVS: i32 = 38;
const PR_SET_SECCOMP: i32 = 22;
const SECCOMP_MODE_FILTER: i32 = 2;
const SYS_SOCKET: u32 = 41;
const SYS_SOCKETPAIR: u32 = 53;
const SYS_IO_URING_SETUP: u32 = 425;
const SYS_CLOSE_RANGE: i64 = 436;
const AF_UNIX: u32 = 1;
const AUDIT_ARCH_X86_64: u32 = 0xc000_003e;
const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
const EPERM: u32 = 1;

const fn stmt(code: u16, k: u32) -> SockFilter {
    SockFilter {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}

const fn jump(code: u16, k: u32, jt: u8, jf: u8) -> SockFilter {
    SockFilter { code, jt, jf, k }
}

// Offsets are the stable Linux seccomp_data ABI: nr=0, arch=4, args[0]=16.
// Only AF_UNIX sockets are permitted. The private CBM daemon uses AF_UNIX IPC;
// Internet, packet and netlink sockets are unavailable, even to its children.
// io_uring_setup is denied so an asynchronous socket opcode cannot evade the
// socket syscall filter. Unknown architectures and x32 syscall numbers fail.
const FILTER: [SockFilter; 15] = [
    stmt(0x20, 4),                         // LD W ABS arch
    jump(0x15, AUDIT_ARCH_X86_64, 1, 0),   // JEQ arch
    stmt(0x06, SECCOMP_RET_KILL_PROCESS),  // RET kill
    stmt(0x20, 0),                         // LD W ABS nr
    jump(0x35, 0x4000_0000, 0, 1),         // JGE x32 bit
    stmt(0x06, SECCOMP_RET_ERRNO | EPERM), // RET EPERM
    jump(0x15, SYS_SOCKET, 3, 0),          // JEQ socket -> domain
    jump(0x15, SYS_SOCKETPAIR, 2, 0),      // JEQ socketpair -> domain
    jump(0x15, SYS_IO_URING_SETUP, 5, 0),  // JEQ io_uring -> deny
    stmt(0x06, SECCOMP_RET_ALLOW),         // RET allow
    stmt(0x20, 16),                        // LD W ABS args[0]
    jump(0x15, AF_UNIX, 1, 0),             // JEQ AF_UNIX
    stmt(0x06, SECCOMP_RET_ERRNO | EPERM), // RET EPERM
    stmt(0x06, SECCOMP_RET_ALLOW),         // RET allow
    stmt(0x06, SECCOMP_RET_ERRNO | EPERM), // RET EPERM
];

fn install_filter() -> io::Result<()> {
    // SAFETY: close_range is passed fixed scalar arguments and closes only
    // inherited descriptors beyond stdio. No borrowed Rust descriptor lives
    // here. An unsupported syscall is a qualification failure.
    if unsafe { syscall(SYS_CLOSE_RANGE, 3u32, u32::MAX, 0u32) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: prctl receives ABI-defined scalar arguments. The filter array is
    // static and the kernel copies the program before the call returns.
    if unsafe { prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let program = SockFprog {
        len: FILTER.len() as u16,
        filter: FILTER.as_ptr(),
    };
    // SAFETY: `program` and its static filters remain valid for this call.
    if unsafe { prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &program, 0, 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn run() -> io::Result<()> {
    let probe = std::env::args_os()
        .nth(1)
        .is_some_and(|a| a == "--probe-network");
    if std::env::args_os().len() != if probe { 2 } else { 1 } {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unexpected launcher argument",
        ));
    }
    install_filter()?;
    if probe {
        let denied = unsafe { syscall(SYS_SOCKET as i64, 2i32, 1i32, 0i32) };
        let internet_error = io::Error::last_os_error();
        let unix = unsafe { syscall(SYS_SOCKET as i64, 1i32, 1i32, 0i32) };
        if denied != -1 || internet_error.raw_os_error() != Some(1) || unix < 0 {
            return Err(io::Error::other(
                "network filter did not deny Internet and retain Unix IPC",
            ));
        }
        println!("internet_socket_denied=true unix_socket_allowed=true");
        return Ok(());
    }
    let error = Command::new("/cbm/bin")
        .arg("--tool-profile=analysis")
        .exec();
    Err(error)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("RP0 CBM launcher refused: {error}");
        std::process::exit(1);
    }
}
