//! Read-only claims tools for the Pi coding bridge (`CLAIMS_BRIDGE_PROFILE`).
//!
//! The Host admits a `claims_request` frame only in that profile, for the four
//! exact tool names below, within a finite per-job count and the job deadline.
//! The trusted server and read-profile pins are held by the embedder inside a
//! `repository_claims_handoff::consumer::ClaimsConsumer` installed behind the
//! existing `CodingEffectGate` composition; neither the model nor the job can
//! name or weaken them. Claims semantics (exact revisions, listed evidence,
//! currency, `not_assessed` applicability) stay in that crate; this adapter only
//! forwards and adds no workspace, grant or reliance authority.
use super::*;
pub use repository_claims_handoff::consumer::{
    ClaimsConsumer, MAX_ARGUMENT_BYTES, RelianceDecision, TOOLS, tool_schemas,
};

/// Wrap any lifecycle gate with a trusted read-only claims consumer. Every
/// `CodingEffectGate` method except `claims` forwards unchanged, so lifecycle
/// admission, grants, transitions and checkpoints remain with the inner gate.
pub struct ClaimsGate {
    inner: Box<dyn CodingEffectGate>,
    consumer: ClaimsConsumer,
}
impl ClaimsGate {
    pub fn new(inner: Box<dyn CodingEffectGate>, consumer: ClaimsConsumer) -> Self {
        Self { inner, consumer }
    }
}
impl CodingEffectGate for ClaimsGate {
    fn execute(
        &mut self,
        intent: &CodingToolIntent,
        actuator: &mut dyn FnMut(i64) -> Result<(bool, Vec<u8>)>,
    ) -> Result<(bool, Vec<u8>)> {
        self.inner.execute(intent, actuator)
    }
    fn frame(&mut self, frame: &Value) -> Result<Vec<Value>> {
        self.inner.frame(frame)
    }
    fn dispatch(&mut self, command: &Value, send: &mut dyn FnMut(i64) -> Result<()>) -> Result<()> {
        self.inner.dispatch(command, send)
    }
    fn observed(
        &mut self,
        intent: &CodingToolIntent,
        result: &Artifact,
        process: &ProcessRecord,
        tool_container: &str,
    ) -> Result<()> {
        self.inner.observed(intent, result, process, tool_container)
    }
    fn claims(
        &mut self,
        tool: &str,
        arguments: &Value,
        until: std::time::Instant,
    ) -> Result<Value> {
        // Owner refusals are model-visible tool failures carrying the stable
        // class; they never fall back to another selection.
        self.consumer
            .call(tool, arguments, Some(until))
            .map_err(|e| Error::Refused(format!("claims {}: {}", e.code(), e.detail())))
    }
}
