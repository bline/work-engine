//! Explicit-run controlled SC0 history workload over public APIs and real SQLite.
//! JSON lines are raw observations; this test makes no latency assertion.
use std::fs;
use std::path::Path;
use std::time::Instant;

use rusqlite::Connection;
use serde_json::json;
use slice_campaign::contract::episode_digest;
use slice_campaign::{
    Campaign, DispatchCommand, DispatchEffect, Preparation, Reconcile, RecoveryLocator,
};

mod support;

fn observe<T>(kind: &str, milestone: usize, operation: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let result = operation();
    println!(
        "{}",
        json!({"event":"sample","kind":kind,"milestone":milestone,
        "elapsed_ns":start.elapsed().as_nanos()})
    );
    result
}

fn storage(root: &std::path::Path, milestone: usize) {
    let db = Connection::open(root.join("slice-campaign.sqlite")).unwrap();
    let (receipts, result_bytes): (i64, i64) = db
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(LENGTH(result_json)),0) FROM operation_receipt",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let slots: i64 = db
        .query_row("SELECT COUNT(*) FROM request_slot", [], |row| row.get(0))
        .unwrap();
    println!(
        "{}",
        json!({"event":"storage","milestone":milestone,"receipts":receipts,
        "result_bytes":result_bytes,"slots":slots})
    );
}

fn retain_root(root: &Path) {
    let Ok(target) = std::env::var("P1_RETAIN_ROOT") else {
        return;
    };
    let target = std::path::PathBuf::from(target);
    assert!(
        !target.exists(),
        "retained corpus destination already exists"
    );
    fn copy_tree(source: &Path, target: &Path) {
        fs::create_dir(target).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let destination = target.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &destination);
            } else {
                assert!(entry.file_type().unwrap().is_file());
                fs::copy(entry.path(), destination).unwrap();
            }
        }
    }
    copy_tree(root, &target);
    println!(
        "{}",
        json!({"event":"retained_root","original":root,
        "artifact":target})
    );
}

