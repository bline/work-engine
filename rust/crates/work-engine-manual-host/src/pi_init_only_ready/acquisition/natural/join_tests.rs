//! Offline join through the real Host projection and durable Campaign store.
//! The raw stock sources have separate tests; no daemon or kernel observation
//! is claimed by these synthetic, typed cuts.

use super::super::super::tests::{campaign_for_purpose, held_capture, recipe, serving};
use super::*;
use slice_campaign::{
    Campaign,
    offline_qualification::{JoinedMarkReadback, QualificationPurpose, TerminalFailure},
};

fn terminal(held: &HeldHistory) -> OfflineNaturalTerminal {
    let key = held.execution();
    let links = held.links();
    let cgroup = OfflineCgroupHandle {
        capture: key.subtree_capture.clone(),
        mount_id: key.subtree_mount_id,
        inode: key.subtree_inode,
    };
    OfflineNaturalTerminal {
        execution: OfflineTerminalExecution {
            selection: links.selection.clone(),
            create_request: links.create_request.clone(),
            create_mark: links.create_mark.clone(),
            start_mark: links.start_mark.clone(),
            live: OfflineLiveSubtree {
                running: held.running.clone(),
                boot: boot_key(&key.serving.boot),
                runtime_namespace: key.runtime_namespace.clone(),
                cgroup: cgroup.clone(),
                capture_position: 15,
                cgroup_events: b"populated 1\nfrozen 0\n".to_vec(),
            },
        },
        retirement_arm: 11,
        start_arm: 12,
        wait_arm: 16,
        held_position: 20,
        wait_reply: 21,
        retirement_read: 22,
        closure_read: 24,
        exit: OfflineNaturalExit {
            lifetime: held.running.lifetime.clone(),
            status: 0,
            runtime_wire_sha256: crate::digest(b"synthetic original Wait wire"),
        },
        retirement: OfflineNaturalRetirement {
            cgroup,
            original_pidfd_exited: true,
            retirement_archive_sha256: crate::digest(b"synthetic original rmdir archive"),
        },
        closure: stock_closure(crate::digest(b"synthetic original closed frontier")),
    }
}

#[test]
fn natural_join_qualifies_one_exact_original_from_durable_held_without_stop() {
    let (_root, config, app, selected) = campaign_for_purpose(
        &recipe(),
        &serving(),
        "b".repeat(64),
        QualificationPurpose::NaturalExit,
    );
    let port = app.offline_qualification_port();
    let held = held_capture(&port, &selected);
    let end = terminal(&held);
    assert!(
        port.mark_offline_scoped_stop(&selected, &end.execution.live, JoinedMarkReadback::Check)
            .is_err()
    );
    assert!(port.enter(&selected).is_err());
    // Opening/operation capabilities have gone away. Exact evidence against
    // the existing spend and historical Held remains admissible.
    drop(port);
    drop(app);
    let reopened = Campaign::open(config()).unwrap();
    let port = reopened.offline_qualification_port();
    assert_eq!(record_settlement(&port, &selected, &held, &end), Ok(()));
    assert!(port.enter(&selected).is_err());
    assert!(
        port.mark_offline_scoped_stop(&selected, &end.execution.live, JoinedMarkReadback::Check)
            .is_err()
    );
    assert!(record_settlement(&port, &selected, &held, &end).is_err());
}

