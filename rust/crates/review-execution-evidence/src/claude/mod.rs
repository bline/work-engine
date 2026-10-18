//! RP0 candidate isolation plus RP1 bounded transport and custody. No type in
//! this module grants campaign admission or a production initial-entry permit.

mod credentials;
mod isolation;
#[cfg(feature = "test-support")]
mod local_probe;
mod profile;
mod qualified;
mod reader;
mod session;
mod transport;
mod writer;

pub use credentials::{AccessProjection, CredentialSource};
pub use isolation::{CandidateBoundary, CandidateIndex, HostileProbeResult};
#[cfg(feature = "test-support")]
pub use local_probe::LocalProbeResult;
pub use profile::{PinnedProgram, Rp0Error, TrustedClaudeProfile, TrustedRp0Profile};
pub use qualified::{
    InitialEntryPermit, QualifiedReaderSelection, QualifiedScope, QualifiedSelection,
    SessionEvolutionPermit, compose_applied_initial_prompt,
};
pub use reader::QualifiedReader;
pub use session::{RetainedReviewSession, SessionLocator};
pub use writer::QualifiedWriter;
