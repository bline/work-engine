//! The scoped Stop cut uses the existing Campaign spend/mark/fact owner. It
//! projects offline source records, never installs or trusts a real producer.

use super::*;
use crate::application::offline_qualification::{
    OfflineAcceptedDisposition, OfflineCleanupTail, OfflineContainment, OfflineEntryFence,
    OfflineLiveSubtree, OfflineOldTask, OfflineOldTaskTransitions, OfflineOtherWork,
    OfflineRecoveryBacking, OfflineStopReadback, OfflineStopResponse, OfflineStopSubject,
    OfflineStopTerminalReadbacks, OfflineTerminalExecution,
};

// Governing candidate-source audit for the pinned STOPPED transition case. No
// caller-supplied pin or audit can select different semantics for this fixture.
const STOPPED_SEMANTICS_AUDIT: &str =
    "2dac87a1896df4aae95e69b6b4e1c4e4deb4c0c28a63a6bfbb523199631c8698";

// Read the complete cgroup-v2 keyed-value file, not a substring or an absent
// key. Parse all lines to avoid accepting contradictory/torn snapshots; allow
// other well-formed numeric kernel keys without assigning them meaning.
fn populated(raw: &[u8]) -> Option<bool> {
    if raw.len() > 4096 {
        return None;
    }
    let mut keys = std::collections::BTreeSet::new();
    let mut value = None;
    for line in std::str::from_utf8(raw)
        .ok()?
        .strip_suffix('\n')?
        .split('\n')
    {
        let (key, digits) = line.split_once(' ')?;
        if key.is_empty()
            || !key
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c == b'_' || c.is_ascii_digit())
            || !keys.insert(key)
            || digits.is_empty()
            || !digits.bytes().all(|c| c.is_ascii_digit())
        {
            return None;
        }
        let number: u64 = digits.parse().ok()?;
        match key {
            "populated" => {
                value = Some(match digits {
                    "0" => false,
                    "1" => true,
                    _ => return None,
                })
            }
            "frozen" if number > 1 => return None,
            _ => {}
        }
    }
    value
}

fn stop_execution<'a>(
    history: &'a History,
    attempt: &AttemptBinding,
    live: &OfflineLiveSubtree,
) -> Result<(OfflineTerminalExecution, &'a Mark)> {
    let (create, start) = exact_execution(history, attempt, &live.running)?;
    validate_id(&live.boot)?;
    validate_id(&live.runtime_namespace)?;
    Digest::new(live.cgroup.capture.clone())?;
    if live.capture_position == 0
        || live.cgroup.mount_id == 0
        || live.cgroup.inode == 0
        || populated(&live.cgroup_events) != Some(true)
    {
        return Err(conflict(
            "bound original live populated cgroup sample absent",
        ));
    }
    Ok((
        OfflineTerminalExecution {
            selection: attempt.binding_digest().into(),
            create_request: create.request.as_str().into(),
            create_mark: create.digest()?,
            start_mark: start.digest()?,
            live: live.clone(),
        },
        start,
    ))
}

// The scoped coordinate domain uses exact u64s (capture positions and inode
// identities). These fixed ASCII object keys retain their ordering, but the
// JS campaign number codec would alias adjacent values beyond 2^53.
fn scoped_digest(value: &serde_json::Value) -> Result<Digest> {
    let canonical = crate::codec::canonical_historical_artifact(value)?;
    Digest::new(crate::codec::raw_sha256(&canonical))
}

fn scoped_proof(binding: &Binding, role: &str, evidence: impl Serialize) -> Verdict<Digest> {
    scoped_digest(&json!([binding.evidence_ingress, role, evidence])).map_err(|_| FactNotConfirmed)
}

fn close_request(execution: &OfflineTerminalExecution) -> Result<Digest> {
    // One exact subject, one scoped Stop. No restart, retry, second Remove,
    // fallbacks or new authority. This is not an installed wire request.
    scoped_digest(&json!(["offline-one-exact-scoped-stop-v1", execution]))
}

fn stop_subject(execution: OfflineTerminalExecution, mark: &Mark) -> Result<OfflineStopSubject> {
    if mark.request != close_request(&execution)? {
        return Err(conflict(
            "scoped Stop request does not bind this live capture",
        ));
    }
    Ok(OfflineStopSubject {
        execution,
        close_request: mark.request.as_str().into(),
        close_mark: mark.digest()?,
    })
}

fn historical_stop<'a>(
    history: &'a History,
    attempt: &AttemptBinding,
    live: &OfflineLiveSubtree,
) -> Result<(OfflineStopSubject, &'a Mark)> {
    let (execution, _) = stop_execution(history, attempt, live)?;
    // History checks Close predecessor, order and the exact target task.
    let mark = history
        .tip()
        .filter(|m| m.kind == Kind::Close)
        .ok_or_else(|| conflict("exact scoped Stop frontier absent"))?;
    Ok((stop_subject(execution, mark)?, mark))
}

