use lifecycle_core::{
    AuthorityExpiresAt, BuildId, ClockSample, ContextGeneration, GrantId, GrantScope, HandoffId,
    NativeSessionId, ProofRunId, ProviderThreadId, Revision, RuntimeIncarnation, SnapshotId,
    SubjectId, TrustedGrant, WaitBudgetMs, WallTimeMs,
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
fn native_session_and_ingress_mutations_invalidate_exported_capsules_and_adoptions() {
    let temporary = tempdir().unwrap();
    let mut source =
        SqliteLifecycleStore::open(&temporary.path().join("source"), "test-issuer".into()).unwrap();
    let subject = SubjectId::parse("subject-native-stale").unwrap();
    let context = ContextGeneration::parse("context-native-stale").unwrap();
    let build = BuildId::parse("build-native-stale").unwrap();
    let proof = ProofRunId::parse("proof-native-stale").unwrap();
    source
        .register_subject(
            subject.clone(),
            context.clone(),
            build.clone(),
            proof.clone(),
            clock(100),
        )
        .unwrap();
    source
        .install_trusted_grant(TrustedGrant {
            id: GrantId::parse("grant-native-stale").unwrap(),
            issuer: "test-issuer".into(),
            principal: "operator:stale".into(),
            subject: subject.clone(),
            context: context.clone(),
            build,
            proof_run: proof,
            scope: GrantScope::NativeTurn,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    let capsule_root = temporary.path().join("capsule");
    let capsule = source.export_current(&capsule_root).unwrap();
    let handoff = HandoffId::parse("handoff-native-stale").unwrap();
    let (_inactive, adoption) = SqliteLifecycleStore::adopt_inactive(
        &capsule,
        &temporary.path().join("destination"),
        &handoff,
        "test-issuer".into(),
    )
    .unwrap();
    let digest = "a".repeat(64);
    let mut unbound_session = source
        .reserve_native_session(
            NativeSessionId::parse("session-native-stale").unwrap(),
            subject,
            context,
            RuntimeIncarnation::parse("incarnation-native-stale").unwrap(),
            &digest,
            &digest,
            &digest,
            SnapshotId::parse("snapshot-native-stale").unwrap(),
            &digest,
            false,
            clock(101),
        )
        .unwrap();
    assert!(
        source.export_current(&capsule_root).is_err(),
        "native session mutation must stale the previous capsule"
    );
    assert!(
        source.fence_source(&handoff, &adoption).is_err(),
        "native session mutation must not fence through stale adoption"
    );
    assert_eq!(source.admin_ownership().unwrap().status, "active");

    source
        .bind_native_thread(
            &mut unbound_session,
            ProviderThreadId::parse("thread-native-stale").unwrap(),
        )
        .unwrap();
    let capsule_after_bind_root = temporary.path().join("capsule-after-bind");
    let capsule_after_bind = source.export_current(&capsule_after_bind_root).unwrap();
    let second_handoff = HandoffId::parse("handoff-ingress-stale").unwrap();
    let (_inactive_second, second_adoption) = SqliteLifecycleStore::adopt_inactive(
        &capsule_after_bind,
        &temporary.path().join("destination-after-bind"),
        &second_handoff,
        "test-issuer".into(),
    )
    .unwrap();
    source
        .record_native_ingress(
            &NativeSessionId::parse("session-native-stale").unwrap(),
            "transport-native-stale",
            1,
            br#"{"method":"warning","params":{"message":"native ingress after export"}}"#,
            "qualified",
            Some("warning"),
            Some("thread-native-stale"),
            None,
            None,
            clock(102),
        )
        .unwrap();
    assert!(
        source.export_current(&capsule_after_bind_root).is_err(),
        "native ingress must stale the previous capsule"
    );
    assert!(
        source
            .fence_source(&second_handoff, &second_adoption)
            .is_err(),
        "native ingress must not fence through stale adoption"
    );
}
