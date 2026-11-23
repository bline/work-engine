//! Data-only offline candidate for the installed fact shapes. This module has
//! no acquisition, authentication transport, effect, or recovery capability.
//! Public inputs are fixture stand-ins for the named protected sources, not
//! independently authenticated evidence from an installation.

use serde::Serialize;

use super::{AttemptBinding, FixtureRunning, QualificationPort, TerminalFailure, check_owner};
use crate::{Result, application::offline_qualification::JoinedMarkReadback};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfflineCgroupHandle {
    // The protected capture slot retaining the open cgroup-v2 directory; an
    // inode or a path alone is not a continuity handle.
    pub capture: String,
    pub mount_id: u64,
    pub inode: u64,
}

/// Capture before Stop and while the Start-associated init is live. Boot and
/// namespace are canonical observed identifiers, not hashes of desired IDs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfflineLiveSubtree {
    pub running: FixtureRunning,
    pub boot: String,
    pub runtime_namespace: String,
    pub cgroup: OfflineCgroupHandle,
    pub capture_position: u64,
    pub cgroup_events: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfflineTerminalExecution {
    pub selection: String,
    pub create_request: String,
    pub create_mark: String,
    pub start_mark: String,
    pub live: OfflineLiveSubtree,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfflineStopSubject {
    pub execution: OfflineTerminalExecution,
    pub close_request: String,
    pub close_mark: String,
}

/// Position in the same attempt-bound capture stream. Ordering is only useful
/// if supplied by the protected source; caller time or file mtime is not enough.
#[derive(Clone, Debug, Serialize)]
pub struct OfflineStopReadback<T> {
    pub subject: OfflineStopSubject,
    pub position: u64,
    pub observed: T,
}

/// Diagnostic of the original call, not a settlement predicate. No reply is
/// inferred from later task observations, nor is a missing receipt success.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OfflineStopResponse {
    ReturnedSuccess,
    ReturnedFailure,
    Lost,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OfflineOldTask {
    // Runtime State on the anchored old task, not Docker Stopped.
    Stopped(OfflineOldTaskTransitions),
    // A positive, execution-revoking disposition of that old object, not an
    // absent lookup, generic remove return, or ordinary TaskDelete event.
    PositivelyRetired,
    AbsentOrUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OfflineOldTaskTransitions {
    /// Retained containerd v2.3.3/runc audit: no old-object Start/Resume/Exec
    /// transitions from STOPPED. Not a claim about earlier work or recovery.
    PinnedContainerd233RuncStopped,
    Unestablished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OfflineAcceptedDisposition {
    Completed,
    // A positive revocation of execution authority for admitted work, not
    // caller cancellation. This does not claim a successful Stop response.
    EffectivelyRevoked,
    Pending,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OfflineCleanupTail {
    Settled,
    CannotExecuteOrReopen,
    CanExecuteOrReopen,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct OfflineClosedFrontier {
    // Fixed typed slots of the only admitted operation budget: 1 Create,
    // 1 Start, 1 scoped Stop. Identifiers are not caller-selected proof hashes.
    pub create: OfflineAcceptedDisposition,
    pub start: OfflineAcceptedDisposition,
    pub stop: OfflineAcceptedDisposition,
    pub other_work: OfflineOtherWork,
    pub cleanup_tail: OfflineCleanupTail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OfflineOtherWork {
    /// Protected acceptance cut over both the controller and selected runtime,
    /// including internal/delayed work; no accepted action outside the frontier.
    ExcludedByClosedAcceptanceCut,
    /// Affirmative evidence of work outside the fixed acceptance frontier at
    /// this cut, not a replay of a historical pending-work observation.
    AdditionalAcceptedWork,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OfflineEntryFence {
    Enforced,
    Open,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct OfflineNewIngress {
    /// Protected controller closes Create/Start/replacement/delivery/release.
    pub controller: OfflineEntryFence,
    /// Exact backend object and aliases cannot enter through selected Docker.
    pub engine: OfflineEntryFence,
    /// No direct/runtime bypass or fresh Start/Exec path for this execution.
    pub runtime: OfflineEntryFence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OfflineRecoveryBacking {
    PositivelyRetired,
    RetainedExecutionInert,
    Restorable,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct OfflineRecoveryAuthority {
    /// Positive disposition of the attempt's exact recovery backing. Absence
    /// in a registry or an unset restart option are not such a disposition.
    pub backing: OfflineRecoveryBacking,
    /// Durable controller recovery cannot replay or replace this attempt.
    pub controller_reconcile: OfflineEntryFence,
    /// Runtime reconciliation, restore and auto-start cannot execute it.
    pub runtime_reconcile: OfflineEntryFence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum OfflineContainment {
    ContinuousThroughFinalRead,
    MigrationOrContinuityUnknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct OfflineSubtreeEnd {
    /// Exit tied to the original retained task handle, not PID/name lookup.
    pub exited_init_lifetime: Option<String>,
    pub cgroup: OfflineCgroupHandle,
    pub containment: OfflineContainment,
    pub cgroup_events: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct OfflineStopTerminalReadbacks {
    pub old_task: Option<OfflineStopReadback<OfflineOldTask>>,
    pub ingress: Option<OfflineStopReadback<OfflineNewIngress>>,
    pub accepted: Option<OfflineStopReadback<OfflineClosedFrontier>>,
    pub recovery: Option<OfflineStopReadback<OfflineRecoveryAuthority>>,
    pub end: Option<OfflineStopReadback<OfflineSubtreeEnd>>,
}

impl QualificationPort {
    /// Model the one checked pre-effect mark in the current Campaign owner. No
    /// sender/callback is accepted and the result is evidence data only. One
    /// lost mark readback suppresses any hypothetical send, not the mark.
    pub fn mark_offline_scoped_stop(
        &self,
        binding: &AttemptBinding,
        live: &OfflineLiveSubtree,
        readback: JoinedMarkReadback,
    ) -> Result<OfflineStopSubject> {
        let mut owner = self.owner()?;
        check_owner(&owner, binding)?;
        owner
            .store
            .mark_offline_scoped_stop(binding, live, readback)
    }

    /// Historical checked reconstruction of the subject only, no send permit.
    pub fn read_offline_scoped_stop(
        &self,
        binding: &AttemptBinding,
        live: &OfflineLiveSubtree,
    ) -> Result<OfflineStopSubject> {
        let mut owner = self.owner()?;
        check_owner(&owner, binding)?;
        owner.store.read_offline_scoped_stop(binding, live)
    }

    pub fn record_offline_stop_response(
        &self,
        binding: &AttemptBinding,
        response: &OfflineStopReadback<OfflineStopResponse>,
    ) -> Result<()> {
        let mut owner = self.owner()?;
        check_owner(&owner, binding)?;
        owner.store.record_offline_stop_response(binding, response)
    }

    /// Campaign chooses first closure or post-closure adjudication from
    /// checked durable history. A later affirmative contradiction of the
    /// same scoped cut returns `Err(Contested)`, never new authority.
    pub fn record_offline_scoped_terminal(
        &self,
        binding: &AttemptBinding,
        subject: &OfflineStopSubject,
        evidence: &OfflineStopTerminalReadbacks,
    ) -> std::result::Result<(), TerminalFailure> {
        let mut owner = self
            .owner()
            .map_err(|_| TerminalFailure::FactNotConfirmed)?;
        check_owner(&owner, binding).map_err(|_| TerminalFailure::FactNotConfirmed)?;
        owner
            .store
            .record_offline_scoped_terminal(binding, subject, evidence)
    }

    /// Compare the same archived inputs against the current exact B readback.
    /// The later qualification join still needs historical Held on this key.
    pub fn confirm_offline_scoped_terminal(
        &self,
        binding: &AttemptBinding,
        subject: &OfflineStopSubject,
        evidence: &OfflineStopTerminalReadbacks,
    ) -> std::result::Result<(), TerminalFailure> {
        let mut owner = self
            .owner()
            .map_err(|_| TerminalFailure::FactNotConfirmed)?;
        check_owner(&owner, binding).map_err(|_| TerminalFailure::FactNotConfirmed)?;
        owner
            .store
            .confirm_offline_scoped_terminal(binding, subject, evidence)
    }

    /// Historical owner readback for the Host fixture join. A single checked
    /// B snapshot must contain both the capture-bound Held and exact archive.
    /// Nothing is written; the return is evidence only, not authority.
    pub fn confirm_offline_init_only_join(
        &self,
        binding: &AttemptBinding,
        subject: &OfflineStopSubject,
        evidence: &OfflineStopTerminalReadbacks,
        held: &super::FixtureHeldReadbacks,
        capture_sha256: &str,
    ) -> std::result::Result<(), TerminalFailure> {
        let mut owner = self
            .owner()
            .map_err(|_| TerminalFailure::FactNotConfirmed)?;
        check_owner(&owner, binding).map_err(|_| TerminalFailure::FactNotConfirmed)?;
        owner
            .store
            .confirm_offline_init_only_join(binding, subject, evidence, held, capture_sha256)
    }
}

#[cfg(test)]
mod tests;