fn bound_stop<'a>(
    history: &'a History,
    attempt: &AttemptBinding,
    subject: &OfflineStopSubject,
) -> Verdict<&'a Mark> {
    if history.contested() {
        return Err(Contested);
    }
    exact_execution(history, attempt, &subject.execution.live.running).map_err(|_| NoExactStart)?;
    let (observed, mark) =
        historical_stop(history, attempt, &subject.execution.live).map_err(|_| CrossedSubject)?;
    if observed != *subject {
        return Err(CrossedSubject);
    }
    Ok(mark)
}

fn scoped<T>(
    readback: &Option<OfflineStopReadback<T>>,
    subject: &OfflineStopSubject,
) -> Verdict<()> {
    if readback.as_ref().is_some_and(|w| {
        w.subject != *subject || w.position <= subject.execution.live.capture_position
    }) {
        return Err(CrossedSubject);
    }
    Ok(())
}

fn scoped_readbacks(reads: &OfflineStopTerminalReadbacks, subject: &OfflineStopSubject) -> Verdict {
    scoped(&reads.old_task, subject)?;
    scoped(&reads.ingress, subject)?;
    scoped(&reads.accepted, subject)?;
    scoped(&reads.recovery, subject)?;
    scoped(&reads.end, subject)
}

fn terminal_fact(
    binding: &Binding,
    mark: &Mark,
    subject: &OfflineStopSubject,
    reads: &OfflineStopTerminalReadbacks,
) -> Verdict<AuthenticatedFact> {
    scoped_readbacks(reads, subject)?;
    let (Some(old_task), Some(ingress), Some(accepted), Some(recovery), Some(end)) = (
        &reads.old_task,
        &reads.ingress,
        &reads.accepted,
        &reads.recovery,
        &reads.end,
    ) else {
        return Err(MissingWitness);
    };
    let runtime_pin = match old_task.observed {
        OfflineOldTask::Stopped(OfflineOldTaskTransitions::PinnedContainerd233RuncStopped) => {
            Some(STOPPED_SEMANTICS_AUDIT)
        }
        OfflineOldTask::PositivelyRetired => None,
        _ => return Err(ActionsUnsettled),
    };
    // The acceptance account must be after admission closes and the old task
    // cannot transition. The independent final sample follows BOTH cuts.
    if [
        ingress.observed.controller,
        ingress.observed.engine,
        ingress.observed.runtime,
    ] != [OfflineEntryFence::Enforced; 3]
        || accepted.position <= old_task.position.max(ingress.position)
        || accepted.observed.create != OfflineAcceptedDisposition::Completed
        || accepted.observed.start != OfflineAcceptedDisposition::Completed
        || !matches!(
            accepted.observed.stop,
            OfflineAcceptedDisposition::Completed | OfflineAcceptedDisposition::EffectivelyRevoked
        )
        || accepted.observed.other_work != OfflineOtherWork::ExcludedByClosedAcceptanceCut
        || !matches!(
            accepted.observed.cleanup_tail,
            OfflineCleanupTail::Settled | OfflineCleanupTail::CannotExecuteOrReopen
        )
    {
        return Err(ActionsUnsettled);
    }
    if recovery.position <= ingress.position
        || !matches!(
            recovery.observed.backing,
            OfflineRecoveryBacking::PositivelyRetired
                | OfflineRecoveryBacking::RetainedExecutionInert
        )
        || [
            recovery.observed.controller_reconcile,
            recovery.observed.runtime_reconcile,
        ] != [OfflineEntryFence::Enforced; 2]
    {
        return Err(RecoveryUnsettled);
    }
    if end.position <= accepted.position.max(recovery.position) {
        return Err(EndUnestablished);
    }
    let live = &subject.execution.live;
    let observation = &end.observed;
    if observation.exited_init_lifetime.as_ref() != Some(&live.running.lifetime)
        || observation.cgroup != live.cgroup
        || observation.containment != OfflineContainment::ContinuousThroughFinalRead
        || populated(&observation.cgroup_events) != Some(false)
    {
        return Err(EndUnestablished);
    }
    let cut = ScopedTerminalCut {
        final_position: NonZeroU64::new(end.position).ok_or(EndUnestablished)?,
        workload_and_subtree_exit: scoped_proof(
            binding,
            "offline-scoped-subtree-final-end-v1",
            end,
        )?,
        accepted_and_recovered_work_fenced: scoped_proof(
            binding,
            "offline-scoped-accepted-work-and-recovery-cut-v1",
            (runtime_pin, old_task, ingress, accepted, recovery),
        )?,
    };
    let task = task(&live.running).map_err(|_| FactNotConfirmed)?;
    let source = cut
        .source(&binding.evidence_ingress, mark, &task)
        .map_err(|_| FactNotConfirmed)?;
    Ok(fact(
        binding,
        mark,
        source,
        Slot::Terminal,
        Some(task),
        Observation::ScopedTerminal(cut),
    ))
}

