//! The natural-exit cut reuses the exact execution, held and fact owners.
//! A Close mark, legacy terminal projection, or mere runtime absence is not
//! a natural-exit witness.

use super::*;
use crate::application::offline_qualification::{
    OfflineAcceptedDisposition, OfflineCleanupTail, OfflineContainment, OfflineEntryFence,
    OfflineNaturalTerminal, OfflineOtherWork, OfflineRecoveryBacking, QualificationPurpose,
};

fn natural_fact(
    history: &History,
    attempt: &AttemptBinding,
    reads: &OfflineNaturalTerminal,
    held: &FixtureHeldReadbacks,
    capture_sha256: &str,
) -> Verdict<AuthenticatedFact> {
    if attempt.purpose() != QualificationPurpose::NaturalExit {
        return Err(CrossedSubject);
    }
    if history.contested() {
        return Err(Contested);
    }
    let binding = attempt.lifecycle();
    let live = &reads.execution.live;
    let (execution, start) =
        scoped::captured_execution(history, attempt, live).map_err(|_| NoExactStart)?;
    if execution != reads.execution || history.tip() != Some(start) {
        return Err(CrossedSubject);
    }
    let expected_held = held_fact(attempt, start, &live.running, held, Some(capture_sha256))
        .map_err(|_| HeldNotConfirmed)?;
    if history.facts[Slot::Hold.index()].as_slice() != [expected_held.fact] {
        return Err(HeldNotConfirmed);
    }
    let order = [
        reads.retirement_arm,
        reads.start_arm,
        live.capture_position,
        reads.wait_arm,
        reads.held_position,
        reads.wait_reply,
        reads.retirement_read,
        reads.closure_read,
    ];
    if order[0] == 0 || !order.windows(2).all(|step| step[0] < step[1]) {
        return Err(EndUnestablished);
    }
    let end = &reads.exit;
    let retired = &reads.retirement;
    let closure = &reads.closure;
    if end.lifetime != live.running.lifetime
        || end.status != 0
        || Digest::new(end.runtime_wire_sha256.clone()).is_err()
        || retired.cgroup != live.cgroup
        || !retired.original_pidfd_exited
        || Digest::new(retired.retirement_archive_sha256.clone()).is_err()
        || closure.containment != OfflineContainment::ContinuousThroughFinalRead
        || !closure.no_intervention
    {
        return Err(EndUnestablished);
    }
    if [
        closure.ingress.controller,
        closure.ingress.engine,
        closure.ingress.runtime,
    ] != [OfflineEntryFence::Enforced; 3]
        || closure.create != OfflineAcceptedDisposition::Completed
        || closure.start != OfflineAcceptedDisposition::Completed
        || closure.other_work != OfflineOtherWork::ExcludedByClosedAcceptanceCut
        || !matches!(
            closure.cleanup_tail,
            OfflineCleanupTail::Settled | OfflineCleanupTail::CannotExecuteOrReopen
        )
    {
        return Err(ActionsUnsettled);
    }
    if !matches!(
        closure.recovery.backing,
        OfflineRecoveryBacking::PositivelyRetired | OfflineRecoveryBacking::RetainedExecutionInert
    ) || [
        closure.recovery.controller_reconcile,
        closure.recovery.runtime_reconcile,
    ] != [OfflineEntryFence::Enforced; 2]
        || Digest::new(closure.evidence_sha256.clone()).is_err()
    {
        return Err(RecoveryUnsettled);
    }
    let cut = ScopedTerminalCut {
        final_position: NonZeroU64::new(reads.closure_read).ok_or(EndUnestablished)?,
        workload_and_subtree_exit: scoped::scoped_proof(binding, "natural-original-end-v1", reads)?,
        accepted_and_recovered_work_fenced: scoped::scoped_proof(
            binding,
            "natural-closed-frontier-v1",
            closure,
        )?,
    };
    let task = task(&live.running).map_err(|_| FactNotConfirmed)?;
    let source = cut
        .natural_source(&binding.evidence_ingress, start, &task)
        .map_err(|_| FactNotConfirmed)?;
    Ok(fact(
        binding,
        start,
        source,
        Slot::Terminal,
        Some(task),
        Observation::NaturalTerminal(cut),
    ))
}

impl Store {
    pub(crate) fn record_offline_natural_terminal(
        &mut self,
        attempt: &AttemptBinding,
        reads: &OfflineNaturalTerminal,
        held: &FixtureHeldReadbacks,
        capture_sha256: &str,
    ) -> Verdict {
        let binding = attempt.lifecycle();
        let unconfirmed = |_| FactNotConfirmed;
        let tx = self
            .b_scoped_transaction(binding, true, false)
            .map_err(unconfirmed)?;
        let history = load(&tx, binding).map_err(unconfirmed)?;
        let auth = natural_fact(&history, attempt, reads, held, capture_sha256)?;
        if Self::write_b_fact(&tx, binding, &auth).map_err(unconfirmed)? != Recorded::Stored {
            return Err(FactNotConfirmed);
        }
        tx.commit().map_err(|_| FactNotConfirmed)?;
        let history = self.read_b_history(binding).map_err(unconfirmed)?;
        let expected = natural_fact(&history, attempt, reads, held, capture_sha256)?;
        exact_terminal(&history, &expected)
    }
}
