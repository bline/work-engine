//! Hosted review adapter for the Pi analysis profile (`work-engine.pi-analysis-hosted.v1`).
//!
//! **Route qualification only.** Nothing here establishes a review condition, admits a result,
//! grants a capability or makes anything consumable. What a run may and may not claim is listed in
//! `CLAIMS.md` beside this crate's manifest.
//!
//! The adapter composes owners it does not own:
//!
//! * [`revision`]: the versioned requirements revision `hosted-route-qualification-r1` (operator
//!   decision D2/OQ-1 (D), 2026-11-17), compiled by the unchanged organization compiler, and the
//!   one realization claim that matches what the Host enforces. [`revision::prepare_world`] calls
//!   the public `prepare` and `FrozenDelivery::freeze` and refuses on any known realization
//!   incompatibility; the frozen requirements stay the reported negative world.
//! * [`task`]: the versioned, digest-pinned instruction template followed by the contract bytes
//!   verbatim, refused above the Host prompt bound.
//! * [`subject`]: the exact frozen subject's changed files and diff, read from git objects, with
//!   verified commit/tree/patch identity, the requirements-to-subject binding, size bounds and the
//!   closed [`secrets`] rule set (operator decision D3/OQ-2, 2026-11-17).
//! * [`inputs`]: the composition that writes the job inputs a separate job writer
//!   (`records/dev-vertical-h2-route-20261117/analyze.py`) consumes.
//! * [`check`]: the result check, which keeps the answer only as a string inside a non-consumable
//!   `owner_pending` envelope.
//!
//! The crate starts no provider, container or Host, and writes only the output paths its binaries
//! are given. The only process it starts is `git`, for read-only object queries
//! ([`subject::materialize`]).

pub mod check;
pub mod inputs;
pub mod revision;
pub mod secrets;
pub mod subject;
pub mod task;

use sha2::{Digest, Sha256};

pub use secrets::SecretHit;

/// SHA-256 as lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Why the adapter refused to produce job inputs. [`inputs::write`] stages its files in a sibling
/// directory and renames it into place only after every write succeeded, so neither a refusal nor
/// an I/O failure leaves the output directory behind (a crash mid-write can leave the staging
/// directory).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
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
    #[error("patchIdentity mismatch: expected {expected}, recomputed {actual}")]
    PatchIdentityMismatch { expected: String, actual: String },
    #[error("requirements scope.digest {scope} is not the subject patchIdentity {patch}")]
    ScopeDigestMismatch { scope: String, patch: String },
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
