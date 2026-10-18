//! Pure skill compilation and captured-byte verification. This crate has no host I/O capability.

mod codec;
mod error;
mod extension;
mod manifest;
mod skill;
mod verification;

pub use codec::{canonical_json, sha256_hex};
pub use error::{CompilerError, ErrorCode};
pub use extension::{
    CompiledExtension, EXTENSION_ATTACHMENT_PROFILE, ExtensionError, ExtensionSkillSource,
    compile_extension_bundle, extension_canonical_json, preflight_extension_bundle,
    verify_extension_attachment,
};
pub use manifest::{ManifestError, project_runtime_manifest, satisfy_runtime_requirements};
pub use skill::{CompiledSkill, compile_skill_unverified};
pub use verification::{ComparedSkill, PreparedSkill, SourceCheckedSkill, prepare_skill_verified};
