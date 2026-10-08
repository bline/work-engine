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
    sync::atomic::{AtomicBool, Ordering},
};

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
        "decision_scopes":["scope-1"],"grant_ids":["grant-production"],
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
        "root_id":"claims-root","trusted_custody":custody,"grants":[grant]});
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
        .read_prepared(&identity, "review-0")
        .unwrap()
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
    let (temp, mut campaign, identity, selected, _) = support::selected_campaign(1, true);
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
        let record = FixtureRecord::controlled(
            "fixture-root",
            "record-join",
            "r1",
            episode_json(&result).into_bytes(),
            provenance,
            CheckedObservation::Absent {
                reason: "controlled fixture has no observed process".into(),
            },
            vec![],
        )
        .unwrap();
        let reference = record.reference.clone();
        let fixture = ControlledFixtureOwner::new(vec![record]).unwrap();
        let owner: Arc<dyn ExecutionEvidenceOwner> = if tamper {
            Arc::new(CorruptAfterCustody {
                inner: fixture,
                database: campaign.anchored_root().join("slice-campaign.sqlite"),
                used: AtomicBool::new(false),
            })
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
    assert_eq!(joined.stages.len(), if process { 6 } else { 5 });
    let replay = campaign
        .recover_initial_completion(&progress.locator, &mut owners)
        .unwrap();
    assert!(matches!(
        replay.state,
        InitialProgressState::OwnerReadsJoined
    ));
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