fn contradiction_source(
    binding: &Binding,
    subject: &OfflineStopSubject,
    reads: &OfflineStopTerminalReadbacks,
    archived_source: &Digest,
    final_position: u64,
) -> Verdict<Option<Digest>> {
    scoped_readbacks(reads, subject)?;
    let ingress = reads.ingress.as_ref().filter(|w| {
        w.position > final_position
            && [w.observed.controller, w.observed.engine, w.observed.runtime]
                .contains(&OfflineEntryFence::Open)
    });
    let accepted = reads.accepted.as_ref().filter(|w| {
        w.position > final_position
            && (w.observed.other_work == OfflineOtherWork::AdditionalAcceptedWork
                || w.observed.cleanup_tail == OfflineCleanupTail::CanExecuteOrReopen)
    });
    let recovery = reads.recovery.as_ref().filter(|w| {
        w.position > final_position
            && (w.observed.backing == OfflineRecoveryBacking::Restorable
                || [
                    w.observed.controller_reconcile,
                    w.observed.runtime_reconcile,
                ]
                .contains(&OfflineEntryFence::Open))
    });
    // A repeat read need not repeat the exited-init witness. It must still
    // observe the same continuously retained subtree, never a replacement.
    let live = &subject.execution.live;
    if reads.end.as_ref().is_some_and(|w| {
        w.observed.cgroup != live.cgroup
            || w.observed
                .exited_init_lifetime
                .as_ref()
                .is_some_and(|lifetime| lifetime != &live.running.lifetime)
    }) {
        return Err(CrossedSubject);
    }
    let populated_subtree = reads.end.as_ref().filter(|w| {
        w.position > final_position
            && w.observed.containment == OfflineContainment::ContinuousThroughFinalRead
            && populated(&w.observed.cgroup_events) == Some(true)
    });
    // Pending/unknown alone is not proof of executable work. Never mistake
    // arrival after settlement for an observation after its final position.
    if ingress.is_none() && accepted.is_none() && recovery.is_none() && populated_subtree.is_none()
    {
        return Ok(None);
    }
    scoped_proof(
        binding,
        "offline-scoped-terminal-contradiction-v1",
        (
            archived_source,
            final_position,
            ingress,
            accepted,
            recovery,
            populated_subtree,
        ),
    )
    .map(Some)
}

fn exact_scoped_terminal(
    history: &History,
    attempt: &AttemptBinding,
    subject: &OfflineStopSubject,
    reads: &OfflineStopTerminalReadbacks,
) -> Verdict<AuthenticatedFact> {
    let mark = bound_stop(history, attempt, subject)?;
    terminal_fact(attempt.lifecycle(), mark, subject, reads)
}

impl Store {
    pub(in crate::store) fn write_offline_stop_mark(
        tx: &Transaction<'_>,
        attempt: &AttemptBinding,
        live: &OfflineLiveSubtree,
    ) -> Result<OfflineStopSubject> {
        let binding = attempt.lifecycle();
        let history = load(tx, binding)?;
        let (execution, start) = stop_execution(&history, attempt, live)?;
        let mark = Self::write_b_mark(
            tx,
            binding,
            Some(start),
            Kind::Close,
            close_request(&execution)?,
            Some(Target::Close(task(&live.running)?)),
        )?;
        stop_subject(execution, &mark)
    }

    pub(crate) fn read_offline_scoped_stop(
        &mut self,
        attempt: &AttemptBinding,
        live: &OfflineLiveSubtree,
    ) -> Result<OfflineStopSubject> {
        let history = self.read_b_history(attempt.lifecycle())?;
        historical_stop(&history, attempt, live).map(|(subject, _)| subject)
    }

