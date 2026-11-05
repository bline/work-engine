//! Operational serialization on the protected registry inode, never approval.
//! flock is attached to the open file description: closing one inherited copy
//! does not release surviving publishers. Never issue LOCK_UN in Drop.
use std::{
    fs::File,
    os::fd::{AsRawFd, RawFd},
    time::{Duration, Instant},
};
pub(crate) struct Fence(Option<File>);
impl Fence {
    pub(crate) fn acquire(db: &rusqlite::Connection) -> std::io::Result<Self> {
        let Some(path) = db.path().filter(|s| !s.is_empty()) else {
            // In-memory registries cannot be shared across processes.
            return Ok(Self(None));
        };
        let file = File::open(path)?;
        let end = Instant::now() + Duration::from_secs(10);
        loop {
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                return Ok(Self(Some(file)));
            }
            let e = std::io::Error::last_os_error();
            if e.kind() != std::io::ErrorKind::WouldBlock || Instant::now() >= end {
                return Err(e);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    pub(crate) fn fd(&self) -> Option<RawFd> {
        self.0.as_ref().map(AsRawFd::as_raw_fd)
    }
}
