//! Composition tests for the direct realization of `establish_checkpoint`.
//!
//! Evidence class: synthetic. The port is a scripted double, not a provider; it
//! proves that the real `establish_checkpoint` composes compile, a separate
//! verify, host assessment, currentness, durable custody and publication, and
//! that the published checkpoint is proof-scoped. It says nothing about provider
//! behavior or production eligibility.

use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use lifecycle_core::{
    AuthorityExpiresAt, BuildId, ContextGeneration, GrantId, GrantScope, ProofRunId, Revision,
    RuntimeIncarnation, SnapshotId, SubjectId, TransitionId, TransitionStage, TrustedGrant,
    WallTimeMs,
};
use lifecycle_semantic::{
    AttributedFinding, AttributedStatement, ContinuationPlan, CriticalFinding,
    DependencyObservation, DependencyRevision, DirectCapture, DirectEvidenceClass,
    DirectInferencePort, DirectPreparedCall, DirectProductionProfile, DirectReleaseState, DirectRole, DirectSubmitError, DirectTerminal,
    DirectWire, ErrorKind, FindingStatus, InteractionStatus, MaterialInput, ObligationKind,
    RequiredObligation, RetainedDecision, RetainedInteraction, SemanticPolicy, SourceAvailability,
    SourceKind, SourceMaterial, SourceReference, SourceTrust, VerifierReport, sha256_hex,
};
use lifecycle_store::{DirectCheckpointOrigin, SqliteLifecycleStore, StoreError};
use serde_json::Value;

use crate::{
    CanonicalMaterialPort, MaterialPortError, NativeConfig, NativeProviderRoute,
    SemanticNativeConfig, ServiceError,
    actor::{NativeGate, clock_sample},
    semantic::{
        ControlRealization, SemanticRuntime, establish_checkpoint, establish_checkpoint_scoped,
    },
    semantic_direct::DirectRealization,
};

pub(crate) const CORPUS: &str = include_str!("../../../tests/fixtures/s67-preparation/semantic-corpus.v1.json");
pub(crate) const MODEL: &str = "synthetic-model-1";
pub(crate) const PROVIDER: &str = "synthetic-provider";

pub(crate) fn fixture() -> Value {
    serde_json::from_str(CORPUS).unwrap()
}

pub(crate) fn source_from_fixture(fixture: &Value, id: &str, current: bool) -> SourceMaterial {
    let row = fixture["source_revisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    let (kind, trust) = if id == "governing" {
        (SourceKind::Governing, SourceTrust::HostGoverning)
    } else if id.starts_with("question-") {
        (SourceKind::HumanInteraction, SourceTrust::HostCanonical)
    } else if id.starts_with("decision-") {
        (SourceKind::Decision, SourceTrust::HostCanonical)
    } else if id.starts_with("queue-") {
        (SourceKind::DeliveryConsequence, SourceTrust::HostCanonical)
    } else {
        (SourceKind::CanonicalRecord, SourceTrust::HostCanonical)
    };
    let dependency_key = if id.starts_with("question-q1-") {
        "question-q1"
    } else if id.starts_with("decision-d") {
        "decision-route"
    } else {
        id
    };
    SourceMaterial {
        id: id.into(),
        owner_namespace: row["owner"].as_str().unwrap().into(),
        owner_ref: id.into(),
        revision: row["revision"].as_u64().unwrap(),
        dependency_key: dependency_key.into(),
        current,
        dependency_required: true,
        kind,
        trust,
        availability: SourceAvailability::Complete,
        bytes: row["bytes_utf8"].as_str().unwrap().as_bytes().to_vec(),
        sha256: row["sha256"].as_str().unwrap().into(),
    }
}

pub(crate) fn c1_input(fixture: &Value) -> MaterialInput {
    let case = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "C1")
        .unwrap();
    let sources = case["source_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| {
            let id = id.as_str().unwrap();
            source_from_fixture(fixture, id, id != "decision-d1")
        })
        .collect();
    let scopes = [
        "Current decision and supersession among cited decision revisions",
        "Human interaction and governing publication scope",
        "Task commitment, uncertainty, and canonical reference custody",
        "Queued delivery effect and its current disposition",
    ];
    let obligations = case["obligations"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, row)| RequiredObligation {
            id: format!("C1-O{}", index + 1),
            kind: ObligationKind::Commitment,
            verifier_scope: scopes[index].into(),
            description: row["consequence"].as_str().unwrap().into(),
            sources: row["attributed_to"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| SourceReference {
                    id: item["source_id"].as_str().unwrap().into(),
                    owner_namespace: item["owner"].as_str().unwrap().into(),
                    owner_ref: item["source_id"].as_str().unwrap().into(),
                    revision: item["revision"].as_u64().unwrap(),
                    sha256: item["sha256"].as_str().unwrap().into(),
                })
                .collect(),
        })
        .collect();
    MaterialInput {
        subject: SubjectId::parse("subject-C1").unwrap(),
        transition: TransitionId::parse("transition-C1").unwrap(),
        capture_id: "capture-C1".into(),
        sources,
        obligations,
        policy: SemanticPolicy::default(),
    }
}

pub(crate) fn c1_candidate(input: &MaterialInput) -> ContinuationPlan {
    let source = |id: &str| input.sources.iter().find(|item| item.id == id).unwrap();
    let attributed = |id: &str| AttributedStatement {
        text: String::from_utf8(source(id).bytes.clone()).unwrap(),
        sources: vec![source(id).reference()],
    };
    ContinuationPlan {
        schema_version: 1,
        objective: attributed("governing"),
        work_position: attributed("commitment-k1"),
        completed: vec![],
        commitments: vec![attributed("commitment-k1")],
        decisions: vec![
            RetainedDecision {
                statement: attributed("decision-d1"),
                current: false,
                supersedes: vec![],
                invalidation_condition: None,
            },
            RetainedDecision {
                statement: attributed("decision-d2"),
                current: true,
                supersedes: vec![source("decision-d1").reference()],
                invalidation_condition: Some(
                    "D-1 external route E-1 is no longer actionable".into(),
                ),
            },
        ],
        uncertainties: vec![attributed("uncertainty-u1")],
        interactions: vec![RetainedInteraction {
            question_id: "Q-1".into(),
            recipient: "O-1".into(),
            scope: "publish T-1 report to channel P-1".into(),
            status: InteractionStatus::Pending,
            answer_id: None,
            statement: attributed("question-q1-r7"),
        }],
        governing: vec![attributed("governing")],
        canonical_refs: vec![attributed("reference-r1")],
        delivery_custody: vec![attributed("queue-i2")],
        next_action: AttributedStatement {
            text: "Continue the internal draft after source verification; ask before publication."
                .into(),
            sources: vec![
                source("governing").reference(),
                source("decision-d2").reference(),
            ],
        },
    }
}

pub(crate) fn c1_report(input: &MaterialInput, status: FindingStatus) -> VerifierReport {
    VerifierReport {
        whole_response_assessed: true,
        findings: input
            .obligations
            .iter()
            .map(|obligation| AttributedFinding {
                obligation_id: obligation.id.clone(),
                status,
                sources: obligation.sources.clone(),
                explanation: "Synthetic finding against the exact C1 material".into(),
            })
            .collect(),
        critical_errors: vec![],
    }
}

/// One scripted reply per submitted call, consumed in call order.
#[derive(Clone)]
pub(crate) struct Reply {
    pub(crate) output: Vec<u8>,
    pub(crate) terminal: DirectTerminal,
    pub(crate) observed_model: String,
    /// The transport lost the outcome after send.
    pub(crate) unknown: bool,
    /// The adapter refused the call with this classification instead.
    pub(crate) refusal: Option<DirectSubmitError>,
}

