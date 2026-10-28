//! Direct realization of the compile and separate verify calls inside
//! `establish_checkpoint`.
//!
//! The pipeline is the native one: derive a candidate, assess it with a
//! separately submitted verifier, revalidate dependencies, record the result and
//! publish through the one checkpoint owner. Only the control-call identity
//! differs: a direct operation has no provider session or thread, and none is
//! invented. The trust boundary is the store's accessor split plus owner
//! admission: a direct result is proof-scoped unless, in the publish
//! transaction, a trusted-issuer owner grant admits a profile that covers its
//! evidence. Native successor consumers (prompt, rehydration, commit) never read
//! a direct checkpoint; widening them is a separate owner decision.

use std::sync::{Arc, Mutex};

use lifecycle_core::{ContextGeneration, SubjectId};
use lifecycle_semantic::{
    DirectCapture, DirectInferencePort, DirectOperationBinding, DirectPreparedCall,
    DirectReleaseState, DirectRole, DirectSubmitError, DirectTerminal, SemanticError,
    SemanticPolicy, SourceBasis, VerdictStatus, assess_direct_control_candidate,
    direct_candidate_from_capture, direct_compile_input, direct_verify_input,
    prepare_direct_compile, prepare_direct_verify, revalidate_dependencies,
};
use lifecycle_store::{DirectAttemptStart, DirectControlEntry, SqliteLifecycleStore, StoreError};

use crate::{
    actor::clock_sample,
    semantic::{SemanticRuntime, publish_accepted_checkpoint},
};

/// A prepared direct call waiting for its owner release. Nothing about it is
/// durable: it only tells the actor that re-driving the transition later may
/// make progress. Each re-drive repeats every check from the beginning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectPending {
    pub operation_id: String,
    pub prepared_binding_sha256: String,
    pub reason: String,
}

/// Owned by the actor and shared with each realization it builds.
pub type DirectPendingSlot = Arc<Mutex<Option<DirectPending>>>;

/// Operation identity and endpoint for one direct checkpoint. Authorization to
/// call comes from the runtime's existing semantic control grant. The evidence
/// class belongs to the port, and eligibility to the owner's admission grant
/// read by the store at publication: no field here can declare either.
#[derive(Clone)]
pub struct DirectRealization {
    pub port: Arc<dyn DirectInferencePort>,
    pub compile_operation_id: String,
    pub compile_attempt_id: String,
    pub compile_exposure_id: String,
    pub verify_operation_id: String,
    pub verify_attempt_id: String,
    pub verify_exposure_id: String,
    pub pending: DirectPendingSlot,
}

impl DirectRealization {
    /// Direct operation identities come from the same configured semantic
    /// capture the native route uses, so a restart or a second configuration
    /// reuses the same durable operations and can never resend them. The port is
    /// supplied by trusted composition; nothing here names an evidence class.
    pub(crate) fn for_semantic(
        port: Arc<dyn DirectInferencePort>,
        semantic: &crate::SemanticNativeConfig,
        pending: DirectPendingSlot,
    ) -> Self {
        Self {
            port,
            compile_operation_id: semantic.compile_invocation_id.clone(),
            compile_attempt_id: semantic.compile_attempt_id.clone(),
            compile_exposure_id: format!("exposure:{}", semantic.compile_invocation_id),
            verify_operation_id: semantic.candidate_verify_invocation_id.clone(),
            verify_attempt_id: semantic.candidate_verify_attempt_id.clone(),
            verify_exposure_id: format!("exposure:{}", semantic.candidate_verify_invocation_id),
            pending,
        }
    }

    fn set_pending(&self, value: Option<DirectPending>) {
        if let Ok(mut slot) = self.pending.lock() {
            *slot = value;
        }
    }
}

/// What driving one prepared call produced.
enum DirectCallOutcome {
    Captured(DirectCapture),
    /// No durable attempt exists and the owner release is not present or not
    /// consistent: nothing was recorded, consumed or sent.
    Pending,
}

fn semantic_store_error(_: SemanticError) -> StoreError {
    StoreError::Rejected
}

fn operation_binding(
    runtime: &SemanticRuntime,
    basis: &SourceBasis,
    role: DirectRole,
    operation_id: &str,
    attempt_id: &str,
    exposure_id: &str,
) -> DirectOperationBinding {
    DirectOperationBinding {
        subject: basis.subject.clone(),
        transition: basis.transition.clone(),
        capture_id: runtime.semantic.capture_id.clone(),
        role,
        policy_id: basis.policy_id.clone(),
        operation_id: operation_id.into(),
        attempt_id: attempt_id.into(),
        exposure_id: exposure_id.into(),
    }
}

/// Whether the store already holds an attempt row for this operation, in any
/// state. Read through the store's recovery projection; it changes nothing.
fn attempt_recorded(store: &SqliteLifecycleStore, operation: &str) -> Result<bool, StoreError> {
    let input = format!("direct_semantic:{operation}");
    Ok(store
        .load_recovery_entries()?
        .iter()
        .any(|entry| entry.input_id == input))
}

