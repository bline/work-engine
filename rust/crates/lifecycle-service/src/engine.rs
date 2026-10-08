use lifecycle_core::{
    CheckpointId, ContextGeneration, EvidenceId, GrantId, ProviderThreadId, Revision, SubjectId,
    TransitionFact, TransitionId, TransitionStage,
};
use lifecycle_runtime::{ActivationState, CloseDisposition, ProcessSupervisor};
use lifecycle_store::{SqliteLifecycleStore, StoreError};
use serde::Deserialize;
use std::{
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::process::Command as ProcessCommand;
use work_engine_types::{CodecContract, IdValue};

use crate::{
    ServiceConfig,
    actor::{clock_sample, proof_barrier},
};

/// The controlled peer's exact final response is a receipt with supplied continuation facts.
/// It is consumed only after that response and its source/attempt have committed in S1.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ControlledContinuation {
    protocol_version: u16,
    predecessor_safe: bool,
    source_frozen: bool,
    checkpoint_ready: bool,
    switch_authorized: bool,
    successor_observed: bool,
    rehydration_verified: bool,
    successor_context: String,
    successor_thread: String,
}

/// The socket signal handler closes this gate without waiting for the actor.
/// Each verifier retains its own exact child registry until drained.
pub struct VerifierGate {
    state: Mutex<VerifierGateState>,
}

struct VerifierGateState {
    closed: bool,
    active: Option<Arc<ProcessSupervisor>>,
}

impl VerifierGate {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(VerifierGateState {
                closed: false,
                active: None,
            }),
        }
    }

    pub fn close_gate(&self) {
        let mut state = self.state.lock().unwrap();
        state.closed = true;
        if let Some(supervisor) = &state.active {
            supervisor.close_gate();
        }
    }

    fn register(&self) -> Result<VerifierLease<'_>, StoreError> {
        let supervisor = Arc::new(ProcessSupervisor::new(1).map_err(|_| StoreError::Unavailable)?);
        let mut state = self.state.lock().unwrap();
        if state.closed || state.active.is_some() {
            return Err(StoreError::Rejected);
        }
        state.active = Some(supervisor.clone());
        Ok(VerifierLease {
            gate: self,
            supervisor,
        })
    }
}

struct VerifierLease<'a> {
    gate: &'a VerifierGate,
    supervisor: Arc<ProcessSupervisor>,
}

impl Drop for VerifierLease<'_> {
    fn drop(&mut self) {
        self.supervisor.close_gate();
        let mut state = self.gate.state.lock().unwrap();
        if state
            .active
            .as_ref()
            .is_some_and(|active| Arc::ptr_eq(active, &self.supervisor))
        {
            state.active = None;
        }
    }
}

pub async fn drive_transition(
    store: &mut SqliteLifecycleStore,
    config: &ServiceConfig,
    verifier_gate: &VerifierGate,
) -> Result<(), StoreError> {
    let subject = SubjectId::parse(config.subject_id.clone()).map_err(|_| StoreError::Rejected)?;
    for _ in 0..8 {
        let snapshot = store.subject_projection(&subject, None, None)?;
        if snapshot.owner_kind != "transition" {
            return Ok(());
        }
        let Some(transition) = snapshot.transition else {
            return Err(StoreError::Unavailable);
        };
        let id =
            TransitionId::parse(transition.transition_id).map_err(|_| StoreError::Unavailable)?;
        let Some(result) = store.committed_text_for_transition(&id)? else {
            return Ok(());
        };
        let continuation: ControlledContinuation = match serde_json::from_str(&result.final_text) {
            Ok(value) => value,
            Err(_) => return Ok(()),
        };
        if continuation.protocol_version != 1
            || IdValue::parse(continuation.successor_context.clone()).is_err()
            || IdValue::parse(continuation.successor_thread.clone()).is_err()
            || !config.grants.iter().any(|grant| {
                grant.context_generation == continuation.successor_context
                    && grant.scope == "enqueue_input"
            })
        {
            return Ok(());
        }
        let stage = TransitionStage::parse(&transition.stage).ok_or(StoreError::Unavailable)?;
        let evidence = |suffix: &str| {
            EvidenceId::parse(format!("{}:{suffix}", result.source_id))
                .map_err(|_| StoreError::Unavailable)
        };
        let fact = match stage {
            TransitionStage::Quiescing if continuation.predecessor_safe => {
                TransitionFact::PredecessorSafe {
                    evidence: evidence("predecessor-safe")?,
                }
            }
            TransitionStage::Capturing if continuation.source_frozen => {
                TransitionFact::SourceFrozen {
                    evidence: evidence("source-frozen")?,
                    semantic_revision: Revision::new(snapshot.semantic_revision),
                }
            }
            TransitionStage::Verifying if continuation.checkpoint_ready => {
                let digest = CodecContract::BinaryArtifactV1
                    .digest_binary(result.final_text.as_bytes())
                    .map_err(|_| StoreError::Unavailable)?;
                let verified = match store.verifier_result(&id)? {
                    Some(value) => value,
                    None => {
                        verify_external_child(
                            store,
                            config,
                            verifier_gate,
                            &id,
                            &result.source_id,
                            &digest.hex(),
                        )
                        .await?
                    }
                };
                if verified {
                    TransitionFact::VerificationEstablished {
                        evidence: evidence("checkpoint")?,
                        checkpoint: CheckpointId::parse(format!("checkpoint:{}", digest.hex()))
                            .map_err(|_| StoreError::Unavailable)?,
                    }
                } else {
                    TransitionFact::Unresolved {
                        evidence: evidence("verifier-failure")?,
                    }
                }
            }
            TransitionStage::Checkpointed if continuation.switch_authorized => {
                TransitionFact::SwitchAuthorized {
                    evidence: evidence("switch")?,
                }
            }
            TransitionStage::Switching if continuation.successor_observed => {
                TransitionFact::SuccessorObserved {
                    evidence: evidence("successor")?,
                    context: ContextGeneration::parse(continuation.successor_context.clone())
                        .map_err(|_| StoreError::Unavailable)?,
                    thread: ProviderThreadId::parse(continuation.successor_thread.clone())
                        .map_err(|_| StoreError::Unavailable)?,
                }
            }
            TransitionStage::Rehydrating if continuation.rehydration_verified => {
                TransitionFact::RehydrationVerified {
                    evidence: evidence("rehydration")?,
                }
            }
            TransitionStage::ReadyToCommit => TransitionFact::CommitSuccessor {
                evidence: evidence("commit")?,
                semantic_revision: Revision::new(snapshot.semantic_revision),
            },
            _ => return Ok(()),
        };
        let successor_grant = config
            .grants
            .iter()
            .find(|grant| {
                grant.context_generation == continuation.successor_context
                    && grant.scope == "enqueue_input"
            })
            .map(|grant| {
                GrantId::parse(grant.grant_ref.clone()).map_err(|_| StoreError::Unavailable)
            })
            .transpose()?;
        store.apply_transition_fact(
            &id,
            Revision::new(snapshot.revision),
            fact,
            successor_grant.as_ref(),
            clock_sample(),
        )?;
    }
    Ok(())
}