impl Reply {
    pub(crate) fn complete(output: impl Into<Vec<u8>>) -> Self {
        Self {
            output: output.into(),
            terminal: DirectTerminal::Complete,
            observed_model: MODEL.into(),
            unknown: false,
            refusal: None,
        }
    }

    pub(crate) fn refused(refusal: DirectSubmitError) -> Self {
        Self {
            refusal: Some(refusal),
            ..Self::complete(Vec::new())
        }
    }
}

/// Runs with the zero-based call index while a call is in flight.
pub(crate) type SubmitHook = Box<dyn FnMut(usize) + Send>;

pub(crate) struct ScriptedPort {
    /// The class this scripted double reports. `public_class` is a double that
    /// claims the public transport class; it is not an observation of one.
    pub(crate) evidence: DirectEvidenceClass,
    pub(crate) replies: Mutex<Vec<Reply>>,
    pub(crate) calls: AtomicUsize,
    pub(crate) seen: Mutex<Vec<DirectPreparedCall>>,
    /// Runs after a reply is chosen and before it is returned (for example to
    /// revoke a grant while the call is in flight).
    pub(crate) on_submit: Mutex<Option<SubmitHook>>,
    /// Synthetic release owner. `None`: every call is released (no external
    /// owner). `Some(set)`: only these operation ids are released; any other
    /// call is reported `Pending`.
    pub(crate) released: Mutex<Option<std::collections::HashSet<String>>>,
    /// Calls offered to the release owner, in order.
    pub(crate) offered: Mutex<Vec<DirectPreparedCall>>,
}

impl ScriptedPort {
    pub(crate) fn new(replies: Vec<Reply>) -> Arc<Self> {
        Self::with_evidence(
            replies,
            DirectEvidenceClass::SyntheticLocal {
                fixture_ref: "fixture:service-composition".into(),
            },
        )
    }

    pub(crate) fn public_class(replies: Vec<Reply>) -> Arc<Self> {
        Self::with_evidence(replies, DirectProductionProfile::public_api())
    }

    pub(crate) fn with_evidence(replies: Vec<Reply>, evidence: DirectEvidenceClass) -> Arc<Self> {
        Arc::new(Self {
            evidence,
            replies: Mutex::new(replies),
            calls: AtomicUsize::new(0),
            seen: Mutex::new(Vec::new()),
            on_submit: Mutex::new(None),
            released: Mutex::new(None),
            offered: Mutex::new(Vec::new()),
        })
    }

    /// Require a release for every operation; none is released yet.
    pub(crate) fn hold_releases(&self) {
        *self.released.lock().unwrap() = Some(Default::default());
    }

    pub(crate) fn release(&self, operation: &str) {
        self.released
            .lock()
            .unwrap()
            .get_or_insert_with(Default::default)
            .insert(operation.into());
    }

    pub(crate) fn offered_roles(&self) -> Vec<DirectRole> {
        self.offered.lock().unwrap().iter().map(|c| c.binding.role).collect()
    }

    pub(crate) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl DirectInferencePort for ScriptedPort {
    fn evidence(&self) -> DirectEvidenceClass {
        self.evidence.clone()
    }

    fn wire(&self, role: DirectRole, input: &str) -> Result<DirectWire, DirectSubmitError> {
        let instructions = match role {
            DirectRole::Compile => "compile the continuation",
            DirectRole::Verify => "verify the candidate independently",
        };
        let wire = serde_json::json!({
            "model": MODEL,
            "instructions": instructions,
            "input": [{"role": "user", "content": [{"type": "input_text", "text": input}]}],
            "store": false,
            "stream": true,
        });
        Ok(DirectWire {
            instructions: instructions.into(),
            wire_bytes: serde_json::to_vec(&wire).unwrap(),
            profile_revision: "synthetic-profile-v1".into(),
            requested_provider: PROVIDER.into(),
            requested_model: MODEL.into(),
        })
    }

    fn offer_release(&self, call: &DirectPreparedCall) -> DirectReleaseState {
        self.offered.lock().unwrap().push(call.clone());
        match self.released.lock().unwrap().as_ref() {
            Some(set) if !set.contains(&call.binding.operation_id) => DirectReleaseState::Pending {
                reason: "synthetic release absent".into(),
            },
            _ => DirectReleaseState::Ready,
        }
    }

    fn submit(&self, call: &DirectPreparedCall) -> Result<DirectCapture, DirectSubmitError> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().unwrap().push(call.clone());
        let reply = self.replies.lock().unwrap().remove(0);
        if let Some(hook) = self.on_submit.lock().unwrap().as_mut() {
            hook(index);
        }
        if reply.unknown {
            return Err(DirectSubmitError::OutcomeUnknown);
        }
        if let Some(refusal) = reply.refusal {
            return Err(refusal);
        }
        Ok(DirectCapture {
            operation_id: call.binding.operation_id.clone(),
            attempt_id: call.binding.attempt_id.clone(),
            prepared_wire_sha256: call.wire_sha256.clone(),
            prepared_binding_sha256: call.binding_sha256.clone(),
            capture_ref: format!("capture:{}", call.binding.operation_id),
            captured_stream_sha256: sha256_hex(&reply.output),
            terminal: reply.terminal,
            response_id: Some(format!("resp-{index}")),
            observed_provider: Some(PROVIDER.into()),
            observed_model: Some(reply.observed_model),
            output_sha256: sha256_hex(&reply.output),
            output: reply.output,
            evidence: self.evidence.clone(),
        })
    }
}

pub(crate) struct Material {
    pub(crate) input: MaterialInput,
    pub(crate) stale: AtomicBool,
    pub(crate) unavailable: AtomicBool,
    /// The owner's source changed after the last unfenced observation: only the
    /// fence, which re-reads the owner at commit time, can see it.
    pub(crate) changed_after_observation: AtomicBool,
    /// Commits that ran inside the fence.
    pub(crate) fenced_commits: AtomicUsize,
}

impl CanonicalMaterialPort for Material {
    fn project(
        &self,
        _: &SubjectId,
        _: &TransitionId,
        _: &str,
    ) -> Result<MaterialInput, MaterialPortError> {
        Ok(self.input.clone())
    }

    fn observe_dependencies(
        &self,
        expected: &[DependencyRevision],
    ) -> Result<Vec<DependencyObservation>, MaterialPortError> {
        if self.unavailable.load(Ordering::SeqCst) {
            return Err(MaterialPortError::Unavailable);
        }
        Ok(expected
            .iter()
            .map(|dependency| {
                let mut seen = dependency.clone();
                if self.stale.load(Ordering::SeqCst) {
                    seen.revision += 1;
                    seen.sha256 = sha256_hex(b"changed");
                }
                DependencyObservation::Current(seen)
            })
            .collect())
    }

    fn with_current_fence(
        &self,
        _: &[DependencyRevision],
        commit: &mut dyn FnMut() -> Result<(), ServiceError>,
    ) -> Result<(), ServiceError> {
        if self.changed_after_observation.load(Ordering::SeqCst) {
            return Err(ServiceError::Material(MaterialPortError::Conflict));
        }
        self.fenced_commits.fetch_add(1, Ordering::SeqCst);
        commit()
    }
}

