#![cfg(unix)]
mod support;

#[cfg(feature = "test-faults")]
use claim_evidence::OpenFiles;
use claim_evidence::{
    BootstrapFiles, RootIdentity,
    codec::{digest as claim_digest, parse_json as parse_claim_json},
    sha256_bytes,
};
use review_episode::{
    Application as EpisodeApplication,
    host_admission::NativeHostAdmission,
    in_process_admission::{EpisodeReadTarget, TrustedEpisodeReadScope},
    protocol::{DEFAULT_RESPONSE_LIMIT, Request},
};
use review_episode_core::{
    codec::{
        JsString as EpisodeString, JsValue as EpisodeValue, canonical_json as episode_json,
        digest as episode_digest, field as episode_field, parse_json as parse_episode_json,
    },
    identity::Authority,
};
#[cfg(feature = "test-faults")]
use review_episode_store::read_native_selection;
use review_episode_store::{EpisodeStore, NativeRootSelection, StoreOptions, init_native_root};
use review_execution_evidence::test_support::{
    ControlledFixtureOwner, FIXTURE_OWNER, FIXTURE_PROFILE, FIXTURE_VERIFIER, FixtureRecord,
};
use review_execution_evidence::{
    CheckedExecutionResult, CheckedObservation, ControlledScope, EvidenceReadError, EvidenceReader,
    EvidenceReaderProfile, EvidenceWriter, ExecutionBinding, ExecutionEvidenceOwner,
    ExecutionEvidenceRef, ExecutionProvenance, ExecutionPublication,
};
use serde_json::{Value, json};
#[cfg(feature = "test-faults")]
use slice_campaign::{Campaign, Reconcile, RecoveryLocator};
use slice_campaign::{
    DispatchCommand, DispatchEffect, InitialCompletionOwners, InitialJoinSelection,
    InitialProgressState, Preparation,
};
#[cfg(feature = "test-faults")]
use std::time::{Duration, Instant};
use std::{
    fs,
    fs::OpenOptions,
    os::{
        fd::AsRawFd,
        unix::{fs::PermissionsExt, process::CommandExt},
    },
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
    sync::Mutex,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
struct MutableFixtureOwner {
    records: Mutex<Vec<FixtureRecord>>,
}
impl ExecutionEvidenceOwner for MutableFixtureOwner {
    fn read_result(
        &self,
        reference: &ExecutionEvidenceRef,
    ) -> Result<CheckedExecutionResult, EvidenceReadError> {
        ControlledFixtureOwner::new(self.records.lock().unwrap().clone())?.read_result(reference)
    }
    fn read_custody(
        &self,
        reference: &ExecutionEvidenceRef,
        binding: &claim_evidence::ProductionPathAdmissionBinding,
        observation: Option<&claim_evidence::codec::JsValue>,
    ) -> Result<claim_evidence::CustodyEvidence, EvidenceReadError> {
        ControlledFixtureOwner::new(self.records.lock().unwrap().clone())?.read_custody(
            reference,
            binding,
            observation,
        )
    }
}

#[cfg(feature = "test-faults")]
fn test_fault_cut(name: &str) {
    if std::env::var("J1_TEST_FAULT_CUT").ok().as_deref() != Some(name) {
        return;
    }
    let root = std::path::PathBuf::from(std::env::var("SLICE_CAMPAIGN_FAULT_DIR").unwrap());
    fs::write(root.join(format!("{name}.ready")), b"ready\n").unwrap();
    loop {
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(feature = "test-faults")]
#[test]
fn crash_child_entry() {
    if std::env::var_os("J1_CUT_CHILD").is_some() {
        finish_native(setup_pair(true), true, false);
    }
}

#[cfg(feature = "test-faults")]
#[test]
fn serial_actual_child_crash_cut_recovery_ledger() {
    let cases = [
        "j1_after_prepare_commit",
        "j1_after_observation_commit",
        "j1_after_establishment_builder_commit",
        "j1_after_establishment_campaign_commit",
        "j1_after_admission_builder_read",
        "j1_after_admission_campaign_read",
        "j1_after_episode_commit",
        "j1_after_episode_read",
        "j1_after_joined_commit",
    ];
    for (index, cut) in cases.iter().enumerate() {
        if std::env::var("J1_ONLY_CUT")
            .ok()
            .is_some_and(|selected| selected != *cut)
        {
            continue;
        }
        eprintln!("J1 crash recovery cut {cut}");
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(format!("case-{index}"));
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "crash_child_entry", "--nocapture"])
            .env("J1_CUT_CHILD", "1")
            .env("P1_CORPUS_ROOT", &root)
            .env("SLICE_CAMPAIGN_FAULT_CUT", cut)
            .env("SLICE_CAMPAIGN_FAULT_DIR", &root)
            .env("J1_TEST_FAULT_CUT", cut)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let ready = root.join(format!("{cut}.ready"));
        let deadline = Instant::now() + Duration::from_secs(45);
        while !ready.exists() {
            if let Some(status) = child.try_wait().unwrap() {
                let output = child.wait_with_output().unwrap();
                panic!(
                    "cut {cut} exited {status}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            assert!(
                Instant::now() < deadline,
                "cut {cut} did not reach boundary"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        finish_native(reopen_process_pair(&root), true, false);
    }
}

struct CorruptAfterCustody {
    inner: ControlledFixtureOwner,
    database: std::path::PathBuf,
    used: AtomicBool,
}
impl ExecutionEvidenceOwner for CorruptAfterCustody {
    fn read_result(
        &self,
        reference: &ExecutionEvidenceRef,
    ) -> Result<CheckedExecutionResult, EvidenceReadError> {
        self.inner.read_result(reference)
    }
    fn read_custody(
        &self,
        reference: &ExecutionEvidenceRef,
        binding: &claim_evidence::ProductionPathAdmissionBinding,
        observation: Option<&claim_evidence::codec::JsValue>,
    ) -> Result<claim_evidence::CustodyEvidence, EvidenceReadError> {
        let custody = self.inner.read_custody(reference, binding, observation)?;
        if !self.used.swap(true, Ordering::SeqCst) {
            rusqlite::Connection::open(&self.database)
                .unwrap()
                .execute("UPDATE campaign_state SET state_json='{}'", [])
                .unwrap();
        }
        Ok(custody)
    }
}

#[test]
fn native_child_entry() {
    if std::env::var("J1_NATIVE_CHILD").ok().as_deref() != Some("write") {
        return;
    }
    let root = std::path::PathBuf::from(std::env::var("J1_NATIVE_ROOT").unwrap());
    let bytes = fs::read(std::env::var("J1_NATIVE_REQUEST").unwrap()).unwrap();
    let request = Request::parse(&bytes).unwrap();
    let (store, selection) = EpisodeStore::open_native(&root, StoreOptions::default()).unwrap();
    let admission = NativeHostAdmission::from_inherited_fd(selection).unwrap();
    let mut app = EpisodeApplication::new(store, admission, DEFAULT_RESPONSE_LIMIT);
    let reply = app.execute(&request).unwrap();
    fs::write(std::env::var("J1_NATIVE_REPLY").unwrap(), reply).unwrap();
}

fn native_request(
    operation: &str,
    id: &str,
    selection: &NativeRootSelection,
    args: EpisodeValue,
) -> EpisodeValue {
    EpisodeValue::object([
        ("version", EpisodeValue::Number(2.0)),
        ("domain", EpisodeValue::text("review-episode")),
        ("profile", EpisodeValue::text("native-host-v1")),
        ("requestId", EpisodeValue::text(id)),
        ("operation", EpisodeValue::text(operation)),
        ("grantId", EpisodeValue::text(id)),
        ("selectionRevision", EpisodeValue::text(&"a".repeat(64))),
        (
            "rootSelectionDigest",
            EpisodeValue::text(&selection.selection_digest),
        ),
        ("args", args),
    ])
}
fn native_descriptor(
    request: &EpisodeValue,
    selection: &NativeRootSelection,
    observed: Option<&str>,
) -> EpisodeValue {
    let operation = episode_field(request, "operation")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    let args = episode_field(request, "args").unwrap();
    let authority = args.get("authority").unwrap();
    let identity = authority.get("identity").unwrap();
    let key = review_episode_core::identity::Identity::parse(identity)
        .unwrap()
        .key()
        .0;
    let content = if operation == "begin" {
        EpisodeValue::object([
            ("action", EpisodeValue::text("begin")),
            (
                "unresolvedQuestions",
                args.get("unresolvedQuestions").unwrap().clone(),
            ),
        ])
    } else {
        EpisodeValue::object([
            ("action", args.get("action").unwrap().clone()),
            ("payload", args.get("payload").unwrap().clone()),
        ])
    };
    let result = args.get("payload").and_then(|p| p.get("result"));
    let evidence = args
        .get("payload")
        .and_then(|p| p.get("evidenceAdmissions"));
    EpisodeValue::object([
        ("schemaVersion", EpisodeValue::Number(1.0)),
        ("profile", EpisodeValue::text("native-host-v1")),
        ("root", EpisodeValue::text(&selection.root)),
        (
            "executableSha256",
            EpisodeValue::text(&selection.executable_sha256),
        ),
        (
            "rootSelectionDigest",
            EpisodeValue::text(&selection.selection_digest),
        ),
        (
            "requestId",
            episode_field(request, "requestId").unwrap().clone(),
        ),
        (
            "operation",
            episode_field(request, "operation").unwrap().clone(),
        ),
        (
            "grantId",
            episode_field(request, "grantId").unwrap().clone(),
        ),
        (
            "selectionRevision",
            episode_field(request, "selectionRevision").unwrap().clone(),
        ),
        (
            "requestDigest",
            EpisodeValue::text(&episode_digest(request)),
        ),
        ("identityKey", EpisodeValue::text(&key)),
        ("argsDigest", EpisodeValue::text(&episode_digest(args))),
        (
            "authorityDigest",
            EpisodeValue::text(&episode_digest(authority)),
        ),
        (
            "resultDigest",
            result.map_or(EpisodeValue::Null, |v| {
                EpisodeValue::text(&episode_digest(v))
            }),
        ),
        (
            "evidenceDigest",
            evidence.map_or(EpisodeValue::Null, |v| {
                EpisodeValue::text(&episode_digest(v))
            }),
        ),
        (
            "contentDigest",
            EpisodeValue::text(&episode_digest(&content)),
        ),
        (
            "observedRevision",
            observed.map_or(EpisodeValue::Null, EpisodeValue::text),
        ),
        (
            "principal",
            EpisodeValue::object([
                ("id", EpisodeValue::text("native-review-host")),
                ("identityKey", EpisodeValue::text(&key)),
                ("access", EpisodeValue::text("write")),
            ]),
        ),
    ])
}
fn write_native(
    root: &Path,
    selection: &NativeRootSelection,
    request: &EpisodeValue,
    observed: Option<&str>,
) -> EpisodeValue {
    let parent = root.parent().unwrap();
    let descriptor = parent.join("j1-native-descriptor");
    fs::write(
        &descriptor,
        episode_json(&native_descriptor(request, selection, observed)),
    )
    .unwrap();
    fs::set_permissions(&descriptor, fs::Permissions::from_mode(0o600)).unwrap();
    let file = OpenOptions::new().read(true).open(&descriptor).unwrap();
    fs::remove_file(&descriptor).unwrap();
    let request_path = parent.join("j1-native-request");
    let reply_path = parent.join("j1-native-reply");
    fs::write(&request_path, episode_json(request)).unwrap();
    if reply_path.exists() {
        fs::remove_file(&reply_path).unwrap();
    }
    let fd = file.as_raw_fd();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "native_child_entry", "--nocapture"])
        .env("J1_NATIVE_CHILD", "write")
        .env("J1_NATIVE_ROOT", root)
        .env("J1_NATIVE_REQUEST", &request_path)
        .env("J1_NATIVE_REPLY", &reply_path)
        .stdin(Stdio::null());
    unsafe {
        command.pre_exec(move || {
            unsafe extern "C" {
                fn dup2(oldfd: i32, newfd: i32) -> i32;
                fn fcntl(fd: i32, cmd: i32, arg: i32) -> i32;
            }
            if dup2(fd, 3) < 0 || (fd == 3 && fcntl(3, 2, 0) < 0) {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "native writer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    parse_episode_json(&fs::read_to_string(reply_path).unwrap()).unwrap()
}

fn claim_value(value: &Value) -> claim_evidence::codec::JsValue {
    parse_claim_json(&value.to_string()).unwrap()
}
fn authority(
    identity: &slice_campaign::CampaignIdentity,
    obligation: &str,
    episode_id: &str,
    candidate: &slice_campaign::CandidateRef,
) -> Authority {
    let mut base: Value = json!({"schemaVersion":1,"grantId":"grant-episode",
        "identity":{"runId":identity.run_id,"sliceNumber":identity.slice_number,
            "attemptId":identity.attempt_id,"planVersion":identity.plan_version,
            "reviewObligationId":obligation,"reviewEpisodeId":episode_id},
        "source":{"owner":"human","reference":"accepted-plan","revision":"plan-v1",
            "sha256":sha256_bytes(b"accepted-plan"),"freshness":"exact_revision"},
        "writer":{"actorId":"native-review-host","provider":"controlled",
            "generation":1,"runtimeSession":{"owner":"runtime","reference":"session-1",
                "revision":"generation-1","sha256":sha256_bytes(b"session-1"),
                "freshness":"exact_revision"}},
        "readers":["native-review-host","builder","supervisor"],
        "initialSubject":{"owner":"checkpoint","reference":"candidate",
            "revision":candidate.commit,"sha256":"","freshness":"exact_revision"},
        "predecessorRevision":null});
    let subject = json!({"commit":candidate.commit,"tree":candidate.tree,
        "patchIdentity":candidate.patch_identity});
    base["initialSubject"]["sha256"] = json!(episode_digest(
        &parse_episode_json(&subject.to_string()).unwrap()
    ));
    Authority::parse(parse_episode_json(&base.to_string()).unwrap()).unwrap()
}
fn claims_files(base: &std::path::Path, process: bool) -> (BootstrapFiles, RootIdentity, String) {
    let verifier = if process {
        "review-execution-evidence-process-v1"
    } else {
        FIXTURE_VERIFIER
    };
    let profile = if process {
        "controlled-process-v1"
    } else {
        FIXTURE_PROFILE
    };
    let artifact_scheme = if process {
        "execution-artifact:"
    } else {
        "artifact:"
    };
    let custody = json!({"owner":FIXTURE_OWNER,"verifier":verifier,
        "profile":profile,"artifact_schemes":[artifact_scheme],
        "reference_schemes":["execution-evidence:"]});
    let custody_sha = claim_digest(&claim_value(&custody)).unwrap();
    let source = json!({"schema_version":2,"owner":"host-1","reference":"source-1",
        "revision":"r1","freshness":"exact immutable revision",
        "profile":"native-review-claims-v1","actors":["campaign"],
        "decision_scopes":["scope-1"],"grant_ids":["grant-production","grant-findings"],
        "custody_config_sha256":custody_sha});
    let source_bytes = source.to_string();
    let source_sha = sha256_bytes(source_bytes.as_bytes());
    let reference = json!({"owner":"host-1","reference":"source-1","revision":"r1",
        "integrity_sha256":source_sha,"freshness":"exact immutable revision","status":"verified"});
    let grant = json!({"schema_version":1,"grant_id":"grant-production","actor":"campaign",
        "profile":"production-path-v1","permissions":["record_observation","establish_claim",
            "read_admission"],"decision_scope":"scope-1","authority_reference":reference});
    let grant_sha = claim_digest(&claim_value(&grant)).unwrap();
    let config = json!({"schema_version":2,"profile":"native-review-claims-v1",
        "root_id":"claims-root","trusted_custody":custody,"grants":[grant,
        {"schema_version":1,"grant_id":"grant-findings","actor":"campaign",
        "profile":"revision-bound-review-finding-v1","permissions":["create_claim","publish_revision","record_reliance"],
        "decision_scope":"scope-1","authority_reference":reference}]});
    let config_bytes = config.to_string();
    let config_path = base.join("claims-config.json");
    let source_path = base.join("claims-source.json");
    fs::write(&config_path, &config_bytes).unwrap();
    fs::write(&source_path, &source_bytes).unwrap();
    let root = base.join("claims-root");
    let files = BootstrapFiles {
        root: root.clone(),
        config: config_path,
        source: source_path,
        expected_config_sha256: sha256_bytes(config_bytes.as_bytes()),
        expected_source_sha256: source_sha,
    };
    (
        files,
        RootIdentity {
            absolute_path: root,
            root_id: "claims-root".into(),
            profile: "native-review-claims-v1".into(),
        },
        grant_sha,
    )
}

#[cfg(feature = "test-faults")]
fn reopen_process_pair(root: &Path) -> Pair {
    let (repo, workspace, identity, baseline, boundary) = support::setup_paths(root);
    let campaign = Campaign::open(support::config(
        root.join("campaign"),
        repo,
        workspace,
        identity.clone(),
        baseline,
        boundary,
    ))
    .unwrap();
    let locator = campaign
        .locate_initial_completion(&identity, "review-0", "complete-join")
        .unwrap()
        .expect("receipt-bound completion locator after crash");
    let state = campaign.read(&identity).unwrap().unwrap();
    let request = campaign
        .read(&identity)
        .unwrap()
        .unwrap()
        .obligations
        .iter()
        .find(|o| o.obligation_id == "review-0")
        .unwrap()
        .request
        .clone()
        .unwrap();
    let dispatch = state
        .progress
        .attempts
        .iter()
        .find(|a| a.attempt_id == request.identity.attempt_id)
        .and_then(|a| a.dispatch.as_ref())
        .unwrap();
    let dispatch_digest = slice_campaign::codec::campaign_digest(&json!({
        "identity":identity,"obligationId":"review-0",
        "preparationOperationId":request.operation_id,"requestDigest":request.request_digest,
        "expectedRevision":request.prepared_revision,"operationId":dispatch.operation_id,
        "command":dispatch.command}))
    .unwrap();
    let dispatch_locator = RecoveryLocator {
        root_id: campaign.root_id().into(),
        anchored_root: campaign.anchored_root().into(),
        identity: identity.clone(),
        operation_id: dispatch.operation_id.clone(),
        kind: "authorize_dispatch".into(),
        request_digest: dispatch_digest,
        expected_revision: Some(request.prepared_revision.clone()),
        profile_digest: locator.config_digest.clone(),
    };
    let dispatch_revision = match campaign.reconcile_operation(&dispatch_locator) {
        Reconcile::Committed(receipt) => receipt.result_revision.as_str().to_string(),
        other => panic!("dispatch receipt after crash: {other:?}"),
    };
    let mut result: Value = serde_json::from_str(include_str!(
        "../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json"
    ))
    .unwrap();
    result["subject"] = json!({"commit":request.candidate.commit,"tree":request.candidate.tree,
        "patchIdentity":request.candidate.patch_identity});
    let result = parse_episode_json(&result.to_string()).unwrap();
    let result_bytes = episode_json(&result).into_bytes();
    let count = root.join("tee-invocations");
    assert_eq!(fs::read(&count).unwrap(), result_bytes);
    let executable = fs::canonicalize("/usr/bin/tee").unwrap();
    let scope = ControlledScope {
        root_id: "process-root".into(),
        provenance: ExecutionProvenance {
            binding: ExecutionBinding {
                campaign_root_id: campaign.root_id().into(),
                obligation_id: "review-0".into(),
                candidate_digest: request.candidate.receipt_sha256.clone(),
                selection_digest: request.selection.campaign_digest.clone(),
                profile_digest: request.profile_digest.clone(),
                prepared_request_sha256: request.request_digest.clone(),
                review_episode_id: dispatch.command.episode_id.clone(),
                attempt_id: request.identity.attempt_id.clone(),
            },
            dispatch_operation_id: dispatch.operation_id.clone(),
            dispatch_revision,
            execution_session_id: Some("controlled-session-join".into()),
        },
        selection_id: request.selection.selection_id.clone(),
        candidate_commit: request.candidate.commit.clone(),
        candidate_tree: request.candidate.tree.clone(),
        candidate_patch_identity: request.candidate.patch_identity.clone(),
        request_bytes: result_bytes.clone(),
        executable_sha256: sha256_bytes(&fs::read(&executable).unwrap()),
        executable,
        arguments: vec!["-a".into(), count.to_string_lossy().into()],
        workspace: root.canonicalize().unwrap(),
        source_sha256: "e".repeat(64),
    };
    let pin = scope.reader_pin().unwrap();
    let evidence_root = root.join("evidence-root");
    let mut writer = EvidenceWriter::resume(&evidence_root, scope).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    assert!(matches!(
        runtime.block_on(writer.execute()).unwrap(),
        ExecutionPublication::Replayed(_)
    ));
    assert_eq!(fs::read(&count).unwrap(), result_bytes);
    drop(writer);
    let reader: Arc<dyn ExecutionEvidenceOwner> =
        Arc::new(EvidenceReader::open(&evidence_root, pin).unwrap());
    let config_path = root.join("claims-config.json");
    let source_path = root.join("claims-source.json");
    let config_bytes = fs::read(&config_path).unwrap();
    let source_bytes = fs::read(&source_path).unwrap();
    let config_json: Value = serde_json::from_slice(&config_bytes).unwrap();
    let grant_sha = claim_digest(&claim_value(&config_json["grants"][0])).unwrap();
    let claims_root = RootIdentity {
        absolute_path: root.join("claims-root"),
        root_id: "claims-root".into(),
        profile: "native-review-claims-v1".into(),
    };
    let files = OpenFiles {
        root: claims_root.absolute_path.clone(),
        config: config_path,
        source: source_path,
        expected_config_sha256: sha256_bytes(&config_bytes),
        expected_source_sha256: sha256_bytes(&source_bytes),
    };
    let native = read_native_selection(&root.join("native-episode")).unwrap();
    let episode_authority = authority(
        &identity,
        "review-0",
        &dispatch.command.episode_id,
        &request.candidate,
    );
    let selection = InitialJoinSelection {
        claims_root,
        production_grant_id: "grant-production".into(),
        production_grant_sha256: grant_sha,
        episode_selection: native.clone(),
        episode_authority: episode_authority.clone(),
        evidence_profile: EvidenceReaderProfile::ControlledProcess,
        evidence_owner: FIXTURE_OWNER.into(),
        evidence_root_id: "process-root".into(),
        evidence_record_profile: "controlled-process-v1".into(),
    };
    let claims = campaign
        .open_initial_claims(files, selection, reader)
        .unwrap();
    let intent = state
        .progress
        .attempts
        .iter()
        .find(|a| a.attempt_id == locator.attempt_id)
        .and_then(|a| a.completion_intent.as_ref())
        .unwrap();
    let progress = slice_campaign::InitialCompletionProgress {
        locator,
        last_checked_revision: state.revision,
        stages: intent.stages.clone(),
        newly_observed_receipts: vec![],
        state: InitialProgressState::Pending {
            stage: slice_campaign::StageKind::EpisodeResultRead,
            reason: "reopen".into(),
        },
    };
    (
        support::CorpusRoot::Fixed(root.to_path_buf()),
        campaign,
        identity,
        claims,
        progress,
        result,
        episode_authority,
        native,
        Some(count),
    )
}

type Pair = (
    support::CorpusRoot,
    slice_campaign::Campaign,
    slice_campaign::CampaignIdentity,
    slice_campaign::InitialClaimsOwner,
    slice_campaign::InitialCompletionProgress,
    EpisodeValue,
    Authority,
    review_episode_store::NativeRootSelection,
    Option<std::path::PathBuf>,
);

fn setup_pair(process: bool) -> Pair {
    setup_pair_with(process, false, false)
}
fn setup_pair_with(process: bool, surrogate: bool, tamper: bool) -> Pair {
    setup_pair_custom(process, surrogate, tamper, None)
}
fn setup_pair_custom(
    process: bool,
    surrogate: bool,
    tamper: bool,
    finding_count: Option<usize>,
) -> Pair {
    setup_pair_claims(process, surrogate, tamper, finding_count, None, false, None)
}
fn setup_pair_claims(
    process: bool,
    surrogate: bool,
    tamper: bool,
    finding_count: Option<usize>,
    claim_mode: Option<&str>,
    remediation_v4: bool,
    additional_fixture: Option<Arc<MutableFixtureOwner>>,
) -> Pair {
    let (temp, mut campaign, identity, mut selected, _) = support::selected_campaign_with_profile(
        if claim_mode.is_some() { 0 } else { 1 },
        true,
        remediation_v4,
    );
    if let Some(mode) = claim_mode {
        let candidate = selected.candidate.as_ref().unwrap();
        let subject = json!({"commit":candidate.commit,"tree":candidate.tree,"patchIdentity":candidate.patch_identity});
        let claims: Vec<_> = ["builder_projection", "campaign_terminalization"]
            .iter()
            .map(|boundary| {
                let mut claim = support::claim(&identity, "review-0", &subject, boundary);
                if *boundary == mode {
                    claim["profile"]["requiredRealization"] =
                        json!("unobserved-fixture-realization");
                    let mut unsigned = claim.clone();
                    unsigned.as_object_mut().unwrap().remove("revision");
                    claim["revision"] = json!(format!(
                        "production-path-claim-revision-v1@{}",
                        claim_digest(&claim_value(&unsigned)).unwrap()
                    ));
                }
                claim
            })
            .collect();
        selected=support::applied(campaign.bind_selection(&identity,&selected.revision,"selection-many",json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"selection-many","subject":subject,"specialists":[{"obligationId":"review-0","skill":"implementation-review","selection":"selected","requiredClaims":claims}]})).unwrap());
    }
    let prepared = match campaign
        .prepare_initial(&identity, &selected.revision, "review-0", "prepare-join")
        .unwrap()
    {
        Preparation::Applied(value) => value,
        other => panic!("prepare: {other:?}"),
    };
    let episode_id = slice_campaign::contract::episode_digest(&json!({"identity":identity,
        "obligationId":"review-0"}))
    .unwrap()[..32]
        .to_string();
    let command = DispatchCommand {
        episode_id: episode_id.clone(),
        writer_actor: "native-review-host".into(),
        runtime_session: "session-join".into(),
        reviewer_profile: "direct-initial".into(),
        begin_transition_id: "begin-join".into(),
        result_transition_id: "result-join".into(),
    };
    let DispatchEffect::Applied {
        permit, receipt, ..
    } = campaign
        .authorize_dispatch(prepared.handle, "dispatch-join", command)
        .unwrap()
    else {
        panic!("dispatch")
    };
    campaign.consume_dispatch_permit(permit).unwrap();
    let request = campaign
        .read_prepared(&identity, "review-0")
        .unwrap()
        .unwrap();
    let mut result: Value = serde_json::from_str(include_str!(
        "../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json"
    ))
    .unwrap();
    result["subject"] = json!({"commit":request.candidate.commit,"tree":request.candidate.tree,
        "patchIdentity":request.candidate.patch_identity});
    if let Some(count) = finding_count
        && count > 0
    {
        let template: Value = serde_json::from_str(include_str!(
            "../../../../app-server/tests/fixtures/implementation-review/remediation-required.json"
        ))
        .unwrap();
        result["verdict"] = json!("remediation_required");
        result["findings"] = Value::Array(
            (0..count)
                .map(|i| {
                    let mut f = template["findings"][0].clone();
                    f["id"] = json!(format!("F-{i}"));
                    f
                })
                .collect(),
        );
    }
    let mut result = parse_episode_json(&result.to_string()).unwrap();
    if surrogate {
        let EpisodeValue::Object(fields) = &mut result else {
            panic!("result object")
        };
        fields.insert(
            EpisodeString::new("verdict"),
            EpisodeValue::text("incomplete"),
        );
        fields.insert(
            EpisodeString::new("limitations"),
            EpisodeValue::Array(vec![EpisodeValue::String(EpisodeString(vec![0xd800]))]),
        );
    }
    if surrogate
        && finding_count.is_some_and(|n| n > 0)
        && let EpisodeValue::Object(fields) = &mut result
        && let EpisodeValue::Array(findings) =
            fields.get_mut(&EpisodeString::new("findings")).unwrap()
        && let EpisodeValue::Object(f) = &mut findings[0]
    {
        f.insert(
            EpisodeString::new("id"),
            EpisodeValue::String(EpisodeString(vec![0xd800])),
        );
    }
    let attempt = request.identity.attempt_id.clone();
    let provenance = ExecutionProvenance {
        binding: ExecutionBinding {
            campaign_root_id: campaign.root_id().into(),
            obligation_id: "review-0".into(),
            candidate_digest: request.candidate.receipt_sha256.clone(),
            selection_digest: request.selection.campaign_digest.clone(),
            profile_digest: request.profile_digest.clone(),
            prepared_request_sha256: request.request_digest.clone(),
            review_episode_id: episode_id.clone(),
            attempt_id: attempt.clone(),
        },
        dispatch_operation_id: "dispatch-join".into(),
        dispatch_revision: receipt.result_revision.as_str().into(),
        execution_session_id: None,
    };
    let (evidence_ref, evidence, count_path, profile, root_id, record_profile): (
        _,
        Arc<dyn ExecutionEvidenceOwner>,
        _,
        _,
        _,
        _,
    ) = if process {
        let path = temp.path().join("tee-invocations");
        let executable = fs::canonicalize("/usr/bin/tee").unwrap();
        let bytes = episode_json(&result).into_bytes();
        let scope = ControlledScope {
            root_id: "process-root".into(),
            provenance: ExecutionProvenance {
                execution_session_id: Some("controlled-session-join".into()),
                ..provenance
            },
            selection_id: request.selection.selection_id.clone(),
            candidate_commit: request.candidate.commit.clone(),
            candidate_tree: request.candidate.tree.clone(),
            candidate_patch_identity: request.candidate.patch_identity.clone(),
            request_bytes: bytes.clone(),
            executable_sha256: sha256_bytes(&fs::read(&executable).unwrap()),
            executable,
            arguments: vec!["-a".into(), path.to_string_lossy().into()],
            workspace: temp.path().canonicalize().unwrap(),
            source_sha256: "e".repeat(64),
        };
        let pin = scope.reader_pin().unwrap();
        let root = temp.path().join("evidence-root");
        let mut writer = EvidenceWriter::create(&root, scope).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let reference = match runtime.block_on(writer.execute()).unwrap() {
            ExecutionPublication::Committed(v) => v,
            other => panic!("E1 publication: {other:?}"),
        };
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(
            runtime.block_on(writer.execute()).unwrap(),
            ExecutionPublication::Replayed(reference.clone())
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        drop(writer);
        let reader = EvidenceReader::open(&root, pin).unwrap();
        (
            reference,
            Arc::new(reader),
            Some(path),
            EvidenceReaderProfile::ControlledProcess,
            "process-root",
            "controlled-process-v1",
        )
    } else {
        let (provenance, observation, artifacts) = if finding_count.is_some() {
            let mut provenance = provenance;
            provenance.execution_session_id = Some("fixture-session".into());
            let mut observation = json!({"schema_version":2,"event_identity":"j2-fixture-event","kind":"production_path",
                "selection":{"id":request.selection.selection_id,"revision":request.selection.campaign_digest},
                "obligationId":"review-0","subject":{"candidate":{"commit":request.candidate.commit,"tree":request.candidate.tree,"patchIdentity":request.candidate.patch_identity},"reviewEpisodeId":episode_id},
                "coveredState":"initial review","execution":{"attemptId":attempt,"resultDigest":claim_digest(&parse_claim_json(&episode_json(&result)).unwrap()).unwrap()},
                "realization":{"requested":"claude-sonnet-5","observed":"claude-sonnet-5"},
                "capabilityEnvelope":{"capabilities":[],"mutationAuthorized":false},
                "continuity":{"mode":"fresh_initial","sessionId":"fixture-session"},
                "transport":{"mechanism":"direct","digest":"c".repeat(64)},"observer":{"identity":"host","kind":"trusted"},
                "observedAt":"2026-10-08T01:02:03.004Z","adapterVersion":"v1",
                "artifacts":[{"owner":FIXTURE_OWNER,"reference":"artifact:j2","digest":"a".repeat(64),"status":"verified"}]});
            if claim_mode == Some("both_false") {
                observation["capabilityEnvelope"]["mutationAuthorized"] = json!(true);
            }
            let digest = claim_digest(&claim_value(&observation)).unwrap();
            observation["id"] = json!(format!("production-path-observation-v1@{digest}"));
            (
                provenance,
                CheckedObservation::Present(claim_value(&observation)),
                vec!["artifact:j2".into()],
            )
        } else {
            (
                provenance,
                CheckedObservation::Absent {
                    reason: "controlled fixture has no observed process".into(),
                },
                vec![],
            )
        };
        if let CheckedObservation::Present(value) = &observation {
            fs::write(
                temp.path().join("j2-fixture-observation.json"),
                claim_evidence::codec::canonical_json(value).unwrap(),
            )
            .unwrap();
            fs::write(
                temp.path().join("j2-fixture-result.json"),
                episode_json(&result),
            )
            .unwrap();
        }
        let record = FixtureRecord::controlled(
            "fixture-root",
            "record-join",
            "r1",
            episode_json(&result).into_bytes(),
            provenance,
            observation,
            artifacts,
        )
        .unwrap();
        let reference = record.reference.clone();
        let fixture = ControlledFixtureOwner::new(vec![record.clone()]).unwrap();
        let owner: Arc<dyn ExecutionEvidenceOwner> = if tamper {
            Arc::new(CorruptAfterCustody {
                inner: fixture,
                database: campaign.anchored_root().join("slice-campaign.sqlite"),
                used: AtomicBool::new(false),
            })
        } else if let Some(shared) = additional_fixture {
            shared.records.lock().unwrap().push(record);
            shared
        } else {
            Arc::new(fixture)
        };
        (
            reference,
            owner,
            None,
            EvidenceReaderProfile::ControlledFixture,
            "fixture-root",
            FIXTURE_PROFILE,
        )
    };
    let (claims_files, claims_root, grant_sha) = claims_files(temp.path(), process);
    let native = init_native_root(&temp.path().join("native-episode")).unwrap();
    let episode_authority = authority(&identity, "review-0", &episode_id, &request.candidate);
    let selection = InitialJoinSelection {
        claims_root,
        production_grant_id: "grant-production".into(),
        production_grant_sha256: grant_sha,
        episode_selection: native.clone(),
        episode_authority: episode_authority.clone(),
        evidence_profile: profile,
        evidence_owner: FIXTURE_OWNER.into(),
        evidence_root_id: root_id.into(),
        evidence_record_profile: record_profile.into(),
    };
    let mut claims = campaign
        .initialize_initial_claims(claims_files, selection, evidence)
        .unwrap();
    if finding_count.is_some() {
        let config: Value =
            serde_json::from_slice(&fs::read(temp.path().join("claims-config.json")).unwrap())
                .unwrap();
        let grant = &config["grants"][1];
        let digest = claim_digest(&claim_value(grant)).unwrap();
        campaign
            .install_completion_selection(
                slice_campaign::CompletionSelection::new(
                    claims.claims().root().clone(),
                    "grant-findings".into(),
                    digest.clone(),
                    "grant-findings".into(),
                    digest,
                    "slice-builder:j2".into(),
                    request.candidate.tree.clone(),
                    "scope-1".into(),
                    vec![grant["authority_reference"].clone()],
                )
                .unwrap(),
            )
            .unwrap();
    }
    let current = campaign.read(&identity).unwrap().unwrap();
    let prepared = support::applied(
        campaign
            .prepare_initial_completion(
                &identity,
                &current.revision,
                "review-0",
                "complete-join",
                &evidence_ref,
            )
            .unwrap(),
    );
    let mut owners = InitialCompletionOwners::new(&mut claims, None);
    let progress = campaign
        .complete_initial(
            &identity,
            &prepared.snapshot.revision,
            "review-0",
            "complete-join",
            &mut owners,
        )
        .unwrap();
    (
        temp,
        campaign,
        identity,
        claims,
        progress,
        result,
        episode_authority,
        native,
        count_path,
    )
}

#[test]
fn controlled_fixture_drives_real_claim_pair_to_registered_episode_target() {
    let (_temp, _campaign, _identity, _claims, progress, _result, _authority, _native, _count) =
        setup_pair(false);
    assert!(
        matches!(
            progress.state,
            InitialProgressState::Pending {
                stage: slice_campaign::StageKind::EpisodeResultRead,
                ..
            }
        ),
        "{:?}",
        progress.state
    );
    assert_eq!(progress.stages.len(), 4);
    assert_eq!(
        progress
            .stages
            .iter()
            .filter(|s| matches!(s.progress, slice_campaign::StageProgress::Checked { .. }))
            .count(),
        2
    );
}

#[test]
fn concrete_claims_wrapper_cannot_be_installed_on_another_campaign() {
    let (_left_root, _left, _left_identity, mut left_claims, _left_progress, _, _, _, _) =
        setup_pair(false);
    let (_right_root, mut right, _right_identity, _right_claims, right_progress, _, _, _, _) =
        setup_pair(false);
    let before = right
        .read(&right_progress.locator.identity)
        .unwrap()
        .unwrap()
        .revision;
    let mut owners = InitialCompletionOwners::new(&mut left_claims, None);
    assert!(
        right
            .recover_initial_completion(&right_progress.locator, &mut owners)
            .is_err()
    );
    assert_eq!(
        right
            .read(&right_progress.locator.identity)
            .unwrap()
            .unwrap()
            .revision,
        before
    );
}

fn episode_admission(record: &Value, registration_operation: &str) -> Value {
    let claim = &record["claimRevisionRef"];
    let body = json!({"schemaVersion":1,"claimRevision":claim["revision"],
        "establishment":record["establishmentRef"],"boundary":record["boundary"],
        "consumer":record["consumer"]});
    let digest = claim_digest(&claim_value(&body)).unwrap();
    let _ = registration_operation;
    let consumption = json!({"owner":"slice-campaign",
        "reference":format!("claim-consumption:{}",claim["reference"].as_str().unwrap()),
        "revision":claim["revision"],"sha256":digest,
        "freshness":"exact immutable revision"});
    json!({"claimRevisionRef":record["claimRevisionRef"],
        "establishmentRef":record["establishmentRef"],
        "observationRef":record["observationRef"],"consumptionRef":consumption,
        "status":record["status"],"boundary":record["boundary"],
        "consumer":record["consumer"]})
}

fn finish_native(pair: Pair, process: bool, surrogate: bool) {
    finish_native_with(pair, process, surrogate, |_, _, _, _| {});
}
fn finish_native_with(
    pair: Pair,
    process: bool,
    surrogate: bool,
    after: impl FnOnce(
        &mut slice_campaign::Campaign,
        &slice_campaign::CampaignIdentity,
        &slice_campaign::InitialCompletionProgress,
        &mut InitialCompletionOwners<'_>,
    ),
) {
    finish_native_with_reader(
        pair,
        process,
        surrogate,
        None,
        false,
        |campaign, identity, joined, _, owners| after(campaign, identity, joined, owners),
    )
}
fn finish_native_with_reader(
    pair: Pair,
    process: bool,
    surrogate: bool,
    initial_outcome_before_close: Option<&str>,
    evaluate_initial_before_close: bool,
    after: impl FnOnce(
        &mut slice_campaign::Campaign,
        &slice_campaign::CampaignIdentity,
        &slice_campaign::InitialCompletionProgress,
        Option<slice_campaign::InitialOutcomeProgress>,
        &mut InitialCompletionOwners<'_>,
    ),
) {
    let (_temp, mut campaign, identity, mut claims, mut progress, result, authority, native, count) =
        pair;
    let result_bytes = episode_json(&result).into_bytes();
    if !progress
        .stages
        .iter()
        .any(|stage| stage.kind == slice_campaign::StageKind::EpisodeResultRead)
    {
        let mut owners = InitialCompletionOwners::new(&mut claims, None);
        progress = campaign
            .recover_initial_completion(&progress.locator, &mut owners)
            .unwrap();
    }
    let record = |kind| {
        progress
            .stages
            .iter()
            .find(|stage| stage.kind == kind)
            .and_then(|stage| match &stage.progress {
                slice_campaign::StageProgress::Checked { exact_refs, .. } => {
                    exact_refs.first().cloned()
                }
                _ => None,
            })
            .unwrap()
    };
    let admissions = json!([
        episode_admission(
            &record(slice_campaign::StageKind::AdmissionBuilder),
            "register-episode-join"
        ),
        episode_admission(
            &record(slice_campaign::StageKind::AdmissionCampaign),
            "register-episode-join"
        )
    ]);
    if process {
        assert!(
            admissions
                .as_array()
                .unwrap()
                .iter()
                .all(|a| a["status"] != "established")
        );
        assert_eq!(fs::read(count.as_ref().unwrap()).unwrap(), result_bytes);
    }
    let existing = progress
        .stages
        .iter()
        .find(|stage| stage.kind == slice_campaign::StageKind::EpisodeResultRead);
    let content_value = if let Some(stage) = existing {
        parse_episode_json(stage.command["contentCanonicalJson"].as_str().unwrap()).unwrap()
    } else {
        let questions = if surrogate {
            vec![EpisodeValue::String(EpisodeString(vec![0xd800]))]
        } else {
            vec![]
        };
        EpisodeValue::object([
            ("action", EpisodeValue::text("record_result")),
            (
                "payload",
                EpisodeValue::object([
                    ("result", result),
                    ("unresolvedQuestions", EpisodeValue::Array(questions)),
                    (
                        "evidenceAdmissions",
                        parse_episode_json(&admissions.to_string()).unwrap(),
                    ),
                ]),
            ),
        ])
    };
    if existing.is_none() {
        let registered = support::applied(
            campaign
                .register_initial_episode_read(
                    &identity,
                    &progress.last_checked_revision,
                    "complete-join",
                    "register-episode-join",
                    &content_value,
                )
                .unwrap(),
        );
        assert_eq!(
            registered.registration_receipt.kind,
            "register_initial_episode_read"
        );
        assert!(matches!(
            campaign
                .register_initial_episode_read(
                    &identity,
                    &progress.last_checked_revision,
                    "complete-join",
                    "register-episode-join",
                    &content_value
                )
                .unwrap(),
            slice_campaign::Effect::Replayed { .. }
        ));
        let begin = native_request(
            "begin",
            "begin-native",
            &native,
            EpisodeValue::object([
                ("authority", authority.value().clone()),
                ("transitionId", EpisodeValue::text("begin-join")),
                ("unresolvedQuestions", EpisodeValue::Array(vec![])),
            ]),
        );
        let first = write_native(Path::new(&native.root), &native, &begin, None);
        let first_revision = episode_field(&first, "observedRevision")
            .unwrap()
            .as_text()
            .unwrap()
            .to_string_checked()
            .unwrap();
        let payload = content_value.get("payload").unwrap().clone();
        let transition = native_request(
            "transition",
            "result-native",
            &native,
            EpisodeValue::object([
                ("authority", authority.value().clone()),
                ("expectedRevision", EpisodeValue::text(&first_revision)),
                ("transitionId", EpisodeValue::text("result-join")),
                ("action", EpisodeValue::text("record_result")),
                ("payload", payload),
            ]),
        );
        write_native(
            Path::new(&native.root),
            &native,
            &transition,
            Some(&first_revision),
        );
        #[cfg(feature = "test-faults")]
        test_fault_cut("j1_after_episode_commit");
    }
    let target = EpisodeReadTarget::Transition {
        transition_id: review_episode_core::codec::JsString::new("result-join"),
        content_digest: review_episode_core::identity::Revision(episode_digest(&content_value)),
    };
    let scope = TrustedEpisodeReadScope::new(authority, vec![target]).unwrap();
    let mut episode = EpisodeApplication::open_read_only(
        Path::new(&native.root),
        native.clone(),
        scope,
        StoreOptions::default(),
        DEFAULT_RESPONSE_LIMIT,
    )
    .unwrap();
    let mut owners = InitialCompletionOwners::new(&mut claims, Some(&mut episode));
    let joined = campaign
        .recover_initial_completion(&progress.locator, &mut owners)
        .unwrap();
    assert!(
        matches!(joined.state, InitialProgressState::OwnerReadsJoined),
        "{:?}",
        joined.state
    );
    assert_eq!(
        joined
            .stages
            .iter()
            .filter(|s| matches!(
                s.kind,
                slice_campaign::StageKind::Observation
                    | slice_campaign::StageKind::EstablishmentBuilder
                    | slice_campaign::StageKind::EstablishmentCampaign
                    | slice_campaign::StageKind::AdmissionBuilder
                    | slice_campaign::StageKind::AdmissionCampaign
                    | slice_campaign::StageKind::EpisodeResultRead
            ))
            .count(),
        if joined
            .stages
            .iter()
            .any(|s| s.kind == slice_campaign::StageKind::Observation)
        {
            6
        } else {
            5
        }
    );
    let replay = campaign
        .recover_initial_completion(&progress.locator, &mut owners)
        .unwrap();
    assert!(matches!(
        replay.state,
        InitialProgressState::OwnerReadsJoined
    ));
    if let Some(operation_id) = initial_outcome_before_close {
        let initial = campaign
            .finalize_initial_completion(
                &identity,
                &joined.last_checked_revision,
                &joined.locator,
                operation_id,
                &mut owners,
            )
            .unwrap();
        if evaluate_initial_before_close {
            let slice_campaign::InitialOutcomeState::Committed { view, .. } = &initial.state else {
                panic!("initial outcome must be committed")
            };
            let finding = &view.findings[0];
            let state = campaign.read(&identity).unwrap().unwrap();
            let prepared = support::applied(
                campaign
                    .prepare_builder_evaluation(
                        &identity,
                        &state.revision,
                        &joined.locator,
                        "initial-prior-evaluation",
                        slice_campaign::BuilderEvaluationDecision {
                            claim_id: claim_evidence::ClaimId::new(
                                finding["claimId"].as_str().unwrap(),
                            )
                            .unwrap(),
                            finding_revision: claim_evidence::FindingRevisionId::new(
                                finding["revision"].as_str().unwrap(),
                            )
                            .unwrap(),
                            valid: true,
                        },
                        &mut owners,
                    )
                    .unwrap(),
            );
            let evaluated = campaign
                .complete_builder_evaluation(
                    &identity,
                    &prepared.snapshot.revision,
                    &prepared.locator,
                    &mut owners,
                )
                .unwrap();
            assert!(matches!(
                evaluated.state,
                slice_campaign::BuilderEvaluationState::Committed { .. }
            ));
        }
        drop(episode);
        let mut owners = InitialCompletionOwners::new(&mut claims, None);
        after(
            &mut campaign,
            &identity,
            &joined,
            Some(initial),
            &mut owners,
        );
    } else {
        after(&mut campaign, &identity, &joined, None, &mut owners);
    }
    if process {
        assert_eq!(fs::read(count.as_ref().unwrap()).unwrap(), result_bytes);
    }
}

#[test]
fn exact_ce_pair_and_native_episode_historical_read_join() {
    finish_native(setup_pair(false), false, false);
}

#[test]
fn actual_controlled_child_e1_ce_episode_join_without_provider_attestation() {
    finish_native(setup_pair(true), true, false);
}

#[test]
fn native_surrogate_result_and_question_keep_exact_owner_codec() {
    finish_native(setup_pair_with(true, true, false), true, true);
}

#[test]
fn post_ce_commit_campaign_read_failure_retains_in_memory_receipt() {
    let (_temp, _campaign, _identity, _claims, progress, _result, _authority, _native, _count) =
        setup_pair_with(false, false, true);
    assert!(matches!(
        progress.state,
        InitialProgressState::Unresolved {
            stage: slice_campaign::StageKind::EstablishmentBuilder,
            ..
        }
    ));
    assert_eq!(progress.newly_observed_receipts.len(), 1);
    assert_eq!(progress.stages.len(), 1);
    assert!(matches!(
        progress.stages[0].progress,
        slice_campaign::StageProgress::Registered
    ));
}

#[test]
fn j2_actual_process_unestablished_and_lost_reply_locator() {
    finish_native_with(
        setup_pair(true),
        true,
        false,
        |campaign, identity, joined, owners| {
            let p = campaign
                .finalize_initial_completion(
                    identity,
                    &joined.last_checked_revision,
                    &joined.locator,
                    "outcome-j2",
                    owners,
                )
                .unwrap();
            let slice_campaign::InitialOutcomeState::Committed { view, receipt } = p.state else {
                panic!("{:?}", p.state)
            };
            assert_eq!(
                view.kind,
                slice_campaign::InitialOutcomeKind::EvidenceUnestablished
            );
            assert!(
                view.pair["builder"]["status"] == "false"
                    || view.pair["builder"]["status"] == "unestablished"
            );
            assert!(view.projection.is_none());
            assert!(
                view.consumptions
                    .iter()
                    .all(|c| c.consumed_at_operation_id.is_none())
            );
            let locator = campaign
                .locate_initial_outcome(identity, "review-0", "outcome-j2")
                .unwrap()
                .unwrap();
            let recovered = campaign.recover_initial_outcome(&locator, owners).unwrap();
            let slice_campaign::InitialOutcomeState::Committed {
                receipt: replay, ..
            } = recovered.state
            else {
                panic!("{:?}", recovered.state)
            };
            assert_eq!(replay.operation_id, receipt.operation_id);
            assert_eq!(
                campaign.read(identity).unwrap().unwrap().obligations[0].status,
                "failed"
            );
        },
    );
}

#[test]
fn j2_established_empty_incomplete_preserves_native_details() {
    finish_native_with(
        setup_pair_custom(false, true, false, Some(0)),
        false,
        true,
        |campaign, identity, joined, owners| {
            let p = campaign
                .finalize_initial_completion(
                    identity,
                    &joined.last_checked_revision,
                    &joined.locator,
                    "outcome-j2",
                    owners,
                )
                .unwrap();
            let slice_campaign::InitialOutcomeState::Committed { view, .. } = p.state else {
                panic!("{:?}", p.state)
            };
            assert_eq!(view.kind, slice_campaign::InitialOutcomeKind::Reported);
            assert_eq!(view.native_verdict, "incomplete");
            assert_eq!(view.native_phase, "remediation");
            assert!(view.limitations_canonical_json.contains("\\ud800"));
            assert!(view.unresolved_questions_canonical_json.contains("\\ud800"));
            assert_eq!(view.projection.as_ref().unwrap()["kind"], "no_findings");
            assert!(view.consumptions[0].consumed_at_operation_id.is_some());
            assert!(view.consumptions[1].consumed_at_operation_id.is_none());
            campaign.read(identity).unwrap().unwrap();
        },
    );
}

#[test]
fn schema_four_preserves_real_initial_owner_join_before_remediation() {
    finish_native_with(
        setup_pair_claims(false, false, false, Some(0), None, true, None),
        false,
        false,
        |campaign, identity, joined, owners| {
            let outcome = campaign
                .finalize_initial_completion(
                    identity,
                    &joined.last_checked_revision,
                    &joined.locator,
                    "initial-outcome-v4",
                    owners,
                )
                .unwrap();
            let slice_campaign::InitialOutcomeState::Committed { view, .. } = outcome.state else {
                panic!("{:?}", outcome.state)
            };
            assert_eq!(view.kind, slice_campaign::InitialOutcomeKind::Reported);
            let snapshot = campaign.read(identity).unwrap().unwrap();
            assert_eq!(snapshot.schema_version, 4);
            assert!(snapshot.remediation.is_none());
        },
    );
}

#[test]
fn schema_four_prepares_changed_tree_only_after_native_initial_outcome() {
    run_schema_four_remediation_flow(false, false, 0, 0);
}
#[test]
fn schema_four_established_empty_round_reports_fresh_projection() {
    run_schema_four_remediation_flow(true, false, 0, 0);
}
#[test]
fn schema_four_false_claim_is_distinct_from_unestablished_observation() {
    run_schema_four_remediation_flow(true, true, 0, 0);
}
#[test]
fn schema_four_revises_retained_findings_and_creates_new_claim() {
    run_schema_four_remediation_flow(true, false, 2, 3);
}
#[test]
fn schema_four_preserves_complete_100_and_refuses_101_before_ce_effects() {
    run_schema_four_remediation_flow(true, false, 0, 100);
}
#[test]
fn schema_four_reported_retained_findings_cannot_enter_generation_two() {
    run_schema_four_remediation_flow_case(true, false, 2, 2, Sc3Case::ResolvedRetained);
}
#[test]
fn schema_four_awaiting_builder_can_enter_generation_two() {
    run_schema_four_remediation_flow_case(true, false, 2, 3, Sc3Case::AwaitingBuilderSecond);
}
#[test]
fn schema_four_finalized_replay_requires_exact_finding_admission() {
    run_schema_four_remediation_flow_case(true, false, 0, 1, Sc3Case::FindingAdmissionTamper);
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Sc3Case {
    Standard,
    ResolvedRetained,
    AwaitingBuilderSecond,
    FindingAdmissionTamper,
}

fn run_schema_four_remediation_flow(
    observed: bool,
    false_claim: bool,
    initial_findings: usize,
    next_findings: usize,
) {
    run_schema_four_remediation_flow_case(
        observed,
        false_claim,
        initial_findings,
        next_findings,
        Sc3Case::Standard,
    );
}

fn run_schema_four_remediation_flow_case(
    observed: bool,
    false_claim: bool,
    initial_findings: usize,
    next_findings: usize,
    case: Sc3Case,
) {
    assert!(!false_claim || observed);
    let established = observed && !false_claim;
    let fixture = Arc::new(MutableFixtureOwner::default());
    let pair = setup_pair_claims(
        false,
        false,
        false,
        Some(initial_findings),
        None,
        true,
        Some(Arc::clone(&fixture)),
    );
    let repo = pair.0.path().join("repo");
    let artifacts_dir = pair.0.path().join("remediation-artifacts");
    let output = Command::new("/usr/bin/python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/remediation_artifacts.py"))
        .arg(&repo)
        .arg(&artifacts_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let paths: Value = serde_json::from_slice(&output.stdout).unwrap();
    let raw = |key: &str| fs::read(paths[key].as_str().unwrap()).unwrap();
    let candidate_raw = raw("candidate");
    let profile_raw = raw("profile");
    let capture_raw = raw("capture");
    let manifest_raw = raw("manifest");
    let receipt_raw = raw("receipt");
    let candidate: Value = serde_json::from_slice(&candidate_raw).unwrap();
    let subject = json!({"commit":candidate["checkpoint_commit_oid"],
        "tree":candidate["checkpoint_tree_oid"],
        "patchIdentity":candidate["task_patch_digest"]});
    let identity = pair.2.clone();
    let claims_root = pair.3.claims().root().clone();
    let native = pair.7.clone();
    let authority = pair.6.clone();
    let claims_config: Value =
        serde_json::from_slice(&fs::read(pair.0.path().join("claims-config.json")).unwrap())
            .unwrap();
    let grant = &claims_config["grants"][1];
    let grant_digest = claim_digest(&claim_value(grant)).unwrap();
    finish_native_with_reader(
        pair,
        false,
        false,
        Some("outcome-remediation-base"),
        initial_findings > 0,
        |campaign, identity, _joined, initial, owners| {
            let initial = initial.unwrap();
            let slice_campaign::InitialOutcomeState::Committed { view, .. } = initial.state else {
                panic!("{:?}", initial.state)
            };
            assert_eq!(
                view.kind,
                if initial_findings == 0 {
                    slice_campaign::InitialOutcomeKind::Reported
                } else {
                    slice_campaign::InitialOutcomeKind::AwaitingBuilder
                }
            );
            let prior_finding_refs = view.findings.clone();
            if initial_findings > 0 {
                let retained = campaign.read(identity).unwrap().unwrap();
                assert_eq!(retained.progress.evaluations.len(), 1);
                assert_eq!(
                    retained.progress.evaluations[0].consumer_tree,
                    retained.candidate.as_ref().unwrap().tree
                );
            }
            let snapshot = campaign.read(identity).unwrap().unwrap();
            let (mut store, _) = EpisodeStore::open_native_read_only(
                Path::new(&native.root),
                StoreOptions::default(),
            )
            .unwrap();
            let episode_current = store
                .read(&authority.identity().key().0)
                .unwrap()
                .current
                .unwrap();
            drop(store);
            let target = EpisodeReadTarget::Revision(episode_current.revision().clone());
            let scope = TrustedEpisodeReadScope::new(authority.clone(), vec![target]).unwrap();
            let mut reader = EpisodeApplication::open_read_only(
                Path::new(&native.root),
                native.clone(),
                scope,
                StoreOptions::default(),
                DEFAULT_RESPONSE_LIMIT,
            )
            .unwrap();
            let read = reader
                .read_remediation_checked(
                    &review_episode::in_process_admission::EpisodeReadRequest::Revision {
                        identity: authority.identity().clone(),
                        revision: episode_current.revision().clone(),
                    },
                )
                .unwrap();
            let mut claims = vec![
                support::claim(identity, "review-0", &subject, "builder_projection"),
                support::claim(identity, "review-0", &subject, "campaign_terminalization"),
            ];
            for claim in &mut claims {
                claim["proposition"] = json!("retained remediation review");
                claim["coveredState"] = json!("remediation review");
                claim["profile"]["continuity"] = json!("retained");
                let id_body = json!({"proposition":claim["proposition"],
                "subject":claim["subject"],"coveredState":claim["coveredState"],
                "consumptionBoundary":claim["consumptionBoundary"],
                "consumer":claim["consumer"]});
                claim["claimId"] = json!(format!(
                    "production-path-claim-v1@{}",
                    claim_digest(&claim_value(&id_body)).unwrap()
                ));
                let mut unsigned = claim.clone();
                unsigned.as_object_mut().unwrap().remove("revision");
                claim["revision"] = json!(format!(
                    "production-path-claim-revision-v1@{}",
                    claim_digest(&claim_value(&unsigned)).unwrap()
                ));
            }
            let selection = json!({"schemaVersion":2,"owner":"slice-supervisor",
            "selectionId":"selection-remediation-1","subject":subject,
            "specialists":[{"obligationId":"review-0","skill":"implementation-review",
                "selection":"selected","requiredClaims":claims}]});
            let completion = slice_campaign::CompletionSelection::new(
                claims_root.clone(),
                "grant-findings".into(),
                grant_digest.clone(),
                "grant-findings".into(),
                grant_digest.clone(),
                "slice-builder:j2".into(),
                candidate["checkpoint_tree_oid"].as_str().unwrap().into(),
                "scope-1".into(),
                vec![grant["authority_reference"].clone()],
            )
            .unwrap();
            let attempt_id = format!("{}:remediation:generation-1", identity.key());
            let dispatch = slice_campaign::RemediationDispatchCommand {
                episode_id: authority
                    .identity()
                    .review_episode_id
                    .to_string_checked()
                    .unwrap(),
                writer_actor: "native-review-host".into(),
                runtime_session: "fixture-session".into(),
                reviewer_profile: "direct-retained".into(),
                execution_attempt_id: attempt_id,
                subject_transition_id_canonical_json: "\"remediation-subject-1\"".into(),
                result_transition_id_canonical_json: "\"remediation-result-1\"".into(),
            };
            let parts = slice_campaign::RemediationSelectionParts {
                candidate_raw: candidate_raw.clone(),
                profile_raw: profile_raw.clone(),
                gate_capture: capture_raw.clone(),
                gate_manifest: manifest_raw.clone(),
                gate_receipt: receipt_raw.clone(),
                selection,
                completion,
                session: slice_campaign::RemediationSessionRef {
                    owner: FIXTURE_OWNER.into(),
                    root_id: "fixture-root".into(),
                    initial_attempt_id: identity.attempt_id.clone(),
                    session_id: "fixture-session".into(),
                    custody_reference: "fixture retained session".into(),
                    custody_sha256: "a".repeat(64),
                },
                dispatch_command: dispatch,
                acceptance_sources: vec![json!({"owner":"operator","reference":"accepted-sc3",
                    "revision":"v1","freshness":"exact_revision",
                    "integrity_sha256":"b".repeat(64),"status":"verified"})],
            };
            let mut wrong_gate = parts.clone();
            wrong_gate.gate_receipt = support::fixture(
                "c927b51b887e37f6fdc02e1fabfadddd211d0ac30712cbc6c5028993ad3889c2",
            );
            let bad = slice_campaign::RemediationSelection::new(wrong_gate).unwrap();
            assert!(
                campaign
                    .prepare_remediation(
                        identity,
                        &snapshot.revision,
                        "review-0",
                        "generation-1",
                        "prepare-wrong-gate",
                        &initial.locator,
                        bad,
                        &read
                    )
                    .is_err()
            );
            assert_eq!(
                campaign.read(identity).unwrap().unwrap().revision,
                snapshot.revision
            );
            let mut wrong_session = parts.clone();
            wrong_session.session.session_id = "wrong-session".into();
            let mut substituted_session = parts.clone();
            substituted_session.session.session_id = "substituted-session".into();
            substituted_session.dispatch_command.runtime_session = "substituted-session".into();
            let mut missing_custody = parts.clone();
            missing_custody.session.custody_reference.clear();
            assert!(slice_campaign::RemediationSelection::new(missing_custody).is_err());
            let mut wrong_selection = parts.clone();
            wrong_selection.selection["subject"]["tree"] = json!("0".repeat(40));
            let mut wrong_profile = parts.clone();
            wrong_profile.profile_raw = b"{}".to_vec();
            let mut wrong_candidate = parts.clone();
            wrong_candidate.candidate_raw = b"{}".to_vec();
            for (label, wrong) in [
                ("wrong-session", wrong_session),
                ("substituted-session", substituted_session),
                ("wrong-selection", wrong_selection),
                ("wrong-profile", wrong_profile),
                ("wrong-candidate", wrong_candidate),
            ]
            .into_iter()
            .filter(|(label, _)| case == Sc3Case::Standard || *label != "substituted-session")
            {
                assert!(
                    campaign
                        .prepare_remediation(
                            identity,
                            &snapshot.revision,
                            "review-0",
                            "generation-1",
                            label,
                            &initial.locator,
                            slice_campaign::RemediationSelection::new(wrong).unwrap(),
                            &read
                        )
                        .is_err()
                );
                assert_eq!(
                    campaign.read(identity).unwrap().unwrap().revision,
                    snapshot.revision
                );
            }
            let accepted = slice_campaign::RemediationSelection::new(parts.clone()).unwrap();
            let prepared = campaign
                .prepare_remediation(
                    identity,
                    &snapshot.revision,
                    "review-0",
                    "generation-1",
                    "prepare-remediation-1",
                    &initial.locator,
                    accepted,
                    &read,
                )
                .unwrap();
            let slice_campaign::RemediationPreparation::Applied { prepared, .. } = prepared else {
                panic!("{prepared:?}");
            };
            let mut conflicting = parts.clone();
            conflicting.session.custody_reference.push_str("-changed");
            assert!(
                campaign
                    .prepare_remediation(
                        identity,
                        &snapshot.revision,
                        "review-0",
                        "generation-1",
                        "prepare-remediation-1",
                        &initial.locator,
                        slice_campaign::RemediationSelection::new(conflicting).unwrap(),
                        &read
                    )
                    .is_err()
            );
            let state = campaign.read(identity).unwrap().unwrap();
            assert_eq!(state.remediation.as_ref().unwrap().current_generation, 1);
            assert_eq!(
                state.remediation.as_ref().unwrap().rounds[0]
                    .request
                    .candidate
                    .tree,
                candidate["checkpoint_tree_oid"].as_str().unwrap()
            );
            let recovered = campaign.recover_remediation(&prepared).unwrap();
            assert!(recovered.dispatch.is_none());
            assert!(recovered.outcome.is_none());
            let dispatch = state.remediation.as_ref().unwrap().rounds[0]
                .frozen_dispatch_command
                .clone();
            let applied = campaign
                .authorize_remediation_dispatch(
                    &prepared,
                    "dispatch-remediation-1",
                    dispatch.clone(),
                )
                .unwrap();
            let slice_campaign::RemediationDispatchEffect::Applied {
                permit,
                receipt: dispatch_receipt,
            } = applied
            else {
                panic!("{applied:?}");
            };
            let issued = campaign
                .consume_remediation_dispatch_permit(permit)
                .unwrap();
            assert_eq!(issued.command, dispatch);
            assert_eq!(
                issued.request.request_digest,
                prepared.request.request_digest
            );
            assert!(matches!(
                campaign
                    .authorize_remediation_dispatch(&prepared, "dispatch-remediation-1", dispatch)
                    .unwrap(),
                slice_campaign::RemediationDispatchEffect::Replayed { .. }
            ));
            let recovered = campaign.recover_remediation(&prepared).unwrap();
            assert_eq!(
                recovered.dispatch.unwrap().operation_id,
                "dispatch-remediation-1"
            );
            assert!(recovered.outcome.is_none());
            let mut subject_ref = authority.initial_subject().as_object().unwrap().clone();
            subject_ref.insert(
                EpisodeString::new("revision"),
                EpisodeValue::text(candidate["checkpoint_commit_oid"].as_str().unwrap()),
            );
            subject_ref.insert(
                EpisodeString::new("sha256"),
                EpisodeValue::text(&episode_digest(
                    &parse_episode_json(&subject.to_string()).unwrap(),
                )),
            );
            let subject_ref = EpisodeValue::Object(subject_ref);
            let subject_payload = EpisodeValue::object([("subject", subject_ref.clone())]);
            let subject_content = EpisodeValue::object([
                ("action", EpisodeValue::text("record_remediation_subject")),
                ("payload", subject_payload.clone()),
            ]);
            let subject_request = native_request(
                "transition",
                "remediation-subject-native",
                &native,
                EpisodeValue::object([
                    ("authority", authority.value().clone()),
                    ("expectedRevision", episode_current.revision().value()),
                    ("transitionId", EpisodeValue::text("remediation-subject-1")),
                    ("action", EpisodeValue::text("record_remediation_subject")),
                    ("payload", subject_payload),
                ]),
            );
            let subject_reply = write_native(
                Path::new(&native.root),
                &native,
                &subject_request,
                Some(&episode_current.revision().0),
            );
            let subject_revision = review_episode_core::identity::Revision::parse(
                episode_field(&subject_reply, "observedRevision").unwrap(),
                "subject revision",
            )
            .unwrap();
            let scope = TrustedEpisodeReadScope::new(
                authority.clone(),
                vec![EpisodeReadTarget::Transition {
                    transition_id: EpisodeString::new("remediation-subject-1"),
                    content_digest: review_episode_core::identity::Revision(episode_digest(
                        &subject_content,
                    )),
                }],
            )
            .unwrap();
            let mut post = EpisodeApplication::open_read_only(
                Path::new(&native.root),
                native.clone(),
                scope,
                StoreOptions::default(),
                DEFAULT_RESPONSE_LIMIT,
            )
            .unwrap();
            let readback = post
                .read_remediation_checked(
                    &review_episode::in_process_admission::EpisodeReadRequest::Transition {
                        identity: authority.identity().clone(),
                        transition_id: EpisodeString::new("remediation-subject-1"),
                        content_digest: review_episode_core::identity::Revision(episode_digest(
                            &subject_content,
                        )),
                    },
                )
                .unwrap();
            assert_eq!(readback.resolved_revision(), Some(&subject_revision));
            assert_eq!(readback.state().unwrap().get("subject"), &subject_ref);
            drop(post);
            drop(reader);
            let state = campaign.read(identity).unwrap().unwrap();
            assert!(
                campaign
                    .confirm_remediation_subject_read(
                        &prepared,
                        &prepared.request.prepared_revision,
                        "subject-read-stale",
                        &readback
                    )
                    .is_err()
            );
            assert!(matches!(
                campaign
                    .confirm_remediation_subject_read(
                        &prepared,
                        &state.revision,
                        "subject-read-remediation-1",
                        &readback
                    )
                    .unwrap(),
                slice_campaign::Effect::Applied { .. }
            ));
            assert_eq!(
                authority.initial_subject().get("revision").unwrap(),
                &EpisodeValue::text(&snapshot.candidate.as_ref().unwrap().commit)
            );
            let mut next_result: Value = if initial_findings > 0 {
                serde_json::from_str(&episode_json(
                    &read
                        .state()
                        .unwrap()
                        .current_result()
                        .unwrap()
                        .unwrap()
                        .value,
                ))
                .unwrap()
            } else {
                serde_json::from_str(if next_findings == 0 {
            include_str!("../../../../app-server/tests/fixtures/implementation-review/acceptable-as-is.json")
        } else {
            include_str!("../../../../app-server/tests/fixtures/implementation-review/remediation-required.json")
        }).unwrap()
            };
            next_result["subject"] = subject.clone();
            if !observed && next_findings == 0 {
                next_result["verdict"] = json!("incomplete");
                next_result["limitations"] = json!(["fresh review evidence not established"]);
            }
            if initial_findings > 0 {
                let mut findings = next_result["findings"].as_array().unwrap().clone();
                for index in initial_findings..next_findings {
                    let mut finding = findings[0].clone();
                    finding["id"] = json!(format!("SC3-new-{index}"));
                    findings.push(finding);
                }
                next_result["findings"] = json!(findings);
            } else if next_findings > 0 {
                let prototype = next_result["findings"][0].clone();
                next_result["findings"] = json!(
                    (0..next_findings)
                        .map(|index| {
                            let mut finding = prototype.clone();
                            finding["id"] = json!(format!("SC3-{index:03}"));
                            finding
                        })
                        .collect::<Vec<_>>()
                );
            }
            if case == Sc3Case::ResolvedRetained {
                next_result["verdict"] = json!("acceptable_as_is");
                next_result["decisiveEvidence"] = next_result["findings"][0]["evidence"].clone();
                for finding in next_result["findings"].as_array_mut().unwrap() {
                    finding["status"] = json!("verified_resolved");
                    finding["remediationEvidence"] = finding["evidence"].clone();
                }
            }
            let mut native_result = parse_episode_json(&next_result.to_string()).unwrap();
            if observed && initial_findings > 0 && case != Sc3Case::ResolvedRetained {
                let EpisodeValue::Object(fields) = &mut native_result else {
                    panic!("result")
                };
                let EpisodeValue::Array(findings) =
                    fields.get_mut(&EpisodeString::new("findings")).unwrap()
                else {
                    panic!("findings")
                };
                let EpisodeValue::Object(added) = findings.last_mut().unwrap() else {
                    panic!("added finding")
                };
                added.insert(
                    EpisodeString::new("id"),
                    EpisodeValue::String(EpisodeString(vec![0xd800])),
                );
            }
            let provenance = ExecutionProvenance {
                binding: ExecutionBinding {
                    campaign_root_id: campaign.root_id().into(),
                    obligation_id: "review-0".into(),
                    candidate_digest: prepared.request.candidate.receipt_sha256.clone(),
                    selection_digest: prepared.request.selection.campaign_digest.clone(),
                    profile_digest: prepared.request.profile_digest.clone(),
                    prepared_request_sha256: prepared.request.request_digest.clone(),
                    review_episode_id: issued.command.episode_id.clone(),
                    attempt_id: prepared.request.execution_attempt_id.clone(),
                },
                dispatch_operation_id: "dispatch-remediation-1".into(),
                dispatch_revision: dispatch_receipt.result_revision.as_str().into(),
                execution_session_id: Some("fixture-session".into()),
            };
            if initial_findings > 0 {
                for (label, rewrite) in [("dropped", false), ("rewritten", true)] {
                    let mut malformed = next_result.clone();
                    if rewrite {
                        malformed["findings"][0]["id"] = json!("rewritten-prior-ID");
                    } else {
                        malformed["findings"].as_array_mut().unwrap().remove(0);
                    }
                    let malformed = parse_episode_json(&malformed.to_string()).unwrap();
                    let record = FixtureRecord::controlled(
                        "fixture-root",
                        &format!("record-remediation-{label}"),
                        "r1",
                        episode_json(&malformed).into_bytes(),
                        provenance.clone(),
                        CheckedObservation::Absent {
                            reason: "preflight refusal fixture".into(),
                        },
                        vec![],
                    )
                    .unwrap();
                    let reference = record.reference.clone();
                    fixture.records.lock().unwrap().push(record);
                    let before = campaign.read(identity).unwrap().unwrap();
                    assert!(
                        campaign
                            .prepare_remediation_completion(
                                &prepared,
                                &before.revision,
                                &format!("complete-remediation-{label}"),
                                &reference
                            )
                            .is_err()
                    );
                    assert_eq!(
                        campaign.read(identity).unwrap().unwrap().revision,
                        before.revision
                    );
                }
            }
            let (observation, artifacts) = if observed {
                let mut observation = json!({"schema_version":2,
                "event_identity":"sc3-remediation-fixture-event","kind":"production_path",
                "selection":{"id":prepared.request.selection.selection_id,
                    "revision":prepared.request.selection.campaign_digest},
                "obligationId":"review-0",
                "subject":{"candidate":{"commit":prepared.request.candidate.commit,
                    "tree":prepared.request.candidate.tree,
                    "patchIdentity":prepared.request.candidate.patch_identity},
                    "reviewEpisodeId":issued.command.episode_id},
                "coveredState":"remediation review",
                "execution":{"attemptId":prepared.request.execution_attempt_id,
                    "resultDigest":claim_digest(&parse_claim_json(&episode_json(&native_result)).unwrap()).unwrap()},
                "realization":{"requested":"claude-sonnet-5","observed":"claude-sonnet-5"},
                "capabilityEnvelope":{"capabilities":[],"mutationAuthorized":false_claim},
                "continuity":{"mode":"same_session_resume","sessionId":"fixture-session"},
                "transport":{"mechanism":"direct","digest":"c".repeat(64)},
                "observer":{"identity":"host","kind":"trusted"},
                "observedAt":"2026-10-13T18:00:00.000Z","adapterVersion":"v1",
                "artifacts":[{"owner":FIXTURE_OWNER,"reference":"artifact:sc3",
                    "digest":"a".repeat(64),"status":"verified"}]});
                let digest = claim_digest(&claim_value(&observation)).unwrap();
                observation["id"] = json!(format!("production-path-observation-v1@{digest}"));
                (
                    CheckedObservation::Present(claim_value(&observation)),
                    vec!["artifact:sc3".into()],
                )
            } else {
                (
                    CheckedObservation::Absent {
                        reason: "bounded no-observation fixture".into(),
                    },
                    vec![],
                )
            };
            if next_findings == 100 {
                let mut oversized = next_result.clone();
                let mut extra = oversized["findings"][0].clone();
                extra["id"] = json!("SC3-100");
                oversized["findings"].as_array_mut().unwrap().push(extra);
                let oversized = parse_episode_json(&oversized.to_string()).unwrap();
                let record = FixtureRecord::controlled(
                    "fixture-root",
                    "record-remediation-101",
                    "r1",
                    episode_json(&oversized).into_bytes(),
                    provenance.clone(),
                    CheckedObservation::Absent {
                        reason: "oversized fixture has no observation".into(),
                    },
                    vec![],
                )
                .unwrap();
                let reference = record.reference.clone();
                fixture.records.lock().unwrap().push(record);
                let state = campaign.read(identity).unwrap().unwrap();
                assert!(
                    campaign
                        .prepare_remediation_completion(
                            &prepared,
                            &state.revision,
                            "complete-remediation-101",
                            &reference
                        )
                        .is_err()
                );
                assert!(
                    campaign
                        .read(identity)
                        .unwrap()
                        .unwrap()
                        .remediation
                        .as_ref()
                        .unwrap()
                        .rounds[0]
                        .completion
                        .is_none()
                );
            }
            let fixture_record = FixtureRecord::controlled(
                "fixture-root",
                "record-remediation-1",
                "r1",
                episode_json(&native_result).into_bytes(),
                provenance,
                observation,
                artifacts,
            )
            .unwrap();
            let reference = fixture_record.reference.clone();
            fixture.records.lock().unwrap().push(fixture_record);
            let state = campaign.read(identity).unwrap().unwrap();
            let completion = campaign
                .prepare_remediation_completion(
                    &prepared,
                    &state.revision,
                    "complete-remediation-1",
                    &reference,
                )
                .unwrap();
            let slice_campaign::Effect::Applied {
                value: completion, ..
            } = completion
            else {
                panic!("{completion:?}")
            };
            assert!(matches!(
                campaign
                    .prepare_remediation_completion(
                        &prepared,
                        &state.revision,
                        "complete-remediation-1",
                        &reference
                    )
                    .unwrap(),
                slice_campaign::Effect::Replayed { .. }
            ));
            let mut conflicting_reference = reference.clone();
            conflicting_reference.record_id.push_str("-different");
            assert!(
                campaign
                    .prepare_remediation_completion(
                        &prepared,
                        &state.revision,
                        "complete-remediation-1",
                        &conflicting_reference
                    )
                    .is_err()
            );
            let state = campaign.read(identity).unwrap().unwrap();
            assert_eq!(
                state.remediation.as_ref().unwrap().rounds[0]
                    .completion
                    .as_ref()
                    .unwrap()
                    .execution_evidence_ref
                    .attempt_id,
                prepared.request.execution_attempt_id
            );
            if observed {
                assert!(matches!(
                    campaign
                        .register_remediation_stage(
                            &completion,
                            slice_campaign::StageKind::Observation,
                            owners
                        )
                        .unwrap(),
                    slice_campaign::Effect::Applied { .. }
                ));
                assert!(matches!(
                    campaign
                        .complete_remediation_stage(
                            &completion,
                            slice_campaign::StageKind::Observation,
                            owners
                        )
                        .unwrap(),
                    slice_campaign::RemediationStageOutcome::Ready { .. }
                ));
            }
            let stage = campaign
                .register_remediation_stage(
                    &completion,
                    slice_campaign::StageKind::EstablishmentBuilder,
                    owners,
                )
                .unwrap();
            assert!(matches!(stage, slice_campaign::Effect::Applied { .. }));
            let establishment_result = campaign
                .complete_remediation_stage(
                    &completion,
                    slice_campaign::StageKind::EstablishmentBuilder,
                    owners,
                )
                .unwrap();
            assert!(
                matches!(
                    establishment_result,
                    slice_campaign::RemediationStageOutcome::Ready {
                        progress: slice_campaign::StageProgress::OwnerCommitted { .. },
                        ..
                    }
                ),
                "{establishment_result:?}"
            );
            for (kind, checked) in [
                (slice_campaign::StageKind::EstablishmentCampaign, false),
                (slice_campaign::StageKind::AdmissionBuilder, true),
                (slice_campaign::StageKind::AdmissionCampaign, true),
            ] {
                assert!(matches!(
                    campaign
                        .register_remediation_stage(&completion, kind, owners)
                        .unwrap(),
                    slice_campaign::Effect::Applied { .. }
                ));
                let progress = campaign
                    .complete_remediation_stage(&completion, kind, owners)
                    .unwrap();
                assert!(
                    matches!(
                        progress,
                        slice_campaign::RemediationStageOutcome::Ready {
                            progress: slice_campaign::StageProgress::Checked { .. },
                            ..
                        }
                    ) == checked,
                    "{progress:?}"
                );
            }
            let state = campaign.read(identity).unwrap().unwrap();
            let stages = &state.remediation.as_ref().unwrap().rounds[0]
                .completion
                .as_ref()
                .unwrap()
                .stages;
            assert_eq!(stages.len(), if observed { 5 } else { 4 });
            assert_eq!(
                stages
                    .iter()
                    .filter(|stage| matches!(
                        stage.progress,
                        slice_campaign::StageProgress::Checked { .. }
                    ))
                    .count(),
                2
            );
            let record = |kind| {
                stages
                    .iter()
                    .find(|stage| stage.kind == kind)
                    .and_then(|stage| match &stage.progress {
                        slice_campaign::StageProgress::Checked { exact_refs, .. } => {
                            exact_refs.first().cloned()
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            let admissions = json!([
                episode_admission(
                    &record(slice_campaign::StageKind::AdmissionBuilder),
                    "register-remediation-result"
                ),
                episode_admission(
                    &record(slice_campaign::StageKind::AdmissionCampaign),
                    "register-remediation-result"
                ),
            ]);
            let payload = EpisodeValue::object([
                ("result", native_result),
                (
                    "unresolvedQuestions",
                    EpisodeValue::Array(if !observed && next_findings == 0 {
                        vec![EpisodeValue::text("confirm the changed candidate review")]
                    } else {
                        vec![]
                    }),
                ),
                (
                    "evidenceAdmissions",
                    parse_episode_json(&admissions.to_string()).unwrap(),
                ),
            ]);
            let content = EpisodeValue::object([
                ("action", EpisodeValue::text("record_result")),
                ("payload", payload.clone()),
            ]);
            assert!(matches!(
                campaign
                    .register_remediation_episode_read(
                        &completion,
                        &state.revision,
                        "register-remediation-result",
                        &content
                    )
                    .unwrap(),
                slice_campaign::Effect::Applied { .. }
            ));
            let result_request = native_request(
                "transition",
                "remediation-result-native",
                &native,
                EpisodeValue::object([
                    ("authority", authority.value().clone()),
                    ("expectedRevision", subject_revision.value()),
                    ("transitionId", EpisodeValue::text("remediation-result-1")),
                    ("action", EpisodeValue::text("record_result")),
                    ("payload", payload),
                ]),
            );
            let result_reply = write_native(
                Path::new(&native.root),
                &native,
                &result_request,
                Some(&subject_revision.0),
            );
            let result_revision = review_episode_core::identity::Revision::parse(
                episode_field(&result_reply, "observedRevision").unwrap(),
                "result revision",
            )
            .unwrap();
            let scope = TrustedEpisodeReadScope::new(
                authority.clone(),
                vec![EpisodeReadTarget::Transition {
                    transition_id: EpisodeString::new("remediation-result-1"),
                    content_digest: review_episode_core::identity::Revision(episode_digest(
                        &content,
                    )),
                }],
            )
            .unwrap();
            let mut result_reader = EpisodeApplication::open_read_only(
                Path::new(&native.root),
                native.clone(),
                scope,
                StoreOptions::default(),
                DEFAULT_RESPONSE_LIMIT,
            )
            .unwrap();
            let read = result_reader
                .read_remediation_checked(
                    &review_episode::in_process_admission::EpisodeReadRequest::Transition {
                        identity: authority.identity().clone(),
                        transition_id: EpisodeString::new("remediation-result-1"),
                        content_digest: review_episode_core::identity::Revision(episode_digest(
                            &content,
                        )),
                    },
                )
                .unwrap();
            assert_eq!(read.resolved_revision(), Some(&result_revision));
            assert_eq!(read.state().unwrap().get("subject"), &subject_ref);
            assert_eq!(
                read.state().unwrap().phase(),
                if established {
                    if next_findings == 0 || case == Sc3Case::ResolvedRetained {
                        review_episode_core::state::Phase::Reported
                    } else {
                        review_episode_core::state::Phase::Remediation
                    }
                } else {
                    review_episode_core::state::Phase::EvidenceUnestablished
                }
            );
            let joined = campaign
                .complete_remediation(&completion, &mut result_reader, owners)
                .unwrap();
            let slice_campaign::Effect::Applied { value: joined, .. } = joined else {
                panic!("{joined:?}")
            };
            assert_eq!(joined.episode["resolvedRevision"], result_revision.0);
            let status = if false_claim {
                "false"
            } else if observed {
                "established"
            } else {
                "unestablished"
            };
            assert_eq!(joined.builder["status"], status);
            assert_eq!(joined.campaign["status"], status);
            let saved = campaign.read(identity).unwrap().unwrap();
            assert!(
                saved.remediation.as_ref().unwrap().rounds[0]
                    .completion
                    .as_ref()
                    .unwrap()
                    .joined_readbacks
                    .is_some()
            );
            assert!(matches!(
                campaign
                    .prepare_remediation(
                        identity,
                        &saved.revision,
                        "review-0",
                        "generation-2",
                        "premature-generation-2",
                        &initial.locator,
                        slice_campaign::RemediationSelection::new(parts.clone()).unwrap(),
                        &read,
                    )
                    .unwrap(),
                slice_campaign::RemediationPreparation::NoEffect(_)
            ));
            let finalized = campaign
                .finalize_remediation(
                    &completion,
                    "outcome-remediation-1",
                    &mut result_reader,
                    owners,
                )
                .unwrap();
            let slice_campaign::Effect::Applied {
                value: finalized, ..
            } = finalized
            else {
                panic!("{finalized:?}")
            };
            assert_eq!(
                finalized["kind"],
                if !established {
                    "evidence_unestablished"
                } else if next_findings == 0 || case == Sc3Case::ResolvedRetained {
                    "reported"
                } else {
                    "awaiting_builder"
                }
            );
            assert_eq!(finalized["nativeFindingCount"], next_findings);
            assert_eq!(finalized["projection"].is_null(), !established);
            let recovered = campaign.recover_remediation(&prepared).unwrap();
            assert_eq!(
                recovered.dispatch.unwrap().operation_id,
                "dispatch-remediation-1"
            );
            assert_eq!(
                recovered.completion.unwrap().completion_operation_id,
                "complete-remediation-1"
            );
            assert!(recovered.subject_read.is_some());
            assert!(recovered.joined_readbacks.is_some());
            assert_eq!(recovered.outcome.as_ref().unwrap(), &finalized);
            let consumed = campaign.read(identity).unwrap().unwrap();
            let intents = &consumed.remediation.as_ref().unwrap().rounds[0]
                .completion
                .as_ref()
                .unwrap()
                .consumption_intents;
            assert_eq!(intents.len(), 2);
            assert_eq!(intents[0].consumed_at_operation_id.is_some(), established);
            assert!(intents[1].consumed_at_operation_id.is_none());
            if initial_findings > 0 && case != Sc3Case::ResolvedRetained {
                let new_refs = finalized["findings"].as_array().unwrap();
                assert_eq!(new_refs.len(), next_findings);
                for prior in &prior_finding_refs {
                    let next = new_refs
                        .iter()
                        .find(|next| {
                            next["nativeIdCanonicalJson"] == prior["nativeIdCanonicalJson"]
                        })
                        .unwrap();
                    assert_eq!(prior["claimId"], next["claimId"]);
                    assert_ne!(prior["revision"], next["revision"]);
                    assert_eq!(prior["initialRevisionRef"], next["initialRevisionRef"]);
                    assert_eq!(prior["revision"], next["predecessorRevision"]);
                }
                let added = new_refs
                    .iter()
                    .find(|next| {
                        prior_finding_refs.iter().all(|prior| {
                            prior["nativeIdCanonicalJson"] != next["nativeIdCanonicalJson"]
                        })
                    })
                    .unwrap();
                assert_eq!(added["predecessorRevision"], Value::Null);
                assert!(
                    added["nativeIdCanonicalJson"]
                        .as_str()
                        .unwrap()
                        .contains("\\ud800")
                );
                assert!(
                    prior_finding_refs
                        .iter()
                        .all(|prior| prior["claimId"] != added["claimId"])
                );
            }
            if case == Sc3Case::FindingAdmissionTamper {
                let finding_operation = campaign
                    .read(identity)
                    .unwrap()
                    .unwrap()
                    .remediation
                    .as_ref()
                    .unwrap()
                    .rounds[0]
                    .completion
                    .as_ref()
                    .unwrap()
                    .stages
                    .iter()
                    .find(|stage| stage.kind == slice_campaign::StageKind::Finding)
                    .unwrap()
                    .stage_id
                    .clone();
                let database = campaign
                    .anchored_root()
                    .parent()
                    .unwrap()
                    .join("claims-root/claim-evidence.sqlite3");
                let sql = rusqlite::Connection::open(&database).unwrap();
                let (admission, grant): (String, String) = sql
                    .query_row(
                        "SELECT admission_sha256,grant_sha256 FROM operation_admissions WHERE operation_id=?",
                        [&finding_operation],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .unwrap();
                let moved = database.with_extension("unavailable");
                for damage in ["missing", "conflicting", "unavailable"] {
                    if damage == "missing" {
                        sql.execute(
                            "DELETE FROM operation_admissions WHERE operation_id=?",
                            [&finding_operation],
                        )
                        .unwrap();
                    } else if damage == "conflicting" {
                        sql.execute(
                            "UPDATE operation_admissions SET admission_sha256=? WHERE operation_id=?",
                            rusqlite::params!["0".repeat(64), finding_operation],
                        )
                        .unwrap();
                    } else {
                        fs::rename(&database, &moved).unwrap();
                    }
                    let before = campaign.read(identity).unwrap().unwrap();
                    assert!(
                        campaign
                            .finalize_remediation(
                                &completion,
                                "outcome-remediation-1",
                                &mut result_reader,
                                owners,
                            )
                            .is_err(),
                        "finalized replay accepted {damage} CE finding admission"
                    );
                    assert_eq!(
                        campaign.read(identity).unwrap().unwrap().revision,
                        before.revision
                    );
                    if damage == "unavailable" {
                        fs::rename(&moved, &database).unwrap();
                    } else {
                        sql.execute(
                            "INSERT OR REPLACE INTO operation_admissions(operation_id,admission_sha256,grant_sha256) VALUES (?,?,?)",
                            rusqlite::params![finding_operation, admission, grant],
                        )
                        .unwrap();
                    }
                }
            }
            assert!(matches!(
                campaign
                    .finalize_remediation(
                        &completion,
                        "outcome-remediation-1",
                        &mut result_reader,
                        owners
                    )
                    .unwrap(),
                slice_campaign::Effect::Replayed { .. }
            ));
            if observed && initial_findings > 0 && case != Sc3Case::ResolvedRetained {
                let old = &prior_finding_refs[0];
                let before = campaign.read(identity).unwrap().unwrap();
                assert!(
                    campaign
                        .prepare_remediation_builder_evaluation(
                            &completion,
                            &before.revision,
                            "remediation-eval-stale",
                            slice_campaign::BuilderEvaluationDecision {
                                claim_id: claim_evidence::ClaimId::new(
                                    old["claimId"].as_str().unwrap()
                                )
                                .unwrap(),
                                finding_revision: claim_evidence::FindingRevisionId::new(
                                    old["revision"].as_str().unwrap()
                                )
                                .unwrap(),
                                valid: false,
                            },
                            owners
                        )
                        .is_err()
                );
                assert_eq!(
                    campaign.read(identity).unwrap().unwrap().revision,
                    before.revision
                );
                for (index, valid) in [(0usize, false), (1usize, true)] {
                    let finding = &finalized["findings"][index];
                    let decision = slice_campaign::BuilderEvaluationDecision {
                        claim_id: claim_evidence::ClaimId::new(
                            finding["claimId"].as_str().unwrap(),
                        )
                        .unwrap(),
                        finding_revision: claim_evidence::FindingRevisionId::new(
                            finding["revision"].as_str().unwrap(),
                        )
                        .unwrap(),
                        valid,
                    };
                    let state = campaign.read(identity).unwrap().unwrap();
                    let prepared = support::applied(
                        campaign
                            .prepare_remediation_builder_evaluation(
                                &completion,
                                &state.revision,
                                &format!("remediation-eval-{index}"),
                                decision,
                                owners,
                            )
                            .unwrap(),
                    );
                    let evaluated = support::applied(
                        campaign
                            .complete_remediation_builder_evaluation(
                                &prepared,
                                &mut result_reader,
                                owners,
                            )
                            .unwrap(),
                    );
                    assert_eq!(evaluated["valid"], valid);
                    assert!(!evaluated["reliance"].is_null());
                    assert!(!evaluated["projection"].is_null());
                    assert_eq!(evaluated["outcome"]["kind"], "awaiting_builder");
                }
                if case == Sc3Case::AwaitingBuilderSecond {
                    assert!(matches!(
                        campaign
                            .finalize_remediation(
                                &completion,
                                "outcome-remediation-1",
                                &mut result_reader,
                                owners,
                            )
                            .unwrap(),
                        slice_campaign::Effect::Replayed { .. }
                    ));
                    let database = campaign
                        .anchored_root()
                        .parent()
                        .unwrap()
                        .join("claims-root/claim-evidence.sqlite3");
                    let sql = rusqlite::Connection::open(database).unwrap();
                    let operation = "remediation-eval-0:reliance";
                    let (admission, grant): (String, String) = sql
                        .query_row(
                            "SELECT admission_sha256,grant_sha256 FROM operation_admissions WHERE operation_id=?",
                            [operation],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .unwrap();
                    sql.execute(
                        "DELETE FROM operation_admissions WHERE operation_id=?",
                        [operation],
                    )
                    .unwrap();
                    assert!(
                        campaign
                            .finalize_remediation(
                                &completion,
                                "outcome-remediation-1",
                                &mut result_reader,
                                owners,
                            )
                            .is_err()
                    );
                    sql.execute(
                        "INSERT INTO operation_admissions(operation_id,admission_sha256,grant_sha256) VALUES (?,?,?)",
                        rusqlite::params![operation, admission, grant],
                    )
                    .unwrap();
                }
            }
            if established
                && ((initial_findings == 0 && next_findings == 0)
                    || matches!(
                        case,
                        Sc3Case::ResolvedRetained | Sc3Case::AwaitingBuilderSecond
                    ))
            {
                let second_output = Command::new("/usr/bin/python3")
                    .arg(
                        Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("tests/support/remediation_artifacts.py"),
                    )
                    .arg(&repo)
                    .arg(artifacts_dir.join("generation-2"))
                    .arg(paths["candidate"].as_str().unwrap())
                    .output()
                    .unwrap();
                assert!(
                    second_output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&second_output.stderr)
                );
                let second_paths: Value = serde_json::from_slice(&second_output.stdout).unwrap();
                let read_raw = |key: &str| fs::read(second_paths[key].as_str().unwrap()).unwrap();
                let next_candidate_raw = read_raw("candidate");
                let next_candidate: Value = serde_json::from_slice(&next_candidate_raw).unwrap();
                let next_subject = json!({
                    "commit":next_candidate["checkpoint_commit_oid"],
                    "tree":next_candidate["checkpoint_tree_oid"],
                    "patchIdentity":next_candidate["task_patch_digest"],
                });
                let mut second = parts.clone();
                second.candidate_raw = next_candidate_raw;
                second.profile_raw = read_raw("profile");
                second.gate_capture = read_raw("capture");
                second.gate_manifest = read_raw("manifest");
                second.gate_receipt = read_raw("receipt");
                second.selection["selectionId"] = json!("selection-remediation-2");
                second.selection["subject"] = next_subject.clone();
                for claim in second.selection["specialists"][0]["requiredClaims"]
                    .as_array_mut()
                    .unwrap()
                {
                    claim["subject"]["candidate"] = next_subject.clone();
                    let body = json!({"proposition":claim["proposition"],
                    "subject":claim["subject"],"coveredState":claim["coveredState"],
                    "consumptionBoundary":claim["consumptionBoundary"],
                    "consumer":claim["consumer"]});
                    claim["claimId"] = json!(format!(
                        "production-path-claim-v1@{}",
                        claim_digest(&claim_value(&body)).unwrap()
                    ));
                    let mut unsigned = claim.clone();
                    unsigned.as_object_mut().unwrap().remove("revision");
                    claim["revision"] = json!(format!(
                        "production-path-claim-revision-v1@{}",
                        claim_digest(&claim_value(&unsigned)).unwrap()
                    ));
                }
                second.completion = slice_campaign::CompletionSelection::new(
                    claims_root.clone(),
                    "grant-findings".into(),
                    grant_digest.clone(),
                    "grant-findings".into(),
                    grant_digest.clone(),
                    "slice-builder:j2".into(),
                    next_candidate["checkpoint_tree_oid"]
                        .as_str()
                        .unwrap()
                        .into(),
                    "scope-1".into(),
                    vec![grant["authority_reference"].clone()],
                )
                .unwrap();
                second.dispatch_command.execution_attempt_id =
                    format!("{}:remediation:generation-2", identity.key());
                second.dispatch_command.subject_transition_id_canonical_json =
                    "\"remediation-subject-2\"".into();
                second.dispatch_command.result_transition_id_canonical_json =
                    "\"remediation-result-2\"".into();
                second.acceptance_sources[0]["revision"] = json!("v2");
                let state = campaign.read(identity).unwrap().unwrap();
                if case == Sc3Case::ResolvedRetained {
                    let refused = campaign
                        .prepare_remediation(
                            identity,
                            &state.revision,
                            "review-0",
                            "generation-2",
                            "prepare-remediation-2",
                            &initial.locator,
                            slice_campaign::RemediationSelection::new(second).unwrap(),
                            &read,
                        )
                        .unwrap();
                    assert!(
                        matches!(refused, slice_campaign::RemediationPreparation::NoEffect(_)),
                        "reported nonempty predecessor admitted generation two: {refused:?}"
                    );
                    assert_eq!(
                        campaign.read(identity).unwrap().unwrap().revision,
                        state.revision
                    );
                    return;
                }
                let mut substituted = second.clone();
                substituted.session.session_id = "later-substituted-session".into();
                substituted.dispatch_command.runtime_session = "later-substituted-session".into();
                assert!(
                    campaign
                        .prepare_remediation(
                            identity,
                            &state.revision,
                            "review-0",
                            "generation-2",
                            "prepare-later-substituted-session",
                            &initial.locator,
                            slice_campaign::RemediationSelection::new(substituted).unwrap(),
                            &read,
                        )
                        .is_err()
                );
                assert_eq!(
                    campaign.read(identity).unwrap().unwrap().revision,
                    state.revision
                );
                let next = campaign
                    .prepare_remediation(
                        identity,
                        &state.revision,
                        "review-0",
                        "generation-2",
                        "prepare-remediation-2",
                        &initial.locator,
                        slice_campaign::RemediationSelection::new(second).unwrap(),
                        &read,
                    )
                    .unwrap();
                let slice_campaign::RemediationPreparation::Applied { prepared: next, .. } = next
                else {
                    panic!("{next:?}")
                };
                assert_eq!(next.request.generation, 2);
                let state = campaign.read(identity).unwrap().unwrap();
                assert_eq!(state.remediation.as_ref().unwrap().rounds.len(), 2);
                assert!(
                    state.remediation.as_ref().unwrap().rounds[0]
                        .outcome
                        .is_some()
                );
                assert_ne!(
                    next.request.candidate.tree,
                    state.remediation.as_ref().unwrap().rounds[0]
                        .request
                        .candidate
                        .tree
                );
                // A successor owns the live CE admission slot. The older
                // publication stays recoverable as history, not as a fresh
                // finalized current-round replay.
                assert!(
                    campaign
                        .finalize_remediation(
                            &completion,
                            "outcome-remediation-1",
                            &mut result_reader,
                            owners,
                        )
                        .is_err()
                );
                assert_eq!(
                    campaign.recover_remediation(&prepared).unwrap().outcome,
                    Some(finalized.clone())
                );
            }
            if !observed && initial_findings == 0 && next_findings == 0 {
                let database = Path::new(campaign.anchored_root()).join("slice-campaign.sqlite");
                rusqlite::Connection::open(database)
                    .unwrap()
                    .execute(
                        "UPDATE operation_receipt SET result_json='{}' WHERE operation_id=?1",
                        ["outcome-remediation-1:publish"],
                    )
                    .unwrap();
                assert!(campaign.read(identity).is_err());
            }
        },
    );
    assert_eq!(identity.attempt_id, "sc0-attempt");
}

#[cfg(feature = "test-faults")]
#[test]
fn schema_four_fault_child_entry() {
    if std::env::var_os("SC3_FAULT_CHILD").is_some() {
        run_schema_four_remediation_flow(false, false, 0, 0);
    }
}

#[cfg(feature = "test-faults")]
#[test]
fn schema_four_prepare_dispatch_crash_cuts_preserve_slot_and_no_reentry() {
    use rusqlite::OpenFlags;
    for cut in [
        "before_commit_prepare_remediation",
        "after_commit_prepare_remediation",
        "before_commit_authorize_remediation_dispatch",
        "after_commit_authorize_remediation_dispatch",
        "after_commit_prepare_remediation_completion",
        "after_commit_record_remediation_stage",
        "after_commit_register_remediation_episode_read",
        "after_commit_commit_remediation_readbacks",
        "after_commit_register_remediation_outcome",
        "after_commit_publish_remediation_outcome",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("case");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "schema_four_fault_child_entry", "--nocapture"])
            .env("SC3_FAULT_CHILD", "1")
            .env("P1_CORPUS_ROOT", &root)
            .env("SLICE_CAMPAIGN_FAULT_CUT", cut)
            .env("SLICE_CAMPAIGN_FAULT_DIR", &root)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let ready = root.join(format!("{cut}.ready"));
        let deadline = Instant::now() + Duration::from_secs(180);
        while !ready.exists() {
            if let Some(status) = child.try_wait().unwrap() {
                let output = child.wait_with_output().unwrap();
                panic!(
                    "{cut}: {status}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            assert!(Instant::now() < deadline, "{cut} did not reach cut");
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let database = root.join("campaign/slice-campaign.sqlite");
        let connection =
            rusqlite::Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let state_raw: String = connection
            .query_row("SELECT state_json FROM campaign_state", [], |row| {
                row.get(0)
            })
            .unwrap();
        let state: slice_campaign::Snapshot = serde_json::from_str(&state_raw).unwrap();
        let mut statement = connection
            .prepare(
                "SELECT operation_id,active,dispatch_operation_id,may_have_entered \
             FROM request_slot ORDER BY operation_id",
            )
            .unwrap();
        let slots: Vec<(String, i64, Option<String>, i64)> = statement
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        drop(statement);
        drop(connection);
        let config = slice_campaign::TrustedConfig::remediation_v4(
            root.join("campaign"),
            root.join("repo"),
            root.join("workspace"),
            state.identity.clone(),
            state.accepted_boundary.clone(),
            state.baseline.clone(),
            "sc3-fixture-writer".into(),
            support::fixture("f52792e22d2a1b73e007bc7078607d6657989248a35de1371a1180d23101d206"),
            support::fixture("d155cc2ac1cf1ab26b0e186d6a6ac86f547b1a0b9c3ce74f0ba69eecd37a2f34"),
            support::fixture("c927b51b887e37f6fdc02e1fabfadddd211d0ac30712cbc6c5028993ad3889c2"),
        )
        .unwrap();
        let mut reopened = Campaign::open(config).unwrap();
        assert_eq!(
            reopened.read(&state.identity).unwrap().unwrap().revision,
            state.revision
        );
        let prepared = reopened
            .locate_remediation(
                &state.identity,
                "review-0",
                "generation-1",
                "prepare-remediation-1",
            )
            .unwrap();
        if cut == "before_commit_prepare_remediation" {
            assert!(prepared.is_none());
            assert!(state.remediation.is_none());
            assert_eq!(slots.len(), 1);
            assert_eq!(slots[0].1, 1);
        } else {
            let prepared = prepared.unwrap();
            let recovered = reopened.recover_remediation(&prepared).unwrap();
            assert_eq!(
                recovered.prepared.request.operation_id,
                "prepare-remediation-1"
            );
            let completion_committed = !matches!(
                cut,
                "after_commit_prepare_remediation"
                    | "before_commit_authorize_remediation_dispatch"
                    | "after_commit_authorize_remediation_dispatch"
            );
            assert_eq!(recovered.completion.is_some(), completion_committed);
            if let Some(completion) = &recovered.completion {
                assert_eq!(completion.completion_operation_id, "complete-remediation-1");
            }
            assert_eq!(
                recovered.joined_readbacks.is_some(),
                matches!(
                    cut,
                    "after_commit_commit_remediation_readbacks"
                        | "after_commit_register_remediation_outcome"
                        | "after_commit_publish_remediation_outcome"
                )
            );
            assert_eq!(
                recovered.outcome.is_some(),
                cut == "after_commit_publish_remediation_outcome"
            );
            assert_eq!(slots.len(), 2);
            assert_eq!(slots.iter().filter(|slot| slot.1 == 1).count(), 1);
            assert_eq!(
                slots
                    .iter()
                    .find(|slot| slot.0 == "prepare-remediation-1")
                    .unwrap()
                    .1,
                1
            );
            if cut == "after_commit_prepare_remediation"
                || cut == "before_commit_authorize_remediation_dispatch"
            {
                assert!(recovered.dispatch.is_none());
                assert!(
                    slots
                        .iter()
                        .all(|slot| slot.2.is_none() || slot.0 != "prepare-remediation-1")
                );
            } else {
                let dispatch = recovered.dispatch.unwrap();
                assert_eq!(dispatch.operation_id, "dispatch-remediation-1");
                let current = state.remediation.as_ref().unwrap().rounds.last().unwrap();
                let replay = reopened
                    .authorize_remediation_dispatch(
                        &prepared,
                        "dispatch-remediation-1",
                        current.frozen_dispatch_command.clone(),
                    )
                    .unwrap();
                assert!(matches!(
                    replay,
                    slice_campaign::RemediationDispatchEffect::Replayed { .. }
                ));
                let active = slots
                    .iter()
                    .find(|slot| slot.0 == "prepare-remediation-1")
                    .unwrap();
                assert_eq!(active.2.as_deref(), Some("dispatch-remediation-1"));
                assert_eq!(active.3, 1);
            }
        }
        eprintln!("SC3-D crash cut {cut}: original slot and entry custody recovered");
    }
}
#[test]
fn j2_exact_findings_and_both_builder_decisions() {
    for valid in [false, true] {
        finish_native_with(
            setup_pair_custom(false, true, false, Some(2)),
            false,
            true,
            |campaign, identity, joined, owners| {
                let p = campaign
                    .finalize_initial_completion(
                        identity,
                        &joined.last_checked_revision,
                        &joined.locator,
                        "outcome-j2",
                        owners,
                    )
                    .unwrap();
                let slice_campaign::InitialOutcomeState::Committed { view, .. } = p.state else {
                    panic!("{:?}", p.state)
                };
                assert_eq!(
                    view.kind,
                    slice_campaign::InitialOutcomeKind::AwaitingBuilder
                );
                assert_eq!(view.findings.len(), 2);
                assert!(
                    view.findings[0]["nativeIdCanonicalJson"]
                        .as_str()
                        .unwrap()
                        .contains("\\ud800")
                );
                for (index, f) in view.findings.iter().enumerate() {
                    let state = campaign.read(identity).unwrap().unwrap();
                    let decision = slice_campaign::BuilderEvaluationDecision {
                        claim_id: claim_evidence::ClaimId::new(f["claimId"].as_str().unwrap())
                            .unwrap(),
                        finding_revision: claim_evidence::FindingRevisionId::new(
                            f["revision"].as_str().unwrap(),
                        )
                        .unwrap(),
                        valid,
                    };
                    let prepared = support::applied(
                        campaign
                            .prepare_builder_evaluation(
                                identity,
                                &state.revision,
                                &joined.locator,
                                &format!("eval-{index}"),
                                decision.clone(),
                                owners,
                            )
                            .unwrap(),
                    );
                    let p = campaign
                        .complete_builder_evaluation(
                            identity,
                            &prepared.snapshot.revision,
                            &prepared.locator,
                            owners,
                        )
                        .unwrap();
                    let slice_campaign::BuilderEvaluationState::Committed { view, .. } = p.state
                    else {
                        panic!("{:?}", p.state)
                    };
                    assert_eq!(view.valid, valid);
                    assert!(!view.reliance.is_null());
                    if index == 1 {
                        assert_eq!(
                            view.outcome.kind,
                            if valid {
                                slice_campaign::InitialOutcomeKind::AwaitingBuilder
                            } else {
                                slice_campaign::InitialOutcomeKind::Reported
                            }
                        );
                    }
                    let located = campaign
                        .locate_builder_evaluation(identity, "review-0", &format!("eval-{index}"))
                        .unwrap()
                        .unwrap();
                    assert!(matches!(
                        campaign
                            .recover_builder_evaluation(&located, owners)
                            .unwrap()
                            .state,
                        slice_campaign::BuilderEvaluationState::Committed { .. }
                    ));
                    let mut opposite = decision;
                    opposite.valid = !valid;
                    assert!(
                        campaign
                            .prepare_builder_evaluation(
                                identity,
                                &prepared.snapshot.revision,
                                &joined.locator,
                                &format!("eval-{index}"),
                                opposite,
                                owners
                            )
                            .is_err()
                    );
                }
            },
        );
    }
}

#[cfg(feature = "test-faults")]
#[test]
fn j2_crash_child_entry() {
    if std::env::var_os("J2_CUT_CHILD").is_some() {
        finish_native_with(
            setup_pair(true),
            true,
            false,
            |campaign, identity, joined, owners| {
                let progress = campaign
                    .finalize_initial_completion(
                        identity,
                        &joined.last_checked_revision,
                        &joined.locator,
                        "outcome-j2",
                        owners,
                    )
                    .unwrap();
                assert!(
                    matches!(
                        progress.state,
                        slice_campaign::InitialOutcomeState::Committed { .. }
                    ),
                    "{:?}",
                    progress.state
                );
            },
        );
    }
}

#[test]
fn j2_committed_view_still_requires_readable_real_owners() {
    finish_native_with(
        setup_pair_custom(false, false, false, Some(1)),
        false,
        false,
        |campaign, identity, joined, owners| {
            let published = campaign
                .finalize_initial_completion(
                    identity,
                    &joined.last_checked_revision,
                    &joined.locator,
                    "outcome-j2",
                    owners,
                )
                .unwrap();
            assert!(matches!(
                published.state,
                slice_campaign::InitialOutcomeState::Committed { .. }
            ));
            let root = campaign
                .anchored_root()
                .parent()
                .unwrap()
                .join("claims-root");
            let original = root.join("claim-evidence.sqlite3");
            // The checked root must notice loss of its actual owner store even
            // when the campaign already retains a serialized publication view.
            let moved = root.join("claim-evidence.sqlite3.unavailable");
            fs::rename(&original, &moved).unwrap();
            let unavailable = campaign.recover_initial_outcome(&published.locator, owners);
            fs::rename(&moved, &original).unwrap();
            let unavailable = unavailable.unwrap();
            assert!(
                matches!(
                    unavailable.state,
                    slice_campaign::InitialOutcomeState::Unresolved { .. }
                ),
                "{:?}",
                unavailable.state
            );
            let recovered = campaign
                .recover_initial_outcome(&published.locator, owners)
                .unwrap();
            let slice_campaign::InitialOutcomeState::Committed { view, .. } = recovered.state
            else {
                panic!("{:?}", recovered.state)
            };
            assert_eq!(view.findings.len(), 1);
            assert_eq!(view.projection.unwrap()["kind"], "ce_projection");
        },
    );
}

#[test]
fn j2_disjoint_pending_evaluations_keep_exact_members_and_original_receipts() {
    finish_native_with(
        setup_pair_custom(false, true, false, Some(2)),
        false,
        true,
        |campaign, identity, joined, owners| {
            let published = campaign
                .finalize_initial_completion(
                    identity,
                    &joined.last_checked_revision,
                    &joined.locator,
                    "outcome-j2",
                    owners,
                )
                .unwrap();
            let slice_campaign::InitialOutcomeState::Committed {
                view,
                receipt: original,
            } = published.state
            else {
                panic!("{:?}", published.state)
            };
            let decisions: Vec<_> = view
                .findings
                .iter()
                .map(|f| slice_campaign::BuilderEvaluationDecision {
                    claim_id: claim_evidence::ClaimId::new(f["claimId"].as_str().unwrap()).unwrap(),
                    finding_revision: claim_evidence::FindingRevisionId::new(
                        f["revision"].as_str().unwrap(),
                    )
                    .unwrap(),
                    valid: false,
                })
                .collect();
            let before = campaign.read(identity).unwrap().unwrap();
            let mut stale = decisions[0].clone();
            stale.finding_revision = decisions[1].finding_revision.clone();
            assert!(
                campaign
                    .prepare_builder_evaluation(
                        identity,
                        &before.revision,
                        &joined.locator,
                        "eval-stale",
                        stale,
                        owners
                    )
                    .is_err()
            );
            assert_eq!(
                campaign.read(identity).unwrap().unwrap().revision,
                before.revision
            );
            assert!(
                campaign
                    .locate_builder_evaluation(identity, "review-0", "eval-stale")
                    .unwrap()
                    .is_none()
            );
            let mut prepared = Vec::new();
            for (index, decision) in decisions.iter().enumerate() {
                let current = campaign.read(identity).unwrap().unwrap();
                prepared.push(support::applied(
                    campaign
                        .prepare_builder_evaluation(
                            identity,
                            &current.revision,
                            &joined.locator,
                            &format!("eval-pending-{index}"),
                            decision.clone(),
                            owners,
                        )
                        .unwrap(),
                ));
            }
            let current = campaign.read(identity).unwrap().unwrap();
            assert!(
                campaign
                    .prepare_builder_evaluation(
                        identity,
                        &current.revision,
                        &joined.locator,
                        "eval-duplicate-revision",
                        decisions[0].clone(),
                        owners
                    )
                    .is_err()
            );
            for (number, registration) in prepared.iter().rev().enumerate() {
                let mut wrong = registration.locator.clone();
                wrong.registration_request_sha256 = "0".repeat(64);
                assert!(campaign.recover_builder_evaluation(&wrong, owners).is_err());
                let result = campaign
                    .recover_builder_evaluation(&registration.locator, owners)
                    .unwrap();
                let slice_campaign::BuilderEvaluationState::Committed { view, receipt } =
                    result.state
                else {
                    panic!("{:?}", result.state)
                };
                assert_eq!(
                    view.outcome.kind,
                    if number == 0 {
                        slice_campaign::InitialOutcomeKind::AwaitingBuilder
                    } else {
                        slice_campaign::InitialOutcomeKind::Reported
                    }
                );
                assert_eq!(
                    view.outcome.initial_publication_operation_id,
                    original.operation_id
                );
                assert_eq!(
                    receipt.operation_id,
                    format!("{}:publish", registration.locator.evaluation_operation_id)
                );
            }
            for registration in prepared {
                let replay = campaign
                    .recover_builder_evaluation(&registration.locator, owners)
                    .unwrap();
                assert!(matches!(
                    replay.state,
                    slice_campaign::BuilderEvaluationState::Committed { .. }
                ));
            }
            let recovered = campaign
                .recover_initial_outcome(&published.locator, owners)
                .unwrap();
            let slice_campaign::InitialOutcomeState::Committed { view, receipt } = recovered.state
            else {
                panic!("{:?}", recovered.state)
            };
            assert_eq!(view.kind, slice_campaign::InitialOutcomeKind::Reported);
            assert_eq!(receipt.operation_id, original.operation_id);
        },
    );
}

fn assert_aggregate_rechecks_evaluation_owner(damage: &str) {
    finish_native_with(
        setup_pair_custom(false, true, false, Some(2)),
        false,
        true,
        |campaign, identity, joined, owners| {
            let published = campaign
                .finalize_initial_completion(
                    identity,
                    &joined.last_checked_revision,
                    &joined.locator,
                    "outcome-j2",
                    owners,
                )
                .unwrap();
            let slice_campaign::InitialOutcomeState::Committed {
                view,
                receipt: original,
            } = published.state
            else {
                panic!("{:?}", published.state)
            };
            let mut prepared = vec![];
            for (index, finding) in view.findings.iter().enumerate() {
                let state = campaign.read(identity).unwrap().unwrap();
                prepared.push(support::applied(
                    campaign
                        .prepare_builder_evaluation(
                            identity,
                            &state.revision,
                            &joined.locator,
                            &format!("eval-r2-{index}"),
                            slice_campaign::BuilderEvaluationDecision {
                                claim_id: claim_evidence::ClaimId::new(
                                    finding["claimId"].as_str().unwrap(),
                                )
                                .unwrap(),
                                finding_revision: claim_evidence::FindingRevisionId::new(
                                    finding["revision"].as_str().unwrap(),
                                )
                                .unwrap(),
                                valid: false,
                            },
                            owners,
                        )
                        .unwrap(),
                ));
            }
            let first = campaign
                .recover_builder_evaluation(&prepared[0].locator, owners)
                .unwrap();
            let slice_campaign::BuilderEvaluationState::Committed {
                receipt: first_receipt,
                ..
            } = first.state
            else {
                panic!("{:?}", first.state)
            };
            let database = campaign
                .anchored_root()
                .parent()
                .unwrap()
                .join("claims-root/claim-evidence.sqlite3");
            let moved = database.with_extension("unavailable");
            let sql = rusqlite::Connection::open(&database).unwrap();
            let operation = "eval-r2-0:reliance";
            let (admission, grant): (String, String) = sql.query_row(
                "SELECT admission_sha256,grant_sha256 FROM operation_admissions WHERE operation_id=?",
                [operation], |r| Ok((r.get(0)?, r.get(1)?)),
            ).unwrap();
            let change_owner = |restore: bool| {
                if damage == "unavailable" {
                    if restore {
                        fs::rename(&moved, &database).unwrap();
                    } else {
                        fs::rename(&database, &moved).unwrap();
                    }
                } else if restore {
                    sql.execute("INSERT OR REPLACE INTO operation_admissions(operation_id,admission_sha256,grant_sha256) VALUES (?,?,?)",
                        rusqlite::params![operation, admission, grant]).unwrap();
                } else if damage == "missing" {
                    sql.execute(
                        "DELETE FROM operation_admissions WHERE operation_id=?",
                        [operation],
                    )
                    .unwrap();
                } else {
                    assert_ne!(admission, "0".repeat(64));
                    sql.execute(
                        "UPDATE operation_admissions SET admission_sha256=? WHERE operation_id=?",
                        rusqlite::params!["0".repeat(64), operation],
                    )
                    .unwrap();
                }
            };
            change_owner(false);
            let before = campaign.read(identity).unwrap().unwrap();
            let initial = campaign
                .recover_initial_outcome(&published.locator, owners)
                .unwrap();
            assert!(
                matches!(
                    initial.state,
                    slice_campaign::InitialOutcomeState::Conflicting { .. }
                        | slice_campaign::InitialOutcomeState::Unresolved { .. }
                        | slice_campaign::InitialOutcomeState::Pending { .. }
                ),
                "initial aggregate accepted {damage} reliance: {:?}",
                initial.state
            );
            if damage == "changed" {
                assert!(matches!(initial.state,
                    slice_campaign::InitialOutcomeState::Conflicting { ref reason, .. }
                    if reason.contains("committed reliance owner record differs")));
            }
            for registration in &prepared {
                let result = campaign
                    .recover_builder_evaluation(&registration.locator, owners)
                    .unwrap();
                assert!(
                    matches!(
                        result.state,
                        slice_campaign::BuilderEvaluationState::Conflicting { .. }
                            | slice_campaign::BuilderEvaluationState::Unresolved { .. }
                            | slice_campaign::BuilderEvaluationState::Pending { .. }
                    ),
                    "evaluation aggregate accepted {damage} prior reliance: {:?}",
                    result.state
                );
                assert!(result.newly_observed_receipts.is_empty());
            }
            assert_eq!(
                campaign.read(identity).unwrap().unwrap().revision,
                before.revision
            );
            change_owner(true);
            let second = campaign
                .recover_builder_evaluation(&prepared[1].locator, owners)
                .unwrap();
            let slice_campaign::BuilderEvaluationState::Committed {
                view,
                receipt: second_receipt,
            } = second.state
            else {
                panic!("{:?}", second.state)
            };
            assert_eq!(
                view.outcome.kind,
                slice_campaign::InitialOutcomeKind::Reported
            );

            // A committed disjoint evaluation must recheck the first one's
            // owner evidence too, not just its own still-intact reliance.
            change_owner(false);
            let initial = campaign
                .recover_initial_outcome(&published.locator, owners)
                .unwrap();
            assert!(
                !matches!(
                    initial.state,
                    slice_campaign::InitialOutcomeState::Committed { .. }
                ),
                "reported aggregate accepted {damage} prior reliance"
            );
            let second = campaign
                .recover_builder_evaluation(&prepared[1].locator, owners)
                .unwrap();
            assert!(
                !matches!(
                    second.state,
                    slice_campaign::BuilderEvaluationState::Committed { .. }
                ),
                "committed disjoint aggregate accepted {damage} prior reliance"
            );
            change_owner(true);
            for (registration, receipt) in prepared.iter().zip([first_receipt, second_receipt]) {
                let restored = campaign
                    .recover_builder_evaluation(&registration.locator, owners)
                    .unwrap();
                let slice_campaign::BuilderEvaluationState::Committed {
                    receipt: recovered, ..
                } = restored.state
                else {
                    panic!("{:?}", restored.state)
                };
                assert_eq!(
                    serde_json::to_value(recovered).unwrap(),
                    serde_json::to_value(receipt).unwrap()
                );
            }
            let restored = campaign
                .recover_initial_outcome(&published.locator, owners)
                .unwrap();
            let slice_campaign::InitialOutcomeState::Committed { view, receipt } = restored.state
            else {
                panic!("{:?}", restored.state)
            };
            assert_eq!(view.kind, slice_campaign::InitialOutcomeKind::Reported);
            assert_eq!(
                serde_json::to_value(receipt).unwrap(),
                serde_json::to_value(original).unwrap()
            );
            assert_eq!(sql.query_row("SELECT count(*) FROM operation_admissions WHERE operation_id LIKE 'eval-r2-%:reliance'",
                [], |r| r.get::<_, i64>(0)).unwrap(), 2);
        },
    );
}

#[test]
fn j2_r2_aggregate_rejects_changed_exact_reliance_admission() {
    assert_aggregate_rechecks_evaluation_owner("changed");
}
#[test]
fn j2_r2_aggregate_rejects_missing_reliance_admission() {
    assert_aggregate_rechecks_evaluation_owner("missing");
}
#[test]
fn j2_r2_aggregate_requires_readable_reliance_owner() {
    assert_aggregate_rechecks_evaluation_owner("unavailable");
}
#[cfg(feature = "test-faults")]
#[test]
fn j2_actual_process_crash_recovery_without_second_execution() {
    for cut in [
        "j2_after_outcome_registration",
        "j2_before_outcome_commit",
        "j2_after_outcome_commit",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("case");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "j2_crash_child_entry", "--nocapture"])
            .env("J2_CUT_CHILD", "1")
            .env("P1_CORPUS_ROOT", &root)
            .env("SLICE_CAMPAIGN_FAULT_CUT", cut)
            .env("SLICE_CAMPAIGN_FAULT_DIR", &root)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let ready = root.join(format!("{cut}.ready"));
        let deadline = Instant::now() + Duration::from_secs(180);
        while !ready.exists() {
            if let Some(status) = child.try_wait().unwrap() {
                let output = child.wait_with_output().unwrap();
                panic!(
                    "{cut}: {status}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            assert!(Instant::now() < deadline, "{cut}");
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        // Discard every returned locator. Reconstruct only from the original operation IDs.
        finish_native_with(
            reopen_process_pair(&root),
            true,
            false,
            |campaign, identity, _joined, owners| {
                let locator = campaign
                    .locate_initial_outcome(identity, "review-0", "outcome-j2")
                    .unwrap()
                    .unwrap();
                let p = campaign.recover_initial_outcome(&locator, owners).unwrap();
                let slice_campaign::InitialOutcomeState::Committed { view, receipt } = p.state
                else {
                    panic!("{cut}: {:?}", p.state)
                };
                assert_eq!(
                    view.kind,
                    slice_campaign::InitialOutcomeKind::EvidenceUnestablished
                );
                assert_eq!(receipt.operation_id, "outcome-j2:publish");
            },
        );
        eprintln!(
            "J2 actual-process crash ledger: {cut}: original publication recovered, no second execution"
        );
    }
}

#[cfg(feature = "test-faults")]
fn reopen_j2_fixture(root: &Path) -> Pair {
    let (repo, workspace, identity, baseline, boundary) = support::setup_paths(root);
    let campaign = Campaign::open(support::config(
        root.join("campaign"),
        repo,
        workspace,
        identity.clone(),
        baseline,
        boundary,
    ))
    .unwrap();
    let locator = campaign
        .locate_initial_completion(&identity, "review-0", "complete-join")
        .unwrap()
        .unwrap();
    let state = campaign.read(&identity).unwrap().unwrap();
    let request = state
        .obligations
        .iter()
        .find(|o| o.obligation_id == "review-0")
        .unwrap()
        .request
        .as_ref()
        .unwrap();
    let attempt = state
        .progress
        .attempts
        .iter()
        .find(|a| a.attempt_id == locator.attempt_id)
        .unwrap();
    let dispatch = attempt.dispatch.as_ref().unwrap();
    let digest=slice_campaign::codec::campaign_digest(&json!({"identity":identity,"obligationId":"review-0","preparationOperationId":request.operation_id,"requestDigest":request.request_digest,"expectedRevision":request.prepared_revision,"operationId":dispatch.operation_id,"command":dispatch.command})).unwrap();
    let dl = RecoveryLocator {
        root_id: campaign.root_id().into(),
        anchored_root: campaign.anchored_root().into(),
        identity: identity.clone(),
        operation_id: dispatch.operation_id.clone(),
        kind: "authorize_dispatch".into(),
        request_digest: digest,
        expected_revision: Some(request.prepared_revision.clone()),
        profile_digest: locator.config_digest.clone(),
    };
    let Reconcile::Committed(receipt) = campaign.reconcile_operation(&dl) else {
        panic!("dispatch absent")
    };
    let result =
        parse_episode_json(&fs::read_to_string(root.join("j2-fixture-result.json")).unwrap())
            .unwrap();
    let observation =
        parse_claim_json(&fs::read_to_string(root.join("j2-fixture-observation.json")).unwrap())
            .unwrap();
    let record = FixtureRecord::controlled(
        "fixture-root",
        "record-join",
        "r1",
        episode_json(&result).into_bytes(),
        ExecutionProvenance {
            binding: ExecutionBinding {
                campaign_root_id: campaign.root_id().into(),
                obligation_id: "review-0".into(),
                candidate_digest: request.candidate.receipt_sha256.clone(),
                selection_digest: request.selection.campaign_digest.clone(),
                profile_digest: request.profile_digest.clone(),
                prepared_request_sha256: request.request_digest.clone(),
                review_episode_id: dispatch.command.episode_id.clone(),
                attempt_id: locator.attempt_id.clone(),
            },
            dispatch_operation_id: dispatch.operation_id.clone(),
            dispatch_revision: receipt.result_revision.as_str().into(),
            execution_session_id: Some("fixture-session".into()),
        },
        CheckedObservation::Present(observation),
        vec!["artifact:j2".into()],
    )
    .unwrap();
    let reader: Arc<dyn ExecutionEvidenceOwner> =
        Arc::new(ControlledFixtureOwner::new(vec![record]).unwrap());
    let config_bytes = fs::read(root.join("claims-config.json")).unwrap();
    let source_bytes = fs::read(root.join("claims-source.json")).unwrap();
    let config: Value = serde_json::from_slice(&config_bytes).unwrap();
    let files = OpenFiles {
        root: root.join("claims-root"),
        config: root.join("claims-config.json"),
        source: root.join("claims-source.json"),
        expected_config_sha256: sha256_bytes(&config_bytes),
        expected_source_sha256: sha256_bytes(&source_bytes),
    };
    let native = read_native_selection(&root.join("native-episode")).unwrap();
    let authority = authority(
        &identity,
        "review-0",
        &dispatch.command.episode_id,
        &request.candidate,
    );
    let selection = InitialJoinSelection {
        claims_root: RootIdentity {
            absolute_path: files.root.clone(),
            root_id: "claims-root".into(),
            profile: "native-review-claims-v1".into(),
        },
        production_grant_id: "grant-production".into(),
        production_grant_sha256: claim_digest(&claim_value(&config["grants"][0])).unwrap(),
        episode_selection: native.clone(),
        episode_authority: authority.clone(),
        evidence_profile: EvidenceReaderProfile::ControlledFixture,
        evidence_owner: FIXTURE_OWNER.into(),
        evidence_root_id: "fixture-root".into(),
        evidence_record_profile: FIXTURE_PROFILE.into(),
    };
    let claims = campaign
        .open_initial_claims(files, selection, reader)
        .unwrap();
    let grant = &config["grants"][1];
    let grant_digest = claim_digest(&claim_value(grant)).unwrap();
    campaign
        .install_completion_selection(
            slice_campaign::CompletionSelection::new(
                claims.claims().root().clone(),
                "grant-findings".into(),
                grant_digest.clone(),
                "grant-findings".into(),
                grant_digest,
                "slice-builder:j2".into(),
                request.candidate.tree.clone(),
                "scope-1".into(),
                vec![grant["authority_reference"].clone()],
            )
            .unwrap(),
        )
        .unwrap();
    let progress = slice_campaign::InitialCompletionProgress {
        locator,
        last_checked_revision: state.revision.clone(),
        stages: attempt.completion_intent.as_ref().unwrap().stages.clone(),
        newly_observed_receipts: vec![],
        state: InitialProgressState::OwnerReadsJoined,
    };
    (
        support::CorpusRoot::Fixed(root.into()),
        campaign,
        identity,
        claims,
        progress,
        result,
        authority,
        native,
        None,
    )
}
#[cfg(feature = "test-faults")]
fn complete_fixture_j2(
    campaign: &mut Campaign,
    identity: &slice_campaign::CampaignIdentity,
    joined: &slice_campaign::InitialCompletionProgress,
    owners: &mut InitialCompletionOwners<'_>,
) {
    let p = campaign
        .finalize_initial_completion(
            identity,
            &joined.last_checked_revision,
            &joined.locator,
            "outcome-j2",
            owners,
        )
        .unwrap();
    let slice_campaign::InitialOutcomeState::Committed { view, .. } = p.state else {
        panic!("{:?}", p.state)
    };
    assert_eq!(view.findings.len(), 1);
    let f = &view.findings[0];
    let decision = slice_campaign::BuilderEvaluationDecision {
        claim_id: claim_evidence::ClaimId::new(f["claimId"].as_str().unwrap()).unwrap(),
        finding_revision: claim_evidence::FindingRevisionId::new(f["revision"].as_str().unwrap())
            .unwrap(),
        valid: false,
    };
    let current = campaign.read(identity).unwrap().unwrap();
    let effect = campaign
        .prepare_builder_evaluation(
            identity,
            &current.revision,
            &joined.locator,
            "eval-j2",
            decision,
            owners,
        )
        .unwrap();
    let prepared = match effect {
        slice_campaign::Effect::Applied { value, .. }
        | slice_campaign::Effect::Replayed { value, .. } => value,
        other => panic!("{other:?}"),
    };
    // Recovery uses the original ID lookup, deliberately discarding the returned locator.
    drop(prepared);
    let locator = campaign
        .locate_builder_evaluation(identity, "review-0", "eval-j2")
        .unwrap()
        .unwrap();
    let p = campaign
        .recover_builder_evaluation(&locator, owners)
        .unwrap();
    let slice_campaign::BuilderEvaluationState::Committed { view, receipt } = p.state else {
        panic!("{:?}", p.state)
    };
    assert_eq!(
        view.outcome.kind,
        slice_campaign::InitialOutcomeKind::Reported
    );
    assert_eq!(receipt.operation_id, "eval-j2:publish");
}
#[cfg(feature = "test-faults")]
#[test]
fn j2_fixture_crash_child_entry() {
    if std::env::var_os("J2_FIXTURE_CUT_CHILD").is_some() {
        finish_native_with(
            setup_pair_custom(false, true, false, Some(1)),
            false,
            true,
            complete_fixture_j2,
        );
    }
}
#[cfg(feature = "test-faults")]
#[test]
fn j2_fixture_finding_projection_evaluation_crash_recovery() {
    for cut in [
        "j2_after_outcome_registration",
        "j2_after_finding_commit",
        "j2_after_projection_read",
        "j2_before_outcome_commit",
        "j2_after_outcome_commit",
        "j2_after_evaluation_registration",
        "j2_after_reliance_commit",
        "j2_after_evaluation_projection_read",
        "j2_before_evaluation_commit",
        "j2_after_evaluation_commit",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("case");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "j2_fixture_crash_child_entry", "--nocapture"])
            .env("J2_FIXTURE_CUT_CHILD", "1")
            .env("P1_CORPUS_ROOT", &root)
            .env("SLICE_CAMPAIGN_FAULT_CUT", cut)
            .env("SLICE_CAMPAIGN_FAULT_DIR", &root)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let ready = root.join(format!("{cut}.ready"));
        let deadline = Instant::now() + Duration::from_secs(180);
        while !ready.exists() {
            if let Some(status) = child.try_wait().unwrap() {
                let output = child.wait_with_output().unwrap();
                panic!(
                    "{cut}: {status}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            assert!(Instant::now() < deadline, "{cut}");
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        finish_native_with(reopen_j2_fixture(&root), false, true, complete_fixture_j2);
        eprintln!("J2 fixture domain crash ledger: {cut}: original owner identities recovered");
    }
}

#[test]
fn j2_claim_pair_statuses_remain_distinct() {
    for (mode, expected) in [
        ("builder_projection", ["unestablished", "established"]),
        ("campaign_terminalization", ["established", "unestablished"]),
        ("both_false", ["false", "false"]),
    ] {
        finish_native_with(
            setup_pair_claims(false, false, false, Some(1), Some(mode), false, None),
            false,
            false,
            |campaign, identity, joined, owners| {
                let p = campaign
                    .finalize_initial_completion(
                        identity,
                        &joined.last_checked_revision,
                        &joined.locator,
                        "outcome-j2",
                        owners,
                    )
                    .unwrap();
                let slice_campaign::InitialOutcomeState::Committed { view, .. } = p.state else {
                    panic!("{:?}", p.state)
                };
                assert_eq!(
                    view.kind,
                    slice_campaign::InitialOutcomeKind::EvidenceUnestablished
                );
                assert_eq!(view.pair["builder"]["status"], expected[0]);
                assert_eq!(view.pair["campaign"]["status"], expected[1]);
                assert!(view.findings.is_empty());
                assert!(view.projection.is_none());
                assert!(
                    !p.stages
                        .iter()
                        .any(|s| s.kind == slice_campaign::StageKind::Finding)
                );
            },
        );
    }
}
#[test]
fn j2_complete_set_count_and_capacity_refuse_before_registration() {
    for count in [100, 101] {
        finish_native_with(
            setup_pair_custom(false, false, false, Some(count)),
            false,
            false,
            |campaign, identity, joined, owners| {
                let before = campaign.read(identity).unwrap().unwrap();
                let result = campaign.finalize_initial_completion(
                    identity,
                    &joined.last_checked_revision,
                    &joined.locator,
                    "outcome-j2",
                    owners,
                );
                match result {
                    Ok(p) if count == 100 => {
                        let slice_campaign::InitialOutcomeState::Committed { view, .. } = p.state
                        else {
                            panic!("{:?}", p.state)
                        };
                        assert_eq!(view.findings.len(), 100);
                        eprintln!("J2 count100: complete exact publication");
                    }
                    Err(slice_campaign::CampaignError::Contract(reason)) => {
                        assert!(reason.contains(if count == 101 {
                            "count exceeds 100"
                        } else {
                            "capacity"
                        }));
                        assert_eq!(
                            campaign.read(identity).unwrap().unwrap().revision,
                            before.revision
                        );
                        assert!(
                            campaign
                                .locate_initial_outcome(identity, "review-0", "outcome-j2")
                                .unwrap()
                                .is_none()
                        );
                        eprintln!("J2 count{count}: pre-effect refusal: {reason}");
                    }
                    other => panic!("count {count}: {other:?}"),
                }
            },
        );
    }
}
#[test]
fn j2_registration_and_historical_publication_corruption_refused() {
    finish_native_with(
        setup_pair_custom(false, false, false, Some(1)),
        false,
        false,
        |campaign, identity, joined, owners| {
            let p = campaign
                .finalize_initial_completion(
                    identity,
                    &joined.last_checked_revision,
                    &joined.locator,
                    "outcome-j2",
                    owners,
                )
                .unwrap();
            assert!(
                matches!(
                    p.state,
                    slice_campaign::InitialOutcomeState::Committed { .. }
                ),
                "{:?}",
                p.state
            );
            let sql =
                rusqlite::Connection::open(campaign.anchored_root().join("slice-campaign.sqlite"))
                    .unwrap();
            for operation in [
                "outcome-j2",
                "outcome-j2:publish",
                "outcome-j2:projection-register",
            ] {
                let original: String = sql
                    .query_row(
                        "SELECT request_digest FROM operation_receipt WHERE operation_id=?1",
                        [operation],
                        |r| r.get(0),
                    )
                    .unwrap();
                sql.execute(
                    "UPDATE operation_receipt SET request_digest=?1 WHERE operation_id=?2",
                    rusqlite::params!["0".repeat(64), operation],
                )
                .unwrap();
                assert!(campaign.read(identity).is_err());
                assert!(
                    campaign
                        .locate_initial_outcome(identity, "review-0", "outcome-j2")
                        .is_err()
                );
                sql.execute(
                    "UPDATE operation_receipt SET request_digest=?1 WHERE operation_id=?2",
                    rusqlite::params![original, operation],
                )
                .unwrap();
                campaign.read(identity).unwrap().unwrap();
            }
            let mut wrong = p.locator.clone();
            wrong.registration_request_sha256 = "0".repeat(64);
            assert!(campaign.recover_initial_outcome(&wrong, owners).is_err());
            let mut wrong = p.locator;
            wrong.joined.anchored_root = campaign.anchored_root().join("other");
            assert!(campaign.recover_initial_outcome(&wrong, owners).is_err());
        },
    );
}

#[cfg(feature = "test-faults")]
#[test]
fn j2_post_owner_failure_child() {
    let Ok(cut) = std::env::var("J2_READ_FAILURE_CHILD") else {
        return;
    };
    finish_native_with(
        setup_pair_custom(false, true, false, Some(1)),
        false,
        true,
        |campaign, identity, joined, owners| {
            let p = campaign
                .finalize_initial_completion(
                    identity,
                    &joined.last_checked_revision,
                    &joined.locator,
                    "outcome-j2",
                    owners,
                )
                .unwrap();
            let (receipts, locator) = if cut == "j2_after_finding_commit"
                || cut == "j2_after_projection_read"
            {
                assert!(
                    matches!(
                        p.state,
                        slice_campaign::InitialOutcomeState::Unresolved { .. }
                    ),
                    "{:?}",
                    p.state
                );
                (
                    p.newly_observed_receipts,
                    serde_json::to_value(p.locator).unwrap(),
                )
            } else {
                let slice_campaign::InitialOutcomeState::Committed { view, .. } = p.state else {
                    panic!("{:?}", p.state)
                };
                let f = &view.findings[0];
                let decision = slice_campaign::BuilderEvaluationDecision {
                    claim_id: claim_evidence::ClaimId::new(f["claimId"].as_str().unwrap()).unwrap(),
                    finding_revision: claim_evidence::FindingRevisionId::new(
                        f["revision"].as_str().unwrap(),
                    )
                    .unwrap(),
                    valid: false,
                };
                let state = campaign.read(identity).unwrap().unwrap();
                let prepared = support::applied(
                    campaign
                        .prepare_builder_evaluation(
                            identity,
                            &state.revision,
                            &joined.locator,
                            "eval-j2",
                            decision,
                            owners,
                        )
                        .unwrap(),
                );
                let p = campaign
                    .recover_builder_evaluation(&prepared.locator, owners)
                    .unwrap();
                assert!(
                    matches!(
                        p.state,
                        slice_campaign::BuilderEvaluationState::Unresolved { .. }
                    ),
                    "{:?}",
                    p.state
                );
                (
                    p.newly_observed_receipts,
                    serde_json::to_value(p.locator).unwrap(),
                )
            };
            assert!(!receipts.is_empty());
            fs::write(
                campaign
                    .anchored_root()
                    .parent()
                    .unwrap()
                    .join("j2-failure-progress.json"),
                json!({"locator":locator,"receipts":receipts}).to_string(),
            )
            .unwrap();
            std::process::exit(0); // Keep owner files for the parent's recovery check.
        },
    );
}
#[cfg(feature = "test-faults")]
#[test]
fn j2_post_owner_read_failure_retains_receipts_and_original_locators() {
    for cut in [
        "j2_after_finding_commit",
        "j2_after_projection_read",
        "j2_after_reliance_commit",
        "j2_after_evaluation_projection_read",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("case");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "j2_post_owner_failure_child", "--nocapture"])
            .env("J2_READ_FAILURE_CHILD", cut)
            .env("P1_CORPUS_ROOT", &root)
            .env("SLICE_CAMPAIGN_FAULT_CUT", cut)
            .env("SLICE_CAMPAIGN_FAULT_DIR", &root)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let ready = root.join(format!("{cut}.ready"));
        let deadline = Instant::now() + Duration::from_secs(180);
        while !ready.exists() {
            if let Some(status) = child.try_wait().unwrap() {
                let output = child.wait_with_output().unwrap();
                panic!(
                    "{cut}: {status}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            assert!(Instant::now() < deadline, "{cut}");
            std::thread::sleep(Duration::from_millis(10));
        }
        let sql = rusqlite::Connection::open(root.join("campaign/slice-campaign.sqlite")).unwrap();
        let original: String = sql
            .query_row("SELECT state_json FROM campaign_state", [], |r| r.get(0))
            .unwrap();
        sql.execute("UPDATE campaign_state SET state_json='{}'", [])
            .unwrap();
        fs::write(root.join(format!("{cut}.release")), b"release").unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{cut}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let progress: Value =
            serde_json::from_slice(&fs::read(root.join("j2-failure-progress.json")).unwrap())
                .unwrap();
        assert!(!progress["receipts"].as_array().unwrap().is_empty());
        sql.execute("UPDATE campaign_state SET state_json=?1", [original])
            .unwrap();
        drop(sql);
        finish_native_with(reopen_j2_fixture(&root), false, true, complete_fixture_j2);
        eprintln!(
            "J2 post-owner campaign read failure: {cut}: returned receipt preserved and recovered"
        );
    }
}
