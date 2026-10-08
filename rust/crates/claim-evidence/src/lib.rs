//! Private-root claim evidence owner for the first native finding profile.
//! Campaign admission is a separately composed trusted port; this crate owns
//! grant registration, finding identity, SQLite custody, and exact readback.

mod application;
mod authority;
pub mod codec;
mod contract;
mod findings;
mod projection;
mod reliance;
pub mod schema;
mod store;

pub use application::{AdmissionBinding, ClaimsApplication, FindingAdmissionPort, FindingLease};
pub use application::{FINDING_REQUEST_MAX_BYTES, claims_request_canonical, claims_request_sha256};
pub use application::{reliance_request_canonical, reliance_request_sha256};
pub use authority::sha256_bytes;
pub use authority::{BootstrapFiles, OpenFiles};
pub use contract::WriteError;
pub use contract::{
    ClaimError, ClaimResult, FindingCommand, FindingMutation, FindingReceipt, OperationLocator,
    Publication, Reconciliation, RootIdentity,
};
pub use findings::stable_claim_id;
pub use findings::{ClaimId, FindingRevisionId, OperationId};
pub use projection::{
    ClaimProjection, PROJECTION_BUILD_VERSION, PROJECTION_CONTEXT_KIND, ProjectedRevision,
    ProjectionConsumer, ProjectionProvenance, ProjectionRequest, ProjectionSelection,
    projection_request_sha256,
};
pub use reliance::{
    ExactFindingRevisionRef, ExactRelianceRef, RelianceCommand, RelianceId, ReliancePublication,
    RelianceReceipt, RelianceReconciliation,
};

#[cfg(feature = "test-faults")]
fn fault_cut(name: &str) {
    use std::{path::Path, time::Duration};
    if std::env::var("CLAIM_EVIDENCE_FAULT_CUT").ok().as_deref() != Some(name) {
        return;
    }
    let Ok(directory) = std::env::var("CLAIM_EVIDENCE_FAULT_DIR") else {
        return;
    };
    let ready = Path::new(&directory).join(format!("{name}.ready"));
    let release = Path::new(&directory).join(format!("{name}.release"));
    std::fs::write(&ready, b"ready\n").expect("fault ready signal");
    while !release.exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(not(feature = "test-faults"))]
fn fault_cut(_: &str) {}
