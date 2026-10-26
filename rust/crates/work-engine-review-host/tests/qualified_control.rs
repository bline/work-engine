#![cfg(all(unix, feature = "test-peer", feature = "test-support"))]
mod support;

use claim_evidence::sha256_bytes;
use review_execution_evidence::claude::{
    CandidateBoundary, CredentialSource, PinnedProgram, TrustedClaudeProfile, TrustedRp0Profile,
};
use review_workflow::startup::{
    FilePin, OFFLINE_PROFILE, QualifiedInputs, QualifiedStartupDocument, RP0_COVERAGE_GUIDANCE_V2,
};
use serde_json::{Value, json};
use slice_campaign::{OfflineAcceptancePin, contract::validate_selection, verify_existing_subject};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use support::*;

fn own(pin: &mut FilePin) {
    pin.owner_uid = Some(fs::metadata(&pin.path).unwrap().uid());
}
fn own_base(d: &mut review_workflow::startup::StartupDocument) {
    for p in [
        &mut d.host.executable,
        &mut d.host.source_archive,
        &mut d.host.toolchain,
        &mut d.host.lockfile,
        &mut d.gate_capture,
        &mut d.gate_manifest,
        &mut d.gate_receipt,
        &mut d.candidate,
        &mut d.physical_profile,
        &mut d.selection,
        &mut d.episode_authority,
        &mut d.compiler.document,
        &mut d.compiler.options,
        &mut d.request_input,
        &mut d.peer.executable,
        &mut d.peer.source,
        &mut d.claims.source,
        &mut d.claims.configuration,
    ] {
        own(p);
    }
    for p in &mut d.host.runtime_dependencies {
        own(p);
    }
    if let Some(p) = &mut d.compiler.requirements {
        own(p);
    }
}
fn copy_program(base: &Path, source: &Path, name: &str) -> FilePin {
    let target = base.join(name);
    fs::copy(source, &target).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o500)).unwrap();
    let mut result = pin(&target);
    own(&mut result);
    result
}
fn e1_bin(name: &str) -> PathBuf {
    let current = std::env::current_exe().unwrap();
    current.parent().unwrap().parent().unwrap().join(name)
}

