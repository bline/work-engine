use super::*;
use slice_campaign::offline_qualification::{
    OfflineAcceptedDisposition, OfflineCgroupHandle, OfflineCleanupTail, OfflineClosedFrontier,
    OfflineContainment, OfflineEntryFence, OfflineLiveSubtree, OfflineNewIngress, OfflineOldTask,
    OfflineOldTaskTransitions, OfflineOtherWork, OfflineRecoveryAuthority, OfflineRecoveryBacking,
    OfflineStopReadback, OfflineSubtreeEnd,
};

fn live(held: &HeldHistory) -> OfflineLiveSubtree {
    let key = held.execution();
    OfflineLiveSubtree {
        running: held.running.clone(),
        boot: "01".repeat(16),
        runtime_namespace: "moby".into(),
        cgroup: OfflineCgroupHandle {
            capture: key.subtree_capture.clone(),
            mount_id: key.subtree_mount_id,
            inode: key.subtree_inode,
        },
        // Ordinal of the independent Terminal capture, not a Ready timestamp.
        capture_position: 2,
        cgroup_events: b"populated 1\nfrozen 0\n".to_vec(),
    }
}

fn read<T>(subject: &OfflineStopSubject, position: u64, observed: T) -> OfflineStopReadback<T> {
    OfflineStopReadback {
        subject: subject.clone(),
        position,
        observed,
    }
}

fn archive(subject: &OfflineStopSubject) -> OfflineStopTerminalReadbacks {
    use OfflineEntryFence::Enforced;
    OfflineStopTerminalReadbacks {
        old_task: Some(read(
            subject,
            5,
            OfflineOldTask::Stopped(OfflineOldTaskTransitions::PinnedContainerd233RuncStopped),
        )),
        ingress: Some(read(
            subject,
            3,
            OfflineNewIngress {
                controller: Enforced,
                engine: Enforced,
                runtime: Enforced,
            },
        )),
        accepted: Some(read(
            subject,
            6,
            OfflineClosedFrontier {
                create: OfflineAcceptedDisposition::Completed,
                start: OfflineAcceptedDisposition::Completed,
                stop: OfflineAcceptedDisposition::EffectivelyRevoked,
                other_work: OfflineOtherWork::ExcludedByClosedAcceptanceCut,
                cleanup_tail: OfflineCleanupTail::CannotExecuteOrReopen,
            },
        )),
        recovery: Some(read(
            subject,
            7,
            OfflineRecoveryAuthority {
                backing: OfflineRecoveryBacking::RetainedExecutionInert,
                controller_reconcile: Enforced,
                runtime_reconcile: Enforced,
            },
        )),
        end: Some(read(
            subject,
            8,
            OfflineSubtreeEnd {
                exited_init_lifetime: Some(subject.execution.live.running.lifetime.clone()),
                cgroup: subject.execution.live.cgroup.clone(),
                containment: OfflineContainment::ContinuousThroughFinalRead,
                cgroup_events: b"populated 0\nfrozen 0\n".to_vec(),
            },
        )),
    }
}

