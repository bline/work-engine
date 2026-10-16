use lifecycle_core::{
    BuildId, ClockSample, ContextGeneration, HandoffId, ProofRunId, SubjectId, WaitBudgetMs,
    WallTimeMs,
};
use lifecycle_store::SqliteLifecycleStore;
use tempfile::tempdir;

#[test]
fn ordinary_schema_six_keeps_normal_admission_and_cannot_masquerade_as_held() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("ordinary");
    let mut store = SqliteLifecycleStore::open(&root, "trusted".into()).unwrap();
    store
        .register_subject(
            SubjectId::parse("ordinary-new-id").unwrap(),
            ContextGeneration::parse("ordinary-context").unwrap(),
            BuildId::parse("ordinary-build").unwrap(),
            ProofRunId::parse("ordinary-proof").unwrap(),
            ClockSample {
                wall: WallTimeMs::new(1),
                wait_budget: WaitBudgetMs::new(1),
            },
        )
        .unwrap();
    assert!(store.held_projection().is_err());
    assert!(
        store
            .export_held(&temp.path().join("must-not-held-export"))
            .is_err()
    );
    let native = temp.path().join("native-capsule");
    let receipt = store.export_current(&native).unwrap();
    assert_eq!(receipt.source_schema, 6);
    assert_eq!(
        SqliteLifecycleStore::read_capsule(&native).unwrap(),
        receipt
    );
    assert!(SqliteLifecycleStore::read_held_capsule(&native).is_err());
    let destination = temp.path().join("normal-adoption");
    let (adopted, accepted) = SqliteLifecycleStore::adopt_inactive(
        &receipt,
        &destination,
        &HandoffId::parse("handoff:normal-six").unwrap(),
        "trusted".into(),
    )
    .unwrap();
    assert_eq!(accepted.status, "accepted_inactive");
    assert_eq!(adopted.admin_ownership().unwrap().status, "inactive");
    assert!(adopted.held_projection().is_err());
}