pub(crate) fn private_dir(path: &Path) {
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

pub(crate) const ISSUER: &str = "issuer:direct";

pub(crate) struct Harness {
    pub(crate) _temp: tempfile::TempDir,
    pub(crate) root: std::path::PathBuf,
    pub(crate) material: Arc<Material>,
    pub(crate) subject: SubjectId,
    pub(crate) context: ContextGeneration,
    pub(crate) transition: TransitionId,
    pub(crate) input: MaterialInput,
}

impl Harness {
    pub(crate) fn new() -> (Self, SqliteLifecycleStore) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        private_dir(&root);
        let input = c1_input(&fixture());
        let subject = input.subject.clone();
        let transition = input.transition.clone();
        let context = ContextGeneration::parse("context:direct").unwrap();
        let mut store = SqliteLifecycleStore::open(&root, ISSUER.into()).unwrap();
        store
            .register_subject(
                subject.clone(),
                context.clone(),
                BuildId::parse("build:direct").unwrap(),
                ProofRunId::parse("proof:direct").unwrap(),
                clock_sample(),
            )
            .unwrap();
        let connection = rusqlite::Connection::open(root.join("lifecycle.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO transitions(transition_id,subject_id,predecessor_context,stage,basis_revision,created_wall_ms) VALUES(?1,?2,?3,'verifying',0,100)",
                rusqlite::params![transition.as_str(), subject.as_str(), context.as_str()],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE transitions SET semantic_required=1 WHERE transition_id=?1",
                [transition.as_str()],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE subjects SET owner_kind='transition',owner_ref=?1,semantic_revision=1 WHERE subject_id=?2",
                rusqlite::params![transition.as_str(), subject.as_str()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO grants(grant_id,issuer,principal,subject_id,context_id,build_id,proof_run_id,scope,expires_ms,revision,revoked) VALUES('grant:direct',?1,'principal:direct',?2,?3,'build:direct','proof:direct','semantic_control',?4,1,0)",
                rusqlite::params![
                    ISSUER,
                    subject.as_str(),
                    context.as_str(),
                    i64::MAX / 2
                ],
            )
            .unwrap();
        let material = Arc::new(Material {
            input: input.clone(),
            stale: AtomicBool::new(false),
            unavailable: AtomicBool::new(false),
            changed_after_observation: AtomicBool::new(false),
            fenced_commits: AtomicUsize::new(0),
        });
        (
            Self {
                _temp: temp,
                root,
                material,
                subject,
                context,
                transition,
                input,
            },
            store,
        )
    }

    pub(crate) fn reopen(&self) -> SqliteLifecycleStore {
        SqliteLifecycleStore::open(&self.root, ISSUER.into()).unwrap()
    }

    pub(crate) fn count(&self, query: &str) -> i64 {
        rusqlite::Connection::open(self.root.join("lifecycle.sqlite"))
            .unwrap()
            .query_row(query, [], |row| row.get(0))
            .unwrap()
    }

    pub(crate) fn runtime(&self, port: Arc<ScriptedPort>) -> SemanticRuntime {
        self.runtime_with(port)
    }

    /// As `runtime`, over any trusted-composition port (for example the real
    /// bounded-inference adapter).
    pub(crate) fn runtime_with(&self, port: Arc<dyn DirectInferencePort>) -> SemanticRuntime {
        let semantic = SemanticNativeConfig {
            capture_id: "capture-C1".into(),
            compile_session_id: "session:unused-compile".into(),
            verify_session_id: "session:unused-verify".into(),
            successor_verify_session_id: "session:unused-successor".into(),
            compile_invocation_id: "invocation:unused-compile".into(),
            compile_attempt_id: "attempt:unused-compile".into(),
            candidate_verify_invocation_id: "invocation:unused-verify".into(),
            candidate_verify_attempt_id: "attempt:unused-verify".into(),
            successor_verify_invocation_id: "invocation:unused-successor".into(),
            successor_verify_attempt_id: "attempt:unused-successor".into(),
            challenge_id: "challenge:direct".into(),
            challenge_text: "offline".into(),
            control_grant_ref: "grant:direct".into(),
            control_grant_revision: 1,
            successor_control_grant_ref: None,
            successor_control_grant_revision: None,
        };
        let native = NativeConfig {
            executable: self._temp.path().join("never-executed"),
            root: self._temp.path().join("native-unused"),
            provider_route: NativeProviderRoute::NativeSimProofV1,
            chatgpt_access_source: None,
            simulator_base_url: "http://127.0.0.1:1/v1".into(),
            session_id: "session:unused".into(),
            invocation_id: "invocation:unused".into(),
            attempt_id: "attempt:unused".into(),
            prompt: "unused".into(),
            turn_grant_ref: "grant:direct".into(),
            turn_grant_revision: 1,
            snapshot_grant_ref: "grant:unused-snapshot".into(),
            snapshot_grant_revision: 1,
            same_native_session_required: false,
            followup_turns: Vec::new(),
            snapshot: crate::operations::workspace_snapshot_read::TrustedSnapshot {
                id: "snapshot:direct".into(),
                source_revision: "source:direct".into(),
                members: Vec::new(),
            },
            semantic: Some(semantic.clone()),
            semantic_contexts: Vec::new(),
        };
        SemanticRuntime {
            material_port: self.material.clone(),
            native,
            semantic,
            incarnation: RuntimeIncarnation::parse("incarnation:direct").unwrap(),
            snapshot: SnapshotId::parse("snapshot:direct").unwrap(),
            snapshot_manifest_sha256: "0".into(),
            gate: Arc::new(NativeGate::new(false)),
            uid: fs::metadata("/proc/self").unwrap().uid(),
            principal: "principal:direct".into(),
            proof_dir: None,
            realization: ControlRealization::Direct(DirectRealization {
                port,
                compile_operation_id: "direct-compile-1".into(),
                compile_attempt_id: "direct-compile-attempt-1".into(),
                compile_exposure_id: "exposure:direct-compile-1".into(),
                verify_operation_id: "direct-verify-1".into(),
                verify_attempt_id: "direct-verify-attempt-1".into(),
                verify_exposure_id: "exposure:direct-verify-1".into(),
                pending: Default::default(),
            }),
        }
    }

    /// Proof scope: returns Some for any published direct checkpoint, admitted
    /// or not. The existing proof tests keep this meaning.
    pub(crate) async fn establish(
        &self,
        store: &mut SqliteLifecycleStore,
        runtime: &SemanticRuntime,
    ) -> Result<Option<lifecycle_core::CheckpointId>, StoreError> {
        establish_checkpoint_scoped(
            store,
            runtime,
            &self.subject,
            &self.context,
            &self.transition,
            1,
        )
        .await
    }

    /// The production entry the engine calls.
    pub(crate) async fn establish_production(
        &self,
        store: &mut SqliteLifecycleStore,
        runtime: &SemanticRuntime,
    ) -> Result<Option<lifecycle_core::CheckpointId>, StoreError> {
        establish_checkpoint(store, runtime, &self.subject, &self.context, &self.transition, 1)
            .await
    }

    /// Install the owner's admission grant exactly as trusted startup does:
    /// through the store's trusted-issuer path.
    pub(crate) fn install_admission(&self, store: &mut SqliteLifecycleStore, issuer: &str) -> Result<(), StoreError> {
        store.install_trusted_grant(TrustedGrant {
            id: GrantId::parse("grant:direct-admission").unwrap(),
            issuer: issuer.into(),
            principal: "principal:direct".into(),
            subject: self.subject.clone(),
            context: self.context.clone(),
            build: BuildId::parse("build:direct").unwrap(),
            proof_run: ProofRunId::parse("proof:direct").unwrap(),
            scope: GrantScope::SemanticDirectCheckpoint,
            expires: AuthorityExpiresAt::new(WallTimeMs::new(i64::MAX / 2)),
            revision: Revision::new(1),
            revoked: false,
        })
    }

    pub(crate) fn candidate_json(&self) -> Vec<u8> {
        serde_json::to_vec(&c1_candidate(&self.input)).unwrap()
    }

    pub(crate) fn report_json(&self, status: FindingStatus) -> Vec<u8> {
        serde_json::to_vec(&c1_report(&self.input, status)).unwrap()
    }

    pub(crate) fn rejecting_report_json(&self) -> Vec<u8> {
        let mut report = c1_report(&self.input, FindingStatus::Satisfied);
        report.critical_errors.push(CriticalFinding {
            kind: ErrorKind::OmittedMaterialCommitment,
            sources: self.input.obligations[0].sources.clone(),
            explanation: "synthetic critical finding".into(),
        });
        serde_json::to_vec(&report).unwrap()
    }

    pub(crate) fn good_replies(&self) -> Vec<Reply> {
        vec![
            Reply::complete(self.candidate_json()),
            Reply::complete(self.report_json(FindingStatus::Satisfied)),
        ]
    }
}

/// No authoritative consumer may see a direct-origin checkpoint.
pub(crate) fn assert_not_authoritative(store: &SqliteLifecycleStore, transition: &TransitionId) {
    assert!(store.semantic_checkpoint(transition).unwrap().is_none());
    assert!(store.native_rehydration_receipt(transition).unwrap().is_none());
}

#[tokio::test]
async fn success_publishes_a_proof_scoped_checkpoint_from_two_distinct_calls() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(harness.good_replies());
    let runtime = harness.runtime(port.clone());
    let id = harness
        .establish(&mut store, &runtime)
        .await
        .unwrap()
        .expect("accepted direct checkpoint");
    assert_eq!(port.calls(), 2);

    let seen = port.seen.lock().unwrap().clone();
    assert_eq!(seen[0].binding.role, DirectRole::Compile);
    assert_eq!(seen[1].binding.role, DirectRole::Verify);
    assert_ne!(seen[0].binding.operation_id, seen[1].binding.operation_id);
    assert_ne!(seen[0].binding.attempt_id, seen[1].binding.attempt_id);
    assert_ne!(seen[0].binding.exposure_id, seen[1].binding.exposure_id);
    assert_ne!(seen[0].wire_sha256, seen[1].wire_sha256);
    assert_ne!(seen[0].input_sha256, seen[1].input_sha256);
    assert_eq!(
        seen[1].compile_binding_sha256.as_deref(),
        Some(seen[0].binding_sha256.as_str())
    );

    let proof = store
        .direct_proof_checkpoint(&harness.transition)
        .unwrap()
        .expect("proof-scoped checkpoint");
    assert_eq!(id.as_str(), format!("checkpoint:{}", proof.package.candidate.sha256));
    assert_eq!(proof.evidence_class, "synthetic_local");
    assert_eq!(proof.evidence_ref, "fixture:service-composition");
    assert_eq!(proof.package.candidate.compile_session_id, None);
    let provenance = proof.package.candidate.direct.as_ref().unwrap();
    assert_eq!(provenance.prepared_binding_sha256, seen[0].binding_sha256);
    assert_eq!(provenance.observed_model, MODEL);
    assert!(matches!(
        store.semantic_checkpoint_origin(&harness.transition).unwrap(),
        Some(DirectCheckpointOrigin::Direct { .. })
    ));
    // The decisive boundary: nothing authoritative reads this checkpoint.
    assert_not_authoritative(&store, &harness.transition);
    assert_eq!(harness.count("SELECT COUNT(*) FROM semantic_results"), 0);
    assert_eq!(harness.count("SELECT COUNT(*) FROM native_sessions"), 0);

    // Repeating the operation is idempotent and sends nothing.
    assert_eq!(
        harness.establish(&mut store, &runtime).await.unwrap().as_ref(),
        Some(&id)
    );
    assert_eq!(port.calls(), 2);
}

pub(crate) async fn assert_no_checkpoint(
    harness: &Harness,
    store: &SqliteLifecycleStore,
    result: Result<Option<lifecycle_core::CheckpointId>, StoreError>,
) {
    assert!(!matches!(result, Ok(Some(_))), "{result:?}");
    assert!(store.direct_proof_checkpoint(&harness.transition).unwrap().is_none());
    assert_not_authoritative(store, &harness.transition);
}

#[tokio::test]
async fn rejecting_verifier_records_a_verdict_but_never_publishes() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(vec![
        Reply::complete(harness.candidate_json()),
        Reply::complete(harness.rejecting_report_json()),
    ]);
    let runtime = harness.runtime(port.clone());
    let result = harness.establish(&mut store, &runtime).await;
    assert!(matches!(&result, Ok(None)), "{result:?}");
    assert_no_checkpoint(&harness, &store, result).await;
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM direct_semantic_results WHERE verdict='rejected'"),
        1
    );
}

