use lifecycle_codex::{parse_event, read_state_changed_matches};
use lifecycle_core::{
    AttemptId, AuthorityExpiresAt, BuildId, ClockSample, ContextGeneration, GrantId, GrantScope,
    NativeInvocationPurpose, NativeSessionId, ProofRunId, ProviderThreadId, ProviderTurnId,
    Revision, RuntimeIncarnation, SnapshotId, SubjectId, TransitionId, TrustedGrant, WaitBudgetMs,
    WallTimeMs,
};
use lifecycle_store::{NativeTurnRequest, SqliteLifecycleStore, StoreError};
use rusqlite::Connection;
use serde_json::{Value, json};

fn clock() -> ClockSample {
    ClockSample {
        wall: WallTimeMs::new(100),
        wait_budget: WaitBudgetMs::new(1_000),
    }
}

fn fixture() -> (
    tempfile::TempDir,
    SqliteLifecycleStore,
    lifecycle_store::NativeTurnEntry,
    NativeSessionId,
    TransitionId,
) {
    let temp = tempfile::tempdir().unwrap();
    let mut store = SqliteLifecycleStore::open(temp.path(), "issuer:synthetic".into()).unwrap();
    let subject = SubjectId::parse("subject:synthetic").unwrap();
    let context = ContextGeneration::parse("context:synthetic").unwrap();
    let build = BuildId::parse("build:synthetic").unwrap();
    let proof = ProofRunId::parse("proof:synthetic").unwrap();
    store
        .register_subject(
            subject.clone(),
            context.clone(),
            build.clone(),
            proof.clone(),
            clock(),
        )
        .unwrap();
    let grant = GrantId::parse("grant:synthetic").unwrap();
    store
        .install_trusted_grant(TrustedGrant {
            id: grant.clone(),
            issuer: "issuer:synthetic".into(),
            principal: "principal:synthetic".into(),
            subject: subject.clone(),
            context: context.clone(),
            build,
            proof_run: proof,
            scope: GrantScope::parse("native_turn").unwrap(),
            expires: AuthorityExpiresAt::new(WallTimeMs::new(100_000)),
            revision: Revision::new(1),
            revoked: false,
        })
        .unwrap();
    let digest = "a".repeat(64);
    let mut session = store
        .reserve_native_session(
            NativeSessionId::parse("session:synthetic").unwrap(),
            subject,
            context,
            RuntimeIncarnation::parse("incarnation:synthetic").unwrap(),
            &digest,
            &digest,
            &digest,
            SnapshotId::parse("snapshot:synthetic").unwrap(),
            &digest,
            false,
            clock(),
        )
        .unwrap();
    store
        .bind_native_thread(
            &mut session,
            ProviderThreadId::parse("thread:synthetic").unwrap(),
        )
        .unwrap();
    let mut turn = store
        .claim_native_turn(
            &session,
            NativeTurnRequest {
                invocation_id: "invocation:synthetic".into(),
                attempt: AttemptId::parse("native-attempt:synthetic").unwrap(),
                grant,
                grant_revision: 1,
                purpose: NativeInvocationPurpose::DomainWork,
                prompt: b"test".to_vec(),
            },
            "principal:synthetic",
            clock(),
        )
        .unwrap();
    let connection = Connection::open(temp.path().join("lifecycle.sqlite")).unwrap();
    connection.execute_batch("\
        INSERT INTO inputs(input_id,subject_id,sequence,delivery_id,producer_ref,text,text_digest,grant_id,state)
        VALUES('input:synthetic','subject:synthetic',1,'delivery:synthetic','producer:synthetic',X'74657374','digest:synthetic','grant:synthetic','entered');
        INSERT INTO effects(effect_id,subject_id,input_id,expected_revision,state)
        VALUES('effect:synthetic','subject:synthetic','input:synthetic',0,'entered');
        INSERT INTO attempts(attempt_id,effect_id,incarnation_id,state,entered_wall_ms)
        VALUES('queue-attempt:synthetic','effect:synthetic','incarnation:synthetic','entered',100);
    ").unwrap();
    store
        .bind_native_delivery_segment(
            turn.invocation_id(),
            &AttemptId::parse("queue-attempt:synthetic").unwrap(),
            0,
            None,
            None,
        )
        .unwrap();
    store.mark_native_send_intent(&turn).unwrap();
    store
        .bind_native_turn(&mut turn, ProviderTurnId::parse("turn:synthetic").unwrap())
        .unwrap();
    let transition = TransitionId::parse("transition:synthetic").unwrap();
    // The transition command is outside this fixture's scope; this row supplies its owned
    // basis relation. All turn, delivery, ingress, settlement and readiness work uses APIs.
    connection.execute("INSERT INTO transitions(transition_id,subject_id,predecessor_context,stage,basis_revision,created_wall_ms,native_basis_source) VALUES (?1,'subject:synthetic','context:synthetic','quiescing',0,100,'invocation:synthetic')", [transition.as_str()]).unwrap();
    (temp, store, turn, session.id().clone(), transition)
}

fn ingress(
    store: &mut SqliteLifecycleStore,
    session: &NativeSessionId,
    sequence: u64,
    value: Value,
    status: &str,
) {
    let event = parse_event(
        "transport:synthetic".into(),
        sequence,
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let method = value.get("method").and_then(Value::as_str);
    let params = value.get("params");
    let thread = params
        .and_then(|p| p.get("threadId"))
        .and_then(Value::as_str);
    let turn = params
        .and_then(|p| p.get("turnId").or_else(|| p.get("turn")?.get("id")))
        .and_then(Value::as_str);
    store
        .record_native_ingress(
            session,
            &event.transport_session,
            event.sequence,
            &event.raw,
            status,
            method,
            thread,
            turn,
            None,
            clock(),
        )
        .unwrap();
}

#[test]
fn opaque_read_state_keeps_task_settlement_and_readiness_obligations() {
    for case in ["complete", "adverse", "missing_history", "pending_item"] {
        let (_temp, mut store, turn, session, transition) = fixture();
        assert!(!store.task_transition_source_ready(&transition).unwrap());
        // A historical unsupported turnless row retains its status and is outside the
        // turn-bound settlement filter; the selected future policy does not relabel it.
        ingress(
            &mut store,
            &session,
            1,
            json!({"method":"thread/readState/changed","params":{
            "threadId":"thread:synthetic","readState":{"revision":"revision:old","status":"old"}}}),
            "unsupported",
        );
        let opaque = json!({"method":"thread/readState/changed","params":{
            "threadId":"thread:synthetic","readState":{"revision":"revision:new",
            "error":"failed","needsInput":true,"unreadCount":3}}});
        assert!(read_state_changed_matches(&opaque));
        ingress(&mut store, &session, 2, opaque, "qualified");
        ingress(
            &mut store,
            &session,
            3,
            json!({"method":"turn/completed","params":{
            "threadId":"thread:synthetic","turn":{"id":"turn:synthetic","status":"completed"}}}),
            "qualified",
        );
        if case == "adverse" {
            ingress(
                &mut store,
                &session,
                4,
                json!({"method":"turn/failed","params":{
                "threadId":"thread:synthetic","turn":{"id":"turn:synthetic","status":"failed"}}}),
                "qualified",
            );
        }
        if case == "pending_item" {
            ingress(
                &mut store,
                &session,
                5,
                json!({"method":"item/started","params":{
                "threadId":"thread:synthetic","turnId":"turn:synthetic","item":{
                    "id":"item:pending","type":"dynamicToolCall","namespace":"workspace","tool":"snapshot_read"}}}),
                "qualified",
            );
        }
        let final_item = json!({"type":"agentMessage","id":"item:final","text":"done"});
        ingress(
            &mut store,
            &session,
            6,
            json!({"method":"item/completed","params":{
            "threadId":"thread:synthetic","turnId":"turn:synthetic","item":final_item.clone()}}),
            "qualified",
        );
        if case != "missing_history" {
            ingress(
                &mut store,
                &session,
                7,
                json!({"id":7,"result":{"thread":{
                "id":"thread:synthetic","historyMode":"legacy","turns":[{
                    "id":"turn:synthetic","status":"completed","itemsView":"full","items":[final_item]
                }]}}}),
                "qualified",
            );
        }
        let settlement = store.settle_native_turn(
            &turn,
            "done",
            "observed_complete",
            "transport:synthetic",
            7,
            clock(),
        );
        if case == "complete" {
            settlement.unwrap();
            assert!(
                store
                    .settle_native_delivery_segment(turn.invocation_id(), clock())
                    .unwrap()
            );
            assert!(store.task_transition_source_ready(&transition).unwrap());
            assert_eq!(
                store
                    .native_recovery(turn.invocation_id())
                    .unwrap()
                    .unwrap()
                    .class,
                "CompletedSettled"
            );
        } else {
            assert!(
                matches!(
                    settlement,
                    Err(StoreError::ClaimConflict | StoreError::Unavailable)
                ),
                "{case}: {settlement:?}"
            );
            assert!(
                !store.task_transition_source_ready(&transition).unwrap(),
                "{case}"
            );
            assert!(matches!(
                store.settle_native_delivery_segment(turn.invocation_id(), clock()),
                Err(StoreError::ClaimConflict)
            ));
        }
    }
}
