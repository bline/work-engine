use lifecycle_core::{
    BuildId, ClockSample, ContextGeneration, HandoffId, ProofRunId, SubjectId, WaitBudgetMs,
    WallTimeMs,
};
use lifecycle_store::SqliteLifecycleStore;
use std::fs;
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

#[test]
fn untrusted_selector_file_cannot_create_held_custody() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("ordinary");
    let store = SqliteLifecycleStore::open(&root, "trusted".into()).unwrap();
    let original = store.admin_ownership().unwrap();
    drop(store);
    // A caller-authored file is not a sealed C0 session, B1 witness, or store
    // transition. C0 has no public selector-import or witness-mint API.
    fs::write(
        root.join("held-selection.json"),
        br#"{"kind":"held","phase":"selected_recovery_only"}"#,
    )
    .unwrap();
    let reopened = SqliteLifecycleStore::open(&root, "trusted".into()).unwrap();
    assert_eq!(reopened.admin_ownership().unwrap(), original);
    assert!(reopened.held_projection().is_err());
    assert!(!root.join("legacy-held-artifacts").exists());
}
