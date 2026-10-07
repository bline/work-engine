use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use lifecycle_core::{
    AuthorityExpiresAt, BuildId, ClockSample, Command, CommandAdmission, CommandId, CommandRequest,
    ContextGeneration, ControlledTextInput, EffectObservation, EffectSettlement, EvidenceId,
    ExecutionOutcome, GrantId, GrantScope, InputId, ProofRunId, ProviderThreadId, ProviderTurnId,
    Revision, RuntimeIncarnation, SubjectId, TrustedGrant, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::SqliteLifecycleStore;
use serde_json::json;
use tempfile::tempdir;
use work_engine_types::CodecContract;

fn clock() -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(100),
        wait_budget: WaitBudgetMs::new(100),
    }
}

fn bootstrap(store: &mut SqliteLifecycleStore) {
    store
        .register_subject(
            SubjectId::parse("subject-process").unwrap(),
            ContextGeneration::parse("context-1").unwrap(),
            BuildId::parse("build-1").unwrap(),
            ProofRunId::parse("proof-1").unwrap(),
            clock(),
        )
        .unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: GrantId::parse("grant-1").unwrap(),
            issuer: "trusted".into(),
            principal: "operator:process".into(),
            subject: SubjectId::parse("subject-process").unwrap(),
            context: ContextGeneration::parse("context-1").unwrap(),
            build: BuildId::parse("build-1").unwrap(),
            proof_run: ProofRunId::parse("proof-1").unwrap(),
            scope: GrantScope::EnqueueInput,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
}

fn admission() -> CommandAdmission {
    let input = InputId::parse("input-process").unwrap();
    let digest = CodecContract::LifecycleTextInputV1
        .digest_binary(b"held work")
        .unwrap();
    let text = ControlledTextInput::new(input.clone(), "held work".into(), digest.clone()).unwrap();
    let basis = json!({
        "protocol_version":1,"principal_ref":"operator:process","command_id":"command-process",
        "subject_id":"subject-process","context_generation":"context-1","build_id":"build-1",
        "proof_run_id":"proof-1","grant_ref":"grant-1","expected_revision":"0",
        "kind":"enqueue_input","payload":{"input_id":"input-process","producer_ref":"fixture:process","text":"held work","text_digest":digest.hex()}
    });
    let request = CommandRequest::new(
        CommandId::parse("command-process").unwrap(),
        SubjectId::parse("subject-process").unwrap(),
        ContextGeneration::parse("context-1").unwrap(),
        BuildId::parse("build-1").unwrap(),
        ProofRunId::parse("proof-1").unwrap(),
        GrantId::parse("grant-1").unwrap(),
        Revision::new(0),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        Command::EnqueueInput { input },
    )
    .unwrap();
    CommandAdmission::new(request, "operator:process".into(), basis, Some(text)).unwrap()
}

#[test]
fn peer() {
    let Ok(mode) = std::env::var("S1_PROCESS_PEER_MODE") else {
        return;
    };
    let path = std::env::var("S1_PROCESS_PEER_ROOT").unwrap();
    let root = Path::new(&path);
    if mode == "contender" {
        assert!(SqliteLifecycleStore::open(root, "trusted".into()).is_err());
        println!("DENIED");
        std::io::stdout().flush().unwrap();
        return;
    }
    let mut store = SqliteLifecycleStore::open(root, "trusted".into()).unwrap();
    bootstrap(&mut store);
    if mode == "artifact_staged" {
        let _staged_unpublished = store
            .stage_artifact(
                SubjectId::parse("subject-process").unwrap(),
                b"staged before process death",
            )
            .unwrap();
    }
    if matches!(
        mode.as_str(),
        "command" | "prepared" | "entry" | "sent" | "observed"
    ) {
        store.apply_checked_command(admission(), clock()).unwrap();
    }
    if matches!(mode.as_str(), "prepared" | "entry" | "sent" | "observed") {
        let subject = SubjectId::parse("subject-process").unwrap();
        let plan = store.prepare_next_input(&subject).unwrap();
        if mode != "prepared" {
            let entry = store
                .claim_prepared_entry(
                    &plan.effect,
                    RuntimeIncarnation::parse("incarnation-process").unwrap(),
                    clock(),
                )
                .unwrap();
            if mode == "sent" {
                // Diagnostic barrier only: no provider is invoked by S1.
                println!("SIMULATED_SEND:{}", std::process::id());
                std::io::stdout().flush().unwrap();
            }
            if mode == "observed" {
                store
                    .apply_bound_observation(
                        EffectObservation {
                            source: EvidenceId::parse("source-process").unwrap(),
                            effect: plan.effect.clone(),
                            attempt: entry.attempt().clone(),
                            incarnation: entry.incarnation().clone(),
                            provider_thread: ProviderThreadId::parse("thread-process").unwrap(),
                            provider_turn: ProviderTurnId::parse("turn-process").unwrap(),
                            outcome: ExecutionOutcome::Completed,
                            settlement: EffectSettlement::Established(
                                EvidenceId::parse("settlement-process").unwrap(),
                            ),
                        },
                        clock(),
                    )
                    .unwrap();
            }
        }
    }
    println!("READY:{}:{}", std::process::id(), mode);
    std::io::stdout().flush().unwrap();
    let mut byte = [0];
    let _ = std::io::stdin().read(&mut byte);
}