#[tokio::test]
async fn malformed_incomplete_and_wrong_model_verifiers_cannot_publish() {
    type Case = (&'static str, Box<dyn Fn(&Harness) -> Reply>);
    let cases: Vec<Case> = vec![
        ("malformed", Box::new(|_| Reply::complete(b"not json".to_vec()))),
        (
            "incomplete",
            Box::new(|h| Reply {
                terminal: DirectTerminal::Incomplete,
                ..Reply::complete(h.report_json(FindingStatus::Satisfied))
            }),
        ),
        (
            "failed",
            Box::new(|h| Reply {
                terminal: DirectTerminal::Failed,
                ..Reply::complete(h.report_json(FindingStatus::Satisfied))
            }),
        ),
        (
            "wrong_model",
            Box::new(|h| Reply {
                observed_model: "other-model".into(),
                ..Reply::complete(h.report_json(FindingStatus::Satisfied))
            }),
        ),
    ];
    for (name, verifier) in cases {
        let (harness, mut store) = Harness::new();
        let port = ScriptedPort::new(vec![
            Reply::complete(harness.candidate_json()),
            verifier(&harness),
        ]);
        let runtime = harness.runtime(port.clone());
        let result = harness.establish(&mut store, &runtime).await;
        // An unusable verifier call is refused before any verdict exists; a
        // complete call whose model identity differs yields a recorded verdict
        // that is not acceptance.
        if name == "wrong_model" {
            assert!(matches!(&result, Ok(None)), "{name}: {result:?}");
            assert_eq!(
                harness.count("SELECT COUNT(*) FROM direct_semantic_results"),
                1,
                "{name}"
            );
        } else {
            assert!(matches!(&result, Err(StoreError::Rejected)), "{name}: {result:?}");
            assert_eq!(
                harness.count("SELECT COUNT(*) FROM direct_semantic_results"),
                0,
                "{name}"
            );
        }
        assert_no_checkpoint(&harness, &store, result).await;
        assert_eq!(
            harness.count("SELECT COUNT(*) FROM direct_semantic_results WHERE verdict='accepted'"),
            0,
            "{name}"
        );
        assert_eq!(port.calls(), 2, "{name}");
    }
}

#[tokio::test]
async fn invalid_compile_output_is_refused_and_never_resent() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(vec![Reply::complete(b"{\"not\":\"a plan\"}".to_vec())]);
    let runtime = harness.runtime(port.clone());
    assert!(matches!(harness.establish(&mut store, &runtime).await, Err(StoreError::Rejected)));
    assert_eq!(port.calls(), 1);
    drop(store);
    let mut store = harness.reopen();
    assert!(harness.establish(&mut store, &runtime).await.is_err());
    assert_eq!(port.calls(), 1);
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM direct_semantic_attempts WHERE state='refused'"),
        1
    );
}

#[tokio::test]
async fn source_stale_before_record_fences_the_output() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(harness.good_replies());
    let runtime = harness.runtime(port.clone());
    harness.material.stale.store(true, Ordering::SeqCst);
    let result = harness.establish(&mut store, &runtime).await;
    assert!(matches!(&result, Ok(None)), "{result:?}");
    assert_no_checkpoint(&harness, &store, result).await;
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_results"), 0);
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM semantic_captures WHERE eligibility!='current'"),
        1
    );
}

