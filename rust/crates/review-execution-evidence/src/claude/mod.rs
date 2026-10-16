//! RP0 candidate isolation plus RP1 bounded transport and custody. No type in
//! this module grants campaign admission or a production initial-entry permit.

mod credentials;
mod isolation;
mod profile;
mod qualified;
mod reader;
mod session;
mod transport;
mod writer;

pub use credentials::{AccessProjection, CredentialSource};
pub use isolation::{CandidateBoundary, CandidateIndex, HostileProbeResult};
pub use profile::{PinnedProgram, Rp0Error, TrustedClaudeProfile, TrustedRp0Profile};
pub use qualified::{
    InitialEntryPermit, QualifiedReaderSelection, QualifiedScope, QualifiedSelection,
    SessionEvolutionPermit,
};
pub use reader::QualifiedReader;
pub use session::{RetainedReviewSession, SessionLocator};
pub use writer::QualifiedWriter;
