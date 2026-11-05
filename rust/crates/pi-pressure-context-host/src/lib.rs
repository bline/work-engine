//! One serialized outer dispatch boundary. Lifecycle owns durable admission.
use lifecycle_core::*;
use lifecycle_store::*;
use pi_context_transition::{Error, PiAdapter};
use serde_json::{Value, json};
pub mod coding;

const GRANTS: [&str; 3] = ["grant:old", "grant:new", "grant:replace"];
const STOP_BYTES: &[u8] = b"provider-disabled-fixture-stop-v1";
/// No pending actuator or permit may escape this actor. Controls are fixture
/// inputs; canonical grant contraction is the durable stop fence.
pub struct FencedPi {
    adapter: PiAdapter,
    expires_ms: i64,
}
impl FencedPi {
    pub fn new(adapter: PiAdapter, expires_ms: i64) -> Self {
        Self {
            adapter,
            expires_ms,
        }
    }
    pub fn check(
        store: &mut SqliteLifecycleStore,
        expires_ms: i64,
        clock: ClockSample,
    ) -> Result<(), Error> {
        if clock.wall.get() >= expires_ms {
            return Err("fixture deadline exhausted".into());
        }
        for id in GRANTS {
            if store
                .grant_contraction_readback(
                    &format!("stop:{id}"),
                    &SubjectId::parse("subject:pi:repair")?,
                    "operator:fixture",
                    &GrantId::parse(id)?,
                    Revision::new(1),
                    STOP_BYTES,
                )?
                .is_some()
            {
                return Err("durable stop contracted".into());
            }
        }
        Ok(())
    }
    pub fn stop(store: &mut SqliteLifecycleStore, clock: ClockSample) -> Result<(), Error> {
        for id in GRANTS {
            store.contract_grant_command(
                &format!("stop:{id}"),
                &SubjectId::parse("subject:pi:repair")?,
                "operator:fixture",
                &GrantId::parse(id)?,
                Revision::new(1),
                STOP_BYTES,
                clock,
            )?;
        }
        Ok(())
    }
    pub async fn initialize(
        &mut self,
        store: &mut SqliteLifecycleStore,
        intent: NativeSessionEntry,
        clock: ClockSample,
    ) -> Result<(NativeSessionEntry, Value), Error> {
        Self::check(store, self.expires_ms, clock)?;
        self.adapter.initialize(intent).await
    }
    pub async fn observe(&mut self) -> Result<Value, Error> {
        self.adapter.observe().await
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn collect_prompt(
        &mut self,
        store: &mut SqliteLifecycleStore,
        session: &NativeSessionId,
        turn: &mut NativeTurnEntry,
        prompt: &str,
        reply: &str,
        clock: ClockSample,
    ) -> Result<PiSettlementProof, Error> {
        // No await between canonical check and the adapter's durable send CAS.
        Self::check(store, self.expires_ms, clock)?;
        self.adapter
            .collect_prompt(store, session, turn, prompt, reply, clock)
            .await
    }
    pub fn replacement_ready(
        &self,
        store: &mut SqliteLifecycleStore,
        subject: &SubjectId,
        transition: &TransitionId,
        checkpoint: &str,
    ) -> Result<(), Error> {
        self.adapter
            .replacement_ready(store, subject, transition, checkpoint)
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn replace(
        &mut self,
        store: &mut SqliteLifecycleStore,
        subject: &SubjectId,
        transition: &TransitionId,
        checkpoint: &str,
        intent: NativeSessionEntry,
        clock: ClockSample,
    ) -> Result<(NativeSessionEntry, Value), Error> {
        Self::check(store, self.expires_ms, clock)?;
        self.adapter
            .replace(store, subject, transition, checkpoint, intent)
            .await
    }
    pub async fn close(self) -> Result<lifecycle_runtime::NativeChildExit, Error> {
        self.adapter.close().await
    }
    pub async fn terminate(self) -> Result<lifecycle_runtime::NativeChildExit, Error> {
        self.adapter.terminate().await
    }
}
/// Explicit injected evidence. SDK usage is separately observed and never forged.
pub fn pressure_candidate(sample: Option<&Value>) -> Result<bool, Error> {
    let Some(sample) = sample else {
        return Ok(false);
    };
    let mut controller = context_pressure::ContextPressureController::from_options(
        &json!({"policy":{"schemaVersion":1,"unit":"basis_points","approaching":{"exit":4000,"enter":5000},"replacementCandidate":{"exit":6000,"enter":7000},"critical":{"exit":8000,"enter":9000}}}),
    )?;
    controller.observe(sample)?;
    Ok(matches!(
        controller.snapshot().disposition(),
        context_pressure::Disposition::ReplacementCandidate
            | context_pressure::Disposition::Critical
    ))
}
