use super::*;
use crate::application::offline_qualification::{
    FixtureCreateReadback, MarkedStart, SelectionPins,
};
use crate::store::{
    offline_qualification::FIXTURE_NOW_MS,
    task_free_b::{Evidence, Kind, Observation, Slot},
};
use crate::{AdmitRequest, Campaign, TrustedConfig, test_support as support};

fn own_attempt() -> (
    tempfile::TempDir,
    impl Fn() -> TrustedConfig,
    Campaign,
    AttemptBinding,
) {
    let (temp, repository, workspace, identity, baseline, boundary) = support::setup();
    let root = temp.path().join("campaign");
    let (r, w, i, a, b) = (
        repository.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    );
    let config = move || {
        support::config(
            root.clone(),
            r.clone(),
            w.clone(),
            i.clone(),
            a.clone(),
            b.clone(),
        )
    };
    let mut app = Campaign::initialize(config()).unwrap();
    let head = support::applied(
        app.admit(
            "admit-scoped-stop",
            AdmitRequest {
                identity: identity.clone(),
                repository,
                workspace,
                accepted_boundary: boundary,
                baseline,
                expected_impact: None,
            },
        )
        .unwrap(),
    )
    .revision;
    app.attach_joined_offline_qualification_fixture().unwrap();
    let binding = app
        .select_offline_qualification(
            &identity,
            &head,
            SelectionPins {
                grant_id: "a".repeat(64),
                intent_sha256: "b".repeat(64),
                recipe_sha256: "c".repeat(64),
                serving_sha256: "d".repeat(64),
                expires_at_unix_ms: 4_102_444_800_000,
            },
        )
        .unwrap();
    (temp, config, app, binding)
}

fn starting(app: &Campaign, binding: &AttemptBinding) -> (MarkedStart, OfflineLiveSubtree) {
    use crate::application::offline_qualification::FixtureObject;
    let create = binding.create_request_digest().unwrap();
    let object = FixtureObject {
        full_id: "f".repeat(64),
        serving_generation: binding.serving_sha256().into(),
    };
    let start = app
        .offline_qualification_port()
        .enter(binding)
        .unwrap()
        .consume()
        .unwrap()
        .mark_joined_create(&create, JoinedMarkReadback::Check)
        .unwrap()
        .confirm_created(
            &create,
            &object.full_id,
            FixtureCreateReadback {
                request_digest: create.clone(),
                object: object.clone(),
            },
        )
        .unwrap();
    let live = OfflineLiveSubtree {
        running: FixtureRunning {
            start_request_digest: start.request().request_digest.clone(),
            object,
            task_id: "rt-init".into(),
            lifetime: "1".repeat(64),
            exec_generation: "2".repeat(64),
        },
        boot: "boot-A".into(),
        runtime_namespace: "moby".into(),
        cgroup: OfflineCgroupHandle {
            capture: "3".repeat(64),
            mount_id: 10,
            inode: 200,
        },
        capture_position: 2,
        cgroup_events: b"populated 1\nfrozen 0\n".to_vec(),
    };
    (start.mark(JoinedMarkReadback::Check).unwrap(), live)
}

fn running(app: &Campaign, binding: &AttemptBinding) -> OfflineLiveSubtree {
    let (start, live) = starting(app, binding);
    let _ = start.confirm_running(live.running.clone()).unwrap();
    live
}

fn read<T>(subject: &OfflineStopSubject, position: u64, observed: T) -> OfflineStopReadback<T> {
    OfflineStopReadback {
        subject: subject.clone(),
        position,
        observed,
    }
}