    pub(crate) fn record_offline_stop_response(
        &mut self,
        attempt: &AttemptBinding,
        readback: &OfflineStopReadback<OfflineStopResponse>,
    ) -> Result<()> {
        let binding = attempt.lifecycle();
        let tx = self.b_scoped_transaction(binding, true, false)?;
        let history = load(&tx, binding)?;
        let scope_error = |_| conflict("scoped Stop response does not bind exact history");
        let mark = bound_stop(&history, attempt, &readback.subject).map_err(scope_error)?;
        if readback.position <= readback.subject.execution.live.capture_position {
            return Err(conflict("scoped Stop response predates the live capture"));
        }
        let disposition = match readback.observed {
            OfflineStopResponse::ReturnedSuccess => Observation::Closed,
            OfflineStopResponse::ReturnedFailure => Observation::Failed,
            OfflineStopResponse::Lost => Observation::Lost,
            OfflineStopResponse::Unknown => Observation::Unknown,
        };
        let auth = fact(
            binding,
            mark,
            proof(binding, "offline-scoped-stop-call-response-v1", readback)
                .map_err(scope_error)?,
            Slot::Operation(Kind::Close),
            Some(task(&readback.subject.execution.live.running)?),
            disposition,
        );
        Self::write_b_fact(&tx, binding, &auth)?;
        tx.commit().map_err(sql)?;
        // Check this is still the same committed frontier and response, also
        // on duplicate or contested writes. Never overwrite a lost diagnosis.
        let history = self.read_b_history(binding)?;
        bound_stop(&history, attempt, &readback.subject).map_err(scope_error)?;
        if history.facts[Slot::Operation(Kind::Close).index()].as_slice()
            != std::slice::from_ref(&auth.fact)
        {
            return Err(integrity("scoped Stop response not confirmed"));
        }
        Ok(())
    }

    pub(crate) fn record_offline_scoped_terminal(
        &mut self,
        attempt: &AttemptBinding,
        subject: &OfflineStopSubject,
        reads: &OfflineStopTerminalReadbacks,
    ) -> Verdict {
        let binding = attempt.lifecycle();
        let unconfirmed = |_| FactNotConfirmed;
        let tx = self
            .b_scoped_transaction(binding, true, false)
            .map_err(unconfirmed)?;
        let history = load(&tx, binding).map_err(unconfirmed)?;
        let mark = bound_stop(&history, attempt, subject)?;
        // The same checked transaction selects the path and writes the fact.
        // Only the scoped positive shape carries an authenticated cut; an
        // old/unprojected or legacy row must not enable positive-only intake.
        if let [prior] = history.facts[Slot::Terminal.index()].as_slice() {
            let Observation::ScopedTerminal(cut) = &prior.observation else {
                return Err(FactNotConfirmed);
            };
            if prior.mark != *mark
                || cut.final_position.get() <= subject.execution.live.capture_position
            {
                return Err(FactNotConfirmed);
            }
            if let Some(source) = contradiction_source(
                binding,
                subject,
                reads,
                &prior.source,
                cut.final_position.get(),
            )? {
                let contradiction = fact(
                    binding,
                    mark,
                    source,
                    Slot::Terminal,
                    prior.task.clone(),
                    Observation::Failed,
                );
                if Self::write_b_fact(&tx, binding, &contradiction).map_err(unconfirmed)?
                    != Recorded::Contested
                {
                    return Err(FactNotConfirmed);
                }
                tx.commit().map_err(|_| FactNotConfirmed)?;
                let committed = self.read_b_history(binding).map_err(unconfirmed)?;
                return if committed.facts[Slot::Terminal.index()].as_slice()
                    == [prior.clone(), contradiction.fact]
                {
                    Err(Contested)
                } else {
                    Err(FactNotConfirmed)
                };
            }
        }
        let auth = terminal_fact(binding, mark, subject, reads)?;
        Self::write_b_fact(&tx, binding, &auth).map_err(unconfirmed)?;
        tx.commit().map_err(|_| FactNotConfirmed)?;
        self.confirm_offline_scoped_terminal(attempt, subject, reads)
    }

    pub(crate) fn confirm_offline_scoped_terminal(
        &mut self,
        attempt: &AttemptBinding,
        subject: &OfflineStopSubject,
        reads: &OfflineStopTerminalReadbacks,
    ) -> Verdict {
        let history = self
            .read_b_history(attempt.lifecycle())
            .map_err(|_| FactNotConfirmed)?;
        let expected = exact_scoped_terminal(&history, attempt, subject, reads)?;
        exact_terminal(&history, &expected)
    }

    pub(crate) fn confirm_offline_init_only_join(
        &mut self,
        attempt: &AttemptBinding,
        subject: &OfflineStopSubject,
        reads: &OfflineStopTerminalReadbacks,
        held: &FixtureHeldReadbacks,
        capture_sha256: &str,
    ) -> Verdict {
        let history = self
            .read_b_history(attempt.lifecycle())
            .map_err(|_| FactNotConfirmed)?;
        let terminal = exact_scoped_terminal(&history, attempt, subject, reads)?;
        exact_terminal(&history, &terminal)?;
        // The scoped Terminal check already binds the exact Close tip to
        // this Start and live capture. Held is historical, but its original
        // write and readback still required the Start tip, before this Close.
        let start = history.mark(Kind::Start).ok_or(NoExactStart)?;
        let expected = held_fact(
            attempt,
            start,
            &subject.execution.live.running,
            held,
            Some(capture_sha256),
        )
        .map_err(|_| HeldNotConfirmed)?;
        if history.facts[Slot::Hold.index()].as_slice() != [expected.fact] {
            return Err(HeldNotConfirmed);
        }
        Ok(())
    }
}
