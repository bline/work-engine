//! RP0's provider-disabled, candidate-bound Linux realization. No type in this
//! module grants provider entry, campaign admission or a production observation.

mod credentials;
mod isolation;
mod profile;

pub use credentials::{AccessProjection, CredentialSource};
pub use isolation::{CandidateBoundary, CandidateIndex, HostileProbeResult};
pub use profile::{PinnedProgram, Rp0Error, TrustedRp0Profile};
