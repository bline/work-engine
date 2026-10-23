//! Bounded reads of paths taken from a stored record (contract S6, lesson EXT-006).
//!
//! A stored path can name a FIFO, a device or a directory, and opening a FIFO waits for a writer
//! indefinitely. The file is therefore opened without blocking and the opened descriptor, not the
//! pathname, is checked to be a regular file; the bytes read are capped. The pattern is
//! `decision-readiness`'s `open_regular`/`read_stored`, which are private there and so are
//! restated here; the upstream crates are not modified (contract UD5).

use std::io::Read;

/// Most bytes read from one stored path. A larger file is refused as unresolved.
pub const MAX_STORED_BYTES: u64 = 32 * 1024 * 1024;

fn not_regular() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "not a regular file (a stored path must name a regular file)",
    )
}

/// Read the regular file at `path`, at most [`MAX_STORED_BYTES`]. Symlinks are followed. A
/// directory, FIFO or device, and a file over the cap, are errors.
pub(crate) fn read_stored(path: &str) -> std::io::Result<Vec<u8>> {
    read_stored_capped(path, MAX_STORED_BYTES)
}

pub(crate) fn read_stored_capped(path: &str, cap: u64) -> std::io::Result<Vec<u8>> {
    // A path that is already not a regular file is refused before it is opened; the check on the
    // descriptor in `open_regular` is the one that holds if the path is swapped in between.
    if !std::fs::metadata(path)?.is_file() {
        return Err(not_regular());
    }
    let file = open_regular(path)?;
    let mut bytes = Vec::new();
    file.take(cap.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > cap {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("file is larger than the {cap}-byte read cap"),
        ));
    }
    Ok(bytes)
}

fn open_regular(path: &str) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK | O_NOCTTY)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(not_regular());
    }
    Ok(file)
}

// Linux `O_NONBLOCK` / `O_NOCTTY` (kernel uapi values; they differ on mips and sparc):
//   generic:       O_NONBLOCK 0o4000, O_NOCTTY 0o400
//   mips, mips64:  O_NONBLOCK 0o200,  O_NOCTTY 0x800
//   sparc:         O_NONBLOCK 0x4000, O_NOCTTY 0x8000
// Elsewhere only the pathname check in `read_stored_capped` guards the open.
#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "mips",
        target_arch = "mips32r6",
        target_arch = "mips64",
        target_arch = "mips64r6"
    )
))]
const O_NONBLOCK: i32 = 0o200;
#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "mips",
        target_arch = "mips32r6",
        target_arch = "mips64",
        target_arch = "mips64r6"
    )
))]
const O_NOCTTY: i32 = 0x800;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "sparc", target_arch = "sparc64")
))]
const O_NONBLOCK: i32 = 0x4000;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "sparc", target_arch = "sparc64")
))]
const O_NOCTTY: i32 = 0x8000;
#[cfg(all(
    target_os = "linux",
    not(any(
        target_arch = "mips",
        target_arch = "mips32r6",
        target_arch = "mips64",
        target_arch = "mips64r6",
        target_arch = "sparc",
        target_arch = "sparc64"
    ))
))]
const O_NONBLOCK: i32 = 0o4000;
#[cfg(all(
    target_os = "linux",
    not(any(
        target_arch = "mips",
        target_arch = "mips32r6",
        target_arch = "mips64",
        target_arch = "mips64r6",
        target_arch = "sparc",
        target_arch = "sparc64"
    ))
))]
const O_NOCTTY: i32 = 0o400;
#[cfg(not(target_os = "linux"))]
const O_NONBLOCK: i32 = 0;
#[cfg(not(target_os = "linux"))]
const O_NOCTTY: i32 = 0;
