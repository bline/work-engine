#![cfg(feature = "test-support")]
mod claude_common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use claude_common::{fixture, sha};
use review_execution_evidence::claude::{
    CandidateBoundary, InitialEntryPermit, PinnedProgram, QualifiedReader, QualifiedWriter,
    TrustedClaudeProfile, TrustedRp0Profile,
};
use review_execution_evidence::{
    CheckedObservation, EvidenceClass, ExecutionEvidenceOwner, ExecutionPublication,
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
    let f = fixture(temp.path(), "success", Duration::from_secs(10));
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
    let reference = match writer.execute_initial(permit).await.unwrap() {
        ExecutionPublication::Committed(reference) => reference,
        other => panic!("guarded peer unresolved: {other:?}"),
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
    let mut oversized = fixture(temp.path(), "oversized_session", Duration::from_secs(10));
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
        let mut case = fixture(temp.path(), mode, Duration::from_secs(25));
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
    let mut hostile = fixture(temp.path(), "hostile_write", Duration::from_secs(10));
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
