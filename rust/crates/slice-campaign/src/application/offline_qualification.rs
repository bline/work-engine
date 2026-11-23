//! Task-free B Campaign owner. Compiled only for offline qualification
//! fixtures; no transport, real grant ingestion, or installed activation.

use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::json;

use super::{Campaign, CampaignInner};
use crate::codec::campaign_digest;
use crate::store::offline_qualification::refused;
use crate::store::task_free_b::{Binding as LifecycleBinding, Digest, PURPOSE};
use crate::{CampaignError, CampaignIdentity, CampaignRevision, Result, require_sha};

mod start;
mod terminal;
pub use start::*;
pub use terminal::*;

/// Exact values already selected at the trusted owner, never from a task or
/// client. The serving pin covers the intended serving/endpoint generation.
#[derive(Clone, Debug)]
pub struct SelectionPins {
    pub grant_id: String,
    pub intent_sha256: String,
    pub recipe_sha256: String,
    pub serving_sha256: String,
    pub expires_at_unix_ms: i64,
}

/// Descriptive data, not authority. The operation ID/attempt ID do not depend
/// on request revision, time, recipe, serving generation, or holder identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptBinding {
    root: String,
    identity_key: String,
    head: CampaignRevision,
    grant: String,
    attempt: String,
    operation: String,
    digest: String,
    recipe: String,
    serving: String,
    expiry: i64,
    lifecycle: LifecycleBinding,
}

impl AttemptBinding {
    pub fn root_id(&self) -> &str {
        &self.root
    }
    pub fn identity_key(&self) -> &str {
        &self.identity_key
    }
    pub fn head(&self) -> &CampaignRevision {
        &self.head
    }
    pub fn grant_id(&self) -> &str {
        &self.grant
    }
    pub fn attempt_id(&self) -> &str {
        &self.attempt
    }
    pub fn operation_id(&self) -> &str {
        &self.operation
    }
    pub fn binding_digest(&self) -> &str {
        &self.digest
    }
    pub fn recipe_sha256(&self) -> &str {
        &self.recipe
    }
    pub fn serving_sha256(&self) -> &str {
        &self.serving
    }
    pub fn expires_at(&self) -> i64 {
        self.expiry
    }
    pub(crate) fn lifecycle(&self) -> &LifecycleBinding {
        &self.lifecycle
    }

    /// Version 1, sequence 1, no predecessor or target. The persist gate
    /// recomputes this digest from the fresh owner's binding, not the caller's.
    pub fn create_request_digest(&self) -> Result<String> {
        campaign_digest(&json!([
            "b-offline-operation",
            1,
            1,
            "create",
            self.root_id(),
            self.grant_id(),
            self.attempt_id(),
            self.operation_id(),
            self.binding_digest(),
            null,
            null
        ]))
    }
}

impl Campaign {
    /// Isolated A simulation only. Mutually exclusive with joined attachment.
    pub fn attach_offline_qualification_fixture(&self) -> Result<()> {
        self.owner()?
            .store
            .attach_offline_qualification_fixture(false)
    }

    /// Explicit, one-time A+B attachment in a disposable private root. Never
    /// called by default Campaign initialize/open or any dispatcher.
    pub fn attach_joined_offline_qualification_fixture(&self) -> Result<()> {
        self.owner()?
            .store
            .attach_offline_qualification_fixture(true)
    }

    pub fn select_offline_qualification(
        &self,
        identity: &CampaignIdentity,
        head: &CampaignRevision,
        pins: SelectionPins,
    ) -> Result<AttemptBinding> {
        let mut owner = self.owner()?;
        let binding = owner.qualification_binding(identity, head, pins)?;
        owner.store.register_qualification(&binding)?;
        Ok(binding)
    }

    pub fn revoke_offline_qualification(&self, binding: &AttemptBinding) -> Result<()> {
        let mut owner = self.owner()?;
        check_owner(&owner, binding)?;
        owner.store.revoke_qualification(binding)
    }