#[test]
fn natural_join_refuses_crossed_unclosed_or_lost_cuts_without_reopening_spend() {
    type Change = fn(&mut OfflineNaturalTerminal);
    let cases: [(&str, Change, JoinRefusal); 8] = [
        (
            "wrong Start",
            |end| end.execution.start_mark = "f".repeat(64),
            JoinRefusal::CrossedExecution,
        ),
        (
            "wrong original subtree in both terminal records",
            |end| {
                end.execution.live.cgroup.inode += 1;
                end.retirement.cgroup.inode += 1;
            },
            JoinRefusal::CrossedExecution,
        ),
        (
            "wrong exit lifetime",
            |end| end.exit.lifetime = "another-original".into(),
            JoinRefusal::Owner(TerminalFailure::EndUnestablished),
        ),
        (
            "lost Wait reply",
            |end| end.wait_reply = 0,
            JoinRefusal::Owner(TerminalFailure::EndUnestablished),
        ),
        (
            "missing retirement archive",
            |end| end.retirement.retirement_archive_sha256.clear(),
            JoinRefusal::Owner(TerminalFailure::EndUnestablished),
        ),
        (
            "pending accepted Start",
            |end| end.closure.start = OfflineAcceptedDisposition::Pending,
            JoinRefusal::Owner(TerminalFailure::ActionsUnsettled),
        ),
        (
            "extra accepted work",
            |end| end.closure.other_work = OfflineOtherWork::AdditionalAcceptedWork,
            JoinRefusal::Owner(TerminalFailure::ActionsUnsettled),
        ),
        (
            "open recovery",
            |end| end.closure.recovery.runtime_reconcile = OfflineEntryFence::Open,
            JoinRefusal::Owner(TerminalFailure::RecoveryUnsettled),
        ),
    ];
    for (case, change, error) in cases {
        let (_root, config, app, selected) = campaign_for_purpose(
            &recipe(),
            &serving(),
            "b".repeat(64),
            QualificationPurpose::NaturalExit,
        );
        let port = app.offline_qualification_port();
        let held = held_capture(&port, &selected);
        let mut end = terminal(&held);
        change(&mut end);
        assert_eq!(
            record_settlement(&port, &selected, &held, &end),
            Err(error),
            "{case}"
        );
        drop(port);
        drop(app);
        let reopened = Campaign::open(config()).unwrap();
        let port = reopened.offline_qualification_port();
        assert!(port.enter(&selected).is_err(), "{case}");
        assert!(
            port.mark_offline_scoped_stop(
                &selected,
                &end.execution.live,
                JoinedMarkReadback::Check
            )
            .is_err(),
            "{case}"
        );
    }
}

#[test]
fn natural_unspent_selection_cannot_be_entered_from_a_new_campaign_opening() {
    let (_root, config, app, selected) = campaign_for_purpose(
        &recipe(),
        &serving(),
        "b".repeat(64),
        QualificationPurpose::NaturalExit,
    );
    let key = selected.history_key();
    assert!(
        !app.offline_qualification_port()
            .read_qualification_spend(&key)
            .unwrap()
    );
    drop(app);

    let reopened = Campaign::open(config()).unwrap();
    let port = reopened.offline_qualification_port();
    let Err(error) = port.enter(&selected) else {
        panic!("a historic selection cannot recreate current entry authority");
    };
    assert!(
        error
            .to_string()
            .contains("selection Store opening differs")
    );
    assert!(!port.read_qualification_spend(&key).unwrap());
}

#[test]
fn natural_join_cannot_substitute_one_stop_purpose_or_another_held_cut() {
    let (_root, _config, app, selected) = campaign_for_purpose(
        &recipe(),
        &serving(),
        "b".repeat(64),
        QualificationPurpose::OneStop,
    );
    let port = app.offline_qualification_port();
    let held = held_capture(&port, &selected);
    let end = terminal(&held);
    assert_eq!(
        record_settlement(&port, &selected, &held, &end),
        Err(JoinRefusal::Owner(TerminalFailure::CrossedSubject))
    );

    let (_root, _config, app, selected) = campaign_for_purpose(
        &recipe(),
        &serving(),
        "b".repeat(64),
        QualificationPurpose::NaturalExit,
    );
    let port = app.offline_qualification_port();
    let mut held = held_capture(&port, &selected);
    let end = terminal(&held);
    held.observed_at_ns -= 1;
    assert_eq!(
        record_settlement(&port, &selected, &held, &end),
        Err(JoinRefusal::Owner(TerminalFailure::HeldNotConfirmed))
    );
}
