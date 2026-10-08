#![recursion_limit = "256"]

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use lifecycle_core::{
    ClockSample, EvidenceId, GrantId, Revision, SubjectId, TransitionFact, TransitionId,
    WaitBudgetMs, WallTimeMs,
};
use lifecycle_semantic::{
    AttributedFinding, AttributedStatement, ContinuationPlan, CriticalFinding,
    DependencyObservation, DependencyRevision, ErrorKind, FindingStatus, InteractionStatus,
    MaterialInput, ObligationKind, RequiredObligation, RetainedDecision, RetainedInteraction,
    SemanticPolicy, SourceAvailability, SourceKind, SourceMaterial, SourceReference, SourceTrust,
    VerifierReport, project_material, sha256_hex,
};
use lifecycle_service::{CanonicalMaterialPort, MaterialPortError, ServiceError};
use lifecycle_store::SqliteLifecycleStore;
use lifecycle_test_support::semantic_owners::{FenceError, SyntheticOwnerJournal};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use tempfile::tempdir;
use work_engine_types::CodecContract;

/// Exact local Responses frames for the pinned native executable. The fixture
/// has no upstream transport, credentials or dynamic tool responses.
struct LocalSimulator {
    address: SocketAddr,
    captured: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<Result<(), String>>>,
}

impl LocalSimulator {
    fn start(outputs: Vec<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let observed = captured.clone();
        let stopping = stop.clone();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(60);
            while !stopping.load(Ordering::SeqCst) && Instant::now() < deadline {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let (body, authorization) =
                            read_native_request(stream.try_clone().map_err(|e| e.to_string())?)?;
                        if authorization {
                            return Err("native simulator received authorization header".into());
                        }
                        let count = {
                            let mut requests = observed.lock().map_err(|_| "capture poisoned")?;
                            requests.push(body);
                            requests.len()
                        };
                        let text = outputs
                            .get(count - 1)
                            .ok_or("unexpected extra native request")?;
                        write_native_response(stream, count, text)?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => return Err(error.to_string()),
                }
            }
            Ok(())
        });
        Self {
            address,
            captured,
            stop,
            worker: Some(worker),
        }
    }

    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.address.port())
    }

    fn requests(&self) -> Vec<Value> {
        self.captured.lock().unwrap().clone()
    }

    fn finish(mut self) -> Vec<Value> {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap().unwrap();
        self.requests()
    }
}

impl Drop for LocalSimulator {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn read_native_request(stream: TcpStream) -> Result<(Value, bool), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    if !line.starts_with("POST /v1/responses ") {
        return Err(format!("unexpected native route: {}", line.trim()));
    }
    let mut length = None;
    let mut authorization = false;
    loop {
        line.clear();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if line == "\r\n" {
            break;
        }
        if line.is_empty() {
            return Err("native header EOF".into());
        }
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("content-length:") {
            length = Some(value.trim().parse::<usize>().map_err(|e| e.to_string())?);
        }
        authorization |= lower.starts_with("authorization:");
    }
    let length = length.ok_or("native content length missing")?;
    if !(1..=2_000_000).contains(&length) {
        return Err("native request outside bounded frame".into());
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    let body = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok((body, authorization))
}

fn write_native_response(mut stream: TcpStream, count: usize, text: &str) -> Result<(), String> {
    let item = json!({
        "type":"message","id":format!("s6-message-{count}"),"role":"assistant",
        "status":"completed","content":[{"type":"output_text","text":text,"annotations":[]}]
    });
    let response = json!({
        "id":format!("s6-response-{count}"),"object":"response","created_at":1,
        "status":"completed","output":[item.clone()],
        "usage":{"input_tokens":3,"output_tokens":2,"total_tokens":5}
    });
    let events = [
        json!({"type":"response.created","response":{"id":format!("s6-response-{count}"),"status":"in_progress","output":[]}}),
        json!({"type":"response.output_item.done","output_index":0,"item":item}),
        json!({"type":"response.completed","response":response}),
    ];
    let wire = events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect::<String>();
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{wire}", wire.len()).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())
}

struct SyntheticPort {
    journal: SyntheticOwnerJournal,
    project_override: Option<MaterialInput>,
}

impl CanonicalMaterialPort for SyntheticPort {
    fn project(
        &self,
        subject: &SubjectId,
        transition: &TransitionId,
        capture_id: &str,
    ) -> Result<MaterialInput, MaterialPortError> {
        let input = match &self.project_override {
            Some(input) => input.clone(),
            None => self
                .journal
                .snapshot_input()
                .map_err(|_| MaterialPortError::Unavailable)?,
        };
        if &input.subject != subject || capture_id.is_empty() {
            return Err(MaterialPortError::Invalid);
        }
        let mut projected = input;
        projected.transition = transition.clone();
        projected.capture_id = capture_id.into();
        Ok(projected)
    }

    fn observe_dependencies(
        &self,
        _expected: &[DependencyRevision],
    ) -> Result<Vec<DependencyObservation>, MaterialPortError> {
        self.journal
            .observe_dependencies()
            .map_err(|_| MaterialPortError::Unavailable)
    }

    fn with_current_fence(
        &self,
        expected: &[DependencyRevision],
        commit: &mut dyn FnMut() -> Result<(), ServiceError>,
    ) -> Result<(), ServiceError> {
        self.journal
            .with_current_fence(expected, commit)
            .map_err(|error| match error {
                FenceError::OwnerUnavailable => {
                    ServiceError::Material(MaterialPortError::Unavailable)
                }
                FenceError::InvalidMaterial(_) => {
                    ServiceError::Material(MaterialPortError::Invalid)
                }
                FenceError::Stale(_) => ServiceError::Material(MaterialPortError::Conflict),
                FenceError::Commit(error) => error,
            })
    }
}

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/s67-preparation/semantic-corpus.v1.json"
    ))
    .unwrap()
}

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
    } else if id.starts_with("reference-") {
        (SourceKind::CanonicalRecord, SourceTrust::HostCanonical)
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

fn c1_input(fixture: &Value) -> MaterialInput {
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
    let obligations = case["obligations"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, row)| RequiredObligation {
            id: format!("C1-O{}", index + 1),
            kind: ObligationKind::Commitment,
            verifier_scope: [
                "Current decision and supersession among cited decision revisions",
                "Human interaction and governing publication scope",
                "Task commitment, uncertainty, and canonical reference custody",
                "Queued delivery effect and its current disposition",
            ][index]
                .into(),
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
                explanation: "Controlled simulator finding against the exact C1 material".into(),
            })
            .collect(),
        critical_errors: vec![],
    }
}

enum ControlledVariant {
    Complete,
    OmitCommitment,
    EchoSuccessor,
}

