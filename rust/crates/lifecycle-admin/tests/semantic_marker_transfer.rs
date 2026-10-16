use std::fs;
use std::path::Path;

use lifecycle_admin::selection::{FileLaunchSelectionPort, SelectionRecord};
use lifecycle_admin::transfer::{resume_transfer, transfer_current};
use lifecycle_core::{
    AttemptId, AuthorityExpiresAt, BuildId, ClockSample, Command, CommandAdmission, CommandId,
    CommandOutcome, CommandRequest, ContextGeneration, GrantId, GrantScope, HandoffId,
    NativeInvocationPurpose, NativeSessionId, ProofRunId, ProviderThreadId, ProviderTurnId,
    Revision, RuntimeIncarnation, SnapshotId, SubjectId, TransitionId, TrustedGrant, WaitBudgetMs,
    WallTimeMs,
};
use lifecycle_store::SqliteLifecycleStore;
use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use work_engine_types::CodecContract;

fn clock(wall: i64) -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(wall),
        wait_budget: WaitBudgetMs::new(100),
    }
}

fn create_semantic_required_transition(store: &mut SqliteLifecycleStore) -> TransitionId {
    let subject = SubjectId::parse("subject-semantic-marker").unwrap();
    let context = ContextGeneration::parse("context-semantic-marker").unwrap();
    let build = BuildId::parse("build-semantic-marker").unwrap();
    let proof = ProofRunId::parse("proof-semantic-marker").unwrap();
    store
        .register_subject(
            subject.clone(),
            context.clone(),
            build.clone(),
            proof.clone(),
            clock(100),
        )
        .unwrap();
    for (id, scope) in [
        ("grant-marker-native", GrantScope::NativeTurn),
        ("grant-marker-replace", GrantScope::RequestReplacement),
    ] {
        store
            .install_trusted_grant(TrustedGrant {
                id: GrantId::parse(id).unwrap(),
                issuer: "test-issuer".into(),
                principal: "operator:marker".into(),
                subject: subject.clone(),
                context: context.clone(),
                build: build.clone(),
                proof_run: proof.clone(),
                scope,
                expires: AuthorityExpiresAt::new(WallTimeMs::new(10_000)),
                revision: Revision::new(1),
                revoked: false,
            })
            .unwrap();
    }
    let digest = "a".repeat(64);
    let session_id = NativeSessionId::parse("session-semantic-marker").unwrap();
    let mut session = store
        .reserve_native_session(
            session_id.clone(),
            subject.clone(),
            context.clone(),
            RuntimeIncarnation::parse("incarnation-semantic-marker").unwrap(),
            &digest,
            &digest,
            &digest,
            SnapshotId::parse("snapshot-semantic-marker").unwrap(),
            &digest,
            false,
            clock(101),
        )
        .unwrap();
    store
        .bind_native_thread(
            &mut session,
            ProviderThreadId::parse("thread-semantic-marker").unwrap(),
        )
        .unwrap();
    let mut turn = store
        .claim_native_turn(
            &session,
            lifecycle_store::NativeTurnRequest {
                invocation_id: "invocation-semantic-marker".into(),
                attempt: AttemptId::parse("attempt-semantic-marker").unwrap(),
                grant: GrantId::parse("grant-marker-native").unwrap(),
                grant_revision: 1,
                purpose: NativeInvocationPurpose::DomainWork,
                prompt: b"controlled marker fixture".to_vec(),
            },
            "operator:marker",
            clock(102),
        )
        .unwrap();
    store.mark_native_send_intent(&turn).unwrap();
    store
        .bind_native_turn(
            &mut turn,
            ProviderTurnId::parse("turn-semantic-marker").unwrap(),
        )
        .unwrap();
    let final_item = json!({"type":"agentMessage","id":"final-semantic-marker","text":"done"});
    let item = json!({"method":"item/completed","params":{
        "threadId":"thread-semantic-marker","turnId":"turn-semantic-marker","item":final_item.clone()
    }});
    let terminal = json!({"method":"turn/completed","params":{
        "threadId":"thread-semantic-marker","turn":{"id":"turn-semantic-marker","status":"completed"}
    }});
    let history = json!({"id":3,"result":{"thread":{
        "id":"thread-semantic-marker","historyMode":"legacy","turns":[{
            "id":"turn-semantic-marker","status":"completed","itemsView":"full","items":[final_item]
        }]
    }}});
    for (sequence, raw, method, thread, turn_id) in [
        (
            1,
            serde_json::to_vec(&item).unwrap(),
            Some("item/completed"),
            Some("thread-semantic-marker"),
            Some("turn-semantic-marker"),
        ),
        (
            2,
            serde_json::to_vec(&terminal).unwrap(),
            Some("turn/completed"),
            Some("thread-semantic-marker"),
            Some("turn-semantic-marker"),
        ),
        (3, serde_json::to_vec(&history).unwrap(), None, None, None),
    ] {
        store
            .record_native_ingress(
                &session_id,
                "transport-semantic-marker",
                sequence,
                &raw,
                "qualified",
                method,
                thread,
                turn_id,
                None,
                clock(103),
            )
            .unwrap();
    }
    store
        .settle_native_turn(
            &turn,
            "done",
            "observed_complete",
            "transport-semantic-marker",
            3,
            clock(104),
        )
        .unwrap();

    let revision = store
        .subject_projection(&subject, None, None)
        .unwrap()
        .revision;
    let basis = json!({
        "protocol_version":1,
        "principal_ref":"operator:marker",
        "command_id":"command-marker-replace",
        "subject_id":subject.as_str(),
        "context_generation":context.as_str(),
        "build_id":build.as_str(),
        "proof_run_id":proof.as_str(),
        "grant_ref":"grant-marker-replace",
        "expected_revision":revision.to_string(),
        "kind":"request_replacement",
        "payload":{"reason":"controlled semantic marker transfer"}
    });
    let request = CommandRequest::new(
        CommandId::parse("command-marker-replace").unwrap(),
        subject,
        context,
        build,
        proof,
        GrantId::parse("grant-marker-replace").unwrap(),
        Revision::new(revision),
        CodecContract::LifecycleCommandV1
            .digest_json(&basis)
            .unwrap(),
        Command::RequestReplacement,
    )
    .unwrap();
    let admission = CommandAdmission::new(request, "operator:marker".into(), basis, None).unwrap();
    let result = store.apply_checked_command(admission, clock(105)).unwrap();
    let CommandOutcome::ReplacementRequested { transition } = result.outcome else {
        panic!("native completed boundary did not request replacement");
    };
    store.require_semantic_transition(&transition).unwrap();
    assert!(store.semantic_transition_required(&transition).unwrap());
    transition
}

