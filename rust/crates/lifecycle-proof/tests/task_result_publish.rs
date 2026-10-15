#![recursion_limit = "256"]

mod common;

use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    process::Command,
    thread,
    time::{Duration, Instant},
};

use lifecycle_proof::{
    ProofError,
    host::{HostClient, HostReportedIdentity},
    identity::{HostProcess, observe_process},
    provider::{Reply, ScriptedProvider, Step},
    receipt::{
        ArtifactBinding, EvidenceBundle, TO2_RECEIPT_SCHEMA, to2_authority_from_environment,
        to2_lifecycle_projection,
    },
};
use serde_json::{Value, json};

use common::{BUILD_ID, CHECKPOINT, SNAPSHOT_ID, TASK_ID, create_fixture, initial_submit};

const RESULT_ID: &str = "result:s8:attributed";
const RESULT_BODY: &str =
    "Approved snapshot member m1 contained ORION-7; repository mutation approval remains pending.";

struct PreserveCrashRootOnPanic(std::path::PathBuf);

impl Drop for PreserveCrashRootOnPanic {
    fn drop(&mut self) {
        if std::thread::panicking() && self.0.exists() {
            let retained = self
                .0
                .with_file_name(format!("s8-result-crash-failure-{}", std::process::id()));
            match fs::rename(&self.0, &retained) {
                Ok(()) => eprintln!(
                    "S8 failed result crash root retained at {}",
                    retained.display()
                ),
                Err(error) => eprintln!(
                    "S8 failed result crash root preservation failed: original={} error={error}",
                    self.0.display()
                ),
            }
        }
    }
}

fn source_reference(source: &Value) -> Value {
    assert_eq!(source["current"], true);
    assert_eq!(source["availability"], "complete");
    json!({
        "id":source["id"],
        "owner_namespace":source["owner_namespace"],
        "owner_ref":source["owner_ref"],
        "revision":source["revision"],
        "sha256":source["sha256"]
    })
}