fn closed(subject: &OfflineStopSubject) -> OfflineStopTerminalReadbacks {
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
                stop: OfflineAcceptedDisposition::Completed,
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
fn response_diagnostic_neither_required_nor_relabelled_by_terminal() {
    for response in [
        None,
        Some(OfflineStopResponse::Lost),
        Some(OfflineStopResponse::Unknown),
        Some(OfflineStopResponse::ReturnedFailure),
    ] {
        let (_temp, _, app, b) = own_attempt();
        let live = running(&app, &b);
        let port = app.offline_qualification_port();
        let subject = port
            .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
            .unwrap();
        if let Some(response) = response {
            port.record_offline_stop_response(&b, &read(&subject, 4, response))
                .unwrap();
        }
        let mut evidence = closed(&subject);
        if response == Some(OfflineStopResponse::Lost) {
            evidence.accepted.as_mut().unwrap().observed.stop =
                OfflineAcceptedDisposition::EffectivelyRevoked;
        }
        // End and exclusion remain recordable after revoke. Held is not a
        // prerequisite to settle the exact spent attempt.
        app.revoke_offline_qualification(&b).unwrap();
        assert_eq!(
            port.confirm_offline_scoped_terminal(&b, &subject, &evidence),
            Err(TerminalFailure::FactNotConfirmed)
        );
        assert_eq!(
            port.record_offline_scoped_terminal(&b, &subject, &evidence),
            Ok(())
        );
        assert_eq!(
            port.confirm_offline_scoped_terminal(&b, &subject, &evidence),
            Ok(())
        );
        let history = app
            .owner()
            .unwrap()
            .store
            .read_b_history(b.lifecycle())
            .unwrap();
        let actual = history.evidence(Slot::Operation(Kind::Close));
        assert!(match response {
            None => actual == Evidence::Absent,
            Some(OfflineStopResponse::Lost) => matches!(actual, Evidence::Lost(_)),
            Some(OfflineStopResponse::Unknown) => matches!(actual, Evidence::Unknown(_)),
            Some(OfflineStopResponse::ReturnedFailure) => matches!(actual, Evidence::Failed(_)),
            _ => false,
        });
        assert_eq!(history.evidence(Slot::Hold), Evidence::Absent);
        assert!(matches!(
            history.evidence(Slot::Terminal),
            Evidence::Observed {
                observation: Observation::ScopedTerminal(_),
                ..
            }
        ));
    }
}

#[test]
fn retained_history_after_lost_close_confirmation_never_remints_a_mark() {
    let (_temp, config, app, b) = own_attempt();
    let live = running(&app, &b);
    let port = app.offline_qualification_port();
    assert!(
        port.mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::LostAfterCommit)
            .is_err()
    );
    assert!(
        port.mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
            .is_err()
    );
    drop(port);
    drop(app);
    let app = Campaign::open(config()).unwrap();
    let port = app.offline_qualification_port();
    assert!(port.enter(&b).is_err());
    assert!(
        port.mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
            .is_err()
    );
    let subject = port.read_offline_scoped_stop(&b, &live).unwrap();
    // No reply was observed. A lost mark confirmation is not a lost Stop
    // response. Independent revocation covers the possible issue.
    let mut evidence = closed(&subject);
    evidence.accepted.as_mut().unwrap().observed.stop =
        OfflineAcceptedDisposition::EffectivelyRevoked;
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &evidence),
        Ok(())
    );
    drop(port);
    drop(app);
    let app = Campaign::open(config()).unwrap();
    let port = app.offline_qualification_port();
    assert_eq!(port.read_offline_scoped_stop(&b, &live).unwrap(), subject);
    assert_eq!(
        port.confirm_offline_scoped_terminal(&b, &subject, &evidence),
        Ok(())
    );
    assert!(port.enter(&b).is_err());
    let history = app
        .owner()
        .unwrap()
        .store
        .read_b_history(b.lifecycle())
        .unwrap();
    assert_eq!(history.tip(), history.mark(Kind::Close));
    assert_eq!(
        history.evidence(Slot::Operation(Kind::Close)),
        Evidence::Absent
    );
}

#[test]
fn reopen_even_before_close_does_not_offer_a_new_current_spend() {
    let (_temp, config, app, b) = own_attempt();
    let live = running(&app, &b);
    drop(app);
    let app = Campaign::open(config()).unwrap();
    let port = app.offline_qualification_port();
    assert!(port.enter(&b).is_err());
    assert!(
        port.mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
            .is_err()
    );
    assert!(port.read_offline_scoped_stop(&b, &live).is_err());
    let history = app
        .owner()
        .unwrap()
        .store
        .read_b_history(b.lifecycle())
        .unwrap();
    assert_eq!(history.tip(), history.mark(Kind::Start));
    assert_eq!(history.evidence(Slot::Terminal), Evidence::Absent);
}