fn diagnostic_e1_events(root: &Path) -> Vec<(String, String)> {
    let Ok(db) = rusqlite::Connection::open_with_flags(
        root.join("evidence.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) else {
        return Vec::new();
    };
    let Ok(mut statement) =
        db.prepare("SELECT stage, payload FROM events WHERE stage != 'prepared' ORDER BY rowid")
    else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map([], |row| {
        let stage: String = row.get(0)?;
        let payload: Vec<u8> = row.get(1)?;
        Ok((
            stage,
            String::from_utf8_lossy(&payload[..payload.len().min(512)]).into_owned(),
        ))
    }) else {
        return Vec::new();
    };
    rows.flatten().collect()
}
fn offline_fixture() -> (Fixture, PathBuf) {
    let mut f = Fixture::new(0, "result");
    let base = f.temp.path();
    // The historical Git subject is kept exact while the candidate path gains
    // a checked-out source tree for the candidate-private CBM index.
    let working = base.join("candidate-working");
    let status = Command::new("/usr/bin/git")
        .args(["clone", "-q"])
        .arg(&f.document.repository)
        .arg(&working)
        .status()
        .unwrap();
    assert!(status.success());
    let status = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&working)
        .args([
            "fetch",
            "-q",
            "origin",
            "refs/work-engine/*:refs/work-engine/*",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let status = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&working)
        .args(["checkout", "-q", "49624730af1663f50a3ff85abfdd1b90647d0cdb"])
        .status()
        .unwrap();
    assert!(status.success());
    f.document.repository = working.clone();
    let (candidate, physical) = anchored_artifacts(&working);
    f.document.candidate = captured(base, "candidate-qualified.json", &candidate);
    f.document.physical_profile = captured(base, "physical-qualified.json", &physical);
    f.document.profile = OFFLINE_PROFILE.into();
    f.document.evidence_profile = "claude-offline-peer-v1".into();
    f.document.peer.executable =
        copy_program(base, &e1_bin("claude_offline_peer"), "offline-provider");
    let bwrap = pin(Path::new("/usr/bin/bwrap"));
    let cbm = pin(Path::new("/home/bline/.local/bin/codebase-memory-mcp"));
    let launcher = copy_program(base, &e1_bin("claude_cbm_restricted"), "cbm-launcher");
    let client = copy_program(base, &e1_bin("claude_cbm_client"), "cbm-client");
    let guard = copy_program(base, &e1_bin("claude_launch_guard"), "launch-guard");
    let mut bwrap = bwrap;
    own(&mut bwrap);
    let mut cbm = cbm;
    own(&mut cbm);
    let rp0 = TrustedRp0Profile::load(
        PinnedProgram::load(&bwrap.path, &bwrap.sha256).unwrap(),
        PinnedProgram::load(&cbm.path, &cbm.sha256).unwrap(),
        PinnedProgram::load(&launcher.path, &launcher.sha256).unwrap(),
        PinnedProgram::load(&client.path, &client.sha256).unwrap(),
    )
    .unwrap();
    let candidate_value: Value = serde_json::from_slice(&candidate).unwrap();
    let refused_root = base.join("wrong-selected-tree-private");
    assert!(
        CandidateBoundary::prepare_selected(
            rp0.clone(),
            &working,
            &refused_root,
            "rp2_fixture",
            &"a".repeat(40),
        )
        .is_err()
    );
    assert!(
        !refused_root.exists(),
        "wrong tree sealed a private boundary"
    );
    let boundary = CandidateBoundary::prepare_selected(
        rp0,
        &working,
        base.join("candidate-private"),
        "rp2_fixture",
        candidate_value["checkpoint_tree_oid"].as_str().unwrap(),
    )
    .unwrap();
    assert!(boundary.index().coverage_complete());
    let (boundary, manifest_sha) = boundary.publish_manifest().unwrap();
    let catalog = captured(base, "catalog.json", br#"{"offline":true}"#);
    let profile = TrustedClaudeProfile::load_offline_guarded_peer(
        PinnedProgram::load(
            &f.document.peer.executable.path,
            &f.document.peer.executable.sha256,
        )
        .unwrap(),
        PinnedProgram::load(&guard.path, &guard.sha256).unwrap(),
        "offline-test-model-v1",
        "high",
        vec!["offline-test-model-v1".into()],
        &fs::read(&catalog.path).unwrap(),
    )
    .unwrap();
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        + 86_400_000;
    let credential = base.join("credential-canary.json");
    fs::write(
        &credential,
        json!({"claudeAiOauth":{"accessToken":"rp2-offline-canary",
        "expiresAt":expires}})
        .to_string(),
    )
    .unwrap();
    fs::set_permissions(&credential, fs::Permissions::from_mode(0o600)).unwrap();
    let credential_sha = CredentialSource::new(&credential)
        .unwrap()
        .metadata_digest();
    let verified = verify_existing_subject(
        &working,
        &f.document.identity,
        &f.document.baseline,
        &candidate,
        &physical,
        &fs::read(&f.document.gate_capture.path).unwrap(),
        &fs::read(&f.document.gate_manifest.path).unwrap(),
        &fs::read(&f.document.gate_receipt.path).unwrap(),
    )
    .unwrap();
    let selection: Value =
        serde_json::from_slice(&fs::read(&f.document.selection.path).unwrap()).unwrap();
    let selected =
        validate_selection(&selection, &f.document.identity, &verified.reference).unwrap();
    let subject = selection["subject"].clone();
    let result = json!({"schemaVersion":1,"subject":subject,"verdict":"acceptable_as_is",
        "findings":[],"decisiveEvidence":[{"path":"source.rs","startLine":1,"endLine":1,
        "sha256":"a".repeat(64)}],"limitations":[]});
    f.document.request_input = data(
        base,
        "offline-prompt.json",
        &json!({"mode":"held_success","result":result}),
    );
    let mut compiler_document: Value =
        serde_json::from_slice(&fs::read(&f.document.compiler.document.path).unwrap()).unwrap();
    compiler_document["roles"]["reviewer"]["developer_instructions"] =
        json!(RP0_COVERAGE_GUIDANCE_V2);
    let mut guided_document = data(base, "manifest-rp0-v2.json", &compiler_document);
    own(&mut guided_document);
    f.document.compiler.document = guided_document;
    let accepted = json!({"schemaVersion":1,"decision":"accepted","reference":"rp2-offline-held",
        "revision":"1","identity":f.document.identity,"obligationId":f.document.obligation_id,
        "candidateTree":verified.reference.tree,
        "builderClaimId":selection["specialists"][0]["requiredClaims"][0]["claimId"],
        "campaignClaimId":selection["specialists"][0]["requiredClaims"][1]["claimId"],
        "selectionDigest":selected.campaign_digest,"physicalProfileDigest":verified.profile_digest,
        "claudeProfileDigest":profile.profile_digest(),"programSha256":f.document.peer.executable.sha256,
        "guardSha256":guard.sha256,"cbmStoreSha256":boundary.index().store_sha256(),
        "catalogSha256":profile.catalog_sha256(),"requestedModel":profile.requested_model(),
        "credentialSourceMetadataSha256":credential_sha,
        "resourcePolicy":"strict_aggregate_session_64mib_v1",
        "sessionId":f.document.peer.session,"attemptId":f.document.identity.attempt_id,
        "acceptedBoundarySha256":f.document.accepted_boundary.sha256,
        "promptTemplateSha256":sha256_bytes(
            &review_workflow::startup::qualified_prompt_template_for_document(&f.document)
                .unwrap()),
        "attemptHorizonMillis":15_000});
    let accepted_raw = serde_json::to_vec(&accepted).unwrap();
    OfflineAcceptancePin::test_owned(&accepted_raw).unwrap();
    let mut acceptance = captured(base, "accepted-offline.json", &accepted_raw);
    own(&mut acceptance);
    let custody = json!({"owner":"review-execution-evidence",
        "verifier":"review-execution-evidence-claude-offline-v1",
        "profile":"claude-offline-peer-v1","artifact_schemes":["execution-artifact:"],
        "reference_schemes":["execution-evidence:"]});
    let old_source: Value =
        serde_json::from_slice(&fs::read(&f.document.claims.source.path).unwrap()).unwrap();
    let old_config: Value =
        serde_json::from_slice(&fs::read(&f.document.claims.configuration.path).unwrap()).unwrap();
    let old_grant = &old_config["grants"][0];
    let source = json!({"schema_version":2,"owner":old_source["owner"],
        "reference":old_source["reference"],"revision":old_source["revision"],
        "freshness":old_source["freshness"],"profile":"native-review-claims-v1",
        "actors":["campaign"],"decision_scopes":[old_grant["decision_scope"]],
        "grant_ids":["h1-production","h1-completion"],"custody_config_sha256":claim_digest(&custody)});
    let mut source_pin = data(base, "claims-source-qualified.json", &source);
    own(&mut source_pin);
    let authority_ref = json!({"owner":source["owner"],"reference":source["reference"],
        "revision":source["revision"],"freshness":source["freshness"],
        "integrity_sha256":source_pin.sha256,"status":"verified"});
    let production = json!({"schema_version":1,"grant_id":"h1-production","actor":"campaign",
        "profile":"production-path-v1","permissions":["record_observation","establish_claim","read_admission"],
        "decision_scope":old_grant["decision_scope"],"authority_reference":authority_ref});
    let completion = json!({"schema_version":1,"grant_id":"h1-completion","actor":"campaign",
        "profile":"revision-bound-review-finding-v1",
        "permissions":["create_claim","publish_revision","record_reliance"],
        "decision_scope":old_grant["decision_scope"],"authority_reference":authority_ref});
    let config = json!({"schema_version":2,"profile":"native-review-claims-v1",
        "root_id":f.document.claims.root_id,"trusted_custody":custody,
        "grants":[production,completion]});
    let mut config_pin = data(base, "claims-config-qualified.json", &config);
    own(&mut config_pin);
    f.document.claims.source = source_pin;
    f.document.claims.configuration = config_pin;
    f.document.claims.production_grant_sha256 = claim_digest(&production);
    let mut catalog = catalog;
    own(&mut catalog);
    own_base(&mut f.document);
    let witness = base.join("offline-peer-witness");
    let qualified = QualifiedInputs {
        bwrap,
        cbm,
        cbm_launcher: launcher,
        cbm_client: client,
        guard,
        catalog,
        boundary_root: base.join("candidate-private"),
        boundary_manifest_sha256: manifest_sha,
        requested_model: "offline-test-model-v1".into(),
        requested_effort: "high".into(),
        accepted_observed_models: vec!["offline-test-model-v1".into()],
        credential_source: credential,
        credential_source_metadata_sha256: credential_sha,
        resource_policy: "strict_aggregate_session_64mib_v1".into(),
        attempt_horizon_millis: 15_000,
        completion_grant_id: "h1-completion".into(),
        completion_grant_sha256: claim_digest(&completion),
        completion_consumer: "campaign".into(),
        completion_decision_scope: old_grant["decision_scope"].as_str().unwrap().into(),
        completion_authority_source: authority_ref,
        offline_acceptance: Some(acceptance),
        offline_witness: Some(witness.clone()),
    };
    let envelope = QualifiedStartupDocument {
        schema_version: 2,
        profile: OFFLINE_PROFILE.into(),
        base: f.document.clone(),
        qualified,
    };
    fs::set_permissions(&f.startup, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&f.startup, serde_json::to_vec(&envelope).unwrap()).unwrap();
    fs::set_permissions(&f.startup, fs::Permissions::from_mode(0o400)).unwrap();
    (f, witness)
}

fn replace_qualified_gate_capture(
    fixture: &Fixture,
    envelope: &mut QualifiedStartupDocument,
    name: &str,
    bytes: &[u8],
) {
    let mut capture = captured(fixture.temp.path(), name, bytes);
    own(&mut capture);
    envelope.base.gate_capture = capture;
    fs::set_permissions(&fixture.startup, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&fixture.startup, serde_json::to_vec(envelope).unwrap()).unwrap();
    fs::set_permissions(&fixture.startup, fs::Permissions::from_mode(0o400)).unwrap();
}

fn write_qualified_startup(fixture: &Fixture, envelope: &QualifiedStartupDocument) {
    fs::set_permissions(&fixture.startup, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&fixture.startup, serde_json::to_vec(envelope).unwrap()).unwrap();
    fs::set_permissions(&fixture.startup, fs::Permissions::from_mode(0o400)).unwrap();
}

fn qualified_startup_result(fixture: &Fixture) -> std::process::Output {
    Command::new(&fixture.host)
        .arg("initialize")
        .arg("--startup")
        .arg(&fixture.startup)
        .output()
        .unwrap()
}

#[test]
fn schema_two_qualified_startup_requires_actual_selected_role_guidance() {
    let (fixture, witness) = offline_fixture();
    let mut envelope: QualifiedStartupDocument =
        serde_json::from_slice(&fs::read(&fixture.startup).unwrap()).unwrap();
    let old_template =
        review_workflow::startup::qualified_prompt_template_for_document(&envelope.base).unwrap();
    assert!(String::from_utf8_lossy(&old_template).contains(RP0_COVERAGE_GUIDANCE_V2));
    let mut compiler_document: Value =
        serde_json::from_slice(&fs::read(&envelope.base.compiler.document.path).unwrap()).unwrap();
    compiler_document["roles"]["reviewer"]["developer_instructions"] =
        json!("Review the candidate without a coverage locator.");
    let mut altered = data(
        fixture.temp.path(),
        "manifest-rp0-unguided.json",
        &compiler_document,
    );
    own(&mut altered);
    envelope.base.compiler.document = altered;
    write_qualified_startup(&fixture, &envelope);
    let refused = qualified_startup_result(&fixture);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("schema-2 coverage guidance absent"));
    assert!(!witness.exists());
}

#[test]
fn v2_protected_startup_refuses_mixed_resource_policies_before_entry() {
    const V1: &str = "strict_aggregate_session_64mib_v1";
    const V2: &str = "session64mib_combined1g_zero_swap_v2";
    let (fixture, witness) = offline_fixture();
    let mut envelope: QualifiedStartupDocument =
        serde_json::from_slice(&fs::read(&fixture.startup).unwrap()).unwrap();
    let original_acceptance = envelope.qualified.offline_acceptance.as_ref().unwrap();
    let mut accepted: Value =
        serde_json::from_slice(&fs::read(&original_acceptance.path).unwrap()).unwrap();
    assert_eq!(accepted["resourcePolicy"], V1);

    // A descriptive startup field cannot switch the protected Applied policy.
    envelope.qualified.resource_policy = V2.into();
    write_qualified_startup(&fixture, &envelope);
    let refused = qualified_startup_result(&fixture);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("accepted execution selection differs")
    );
    assert!(!witness.exists());
    assert!(
        !fixture
            .document
            .roots
            .evidence
            .join("evidence.sqlite")
            .exists()
    );

    // Nor can a v2 accepted record be paired with a v1 E1 boundary.
    accepted["resourcePolicy"] = V2.into();
    let bytes = serde_json::to_vec(&accepted).unwrap();
    OfflineAcceptancePin::test_owned(&bytes).unwrap();
    let mut pin = captured(fixture.temp.path(), "accepted-offline-v2.json", &bytes);
    own(&mut pin);
    envelope.qualified.offline_acceptance = Some(pin);
    envelope.qualified.resource_policy = V1.into();
    write_qualified_startup(&fixture, &envelope);
    let refused = qualified_startup_result(&fixture);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("accepted execution selection differs")
    );
    assert!(!witness.exists());
    assert!(
        !fixture
            .document
            .roots
            .evidence
            .join("evidence.sqlite")
            .exists()
    );

    // The same production validation core admits the coherent v2 pair, while
    // this read-only load still issues no campaign entry or provider launch.
    envelope.qualified.resource_policy = V2.into();
    write_qualified_startup(&fixture, &envelope);
    let admitted = qualified_startup_result(&fixture);
    assert!(
        admitted.status.success(),
        "{}",
        String::from_utf8_lossy(&admitted.stderr)
    );
    assert!(!witness.exists());
    assert!(
        !fixture
            .document
            .roots
            .evidence
            .join("evidence.sqlite")
            .exists()
    );
}

