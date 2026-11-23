//! Synthetic sources for the isolated joined fixture. The three readback types
//! substitute for protected kernel membership, accepted-work, and recovery
//! custody respectively. None is an installed observation or an effect permit.

use serde::Serialize;

use super::{AttemptBinding, FixtureHeldReadbacks, FixtureRunning, QualificationPort, check_owner};
use crate::Result;

mod scoped;
pub use scoped::*;

/// Read-only data fixed before joining end, fencing, and recovery witnesses.
/// This fixture has two marks (Start is the closed frontier). These are stable
/// surrogate identities for the fixture, not locators supplied by a witness.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FixtureTerminalSubject {
    pub selection: String,
    pub create_mark: String,
    pub start_mark: String,
    pub frontier: String,
    pub running: FixtureRunning,
    pub boot: String,
    pub namespace: String,
    pub subtree: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FixtureEndReadback {
    pub subject: FixtureTerminalSubject,
    pub init: FixtureLifeEnd,
    pub descendant: FixtureLifeEnd,
    pub membership: FixtureMembership,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FixtureLifeEnd {
    Exited,
    Running,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FixtureMembership {
    /// The entire synthetic cohort is exactly init and one child, continuously
    /// tracked in the original subtree through end, not an empty lookup.
    ContinuousInitAndChild,
    SnapshotOnly,
    MigrationUnaccounted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FixtureAction {
    Completed,
    PendingExecution,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FixtureExecutionPath {
    Excluded,
    CanExecute,
    Unknown,
}

/// Fixed ingress/action inventory of the simulated init-only route. An
/// exclusion is a synthetic source observation, not inferred from absent rows
/// or the client's missing reply. Create and Start are the only accepted marks.
#[derive(Clone, Debug, Serialize)]
pub struct FixtureAcceptedWorkReadback {
    pub subject: FixtureTerminalSubject,
    pub create: FixtureAction,
    pub start: FixtureAction,
    pub new_entry: FixtureExecutionPath,
    pub task_delivery: FixtureExecutionPath,
    pub release: FixtureExecutionPath,
    pub exec: FixtureExecutionPath,
    pub competing_start: FixtureExecutionPath,
    pub restore: FixtureExecutionPath,
    pub auto_start: FixtureExecutionPath,
    pub cleanup_tail: FixtureCleanupTail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FixtureCleanupTail {
    Absent,
    /// Checked for this exact spec/runtime: neither remaining helper work nor
    /// hooks can execute the attempt, or reopen its recovery path.
    PendingNonExecuting,
    CanExecuteOrReopen,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct FixtureRecoveryReadback {
    pub subject: FixtureTerminalSubject,
    pub backing: FixtureRecoveryBacking,
    pub controller_reconcile: FixtureExecutionPath,
    pub runtime_reconcile: FixtureExecutionPath,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FixtureRecoveryBacking {
    Retired,
    RetainedInert,
    Restorable,
    Unknown,
}

#[derive(Clone, Debug, Default)]
pub struct FixtureTerminalReadbacks {
    pub end: Option<FixtureEndReadback>,
    pub accepted: Option<FixtureAcceptedWorkReadback>,
    pub recovery: Option<FixtureRecoveryReadback>,
}

/// Each refusal means terminal was not established at this boundary. Only
/// ExecutablePath positively reports a survivor or an executable continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalFailure {
    NoExactStart,
    CrossedSubject,
    MissingWitness,
    EndUnestablished,
    ActionsUnsettled,
    RecoveryUnsettled,
    ExecutablePath,
    Contested,
    FactNotConfirmed,
    HeldNotConfirmed,
}

type Verdict = std::result::Result<(), TerminalFailure>;

impl QualificationPort {
    pub fn fixture_terminal_subject(
        &self,
        binding: &AttemptBinding,
        running: &FixtureRunning,
    ) -> Result<FixtureTerminalSubject> {
        let mut owner = self.owner()?;
        check_owner(&owner, binding)?;
        owner.store.fixture_terminal_subject(binding, running)
    }

    /// Observe historical evidence against the exact spent attempt, independent
    /// of A's live eligibility. Success does not assert Held or grant an effect.
    pub fn record_fixture_terminal(
        &self,
        binding: &AttemptBinding,
        subject: &FixtureTerminalSubject,
        sources: &FixtureTerminalReadbacks,
    ) -> Verdict {
        let mut owner = self
            .owner()
            .map_err(|_| TerminalFailure::FactNotConfirmed)?;
        check_owner(&owner, binding).map_err(|_| TerminalFailure::FactNotConfirmed)?;
        owner
            .store
            .record_fixture_terminal(binding, subject, sources)
    }

    /// Compare the archived synthetic witnesses against the checked current B
    /// readback. No new observation is recorded, and no send can be recovered.
    pub fn qualify_fixture_terminal(
        &self,
        binding: &AttemptBinding,
        subject: &FixtureTerminalSubject,
        sources: &FixtureTerminalReadbacks,
        held: &FixtureHeldReadbacks,
    ) -> Verdict {
        let mut owner = self
            .owner()
            .map_err(|_| TerminalFailure::FactNotConfirmed)?;
        check_owner(&owner, binding).map_err(|_| TerminalFailure::FactNotConfirmed)?;
        owner
            .store
            .qualify_fixture_terminal(binding, subject, sources, held)
    }
}
