//! Pure organizational compilation (Slice 1A).
//!
//! This crate is meaning only: it has no I/O, no clock, no environment access and
//! no path to any effect. Its identities are digests of a canonical form
//! (contract revision 5.1, section 17).
//!
//! Phase 1 content: the restricted RFC 8785 canonicalizer, the digest framing and the scalar and
//! identifier rules. The schema, derivation and candidate types are added once the schema
//! decisions are recorded.

pub mod digest;
pub mod error;
pub mod ids;
pub mod json;

pub use digest::{CODEC_DOMAIN, CODEC_VERSION, Digest, DigestKind};
pub use error::{CodecError, IdError, Position};
pub use ids::Id;
pub use json::{JsonValue, MAX_DEPTH, MAX_SAFE_INTEGER};