#[tokio::test]
async fn revoked_or_revised_grant_fences_the_result() {
    for (name, statement) in [
        ("revoked", "UPDATE grants SET revoked=1 WHERE grant_id='grant:direct'"),
        ("revised", "UPDATE grants SET revision=2 WHERE grant_id='grant:direct'"),
    ] {
        let (harness, mut store) = Harness::new();
        let port = ScriptedPort::new(harness.good_replies());
        let runtime = harness.runtime(port.clone());
        // The grant changes while the verifier call is in flight.
        let root = harness.root.clone();
        *port.on_submit.lock().unwrap() = Some(Box::new(move |index| {
            if index == 1 {
                rusqlite::Connection::open(root.join("lifecycle.sqlite"))
                    .unwrap()
                    .execute_batch(statement)
                    .unwrap();
            }
        }));
        let result = harness.establish(&mut store, &runtime).await;
        assert!(result.is_err(), "{name}: {result:?}");
        assert_no_checkpoint(&harness, &store, result).await;
        assert_eq!(
            harness.count("SELECT COUNT(*) FROM direct_semantic_results"),
            0,
            "{name}"
        );
    }
}

#[tokio::test]
async fn unknown_outcome_and_restart_never_retransmit() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(vec![Reply {
        unknown: true,
        ..Reply::complete(Vec::new())
    }]);
    let runtime = harness.runtime(port.clone());
    assert!(matches!(harness.establish(&mut store, &runtime).await, Err(StoreError::ClaimConflict)));
    assert_eq!(port.calls(), 1);
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM direct_semantic_attempts WHERE state='intent_recorded'"),
        1
    );
    // Process restart: a fresh store handle and a fresh runtime see the intent.
    drop(store);
    let mut store = harness.reopen();
    let again = ScriptedPort::new(harness.good_replies());
    let runtime = harness.runtime(again.clone());
    assert!(matches!(harness.establish(&mut store, &runtime).await, Err(StoreError::ClaimConflict)));
    assert_eq!(again.calls(), 0);
    assert_eq!(port.calls(), 1);
}

#[tokio::test]
async fn terminal_persisted_before_interpretation_survives_restart_without_resend() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(harness.good_replies());
    let runtime = harness.runtime(port.clone());
    // Both terminals are durable, then the dependency owner becomes unavailable
    // before the verdict can be recorded.
    harness.material.unavailable.store(true, Ordering::SeqCst);
    assert!(matches!(harness.establish(&mut store, &runtime).await, Err(StoreError::Rejected)));
    assert_eq!(port.calls(), 2);
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_results"), 0);
    drop(store);

    let mut store = harness.reopen();
    harness.material.unavailable.store(false, Ordering::SeqCst);
    let resumed = ScriptedPort::new(Vec::new());
    let runtime = harness.runtime(resumed.clone());
    let id = harness.establish(&mut store, &runtime).await.unwrap();
    assert!(id.is_some());
    assert_eq!(resumed.calls(), 0, "restart must reuse durable terminals");
    assert!(
        store
            .direct_proof_checkpoint(&harness.transition)
            .unwrap()
            .is_some()
    );
    assert_not_authoritative(&store, &harness.transition);
}

// ---- Owner admission and the production entry -------------------------------

/// No admitted-direct reader may see this transition, and no native consumer
/// ever reads a direct checkpoint.
pub(crate) fn assert_unadmitted(store: &SqliteLifecycleStore, transition: &TransitionId) {
    assert!(store.admitted_direct_checkpoint(transition).unwrap().is_none());
    assert_not_authoritative(store, transition);
}

#[tokio::test]
async fn production_entry_does_not_establish_synthetic_or_unadmitted_public_class() {
    // Synthetic port, no grant; public-class port, no grant; synthetic port
    // with a grant. None is established by the production entry, each leaves
    // its proof, and none is visible to any authority consumer.
    for (name, public, grant) in [
        ("synthetic", false, false),
        ("public_no_grant", true, false),
        ("synthetic_with_grant", false, true),
    ] {
        let (harness, mut store) = Harness::new();
        if grant {
            harness.install_admission(&mut store, ISSUER).unwrap();
        }
        let replies = harness.good_replies();
        let port = if public {
            ScriptedPort::public_class(replies)
        } else {
            ScriptedPort::new(replies)
        };
        let runtime = harness.runtime(port.clone());
        let result = harness.establish_production(&mut store, &runtime).await;
        assert!(matches!(result, Ok(None)), "{name}: {result:?}");
        assert_eq!(port.calls(), 2, "{name}");
        assert!(store.direct_proof_checkpoint(&harness.transition).unwrap().is_some(), "{name}");
        assert_unadmitted(&store, &harness.transition);
        assert!(matches!(
            store.semantic_checkpoint_origin(&harness.transition).unwrap(),
            Some(DirectCheckpointOrigin::Direct { .. })
        ), "{name}");
        // Repeating neither resends nor establishes.
        assert!(matches!(harness.establish_production(&mut store, &runtime).await, Ok(None)), "{name}");
        assert_eq!(port.calls(), 2, "{name}");
    }
}

#[tokio::test]
async fn owner_grant_with_public_class_establishes_an_admitted_direct_checkpoint() {
    let (harness, mut store) = Harness::new();
    harness.install_admission(&mut store, ISSUER).unwrap();
    let port = ScriptedPort::public_class(harness.good_replies());
    let runtime = harness.runtime(port.clone());
    let id = harness
        .establish_production(&mut store, &runtime)
        .await
        .unwrap()
        .expect("admitted direct checkpoint");
    assert_eq!(port.calls(), 2);
    let admitted = store
        .admitted_direct_checkpoint(&harness.transition)
        .unwrap()
        .expect("admitted");
    assert_eq!(id.as_str(), format!("checkpoint:{}", admitted.package.candidate.sha256));
    assert_eq!(admitted.evidence_class, "public_api");
    assert_eq!(admitted.admission_grant_id, "grant:direct-admission");
    assert_eq!(admitted.admission_grant_revision, 1);
    assert!(matches!(
        store.semantic_checkpoint_origin(&harness.transition).unwrap(),
        Some(DirectCheckpointOrigin::DirectAdmitted { .. })
    ));
    // The native successor consumers still see nothing.
    assert!(store.semantic_checkpoint(&harness.transition).unwrap().is_none());
    assert!(store.native_rehydration_receipt(&harness.transition).unwrap().is_none());
    assert_eq!(harness.count("SELECT COUNT(*) FROM semantic_results"), 0);
    assert_eq!(harness.count("SELECT COUNT(*) FROM native_sessions"), 0);
    // Idempotent, no resend, also after restart.
    assert_eq!(harness.establish_production(&mut store, &runtime).await.unwrap().as_ref(), Some(&id));
    drop(store);
    let mut store = harness.reopen();
    let fresh = ScriptedPort::new(Vec::new());
    let runtime = harness.runtime(fresh.clone());
    assert_eq!(harness.establish_production(&mut store, &runtime).await.unwrap().as_ref(), Some(&id));
    assert_eq!(fresh.calls(), 0);
}

#[tokio::test]
async fn admission_requires_the_trusted_issuer() {
    let (harness, mut store) = Harness::new();
    assert!(matches!(
        harness.install_admission(&mut store, "issuer:someone-else"),
        Err(StoreError::Rejected)
    ));
    let port = ScriptedPort::public_class(harness.good_replies());
    let runtime = harness.runtime(port);
    assert!(matches!(harness.establish_production(&mut store, &runtime).await, Ok(None)));
    assert_unadmitted(&store, &harness.transition);
}

