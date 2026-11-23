use super::*;
use slice_campaign::offline_qualification::{
    FixtureCheckpoint, FixtureConfinement, FixtureCreateReadback, FixtureEffectiveReadback,
    FixtureExecReadback, FixtureHeldReadbacks, FixtureObject, FixtureReadyPeer, FixtureRunning,
    HoldFailure, fixture_init_policy, fixture_listener_id,
};

mod terminal;

fn create_reply(
    op: &EffectRequest,
    mutate: impl FnOnce(&mut FixtureCreateReadback),
) -> EffectObservation {
    assert_eq!(op.kind, OperationKind::Create);
    let full_id = "f".repeat(64);
    let mut serving_readback = FixtureCreateReadback {
        request_digest: op.request_digest.clone(),
        object: FixtureObject {
            full_id: full_id.clone(),
            serving_generation: op.binding.serving_sha256().into(),
        },
    };
    mutate(&mut serving_readback);
    EffectObservation::JoinedCreated {
        request_digest: op.request_digest.clone(),
        full_id,
        serving_readback,
    }
}

fn native_surrogates(op: &EffectRequest) -> (FixtureRunning, FixtureHeldReadbacks) {
    assert_eq!(op.kind, OperationKind::Start);
    let running = FixtureRunning {
        start_request_digest: op.request_digest.clone(),
        object: FixtureObject {
            full_id: op.target_id.clone().unwrap(),
            serving_generation: op.binding.serving_sha256().into(),
        },
        task_id: "opaque-running-task".into(),
        lifetime: "7".repeat(64),
        exec_generation: "8".repeat(64),
    };
    let policy = fixture_init_policy();
    let held = FixtureHeldReadbacks {
        exec: Some(FixtureExecReadback {
            running: running.clone(),
            executable_sha256: policy.executable_sha256,
            startup_closure_sha256: policy.startup_closure_sha256,
        }),
        ready: Some(FixtureReadyPeer {
            listener: fixture_listener_id(&op.binding, &op.request_digest).unwrap(),
            peer: running.clone(),
            checkpoint: FixtureCheckpoint::HeldBeforeWorkload,
        }),
        confinement: FixtureConfinement::Observed(FixtureEffectiveReadback {
            running: running.clone(),
            credentials: Some(policy.credentials),
            isolation: Some(policy.isolation),
        }),
    };
    (running, held)
}

fn running_reply(
    op: &EffectRequest,
    change: impl FnOnce(&mut FixtureRunning, &mut FixtureHeldReadbacks),
) -> EffectObservation {
    let (mut running, mut held) = native_surrogates(op);
    change(&mut running, &mut held);
    EffectObservation::JoinedRunning {
        running,
        held: Box::new(held),
    }
}

fn facts(root: &Root) -> Vec<(i64, serde_json::Value)> {
    let conn = root.conn();
    conn.prepare("SELECT slot,fact_json FROM b_lifecycle_fact ORDER BY slot,ordinal")
        .unwrap()
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .map(|row| {
            let (slot, data) = row.unwrap();
            (slot, serde_json::from_str(&data).unwrap())
        })
        .collect()
}

#[test]
fn joined_start_inert_one_shot_then_exact_held_init_no_close_or_other_effect() {
    let r = root_with_mode(true);
    let collector = r.collector();
    let mut seen = Vec::new();
    let result = collector.collect_joined(&r.request, CHECK, |op| {
        assert_eq!(op.sequence as usize, seen.len() + 1);
        seen.push(op.kind);
        if op.kind == OperationKind::Create {
            return create_reply(op, |_| {});
        }
        let row: String = r.conn().query_row(
            "SELECT m.mark_json FROM b_lifecycle_mark m JOIN task_free_b_committed_spend_v1 s USING(attempt_id)
             WHERE s.grant_id = ?1 AND s.selection_digest = ?2 AND m.sequence=2 AND m.kind=2",
            rusqlite::params![op.binding.grant_id(), op.binding.binding_digest()], |row| row.get(0),
        ).unwrap();
        let start: serde_json::Value = serde_json::from_str(&row).unwrap();
        assert_eq!(start["request"].as_str(), Some(op.request_digest.as_str()));
        assert_eq!(start["predecessor"].as_str(), op.predecessor_mark_digest.as_deref());
        assert_eq!(start["target"]["Start"]["full_id"].as_str(), op.target_id.as_deref());
        assert_eq!(start["target"]["Start"]["serving_generation"], op.binding.serving_sha256());
        let pre_start = facts(&r);
        assert_eq!(pre_start.len(), 1);
        let (slot, created) = &pre_start[0];
        assert_eq!(*slot, 1);
        assert_eq!(created["mark"]["request"].as_str(), op.predecessor_digest.as_deref());
        assert!(created["task"].is_null());
        // Reenter the collector and Campaign independently: neither gate is
        // held by this callback, and neither can mint a second continuation.
        assert_eq!(collector.collect_joined(&r.request, CHECK, unexpected), Collected::ClosedOrAlreadyAdmitted);
        assert!(r.app.offline_qualification_port().enter(&op.binding).is_err());
        running_reply(op, |_, _| {})
    });
    assert_eq!(result, Collected::Host(Outcome::InertInitHeld));
    assert_eq!(seen, [OperationKind::Create, OperationKind::Start]);
    assert_eq!(r.marks(), 2);
    let result = facts(&r);
    assert_eq!(
        result.iter().map(|(slot, _)| *slot).collect::<Vec<_>>(),
        [1, 2, 4]
    );
    assert_eq!(result[1].1["observation"], "Started");
    assert_eq!(result[2].1["observation"], "Held");
    assert_eq!(result[2].1["mark"], result[1].1["mark"]);
    assert_eq!(result[2].1["task"], result[1].1["task"]);
    assert_eq!(result[2].1["task"]["lifetime"], "7".repeat(64));
    assert_eq!(result[2].1["task"]["exec_generation"], "8".repeat(64));
    assert_eq!(
        r.collector().collect_joined(&r.request, CHECK, unexpected),
        Collected::EntryNotConfirmed
    );
}