#[test]
fn semantic_required_marker_survives_export_adoption_and_current_rollback() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let capsule_one = temporary.path().join("capsule-one");
    let candidate_root = temporary.path().join("candidate");
    let capsule_two = temporary.path().join("capsule-two");
    let rollback_root = temporary.path().join("rollback");
    let selector_root = temporary.path().join("selector");
    fs::create_dir(&selector_root).unwrap();
    let mut source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    let transition = create_semantic_required_transition(&mut source);
    let expected = SelectionRecord {
        generation: 0,
        store_id: source.admin_store_identity().unwrap(),
        root: source.root().to_owned(),
        handoff_id: None,
        capsule_digest: None,
    };
    let mut selection =
        FileLaunchSelectionPort::initialize(&selector_root, expected.clone()).unwrap();
    let first = transfer_current(
        &mut source,
        &mut selection,
        &expected,
        &capsule_one,
        &candidate_root,
        &HandoffId::parse("handoff-marker-one").unwrap(),
        "test-issuer".into(),
    )
    .unwrap();
    assert_eq!(first.capsule.source_schema, 6);
    assert!(SqliteLifecycleStore::read_capsule(&capsule_one).is_ok());
    let mut candidate = SqliteLifecycleStore::open(&candidate_root, "test-issuer".into()).unwrap();
    assert!(candidate.semantic_transition_required(&transition).unwrap());
    let second = transfer_current(
        &mut candidate,
        &mut selection,
        &first.selected,
        &capsule_two,
        &rollback_root,
        &HandoffId::parse("handoff-marker-two").unwrap(),
        "test-issuer".into(),
    )
    .unwrap();
    assert_eq!(second.selected.generation, 2);
    assert!(SqliteLifecycleStore::read_capsule(&capsule_two).is_ok());
    let rollback = SqliteLifecycleStore::open(&rollback_root, "test-issuer".into()).unwrap();
    assert!(rollback.semantic_transition_required(&transition).unwrap());
    assert_eq!(rollback.admin_ownership().unwrap().status, "active");
    assert_eq!(source.admin_ownership().unwrap().status, "fenced");
    assert_eq!(candidate.admin_ownership().unwrap().status, "fenced");
}