#[tokio::test]
async fn admission_grant_revoked_in_flight_or_at_publication_leaves_proof_only() {
    for (name, statement) in [
        ("revoked", "UPDATE grants SET revoked=1 WHERE grant_id='grant:direct-admission'"),
        ("expired", "UPDATE grants SET expires_ms=1 WHERE grant_id='grant:direct-admission'"),
    ] {
        let (harness, mut store) = Harness::new();
        harness.install_admission(&mut store, ISSUER).unwrap();
        let port = ScriptedPort::public_class(harness.good_replies());
        let runtime = harness.runtime(port.clone());
        let root = harness.root.clone();
        *port.on_submit.lock().unwrap() = Some(Box::new(move |index| {
            if index == 1 {
                rusqlite::Connection::open(root.join("lifecycle.sqlite"))
                    .unwrap()
                    .execute_batch(statement)
                    .unwrap();
            }
        }));
        let result = harness.establish_production(&mut store, &runtime).await;
        assert!(matches!(result, Ok(None)), "{name}: {result:?}");
        assert_unadmitted(&store, &harness.transition);
        assert!(store.direct_proof_checkpoint(&harness.transition).unwrap().is_some(), "{name}");
    }
}

#[tokio::test]
async fn admission_installed_after_a_proof_result_does_not_retroactively_admit_it() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::public_class(harness.good_replies());
    let runtime = harness.runtime(port.clone());
    assert!(matches!(harness.establish_production(&mut store, &runtime).await, Ok(None)));
    harness.install_admission(&mut store, ISSUER).unwrap();
    assert!(matches!(harness.establish_production(&mut store, &runtime).await, Ok(None)));
    assert_unadmitted(&store, &harness.transition);
    assert_eq!(port.calls(), 2);
}

impl Harness {
    /// The transition machinery requires a native basis for a semantic
    /// transition. A raw connection stands in for it (foreign keys are disabled on
    /// this connection); the guard under test reads only the package.
    pub(crate) fn mark_native_basis(&self, stage: &str) {
        let connection = rusqlite::Connection::open(self.root.join("lifecycle.sqlite")).unwrap();
        connection.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        connection
            .execute(
                "UPDATE transitions SET native_basis_source='invocation:basis',stage=?2 WHERE transition_id=?1",
                rusqlite::params![self.transition.as_str(), stage],
            )
            .unwrap();
    }

    pub(crate) fn subject_revision(&self) -> u64 {
        u64::try_from(self.count(&format!(
            "SELECT revision FROM subjects WHERE subject_id='{}'",
            self.subject.as_str()
        )))
        .unwrap()
    }
}

#[tokio::test]
async fn admitted_direct_package_is_accepted_at_verification_and_nowhere_after() {
    use lifecycle_core::{EvidenceId, TransitionFact};
    let verified = |checkpoint: lifecycle_core::CheckpointId| TransitionFact::VerificationEstablished {
        evidence: EvidenceId::parse("semantic:transition-C1:checkpoint").unwrap(),
        checkpoint,
    };

    // Unadmitted (proof-only) direct package: verification is refused.
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::public_class(harness.good_replies());
    let runtime = harness.runtime(port);
    let proof = harness.establish(&mut store, &runtime).await.unwrap().unwrap();
    harness.mark_native_basis("verifying");
    let refused = store.apply_transition_fact(
        &harness.transition,
        Revision::new(harness.subject_revision()),
        verified(proof),
        None,
        clock_sample(),
    );
    assert!(matches!(refused, Err(StoreError::Rejected)), "{refused:?}");

    // Admitted: verification is accepted.
    let (harness, mut store) = Harness::new();
    harness.install_admission(&mut store, ISSUER).unwrap();
    let port = ScriptedPort::public_class(harness.good_replies());
    let runtime = harness.runtime(port);
    let checkpoint = harness.establish_production(&mut store, &runtime).await.unwrap().unwrap();
    harness.mark_native_basis("verifying");
    let state = store
        .apply_transition_fact(
            &harness.transition,
            Revision::new(harness.subject_revision()),
            verified(checkpoint),
            None,
            clock_sample(),
        )
        .expect("admitted direct package establishes verification");
    assert_eq!(state.stage, TransitionStage::Checkpointed);

    // The transition may not move on toward a native successor: authorizing the
    // switch would start a session with no native checkpoint to rehydrate from.
    let switch = store.apply_transition_fact(
        &harness.transition,
        Revision::new(harness.subject_revision()),
        TransitionFact::SwitchAuthorized {
            evidence: EvidenceId::parse("semantic:transition-C1:switch").unwrap(),
        },
        None,
        clock_sample(),
    );
    assert!(matches!(switch, Err(StoreError::Rejected)), "{switch:?}");
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM transitions WHERE stage='checkpointed'"),
        1
    );

    // The native successor path stays closed to it: a commit with no native
    // successor adequacy cannot be applied.
    harness.mark_native_basis("ready_to_commit");
    let commit = store.apply_transition_fact(
        &harness.transition,
        Revision::new(harness.subject_revision()),
        TransitionFact::CommitSuccessor {
            evidence: EvidenceId::parse("semantic:transition-C1:commit").unwrap(),
            semantic_revision: Revision::new(1),
        },
        None,
        clock_sample(),
    );
    assert!(matches!(commit, Err(StoreError::Rejected)), "{commit:?}");
    assert!(store.native_rehydration_receipt(&harness.transition).unwrap().is_none());
}

// ---- Currentness between publication and verification ------------------------

fn verification_fact(checkpoint: lifecycle_core::CheckpointId) -> lifecycle_core::TransitionFact {
    lifecycle_core::TransitionFact::VerificationEstablished {
        evidence: lifecycle_core::EvidenceId::parse("semantic:transition-C1:checkpoint").unwrap(),
        checkpoint,
    }
}

#[tokio::test]
async fn source_change_after_admitted_publication_fences_verification() {
    let (harness, mut store) = Harness::new();
    harness.install_admission(&mut store, ISSUER).unwrap();
    let port = ScriptedPort::public_class(harness.good_replies());
    let runtime = harness.runtime(port.clone());
    let checkpoint = harness.establish_production(&mut store, &runtime).await.unwrap().unwrap();
    harness.mark_native_basis("verifying");

    // The engine re-enters establishment after a restart while the source has
    // moved on: the stale capture is retired and the checkpoint is not visible.
    harness.material.stale.store(true, Ordering::SeqCst);
    let again = harness.establish_production(&mut store, &runtime).await;
    assert!(matches!(again, Ok(None)), "{again:?}");
    assert_eq!(port.calls(), 2, "nothing is resent");
    assert!(store.admitted_direct_checkpoint(&harness.transition).unwrap().is_none());
    let refused = store.apply_transition_fact(
        &harness.transition,
        Revision::new(harness.subject_revision()),
        verification_fact(checkpoint),
        None,
        clock_sample(),
    );
    assert!(matches!(refused, Err(StoreError::Rejected)), "{refused:?}");
}

#[tokio::test]
async fn admission_withdrawn_after_publication_refuses_verification() {
    let (harness, mut store) = Harness::new();
    harness.install_admission(&mut store, ISSUER).unwrap();
    let port = ScriptedPort::public_class(harness.good_replies());
    let runtime = harness.runtime(port);
    let checkpoint = harness.establish_production(&mut store, &runtime).await.unwrap().unwrap();
    harness.mark_native_basis("verifying");
    assert!(store.admitted_direct_checkpoint(&harness.transition).unwrap().is_some());

    // The owner withdraws the profile after publication, before the lifecycle
    // consumes the checkpoint.
    rusqlite::Connection::open(harness.root.join("lifecycle.sqlite"))
        .unwrap()
        .execute_batch("UPDATE grants SET revoked=1 WHERE grant_id='grant:direct-admission'")
        .unwrap();
    let refused = store.apply_transition_fact(
        &harness.transition,
        Revision::new(harness.subject_revision()),
        verification_fact(checkpoint),
        None,
        clock_sample(),
    );
    assert!(matches!(refused, Err(StoreError::Rejected)), "{refused:?}");
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM transitions WHERE stage='verifying'"),
        1
    );
}

