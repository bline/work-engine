//! Runtime selection of the direct realization through a real `Actor`.
//!
//! Evidence class: synthetic. The port is a scripted double. These tests show
//! that trusted composition (`Actor::with_direct_control`) is what selects the
//! direct realization in the one function `drive_transition` uses to build the
//! semantic runtime, that the identities come from the configured semantic
//! capture, and that owner admission reaches the store through
//! `ServiceConfig.grants` installed by the trusted issuer at `Actor::new`. They
//! establish nothing about a provider or about production eligibility.
//!
//! The engine's Verifying-stage call is driven too: `actor_with_custody` builds
//! the native custody the task profile requires (a settled domain-work turn with
//! qualified ingress and a yielded delivery segment) through the store's own
//! APIs, and the tests call `engine::drive_transition` with the runtime the
//! actor selects. Only the table reads at the end of each test are direct.

use std::{fs, os::unix::fs::{MetadataExt, PermissionsExt}, sync::Arc};

use lifecycle_core::{
    AttemptId, AuthorityExpiresAt, BuildId, ClockSample, ContextGeneration, GrantId, GrantScope,
    NativeInvocationPurpose, ProofRunId, ProviderThreadId, ProviderTurnId, SubjectId,
    TransitionId, TrustedGrant, WallTimeMs,
};
use lifecycle_semantic::{DirectInferencePort, MaterialInput};
use lifecycle_store::{DirectCheckpointOrigin, NativeTurnRequest};

use super::*;
use super::restart_epoch_tests::EmptyTask;
use crate::{
    GrantConfig, NativeConfig, NativeProviderRoute, PrincipalConfig, SemanticNativeConfig,
    semantic::{ControlRealization, establish_checkpoint},
    semantic_direct_tests::{
        Material, Reply, ScriptedPort, assert_unadmitted, c1_candidate, c1_input, c1_report,
        fixture, private_dir,
    },
};
use lifecycle_semantic::FindingStatus;

const ISSUER: &str = "issuer:actor-direct";
const CONTROL_GRANT: &str = "grant:actor-direct-control";
const ADMISSION_GRANT: &str = "grant:actor-direct-admission";
const NEXT_CONTEXT: &str = "context:actor-direct-next";
const TRANSPORT: &str = "transport:actor-direct-synthetic";

struct Rig {
    _temp: tempfile::TempDir,
    config: ServiceConfig,
    material: Arc<Material>,
    input: MaterialInput,
}

