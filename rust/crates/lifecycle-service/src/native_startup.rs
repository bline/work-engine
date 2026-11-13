//! Trusted in-process selected-native admission. No socket/serde constructor.
//! The runner retains child creation, protocol and termination ownership.
pub use lifecycle_codex::NativeCurrentnessGuard;
use lifecycle_codex::{OwnedNativeObservation, ProtectedSurfaceSpec};
use lifecycle_store::StoreError;
use serde_json::Value;
use tokio::sync::{oneshot, watch};

pub struct NativeStartupHold {
    pub(crate) cancellation: watch::Receiver<()>,
    pub(crate) spec: ProtectedSurfaceSpec,
    pub(crate) controlled: bool,
    pub(crate) ready: oneshot::Sender<PendingNativeStartup>,
}
pub struct NativeStartupReceiver {
    ready: oneshot::Receiver<PendingNativeStartup>,
    lifetime: watch::Sender<()>,
}
pub struct ActiveNativeStartup {
    _lifetime: watch::Sender<()>,
}
pub struct PendingNativeStartup {
    observation: OwnedNativeObservation,
    evidence: Value,
    activate: oneshot::Sender<oneshot::Sender<Result<(), StoreError>>>,
    lifetime: Option<watch::Sender<()>>,
}
impl PendingNativeStartup {
    /// Controlled qualification corrupts only candidate protocol evidence.
    /// The owned observation, guard and activation channel remain unchanged.
    #[cfg(feature = "native-sim-proof")]
    pub fn controlled_test_evidence_mut(&mut self) -> &mut Value {
        &mut self.evidence
    }
    pub fn observation(&self) -> &OwnedNativeObservation {
        &self.observation
    }
    pub fn evidence(&self) -> &Value {
        &self.evidence
    }
    pub fn guard(&self) -> NativeCurrentnessGuard {
        self.observation.guard()
    }
    pub fn has_pinned_protocol(&self) -> bool {
        self.evidence["protocol"] == lifecycle_codex::SCHEMA_THREAD_START_SHA256
            && self.evidence["binary"] == lifecycle_codex::NATIVE_BINARY_SHA256
    }
    /// Consuming one-use activation; runner independently rechecks before entry.
    pub async fn activate(mut self) -> Result<ActiveNativeStartup, StoreError> {
        self.observation
            .guard()
            .check_current()
            .map_err(|_| StoreError::Rejected)?;
        let (ack, received) = oneshot::channel();
        self.activate
            .send(ack)
            .map_err(|_| StoreError::Unavailable)?;
        received.await.map_err(|_| StoreError::Unavailable)??;
        Ok(ActiveNativeStartup {
            _lifetime: self.lifetime.take().ok_or(StoreError::Unavailable)?,
        })
    }
    pub fn cancel(self) {
        drop(self);
    }
}
impl NativeStartupReceiver {
    /// Qualification closes only the readiness endpoint, retaining lifetime.
    #[cfg(feature = "native-sim-proof")]
    pub fn controlled_close_readiness(&mut self) {
        self.ready.close();
    }

    pub async fn pending(self) -> Result<PendingNativeStartup, StoreError> {
        let mut pending = self.ready.await.map_err(|_| StoreError::Unavailable)?;
        pending.lifetime = Some(self.lifetime);
        Ok(pending)
    }
}
/// Trusted composition supplies the exact protected set, never a native PID.
/// Production admission additionally needs qualified pinned-protocol tool
/// installation semantics, currently unavailable. This hold still cleans up on
/// missing evidence; a caller cannot supply a `complete` claim to release it.
pub fn hold(spec: ProtectedSurfaceSpec) -> (NativeStartupHold, NativeStartupReceiver) {
    make(spec, false)
}
#[cfg(feature = "native-sim-proof")]
pub fn controlled_hold(spec: ProtectedSurfaceSpec) -> (NativeStartupHold, NativeStartupReceiver) {
    make(spec, true)
}
fn make(
    spec: ProtectedSurfaceSpec,
    controlled: bool,
) -> (NativeStartupHold, NativeStartupReceiver) {
    let (ready, receiver) = oneshot::channel();
    let (lifetime, cancellation) = watch::channel(());
    (
        NativeStartupHold {
            spec,
            controlled,
            ready,
            cancellation,
        },
        NativeStartupReceiver {
            ready: receiver,
            lifetime,
        },
    )
}
pub(crate) async fn publish(
    hold: NativeStartupHold,
    observation: OwnedNativeObservation,
    evidence: Value,
) -> Result<
    (
        oneshot::Receiver<oneshot::Sender<Result<(), StoreError>>>,
        NativeCurrentnessGuard,
    ),
    StoreError,
> {
    // Request+ack is retained evidence, not independent proof of the installed
    // tool surface. No qualified production contract exists for this build yet.
    if !hold.controlled || !observation.controlled_only() {
        return Err(StoreError::Unavailable);
    }
    let guard = observation.guard();
    let (activate, receiver) = oneshot::channel();
    hold.ready
        .send(PendingNativeStartup {
            observation,
            evidence,
            activate,
            lifetime: None,
        })
        .map_err(|_| StoreError::Unavailable)?;
    Ok((receiver, guard))
}
