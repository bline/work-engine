use lifecycle_core::{
    BuildId, ClockSample, ContextGeneration, HandoffId, ProofRunId, SubjectId, WaitBudgetMs,
    WallTimeMs,
};
use lifecycle_store::SqliteLifecycleStore;
use tempfile::tempdir;

fn clock(wall: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(wall),
        wait_budget: WaitBudgetMs::new(100),
    }
}

#[test]
fn absent_destination_adopts_complete_capsule_inactive_and_reopens_same_receipt() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let capsule_root = temporary.path().join("capsule");
    let destination_root = temporary.path().join("destination");
    let mut source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let subject = SubjectId::parse("subject-adoption").unwrap();
    source
        .register_subject(
            subject.clone(),
            ContextGeneration::parse("context-adoption").unwrap(),
            BuildId::parse("build-adoption").unwrap(),
            ProofRunId::parse("proof-adoption").unwrap(),
            clock(100),
        )
        .unwrap();
    let artifact = source
        .stage_artifact(subject.clone(), b"adoption bytes")
        .unwrap();
    let artifact = source.publish_artifact(artifact, clock(101)).unwrap();
    let capsule = source.export_current(&capsule_root).unwrap();
    let handoff = HandoffId::parse("handoff-adoption").unwrap();
    let (mut destination, accepted) = SqliteLifecycleStore::adopt_inactive(
        &capsule,
        &destination_root,
        &handoff,
        "test-issuer".into(),
    )
    .unwrap();
    assert_eq!(accepted.source_store_id, capsule.source_store_id);
    assert_ne!(accepted.destination_store_id, accepted.source_store_id);
    assert_eq!(accepted.capsule_sha256, capsule.capsule_sha256);
    assert_eq!(destination.admin_ownership().unwrap().status, "inactive");
    assert_eq!(
        destination
            .read_committed_artifact(&artifact.digest_hex)
            .unwrap()
            .unwrap(),
        b"adoption bytes"
    );
    assert!(destination.prepare_next_input(&subject).is_err());
    drop(destination);
    let (reopened, same) = SqliteLifecycleStore::open_adopted_inactive(
        &destination_root,
        &handoff,
        &capsule.capsule_sha256,
        "test-issuer".into(),
    )
    .unwrap();
    assert_eq!(same, accepted);
    assert_eq!(reopened.admin_ownership().unwrap().status, "inactive");
    assert!(
        SqliteLifecycleStore::adopt_inactive(
            &capsule,
            &destination_root,
            &handoff,
            "test-issuer".into(),
        )
        .is_err()
    );
}