fn c1_native_outputs(input: &MaterialInput, variant: ControlledVariant) -> Vec<String> {
    let mut candidate = c1_candidate(input);
    if matches!(variant, ControlledVariant::OmitCommitment) {
        candidate.commitments.clear();
        candidate.work_position = candidate.objective.clone();
    }
    let candidate_raw = serde_json::to_string(&candidate).unwrap();
    let mut candidate_report = c1_report(input, FindingStatus::Satisfied);
    if matches!(variant, ControlledVariant::OmitCommitment) {
        candidate_report.critical_errors.push(CriticalFinding {
            kind: ErrorKind::OmittedMaterialCommitment,
            sources: input
                .obligations
                .iter()
                .flat_map(|obligation| obligation.sources.clone())
                .filter(|source| source.id == "commitment-k1")
                .collect(),
            explanation: "K-1 unfinished commitment omitted from candidate".into(),
        });
    }
    let candidate_report_raw = serde_json::to_string(&candidate_report).unwrap();
    if matches!(variant, ControlledVariant::OmitCommitment) {
        return vec![candidate_raw, candidate_report_raw];
    }
    let successor = if matches!(variant, ControlledVariant::EchoSuccessor) {
        sha256_hex(candidate_raw.as_bytes())
    } else {
        "D-2 is current; D-1 is historical. Q-1 to O-1 for publication remains pending. K-1 and U-1 are unfinished. R-1 revision 5 is unverified. I-2 stays queued; do not publish or replay work. Prepare the internal draft after source verification.".into()
    };
    let successor_report = c1_report(
        input,
        if matches!(variant, ControlledVariant::EchoSuccessor) {
            FindingStatus::Indeterminate
        } else {
            FindingStatus::Satisfied
        },
    );
    vec![
        candidate_raw,
        candidate_report_raw,
        successor,
        serde_json::to_string(&successor_report).unwrap(),
    ]
}