#[test]
fn stale_spend_or_ambiguous_start_cannot_manufacture_scoped_stop() {
    for case in [
        "expired",
        "revoked",
        "start-unreturned",
        "lost-start-response",
    ] {
        let (_temp, _, app, b) = own_attempt();
        let (start, live) = starting(&app, &b);
        if matches!(case, "expired" | "revoked") {
            let _ = start.confirm_running(live.running.clone()).unwrap();
        }
        if case == "expired" {
            FIXTURE_NOW_MS.set(Some(b.expires_at()));
        }
        if case == "revoked" {
            app.revoke_offline_qualification(&b).unwrap();
        }
        if case == "lost-start-response" {
            // Inject the audited B slot shape for a lost Start reply with NO
            // independently bound task association. No effects are simulated.
            app.owner().unwrap().store.conn.execute_batch(
                "INSERT INTO b_lifecycle_fact SELECT attempt_id,2,0,
                 json_object('ingress',json_extract(binding_json,'$.evidence_ingress'),
                 'source',printf('%064d',0),'slot',json_object('Operation','Start'),
                 'mark',json(mark_json),'task',NULL,'observation','Lost')
                 FROM b_lifecycle_attempt JOIN b_lifecycle_mark USING(attempt_id) WHERE sequence=2;"
            ).unwrap();
        }
        let port = app.offline_qualification_port();
        let result = port.mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check);
        FIXTURE_NOW_MS.set(None);
        assert!(result.is_err(), "{case}");
        assert!(port.enter(&b).is_err());
        let h = app
            .owner()
            .unwrap()
            .store
            .read_b_history(b.lifecycle())
            .unwrap();
        assert_eq!(h.tip(), h.mark(Kind::Start), "{case}");
        assert_eq!(h.evidence(Slot::Terminal), Evidence::Absent);
        if case == "lost-start-response" {
            assert!(matches!(
                h.evidence(Slot::Operation(Kind::Start)),
                Evidence::Lost(_)
            ));
        }
    }
}

#[test]
fn subtree_and_time_are_bound_again_at_the_final_cut() {
    let (_temp, _, app, b) = own_attempt();
    let mut live = running(&app, &b);
    let port = app.offline_qualification_port();
    live.cgroup_events = b"populated 1\npopulated 0\n".to_vec();
    assert!(
        port.mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
            .is_err()
    );
    live.cgroup_events = b"populated 1\nfrozen 0\n".to_vec();
    let subject = port
        .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
        .unwrap();
    for fault in [
        "survivor",
        "frozen-not-end",
        "torn",
        "missing-key",
        "duplicate-key",
        "reused-inode",
        "untracked-members",
        "no-init-exit",
        "crossed-init",
        "too-early",
    ] {
        let mut evidence = closed(&subject);
        let end = evidence.end.as_mut().unwrap();
        match fault {
            "survivor" => end.observed.cgroup_events = b"populated 1\n".to_vec(),
            "frozen-not-end" => end.observed.cgroup_events = b"populated 1\nfrozen 1\n".to_vec(),
            "torn" => end.observed.cgroup_events = b"populated 0".to_vec(),
            "missing-key" => end.observed.cgroup_events = b"frozen 0\n".to_vec(),
            "duplicate-key" => end.observed.cgroup_events = b"populated 0\npopulated 0\n".to_vec(),
            "reused-inode" => end.observed.cgroup.capture = "4".repeat(64),
            "untracked-members" => {
                end.observed.containment = OfflineContainment::MigrationOrContinuityUnknown
            }
            "no-init-exit" => end.observed.exited_init_lifetime = None,
            "crossed-init" => end.observed.exited_init_lifetime = Some("5".repeat(64)),
            "too-early" => end.position = 6,
            _ => unreachable!(),
        }
        assert_eq!(
            port.record_offline_scoped_terminal(&b, &subject, &evidence),
            Err(TerminalFailure::EndUnestablished),
            "{fault}"
        );
    }
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &closed(&subject)),
        Ok(())
    );
}

