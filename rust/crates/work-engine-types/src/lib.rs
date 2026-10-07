//! Public identity values and versioned encodings. These values convey no grant.

pub mod codec;
pub mod identity;

pub use codec::{CodecContract, CodecError, Digest, PayloadKind};
pub use identity::{IdError, IdValue};
