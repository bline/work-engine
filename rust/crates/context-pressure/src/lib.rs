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
//! are byte-identical to the JS for the same input, proven by JS-generated golden vectors under
//! `tests/`. Failures carry the JS message and a closed [`ErrorKind`].
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