#[test]
fn joined_start_refuses_crossed_target_generation_or_create_request() {
    for changed in [
        (|readback: &mut FixtureCreateReadback| readback.object.full_id = "b".repeat(64))
            as fn(&mut FixtureCreateReadback),
        |readback| readback.object.serving_generation = "b".repeat(64),
        |readback| readback.request_digest = "b".repeat(64),
    ] {
        let r = root_with_mode(true);
        assert_eq!(
            r.collector()
                .collect_joined(&r.request, CHECK, |op| create_reply(op, changed)),
            Collected::Host(Outcome::Unresolved(OperationKind::Create))
        );
        assert_eq!(r.marks(), 1);
        assert!(facts(&r).is_empty());
    }
}

#[test]
fn joined_close_or_owner_revoke_winning_before_start_mark_prevents_start() {
    for close in [true, false] {
        let r = root_with_mode(true);
        let collector = r.collector();
        let outcome = collector.collect_joined(&r.request, CHECK, |op| {
            assert_eq!(op.kind, OperationKind::Create);
            if close {
                collector.close();
            } else {
                r.app.revoke_offline_qualification(&op.binding).unwrap();
            }
            create_reply(op, |_| {})
        });
        assert_eq!(
            outcome,
            Collected::Host(Outcome::NotSubmitted(OperationKind::Start))
        );
        assert!(r.spent_on_disk());
        assert_eq!(r.marks(), 1);
        assert_eq!(facts(&r).len(), 1);
        assert_eq!(
            r.collector().collect_joined(&r.request, CHECK, unexpected),
            Collected::EntryNotConfirmed
        );
    }
}

#[test]
fn joined_start_does_not_delegate_after_lost_or_crossed_commit_readback() {
    for (setup, start_mark) in [
        (None, JoinedMarkReadback::LostAfterCommit),
        (
            Some(
                "UPDATE b_lifecycle_mark SET mark_json=json_set(mark_json,'$.request',printf('%064d',0)) WHERE sequence=2;",
            ),
            CHECK,
        ),
        (
            Some(
                "UPDATE b_lifecycle_fact SET fact_json=json_set(fact_json,'$.source',printf('%064d',0)) WHERE slot=1;",
            ),
            CHECK,
        ),
    ] {
        let r = root_with_mode(true);
        if let Some(change) = setup {
            r.conn().execute_batch(&format!(
                "CREATE TRIGGER wrong_start_readback AFTER INSERT ON b_lifecycle_mark WHEN NEW.sequence=2 BEGIN {change} END;"
            )).unwrap();
        }
        let faults = JoinedReadbacks {
            create_mark: CHECK,
            start_mark,
        };
        assert_eq!(
            r.collector()
                .collect_joined(&r.request, faults, |op| create_reply(op, |_| {})),
            Collected::Host(Outcome::NotSubmitted(OperationKind::Start))
        );
        assert_eq!(r.marks(), 2, "possibly issued even when no fake delegates");
        assert_eq!(facts(&r).len(), 1);
        assert_eq!(
            r.collector().collect_joined(&r.request, CHECK, unexpected),
            Collected::EntryNotConfirmed
        );
    }
}

