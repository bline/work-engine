use ed25519_dalek::{Signer, SigningKey};
use lifecycle_admin::recovery::{apply_qualified_fact, inspect_exact, retire_exact_no_retry};
use lifecycle_admin::selection::{FileLaunchSelectionPort, SelectionRecord};
use lifecycle_admin::transfer::transfer_current;
use lifecycle_core::{
    AttemptId, AuthorityExpiresAt, BuildId, ClockSample, ContextGeneration, GrantId, GrantScope,
    HandoffId, NativeInvocationPurpose, NativeSessionId, NativeToolCallId, OperationAttemptId,
    OperationContractId, OperationExecutorId, OperationImplementationId, ProofRunId,
    ProviderThreadId, ProviderTurnId, Revision, RuntimeIncarnation, SnapshotId, SubjectId,
    TrustedGrant, WaitBudgetMs, WallTimeMs,
};
use lifecycle_store::{
    DispositionGrant, EffectCoverage, ExactAttemptSelector, NativeOperationClaim,
    NativeTurnRequest, QualifiedRecoveryFact, SignedDispositionRequest, SnapshotReadRequest,
    SqliteLifecycleStore,
};
use std::fs;
use tempfile::tempdir;
use work_engine_types::CodecContract;

fn clock(wall: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(wall),
        wait_budget: WaitBudgetMs::new(100),
    }
}

fn fixture_sign(bytes: &[u8]) -> Vec<u8> {
    let hex = include_str!("fixtures/ed25519-rfc8032-vector1-seed.hex").trim();
    let seed: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
        .collect();
    SigningKey::from_bytes(&seed.try_into().unwrap())
        .sign(bytes)
        .to_bytes()
        .to_vec()
}