async fn verify_external_child(
    store: &mut SqliteLifecycleStore,
    config: &ServiceConfig,
    verifier_gate: &VerifierGate,
    transition: &TransitionId,
    source: &str,
    digest: &str,
) -> Result<bool, StoreError> {
    let executable = config
        .verifier_executable
        .as_ref()
        .ok_or(StoreError::Rejected)?;
    let ledger = config
        .verifier_ledger_path
        .as_ref()
        .ok_or(StoreError::Rejected)?;
    let lease = verifier_gate.register()?;
    let supervisor = &lease.supervisor;
    let mut command = ProcessCommand::new(executable);
    command
        .arg("verify")
        .arg(ledger)
        .arg(source)
        .arg(digest)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(root) = &config.proof_barrier_dir {
        command.env("LIFECYCLE_VERIFIER_BARRIER_DIR", root);
    }
    proof_barrier(config, "verifier_before_launch").await?;
    let child = supervisor.launch(&mut command);
    let (
        activation,
        disposition,
        primary_error,
        exit_code,
        success,
        unsafe_at_close,
        late_at_close,
    ) = match child {
        Ok(child) => {
            proof_barrier(config, "verifier_after_spawn").await?;
            let activated = supervisor.activate(child).await.is_ok();
            let now = Instant::now();
            let proof_deadline =
                now + Duration::from_millis(config.verifier_proof_ms.unwrap_or(5_000));
            let report = supervisor
                .close_and_drain(
                    proof_deadline,
                    now + Duration::from_millis(config.verifier_cleanup_ms.unwrap_or(6_000)),
                )
                .await;
            let close = report
                .children
                .iter()
                .find(|value| value.child == child)
                .ok_or(StoreError::Unavailable)?;
            let success = activated
                && close.activation == ActivationState::Confirmed
                && close.disposition == CloseDisposition::Resolved
                && close.exit.as_ref().is_some_and(|exit| exit.success)
                && report.unresolved.is_empty();
            let late = close.disposition == CloseDisposition::ResolvedLate;
            let primary_error = if success {
                "none"
            } else if !activated {
                "activation_uncertain"
            } else if late {
                "proof_deadline_expired"
            } else if close.disposition != CloseDisposition::Resolved {
                "cleanup_unresolved"
            } else {
                "verification_failed"
            };
            (
                format!("{:?}", close.activation),
                format!("{:?}", close.disposition),
                primary_error.to_owned(),
                close.exit.as_ref().and_then(|exit| exit.code),
                success,
                report.unsafe_at_close,
                late,
            )
        }
        Err(_) => (
            "NotAttempted".into(),
            "Rejected".into(),
            "launch_rejected".into(),
            None,
            false,
            false,
            false,
        ),
    };
    store.record_verifier_result(
        transition,
        &EvidenceId::parse(source.to_owned()).map_err(|_| StoreError::Unavailable)?,
        &activation,
        &disposition,
        &primary_error,
        exit_code,
        success,
        unsafe_at_close,
        late_at_close,
        clock_sample().wall,
    )?;
    Ok(success)
}
