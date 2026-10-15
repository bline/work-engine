//! External S8 proof machinery. This crate has no lifecycle or task-store handle.
//! It may submit ordinary host commands, observe supported replies, and control
//! only the loopback provider response schedule.

pub mod host;
pub mod identity;
pub mod provider;
pub mod receipt;

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    #[error("proof I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("proof JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("proof contract failed: {0}")]
    Contract(String),
    #[error("proof deadline expired: {0}")]
    Deadline(&'static str),
}

pub type Result<T> = std::result::Result<T, ProofError>;
