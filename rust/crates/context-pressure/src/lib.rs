//! Deterministic context-pressure evidence: a port of the JS `TokenUsagePressureProjector`
//! (`app-server/src/token-usage-pressure-projection.mjs`) and `ContextPressureController`
//! (`app-server/src/context-pressure-controller.mjs`).
//!
//! Scope: evidence producer only. Dispositions are scheduling evidence; they authorize no
//! retirement and own neither transition progress nor reset authority. The crate reads no
//! clock, store or lifecycle row, and performs no inference. Recovery, persistence and the
//! disposition-to-trigger mapping belong to other owners.
//!
//! Parity. Inputs are the JS plain-object shapes as `serde_json::Value`; outputs serialize to
//! the JS wire forms. Revision strings (`sha256:<hex>` over the sorted-key canonical JSON text)
//! are built to match the JS canonical text; that they do is established only within the
//! evidence domain stated below. Failures carry the JS message and a closed
//! [`ErrorKind`].
//!
//! Evidence domain. Byte parity is established only over two JS-generated bodies of evidence
//! under `tests/`: the golden corpus in `tests/golden/vectors.json` (policy, profile, projector,
//! controller, pipeline, timestamp and pressure-sweep cases, replayed through the public API,
//! with the declared deviations listed in `tests/golden.rs`), and a differential sample of
//! f64 bit patterns checked against Node's `JSON.stringify` (seed, stratum sizes and edge values
//! in `tests/golden/generate.mjs`). Number text is produced by `ryu-js`, an ECMAScript
//! `Number::toString` implementation; Rust's own shortest formatting is not used because it
//! breaks ties between equally short candidates differently. Not established: parity for every
//! f64 (the sample is deterministic and finite, not exhaustive), numbers outside f64 range and
//! lone-surrogate strings (not representable in `serde_json` input), V8's legacy `Date.parse`
//! forms (only the ECMAScript date-time grammar is accepted), and any input outside the corpus
//! shapes. Non-integer numbers in hashed input (extra nested fields) additionally require the
//! caller's `serde_json` to parse floats correctly rounded; this crate enables `serde_json`'s
//! `float_roundtrip` feature, which Cargo unifies across a workspace, and the golden vectors
//! fail without it.
//!
//! Known open issue (not resolved here). The controller requires `sequence` to increase
//! strictly. Rust native-ingress sequences restart per transport session, so a lifecycle-owned
//! observation sequence (or a `(session, sequence)` order) is needed before this crate is
//! wired to Rust ingress rows. No Rust lifecycle type carries this evidence today; wiring is the
//! lifecycle owner's step.

mod controller;
mod error;
mod fields;
mod js;
mod observation;
mod policy;
mod profile;
mod projector;
mod timestamp;

pub use controller::{
    ContextPressureController, ControllerSnapshot, ObserveResult, ObserveStatus,
    PressureTransition, TransitionReason,
};
pub use error::{ErrorKind, PressureError};
pub use observation::{PRESSURE_OBSERVATION_SCHEMA_VERSION, PressureObservation};
pub use policy::{
    Band, CONTEXT_PRESSURE_POLICY_SCHEMA_VERSION, ContextPressurePolicy, Disposition, PolicyUnit,
};
pub use profile::{
    Rounding, Saturation, TOKEN_USAGE_PRESSURE_PROFILE_SCHEMA_VERSION, TokenUsagePressureProfile,
    UsageField, WindowField,
};
pub use projector::{
    PressureProjection, ProjectedPressure, TokenUsagePressureProjector, UnavailableReason,
};
