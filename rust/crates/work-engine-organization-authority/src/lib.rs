//! Pure organizational admission core (R2 S-A).
//!
//! Meaning only: no filesystem, database, network, clock or environment access, and no
//! dependency on lifecycle, hosts or review preparation. Time is an input. It reads the pure
//! derivation, view and enforceability report of `work-engine-organization`, which it does not
//! change.
//!
//! - [`records`]: owner-attested `Principal`, `VantageActorBinding`, `ComponentBinding`,
//!   `DecisionScope` (no `decisions` field), `AuthorityGrant` and `RealizationAcceptance`, with
//!   [`IssuanceId`] (which act of issuing) separate from [`BodyDigest`] (which terms).
//! - [`AuthorityObservation`]: the source contract with its four obligations; [`reserve`] and
//!   [`observe_for`] seal one observation to one boundary.
//! - [`admit`]: Gate O subset (O3, O4, O5, O8, O9) and Gate R (R1-R4), every check evaluated,
//!   one outcome per [`CheckId`]; [`EffectAdmission::admission_state`]: current, stale, revoked,
//!   unknown or refused.
//!
//! Production policy refuses every admission (O5 `none` separation row; O4 component bindings,
//! O8 and R3 `binding_contract_unqualified`). Positive results exist only with the `fixture`
//! feature, whose records, source and policy are synthetic. This crate stores nothing, issues
//! nothing and records no single use of a boundary: those belong to later storage and to the
//! consuming host.
//!
//! An observation cannot be duplicated, so it cannot be kept for another decision by copying:
//!
//! ```compile_fail,E0599
//! fn keep(o: work_engine_organization_authority::SealedObservation) {
//!     let _copy = o.clone();
//! }
//! ```
//!
//! An admission cannot be duplicated:
//!
//! ```compile_fail,E0599
//! fn twice(a: work_engine_organization_authority::EffectAdmission) {
//!     let _copy = a.clone();
//! }
//! ```
//!
//! A reservation cannot be duplicated either:
//!
//! ```compile_fail,E0599
//! fn twice(r: work_engine_organization_authority::Reservation) {
//!     let _copy = r.clone();
//! }
//! ```
//!
//! [`admit`] consumes its observation, so one observation decides at most one admission:
//!
//! ```compile_fail,E0382
//! use work_engine_organization_authority::{
//!     DeclaredRealization, OperationRequest, Policy, Reservation, SealedObservation, admit,
//! };
//! fn twice<P: Policy>(
//!     r: &Reservation,
//!     o: SealedObservation,
//!     q: &OperationRequest,
//!     d: &DeclaredRealization,
//! ) {
//!     let _first = admit::<P>(r, o, q, d, 0);
//!     let _second = admit::<P>(r, o, q, d, 0);
//! }
//! ```
//!
//! An observation cannot be built outside this crate, with or without the `fixture` feature (only
//! an observation source in this crate produces an [`AuthorityState`], and only
//! [`observe_for`] seals one). Neither a struct literal nor the crate-private constructor is
//! reachable:
//!
//! ```compile_fail,E0451
//! use work_engine_organization_authority::{AuthorityState, Boundary, SealedObservation};
//! fn forge(boundary: Boundary, state: AuthorityState) -> SealedObservation {
//!     SealedObservation { boundary, state }
//! }
//! ```
//!
//! ```compile_fail,E0624
//! use work_engine_organization_authority::AuthorityState;
//! fn forge() -> AuthorityState {
//!     AuthorityState::new(0, Default::default())
//! }
//! ```
//!
//! A state cannot be assembled field by field either:
//!
//! ```compile_fail,E0451
//! use work_engine_organization_authority::{AuthorityState, ObservationRevision};
//! fn forge(revision: ObservationRevision) -> AuthorityState {
//!     AuthorityState { revision, entries: Default::default() }
//! }
//! ```

mod gate;
mod ids;
mod observation;
pub mod records;

#[cfg(feature = "fixture")]
pub mod fixture;

#[cfg(feature = "fixture")]
pub use gate::Fixture;
pub use gate::{
    AdmissionRefusal, AdmissionState, AuthorityPins, CheckId, CheckOutcome, DeclaredRealization,
    Domain, EffectAdmission, OperationRequest, Outcomes, Policy, Production, Reason, ReasonCode,
    StaleReason, SubjectRef, UnknownReason, admit,
};
pub use ids::{
    BodyDigest, Boundary, BoundaryKind, ExecutableDigest, IssuanceId, ObservationRevision,
    RealizationDigest,
};
pub use observation::{
    AuthorityObservation, AuthorityState, IssuanceStatus, Reservation, Resolution,
    SealedObservation, Unobserved, observe_for, reserve,
};