fn spawn_peer(root: &Path, mode: &str) -> Child {
    ProcessCommand::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("peer")
        .arg("--nocapture")
        .env("S1_PROCESS_PEER_MODE", mode)
        .env("S1_PROCESS_PEER_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn wait_for(child: &mut Child, expected: &str) {
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let expected_owned = expected.to_owned();
    std::thread::spawn(move || {
        let mut acknowledged = false;
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if !acknowledged && line.contains(&expected_owned) {
                let _ = sender.send(line);
                acknowledged = true;
            }
        }
    });
    if receiver.recv_timeout(Duration::from_secs(5)).is_err() {
        let _ = child.kill();
        panic!("peer did not acknowledge exact boundary {expected}");
    }
}

fn wait_exit(child: &mut Child) -> std::process::ExitStatus {
    for _ in 0..500 {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    panic!("peer did not exit within bounded wait");
}

#[test]
fn competing_process_and_symlink_share_one_writer_fence() {
    let root = tempdir().unwrap();
    let alias = tempdir().unwrap();
    let symlink = alias.path().join("same-store");
    std::os::unix::fs::symlink(root.path(), &symlink).unwrap();
    let mut owner = spawn_peer(root.path(), "hold");
    wait_for(&mut owner, "READY:");
    let mut contender = spawn_peer(&symlink, "contender");
    wait_for(&mut contender, "DENIED");
    assert!(wait_exit(&mut contender).success());
    let database_alias = tempdir().unwrap();
    std::os::unix::fs::symlink(
        root.path().join("lifecycle.sqlite"),
        database_alias.path().join("lifecycle.sqlite"),
    )
    .unwrap();
    let mut database_contender = spawn_peer(database_alias.path(), "contender");
    wait_for(&mut database_contender, "DENIED");
    assert!(wait_exit(&mut database_contender).success());
    assert!(SqliteLifecycleStore::open(root.path(), "trusted".into()).is_err());
    owner.stdin.take().unwrap().write_all(b"x").unwrap();
    assert!(wait_exit(&mut owner).success());
    let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
    assert_eq!(store.load_recovery().unwrap().subjects, 1);
    drop(store);

    let hardlink_root = tempdir().unwrap();
    std::fs::hard_link(
        root.path().join("lifecycle.sqlite"),
        hardlink_root.path().join("lifecycle.sqlite"),
    )
    .unwrap();
    assert!(SqliteLifecycleStore::open(hardlink_root.path(), "trusted".into()).is_err());
}

#[test]
fn killed_at_each_module_commit_boundary_preserves_exact_counts_without_resend() {
    for (mode, expected) in [
        ("command", (1, 1, 0)),
        ("prepared", (1, 1, 0)),
        ("entry", (1, 1, 1)),
        ("sent", (1, 1, 1)),
        ("observed", (1, 1, 1)),
    ] {
        let root = tempdir().unwrap();
        let mut child = spawn_peer(root.path(), mode);
        let child_id = child.id();
        let barrier = if mode == "sent" {
            "SIMULATED_SEND"
        } else {
            "READY"
        };
        wait_for(&mut child, &format!("{barrier}:{child_id}"));
        child.kill().unwrap();
        assert!(!wait_exit(&mut child).success());
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        assert_eq!(store.counts().unwrap(), expected);
        assert_eq!(
            store
                .apply_checked_command(admission(), clock())
                .unwrap()
                .revision
                .get(),
            1
        );
        if matches!(mode, "entry" | "sent") {
            let subject = SubjectId::parse("subject-process").unwrap();
            assert!(store.prepare_next_input(&subject).is_err());
        }
        if mode == "observed" {
            let subject = SubjectId::parse("subject-process").unwrap();
            assert!(store.prepare_next_input(&subject).is_err());
            assert_eq!(store.load_recovery().unwrap().unresolved_attempts, 0);
        }
    }
}

#[test]
fn process_death_after_artifact_stage_leaves_only_collectible_orphan() {
    let root = tempdir().unwrap();
    let mut child = spawn_peer(root.path(), "artifact_staged");
    let child_id = child.id();
    wait_for(&mut child, &format!("READY:{child_id}:artifact_staged"));
    let digest = CodecContract::BinaryArtifactV1
        .digest_binary(b"staged before process death")
        .unwrap()
        .hex();
    let inspection = rusqlite::Connection::open_with_flags(
        root.path().join("lifecycle.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let committed: i64 = inspection
        .query_row(
            "SELECT COUNT(*) FROM artifacts WHERE digest_hex=?1",
            [digest.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(committed, 0);
    child.kill().unwrap();
    assert!(!wait_exit(&mut child).success());
    let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
    assert!(store.read_committed_artifact(&digest).unwrap().is_none());
    assert_eq!(store.collect_orphan_staging().unwrap(), 1);
}