/// Offer the exact call to its release owner before any intent exists. A
/// recorded attempt is never offered: `begin_direct_attempt` reports it as it
/// is (terminal, possibly sent, refused or not sent), so a prior possible send
/// or recorded uncertainty can never be hidden behind `Pending`.
fn release_ready(
    store: &SqliteLifecycleStore,
    direct: &DirectRealization,
    call: &DirectPreparedCall,
) -> Result<bool, StoreError> {
    if attempt_recorded(store, &call.binding.operation_id)? {
        return Ok(true);
    }
    match direct.port.offer_release(call) {
        DirectReleaseState::Ready => {
            direct.set_pending(None);
            Ok(true)
        }
        DirectReleaseState::Pending { reason } => {
            direct.set_pending(Some(DirectPending {
                operation_id: call.binding.operation_id.clone(),
                prepared_binding_sha256: call.binding_sha256.clone(),
                reason,
            }));
            Ok(false)
        }
    }
}

/// Commit send intent, submit at most once, and persist the observation before
/// anything interprets it. A possibly sent operation is never offered again.
async fn drive_call(
    store: &mut SqliteLifecycleStore,
    runtime: &SemanticRuntime,
    direct: &DirectRealization,
    context: &ContextGeneration,
    call: &DirectPreparedCall,
) -> Result<DirectCallOutcome, StoreError> {
    if !release_ready(store, direct, call)? {
        return Ok(DirectCallOutcome::Pending);
    }
    direct_attempt(store, runtime, direct, context, call).await
}

/// Commit intent and submit once. An adapter `Withheld` (nothing consumed,
/// nothing sent) withdraws the intent and leaves the call `Pending`; the next
/// drive offers it again and repeats every entry check.
async fn direct_attempt(
    store: &mut SqliteLifecycleStore,
    runtime: &SemanticRuntime,
    direct: &DirectRealization,
    context: &ContextGeneration,
    call: &DirectPreparedCall,
) -> Result<DirectCallOutcome, StoreError> {
    let operation = call.binding.operation_id.clone();
    // Entry is authorized by the same trusted tuple the native control entry
    // compares: this service's acting principal, the subject's current context
    // and the configured control grant.
    let entry = DirectControlEntry {
        principal: &runtime.principal,
        context: context.as_str(),
        grant_id: &runtime.semantic.control_grant_ref,
        grant_revision: runtime.semantic.control_grant_revision,
    };
    let start = store.begin_direct_attempt(call, &entry, &direct.port.evidence(), clock_sample())?;
    match start {
        DirectAttemptStart::Terminal(capture) => Ok(DirectCallOutcome::Captured(*capture)),
        DirectAttemptStart::PossiblySent => Err(StoreError::ClaimConflict),
        DirectAttemptStart::Refused(_) => Err(StoreError::Rejected),
        DirectAttemptStart::NotSent => Err(StoreError::Unavailable),
        DirectAttemptStart::Fresh => {
            if runtime.gate.load(std::sync::atomic::Ordering::SeqCst) {
                // Intent is durable, but nothing was submitted.
                store.record_direct_not_sent(&operation)?;
                return Err(StoreError::ClaimConflict);
            }
            let port = direct.port.clone();
            let offered = call.clone();
            let submitted = tokio::task::spawn_blocking(move || port.submit(&offered)).await;
            match submitted {
                Ok(Ok(capture)) => {
                    store.record_direct_terminal(&operation, &capture, clock_sample())?;
                    Ok(DirectCallOutcome::Captured(capture))
                }
                Ok(Err(DirectSubmitError::Withheld)) => {
                    store.withdraw_direct_intent(&operation, &call.binding_sha256)?;
                    direct.set_pending(Some(DirectPending {
                        operation_id: operation,
                        prepared_binding_sha256: call.binding_sha256.clone(),
                        reason: "release withheld at the send boundary before consumption".into(),
                    }));
                    Ok(DirectCallOutcome::Pending)
                }
                Ok(Err(DirectSubmitError::NotSent)) => {
                    store.record_direct_not_sent(&operation)?;
                    Err(StoreError::Unavailable)
                }
                // The outcome is unknown: leave intent recorded and fence.
                Ok(Err(DirectSubmitError::OutcomeUnknown)) | Err(_) => {
                    Err(StoreError::ClaimConflict)
                }
            }
        }
    }
}

