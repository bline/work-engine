//! B's explicitly synthetic terminal ingress. It owns both evidence cuts and
//! compares recognition of the exact attempt in the transaction and after commit.

use super::*;
use crate::application::offline_qualification::{
    FixtureAction, FixtureCleanupTail, FixtureExecutionPath, FixtureLifeEnd, FixtureMembership,
    FixtureRecoveryBacking, FixtureTerminalReadbacks, FixtureTerminalSubject, TerminalFailure,
};

use TerminalFailure::*;
type Verdict<T = ()> = std::result::Result<T, TerminalFailure>;

mod scoped;

fn exact_execution<'a>(
    history: &'a History,
    attempt: &AttemptBinding,
    running: &FixtureRunning,
) -> Result<(&'a Mark, &'a Mark)> {
    let create = history
        .mark(Kind::Create)
        .ok_or_else(|| conflict("terminal fixture has no Create anchor"))?;
    let start = history
        .mark(Kind::Start)
        .ok_or_else(|| conflict("terminal fixture has no Start anchor"))?;
    let binding = attempt.lifecycle();
    let expected_create = created(
        binding,
        create,
        &FixtureCreateReadback {
            request_digest: attempt.create_request_digest()?,
            object: running.object.clone(),
        },
    )?;
    let expected_start = started(binding, start, running)?;
    if history.contested()
        || history.facts[Slot::Operation(Kind::Create).index()].as_slice() != [expected_create.fact]
        || history.facts[Slot::Operation(Kind::Start).index()].as_slice() != [expected_start.fact]
    {
        return Err(conflict(
            "terminal fixture requires exact Create and running association",
        ));
    }

    Ok((create, start))
}

fn subject(
    history: &History,
    attempt: &AttemptBinding,
    running: &FixtureRunning,
) -> Result<FixtureTerminalSubject> {
    let (create, start) = exact_execution(history, attempt, running)?;
    if history.tip() != Some(start) {
        return Err(conflict(
            "terminal fixture requires Start as the exact frontier",
        ));
    }
    // The finite offline model has fixed derivations for its original runtime
    // instance and subtree. On an installed route these would have to come from
    // protected continuity evidence, never by hashing the expected task.
    let start_mark = start.digest()?;
    let boot = campaign_digest(&json!([
        "offline-runtime-boot-v1",
        attempt.binding_digest(),
        running.object.serving_generation
    ]))?;
    let namespace = campaign_digest(&json!([
        "offline-runtime-namespace-v1",
        boot,
        running.lifetime
    ]))?;
    let subtree = campaign_digest(&json!([
        "offline-runtime-subtree-v1",
        namespace,
        start_mark,
        running
    ]))?;
    Ok(FixtureTerminalSubject {
        selection: attempt.binding_digest().into(),
        create_mark: create.digest()?,
        start_mark: start_mark.clone(),
        frontier: start_mark,
        running: running.clone(),
        boot,
        namespace,
        subtree,
    })
}

fn bound_start<'a>(
    history: &'a History,
    attempt: &AttemptBinding,
    expected: &FixtureTerminalSubject,
) -> Verdict<&'a Mark> {
    if history.contested() {
        return Err(Contested);
    }
    if subject(history, attempt, &expected.running).map_err(|_| NoExactStart)? != *expected {
        return Err(CrossedSubject);
    }
    history.mark(Kind::Start).ok_or(NoExactStart)
}

// Fixed ingress role names (not witness-provided locators) and exact checked
// stand-in bytes, tied to the selected fixture ingress and closed subject.
fn proof(binding: &Binding, role: &str, evidence: impl Serialize) -> Verdict<Digest> {
    Digest::new(
        campaign_digest(&json!([binding.evidence_ingress, role, evidence]))
            .map_err(|_| FactNotConfirmed)?,
    )
    .map_err(|_| FactNotConfirmed)
}

enum Checked {
    Positive(AuthenticatedFact),
    Contradiction(AuthenticatedFact),
}

