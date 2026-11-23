use super::*;
use TerminalFailure::*;
use slice_campaign::offline_qualification::{
    FixtureAcceptedWorkReadback, FixtureAction, FixtureCleanupTail, FixtureEndReadback,
    FixtureExecutionPath, FixtureLifeEnd, FixtureMembership, FixtureRecoveryBacking,
    FixtureRecoveryReadback, FixtureTerminalReadbacks, FixtureTerminalSubject, TerminalFailure,
};

struct SyntheticInit {
    original: FixtureTerminalSubject,
    held: FixtureHeldReadbacks,
}

impl SyntheticInit {
    // The finite simulated init and its only child end naturally. This is not
    // a cleanup operation or an observation of any real process or runtime.
    fn end(&self) -> FixtureTerminalReadbacks {
        let anchor = &self.original;
        let excluded = FixtureExecutionPath::Excluded;
        FixtureTerminalReadbacks {
            end: Some(FixtureEndReadback {
                subject: anchor.clone(),
                init: FixtureLifeEnd::Exited,
                descendant: FixtureLifeEnd::Exited,
                membership: FixtureMembership::ContinuousInitAndChild,
            }),
            accepted: Some(FixtureAcceptedWorkReadback {
                subject: anchor.clone(),
                create: FixtureAction::Completed,
                start: FixtureAction::Completed,
                new_entry: excluded,
                task_delivery: excluded,
                release: excluded,
                exec: excluded,
                competing_start: excluded,
                restore: excluded,
                auto_start: excluded,
                cleanup_tail: FixtureCleanupTail::PendingNonExecuting,
            }),
            recovery: Some(FixtureRecoveryReadback {
                subject: anchor.clone(),
                backing: FixtureRecoveryBacking::RetainedInert,
                controller_reconcile: excluded,
                runtime_reconcile: excluded,
            }),
        }
    }

    fn record(&self, r: &Root, sources: &FixtureTerminalReadbacks) -> Result<(), TerminalFailure> {
        r.app.offline_qualification_port().record_fixture_terminal(
            &r.request.binding,
            &self.original,
            sources,
        )
    }

    fn qualify(&self, r: &Root, sources: &FixtureTerminalReadbacks) -> Result<(), TerminalFailure> {
        r.app.offline_qualification_port().qualify_fixture_terminal(
            &r.request.binding,
            &self.original,
            sources,
            &self.held,
        )
    }
}

fn drive(r: &Root, held_confirmed: bool) -> SyntheticInit {
    let collector = r.collector();
    let mut seen = vec![];
    let mut archived = None;
    let result = collector.collect_joined(&r.request, CHECK, |op| {
        seen.push(op.kind);
        assert!(r.spent_on_disk());
        let mark: String = r
            .conn()
            .query_row(
                "SELECT mark_json FROM b_lifecycle_mark WHERE attempt_id=?1 AND sequence=?2",
                rusqlite::params![op.binding.attempt_id(), op.sequence],
                |row| row.get(0),
            )
            .unwrap();
        let mark: serde_json::Value = serde_json::from_str(&mark).unwrap();
        assert_eq!(mark["request"], op.request_digest);
        if op.kind == OperationKind::Create {
            return create_reply(op, |_| {});
        }
        assert_eq!(
            mark["predecessor"].as_str(),
            op.predecessor_mark_digest.as_deref()
        );
        assert_eq!(
            mark["target"]["Start"]["full_id"].as_str(),
            op.target_id.as_deref()
        );
        archived = Some(native_surrogates(op));
        running_reply(op, |_, held| {
            if !held_confirmed {
                held.ready = None;
            }
        })
    });
    if held_confirmed {
        assert_eq!(result, Collected::Host(Outcome::InertInitHeld));
    } else {
        assert_ne!(result, Collected::Host(Outcome::InertInitHeld));
        assert!(!facts(r).iter().any(|(slot, _)| *slot == 4));
    }
    assert_eq!(seen, [OperationKind::Create, OperationKind::Start]);
    assert_eq!(
        collector.collect_joined(&r.request, CHECK, unexpected),
        Collected::ClosedOrAlreadyAdmitted
    );
    let (running, held) = archived.unwrap();
    let original = r
        .app
        .offline_qualification_port()
        .fixture_terminal_subject(&r.request.binding, &running)
        .unwrap();
    SyntheticInit { original, held }
}

fn reopen(r: Root) -> Root {
    let Root {
        _temp,
        app,
        request,
    } = r;
    drop(app);
    let (config, _) = offline_config(_temp.path());
    let app = Campaign::open(config).unwrap();
    Root {
        _temp,
        app,
        request,
    }
}

fn no_effect_can_reopen(r: &Root) {
    assert!(r.spent_on_disk());
    assert_eq!(r.marks(), 2);
    assert_eq!(
        r.collector().collect_joined(&r.request, CHECK, unexpected),
        Collected::EntryNotConfirmed
    );
    assert!(
        r.app
            .offline_qualification_port()
            .enter(&r.request.binding)
            .is_err()
    );
}