pub(crate) async fn establish_direct(
    store: &mut SqliteLifecycleStore,
    runtime: &SemanticRuntime,
    direct: &DirectRealization,
    subject: &SubjectId,
    context: &ContextGeneration,
    basis: &SourceBasis,
    policy: &SemanticPolicy,
) -> Result<Option<lifecycle_core::CheckpointId>, StoreError> {
    let _ = subject;
    let compile_binding = operation_binding(
        runtime,
        basis,
        DirectRole::Compile,
        &direct.compile_operation_id,
        &direct.compile_attempt_id,
        &direct.compile_exposure_id,
    );
    let (compile_input, _) = direct_compile_input(basis, &compile_binding, policy)
        .map_err(semantic_store_error)?;
    let compile_wire = direct
        .port
        .wire(DirectRole::Compile, &compile_input)
        .map_err(|_| StoreError::Unavailable)?;
    let compile_call = prepare_direct_compile(basis, compile_binding, policy, compile_wire)
        .map_err(semantic_store_error)?;
    let DirectCallOutcome::Captured(compile_capture) =
        drive_call(store, runtime, direct, context, &compile_call).await?
    else {
        return Ok(None);
    };
    let candidate = match direct_candidate_from_capture(basis, &compile_call, &compile_capture, policy)
    {
        Ok(candidate) => candidate,
        Err(
            error @ (SemanticError::InvalidOutput
            | SemanticError::InvalidReference
            | SemanticError::OutputTooLarge
            | SemanticError::WrongBinding),
        ) => {
            // The observation stays; this operation can never be sent again.
            store.record_direct_refusal(&direct.compile_operation_id, &format!("{error:?}"))?;
            return Err(StoreError::Rejected);
        }
        Err(error) => return Err(semantic_store_error(error)),
    };
    let candidate_artifact = store.settle_direct_attempt(&direct.compile_operation_id, clock_sample())?;
    if candidate_artifact != candidate.sha256 {
        return Err(StoreError::ClaimConflict);
    }

    let verify_binding = operation_binding(
        runtime,
        basis,
        DirectRole::Verify,
        &direct.verify_operation_id,
        &direct.verify_attempt_id,
        &direct.verify_exposure_id,
    );
    let (verify_input, _) =
        direct_verify_input(basis, &candidate, &verify_binding, &compile_call, policy)
            .map_err(semantic_store_error)?;
    let verify_wire = direct
        .port
        .wire(DirectRole::Verify, &verify_input)
        .map_err(|_| StoreError::Unavailable)?;
    let verify_call = prepare_direct_verify(
        basis,
        &candidate,
        &compile_call,
        verify_binding,
        policy,
        verify_wire,
    )
    .map_err(semantic_store_error)?;
    // The verify call exists only now, from the settled candidate; its own
    // release is requested for this exact call.
    let DirectCallOutcome::Captured(verify_capture) =
        drive_call(store, runtime, direct, context, &verify_call).await?
    else {
        return Ok(None);
    };
    // A rejected, incomplete or failed verifier call can never publish.
    if verify_capture.terminal != DirectTerminal::Complete {
        return Err(StoreError::Rejected);
    }
    store.settle_direct_attempt(&direct.verify_operation_id, clock_sample())?;
    let assessed = assess_direct_control_candidate(
        basis,
        &candidate,
        &compile_call,
        &verify_call,
        &verify_capture,
        policy,
    )
    .map_err(semantic_store_error)?;
    let checked = assessed.checked;

    let observations = runtime
        .material_port
        .observe_dependencies(&basis.dependencies)
        .map_err(|_| StoreError::Rejected)?;
    if !revalidate_dependencies(&basis.dependencies, &observations)
        .map_err(semantic_store_error)?
        .is_empty()
    {
        store.mark_semantic_capture_stale(&basis.capture_id, &basis.sha256, clock_sample())?;
        return Ok(None);
    }
    let result_id = format!("semantic:{}", direct.verify_operation_id);
    store.record_direct_result(
        &result_id,
        &direct.compile_operation_id,
        &direct.verify_operation_id,
        &checked,
        clock_sample(),
    )?;
    crate::operations::optional_native_barrier(
        runtime.proof_dir.as_deref(),
        "semantic_after_candidate_result",
    )?;
    if checked.status != VerdictStatus::Accepted {
        return Ok(None);
    }
    // The observation above is released before this point (and the post-result
    // barrier can wait). Publication holds the material owner's current fence,
    // so a source update after observation neither publishes nor admits.
    let mut published = None;
    let fenced = runtime
        .material_port
        .with_current_fence(&basis.dependencies, &mut || {
            published = publish_accepted_checkpoint(
                store, runtime, basis, &candidate, policy, &result_id,
            )
            .map_err(crate::ServiceError::Store)?;
            Ok(())
        });
    match fenced {
        Ok(()) => Ok(published),
        Err(crate::ServiceError::Material(crate::MaterialPortError::Conflict)) => {
            store.mark_semantic_capture_stale(&basis.capture_id, &basis.sha256, clock_sample())?;
            Ok(None)
        }
        Err(crate::ServiceError::Store(inner)) => Err(inner),
        Err(_) => Err(StoreError::ClaimConflict),
    }
}
