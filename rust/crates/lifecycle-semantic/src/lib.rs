//! Pure, bounded semantic preservation values and checks.
//!
//! The trusted host assigns source provenance and invocation bindings. This crate
//! validates their internal consistency and derives a strict result from attributed
//! findings; it neither executes inference nor grants domain authority.

mod assessment;
mod continuation;
mod material;
mod policy;

pub use assessment::*;
pub use continuation::*;
pub use material::*;
pub use policy::*;

use sha2::{Digest as _, Sha256};
use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticError {
    InvalidIdentifier,
    UnsupportedPolicy,
    DuplicateIdentifier,
    InvalidDigest,
    InvalidSource,
    InvalidReference,
    WrongBinding,
    WrongPurpose,
    OutputTooLarge,
    InvalidOutput,
    PromptTooLarge,
    IneligibleMaterial,
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "semantic contract error: {self:?}")
    }
}

impl Error for SemanticError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte))
}

pub(crate) fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn json_digest<T: serde::Serialize>(value: &T) -> Result<String, SemanticError> {
    let bytes = serde_json::to_vec(value).map_err(|_| SemanticError::InvalidOutput)?;
    Ok(sha256_hex(&bytes))
}
