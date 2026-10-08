//! Exact, bounded controlled-process execution custody. This crate never grants
//! campaign admission or provider authority.
//!
//! A launch ticket is private to the producer after durable launch intent:
//!
//! ```compile_fail
//! use review_execution_evidence::producer::LaunchTicket;
//! ```
//!
//! A reader has no mutation API:
//!
//! ```compile_fail
//! fn append(reader: &mut review_execution_evidence::EvidenceReader) {
//!     reader.append("attempt", "stage", b"forged", &[]).unwrap();
//! }
//! ```

mod contract;
mod process;
mod producer;
mod reader;
mod store;
#[cfg(feature = "test-support")]
pub mod test_support;

pub use contract::{
    ARTIFACT_MAX_BYTES, CheckedExecutionResult, CheckedObservation, EvidenceClass,
    EvidenceReadError, EvidenceReaderProfile, ExecutionBinding, ExecutionEvidenceOwner,
    ExecutionEvidenceRef, ExecutionProvenance, MAX_DATABASE_BUSY_MILLIS, OBSERVATION_MAX_BYTES,
    REQUEST_MAX_BYTES, RESULT_MAX_BYTES, ROOT_PAYLOAD_MAX_BYTES, STREAM_MAX_BYTES,
    TERMINAL_MAX_BYTES,
};
pub use process::{CleanupObservation, FailureKind, ObservedFailure};
pub use producer::{ControlledScope, EvidenceWriter, ExecutionPublication};
pub use reader::EvidenceReader;
pub use store::ReaderPin;