impl Rig {
    fn new(owner_admission: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        private_dir(&root);
        let native_root = temp.path().join("native");
        private_dir(&native_root);
        let input = c1_input(&fixture());
        let context = ContextGeneration::parse("context:actor-direct").unwrap();
        let member_bytes = b"direct source\n".to_vec();
        let member_sha = CodecContract::BinaryArtifactV1
            .digest_binary(&member_bytes)
            .unwrap()
            .hex();
        let semantic = SemanticNativeConfig {
            capture_id: "capture-C1".into(),
            compile_session_id: "session:actor-direct-compile".into(),
            verify_session_id: "session:actor-direct-verify".into(),
            successor_verify_session_id: "session:actor-direct-successor".into(),
            compile_invocation_id: "invocation:actor-direct-compile".into(),
            compile_attempt_id: "attempt:actor-direct-compile".into(),
            candidate_verify_invocation_id: "invocation:actor-direct-verify".into(),
            candidate_verify_attempt_id: "attempt:actor-direct-verify".into(),
            successor_verify_invocation_id: "invocation:actor-direct-successor".into(),
            successor_verify_attempt_id: "attempt:actor-direct-successor".into(),
            challenge_id: "challenge:actor-direct".into(),
            challenge_text: "offline".into(),
            control_grant_ref: CONTROL_GRANT.into(),
            control_grant_revision: 1,
            successor_control_grant_ref: Some("grant:actor-direct-control:next".into()),
            successor_control_grant_revision: Some(1),
        };
        let native = NativeConfig {
            executable: temp.path().join("never-executed"),
            root: native_root,
            provider_route: NativeProviderRoute::NativeSimProofV1,
            chatgpt_access_source: None,
            simulator_base_url: "http://127.0.0.1:12345/v1".into(),
            session_id: "session:actor-direct".into(),
            invocation_id: "invocation:actor-direct".into(),
            attempt_id: "attempt:actor-direct".into(),
            prompt: "unused".into(),
            turn_grant_ref: "grant:actor-direct-turn".into(),
            turn_grant_revision: 1,
            snapshot_grant_ref: "grant:actor-direct-snapshot".into(),
            snapshot_grant_revision: 1,
            same_native_session_required: false,
            followup_turns: Vec::new(),
            snapshot: crate::operations::workspace_snapshot_read::TrustedSnapshot {
                id: "snapshot:actor-direct".into(),
                source_revision: "source:actor-direct".into(),
                members: vec![crate::operations::workspace_snapshot_read::TrustedMember {
                    id: "source".into(),
                    sha256: member_sha,
                    bytes: member_bytes,
                }],
            },
            semantic: Some(semantic),
            semantic_contexts: Vec::new(),
        };
        let grant = |id: &str, scope: &str| GrantConfig {
            grant_ref: id.into(),
            context_generation: context.as_str().into(),
            scope: scope.into(),
            expires_wall_ms: i64::MAX / 2,
            revision: 1,
        };
        let mut grants = vec![grant(CONTROL_GRANT, "semantic_control")];
        // The task profile names its successor from two distinct enqueue grants.
        grants.push(grant("grant:actor-direct-enqueue", "enqueue_input"));
        grants.push(GrantConfig {
            grant_ref: "grant:actor-direct-enqueue-next".into(),
            context_generation: NEXT_CONTEXT.into(),
            scope: "enqueue_input".into(),
            expires_wall_ms: i64::MAX / 2,
            revision: 1,
        });
        if owner_admission {
            grants.push(grant(ADMISSION_GRANT, "semantic_direct_checkpoint_v1"));
        }
        let config = ServiceConfig {
            profile: "retained-repository-analysis-v1".into(),
            store_root: root,
            socket_path: temp.path().join("service.sock"),
            peer_socket_path: temp.path().join("peer.sock"),
            verifier_executable: None,
            verifier_ledger_path: None,
            proof_barrier_dir: None,
            trusted_issuer: ISSUER.into(),
            principal: PrincipalConfig {
                uid: fs::metadata("/proc/self").unwrap().uid(),
                principal_ref: "principal:actor-direct".into(),
                producer_ref: "producer:actor-direct".into(),
            },
            subject_id: input.subject.as_str().into(),
            context_generation: context.as_str().into(),
            build_id: "build:actor-direct".into(),
            proof_run_id: "proof:actor-direct".into(),
            grants,
            max_executions: 1,
            max_wait_ms: 1000,
            verifier_proof_ms: None,
            verifier_cleanup_ms: None,
            native: Some(native),
            selected_restart: None,
        };
        let material = Arc::new(Material {
            input: input.clone(),
            stale: std::sync::atomic::AtomicBool::new(false),
            unavailable: std::sync::atomic::AtomicBool::new(false),
            changed_after_observation: std::sync::atomic::AtomicBool::new(false),
            fenced_commits: std::sync::atomic::AtomicUsize::new(0),
        });
        Self {
            _temp: temp,
            config,
            material,
            input,
        }
    }

    /// A real actor over a fresh store; `Actor::new` registers the subject and
    /// installs `config.grants` through the trusted-issuer path. No transition
    /// exists yet.
    fn bare_actor(&self, direct: Option<Arc<dyn DirectInferencePort>>) -> Actor {
        Actor::new(
            self.config.clone(),
            Some(self.material.clone()),
            Some(Arc::new(EmptyTask::default())),
            None,
        )
        .unwrap()
        .with_direct_control(direct)
    }

    /// The semantic transition the checkpoint establishes, as the engine finds
    /// it at the Verifying stage.
    fn open_transition(&self) {
        self.open_transition_at("verifying", None);
    }

