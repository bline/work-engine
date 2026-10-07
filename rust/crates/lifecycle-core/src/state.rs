//! Pure transition-stage ordering. Trusted producers and durable fact application belong to the service/store.

use crate::{
    CheckpointId, ContextGeneration, EvidenceId, ProviderThreadId, Revision, SubjectId,
    TransitionId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionStage {
    Quiescing,
    Capturing,
    Verifying,
    Checkpointed,
    Switching,
    Rehydrating,
    ReadyToCommit,
    Reconciled,
    RecoveryRequired,
    AbortedBeforeEntry,
}

impl TransitionStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Quiescing => "quiescing",
            Self::Capturing => "capturing",
            Self::Verifying => "verifying",
            Self::Checkpointed => "checkpointed",
            Self::Switching => "switching",
            Self::Rehydrating => "rehydrating",
            Self::ReadyToCommit => "ready_to_commit",
            Self::Reconciled => "reconciled",
            Self::RecoveryRequired => "recovery_required",
            Self::AbortedBeforeEntry => "aborted_before_entry",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "quiescing" => Some(Self::Quiescing),
            "capturing" => Some(Self::Capturing),
            "verifying" => Some(Self::Verifying),
            "checkpointed" => Some(Self::Checkpointed),
            "switching" => Some(Self::Switching),
            "rehydrating" => Some(Self::Rehydrating),
            "ready_to_commit" => Some(Self::ReadyToCommit),
            "reconciled" => Some(Self::Reconciled),
            "recovery_required" => Some(Self::RecoveryRequired),
            "aborted_before_entry" => Some(Self::AbortedBeforeEntry),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionState {
    pub id: TransitionId,
    pub subject: SubjectId,
    pub predecessor: ContextGeneration,
    pub stage: TransitionStage,
    pub basis_semantic_revision: Revision,
    pub checkpoint: Option<CheckpointId>,
    pub successor: Option<ContextGeneration>,
    pub successor_thread: Option<ProviderThreadId>,
    pub evidence: Vec<EvidenceId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionFact {
    PredecessorSafe {
        evidence: EvidenceId,
    },
    SourceFrozen {
        evidence: EvidenceId,
        semantic_revision: Revision,
    },
    VerificationEstablished {
        evidence: EvidenceId,
        checkpoint: CheckpointId,
    },
    SwitchAuthorized {
        evidence: EvidenceId,
    },
    SuccessorObserved {
        evidence: EvidenceId,
        context: ContextGeneration,
        thread: ProviderThreadId,
    },
    RehydrationVerified {
        evidence: EvidenceId,
    },
    CommitSuccessor {
        evidence: EvidenceId,
        semantic_revision: Revision,
    },
    Unresolved {
        evidence: EvidenceId,
    },
    AbortNoEntry {
        evidence: EvidenceId,
    },
}

impl TransitionFact {
    pub fn evidence(&self) -> &EvidenceId {
        match self {
            Self::PredecessorSafe { evidence }
            | Self::SourceFrozen { evidence, .. }
            | Self::VerificationEstablished { evidence, .. }
            | Self::SwitchAuthorized { evidence }
            | Self::SuccessorObserved { evidence, .. }
            | Self::RehydrationVerified { evidence }
            | Self::CommitSuccessor { evidence, .. }
            | Self::Unresolved { evidence }
            | Self::AbortNoEntry { evidence } => evidence,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionFailure {
    WrongStage,
    StaleSemanticBasis,
    MissingCheckpoint,
    MissingSuccessor,
    DuplicateEvidence,
}

pub fn reduce_transition(
    current: &TransitionState,
    fact: TransitionFact,
) -> Result<TransitionState, TransitionFailure> {
    let evidence = fact.evidence().clone();
    if current.evidence.contains(&evidence) {
        return Err(TransitionFailure::DuplicateEvidence);
    }
    let mut next = current.clone();
    match (current.stage, fact) {
        (TransitionStage::Quiescing, TransitionFact::PredecessorSafe { .. }) => {
            next.stage = TransitionStage::Capturing;
        }
        (
            TransitionStage::Capturing,
            TransitionFact::SourceFrozen {
                semantic_revision, ..
            },
        ) => {
            if semantic_revision != current.basis_semantic_revision {
                return Err(TransitionFailure::StaleSemanticBasis);
            }
            next.stage = TransitionStage::Verifying;
        }
        (
            TransitionStage::Verifying,
            TransitionFact::VerificationEstablished { checkpoint, .. },
        ) => {
            next.checkpoint = Some(checkpoint);
            next.stage = TransitionStage::Checkpointed;
        }
        (TransitionStage::Checkpointed, TransitionFact::SwitchAuthorized { .. }) => {
            if current.checkpoint.is_none() {
                return Err(TransitionFailure::MissingCheckpoint);
            }
            next.stage = TransitionStage::Switching;
        }
        (
            TransitionStage::Switching,
            TransitionFact::SuccessorObserved {
                context, thread, ..
            },
        ) => {
            next.successor = Some(context);
            next.successor_thread = Some(thread);
            next.stage = TransitionStage::Rehydrating;
        }
        (TransitionStage::Rehydrating, TransitionFact::RehydrationVerified { .. }) => {
            if current.successor.is_none() || current.successor_thread.is_none() {
                return Err(TransitionFailure::MissingSuccessor);
            }
            next.stage = TransitionStage::ReadyToCommit;
        }
        (
            TransitionStage::ReadyToCommit,
            TransitionFact::CommitSuccessor {
                semantic_revision, ..
            },
        ) => {
            if semantic_revision != current.basis_semantic_revision {
                return Err(TransitionFailure::StaleSemanticBasis);
            }
            if current.checkpoint.is_none()
                || current.successor.is_none()
                || current.successor_thread.is_none()
            {
                return Err(TransitionFailure::MissingSuccessor);
            }
            next.stage = TransitionStage::Reconciled;
        }
        (
            TransitionStage::Quiescing | TransitionStage::Capturing | TransitionStage::Verifying,
            TransitionFact::AbortNoEntry { .. },
        ) => {
            next.stage = TransitionStage::AbortedBeforeEntry;
        }
        (
            TransitionStage::Quiescing
            | TransitionStage::Capturing
            | TransitionStage::Verifying
            | TransitionStage::Checkpointed
            | TransitionStage::Switching
            | TransitionStage::Rehydrating
            | TransitionStage::ReadyToCommit,
            TransitionFact::Unresolved { .. },
        ) => {
            next.stage = TransitionStage::RecoveryRequired;
        }
        _ => return Err(TransitionFailure::WrongStage),
    }
    next.evidence.push(evidence);
    Ok(next)
}