fn wait_path(path: &Path) {
    let started = Instant::now();
    while !path.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "missing {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_semantic_path(path: &Path, db: &Path, deadline: Duration) {
    let started = Instant::now();
    while !path.exists() {
        if started.elapsed() >= deadline {
            panic!("missing {}: {}", path.display(), diagnose_semantic_db(db));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_socket(path: &Path) {
    let started = Instant::now();
    loop {
        if UnixStream::connect(path).is_ok() {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "socket unavailable: {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn native_binary() -> PathBuf {
    let path = PathBuf::from(
        "/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex",
    );
    assert!(path.is_file(), "pinned S5 native binary is absent");
    path
}

fn service_config(root: &Path, base_url: &str, generation: u8) -> Value {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let grant = |id: &str, context: &str, scope: &str| {
        json!({
            "grant_ref":id,"context_generation":context,"scope":scope,
            "expires_wall_ms":now+120_000,"revision":1
        })
    };
    let grants = vec![
        grant("grant:s6-native", "context-C1", "native_turn"),
        grant("grant:s6-snapshot", "context-C1", "snapshot_read"),
        grant("grant:s6-control", "context-C1", "semantic_control"),
        grant("grant:s6-replace", "context-C1", "request_replacement"),
        grant(
            "grant:s6-successor-input",
            "context-C1-next",
            "enqueue_input",
        ),
        grant("grant:s6-next-native", "context-C1-next", "native_turn"),
        grant("grant:s6-next-snapshot", "context-C1-next", "snapshot_read"),
        grant(
            "grant:s6-next-control",
            "context-C1-next",
            "semantic_control",
        ),
    ];
    let suffix = format!("g{generation}");
    let native_root = root.join(format!("native-{suffix}"));
    fs::create_dir(&native_root).unwrap();
    fs::set_permissions(&native_root, fs::Permissions::from_mode(0o700)).unwrap();
    let context = if generation == 1 {
        "context-C1"
    } else {
        "context-C1-next"
    };
    let native_grant = if generation == 1 {
        "grant:s6-native"
    } else {
        "grant:s6-next-native"
    };
    let snapshot_grant = if generation == 1 {
        "grant:s6-snapshot"
    } else {
        "grant:s6-next-snapshot"
    };
    let control_grant = if generation == 1 {
        "grant:s6-control"
    } else {
        "grant:s6-next-control"
    };
    json!({
        "profile":"native_simulated","store_root":root.join("store"),
        "socket_path":root.join(format!("service-{suffix}.sock")),
        "peer_socket_path":root.join("unused-peer.sock"),
        "verifier_executable":null,"verifier_ledger_path":null,
        "proof_barrier_dir":root.join("barrier"),
        "trusted_issuer":"issuer-C1",
        "principal":{"uid":fs::metadata(root).unwrap().uid(),"principal_ref":"principal-C1","producer_ref":"producer-C1"},
        "subject_id":"subject-C1","context_generation":context,"build_id":"build-C1","proof_run_id":"proof-C1",
        "grants":grants,"max_executions":1,"max_wait_ms":1000,
        "native":{
            "executable":native_binary(),"root":native_root,
            "simulator_base_url":base_url,
            "session_id":format!("session:s6:{suffix}"),"invocation_id":format!("invocation:s6:domain:{suffix}"),
            "attempt_id":format!("attempt:s6:domain:{suffix}"),
            "prompt":"Preserve the controlled T-1 transition material.",
            "turn_grant_ref":native_grant,"turn_grant_revision":1,
            "snapshot_grant_ref":snapshot_grant,"snapshot_grant_revision":1,
            "same_native_session_required":false,
            "snapshot":{"id":format!("snapshot:s6:{suffix}"),"source_revision":format!("synthetic:s6:{suffix}"),"members":[{"id":"marker","sha256":sha256_hex(b"S6"),"bytes":[83,54]}]},
            "semantic":{
                "capture_id":"capture-C1",
                "compile_session_id":format!("session:s6:compile:{suffix}"),
                "verify_session_id":format!("session:s6:verify:{suffix}"),
                "successor_verify_session_id":format!("session:s6:successor-verify:{suffix}"),
                "compile_invocation_id":format!("invocation:s6:compile:{suffix}"),
                "compile_attempt_id":format!("attempt:s6:compile:{suffix}"),
                "candidate_verify_invocation_id":format!("invocation:s6:candidate-verify:{suffix}"),
                "candidate_verify_attempt_id":format!("attempt:s6:candidate-verify:{suffix}"),
                "successor_verify_invocation_id":format!("invocation:s6:successor-verify:{suffix}"),
                "successor_verify_attempt_id":format!("attempt:s6:successor-verify:{suffix}"),
                "challenge_id":format!("challenge:s6:{suffix}"),
                "challenge_text":c1_challenge_text(),
                "control_grant_ref":control_grant,"control_grant_revision":1
            }
        }
    })
}

struct ServiceChild(Child);

impl ServiceChild {
    fn start(config_path: &Path, update_dir: &Path) -> Self {
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "semantic_service_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("S6_CHILD_CONFIG", config_path)
            .env("S6_CHILD_UPDATE_DIR", update_dir)
            .env(
                "TMPDIR",
                "/home/bline/code/.work-engine-tmp/s4-20261007/tmp",
            )
            .env(
                "CARGO_TARGET_DIR",
                "/home/bline/code/.work-engine-tmp/s4-20261007/target",
            )
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        Self(child)
    }

    #[cfg(feature = "controlled-proof")]
    fn start_semantic_disabled(config_path: &Path) -> Self {
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "semantic_disabled_restart_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("S6_DISABLED_CONFIG", config_path)
            .env(
                "TMPDIR",
                "/home/bline/code/.work-engine-tmp/s4-20261007/tmp",
            )
            .env(
                "CARGO_TARGET_DIR",
                "/home/bline/code/.work-engine-tmp/s4-20261007/target",
            )
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        Self(child)
    }

    fn terminate(&mut self) {
        let result = Command::new("kill")
            .args(["-TERM", &self.0.id().to_string()])
            .env(
                "TMPDIR",
                "/home/bline/code/.work-engine-tmp/s4-20261007/tmp",
            )
            .env(
                "CARGO_TARGET_DIR",
                "/home/bline/code/.work-engine-tmp/s4-20261007/target",
            )
            .status()
            .unwrap();
        assert!(result.success());
        let started = Instant::now();
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success(), "semantic service child failed: {status}");
                return;
            }
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "semantic service child did not stop"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn crash(&mut self) {
        self.0.kill().unwrap();
        self.0.wait().unwrap();
    }

    fn wait_success(&mut self, deadline: Duration) {
        let started = Instant::now();
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success(), "semantic service child failed: {status}");
                return;
            }
            assert!(
                started.elapsed() < deadline,
                "semantic service child did not exit"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for ServiceChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[test]
fn semantic_service_child() {
    let Ok(path) = std::env::var("S6_CHILD_CONFIG") else {
        return;
    };
    let config: lifecycle_service::ServiceConfig =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let mut input = c1_input(&fixture());
    if let Ok(dir) = std::env::var("S6_CHILD_UPDATE_DIR") {
        let dir = PathBuf::from(dir);
        if dir.join("unknown-schema").exists() {
            input.policy.schema_version = 2;
        }
        if dir.join("unknown-policy").exists() {
            input.policy.id = "unknown-policy-v1".into();
        }
        let availability = if dir.join("material-unavailable").exists() {
            Some(SourceAvailability::Unavailable)
        } else if dir.join("material-truncated").exists() {
            Some(SourceAvailability::Truncated)
        } else {
            None
        };
        if let Some(availability) = availability {
            let source = input
                .sources
                .iter_mut()
                .find(|source| source.id == "reference-r1")
                .unwrap();
            source.availability = availability;
            source.bytes.clear();
        }
    }
    let unsupported =
        input.policy.schema_version != 1 || input.policy.id != "s6-strict-synthetic-v1";
    let project_override = unsupported.then_some(input.clone());
    // The synthetic owner retains a valid journal for currentness; the
    // service receives the exact invalid projection through its port.
    let journal = SyntheticOwnerJournal::new(if unsupported {
        c1_input(&fixture())
    } else {
        input
    })
    .unwrap();
    if let Ok(dir) = std::env::var("S6_CHILD_UPDATE_DIR") {
        let dir = PathBuf::from(dir);
        if dir.join("seed-q1-r8").exists() {
            journal
                .update_source(source_from_fixture(&fixture(), "question-q1-r8", true))
                .unwrap();
        }
    }
    if let Ok(dir) = std::env::var("S6_CHILD_UPDATE_DIR") {
        let journal = journal.clone();
        thread::spawn(move || {
            let dir = PathBuf::from(dir);
            let update = dir.join("update-q1");
            let deadline = Instant::now() + Duration::from_secs(90);
            while Instant::now() < deadline {
                if update.exists() {
                    let source = source_from_fixture(&fixture(), "question-q1-r8", true);
                    journal.update_source(source).unwrap();
                    fs::write(dir.join("updated-q1"), b"updated\n").unwrap();
                    return;
                }
                thread::sleep(Duration::from_millis(5));
            }
        });
    }
    let port = Arc::new(SyntheticPort {
        journal,
        project_override,
    });
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = runtime.block_on(lifecycle_service::run_with_material_port(config, port));
    let expected_reject = std::env::var("S6_CHILD_UPDATE_DIR")
        .ok()
        .is_some_and(|dir| PathBuf::from(dir).join("expect-startup-reject").exists());
    if expected_reject {
        assert!(matches!(result, Err(ServiceError::Configuration)));
    } else {
        result.unwrap();
    }
}

#[cfg(feature = "controlled-proof")]
#[test]
fn semantic_disabled_restart_child() {
    let Ok(path) = std::env::var("S6_DISABLED_CONFIG") else {
        return;
    };
    let config: lifecycle_service::ServiceConfig =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(config.profile, "controlled");
    assert!(config.native.is_none());
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    // Rejection during startup or a fenced, idle service is acceptable. The
    // parent reads the durable transition after this exact configuration.
    let _ = runtime.block_on(lifecycle_service::run(config));
}

fn subject_revision(db: &Path) -> u64 {
    let connection = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    connection
        .query_row(
            "SELECT revision FROM subjects WHERE subject_id='subject-C1'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap() as u64
}

fn poll_domain_settled(db: &Path, deadline: Duration) {
    let start = Instant::now();
    loop {
        if db.exists()
            && let Ok(connection) =
                Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            let state: rusqlite::Result<String> = connection.query_row(
                "SELECT state FROM native_turns WHERE invocation_id='invocation:s6:domain:g1'",
                [],
                |row| row.get(0),
            );
            if state.as_deref() == Ok("completed_settled") {
                return;
            }
        }
        assert!(
            start.elapsed() < deadline,
            "native domain turn did not settle"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn poll_native_exit(db: &Path, session: &str, deadline: Duration) {
    let start = Instant::now();
    loop {
        if db.exists()
            && let Ok(connection) =
                Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            let recorded: rusqlite::Result<i64> = connection.query_row(
                "SELECT COUNT(*) FROM native_process_exits WHERE session_id=?1",
                [session],
                |row| row.get(0),
            );
            if recorded == Ok(1) {
                return;
            }
        }
        assert!(
            start.elapsed() < deadline,
            "native process exit for {session} was not recorded"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn try_request_replacement(socket: &Path, revision: u64) -> Result<Value, String> {
    let mut command = json!({
        "protocol_version":2,"principal_ref":"principal-C1","command_id":"command:s6:replace",
        "subject_id":"subject-C1","context_generation":"context-C1","build_id":"build-C1",
        "proof_run_id":"proof-C1","grant_ref":"grant:s6-replace","expected_revision":revision.to_string(),
        "kind":"request_replacement","payload":{"reason":"S6 controlled semantic checkpoint"}
    });
    command["request_digest"] = json!(
        CodecContract::LifecycleCommandV2
            .digest_json(&command)
            .map_err(|e| e.to_string())?
            .hex()
    );
    let mut stream = UnixStream::connect(socket).map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut stream, &json!({"op":"command","command":command}))
        .map_err(|e| e.to_string())?;
    stream.write_all(b"\n").map_err(|e| e.to_string())?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&line).map_err(|e| e.to_string())
}

fn request_replacement(socket: &Path, revision: u64) -> Value {
    try_request_replacement(socket, revision).unwrap()
}

fn release_native_barriers(dir: &Path, hold_before_commit: bool) {
    fs::create_dir_all(dir).unwrap();
    for point in [
        "native_before_turn_entry",
        "native_after_turn_send_intent",
        "native_after_result_commit",
        "semantic_before_turn_entry",
    ] {
        fs::write(dir.join(format!("{point}.release")), b"release\n").unwrap();
    }
    if !hold_before_commit {
        fs::write(dir.join("semantic_before_commit.release"), b"release\n").unwrap();
    }
}

fn poll_semantic_result(db: &Path, phase: &str, deadline: Duration) -> (String, String) {
    let start = Instant::now();
    loop {
        if db.exists()
            && let Ok(connection) =
                Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            let result = connection.query_row(
                    "SELECT verdict,eligibility FROM semantic_results WHERE phase=?1 ORDER BY rowid DESC LIMIT 1",
                    [phase], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))
                );
            if let Ok(result) = result {
                return result;
            }
        }
        if start.elapsed() >= deadline {
            panic!(
                "semantic result {phase} did not commit in {}: {}",
                db.display(),
                diagnose_semantic_db(db)
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn diagnose_semantic_db(db: &Path) -> String {
    let Ok(connection) = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return "database unavailable".into();
    };
    let queries = [
        (
            "subject",
            "SELECT owner_kind,owner_ref,revision,semantic_revision FROM subjects",
        ),
        ("transition", "SELECT transition_id,stage FROM transitions"),
        (
            "native_sessions",
            "SELECT session_id,role,allocation_state FROM native_sessions",
        ),
        (
            "native_turns",
            "SELECT invocation_id,purpose,state FROM native_turns",
        ),
        (
            "capture",
            "SELECT capture_id,eligibility FROM semantic_captures",
        ),
        (
            "attempts",
            "SELECT invocation_id,phase,state FROM semantic_attempts",
        ),
        (
            "results",
            "SELECT phase,verdict,eligibility FROM semantic_results",
        ),
        (
            "exits",
            "SELECT session_id,outcome_kind FROM native_process_exits",
        ),
    ];
    let mut out = Vec::new();
    for (label, query) in queries {
        let Ok(mut statement) = connection.prepare(query) else {
            continue;
        };
        let width = statement.column_count();
        let rows = statement.query_map([], |row| {
            (0..width)
                .map(|index| {
                    row.get::<_, rusqlite::types::Value>(index)
                        .map(|value| format!("{value:?}"))
                })
                .collect::<rusqlite::Result<Vec<_>>>()
        });
        if let Ok(rows) = rows {
            out.push(format!(
                "{label}={:?}",
                rows.filter_map(Result::ok).collect::<Vec<_>>()
            ));
        }
    }
    out.join("; ")
}

fn poll_transition_stage(db: &Path, expected: &str, deadline: Duration) {
    let start = Instant::now();
    loop {
        if db.exists()
            && let Ok(connection) =
                Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        {
            let stage: rusqlite::Result<String> = connection.query_row(
                "SELECT stage FROM transitions ORDER BY rowid DESC LIMIT 1",
                [],
                |row| row.get(0),
            );
            if stage.as_deref() == Ok(expected) {
                return;
            }
        }
        if start.elapsed() >= deadline {
            panic!(
                "transition stage {expected} did not commit in {}: {}",
                db.display(),
                diagnose_semantic_db(db)
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn assert_isolated_candidate_sessions(db: &Path) {
    let connection = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut statement = connection.prepare(
        "SELECT role,session_id,thread_id FROM native_sessions WHERE role IN ('domain','semantic_compile','semantic_verify') ORDER BY role"
    ).unwrap();
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(
        rows.len(),
        3,
        "expected domain, compiler and verifier session rows: {rows:?}"
    );
    let roles = rows
        .iter()
        .map(|row| row.0.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        roles,
        ["domain", "semantic_compile", "semantic_verify"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        rows.iter()
            .map(|row| row.1.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    assert_eq!(
        rows.iter()
            .map(|row| row.2.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
}

fn domain_checkpoint_text() -> String {
    json!({"protocol_version":1,"predecessor_safe":true,"source_frozen":true,
        "checkpoint_ready":true,"switch_authorized":true,"successor_context":"context-C1-next"})
    .to_string()
}

fn c1_challenge_text() -> String {
    fixture()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "C1")
        .unwrap()["next_domain_prompt"]
        .as_str()
        .unwrap()
        .into()
}

fn successor_envelope(value: &Value) -> Option<Value> {
    if value.get("task").and_then(Value::as_str) == Some("use_attributed_continuation") {
        return Some(value.clone());
    }
    match value {
        Value::Object(object) => object.values().find_map(successor_envelope),
        Value::Array(items) => items.iter().find_map(successor_envelope),
        Value::String(text) => serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|nested| successor_envelope(&nested)),
        _ => None,
    }
}

#[test]
fn installed_native_semantic_c1_candidate_verdict() {
    for (variant, expected) in [
        (ControlledVariant::Complete, "accepted"),
        (ControlledVariant::OmitCommitment, "rejected"),
    ] {
        let fixture = fixture();
        let input = c1_input(&fixture);
        let mut outputs = vec![domain_checkpoint_text()];
        outputs.extend(c1_native_outputs(&input, variant));
        let simulator = LocalSimulator::start(outputs);
        let root = tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let barrier = root.path().join("barrier");
        release_native_barriers(&barrier, false);
        let config = service_config(root.path(), &simulator.base_url(), 1);
        let path = root.path().join("gen1.json");
        fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
        let socket = root.path().join("service-g1.sock");
        let mut child = ServiceChild::start(&path, root.path());
        wait_socket(&socket);
        let db = root.path().join("store/lifecycle.sqlite");
        poll_domain_settled(&db, Duration::from_secs(30));
        let revision = subject_revision(&db);
        let reply = request_replacement(&socket, revision);
        assert_eq!(reply["accepted"], true, "{reply}");
        let (verdict, eligibility) =
            poll_semantic_result(&db, "candidate_verify", Duration::from_secs(30));
        assert_eq!(verdict, expected);
        assert_eq!(eligibility, "current");
        assert_isolated_candidate_sessions(&db);
        if expected == "accepted" {
            poll_transition_stage(&db, "switching", Duration::from_secs(30));
        }
        child.terminate();
        let requests = simulator.finish();
        assert_eq!(
            requests.len(),
            3,
            "expected domain, compiler, verifier native entries"
        );
    }
}

#[test]
fn staged_or_published_source_basis_without_link_does_not_establish_checkpoint() {
    for point in [
        "semantic_after_capture_staged_before_publish",
        "semantic_after_capture_published_before_link",
    ] {
        let simulator = LocalSimulator::start(vec![domain_checkpoint_text()]);
        let root = tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let barrier = root.path().join("barrier");
        release_native_barriers(&barrier, false);
        fs::write(barrier.join(format!("{point}.arm")), b"arm\n").unwrap();
        let config = service_config(root.path(), &simulator.base_url(), 1);
        let path = root.path().join("gen1.json");
        fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
        let socket = root.path().join("service-g1.sock");
        let mut child = ServiceChild::start(&path, root.path());
        wait_socket(&socket);
        let db = root.path().join("store/lifecycle.sqlite");
        poll_domain_settled(&db, Duration::from_secs(30));
        let revision = subject_revision(&db);
        let command_socket = socket.clone();
        let pending = thread::spawn(move || try_request_replacement(&command_socket, revision));
        wait_semantic_path(
            &barrier.join(format!("{point}.reached")),
            &db,
            Duration::from_secs(45),
        );
        child.crash();
        assert!(
            pending.join().unwrap().is_err(),
            "crash must interrupt awaiting command reply"
        );
        let connection =
            Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let linked: i64 = connection
            .query_row("SELECT COUNT(*) FROM semantic_captures", [], |row| {
                row.get(0)
            })
            .unwrap();
        let attempts: i64 = connection
            .query_row("SELECT COUNT(*) FROM semantic_attempts", [], |row| {
                row.get(0)
            })
            .unwrap();
        let results: i64 = connection
            .query_row("SELECT COUNT(*) FROM semantic_results", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!((linked, attempts, results), (0, 0, 0));
        let stage: String = connection
            .query_row(
                "SELECT stage FROM transitions ORDER BY rowid DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stage, "verifying");
        drop(connection);
        let mut reopened =
            SqliteLifecycleStore::open(&root.path().join("store"), "issuer-C1".into()).unwrap();
        assert!(
            reopened
                .semantic_capture_record("capture-C1")
                .unwrap()
                .is_none(),
            "{point}: orphan bytes must not become a linked capture on recovery"
        );
        let projection = reopened
            .subject_projection(&SubjectId::parse("subject-C1").unwrap(), None, None)
            .unwrap();
        assert_eq!(projection.transition.unwrap().stage, "verifying", "{point}");
        let requests = simulator.finish();
        assert_eq!(
            requests.len(),
            1,
            "{point}: capture bytes may not send compiler inference"
        );
    }
}

#[test]
fn installed_successor_use_requires_attributed_adequacy_not_digest_echo() {
    for (variant, expected, final_stage, expected_requests) in [
        (ControlledVariant::Complete, "accepted", "reconciled", 6),
        (
            ControlledVariant::EchoSuccessor,
            "unresolved",
            "rehydrating",
            5,
        ),
    ] {
        let input = c1_input(&fixture());
        let mut outputs = vec![domain_checkpoint_text()];
        outputs.extend(c1_native_outputs(&input, variant));
        outputs.push("GEN2 domain work: no external publication requested.".into());
        let simulator = LocalSimulator::start(outputs);
        let root = tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        release_native_barriers(&root.path().join("barrier"), false);
        let db = root.path().join("store/lifecycle.sqlite");
        let first = service_config(root.path(), &simulator.base_url(), 1);
        let first_path = root.path().join("gen1.json");
        fs::write(&first_path, serde_json::to_vec(&first).unwrap()).unwrap();
        let first_socket = root.path().join("service-g1.sock");
        let mut first_child = ServiceChild::start(&first_path, root.path());
        wait_socket(&first_socket);
        poll_domain_settled(&db, Duration::from_secs(30));
        let reply = request_replacement(&first_socket, subject_revision(&db));
        assert_eq!(reply["accepted"], true, "{reply}");
        assert_eq!(
            poll_semantic_result(&db, "candidate_verify", Duration::from_secs(30)).0,
            "accepted"
        );
        poll_transition_stage(&db, "switching", Duration::from_secs(30));
        first_child.terminate();

        let second = service_config(root.path(), &simulator.base_url(), 2);
        let second_path = root.path().join("gen2.json");
        fs::write(&second_path, serde_json::to_vec(&second).unwrap()).unwrap();
        let second_socket = root.path().join("service-g2.sock");
        let mut second_child = ServiceChild::start(&second_path, root.path());
        wait_socket(&second_socket);
        let (verdict, eligibility) =
            poll_semantic_result(&db, "successor_adequacy", Duration::from_secs(30));
        assert_eq!(verdict, expected);
        assert_eq!(eligibility, "current");
        poll_transition_stage(&db, final_stage, Duration::from_secs(30));
        poll_native_exit(&db, "session:s6:g2", Duration::from_secs(30));
        second_child.terminate();
        let requests = simulator.finish();
        assert_eq!(
            requests.len(),
            expected_requests,
            "unexpected native request sequence"
        );
        let successor_input = requests[3].to_string();
        assert!(
            successor_input.contains("commitment-k1"),
            "successor lacked attributed candidate"
        );
        assert!(
            successor_input.contains("challenge_sha256"),
            "successor lacked challenge binding"
        );
        let envelope = successor_envelope(&requests[3]).expect("exact successor envelope missing");
        assert_eq!(envelope["challenge"], c1_challenge_text());
        let challenge = envelope["challenge"].as_str().unwrap();
        for answer in [
            "K-1",
            "U-1",
            "Q-1",
            "D-2",
            "I-2",
            "unfinished",
            "publication scope",
        ] {
            assert!(
                !challenge.contains(answer),
                "challenge leaked obligation answer {answer}"
            );
        }
        let verifier_input = requests[4].to_string();
        assert!(
            verifier_input.contains("successor_response_sha256"),
            "adequacy verifier lacked exact response binding"
        );
        assert!(!verifier_input.contains("compiler_reasoning"));
        assert!(!verifier_input.contains("hidden_oracle"));
    }
}

#[test]
fn dependent_owner_revision_wins_final_activation_race() {
    let input = c1_input(&fixture());
    let mut outputs = vec![domain_checkpoint_text()];
    outputs.extend(c1_native_outputs(&input, ControlledVariant::Complete));
    outputs.push("GEN2 domain work must not start after a stale owner fence.".into());
    let simulator = LocalSimulator::start(outputs);
    let root = tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let barrier = root.path().join("barrier");
    release_native_barriers(&barrier, true);
    fs::write(barrier.join("semantic_before_commit.arm"), b"arm\n").unwrap();
    let db = root.path().join("store/lifecycle.sqlite");

    let first = service_config(root.path(), &simulator.base_url(), 1);
    let first_path = root.path().join("gen1.json");
    fs::write(&first_path, serde_json::to_vec(&first).unwrap()).unwrap();
    let first_socket = root.path().join("service-g1.sock");
    let mut first_child = ServiceChild::start(&first_path, root.path());
    wait_socket(&first_socket);
    poll_domain_settled(&db, Duration::from_secs(30));
    assert_eq!(
        request_replacement(&first_socket, subject_revision(&db))["accepted"],
        true
    );
    assert_eq!(
        poll_semantic_result(&db, "candidate_verify", Duration::from_secs(30)).0,
        "accepted"
    );
    poll_transition_stage(&db, "switching", Duration::from_secs(30));
    first_child.terminate();

    let second = service_config(root.path(), &simulator.base_url(), 2);
    let second_path = root.path().join("gen2.json");
    fs::write(&second_path, serde_json::to_vec(&second).unwrap()).unwrap();
    let second_socket = root.path().join("service-g2.sock");
    let mut second_child = ServiceChild::start(&second_path, root.path());
    wait_socket(&second_socket);
    wait_semantic_path(
        &barrier.join("semantic_before_commit.reached"),
        &db,
        Duration::from_secs(45),
    );
    assert_eq!(
        poll_semantic_result(&db, "successor_adequacy", Duration::from_secs(30)).0,
        "accepted"
    );
    let before_revision = subject_revision(&db);
    fs::write(root.path().join("update-q1"), b"update\n").unwrap();
    wait_path(&root.path().join("updated-q1"));
    fs::write(barrier.join("semantic_before_commit.release"), b"release\n").unwrap();
    poll_native_exit(&db, "session:s6:g2", Duration::from_secs(30));
    poll_transition_stage(&db, "ready_to_commit", Duration::from_secs(5));
    assert_eq!(
        subject_revision(&db),
        before_revision,
        "stale Q-1 revision may not activate successor"
    );
    second_child.terminate();
    let mut store =
        SqliteLifecycleStore::open(&root.path().join("store"), "issuer-C1".into()).unwrap();
    let subject = SubjectId::parse("subject-C1").unwrap();
    let projection = store.subject_projection(&subject, None, None).unwrap();
    let transition = TransitionId::parse(projection.owner_ref.clone().unwrap()).unwrap();
    assert!(
        store.semantic_transition_required(&transition).unwrap(),
        "semantic path must remain durably required"
    );
    assert_eq!(
        store
            .semantic_capture_record("capture-C1")
            .unwrap()
            .unwrap()
            .eligibility,
        "stale"
    );
    assert!(
        store.semantic_checkpoint(&transition).unwrap().is_none(),
        "stale package cannot serve as current checkpoint"
    );
    let wall = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let direct = store.apply_transition_fact(
        &transition,
        Revision::new(projection.revision),
        TransitionFact::CommitSuccessor {
            evidence: EvidenceId::parse("evidence:s6:no-fallback").unwrap(),
            semantic_revision: Revision::new(projection.semantic_revision),
        },
        Some(&GrantId::parse("grant:s6-successor-input").unwrap()),
        ClockSample {
            wall: WallTimeMs::new(wall),
            wait_budget: WaitBudgetMs::new(1000),
        },
    );
    assert!(
        matches!(direct, Err(lifecycle_store::StoreError::Rejected)),
        "semantic-required transition may not fall back to S5 commit: {direct:?}"
    );
    let requests = simulator.finish();
    assert_eq!(
        requests.len(),
        5,
        "stale owner revision must prevent successor domain work"
    );
}

#[cfg(feature = "controlled-proof")]
#[test]
fn semantic_required_ready_to_commit_rejects_controlled_restart_without_owner_port() {
    let input = c1_input(&fixture());
    let mut outputs = vec![domain_checkpoint_text()];
    outputs.extend(c1_native_outputs(&input, ControlledVariant::Complete));
    outputs.push("Successor domain work must not start after a disabled semantic restart.".into());
    let simulator = LocalSimulator::start(outputs);
    let root = tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let barrier = root.path().join("barrier");
    release_native_barriers(&barrier, true);
    fs::write(barrier.join("semantic_before_commit.arm"), b"arm\n").unwrap();
    let db = root.path().join("store/lifecycle.sqlite");

    let first = service_config(root.path(), &simulator.base_url(), 1);
    let first_path = root.path().join("gen1.json");
    fs::write(&first_path, serde_json::to_vec(&first).unwrap()).unwrap();
    let first_socket = root.path().join("service-g1.sock");
    let mut first_child = ServiceChild::start(&first_path, root.path());
    wait_socket(&first_socket);
    poll_domain_settled(&db, Duration::from_secs(30));
    assert_eq!(
        request_replacement(&first_socket, subject_revision(&db))["accepted"],
        true
    );
    assert_eq!(
        poll_semantic_result(&db, "candidate_verify", Duration::from_secs(30)).0,
        "accepted"
    );
    poll_transition_stage(&db, "switching", Duration::from_secs(30));
    first_child.terminate();

    let second = service_config(root.path(), &simulator.base_url(), 2);
    let second_path = root.path().join("gen2.json");
    fs::write(&second_path, serde_json::to_vec(&second).unwrap()).unwrap();
    let second_socket = root.path().join("service-g2.sock");
    let mut second_child = ServiceChild::start(&second_path, root.path());
    wait_socket(&second_socket);
    wait_semantic_path(
        &barrier.join("semantic_before_commit.reached"),
        &db,
        Duration::from_secs(45),
    );
    assert_eq!(
        poll_semantic_result(&db, "successor_adequacy", Duration::from_secs(30)),
        ("accepted".into(), "current".into())
    );
    poll_transition_stage(&db, "ready_to_commit", Duration::from_secs(5));
    second_child.crash();

    let connection = Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (transition, required, stage): (String, i64, String) = connection
        .query_row(
            "SELECT transition_id,semantic_required,stage FROM transitions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!((required, stage.as_str()), (1, "ready_to_commit"));
    let eligibility: String = connection
        .query_row(
            "SELECT eligibility FROM semantic_captures WHERE transition_id=?1",
            [&transition],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        eligibility, "current",
        "regression requires DB-current cached basis"
    );
    drop(connection);

    // The external synthetic question owner has advanced. A restarted service
    // without the canonical material port cannot observe or fence this state.
    fs::write(root.path().join("seed-q1-r8"), b"owner revision 8\n").unwrap();
    let mut disabled = second;
    disabled["profile"] = json!("controlled");
    disabled["native"] = Value::Null;
    disabled["socket_path"] = json!(root.path().join("service-disabled.sock"));
    disabled["proof_barrier_dir"] = Value::Null;
    disabled["verifier_executable"] = json!("/bin/true");
    disabled["verifier_ledger_path"] = json!(root.path().join("unused-verifier-ledger"));
    fs::write(root.path().join("unused-verifier-ledger"), b"").unwrap();
    let disabled_path = root.path().join("disabled.json");
    fs::write(&disabled_path, serde_json::to_vec(&disabled).unwrap()).unwrap();
    let mut child = ServiceChild::start_semantic_disabled(&disabled_path);
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(2) {
        if child.0.try_wait().unwrap().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    if child.0.try_wait().unwrap().is_none() {
        child.terminate();
    }

    let connection = Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (stage, required): (String, i64) = connection
        .query_row(
            "SELECT stage,semantic_required FROM transitions WHERE transition_id=?1",
            [&transition],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        (stage.as_str(), required),
        ("ready_to_commit", 1),
        "controlled restart without owner fence committed a DB-current semantic basis"
    );
    let eligibility: String = connection
        .query_row(
            "SELECT eligibility FROM semantic_captures WHERE transition_id=?1",
            [&transition],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        eligibility, "current",
        "test must not rely on stale-package rejection"
    );
    drop(connection);
    let requests = simulator.finish();
    assert_eq!(
        requests.len(),
        5,
        "disabled restart must not send successor domain work"
    );
}

#[test]
fn unavailable_or_truncated_owner_material_has_durable_unresolved_capture_without_control() {
    for marker in ["material-unavailable", "material-truncated"] {
        let simulator = LocalSimulator::start(vec![domain_checkpoint_text()]);
        let root = tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.path().join(marker), b"issue\n").unwrap();
        release_native_barriers(&root.path().join("barrier"), false);
        let config = service_config(root.path(), &simulator.base_url(), 1);
        let path = root.path().join("gen1.json");
        fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
        let socket = root.path().join("service-g1.sock");
        let mut child = ServiceChild::start(&path, root.path());
        wait_socket(&socket);
        let db = root.path().join("store/lifecycle.sqlite");
        poll_domain_settled(&db, Duration::from_secs(30));
        assert_eq!(
            request_replacement(&socket, subject_revision(&db))["accepted"],
            true
        );
        poll_transition_stage(&db, "verifying", Duration::from_secs(5));
        child.terminate();
        let mut store =
            SqliteLifecycleStore::open(&root.path().join("store"), "issuer-C1".into()).unwrap();
        let capture = store
            .semantic_capture_record("capture-C1")
            .unwrap()
            .unwrap();
        assert_eq!(capture.eligibility, "unresolved", "{marker}");
        assert!(!capture.basis_artifact_sha256.is_empty());
        let mut conflicting_input = c1_input(&fixture());
        conflicting_input.transition = TransitionId::parse(capture.transition_id.clone()).unwrap();
        let conflicting_basis = project_material(conflicting_input).unwrap();
        let projection = store
            .subject_projection(&SubjectId::parse("subject-C1").unwrap(), None, None)
            .unwrap();
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let conflict = store.record_semantic_capture(
            &conflicting_basis,
            &SemanticPolicy::default(),
            Revision::new(projection.semantic_revision),
            ClockSample {
                wall: WallTimeMs::new(wall),
                wait_budget: WaitBudgetMs::new(1000),
            },
        );
        assert!(
            matches!(conflict, Err(lifecycle_store::StoreError::ClaimConflict)),
            "{marker}: same capture ID with different material bytes must conflict: {conflict:?}"
        );
        let connection =
            Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let attempts: i64 = connection
            .query_row("SELECT COUNT(*) FROM semantic_attempts", [], |row| {
                row.get(0)
            })
            .unwrap();
        let results: i64 = connection
            .query_row("SELECT COUNT(*) FROM semantic_results", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            (attempts, results),
            (0, 0),
            "{marker} may not enter control"
        );
        assert_eq!(
            simulator.finish().len(),
            1,
            "{marker} may not send compiler inference"
        );
    }
}

#[test]
fn unknown_schema_or_policy_has_durable_rejection_without_control() {
    for marker in ["unknown-schema", "unknown-policy"] {
        let simulator = LocalSimulator::start(vec![domain_checkpoint_text()]);
        let root = tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.path().join(marker), b"unsupported\n").unwrap();
        release_native_barriers(&root.path().join("barrier"), false);
        let config = service_config(root.path(), &simulator.base_url(), 1);
        let path = root.path().join("gen1.json");
        fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
        let socket = root.path().join("service-g1.sock");
        let db = root.path().join("store/lifecycle.sqlite");
        let mut child = ServiceChild::start(&path, root.path());
        wait_socket(&socket);
        poll_domain_settled(&db, Duration::from_secs(30));
        assert_eq!(
            request_replacement(&socket, subject_revision(&db))["accepted"],
            true
        );
        poll_transition_stage(&db, "verifying", Duration::from_secs(5));
        child.terminate();
        let mut store =
            SqliteLifecycleStore::open(&root.path().join("store"), "issuer-C1".into()).unwrap();
        let rejection = store
            .semantic_material_rejection("capture-C1")
            .unwrap()
            .unwrap();
        assert_eq!(rejection.eligibility, "unresolved", "{marker}");
        assert_eq!(rejection.error_code, "unsupported_policy", "{marker}");
        assert_eq!(
            rejection.schema_version,
            if marker == "unknown-schema" { 2 } else { 1 }
        );
        assert_eq!(
            rejection.policy_id,
            if marker == "unknown-policy" {
                "unknown-policy-v1"
            } else {
                "s6-strict-synthetic-v1"
            }
        );
        assert!(
            store
                .semantic_capture_record("capture-C1")
                .unwrap()
                .is_none()
        );
        let projection = store
            .subject_projection(&SubjectId::parse("subject-C1").unwrap(), None, None)
            .unwrap();
        assert_eq!(projection.transition.unwrap().stage, "verifying");
        drop(store);
        let connection =
            Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let attempts: i64 = connection
            .query_row("SELECT COUNT(*) FROM semantic_attempts", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(attempts, 0, "{marker}: no control entry");
        assert_eq!(simulator.finish().len(), 1, "{marker}: no compiler request");
    }
}

#[test]
fn semantic_native_config_requires_explicit_material_port_before_store_open() {
    let simulator = LocalSimulator::start(vec![]);
    let root = tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let config: lifecycle_service::ServiceConfig =
        serde_json::from_value(service_config(root.path(), &simulator.base_url(), 1)).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = runtime.block_on(lifecycle_service::run(config));
    assert!(matches!(result, Err(ServiceError::Configuration)));
    assert!(
        !root.path().join("store").exists(),
        "missing canonical owner port must fail before store open"
    );
    assert!(
        simulator.finish().is_empty(),
        "missing owner port may not enter inference"
    );
}

#[test]
fn control_grant_contraction_respects_entry_boundary() {
    for (point, expected_requests, expected_compile_attempts) in [
        ("semantic_compile_before_entry", 1, 0),
        ("semantic_compile_after_entry", 2, 1),
    ] {
        let input = c1_input(&fixture());
        let mut outputs = vec![domain_checkpoint_text()];
        outputs.extend(c1_native_outputs(&input, ControlledVariant::Complete));
        let simulator = LocalSimulator::start(outputs);
        let root = tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let barrier = root.path().join("barrier");
        release_native_barriers(&barrier, false);
        fs::write(barrier.join(format!("{point}.arm")), b"arm\n").unwrap();
        let config = service_config(root.path(), &simulator.base_url(), 1);
        let path = root.path().join("gen1.json");
        fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
        let socket = root.path().join("service-g1.sock");
        let db = root.path().join("store/lifecycle.sqlite");
        let mut child = ServiceChild::start(&path, root.path());
        wait_socket(&socket);
        poll_domain_settled(&db, Duration::from_secs(30));
        let revision = subject_revision(&db);
        let command_socket = socket.clone();
        let pending = thread::spawn(move || try_request_replacement(&command_socket, revision));
        wait_path(&barrier.join(format!("{point}.reached")));
        fs::write(barrier.join(format!("{point}.revoke_grant")), b"revoke\n").unwrap();
        fs::write(barrier.join(format!("{point}.release")), b"release\n").unwrap();
        let reply = pending.join().unwrap().unwrap();
        assert_eq!(reply["accepted"], true, "{point}: {reply}");
        poll_native_exit(&db, "session:s6:compile:g1", Duration::from_secs(30));
        child.terminate();
        let requests = simulator.finish();
        assert_eq!(
            requests.len(),
            expected_requests,
            "{point}: no post-revocation dependent send"
        );
        let connection =
            Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let revoked: i64 = connection
            .query_row(
                "SELECT revoked FROM grants WHERE grant_id='grant:s6-control'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            revoked, 1,
            "{point}: exact grant must be revoked by store API"
        );
        let compile_attempts:i64=connection.query_row("SELECT COUNT(*) FROM semantic_attempts WHERE phase='compile' AND state='completed_settled'",[],|row|row.get(0)).unwrap();
        assert_eq!(compile_attempts, expected_compile_attempts, "{point}");
        let results: i64 = connection
            .query_row("SELECT COUNT(*) FROM semantic_results", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            results, 0,
            "{point}: revoked grant must block later assessment/activation"
        );
        let stage: String = connection
            .query_row(
                "SELECT stage FROM transitions ORDER BY rowid DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stage, "verifying", "{point}");
    }
}

#[test]
fn dependent_owner_update_after_freeze_or_during_verification_retires_basis() {
    for point in [
        "semantic_after_capture_published_before_link",
        "semantic_verify_after_entry",
    ] {
        let input = c1_input(&fixture());
        let mut outputs = vec![domain_checkpoint_text()];
        outputs.extend(c1_native_outputs(&input, ControlledVariant::Complete));
        let simulator = LocalSimulator::start(outputs);
        let root = tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let barrier = root.path().join("barrier");
        release_native_barriers(&barrier, false);
        fs::write(barrier.join(format!("{point}.arm")), b"arm\n").unwrap();
        let config = service_config(root.path(), &simulator.base_url(), 1);
        let path = root.path().join("gen1.json");
        fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
        let socket = root.path().join("service-g1.sock");
        let db = root.path().join("store/lifecycle.sqlite");
        let mut child = ServiceChild::start(&path, root.path());
        wait_socket(&socket);
        poll_domain_settled(&db, Duration::from_secs(30));
        let revision = subject_revision(&db);
        let command_socket = socket.clone();
        let pending = thread::spawn(move || try_request_replacement(&command_socket, revision));
        wait_semantic_path(
            &barrier.join(format!("{point}.reached")),
            &db,
            Duration::from_secs(45),
        );
        fs::write(root.path().join("update-q1"), b"update\n").unwrap();
        wait_path(&root.path().join("updated-q1"));
        fs::write(barrier.join(format!("{point}.release")), b"release\n").unwrap();
        assert_eq!(pending.join().unwrap().unwrap()["accepted"], true);
        child.terminate();
        let mut store =
            SqliteLifecycleStore::open(&root.path().join("store"), "issuer-C1".into()).unwrap();
        let capture = store
            .semantic_capture_record("capture-C1")
            .unwrap()
            .unwrap();
        assert_eq!(capture.eligibility, "stale", "{point}");
        let projection = store
            .subject_projection(&SubjectId::parse("subject-C1").unwrap(), None, None)
            .unwrap();
        assert_eq!(projection.transition.unwrap().stage, "verifying", "{point}");
        let transition = TransitionId::parse(capture.transition_id).unwrap();
        assert!(store.semantic_checkpoint(&transition).unwrap().is_none());
        let requests = simulator.finish();
        assert!(
            (1..=3).contains(&requests.len()),
            "{point}: no successor inference"
        );
    }
}

#[test]
fn changed_owner_after_checkpoint_blocks_successor_startup() {
    let input = c1_input(&fixture());
    let mut outputs = vec![domain_checkpoint_text()];
    outputs.extend(c1_native_outputs(&input, ControlledVariant::Complete));
    let simulator = LocalSimulator::start(outputs);
    let root = tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    release_native_barriers(&root.path().join("barrier"), false);
    let db = root.path().join("store/lifecycle.sqlite");
    let first = service_config(root.path(), &simulator.base_url(), 1);
    let first_path = root.path().join("gen1.json");
    fs::write(&first_path, serde_json::to_vec(&first).unwrap()).unwrap();
    let socket = root.path().join("service-g1.sock");
    let mut first_child = ServiceChild::start(&first_path, root.path());
    wait_socket(&socket);
    poll_domain_settled(&db, Duration::from_secs(30));
    assert_eq!(
        request_replacement(&socket, subject_revision(&db))["accepted"],
        true
    );
    assert_eq!(
        poll_semantic_result(&db, "candidate_verify", Duration::from_secs(30)).0,
        "accepted"
    );
    poll_transition_stage(&db, "switching", Duration::from_secs(30));
    first_child.terminate();
    fs::write(root.path().join("seed-q1-r8"), b"seed\n").unwrap();
    fs::write(root.path().join("expect-startup-reject"), b"expected\n").unwrap();
    let second = service_config(root.path(), &simulator.base_url(), 2);
    let second_path = root.path().join("gen2.json");
    fs::write(&second_path, serde_json::to_vec(&second).unwrap()).unwrap();
    let mut second_child = ServiceChild::start(&second_path, root.path());
    second_child.wait_success(Duration::from_secs(20));
    let mut store =
        SqliteLifecycleStore::open(&root.path().join("store"), "issuer-C1".into()).unwrap();
    let capture = store
        .semantic_capture_record("capture-C1")
        .unwrap()
        .unwrap();
    assert_eq!(capture.eligibility, "stale");
    let projection = store
        .subject_projection(&SubjectId::parse("subject-C1").unwrap(), None, None)
        .unwrap();
    assert_eq!(projection.transition.unwrap().stage, "switching");
    let connection = Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let successor_sessions: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM native_sessions WHERE role='successor'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        successor_sessions, 0,
        "stale owner must be checked before successor native session allocation"
    );
    assert_eq!(
        simulator.finish().len(),
        3,
        "no successor inference after checkpoint staleness"
    );
}

#[test]
fn control_entry_and_result_crashes_preserve_exact_no_replay_custody() {
    for (point, phase, expected_state, expected_requests, expected_results) in [
        (
            "semantic_compile_after_entry",
            "compile",
            "entered_uncertain",
            1,
            0,
        ),
        (
            "semantic_verify_after_entry",
            "candidate_verify",
            "entered_uncertain",
            2,
            0,
        ),
        (
            "semantic_verify_after_result",
            "candidate_verify",
            "completed_settled",
            3,
            0,
        ),
        (
            "semantic_after_candidate_result",
            "candidate_verify",
            "completed_settled",
            3,
            1,
        ),
    ] {
        let input = c1_input(&fixture());
        let mut outputs = vec![domain_checkpoint_text()];
        outputs.extend(c1_native_outputs(&input, ControlledVariant::Complete));
        let simulator = LocalSimulator::start(outputs);
        let root = tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let barrier = root.path().join("barrier");
        release_native_barriers(&barrier, false);
        fs::write(barrier.join(format!("{point}.arm")), b"arm\n").unwrap();
        let config = service_config(root.path(), &simulator.base_url(), 1);
        let path = root.path().join("gen1.json");
        fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
        let socket = root.path().join("service-g1.sock");
        let db = root.path().join("store/lifecycle.sqlite");
        let mut child = ServiceChild::start(&path, root.path());
        wait_socket(&socket);
        poll_domain_settled(&db, Duration::from_secs(30));
        let revision = subject_revision(&db);
        let command_socket = socket.clone();
        let pending = thread::spawn(move || try_request_replacement(&command_socket, revision));
        wait_path(&barrier.join(format!("{point}.reached")));
        child.crash();
        assert!(
            pending.join().unwrap().is_err(),
            "{point}: command reply must be interrupted by crash"
        );
        let mut store =
            SqliteLifecycleStore::open(&root.path().join("store"), "issuer-C1".into()).unwrap();
        let capture = store
            .semantic_capture_record("capture-C1")
            .unwrap()
            .unwrap();
        assert_eq!(capture.eligibility, "current", "{point}");
        let projection = store
            .subject_projection(&SubjectId::parse("subject-C1").unwrap(), None, None)
            .unwrap();
        let transition = TransitionId::parse(projection.transition.unwrap().transition_id).unwrap();
        assert!(
            store.semantic_checkpoint(&transition).unwrap().is_none(),
            "{point}: no package may appear from unlinked result"
        );
        assert_eq!(
            store
                .subject_projection(&SubjectId::parse("subject-C1").unwrap(), None, None)
                .unwrap()
                .transition
                .unwrap()
                .stage,
            "verifying",
            "{point}"
        );
        drop(store);
        let connection =
            Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let state: String = connection
            .query_row(
                "SELECT state FROM semantic_attempts WHERE phase=?1",
                [phase],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state, expected_state, "{point}");
        let results: i64 = connection
            .query_row("SELECT COUNT(*) FROM semantic_results", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            results, expected_results,
            "{point}: exact semantic result linkage after crash"
        );
        assert_eq!(
            simulator.finish().len(),
            expected_requests,
            "{point}: store reopen must not resend inference"
        );
    }
}

#[test]
fn synthetic_owner_fence_serializes_dependent_revision_with_short_commit() {
    let fixture = fixture();
    let input = c1_input(&fixture);
    let journal = SyntheticOwnerJournal::new(input.clone()).unwrap();
    let port = SyntheticPort {
        journal: journal.clone(),
        project_override: None,
    };
    let projected = port
        .project(&input.subject, &input.transition, &input.capture_id)
        .unwrap();
    let basis = project_material(projected).unwrap();
    assert!(basis.is_eligible());
    let expected = basis.dependencies.clone();

    let (go_tx, go_rx) = mpsc::channel();
    let (started_tx, started_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let update = source_from_fixture(&fixture, "question-q1-r8", true);
    let updater = thread::spawn(move || {
        go_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        started_tx.send(()).unwrap();
        journal.update_source(update).unwrap();
        done_tx.send(()).unwrap();
    });
    let committed = Arc::new(AtomicBool::new(false));
    let committed_in_guard = committed.clone();
    port.with_current_fence(&expected, &mut || {
        go_tx.send(()).unwrap();
        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        committed_in_guard.store(true, Ordering::SeqCst);
        Ok(())
    })
    .unwrap();
    updater.join().unwrap();
    assert!(committed.load(Ordering::SeqCst));
    assert!(matches!(
        port.with_current_fence(&expected, &mut || Ok(())),
        Err(ServiceError::Material(MaterialPortError::Conflict))
    ));
}

#[test]
fn unrelated_t9_owner_revision_does_not_invalidate_c1_fence() {
    let fixtures = fixture();
    let input = c1_input(&fixtures);
    let basis = project_material(input.clone()).unwrap();
    let journal = SyntheticOwnerJournal::new(input).unwrap();
    let mut t9 = source_from_fixture(&fixtures, "unrelated-r9", true);
    t9.dependency_required = false;
    journal.add_unrelated_source(t9.clone()).unwrap();
    let mut next = t9;
    next.id = "unrelated-r9-r10".into();
    next.owner_ref = next.id.clone();
    next.revision = 10;
    next.bytes = b"Other subject T-9 changed its title again; T-1 has no dependency.\n".to_vec();
    next.sha256 = sha256_hex(&next.bytes);
    journal.update_source(next).unwrap();
    assert!(journal.revalidate(&basis.dependencies).unwrap().is_empty());
    journal
        .with_current_fence(&basis.dependencies, || Ok::<_, ()>(()))
        .unwrap();
}