    fn open_transition_at(&self, stage: &str, basis: Option<&str>) {
        let connection =
            rusqlite::Connection::open(self.config.store_root.join("lifecycle.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO transitions(transition_id,subject_id,predecessor_context,stage,basis_revision,created_wall_ms,native_basis_source) VALUES(?1,?2,?3,?4,0,100,?5)",
                rusqlite::params![
                    self.input.transition.as_str(),
                    self.input.subject.as_str(),
                    self.config.context_generation,
                    stage,
                    basis
                ],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE transitions SET semantic_required=1 WHERE transition_id=?1",
                [self.input.transition.as_str()],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE subjects SET owner_kind='transition',owner_ref=?1,semantic_revision=1 WHERE subject_id=?2",
                rusqlite::params![self.input.transition.as_str(), self.input.subject.as_str()],
            )
            .unwrap();
    }

    fn actor(&self, direct: Option<Arc<dyn DirectInferencePort>>) -> Actor {
        let actor = self.bare_actor(direct);
        self.open_transition();
        actor
    }

    fn replies(&self) -> Vec<Reply> {
        vec![
            Reply::complete(serde_json::to_vec(&c1_candidate(&self.input)).unwrap()),
            Reply::complete(
                serde_json::to_vec(&c1_report(&self.input, FindingStatus::Satisfied)).unwrap(),
            ),
        ]
    }

    fn subject(&self) -> SubjectId {
        self.input.subject.clone()
    }

    fn context(&self) -> ContextGeneration {
        ContextGeneration::parse(self.config.context_generation.clone()).unwrap()
    }

    fn transition(&self) -> TransitionId {
        self.input.transition.clone()
    }

    fn count(&self, query: &str) -> i64 {
        rusqlite::Connection::open(self.config.store_root.join("lifecycle.sqlite"))
            .unwrap()
            .query_row(query, [], |row| row.get(0))
            .unwrap()
    }
}

/// The actor registered the subject with the real clock, and the store refuses a
/// wall-clock rollback, so seeding uses the same source.
fn seed_clock() -> ClockSample {
    clock_sample()
}

fn ingress(
    actor: &mut Actor,
    session: &lifecycle_core::NativeSessionId,
    sequence: u64,
    value: serde_json::Value,
) {
    let event = lifecycle_codex::parse_event(
        TRANSPORT.into(),
        sequence,
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let method = value.get("method").and_then(serde_json::Value::as_str);
    let params = value.get("params");
    let thread = params
        .and_then(|p| p.get("threadId"))
        .and_then(serde_json::Value::as_str);
    let turn = params
        .and_then(|p| p.get("turnId").or_else(|| p.get("turn")?.get("id")))
        .and_then(serde_json::Value::as_str);
    actor
        .store
        .record_native_ingress(
            session,
            &event.transport_session,
            event.sequence,
            &event.raw,
            "qualified",
            method,
            thread,
            turn,
            None,
            seed_clock(),
        )
        .unwrap();
}

impl Rig {
    /// Native custody the task profile requires before it will drive the
    /// transition: one settled domain-work turn on the actor's own native
    /// session, qualified ingress for it, a yielded delivery segment, and the
    /// transition's native basis pointing at that turn. All of it goes through
    /// the store's own APIs except the queue rows (`inputs`, `effects`,
    /// `attempts`) and the transition row itself, which belong to commands this
    /// rig does not run. The turn text and ingress are synthetic; no native
    /// process exists.
    ///
    /// The order is the store's: the turn is claimed while the subject is
    /// domain-owned and no transition exists, the transition (with its native
    /// basis) is reserved before the segment settles so the segment yields to
    /// it, and only then does the transition advance to `verifying`.
    fn actor_with_custody(&self, direct: Option<Arc<dyn DirectInferencePort>>) -> Actor {
        let mut actor = self.bare_actor(direct);
        let (turn, session_id) = self.claim_domain_turn(&mut actor);
        self.open_transition_at("quiescing", Some("invocation:actor-direct-domain"));
        self.settle_domain_turn(&mut actor, &turn, &session_id);
        let connection =
            rusqlite::Connection::open(self.config.store_root.join("lifecycle.sqlite")).unwrap();
        connection
            .execute(
                "UPDATE transitions SET stage='verifying' WHERE transition_id=?1",
                [self.input.transition.as_str()],
            )
            .unwrap();
        assert!(
            actor
                .store
                .task_transition_source_ready(&self.transition())
                .unwrap(),
            "seeded custody must satisfy the task profile's own readiness query"
        );
        actor
    }

    fn claim_domain_turn(
        &self,
        actor: &mut Actor,
    ) -> (lifecycle_store::NativeTurnEntry, lifecycle_core::NativeSessionId) {
        let subject = self.subject();
        let context = self.context();
        let grant = GrantId::parse("grant:actor-direct-turn").unwrap();
        actor
            .store
            .install_trusted_grant(TrustedGrant {
                id: grant.clone(),
                issuer: ISSUER.into(),
                principal: "principal:actor-direct".into(),
                subject: subject.clone(),
                context: context.clone(),
                build: BuildId::parse(self.config.build_id.clone()).unwrap(),
                proof_run: ProofRunId::parse(self.config.proof_run_id.clone()).unwrap(),
                scope: GrantScope::parse("native_turn").unwrap(),
                expires: AuthorityExpiresAt::new(WallTimeMs::new(i64::MAX / 2)),
                revision: lifecycle_core::Revision::new(1),
                revoked: false,
            })
            .unwrap();
        let session = actor.native_session.as_mut().expect("actor native session");
        actor
            .store
            .bind_native_thread(session, ProviderThreadId::parse("thread:actor-direct").unwrap())
            .unwrap();
        let session = actor.native_session.as_ref().unwrap();
        let session_id = session.id().clone();
        let mut turn = actor
            .store
            .claim_native_turn(
                session,
                NativeTurnRequest {
                    invocation_id: "invocation:actor-direct-domain".into(),
                    attempt: AttemptId::parse("native-attempt:actor-direct-domain").unwrap(),
                    grant,
                    grant_revision: 1,
                    purpose: NativeInvocationPurpose::DomainWork,
                    prompt: b"test".to_vec(),
                },
                "principal:actor-direct",
                seed_clock(),
            )
            .unwrap();
        let connection =
            rusqlite::Connection::open(self.config.store_root.join("lifecycle.sqlite")).unwrap();
        let incarnation = actor.incarnation.as_str().to_string();
        connection
            .execute_batch(&format!(
                "INSERT INTO inputs(input_id,subject_id,sequence,delivery_id,producer_ref,text,text_digest,grant_id,state)
                 VALUES('input:actor-direct','{subject}',1,'delivery:actor-direct','producer:actor-direct',X'74657374','digest:actor-direct','grant:actor-direct-turn','entered');
                 INSERT INTO effects(effect_id,subject_id,input_id,expected_revision,state)
                 VALUES('effect:actor-direct','{subject}','input:actor-direct',0,'entered');
                 INSERT INTO attempts(attempt_id,effect_id,incarnation_id,state,entered_wall_ms)
                 VALUES('queue-attempt:actor-direct','effect:actor-direct','{incarnation}','entered',100);",
                subject = subject.as_str(),
            ))
            .unwrap();
        actor
            .store
            .bind_native_delivery_segment(
                turn.invocation_id(),
                &AttemptId::parse("queue-attempt:actor-direct").unwrap(),
                0,
                None,
                None,
            )
            .unwrap();
        actor.store.mark_native_send_intent(&turn).unwrap();
        actor
            .store
            .bind_native_turn(&mut turn, ProviderTurnId::parse("turn:actor-direct").unwrap())
            .unwrap();
        (turn, session_id)
    }

    fn settle_domain_turn(
        &self,
        actor: &mut Actor,
        turn: &lifecycle_store::NativeTurnEntry,
        session_id: &lifecycle_core::NativeSessionId,
    ) {
        ingress(
            actor,
            session_id,
            3,
            serde_json::json!({"method":"turn/completed","params":{
                "threadId":"thread:actor-direct",
                "turn":{"id":"turn:actor-direct","status":"completed"}}}),
        );
        let final_item =
            serde_json::json!({"type":"agentMessage","id":"item:final","text":"done"});
        ingress(
            actor,
            session_id,
            6,
            serde_json::json!({"method":"item/completed","params":{
                "threadId":"thread:actor-direct","turnId":"turn:actor-direct",
                "item":final_item.clone()}}),
        );
        ingress(
            actor,
            session_id,
            7,
            serde_json::json!({"id":7,"result":{"thread":{
                "id":"thread:actor-direct","historyMode":"legacy","turns":[{
                    "id":"turn:actor-direct","status":"completed","itemsView":"full",
                    "items":[final_item]}]}}}),
        );
        actor
            .store
            .settle_native_turn(turn, "done", "observed_complete", TRANSPORT, 7, seed_clock())
            .unwrap();
        assert!(
            actor
                .store
                .settle_native_delivery_segment(turn.invocation_id(), seed_clock())
                .unwrap(),
            "the segment yields to the reserved transition"
        );
    }

    fn stage(&self, actor: &mut Actor) -> String {
        actor
            .store
            .subject_projection(&self.subject(), None, None)
            .unwrap()
            .transition
            .expect("transition")
            .stage
    }

    fn checkpoint_id(&self) -> Option<String> {
        rusqlite::Connection::open(self.config.store_root.join("lifecycle.sqlite"))
            .unwrap()
            .query_row(
                "SELECT checkpoint_id FROM transitions WHERE transition_id=?1",
                [self.input.transition.as_str()],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn capture_eligibility(&self) -> String {
        rusqlite::Connection::open(self.config.store_root.join("lifecycle.sqlite"))
            .unwrap()
            .query_row(
                "SELECT eligibility FROM semantic_captures WHERE capture_id='capture-C1'",
                [],
                |row| row.get(0),
            )
            .unwrap()
    }
}

/// What the actor's `drive_transition` hands the engine: the runtime the actor
/// selects and nothing else. The actor wrapper additionally asks the native
/// runner to start a successor once the transition reaches `switching`, which
/// this rig does not have; the host startup test drives the whole path.
async fn drive(actor: &mut Actor) -> Result<(), StoreError> {
    let runtime = actor.semantic_runtime()?;
    engine::drive_transition(
        &mut actor.store,
        &actor.config,
        &actor.verifier_gate,
        runtime.as_ref(),
    )
    .await
}

#[tokio::test]
async fn actor_without_trusted_direct_control_selects_native() {
    let rig = Rig::new(true);
    let mut actor = rig.actor(None);
    let runtime = actor.semantic_runtime().unwrap().expect("semantic runtime");
    assert!(matches!(runtime.realization, ControlRealization::Native));
}

#[tokio::test]
async fn actor_with_trusted_direct_control_selects_direct_with_configured_identities() {
    let rig = Rig::new(true);
    let port = ScriptedPort::public_class(rig.replies());
    let mut actor = rig.actor(Some(port.clone()));
    let runtime = actor.semantic_runtime().unwrap().expect("semantic runtime");
    let semantic = rig.config.native.as_ref().unwrap().semantic.clone().unwrap();
    let ControlRealization::Direct(direct) = &runtime.realization else {
        panic!("a supplied direct port must select the direct realization");
    };
    assert_eq!(direct.compile_operation_id, semantic.compile_invocation_id);
    assert_eq!(direct.compile_attempt_id, semantic.compile_attempt_id);
    assert_eq!(direct.verify_operation_id, semantic.candidate_verify_invocation_id);
    assert_eq!(direct.verify_attempt_id, semantic.candidate_verify_attempt_id);
    assert_eq!(
        direct.compile_exposure_id,
        format!("exposure:{}", semantic.compile_invocation_id)
    );
    assert_eq!(
        direct.verify_exposure_id,
        format!("exposure:{}", semantic.candidate_verify_invocation_id)
    );
    assert_eq!(port.calls(), 0, "selection alone submits nothing");
}

#[tokio::test]
async fn actor_selected_direct_with_owner_grant_and_public_class_admits_through_its_own_store() {
    let rig = Rig::new(true);
    let port = ScriptedPort::public_class(rig.replies());
    let mut actor = rig.actor(Some(port.clone()));
    let runtime = actor.semantic_runtime().unwrap().expect("semantic runtime");
    let checkpoint = establish_checkpoint(
        &mut actor.store,
        &runtime,
        &rig.subject(),
        &rig.context(),
        &rig.transition(),
        1,
    )
    .await
    .unwrap();
    assert!(checkpoint.is_some(), "owner grant + public class admits");
    assert_eq!(port.calls(), 2);
    let seen = port.seen.lock().unwrap().clone();
    let semantic = rig.config.native.as_ref().unwrap().semantic.clone().unwrap();
    assert_eq!(seen[0].binding.operation_id, semantic.compile_invocation_id);
    assert_eq!(seen[1].binding.operation_id, semantic.candidate_verify_invocation_id);
    assert_ne!(seen[0].wire_sha256, seen[1].wire_sha256);
    assert!(actor.store.admitted_direct_checkpoint(&rig.transition()).unwrap().is_some());
    assert!(matches!(
        actor.store.semantic_checkpoint_origin(&rig.transition()).unwrap(),
        Some(DirectCheckpointOrigin::DirectAdmitted { .. })
    ));
    // Native consumers still see nothing, and no provider session was created
    // for either control call.
    assert!(actor.store.semantic_checkpoint(&rig.transition()).unwrap().is_none());
    assert!(actor.store.native_rehydration_receipt(&rig.transition()).unwrap().is_none());
    assert_eq!(
        rig.count(&format!(
            "SELECT COUNT(*) FROM native_sessions WHERE session_id IN ('{}','{}')",
            semantic.compile_session_id, semantic.verify_session_id
        )),
        0
    );
}

#[tokio::test]
async fn actor_selected_direct_without_owner_grant_or_with_synthetic_class_is_proof_only() {
    for (name, admission, public) in [
        ("public_class_no_owner_grant", false, true),
        ("synthetic_class_with_owner_grant", true, false),
    ] {
        let rig = Rig::new(admission);
        let port = if public {
            ScriptedPort::public_class(rig.replies())
        } else {
            ScriptedPort::new(rig.replies())
        };
        let mut actor = rig.actor(Some(port.clone()));
        let runtime = actor.semantic_runtime().unwrap().expect("semantic runtime");
        let result = establish_checkpoint(
            &mut actor.store,
            &runtime,
            &rig.subject(),
            &rig.context(),
            &rig.transition(),
            1,
        )
        .await;
        assert!(matches!(result, Ok(None)), "{name}: {result:?}");
        assert_eq!(port.calls(), 2, "{name}");
        assert!(actor.store.direct_proof_checkpoint(&rig.transition()).unwrap().is_some(), "{name}");
        assert_unadmitted(&actor.store, &rig.transition());
    }
}

#[tokio::test]
async fn direct_port_is_rejected_by_configuration_before_any_bind() {
    // A direct port needs the retained profile, a material owner and a
    // configured semantic capture. Each failure is a configuration error.
    let rig = Rig::new(true);
    let port: Arc<dyn DirectInferencePort> = ScriptedPort::public_class(Vec::new());
    let material: Arc<dyn CanonicalMaterialPort> = rig.material.clone();
    let task: Arc<dyn TaskOperationPort> = Arc::new(EmptyTask::default());

    let mut other_profile = rig.config.clone();
    other_profile.profile = "read_only".into();
    other_profile.native = None;
    let result =
        crate::run_with_direct_control(other_profile, material.clone(), task.clone(), port.clone())
            .await;
    assert!(matches!(result, Err(ServiceError::Configuration)), "{result:?}");

    let mut no_semantic = rig.config.clone();
    no_semantic.native.as_mut().unwrap().semantic = None;
    let result =
        crate::run_with_direct_control(no_semantic, material.clone(), task.clone(), port.clone())
            .await;
    assert!(matches!(result, Err(ServiceError::Configuration)), "{result:?}");

    // No store was opened and no socket created by either rejected start.
    assert!(!rig.config.socket_path.exists());
    assert_eq!(
        fs::metadata(&rig.config.store_root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert!(!rig.config.store_root.join("lifecycle.sqlite").exists());
}

#[tokio::test]
async fn selected_store_entry_applies_the_same_direct_validation() {
    let rig = Rig::new(true);
    let port: Arc<dyn DirectInferencePort> = ScriptedPort::public_class(Vec::new());
    let material: Arc<dyn CanonicalMaterialPort> = rig.material.clone();
    let task: Arc<dyn TaskOperationPort> = Arc::new(EmptyTask::default());

    // A direct port without a configured semantic capture is refused by the
    // selected-store entry exactly as by the plain one.
    let mut no_semantic = rig.config.clone();
    no_semantic.native.as_mut().unwrap().semantic = None;
    let selected = crate::SelectedLifecycleStore::open(&no_semantic).unwrap();
    let result = crate::run_with_selected_task_ports_and_direct_control(
        no_semantic,
        selected,
        material.clone(),
        task.clone(),
        port.clone(),
    )
    .await;
    assert!(matches!(result, Err(ServiceError::Configuration)), "{result:?}");
    assert!(!rig.config.socket_path.exists());

    // So is any profile other than the retained task profile.
    let mut other_profile = rig.config.clone();
    other_profile.profile = "read_only".into();
    other_profile.native = None;
    let selected = crate::SelectedLifecycleStore::open(&other_profile).unwrap();
    let result = crate::run_with_selected_task_ports_and_direct_control(
        other_profile,
        selected,
        material,
        task,
        port,
    )
    .await;
    assert!(matches!(result, Err(ServiceError::Configuration)), "{result:?}");
    assert!(!rig.config.socket_path.exists());
}

// ---- The engine's Verifying stage, driven through the actor's selection ----

fn fenced(rig: &Rig) -> usize {
    rig.material.fenced_commits.load(std::sync::atomic::Ordering::SeqCst)
}

#[tokio::test]
async fn engine_direct_verification_commits_inside_the_owner_fence() {
    let rig = Rig::new(true);
    let port = ScriptedPort::public_class(rig.replies());
    let mut actor = rig.actor_with_custody(Some(port.clone()));
    assert_eq!(rig.stage(&mut actor), "verifying");
    assert_eq!(fenced(&rig), 0);

    let driven = drive(&mut actor).await;

    // The checkpoint was established and the lifecycle fact applied; the
    // engine stops there instead of asking to authorize a native switch.
    assert!(driven.is_ok(), "{driven:?}");
    assert_eq!(rig.stage(&mut actor), "checkpointed");
    // Driving again is the same graceful stop, and no switch fact exists.
    assert!(drive(&mut actor).await.is_ok());
    assert_eq!(rig.stage(&mut actor), "checkpointed");
    assert_eq!(
        rig.count(&format!(
            "SELECT COUNT(*) FROM transitions WHERE transition_id='{}' AND (successor_context IS NOT NULL OR successor_thread IS NOT NULL)",
            rig.transition().as_str()
        )),
        0,
        "no native successor was authorized"
    );
    assert!(rig.checkpoint_id().is_some_and(|id| id.starts_with("checkpoint:")));
    assert_eq!(port.calls(), 2, "one compile and one verify exchange");
    assert!(actor.store.admitted_direct_checkpoint(&rig.transition()).unwrap().is_some());
    // Two owner fences: the publication of the direct result, and the engine's
    // consumption of it as `VerificationEstablished`. A consumption that did
    // not select `with_current_fence` leaves this at one.
    assert_eq!(fenced(&rig), 2);
    let semantic = rig.config.native.as_ref().unwrap().semantic.clone().unwrap();
    assert_eq!(
        rig.count(&format!(
            "SELECT COUNT(*) FROM native_sessions WHERE session_id IN ('{}','{}')",
            semantic.compile_session_id, semantic.verify_session_id
        )),
        0,
        "no native control session was used"
    );
    assert_eq!(rig.capture_eligibility(), "current");
}

#[tokio::test]
async fn engine_direct_verification_refuses_cached_checkpoint_after_owner_update() {
    let rig = Rig::new(true);
    let port = ScriptedPort::public_class(rig.replies());
    let mut actor = rig.actor_with_custody(Some(port.clone()));

    // The checkpoint is established and admitted while the source is current,
    // before the engine consumes it.
    let runtime = actor.semantic_runtime().unwrap().expect("semantic runtime");
    establish_checkpoint(
        &mut actor.store,
        &runtime,
        &rig.subject(),
        &rig.context(),
        &rig.transition(),
        1,
    )
    .await
    .unwrap()
    .expect("admitted");
    drop(runtime);
    assert_eq!(port.calls(), 2);
    let published = fenced(&rig);
    assert_eq!(published, 1, "publication ran inside the owner fence");

    // The owner's source then moves in a way only the commit-time fence sees:
    // the cached offer revalidates unfenced dependencies and finds them current.
    rig.material
        .changed_after_observation
        .store(true, std::sync::atomic::Ordering::SeqCst);

    drive(&mut actor).await.unwrap();

    assert_eq!(rig.stage(&mut actor), "verifying", "stale checkpoint was not consumed");
    assert!(rig.checkpoint_id().is_none());
    assert_eq!(fenced(&rig), published, "the owner refused before any lifecycle commit");
    assert_eq!(rig.capture_eligibility(), "stale");
    assert_eq!(port.calls(), 2, "consumption sent no further exchange");
    assert_eq!(
        rig.count("SELECT COUNT(*) FROM journal WHERE event_kind='semantic_capture_stale'"),
        1
    );
}

#[tokio::test]
async fn engine_without_direct_control_never_reaches_the_direct_fence() {
    // Same custody and the same material double, native selection: the engine
    // does not read an admitted direct checkpoint, makes no direct exchange and
    // does not fall back to Direct. Native control would need a native session
    // to run; whatever it reports, nothing Direct was produced.
    let rig = Rig::new(true);
    let mut actor = rig.actor_with_custody(None);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), drive(&mut actor)).await;
    assert_eq!(rig.stage(&mut actor), "verifying");
    assert!(actor.store.admitted_direct_checkpoint(&rig.transition()).unwrap().is_none());
    assert!(actor.store.direct_proof_checkpoint(&rig.transition()).unwrap().is_none());
    assert_eq!(fenced(&rig), 0);
    assert_eq!(rig.count("SELECT COUNT(*) FROM direct_semantic_results"), 0);
}

#[tokio::test]
async fn engine_direct_refused_unknown_and_unadmitted_replies_do_not_advance() {
    // (name, owner admission grant, public evidence class, scripted replies,
    // publication fences). A proof-only result is still published under the
    // fence; an unknown transport outcome publishes nothing.
    let cases: [(&str, bool, bool, bool, usize); 3] = [
        ("no_owner_grant", false, true, true, 1),
        ("synthetic_evidence_class", true, false, true, 1),
        ("transport_unknown", true, true, false, 0),
    ];
    for (name, admission, public, replies, publications) in cases {
        let rig = Rig::new(admission);
        let replies = if replies { rig.replies() } else { Vec::new() };
        let port = if public {
            ScriptedPort::public_class(replies)
        } else {
            ScriptedPort::new(replies)
        };
        let mut actor = rig.actor_with_custody(Some(port.clone()));
        let result = drive(&mut actor).await;
        assert_eq!(rig.stage(&mut actor), "verifying", "{name}: {result:?}");
        assert!(
            actor.store.admitted_direct_checkpoint(&rig.transition()).unwrap().is_none(),
            "{name}"
        );
        assert!(rig.checkpoint_id().is_none(), "{name}");
        assert_eq!(fenced(&rig), publications, "{name}");
    }
}