    /// Restricted use port, not the selection/revocation owner.
    pub fn offline_qualification_port(&self) -> QualificationPort {
        QualificationPort {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl CampaignInner {
    fn qualification_binding(
        &self,
        identity: &CampaignIdentity,
        head: &CampaignRevision,
        pins: SelectionPins,
    ) -> Result<AttemptBinding> {
        self.check_identity(identity)?;
        self.current(identity, head)?;
        require_sha(&pins.grant_id, "qualification grant ID")?;
        require_sha(&pins.intent_sha256, "qualification intent digest")?;
        require_sha(&pins.recipe_sha256, "qualification recipe digest")?;
        require_sha(&pins.serving_sha256, "qualification serving digest")?;
        let root = self.store.root_id.clone();
        let identity_key = identity.key();
        let (profile, opening_epoch) = self.store.qualification_scope();
        let issuer = campaign_digest(&json!([
            PURPOSE,
            "sole-issuer",
            root,
            self.config.writer_identity
        ]))?;
        // Pin the fixed offline verification policy and synthetic custody roles,
        // not an installed evidence producer or credential.
        let evidence_ingress = start::fixture_ingress_digest()?;
        let attempt = campaign_digest(&json!([PURPOSE, "attempt", root, pins.grant_id]))?;
        let operation = campaign_digest(&json!([PURPOSE, "server-operation", attempt]))?;
        let digest = campaign_digest(&json!([
            PURPOSE,
            "selection",
            root,
            self.config.writer_identity,
            self.config.digest,
            profile,
            opening_epoch,
            issuer,
            evidence_ingress,
            identity_key,
            head,
            pins.grant_id,
            attempt,
            operation,
            pins.intent_sha256,
            pins.recipe_sha256,
            pins.serving_sha256,
            pins.expires_at_unix_ms.to_string()
        ]))?;
        let spend_receipt = campaign_digest(&json!([PURPOSE, "spend-receipt-v1", digest]))?;
        let lifecycle = LifecycleBinding {
            root: root.clone(),
            profile,
            opening_epoch,
            issuer,
            grant: pins.grant_id.clone(),
            attempt: attempt.clone(),
            spend_receipt,
            head: Digest::new(head.as_str().to_owned())?,
            canonical_selection: Digest::new(digest.clone())?,
            recipe: Digest::new(pins.recipe_sha256.clone())?,
            server_operation: operation.clone(),
            serving_generation: Digest::new(pins.serving_sha256.clone())?,
            evidence_ingress: Digest::new(evidence_ingress)?,
        };
        Ok(AttemptBinding {
            root,
            identity_key,
            head: head.clone(),
            grant: pins.grant_id,
            attempt,
            operation,
            digest,
            recipe: pins.recipe_sha256,
            serving: pins.serving_sha256,
            expiry: pins.expires_at_unix_ms,
            lifecycle,
        })
    }
}

fn check_owner(owner: &CampaignInner, binding: &AttemptBinding) -> Result<()> {
    if owner.store.root_id != binding.root || owner.config.identity.key() != binding.identity_key {
        return Err(refused("root or issuer scope differs"));
    }
    Ok(())
}

pub struct QualificationPort {
    inner: Arc<Mutex<CampaignInner>>,
}

impl QualificationPort {
    fn owner(&self) -> Result<MutexGuard<'_, CampaignInner>> {
        self.inner
            .lock()
            .map_err(|_| CampaignError::Integrity("campaign owner gate poisoned".into()))
    }

    /// Seal the single possible entry before offering it for spend. No
    /// entered state observed on restart is converted into this value.
    pub fn enter(&self, binding: &AttemptBinding) -> Result<EnteredQualification> {
        let mut owner = self.owner()?;
        check_owner(&owner, binding)?;
        owner.store.enter_qualification(binding)?;
        Ok(EnteredQualification {
            port: Self {
                inner: Arc::clone(&self.inner),
            },
            binding: binding.clone(),
        })
    }
}

pub struct EnteredQualification {
    port: QualificationPort,
    binding: AttemptBinding,
}

impl EnteredQualification {
    pub fn consume(self) -> Result<SpentQualification> {
        self.consume_with_confirmation(true)
    }

    fn consume_with_confirmation(self, confirm: bool) -> Result<SpentQualification> {
        {
            let mut owner = self.port.owner()?;
            check_owner(&owner, &self.binding)?;
            owner.store.spend_qualification(&self.binding, confirm)?;
        }
        Ok(SpentQualification {
            port: self.port,
            binding: self.binding,
            next: Some(OperationKind::Create),
        })
    }
}

/// Fault injection for the selected offline fixture, not a persistence or
/// recovery option. Lost confirmation can only suppress an inert send.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinedMarkReadback {
    Check,
    LostAfterCommit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationKind {
    Create,
    Start,
    Close,
}

/// One non-serializable, non-cloneable continuation, produced only by fresh
/// committed consume + exact readback. No constructor from binding or status.
pub struct SpentQualification {
    port: QualificationPort,
    binding: AttemptBinding,
    next: Option<OperationKind>,
}

impl SpentQualification {
    pub fn binding(&self) -> &AttemptBinding {
        &self.binding
    }

    /// Consume the fresh handoff even when the request or commit is uncertain.
    /// Only its matching confirmed Create and independent serving-object
    /// readback may advance the process-local continuation to Start.
    pub fn mark_joined_create(
        self,
        request_digest: &str,
        readback: JoinedMarkReadback,
    ) -> Result<MarkedCreate> {
        if self.next != Some(OperationKind::Create)
            || self.binding.create_request_digest()? != request_digest
        {
            return Err(refused("joined Create request differs"));
        }
        let mark = {
            let mut owner = self.port.owner()?;
            check_owner(&owner, &self.binding)?;
            owner.store.mark_qualification_fixture(
                &self.binding,
                Digest::new(request_digest.to_owned())?,
                readback,
            )?
        };
        Ok(MarkedCreate {
            port: self.port,
            binding: self.binding,
            mark,
            request: request_digest.to_owned(),
        })
    }

    /// A-only marker simulation. Joined roots refuse before invoking this
    /// closure, rather than allowing a fallback to a simulated mark.
    pub fn mark_inert_send<T>(
        &mut self,
        kind: OperationKind,
        inert_mark: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        if self.next.take() != Some(kind) {
            return Err(refused("operation out of order"));
        }
        let mut owner = self.port.owner()?;
        check_owner(&owner, &self.binding)?;
        let mark = owner
            .store
            .mark_isolated_fixture(&self.binding, inert_mark)?;
        self.next = match kind {
            OperationKind::Create => Some(OperationKind::Start),
            OperationKind::Start => Some(OperationKind::Close),
            OperationKind::Close => None,
        };
        Ok(mark)
    }
}

#[cfg(test)]
mod tests;
