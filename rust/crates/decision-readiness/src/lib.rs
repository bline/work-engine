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
//! Provenance boundary (contract N3). `check --kind assessment` validates structure and
//! digests, not provenance: it confirms an assessment is consistent with the evaluation file it
//! cites, not that the evaluation was derived from the plan, battery and subject. An internally
//! consistent forged evaluation passes both `check --kind evaluation` and `check --kind
//! assessment`; only `check --kind evaluation --rederive` catches it, so a decision must cite a
//! rederive result alongside the assessment check.
//!
//! An assessment's `evaluation.path` is the pointer a reader follows, so `check --kind assessment`
//! reads it: the path must name a readable regular file, following symlinks
//! (`evaluation_path_unresolved` otherwise), and its bytes must equal the file passed with
//! `--evaluation` (`evaluation_path_mismatch`). A FIFO, device or directory is opened without
//! blocking and refused on the opened descriptor, never waited on, and at most the supplied
//! file's length plus one byte is read. The path need not be the same pathname as `--evaluation`;
//! a byte-identical copy at the cited path is accepted, since `evaluation.sha256` binds the
//! content. The check runs at the time of checking and cannot show the file was at that path when
//! the assessment was written.
//!
//! Every other path read from a stored document (the recorded plan, battery and observation of an
//! evaluation under `--rederive` and `recheck`, and a subject's manifests) gets the same guard but
//! with no length cap. Files named directly on the command line are read as given: a deliberate
//! FIFO such as process substitution is the caller's choice, and the guard would refuse it.
//!
//! The attributed-basis rule trusts the evaluation's status, so the evaluation's requirement
//! classes are compared with the profile's: an evaluation that relabels a requirement's class is
//! rejected with `evaluation_class_mismatch` by `check --kind assessment` and by `check --kind
//! evaluation --rederive`. `check --kind evaluation` without a profile cannot make that comparison.
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