#[test]
fn old_task_and_successful_response_do_not_settle_other_work_or_recovery() {
    let (_temp, _, app, b) = own_attempt();
    let live = running(&app, &b);
    let port = app.offline_qualification_port();
    let subject = port
        .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
        .unwrap();
    port.record_offline_stop_response(&b, &read(&subject, 4, OfflineStopResponse::ReturnedSuccess))
        .unwrap();
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &OfflineStopTerminalReadbacks::default()),
        Err(TerminalFailure::MissingWitness)
    );
    for (fault, refusal) in [
        ("missing-recovery", TerminalFailure::MissingWitness),
        ("plain-stopped", TerminalFailure::ActionsUnsettled),
        ("lookup-absent", TerminalFailure::ActionsUnsettled),
        ("delayed", TerminalFailure::ActionsUnsettled),
        ("competing", TerminalFailure::ActionsUnsettled),
        ("cleanup-tail", TerminalFailure::ActionsUnsettled),
        ("open-ingress", TerminalFailure::ActionsUnsettled),
        ("early-snapshot", TerminalFailure::ActionsUnsettled),
        ("open-recovery", TerminalFailure::RecoveryUnsettled),
        ("unknown-backing", TerminalFailure::RecoveryUnsettled),
    ] {
        let mut evidence = closed(&subject);
        match fault {
            "missing-recovery" => evidence.recovery = None,
            "plain-stopped" => {
                evidence.old_task.as_mut().unwrap().observed =
                    OfflineOldTask::Stopped(OfflineOldTaskTransitions::Unestablished)
            }
            "lookup-absent" => {
                evidence.old_task.as_mut().unwrap().observed = OfflineOldTask::AbsentOrUnknown
            }
            "delayed" => {
                evidence.old_task.as_mut().unwrap().observed = OfflineOldTask::PositivelyRetired;
                evidence.accepted.as_mut().unwrap().observed.stop =
                    OfflineAcceptedDisposition::Pending;
            }
            "competing" => {
                evidence.accepted.as_mut().unwrap().observed.other_work =
                    OfflineOtherWork::AdditionalAcceptedWork
            }
            "cleanup-tail" => {
                evidence.accepted.as_mut().unwrap().observed.cleanup_tail =
                    OfflineCleanupTail::CanExecuteOrReopen
            }
            "open-ingress" => {
                evidence.ingress.as_mut().unwrap().observed.runtime = OfflineEntryFence::Open
            }
            "early-snapshot" => evidence.accepted.as_mut().unwrap().position = 4,
            "open-recovery" => {
                evidence
                    .recovery
                    .as_mut()
                    .unwrap()
                    .observed
                    .runtime_reconcile = OfflineEntryFence::Open
            }
            "unknown-backing" => {
                evidence.recovery.as_mut().unwrap().observed.backing =
                    OfflineRecoveryBacking::Unknown
            }
            _ => unreachable!(),
        }
        assert_eq!(
            port.record_offline_scoped_terminal(&b, &subject, &evidence),
            Err(refusal),
            "{fault}"
        );
    }
    let mut evidence = closed(&subject);
    evidence.old_task.as_mut().unwrap().observed = OfflineOldTask::PositivelyRetired;
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &evidence),
        Ok(())
    );
}