fn read_only_lifecycle_rows(database: &Path, query: &str) -> Value {
    let output = Command::new("/usr/bin/sqlite3")
        .args(["-readonly", "-batch", "-json"])
        .arg(database)
        .arg(query)
        .env("TMPDIR", std::env::var_os("TMPDIR").unwrap())
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR").unwrap(),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "read-only lifecycle query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn lifecycle_custody_rows(root: &Path) -> Value {
    let database = root.join("lifecycle-store/lifecycle.sqlite");
    json!({
        "sessions":read_only_lifecycle_rows(&database,
            "SELECT session_id,incarnation_id,allocation_state,thread_id FROM native_sessions ORDER BY session_id"),
        "task_operations":read_only_lifecycle_rows(&database,
            "SELECT operation_id,state,owner_commit_sha256,CAST(owner_commit_json AS TEXT) AS owner_commit_json FROM native_task_operations ORDER BY operation_id")
    })
}

fn direct_child_pids(pid: u32) -> Vec<u32> {
    let mut children = std::collections::BTreeSet::new();
    for task in fs::read_dir(format!("/proc/{pid}/task")).unwrap() {
        let path = task.unwrap().path().join("children");
        for child in fs::read_to_string(path).unwrap().split_whitespace() {
            children.insert(child.parse::<u32>().unwrap());
        }
    }
    children.into_iter().collect()
}

#[test]
fn installed_attributed_result_commits_under_owner_fence() {
    let to2_authority =
        to2_authority_from_environment("s8-installed-attributed-result-owner-commit").unwrap();
    let contract_before_entry = std::env::var_os("S8_CONTRACT_BEFORE_RESULT").is_some();
    let contract_at_commit = std::env::var_os("S8_CONTRACT_AT_COMMIT").is_some();
    let contract_after_owner_commit = std::env::var_os("S8_CONTRACT_AFTER_OWNER_COMMIT").is_some();
    let kill_after_owner_commit = std::env::var_os("S8_KILL_AFTER_OWNER_COMMIT").is_some();
    let kill_before_owner_commit = std::env::var_os("S8_KILL_BEFORE_OWNER_COMMIT").is_some();
    let drop_grant_reply = std::env::var_os("S8_DROP_GRANT_REPLY").is_some();
    let owner_lock_busy = std::env::var_os("S8_OWNER_LOCK_BUSY").is_some();
    assert!(!drop_grant_reply || contract_at_commit);
    assert!(
        (contract_before_entry as u8
            + contract_at_commit as u8
            + contract_after_owner_commit as u8
            + kill_after_owner_commit as u8
            + kill_before_owner_commit as u8
            + owner_lock_busy as u8)
            <= 1
    );
    let expect_owner_result =
        !contract_before_entry && !contract_at_commit && !kill_before_owner_commit;
    let tmp = std::env::var_os("TMPDIR").expect("TMPDIR must be pinned for S8 proof");
    let build_provenance = common::preflight_installed_host();
    let provider_root = tempfile::Builder::new()
        .prefix("s8-provider-result-")
        .tempdir_in(tmp)
        .unwrap();
    let arguments_path = provider_root.path().join("result-arguments.json");
    let mut script = vec![
        Step {
            required: vec![SNAPSHOT_ID.into()],
            forbidden: Vec::new(),
            reply: Reply::ToolCall {
                call_id: "call:s8:result-read".into(),
                namespace: "workspace".into(),
                name: "snapshot_read".into(),
                arguments: json!({"snapshot_id":SNAPSHOT_ID,"member_id":"m1","offset":0,"length":7}),
            },
            release_path: None,
        },
        Step {
            required: vec![CHECKPOINT.into()],
            forbidden: Vec::new(),
            reply: Reply::ToolCallFromFile {
                call_id: "call:s8:publish".into(),
                namespace: "task".into(),
                name: "result_publish".into(),
                arguments_path: arguments_path.clone(),
            },
            release_path: Some(arguments_path.clone()),
        },
    ];
    if expect_owner_result && !kill_after_owner_commit {
        script.push(Step {
            required: vec![RESULT_ID.into()],
            forbidden: Vec::new(),
            reply: Reply::Text(format!(
                "Result {RESULT_ID} was published with exact owner source references."
            )),
            release_path: None,
        });
    }
    let provider =
        ScriptedProvider::start(script, provider_root.path().join("provider-ledger.jsonl"))
            .unwrap();
    let fixture = create_fixture(&provider.base_url(), build_provenance);
    let _preserve_crash_failure = (kill_after_owner_commit || kill_before_owner_commit)
        .then(|| PreserveCrashRootOnPanic(fixture.root.path().to_path_buf()));
    let barrier = fixture.root.path().join("barrier");
    if contract_at_commit || kill_before_owner_commit {
        fs::write(barrier.join("task_before_owner_commit.arm"), b"arm\n").unwrap();
    }
    if contract_after_owner_commit || kill_after_owner_commit {
        fs::write(barrier.join("task_after_owner_commit.arm"), b"arm\n").unwrap();
    }
    if owner_lock_busy {
        fs::write(barrier.join("task_result_after_owner_begin.arm"), b"arm\n").unwrap();
    }
    let mut host = HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
    let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    host.assert_same_execution().unwrap();
    let admitted = client.submit(initial_submit()).unwrap();
    assert_eq!(admitted["status"], "admitted", "{admitted}");
    assert_eq!(admitted["lifecycle"]["outcome_kind"], "enqueued");
    let delivery_id = admitted["lifecycle"]["outcome_ref"]
        .as_str()
        .unwrap()
        .to_owned();
    let reported = HostReportedIdentity::from_reply(&admitted).unwrap();
    reported.bind_process(host.identity(), BUILD_ID).unwrap();
    reported.bind_startup(&fixture.startup).unwrap();
    let mut observations = vec![admitted];

    let until = Instant::now() + Duration::from_secs(30);
    let before_result = loop {
        if provider.entry_count() >= 2
            && let Ok(readback) = client.observe(TASK_ID, Some(&delivery_id))
            && readback["components"]["owner"]["value"]["completed_effects"]
                .as_array()
                .is_some_and(|effects| effects.len() == 1)
        {
            break readback;
        }
        if Instant::now() >= until {
            let root = fixture.root.path().to_path_buf();
            let provider_path = provider_root.path().to_path_buf();
            let entries = provider.entry_count();
            let outcome = format!("{:?}", provider.finish());
            std::mem::forget(fixture);
            std::mem::forget(provider_root);
            panic!(
                "result source readback absent; root={}, provider_root={}, entries={}, provider_outcome={outcome}",
                root.display(),
                provider_path.display(),
                entries
            );
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert!(before_result["components"]["owner"]["value"]["result"].is_null());
    observations.push(before_result.clone());
    let owner_sources = before_result["components"]["owner"]["value"]["sources"]
        .as_array()
        .unwrap();
    let effect = owner_sources
        .iter()
        .find(|source| {
            source["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("effect-"))
        })
        .expect("snapshot effect source absent");
    let approval = owner_sources
        .iter()
        .find(|source| source["id"] == "approval")
        .expect("approval source absent");
    let source_refs = vec![source_reference(effect), source_reference(approval)];
    let arguments = json!({
        "result_id":RESULT_ID,
        "body":RESULT_BODY,
        "source_refs":source_refs,
        "attribution":"source:s8:approved-analysis"
    });
    if contract_before_entry {
        let contracted = client
            .contract_grant(json!({
                "task_id":TASK_ID,
                "command_id":"command:s8:contract-result-before-entry",
                "authority_ref":common::ANSWER_AUTHORITY,
                "grant_ref":"grant:s8:result",
                "expected_revision":1
            }))
            .unwrap();
        assert_eq!(contracted["status"], "grant_contracted", "{contracted}");
        assert_eq!(contracted["lifecycle"]["revoked"], true);
        observations.push(contracted);
    }
    let temp_path = provider_root.path().join("result-arguments.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp_path)
        .unwrap();
    file.write_all(&serde_json::to_vec(&arguments).unwrap())
        .unwrap();
    file.sync_all().unwrap();
    fs::rename(&temp_path, &arguments_path).unwrap();
    let mut owner_lock_fault = None;
    if owner_lock_busy {
        let reached = barrier.join("task_result_after_owner_begin.reached");
        let until = Instant::now() + Duration::from_secs(10);
        while !reached.exists() {
            assert!(Instant::now() < until, "owner-lock barrier not reached");
            thread::sleep(Duration::from_millis(5));
        }
        let payload = json!({
            "task_id":TASK_ID,
            "command_id":"command:s8:contract-result-owner-lock-busy",
            "authority_ref":common::ANSWER_AUTHORITY,
            "grant_ref":"grant:s8:result",
            "expected_revision":1
        });
        let short_client =
            HostClient::new(&fixture.host_socket, Duration::from_millis(200)).unwrap();
        let fault = match short_client.contract_grant(payload) {
            Err(ProofError::Io(ref error))
                if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
            {
                json!({"kind":"owner_lock_control_reply_timeout","error_kind":format!("{:?}", error.kind()),"owner_lock_held":true})
            }
            Ok(reply) if reply["status"] == "pending_grant_contraction" => {
                json!({"kind":"owner_lock_control_pending","reply":reply,"owner_lock_held":true})
            }
            outcome => panic!("owner-lock control did not remain uncertain: {outcome:?}"),
        };
        owner_lock_fault = Some(fault);
        fs::write(
            barrier.join("task_result_after_owner_begin.release"),
            b"release\n",
        )
        .unwrap();
    }
    if contract_at_commit
        || contract_after_owner_commit
        || kill_after_owner_commit
        || kill_before_owner_commit
    {
        let point = if contract_at_commit || kill_before_owner_commit {
            "task_before_owner_commit"
        } else {
            "task_after_owner_commit"
        };
        let reached = barrier.join(format!("{point}.reached"));
        let until = Instant::now() + Duration::from_secs(10);
        while !reached.exists() {
            if Instant::now() >= until {
                let root = fixture.root.path().to_path_buf();
                let provider_path = provider_root.path().to_path_buf();
                let entries = provider.entry_count();
                let outcome = format!("{:?}", provider.finish());
                std::mem::forget(fixture);
                std::mem::forget(provider_root);
                panic!(
                    "task commit barrier not reached; point={point}, root={}, provider_root={}, entries={entries}, provider_outcome={outcome}",
                    root.display(),
                    provider_path.display()
                );
            }
            thread::sleep(Duration::from_millis(5));
        }
        if kill_after_owner_commit || kill_before_owner_commit {
            let before_recovery_custody = lifecycle_custody_rows(fixture.root.path());
            assert_eq!(
                before_recovery_custody["sessions"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            assert_eq!(
                before_recovery_custody["task_operations"][0]["state"],
                "entered_uncertain"
            );
            let first_process = host.identity().clone();
            let first_close = host.kill_and_reap().unwrap();
            assert_eq!(first_close.signal, Some(9));
            fs::remove_file(barrier.join(format!("{point}.arm"))).unwrap();
            let mut successor =
                HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
            if let Err(error) = client.await_ready(Duration::from_secs(10)) {
                let root = fixture.root.path().to_path_buf();
                let _ = fixture.root.keep();
                let _ = provider_root.keep();
                panic!(
                    "successor host startup failed: {error}; retained root={}",
                    root.display()
                );
            }
            let until = Instant::now() + Duration::from_secs(20);
            let recovery = loop {
                if let Ok(value) = client.observe(TASK_ID, Some(&delivery_id))
                    && value["components"]["native_delivery_status"]["value"]["recovery_only"]
                        == true
                    && value["components"]["owner"]["value"]["result"].is_object()
                        == kill_after_owner_commit
                {
                    break value;
                }
                assert!(
                    Instant::now() < until,
                    "expected owner result state not observed after host restart"
                );
                thread::sleep(Duration::from_millis(20));
            };
            let successor_reported = HostReportedIdentity::from_reply(&recovery).unwrap();
            successor_reported
                .bind_process(successor.identity(), BUILD_ID)
                .unwrap();
            successor_reported.bind_startup(&fixture.startup).unwrap();
            assert_ne!(successor_reported.incarnation, reported.incarnation);
            assert_eq!(
                recovery["components"]["native_delivery_status"]["value"]["recovery_only"],
                true
            );
            assert_eq!(
                recovery["components"]["native_delivery_status"]["value"]["segments"],
                before_result["components"]["native_delivery_status"]["value"]["segments"]
            );
            assert_eq!(
                recovery["components"]["native_delivery_status"]["value"]["segments"][0]["native_state"],
                "entered_uncertain"
            );
            assert_eq!(
                recovery["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"],
                "pending"
            );
            if kill_after_owner_commit {
                assert_eq!(
                    recovery["components"]["owner"]["value"]["result"]["result_id"],
                    RESULT_ID
                );
                assert_eq!(
                    recovery["components"]["owner"]["value"]["result"]["body"],
                    RESULT_BODY
                );
                assert_eq!(
                    recovery["components"]["owner"]["value"]["result"]["source_refs"],
                    arguments["source_refs"]
                );
                assert_eq!(
                    recovery["components"]["owner"]["value"]["revision"]
                        .as_u64()
                        .unwrap(),
                    before_result["components"]["owner"]["value"]["revision"]
                        .as_u64()
                        .unwrap()
                        + 1
                );
            } else {
                assert!(recovery["components"]["owner"]["value"]["result"].is_null());
                assert_eq!(
                    recovery["components"]["owner"]["value"]["revision"],
                    before_result["components"]["owner"]["value"]["revision"]
                );
            }
            let after_recovery_custody = lifecycle_custody_rows(fixture.root.path());
            assert_eq!(
                after_recovery_custody["sessions"],
                before_recovery_custody["sessions"]
            );
            assert_eq!(
                after_recovery_custody["task_operations"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            let reconciled = &after_recovery_custody["task_operations"][0];
            if kill_after_owner_commit {
                assert_eq!(reconciled["state"], "completed_settled");
                assert_eq!(
                    reconciled["operation_id"],
                    recovery["components"]["owner"]["value"]["result"]["operation_id"]
                );
                assert!(
                    reconciled["owner_commit_sha256"]
                        .as_str()
                        .is_some_and(|sha| sha.len() == 64)
                );
                let owner_commit: Value =
                    serde_json::from_str(reconciled["owner_commit_json"].as_str().unwrap())
                        .unwrap();
                assert_eq!(owner_commit["operation_id"], reconciled["operation_id"]);
                assert_eq!(
                    owner_commit["owner_revision"],
                    recovery["components"]["owner"]["value"]["revision"]
                );
            } else {
                assert_eq!(reconciled["state"], "entered_uncertain");
                assert!(reconciled["owner_commit_sha256"].is_null());
                assert!(reconciled["owner_commit_json"].is_null());
                assert_eq!(
                    after_recovery_custody["task_operations"],
                    before_recovery_custody["task_operations"]
                );
            }
            let successor_children = direct_child_pids(successor.identity().pid);
            assert!(
                successor_children.is_empty(),
                "recovery-only successor spawned children: {successor_children:?}"
            );
            let refused_submit = client
                .submit(common::later_submit(
                    "command:s8:recovery-only-submit",
                    "A new input must not enter recovery-only custody.",
                ))
                .unwrap();
            let refused_replacement = client
                .request_replacement(json!({
                    "task_id":TASK_ID,
                    "command_id":"command:s8:recovery-only-replacement",
                    "expected_task_revision":recovery["components"]["owner"]["value"]["revision"],
                    "authority_ref":common::ANSWER_AUTHORITY,
                    "reason":"No new replacement in recovery-only custody"
                }))
                .unwrap();
            let refused_grant = client
                .contract_grant(json!({
                    "task_id":TASK_ID,
                    "command_id":"command:s8:recovery-only-grant",
                    "authority_ref":common::ANSWER_AUTHORITY,
                    "grant_ref":"grant:s8:result",
                    "expected_revision":1
                }))
                .unwrap();
            for refusal in [&refused_submit, &refused_replacement, &refused_grant] {
                assert_eq!(refusal, &json!({"status":"error","code":"recovery_only"}));
            }
            let after_refusals = client.observe(TASK_ID, Some(&delivery_id)).unwrap();
            assert_eq!(
                after_refusals["components"]["owner"]["value"],
                recovery["components"]["owner"]["value"]
            );
            assert_eq!(
                after_refusals["components"]["native_delivery_status"]["value"],
                recovery["components"]["native_delivery_status"]["value"]
            );
            assert_eq!(
                lifecycle_custody_rows(fixture.root.path()),
                after_recovery_custody
            );
            let successor_process = successor.identity().clone();
            successor.assert_same_execution().unwrap();
            let successor_close = successor
                .terminate_and_reap(Duration::from_secs(15))
                .unwrap();
            let ledger_path = provider.ledger().to_path_buf();
            let entries = provider.finish().unwrap();
            assert_eq!(entries.len(), 2);
            fixture.source_manifest.verify().unwrap();
            for source in &fixture.sources {
                source.verify().unwrap();
            }
            assert_eq!(
                lifecycle_proof::identity::sha256_file(&fixture.host_executable).unwrap(),
                fixture.build_provenance.executable_sha256
            );
            if let Some(destination) = std::env::var_os("S8_EVIDENCE_DIR") {
                let destination = std::path::PathBuf::from(destination);
                assert!(destination.is_absolute());
                fs::create_dir_all(&destination).unwrap();
                let contract = ArtifactBinding::read(std::path::Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v9.md")).unwrap();
                assert_eq!(
                    contract.sha256,
                    "d6560d2625852eff679d0a9c43265b59cb9a13ad9940694e5f985e3ae86edf2d"
                );
                let predecessor_v8 = ArtifactBinding::read(std::path::Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v8.md")).unwrap();
                assert_eq!(
                    predecessor_v8.sha256,
                    "149639c1a0e2777235d543cd28de3a2dd89a933ad3888dd3e93f971986fba046"
                );
                let predecessor_v7 = ArtifactBinding::read(std::path::Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v7.md")).unwrap();
                assert_eq!(
                    predecessor_v7.sha256,
                    "cfbe11a0b6cedcc98bf4a2c2e2bd284d921f3b99dbb326622fce1e2d64ab0276"
                );
                let evidence = json!({
                    "schema":if kill_after_owner_commit {"s8-installed-result-owner-commit-crash-recovery-v2"} else {"s8-installed-result-before-owner-commit-crash-recovery-v1"},
                    "contract":contract,
                    "predecessor_v8":predecessor_v8,
                    "predecessor_v7":predecessor_v7,
                    "build_provenance":fixture.build_provenance,
                    "host_executable":ArtifactBinding::read(&fixture.host_executable).unwrap(),
                    "sqlite_read_only_tool":ArtifactBinding::read(Path::new("/usr/bin/sqlite3")).unwrap(),
                    "source_manifest":fixture.source_manifest,
                    "source_entries":fixture.sources,
                    "startup":ArtifactBinding::read(&fixture.startup_path).unwrap(),
                    "provider_ledger":ArtifactBinding::read(&ledger_path).unwrap(),
                    "provider_entries":entries,
                    "result_arguments":ArtifactBinding::read(&arguments_path).unwrap(),
                    "before_result":before_result,
                    "first":{"process":first_process,"reported":reported,"close":first_close},
                    "first_lifecycle_custody":before_recovery_custody,
                    "successor":{"process":successor_process,"reported":successor_reported,"observation":recovery,"lifecycle_custody":after_recovery_custody,"direct_child_pids":successor_children,"refused_new_mutations":[refused_submit,refused_replacement,refused_grant],"after_refusals":after_refusals,"close":successor_close}
                });
                let path = destination.join(format!(
                    "s8-result-{}-crash-recovery-{}.json",
                    if kill_after_owner_commit {
                        "after-owner-commit"
                    } else {
                        "before-owner-commit"
                    },
                    std::process::id()
                ));
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&path)
                    .unwrap();
                file.write_all(&serde_json::to_vec(&evidence).unwrap())
                    .unwrap();
                file.sync_all().unwrap();
                eprintln!(
                    "S8 result crash receipt {} sha256 {}",
                    path.display(),
                    lifecycle_proof::identity::sha256_file(&path).unwrap()
                );
                let _ = fixture.root.keep();
                let _ = provider_root.keep();
            }
            return;
        }
        let contract_payload = json!({
            "task_id":TASK_ID,
            "command_id":if contract_at_commit {"command:s8:contract-result-before-commit"} else {"command:s8:contract-result-after-owner-commit"},
            "authority_ref":common::ANSWER_AUTHORITY,
            "grant_ref":"grant:s8:result",
            "expected_revision":1
        });
        let contracted = if drop_grant_reply {
            client
                .send_without_reply(
                    &json!({"op":"task.contract_grant.v1","payload":contract_payload}),
                )
                .unwrap();
            let until = Instant::now() + Duration::from_secs(5);
            loop {
                let readback = client.contract_grant(contract_payload.clone()).unwrap();
                if readback["status"] == "grant_contracted" {
                    break readback;
                }
                assert_eq!(
                    readback["status"], "pending_grant_contraction",
                    "{readback}"
                );
                assert!(
                    Instant::now() < until,
                    "dropped grant reply did not reconcile"
                );
                thread::sleep(Duration::from_millis(20));
            }
        } else {
            client.contract_grant(contract_payload.clone()).unwrap()
        };
        assert_eq!(contracted["status"], "grant_contracted", "{contracted}");
        assert_eq!(contracted["lifecycle"]["revoked"], true);
        observations.push(contracted);
        let duplicate = client.contract_grant(contract_payload).unwrap();
        assert_eq!(duplicate["status"], "grant_contracted", "{duplicate}");
        assert_eq!(duplicate["lifecycle"]["resulting_revision"], "2");
        observations.push(duplicate);
        fs::write(barrier.join(format!("{point}.release")), b"release\n").unwrap();
    }

    let until = Instant::now() + Duration::from_secs(30);
    let complete = loop {
        if (contract_before_entry || contract_at_commit) && provider.response_count() >= 2 {
            let readback = client.observe(TASK_ID, Some(&delivery_id)).unwrap();
            assert!(readback["components"]["owner"]["value"]["result"].is_null());
            assert_eq!(
                readback["components"]["owner"]["value"]["revision"],
                before_result["components"]["owner"]["value"]["revision"]
            );
            assert_eq!(
                readback["components"]["native_delivery_status"]["value"]["segments"][0]["native_state"],
                "entered_uncertain"
            );
            break readback;
        }
        if let Ok(readback) = client.observe(TASK_ID, Some(&delivery_id))
            && readback["components"]["owner"]["value"]["result"].is_object()
            && readback["components"]["lifecycle"]["value"]["delivery"]["value"]["outcome"]
                == "completed"
        {
            break readback;
        }
        if Instant::now() >= until {
            let root = fixture.root.path().to_path_buf();
            let provider_path = provider_root.path().to_path_buf();
            let entries = provider.entry_count();
            let outcome = format!("{:?}", provider.finish());
            std::mem::forget(fixture);
            std::mem::forget(provider_root);
            panic!(
                "result owner commit absent; root={}, provider_root={}, entries={}, provider_outcome={outcome}",
                root.display(),
                provider_path.display(),
                entries
            );
        }
        thread::sleep(Duration::from_millis(20));
    };
    if !expect_owner_result {
        assert!(complete["components"]["owner"]["value"]["result"].is_null());
        assert_eq!(
            complete["components"]["owner"]["value"]["revision"],
            before_result["components"]["owner"]["value"]["revision"]
        );
    } else {
        common::ui_observe(
            &fixture,
            json!({"task_id":TASK_ID,"delivery_id":delivery_id}),
            "result",
            complete["host_identity"].clone(),
        );
        assert_eq!(
            complete["components"]["owner"]["value"]["result"]["result_id"],
            RESULT_ID
        );
        assert_eq!(
            complete["components"]["owner"]["value"]["result"]["body"],
            RESULT_BODY
        );
        assert_eq!(
            complete["components"]["owner"]["value"]["result"]["source_refs"],
            arguments["source_refs"]
        );
        assert!(
            complete["components"]["owner"]["value"]["result"]["operation_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("taskop:"))
        );
    }
    if expect_owner_result {
        assert_eq!(
            complete["components"]["native_delivery_status"]["value"]["segments"][0]["segment_state"],
            "completed"
        );
    }
    assert_eq!(
        HostReportedIdentity::from_reply(&complete).unwrap(),
        reported
    );
    observations.push(complete.clone());
    if owner_lock_busy {
        let payload = json!({
            "task_id":TASK_ID,
            "command_id":"command:s8:contract-result-owner-lock-busy",
            "authority_ref":common::ANSWER_AUTHORITY,
            "grant_ref":"grant:s8:result",
            "expected_revision":1
        });
        let until = Instant::now() + Duration::from_secs(10);
        let contracted = loop {
            let reply = client.contract_grant(payload.clone()).unwrap();
            if reply["status"] == "grant_contracted" {
                break reply;
            }
            assert_eq!(reply["status"], "pending_grant_contraction", "{reply}");
            assert!(
                Instant::now() < until,
                "owner-lock grant readback unresolved"
            );
            thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(contracted["lifecycle"]["revoked"], true);
        assert_eq!(contracted["lifecycle"]["resulting_revision"], "2");
        observations.push(contracted);
        let duplicate = client.contract_grant(payload).unwrap();
        assert_eq!(duplicate["status"], "grant_contracted", "{duplicate}");
        assert_eq!(duplicate["lifecycle"]["resulting_revision"], "2");
        observations.push(duplicate);
        let after_grant = client.observe(TASK_ID, Some(&delivery_id)).unwrap();
        assert_eq!(
            after_grant["components"]["owner"]["value"]["result"],
            complete["components"]["owner"]["value"]["result"]
        );
        assert_eq!(
            after_grant["components"]["owner"]["value"]["revision"],
            complete["components"]["owner"]["value"]["revision"]
        );
        observations.push(after_grant);
    }
    host.assert_same_execution().unwrap();
    let host_identity = host.identity().clone();
    let host_identity_after = observe_process(host_identity.pid).unwrap();
    let ledger_path = provider.ledger().to_path_buf();
    let close = host.terminate_and_reap(Duration::from_secs(15)).unwrap();
    let entries = provider.finish().unwrap();
    assert_eq!(entries.len(), if expect_owner_result { 3 } else { 2 });
    assert!(close.reaped);

    if let Some(destination) = std::env::var_os("S8_EVIDENCE_DIR") {
        let destination = std::path::PathBuf::from(destination);
        assert!(destination.is_absolute());
        fs::create_dir_all(&destination).unwrap();
        let repo = common::repository_root();
        let contract = ArtifactBinding::read(std::path::Path::new(
            "/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v6.md",
        )).unwrap();
        assert_eq!(
            contract.sha256,
            "b2bf39dbaff3a1ef0bf850aedd5c16c4a86c467f324a6d758e645dab7f069f91"
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
            case_id: if contract_before_entry {
                "s8-installed-result-grant-contracted-before-entry"
            } else if drop_grant_reply {
                "s8-installed-result-grant-lost-reply-reconciled"
            } else if contract_at_commit {
                "s8-installed-result-grant-contracted-before-owner-commit"
            } else if contract_after_owner_commit {
                "s8-installed-result-owner-commit-before-contraction"
            } else if owner_lock_busy {
                "s8-installed-result-owner-lock-control-timeout"
            } else {
                "s8-installed-attributed-result-owner-commit"
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
            subject_kind: "repository_analysis".into(),
            host_identity_before: host_identity,
            host_identity_after,
            host_incarnation: reported.incarnation,
            provider_ledger: ArtifactBinding::read(&ledger_path).unwrap(),
            provider_entries: entries,
            provider_reply_artifacts: vec![ArtifactBinding::read(&arguments_path).unwrap()],
            fault_events: if let Some(fault) = owner_lock_fault {
                vec![fault]
            } else if drop_grant_reply {
                vec![
                    json!({"kind":"client_reply_dropped","command_id":"command:s8:contract-result-before-commit","reconciled_by":"exact_duplicate_readback"}),
                ]
            } else {
                Vec::new()
            },
            lifecycle_observations: observations
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
            host_observations: observations,
            close,
        };
        let name = if contract_before_entry {
            "result-contracted"
        } else if drop_grant_reply {
            "result-grant-lost-reply"
        } else if contract_at_commit {
            "result-contracted-before-commit"
        } else if contract_after_owner_commit {
            "result-owner-first"
        } else if owner_lock_busy {
            "result-owner-lock-busy"
        } else {
            "result-publish"
        };
        let path = destination.join(format!(
            "s8-{name}-{}.json",
            evidence.host_identity_before.pid
        ));
        let digest = evidence.write_once(&path).unwrap();
        eprintln!("S8 evidence receipt {} sha256 {digest}", path.display());
        let _ = fixture.root.keep();
        let _ = provider_root.keep();
    }
}