fn qualified_ready(client: &mut Client) -> Value {
    let deadline = Instant::now() + Duration::from_secs(240);
    loop {
        client.send(json!({"version":1,"requestId":"qualified-ready","operation":"status"}));
        let response = client.receive();
        assert!(
            response["result"]["lastObservation"]["startupError"].is_null(),
            "{response}"
        );
        if response["result"]["busy"] == false {
            return response;
        }
        assert!(Instant::now() < deadline, "qualified startup timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn qualified_startup_uses_dynamic_gate_and_refuses_altered_capture() {
    let (fixture, _) = offline_fixture();
    let mut envelope: QualifiedStartupDocument =
        serde_json::from_slice(&fs::read(&fixture.startup).unwrap()).unwrap();
    let mut fresh_capture = fs::read(&envelope.base.gate_capture.path).unwrap();
    let historical_digest = sha256_bytes(&fresh_capture);
    fresh_capture.push(b'\n');
    assert_ne!(sha256_bytes(&fresh_capture), historical_digest);
    replace_qualified_gate_capture(
        &fixture,
        &mut envelope,
        "gate-dynamic-valid.json",
        &fresh_capture,
    );
    assert_eq!(
        fixture.initialize()["status"],
        "initialized",
        "a differently captured but consistent schema-4 gate must load"
    );
    let original: Value = serde_json::from_slice(&fresh_capture).unwrap();
    for (name, mutation) in [
        ("producer", "producer_sha256_before_after"),
        ("tested_tree", "frozen_gate_tree_oid"),
        ("exit", "exit_status"),
    ] {
        let mut altered = original.clone();
        altered[mutation] = if mutation == "exit_status" {
            json!(1)
        } else {
            json!("invalid")
        };
        replace_qualified_gate_capture(
            &fixture,
            &mut envelope,
            &format!("gate-dynamic-{name}.json"),
            &serde_json::to_vec(&altered).unwrap(),
        );
        let output = Command::new(&fixture.host)
            .args(["initialize", "--startup"])
            .arg(&fixture.startup)
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "altered {name} must be rejected after pin update"
        );
    }
}

#[test]
fn applied_entry_rejects_independent_scope_mutations_before_peer_entry() {
    for field in ["episode", "candidate", "prompt", "horizon"] {
        eprintln!("RP2_APPLIED_MUTATION_FIELD={field}");
        let (fixture, witness) = offline_fixture();
        let initial = fixture.initialize();
        let revision = initial["campaign"]["revision"].as_str().unwrap();
        let mut host = fixture.start(&[("RP2_TEST_APPLIED_SCOPE_MUTATION", field)]);
        qualified_ready(&mut host);
        let dispatch_at = Instant::now();
        host.domain("execute_initial", revision);
        let refusal = host
            .rx
            // This is a pre-entry debug hash/verification path, not the
            // provider attempt horizon or the status responsiveness gate.
            // Candidate-two root validation exceeded the generic 90-second
            // client bound; four observed successful refusals took 51-54 s.
            .recv_timeout(Duration::from_secs(if cfg!(debug_assertions) {
                180
            } else {
                60
            }))
            .unwrap_or_else(|error| {
                eprintln!(
                    "RP2_APPLIED_MUTATION_EVENTS={field}:{:?}",
                    diagnostic_e1_events(&fixture.document.roots.evidence)
                );
                panic!("{field} Applied response timeout: {error}");
            });
        eprintln!(
            "RP2_APPLIED_MUTATION_RESPONSE={field}:{}ms",
            dispatch_at.elapsed().as_millis()
        );
        assert_eq!(
            refusal["error"]["kind"], "execution_evidence",
            "{field} scope mutation bypassed Applied entry: {refusal}"
        );
        assert!(!witness.exists(), "{field} reached the offline peer");
        assert_eq!(fixture.entries(), 0, "{field} launched a peer");
        let journal = fixture.document.roots.evidence.join("evidence.sqlite");
        if journal.exists() {
            let count: i64 = rusqlite::Connection::open_with_flags(
                journal,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM events WHERE stage IN ('launch_intent','spawned')",
                [],
                |row| row.get(0),
            )
            .unwrap();
            assert_eq!(count, 0, "{field} recorded entry intent");
        }
        host.finish();
    }
}

#[test]
fn permanent_host_offline_held_provider_status_probe() {
    let (f, witness) = offline_fixture();
    let hold_root = f.temp.path().join("held-interval");
    fs::create_dir(&hold_root).unwrap();
    let initial = f.initialize();
    let revision = initial["campaign"]["revision"].as_str().unwrap();
    let mut c = f.start(&[
        ("RP2_RELAY_WITNESS", witness.to_str().unwrap()),
        ("RP2_RELAY_HOLD_ROOT", hold_root.to_str().unwrap()),
    ]);
    let ready_deadline = Instant::now() + Duration::from_secs(180);
    loop {
        c.send(json!({"version":1,"requestId":"qualified-ready","operation":"status"}));
        let response = c.receive();
        assert!(
            response["result"]["lastObservation"]["startupError"].is_null(),
            "{response}"
        );
        if response["result"]["busy"] == false {
            break;
        }
        assert!(
            Instant::now() < ready_deadline,
            "qualified worker startup timed out: {response}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let started = Instant::now();
    for n in 0..4 {
        c.send(json!({"version":1,"requestId":format!("execute-{n}"),
        "operation":"execute_initial","obligationId":"review-0","operationId":"h1-initial",
        "expectedRevision":revision}));
    }
    let mut samples = Vec::new();
    let mut replies = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(180);
    while !hold_root.join("entered").exists() {
        while let Ok(reply) = c.rx.try_recv() {
            if reply["requestId"] == "execute-0" {
                eprintln!(
                    "RP2_HELD_EARLY_E1_EVENTS={:?}",
                    diagnostic_e1_events(&f.document.roots.evidence)
                );
            }
            assert!(
                reply["requestId"] != "execute-0",
                "entry refused before peer: {reply}"
            );
            replies.push(reply);
        }
        assert!(
            Instant::now() < deadline,
            "peer hold entry absent after Applied dispatch"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(fs::read(hold_root.join("entered")).unwrap(), b"entered\n");
    let entry_seen_at = Instant::now();
    let entry_seen_micros = started.elapsed().as_micros() as u64;
    assert!(!hold_root.join("exited").exists());
    for n in 0..700 {
        let id = format!("rp2-status-{n}");
        let sent = Instant::now();
        c.send(json!({"version":1,"requestId":id,"operation":"status"}));
        loop {
            let response = c.receive();
            if response["requestId"] == id {
                assert_eq!(
                    response["result"]["busy"], true,
                    "status probe escaped the held operation: {response}"
                );
                samples.push(sent.elapsed().as_micros() as u64);
                break;
            }
            assert_ne!(
                response["requestId"], "execute-0",
                "peer completed before all held status probes: {response}"
            );
            replies.push(response);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut ordered = samples.clone();
    ordered.sort_unstable();
    let p50 = ordered[(ordered.len() * 50).div_ceil(100) - 1];
    let p95 = ordered[(ordered.len() * 95).div_ceil(100) - 1];
    let p99 = ordered[(ordered.len() * 99).div_ceil(100) - 1];
    let max = *ordered.last().unwrap();
    assert!(p99 <= 50_000 && max <= 250_000, "p99={p99} max={max}");
    let probe_finished_micros = started.elapsed().as_micros() as u64;
    assert!(
        !hold_root.join("exited").exists(),
        "peer hold ended before all 700 status probes"
    );
    if entry_seen_at.elapsed() < Duration::from_secs(6) {
        // Fixture synchronization: keep the peer blocked until both the
        // complete probe set and the six-second lower bound have elapsed.
        std::thread::sleep(Duration::from_secs(6) - entry_seen_at.elapsed());
    }
    let mut release = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(hold_root.join("released"))
        .unwrap();
    use std::io::Write;
    release.write_all(b"release\n").unwrap();
    release.sync_all().unwrap();
    fs::File::open(&hold_root).unwrap().sync_all().unwrap();
    let release_micros = started.elapsed().as_micros() as u64;
    let exit_deadline = Instant::now() + Duration::from_secs(30);
    while !hold_root.join("exited").exists() {
        assert!(
            Instant::now() < exit_deadline,
            "peer hold exit marker absent"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(fs::read(hold_root.join("exited")).unwrap(), b"exited\n");
    let exit_seen_micros = started.elapsed().as_micros() as u64;
    assert!(
        entry_seen_micros < probe_finished_micros
            && probe_finished_micros < release_micros
            && release_micros <= exit_seen_micros
    );
    assert_eq!(
        fs::read_to_string(&witness)
            .unwrap_or_default()
            .lines()
            .count(),
        1
    );
    if !replies
        .iter()
        .any(|reply| reply["requestId"] == "execute-0")
    {
        replies.push(
            c.rx.recv_timeout(Duration::from_secs(180))
                .expect("qualified result or refusal"),
        );
    }
    let checked_refusal = replies.iter().any(|reply| {
        reply["requestId"] == "execute-0"
            && reply["error"]["kind"] == "execution_evidence"
            && reply["error"]["message"]
                == "unsupported evidence contract: evidence class outside installed reader profile"
    });
    if !checked_refusal {
        eprintln!(
            "RP2_HELD_E1_EVENTS={:?}",
            diagnostic_e1_events(&f.document.roots.evidence)
        );
    }
    assert!(
        checked_refusal,
        "offline peer did not reach checked production-class refusal: {replies:?}"
    );
    let response_bytes: Vec<u8> = rusqlite::Connection::open_with_flags(
        f.document.roots.evidence.join("evidence.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
    .query_row(
        "SELECT bytes FROM artifacts WHERE kind='response' ORDER BY rowid DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
    .unwrap();
    let response: Value = serde_json::from_slice(&response_bytes).unwrap();
    let observed_hold_millis = response["offlineHoldMillis"].as_u64().unwrap();
    assert!(
        observed_hold_millis >= 6_000,
        "peer did not hold for six seconds"
    );
    assert_eq!(
        replies
            .iter()
            .filter(|reply| reply["error"]["kind"] == "busy")
            .count(),
        3,
        "{replies:?}"
    );
    eprintln!(
        "RP2_HELD_PROBES={}",
        json!({"samples":samples.len(),"p50Micros":p50,"p95Micros":p95,"p99Micros":p99,
        "maxMicros":max,"peerStarts":1,
        "timelineMicros":{"entrySeen":entry_seen_micros,
            "allProbesFinished":probe_finished_micros,
            "releaseSynced":release_micros,"exitSeen":exit_seen_micros},
        "probeLoopMillis":(probe_finished_micros-entry_seen_micros)/1000,
        "observedPeerHoldMillis":observed_hold_millis,
        "rawSamplesMicros":samples,
        "totalMillis":started.elapsed().as_millis(),"busyRefusals":3,"statusTimeouts":0,
        "hostSha256":sha256_bytes(&fs::read(&f.host).unwrap())})
    );
    c.finish();

    // Cold reconstruction observes the exact original attempt. Neither a
    // recovery command nor a replayed execute can regain fresh entry.
    let mut cold = f.start(&[("RP2_RELAY_WITNESS", witness.to_str().unwrap())]);
    let cold_deadline = Instant::now() + Duration::from_secs(180);
    let current = loop {
        cold.send(json!({"version":1,"requestId":"cold-ready","operation":"status"}));
        let response = cold.receive();
        assert!(
            response["result"]["lastObservation"]["startupError"].is_null(),
            "{response}"
        );
        if response["result"]["busy"] == false {
            break response;
        }
        assert!(
            Instant::now() < cold_deadline,
            "qualified cold reopen timed out: {response}"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    let latest = current["result"]["lastObservation"]["revision"]
        .as_str()
        .unwrap();
    cold.domain("recover_initial", latest);
    let recovery = cold.rx.recv_timeout(Duration::from_secs(180)).unwrap();
    assert_eq!(
        recovery["error"]["kind"], "execution_evidence",
        "{recovery}"
    );
    assert_eq!(
        recovery["error"]["message"],
        "unsupported evidence contract: evidence class outside installed reader profile",
        "{recovery}"
    );
    cold.domain("execute_initial", latest);
    let replay = cold.rx.recv_timeout(Duration::from_secs(180)).unwrap();
    assert!(
        !replay["error"].is_null(),
        "recovery minted a fresh launch: {replay}"
    );
    assert_eq!(fs::read_to_string(&witness).unwrap().lines().count(), 1);
    cold.finish();

    let manifest = f
        .temp
        .path()
        .join("candidate-private/boundary-manifest.json");
    let original = fs::read(&manifest).unwrap();
    fs::write(&manifest, b"{}").unwrap();
    assert!(
        review_workflow::startup::TrustedStartup::load(&f.startup).is_err(),
        "changed frozen manifest must refuse startup"
    );
    fs::write(&manifest, original).unwrap();
    let mut envelope: QualifiedStartupDocument =
        serde_json::from_slice(&fs::read(&f.startup).unwrap()).unwrap();
    let acceptance = envelope.qualified.offline_acceptance.take().unwrap().path;
    fs::set_permissions(&acceptance, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&acceptance, b"{}").unwrap();
    fs::set_permissions(&acceptance, fs::Permissions::from_mode(0o400)).unwrap();
    assert!(
        review_workflow::startup::TrustedStartup::load(&f.startup).is_err(),
        "changed accepted input must refuse startup"
    );
}

#[cfg(feature = "test-faults")]
#[test]
fn qualified_episode_begin_cut_cold_recovery_cannot_reenter() {
    let (f, witness) = offline_fixture();
    let initial = f.initialize();
    let revision = initial["campaign"]["revision"].as_str().unwrap();
    let cut_dir = f.temp.path().join("episode-cut");
    fs::create_dir(&cut_dir).unwrap();
    let mut first = f.start(&[
        ("REVIEW_HOST_FAULT_CUT", "after_episode_begin"),
        ("REVIEW_HOST_FAULT_DIR", cut_dir.to_str().unwrap()),
        ("RP2_RELAY_WITNESS", witness.to_str().unwrap()),
    ]);
    let startup_deadline = Instant::now() + Duration::from_secs(180);
    loop {
        first.send(json!({"version":1,"requestId":"cut-ready","operation":"status"}));
        let status = first.receive();
        assert!(
            status["result"]["lastObservation"]["startupError"].is_null(),
            "{status}"
        );
        if status["result"]["busy"] == false {
            break;
        }
        assert!(
            Instant::now() < startup_deadline,
            "qualified cut startup timed out"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    first.domain("execute_initial", revision);
    let cut = cut_dir.join("after_episode_begin.ready");
    let cut_deadline = Instant::now() + Duration::from_secs(180);
    while !cut.exists() {
        assert!(
            Instant::now() < cut_deadline,
            "episode begin cut was not reached"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!witness.exists(), "peer reached CBM before the episode cut");
    first.child.kill().unwrap();
    first.child.wait().unwrap();
    drop(first);

    let mut cold = f.start(&[("RP2_RELAY_WITNESS", witness.to_str().unwrap())]);
    let reopen_deadline = Instant::now() + Duration::from_secs(180);
    let current = loop {
        cold.send(json!({"version":1,"requestId":"cut-reopen","operation":"status"}));
        let status = cold.receive();
        assert!(
            status["result"]["lastObservation"]["startupError"].is_null(),
            "{status}"
        );
        if status["result"]["busy"] == false {
            break status;
        }
        assert!(
            Instant::now() < reopen_deadline,
            "qualified cut reopen timed out"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    let latest = current["result"]["lastObservation"]["revision"]
        .as_str()
        .unwrap();
    cold.domain("recover_initial", latest);
    let recovery = cold.rx.recv_timeout(Duration::from_secs(180)).unwrap();
    assert_eq!(recovery["result"]["status"], "unresolved", "{recovery}");
    assert_eq!(recovery["result"]["launchAuthorized"], false, "{recovery}");
    cold.domain("execute_initial", latest);
    let replay = cold.rx.recv_timeout(Duration::from_secs(180)).unwrap();
    assert!(!replay["error"].is_null(), "cut replay launched: {replay}");
    assert!(
        !witness.exists(),
        "cold recovery or replay entered the peer"
    );
    cold.finish();
}
