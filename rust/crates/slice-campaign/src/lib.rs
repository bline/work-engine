//! New private-root campaign owner for one initial native implementation review.
//! References are descriptive data. Only [`Campaign`] owns a writer and can issue
//! a live admission handle. Provider entry and owner evidence stay external;
//! campaign state records their checked consequences.

mod admission;
mod application;
pub mod codec;
mod completion;
pub mod contract;
mod recovery;
mod store;
pub mod subject_binding;

#[cfg(test)]
extern crate self as slice_campaign;
#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;

pub use admission::{AdmissionHandle, Preparation, PreparedInitial};
pub use application::{
    AdmitRequest, CE3CampaignClaimsAdmission, Campaign, CampaignClaimsAdmission,
    InitialClaimsOwner, InitialCompletionLocator, InitialCompletionOwners,
    InitialCompletionPrepared, InitialCompletionProgress, InitialEpisodeReadRegistered,
    InitialJoinSelection, InitialProgressState, TrustedConfig,
};
pub use completion::{
    CampaignProgress, DispatchCommand, DispatchEffect, DispatchPermit, DispatchRecord,
    DispatchedRequest, RecoveredRequest, StageIntent, StageKind, StageProgress,
};
pub use contract::{
    AcceptedBoundary, AdvancePhase, Baseline, CampaignIdentity, CampaignRef, CampaignRevision,
    CandidateRef, Consequence, NativeReviewRequestRef, SelectionRef, Snapshot,
};
pub use recovery::{Effect, OperationReceipt, Reconcile, RecoveryLocator};
pub use subject_binding::{VerifiedCandidate, VerifiedGate, verify_existing_subject};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CampaignError {
    #[error("invalid campaign contract: {0}")]
    Contract(String),
    #[error("unsupported campaign profile: {0}")]
    Unsupported(String),
    #[error("campaign root or writer is unsafe: {0}")]
    Root(String),
    #[error("campaign revision or operation conflict: {0}")]
    Conflict(String),
    #[error("campaign subject verification failed: {0}")]
    Subject(String),
    #[error("campaign state is corrupt: {0}")]
    Integrity(String),
    #[error("campaign I/O failed before an effect: {0}")]
    Io(String),
}

pub type Result<T> = std::result::Result<T, CampaignError>;

#[cfg(feature = "test-faults")]
pub(crate) fn fault_cut(name: &str) {
    use std::path::Path;
    use std::time::Duration;
    if std::env::var("SLICE_CAMPAIGN_FAULT_CUT").ok().as_deref() != Some(name) {
        return;
    }
    let Ok(directory) = std::env::var("SLICE_CAMPAIGN_FAULT_DIR") else {
        return;
    };
    let directory = Path::new(&directory);
    std::fs::write(directory.join(format!("{name}.ready")), b"ready\n")
        .expect("fault ready signal");
    loop {
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(not(feature = "test-faults"))]
#[inline]
pub(crate) fn fault_cut(_: &str) {}

pub(crate) fn require_text(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(CampaignError::Contract(format!("{name} must be nonempty")));
    }
    Ok(())
}

pub(crate) fn require_sha(value: &str, name: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(CampaignError::Contract(format!(
            "{name} must be lowercase SHA-256"
        )));
    }
    Ok(())
}