#[test]
fn persisted_ingress_is_applied_to_one_original_attempt_without_new_entry() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let mut store = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let subject = SubjectId::parse("subject-qualified").unwrap();
    let context = ContextGeneration::parse("context-qualified").unwrap();
    let build = BuildId::parse("build-qualified").unwrap();
    let proof = ProofRunId::parse("proof-qualified").unwrap();
    store
        .register_subject(
            subject.clone(),
            context.clone(),
            build.clone(),
            proof.clone(),
            clock(100),
        )
        .unwrap();
    let grant = GrantId::parse("grant-qualified").unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: grant.clone(),
            issuer: "test-issuer".into(),
            principal: "operator:qualified".into(),
            subject: subject.clone(),
            context: context.clone(),
            build: build.clone(),
            proof_run: proof.clone(),
            scope: GrantScope::NativeTurn,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: GrantId::parse("grant-snapshot-qualified").unwrap(),
            issuer: "test-issuer".into(),
            principal: "operator:qualified".into(),
            subject: subject.clone(),
            context: context.clone(),
            build,
            proof_run: proof,
            scope: GrantScope::SnapshotRead,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    let digest = "a".repeat(64);
    let manifest = serde_json::to_vec(&serde_json::json!({
        "codec":"workspace-snapshot-manifest-v1",
        "id":"snapshot-qualified",
        "source_revision":"source-qualified",
        "members":[{"id":"m1","sha256":CodecContract::BinaryArtifactV1.digest_binary(b"abc").unwrap().hex(),"length":3}]
    }))
    .unwrap();
    let staged = store.stage_artifact(subject.clone(), &manifest).unwrap();
    let manifest_digest = store
        .publish_artifact(staged, clock(100))
        .unwrap()
        .digest_hex;
    let session_id = NativeSessionId::parse("session-qualified").unwrap();
    let incarnation = RuntimeIncarnation::parse("incarnation-qualified").unwrap();
    let mut session = store
        .reserve_native_session(
            session_id.clone(),
            subject.clone(),
            context,
            incarnation.clone(),
            &digest,
            &digest,
            &digest,
            SnapshotId::parse("snapshot-qualified").unwrap(),
            &manifest_digest,
            false,
            clock(101),
        )
        .unwrap();
    store
        .bind_native_thread(
            &mut session,
            ProviderThreadId::parse("thread-qualified").unwrap(),
        )
        .unwrap();
    let mut turn = store
        .claim_native_turn(
            &session,
            NativeTurnRequest {
                invocation_id: "invocation-qualified".into(),
                attempt: AttemptId::parse("attempt-qualified").unwrap(),
                grant,
                grant_revision: 1,
                purpose: NativeInvocationPurpose::DomainWork,
                prompt: b"fixture prompt".to_vec(),
            },
            "operator:qualified",
            clock(102),
        )
        .unwrap();
    store.mark_native_send_intent(&turn).unwrap();
    store
        .bind_native_turn(&mut turn, ProviderTurnId::parse("turn-qualified").unwrap())
        .unwrap();
    let raw = br#"{"method":"warning","params":{"message":"read-only observation"}}"#;
    store
        .record_native_ingress(
            &session_id,
            "transport-qualified",
            1,
            raw,
            "qualified",
            Some("warning"),
            Some("thread-qualified"),
            Some("turn-qualified"),
            None,
            clock(103),
        )
        .unwrap();
    let tool_call = br#"{"method":"item/tool/call","id":2,"params":{"threadId":"thread-qualified","turnId":"turn-qualified","callId":"call-qualified","namespace":"workspace","tool":"snapshot_read","arguments":{"snapshot_id":"snapshot-qualified","member_id":"m1","offset":0,"length":3}}}"#;
    store
        .record_native_ingress(
            &session_id,
            "transport-qualified",
            2,
            tool_call,
            "qualified",
            Some("item/tool/call"),
            Some("thread-qualified"),
            Some("turn-qualified"),
            None,
            clock(103),
        )
        .unwrap();
    let make_request = |contract| SnapshotReadRequest {
        attempt: OperationAttemptId::parse("opattempt-qualified").unwrap(),
        call: NativeToolCallId::parse("call-qualified").unwrap(),
        transport_session: "transport-qualified".into(),
        sequence: 2,
        grant: GrantId::parse("grant-snapshot-qualified").unwrap(),
        grant_revision: 1,
        contract: OperationContractId::parse(contract).unwrap(),
        implementation: OperationImplementationId::parse("rust.snapshot.read.v1").unwrap(),
        executor: OperationExecutorId::parse("lifecycle.service.snapshot").unwrap(),
        snapshot: SnapshotId::parse("snapshot-qualified").unwrap(),
        member_id: "m1".into(),
        range_start: 0,
        range_length: 3,
    };
    assert!(
        store
            .claim_snapshot_read(
                &turn,
                make_request("unknown.remote.effect.v1"),
                "operator:qualified",
                clock(103),
            )
            .is_err()
    );
    let NativeOperationClaim::Entered(first_child) = store
        .claim_snapshot_read(
            &turn,
            make_request("workspace.snapshot.read.v1"),
            "operator:qualified",
            clock(103),
        )
        .unwrap()
    else {
        panic!("expected entered child")
    };
    let native = store
        .native_recovery("invocation-qualified")
        .unwrap()
        .unwrap();
    assert_eq!(native.admitted_children, 1);
    assert_eq!(native.unsettled_children, 1);
    let other_subject = SubjectId::parse("subject-other").unwrap();
    let other_context = ContextGeneration::parse("context-other").unwrap();
    store
        .register_subject(
            other_subject.clone(),
            other_context.clone(),
            BuildId::parse("build-other").unwrap(),
            ProofRunId::parse("proof-other").unwrap(),
            clock(100),
        )
        .unwrap();
    let other_session_id = NativeSessionId::parse("session-other").unwrap();
    let mut other_session = store
        .reserve_native_session(
            other_session_id.clone(),
            other_subject,
            other_context,
            RuntimeIncarnation::parse("incarnation-other").unwrap(),
            &"b".repeat(64),
            &"b".repeat(64),
            &"b".repeat(64),
            SnapshotId::parse("snapshot-other").unwrap(),
            &"b".repeat(64),
            false,
            clock(101),
        )
        .unwrap();
    store
        .bind_native_thread(
            &mut other_session,
            ProviderThreadId::parse("thread-other").unwrap(),
        )
        .unwrap();
    assert!(
        store
            .record_native_ingress(
                &other_session_id,
                "transport-other",
                1,
                raw,
                "qualified",
                Some("warning"),
                Some("thread-qualified"),
                Some("turn-qualified"),
                None,
                clock(103),
            )
            .is_err()
    );
    let revision = store
        .subject_projection(&subject, None, None)
        .unwrap()
        .revision;
    let selector = ExactAttemptSelector {
        store_id: store.admin_store_identity().unwrap(),
        source_lineage: None,
        subject_id: subject.as_str().into(),
        input_id: None,
        delivery_id: None,
        effect_id: None,
        attempt_id: Some("attempt-qualified".into()),
        native_session_id: Some(session_id.as_str().into()),
        invocation_id: Some("invocation-qualified".into()),
        incarnation_id: Some(incarnation.as_str().into()),
        operation_contract_id: None,
        operation_implementation_id: None,
        operation_executor_id: None,
        profile_digest: Some(digest),
        claim_revision: revision,
        observation_revision: 0,
    };
    let before = inspect_exact(&store, &selector).unwrap();
    assert_eq!(before.effect_coverage, EffectCoverage::Unknown);
    let entries_before = store.load_recovery_entries().unwrap();
    let fact = QualifiedRecoveryFact {
        command_id: "fact-qualified".into(),
        selector: selector.clone(),
        expected_observation_revision: 0,
        source_profile: "s5-native-ingress".into(),
        transport_session: "transport-qualified".into(),
        transport_sequence: 1,
        provider_thread_id: Some("thread-qualified".into()),
        provider_turn_id: Some("turn-qualified".into()),
        native_call_id: None,
        raw_sha256: CodecContract::BinaryArtifactV1
            .digest_binary(raw)
            .unwrap()
            .hex(),
        raw_artifact_sha256: None,
        codec: "native-ingress.raw.v1".into(),
        observed_wall_ms: 103,
        availability: before.availability,
        outcome: before.outcome,
        settlement: before.settlement,
        effect_coverage: before.effect_coverage,
    };
    let pre_fact_capsule_root = temporary.path().join("capsule-before-fact");
    let pre_fact_capsule = store.export_current(&pre_fact_capsule_root).unwrap();
    let pre_fact_handoff = HandoffId::parse("handoff-before-fact").unwrap();
    let (_pre_fact_destination, pre_fact_adoption) = SqliteLifecycleStore::adopt_inactive(
        &pre_fact_capsule,
        &temporary.path().join("destination-before-fact"),
        &pre_fact_handoff,
        "test-issuer".into(),
    )
    .unwrap();
    let receipt = apply_qualified_fact(&mut store, &fact).unwrap();
    assert!(receipt.applied);
    assert_eq!(receipt.observation_revision, 1);
    assert!(
        store.export_current(&pre_fact_capsule_root).is_err(),
        "qualified recovery mutation must stale its earlier capsule"
    );
    assert!(
        store
            .fence_source(&pre_fact_handoff, &pre_fact_adoption)
            .is_err(),
        "qualified recovery mutation must not fence through a stale adoption"
    );
    assert_eq!(apply_qualified_fact(&mut store, &fact).unwrap(), receipt);
    let mut changed = fact.clone();
    changed.raw_sha256 = "b".repeat(64);
    assert!(apply_qualified_fact(&mut store, &changed).is_err());
    assert_eq!(store.load_recovery_entries().unwrap(), entries_before);
    let mut revised = selector;
    revised.observation_revision = 1;
    assert_eq!(
        inspect_exact(&store, &revised)
            .unwrap()
            .observation_revision,
        1
    );

    store
        .install_trusted_grant(TrustedGrant {
            id: GrantId::parse("grant-retire-unknown-native").unwrap(),
            issuer: "test-issuer".into(),
            principal: "operator:qualified".into(),
            subject,
            context: ContextGeneration::parse("context-qualified").unwrap(),
            build: BuildId::parse("build-qualified").unwrap(),
            proof_run: ProofRunId::parse("proof-qualified").unwrap(),
            scope: GrantScope::RetireNoRetry,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    let grant = DispositionGrant {
        codec: "json-exact-ed25519.v1".into(),
        domain: "lifecycle.disposition.v1".into(),
        grant_id: "grant-retire-unknown-native".into(),
        grant_revision: 1,
        issuer: "test-issuer".into(),
        principal: "operator:qualified".into(),
        store_id: revised.store_id.clone(),
        subject_id: revised.subject_id.clone(),
        claim_id: "native:invocation-qualified".into(),
        observation_revision: revised.observation_revision,
        scope: "retire_no_retry".into(),
        expires_wall_ms: 10_000,
        effect_coverage: EffectCoverage::Unknown,
    };
    let bytes = serde_json::to_vec(&grant).unwrap();
    let request = SignedDispositionRequest {
        command_id: "retire-unaccounted-native".into(),
        selector: revised,
        grant_bytes: bytes.clone(),
        signature: fixture_sign(&bytes),
        key_id: "test-rfc8032-1".into(),
        clock_wall_ms: 200,
        expected_store_fence_revision: store.admin_ownership().unwrap().revision,
    };
    assert!(retire_exact_no_retry(&mut store, &request).is_err());
    store
        .commit_snapshot_result(&first_child, b"abc", clock(200))
        .unwrap();

    // Retain both an entered parent and an unsent turn across a real owner
    // handoff. Neither held entry may be used to enter new work on the fence.
    let unsent_subject = SubjectId::parse("subject-unsent-fence").unwrap();
    let unsent_context = ContextGeneration::parse("context-unsent-fence").unwrap();
    let unsent_build = BuildId::parse("build-unsent-fence").unwrap();
    let unsent_proof = ProofRunId::parse("proof-unsent-fence").unwrap();
    store
        .register_subject(
            unsent_subject.clone(),
            unsent_context.clone(),
            unsent_build.clone(),
            unsent_proof.clone(),
            clock(190),
        )
        .unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: GrantId::parse("grant-unsent-fence").unwrap(),
            issuer: "test-issuer".into(),
            principal: "operator:qualified".into(),
            subject: unsent_subject.clone(),
            context: unsent_context.clone(),
            build: unsent_build,
            proof_run: unsent_proof,
            scope: GrantScope::NativeTurn,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    let mut unsent_session = store
        .reserve_native_session(
            NativeSessionId::parse("session-unsent-fence").unwrap(),
            unsent_subject,
            unsent_context,
            RuntimeIncarnation::parse("incarnation-unsent-fence").unwrap(),
            &"c".repeat(64),
            &"c".repeat(64),
            &"c".repeat(64),
            SnapshotId::parse("snapshot-unsent-fence").unwrap(),
            &"c".repeat(64),
            false,
            clock(191),
        )
        .unwrap();
    store
        .bind_native_thread(
            &mut unsent_session,
            ProviderThreadId::parse("thread-unsent-fence").unwrap(),
        )
        .unwrap();
    let unsent = store
        .claim_native_turn(
            &unsent_session,
            NativeTurnRequest {
                invocation_id: "invocation-unsent-fence".into(),
                attempt: AttemptId::parse("attempt-unsent-fence").unwrap(),
                grant: GrantId::parse("grant-unsent-fence").unwrap(),
                grant_revision: 1,
                purpose: NativeInvocationPurpose::DomainWork,
                prompt: b"unsent fixture".to_vec(),
            },
            "operator:qualified",
            clock(200),
        )
        .unwrap();
    let second_tool_call = br#"{"method":"item/tool/call","id":3,"params":{"threadId":"thread-qualified","turnId":"turn-qualified","callId":"call-before-fence","namespace":"workspace","tool":"snapshot_read","arguments":{"snapshot_id":"snapshot-qualified","member_id":"m1","offset":0,"length":3}}}"#;
    store
        .record_native_ingress(
            &session_id,
            "transport-qualified",
            3,
            second_tool_call,
            "qualified",
            Some("item/tool/call"),
            Some("thread-qualified"),
            Some("turn-qualified"),
            None,
            clock(201),
        )
        .unwrap();
    let mut child_before_fence = make_request("workspace.snapshot.read.v1");
    child_before_fence.attempt = OperationAttemptId::parse("opattempt-before-fence").unwrap();
    child_before_fence.call = NativeToolCallId::parse("call-before-fence").unwrap();
    child_before_fence.sequence = 3;
    let NativeOperationClaim::Entered(second_child) = store
        .claim_snapshot_read(&turn, child_before_fence, "operator:qualified", clock(201))
        .unwrap()
    else {
        panic!("expected entered second child")
    };
    store
        .commit_snapshot_result(&second_child, b"abc", clock(201))
        .unwrap();
    let third_tool_call = br#"{"method":"item/tool/call","id":4,"params":{"threadId":"thread-qualified","turnId":"turn-qualified","callId":"call-after-fence","namespace":"workspace","tool":"snapshot_read","arguments":{"snapshot_id":"snapshot-qualified","member_id":"m1","offset":0,"length":3}}}"#;
    store
        .record_native_ingress(
            &session_id,
            "transport-qualified",
            4,
            third_tool_call,
            "qualified",
            Some("item/tool/call"),
            Some("thread-qualified"),
            Some("turn-qualified"),
            None,
            clock(202),
        )
        .unwrap();
    let selector_root = temporary.path().join("selector");
    fs::create_dir(&selector_root).unwrap();
    let expected = SelectionRecord {
        generation: 0,
        store_id: store.admin_store_identity().unwrap(),
        root: store.root().to_owned(),
        handoff_id: None,
        capsule_digest: None,
    };
    let mut selection =
        FileLaunchSelectionPort::initialize(&selector_root, expected.clone()).unwrap();
    transfer_current(
        &mut store,
        &mut selection,
        &expected,
        &temporary.path().join("capsule"),
        &temporary.path().join("destination"),
        &HandoffId::parse("handoff-native-fence").unwrap(),
        "test-issuer".into(),
    )
    .unwrap();
    assert_eq!(store.admin_ownership().unwrap().status, "fenced");
    let parent_before_rejected_claim = store
        .native_recovery("invocation-qualified")
        .unwrap()
        .unwrap();
    let unsent_before_rejected_send = store
        .native_recovery("invocation-unsent-fence")
        .unwrap()
        .unwrap();
    assert_eq!(parent_before_rejected_claim.admitted_children, 2);
    assert_eq!(parent_before_rejected_claim.unsettled_children, 0);
    assert!(!unsent_before_rejected_send.send_intent);
    let mut child_after_fence = make_request("workspace.snapshot.read.v1");
    child_after_fence.attempt = OperationAttemptId::parse("opattempt-after-fence").unwrap();
    child_after_fence.call = NativeToolCallId::parse("call-after-fence").unwrap();
    child_after_fence.sequence = 4;
    assert!(
        store
            .claim_snapshot_read(&turn, child_after_fence, "operator:qualified", clock(202))
            .is_err(),
        "retained parent must not enter a new child after source fence"
    );
    assert!(
        store.mark_native_send_intent(&unsent).is_err(),
        "retained unsent turn must not send after source fence"
    );
    assert_eq!(
        store
            .native_recovery("invocation-qualified")
            .unwrap()
            .unwrap(),
        parent_before_rejected_claim
    );
    assert_eq!(
        store
            .native_recovery("invocation-unsent-fence")
            .unwrap()
            .unwrap(),
        unsent_before_rejected_send
    );
}