fn no_terminal(r: &Root) {
    assert!(!facts(r).iter().any(|(slot, _)| *slot == 5));
    no_effect_can_reopen(r);
}

#[test]
fn exact_held_then_terminal_survives_revoke_and_reopen_without_cleanup_ack_or_send() {
    // Cover both a write before reopen and a first observation after reopen.
    for observe_after_reopen in [false, true] {
        let mut r = root_with_mode(true);
        let task = drive(&r, true);
        let ended = task.end();
        assert_eq!(task.qualify(&r, &ended), Err(FactNotConfirmed));
        no_terminal(&r);
        r.app
            .revoke_offline_qualification(&r.request.binding)
            .unwrap();
        if observe_after_reopen {
            r = reopen(r);
        }
        assert_eq!(task.record(&r, &ended), Ok(()));
        assert_eq!(task.qualify(&r, &ended), Ok(()));
        let recorded = facts(&r);
        assert_eq!(
            recorded.iter().map(|(slot, _)| *slot).collect::<Vec<_>>(),
            [1, 2, 4, 5]
        );
        let terminal = &recorded[3].1;
        assert_eq!(terminal["mark"], recorded[1].1["mark"]);
        assert_eq!(terminal["task"], recorded[2].1["task"]);
        let cuts = &terminal["observation"]["Terminal"];
        let exit = cuts["workload_and_subtree_exit"].as_str().unwrap();
        let fence = cuts["accepted_and_recovered_work_fenced"].as_str().unwrap();
        assert!(exit.len() == 64 && fence.len() == 64 && exit != fence);
        r = reopen(r);
        assert_eq!(task.qualify(&r, &ended), Ok(()));
        assert_eq!(facts(&r), recorded, "historical qualification is read-only");
        no_effect_can_reopen(&r);
    }
}

#[test]
fn crossed_source_scope_and_uncovered_membership_do_not_close_the_attempt() {
    let r = root_with_mode(true);
    let task = drive(&r, true);
    type Change = fn(&mut FixtureTerminalReadbacks);
    let cases: &[(Change, TerminalFailure)] = &[
        (
            |s| s.end.as_mut().unwrap().subject.start_mark = "e".repeat(64),
            CrossedSubject,
        ),
        (
            |s| s.end.as_mut().unwrap().subject.running.exec_generation = "e".repeat(64),
            CrossedSubject,
        ),
        (
            |s| s.end.as_mut().unwrap().subject.running.lifetime = "e".repeat(64),
            CrossedSubject,
        ),
        (
            |s| s.end.as_mut().unwrap().subject.subtree = "e".repeat(64),
            CrossedSubject,
        ),
        (
            |s| s.end.as_mut().unwrap().subject.boot = "e".repeat(64),
            CrossedSubject,
        ),
        (
            |s| s.end.as_mut().unwrap().subject.namespace = "e".repeat(64),
            CrossedSubject,
        ),
        (
            |s| s.accepted.as_mut().unwrap().subject.frontier = "e".repeat(64),
            CrossedSubject,
        ),
        (
            |s| {
                let w = s.recovery.as_mut().unwrap();
                w.subject.running.object.serving_generation = "e".repeat(64);
                w.backing = FixtureRecoveryBacking::Unknown;
            },
            CrossedSubject,
        ),
        (
            |s| s.end.as_mut().unwrap().membership = FixtureMembership::SnapshotOnly,
            EndUnestablished,
        ),
        (
            |s| s.end.as_mut().unwrap().membership = FixtureMembership::MigrationUnaccounted,
            EndUnestablished,
        ),
        (
            |s| s.end.as_mut().unwrap().descendant = FixtureLifeEnd::Unknown,
            EndUnestablished,
        ),
        (
            |s| s.end.as_mut().unwrap().descendant = FixtureLifeEnd::Running,
            ExecutablePath,
        ),
        (|s| s.end = None, MissingWitness),
    ];
    for (mutate, expected) in cases {
        let mut readbacks = task.end();
        mutate(&mut readbacks);
        assert_eq!(task.record(&r, &readbacks), Err(*expected));
        no_terminal(&r);
    }
    let ended = task.end();
    assert_eq!(task.record(&r, &ended), Ok(()));
    assert_eq!(task.qualify(&r, &ended), Ok(()));
}

