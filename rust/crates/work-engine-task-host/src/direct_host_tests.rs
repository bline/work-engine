//! Installed-host startup to an admitted Direct checkpoint.
//!
//! Evidence class: synthetic. Each test enters through `serve` with a startup
//! file whose `control` selects Direct or Native, so file and startup
//! validation, the real `compose_control` match, the task owner, the lifecycle
//! service with its actor and engine, and both sockets run as installed. Three
//! things are substituted, each only for the test's own private directory:
//!
//! - the Direct port is a scripted double; `public_class` claims the public
//!   transport class, it does not observe one;
//! - the canonical material is a double holding the C1 corpus case;
//! - the native custody the task profile requires is inserted by SQL into the
//!   running store's database, because no native process exists. It is one
//!   settled domain-work turn whose delivery segment is already `yielded`
//!   before any transition exists, an order the store never produces.
//!
//! These tests therefore establish nothing about native qualification, a
//! provider, or production eligibility.

use std::{
    fs,
    io::{BufRead, BufReader, Write as _},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use lifecycle_core::{SubjectId, TransitionId};
use lifecycle_semantic::{
    AttributedFinding, AttributedStatement, ContinuationPlan, DependencyObservation,
    DependencyRevision, DirectCapture, DirectEvidenceClass, DirectInferencePort,
    DirectPreparedCall, DirectProductionProfile, DirectReleaseState, DirectRole, DirectSubmitError, DirectTerminal,
    DirectWire, FindingStatus, InteractionStatus, MaterialInput, ObligationKind,
    RequiredObligation, RetainedDecision, RetainedInteraction, SemanticPolicy, SourceAvailability,
    SourceKind, SourceMaterial, SourceReference, SourceTrust, VerifierReport, project_material,
    sha256_hex,
};
use lifecycle_service::{CanonicalMaterialPort, MaterialPortError, ServiceError};
use retained_task_workflow::{
    AnswerQuestion, OperationEntry, QuestionOpenPayload, SubmitTask, TaskError, TaskStore,
    json_digest,
};
use serde_json::{Value, json};
use work_engine_types::CodecContract;

use crate::{HostError, current_uid, serve, sha_file};

// ---- Registration: the only substitution point, keyed by private directory ----

struct Registration {
    root: PathBuf,
    port: Option<Arc<dyn DirectInferencePort>>,
    material: Arc<dyn CanonicalMaterialPort>,
    /// How many times startup asked for the Direct port of this directory.
    port_lookups: AtomicUsize,
}

static REGISTRY: Mutex<Vec<Arc<Registration>>> = Mutex::new(Vec::new());

fn registered(root: Option<&Path>) -> Option<Arc<Registration>> {
    let root = root?;
    REGISTRY
        .lock()
        .unwrap()
        .iter()
        .find(|registration| registration.root == root)
        .cloned()
}

/// The synthetic port registered for the directory holding `journal_dir`.
pub(crate) fn registered_port(journal_dir: &Path) -> Option<Arc<dyn DirectInferencePort>> {
    let registration = registered(journal_dir.parent())?;
    registration.port_lookups.fetch_add(1, Ordering::SeqCst);
    registration.port.clone()
}

/// The material double registered for the directory holding `store_root`.
pub(crate) fn registered_material(store_root: &Path) -> Option<Arc<dyn CanonicalMaterialPort>> {
    registered(store_root.parent()).map(|registration| registration.material.clone())
}

struct Registered(Arc<Registration>);

impl Drop for Registered {
    fn drop(&mut self) {
        REGISTRY
            .lock()
            .unwrap()
            .retain(|registration| !Arc::ptr_eq(registration, &self.0));
    }
}

/// The host and service both stop on the process's SIGTERM, so these tests
/// never overlap.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---- C1 corpus material, scripted port and material double ----

const CORPUS: &str =
    include_str!("../../../tests/fixtures/s67-preparation/semantic-corpus.v1.json");
const MODEL: &str = "synthetic-model-1";
const PROVIDER: &str = "synthetic-provider";

fn source_from_fixture(fixture: &Value, id: &str, current: bool) -> SourceMaterial {
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

fn c1_input() -> MaterialInput {
    let fixture: Value = serde_json::from_str(CORPUS).unwrap();
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
            source_from_fixture(&fixture, id, id != "decision-d1")
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
        subject: SubjectId::parse(SUBJECT).unwrap(),
        transition: TransitionId::parse("transition-C1").unwrap(),
        capture_id: CAPTURE.into(),
        sources,
        obligations,
        policy: SemanticPolicy::default(),
    }
}

fn c1_candidate(input: &MaterialInput) -> ContinuationPlan {
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

fn c1_report(input: &MaterialInput, status: FindingStatus) -> VerifierReport {
    VerifierReport {
        whole_response_assessed: true,
        findings: input
            .obligations
            .iter()
            .map(|obligation| AttributedFinding {
                obligation_id: obligation.id.clone(),
                status,
                sources: obligation.sources.clone(),
                explanation: "Synthetic finding against the exact projected material".into(),
            })
            .collect(),
        critical_errors: vec![],
    }
}

/// One scripted reply per submitted call, consumed in call order.
struct Reply {
    output: Vec<u8>,
    /// The transport lost the outcome after send.
    unknown: bool,
}

fn complete(value: &impl serde::Serialize) -> Reply {
    Reply {
        output: serde_json::to_vec(value).unwrap(),
        unknown: false,
    }
}

fn replies(status: FindingStatus) -> Vec<Reply> {
    let input = c1_input();
    vec![complete(&c1_candidate(&input)), complete(&c1_report(&input, status))]
}

struct ScriptedPort {
    evidence: DirectEvidenceClass,
    replies: Mutex<Vec<Reply>>,
    seen: Mutex<Vec<DirectPreparedCall>>,
    /// Synthetic release owner: `None` releases every call; `Some(roles)`
    /// releases only calls of those roles and reports the rest `Pending`.
    released: Mutex<Option<Vec<DirectRole>>>,
    offered: Mutex<Vec<DirectRole>>,
    /// Submits of these roles return `Withheld` (nothing claimed, nothing
    /// sent) once each, before any reply is consumed.
    withhold: Mutex<Vec<DirectRole>>,
    withheld: Mutex<Vec<String>>,
    /// Runs inside the next submit, before its result (sent or withheld)
    /// returns to the service.
    during_next_submit: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

impl ScriptedPort {
    /// A double that claims the public transport class.
    fn public_class(replies: Vec<Reply>) -> Arc<Self> {
        Self::with_evidence(replies, DirectProductionProfile::public_api())
    }

    fn synthetic_class(replies: Vec<Reply>) -> Arc<Self> {
        Self::with_evidence(
            replies,
            DirectEvidenceClass::SyntheticLocal {
                fixture_ref: "fixture:task-host".into(),
            },
        )
    }

    fn with_evidence(replies: Vec<Reply>, evidence: DirectEvidenceClass) -> Arc<Self> {
        Arc::new(Self {
            evidence,
            replies: Mutex::new(replies),
            seen: Mutex::new(Vec::new()),
            released: Mutex::new(None),
            offered: Mutex::new(Vec::new()),
            withhold: Mutex::new(Vec::new()),
            withheld: Mutex::new(Vec::new()),
            during_next_submit: Mutex::new(None),
        })
    }

    fn withhold_once(&self, role: DirectRole) {
        self.withhold.lock().unwrap().push(role);
    }

    fn during_next_submit(&self, action: impl FnOnce() + Send + 'static) {
        *self.during_next_submit.lock().unwrap() = Some(Box::new(action));
    }

    fn hold_releases(&self) {
        *self.released.lock().unwrap() = Some(Vec::new());
    }

    fn release(&self, role: DirectRole) {
        self.released.lock().unwrap().get_or_insert_with(Vec::new).push(role);
    }

    fn calls(&self) -> usize {
        self.seen.lock().unwrap().len()
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
        let wire = json!({
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
        self.offered.lock().unwrap().push(call.binding.role);
        match self.released.lock().unwrap().as_ref() {
            Some(roles) if !roles.contains(&call.binding.role) => DirectReleaseState::Pending {
                reason: "synthetic release absent".into(),
            },
            _ => DirectReleaseState::Ready,
        }
    }

    fn submit(&self, call: &DirectPreparedCall) -> Result<DirectCapture, DirectSubmitError> {
        let action = self.during_next_submit.lock().unwrap().take();
        if let Some(action) = action {
            action();
        }
        {
            let mut withhold = self.withhold.lock().unwrap();
            if let Some(at) = withhold.iter().position(|role| *role == call.binding.role) {
                withhold.remove(at);
                self.withheld.lock().unwrap().push(call.binding_sha256.clone());
                return Err(DirectSubmitError::Withheld);
            }
        }
        let index = {
            let mut seen = self.seen.lock().unwrap();
            seen.push(call.clone());
            seen.len() - 1
        };
        let mut replies = self.replies.lock().unwrap();
        if replies.is_empty() {
            return Err(DirectSubmitError::NotSent);
        }
        let reply = replies.remove(0);
        if reply.unknown {
            return Err(DirectSubmitError::OutcomeUnknown);
        }
        Ok(DirectCapture {
            operation_id: call.binding.operation_id.clone(),
            attempt_id: call.binding.attempt_id.clone(),
            prepared_wire_sha256: call.wire_sha256.clone(),
            prepared_binding_sha256: call.binding_sha256.clone(),
            capture_ref: format!("capture:{}", call.binding.operation_id),
            captured_stream_sha256: sha256_hex(&reply.output),
            terminal: DirectTerminal::Complete,
            response_id: Some(format!("resp-{index}")),
            observed_provider: Some(PROVIDER.into()),
            observed_model: Some(MODEL.into()),
            output_sha256: sha256_hex(&reply.output),
            output: reply.output,
            evidence: self.evidence.clone(),
        })
    }
}

/// The C1 case as the owner's current material, rebound to the transition the
/// replacement minted. The owner's source moves, as only the commit-time fence
/// can see, once `current_fences` fenced commits have run.
struct Material {
    input: MaterialInput,
    current_fences: AtomicUsize,
    fenced_commits: AtomicUsize,
}

impl Material {
    fn new(current_fences: usize) -> Arc<Self> {
        Arc::new(Self {
            input: c1_input(),
            current_fences: AtomicUsize::new(current_fences),
            fenced_commits: AtomicUsize::new(0),
        })
    }

    fn fenced(&self) -> usize {
        self.fenced_commits.load(Ordering::SeqCst)
    }
}

impl CanonicalMaterialPort for Material {
    fn project(
        &self,
        subject: &SubjectId,
        transition: &TransitionId,
        capture_id: &str,
    ) -> Result<MaterialInput, MaterialPortError> {
        if subject != &self.input.subject || capture_id != self.input.capture_id {
            return Err(MaterialPortError::Invalid);
        }
        let mut input = self.input.clone();
        input.transition = transition.clone();
        Ok(input)
    }

    fn observe_dependencies(
        &self,
        expected: &[DependencyRevision],
    ) -> Result<Vec<DependencyObservation>, MaterialPortError> {
        Ok(expected
            .iter()
            .cloned()
            .map(DependencyObservation::Current)
            .collect())
    }

    fn with_current_fence(
        &self,
        _: &[DependencyRevision],
        commit: &mut dyn FnMut() -> Result<(), ServiceError>,
    ) -> Result<(), ServiceError> {
        if self.fenced_commits.load(Ordering::SeqCst) >= self.current_fences.load(Ordering::SeqCst)
        {
            return Err(ServiceError::Material(MaterialPortError::Conflict));
        }
        self.fenced_commits.fetch_add(1, Ordering::SeqCst);
        commit()
    }
}

/// A material port whose commits inside the owner fence can be counted.
trait Counted: CanonicalMaterialPort {
    fn fenced(&self) -> usize;
}

impl Counted for Material {
    fn fenced(&self) -> usize {
        Material::fenced(self)
    }
}

const QUESTION: &str = "q:direct-host";
const RECIPIENT: &str = "recipient:direct-host";
const SCOPE: &str = "scope:direct-host";

fn submit_task() -> SubmitTask {
    SubmitTask {
        command_id: "command:direct-host-submit".into(),
        task_id: TASK.into(),
        subject_id: SUBJECT.into(),
        kind: serde_json::from_value(json!("repository_analysis")).unwrap(),
        objective: "Analyze approved source".into(),
        engineering_question: "Who owns this?".into(),
        snapshot_ref: "snapshot:direct-host".into(),
        facts: Vec::new(),
    }
}

/// The task owner's own material realization, instrumented only by counting:
/// projection, dependency observation and the commit fence are `TaskStore`'s,
/// unchanged. Its database holds the served task again because the running
/// host holds its own owner's writer lock. Its question is opened through the
/// owner's operation entry with a pass-through grant fence, as no lifecycle
/// operation exists to open it; the answer that revises it is the owner's own.
struct OwnerMaterial {
    _dir: tempfile::TempDir,
    tasks: TaskStore,
    projections: AtomicUsize,
    fenced_commits: AtomicUsize,
    /// Where the owner answers its question, once, as the service uses it.
    answer_at: Mutex<Option<AnswerAt>>,
}

/// A fixed point in the service's use of the owner, after the owner call
/// returns and releases its lock.
#[derive(Clone, Copy, Debug, PartialEq)]
enum AnswerAt {
    /// After the dependency observation that precedes recording a verdict.
    Observation,
    /// After the first commit inside the owner fence: publication.
    FencedCommit,
}

impl OwnerMaterial {
    fn new() -> Arc<Self> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap().join("owner.sqlite");
        let tasks = TaskStore::open(&path, AUTHORITY.into()).unwrap();
        tasks.submit(&submit_task()).unwrap();
        let payload = QuestionOpenPayload {
            question_id: QUESTION.into(),
            recipient_ref: RECIPIENT.into(),
            scope_ref: SCOPE.into(),
            text: "May the analysis be published?".into(),
        };
        let entry = OperationEntry {
            operation_id: "operation:direct-host-question".into(),
            task_id: TASK.into(),
            subject_id: SUBJECT.into(),
            delivery_id: "delivery:direct-host-question".into(),
            attempt_id: "attempt:direct-host-question".into(),
            expected_task_revision: 1,
            expected_cursor_revision: 1,
            grant_ref: "grant:task-operation".into(),
            grant_revision: 1,
            payload_sha256: json_digest(&payload).unwrap(),
        };
        tasks
            .open_question(
                &entry,
                &payload,
                &mut |commit: &mut dyn FnMut() -> Result<(), TaskError>| commit(),
            )
            .unwrap();
        Arc::new(Self {
            _dir: dir,
            tasks,
            projections: AtomicUsize::new(0),
            fenced_commits: AtomicUsize::new(0),
            answer_at: Mutex::new(None),
        })
    }

    fn answer_at(&self, point: AnswerAt) {
        *self.answer_at.lock().unwrap() = Some(point);
    }

    fn answer_if(&self, point: AnswerAt) {
        let due = {
            let mut answer_at = self.answer_at.lock().unwrap();
            let due = *answer_at == Some(point);
            if due {
                *answer_at = None;
            }
            due
        };
        if due {
            self.answer();
        }
    }

    /// The owner's answer revises the question source: new revision, new digest.
    fn answer(&self) {
        let revision = self.tasks.observe(TASK).unwrap().unwrap().revision;
        self.tasks
            .answer(&AnswerQuestion {
                command_id: "command:direct-host-answer".into(),
                task_id: TASK.into(),
                question_id: QUESTION.into(),
                recipient_ref: RECIPIENT.into(),
                scope_ref: SCOPE.into(),
                answer: "Yes, after review.".into(),
                expected_revision: revision,
                authority_ref: AUTHORITY.into(),
            })
            .unwrap();
    }

    fn projected(&self) -> usize {
        self.projections.load(Ordering::SeqCst)
    }

    /// The basis the owner's current record projects for `transition`, read
    /// without counting as a service projection.
    fn basis_sha256(&self, transition: &str) -> String {
        let input = self
            .tasks
            .project(
                &SubjectId::parse(SUBJECT).unwrap(),
                &TransitionId::parse(transition).unwrap(),
                CAPTURE,
            )
            .unwrap();
        project_material(input).unwrap().sha256
    }

    /// A compile reply and a verdict for the owner's current material.
    fn replies(&self, status: FindingStatus) -> Vec<Reply> {
        let input = self
            .tasks
            .project(
                &SubjectId::parse(SUBJECT).unwrap(),
                &TransitionId::parse("transition-owner").unwrap(),
                CAPTURE,
            )
            .unwrap();
        vec![complete(&owner_candidate(&input)), complete(&c1_report(&input, status))]
    }
}

impl CanonicalMaterialPort for OwnerMaterial {
    fn project(
        &self,
        subject: &SubjectId,
        transition: &TransitionId,
        capture_id: &str,
    ) -> Result<MaterialInput, MaterialPortError> {
        self.projections.fetch_add(1, Ordering::SeqCst);
        self.tasks.project(subject, transition, capture_id)
    }

    fn observe_dependencies(
        &self,
        expected: &[DependencyRevision],
    ) -> Result<Vec<DependencyObservation>, MaterialPortError> {
        let observed = self.tasks.observe_dependencies(expected);
        self.answer_if(AnswerAt::Observation);
        observed
    }

    fn with_current_fence(
        &self,
        expected: &[DependencyRevision],
        commit: &mut dyn FnMut() -> Result<(), ServiceError>,
    ) -> Result<(), ServiceError> {
        self.tasks.with_current_fence(expected, commit)?;
        self.fenced_commits.fetch_add(1, Ordering::SeqCst);
        self.answer_if(AnswerAt::FencedCommit);
        Ok(())
    }
}

impl Counted for OwnerMaterial {
    fn fenced(&self) -> usize {
        self.fenced_commits.load(Ordering::SeqCst)
    }
}

/// A candidate attributing each current owner source, with the owner's
/// question as a pending interaction.
fn owner_candidate(input: &MaterialInput) -> ContinuationPlan {
    let source = |id: &str| input.sources.iter().find(|item| item.id == id).unwrap();
    let attributed = |id: &str| AttributedStatement {
        text: String::from_utf8(source(id).bytes.clone()).unwrap(),
        sources: vec![source(id).reference()],
    };
    ContinuationPlan {
        schema_version: 1,
        objective: attributed("objective"),
        work_position: attributed("engineering-question"),
        completed: vec![],
        commitments: vec![],
        decisions: vec![],
        uncertainties: vec![],
        interactions: vec![RetainedInteraction {
            question_id: QUESTION.into(),
            recipient: RECIPIENT.into(),
            scope: SCOPE.into(),
            status: InteractionStatus::Pending,
            answer_id: None,
            statement: attributed(&format!("question-{QUESTION}")),
        }],
        governing: vec![],
        canonical_refs: vec![attributed("snapshot-reference")],
        delivery_custody: vec![],
        next_action: AttributedStatement {
            text: "Ask before publishing the analysis.".into(),
            sources: vec![source("objective").reference()],
        },
    }
}

// ---- Installed-host fixture ----

const SUBJECT: &str = "subject-C1";
const CAPTURE: &str = "capture-C1";
const CONTEXT: &str = "context:direct-host";
const NEXT_CONTEXT: &str = "context:direct-host-next";
const SESSION: &str = "session:direct-host";
const AUTHORITY: &str = "operator:direct-host";
const ADMISSION_GRANT: &str = "grant:direct-admission";
const TASK: &str = "task:direct-host";
const TRANSPORT: &str = "transport:direct-host-synthetic";
const DOMAIN_TURN: &str = "invocation:direct-host-domain";

#[derive(Clone, Copy, PartialEq)]
enum Control {
    Direct,
    Native,
}

struct Host {
    _temp: tempfile::TempDir,
    config: PathBuf,
    host_socket: PathBuf,
    service_socket: PathBuf,
    database: PathBuf,
    registration: Registered,
    material: Arc<dyn Counted>,
    runtime: tokio::runtime::Runtime,
    serving: Option<tokio::task::JoinHandle<Result<(), HostError>>>,
}

fn private_dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

fn digest(bytes: &[u8]) -> String {
    CodecContract::BinaryArtifactV1
        .digest_binary(bytes)
        .unwrap()
        .hex()
}

impl Host {
    /// Writes a startup file for the selected control, registers the doubles
    /// for this directory, and creates the task the replacement names.
    fn new(
        control: Control,
        owner_admission: bool,
        port: Option<Arc<ScriptedPort>>,
        material: Arc<dyn Counted>,
    ) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().canonicalize().unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        let config = dir.join("host.json");
        let task_store = dir.join("tasks.sqlite");
        let store_root = dir.join("lifecycle-store");
        let host_socket = dir.join("host.sock");
        let service_socket = dir.join("service.sock");
        private_dir(&dir.join("native-root"));
        let binary = fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
        fs::write(dir.join("source.txt"), b"approved source\n").unwrap();
        let manifest = dir.join("source-manifest.json");
        fs::write(
            &manifest,
            serde_json::to_vec(&json!([{"path":"source.txt",
                "sha256":sha_file(&dir.join("source.txt")).unwrap()}]))
            .unwrap(),
        )
        .unwrap();
        let lock = dir.join("Cargo.lock");
        let toolchain = dir.join("rust-toolchain.toml");
        fs::write(&lock, b"pinned lock\n").unwrap();
        fs::write(&toolchain, b"pinned toolchain\n").unwrap();
        let grant = |reference: &str, context: &str, scope: &str| {
            json!({"grant_ref":reference,"context_generation":context,"scope":scope,
                "expires_wall_ms":i64::MAX / 2,"revision":1})
        };
        let mut grants = vec![
            grant("grant:enqueue", CONTEXT, "enqueue_input"),
            grant("grant:replace", CONTEXT, "request_replacement"),
            grant("grant:semantic", CONTEXT, "semantic_control"),
            grant("grant:native", CONTEXT, "native_turn"),
            // The task profile names its successor from the next enqueue grant.
            grant("grant:enqueue-next", NEXT_CONTEXT, "enqueue_input"),
        ];
        if owner_admission {
            grants.push(grant(ADMISSION_GRANT, CONTEXT, "semantic_direct_checkpoint_v1"));
        }
        let member = b"direct host source\n";
        let semantic = json!({
            "capture_id":CAPTURE,"compile_session_id":"session:direct-host-compile",
            "verify_session_id":"session:direct-host-verify",
            "successor_verify_session_id":"session:direct-host-successor",
            "compile_invocation_id":"invocation:direct-host-compile",
            "compile_attempt_id":"attempt:direct-host-compile",
            "candidate_verify_invocation_id":"invocation:direct-host-verify",
            "candidate_verify_attempt_id":"attempt:direct-host-verify",
            "successor_verify_invocation_id":"invocation:direct-host-successor",
            "successor_verify_attempt_id":"attempt:direct-host-successor",
            "challenge_id":"challenge:direct-host","challenge_text":"offline",
            "control_grant_ref":"grant:semantic","control_grant_revision":1,
            "successor_control_grant_ref":"grant:semantic:next",
            "successor_control_grant_revision":1
        });
        let native = json!({
            "executable":dir.join("never-executed"),"root":dir.join("native-root"),
            "simulator_base_url":"http://127.0.0.1:12345/v1",
            "session_id":SESSION,"invocation_id":"invocation:direct-host",
            "attempt_id":"attempt:direct-host","prompt":"unused offline prompt",
            "turn_grant_ref":"grant:native","turn_grant_revision":1,
            "snapshot_grant_ref":"grant:snapshot","snapshot_grant_revision":1,
            "snapshot":{"id":"snapshot:direct-host","source_revision":"source:direct-host",
                "members":[{"id":"source","sha256":digest(member),"bytes":member}]},
            "semantic":semantic
        });
        let lifecycle = json!({
            "profile":crate::PROFILE,"store_root":store_root,"socket_path":service_socket,
            "peer_socket_path":dir.join("unused-peer.sock"),
            "verifier_executable":null,"verifier_ledger_path":null,"proof_barrier_dir":null,
            "trusted_issuer":"issuer:direct-host",
            "principal":{"uid":current_uid().unwrap(),"principal_ref":"principal:direct-host",
                "producer_ref":"producer:direct-host"},
            "subject_id":SUBJECT,"context_generation":CONTEXT,
            "build_id":"build:direct-host","proof_run_id":"proof:direct-host",
            "grants":grants,"max_executions":1,"max_wait_ms":1000,"native":native
        });
        let mut startup = json!({
            "profile":crate::PROFILE,"task_store_path":task_store,"socket_path":host_socket,
            "approved_snapshot_ref":"snapshot:direct-host","answer_authority_ref":AUTHORITY,
            "enqueue_grant_ref":"grant:enqueue","replacement_grant_ref":"grant:replace",
            "executable_sha256":sha_file(&binary).unwrap(),"source_root":dir,
            "source_manifest_path":manifest,"source_sha256":sha_file(&manifest).unwrap(),
            "lock_path":lock,"lock_sha256":sha_file(&lock).unwrap(),
            "toolchain_path":toolchain,"toolchain_sha256":sha_file(&toolchain).unwrap(),
            "features":crate::compiled_features(),"provider_identity":"native-sim-proof-v1",
            "lifecycle":lifecycle
        });
        if control == Control::Direct {
            let journal = dir.join("direct-journal");
            private_dir(&journal);
            let release = dir.join("direct-release");
            private_dir(&release);
            // The credential store does not exist: only the registered
            // synthetic port can stand in for the adapter. The bounds are the
            // public route's fixed bounds, which validation requires.
            startup["control"] = json!({"kind":"direct",
                "credential_dir":dir.join("direct-credentials"),
                "credential_host_id":"host_direct","journal_dir":journal,
                "release_dir":release,"model":MODEL,
                "bounds":{"request_bytes":65536,"stream_bytes":1048576,"frame_bytes":131072,
                    "output_bytes":16384,"events":4096,"local_deadline_ms":30000}});
        }
        fs::write(&config, serde_json::to_vec(&startup).unwrap()).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();

        // The task the replacement names, created through the task owner.
        let tasks = TaskStore::open(&task_store, AUTHORITY.into()).unwrap();
        tasks.submit(&submit_task()).unwrap();
        drop(tasks);

        let registration = Arc::new(Registration {
            root: dir.clone(),
            port: port.map(|port| port as Arc<dyn DirectInferencePort>),
            material: material.clone(),
            port_lookups: AtomicUsize::new(0),
        });
        REGISTRY.lock().unwrap().push(registration.clone());
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();
        Self {
            _temp: temp,
            config,
            host_socket,
            service_socket,
            database: store_root.join("lifecycle.sqlite"),
            registration: Registered(registration),
            material,
            runtime,
            serving: None,
        }
    }

    /// Starts the installed entry and waits for its admission socket.
    fn start(&mut self) {
        let config = self.config.clone();
        let serving = self.runtime.spawn(async move { serve(&config).await });
        for _ in 0..500 {
            if UnixStream::connect(&self.host_socket).is_ok() {
                self.serving = Some(serving);
                return;
            }
            if serving.is_finished() {
                let result = self.runtime.block_on(serving).unwrap();
                panic!("host ended before serving: {result:?}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("host socket never opened");
    }

    fn request(&self, request: Value) -> Value {
        let mut stream = UnixStream::connect(&self.host_socket).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
        stream
            .write_all(&[serde_json::to_vec(&request).unwrap(), b"\n".to_vec()].concat())
            .unwrap();
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    fn task_revision(&self) -> u64 {
        let observed = self.request(json!({"op":"task.observe.v2","task_id":TASK}));
        observed["components"]["owner"]["revision"]["owner_revision"]
            .as_u64()
            .unwrap_or_else(|| panic!("task observation: {observed}"))
    }

    fn replace(&self, command_id: &str, revision: u64) -> Value {
        self.request(json!({"op":"task.request_replacement.v1","payload":{
            "task_id":TASK,"command_id":command_id,"expected_task_revision":revision,
            "authority_ref":AUTHORITY,"reason":"checkpoint the retained context"}}))
    }

    fn db(&self) -> rusqlite::Connection {
        let connection = rusqlite::Connection::open(&self.database).unwrap();
        connection.busy_timeout(Duration::from_secs(10)).unwrap();
        connection
    }

    fn count(&self, query: &str) -> i64 {
        self.db().query_row(query, [], |row| row.get(0)).unwrap()
    }

    fn transition(&self) -> Option<(String, String, Option<String>)> {
        use rusqlite::OptionalExtension as _;
        self.db()
            .query_row(
                "SELECT transition_id,stage,checkpoint_id FROM transitions WHERE subject_id=?1",
                [SUBJECT],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .unwrap()
    }

    fn capture_eligibility(&self) -> String {
        self.db()
            .query_row(
                "SELECT eligibility FROM semantic_captures WHERE capture_id=?1",
                [CAPTURE],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn capture_basis(&self) -> String {
        self.db()
            .query_row(
                "SELECT basis_sha256 FROM semantic_captures WHERE capture_id=?1",
                [CAPTURE],
                |row| row.get(0),
            )
            .unwrap()
    }

    /// Synthetic custody, inserted into the running store's database because
    /// the store's writer lock is held by the service and no native process
    /// exists: a settled domain-work turn on the actor's own reserved native
    /// session, qualified ingress for it, and its delivery segment already
    /// `yielded` with no transition. The store would only yield a segment to a
    /// transition that already exists; this order is a test fabrication.
    fn seed_custody(&self) {
        let db = self.db();
        let (incarnation, profile): (String, String) = db
            .query_row(
                "SELECT incarnation_id,profile_digest FROM native_sessions WHERE session_id=?1 AND role='domain'",
                [SESSION],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("the actor reserved its own native session");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let tx = db.unchecked_transaction().unwrap();
        tx.execute(
            "INSERT INTO inputs(input_id,subject_id,sequence,delivery_id,producer_ref,text,text_digest,grant_id,state) \
             VALUES('input:direct-host',?1,1,'delivery:direct-host','producer:direct-host',X'74657374','digest:direct-host','grant:native','entered')",
            [SUBJECT],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO effects(effect_id,subject_id,input_id,expected_revision,state) \
             VALUES('effect:direct-host',?1,'input:direct-host',0,'entered')",
            [SUBJECT],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO attempts(attempt_id,effect_id,incarnation_id,state,entered_wall_ms) \
             VALUES('queue-attempt:direct-host','effect:direct-host',?1,'entered',?2)",
            rusqlite::params![incarnation, now],
        )
        .unwrap();
        let final_item = json!({"type":"agentMessage","id":"item:final","text":"done"});
        let ingress = [
            (
                3,
                json!({"method":"turn/completed","params":{"threadId":"thread:direct-host",
                    "turn":{"id":"turn:direct-host","status":"completed"}}}),
                Some("turn/completed"),
            ),
            (
                6,
                json!({"method":"item/completed","params":{"threadId":"thread:direct-host",
                    "turnId":"turn:direct-host","item":final_item.clone()}}),
                Some("item/completed"),
            ),
            (
                7,
                json!({"id":7,"result":{"thread":{"id":"thread:direct-host","historyMode":"legacy",
                    "turns":[{"id":"turn:direct-host","status":"completed","itemsView":"full",
                        "items":[final_item]}]}}}),
                None,
            ),
        ];
        for (sequence, value, method) in ingress {
            let raw = serde_json::to_vec(&value).unwrap();
            let (thread, turn) = if method.is_some() {
                (Some("thread:direct-host"), Some("turn:direct-host"))
            } else {
                (None, None)
            };
            tx.execute(
                "INSERT INTO native_ingress(transport_session,sequence,raw_sha256,raw,decode_status,method,thread_id,turn_id,item_id,observed_wall_ms,session_id) \
                 VALUES(?1,?2,?3,?4,'qualified',?5,?6,?7,NULL,?8,?9)",
                rusqlite::params![TRANSPORT, sequence, digest(&raw), raw, method, thread, turn, now, SESSION],
            )
            .unwrap();
        }
        tx.execute(
            "INSERT INTO native_turns(invocation_id,session_id,attempt_id,grant_id,grant_revision,purpose,prompt_sha256,prompt,profile_digest,state,thread_id,turn_id,send_intent,terminal_status,final_text,final_sha256,history_status,history_transport_session,history_sequence,entered_wall_ms,settled_wall_ms) \
             VALUES(?1,?2,'native-attempt:direct-host-domain','grant:native',1,'domain_work',?3,X'74657374',?4,'completed_settled','thread:direct-host','turn:direct-host',1,'completed',X'646F6E65',?5,'observed_complete',?6,7,?7,?7)",
            rusqlite::params![DOMAIN_TURN, SESSION, digest(b"test"), profile, digest(b"done"), TRANSPORT, now],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO native_delivery_segments(invocation_id,queue_attempt_id,segment_no,predecessor_invocation_id,transition_id,state,settled_wall_ms) \
             VALUES(?1,'queue-attempt:direct-host',0,NULL,NULL,'yielded',?2)",
            rusqlite::params![DOMAIN_TURN, now],
        )
        .unwrap();
        tx.commit().unwrap();
    }

    /// Ends the host as an installed one ends: SIGTERM to the process, which
    /// both the host loop and the lifecycle service observe.
    fn stop(&mut self) -> Result<(), HostError> {
        let serving = self.serving.take().expect("host serving");
        let status = Command::new("kill")
            .args(["-TERM", &std::process::id().to_string()])
            .status()
            .unwrap();
        assert!(status.success());
        let result = self.runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(60), serving).await
        });
        result.expect("host stopped").expect("host task joined")
    }
}

/// What one run through the installed host observed.
struct Observed {
    reply: Value,
    stage: String,
    transition_id: String,
    checkpoint_id: Option<String>,
    admitted: i64,
    direct_results: i64,
    successor: i64,
    native_sessions: i64,
    port_lookups: usize,
    fenced: usize,
    eligibility: String,
}

/// Starts the host, seeds custody, requests the replacement through the host
/// socket and reads the durable outcome while the host still serves.
fn run(host: &mut Host) -> Observed {
    host.start();
    let revision = host.task_revision();
    host.seed_custody();
    let reply = host.replace("command:direct-host-replace", revision);
    let (transition_id, stage, checkpoint_id) = host.transition().expect("transition created");
    eprintln!(
        "observed: reply_status={} transition={transition_id} stage={stage} checkpoint={checkpoint_id:?}",
        reply["status"]
    );
    Observed {
        reply,
        admitted: host.count(&format!(
            "SELECT COUNT(*) FROM direct_semantic_results WHERE admission_grant_id='{ADMISSION_GRANT}'"
        )),
        direct_results: host.count("SELECT COUNT(*) FROM direct_semantic_results"),
        successor: host.count(&format!(
            "SELECT COUNT(*) FROM transitions WHERE transition_id='{transition_id}' \
             AND (successor_context IS NOT NULL OR successor_thread IS NOT NULL)"
        )),
        native_sessions: host.count("SELECT COUNT(*) FROM native_sessions"),
        port_lookups: host.registration.0.port_lookups.load(Ordering::SeqCst),
        fenced: host.material.fenced(),
        eligibility: host.capture_eligibility(),
        stage,
        transition_id,
        checkpoint_id,
    }
}

fn assert_stopped(host: &mut Host) {
    host.stop().expect("graceful host stop");
    assert!(!host.host_socket.exists(), "host socket removed");
    assert!(!host.service_socket.exists(), "service socket removed");
}

#[test]
fn installed_host_direct_selection_reaches_admitted_checkpoint_and_stops_gracefully() {
    let _serial = serial();
    let port = ScriptedPort::public_class(replies(FindingStatus::Satisfied));
    let mut host = Host::new(Control::Direct, true, Some(port.clone()), Material::new(usize::MAX));
    let observed = run(&mut host);

    assert_eq!(observed.reply["status"], "replacement_requested", "{}", observed.reply);
    // Startup asked for the configured Direct port twice: validation's
    // composability check and the composition the service received.
    assert_eq!(observed.port_lookups, 2);
    assert!(observed.transition_id.starts_with("transition:"));
    // The engine checkpointed and stopped there: no switch, no successor.
    assert_eq!(observed.stage, "checkpointed");
    assert!(observed.checkpoint_id.as_deref().is_some_and(|id| id.starts_with("checkpoint:")));
    assert_eq!(observed.successor, 0, "no native successor was authorized");
    // One compile and one separate verify exchange, through the selected port.
    let seen = port.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 2);
    assert!(seen[0].candidate_sha256.is_none() && seen[1].candidate_sha256.is_some());
    // Owner fences: publication of the direct result and its consumption.
    assert_eq!(observed.fenced, 2);
    assert!(observed.admitted > 0, "the owner grant admitted the direct result");
    assert_eq!(observed.native_sessions, 1, "only the domain session; no native control session");
    assert_eq!(observed.eligibility, "current");

    // The host and lifecycle service still serve after the checkpoint: the
    // replayed request reads the same accepted command back, and the task
    // owner still answers.
    let replay = host.replace("command:direct-host-replace", host.task_revision());
    assert_eq!(replay["status"], "replacement_requested", "{replay}");
    assert_eq!(host.transition().unwrap().1, "checkpointed");
    assert_eq!(port.calls(), 2, "serving on sent no further exchange");
    assert_stopped(&mut host);
    assert_eq!(port.calls(), 2);
    assert_eq!(host.count("SELECT COUNT(*) FROM transitions WHERE stage='checkpointed'"), 1);
}

#[test]
fn installed_host_direct_without_admission_does_not_checkpoint() {
    let _serial = serial();
    // (name, owner admission grant, port, publication fences, exchanges)
    let cases: Vec<(&str, bool, Arc<ScriptedPort>, usize, usize)> = vec![
        (
            "verifier_refused",
            true,
            ScriptedPort::public_class(replies(FindingStatus::Violated)),
            usize::MAX,
            2,
        ),
        (
            "transport_unknown",
            true,
            ScriptedPort::public_class(vec![Reply {
                output: Vec::new(),
                unknown: true,
            }]),
            0,
            1,
        ),
        (
            "no_owner_grant",
            false,
            ScriptedPort::public_class(replies(FindingStatus::Satisfied)),
            usize::MAX,
            2,
        ),
        (
            "synthetic_evidence_class",
            true,
            ScriptedPort::synthetic_class(replies(FindingStatus::Satisfied)),
            usize::MAX,
            2,
        ),
    ];
    for (name, admission, port, max_fences, exchanges) in cases {
        let mut host = Host::new(Control::Direct, admission, Some(port.clone()), Material::new(usize::MAX));
        let observed = run(&mut host);
        assert_eq!(observed.port_lookups, 2, "{name}");
        assert_eq!(observed.stage, "verifying", "{name}: {}", observed.reply);
        assert!(observed.checkpoint_id.is_none(), "{name}");
        assert_eq!(observed.admitted, 0, "{name}");
        assert_eq!(observed.successor, 0, "{name}");
        assert_eq!(port.calls(), exchanges, "{name}");
        assert!(observed.fenced <= max_fences.min(1), "{name}: {}", observed.fenced);
        assert_eq!(observed.native_sessions, 1, "{name}");
        assert_stopped(&mut host);
        assert_eq!(port.calls(), exchanges, "{name}: no retransmission");
    }
}

#[test]
fn installed_host_direct_fences_owner_change_before_publication_and_before_consumption() {
    let _serial = serial();
    // The owner's source changes before publication: the verdict is recorded
    // outside the fence, but the fence refuses publication, so nothing is
    // admitted or checkpointed and the capture is stale.
    let port = ScriptedPort::public_class(replies(FindingStatus::Satisfied));
    let mut host = Host::new(Control::Direct, true, Some(port.clone()), Material::new(0));
    let before = run(&mut host);
    assert_eq!(before.stage, "verifying");
    assert!(before.checkpoint_id.is_none());
    assert_eq!(before.direct_results, 1, "the recorded verdict");
    assert_eq!(before.admitted, 0);
    assert_eq!(before.fenced, 0);
    assert_eq!(before.eligibility, "stale");
    assert_eq!(port.calls(), 2);
    assert_stopped(&mut host);

    // The source changes after publication, before the engine consumes the
    // admitted result: the checkpoint is not consumed and the capture is stale.
    let port = ScriptedPort::public_class(replies(FindingStatus::Satisfied));
    let mut host = Host::new(Control::Direct, true, Some(port.clone()), Material::new(1));
    let after = run(&mut host);
    assert_eq!(after.stage, "verifying");
    assert!(after.checkpoint_id.is_none());
    assert!(after.admitted > 0, "published and admitted before the change");
    assert_eq!(after.fenced, 1, "only publication committed inside the fence");
    assert_eq!(after.eligibility, "stale");
    assert_eq!(port.calls(), 2);
    assert_stopped(&mut host);
}

#[test]
fn installed_host_native_selection_never_composes_or_reaches_direct() {
    let _serial = serial();
    // A registered port exists for this directory; Native selection must not
    // ask for it, and nothing Direct may be produced.
    let port = ScriptedPort::public_class(replies(FindingStatus::Satisfied));
    let mut host = Host::new(Control::Native, true, Some(port.clone()), Material::new(usize::MAX));
    let observed = run(&mut host);
    assert_eq!(observed.port_lookups, 0);
    assert_eq!(port.calls(), 0);
    assert_ne!(observed.stage, "checkpointed", "{}", observed.reply);
    assert!(observed.checkpoint_id.is_none());
    assert_eq!(observed.direct_results, 0);
    assert_eq!(observed.admitted, 0);
    assert_eq!(observed.fenced, 0);
    assert_eq!(observed.successor, 0);
    assert_stopped(&mut host);
}

/// Waits (bounded) until the transition reaches `stage`.
fn wait_for_stage(host: &Host, stage: &str) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        if host.transition().is_some_and(|(_, current, _)| current == stage) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// Waits (bounded) until the port has seen `calls` submissions.
fn wait_for_calls(port: &ScriptedPort, calls: usize) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        if port.calls() >= calls {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// Pending release through the installed host. Evidence class: synthetic
/// (scripted port and release owner; SQL-seeded custody as in the other host
/// tests). The release owner's arrival is observed only by the service's
/// guarded re-drive: no host request is made after the replacement.
#[test]
fn installed_host_direct_pending_release_is_redriven_without_resend() {
    let _serial = serial();
    let port = ScriptedPort::public_class(replies(FindingStatus::Satisfied));
    port.hold_releases();
    let mut host = Host::new(Control::Direct, true, Some(port.clone()), Material::new(usize::MAX));
    let observed = run(&mut host);
    assert_eq!(observed.reply["status"], "replacement_requested", "{}", observed.reply);
    // Missing release: no store attempt, no exchange, no checkpoint.
    assert_eq!(observed.stage, "verifying");
    assert_eq!(port.calls(), 0);
    assert_eq!(host.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 0);
    // The guarded re-drive keeps offering the same compile call and nothing else.
    std::thread::sleep(Duration::from_millis(4500));
    assert_eq!(port.calls(), 0);
    assert_eq!(host.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 0);
    let offered = port.offered.lock().unwrap().clone();
    assert!(offered.len() >= 2, "re-driven while pending: {offered:?}");
    assert!(offered.iter().all(|role| *role == DirectRole::Compile));

    // The compile release arrives: one compile exchange; the verify call is
    // then prepared from the settled candidate and waits for its own release.
    port.release(DirectRole::Compile);
    assert!(wait_for_calls(&port, 1), "compile sent after its release");
    std::thread::sleep(Duration::from_millis(4500));
    assert_eq!(port.calls(), 1, "verify is not sent without its release");
    assert_eq!(host.transition().unwrap().1, "verifying");
    assert_eq!(host.count("SELECT COUNT(*) FROM direct_semantic_attempts WHERE role='verify'"), 0);
    assert!(port.offered.lock().unwrap().contains(&DirectRole::Verify));

    port.release(DirectRole::Verify);
    assert!(wait_for_stage(&host, "checkpointed"), "re-drive reached the checkpoint");
    let seen = port.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].binding.role, DirectRole::Compile);
    assert_eq!(seen[1].binding.role, DirectRole::Verify);
    assert_eq!(host.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 2);
    // Serving on after the checkpoint sends nothing further.
    std::thread::sleep(Duration::from_millis(2500));
    assert_eq!(port.calls(), 2);
    assert_stopped(&mut host);
    assert_eq!(port.calls(), 2);
}

#[test]
fn installed_host_direct_withheld_send_is_withdrawn_and_redriven_once() {
    let _serial = serial();
    let port = ScriptedPort::public_class(replies(FindingStatus::Satisfied));
    // The compile release is refused before any claim at its first send.
    port.withhold_once(DirectRole::Compile);
    let mut host = Host::new(Control::Direct, true, Some(port.clone()), Material::new(usize::MAX));
    let observed = run(&mut host);
    assert_eq!(observed.reply["status"], "replacement_requested", "{}", observed.reply);
    assert!(wait_for_stage(&host, "checkpointed"), "the served host re-drove after withdrawal");
    let withheld = port.withheld.lock().unwrap().clone();
    assert_eq!(withheld.len(), 1);
    // The withdrawn intent left no terminal NotSent: the same prepared compile
    // call was offered again and sent once, then the verify call once.
    let seen = port.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].binding.role, DirectRole::Compile);
    assert_eq!(seen[0].binding_sha256, withheld[0]);
    assert_eq!(seen[1].binding.role, DirectRole::Verify);
    let offered = port.offered.lock().unwrap().clone();
    assert!(offered.iter().filter(|role| **role == DirectRole::Compile).count() >= 2, "{offered:?}");
    assert_eq!(host.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 2);
    assert_eq!(host.count("SELECT COUNT(*) FROM transitions WHERE stage='checkpointed'"), 1);
    std::thread::sleep(Duration::from_millis(2500));
    assert_eq!(port.calls(), 2);
    assert_stopped(&mut host);
    assert_eq!(port.calls(), 2);
}

/// Waits (bounded) until `condition` holds.
fn wait_until(condition: impl Fn() -> bool) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// R03 with the task owner's material port. The compile release is withheld
/// at its first send, and inside that withheld submit - before it returns, so
/// before the guarded re-drive can exist - the owner either stays unchanged or
/// answers its question. The re-drive is observed by its projection of the
/// owner, and its end by a lifecycle snapshot, which the serial actor answers
/// only after the drive in progress returns.
///
/// Unchanged, the re-drive offers the same prepared compile call again and the
/// transition checkpoints. Answered, the re-drive projects the owner's updated
/// record, the capture barrier refuses the changed basis before Direct entry,
/// and nothing is offered, attempted or sent; the captured basis is neither
/// rebased nor reset, and no NotSent is recorded.
#[test]
fn installed_host_direct_owner_update_before_redrive_admits_no_fresh_attempt() {
    let _serial = serial();
    for answered in [false, true] {
        let owner = OwnerMaterial::new();
        let port = ScriptedPort::public_class(owner.replies(FindingStatus::Satisfied));
        port.withhold_once(DirectRole::Compile);
        // The service's projections when the withheld submit ran.
        let at_withhold = Arc::new(AtomicUsize::new(usize::MAX));
        {
            let owner = owner.clone();
            let at_withhold = at_withhold.clone();
            port.during_next_submit(move || {
                if answered {
                    owner.answer();
                }
                at_withhold.store(owner.projected(), Ordering::SeqCst);
            });
        }
        let mut host = Host::new(Control::Direct, true, Some(port.clone()), owner.clone());
        let observed = run(&mut host);
        assert_eq!(observed.reply["status"], "replacement_requested", "{answered}: {}", observed.reply);
        assert!(
            wait_until(|| {
                let at = at_withhold.load(Ordering::SeqCst);
                at != usize::MAX && owner.projected() > at
            }),
            "{answered}: the re-drive projected the owner after the withheld send"
        );
        // Answered by the actor only after the re-drive in progress returned.
        host.task_revision();
        let withheld = port.withheld.lock().unwrap().clone();
        assert_eq!(withheld.len(), 1, "{answered}");

        if !answered {
            assert!(wait_for_stage(&host, "checkpointed"), "the unchanged owner re-drove to the checkpoint");
            let seen = port.seen.lock().unwrap().clone();
            assert_eq!(seen.len(), 2);
            assert_eq!(seen[0].binding.role, DirectRole::Compile);
            assert_eq!(seen[0].binding_sha256, withheld[0]);
            assert_eq!(seen[1].binding.role, DirectRole::Verify);
            assert_eq!(host.material.fenced(), 2, "publication and consumption inside the owner fence");
            assert_eq!(host.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 2);
            assert_eq!(host.capture_eligibility(), "current");
            assert_stopped(&mut host);
            assert_eq!(port.calls(), 2);
            continue;
        }

        // The re-drive saw the owner's new record; the capture still holds the
        // basis projected before the answer.
        let transition = host.transition().unwrap().0;
        let captured = host.capture_basis();
        assert_ne!(captured, owner.basis_sha256(&transition), "the owner's material changed");
        assert_eq!(host.capture_eligibility(), "current", "the capture is not rebased or reset");
        // No offer, intent, attempt, NotSent, send, verdict or checkpoint.
        assert_eq!(*port.offered.lock().unwrap(), vec![DirectRole::Compile]);
        assert_eq!(port.calls(), 0);
        assert_eq!(host.count("SELECT COUNT(*) FROM direct_semantic_attempts"), 0);
        assert_eq!(host.count("SELECT COUNT(*) FROM direct_semantic_results"), 0);
        let (_, stage, checkpoint) = host.transition().unwrap();
        assert_eq!(stage, "verifying");
        assert!(checkpoint.is_none());
        assert_eq!(host.material.fenced(), 0);
        assert_stopped(&mut host);
        assert_eq!(port.calls(), 0);
        assert_eq!(host.capture_basis(), captured);
    }
}

/// The same owner change after projection, kept apart from the re-drive case
/// and placed at three fixed points. No owner lock spans a send, so compile and
/// verify both go out on the projected basis each time; the owner refuses what
/// follows. Answered during the compile send, the observation before the
/// verdict sees it and nothing is recorded. Answered after that observation,
/// the owner's commit fence refuses publication of the recorded verdict.
/// Answered after publication, the fence refuses its consumption. In every
/// case nothing checkpoints and the capture is retired as stale.
#[test]
fn installed_host_direct_owner_update_after_projection_never_checkpoints() {
    let _serial = serial();
    // (point, recorded verdicts, admitted, commits inside the owner fence)
    let cases = [(None, 0, false, 0), (Some(AnswerAt::Observation), 1, false, 0),
        (Some(AnswerAt::FencedCommit), 1, true, 1)];
    for (point, results, admitted, fenced) in cases {
        let owner = OwnerMaterial::new();
        let port = ScriptedPort::public_class(owner.replies(FindingStatus::Satisfied));
        match point {
            Some(point) => owner.answer_at(point),
            None => {
                let owner = owner.clone();
                port.during_next_submit(move || owner.answer());
            }
        }
        let mut host = Host::new(Control::Direct, true, Some(port.clone()), owner.clone());
        let observed = run(&mut host);
        assert_eq!(observed.reply["status"], "replacement_requested", "{point:?}: {}", observed.reply);
        assert!(wait_for_calls(&port, 2), "{point:?}: compile and verify sent on the projected basis");
        assert!(
            wait_until(|| host.capture_eligibility() == "stale"),
            "{point:?}: the owner retired the capture"
        );
        // Answered by the actor only after the drive in progress returned.
        host.task_revision();
        assert_eq!(port.calls(), 2, "{point:?}");
        assert_eq!(host.count("SELECT COUNT(*) FROM direct_semantic_results"), results, "{point:?}");
        let admissions = host.count(&format!(
            "SELECT COUNT(*) FROM direct_semantic_results WHERE admission_grant_id='{ADMISSION_GRANT}'"
        ));
        assert_eq!(admissions > 0, admitted, "{point:?}: {admissions}");
        assert_eq!(host.material.fenced(), fenced, "{point:?}");
        let (transition, stage, checkpoint) = host.transition().unwrap();
        assert_eq!(stage, "verifying", "{point:?}");
        assert!(checkpoint.is_none(), "{point:?}");
        assert_ne!(host.capture_basis(), owner.basis_sha256(&transition), "{point:?}");
        assert_stopped(&mut host);
        assert_eq!(port.calls(), 2, "{point:?}");
    }
}
