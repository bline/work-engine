use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use lifecycle_proof::{
    identity::sha256_file,
    receipt::{ArtifactBinding, BuildProvenance, write_source_manifest},
};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

pub const BUILD_ID: &str = "build:s8:external-proof";
pub const SUBJECT_ID: &str = "subject:s8:task";
pub const TASK_ID: &str = "task:s8:analysis";
pub const SNAPSHOT_ID: &str = "snapshot:s8:approved";
pub const ANSWER_AUTHORITY: &str = "operator:s8:authorized";
pub const CHECKPOINT: &str = "ORION-7";
#[allow(dead_code)]
pub const Q1_QUESTION: &str = "Q1: Which approved snapshot constraint remains active?";
#[allow(dead_code)]
pub const Q2_QUESTION: &str = "Q2: What attributed result follows without repository mutation?";
pub const NATIVE_BINARY: &str = "/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex";

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub startup: Value,
    pub startup_path: PathBuf,
    pub host_socket: PathBuf,
    pub host_executable: PathBuf,
    pub source_manifest: ArtifactBinding,
    pub sources: Vec<ArtifactBinding>,
    pub build_provenance: BuildProvenance,
}

pub fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

/// Execute the separately compiled production UI client; the UI gets no owner APIs.
#[allow(dead_code)]
pub fn ui_observe(fixture: &Fixture, request: Value, case: &str, identity: Value) -> Value {
    use std::{
        process::Stdio,
        thread,
        time::{Duration, Instant},
    };
    let binary =
        PathBuf::from(std::env::var_os("S8_UI_CONSUMER_TEST_BINARY").expect("UI test binary pin"));
    let expected = std::env::var("S8_UI_CONSUMER_TEST_SHA256").expect("UI test hash pin");
    assert!(binary.is_absolute());
    assert_eq!(sha256_file(&binary).unwrap(), expected);
    let manifest =
        PathBuf::from(std::env::var_os("S8_UI_SOURCE_MANIFEST").expect("UI source manifest"));
    let manifest_binding = ArtifactBinding::read(&manifest).unwrap();
    let sources: Vec<ArtifactBinding> =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    assert!(!sources.is_empty());
    for source in &sources {
        source.verify().unwrap();
    }
    let log = tempfile::Builder::new()
        .prefix("ui-consumer-")
        .tempdir_in(fixture.root.path())
        .unwrap();
    let stdout = log.path().join("stdout");
    let stderr = log.path().join("stderr");
    let mut child = Command::new(&binary)
        .args([
            "--ignored",
            "--exact",
            "installed_host_read_only_consumer",
            "--nocapture",
        ])
        .env_clear()
        .env("S8_UI_ENDPOINT", &fixture.host_socket)
        .env("S8_UI_REQUEST", request.to_string())
        .env("S8_UI_CASE", case)
        .env("S8_UI_IDENTITY", identity.to_string())
        .stdout(Stdio::from(fs::File::create(&stdout).unwrap()))
        .stderr(Stdio::from(fs::File::create(&stderr).unwrap()))
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "UI consumer exceeded owned deadline; logs {}",
                log.keep().display()
            );
        }
        thread::sleep(Duration::from_millis(10));
    };
    let evidence = json!({"case":case,"request":request,"consumer_binary":ArtifactBinding::read(&binary).unwrap(),"source_manifest":manifest_binding,"source_entries":sources,"stdout":fs::read_to_string(&stdout).unwrap(),"stderr":fs::read_to_string(&stderr).unwrap(),"exit_code":status.code(),"reaped":true});
    assert!(status.success(), "UI consumer failed: {evidence}");
    assert_eq!(sha256_file(&binary).unwrap(), expected);
    manifest_binding.verify().unwrap();
    for source in &sources {
        source.verify().unwrap();
    }
    let path = log.path().join("consumer-receipt.json");
    fs::write(&path, serde_json::to_vec(&evidence).unwrap()).unwrap();
    let _ = log.keep();
    evidence
}