// ---- Owner release before intent ------------------------------------------
//
// Evidence class: synthetic. The scripted port stands in for the release owner;
// these tests prove the lifecycle side: an absent release records and consumes
// nothing, a re-drive repeats every check, and recorded attempts are reported
// as they are whether or not a release is present.

fn waiting(runtime: &SemanticRuntime) -> Option<crate::semantic_direct::DirectPending> {
    match &runtime.realization {
        ControlRealization::Direct(direct) => direct.pending.lock().unwrap().clone(),
        ControlRealization::Native => None,
    }
}

#[tokio::test]
async fn missing_release_is_pending_before_intent_and_each_release_sends_its_call_once() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(harness.good_replies());
    port.hold_releases();
    let runtime = harness.runtime(port.clone());

    for _ in 0..2 {
        let result = harness.establish(&mut store, &runtime).await;
        assert!(matches!(result, Ok(None)), "{result:?}");
        assert_eq!(port.calls(), 0);
        assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 0);
        let pending = waiting(&runtime).expect("compile waits for its release");
        assert_eq!(pending.operation_id, "direct-compile-1");
    }
    assert_eq!(port.offered_roles(), vec![DirectRole::Compile, DirectRole::Compile]);

    // The compile release arrives: exactly one compile send, then the verify
    // call is prepared from the settled candidate and waits for its own release.
    port.release("direct-compile-1");
    let result = harness.establish(&mut store, &runtime).await;
    assert!(matches!(result, Ok(None)), "{result:?}");
    assert_eq!(port.calls(), 1);
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM direct_semantic_attempts WHERE role='compile' AND state='completed_settled'"),
        1
    );
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts WHERE role='verify'"), 0);
    let pending = waiting(&runtime).expect("verify waits for its release");
    assert_eq!(pending.operation_id, "direct-verify-1");
    let offered = port.offered.lock().unwrap().clone();
    let compile = port.seen.lock().unwrap()[0].clone();
    let verify = offered.last().unwrap();
    assert_eq!(verify.binding.role, DirectRole::Verify);
    assert_eq!(verify.compile_binding_sha256.as_deref(), Some(compile.binding_sha256.as_str()));
    assert_eq!(pending.prepared_binding_sha256, verify.binding_sha256);

    // Restart while verify is pending: the recorded compile is reused and
    // never offered or sent again; the verify call is offered again.
    drop(store);
    let mut store = harness.reopen();
    let again = ScriptedPort::new(vec![Reply::complete(harness.report_json(FindingStatus::Satisfied))]);
    again.hold_releases();
    let runtime = harness.runtime(again.clone());
    assert!(matches!(harness.establish(&mut store, &runtime).await, Ok(None)));
    assert_eq!(again.calls(), 0);
    assert_eq!(again.offered_roles(), vec![DirectRole::Verify]);
    assert_eq!(again.offered.lock().unwrap()[0].binding_sha256, verify.binding_sha256);

    again.release("direct-verify-1");
    let id = harness.establish(&mut store, &runtime).await.unwrap();
    assert!(id.is_some());
    assert_eq!(again.calls(), 1);
    assert_eq!(port.calls(), 1, "compile was sent once, before the restart");
    assert_eq!(waiting(&runtime), None);
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 2);
}

#[tokio::test]
async fn release_arrival_repeats_gate_and_grant_checks_before_any_intent() {
    for case in ["gate", "revoked"] {
        let (harness, mut store) = Harness::new();
        let port = ScriptedPort::new(harness.good_replies());
        port.hold_releases();
        let mut runtime = harness.runtime(port.clone());
        assert!(matches!(harness.establish(&mut store, &runtime).await, Ok(None)), "{case}");
        match case {
            "gate" => runtime.gate = Arc::new(NativeGate::new(true)),
            _ => {
                rusqlite::Connection::open(harness.root.join("lifecycle.sqlite"))
                    .unwrap()
                    .execute_batch("UPDATE grants SET revoked=1 WHERE grant_id='grant:direct'")
                    .unwrap();
            }
        }
        port.release("direct-compile-1");
        let result = harness.establish(&mut store, &runtime).await;
        assert!(result.is_err(), "{case}: {result:?}");
        assert_eq!(port.calls(), 0, "{case}");
        assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 0, "{case}");
    }
}

#[tokio::test]
async fn recorded_uncertainty_is_reported_not_hidden_behind_a_missing_release() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(vec![Reply {
        unknown: true,
        ..Reply::complete(Vec::new())
    }]);
    let runtime = harness.runtime(port.clone());
    assert!(matches!(harness.establish(&mut store, &runtime).await, Err(StoreError::ClaimConflict)));
    assert_eq!(port.calls(), 1);

    // Restart with no release present at all: the possibly sent compile is
    // still reported as uncertain, never as Pending, and never offered again.
    drop(store);
    let mut store = harness.reopen();
    let again = ScriptedPort::new(harness.good_replies());
    again.hold_releases();
    let runtime = harness.runtime(again.clone());
    assert!(matches!(harness.establish(&mut store, &runtime).await, Err(StoreError::ClaimConflict)));
    assert_eq!(again.calls(), 0);
    assert!(again.offered_roles().is_empty());
    assert_eq!(waiting(&runtime), None);
    assert_eq!(
        harness.count("SELECT COUNT(*) FROM direct_semantic_attempts WHERE state='intent_recorded'"),
        1
    );
}

// Withheld at the send boundary. Evidence class: synthetic (scripted port and
// release owner). The adapter's `Withheld` classification itself is exercised
// against the public endpoint in bounded-inference; these controls prove what
// the lifecycle owner does with it: only a positively unconsumed, unsent call
// is withdrawn and re-offered, and every consumed, crashed or uncertain
// attempt stays fenced.

fn withheld_then_good(harness: &Harness) -> Vec<Reply> {
    let mut replies = vec![Reply::refused(DirectSubmitError::Withheld)];
    replies.extend(harness.good_replies());
    replies
}

#[tokio::test]
async fn withheld_send_withdraws_intent_and_the_redrive_sends_the_same_call_once() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(withheld_then_good(&harness));
    let runtime = harness.runtime(port.clone());

    let result = harness.establish(&mut store, &runtime).await;
    assert!(matches!(result, Ok(None)), "{result:?}");
    assert_eq!(port.calls(), 1);
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 0);
    let withheld = port.seen.lock().unwrap()[0].clone();
    let pending = waiting(&runtime).expect("a withheld compile stays pending");
    assert_eq!(pending.operation_id, "direct-compile-1");
    assert_eq!(pending.prepared_binding_sha256, withheld.binding_sha256);
    assert!(pending.reason.contains("withheld"), "{}", pending.reason);
    assert_not_authoritative(&store, &harness.transition);

    // Re-drive: offered again, entered again, and the same exact call is sent
    // once; the verify call follows from its settled candidate.
    let id = harness.establish(&mut store, &runtime).await.unwrap();
    assert!(id.is_some());
    let seen = port.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[1], withheld, "the re-drive submits the same prepared call");
    assert_eq!(seen[2].binding.role, DirectRole::Verify);
    assert_eq!(
        port.offered_roles(),
        vec![DirectRole::Compile, DirectRole::Compile, DirectRole::Verify]
    );
    assert_eq!(waiting(&runtime), None);
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 2);

    // Restart after the checkpoint: nothing is offered or sent again.
    drop(store);
    let mut store = harness.reopen();
    let again = ScriptedPort::new(harness.good_replies());
    let runtime = harness.runtime(again.clone());
    let _ = harness.establish(&mut store, &runtime).await;
    assert_eq!(again.calls(), 0);
    assert_eq!(port.calls(), 3);
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 2);
}