#[test]
fn crossed_runtime_live_anchor_and_frontier_are_never_spliced() {
    let (_temp, _, app, b) = own_attempt();
    let live = running(&app, &b);
    let port = app.offline_qualification_port();
    let mut crossed = live.clone();
    crossed.running.exec_generation = "6".repeat(64);
    assert!(
        port.mark_offline_scoped_stop(&b, &crossed, JoinedMarkReadback::Check)
            .is_err()
    );
    let subject = port
        .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
        .unwrap();
    for fault in [
        "boot",
        "serving",
        "namespace",
        "frontier",
        "subtree",
        "stale-position",
    ] {
        let mut evidence = closed(&subject);
        let part = evidence.recovery.as_mut().unwrap();
        match fault {
            "boot" => part.subject.execution.live.boot = "boot-other".into(),
            "serving" => {
                part.subject
                    .execution
                    .live
                    .running
                    .object
                    .serving_generation = "5".repeat(64)
            }
            "namespace" => part.subject.execution.live.runtime_namespace = "other".into(),
            "frontier" => part.subject.close_mark = part.subject.execution.start_mark.clone(),
            "subtree" => part.subject.execution.live.cgroup.inode += 1,
            "stale-position" => part.position = live.capture_position,
            _ => unreachable!(),
        }
        assert_eq!(
            port.record_offline_scoped_terminal(&b, &subject, &evidence),
            Err(TerminalFailure::CrossedSubject),
            "{fault}"
        );
    }
    let mut crossed = live.clone();
    crossed.cgroup.capture = "6".repeat(64);
    assert!(port.read_offline_scoped_stop(&b, &crossed).is_err());
    let mut crossed_subject = subject.clone();
    crossed_subject.execution.live = crossed;
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &crossed_subject, &closed(&crossed_subject)),
        Err(TerminalFailure::CrossedSubject)
    );
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &closed(&subject)),
        Ok(())
    );
}

#[test]
fn later_affirmative_evidence_contests_terminal_in_the_existing_owner() {
    for case in [
        "restorable",
        "populated",
        "complete-restorable",
        "complete-populated",
        "ingress",
        "reconcile",
        "extra-work",
        "cleanup",
    ] {
        let (_temp, config, app, b) = own_attempt();
        let live = running(&app, &b);
        let port = app.offline_qualification_port();
        let subject = port
            .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
            .unwrap();
        let archive = closed(&subject);
        port.record_offline_scoped_terminal(&b, &subject, &archive)
            .unwrap();
        let later = archive.end.as_ref().unwrap().position + 1;
        drop(port);
        drop(app);
        let app = Campaign::open(config()).unwrap();
        let port = app.offline_qualification_port();
        assert_eq!(
            port.confirm_offline_scoped_terminal(&b, &subject, &archive),
            Ok(())
        );

        // Each positive piece stands alone; absent sibling evidence is not a
        // contradiction, but also cannot erase an affirmative observation.
        let mut reads = if case.starts_with("complete-") {
            archive.clone()
        } else {
            OfflineStopTerminalReadbacks::default()
        };
        match case {
            "restorable" | "complete-restorable" | "reconcile" => {
                let mut recovery = archive.recovery.clone().unwrap();
                recovery.position = later;
                if case != "reconcile" {
                    recovery.observed.backing = OfflineRecoveryBacking::Restorable;
                } else {
                    recovery.observed.runtime_reconcile = OfflineEntryFence::Open;
                }
                reads.recovery = Some(recovery);
                if case == "complete-restorable" {
                    reads.end.as_mut().unwrap().position = later + 1;
                }
            }
            "populated" | "complete-populated" => {
                let mut end = archive.end.clone().unwrap();
                end.position = later;
                end.observed.exited_init_lifetime = None;
                end.observed.cgroup_events = b"populated 1\nfrozen 0\n".to_vec();
                // A nonaffirmative sibling is not required for, and
                // cannot suppress, the independently attributable sample.
                reads.end = Some(end);
                if case == "populated" {
                    let mut unknown = archive.recovery.clone().unwrap();
                    unknown.position = later;
                    unknown.observed.backing = OfflineRecoveryBacking::Unknown;
                    reads.recovery = Some(unknown);
                }
            }
            "ingress" => {
                let mut ingress = archive.ingress.clone().unwrap();
                ingress.position = later;
                ingress.observed.engine = OfflineEntryFence::Open;
                reads.ingress = Some(ingress);
            }
            "extra-work" | "cleanup" => {
                let mut accepted = archive.accepted.clone().unwrap();
                accepted.position = later;
                if case == "extra-work" {
                    accepted.observed.other_work = OfflineOtherWork::AdditionalAcceptedWork;
                } else {
                    accepted.observed.cleanup_tail = OfflineCleanupTail::CanExecuteOrReopen;
                }
                reads.accepted = Some(accepted);
            }
            _ => unreachable!(),
        }
        assert_eq!(
            port.record_offline_scoped_terminal(&b, &subject, &reads),
            Err(TerminalFailure::Contested),
            "{case}"
        );
        assert_eq!(
            port.confirm_offline_scoped_terminal(&b, &subject, &archive),
            Err(TerminalFailure::Contested),
            "{case}"
        );
        drop(port);
        drop(app);
        let app = Campaign::open(config()).unwrap();
        let port = app.offline_qualification_port();
        assert_eq!(
            port.confirm_offline_scoped_terminal(&b, &subject, &archive),
            Err(TerminalFailure::Contested),
            "{case}"
        );
        assert_eq!(
            port.record_offline_scoped_terminal(&b, &subject, &archive),
            Err(TerminalFailure::Contested),
            "{case}"
        );
        assert!(port.enter(&b).is_err());
    }
}