#[test]
fn held_then_scoped_terminal_qualifies_from_same_owner_history_after_reopen() {
    let (_temp, config, app, b) = campaign(&recipe(), &serving());
    let port = app.offline_qualification_port();
    let (fixture, continuation) = running_capture(&port, &b);
    let held = verify_fixture(
        &port,
        &b,
        &fixture.owner.running,
        continuation,
        fixture.input(),
    )
    .unwrap();
    let live = live(&held);
    let stop = port
        .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
        .unwrap();
    let terminal = archive(&stop);
    assert_eq!(
        qualify_fixture(&port, &b, &held, &stop, &terminal),
        Err(JoinRefusal::Owner(TerminalFailure::FactNotConfirmed))
    );
    // No response diagnostic at any point; revoke does not restore authority
    // and does not discard this attempt's independently provable closure.
    app.revoke_offline_qualification(&b).unwrap();
    port.record_offline_scoped_terminal(&b, &stop, &terminal)
        .unwrap();
    assert_eq!(qualify_fixture(&port, &b, &held, &stop, &terminal), Ok(()));
    drop(port);
    drop(app);

    let reopened = Campaign::open(config()).unwrap();
    let port = reopened.offline_qualification_port();
    assert_eq!(port.read_offline_scoped_stop(&b, &live).unwrap(), stop);
    assert_eq!(qualify_fixture(&port, &b, &held, &stop, &terminal), Ok(()));
    assert!(port.enter(&b).is_err());
    assert!(port.fixture_terminal_subject(&b, &live.running).is_err());
    assert!(
        port.mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
            .is_err()
    );

    let mut other_cut = fixture.check().unwrap();
    other_cut.observed_at_ns -= 1;
    assert_eq!(
        qualify_fixture(&port, &b, &other_cut, &stop, &terminal),
        Err(JoinRefusal::Owner(TerminalFailure::HeldNotConfirmed))
    );
    let mut unarchived = terminal.clone();
    unarchived.end.as_mut().unwrap().position += 1;
    assert_eq!(
        qualify_fixture(&port, &b, &held, &stop, &unarchived),
        Err(JoinRefusal::Owner(TerminalFailure::FactNotConfirmed))
    );

    let mut incomplete = terminal.clone();
    incomplete.end = None;
    assert_eq!(
        qualify_fixture(&port, &b, &held, &stop, &incomplete),
        Err(JoinRefusal::Owner(TerminalFailure::MissingWitness))
    );
    let mut delayed = terminal.clone();
    delayed.accepted.as_mut().unwrap().observed.other_work =
        OfflineOtherWork::AdditionalAcceptedWork;
    assert_eq!(
        qualify_fixture(&port, &b, &held, &stop, &delayed),
        Err(JoinRefusal::Owner(TerminalFailure::ActionsUnsettled))
    );
    let mut recoverable = terminal.clone();
    recoverable.recovery.as_mut().unwrap().observed.backing = OfflineRecoveryBacking::Restorable;
    assert_eq!(
        qualify_fixture(&port, &b, &held, &stop, &recoverable),
        Err(JoinRefusal::Owner(TerminalFailure::RecoveryUnsettled))
    );

    type Mutation = (&'static str, fn(&mut OfflineStopSubject));
    let crossed: [Mutation; 14] = [
        ("selection", |s| s.execution.selection = hash('a')),
        ("Create request", |s| s.execution.create_request = hash('a')),
        ("Create mark", |s| s.execution.create_mark = hash('a')),
        ("Start request", |s| {
            s.execution.live.running.start_request_digest = hash('a')
        }),
        ("Start mark", |s| s.execution.start_mark = hash('a')),
        ("full object", |s| {
            s.execution.live.running.object.full_id = hash('a')
        }),
        ("serving", |s| {
            s.execution.live.running.object.serving_generation = hash('a')
        }),
        ("task", |s| s.execution.live.running.task_id = hash('a')),
        ("lifetime", |s| {
            s.execution.live.running.lifetime = hash('a')
        }),
        ("exec", |s| {
            s.execution.live.running.exec_generation = hash('a')
        }),
        ("boot", |s| s.execution.live.boot = "02".repeat(16)),
        ("namespace", |s| {
            s.execution.live.runtime_namespace = "other".into()
        }),
        ("mount", |s| s.execution.live.cgroup.mount_id += 1),
        ("inode", |s| s.execution.live.cgroup.inode += 1),
    ];
    for (field, change) in crossed {
        let mut altered = stop.clone();
        change(&mut altered);
        assert_eq!(
            qualify_fixture(&port, &b, &held, &altered, &terminal),
            Err(JoinRefusal::CrossedExecution),
            "{field}",
        );
    }
}

#[test]
fn later_recovery_or_population_refuses_actual_composition_through_reopen() {
    for (populated, complete) in [(false, false), (true, false), (false, true), (true, true)] {
        let (_temp, config, app, b) = campaign(&recipe(), &serving());
        let port = app.offline_qualification_port();
        let (fixture, continuation) = running_capture(&port, &b);
        let held = verify_fixture(
            &port,
            &b,
            &fixture.owner.running,
            continuation,
            fixture.input(),
        )
        .unwrap();
        let stop = port
            .mark_offline_scoped_stop(&b, &live(&held), JoinedMarkReadback::Check)
            .unwrap();
        let terminal = archive(&stop);
        app.revoke_offline_qualification(&b).unwrap();
        port.record_offline_scoped_terminal(&b, &stop, &terminal)
            .unwrap();
        assert_eq!(qualify_fixture(&port, &b, &held, &stop, &terminal), Ok(()));
        drop(port);
        drop(app);
        let app = Campaign::open(config()).unwrap();
        let port = app.offline_qualification_port();
        assert_eq!(qualify_fixture(&port, &b, &held, &stop, &terminal), Ok(()));
        let mut late = if complete {
            terminal.clone()
        } else {
            OfflineStopTerminalReadbacks::default()
        };
        if populated {
            let mut end = terminal.end.clone().unwrap();
            end.position += 1;
            if !complete {
                end.observed.exited_init_lifetime = None;
            }
            end.observed.cgroup_events = b"populated 1\nfrozen 0\n".to_vec();
            late.end = Some(end);
        } else {
            let mut recovery = terminal.recovery.clone().unwrap();
            recovery.position = terminal.end.as_ref().unwrap().position + 1;
            recovery.observed.backing = OfflineRecoveryBacking::Restorable;
            late.recovery = Some(recovery);
            if complete {
                late.end.as_mut().unwrap().position += 2;
            }
        }
        assert_eq!(
            port.record_offline_scoped_terminal(&b, &stop, &late),
            Err(TerminalFailure::Contested)
        );
        let refused = Err(JoinRefusal::Owner(TerminalFailure::Contested));
        assert_eq!(
            port.confirm_offline_scoped_terminal(&b, &stop, &terminal),
            Err(TerminalFailure::Contested)
        );
        assert_eq!(qualify_fixture(&port, &b, &held, &stop, &terminal), refused);
        drop(port);
        drop(app);
        let app = Campaign::open(config()).unwrap();
        let port = app.offline_qualification_port();
        assert_eq!(
            port.confirm_offline_scoped_terminal(&b, &stop, &terminal),
            Err(TerminalFailure::Contested)
        );
        assert_eq!(qualify_fixture(&port, &b, &held, &stop, &terminal), refused);
        assert!(port.enter(&b).is_err());
    }
}

#[test]
fn authentic_held_and_terminal_on_same_inode_do_not_join_across_slots() {
    let (_temp, _, app, b) = campaign(&recipe(), &serving());
    let port = app.offline_qualification_port();
    let (fixture, continuation) = running_capture(&port, &b);
    let held = verify_fixture(
        &port,
        &b,
        &fixture.owner.running,
        continuation,
        fixture.input(),
    )
    .unwrap();
    let mut live = live(&held);
    live.cgroup.capture = hash('5');
    let stop = port
        .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
        .unwrap();
    let terminal = archive(&stop);
    port.record_offline_scoped_terminal(&b, &stop, &terminal)
        .unwrap();
    assert_eq!(
        port.confirm_offline_scoped_terminal(&b, &stop, &terminal),
        Ok(())
    );
    assert_eq!(
        qualify_fixture(&port, &b, &held, &stop, &terminal),
        Err(JoinRefusal::CrossedExecution)
    );
}

#[test]
fn absent_held_and_stale_continuation_do_not_turn_terminal_into_qualification() {
    let (_temp, _, app, b) = campaign(&recipe(), &serving());
    let port = app.offline_qualification_port();
    let (fixture, continuation) = running_capture(&port, &b);
    // Internal capture only, not an owner-confirmed Held. This value cannot
    // escape the public Ready verifier on this path.
    let capture = fixture.check().unwrap();
    let live = live(&capture);
    let stop = port
        .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
        .unwrap();
    assert_eq!(
        continuation.verify_init_only_held(
            &port,
            &b,
            &live.running,
            &fixture_held_projection(&b, &capture).unwrap(),
            &capture.capture_sha256().unwrap(),
        ),
        Err(HoldFailure::FactNotConfirmed)
    );
    let terminal = archive(&stop);
    port.record_offline_scoped_terminal(&b, &stop, &terminal)
        .unwrap();
    assert_eq!(
        port.confirm_offline_scoped_terminal(&b, &stop, &terminal),
        Ok(())
    );
    assert_eq!(
        qualify_fixture(&port, &b, &capture, &stop, &terminal),
        Err(JoinRefusal::Owner(TerminalFailure::HeldNotConfirmed))
    );
    assert!(port.enter(&b).is_err());
}

#[test]
fn ready_rejects_continuation_and_qualification_rejects_history_from_other_owner() {
    let (_temp, _, a, ab) = campaign(&recipe(), &serving());
    let (_other_temp, _, b, bb) = campaign(&recipe(), &serving());
    let (ap, bp) = (
        a.offline_qualification_port(),
        b.offline_qualification_port(),
    );
    let (af, ac) = running_capture(&ap, &ab);
    let (bf, bc) = running_capture(&bp, &bb);
    assert_eq!(
        verify_fixture(&ap, &ab, &af.owner.running, bc, af.input()),
        Err(Refusal::Owner(HoldFailure::NotCurrent))
    );
    let a_held = verify_fixture(&ap, &ab, &af.owner.running, ac, af.input()).unwrap();
    let b_capture = bf.check().unwrap();
    let stop = bp
        .mark_offline_scoped_stop(&bb, &live(&b_capture), JoinedMarkReadback::Check)
        .unwrap();
    let terminal = archive(&stop);
    bp.record_offline_scoped_terminal(&bb, &stop, &terminal)
        .unwrap();
    assert_eq!(
        qualify_fixture(&bp, &bb, &a_held, &stop, &terminal),
        Err(JoinRefusal::CrossedExecution)
    );
    assert_eq!(
        qualify_fixture(&bp, &bb, &b_capture, &stop, &terminal),
        Err(JoinRefusal::Owner(TerminalFailure::HeldNotConfirmed))
    );
}

#[test]
fn boot_encoding_preserves_byte_order_and_has_no_punctuation() {
    assert_eq!(
        boot_key(&[
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff
        ]),
        "00112233445566778899aabbccddeeff"
    );
}
