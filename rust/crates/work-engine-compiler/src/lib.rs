//! Pure skill compilation and captured-byte verification. This crate has no host I/O capability.

mod codec;
mod error;
mod manifest;
mod skill;
mod verification;

pub use codec::{canonical_json, sha256_hex};
pub use error::{CompilerError, ErrorCode};
pub use manifest::{ManifestError, project_runtime_manifest, satisfy_runtime_requirements};
pub use skill::{CompiledSkill, compile_skill_unverified};
pub use verification::{ComparedSkill, PreparedSkill, SourceCheckedSkill, prepare_skill_verified};
