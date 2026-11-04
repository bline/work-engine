use crate::{Error, Result};
use std::fs::{File, OpenOptions};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// The lock inode is deliberately permanent; deleting it can split the fence.
pub struct Guard {
    file: File,
}
impl Guard {
    pub fn acquire(common_dir: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(common_dir.join(".work-engine-publication.lock"))
            .map_err(|e| Error::Io(e.to_string()))?;
        rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive)
            .map_err(|e| Error::Io(e.to_string()))?;
        Ok(Self { file })
    }
    pub fn raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}