fn tree_bytes(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut result = std::collections::BTreeMap::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    result
}

fn source_custody_revision(root: &Path) -> u64 {
    rusqlite::Connection::open_with_flags(
        root.join("lifecycle.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .unwrap()
    .query_row(
        "SELECT revision FROM admin_custody_revision WHERE singleton=1",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

fn rewrite_capsule_manifest(receipt: &mut lifecycle_store::CapsuleReceipt) {
    receipt.capsule_sha256.clear();
    receipt.capsule_sha256 = format!("{:x}", Sha256::digest(serde_json::to_vec(receipt).unwrap()));
    fs::write(
        receipt.capsule_root.join("manifest.json"),
        serde_json::to_vec(receipt).unwrap(),
    )
    .unwrap();
}

#[test]
fn resume_transfer_refuses_rehashed_capsule_with_stale_snapshot_revision_without_mutation() {
    let temporary = tempdir().unwrap();
    let source_root = temporary.path().join("source");
    let capsule_root = temporary.path().join("capsule");
    let destination_root = temporary.path().join("destination");
    let selector_root = temporary.path().join("selector");
    fs::create_dir(&selector_root).unwrap();
    let mut source = SqliteLifecycleStore::open(&source_root, "test-issuer".into()).unwrap();
    create_semantic_required_transition(&mut source);
    let expected = SelectionRecord {
        generation: 0,
        store_id: source.admin_store_identity().unwrap(),
        root: source.root().to_owned(),
        handoff_id: None,
        capsule_digest: None,
    };
    let mut selection =
        FileLaunchSelectionPort::initialize(&selector_root, expected.clone()).unwrap();
    let mut capsule = source.export_current(&capsule_root).unwrap();
    source
        .register_subject(
            SubjectId::parse("subject-after-capsule").unwrap(),
            ContextGeneration::parse("context-after-capsule").unwrap(),
            BuildId::parse("build-after-capsule").unwrap(),
            ProofRunId::parse("proof-after-capsule").unwrap(),
            clock(200),
        )
        .unwrap();
    let current_revision = source_custody_revision(&source_root);
    assert!(current_revision > capsule.source_revision);
    capsule.source_revision = current_revision;
    rewrite_capsule_manifest(&mut capsule);

    let source_before = tree_bytes(&source_root);
    let capsule_before = tree_bytes(&capsule_root);
    let selection_before = fs::read(selector_root.join("selection.json")).unwrap();
    assert!(
        resume_transfer(
            &mut source,
            &mut selection,
            &expected,
            &capsule_root,
            &destination_root,
            &HandoffId::parse("handoff:stale-resume").unwrap(),
            "test-issuer".into(),
        )
        .is_err()
    );
    assert!(!destination_root.exists());
    assert_eq!(source.admin_ownership().unwrap().status, "active");
    assert_eq!(tree_bytes(&source_root), source_before);
    assert_eq!(tree_bytes(&capsule_root), capsule_before);
    assert_eq!(
        fs::read(selector_root.join("selection.json")).unwrap(),
        selection_before
    );
}
