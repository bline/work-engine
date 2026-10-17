//! E1-owned read-only checked custody for controlled and qualified execution.
//! A checked result is constructed only after anchored evidence readback.
pub mod claude;
mod contract;
mod process;
mod reader;
mod record;
mod store_read;
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
pub use reader::EvidenceReader;
pub use record::{CapturedRecord, StoredRequest, TerminalRecord};
pub use store_read::ReaderPin;

/// Pure codecs and shape validators; none constructs a checked result.
pub mod codec {
    pub use crate::contract::{
        record_digest, require_sha, require_text, sha256, validate_raw_result,
    };
}
