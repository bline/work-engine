//! Pure organizational compilation (Slice 1A).
//!
//! This crate is meaning only: it has no I/O, no clock, no environment access and
//! no path to any effect. Its identities are digests of a canonical form
//! (contract revision 5.1, section 17).
//!
//! The canonicalizer and its value type are crate-private. The public surface is the interface
//! of contract section 10.1 and the types needed to name its arguments, results and errors.

mod digest;
mod error;
mod ids;
#[allow(dead_code)]
mod json;

pub use digest::{CODEC_DOMAIN, CODEC_VERSION, Digest, DigestKind};
pub use error::{CodecError, IdError, Position};
pub use ids::Id;

#[cfg(test)]
mod codec_rules_tests;
#[cfg(test)]
mod rfc_vectors_tests;
