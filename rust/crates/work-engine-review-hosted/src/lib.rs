//! Hosted implementation-review/v1 artifact mechanics. R1/R2 are historical-only;
//! this build has no selectable writer or checker route. The private offline tests
//! exercise exact scope through preparation, Git materialization, writing and
//! independent artifact comparison. Nothing here establishes an acceptance
//! condition, observes a Host or provider, grants authority or admits a result.
//! See `CLAIMS.md` for pins and limits.

pub mod check;
pub mod inputs;
pub mod messages_artifacts;
pub mod revision;
pub mod secrets;
pub mod subject;
pub mod task;

#[cfg(test)]
mod offline;

use sha2::{Digest, Sha256};

pub use secrets::SecretHit;

/// SHA-256 as lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Why an artifact operation or historical entrypoint was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    #[error(
        "{0} is historical-only; use the exact archived source/checker pin and retained inputs (records/org-hosted-review-20261117 and the revision DERIVATION.md)"
    )]
    HistoricalOnly(String),
    #[error("requirements: {0}")]
    Requirements(String),
    #[error("preparation refused: {0}")]
    Preparation(String),
    /// The chosen world cannot be realized on this route (P1-2(a)): one id per blocker of class
    /// `KnownIncompatibility`.
    #[error("known realization incompatibility: {0:?}")]
    KnownIncompatibility(Vec<String>),
    #[error("freeze: {0}")]
    Freeze(String),
    #[error("task: {0}")]
    Task(String),
    #[error("artifact comparison: {0}")]
    Artifact(String),
    #[error("prompt bound: task {task} bytes + Host suffix {suffix} bytes exceeds {bound}")]
    PromptBound {
        task: usize,
        suffix: usize,
        bound: usize,
    },
    #[error("git: {0}")]
    Git(String),
    #[error("subject commit {0} is a merge or has no parent")]
    ParentShape(String),
    #[error("subject tree mismatch: expected {expected}, commit has {actual}")]
    TreeMismatch { expected: String, actual: String },
    #[error("subject parent mismatch: expected {expected}, commit has {actual}")]
    BaselineMismatch { expected: String, actual: String },
    #[error("patchIdentity mismatch: expected {expected}, recomputed {actual}")]
    PatchIdentityMismatch { expected: String, actual: String },
    #[error("exact subject scope mismatch: prepared scope.digest {expected}, recomputed {actual}")]
    ScopeDigestMismatch { expected: String, actual: String },
    #[error("snapshot member {path}: {why}")]
    Member { path: String, why: String },
    #[error("snapshot bound: {0}")]
    SnapshotBound(String),
    #[error(
        "request context bound: snapshot {snapshot_bytes} bytes x 1.25 + task {task_bytes} bytes exceeds 400000"
    )]
    ContextBound {
        snapshot_bytes: u64,
        task_bytes: usize,
    },
    #[error(
        "read budget: snapshot_list plus the patch pages need {needed} reads, limit {max_reads}"
    )]
    ReadBudget { needed: u64, max_reads: u32 },
    #[error("private or secret material: {0:?}")]
    Secret(Vec<SecretHit>),
    #[error("io: {0}")]
    Io(String),
}

impl From<std::io::Error> for Refusal {
    fn from(error: std::io::Error) -> Self {
        Refusal::Io(error.to_string())
    }
}