#[test]
fn stale_nonaffirmative_or_crossed_readbacks_cannot_contest_terminal() {
    let (_temp, config, app, b) = own_attempt();
    let live = running(&app, &b);
    let port = app.offline_qualification_port();
    let subject = port
        .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
        .unwrap();
    let mut archive = closed(&subject);
    archive.end.as_mut().unwrap().position = 20;
    port.record_offline_scoped_terminal(&b, &subject, &archive)
        .unwrap();

    for case in [
        "earlier-than-final",
        "same-as-final",
        "mixed-subjects",
        "crossed-readback",
        "crossed-handle",
        "crossed-lifetime",
        "malformed-populated",
        "broken-continuity",
        "post-cut-pending",
        "post-cut-unknown",
        "absent",
    ] {
        let mut later = OfflineStopTerminalReadbacks::default();
        match case {
            "earlier-than-final" | "same-as-final" | "crossed-readback" | "mixed-subjects"
            | "post-cut-unknown" => {
                let mut recovery = archive.recovery.clone().unwrap();
                recovery.position = 21;
                recovery.observed.backing = OfflineRecoveryBacking::Restorable;
                match case {
                    "earlier-than-final" => recovery.position = 10,
                    "same-as-final" => recovery.position = 20,
                    "mixed-subjects" => {
                        let mut ingress = archive.ingress.clone().unwrap();
                        ingress.subject.execution.live.boot = "boot-other".into();
                        later.ingress = Some(ingress);
                    }
                    "crossed-readback" => {
                        recovery.subject.close_mark = subject.close_request.clone()
                    }
                    "post-cut-unknown" => {
                        recovery.observed.backing = OfflineRecoveryBacking::Unknown;
                        recovery.observed.runtime_reconcile = OfflineEntryFence::Unknown;
                    }
                    _ => {}
                }
                later.recovery = Some(recovery);
            }
            "crossed-handle" | "crossed-lifetime" | "malformed-populated" | "broken-continuity" => {
                let mut end = archive.end.clone().unwrap();
                end.position = 21;
                end.observed.cgroup_events = b"populated 1\n".to_vec();
                match case {
                    "crossed-handle" => end.observed.cgroup.capture = "4".repeat(64),
                    "crossed-lifetime" => end.observed.exited_init_lifetime = Some("5".repeat(64)),
                    "malformed-populated" => {
                        end.observed.cgroup_events = b"populated 1\npopulated 0\n".to_vec()
                    }
                    "broken-continuity" => {
                        end.observed.containment = OfflineContainment::MigrationOrContinuityUnknown;
                    }
                    _ => unreachable!(),
                }
                later.end = Some(end);
            }
            "post-cut-pending" => {
                let mut accepted = archive.accepted.clone().unwrap();
                accepted.position = 21;
                accepted.observed.stop = OfflineAcceptedDisposition::Pending;
                later.accepted = Some(accepted);
            }
            "absent" => {}
            _ => unreachable!(),
        }
        assert_eq!(
            port.record_offline_scoped_terminal(&b, &subject, &later),
            Err(match case {
                "crossed-readback" | "crossed-handle" | "crossed-lifetime" | "mixed-subjects" =>
                    TerminalFailure::CrossedSubject,
                _ => TerminalFailure::MissingWitness,
            }),
            "{case}"
        );
        assert_eq!(
            port.confirm_offline_scoped_terminal(&b, &subject, &archive),
            Ok(()),
            "{case}"
        );
    }
    drop(port);
    drop(app);
    let app = Campaign::open(config()).unwrap();
    assert_eq!(
        app.offline_qualification_port()
            .confirm_offline_scoped_terminal(&b, &subject, &archive),
        Ok(())
    );
}

