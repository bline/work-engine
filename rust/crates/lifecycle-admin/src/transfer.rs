//! Offline current-custody transfer through sole-store operations.

use std::fs;
use std::path::Path;

use lifecycle_core::HandoffId;
use lifecycle_store::{
    AdoptionReceipt, CapsuleReceipt, FenceReceipt, OwnershipReceipt, SqliteLifecycleStore,
    StoreError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::selection::{
    LaunchSelectionGuard, LaunchSelectionPort, SelectionError, SelectionRecord,
};

#[derive(Debug, Error)]
pub enum TransferError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("lifecycle store {stage} failed: {source}")]
    StoreStage {
        stage: &'static str,
        source: StoreError,
    },
    #[error(transparent)]
    Selection(#[from] SelectionError),
    #[error("source, capsule, destination or selection identity conflicts")]
    Identity,
    #[error("destination or capsule root must be absent for a new transfer")]
    MustBeAbsent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransferReceipt {
    pub capsule: CapsuleReceipt,
    pub adoption: AdoptionReceipt,
    pub source_fence: FenceReceipt,
    pub destination_ownership: OwnershipReceipt,
    pub selected: SelectionRecord,
}

/// Durable boundaries observed by the disposable process-cut harness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferCut {
    Exported,
    AdoptedInactive,
    SourceFenced,
    SelectionCommitted,
    DestinationOwned,
}

impl TransferCut {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exported => "exported",
            Self::AdoptedInactive => "adopted_inactive",
            Self::SourceFenced => "source_fenced",
            Self::SelectionCommitted => "selection_committed",
            Self::DestinationOwned => "destination_owned",
        }
    }
}

/// Export the current source, accept into an absent inactive destination,
/// then transfer ownership while holding one external launch guard.
pub fn transfer_current<P: LaunchSelectionPort>(
    source: &mut SqliteLifecycleStore,
    selection: &mut P,
    expected_selection: &SelectionRecord,
    capsule_root: &Path,
    destination_root: &Path,
    handoff: &HandoffId,
    trusted_issuer: String,
) -> Result<TransferReceipt, TransferError> {
    transfer_current_with_barrier(
        source,
        selection,
        expected_selection,
        capsule_root,
        destination_root,
        handoff,
        trusted_issuer,
        &mut |_| {},
    )
}

/// A barrier may observe a durable owner receipt, but cannot create one.
/// Each explicit argument binds a separate custody or launch identity at a crash boundary.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn transfer_current_with_barrier<P: LaunchSelectionPort>(
    source: &mut SqliteLifecycleStore,
    selection: &mut P,
    expected_selection: &SelectionRecord,
    capsule_root: &Path,
    destination_root: &Path,
    handoff: &HandoffId,
    trusted_issuer: String,
    barrier: &mut impl FnMut(TransferCut),
) -> Result<TransferReceipt, TransferError> {
    if capsule_root.exists() || destination_root.exists() {
        return Err(TransferError::MustBeAbsent);
    }
    preflight_paths(source.root(), capsule_root, destination_root)?;
    let capsule =
        source
            .export_current(capsule_root)
            .map_err(|source| TransferError::StoreStage {
                stage: "export_current",
                source,
            })?;
    barrier(TransferCut::Exported);
    promote_capsule(
        source,
        selection,
        expected_selection,
        &capsule,
        destination_root,
        handoff,
        trusted_issuer,
        barrier,
    )
}

/// Resume the same handoff after a lost response or a process crash. Capsule
/// and adoption readback are owned and revalidated by lifecycle-store.
pub fn resume_transfer<P: LaunchSelectionPort>(
    source: &mut SqliteLifecycleStore,
    selection: &mut P,
    expected_selection: &SelectionRecord,
    capsule_root: &Path,
    destination_root: &Path,
    handoff: &HandoffId,
    trusted_issuer: String,
) -> Result<TransferReceipt, TransferError> {
    resume_transfer_with_barrier(
        source,
        selection,
        expected_selection,
        capsule_root,
        destination_root,
        handoff,
        trusted_issuer,
        &mut |_| {},
    )
}

#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn resume_transfer_with_barrier<P: LaunchSelectionPort>(
    source: &mut SqliteLifecycleStore,
    selection: &mut P,
    expected_selection: &SelectionRecord,
    capsule_root: &Path,
    destination_root: &Path,
    handoff: &HandoffId,
    trusted_issuer: String,
    barrier: &mut impl FnMut(TransferCut),
) -> Result<TransferReceipt, TransferError> {
    if !capsule_root.exists() {
        return Err(TransferError::Identity);
    }
    preflight_paths(source.root(), capsule_root, destination_root)?;
    let capsule = SqliteLifecycleStore::read_capsule(capsule_root).map_err(|source| {
        TransferError::StoreStage {
            stage: "read_capsule",
            source,
        }
    })?;
    promote_capsule(
        source,
        selection,
        expected_selection,
        &capsule,
        destination_root,
        handoff,
        trusted_issuer,
        barrier,
    )
}

