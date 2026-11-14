//! Restricted fixed-fixture process, not a native SDK launcher. No candidate input.
#[path = "../src/pi_fifo.rs"]
mod fifo;
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    os::fd::AsRawFd,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
};
unsafe extern "C" {
    fn dup2(old: i32, new: i32) -> i32;
    fn close_range(first: u32, last: u32, flags: i32) -> i32;
    fn prctl(option: i32, ...) -> i32;
    fn syscall(number: i64, ...) -> i64;
}
#[repr(C)]
struct Filter {
    code: u16,
    jt: u8,
    jf: u8,
    k: u32,
}
#[repr(C)]
struct Program {
    len: u16,
    filter: *const Filter,
}
fn restrict() -> std::io::Result<()> {
    // Exact x86_64 whitelist after all file opens. No open/exec/clone/prctl/socket/
    // ptrace/pidfd/io_uring/signal delivery path can recover original daemon IO.
    let mut filters = vec![
        Filter {
            code: 0x20,
            jt: 0,
            jf: 0,
            k: 4,
        },
        Filter {
            code: 0x15,
            jt: 1,
            jf: 0,
            k: 0xc000003e,
        },
        Filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0x80000000,
        },
        Filter {
            code: 0x20,
            jt: 0,
            jf: 0,
            k: 0,
        },
    ];
    for n in [
        0, 1, 3, 5, 7, 9, 10, 11, 12, 13, 14, 15, 24, 25, 28, 35, 60, 202, 219, 228, 230, 231, 271,
    ] {
        filters.push(Filter {
            code: 0x15,
            jt: 0,
            jf: 1,
            k: n,
        });
        filters.push(Filter {
            code: 0x06,
            jt: 0,
            jf: 0,
            k: 0x7fff0000,
        });
    }
    filters.push(Filter {
        code: 0x06,
        jt: 0,
        jf: 0,
        k: 0x00050001,
    }); // EPERM; never diagnostic raw bytes.
    let p = Program {
        len: filters.len() as u16,
        filter: filters.as_ptr(),
    };
    if unsafe { prctl(4, 0, 0, 0, 0) } != 0
        || unsafe { prctl(3, 0, 0, 0, 0) } != 0
        || unsafe { prctl(38, 1, 0, 0, 0) } != 0
        || unsafe { prctl(22, 2, &p as *const Program) } != 0
    {
        return Err(std::io::Error::other("fixed_protection_refusal"));
    }
    Ok(())
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let input = OpenOptions::new()
        .read(true)
        .custom_flags(0x20000)
        .open("/control/in")?; // O_NOFOLLOW
    let output = OpenOptions::new()
        .write(true)
        .custom_flags(0x20000)
        .open("/control/out")?;
    for f in [&input, &output] {
        let m = f.metadata()?;
        if !m.file_type().is_fifo() || m.mode() & 0o777 != 0o600 || m.uid() != 1000 {
            return Err("fixed_fifo_identity".into());
        }
    }
    let null = OpenOptions::new().write(true).open("/dev/null")?;
    if unsafe { dup2(input.as_raw_fd(), 0) } < 0
        || unsafe { dup2(output.as_raw_fd(), 1) } < 0
        || unsafe { dup2(null.as_raw_fd(), 2) } < 0
    {
        return Err("fixed_dup_refusal".into());
    }
    // Forget handles BEFORE close_range so Drop cannot close a reused descriptor.
    std::mem::forget(input);
    std::mem::forget(output);
    std::mem::forget(null);
    if unsafe { close_range(3, u32::MAX, 0) } != 0 {
        return Err("fixed_close_refusal".into());
    }
    // Host independently inspects the final descriptor/mount/namespace state
    // while this pinned bootstrap waits, before dump protection hides proc FDs.
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(b"FIFO_BOUND_V1\n")?;
    stdout.flush()?;
    let mut admission = [0u8; 1];
    stdin.read_exact(&mut admission)?;
    if admission != *b"P" {
        return Err("fixed_bootstrap_refusal".into());
    }
    restrict()?;
    // Negative controls execute after replacement/filter: open/exec/clone/prctl/socket must fail.
    for (nr, a, b) in [
        (257, -100, 0),
        (59, 0, 0),
        (56, 17, 0),
        (157, 4, 1),
        (41, 1, 1),
    ] {
        if unsafe { syscall(nr, a as i64, b as i64, 0i64, 0i64, 0i64, 0i64) } != -1
            || std::io::Error::last_os_error().raw_os_error() != Some(1)
        {
            return Err("fixed_filter_refusal".into());
        }
    }
    stdout.write_all(b"FIFO_READY_V1\n")?;
    stdout.flush()?;
    let mut codec = fifo::Codec::new(fifo::FIXTURE);
    let mut buffer = [0u8; fifo::MAX_FRAME];
    loop {
        let mut size = [0u8; 4];
        if stdin.read(&mut size[..1])? == 0 {
            return Ok(());
        }
        stdin.read_exact(&mut size[1..])?;
        let n = u32::from_be_bytes(size) as usize;
        if n > buffer.len() || n < fifo::HEADER {
            return Err("fixed_frame_bound".into());
        }
        stdin.read_exact(&mut buffer[..n])?;
        codec.accept(&buffer[..n])?;
        stdout.write_all(&size)?;
        stdout.write_all(&buffer[..n])?;
        stdout.flush()?;
    }
}
fn main() {
    if run().is_err() {
        std::process::exit(2)
    }
}