#[test]
fn refusal_without_prior_positive_terminal_does_not_poison_first_closure() {
    let (_temp, config, app, b) = own_attempt();
    let live = running(&app, &b);
    let port = app.offline_qualification_port();
    let subject = port
        .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
        .unwrap();
    let archive = closed(&subject);
    let mut later = archive.clone();
    let recovery = later.recovery.as_mut().unwrap();
    recovery.position = 9;
    recovery.observed.backing = OfflineRecoveryBacking::Restorable;
    later.end.as_mut().unwrap().position = 10;
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &later),
        Err(TerminalFailure::RecoveryUnsettled)
    );
    let partial = OfflineStopTerminalReadbacks {
        recovery: later.recovery.clone(),
        ..Default::default()
    };
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &partial),
        Err(TerminalFailure::MissingWitness)
    );
    assert_eq!(
        app.owner()
            .unwrap()
            .store
            .read_b_history(b.lifecycle())
            .unwrap()
            .evidence(Slot::Terminal),
        Evidence::Absent
    );
    drop(port);
    drop(app);
    let app = Campaign::open(config()).unwrap();
    let port = app.offline_qualification_port();
    // Closure after that negative observation uses a genuinely later cut.
    let recovery = later.recovery.as_mut().unwrap();
    recovery.position = 11;
    recovery.observed.backing = OfflineRecoveryBacking::PositivelyRetired;
    later.end.as_mut().unwrap().position = 12;
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &later),
        Ok(())
    );
    assert_eq!(
        port.confirm_offline_scoped_terminal(&b, &subject, &later),
        Ok(())
    );
}

