#![cfg(feature = "test-support")]
mod claude_common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use claude_common::{counts, fixture, sha};
use review_execution_custody::claude::record::QualifiedTerminal;
use review_execution_custody::claude::{Prepared, ResourceSpawn};
use review_execution_custody::codec::record_digest;
use review_execution_evidence::claude::{
    CandidateBoundary, InitialEntryPermit, PinnedProgram, QualifiedReader, QualifiedWriter,
    TrustedClaudeProfile, TrustedRp0Profile,
};
use review_execution_evidence::{
    CheckedObservation, EvidenceClass, EvidenceReadError, ExecutionEvidenceOwner,
    ExecutionPublication,
};

const CBM: &str = "/home/bline/.local/bin/codebase-memory-mcp";
const CBM_SHA: &str = "e1adfa5c41f2e64c9d05620f660ebf9975f91deabc6fd72d4289da3e051a4872";

fn copy_program(source: &Path, target: &Path) -> PinnedProgram {
    fs::copy(source, target).unwrap();
    fs::set_permissions(target, fs::Permissions::from_mode(0o700)).unwrap();
    PinnedProgram::load(target, &sha(&fs::read(target).unwrap())).unwrap()
}

fn boundary(parent: &Path, candidate: &Path) -> Arc<CandidateBoundary> {
    let bwrap = Path::new("/usr/bin/bwrap");
    let launcher = copy_program(
        Path::new(env!("CARGO_BIN_EXE_claude_cbm_restricted")),
        &parent.join("cbm-launcher"),
    );
    let client = copy_program(
        Path::new(env!("CARGO_BIN_EXE_claude_cbm_client")),
        &parent.join("cbm-client"),
    );
    let rp0 = TrustedRp0Profile::load(
        PinnedProgram::load(bwrap, &sha(&fs::read(bwrap).unwrap())).unwrap(),
        PinnedProgram::load(CBM, CBM_SHA).unwrap(),
        launcher,
        client,
    )
    .unwrap();
    Arc::new(
        CandidateBoundary::prepare(rp0, candidate, parent.join("private"), "rp1_fixture").unwrap(),
    )
}

#[cfg(feature = "test-faults")]
fn v2_fault_fixture(parent: &Path, mode: &str, prepare: bool) -> (Fixture, QualifiedScope) {
    let f = fixture(parent, mode, Duration::from_secs(120));
    let boundary = if prepare {
        let prepared = Arc::try_unwrap(boundary(parent, &f.candidate))
            .ok()
            .unwrap()
            .with_resource_policy("session64mib_combined1g_zero_swap_v2")
            .unwrap();
        Arc::new(prepared.publish_manifest().unwrap().0)
    } else {
        let bwrap = Path::new("/usr/bin/bwrap");
        let rp0 = TrustedRp0Profile::load(
            PinnedProgram::load(bwrap, &sha(&fs::read(bwrap).unwrap())).unwrap(),
            PinnedProgram::load(CBM, CBM_SHA).unwrap(),
            PinnedProgram::load(
                parent.join("cbm-launcher"),
                &sha(&fs::read(parent.join("cbm-launcher")).unwrap()),
            )
            .unwrap(),
            PinnedProgram::load(
                parent.join("cbm-client"),
                &sha(&fs::read(parent.join("cbm-client")).unwrap()),
            )
            .unwrap(),
        )
        .unwrap();
        let manifest = parent.join("private/boundary-manifest.json");
        Arc::new(
            CandidateBoundary::open_frozen(
                rp0,
                &f.candidate,
                parent.join("private"),
                &sha(&fs::read(manifest).unwrap()),
            )
            .unwrap()
            .with_resource_policy("session64mib_combined1g_zero_swap_v2")
            .unwrap(),
        )
    };
    let peer_path = parent.join("installed-offline-peer");
    let peer = PinnedProgram::load(&peer_path, &sha(&fs::read(&peer_path).unwrap())).unwrap();
    let guard_path = parent.join("launch-guard");
    let guard = if prepare {
        copy_program(
            Path::new(env!("CARGO_BIN_EXE_claude_launch_guard")),
            &guard_path,
        )
    } else {
        PinnedProgram::load(&guard_path, &sha(&fs::read(&guard_path).unwrap())).unwrap()
    };
    let profile = TrustedClaudeProfile::load_offline_guarded_peer(
        peer,
        guard,
        "offline-test-model-v1",
        "high",
        vec!["offline-test-model-v1".into()],
        b"{\"offline\":true}",
    )
    .unwrap();
    let scope = f
        .scope
        .clone()
        .rebind_offline_guarded_peer(profile, boundary)
        .unwrap();
    (f, scope)
}