#[test]
#[ignore = "explicit bounded performance workload"]
fn controlled_sc0_history() {
    let count: usize = std::env::var("P1_COUNT")
        .unwrap_or_else(|_| "40".into())
        .parse()
        .unwrap();
    let samples: usize = std::env::var("P1_SAMPLES")
        .unwrap_or_else(|_| "30".into())
        .parse()
        .unwrap();
    let warm: usize = std::env::var("P1_WARM")
        .unwrap_or_else(|_| "10".into())
        .parse()
        .unwrap();
    let dispatch = std::env::var("P1_DISPATCH").ok().as_deref() == Some("1");
    let focus_replay = std::env::var("P1_FOCUS_REPLAY").ok().as_deref() == Some("1");
    let selection_only = std::env::var("P1_SELECTION_ONLY").ok().as_deref() == Some("1");
    let all_selected = std::env::var("P1_ALL_SELECTED").ok().as_deref() != Some("0");
    let milestones = [1, 2, 8, 16, 32, 40, 64, 129];
    if focus_replay {
        assert!(dispatch && !selection_only && count == 16 && warm > 0);
    }
    let start = Instant::now();
    let (temp, mut app, identity, selected, reopen) = observe("setup_selection", count, || {
        support::selected_campaign(count, all_selected)
    });
    let selected_count = if count == 0 {
        0
    } else if all_selected {
        count
    } else {
        1
    };
    println!(
        "{}",
        json!({"event":"selection","count":count,"selected":selected_count,
        "omitted":count-selected_count,"phase":selected.phase,"root":temp.path(),
        "state_json_bytes":serde_json::to_vec(&selected).unwrap().len()})
    );
    let root = temp.path().join("campaign");
    storage(&root, 0);
    if selection_only {
        let revision = selected.revision;
        drop(app);
        app = observe("reopen", count, || Campaign::open(reopen()).unwrap());
        let state = observe("first_read_after_reopen", count, || {
            app.read(&identity).unwrap().unwrap()
        });
        assert_eq!(state.revision, revision);
        for _ in 0..warm {
            let _ = app.read(&identity).unwrap().unwrap();
        }
        for _ in 0..samples {
            let read = observe("repeated_current_read", count, || {
                app.read(&identity).unwrap().unwrap()
            });
            assert_eq!(read.revision, revision);
        }
        println!(
            "{}",
            json!({"event":"complete","count":count,
            "elapsed_ns":start.elapsed().as_nanos()})
        );
        retain_root(temp.path());
        return;
    }
    assert!(all_selected, "preparation corpus requires all selected");
    let mut revision = selected.revision;
    let mut locators = Vec::<(String, RecoveryLocator, Option<RecoveryLocator>)>::new();
    for index in 0..count {
        let obligation = format!("review-{index}");
        let operation = format!("prepare-many-{index}");
        let prepared = observe("prepare_initial", index + 1, || {
            app.prepare_initial(&identity, &revision, &obligation, &operation)
        });
        let prepared = match prepared {
            Ok(Preparation::Applied(value)) => value,
            Err(error) if error.to_string().contains("capacity") => {
                let state = app.read(&identity).unwrap().unwrap();
                println!(
                    "{}",
                    json!({"event":"capacity_refusal","requested":count,
                    "completed":index,"error":error.to_string(),
                    "last_revision":state.revision.as_str(),
                    "state_json_bytes":serde_json::to_vec(&state).unwrap().len()})
                );
                retain_root(temp.path());
                return;
            }
            other => panic!(
                "unexpected preparation result: {}",
                other.err().map_or("non-applied".into(), |e| e.to_string())
            ),
        };
        let locator = RecoveryLocator {
            root_id: app.root_id().into(),
            anchored_root: app.anchored_root().into(),
            identity: identity.clone(),
            operation_id: operation,
            kind: "prepare_initial".into(),
            request_digest: prepared.receipt.request_digest.clone(),
            expected_revision: prepared.receipt.prior_revision.clone(),
            profile_digest: prepared.receipt.profile_digest.clone(),
        };
        let dispatch_locator = if dispatch {
            let recovered = app.recover_request(&locator, &obligation).unwrap();
            let handle = recovered.preparation_recovery.unwrap();
            let episode = episode_digest(&json!({"identity":identity,
                "obligationId":obligation}))
            .unwrap();
            let command = DispatchCommand {
                episode_id: episode[..32].into(),
                writer_actor: "native-review-host".into(),
                runtime_session: format!("session-{index}"),
                reviewer_profile: "direct-initial".into(),
                begin_transition_id: format!("begin-{index}"),
                result_transition_id: format!("result-{index}"),
            };
            let dispatch_operation = format!("dispatch-many-{index}");
            let effect = observe("authorize_dispatch", index + 1, || {
                app.authorize_recovered_dispatch(handle, &dispatch_operation, command)
                    .unwrap()
            });
            let DispatchEffect::Applied { permit, receipt } = effect else {
                panic!("expected new dispatch")
            };
            app.consume_dispatch_permit(permit).unwrap();
            Some(RecoveryLocator {
                root_id: app.root_id().into(),
                anchored_root: app.anchored_root().into(),
                identity: identity.clone(),
                operation_id: dispatch_operation,
                kind: "authorize_dispatch".into(),
                request_digest: receipt.request_digest.clone(),
                expected_revision: receipt.prior_revision.clone(),
                profile_digest: receipt.profile_digest.clone(),
            })
        } else {
            None
        };
        locators.push((obligation, locator, dispatch_locator));
        let milestone = index + 1;
        if focus_replay {
            revision = app.read(&identity).unwrap().unwrap().revision;
            continue;
        }
        if milestones.contains(&milestone) {
            let state = observe("current_read", milestone, || {
                app.read(&identity).unwrap().unwrap()
            });
            assert_eq!(state.progress.attempts.len(), milestone);
            revision = state.revision.clone();
            println!(
                "{}",
                json!({"event":"milestone","count":milestone,
                "state_json_bytes":serde_json::to_vec(&state).unwrap().len(),
                "elapsed_ns":start.elapsed().as_nanos()})
            );
            storage(&root, milestone);
            drop(app);
            app = observe("reopen", milestone, || Campaign::open(reopen()).unwrap());
            let first = observe("first_read_after_reopen", milestone, || {
                app.read(&identity).unwrap().unwrap()
            });
            assert_eq!(first.revision, revision);
            for _ in 0..warm {
                let _ = app.read(&identity).unwrap().unwrap();
            }
            for _ in 0..samples {
                let state = observe("repeated_current_read", milestone, || {
                    app.read(&identity).unwrap().unwrap()
                });
                assert_eq!(state.revision, revision);
            }
            for (label, idx) in [("earliest", 0), ("latest", locators.len() - 1)] {
                let (obligation, locator, dispatch_locator) = &locators[idx];
                for _ in 0..samples {
                    assert!(matches!(
                        observe(&format!("replay_prepare_{label}"), milestone, || app
                            .prepare_initial(
                                &identity,
                                locator.expected_revision.as_ref().unwrap(),
                                obligation,
                                &locator.operation_id
                            )
                            .unwrap()),
                        Preparation::Replayed { .. }
                    ));
                    assert!(matches!(
                        observe(&format!("reconcile_{label}"), milestone, || app
                            .reconcile_operation(locator)),
                        Reconcile::Committed(_)
                    ));
                    let recovered = observe(&format!("recover_{label}"), milestone, || {
                        app.recover_request(locator, obligation).unwrap()
                    });
                    assert!(
                        recovered.preparation_recovery.is_some() || recovered.dispatch.is_some()
                    );
                    if let Some(dispatch_locator) = dispatch_locator {
                        let (replayed, _) =
                            observe(&format!("replay_dispatch_{label}"), milestone, || {
                                app.replay_dispatch(dispatch_locator).unwrap()
                            });
                        assert!(replayed.may_have_entered);
                    }
                }
            }
        } else {
            revision = app.read(&identity).unwrap().unwrap().revision;
        }
    }
    if focus_replay {
        println!(
            "{}",
            json!({"event":"focus_constructed","count":count,
            "elapsed_ns":start.elapsed().as_nanos()})
        );
        storage(&root, count);
        drop(app);
        app = observe("focus_reopen", count, || Campaign::open(reopen()).unwrap());
        let (first_obligation, _, first_dispatch) = &locators[0];
        let (last_obligation, _, last_dispatch) = &locators[count - 1];
        let first_dispatch = first_dispatch.as_ref().unwrap();
        let last_dispatch = last_dispatch.as_ref().unwrap();
        println!(
            "{}",
            json!({"event":"focus_locators","first_obligation":first_obligation,
            "first_operation":first_dispatch.operation_id,
            "last_obligation":last_obligation,
            "last_operation":last_dispatch.operation_id,
            "root_id":app.root_id()})
        );
        let last_reference =
            serde_json::to_vec(&app.replay_dispatch(last_dispatch).unwrap()).unwrap();
        for _ in 1..warm {
            let replayed = app.replay_dispatch(last_dispatch).unwrap();
            assert_eq!(serde_json::to_vec(&replayed).unwrap(), last_reference);
        }
        for _ in 0..samples {
            let replayed = observe("focus_replay_dispatch_latest", count, || {
                app.replay_dispatch(last_dispatch).unwrap()
            });
            assert_eq!(serde_json::to_vec(&replayed).unwrap(), last_reference);
        }
        let first_reference =
            serde_json::to_vec(&app.replay_dispatch(first_dispatch).unwrap()).unwrap();
        for _ in 0..2 {
            let replayed = app.replay_dispatch(first_dispatch).unwrap();
            assert_eq!(serde_json::to_vec(&replayed).unwrap(), first_reference);
        }
        for _ in 0..10 {
            let replayed = observe("focus_replay_dispatch_earliest", count, || {
                app.replay_dispatch(first_dispatch).unwrap()
            });
            assert_eq!(serde_json::to_vec(&replayed).unwrap(), first_reference);
        }
        let current_reference = serde_json::to_vec(&app.read(&identity).unwrap().unwrap()).unwrap();
        println!(
            "{}",
            json!({"event":"focus_shapes","current_state_bytes":current_reference.len(),
            "earliest_replay_bytes":first_reference.len(),
            "latest_replay_bytes":last_reference.len()})
        );
        for _ in 0..2 {
            let state = app.read(&identity).unwrap().unwrap();
            assert_eq!(serde_json::to_vec(&state).unwrap(), current_reference);
        }
        for _ in 0..3 {
            let state = observe("focus_current_read", count, || {
                app.read(&identity).unwrap().unwrap()
            });
            assert_eq!(state.revision, revision);
            assert_eq!(serde_json::to_vec(&state).unwrap(), current_reference);
        }
        println!(
            "{}",
            json!({"event":"complete","count":count,
            "elapsed_ns":start.elapsed().as_nanos()})
        );
        retain_root(temp.path());
        return;
    }
    println!(
        "{}",
        json!({"event":"complete","count":count,
        "elapsed_ns":start.elapsed().as_nanos()})
    );
    retain_root(temp.path());
}
