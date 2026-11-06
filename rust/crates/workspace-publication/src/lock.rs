use crate::{Error, Result};
use std::fs::{File, OpenOptions};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
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
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .mode(0o600)
            .open(common_dir.join(".work-engine-publication.lock"))
            .map_err(|e| Error::Io(e.to_string()))?;
        rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive)
            .map_err(|e| Error::Io(e.to_string()))?;
        Ok(Self { file })
    }
    pub(crate) fn file(&self) -> &File {
        &self.file
    }
    pub(crate) fn verify(&self, common_dir: &Path) -> Result<()> {
        let held = self.file.metadata().map_err(|e| Error::Io(e.to_string()))?;
        let named = std::fs::symlink_metadata(common_dir.join(".work-engine-publication.lock"))
            .map_err(|e| Error::Io(e.to_string()))?;
        if !named.is_file()
            || named.file_type().is_symlink()
            || held.dev() != named.dev()
            || held.ino() != named.ino()
        {
            return Err(Error::Integrity("permanent guard inode replaced".into()));
        }
        Ok(())
    }
    pub fn raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}
