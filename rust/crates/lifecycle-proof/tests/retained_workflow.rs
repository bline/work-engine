#![recursion_limit = "256"]

mod common;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use lifecycle_core::{
    BuildId, ClockSample, ContextGeneration, ProofRunId, SubjectId, WaitBudgetMs, WallTimeMs,
};
use lifecycle_proof::{
    host::{HostClient, HostReportedIdentity},
    identity::{HostProcess, observe_process},
    provider::{Reply, ScriptedProvider, Step},
    receipt::{
        ArtifactBinding, EvidenceBundle, TO2_RECEIPT_SCHEMA, to2_authority_from_environment,
        to2_lifecycle_projection,
    },
};
use lifecycle_service::task_effect_hold;
use lifecycle_store::SqliteLifecycleStore;
use retained_task_workflow::TaskStore;
use rusqlite::Connection;
use serde_json::{Value, json};

use common::{
    ANSWER_AUTHORITY, BUILD_ID, CHECKPOINT, Q1_QUESTION, Q2_QUESTION, SNAPSHOT_ID, TASK_ID,
    create_fixture, initial_submit, later_submit,
};

fn wait_observe(
    client: &HostClient,
    delivery_id: &str,
    budget: Duration,
    ready: impl Fn(&Value) -> bool,
) -> Value {
    let until = Instant::now() + budget;
    loop {
        let observed = client.observe(TASK_ID, Some(delivery_id)).unwrap();
        if ready(&observed) {
            return observed;
        }
        assert!(
            Instant::now() < until,
            "qualified host observation timed out: {observed}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

fn effect_store_counts(root: &std::path::Path) -> (u64, u64) {
    let script = "import sqlite3,sys; n=sqlite3.connect(sys.argv[1]); o=sqlite3.connect(sys.argv[2]); print(n.execute(\"SELECT count(*) FROM native_operations WHERE state='completed_settled' AND contract_id='workspace.snapshot.read.v1'\").fetchone()[0],o.execute('SELECT count(*) FROM completed_effects').fetchone()[0])";
    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(root.join("lifecycle-store/lifecycle.sqlite"))
        .arg(root.join("tasks.sqlite"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    let mut counts = text
        .split_whitespace()
        .map(|part| part.parse::<u64>().unwrap());
    (counts.next().unwrap(), counts.next().unwrap())
}

fn effect_store_rows(root: &std::path::Path) -> Value {
    let script = r#"import json,sqlite3,sys
def rows(path,query):
    db=sqlite3.connect(path); db.row_factory=sqlite3.Row
    return [{k:(v.hex() if isinstance(v,bytes) else v) for k,v in dict(row).items()}
            for row in db.execute(query)]
owner=sqlite3.connect(sys.argv[2]).execute('SELECT record_json FROM tasks').fetchone()
record=json.loads(owner[0]) if owner else None
print(json.dumps({'native':rows(sys.argv[1],"SELECT * FROM native_operations WHERE contract_id='workspace.snapshot.read.v1' ORDER BY operation_attempt_id"),
                  'owner':rows(sys.argv[2],"SELECT * FROM completed_effects ORDER BY effect_id"),
                  'owner_record':None if record is None else {
                      'task_id':record['task_id'],'subject_id':record['subject_id'],
                      'cursor_revision':record['cursor_revision'],
                      'owner_revision':record['revision'],
                      'completed_effects':record['completed_effects']}},sort_keys=True))"#;
    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(root.join("lifecycle-store/lifecycle.sqlite"))
        .arg(root.join("tasks.sqlite"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn native_exit_rows(root: &std::path::Path) -> Value {
    let script = "import json,sqlite3,sys; db=sqlite3.connect(sys.argv[1]); db.row_factory=sqlite3.Row; print(json.dumps([dict(row) for row in db.execute('SELECT session_id,exit_code,forced,outcome_kind FROM native_process_exits ORDER BY session_id')],sort_keys=True))";
    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(root.join("lifecycle-store/lifecycle.sqlite"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn real_owner_readback_qualification(root: &std::path::Path) {
    let lifecycle_root = root.join("lifecycle-store");
    let mut lifecycle = SqliteLifecycleStore::open(&lifecycle_root, "issuer:s8".into()).unwrap();
    let subject = SubjectId::parse(common::SUBJECT_ID).unwrap();
    let reads = lifecycle.committed_task_snapshot_reads(&subject).unwrap();
    assert_eq!(
        reads.len(),
        1,
        "one committed read in the real lifecycle store"
    );
    let native = &reads[0];
    let db = Connection::open(lifecycle_root.join("lifecycle.sqlite")).unwrap();
    let native_context: String = db.query_row(
        "SELECT s.context_id FROM native_operations o JOIN native_turns n ON n.invocation_id=o.parent_invocation_id JOIN native_sessions s ON s.session_id=n.session_id WHERE o.operation_attempt_id=?1",
        [&native.operation_attempt_id], |r| r.get(0)).unwrap();
    let current_context: String = db
        .query_row(
            "SELECT context_id FROM subjects WHERE subject_id=?1",
            [subject.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(native_context, "context:s8:g0");
    assert_eq!(current_context, "context:s8:g0");
    let owner_path = root.join("tasks.sqlite");
    let owner = TaskStore::open(&owner_path, common::ANSWER_AUTHORITY.into()).unwrap();
    assert!(
        owner
            .observe(common::TASK_ID)
            .unwrap()
            .unwrap()
            .result
            .is_none(),
        "the retained task remains unfinished"
    );
    assert_eq!(
        task_effect_hold(&lifecycle, &owner, &subject).unwrap(),
        None,
        "exact retained owner readback must release the committed read"
    );
    let effect = owner
        .lookup_completed_effect(&native.operation_attempt_id)
        .unwrap()
        .unwrap();
    assert_eq!(effect.delivery_id, native.delivery_id);
    assert_eq!(effect.receipt_digest, native.result_sha256);
    assert_eq!(effect.attribution.as_ref().unwrap().call_id, native.call_id);
    eprintln!(
        "S10_REAL_OWNER_READBACK {}",
        json!({
            "subject":subject.as_str(),"current_context":current_context,
            "read_context":native_context,"operation_attempt_id":native.operation_attempt_id,
            "delivery_id":native.delivery_id,"result_sha256":native.result_sha256,
            "exact_owner":true
        })
    );

    // This disposable state injection advances the current subject epoch
    // without running the semantic transition. It isolates the historical
    // read selector and the production hold/readback path, not transition
    // correctness or selected-host startup after replacement.
    db.execute(
        "UPDATE subjects SET context_id='context:s8:g1' WHERE subject_id=?1",
        [subject.as_str()],
    )
    .unwrap();
    let later_context: String = db
        .query_row(
            "SELECT context_id FROM subjects WHERE subject_id=?1",
            [subject.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(later_context, "context:s8:g1");
    assert_eq!(
        lifecycle.committed_task_snapshot_reads(&subject).unwrap(),
        reads,
        "subject selection must retain the earlier context's committed read"
    );
    assert_eq!(
        task_effect_hold(&lifecycle, &owner, &subject).unwrap(),
        None,
        "earlier epoch read still requires and has exact owner attribution"
    );
    eprintln!(
        "S10_REAL_OLD_EPOCH {}",
        json!({
            "subject":subject.as_str(),"read_context":native_context,
            "current_context":later_context,"unfinished_task":true,
            "selected_read":native.operation_attempt_id,"exact_owner":"clear",
            "qualification":"disposable current-context SQL advance; semantic transition not claimed"
        })
    );

    let other = SubjectId::parse("subject:s10:unrelated").unwrap();
    lifecycle
        .register_subject(
            other.clone(),
            ContextGeneration::parse("context:s10:unrelated").unwrap(),
            BuildId::parse("build:s10:unrelated").unwrap(),
            ProofRunId::parse("proof:s10:unrelated").unwrap(),
            ClockSample {
                wall: WallTimeMs::new(1),
                wait_budget: WaitBudgetMs::new(1),
            },
        )
        .unwrap();
    assert!(
        lifecycle
            .committed_task_snapshot_reads(&other)
            .unwrap()
            .is_empty()
    );
    assert_eq!(task_effect_hold(&lifecycle, &owner, &other).unwrap(), None);

    let owner_db = Connection::open(&owner_path).unwrap();
    let original: Vec<u8> = owner_db
        .query_row(
            "SELECT record_json FROM tasks WHERE task_id=?1",
            [common::TASK_ID],
            |r| r.get(0),
        )
        .unwrap();
    let mut record: Value = serde_json::from_slice(&original).unwrap();
    record["completed_effects"][0]["attribution"]["call_id"] =
        json!("call:s10:conflicting-durable");
    owner_db
        .execute(
            "UPDATE tasks SET record_json=?1 WHERE task_id=?2",
            (serde_json::to_vec(&record).unwrap(), common::TASK_ID),
        )
        .unwrap();
    assert!(
        task_effect_hold(&lifecycle, &owner, &subject)
            .unwrap()
            .is_some(),
        "durable owner tuple conflict must hold the older-epoch read"
    );
    owner_db
        .execute(
            "UPDATE tasks SET record_json=?1 WHERE task_id=?2",
            (&original, common::TASK_ID),
        )
        .unwrap();

    owner_db
        .execute(
            "DELETE FROM completed_effects WHERE effect_id=?1",
            [&native.operation_attempt_id],
        )
        .unwrap();
    assert_eq!(
        owner
            .lookup_completed_effect(&native.operation_attempt_id)
            .unwrap(),
        None
    );
    assert!(
        task_effect_hold(&lifecycle, &owner, &subject)
            .unwrap()
            .is_some(),
        "missing owner row must hold the selected task"
    );
    assert_eq!(
        task_effect_hold(&lifecycle, &owner, &other).unwrap(),
        None,
        "another registered subject must remain free of this task's hold"
    );
    owner_db
        .execute_batch("DROP TABLE completed_effects")
        .unwrap();
    assert!(
        owner
            .lookup_completed_effect(&native.operation_attempt_id)
            .is_err()
    );
    assert!(
        task_effect_hold(&lifecycle, &owner, &subject)
            .unwrap()
            .is_some(),
        "unavailable owner readback must hold, distinct from missing row"
    );
    eprintln!(
        "S10_REAL_OWNER_READBACK {}",
        json!({
            "subject":subject.as_str(),"unrelated_subject":other.as_str(),
            "missing_owner":"held","conflicting_durable_owner":"held",
            "unavailable_owner_readback":"held","unrelated_subject_result":"clear",
            "injection":"disposable owner SQLite record edit, row deletion, then table removal"
        })
    );
}

#[test]
fn selected_host_reopens_both_stores_at_snapshot_owner_cuts() {
    let provenance = common::preflight_installed_host();
    for (cut, expected_owner) in [
        ("native_after_result_commit", 0),
        ("native_callback_unavailable", 0),
        ("snapshot_before_owner_commit", 0),
        ("snapshot_after_owner_commit", 1),
        ("snapshot_after_owner_commit_shutdown", 1),
        ("completed_callback", 1),
    ] {
        let tmp = std::env::var_os("TMPDIR").expect("short TMPDIR required");
        let provider_root = tempfile::Builder::new()
            .prefix("s10-cut-provider-")
            .tempdir_in(tmp)
            .unwrap();
        let mut script = vec![Step {
            required: vec![SNAPSHOT_ID.into()],
            forbidden: Vec::new(),
            reply: Reply::ToolCall {
                call_id: "call:s8:read0".into(),
                namespace: "workspace".into(),
                name: "snapshot_read".into(),
                arguments: json!({"snapshot_id":SNAPSHOT_ID,"member_id":"m1","offset":0,"length":7}),
            },
            release_path: None,
        }];
        if cut == "completed_callback" {
            script.push(Step {
                required: vec![CHECKPOINT.into()],
                forbidden: Vec::new(),
                reply: Reply::Text(format!("Approved member contains {CHECKPOINT}.")),
                release_path: None,
            });
        }
        let provider =
            ScriptedProvider::start(script, provider_root.path().join("ledger.jsonl")).unwrap();
        let fixture = create_fixture(&provider.base_url(), provenance.clone());
        let barrier = fixture.root.path().join("barrier");
        let barrier_cut = if cut == "native_callback_unavailable" {
            "native_after_result_commit"
        } else if cut == "snapshot_after_owner_commit_shutdown" {
            "snapshot_after_owner_commit"
        } else {
            cut
        };
        if matches!(
            cut,
            "native_after_result_commit" | "native_callback_unavailable"
        ) {
            fs::remove_file(barrier.join(format!("{barrier_cut}.release"))).unwrap();
        } else if cut != "completed_callback" {
            fs::write(barrier.join(format!("{barrier_cut}.arm")), b"armed\n").unwrap();
        }
        let selection_path = fixture.root.path().join("selection.json");
        let selection = json!({
            "version":1,"selection_id":format!("selected-s10-{cut}"),
            "executable_path":fixture.host_executable,
            "executable_sha256":fixture.build_provenance.executable_sha256,
            "config_path":fixture.startup_path,
            "config_sha256":lifecycle_proof::identity::sha256_file(&fixture.startup_path).unwrap(),
            "task_store_path":fixture.root.path().join("tasks.sqlite"),
            "lifecycle_store_root":fixture.root.path().join("lifecycle-store"),
            "host_socket_path":fixture.host_socket,
            "lifecycle_socket_path":fixture.root.path().join("service.sock"),
            "build_id":BUILD_ID,"features":["controlled-proof"],
            "provider_identity":"native-sim-proof-v1"
        });
        fs::write(&selection_path, serde_json::to_vec(&selection).unwrap()).unwrap();
        fs::set_permissions(&selection_path, fs::Permissions::from_mode(0o600)).unwrap();
        let host = HostProcess::launch_selected(&fixture.host_executable, &selection_path).unwrap();
        let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
        client.await_ready(Duration::from_secs(10)).unwrap();
        let submitted = client.submit(initial_submit()).unwrap();
        assert_eq!(submitted["status"], "admitted", "{submitted}");
        let delivery = submitted["lifecycle"]["outcome_ref"]
            .as_str()
            .unwrap()
            .to_owned();
        if cut == "completed_callback" {
            let settled = wait_observe(&client, &delivery, Duration::from_secs(30), |value| {
                value["components"]["owner"]["value"]["completed_effects"]
                    .as_array()
                    .is_some_and(|rows| rows.len() == 1)
                    && value["components"]["native_delivery_status"]["value"]["segments"]
                        .as_array()
                        .is_some_and(|rows| {
                            rows.iter()
                                .any(|row| row["native_state"] == "completed_settled")
                        })
            });
            assert!(
                settled["components"]["native_delivery_status"]["value"]["task_effect_hold"]
                    .is_null(),
                "{settled}"
            );
            assert_eq!(effect_store_counts(fixture.root.path()), (1, 1));
            assert_eq!(provider.entry_count(), 2);
            eprintln!(
                "S10_CUT_EVIDENCE {}",
                json!({
                    "cut":cut,"profile":fixture.startup["profile"],
                    "selection_sha256":lifecycle_proof::identity::sha256_file(&selection_path).unwrap(),
                    "startup_receipt_sha256":lifecycle_proof::identity::sha256_file(&fixture.root.path().join("startup-receipt.json")).unwrap(),
                    "source_manifest_sha256":fixture.source_manifest.sha256,
                    "binary_sha256":fixture.build_provenance.executable_sha256,
                    "after_counts":{"native":1,"owner":1},"provider_entries":2,
                    "native_state":"completed_settled","hold":null,
                    "store_rows":effect_store_rows(fixture.root.path())
                })
            );
            assert!(
                host.terminate_and_reap(Duration::from_secs(15))
                    .unwrap()
                    .reaped
            );
            provider.finish().unwrap();
            real_owner_readback_qualification(fixture.root.path());
            continue;
        }
        let until = Instant::now() + Duration::from_secs(20);
        while !barrier.join(format!("{barrier_cut}.reached")).exists() {
            assert!(Instant::now() < until, "selected host did not reach {cut}");
            thread::sleep(Duration::from_millis(10));
        }
        let owner_transaction_open = cut == "snapshot_before_owner_commit";
        let at_cut = if owner_transaction_open {
            None // An independent reader cannot inspect the in-flight owner transaction.
        } else {
            let counts = effect_store_counts(fixture.root.path());
            assert_eq!(counts, (1, expected_owner), "{cut} counts at cut");
            Some(counts)
        };
        if cut == "native_callback_unavailable" {
            // The required proof barrier times out after native commit. The
            // actor resumes with an unresolved attribution hold in this same
            // selected process, where recovery_only has never been set.
            thread::sleep(Duration::from_secs(16));
            let observed = client.observe(TASK_ID, Some(&delivery)).unwrap();
            let native = &observed["components"]["native_delivery_status"]["value"];
            assert_eq!(native["recovery_only"], false, "{observed}");
            assert!(native["task_effect_hold"].as_str().is_some(), "{observed}");
            let next = client
                .submit(later_submit(
                    "command:s10:callback-unavailable-next",
                    "Must remain held after callback failure",
                ))
                .unwrap();
            assert_eq!(
                next["lifecycle"]["code"], "observation_unavailable",
                "{next}"
            );
            let replacement = client
                .request_replacement(json!({
                    "task_id":TASK_ID,"command_id":"command:s10:callback-unavailable-replace",
                    "expected_task_revision":observed["components"]["owner"]["value"]["revision"],
                    "authority_ref":ANSWER_AUTHORITY,"reason":"Hold must block replacement"
                }))
                .unwrap();
            assert_eq!(replacement["status"], "error", "{replacement}");
            assert_eq!(
                replacement["code"], "invalid_or_unavailable",
                "{replacement}"
            );
            assert_eq!(effect_store_counts(fixture.root.path()), (1, 0));
            assert_eq!(provider.entry_count(), 1);
            eprintln!(
                "S10_CUT_EVIDENCE {}",
                json!({
                    "cut":cut,"profile":fixture.startup["profile"],
                    "selection_sha256":lifecycle_proof::identity::sha256_file(&selection_path).unwrap(),
                    "source_manifest_sha256":fixture.source_manifest.sha256,
                    "binary_sha256":fixture.build_provenance.executable_sha256,
                    "after_callback_counts":{"native":1,"owner":0},
                    "provider_entries":1,"recovery_only":native["recovery_only"],
                    "task_effect_hold":native["task_effect_hold"],
                    "store_rows":effect_store_rows(fixture.root.path()),
                    "new_submit_code":next["lifecycle"]["code"],
                    "replacement_code":replacement["code"]
                })
            );
            assert!(
                host.terminate_and_reap(Duration::from_secs(15))
                    .unwrap()
                    .reaped
            );
            let exits = native_exit_rows(fixture.root.path());
            assert_eq!(exits.as_array().unwrap().len(), 1, "{exits}");
            assert_eq!(effect_store_counts(fixture.root.path()), (1, 0));
            assert_eq!(provider.entry_count(), 1);
            eprintln!(
                "S10_HELD_CLEANUP_EVIDENCE {}",
                json!({
                    "cut":cut,"native_exits":exits,"after_cleanup_counts":{"native":1,"owner":0},
                    "provider_entries":provider.entry_count(),
                    "store_rows":effect_store_rows(fixture.root.path())
                })
            );
            provider.finish().unwrap();
            continue;
        }
        if cut == "snapshot_after_owner_commit_shutdown" {
            let mut host = host;
            host.signal_terminate().unwrap();
            let shutdown_marker = barrier.join("shutdown_gate_closed.reached");
            let until = Instant::now() + Duration::from_secs(10);
            while !shutdown_marker.exists() {
                assert!(
                    Instant::now() < until,
                    "shutdown gate did not close before owner callback release"
                );
                thread::sleep(Duration::from_millis(10));
            }
            fs::write(barrier.join(format!("{barrier_cut}.release")), b"release\n").unwrap();
            let close = host
                .wait_after_terminate_and_reap(Duration::from_secs(15))
                .unwrap();
            assert!(close.reaped);
            assert_eq!(effect_store_counts(fixture.root.path()), (1, 1));
            assert_eq!(
                provider.entry_count(),
                1,
                "native/provider advanced after shutdown"
            );
            eprintln!(
                "S10_CUT_EVIDENCE {}",
                json!({
                    "cut":cut,"profile":fixture.startup["profile"],
                    "selection_sha256":lifecycle_proof::identity::sha256_file(&selection_path).unwrap(),
                    "source_manifest_sha256":fixture.source_manifest.sha256,
                    "binary_sha256":fixture.build_provenance.executable_sha256,
                    "at_cut_counts":{"native":1,"owner":1},
                    "after_close_counts":{"native":1,"owner":1},
                    "provider_entries":1,"signal":"SIGTERM","reaped":close.reaped,
                    "store_rows":effect_store_rows(fixture.root.path())
                })
            );
            drop(client);
            provider.finish().unwrap();
            continue;
        }
        let close = host.kill_and_reap().unwrap();
        assert!(close.reaped);
        drop(client);
        let reopened =
            HostProcess::launch_selected(&fixture.host_executable, &selection_path).unwrap();
        let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
        client.await_ready(Duration::from_secs(10)).unwrap();
        let observed = client.observe(TASK_ID, Some(&delivery)).unwrap();
        let native = &observed["components"]["native_delivery_status"]["value"];
        assert_eq!(native["recovery_only"], true, "{cut}: {observed}");
        let hold = native["task_effect_hold"].as_str();
        if expected_owner == 0 {
            assert!(
                hold.is_some_and(
                    |value| value.starts_with("committed-native/owner-attribution-unresolved")
                ),
                "{cut}: {observed}"
            );
        } else {
            assert!(hold.is_none(), "{cut}: {observed}");
        }
        let next = client
            .submit(later_submit(
                &format!("command:s10:held-next:{cut}"),
                "Queued work cannot start during uncertain recovery",
            ))
            .unwrap();
        assert_eq!(next["code"], "recovery_only", "{cut}: {next}");
        let replacement = client.request_replacement(json!({
            "task_id":TASK_ID,"command_id":format!("command:s10:held-replace:{cut}"),
            "expected_task_revision":observed["components"]["owner"]["value"]["revision"],
            "authority_ref":ANSWER_AUTHORITY,"reason":"No replacement while recovery is uncertain"
        })).unwrap();
        assert_eq!(replacement["code"], "recovery_only", "{cut}: {replacement}");
        assert_eq!(
            effect_store_counts(fixture.root.path()),
            at_cut.unwrap_or((1, 0)),
            "{cut} mutated on restart"
        );
        assert_eq!(
            provider.entry_count(),
            1,
            "{cut} resent or advanced provider turn"
        );
        eprintln!(
            "S10_CUT_EVIDENCE {}",
            json!({
                "cut":cut,"profile":fixture.startup["profile"],
                "selection_sha256":lifecycle_proof::identity::sha256_file(&selection_path).unwrap(),
                "startup_receipt_sha256":lifecycle_proof::identity::sha256_file(&fixture.root.path().join("startup-receipt.json")).unwrap(),
                "source_manifest_sha256":fixture.source_manifest.sha256,
                "binary_sha256":fixture.build_provenance.executable_sha256,
                "at_cut_counts":at_cut.map(|(native,owner)| json!({"native":native,"owner":owner})),
                "after_reopen_counts":{"native":1,"owner":expected_owner},
                "provider_entries":1,"recovery_only":native["recovery_only"],
                "task_effect_hold":hold,"owner_transaction_open":owner_transaction_open,
                "new_submit_code":next["code"],"replacement_code":replacement["code"],
                "store_rows":effect_store_rows(fixture.root.path())
            })
        );
        assert!(
            reopened
                .terminate_and_reap(Duration::from_secs(15))
                .unwrap()
                .reaped
        );
        let _ = provider.finish().unwrap();
    }
}

#[test]
fn installed_retained_workflow_vertical() {
    let to2_authority = to2_authority_from_environment("s8-installed-d0-snapshot-child").unwrap();
    let tmp = std::env::var_os("TMPDIR").expect("TMPDIR must be pinned for S8 proof");
    let build_provenance = common::preflight_installed_host();
    let provider_root = tempfile::Builder::new()
        .prefix("s8-provider-")
        .tempdir_in(tmp)
        .unwrap();
    let two_cycles = std::env::var_os("S8_TWO_CYCLES").is_some();
    let omit_receipt = std::env::var_os("S10_C2_OMIT_RECEIPT").is_some();
    let transition_a = two_cycles || std::env::var_os("S8_TRANSITION_A").is_some();
    let design_inquiry = std::env::var_os("S8_DESIGN_INQUIRY").is_some();
    let hold_q1 = std::env::var_os("S8_HOLD_Q1").is_some();
    let schema7 = std::env::var_os("S10_C2_SCHEMA7").is_some();
    let prelude_entries = usize::from(schema7);
    assert!(!schema7 || transition_a);
    assert!(!design_inquiry || !transition_a);
    assert!(!hold_q1 || two_cycles);
    assert!(!omit_receipt || (transition_a && !two_cycles));
    let final_release = provider_root.path().join("release-d0-final");
    let d0_g1_release = provider_root.path().join("release-d0-g1-final");
    let q1_release = provider_root.path().join("release-q1-final");
    let mut script = vec![
        Step {
            required: vec![SNAPSHOT_ID.into()],
            forbidden: Vec::new(),
            reply: Reply::ToolCall {
                call_id: "call:s8:read0".into(),
                namespace: "workspace".into(),
                name: "snapshot_read".into(),
                arguments: json!({"snapshot_id":SNAPSHOT_ID,"member_id":"m1","offset":0,"length":7}),
            },
            release_path: None,
        },
        Step {
            required: vec![CHECKPOINT.into()],
            forbidden: Vec::new(),
            reply: Reply::Text(format!(
                "Approved snapshot member m1 contains {CHECKPOINT}. Repository mutation approval remains pending."
            )),
            release_path: transition_a.then(|| final_release.clone()),
        },
    ];
    if transition_a {
        script.push(Step {
            required: vec!["compile_attributed_continuation".into()],
            forbidden: Vec::new(),
            reply: Reply::SemanticCompile,
            release_path: None,
        });
        script.push(Step {
            required: vec!["verify_candidate_semantics".into()],
            forbidden: Vec::new(),
            reply: Reply::SemanticVerify,
            release_path: None,
        });
        script.push(Step {
            required: vec!["Choose the next authorized analysis action".into()],
            forbidden: Vec::new(),
            reply: if omit_receipt {
                Reply::SuccessorUseOmitReceipt
            } else {
                Reply::SuccessorUse
            },
            release_path: None,
        });
        script.push(Step {
            required: vec!["verify_successor_use_of_continuation".into()],
            forbidden: Vec::new(),
            reply: Reply::SemanticSuccessorVerify,
            release_path: None,
        });
        script.push(Step {
            required: vec!["ORION-7".into()],
            forbidden: Vec::new(),
            reply: Reply::Text(format!(
                "The approved snapshot member m1 contains {CHECKPOINT}. Its earlier read receipt is retained; repository mutation approval remains pending. Continue attributed analysis without repeating the completed snapshot read."
            )),
            release_path: two_cycles.then(|| d0_g1_release.clone()),
        });
    }
    if two_cycles {
        script.extend([
            Step {
                required: vec!["compile_attributed_continuation".into()],
                forbidden: Vec::new(),
                reply: Reply::SemanticCompile,
                release_path: None,
            },
            Step {
                required: vec!["verify_candidate_semantics".into()],
                forbidden: Vec::new(),
                reply: Reply::SemanticVerify,
                release_path: None,
            },
            Step {
                required: vec!["Choose the next authorized analysis action for queued Q1".into()],
                forbidden: Vec::new(),
                reply: Reply::SuccessorUse,
                release_path: None,
            },
            Step {
                required: vec!["verify_successor_use_of_continuation".into()],
                forbidden: Vec::new(),
                reply: Reply::SemanticSuccessorVerify,
                release_path: None,
            },
            Step {
                required: vec!["input:command:s8:d0".into()],
                forbidden: Vec::new(),
                reply: Reply::Text(format!(
                    "D0 continues in the fresh second successor context: the approved member m1 contains {CHECKPOINT}; the completed snapshot read is retained once, and repository mutation approval remains pending."
                )),
                release_path: None,
            },
            Step {
                required: vec![Q1_QUESTION.into()],
                forbidden: Vec::new(),
                reply: Reply::Text(format!(
                    "Q1 retains the governing rule: use only approved snapshot member m1, which contained {CHECKPOINT}. Repository mutation approval remains pending."
                )),
                release_path: hold_q1.then(|| q1_release.clone()),
            },
            Step {
                required: vec![Q2_QUESTION.into()],
                forbidden: Vec::new(),
                reply: Reply::Text(format!(
                    "Q2 attributed result: approved member m1 contains {CHECKPOINT}; the earlier snapshot read remains completed and repository mutation approval remains pending."
                )),
                release_path: None,
            },
        ]);
    }
    if omit_receipt {
        script.truncate(6);
    }
    if schema7 {
        script.insert(
            0,
            Step {
                required: vec!["command:s10:prelude".into()],
                forbidden: Vec::new(),
                reply: Reply::Text(
                    "The task remains unfinished; its approved snapshot has not been read.".into(),
                ),
                release_path: None,
            },
        );
    }
    let provider =
        ScriptedProvider::start(script, provider_root.path().join("provider-ledger.jsonl"))
            .unwrap();
    let mut fixture = create_fixture(&provider.base_url(), build_provenance);
    let mut host = if schema7 {
        let selected_v1 = fixture.root.path().join("selection-v1.json");
        let selection = json!({
            "version":1,"selection_id":"selected-s10-c2-prelude",
            "executable_path":fixture.host_executable,
            "executable_sha256":fixture.build_provenance.executable_sha256,
            "config_path":fixture.startup_path,
            "config_sha256":lifecycle_proof::identity::sha256_file(&fixture.startup_path).unwrap(),
            "task_store_path":fixture.root.path().join("tasks.sqlite"),
            "lifecycle_store_root":fixture.root.path().join("lifecycle-store"),
            "host_socket_path":fixture.host_socket,
            "lifecycle_socket_path":fixture.root.path().join("service.sock"),
            "build_id":BUILD_ID,"features":["controlled-proof"],
            "provider_identity":"native-sim-proof-v1"
        });
        fs::write(&selected_v1, serde_json::to_vec(&selection).unwrap()).unwrap();
        fs::set_permissions(&selected_v1, fs::Permissions::from_mode(0o600)).unwrap();
        let predecessor =
            HostProcess::launch_selected(&fixture.host_executable, &selected_v1).unwrap();
        let predecessor_identity = predecessor.identity().clone();
        let predecessor_client =
            HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
        predecessor_client
            .await_ready(Duration::from_secs(10))
            .unwrap();
        let mut prelude = initial_submit();
        prelude["command_id"] = json!("command:s10:prelude");
        let admitted = predecessor_client.submit(prelude).unwrap();
        assert_eq!(admitted["status"], "admitted", "{admitted}");
        let prelude_delivery = admitted["lifecycle"]["outcome_ref"].as_str().unwrap();
        wait_observe(
            &predecessor_client,
            prelude_delivery,
            Duration::from_secs(30),
            |reply| {
                reply["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"]
                    == "completed"
            },
        );
        assert_eq!(provider.entry_count(), 1);
        assert!(
            predecessor
                .terminate_and_reap(Duration::from_secs(15))
                .unwrap()
                .reaped
        );
        let owner_path = fixture.root.path().join("tasks.sqlite");
        let owner = TaskStore::open(&owner_path, ANSWER_AUTHORITY.into()).unwrap();
        let owner_record = owner.observe_subject(common::SUBJECT_ID).unwrap().unwrap();
        let mut lifecycle = SqliteLifecycleStore::open(
            &fixture.root.path().join("lifecycle-store"),
            "issuer:s8".into(),
        )
        .unwrap();
        let basis = lifecycle
            .native_restart_basis(
                &SubjectId::parse(common::SUBJECT_ID).unwrap(),
                &ContextGeneration::parse("context:s8:g0").unwrap(),
            )
            .unwrap();
        drop(lifecycle);
        drop(owner);
        let prior_receipt = fixture.root.path().join("startup-receipt.json");
        let receipt: Value = serde_json::from_slice(&fs::read(&prior_receipt).unwrap()).unwrap();
        let restart_config = fixture.root.path().join("host-restart.json");
        let mut startup = fixture.startup.clone();
        startup["lifecycle"]["native"]["session_id"] = json!("session:s10:c2:restart:g0");
        startup["lifecycle"]["native"]["root"] = json!(fixture.root.path().join("native-restart"));
        startup["lifecycle"]["native"]["invocation_id"] = json!("invocation:s10:c2:restart:g0");
        startup["lifecycle"]["native"]["attempt_id"] = json!("attempt:s10:c2:restart:g0");
        fs::write(&restart_config, serde_json::to_vec(&startup).unwrap()).unwrap();
        fs::set_permissions(&restart_config, fs::Permissions::from_mode(0o600)).unwrap();
        let restart_selection = fixture.root.path().join("selection-v3.json");
        let declaration_path = fixture.root.path().join("restart-declaration.json");
        let mut selection = selection;
        selection["version"] = json!(3);
        selection["selection_id"] = json!("selected-s10-c2-restart");
        selection["predecessor_selection_path"] = json!(selected_v1);
        selection["restart_declaration_path"] = json!(declaration_path);
        selection["config_path"] = json!(restart_config);
        selection["config_sha256"] =
            json!(lifecycle_proof::identity::sha256_file(&restart_config).unwrap());
        fs::write(&restart_selection, serde_json::to_vec(&selection).unwrap()).unwrap();
        fs::set_permissions(&restart_selection, fs::Permissions::from_mode(0o600)).unwrap();
        let declaration = json!({
            "version":1,
            "selection_sha256":lifecycle_proof::identity::sha256_file(&restart_selection).unwrap(),
            "predecessor_selection_sha256":lifecycle_proof::identity::sha256_file(&selected_v1).unwrap(),
            "predecessor_receipt_sha256":lifecycle_proof::identity::sha256_file(&prior_receipt).unwrap(),
            "task":receipt["task"],"lifecycle":receipt["lifecycle"],
            "subject_id":common::SUBJECT_ID,"context_generation":"context:s8:g0",
            "task_id":owner_record.task_id,"task_revision":owner_record.revision,
            "task_cursor_revision":owner_record.cursor_revision,
            "predecessor_session_id":basis.predecessor_session_id,"predecessor_epoch":basis.predecessor_epoch,
            "target_session_id":"session:s10:c2:restart:g0",
            "expected_subject_revision":basis.subject_revision,
            "expected_semantic_revision":basis.semantic_revision,
            "expected_queue_revision":basis.queue_revision,
            "expected_journal_cursor":basis.journal_cursor,
            "profile_digest":basis.profile_digest,"binary_sha256":basis.binary_sha256,
            "protocol_digest":basis.protocol_digest,"snapshot_id":basis.snapshot_id,
            "snapshot_manifest_sha256":basis.snapshot_manifest_sha256,
            "operator_ref":"operator:s10:c2:disposable-proof","cooperative_stop_asserted":true,
            "predecessor_host_pid":predecessor_identity.pid,
            "predecessor_host_starttime_ticks":predecessor_identity.linux_starttime_ticks.to_string()
        });
        fs::write(&declaration_path, serde_json::to_vec(&declaration).unwrap()).unwrap();
        fs::set_permissions(&declaration_path, fs::Permissions::from_mode(0o600)).unwrap();
        fixture.startup = startup;
        fixture.startup_path = restart_config;
        let restarted =
            HostProcess::launch_selected(&fixture.host_executable, &restart_selection).unwrap();
        let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
        client.await_ready(Duration::from_secs(10)).unwrap();
        let db =
            Connection::open(fixture.root.path().join("lifecycle-store/lifecycle.sqlite")).unwrap();
        let schema: i64 = db
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(schema, 7);
        drop(db);
        restarted
    } else {
        HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap()
    };
    let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    host.assert_same_execution().unwrap();
    let mut submit = if schema7 {
        later_submit(
            "command:s8:d0",
            "Which approved marker appears in member m1, and what remains pending?",
        )
    } else {
        initial_submit()
    };
    if design_inquiry {
        submit["kind"] = json!("design_inquiry");
    }
    let admitted = client.submit(submit).unwrap();
    assert_eq!(admitted["status"], "admitted", "{admitted}");
    assert_eq!(admitted["lifecycle"]["accepted"], true, "{admitted}");
    assert_eq!(
        admitted["lifecycle"]["outcome_kind"], "enqueued",
        "{admitted}"
    );
    let delivery_id = admitted["lifecycle"]["outcome_ref"]
        .as_str()
        .expect("admission omitted logical delivery ID");
    let reported = HostReportedIdentity::from_reply(&admitted).unwrap();
    reported.bind_process(host.identity(), BUILD_ID).unwrap();
    reported.bind_startup(&fixture.startup).unwrap();
    let mut host_observations = vec![admitted.clone()];
    let mut observed = if transition_a {
        let until = Instant::now() + Duration::from_secs(30);
        while provider.entry_count() < 2 + prelude_entries {
            assert!(
                Instant::now() < until,
                "D0 final response did not enter provider"
            );
            thread::sleep(Duration::from_millis(20));
        }
        let before = client.observe(TASK_ID, Some(delivery_id)).unwrap();
        host_observations.push(before.clone());
        eprintln!(
            "before pressure: delivery={}, transition={}",
            before["components"]["lifecycle"]["value"]["delivery"],
            before["components"]["lifecycle"]["value"]["transition"]
        );
        let revision = before["components"]["owner"]["value"]["revision"]
            .as_u64()
            .unwrap();
        let wrong_authority = client
            .request_replacement(json!({
                "task_id":TASK_ID,"command_id":"command:s8:replace-wrong-authority",
                "expected_task_revision":revision,"authority_ref":"operator:s8:untrusted",
                "reason":"Unauthorized replacement probe"
            }))
            .unwrap();
        assert_eq!(wrong_authority["status"], "error", "{wrong_authority}");
        assert_eq!(wrong_authority["code"], "unauthorized");
        let after_refusal = client.observe(TASK_ID, Some(delivery_id)).unwrap();
        assert_eq!(
            after_refusal["components"]["lifecycle"]["value"]["transition"]["state"],
            "unknown"
        );
        host_observations.push(after_refusal);
        let replacement_command = json!({
            "task_id":TASK_ID,"command_id":"command:s8:replace-a",
            "expected_task_revision":revision,"authority_ref":ANSWER_AUTHORITY,
            "reason":"Context replacement A after the approved snapshot read"
        });
        let replacement = client
            .request_replacement(replacement_command.clone())
            .unwrap();
        eprintln!("replacement reply: {replacement}");
        assert_eq!(
            replacement["status"], "replacement_requested",
            "{replacement}"
        );
        host_observations.push(replacement.clone());
        let duplicate = client.request_replacement(replacement_command).unwrap();
        assert_eq!(duplicate["status"], "replacement_requested", "{duplicate}");
        assert_eq!(
            duplicate["lifecycle"]["outcome_ref"],
            replacement["lifecycle"]["outcome_ref"]
        );
        assert_eq!(
            duplicate["lifecycle"]["resulting_revision"],
            replacement["lifecycle"]["resulting_revision"]
        );
        host_observations.push(duplicate);
        fs::write(&final_release, b"release\n").unwrap();
        if omit_receipt {
            let until = Instant::now() + Duration::from_secs(30);
            while provider.response_count() < 6 + prelude_entries {
                assert!(Instant::now() < until, "omission verifier did not respond");
                thread::sleep(Duration::from_millis(20));
            }
            let store_path = fixture.root.path().join("lifecycle-store/lifecycle.sqlite");
            let until = Instant::now() + Duration::from_secs(20);
            let verdict = loop {
                let connection = Connection::open(&store_path).unwrap();
                let verdict: Option<String> = connection.query_row(
                    "SELECT verdict FROM semantic_results WHERE phase='successor_adequacy' ORDER BY created_wall_ms DESC LIMIT 1",
                    [], |row| row.get(0),
                ).ok();
                if let Some(verdict) = verdict {
                    break verdict;
                }
                assert!(Instant::now() < until, "omission verdict absent");
                thread::sleep(Duration::from_millis(20));
            };
            assert_eq!(verdict, "rejected");
            let rejected = client.observe(TASK_ID, Some(delivery_id)).unwrap();
            host_observations.push(rejected.clone());
            assert_eq!(provider.entry_count(), 6 + prelude_entries);
            assert_eq!(effect_store_counts(fixture.root.path()), (1, 1));
            assert_eq!(
                rejected["components"]["owner"]["value"]["completed_effects"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            let host_pid = host.identity().pid;
            assert!(
                host.terminate_and_reap(Duration::from_secs(15))
                    .unwrap()
                    .reaped
            );
            let entries = provider.finish().unwrap();
            if let Some(destination) = std::env::var_os("S10_C2_EVIDENCE_DIR") {
                let destination = std::path::PathBuf::from(destination);
                fs::create_dir_all(&destination).unwrap();
                let case = destination.join(format!("omission-{host_pid}"));
                fs::create_dir(&case).unwrap();
                fs::write(
                    case.join("host-observations.json"),
                    serde_json::to_vec_pretty(&host_observations).unwrap(),
                )
                .unwrap();
                fs::write(
                    case.join("provider-entries.json"),
                    serde_json::to_vec_pretty(&entries).unwrap(),
                )
                .unwrap();
                fs::copy(&fixture.startup_path, case.join("host-startup.json")).unwrap();
                if schema7 {
                    for name in [
                        "selection-v1.json",
                        "selection-v3.json",
                        "restart-declaration.json",
                        "startup-receipt.json",
                    ] {
                        fs::copy(fixture.root.path().join(name), case.join(name)).unwrap();
                    }
                }
                fs::copy(
                    &fixture.source_manifest.path,
                    case.join("source-manifest.json"),
                )
                .unwrap();
                fs::copy(
                    fixture.root.path().join("tasks.sqlite"),
                    case.join("tasks.sqlite"),
                )
                .unwrap();
                fs::copy(
                    fixture.root.path().join("lifecycle-store/lifecycle.sqlite"),
                    case.join("lifecycle.sqlite"),
                )
                .unwrap();
                eprintln!("S10_C2_EVIDENCE {}", case.display());
            }
            return;
        }
        let until = Instant::now() + Duration::from_secs(30);
        while provider.entry_count() < 7 + prelude_entries {
            if Instant::now() >= until {
                let readback = client.observe(TASK_ID, Some(delivery_id)).unwrap();
                let lifecycle = &readback["components"]["lifecycle"]["value"];
                let root = fixture.root.path().to_path_buf();
                let provider_path = provider_root.path().to_path_buf();
                let provider_outcome = format!("{:?}", provider.finish());
                std::mem::forget(fixture);
                std::mem::forget(provider_root);
                panic!(
                    "transition control absent; root={}, provider_root={}, provider_outcome={}, stage={}, delivery={}, blocking={}, native_status={}",
                    root.display(),
                    provider_path.display(),
                    provider_outcome,
                    lifecycle["transition"],
                    lifecycle["delivery"],
                    lifecycle["blocking_reason"],
                    readback["components"]["native_delivery_status"]["value"]
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
        let until = Instant::now() + Duration::from_secs(45);
        loop {
            let current = client.observe(TASK_ID, Some(delivery_id)).unwrap();
            let segments = current["components"]["native_delivery_status"]["value"]["segments"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter(|item| item["delivery_id"] == delivery_id)
                        .collect::<Vec<_>>()
                });
            if current["components"]["lifecycle"]["value"]["transition"]["value"]["completed"]
                == true
                && current["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"]
                    == if two_cycles { "pending" } else { "completed" }
                && segments.as_ref().is_some_and(|items| {
                    items.len() == 2
                        && if two_cycles {
                            items[1]["send_intent"] == true && items[1]["final_sha256"].is_null()
                        } else {
                            items[1]["native_state"] == "completed_settled"
                        }
                })
            {
                let segments = segments.unwrap();
                assert_eq!(segments[0]["segment_no"], 0);
                assert_eq!(segments[0]["segment_state"], "yielded");
                assert_eq!(segments[0]["history_status"], "observed_tool_omitted");
                assert_eq!(segments[1]["segment_no"], 1);
                assert_eq!(segments[1]["context_generation"], "context:s8:g1");
                assert_eq!(segments[1]["delivery_id"], delivery_id);
                assert_eq!(
                    segments[1]["predecessor_invocation_id"],
                    segments[0]["invocation_id"]
                );
                assert_ne!(
                    segments[1]["native_attempt_id"],
                    segments[0]["native_attempt_id"]
                );
                assert_eq!(
                    current["components"]["owner"]["value"]["completed_effects"]
                        .as_array()
                        .unwrap()
                        .len(),
                    1
                );
                break current;
            }
            if Instant::now() >= until {
                let root = fixture.root.path().to_path_buf();
                let provider_path = provider_root.path().to_path_buf();
                let provider_outcome = format!("{:?}", provider.finish());
                let stage = current["components"]["lifecycle"]["value"]["transition"].clone();
                let segments = current["components"]["native_delivery_status"]["value"].clone();
                std::mem::forget(fixture);
                std::mem::forget(provider_root);
                panic!(
                    "transition A failed to settle; root={}, provider_root={}, provider_outcome={}, stage={}, segments={}",
                    root.display(),
                    provider_path.display(),
                    provider_outcome,
                    stage,
                    segments
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    } else {
        wait_observe(&client, delivery_id, Duration::from_secs(30), |value| {
            value["components"]["owner"]["value"]["completed_effects"]
                .as_array()
                .is_some_and(|effects| {
                    effects
                        .iter()
                        .any(|effect| effect["delivery_id"] == delivery_id)
                })
        })
    };
    assert_eq!(observed["status"], "observation", "{observed}");
    host_observations.push(observed.clone());
    if two_cycles {
        let q1 = client
            .submit(later_submit("command:s8:q1", Q1_QUESTION))
            .unwrap();
        assert_eq!(q1["status"], "admitted", "{q1}");
        let q1_delivery = q1["lifecycle"]["outcome_ref"].as_str().unwrap().to_owned();
        assert_ne!(q1_delivery, delivery_id);
        host_observations.push(q1);
        let q2 = client
            .submit(later_submit("command:s8:q2", Q2_QUESTION))
            .unwrap();
        assert_eq!(q2["status"], "admitted", "{q2}");
        let q2_delivery = q2["lifecycle"]["outcome_ref"].as_str().unwrap().to_owned();
        assert_ne!(q2_delivery, q1_delivery);
        host_observations.push(q2);
        let q2_queued = client.observe(TASK_ID, Some(&q2_delivery)).unwrap();
        assert!(
            q2_queued["components"]["native_delivery_status"]["value"]["segments"]
                .as_array()
                .unwrap()
                .iter()
                .all(|segment| {
                    segment["delivery_id"] != q1_delivery && segment["delivery_id"] != q2_delivery
                })
        );
        assert_eq!(
            provider.entry_count(),
            7 + prelude_entries,
            "queued inputs entered provider early"
        );
        host_observations.push(q2_queued.clone());
        let revision = q2_queued["components"]["owner"]["value"]["revision"]
            .as_u64()
            .unwrap();
        let replacement = client
            .request_replacement(json!({
                "task_id":TASK_ID,"command_id":"command:s8:replace-b",
                "expected_task_revision":revision,"authority_ref":ANSWER_AUTHORITY,
                "reason":"Context replacement B while D0 remains entered and Q1/Q2 remain queued"
            }))
            .unwrap();
        assert_eq!(
            replacement["status"], "replacement_requested",
            "{replacement}"
        );
        let b_transition = replacement["lifecycle"]["outcome_ref"]
            .as_str()
            .unwrap()
            .to_owned();
        host_observations.push(replacement);
        fs::write(&d0_g1_release, b"release\n").unwrap();
        if hold_q1 {
            let until = Instant::now() + Duration::from_secs(60);
            while provider.entry_count() < 13 + prelude_entries {
                assert!(Instant::now() < until, "held Q1 did not enter provider");
                thread::sleep(Duration::from_millis(20));
            }
            let held = client.observe(TASK_ID, Some(&q2_delivery)).unwrap();
            let segments = held["components"]["native_delivery_status"]["value"]["segments"]
                .as_array()
                .unwrap();
            assert_eq!(
                segments
                    .iter()
                    .filter(|segment| segment["delivery_id"] == delivery_id)
                    .count(),
                3
            );
            let q1_entered = segments
                .iter()
                .filter(|segment| segment["delivery_id"] == q1_delivery)
                .collect::<Vec<_>>();
            assert_eq!(q1_entered.len(), 1);
            assert_eq!(q1_entered[0]["segment_state"], "entered");
            assert_eq!(q1_entered[0]["send_intent"], true);
            assert!(
                segments
                    .iter()
                    .all(|segment| segment["delivery_id"] != q2_delivery)
            );
            assert_eq!(
                provider.entry_count(),
                13 + prelude_entries,
                "Q2 overtook held Q1"
            );
            host_observations.push(held);
            fs::write(&q1_release, b"release\n").unwrap();
        }
        let until = Instant::now() + Duration::from_secs(75);
        let mut readback_timeouts = 0_u32;
        loop {
            let current = match client.observe(TASK_ID, Some(&q2_delivery)) {
                Ok(current) => current,
                Err(error) => {
                    readback_timeouts += 1;
                    if Instant::now() >= until {
                        let root = fixture.root.path().to_path_buf();
                        let provider_path = provider_root.path().to_path_buf();
                        let provider_outcome = format!("{:?}", provider.finish());
                        std::mem::forget(fixture);
                        std::mem::forget(provider_root);
                        panic!(
                            "two-cycle readback unavailable; root={}, provider_root={}, provider_outcome={}, timeouts={}, final_error={error}",
                            root.display(),
                            provider_path.display(),
                            provider_outcome,
                            readback_timeouts
                        );
                    }
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
            };
            let segments = current["components"]["native_delivery_status"]["value"]["segments"]
                .as_array()
                .unwrap();
            let q1_segments = segments
                .iter()
                .filter(|segment| segment["delivery_id"] == q1_delivery)
                .collect::<Vec<_>>();
            let q2_segments = segments
                .iter()
                .filter(|segment| segment["delivery_id"] == q2_delivery)
                .collect::<Vec<_>>();
            let d0_segments = segments
                .iter()
                .filter(|segment| segment["delivery_id"] == delivery_id)
                .collect::<Vec<_>>();
            if current["components"]["lifecycle"]["value"]["transition"]["value"]["transition_id"]
                == b_transition
                && current["components"]["lifecycle"]["value"]["transition"]["value"]["completed"]
                    == true
                && current["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"]
                    == "completed"
                && d0_segments.len() == 3
                && q1_segments.len() == 1
                && q2_segments.len() == 1
                && d0_segments[2]["native_state"] == "completed_settled"
                && q1_segments[0]["native_state"] == "completed_settled"
                && q2_segments[0]["native_state"] == "completed_settled"
            {
                assert_eq!(d0_segments[0]["segment_state"], "yielded");
                assert_eq!(d0_segments[1]["segment_state"], "yielded");
                assert_eq!(d0_segments[2]["segment_state"], "completed");
                assert_eq!(d0_segments[2]["context_generation"], "context:s8:g2");
                assert_eq!(
                    d0_segments[2]["predecessor_invocation_id"],
                    d0_segments[1]["invocation_id"]
                );
                assert_eq!(q1_segments[0]["context_generation"], "context:s8:g2");
                assert_eq!(q2_segments[0]["context_generation"], "context:s8:g2");
                assert!(
                    d0_segments[0]["sequence"].as_i64().unwrap()
                        < q1_segments[0]["sequence"].as_i64().unwrap()
                        && q1_segments[0]["sequence"].as_i64().unwrap()
                            < q2_segments[0]["sequence"].as_i64().unwrap()
                );
                assert_eq!(
                    current["components"]["owner"]["value"]["completed_effects"]
                        .as_array()
                        .unwrap()
                        .len(),
                    1
                );
                observed = current;
                host_observations.push(observed.clone());
                break;
            }
            if Instant::now() >= until {
                let root = fixture.root.path().to_path_buf();
                let provider_path = provider_root.path().to_path_buf();
                let provider_outcome = format!("{:?}", provider.finish());
                let stage = current["components"]["lifecycle"]["value"]["transition"].clone();
                let native = current["components"]["native_delivery_status"]["value"].clone();
                std::mem::forget(fixture);
                std::mem::forget(provider_root);
                panic!(
                    "two-cycle FIFO failed; root={}, provider_root={}, provider_outcome={}, stage={}, native={}",
                    root.display(),
                    provider_path.display(),
                    provider_outcome,
                    stage,
                    native
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
    assert!(
        observed["components"]["lifecycle"]["value"].is_object(),
        "{observed}"
    );
    assert_eq!(
        HostReportedIdentity::from_reply(&observed).unwrap(),
        reported
    );
    host.assert_same_execution().unwrap();
    let host_identity = host.identity().clone();
    let host_identity_after = observe_process(host_identity.pid).unwrap();
    assert_eq!(host_identity, host_identity_after);
    let ledger_path = provider.ledger().to_path_buf();
    let close = host.terminate_and_reap(Duration::from_secs(15)).unwrap();
    assert!(close.reaped);
    let entries = provider.finish().unwrap();
    assert_eq!(
        entries.len(),
        if two_cycles {
            14 + prelude_entries
        } else if transition_a {
            7 + prelude_entries
        } else {
            2 + prelude_entries
        }
    );
    assert!(
        entries[1 + prelude_entries]
            .request
            .to_string()
            .contains(CHECKPOINT)
    );
    assert!(fixture.root.path().is_dir());
    assert!(!fixture.sources.is_empty());
    fixture.source_manifest.verify().unwrap();
    if let Some(destination) = std::env::var_os("S10_C2_EVIDENCE_DIR") {
        let destination = std::path::PathBuf::from(destination);
        assert!(destination.is_absolute());
        fs::create_dir_all(&destination).unwrap();
        let name = if two_cycles {
            "two-cycle"
        } else {
            "transition-a"
        };
        let case = destination.join(format!("{name}-{}", host_identity.pid));
        fs::create_dir(&case).unwrap();
        fs::write(
            case.join("host-observations.json"),
            serde_json::to_vec_pretty(&host_observations).unwrap(),
        )
        .unwrap();
        fs::write(
            case.join("provider-entries.json"),
            serde_json::to_vec_pretty(&entries).unwrap(),
        )
        .unwrap();
        fs::copy(&fixture.startup_path, case.join("host-startup.json")).unwrap();
        if schema7 {
            for name in [
                "selection-v1.json",
                "selection-v3.json",
                "restart-declaration.json",
                "startup-receipt.json",
            ] {
                fs::copy(fixture.root.path().join(name), case.join(name)).unwrap();
            }
        }
        fs::copy(
            &fixture.source_manifest.path,
            case.join("source-manifest.json"),
        )
        .unwrap();
        fs::copy(
            fixture.root.path().join("tasks.sqlite"),
            case.join("tasks.sqlite"),
        )
        .unwrap();
        fs::copy(
            fixture.root.path().join("lifecycle-store/lifecycle.sqlite"),
            case.join("lifecycle.sqlite"),
        )
        .unwrap();
        eprintln!("S10_C2_EVIDENCE {}", case.display());
    }
    if let Some(destination) = std::env::var_os("S8_EVIDENCE_DIR") {
        let destination = std::path::PathBuf::from(destination);
        assert!(destination.is_absolute());
        fs::create_dir_all(&destination).unwrap();
        let repo = common::repository_root();
        let contract_path = if two_cycles {
            "/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v5.md"
        } else {
            "/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v4.md"
        };
        let contract = ArtifactBinding::read(std::path::Path::new(contract_path)).unwrap();
        assert_eq!(
            contract.sha256,
            if two_cycles {
                "33e6d653bba08d89dbd3de968140d4d7e4f7c17a8807e627b2a73fe27243b230"
            } else {
                "2afeb81499867f59e5d3b21ce45ce5d7dc61cf9cf5b79702cee09cd497e491b1"
            }
        );
        let plan = if let Some(authority) = &to2_authority {
            authority.plan().unwrap()
        } else {
            let plan = ArtifactBinding::read(
                &repo.join("docs/rust-context-lifecycle-s8-implementation-plan.md"),
            )
            .unwrap();
            assert_eq!(
                plan.sha256,
                "c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7"
            );
            plan
        };
        let evidence = EvidenceBundle {
            schema: if to2_authority.is_some() {
                TO2_RECEIPT_SCHEMA
            } else {
                "s8-external-proof-evidence-v1"
            }
            .into(),
            to2_authority: to2_authority.clone(),
            case_id: if hold_q1 {
                "s8-installed-two-cycle-q1-held-q2-blocked"
            } else if design_inquiry {
                "s8-installed-design-inquiry-snapshot-child"
            } else if two_cycles {
                "s8-installed-two-cycle-fifo"
            } else if transition_a {
                "s8-installed-transition-a"
            } else {
                "s8-installed-d0-snapshot-child"
            }
            .into(),
            contract,
            plan,
            host_executable: ArtifactBinding::read(&fixture.host_executable).unwrap(),
            source_manifest: fixture.source_manifest.clone(),
            sources: fixture.sources.clone(),
            cargo_lock: ArtifactBinding::read(&repo.join("rust/Cargo.lock")).unwrap(),
            toolchain: ArtifactBinding::read(&repo.join("rust/rust-toolchain.toml")).unwrap(),
            startup: ArtifactBinding::read(&fixture.startup_path).unwrap(),
            build_provenance: fixture.build_provenance.clone(),
            build_id: BUILD_ID.into(),
            proof_run_id: fixture.startup["lifecycle"]["proof_run_id"]
                .as_str()
                .unwrap()
                .into(),
            principal_ref: fixture.startup["lifecycle"]["principal"]["principal_ref"]
                .as_str()
                .unwrap()
                .into(),
            features: reported.features.clone(),
            profile: reported.profile.clone(),
            provider_identity: reported.provider_identity.clone(),
            disposable_root: fixture.root.path().to_path_buf(),
            subject_kind: if design_inquiry {
                "design_inquiry"
            } else {
                "repository_analysis"
            }
            .into(),
            host_identity_before: host_identity.clone(),
            host_identity_after,
            host_incarnation: reported.incarnation,
            provider_ledger: ArtifactBinding::read(&ledger_path).unwrap(),
            provider_entries: entries,
            provider_reply_artifacts: Vec::new(),
            fault_events: Vec::new(),
            lifecycle_observations: host_observations
                .iter()
                .map(|item| {
                    if to2_authority.is_some() {
                        to2_lifecycle_projection(item).unwrap()
                    } else if item["status"] == "observation" {
                        item["components"]["lifecycle"].clone()
                    } else {
                        item["lifecycle"].clone()
                    }
                })
                .collect(),
            host_observations,
            close,
        };
        let case = if hold_q1 {
            "two-cycle-q1-held"
        } else if design_inquiry {
            "design-inquiry"
        } else if two_cycles {
            "two-cycle-fifo"
        } else if transition_a {
            "transition-a"
        } else {
            "d0"
        };
        let path = destination.join(format!(
            "s8-{case}-{}.json",
            evidence.host_identity_before.pid
        ));
        let digest = evidence.write_once(&path).unwrap();
        eprintln!("S8 evidence receipt {} sha256 {digest}", path.display());
        let _ = fixture.root.keep();
        let _ = provider_root.keep();
    }
}
