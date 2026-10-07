//! Pure, unverified v1 skill compilation. This crate has no host I/O capability.

mod codec;
mod error;
mod skill;

pub use codec::{canonical_json, sha256_hex};
pub use error::{CompilerError, ErrorCode};
pub use skill::{CompiledSkill, compile_skill_unverified};
