//! Portfolio decision (P1): the `portfolio-decision-v1` record and its mechanical checks.
//!
//! The record is authored; this crate only checks it. Checking never writes, never mutates a
//! proposal lifecycle and never issues a rejection, priority or schedule. `proposal-evaluation`
//! and `decision-readiness` are reached only through their libraries (contract UD5).
//!
//! # Declared limitations of the check
//!
//! These are what a valid report does *not* establish (contract 4.1, 4.2):
//!
//! - **Supersession is checked one hop.** A record that names a `supersedes` target is valid when
//!   that target hashes to its stored digest, parses as `portfolio-decision-v1` (closed schema),
//!   and has the same `portfolio_id` and `revision - 1`. The target is not itself validated and
//!   the chain behind it is not walked: a revision-3 record is accepted over a revision-2 record
//!   that has a null or dangling `supersedes`, or that would fail its own check. Check the earlier
//!   revision separately to establish the chain.
//! - The strategic-basis handoff is re-hashed, never parsed (`strategic_basis_unverified`).
//! - The relationship graph is read from the bound packet bytes; their match to
//!   `relationship_graph.repository_revision` is not verified
//!   (`relationship_graph_revision_unverified`).
//! - An assessment is not re-run through `decision-readiness`'s `check_assessment`, and its
//!   attribution to a `proposal_id` is the author's, not verified.
//!
//! The record's own `limitations` list carries only the contract's named entries; the one-hop
//! limit is declared here and on `upstream::supersedes`, not as a further
//! required entry, because adding a required entry is a contract change.

pub mod check;
pub mod cli;
mod cross;
pub mod diag;
pub mod model;
mod read;
pub mod structure;
mod upstream;

pub use check::{KIND_PORTFOLIO, check_portfolio_bytes};
pub use diag::{CheckReport, ErrorItem, Failure, codes};

/// Probe (P1-AC0): the upstream owners are reached only through their libraries.
pub fn upstream_probe() -> (&'static str, &'static str, &'static str) {
    (
        proposal_evaluation::model::COMPARISON_SCHEMA,
        proposal_evaluation::model::ESTIMATE_SET_SCHEMA,
        decision_readiness::model::ASSESSMENT_SCHEMA,
    )
}