fn adjudicate(
    binding: &Binding,
    start: &Mark,
    expected: &FixtureTerminalSubject,
    sources: &FixtureTerminalReadbacks,
) -> Verdict<Checked> {
    use FixtureExecutionPath::{CanExecute, Excluded};
    let end = sources.end.as_ref();
    let accepted = sources.accepted.as_ref();
    let recovery = sources.recovery.as_ref();
    if end.is_some_and(|w| w.subject != *expected)
        || accepted.is_some_and(|w| w.subject != *expected)
        || recovery.is_some_and(|w| w.subject != *expected)
    {
        return Err(CrossedSubject);
    }
    let paths = |w: &crate::application::offline_qualification::FixtureAcceptedWorkReadback| {
        [
            w.new_entry,
            w.task_delivery,
            w.release,
            w.exec,
            w.competing_start,
            w.restore,
            w.auto_start,
        ]
    };
    let task_id = Some(task(&expected.running).map_err(|_| FactNotConfirmed)?);
    let survivor = end.filter(|w| {
        w.init == FixtureLifeEnd::Running
            || (w.membership == FixtureMembership::ContinuousInitAndChild
                && w.descendant == FixtureLifeEnd::Running)
    });
    let executable = accepted.filter(|w| {
        [w.create, w.start].contains(&FixtureAction::PendingExecution)
            || paths(w).contains(&CanExecute)
            || w.cleanup_tail == FixtureCleanupTail::CanExecuteOrReopen
    });
    let reopens = recovery.filter(|w| {
        w.backing == FixtureRecoveryBacking::Restorable
            || [w.controller_reconcile, w.runtime_reconcile].contains(&CanExecute)
    });
    if survivor.is_some() || executable.is_some() || reopens.is_some() {
        let source = proof(
            binding,
            "synthetic-terminal-execution-contradiction-v1",
            (survivor, executable, reopens),
        )?;
        return Ok(Checked::Contradiction(fact(
            binding,
            start,
            source,
            Slot::Terminal,
            task_id,
            Observation::Failed,
        )));
    }

    let (Some(end), Some(accepted), Some(recovery)) = (end, accepted, recovery) else {
        return Err(MissingWitness);
    };
    if end.membership != FixtureMembership::ContinuousInitAndChild
        || [end.init, end.descendant] != [FixtureLifeEnd::Exited; 2]
    {
        return Err(EndUnestablished);
    }
    if [accepted.create, accepted.start] != [FixtureAction::Completed; 2]
        || paths(accepted).iter().any(|p| *p != Excluded)
        || !matches!(
            accepted.cleanup_tail,
            FixtureCleanupTail::Absent | FixtureCleanupTail::PendingNonExecuting
        )
    {
        return Err(ActionsUnsettled);
    }
    if !matches!(
        recovery.backing,
        FixtureRecoveryBacking::Retired | FixtureRecoveryBacking::RetainedInert
    ) || [recovery.controller_reconcile, recovery.runtime_reconcile] != [Excluded; 2]
    {
        return Err(RecoveryUnsettled);
    }
    let workload_and_subtree_exit = proof(binding, "synthetic-kernel-subtree-end-v1", end)?;
    let accepted_cut = proof(
        binding,
        "synthetic-accepted-action-disposition-v1",
        accepted,
    )?;
    let recovery_cut = proof(
        binding,
        "synthetic-restart-reconciliation-fence-v1",
        recovery,
    )?;
    let accepted_and_recovered_work_fenced = proof(
        binding,
        "synthetic-accepted-work-and-durable-recovery-fence-v1",
        (&accepted_cut, &recovery_cut),
    )?;
    let source = proof(
        binding,
        "synthetic-terminal-cut-join-v1",
        (
            &workload_and_subtree_exit,
            &accepted_and_recovered_work_fenced,
        ),
    )?;
    Ok(Checked::Positive(fact(
        binding,
        start,
        source,
        Slot::Terminal,
        task_id,
        Observation::Terminal {
            workload_and_subtree_exit,
            accepted_and_recovered_work_fenced,
        },
    )))
}

fn exact_terminal(history: &History, expected: &AuthenticatedFact) -> Verdict {
    if history.contested() {
        return Err(Contested);
    }
    if history.facts[Slot::Terminal.index()].as_slice() != std::slice::from_ref(&expected.fact) {
        return Err(FactNotConfirmed);
    }
    Ok(())
}

impl Store {
    pub(crate) fn fixture_terminal_subject(
        &mut self,
        attempt: &AttemptBinding,
        running: &FixtureRunning,
    ) -> Result<FixtureTerminalSubject> {
        let history = self.read_b_history(attempt.lifecycle())?;
        subject(&history, attempt, running)
    }

    pub(crate) fn record_fixture_terminal(
        &mut self,
        attempt: &AttemptBinding,
        expected: &FixtureTerminalSubject,
        sources: &FixtureTerminalReadbacks,
    ) -> Verdict {
        let binding = attempt.lifecycle();
        let unconfirmed = |_| FactNotConfirmed;
        // The historical ingress has only the immutable spend fence, not the
        // live opening/currentness fence. No operation mark or send is offered.
        let tx = self
            .b_scoped_transaction(binding, true, false)
            .map_err(unconfirmed)?;
        let history = load(&tx, binding).map_err(unconfirmed)?;
        let start = bound_start(&history, attempt, expected)?;
        let (auth, contradiction) = match adjudicate(binding, start, expected, sources)? {
            Checked::Positive(auth) => (auth, false),
            Checked::Contradiction(auth) => (auth, true),
        };
        if contradiction
            && !matches!(
                history.evidence(Slot::Terminal),
                Evidence::Observed {
                    observation: Observation::Terminal { .. },
                    ..
                }
            )
        {
            // A refusal is not a terminal fact. A later independent closure
            // can still establish it. Only dispute an already positive record.
            return Err(ExecutablePath);
        }
        let recorded = Self::write_b_fact(&tx, binding, &auth).map_err(unconfirmed)?;
        tx.commit().map_err(|_| FactNotConfirmed)?;
        let history = self.read_b_history(binding).map_err(unconfirmed)?;
        bound_start(&history, attempt, expected)?;
        if recorded == Recorded::Contested || contradiction {
            // A successful quarantine is reported by bound_start's checked
            // readback above. A lost quarantine is not successful readback.
            return Err(FactNotConfirmed);
        }
        exact_terminal(&history, &auth)
    }

    pub(crate) fn qualify_fixture_terminal(
        &mut self,
        attempt: &AttemptBinding,
        expected: &FixtureTerminalSubject,
        sources: &FixtureTerminalReadbacks,
        held: &FixtureHeldReadbacks,
    ) -> Verdict {
        let binding = attempt.lifecycle();
        let history = self.read_b_history(binding).map_err(|_| FactNotConfirmed)?;
        let start = bound_start(&history, attempt, expected)?;
        let Checked::Positive(auth) = adjudicate(binding, start, expected, sources)? else {
            return Err(ExecutablePath);
        };
        exact_terminal(&history, &auth)?;
        let held = held_fact(attempt, start, &expected.running, held, None)
            .map_err(|_| HeldNotConfirmed)?;
        if history.facts[Slot::Hold.index()].as_slice() != [held.fact] {
            return Err(HeldNotConfirmed);
        }
        Ok(())
    }
}
