//! Proposal evaluation, slice E1: the estimate-set record.
//!
//! This crate owns the typed estimate-set schema (`proposal-evaluation-estimate-set-v1`) and
//! its mechanical validation. It issues no readiness judgment, priority or acceptance, reads
//! no clock, and contains no float: quantities are `i64` with a declared decimal exponent.

pub mod check;
pub mod cli;
pub mod contract;
pub mod diag;
pub mod digest;
pub mod model;
pub mod scan;
pub mod validate;

pub use check::{check_contract_bytes, check_estimate_set_bytes};
pub use diag::{CheckReport, ErrorItem, Failure, codes};
pub use digest::sha256_hex;
