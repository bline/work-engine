//! Decision-specific readiness (contract `contract-v3.md`, slice S1).
//!
//! This crate owns the profile, subject, evaluation and assessment schemas and their
//! validation, observation-record validation, and deterministic plan derivation. It
//! never emits a verdict, reads no clock, and does not observe the state a requirement
//! is about: the evidence battery runs unchanged and observes.
//!
//! S1 provides the types, `check` for profile, subject and observation, and `plan`. S2 part a
//! adds `evaluate`.
//!
//! Stated deviations from contract v3 (errata v2): E3, an attributed requirement with
//! `predicates: []` is accepted, and duplicate keys inside a subject's maps are not rejected
//! (the last wins; rejecting needs a custom deserializer). E4, `check` reports only the first
//! serde error on a closed-schema violation, and an unknown key inside a subject value object
//! reports `value_type_mismatch` rather than `schema`.

pub mod check;
pub mod cli;
pub mod diag;
pub mod digest;
pub mod evaluate;
pub mod model;
pub mod plan;
pub mod validate;

pub use diag::{CheckReport, ErrorItem, codes};
pub use digest::sha256_hex;
pub use plan::{Plan, PlanError, derive_plan, plan_to_string};
