//! Portfolio decision (P1): the `portfolio-decision-v1` record and its mechanical checks.
//!
//! The record is authored; this crate only checks it. Checking never writes, never mutates a
//! proposal lifecycle and never issues a rejection, priority or schedule. `proposal-evaluation`
//! and `decision-readiness` are reached only through their libraries (contract UD5).

pub mod check;
pub mod diag;
pub mod model;
pub mod structure;

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