#[test]
fn end_does_not_settle_accepted_work_future_ingress_or_recovery() {
    let r = root_with_mode(true);
    let task = drive(&r, true);
    type Change = fn(&mut FixtureTerminalReadbacks);
    let cases: &[(Change, TerminalFailure)] = &[
        // A stopped/removed/list/cancel or optional Delete ACK signal cannot
        // supply this missing protected accepted-work witness at all.
        (|s| s.accepted = None, MissingWitness),
        (
            |s| s.accepted.as_mut().unwrap().start = FixtureAction::Unknown,
            ActionsUnsettled,
        ),
        (
            |s| s.accepted.as_mut().unwrap().start = FixtureAction::PendingExecution,
            ExecutablePath,
        ),
        (
            |s| s.accepted.as_mut().unwrap().new_entry = FixtureExecutionPath::Unknown,
            ActionsUnsettled,
        ),
        (
            |s| s.accepted.as_mut().unwrap().exec = FixtureExecutionPath::CanExecute,
            ExecutablePath,
        ),
        (
            |s| s.accepted.as_mut().unwrap().cleanup_tail = FixtureCleanupTail::Unknown,
            ActionsUnsettled,
        ),
        (
            |s| s.accepted.as_mut().unwrap().cleanup_tail = FixtureCleanupTail::CanExecuteOrReopen,
            ExecutablePath,
        ),
        (|s| s.recovery = None, MissingWitness),
        (
            |s| s.recovery.as_mut().unwrap().backing = FixtureRecoveryBacking::Unknown,
            RecoveryUnsettled,
        ),
        (
            |s| s.recovery.as_mut().unwrap().backing = FixtureRecoveryBacking::Restorable,
            ExecutablePath,
        ),
        (
            |s| s.recovery.as_mut().unwrap().controller_reconcile = FixtureExecutionPath::Unknown,
            RecoveryUnsettled,
        ),
        (
            |s| s.recovery.as_mut().unwrap().runtime_reconcile = FixtureExecutionPath::CanExecute,
            ExecutablePath,
        ),
    ];
    for (mutate, expected) in cases {
        let mut readbacks = task.end();
        mutate(&mut readbacks);
        assert_eq!(task.record(&r, &readbacks), Err(*expected));
        no_terminal(&r);
    }
    let ended = task.end();
    assert_eq!(task.record(&r, &ended), Ok(()));
    assert_eq!(task.qualify(&r, &ended), Ok(()));
}

#[test]
fn changed_witness_is_not_exact_and_protected_contradiction_quarantines_prior_terminal() {
    let r = root_with_mode(true);
    let task = drive(&r, true);
    let ended = task.end();
    assert_eq!(task.record(&r, &ended), Ok(()));
    assert_eq!(
        task.record(&r, &ended),
        Ok(()),
        "exact duplicate is a readback, not another effect"
    );
    let mut alternate = ended.clone();
    alternate.recovery.as_mut().unwrap().backing = FixtureRecoveryBacking::Retired;
    assert_eq!(task.qualify(&r, &alternate), Err(FactNotConfirmed));
    let mut late = ended.clone();
    late.recovery.as_mut().unwrap().backing = FixtureRecoveryBacking::Restorable;
    assert_eq!(task.record(&r, &late), Err(Contested));
    let stored = facts(&r);
    assert!(stored[3].1["observation"]["Terminal"].is_object());
    assert_eq!(stored[4].1["observation"], "Failed");
    let r = reopen(r);
    assert_eq!(task.qualify(&r, &ended), Err(Contested));
    assert_eq!(facts(&r), stored);
    no_effect_can_reopen(&r);
}

#[test]
fn lost_or_altered_terminal_commit_readback_does_not_qualify() {
    for alter in [false, true] {
        let r = root_with_mode(true);
        let task = drive(&r, true);
        let trap = if alter {
            format!(
                "UPDATE b_lifecycle_fact SET fact_json=json_set(fact_json, '$.observation.Terminal.workload_and_subtree_exit', '{}') WHERE slot=5;",
                "e".repeat(64)
            )
        } else {
            "DELETE FROM b_lifecycle_fact WHERE slot=5;".into()
        };
        r.conn().execute_batch(&format!("CREATE TRIGGER fault_terminal_readback AFTER INSERT ON b_lifecycle_fact WHEN NEW.slot=5 BEGIN {trap} END;")).unwrap();
        let ended = task.end();
        assert_eq!(task.record(&r, &ended), Err(FactNotConfirmed));
        let r = reopen(r);
        assert_eq!(task.qualify(&r, &ended), Err(FactNotConfirmed));
        no_effect_can_reopen(&r);
    }
}

#[test]
fn terminal_without_held_and_start_ack_without_running_are_not_qualified() {
    let r = root_with_mode(true);
    let task = drive(&r, false);
    let ended = task.end();
    assert_eq!(task.record(&r, &ended), Ok(()));
    assert_eq!(task.qualify(&r, &ended), Err(HeldNotConfirmed));
    no_effect_can_reopen(&r);

    let ack = root_with_mode(true);
    let mut candidate = None;
    assert_ne!(
        ack.collector().collect_joined(&ack.request, CHECK, |op| {
            if op.kind == OperationKind::Create {
                return create_reply(op, |_| {});
            }
            candidate = Some(native_surrogates(op).0);
            observed(op)
        }),
        Collected::Host(Outcome::InertInitHeld)
    );
    assert!(
        ack.app
            .offline_qualification_port()
            .fixture_terminal_subject(&ack.request.binding, &candidate.unwrap())
            .is_err()
    );
    no_terminal(&ack);
}