#[tokio::test]
async fn withdrawn_intent_is_absent_after_restart_and_the_call_is_sent_once() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(vec![Reply::refused(DirectSubmitError::Withheld)]);
    let runtime = harness.runtime(port.clone());
    assert!(matches!(harness.establish(&mut store, &runtime).await, Ok(None)));
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 0);

    drop(store);
    let mut store = harness.reopen();
    let again = ScriptedPort::new(harness.good_replies());
    let runtime = harness.runtime(again.clone());
    let id = harness.establish(&mut store, &runtime).await.unwrap();
    assert!(id.is_some());
    assert_eq!(again.calls(), 2);
    assert_eq!(again.offered_roles(), vec![DirectRole::Compile, DirectRole::Verify]);
    assert_eq!(port.calls(), 1);
    assert_eq!(again.seen.lock().unwrap()[0], port.seen.lock().unwrap()[0]);
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 2);
}

#[tokio::test]
async fn redrive_after_withdrawal_repeats_stage_gate_grant_and_material_checks() {
    for case in ["stage", "gate", "revoked", "material"] {
        let (harness, mut store) = Harness::new();
        let port = ScriptedPort::new(withheld_then_good(&harness));
        let mut runtime = harness.runtime(port.clone());
        assert!(matches!(harness.establish(&mut store, &runtime).await, Ok(None)), "{case}");
        assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 0, "{case}");
        let db = || rusqlite::Connection::open(harness.root.join("lifecycle.sqlite")).unwrap();
        match case {
            "stage" => db()
                .execute_batch("UPDATE transitions SET stage='recovery_required'")
                .unwrap(),
            "gate" => runtime.gate = Arc::new(NativeGate::new(true)),
            "revoked" => db()
                .execute_batch("UPDATE grants SET revoked=1 WHERE grant_id='grant:direct'")
                .unwrap(),
            _ => harness.material.stale.store(true, Ordering::SeqCst),
        }
        let result = harness.establish(&mut store, &runtime).await;
        assert!(!matches!(result, Ok(Some(_))), "{case}: {result:?}");
        if case == "material" {
            // Material currentness is fenced when the output is recorded, as
            // on first entry: the re-drive sends each call once and the stale
            // source fences what they returned.
            let seen = port.seen.lock().unwrap().clone();
            assert_eq!(seen.len(), 3, "{case}: one send per call after the withdrawal");
            assert_eq!(seen[1], seen[0]);
            assert_eq!(seen[2].binding.role, DirectRole::Verify);
            assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_results"), 0);
            assert_eq!(
                harness.count("SELECT COUNT(*) FROM semantic_captures WHERE eligibility!='current'"),
                1
            );
            assert_not_authoritative(&store, &harness.transition);
            continue;
        }
        assert_eq!(port.calls(), 1, "{case}: nothing sent after the withdrawal");
        assert_eq!(
            harness.count("SELECT COUNT(*) FROM direct_semantic_attempts WHERE state!='intent_recorded' OR role!='compile'"),
            0,
            "{case}"
        );
        assert_not_authoritative(&store, &harness.transition);
    }
}

#[tokio::test]
async fn consumed_crashed_and_uncertain_attempts_are_never_withdrawn_or_resent() {
    for case in ["not_sent", "unknown", "crashed"] {
        let (harness, mut store) = Harness::new();
        let (reply, state) = match case {
            "not_sent" => (Reply::refused(DirectSubmitError::NotSent), "not_sent"),
            "unknown" => (Reply { unknown: true, ..Reply::complete(Vec::new()) }, "intent_recorded"),
            _ => (Reply::complete(harness.candidate_json()), "intent_recorded"),
        };
        let port = ScriptedPort::new(vec![reply]);
        if case == "crashed" {
            *port.on_submit.lock().unwrap() =
                Some(Box::new(|_| panic!("synthetic crash while the call is in flight")));
        }
        let runtime = harness.runtime(port.clone());
        let result = harness.establish(&mut store, &runtime).await;
        assert!(result.is_err(), "{case}: {result:?}");
        assert_eq!(port.calls(), 1, "{case}");
        let recorded = format!("SELECT COUNT(*) FROM direct_semantic_attempts WHERE state='{state}'");
        assert_eq!(harness.count(&recorded), 1, "{case}");
        assert_eq!(waiting(&runtime), None, "{case}");

        // Neither a re-drive nor a restart withdraws, re-offers or resends it.
        assert!(harness.establish(&mut store, &runtime).await.is_err(), "{case}");
        drop(store);
        let mut store = harness.reopen();
        let again = ScriptedPort::new(harness.good_replies());
        let runtime = harness.runtime(again.clone());
        assert!(harness.establish(&mut store, &runtime).await.is_err(), "{case}");
        assert_eq!(again.calls(), 0, "{case}");
        assert!(again.offered_roles().is_empty(), "{case}");
        assert_eq!(waiting(&runtime), None, "{case}");
        assert_eq!(port.calls(), 1, "{case}");
        assert_eq!(harness.count(&recorded), 1, "{case}");
        assert_not_authoritative(&store, &harness.transition);
    }
}

/// The store owner withdraws only an `intent_recorded` row for the exact
/// prepared call. It cannot know why it is asked; only the service's
/// `Withheld` path asks.
#[tokio::test]
async fn store_withdraws_only_an_exact_recorded_intent() {
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(vec![Reply { unknown: true, ..Reply::complete(Vec::new()) }]);
    let runtime = harness.runtime(port.clone());
    assert!(harness.establish(&mut store, &runtime).await.is_err());
    let binding = port.seen.lock().unwrap()[0].binding_sha256.clone();
    let intents = "SELECT COUNT(*) FROM direct_semantic_attempts WHERE state='intent_recorded'";
    assert_eq!(harness.count(intents), 1);
    store.withdraw_direct_intent("direct-absent", &binding).unwrap();
    assert!(matches!(
        store.withdraw_direct_intent("direct-compile-1", &"0".repeat(64)),
        Err(StoreError::ClaimConflict)
    ));
    assert_eq!(harness.count(intents), 1);
    store.withdraw_direct_intent("direct-compile-1", &binding).unwrap();
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 0);

    // Settled and not-sent attempts are kept and reported as conflicts.
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(harness.good_replies());
    let runtime = harness.runtime(port.clone());
    assert!(harness.establish(&mut store, &runtime).await.unwrap().is_some());
    for call in port.seen.lock().unwrap().iter() {
        assert!(matches!(
            store.withdraw_direct_intent(&call.binding.operation_id, &call.binding_sha256),
            Err(StoreError::ClaimConflict)
        ));
    }
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 2);
    let (harness, mut store) = Harness::new();
    let port = ScriptedPort::new(vec![Reply::refused(DirectSubmitError::NotSent)]);
    let runtime = harness.runtime(port.clone());
    assert!(harness.establish(&mut store, &runtime).await.is_err());
    let call = port.seen.lock().unwrap()[0].clone();
    assert!(matches!(
        store.withdraw_direct_intent(&call.binding.operation_id, &call.binding_sha256),
        Err(StoreError::ClaimConflict)
    ));
    assert_eq!(harness.count("SELECT COUNT(*) FROM direct_semantic_attempts WHERE state='not_sent'"), 1);
}