#[test]
fn changed_or_unprojected_durable_cut_cannot_route_scoped_intake() {
    for defect in ["altered", "missing", "legacy-at-close"] {
        let (_temp, config, app, b) = own_attempt();
        let live = running(&app, &b);
        let port = app.offline_qualification_port();
        let subject = port
            .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
            .unwrap();
        let mut archive = closed(&subject);
        archive.end.as_mut().unwrap().position = 20;
        port.record_offline_scoped_terminal(&b, &subject, &archive)
            .unwrap();
        {
            // Corrupt only a private offline owner fixture. Lowering the
            // projected ordinal to 8 would turn position 10 into a false
            // contradiction if the owner failed to verify the commitment.
            let owner = app.owner().unwrap();
            let conn = &owner.store.conn;
            let old: String = conn
                .query_row(
                    "SELECT fact_json FROM b_lifecycle_fact WHERE slot=5 AND ordinal=0",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            let mut row: serde_json::Value = serde_json::from_str(&old).unwrap();
            let cut = row["observation"]["ScopedTerminal"]
                .as_object_mut()
                .unwrap();
            assert_eq!(cut["final_position"], serde_json::json!(20));
            match defect {
                "altered" => cut["final_position"] = serde_json::json!(8),
                "missing" => {
                    cut.remove("final_position");
                }
                "legacy-at-close" => {
                    cut.remove("final_position");
                    row["observation"] = serde_json::json!({"Terminal": cut});
                }
                _ => unreachable!(),
            }
            assert_eq!(
                conn.execute(
                    "UPDATE b_lifecycle_fact SET fact_json=?1 WHERE slot=5 AND ordinal=0",
                    [row.to_string()],
                )
                .unwrap(),
                1
            );
        }
        drop(port);
        drop(app);
        let app = Campaign::open(config()).unwrap();
        let port = app.offline_qualification_port();
        let mut recovery = archive.recovery.clone().unwrap();
        recovery.position = 10;
        recovery.observed.backing = OfflineRecoveryBacking::Restorable;
        let partial = OfflineStopTerminalReadbacks {
            recovery: Some(recovery),
            ..Default::default()
        };
        for input in [&partial, &archive] {
            assert_eq!(
                port.record_offline_scoped_terminal(&b, &subject, input),
                Err(TerminalFailure::FactNotConfirmed),
                "{defect}"
            );
        }
        assert_eq!(
            port.confirm_offline_scoped_terminal(&b, &subject, &archive),
            Err(TerminalFailure::FactNotConfirmed),
            "{defect}"
        );
        let mut owner = app.owner().unwrap();
        let count: i64 = owner
            .store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM b_lifecycle_fact WHERE slot=5",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "{defect}");
        match (defect, owner.store.read_b_history(b.lifecycle())) {
            ("legacy-at-close", Ok(history)) => assert!(matches!(
                history.evidence(Slot::Terminal),
                Evidence::Observed {
                    observation: Observation::Terminal { .. },
                    ..
                }
            )),
            ("altered" | "missing", Err(crate::CampaignError::Integrity(_))) => {}
            _ => panic!("unexpected legacy/integrity disposition for {defect}"),
        }
    }
}

#[test]
fn scoped_inode_and_final_ordinal_do_not_alias_through_numeric_encoding() {
    let (_temp, config, app, b) = own_attempt();
    let mut live = running(&app, &b);
    let high = 1_u64 << 53;
    live.cgroup.inode = high;
    let port = app.offline_qualification_port();
    let subject = port
        .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
        .unwrap();
    let mut archive = closed(&subject);
    archive.end.as_mut().unwrap().position = high;
    port.record_offline_scoped_terminal(&b, &subject, &archive)
        .unwrap();
    drop(port);
    drop(app);
    let app = Campaign::open(config()).unwrap();
    let port = app.offline_qualification_port();
    let mut alternate_cut = archive.clone();
    alternate_cut.end.as_mut().unwrap().position = high + 1;
    assert_eq!(
        port.confirm_offline_scoped_terminal(&b, &subject, &alternate_cut),
        Err(TerminalFailure::FactNotConfirmed)
    );
    let mut recovery = archive.recovery.clone().unwrap();
    recovery.position = high + 1;
    recovery.observed.backing = OfflineRecoveryBacking::Restorable;
    let mut crossed = subject.clone();
    crossed.execution.live.cgroup.inode += 1;
    recovery.subject = crossed.clone();
    let mut partial = OfflineStopTerminalReadbacks {
        recovery: Some(recovery),
        ..Default::default()
    };
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &crossed, &partial),
        Err(TerminalFailure::CrossedSubject)
    );
    assert_eq!(
        port.confirm_offline_scoped_terminal(&b, &subject, &archive),
        Ok(())
    );
    partial.recovery.as_mut().unwrap().subject = subject.clone();
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &partial),
        Err(TerminalFailure::Contested)
    );
}

#[test]
fn changed_valid_receipts_do_not_overwrite_the_exact_durable_terminal_cut() {
    let (_temp, _, app, b) = own_attempt();
    let live = running(&app, &b);
    let port = app.offline_qualification_port();
    let subject = port
        .mark_offline_scoped_stop(&b, &live, JoinedMarkReadback::Check)
        .unwrap();
    let first = closed(&subject);
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &first),
        Ok(())
    );
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &first),
        Ok(())
    );
    let mut other = first.clone();
    other.end.as_mut().unwrap().position += 1;
    assert_eq!(
        port.confirm_offline_scoped_terminal(&b, &subject, &other),
        Err(TerminalFailure::FactNotConfirmed)
    );
    assert_eq!(
        port.record_offline_scoped_terminal(&b, &subject, &other),
        Err(TerminalFailure::Contested)
    );
    assert_eq!(
        port.confirm_offline_scoped_terminal(&b, &subject, &first),
        Err(TerminalFailure::Contested)
    );
}
