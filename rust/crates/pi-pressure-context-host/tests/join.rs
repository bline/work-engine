use lifecycle_core::*;
use lifecycle_store::*;
use pi_context_transition::PiAdapter;
use pi_pressure_context_host::{FencedPi, pressure_candidate};
use serde_json::json;
fn clock(n: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(n),
        wait_budget: WaitBudgetMs::new(100),
    }
}
fn setup() -> (SqliteLifecycleStore, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "pi-join-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut store = SqliteLifecycleStore::open(&root, "fixture-root".into()).unwrap();
    store
        .register_subject(
            SubjectId::parse("subject:pi:repair").unwrap(),
            ContextGeneration::parse("context:old").unwrap(),
            BuildId::parse("build:pi").unwrap(),
            ProofRunId::parse("proof:pi").unwrap(),
            clock(100),
        )
        .unwrap();
    for (id, context, scope) in [
        ("grant:old", "context:old", GrantScope::NativeTurn),
        ("grant:new", "context:new", GrantScope::NativeTurn),
        (
            "grant:replace",
            "context:old",
            GrantScope::RequestReplacement,
        ),
    ] {
        store
            .install_trusted_grant(TrustedGrant {
                id: GrantId::parse(id).unwrap(),
                issuer: "fixture-root".into(),
                principal: "operator:fixture".into(),
                subject: SubjectId::parse("subject:pi:repair").unwrap(),
                context: ContextGeneration::parse(context).unwrap(),
                build: BuildId::parse("build:pi").unwrap(),
                proof_run: ProofRunId::parse("proof:pi").unwrap(),
                scope,
                expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
    }
    (store, root)
}
#[test]
fn missing_or_low_pressure_cannot_request_transition() {
    assert!(!pressure_candidate(None).unwrap());
    let sample = json!({"schemaVersion":1,"observationId":"low","sequence":1,"observedAt":"2026-11-05T00:00:00Z","pressureBasisPoints":100,"source":"explicit fixture","sourceRevision":format!("sha256:{}","9".repeat(64))});
    assert!(!pressure_candidate(Some(&sample)).unwrap());
    let (mut store, root) = setup();
    assert!(
        store
            .subject_projection(&SubjectId::parse("subject:pi:repair").unwrap(), None, None)
            .unwrap()
            .transition
            .is_none()
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn durable_stop_and_expiry_survive_reopen() {
    let (mut store, root) = setup();
    assert!(FencedPi::check(&mut store, 1000, clock(999)).is_ok());
    assert!(FencedPi::check(&mut store, 1000, clock(1000)).is_err());
    FencedPi::stop(&mut store, clock(101)).unwrap();
    FencedPi::stop(&mut store, clock(102)).unwrap();
    drop(store);
    let mut store = SqliteLifecycleStore::open(&root, "fixture-root".into()).unwrap();
    assert!(FencedPi::check(&mut store, 1000, clock(103)).is_err());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[tokio::test]
async fn preissued_permit_and_reset_never_reach_transport_after_stop() {
    let (mut store, root) = setup();
    let mut intent = store
        .reserve_native_session(
            NativeSessionId::parse("session:old").unwrap(),
            SubjectId::parse("subject:pi:repair").unwrap(),
            ContextGeneration::parse("context:old").unwrap(),
            RuntimeIncarnation::parse("i").unwrap(),
            &pi_profile_digest(),
            PI_SDK_INDEX_SHA256,
            &pi_protocol_digest(),
            SnapshotId::parse("snap").unwrap(),
            &"a".repeat(64),
            false,
            clock(100),
        )
        .unwrap();
    store
        .bind_native_thread(
            &mut intent,
            ProviderThreadId::parse("physical:old").unwrap(),
        )
        .unwrap();
    let mut turn = store
        .claim_native_turn(
            &intent,
            NativeTurnRequest {
                invocation_id: "held".into(),
                attempt: AttemptId::parse("a").unwrap(),
                grant: GrantId::parse("grant:old").unwrap(),
                grant_revision: 1,
                purpose: NativeInvocationPurpose::DomainWork,
                prompt: b"held".to_vec(),
            },
            "operator:fixture",
            clock(100),
        )
        .unwrap();
    let witness = root.join("transport");
    let mut cmd = tokio::process::Command::new("/usr/bin/tee");
    cmd.arg(&witness).env_clear();
    let mut host = FencedPi::new(
        PiAdapter::launch(&mut cmd, "synthetic:boundary".into()).unwrap(),
        1000,
    );
    FencedPi::stop(&mut store, clock(101)).unwrap();
    assert!(
        host.collect_prompt(&mut store, intent.id(), &mut turn, "held", "no", clock(102))
            .await
            .is_err()
    );
    assert!(
        host.replace(
            &mut store,
            &SubjectId::parse("subject:pi:repair").unwrap(),
            &TransitionId::parse("tr").unwrap(),
            "{}",
            intent,
            clock(102)
        )
        .await
        .is_err()
    );
    host.terminate().await.unwrap();
    assert!(std::fs::read(&witness).unwrap_or_default().is_empty());
    let held = store.native_recovery("held").unwrap().unwrap();
    assert_eq!(held.class, "EnteredUncertain");
    assert!(!held.send_intent);
    drop(store);
    let mut reopened = SqliteLifecycleStore::open(&root, "fixture-root".into()).unwrap();
    assert!(
        !reopened
            .native_recovery("held")
            .unwrap()
            .unwrap()
            .send_intent
    );
    assert!(FencedPi::check(&mut reopened, 1000, clock(103)).is_err());
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unbound_allocation_cannot_be_reacquired_after_restart() {
    let (mut store, root) = setup();
    let reserve = |store: &mut SqliteLifecycleStore| {
        store.reserve_native_session(
            NativeSessionId::parse("unbound").unwrap(),
            SubjectId::parse("subject:pi:repair").unwrap(),
            ContextGeneration::parse("context:old").unwrap(),
            RuntimeIncarnation::parse("i").unwrap(),
            &pi_profile_digest(),
            PI_SDK_INDEX_SHA256,
            &pi_protocol_digest(),
            SnapshotId::parse("snap").unwrap(),
            &"a".repeat(64),
            false,
            clock(100),
        )
    };
    let intent = reserve(&mut store).unwrap();
    assert!(intent.thread().is_none());
    drop(intent);
    drop(store);
    let mut reopened = SqliteLifecycleStore::open(&root, "fixture-root".into()).unwrap();
    assert!(reserve(&mut reopened).is_err());
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}