#[allow(clippy::too_many_arguments)]
fn promote_capsule<P: LaunchSelectionPort>(
    source: &mut SqliteLifecycleStore,
    selection: &mut P,
    expected_selection: &SelectionRecord,
    capsule: &CapsuleReceipt,
    destination_root: &Path,
    handoff: &HandoffId,
    trusted_issuer: String,
    barrier: &mut impl FnMut(TransferCut),
) -> Result<TransferReceipt, TransferError> {
    if expected_selection.root != source.root()
        || expected_selection.store_id != capsule.source_store_id
    {
        return Err(TransferError::Identity);
    }
    // The guard covers every shared launch-selection and store-ownership step.
    let mut guard = selection.lock()?;
    let selected = guard.read()?;
    let before_or_successor = selected == *expected_selection
        || (selected.generation == expected_selection.generation.saturating_add(1)
            && selected.handoff_id.as_deref() == Some(handoff.as_str())
            && selected.capsule_digest.as_deref() == Some(capsule.capsule_sha256.as_str()));
    if !before_or_successor {
        return Err(TransferError::Identity);
    }

    let (mut destination, adoption) = if destination_root.exists() {
        // A crash after durable DB copy leaves a store-owned adoption intent.
        // The exact capsule can complete that intent; a committed adoption has
        // no intent and is read back through the ordinary owner API.
        SqliteLifecycleStore::adopt_inactive(
            capsule,
            destination_root,
            handoff,
            trusted_issuer.clone(),
        )
        .or_else(|error| {
            if matches!(error, StoreError::ClaimConflict) {
                SqliteLifecycleStore::open_adopted_inactive(
                    destination_root,
                    handoff,
                    &capsule.capsule_sha256,
                    trusted_issuer,
                )
            } else {
                Err(error)
            }
        })
        .map_err(|source| TransferError::StoreStage {
            stage: "resume_adoption",
            source,
        })?
    } else {
        SqliteLifecycleStore::adopt_inactive(capsule, destination_root, handoff, trusted_issuer)
            .map_err(|source| TransferError::StoreStage {
                stage: "adopt_inactive",
                source,
            })?
    };
    if adoption.source_store_id != capsule.source_store_id
        || adoption.capsule_sha256 != capsule.capsule_sha256
        || adoption.handoff_id != handoff.as_str()
        || adoption.destination_root != destination_root
    {
        return Err(TransferError::Identity);
    }
    barrier(TransferCut::AdoptedInactive);
    let destination_canonical =
        fs::canonicalize(destination_root).map_err(|_| TransferError::Identity)?;
    let successor = SelectionRecord {
        generation: expected_selection
            .generation
            .checked_add(1)
            .ok_or(TransferError::Identity)?,
        store_id: adoption.destination_store_id.clone(),
        root: destination_canonical,
        handoff_id: Some(handoff.as_str().to_owned()),
        capsule_digest: Some(capsule.capsule_sha256.clone()),
    };
    if selected != *expected_selection && selected != successor {
        return Err(TransferError::Identity);
    }
    let fence =
        source
            .fence_source(handoff, &adoption)
            .map_err(|source| TransferError::StoreStage {
                stage: "fence_source",
                source,
            })?;
    barrier(TransferCut::SourceFenced);
    let cas = guard.compare_and_swap(expected_selection, successor.clone())?;
    barrier(TransferCut::SelectionCommitted);
    let evidence = guard.evidence(&cas)?;
    let ownership = destination
        .commit_destination(handoff, &fence, &evidence)
        .map_err(|source| TransferError::StoreStage {
            stage: "commit_destination",
            source,
        })?;
    barrier(TransferCut::DestinationOwned);
    if ownership.destination_store_id != adoption.destination_store_id
        || ownership.capsule_sha256 != capsule.capsule_sha256
        || ownership.selection_revision != successor.generation
    {
        return Err(TransferError::Identity);
    }
    Ok(TransferReceipt {
        capsule: capsule.clone(),
        adoption,
        source_fence: fence,
        destination_ownership: ownership,
        selected: successor,
    })
}

fn preflight_paths(source: &Path, capsule: &Path, destination: &Path) -> Result<(), TransferError> {
    if !capsule.is_absolute() || !destination.is_absolute() || capsule == destination {
        return Err(TransferError::Identity);
    }
    for candidate in [capsule, destination] {
        let parent = candidate.parent().ok_or(TransferError::Identity)?;
        let canonical = fs::canonicalize(parent).map_err(|_| TransferError::Identity)?;
        if canonical != parent || candidate.starts_with(source) || source.starts_with(candidate) {
            return Err(TransferError::Identity);
        }
    }
    Ok(())
}