pub fn preflight_installed_host() -> BuildProvenance {
    let tmp = std::env::var_os("TMPDIR").expect("TMPDIR must be pinned for S8 proof");
    let preflight = tempfile::Builder::new()
        .prefix("s8-build-preflight-")
        .tempdir_in(tmp)
        .unwrap();
    let repo = repository_root();
    let host_executable = PathBuf::from(
        std::env::var_os("S8_INSTALLED_HOST")
            .expect("S8_INSTALLED_HOST must point to installed permanent host"),
    );
    let target_dir = PathBuf::from(
        std::env::var_os("CARGO_TARGET_DIR").expect("CARGO_TARGET_DIR must be pinned for S8 proof"),
    );
    assert!(target_dir.is_absolute());
    assert_eq!(
        host_executable,
        target_dir.join("debug/work-engine-task-host")
    );
    let paths = source_paths(&repo);
    let references = paths.iter().map(String::as_str).collect::<Vec<_>>();
    let (manifest, sources) = write_source_manifest(
        &repo,
        &references,
        &preflight.path().join("source-manifest.json"),
    )
    .unwrap();
    let lock_path = repo.join("rust/Cargo.lock");
    let toolchain_path = repo.join("rust/rust-toolchain.toml");
    let lock_before = sha256_file(&lock_path).unwrap();
    let toolchain_before = sha256_file(&toolchain_path).unwrap();
    let command = [
        "cargo",
        "build",
        "--offline",
        "--locked",
        "-p",
        "work-engine-task-host",
        "--features",
        "controlled-proof",
    ];
    let started_wall_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let output = Command::new(command[0])
        .args(&command[1..])
        .current_dir(repo.join("rust"))
        .env("TMPDIR", std::env::var_os("TMPDIR").unwrap())
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .unwrap();
    let finished_wall_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    assert!(
        output.status.success(),
        "controlled host build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(host_executable.is_file());
    manifest.verify().unwrap();
    for source in &sources {
        source.verify().unwrap();
    }
    assert_eq!(sha256_file(&lock_path).unwrap(), lock_before);
    assert_eq!(sha256_file(&toolchain_path).unwrap(), toolchain_before);
    BuildProvenance {
        command: command.into_iter().map(str::to_owned).collect(),
        started_wall_ms,
        finished_wall_ms,
        exit_code: output.status.code().unwrap(),
        executable_sha256: sha256_file(&host_executable).unwrap(),
        source_manifest_sha256: manifest.sha256,
        lock_sha256: lock_before,
        toolchain_sha256: toolchain_before,
        features: vec!["controlled-proof".into()],
        provenance_limit: "Successful pinned cargo build with source/lock/toolchain unchanged before and after, followed by executable hash and compiled feature attestation; no embedded source digest or reproducible-build proof.".into(),
    }
}

pub fn create_fixture(provider_url: &str, build_provenance: BuildProvenance) -> Fixture {
    let tmp = std::env::var_os("TMPDIR").expect("TMPDIR must be pinned for S8 proof");
    let root = tempfile::Builder::new()
        .prefix("s8-external-proof-")
        .tempdir_in(tmp)
        .unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let repo = repository_root();
    let host_executable = PathBuf::from(
        std::env::var_os("S8_INSTALLED_HOST")
            .expect("S8_INSTALLED_HOST must point to installed permanent host"),
    );
    assert!(host_executable.is_absolute());
    let target_dir = PathBuf::from(
        std::env::var_os("CARGO_TARGET_DIR").expect("CARGO_TARGET_DIR must be pinned for S8 proof"),
    );
    assert!(target_dir.is_absolute());
    assert_eq!(
        host_executable,
        target_dir.join("debug/work-engine-task-host")
    );
    let native_binary = Path::new(NATIVE_BINARY);
    assert!(native_binary.is_file(), "pinned native binary absent");
    let source_paths = source_paths(&repo);
    let references = source_paths.iter().map(String::as_str).collect::<Vec<_>>();
    let (source_manifest, sources) = write_source_manifest(
        &repo,
        &references,
        &root.path().join("source-manifest.json"),
    )
    .unwrap();
    assert!(host_executable.is_file());
    assert_eq!(
        source_manifest.sha256,
        build_provenance.source_manifest_sha256
    );
    assert_eq!(
        sha256_file(&host_executable).unwrap(),
        build_provenance.executable_sha256
    );
    assert_eq!(
        sha256_file(&repo.join("rust/Cargo.lock")).unwrap(),
        build_provenance.lock_sha256
    );
    assert_eq!(
        sha256_file(&repo.join("rust/rust-toolchain.toml")).unwrap(),
        build_provenance.toolchain_sha256
    );
    let service_socket = root.path().join("service.sock");
    let host_socket = root.path().join("host.sock");
    let native_root = root.path().join("native");
    fs::create_dir(&native_root).unwrap();
    fs::set_permissions(&native_root, fs::Permissions::from_mode(0o700)).unwrap();
    let barrier = root.path().join("barrier");
    fs::create_dir(&barrier).unwrap();
    for point in [
        "native_before_turn_entry",
        "native_after_turn_send_intent",
        "native_after_result_commit",
        "semantic_before_turn_entry",
        "semantic_before_commit",
    ] {
        fs::write(barrier.join(format!("{point}.release")), b"release\n").unwrap();
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let grant = |id: &str, context: &str, scope: &str| {
        json!({"grant_ref":id,"context_generation":context,"scope":scope,
               "expires_wall_ms":now+120_000,"revision":1})
    };
    let principal_uid = fs::metadata(root.path()).unwrap().uid();
    let snapshot_bytes = CHECKPOINT.as_bytes();
    let snapshot_sha = format!("{:x}", Sha256::digest(snapshot_bytes));
    let context = "context:s8:g0";
    let successor = "context:s8:g1";
    let second_successor = "context:s8:g2";
    let lifecycle = json!({
        "profile":"retained-repository-analysis-v1",
        "store_root":root.path().join("lifecycle-store"),
        "socket_path":service_socket,
        "peer_socket_path":root.path().join("unused-peer.sock"),
        "verifier_executable":null,
        "verifier_ledger_path":null,
        "proof_barrier_dir":barrier,
        "trusted_issuer":"issuer:s8",
        "principal":{"uid":principal_uid,"principal_ref":"principal:s8","producer_ref":"producer:s8"},
        "subject_id":SUBJECT_ID,
        "context_generation":context,
        "build_id":BUILD_ID,
        "proof_run_id":"proof:s8:external",
        "grants":[
            grant("grant:s8:enqueue",context,"enqueue_input"),
            grant("grant:s8:replace",context,"request_replacement"),
            grant("grant:s8:native",context,"native_turn"),
            grant("grant:s8:snapshot",context,"snapshot_read"),
            grant("grant:s8:semantic",context,"semantic_control"),
            grant("grant:s8:question",context,"task_question_open"),
            grant("grant:s8:result",context,"task_result_publish"),
            grant("grant:s8:enqueue:g1",successor,"enqueue_input"),
            grant("grant:s8:replace:g1",successor,"request_replacement"),
            grant("grant:s8:native:g1",successor,"native_turn"),
            grant("grant:s8:snapshot:g1",successor,"snapshot_read"),
            grant("grant:s8:semantic:g1",successor,"semantic_control"),
            grant("grant:s8:question:g1",successor,"task_question_open"),
            grant("grant:s8:result:g1",successor,"task_result_publish"),
            grant("grant:s8:enqueue:g2",second_successor,"enqueue_input"),
            grant("grant:s8:native:g2",second_successor,"native_turn"),
            grant("grant:s8:snapshot:g2",second_successor,"snapshot_read"),
            grant("grant:s8:semantic:g2",second_successor,"semantic_control"),
            grant("grant:s8:question:g2",second_successor,"task_question_open"),
            grant("grant:s8:result:g2",second_successor,"task_result_publish")
        ],
        "max_executions":3,"max_wait_ms":1000,
        "native":{
            "executable":native_binary,"root":native_root,
            "simulator_base_url":provider_url,
            "session_id":"session:s8:g0",
            "invocation_id":"invocation:s8:domain:g0",
            "attempt_id":"attempt:s8:domain:g0",
            "prompt":"Analyze the approved snapshot while preserving pending authority.",
            "turn_grant_ref":"grant:s8:native","turn_grant_revision":1,
            "snapshot_grant_ref":"grant:s8:snapshot","snapshot_grant_revision":1,
            "same_native_session_required":false,
            "snapshot":{"id":SNAPSHOT_ID,"source_revision":"source:s8:approved",
                "members":[{"id":"m1","sha256":snapshot_sha,"bytes":snapshot_bytes}]},
            "semantic":{
                "capture_id":"capture:s8:g0",
                "compile_session_id":"session:s8:compile:g0",
                "verify_session_id":"session:s8:verify:g0",
                "successor_verify_session_id":"session:s8:successor-verify:g0",
                "compile_invocation_id":"invocation:s8:compile:g0",
                "compile_attempt_id":"attempt:s8:compile:g0",
                "candidate_verify_invocation_id":"invocation:s8:candidate-verify:g0",
                "candidate_verify_attempt_id":"attempt:s8:candidate-verify:g0",
                "successor_verify_invocation_id":"invocation:s8:successor-verify:g0",
                "successor_verify_attempt_id":"attempt:s8:successor-verify:g0",
                "challenge_id":"challenge:s8:g0",
                "challenge_text":"Choose the next authorized analysis action; preserve the unresolved approval.",
                "control_grant_ref":"grant:s8:semantic","control_grant_revision":1,
                "successor_control_grant_ref":"grant:s8:semantic:g1",
                "successor_control_grant_revision":1
            },
            "semantic_contexts":[{
                "predecessor_context":successor,
                "semantic":{
                    "capture_id":"capture:s8:g1",
                    "compile_session_id":"session:s8:compile:g1",
                    "verify_session_id":"session:s8:verify:g1",
                    "successor_verify_session_id":"session:s8:successor-verify:g2",
                    "compile_invocation_id":"invocation:s8:compile:g1",
                    "compile_attempt_id":"attempt:s8:compile:g1",
                    "candidate_verify_invocation_id":"invocation:s8:candidate-verify:g1",
                    "candidate_verify_attempt_id":"attempt:s8:candidate-verify:g1",
                    "successor_verify_invocation_id":"invocation:s8:successor-verify:g2",
                    "successor_verify_attempt_id":"attempt:s8:successor-verify:g2",
                    "challenge_id":"challenge:s8:g1",
                    "challenge_text":"Choose the next authorized analysis action for queued Q1; preserve Q2 custody.",
                    "control_grant_ref":"grant:s8:semantic:g1","control_grant_revision":1,
                    "successor_control_grant_ref":"grant:s8:semantic:g2",
                    "successor_control_grant_revision":1
                }
            }]
        }
    });
    let startup = json!({
        "profile":"retained-repository-analysis-v1",
        "task_store_path":root.path().join("tasks.sqlite"),
        "socket_path":host_socket,
        "approved_snapshot_ref":SNAPSHOT_ID,
        "answer_authority_ref":ANSWER_AUTHORITY,
        "enqueue_grant_ref":"grant:s8:enqueue",
        "replacement_grant_ref":"grant:s8:replace",
        "executable_sha256":sha256_file(&host_executable).unwrap(),
        "source_root":repo,
        "source_manifest_path":source_manifest.path,
        "source_sha256":source_manifest.sha256,
        "lock_path":repository_root().join("rust/Cargo.lock"),
        "lock_sha256":sha256_file(&repository_root().join("rust/Cargo.lock")).unwrap(),
        "toolchain_path":repository_root().join("rust/rust-toolchain.toml"),
        "toolchain_sha256":sha256_file(&repository_root().join("rust/rust-toolchain.toml")).unwrap(),
        "features":["controlled-proof"],
        "provider_identity":"native-sim-proof-v1",
        "lifecycle":lifecycle
    });
    let startup_path = root.path().join("host-startup.json");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&startup_path)
        .unwrap();
    file.write_all(&serde_json::to_vec(&startup).unwrap())
        .unwrap();
    file.sync_all().unwrap();
    Fixture {
        root,
        startup,
        startup_path,
        host_socket,
        host_executable,
        source_manifest,
        sources,
        build_provenance,
    }
}

pub fn initial_submit() -> Value {
    json!({
        "command_id":"command:s8:d0",
        "task_id":TASK_ID,
        "subject_id":SUBJECT_ID,
        "kind":"repository_analysis",
        "objective":"Analyze the approved repository snapshot and identify the ORION-7 marker.",
        "engineering_question":"Which approved marker appears in member m1, and what remains pending?",
        "snapshot_ref":SNAPSHOT_ID,
        "facts":[
            {"id":"rule","kind":"governing","text":"Use only the approved snapshot member m1."},
            {"id":"decision","kind":"decision","text":"Do not revive the superseded editing proposal."},
            {"id":"commitment","kind":"commitment","text":"Preserve the exact read receipt across context changes."},
            {"id":"uncertainty","kind":"uncertainty","text":"The next action awaits an authorized answer."},
            {"id":"reference","kind":"canonical_reference","text":"Canonical reference: snapshot:s8:approved/m1."},
            {"id":"approval","kind":"human_interaction","text":"Approval for repository mutation remains pending."}
        ]
    })
}

#[allow(dead_code)]
pub fn later_submit(command_id: &str, engineering_question: &str) -> Value {
    let mut input = initial_submit();
    input["command_id"] = json!(command_id);
    input["engineering_question"] = json!(engineering_question);
    input["facts"] = json!([]);
    input
}

fn source_paths(root: &Path) -> Vec<String> {
    const CRATES: &[&str] = &[
        "work-engine-task-host",
        "retained-task-workflow",
        "lifecycle-proof",
        "lifecycle-service",
        "lifecycle-core",
        "lifecycle-store",
        "lifecycle-semantic",
        "lifecycle-codex",
        "lifecycle-provider",
        "lifecycle-runtime",
        "lifecycle-wire",
        "lifecycle-client",
        "work-engine-types",
    ];
    let mut files = vec![
        "rust/Cargo.toml".to_owned(),
        "docs/rust-retained-task-host-contract.md".to_owned(),
        "docs/rust-context-lifecycle-s8-implementation-plan.md".to_owned(),
    ];
    for name in CRATES {
        collect(root, &format!("rust/crates/{name}"), &mut files);
    }
    files.sort();
    files.dedup();
    files
}

fn collect(root: &Path, relative: &str, files: &mut Vec<String>) {
    for entry in fs::read_dir(root.join(relative)).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(name.as_str(), "target" | ".git") {
            continue;
        }
        let next = format!("{relative}/{name}");
        if entry.file_type().unwrap().is_dir() {
            collect(root, &next, files);
        } else if Path::new(&name)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext, "rs" | "toml" | "sql" | "json"))
        {
            files.push(next);
        }
    }
}
