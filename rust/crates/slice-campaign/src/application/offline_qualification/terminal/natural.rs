//! Natural exit uses the original Start as its frontier. No Stop request,
//! Close mark, recovery permit, or effect is represented by these readbacks.

use serde::Serialize;

use super::{
    AttemptBinding, FixtureHeldReadbacks, OfflineAcceptedDisposition, OfflineCgroupHandle,
    OfflineCleanupTail, OfflineContainment, OfflineNewIngress, OfflineOtherWork,
    OfflineRecoveryAuthority, OfflineTerminalExecution, QualificationPort, TerminalFailure,
    check_owner,
};

/// The readbacks below are from one protected, ordered archive. The Host
/// acquisition owner, not this data-only checker, authenticates the sources.
#[derive(Clone, Debug, Serialize)]
pub struct OfflineNaturalTerminal {
    pub execution: OfflineTerminalExecution,
    pub retirement_arm: u64,
    pub start_arm: u64,
    pub wait_arm: u64,
    pub held_position: u64,
    pub wait_reply: u64,
    pub retirement_read: u64,
    pub closure_read: u64,
    pub exit: OfflineNaturalExit,
    pub retirement: OfflineNaturalRetirement,
    pub closure: OfflineNaturalClosure,
}

#[derive(Clone, Debug, Serialize)]
pub struct OfflineNaturalExit {
    pub lifetime: String,
    pub status: u32,
    pub runtime_wire_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct OfflineNaturalRetirement {
    pub cgroup: OfflineCgroupHandle,
    pub original_pidfd_exited: bool,
    pub retirement_archive_sha256: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct OfflineNaturalClosure {
    pub ingress: OfflineNewIngress,
    pub create: OfflineAcceptedDisposition,
    pub start: OfflineAcceptedDisposition,
    pub other_work: OfflineOtherWork,
    pub cleanup_tail: OfflineCleanupTail,
    pub recovery: OfflineRecoveryAuthority,
    pub containment: OfflineContainment,
    /// An affirmative, protected account from before Start through the final
    /// retired-subtree sample; absence of an abort record alone is not enough.
    pub no_intervention: bool,
    pub evidence_sha256: String,
}

impl QualificationPort {
    /// Check the same durable Create/Start/Held and record the no-Close end
    /// in the existing attempt store. This issues no effect capability.
    pub fn record_offline_natural_terminal(
        &self,
        binding: &AttemptBinding,
        reads: &OfflineNaturalTerminal,
        held: &FixtureHeldReadbacks,
        capture_sha256: &str,
    ) -> std::result::Result<(), TerminalFailure> {
        let unconfirmed = |_| TerminalFailure::FactNotConfirmed;
        let mut owner = self.owner().map_err(unconfirmed)?;
        check_owner(&owner, binding).map_err(unconfirmed)?;
        owner
            .store
            .record_offline_natural_terminal(binding, reads, held, capture_sha256)
    }
}
