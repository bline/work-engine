//! Inert review preparation (Slice 1, PLAN-R2 first slice).
//!
//! This crate composes owners; it owns none of what it composes. Role and contract meaning stays
//! with the organization compiler's candidate, evidence establishment and grants with their own
//! owners, and episode semantics with `review-episode-core`.
//!
//! * [`prepare`] derives the organization view and the enforceability report from **one**
//!   canonical [`work_engine_organization::OrganizationCandidate`] and returns a
//!   [`ReviewPreparation`]: the projection of one obligation plus explicit blockers that
//!   distinguish a known incompatibility from an unestablished mandatory mapping. The result is
//!   a plain value, not an execution or admission token. No API in this crate allocates a
//!   session, calls a provider or tool, reads the environment or writes product state; the
//!   crate has no dependency that could do so, and none of its source calls such an API
//!   (`std::fs`, `std::process`, `std::net`, `std::env`, threads, clocks and randomness do not
//!   appear in it). That is a code-level property, not runtime confinement.
//! * With the non-default `fixture` feature, [`fixture`] adds a consumer that accepts explicit
//!   synthetic declarations and calls the existing episode transition. Its declarations are test
//!   assumptions: they never satisfy or change [`prepare`].
//!
//! Source-written fixtures and synthetic declarations establish no mapping. A matching string
//! (a boundary, a consumer, an identifier) is not a mapping, and a supplied realization
//! descriptor is a claim, not admission evidence.

#![cfg_attr(
    not(feature = "fixture"),
    doc = "\nWithout the `fixture` feature the consumer does not exist:\n\n```compile_fail,E0432\nuse work_engine_review_preparation::fixture::FixtureDeclaration;\n```\n"
)]

mod prepare;
mod realization;
mod select;

#[cfg(feature = "fixture")]
pub mod fixture;

pub use prepare::{
    AuthorityContext, Blocker, BlockerClass, BlockerKind, CandidateIdentity,
    ComponentBindingContext, ConditionContext, ConstraintContext, EnforcementRow, HostEnforcement,
    ModelContext, NormativeContext, ObligationContext, PreparationRequest, PreparationStatus,
    Provenance, ReviewPreparation, RowEvidenceClass, RowStatus, SeparationContext, Subject,
    TraceEndpoint, VantageContext, prepare,
};
pub use realization::{
    ConditionRealization, Disposition, MutationClaim, RealizationCheck, RealizationClaim,
    RealizationEvaluation,
};
pub use select::{PreparationRefusal, UnsupportedRow};