#[test]
fn joined_start_ack_refusal_ambiguous_effect_and_crossed_running_are_not_held_or_retried() {
    type StartCase = fn(&EffectRequest) -> EffectObservation;
    let cases: [(StartCase, Outcome); 5] = [
        (
            |op| EffectObservation::Acknowledged {
                request_digest: op.request_digest.clone(),
            },
            Outcome::StartAcknowledged,
        ),
        (
            |op| EffectObservation::Refused {
                request_digest: op.request_digest.clone(),
            },
            Outcome::ConfirmedRefusal(OperationKind::Start),
        ),
        (
            |_| EffectObservation::Unknown,
            Outcome::Unresolved(OperationKind::Start),
        ),
        (
            |op| {
                running_reply(op, |running, _| {
                    running.start_request_digest = "c".repeat(64)
                })
            },
            Outcome::Unresolved(OperationKind::Start),
        ),
        (
            |op| running_reply(op, |running, _| running.object.full_id = "c".repeat(64)),
            Outcome::Unresolved(OperationKind::Start),
        ),
    ];
    for (start_effect, expected) in cases {
        let r = root_with_mode(true);
        let result = r.collector().collect_joined(&r.request, CHECK, |op| {
            if op.kind == OperationKind::Create {
                create_reply(op, |_| {})
            } else {
                start_effect(op)
            }
        });
        assert_eq!(result, Collected::Host(expected));
        assert_eq!(r.marks(), 2);
        assert_eq!(facts(&r).len(), 1);
        assert_eq!(
            r.collector().collect_joined(&r.request, CHECK, unexpected),
            Collected::EntryNotConfirmed
        );
    }
}

#[test]
fn joined_hold_requires_exact_exec_peer_and_complete_effective_state() {
    use HoldFailure::*;
    fn effective(held: &mut FixtureHeldReadbacks) -> &mut FixtureEffectiveReadback {
        let FixtureConfinement::Observed(state) = &mut held.confinement else {
            unreachable!()
        };
        state
    }
    type HoldCase = fn(&mut FixtureHeldReadbacks);
    let cases: [(HoldCase, HoldFailure); 10] = [
        (|h| h.exec = None, MissingExec),
        (
            |h| h.exec.as_mut().unwrap().startup_closure_sha256 = "0".repeat(64),
            WrongExec,
        ),
        (
            |h| h.exec.as_mut().unwrap().running.exec_generation = "0".repeat(64),
            WrongExec,
        ),
        (
            |h| h.ready.as_mut().unwrap().peer.lifetime = "0".repeat(64),
            WrongPeer,
        ),
        (
            |h| h.ready.as_mut().unwrap().listener = "0".repeat(64),
            WrongPeer,
        ),
        (
            |h| h.ready.as_mut().unwrap().checkpoint = FixtureCheckpoint::Lost,
            LostReadiness,
        ),
        (
            |h| h.confinement = FixtureConfinement::Denied,
            ConfinementDenied,
        ),
        (|h| effective(h).isolation = None, ConfinementMissing),
        (
            |h| effective(h).isolation.as_mut().unwrap().seccomp_and_lsm = "0".repeat(64),
            ConfinementMismatch,
        ),
        (
            |h| effective(h).credentials.as_mut().unwrap().no_new_privileges = false,
            ConfinementMismatch,
        ),
    ];
    for (change, rejected) in cases {
        let r = root_with_mode(true);
        let outcome = r.collector().collect_joined(&r.request, CHECK, |op| {
            if op.kind == OperationKind::Create {
                create_reply(op, |_| {})
            } else {
                running_reply(op, |_, held| change(held))
            }
        });
        assert_eq!(
            outcome,
            Collected::Host(Outcome::StartedWithoutHeld(rejected))
        );
        assert_eq!(r.marks(), 2);
        assert_eq!(
            facts(&r).iter().map(|(slot, _)| *slot).collect::<Vec<_>>(),
            [1, 2]
        );
    }
}

#[test]
fn joined_revocation_after_start_mark_cannot_recall_send_but_prevents_current_held_result() {
    let r = root_with_mode(true);
    let result = r.collector().collect_joined(&r.request, CHECK, |op| {
        if op.kind == OperationKind::Create {
            return create_reply(op, |_| {});
        }
        assert_eq!(r.marks(), 2);
        r.app.revoke_offline_qualification(&op.binding).unwrap();
        running_reply(op, |_, _| {})
    });
    assert_eq!(
        result,
        Collected::Host(Outcome::StartedWithoutHeld(HoldFailure::NotCurrent))
    );
    assert_eq!(
        facts(&r).iter().map(|(slot, _)| *slot).collect::<Vec<_>>(),
        [1, 2]
    );
}

#[test]
fn joined_confirmed_create_refusal_does_not_advance() {
    let r = root_with_mode(true);
    assert_eq!(
        r.collector().collect_joined(&r.request, CHECK, |op| {
            assert_eq!(op.kind, OperationKind::Create);
            EffectObservation::Refused {
                request_digest: op.request_digest.clone(),
            }
        }),
        Collected::Host(Outcome::ConfirmedRefusal(OperationKind::Create))
    );
    assert_eq!(r.marks(), 1);
    assert!(facts(&r).is_empty());
}
