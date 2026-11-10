//! Failure categories. Each is distinct from invalid register data, which this crate does not judge.

use thiserror::Error;

/// Why a read or snapshot could not be produced.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// A required object or reference could not be read exactly as requested.
    #[error("cannot read: {0}")]
    CannotRead(#[from] CannotRead),
    /// The repository is of a kind this reader refuses.
    #[error("unsupported: {0}")]
    Unsupported(#[from] Unsupported),
    /// The commit was read, but the register path is not a tree in it.
    #[error("no register at commit: {0}")]
    NoRegister(#[from] NoRegister),
    /// The request itself is malformed or names something that is not a commit.
    #[error("bad request: {0}")]
    BadRequest(#[from] BadRequest),
}

impl Error {
    /// Stable category name used in machine-readable output.
    pub fn category(&self) -> &'static str {
        match self {
            Error::CannotRead(_) => "cannot_read",
            Error::Unsupported(_) => "unsupported",
            Error::NoRegister(_) => "no_register_at_commit",
            Error::BadRequest(_) => "bad_request",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CannotRead {
    #[error("repository at {path} could not be opened: {reason}")]
    Open { path: String, reason: String },
    #[error("object {id} is not available in the repository")]
    ObjectUnavailable { id: String },
    #[error("object store error reading {id}: {reason}")]
    ObjectStore { id: String, reason: String },
    #[error("object returned for {requested} has content hashing to {content_hash}")]
    Substituted {
        requested: String,
        content_hash: String,
    },
    #[error("object {id} is a {actual}, expected a {expected}")]
    TypeMismatch {
        id: String,
        expected: String,
        actual: String,
    },
    #[error("object {id} is malformed: {reason}")]
    Malformed { id: String, reason: String },
    #[error("reference {name} could not be read: {reason}")]
    Reference { name: String, reason: String },
    #[error("whether the repository is shallow could not be determined: {reason}")]
    ShallowState { reason: String },
    #[error("whether {ancestor} is an ancestor of {descendant} could not be established")]
    Ancestry {
        ancestor: String,
        descendant: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Unsupported {
    #[error("object format {format}; only sha1 is supported")]
    ObjectFormat { format: String },
    #[error("shallow repository; history is incomplete")]
    ShallowRepository,
    #[error("repository format version {version}; only 0 and 1 are supported")]
    FormatVersion { version: i64 },
    #[error("repository extension {name} = {value:?} is not supported")]
    Extension { name: String, value: String },
    /// git ignores `gitoxide.*` keys but gix reads them from the repository's own configuration
    /// (for example `gitoxide.core.refsNamespace`, `gitoxide.core.shallowFile`), so a repository
    /// that sets one would be read differently from git.
    #[error("repository configuration {file} sets {key}, which git ignores but gix would apply")]
    GitoxideKey { file: String, key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NoRegister {
    #[error("{path} does not exist at the commit")]
    Absent { path: String },
    #[error("{path} is a {kind} at the commit, not a tree")]
    NotATree { path: String, kind: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BadRequest {
    #[error("revision {rev:?}: {reason}")]
    Revision { rev: String, reason: String },
    #[error("path {path:?}: {reason}")]
    Path { path: String, reason: String },
    #[error("prefix {prefix:?}: {reason}")]
    Prefix { prefix: String, reason: String },
}