#[cfg(feature = "test-faults")]
#[tokio::test]
async fn v2_guarded_fault_child() {
    let Ok(parent) = std::env::var("WORK_ENGINE_V2_GUARDED_FAULT_CHILD") else {
        return;
    };
    let (_, scope) = v2_fault_fixture(Path::new(&parent), "success", false);
    let permit = InitialEntryPermit::offline_peer(&scope);
    let mut writer =
        QualifiedWriter::create(&Path::new(&parent).join("evidence-root"), scope).unwrap();
    let _ = writer.execute_initial(permit).await;
}

#[cfg(feature = "test-faults")]
#[test]
fn v2_guarded_crash_cuts_reopen_without_second_entry() {
    let relay_requests = |path: &Path| {
        fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter(|line| *line == "peer_request")
            .count()
    };
    for cut in ["spawn_outcome", "capture_complete", "terminal"] {
        let temp = tempfile::Builder::new()
            .prefix("v2-cut-")
            .tempdir_in(std::env::temp_dir())
            .unwrap();
        let (f, scope) = v2_fault_fixture(temp.path(), "success", true);
        let ready = temp.path().join("cut-ready");
        let relay_witness = temp.path().join("relay-witness");
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "v2_guarded_fault_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("WORK_ENGINE_V2_GUARDED_FAULT_CHILD", temp.path())
            .env("WORK_ENGINE_CLAUDE_FAULT_CUT", cut)
            .env("WORK_ENGINE_CLAUDE_FAULT_READY", &ready)
            .env("RP2_RELAY_WITNESS", &relay_witness)
            .spawn()
            .unwrap();
        // A cold child rechecks the sealed CBM store before it can reach E1.
        let deadline = Instant::now() + Duration::from_secs(180);
        while !ready.exists() && Instant::now() < deadline {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(ready.exists(), "v2 guarded fault cut not reached: {cut}");
        child.kill().unwrap();
        child.wait().unwrap();
        let before_reconcile = relay_requests(&relay_witness);
        let selection = scope.reader_selection().unwrap();
        let mut writer = QualifiedWriter::resume(&f.root, scope).unwrap();
        let first = writer.reconcile().unwrap();
        let second = writer.reconcile().unwrap();
        match cut {
            "spawn_outcome" => {
                assert_eq!(first, ExecutionPublication::Unresolved);
                assert_eq!(second, ExecutionPublication::Unresolved);
                assert_eq!(before_reconcile, 0);
            }
            "capture_complete" => {
                let reference = match first {
                    ExecutionPublication::Committed(reference) => reference,
                    other => panic!("unexpected v2 capture cut recovery: {other:?}"),
                };
                assert_eq!(second, ExecutionPublication::Replayed(reference.clone()));
                drop(writer);
                QualifiedReader::open(&f.root, selection)
                    .unwrap()
                    .read_result(&reference)
                    .unwrap();
                assert_eq!(before_reconcile, 1);
            }
            "terminal" => {
                let reference = match first {
                    ExecutionPublication::Replayed(reference) => reference,
                    other => panic!("unexpected v2 terminal cut recovery: {other:?}"),
                };
                assert_eq!(second, ExecutionPublication::Replayed(reference));
                assert_eq!(before_reconcile, 1);
            }
            _ => unreachable!(),
        }
        assert_eq!(relay_requests(&relay_witness), before_reconcile);
    }
}

fn watch_private_quota(
    stop: Arc<AtomicBool>,
    name: String,
) -> thread::JoinHandle<(u64, u64, u64, bool, u64, bool)> {
    let relative = fs::read_to_string("/proc/self/cgroup")
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap()
        .to_owned();
    let parent = Path::new("/sys/fs/cgroup")
        .join(relative.trim_start_matches('/'))
        .parent()
        .unwrap()
        .to_owned();
    let path = parent.join(name);
    thread::spawn(move || {
        let mut seen = (0, 0, 0, false, 0, false);
        while !stop.load(Ordering::SeqCst) {
            if path.is_dir() {
                let limit = fs::read_to_string(path.join("memory.max"))
                    .ok()
                    .and_then(|value| value.trim().parse::<u64>().ok())
                    .unwrap_or(0);
                let events = fs::read_to_string(path.join("memory.events")).unwrap_or_default();
                let count = |name: &str| {
                    events
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix(name)
                                .and_then(|rest| rest.strip_prefix(' '))
                                .and_then(|rest| rest.parse::<u64>().ok())
                        })
                        .unwrap_or(0)
                };
                seen.0 = seen.0.max(limit);
                seen.1 = seen.1.max(count("max"));
                seen.2 = seen.2.max(count("oom_kill"));
                seen.3 |= fs::read_to_string(path.join("memory.swap.max"))
                    .is_ok_and(|value| value.trim() == "0");
                if let Ok(current) = fs::read_to_string(path.join("memory.swap.current"))
                    && let Ok(current) = current.trim().parse::<u64>()
                {
                    seen.4 = seen.4.max(current);
                    seen.5 = true;
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        seen
    })
}

#[tokio::test]
async fn installed_local_probe_witness_is_after_ready_and_has_no_egress() {
    let scratch = std::env::temp_dir();
    fs::create_dir_all(&scratch).unwrap();
    let temp = tempfile::Builder::new()
        .prefix("local-")
        .tempdir_in(&scratch)
        .unwrap();
    let f = fixture(temp.path(), "success", Duration::from_secs(120));
    let boundary = Arc::new(
        Arc::try_unwrap(boundary(temp.path(), &f.candidate))
            .ok()
            .unwrap()
            .with_resource_policy("session64mib_combined1g_zero_swap_v2")
            .unwrap(),
    );
    let peer = copy_program(
        Path::new(env!("CARGO_BIN_EXE_claude_local_probe_peer")),
        &temp.path().join("selected-installed-fixture"),
    );
    let guard = copy_program(
        Path::new(env!("CARGO_BIN_EXE_claude_launch_guard")),
        &temp.path().join("launch-guard"),
    );
    let profile = TrustedClaudeProfile::load_installed(
        peer,
        guard,
        "offline-test-model-v1",
        "high",
        vec!["offline-test-model-v1".into()],
        b"{\"offline\":true}",
    )
    .unwrap();
    let result = boundary
        .probe_installed_local(
            &profile,
            &temp.path().join("local-probe-root"),
            &"a".repeat(64),
            "session-local-fixture-1",
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    assert!(result.guard_ready && result.network_denied_before_ready);
    assert!(
        result.spawned,
        "READY alone must not pass the local milestone"
    );
    assert!(result.local_milestone_met);
    assert!(result.selected_child_pid.is_some());
    assert_eq!(result.guard_exit_code, Some(0));
    assert!(!result.timed_out);
    assert!(result.swap_max_zero && result.swap_current_zero);
    assert!(result.memory_peak <= result.memory_max);
    assert_eq!(result.oom_kill, 0);
    assert_eq!(
        result.resource_policy,
        "session64mib_combined1g_zero_swap_v2"
    );
    assert_eq!(result.combined_limit, 1024 * 1024 * 1024);
    assert_eq!(
        result.memory_max,
        (result.combined_limit - result.host_custody_charge) / result.quota_page_size
            * result.quota_page_size
    );
    assert!(!temp.path().join("local-probe-root/events").exists());
    eprintln!(
        "RP3_SYNTHETIC_LOCAL_PROBE={}",
        serde_json::to_string(&result).unwrap()
    );
}

#[tokio::test]
async fn v2_guarded_e1_record_reopens_with_exact_resource_equation() {
    let temp = tempfile::Builder::new()
        .prefix("v2-e1-")
        .tempdir_in(std::env::temp_dir())
        .unwrap();
    let f = fixture(temp.path(), "success", Duration::from_secs(120));
    let boundary = Arc::new(
        Arc::try_unwrap(boundary(temp.path(), &f.candidate))
            .ok()
            .unwrap()
            .with_resource_policy("session64mib_combined1g_zero_swap_v2")
            .unwrap(),
    );
    let peer = PinnedProgram::load(
        temp.path().join("installed-offline-peer"),
        &sha(&fs::read(temp.path().join("installed-offline-peer")).unwrap()),
    )
    .unwrap();
    let guard = copy_program(
        Path::new(env!("CARGO_BIN_EXE_claude_launch_guard")),
        &temp.path().join("launch-guard"),
    );
    let profile = TrustedClaudeProfile::load_offline_guarded_peer(
        peer,
        guard,
        "offline-test-model-v1",
        "high",
        vec!["offline-test-model-v1".into()],
        b"{\"offline\":true}",
    )
    .unwrap();
    let scope = f
        .scope
        .rebind_offline_guarded_peer(profile, boundary)
        .unwrap();
    let selection = scope.reader_selection().unwrap();
    let permit = InitialEntryPermit::offline_peer(&scope);
    let mut writer = QualifiedWriter::create(&f.root, scope.clone()).unwrap();
    let reference = match writer.execute_initial(permit).await.unwrap() {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("v2 guarded E1 failed: {other:?}"),
    };
    drop(writer);
    let conn = rusqlite::Connection::open(f.root.join("evidence.sqlite")).unwrap();
    let prepared_raw: Vec<u8> = conn
        .query_row(
            "SELECT payload FROM events WHERE stage='prepared'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let prepared: Prepared = serde_json::from_slice(&prepared_raw).unwrap();
    let raw: Vec<u8> = conn
        .query_row(
            "SELECT payload FROM events WHERE stage='spawn_outcome'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let event: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let spawn: ResourceSpawn = serde_json::from_slice(&raw).unwrap();
    assert!(spawn.validate(&prepared).is_ok());
    for invalid in [
        ResourceSpawn {
            schema_version: 1,
            ..spawn.clone()
        },
        ResourceSpawn {
            resource_policy: "strict_aggregate_session_64mib_v1".into(),
            ..spawn.clone()
        },
        ResourceSpawn {
            host_custody_charge: 64 * 1024 * 1024 + 1,
            ..spawn.clone()
        },
        ResourceSpawn {
            page_size: 0,
            ..spawn.clone()
        },
        ResourceSpawn {
            page_size: 3,
            ..spawn.clone()
        },
        ResourceSpawn {
            memory_max: spawn.memory_max - 1,
            ..spawn.clone()
        },
        ResourceSpawn {
            swap_max_zero: false,
            ..spawn.clone()
        },
        ResourceSpawn {
            swap_current_zero: false,
            ..spawn.clone()
        },
    ] {
        assert_eq!(
            invalid.validate(&prepared),
            Err(EvidenceReadError::Conflicting(
                "Claude v2 resource realization mismatch"
            ))
        );
    }
    let mut wrong_prepared = prepared.clone();
    wrong_prepared.schema_version = 1;
    assert!(spawn.validate(&wrong_prepared).is_err());
    wrong_prepared = prepared.clone();
    wrong_prepared.resource_policy = None;
    assert!(spawn.validate(&wrong_prepared).is_err());
    assert_eq!(
        event["resourcePolicy"],
        "session64mib_combined1g_zero_swap_v2"
    );
    assert_eq!(event["swapMaxZero"], true);
    assert_eq!(event["swapCurrentZero"], true);
    let h = event["hostCustodyCharge"].as_u64().unwrap();
    let page = event["pageSize"].as_u64().unwrap();
    assert!(h <= 64 * 1024 * 1024);
    assert_eq!(
        event["memoryMax"].as_u64().unwrap(),
        (1024 * 1024 * 1024 - h) / page * page
    );
    let reader = QualifiedReader::open(&f.root, selection.clone()).unwrap();
    reader.read_result(&reference).unwrap();
    drop(reader);
    let mut reopened = QualifiedWriter::resume(&f.root, scope.clone()).unwrap();
    assert!(matches!(
        reopened.reconcile().unwrap(),
        ExecutionPublication::Replayed(found) if found == reference
    ));
    drop(reopened);

    // Keep the event chain internally valid so the reopened checked reader
    // reaches the v2 semantic validator, rather than failing only a checksum.
    let starts_before = counts(&f.witness);
    let mut invalid_cases = Vec::new();
    for (label, invalid) in [
        (
            "spawn schema",
            ResourceSpawn {
                schema_version: 1,
                ..spawn.clone()
            },
        ),
        (
            "spawn policy",
            ResourceSpawn {
                resource_policy: "strict_aggregate_session_64mib_v1".into(),
                ..spawn.clone()
            },
        ),
        (
            "host charge",
            ResourceSpawn {
                host_custody_charge: 64 * 1024 * 1024 + 1,
                ..spawn.clone()
            },
        ),
        (
            "zero page",
            ResourceSpawn {
                page_size: 0,
                ..spawn.clone()
            },
        ),
        (
            "nonpower page",
            ResourceSpawn {
                page_size: 3,
                ..spawn.clone()
            },
        ),
        (
            "memory cap",
            ResourceSpawn {
                memory_max: spawn.memory_max - 1,
                ..spawn.clone()
            },
        ),
        (
            "swap max",
            ResourceSpawn {
                swap_max_zero: false,
                ..spawn.clone()
            },
        ),
        (
            "swap current",
            ResourceSpawn {
                swap_current_zero: false,
                ..spawn.clone()
            },
        ),
        (
            "PID",
            ResourceSpawn {
                pid: spawn.pid + 1,
                ..spawn.clone()
            },
        ),
    ] {
        invalid_cases.push((label, 3, serde_json::to_vec(&invalid).unwrap()));
    }
    let mut wrong_prepared = prepared.clone();
    wrong_prepared.schema_version = 1;
    invalid_cases.push((
        "prepared schema",
        0,
        serde_json::to_vec(&wrong_prepared).unwrap(),
    ));
    wrong_prepared = prepared;
    wrong_prepared.resource_policy = None;
    invalid_cases.push((
        "prepared policy",
        0,
        serde_json::to_vec(&wrong_prepared).unwrap(),
    ));
    let mut statement = conn
        .prepare("SELECT ordinal,stage,payload FROM events WHERE attempt_id=?1 ORDER BY ordinal")
        .unwrap();
    let stages = statement
        .query_map([&reference.attempt_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    drop(statement);
    assert_eq!(stages.len(), 6);
    assert_eq!(stages[4].1, "capture_complete");
    assert_eq!(stages[5].1, "terminal");
    let rehash_with = |index: usize, payload: Vec<u8>| {
        let mut changed = stages.clone();
        changed[index].2 = payload;
        let mut prior = String::new();
        let mut capture_digest = String::new();
        for (ordinal, stage, mut payload) in changed {
            if stage == "terminal" {
                let mut terminal: QualifiedTerminal = serde_json::from_slice(&payload).unwrap();
                terminal.capture_event_sha256 = capture_digest.clone();
                payload = serde_json::to_vec(&terminal).unwrap();
            }
            let digest = record_digest(&[
                reference.attempt_id.as_bytes(),
                stage.as_bytes(),
                prior.as_bytes(),
                &payload,
            ]);
            conn.execute(
                "UPDATE events SET predecessor_sha256=?1,payload=?2,sha256=?3 WHERE attempt_id=?4 AND ordinal=?5",
                rusqlite::params![prior, payload, digest, reference.attempt_id, ordinal],
            )
            .unwrap();
            if stage == "capture_complete" {
                capture_digest = digest.clone();
            }
            prior = digest;
        }
        let mut changed_reference = reference.clone();
        changed_reference.sha256 = prior;
        changed_reference
    };
    // Change only JSON whitespace and rehash every dependent record. A valid
    // changed chain must still pass both readback and replay before the hostile
    // variants can establish semantic rejection rather than a stale link.
    let mut valid_spawn = stages[3].2.clone();
    valid_spawn.push(b' ');
    let valid_reference = rehash_with(3, valid_spawn);
    assert_ne!(valid_reference.sha256, reference.sha256);
    QualifiedReader::open(&f.root, selection.clone())
        .unwrap()
        .read_result(&valid_reference)
        .unwrap();
    let mut valid_replay = QualifiedWriter::resume(&f.root, scope.clone()).unwrap();
    assert_eq!(
        valid_replay.reconcile().unwrap(),
        ExecutionPublication::Replayed(valid_reference)
    );
    drop(valid_replay);
    for (label, index, payload) in invalid_cases {
        let changed_reference = rehash_with(index, payload);
        let reader = QualifiedReader::open(&f.root, selection.clone()).unwrap();
        let error = reader.read_result(&changed_reference).unwrap_err();
        if index == 0 {
            assert_eq!(
                error,
                EvidenceReadError::Unsupported("Claude resource policy version"),
                "{label} rejected at unexpected boundary"
            );
        } else {
            assert_eq!(
                error,
                EvidenceReadError::Conflicting("Claude profile/capture binding mismatch"),
                "{label} rejected at unexpected boundary"
            );
        }
        drop(reader);
        let mut reopened = QualifiedWriter::resume(&f.root, scope.clone()).unwrap();
        assert!(
            reopened.reconcile().is_err(),
            "{label} replayed by recovery"
        );
        drop(reopened);
    }
    assert_eq!(
        counts(&f.witness),
        starts_before,
        "recovery relaunched the peer"
    );
}

#[tokio::test]
async fn v2_guarded_oversized_session_refuses_without_relaunch() {
    let temp = tempfile::Builder::new()
        .prefix("v2-capacity-")
        .tempdir_in(std::env::temp_dir())
        .unwrap();
    let f = fixture(temp.path(), "oversized_session", Duration::from_secs(120));
    let boundary = Arc::new(
        Arc::try_unwrap(boundary(temp.path(), &f.candidate))
            .ok()
            .unwrap()
            .with_resource_policy("session64mib_combined1g_zero_swap_v2")
            .unwrap(),
    );
    let peer_path = temp.path().join("installed-offline-peer");
    let peer = PinnedProgram::load(&peer_path, &sha(&fs::read(&peer_path).unwrap())).unwrap();
    let guard = copy_program(
        Path::new(env!("CARGO_BIN_EXE_claude_launch_guard")),
        &temp.path().join("launch-guard"),
    );
    let profile = TrustedClaudeProfile::load_offline_guarded_peer(
        peer,
        guard,
        "offline-test-model-v1",
        "high",
        vec!["offline-test-model-v1".into()],
        b"{\"offline\":true}",
    )
    .unwrap();
    let scope = f
        .scope
        .rebind_offline_guarded_peer(profile, boundary)
        .unwrap();
    let permit = InitialEntryPermit::offline_peer(&scope);
    let mut writer = QualifiedWriter::create(&f.root, scope.clone()).unwrap();
    assert_eq!(
        writer.execute_initial(permit).await.unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(
        writer.reconcile().unwrap(),
        ExecutionPublication::Unresolved
    );
    drop(writer);
    let mut reopened = QualifiedWriter::resume(&f.root, scope).unwrap();
    assert_eq!(
        reopened.reconcile().unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(
        fs::read_dir(f.root.join("session-live")).unwrap().count(),
        0
    );
    let conn = rusqlite::Connection::open(f.root.join("evidence.sqlite")).unwrap();
    let count_stage = |stage: &str| -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM events WHERE stage=?1",
            [stage],
            |row| row.get(0),
        )
        .unwrap()
    };
    assert_eq!(
        count_stage("spawn_outcome"),
        1,
        "recovery repeated guarded spawn"
    );
    assert_eq!(count_stage("capture_failure"), 1);
    assert_eq!(count_stage("terminal"), 0);
    let spawn: Vec<u8> = conn
        .query_row(
            "SELECT payload FROM events WHERE stage='spawn_outcome'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let spawn: ResourceSpawn = serde_json::from_slice(&spawn).unwrap();
    assert_eq!(
        spawn.resource_policy,
        "session64mib_combined1g_zero_swap_v2"
    );
    assert!(spawn.swap_max_zero && spawn.swap_current_zero);
    let failure: Vec<u8> = conn
        .query_row(
            "SELECT payload FROM events WHERE stage='capture_failure'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let failure: serde_json::Value = serde_json::from_slice(&failure).unwrap();
    assert_eq!(failure["exit_code"], 72);
}

#[tokio::test]
#[ignore = "requires an explicit non-authoritative installed CLI/model candidate; never reads credentials or permits network egress"]
async fn installed_cli_local_candidate_probe() {
    let path = std::env::var("RP3_LOCAL_CANDIDATE_PROGRAM")
        .expect("owner or qualifier must select a local CLI candidate");
    let model =
        std::env::var("RP3_LOCAL_CANDIDATE_MODEL").expect("local candidate model alias required");
    let effort =
        std::env::var("RP3_LOCAL_CANDIDATE_EFFORT").expect("local candidate effort required");
    let executable_sha256 = std::env::var("RP3_LOCAL_CANDIDATE_SHA256")
        .expect("exact local candidate executable digest required");
    let scratch = std::env::temp_dir();
    fs::create_dir_all(&scratch).unwrap();
    let temp = tempfile::Builder::new()
        .prefix("candidate-")
        .tempdir_in(&scratch)
        .unwrap();
    let f = fixture(temp.path(), "success", Duration::from_secs(120));
    let boundary = Arc::new(
        Arc::try_unwrap(boundary(temp.path(), &f.candidate))
            .ok()
            .unwrap()
            .with_resource_policy("session64mib_combined1g_zero_swap_v2")
            .unwrap(),
    );
    let program_path = Path::new(&path).canonicalize().unwrap();
    let program = PinnedProgram::load(&program_path, &executable_sha256).unwrap();
    let guard = copy_program(
        Path::new(env!("CARGO_BIN_EXE_claude_launch_guard")),
        &temp.path().join("launch-guard"),
    );
    let profile = TrustedClaudeProfile::load_installed(
        program,
        guard,
        &model,
        &effort,
        vec![model.clone()],
        b"{\"localCompatibilityCandidate\":true}",
    )
    .unwrap();
    let result = boundary
        .probe_installed_local(
            &profile,
            &temp.path().join("local-probe-root"),
            &"b".repeat(64),
            "session-local-candidate-1",
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    eprintln!(
        "RP3_INSTALLED_CANDIDATE_LOCAL_PROBE={}",
        serde_json::to_string(&result).unwrap()
    );
    assert!(result.network_denied_before_ready);
    assert!(
        result.local_milestone_met,
        "selected local candidate failed the post-READY spawn/memory milestone"
    );
}

#[tokio::test]
async fn guarded_offline_peer_uses_real_rp0_candidate_and_cbm_boundary() {
    let scratch = Path::new("/home/bline/code/.work-engine-tmp/rp1g");
    fs::create_dir_all(scratch).unwrap();
    let temp = tempfile::Builder::new()
        .prefix("g-")
        .tempdir_in(scratch)
        .unwrap();
    let candidate = temp.path().join("candidate");
    fs::create_dir(&candidate).unwrap();
    fs::write(candidate.join("lib.rs"), b"pub fn exact() -> usize { 1 }\n").unwrap();
    // The guarded peer performs a real CBM RPC. Keep its deadline above
    // observed cold MCP startup (56.7 seconds in the RP2 diagnostic) so the
    // test measures isolation and custody rather than host scheduling delay.
    // This is a fixture horizon, not a hard cleanup deadline claim.
    let f = fixture(temp.path(), "success", Duration::from_secs(120));
    let boundary = boundary(temp.path(), &f.candidate);
    let peer = PinnedProgram::load(
        temp.path().join("installed-offline-peer"),
        &sha(&fs::read(temp.path().join("installed-offline-peer")).unwrap()),
    )
    .unwrap();
    let guard = copy_program(
        Path::new(env!("CARGO_BIN_EXE_claude_launch_guard")),
        &temp.path().join("launch-guard"),
    );
    let profile = TrustedClaudeProfile::load_offline_guarded_peer(
        peer,
        guard,
        "offline-test-model-v1",
        "high",
        vec!["offline-test-model-v1".into()],
        b"{\"offline\":true}",
    )
    .unwrap();
    let scope = f
        .scope
        .rebind_offline_guarded_peer(profile.clone(), boundary.clone())
        .unwrap();
    let second_permit = InitialEntryPermit::offline_peer(&scope);
    let mut second_writer =
        QualifiedWriter::create(&temp.path().join("second-evidence-root"), scope.clone()).unwrap();
    let selection = scope.reader_selection().unwrap();
    let permit = InitialEntryPermit::offline_peer(&scope);
    let mut writer = QualifiedWriter::create(&f.root, scope).unwrap();
    let peer_started = Instant::now();
    let publication = writer.execute_initial(permit).await.unwrap();
    eprintln!(
        "GUARDED_CBM_SUCCESS_MILLIS={}",
        peer_started.elapsed().as_millis()
    );
    let reference = match publication {
        ExecutionPublication::Committed(reference) => reference,
        other => {
            let diagnostic_root = temp.keep();
            panic!("guarded peer unresolved: {other:?}; diagnostic root={diagnostic_root:?}")
        }
    };
    drop(writer);
    let reader = QualifiedReader::open(&f.root, selection).unwrap();
    let checked = reader.read_result(&reference).unwrap();
    assert_eq!(checked.evidence_class(), EvidenceClass::ControlledFixture);
    let observation = match checked.observation() {
        CheckedObservation::Present(observation) => observation,
        _ => panic!("guarded observation absent"),
    };
    assert_eq!(
        observation
            .get("capabilityEnvelope")
            .and_then(|v| v.get("mutationAuthorized")),
        Some(&claim_evidence::codec::JsValue::Bool(false))
    );
    reader.retained_session(&reference).unwrap();
    let mut oversized = fixture(temp.path(), "oversized_session", Duration::from_secs(120));
    oversized.root = temp.path().join("oversized-evidence-root");
    let oversized_scope = oversized
        .scope
        .rebind_offline_guarded_peer(profile.clone(), boundary.clone())
        .unwrap();
    let oversized_permit = InitialEntryPermit::offline_peer(&oversized_scope);
    let mut oversized_writer =
        QualifiedWriter::create(&oversized.root, oversized_scope.clone()).unwrap();
    assert_eq!(
        oversized_writer
            .execute_initial(oversized_permit)
            .await
            .unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(
        oversized_writer.reconcile().unwrap(),
        ExecutionPublication::Unresolved
    );
    assert_eq!(
        fs::read_dir(oversized.root.join("session-live"))
            .unwrap()
            .count(),
        0
    );
    let db = rusqlite::Connection::open(oversized.root.join("evidence.sqlite")).unwrap();
    let starts: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM events WHERE stage='spawn_outcome'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(starts, 1);
    let failure: Vec<u8> = db
        .query_row(
            "SELECT payload FROM events WHERE stage='capture_failure'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let failure: serde_json::Value = serde_json::from_slice(&failure).unwrap();
    assert_eq!(
        failure.get("exit_code").and_then(serde_json::Value::as_i64),
        Some(72)
    );
    for (mode, expected_exit) in [
        ("sparse_session", Some(71)),
        ("fifo_session", Some(71)),
        ("inode_growth", None),
    ] {
        let mut case = fixture(temp.path(), mode, Duration::from_secs(120));
        case.root = temp.path().join(format!("{mode}-evidence-root"));
        let case_scope = case
            .scope
            .rebind_offline_guarded_peer(profile.clone(), boundary.clone())
            .unwrap();
        let case_permit = InitialEntryPermit::offline_peer(&case_scope);
        let mut case_writer = QualifiedWriter::create(&case.root, case_scope).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let watcher = (mode == "inode_growth").then(|| {
            watch_private_quota(
                stop.clone(),
                format!(
                    "rp1-{}-{}",
                    sha(format!("attempt-1:{}", profile.profile_digest()).as_bytes()),
                    &sha(case
                        .root
                        .join("session-live")
                        .as_os_str()
                        .as_encoded_bytes())[..16]
                ),
            )
        });
        assert_eq!(
            case_writer.execute_initial(case_permit).await.unwrap(),
            ExecutionPublication::Unresolved
        );
        stop.store(true, Ordering::SeqCst);
        if let Some(watcher) = watcher {
            let (limit, max_events, oom_kills, swap_zero, swap_current_max, swap_current_seen) =
                watcher.join().unwrap();
            println!(
                "inode quota witness: memory.max={limit} memory.swap.max=0 observed={swap_zero} memory.swap.current.max={swap_current_max} max_events={max_events} oom_kills={oom_kills}"
            );
            assert!(limit > 0 && limit <= 64 * 1024 * 1024);
            assert!(swap_zero, "kernel zero-swap quota was never observed");
            assert!(swap_current_seen, "kernel swap charge was never observed");
            assert_eq!(swap_current_max, 0, "private cgroup charged swap");
            assert!(
                max_events > 0,
                "empty-file growth never reached the kernel quota"
            );
            assert!(
                oom_kills > 0,
                "empty-file growth did not exhaust inode metadata budget"
            );
        }
        assert_eq!(
            case_writer.reconcile().unwrap(),
            ExecutionPublication::Unresolved
        );
        assert_eq!(
            fs::read_dir(case.root.join("session-live"))
                .unwrap()
                .count(),
            0
        );
        let db = rusqlite::Connection::open(case.root.join("evidence.sqlite")).unwrap();
        let spawns: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM events WHERE stage='spawn_outcome'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(spawns, 1, "{mode} must not relaunch");
        let failure: Vec<u8> = db
            .query_row(
                "SELECT payload FROM events WHERE stage='capture_failure'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let failure: serde_json::Value = serde_json::from_slice(&failure).unwrap();
        if let Some(code) = expected_exit {
            assert_eq!(
                failure.get("exit_code").and_then(serde_json::Value::as_i64),
                Some(code)
            );
        }
    }
    let mut hostile = fixture(temp.path(), "hostile_write", Duration::from_secs(120));
    hostile.root = temp.path().join("hostile-evidence-root");
    let hostile_scope = hostile
        .scope
        .rebind_offline_guarded_peer(profile, boundary.clone())
        .unwrap();
    let hostile_selection = hostile_scope.reader_selection().unwrap();
    let hostile_permit = InitialEntryPermit::offline_peer(&hostile_scope);
    let mut hostile_writer = QualifiedWriter::create(&hostile.root, hostile_scope).unwrap();
    let hostile_reference = match hostile_writer
        .execute_initial(hostile_permit)
        .await
        .unwrap()
    {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("hostile write refusal was not observed: {other:?}"),
    };
    drop(hostile_writer);
    let hostile_reader = QualifiedReader::open(&hostile.root, hostile_selection).unwrap();
    assert_eq!(
        hostile_reader
            .read_result(&hostile_reference)
            .unwrap()
            .evidence_class(),
        EvidenceClass::ControlledFixture
    );
    fs::write(boundary.mcp_config_path(), b"changed config").unwrap();
    assert_eq!(
        second_writer.execute_initial(second_permit).await.unwrap(),
        ExecutionPublication::DefinitePreEntryFailure
    );
}
