// This module is compiled inside the actor's unit boundary so the fixture can
// drive the actual private actor and runner without exposing a product route.
use super::*;
use crate::{NativeConfig, NativeProviderRoute, PrincipalConfig, SemanticNativeConfig};
use rusqlite::params;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Mutex,
};

const CARD: &str = r#"{"synthetic_fixture":"v1","card":{"id":"card-1","objective":"Compare synthetic red and blue","constraints":["fixture only","no external inference"],"basis":["basis-1"]},"request":{"id":"request-1","card":"card-1","question":"Which synthetic option matches basis-1?"},"reconstruction":{"id":"reconstruction-1","request":"request-1","basis":"basis-1","facts":{"red":1,"blue":2},"complete":true}}"#;
type PeerBytes = Arc<Mutex<Vec<(Vec<u8>, Vec<u8>)>>>;
fn dir(path: &Path) {
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}
fn http(stream: &mut TcpStream) -> (Vec<u8>, Vec<u8>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut header = Vec::new();
    let mut b = [0];
    while !header.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut b).unwrap();
        header.push(b[0]);
        assert!(header.len() < 128 * 1024);
    }
    let header = String::from_utf8(header).unwrap();
    assert!(!header.to_ascii_lowercase().contains("authorization:"));
    let length = header
        .lines()
        .find_map(|l| {
            l.split_once(':')
                .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .map(|(_, v)| v.trim().parse::<usize>().unwrap())
        })
        .unwrap();
    assert!(length < 8 * 1024 * 1024);
    let mut wire = vec![0; length];
    stream.read_exact(&mut wire).unwrap();
    let decoded = if header
        .to_ascii_lowercase()
        .contains("content-encoding: zstd")
    {
        zstd::stream::decode_all(wire.as_slice()).unwrap()
    } else {
        wire.clone()
    };
    (wire, decoded)
}
fn peer(
    listener: TcpListener,
    received: PeerBytes,
    stop: Arc<AtomicBool>,
    mode: String,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(400);
        while !stop.load(Ordering::SeqCst) && Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let bytes = http(&mut stream);
                    let mut seen = received.lock().unwrap();
                    seen.push(bytes);
                    let n = seen.len();
                    drop(seen);
                    if mode == "lost" {
                        continue;
                    }
                    let id = format!("input-response-{n}");
                    let output = if n == 1 {
                        json!({"type":"custom_tool_call","call_id":"fixture-code","name":"exec","input":"text('VISIBLE_v1');"})
                    } else {
                        json!({"type":"message","id":"fixture-final","role":"assistant","content":[{"type":"output_text","text":"synthetic done"}]})
                    };
                    let events = [
                        json!({"type":"response.created","response":{"id":id}}),
                        json!({"type":"response.output_item.done","output_index":0,"item":output.clone()}),
                        json!({"type":"response.completed","response":{"id":id,"status":"completed","output":[output],"usage":{"input_tokens":0,"output_tokens":0,"total_tokens":0}}}),
                    ];
                    let body = events
                        .iter()
                        .map(|v| format!("data: {v}\n\n"))
                        .collect::<String>();
                    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                    stream.flush().unwrap();
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(e) => panic!("peer accept: {e}"),
            }
        }
    })
}
struct PeerGuard {
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl PeerGuard {
    fn collect(&mut self) -> std::thread::Result<()> {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().map_or(Ok(()), |worker| worker.join())
    }
}
impl Drop for PeerGuard {
    fn drop(&mut self) {
        let _ = self.collect();
    }
}
fn config(root: &Path, url: String) -> ServiceConfig {
    let now = clock_sample().wall.get();
    let grants = [
        ("grant:turn", "native_turn"),
        ("grant:snapshot", "snapshot_read"),
        ("grant:input", "enqueue_input"),
    ]
    .map(|(id, scope)| crate::GrantConfig {
        grant_ref: id.into(),
        context_generation: "context:input".into(),
        scope: scope.into(),
        expires_wall_ms: now + 600_000,
        revision: 1,
    })
    .to_vec();
    let semantic:SemanticNativeConfig=serde_json::from_value(json!({"capture_id":"capture:input","compile_session_id":"session:compile","verify_session_id":"session:verify","successor_verify_session_id":"session:successor","compile_invocation_id":"invocation:compile","compile_attempt_id":"attempt:compile","candidate_verify_invocation_id":"invocation:verify","candidate_verify_attempt_id":"attempt:verify","successor_verify_invocation_id":"invocation:successor","successor_verify_attempt_id":"attempt:successor","challenge_id":"challenge:input","challenge_text":"synthetic","control_grant_ref":"grant:control","control_grant_revision":1,"successor_control_grant_ref":"grant:next","successor_control_grant_revision":1})).unwrap();
    ServiceConfig {
        profile: "retained-repository-analysis-v1".into(),
        store_root: root.join("store"),
        socket_path: root.join("service.sock"),
        peer_socket_path: root.join("unused.sock"),
        verifier_executable: None,
        verifier_ledger_path: None,
        proof_barrier_dir: None,
        trusted_issuer: "issuer:input".into(),
        principal: PrincipalConfig {
            uid: 1000,
            principal_ref: "principal:input".into(),
            producer_ref: "producer:input".into(),
        },
        subject_id: "subject:input".into(),
        context_generation: "context:input".into(),
        build_id: "build:input".into(),
        proof_run_id: "proof:input".into(),
        grants,
        max_executions: 1,
        max_wait_ms: 1000,
        verifier_proof_ms: None,
        verifier_cleanup_ms: None,
        selected_restart: None,
        native: Some(NativeConfig {
            executable: PathBuf::from("/native-app-server"),
            root: root.join("native"),
            provider_route: NativeProviderRoute::NativeSimProofV1,
            chatgpt_access_source: None,
            simulator_base_url: url,
            session_id: "session:input".into(),
            invocation_id: "invocation:unused".into(),
            attempt_id: "attempt:unused".into(),
            prompt: CARD.into(),
            turn_grant_ref: "grant:turn".into(),
            turn_grant_revision: 1,
            snapshot_grant_ref: "grant:snapshot".into(),
            snapshot_grant_revision: 1,
            same_native_session_required: false,
            followup_turns: Vec::new(),
            snapshot: workspace_snapshot_read::TrustedSnapshot {
                id: "snapshot:input".into(),
                source_revision: "synthetic:input".into(),
                members: vec![workspace_snapshot_read::TrustedMember {
                    id: "member".into(),
                    sha256: CodecContract::BinaryArtifactV1
                        .digest_binary(b"synthetic")
                        .unwrap()
                        .hex(),
                    bytes: b"synthetic".to_vec(),
                }],
            },
            semantic: Some(semantic),
            semantic_contexts: Vec::new(),
        }),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires exact provider-disabled Docker composition"]
async fn owned_native_join() {
    let root = PathBuf::from(std::env::var("WE_NATIVE_INPUT_OUT").unwrap());
    let mode = std::env::var("WE_NATIVE_INPUT_MODE").unwrap_or_else(|_| "http".into());
    dir(&root);
    dir(&root.join("store"));
    dir(&root.join("native"));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let received = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let worker = peer(listener, received.clone(), stop.clone(), mode.clone());
    let mut peer_guard = PeerGuard {
        stop: stop.clone(),
        worker: Some(worker),
    };
    let config = config(&root, url);
    let mut actor = Actor::new(
        config.clone(),
        Some(Arc::new(super::restart_epoch_tests::EmptyMaterial)),
        Some(Arc::new(super::restart_epoch_tests::EmptyTask::default())),
        None,
    )
    .unwrap();
    // Only synthetic input setup uses SQL. FIFO selection, effect claim,
    // native allocation, turn binding, ingress and retention use their owners.
    let db = rusqlite::Connection::open(root.join("store/lifecycle.sqlite")).unwrap();
    db.execute("INSERT INTO inputs(input_id,subject_id,sequence,delivery_id,producer_ref,text,text_digest,grant_id,state) VALUES('input:input','subject:input',1,'delivery:input','producer:input',?1,?2,'grant:input','queued')",params![CARD.as_bytes(),CodecContract::LifecycleTextInputV1.digest_binary(CARD.as_bytes()).unwrap().hex()]).unwrap();
    let launch = actor.native_launch.take().unwrap();
    let native = config.native.clone().unwrap();
    let (work, pending) = mpsc::channel(4);
    actor.native_task_work = Some(work);
    let run = tokio::spawn(operations::run_native(
        native,
        launch,
        "transport:input".into(),
        actor.native_sender.clone(),
        actor.native_gate.clone(),
        Some(pending),
        None,
    ));
    let mut acquired = None;
    let mut captures = 0;
    let mut invocation = None;
    let mut done = false;
    let mut eligibility = Vec::new();
    let mut cleanup = None;
    let mut checks = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    while !done {
        let message = tokio::time::timeout_at(deadline, actor.native_messages.recv())
            .await
            .expect("native actor deadline")
            .expect("runner channel");
        if let NativeMessage::CaptureInput { observation, .. } = &message {
            acquired = Some(observation.clone());
            captures += 1;
            eligibility.push(observation.eligible());
            invocation = Some(
                actor
                    .native_turn
                    .as_ref()
                    .unwrap()
                    .invocation_id()
                    .to_owned(),
            );
        }
        let is_settle = matches!(message, NativeMessage::Settle { .. });
        done = matches!(message, NativeMessage::Done { .. });
        if let NativeMessage::Done {
            exit, outcome_kind, ..
        } = &message
        {
            cleanup = Some(
                json!({"exit":exit.as_ref().map(|e|json!({"pid":e.pid,"code":e.code,"forced":e.forced,"success":e.success})),"outcome":outcome_kind}),
            );
        }
        let capture = if let NativeMessage::CaptureInput { observation, .. } = &message {
            Some((
                observation.metadata().to_vec(),
                observation.terminal().clone(),
                observation
                    .payloads()
                    .iter()
                    .map(|p| lifecycle_store::NativeInputPayload {
                        inference_call_id: p.inference_call_id.clone(),
                        physical_send: p.physical_send,
                        representation: p.representation.clone(),
                        original_sha256: p.original_sha256.clone(),
                        original_codec: p.original_codec.clone(),
                        bytes: p.bytes.clone(),
                    })
                    .collect::<Vec<_>>(),
                observation.eligible(),
                observation.outcome().to_owned(),
            ))
        } else {
            None
        };
        let is_capture = capture.is_some();
        actor.handle_native_message(message).await;
        if let Some((metadata, terminal, payloads, eligible, outcome)) = capture {
            checks.push(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || {
                    let session = actor.native_session.as_ref().unwrap();
                    let turn = actor.native_turn.as_ref().unwrap();
                    let replay = actor
                        .store
                        .retain_native_input(
                            session,
                            turn,
                            &terminal.transport_session,
                            terminal.sequence,
                            &terminal.raw,
                            &metadata,
                            payloads.clone(),
                            eligible,
                            &outcome,
                            clock_sample(),
                        )
                        .unwrap();
                    assert_eq!(replay, lifecycle_store::NativeInputApply::Replay);
                    let original = actor
                        .store
                        .native_input_observation(turn.invocation_id())
                        .unwrap();
                    let mut conflict = metadata.clone();
                    conflict.push(b'\n');
                    let changed = actor
                        .store
                        .retain_native_input(
                            session,
                            turn,
                            &terminal.transport_session,
                            terminal.sequence,
                            &terminal.raw,
                            &conflict,
                            payloads.clone(),
                            false,
                            "conflicting_capture",
                            clock_sample(),
                        )
                        .unwrap();
                    assert_eq!(changed, lifecycle_store::NativeInputApply::Conflict);
                    assert_eq!(
                        actor
                            .store
                            .native_input_observation(turn.invocation_id())
                            .unwrap(),
                        original
                    );
                    for transport in ["transport:other", "transport:copied"] {
                        assert!(
                            actor
                                .store
                                .retain_native_input(
                                    session,
                                    turn,
                                    transport,
                                    terminal.sequence,
                                    &terminal.raw,
                                    &metadata,
                                    payloads.clone(),
                                    eligible,
                                    &outcome,
                                    clock_sample()
                                )
                                .is_err()
                        );
                    }
                },
            )));
        }
        if mode == "http" && is_capture {
            // Present the untouched, eligible acquisition from the actual owned run
            // against a different existing store-minted session/context/turn. Clone
            // exists only under the debug proof feature; it cannot rewrite identities.
            let observation = acquired.take().unwrap();
            let valid_acquired = observation.eligible() && observation.payloads().len() == 4;
            let original_turn = actor.native_turn.take().unwrap();
            let original_session = actor.native_session.take().unwrap();
            let original = actor
                .store
                .native_input_observation(original_turn.invocation_id())
                .unwrap()
                .unwrap();
            let subject = SubjectId::parse("subject:foreign-target").unwrap();
            let context = ContextGeneration::parse("context:foreign-target").unwrap();
            let build = BuildId::parse("build:foreign-target").unwrap();
            let proof = ProofRunId::parse("proof:foreign-target").unwrap();
            actor
                .store
                .register_subject(
                    subject.clone(),
                    context.clone(),
                    build.clone(),
                    proof.clone(),
                    clock_sample(),
                )
                .unwrap();
            let grant = GrantId::parse("grant:foreign-target").unwrap();
            actor
                .store
                .install_trusted_grant(TrustedGrant {
                    id: grant.clone(),
                    issuer: "issuer:input".into(),
                    principal: "principal:input".into(),
                    subject: subject.clone(),
                    context: context.clone(),
                    build,
                    proof_run: proof,
                    scope: GrantScope::parse("native_turn").unwrap(),
                    expires: AuthorityExpiresAt::new(WallTimeMs::new(
                        clock_sample().wall.get() + 600_000,
                    )),
                    revision: Revision::new(1),
                    revoked: false,
                })
                .unwrap();
            let mut target_session = actor
                .store
                .reserve_native_session(
                    NativeSessionId::parse("session:foreign-target").unwrap(),
                    subject,
                    context,
                    RuntimeIncarnation::parse("incarnation:foreign-target").unwrap(),
                    &original.profile_digest,
                    &original.binary_sha256,
                    &original.protocol_digest,
                    lifecycle_core::SnapshotId::parse("snapshot:foreign-target").unwrap(),
                    &"a".repeat(64),
                    false,
                    clock_sample(),
                )
                .unwrap();
            actor
                .store
                .bind_native_thread(
                    &mut target_session,
                    ProviderThreadId::parse("thread:foreign-target").unwrap(),
                )
                .unwrap();
            let mut target_turn = actor
                .store
                .claim_native_turn(
                    &target_session,
                    NativeTurnRequest {
                        invocation_id: "invocation:foreign-target".into(),
                        attempt: AttemptId::parse("attempt:foreign-target").unwrap(),
                        grant,
                        grant_revision: 1,
                        purpose: lifecycle_core::NativeInvocationPurpose::DomainWork,
                        prompt: b"a".to_vec(),
                    },
                    "principal:input",
                    clock_sample(),
                )
                .unwrap();
            // Synthetic target FIFO is populated before the attack, as in initial setup.
            db.execute_batch("INSERT INTO inputs(input_id,subject_id,sequence,delivery_id,producer_ref,text,text_digest,grant_id,state) VALUES('input:foreign-target','subject:foreign-target',1,'delivery:foreign-target','producer:input',X'61','fixture','grant:foreign-target','entered');
INSERT INTO effects(effect_id,subject_id,input_id,expected_revision,state) VALUES('effect:foreign-target','subject:foreign-target','input:foreign-target',0,'entered');
INSERT INTO attempts(attempt_id,effect_id,incarnation_id,state,entered_wall_ms) VALUES('queue-attempt:foreign-target','effect:foreign-target','incarnation:foreign-target','entered',1);").unwrap();
            actor
                .store
                .bind_native_delivery_segment(
                    target_turn.invocation_id(),
                    &AttemptId::parse("queue-attempt:foreign-target").unwrap(),
                    0,
                    None,
                    None,
                )
                .unwrap();
            actor.store.mark_native_send_intent(&target_turn).unwrap();
            actor
                .store
                .bind_native_turn(
                    &mut target_turn,
                    ProviderTurnId::parse("turn:foreign-target").unwrap(),
                )
                .unwrap();
            let target_raw = serde_json::to_vec(&json!({"method":"turn/completed","params":{"threadId":"thread:foreign-target","turn":{"id":"turn:foreign-target","status":"completed","error":null}}})).unwrap();
            // The ingress owner recomputes the enclosing raw digest, rather than
            // accepting a hand-written hash or a nonexistent coordinate.
            actor
                .store
                .record_native_ingress(
                    target_session.id(),
                    "transport:foreign-target",
                    1,
                    &target_raw,
                    "qualified",
                    Some("turn/completed"),
                    Some("thread:foreign-target"),
                    Some("turn:foreign-target"),
                    None,
                    clock_sample(),
                )
                .unwrap();
            let before = received.lock().unwrap().len();
            let intents: i64 = db
                .query_row(
                    "SELECT COUNT(*) FROM native_turns WHERE send_intent=1",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            actor.native_session = Some(target_session);
            actor.native_turn = Some(target_turn);
            let (reply, response) = oneshot::channel();
            actor
                .handle_native_message(NativeMessage::CaptureInput { observation, reply })
                .await;
            let refused = response.await.unwrap().is_err();
            checks.push(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert!(valid_acquired);
        assert!(refused);
        assert!(
            actor
                .store
                .native_input_observation("invocation:foreign-target")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            actor
                .store
                .native_input_observation(original_turn.invocation_id())
                .unwrap(),
            Some(original.clone())
        );
        assert_eq!(received.lock().unwrap().len(), before);
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM native_turns WHERE send_intent=1",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            intents
        );
        // Wrong but EXISTING ingress coordinate in the original session.
        let wrong: u64 = db.query_row("SELECT sequence FROM native_ingress WHERE session_id=?1 AND transport_session=?2 AND sequence<>?3 ORDER BY sequence LIMIT 1",params![original.session_id,original.transport_session,original.terminal_sequence],|r|r.get(0)).unwrap();
        assert!(
            actor
                .store
                .retain_native_input(
                    &original_session,
                    &original_turn,
                    &original.transport_session,
                    wrong,
                    &original.terminal_raw,
                    &original.metadata,
                    original.payloads.clone(),
                    true,
                    &original.outcome,
                    clock_sample()
                )
                .is_err()
        );
        assert_eq!(
            actor
                .store
                .native_input_observation(original_turn.invocation_id())
                .unwrap(),
            Some(original)
        );
        assert!(
            actor
                .store
                .native_input_observation("invocation:foreign-target")
                .unwrap()
                .is_none()
        );
        fs::write(root.join("foreign-refusal.json"), serde_json::to_vec_pretty(&json!({"actual_eligible_acquisition":true,"checked_payloads":4,"distinct_existing_context":"context:foreign-target","distinct_existing_turn":"turn:foreign-target","target_ingress_digest_recomputed_by_owner":true,"actor_refused":true,"existing_wrong_ingress_sequence":wrong,"original_unchanged":true,"target_observation":false,"peer_count_before":before,"peer_count_after":received.lock().unwrap().len(),"send_intents_before":intents,"send_intents_after":db.query_row("SELECT COUNT(*) FROM native_turns WHERE send_intent=1", [], |r|r.get::<_,i64>(0)).unwrap()})).unwrap()).unwrap();
        })));
            actor.native_session = Some(original_session);
            actor.native_turn = Some(original_turn);
        }
        if is_settle {
            actor.native_task_work.take();
        }
    }
    run.await.unwrap();
    fs::write(
        root.join("cleanup.json"),
        serde_json::to_vec_pretty(&cleanup).unwrap(),
    )
    .unwrap();
    let exits: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM native_process_exits WHERE pid IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let before_restart = received.lock().unwrap().len();
    let verified = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if mode != "lost" {
            let invocation = invocation.as_ref().unwrap();
            let record = actor
                .store
                .native_input_observation(invocation)
                .unwrap()
                .unwrap();
            assert_eq!(record.eligible, mode == "http");
            let delivery = record.delivery.as_ref().unwrap();
            assert_eq!(delivery.input_id, "input:input");
            assert_eq!(delivery.delivery_id, "delivery:input");
            assert_eq!(delivery.segment_no, 0);
            assert_eq!(
                record.binary_sha256,
                "0d79af323b3448f4ff2dc75feff233054ae6c4d490cefbac7952c0d219b95699"
            );
            assert!(record.payloads.iter().any(|p| p.original_codec == "zstd"));
            let wire = record
                .payloads
                .iter()
                .filter(|p| p.representation == "wire")
                .collect::<Vec<_>>();
            let decoded = record
                .payloads
                .iter()
                .filter(|p| p.representation == "decoded")
                .collect::<Vec<_>>();
            assert_eq!(wire.len(), 2);
            assert_eq!(decoded.len(), 2);
            let seen = received.lock().unwrap();
            for (n, (wire, decoded)) in wire.iter().zip(&decoded).enumerate() {
                assert_eq!(wire.bytes, seen[n].0);
                assert_eq!(decoded.bytes, seen[n].1);
                assert!(String::from_utf8_lossy(&decoded.bytes).contains("reconstruction-1"));
            }
            drop(seen);
            fs::write(
                root.join("observation.json"),
                serde_json::to_vec_pretty(&record).unwrap(),
            )
            .unwrap();
            drop(actor);
            drop(db);
            let reopened =
                SqliteLifecycleStore::open(&root.join("store"), "issuer:input".into()).unwrap();
            assert_eq!(
                reopened.native_input_observation(invocation).unwrap(),
                Some(record)
            );
        } else {
            let observations: i64 = db
                .query_row("SELECT COUNT(*) FROM native_input_observations", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(observations, 1);
            let record = actor
                .store
                .native_input_observation(invocation.as_ref().unwrap())
                .unwrap()
                .unwrap();
            assert!(!record.eligible);
            assert_eq!(record.payloads.len(), 2);
            let seen = received.lock().unwrap();
            assert_eq!(
                record
                    .payloads
                    .iter()
                    .find(|p| p.representation == "wire")
                    .unwrap()
                    .bytes,
                seen[0].0
            );
            assert_eq!(
                record
                    .payloads
                    .iter()
                    .find(|p| p.representation == "decoded")
                    .unwrap()
                    .bytes,
                seen[0].1
            );
            let rows = String::from_utf8(record.metadata.clone()).unwrap();
            assert_eq!(rows.matches("before_send").count(), 1);
            assert!(rows.contains("send_failed_unknown"));
            assert!(rows.contains("provider_failed_or_unknown"));
            drop(seen);
            fs::write(
                root.join("observation.json"),
                serde_json::to_vec_pretty(&record).unwrap(),
            )
            .unwrap();
            let sends: i64 = db
                .query_row(
                    "SELECT COUNT(*) FROM native_turns WHERE send_intent=1",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(sends, 1);
            drop(actor);
            drop(db);
            let reopened =
                SqliteLifecycleStore::open(&root.join("store"), "issuer:input".into()).unwrap();
            assert_eq!(
                reopened
                    .native_input_observation(invocation.as_ref().unwrap())
                    .unwrap(),
                Some(record)
            );
        }
    }));
    // Observe dispatch after restart/readback too; the peer stays accepting.
    tokio::time::sleep(Duration::from_millis(750)).await;
    let peer_result = peer_guard.collect();
    let received = received.lock().unwrap().clone();
    fs::write(
        root.join("peer.json"),
        serde_json::to_vec(&received).unwrap(),
    )
    .unwrap();
    let count = received.len();
    fs::write(root.join("custody.json"),json!({"done":done,"durable_process_exits":exits,"peer_joined":peer_result.is_ok(),"peer_receptions_before_restart":before_restart,"peer_receptions_after_restart_wait":count,"post_restart_accept_window_ms":750}).to_string()).unwrap();
    peer_result.unwrap();
    for check in checks {
        if let Err(panic) = check {
            std::panic::resume_unwind(panic);
        }
    }
    if let Err(panic) = verified {
        std::panic::resume_unwind(panic);
    }
    assert_eq!(
        exits, 1,
        "actual child wait must reach durable owner before qualification assertions"
    );
    assert!(
        eligibility
            .iter()
            .all(|eligible| *eligible == (mode == "http"))
    );
    assert_eq!(
        count, before_restart,
        "restart or unknown terminal must not dispatch"
    );
    if mode == "lost" {
        assert_eq!(count, 1);
        assert_eq!(captures, 1);
    } else {
        assert_eq!(captures, 1);
        assert_eq!(count, 2);
    }
    fs::write(root.join("result.json"),json!({"mode":mode,"peer_receptions":count,"captures":captures,"native_child_collected":true,"peer_thread_joined":true,"authority":"none"}).to_string()).unwrap();
}
