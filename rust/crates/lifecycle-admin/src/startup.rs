//! Bind a component service launch to the committed offline selection.

use std::path::PathBuf;

use lifecycle_core::HandoffId;
use lifecycle_store::{SqliteLifecycleStore, StoreError};
use thiserror::Error;

use crate::selection::{LaunchSelectionGuard, SelectionError};
use crate::transfer::TransferReceipt;

#[derive(Debug, Error)]
pub enum StartupError {
    #[error(transparent)]
    Selection(#[from] SelectionError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("selected launch does not match the committed handoff and ownership")]
    Mismatch,
}

/// The caller keeps `guard` alive while it launches the selected service.
/// This verifies the selected root, both owner projections and the exact
/// handoff receipt through store APIs, without manufacturing custody facts.
pub fn verify_selected_launch(
    guard: &impl LaunchSelectionGuard,
    transfer: &TransferReceipt,
    source: &SqliteLifecycleStore,
    destination: &SqliteLifecycleStore,
) -> Result<PathBuf, StartupError> {
    let selected = guard.read()?;
    if selected != transfer.selected
        || selected.root != destination.root()
        || transfer.source_fence.source_root != source.root()
        || selected.store_id != transfer.destination_ownership.destination_store_id
        || selected.handoff_id.as_deref() != Some(transfer.adoption.handoff_id.as_str())
        || selected.capsule_digest.as_deref() != Some(transfer.capsule.capsule_sha256.as_str())
    {
        return Err(StartupError::Mismatch);
    }
    let handoff = HandoffId::parse(transfer.adoption.handoff_id.clone())
        .map_err(|_| StartupError::Mismatch)?;
    let source_owner = source.admin_ownership()?;
    let destination_owner = destination.admin_ownership()?;
    let source_handoff = source
        .read_handoff(&handoff)?
        .ok_or(StartupError::Mismatch)?;
    let destination_handoff = destination
        .read_handoff(&handoff)?
        .ok_or(StartupError::Mismatch)?;
    if source_owner.status != "fenced"
        || source_owner.store_id != transfer.source_fence.source_store_id
        || source_owner.handoff_id.as_deref() != Some(handoff.as_str())
        || source_owner.peer_store_id.as_deref() != Some(selected.store_id.as_str())
        || source_owner.capsule_sha256.as_deref() != selected.capsule_digest.as_deref()
        || source_handoff.status != "source_fenced"
        || source_handoff.source_store_id != source_owner.store_id
        || source_handoff.destination_store_id != selected.store_id
        || destination_owner.status != "active"
        || destination_owner.store_id != selected.store_id
        || destination_owner.handoff_id.as_deref() != Some(handoff.as_str())
        || destination_owner.peer_store_id.as_deref() != Some(source_owner.store_id.as_str())
        || destination_owner.capsule_sha256.as_deref() != selected.capsule_digest.as_deref()
        || destination_owner.selection_revision != Some(selected.generation)
        || destination_handoff.status != "destination_owned"
        || destination_handoff.source_store_id != source_owner.store_id
        || destination_handoff.destination_store_id != selected.store_id
        || destination_handoff.capsule_sha256 != transfer.capsule.capsule_sha256
    {
        return Err(StartupError::Mismatch);
    }
    Ok(selected.root)
}
