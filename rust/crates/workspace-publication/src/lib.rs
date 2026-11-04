//! Fresh, local Git publication owner. Saved references are evidence, never authority.
pub mod codec;
pub mod contract;
mod git;
mod lock;
mod store;
mod validation;

pub use contract::*;
pub use store::Workspace;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid publication input: {0}")]
    Invalid(String),
    #[error("publication conflict: {0}")]
    Conflict(String),
    #[error("publication state unresolved: {0}")]
    Unresolved(String),
    #[error("publication owner integrity: {0}")]
    Integrity(String),
    #[error("publication I/O: {0}")]
    Io(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "test-faults")]
pub(crate) fn fault_cut(name: &str) {
    if std::env::var("WORKSPACE_PUBLICATION_FAULT_CUT")
        .ok()
        .as_deref()
        != Some(name)
    {
        return;
    }
    let Ok(dir) = std::env::var("WORKSPACE_PUBLICATION_FAULT_DIR") else {
        return;
    };
    let dir = std::path::Path::new(&dir);
    std::fs::write(dir.join(format!("{name}.ready")), b"ready\n").expect("fault ready signal");
    while !dir.join(format!("{name}.release")).exists() {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
