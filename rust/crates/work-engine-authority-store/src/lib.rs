//! P1-F: private SQLite authority store for a fixture-only provisional computation vertical.
//!
//! This crate mints no ordinary authority. There is no `AdmittedOrganization` and no
//! `DispatchAuthorization` here (Gate O and Gate R are designs, not built). The ordinary
//! admission API ([`store::Store::admit`]) applies the O5 rule and, in this slice, can only
//! refuse. Fixture admissions, grants, schedules and results exist only behind the `fixture`
//! feature, in distinct types and in rows whose `domain` and `provenance` are CHECK-constrained.
pub mod adapter;
pub mod ident;
pub mod receipt;
pub mod report;
pub mod schema;
pub mod store;

#[cfg(feature = "fixture")]
pub mod fixture;

/// Without the `fixture` feature the fixture module does not exist:
///
/// ```compile_fail
/// use work_engine_authority_store::fixture::FixtureAdmission;
/// ```
#[cfg(not(feature = "fixture"))]
pub mod fixture_absent_without_feature {}
