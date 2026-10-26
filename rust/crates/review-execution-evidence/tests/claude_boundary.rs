//! RP0 has no provider call. This test executes a hostile local peer and a real,
//! candidate-private Codebase Memory daemon in the selected Linux boundary.

use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use review_execution_evidence::claude::{
    CandidateBoundary, CredentialSource, PinnedProgram, Rp0Error, TrustedRp0Profile,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const CBM: &str = "/home/bline/.local/bin/codebase-memory-mcp";
const CBM_SHA: &str = "e1adfa5c41f2e64c9d05620f660ebf9975f91deabc6fd72d4289da3e051a4872";

fn digest(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

fn fixture_root() -> tempfile::TempDir {
    let selected = std::env::var_os("RP0_TEST_TMPDIR");
    let parent = selected.as_deref().map(Path::new).unwrap_or(Path::new(
        "/home/bline/code/.work-engine-tmp/rp0-20261013/tests",
    ));
    fs::create_dir_all(parent).unwrap();
    tempfile::Builder::new()
        .prefix("rp0-")
        .tempdir_in(parent)
        .unwrap()
}

#[test]
fn schema_two_accounts_for_graph_gaps_without_claiming_graph_completion() {
    let temporary = fixture_root();
    let candidate = temporary.path().join("candidate");
    fs::create_dir(&candidate).unwrap();
    fs::write(candidate.join("lib.rs"), b"pub fn retained() {}\n").unwrap();
    fs::write(candidate.join("LICENSE"), b"Exact source fallback text.\n").unwrap();
    fs::write(candidate.join("opaque.bin"), [0, 1, 2, 3]).unwrap();
    let private = temporary.path().join("private");
    let boundary = CandidateBoundary::prepare(
        profile(temporary.path()),
        &candidate,
        &private,
        "rp0_gap_fixture",
    )
    .unwrap();
    assert!(!boundary.index().coverage_complete());
    let (boundary, manifest_digest) = boundary.publish_manifest().unwrap();
    assert_eq!(boundary.manifest_schema(), 2);
    let reopened = CandidateBoundary::open_frozen(
        profile(temporary.path()),
        &candidate,
        &private,
        &manifest_digest,
    )
    .unwrap();
    assert_eq!(
        reopened.index().store_sha256(),
        boundary.index().store_sha256()
    );
    assert!(!reopened.index().coverage_complete());
    let coverage_path = private.join("session/coverage.json");
    let manifest_path = private.join("boundary-manifest.json");
    let original_coverage: Value =
        serde_json::from_slice(&fs::read(&coverage_path).unwrap()).unwrap();
    let original_manifest_bytes = fs::read(&manifest_path).unwrap();
    let original_manifest: Value = serde_json::from_slice(&original_manifest_bytes).unwrap();
    let mut invalid_legacy = original_manifest.clone();
    invalid_legacy["schemaVersion"] = Value::from(1);
    invalid_legacy
        .as_object_mut()
        .unwrap()
        .remove("fallbackAccounted");
    let invalid_legacy_bytes = serde_json::to_vec(&invalid_legacy).unwrap();
    fs::write(&manifest_path, &invalid_legacy_bytes).unwrap();
    let invalid_legacy_digest = format!("{:x}", Sha256::digest(&invalid_legacy_bytes));
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &invalid_legacy_digest,
        )
        .is_err(),
        "an incomplete schema-2 graph cannot be downgraded into schema-1 admission"
    );
    fs::write(&manifest_path, &original_manifest_bytes).unwrap();
    assert_eq!(original_coverage["schema_version"], 2);
    let fallback = original_coverage["fallback"].as_array().unwrap();
    assert!(fallback.iter().any(|record| {
        record["path"] == "LICENSE"
            && record["disposition"] == "direct_source_required"
            && record["scope"]["whole_file"] == true
            && record["source_sha256"] == digest(&candidate.join("LICENSE"))
    }));
    assert!(
        original_coverage["opaque_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| {
                record["path"] == "opaque.bin"
                    && record["reason"] == "opaque_binary_not_queried"
                    && record["source_sha256"] == digest(&candidate.join("opaque.bin"))
            })
    );
    let probe = boundary
        .run_hostile_probe(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/coverage_probe.py"),
        )
        .unwrap();
    assert_eq!(
        probe.exit_code,
        Some(0),
        "{}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let mount_report: Value = serde_json::from_slice(&probe.stdout).unwrap();
    assert_eq!(mount_report["ledger_sha256"], digest(&coverage_path));
    assert_eq!(mount_report["fallback_read"]["path"], fallback[0]["path"]);
    assert_eq!(mount_report["host_source_hidden"], true);
    assert_eq!(mount_report["provider_calls"], 0);

    let rehash = |coverage: &Value| {
        // A matching new manifest hash removes the trivial stale-link failure
        // so the reopened reader must decide the ledger semantics itself.
        let coverage_bytes = serde_json::to_vec_pretty(coverage).unwrap();
        fs::write(&coverage_path, &coverage_bytes).unwrap();
        let mut manifest = original_manifest.clone();
        manifest["index"]["coverageSha256"] =
            Value::String(format!("{:x}", Sha256::digest(&coverage_bytes)));
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        fs::write(&manifest_path, &manifest_bytes).unwrap();
        format!("{:x}", Sha256::digest(&manifest_bytes))
    };
    let valid_rehashed_digest = rehash(&original_coverage);
    CandidateBoundary::open_frozen(
        profile(temporary.path()),
        &candidate,
        &private,
        &valid_rehashed_digest,
    )
    .unwrap();
    // The pinned CBM reports metadata_changed when file metadata drifts. It
    // remains a direct-source obligation even when the selected bytes match.
    let mut metadata_changed = original_coverage.clone();
    let records = metadata_changed["batches"][0]["paths"]
        .as_array_mut()
        .unwrap();
    let source = records
        .iter_mut()
        .find(|record| record["path"] == "lib.rs")
        .unwrap();
    assert_eq!(source["freshness"], "metadata_match");
    source["freshness"] = Value::String("metadata_changed".into());
    metadata_changed["fallback"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "path":"lib.rs", "source_sha256":digest(&candidate.join("lib.rs")),
            "status":"no_recorded_issue", "freshness":"metadata_changed",
            "content_verified":true, "reported_coverage":[],
            "scope":{"whole_file":true}, "disposition":"direct_source_required"
        }));
    let metadata_changed_digest = rehash(&metadata_changed);
    let metadata_reopened = CandidateBoundary::open_frozen(
        profile(temporary.path()),
        &candidate,
        &private,
        &metadata_changed_digest,
    )
    .unwrap();
    assert!(!metadata_reopened.index().coverage_complete());
    let mut unknown_freshness = metadata_changed.clone();
    let changed = unknown_freshness["batches"][0]["paths"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|record| record["path"] == "lib.rs")
        .unwrap();
    changed["freshness"] = Value::String("unrecognized_freshness".into());
    unknown_freshness["fallback"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|record| record["path"] == "lib.rs")
        .unwrap()["freshness"] = Value::String("unrecognized_freshness".into());
    let bad_digest = rehash(&unknown_freshness);
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &bad_digest,
        )
        .is_err(),
        "unrecognized freshness cannot become direct-source fallback"
    );
    let mut false_disposition = original_coverage.clone();
    false_disposition["fallback"][0]["disposition"] = Value::String("graph_no_recorded_gap".into());
    let bad_digest = rehash(&false_disposition);
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &bad_digest
        )
        .is_err()
    );
    let mut missing_path = original_coverage.clone();
    missing_path["source_hashes"].as_array_mut().unwrap().pop();
    let bad_digest = rehash(&missing_path);
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &bad_digest
        )
        .is_err()
    );
    let mut wrong_record_path = original_coverage.clone();
    wrong_record_path["batches"][0]["paths"][0]["path"] =
        Value::String("an-unselected-file.rs".into());
    let bad_digest = rehash(&wrong_record_path);
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &bad_digest
        )
        .is_err(),
        "a rehashed ledger cannot substitute a selected path"
    );
    let mut unknown_status = original_coverage.clone();
    unknown_status["batches"][0]["paths"][0]["status"] =
        Value::String("new_unrecognized_status".into());
    let bad_digest = rehash(&unknown_status);
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &bad_digest
        )
        .is_err(),
        "unknown graph states cannot be normalized into source fallback"
    );
    let mut missing_batch = original_coverage.clone();
    missing_batch["batches"].as_array_mut().unwrap().clear();
    let bad_digest = rehash(&missing_batch);
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &bad_digest
        )
        .is_err(),
        "all selected nonbinary paths require an exact graph response"
    );
    let mut duplicate_path = original_coverage.clone();
    let first_path = duplicate_path["batches"][0]["paths"][0]["path"].clone();
    duplicate_path["batches"][0]["paths"][1]["requested_path"] = first_path.clone();
    duplicate_path["batches"][0]["paths"][1]["path"] = first_path;
    let bad_digest = rehash(&duplicate_path);
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &bad_digest
        )
        .is_err(),
        "a duplicate graph path cannot stand in for another selected path"
    );
    let mut extra_path = original_coverage.clone();
    let mut extra = extra_path["batches"][0]["paths"][0].clone();
    extra["requested_path"] = Value::String("not-in-candidate.rs".into());
    extra["path"] = Value::String("not-in-candidate.rs".into());
    extra_path["batches"][0]["paths"]
        .as_array_mut()
        .unwrap()
        .push(extra);
    extra_path["batches"][0]["path_total"] = Value::from(3);
    extra_path["batches"][0]["path_returned"] = Value::from(3);
    let bad_digest = rehash(&extra_path);
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &bad_digest
        )
        .is_err(),
        "an extra graph path cannot expand the selected tree"
    );
    let mut invalid_range = original_coverage.clone();
    let reported = serde_json::json!({"kind":"parse_partial","match":"exact",
        "path":"LICENSE","ranges":[{"start":999,"end":999}]});
    invalid_range["batches"][0]["paths"][0]["coverage"] = Value::Array(vec![reported.clone()]);
    invalid_range["fallback"][0]["reported_coverage"] = Value::Array(vec![reported]);
    invalid_range["fallback"][0]["scope"] = serde_json::json!({"ranges":[{"start":999,"end":999}]});
    let bad_digest = rehash(&invalid_range);
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &bad_digest
        )
        .is_err(),
        "a rehashed out-of-file range cannot become a fallback obligation"
    );
    let mut wrong_gap_path = original_coverage.clone();
    let reported = serde_json::json!({"kind":"parse_partial","match":"exact",
        "path":"an-unselected-file.rs","ranges":[{"start":1,"end":1}]});
    wrong_gap_path["batches"][0]["paths"][0]["coverage"] = Value::Array(vec![reported.clone()]);
    wrong_gap_path["fallback"][0]["reported_coverage"] = Value::Array(vec![reported]);
    wrong_gap_path["fallback"][0]["scope"] = serde_json::json!({"ranges":[{"start":1,"end":1}]});
    let bad_digest = rehash(&wrong_gap_path);
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &bad_digest
        )
        .is_err(),
        "a graph gap cannot name a different source path"
    );
    let restored_digest = rehash(&original_coverage);
    fs::write(candidate.join("opaque.bin"), [9, 1, 2, 3]).unwrap();
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &restored_digest
        )
        .is_err()
    );
    fs::write(candidate.join("opaque.bin"), [0, 1, 2, 3]).unwrap();
    fs::remove_file(&coverage_path).unwrap();
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &restored_digest
        )
        .is_err(),
        "missing coverage ledger cannot reopen a schema-2 boundary"
    );
}

fn make_fifo(path: &Path) {
    unsafe extern "C" {
        fn mkfifo(pathname: *const std::ffi::c_char, mode: u32) -> i32;
    }
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { mkfifo(name.as_ptr(), 0o600) }, 0);
}

fn profile(owned_root: &Path) -> TrustedRp0Profile {
    let bwrap = Path::new("/usr/bin/bwrap");
    let launcher = owned_root.join("claude_cbm_restricted");
    fs::copy(env!("CARGO_BIN_EXE_claude_cbm_restricted"), &launcher).unwrap();
    fs::set_permissions(&launcher, fs::Permissions::from_mode(0o700)).unwrap();
    let client = owned_root.join("claude_cbm_client");
    fs::copy(env!("CARGO_BIN_EXE_claude_cbm_client"), &client).unwrap();
    fs::set_permissions(&client, fs::Permissions::from_mode(0o700)).unwrap();
    TrustedRp0Profile::load(
        PinnedProgram::load(bwrap, &digest(bwrap)).unwrap(),
        PinnedProgram::load(CBM, CBM_SHA).unwrap(),
        PinnedProgram::load(&launcher, &digest(&launcher)).unwrap(),
        PinnedProgram::load(&client, &digest(&client)).unwrap(),
    )
    .unwrap()
}

#[test]
fn candidate_private_cbm_hostile_process() {
    let temporary = fixture_root();
    let candidate = temporary.path().join("candidate");
    fs::create_dir(&candidate).unwrap();
    fs::write(
        candidate.join("lib.rs"),
        "pub fn example() -> usize { 1 }\n",
    )
    .unwrap();
    fs::create_dir(candidate.join(".git")).unwrap();
    fs::write(
        candidate.join(".git/private-admin-canary"),
        b"not selected source",
    )
    .unwrap();
    let boundary = CandidateBoundary::prepare(
        profile(temporary.path()),
        &candidate,
        temporary.path().join("private"),
        "rp0_fixture",
    )
    .unwrap();
    // The permanent host must reopen the exact durable, candidate-private
    // boundary; neither an index sidecar nor a guessed manifest is sufficient.
    let private = temporary.path().join("private");
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &"0".repeat(64)
        )
        .is_err()
    );
    let (boundary, manifest_digest) = boundary.publish_manifest().unwrap();
    let reopened = CandidateBoundary::open_frozen(
        profile(temporary.path()),
        &candidate,
        &private,
        &manifest_digest,
    )
    .unwrap();
    assert_eq!(
        reopened.index().store_sha256(),
        boundary.index().store_sha256()
    );
    assert!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &"1".repeat(64)
        )
        .is_err()
    );
    assert!(!boundary.index().candidate_tree_sha256().is_empty());
    assert!(boundary.index().nodes() > 0);
    assert!(boundary.index().coverage_complete());
    // A clean legacy envelope retains schema-1 admission semantics. The
    // reader must not require the new fallback marker from older manifests.
    let manifest_path = private.join("boundary-manifest.json");
    let schema_two_bytes = fs::read(&manifest_path).unwrap();
    let mut schema_one: Value = serde_json::from_slice(&schema_two_bytes).unwrap();
    schema_one["schemaVersion"] = Value::from(1);
    schema_one
        .as_object_mut()
        .unwrap()
        .remove("fallbackAccounted");
    let schema_one_bytes = serde_json::to_vec(&schema_one).unwrap();
    fs::write(&manifest_path, &schema_one_bytes).unwrap();
    let schema_one_digest = format!("{:x}", Sha256::digest(&schema_one_bytes));
    assert_eq!(
        CandidateBoundary::open_frozen(
            profile(temporary.path()),
            &candidate,
            &private,
            &schema_one_digest,
        )
        .unwrap()
        .manifest_schema(),
        1
    );
    fs::write(&manifest_path, schema_two_bytes).unwrap();
    assert!(!boundary.index().coverage_sha256().is_empty());
    assert!(boundary.mcp_config_path().is_file());
    let peer = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/cbm_probe.py");
    let output = boundary.run_hostile_probe(peer).unwrap();
    assert_eq!(
        output.exit_code,
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    eprintln!("RP0_PEER_REPORT={report}");
    eprintln!("RP0_CBM_OBSERVATION={}", output.cbm_observation);
    let binding = serde_json::json!({
        "project": boundary.index().project(),
        "candidate_tree_sha256": boundary.index().candidate_tree_sha256(),
        "store_sha256": boundary.index().store_sha256(),
        "coverage_sha256": boundary.index().coverage_sha256(),
        "coverage_complete": boundary.index().coverage_complete(),
        "indexed_at": boundary.index().indexed_at(),
        "nodes": boundary.index().nodes(),
        "edges": boundary.index().edges(),
        "cbm_sha256": CBM_SHA,
    });
    eprintln!("RP0_INDEX_BINDING={binding}");
    if let Ok(evidence_dir) = std::env::var("RP0_EVIDENCE_DIR") {
        let evidence = Path::new(&evidence_dir);
        fs::create_dir_all(evidence).unwrap();
        fs::write(
            evidence.join("peer-report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        fs::write(
            evidence.join("cbm-observation.json"),
            serde_json::to_vec_pretty(&output.cbm_observation).unwrap(),
        )
        .unwrap();
        fs::write(
            evidence.join("index-binding.json"),
            serde_json::to_vec_pretty(&binding).unwrap(),
        )
        .unwrap();
        for (source, target) in [
            (candidate.join("lib.rs"), "candidate-lib.rs"),
            (
                temporary.path().join("private/review-cache/rp0_fixture.db"),
                "rp0_fixture.db",
            ),
            (
                temporary.path().join("private/review-cache/config.json"),
                "config.json",
            ),
            (
                temporary.path().join("private/review-cache/_config.db"),
                "_config.db",
            ),
            (
                temporary.path().join("private/session/coverage.json"),
                "coverage.json",
            ),
            (boundary.mcp_config_path(), "mcp.json"),
        ] {
            fs::copy(source, evidence.join(target)).unwrap();
        }
    }
    for field in [
        "candidate_write_refused",
        "cache_unmounted",
        "direct_index_refused",
        "mcp_config_write_refused",
        "hostile_query_refused",
        "other_project_refused",
        "mutation_refused",
    ] {
        assert_eq!(report[field], true, "{field}: {report}");
    }
    for phase in ["daemon_initial", "daemon_after_reload_interval"] {
        let processes = report[phase].as_array().unwrap();
        assert!(
            processes.is_empty(),
            "CBM process crossed into reviewer namespace"
        );
    }
    for phase in ["after_index_status", "after_reload_interval"] {
        let processes = output.cbm_observation[phase].as_array().unwrap();
        assert!(processes.len() >= 2);
        for process in processes {
            assert_eq!(process["no_new_privs"], "1");
            assert_eq!(process["seccomp"], "2");
            assert_eq!(process["separate_net_namespace"], true);
            assert_eq!(process["only_loopback"], true);
            assert_eq!(process["tcp_listeners"], 0);
            for name in [
                "/cache/rp0_fixture.db",
                "/cache/config.json",
                "/cache/_config.db",
            ] {
                assert_eq!(process["protected_mounts"][name]["mount_readonly"], true);
                assert_eq!(process["protected_mounts"][name]["write_open_errno"], 30);
            }
            assert_eq!(
                process["protected_mounts"]["/repo/candidate"]["mount_readonly"],
                true
            );
        }
    }
    let forwarded = output.cbm_observation["backend_forwarded"]
        .as_array()
        .unwrap();
    assert!(
        output.cbm_observation["mcp_request_count"]
            .as_u64()
            .unwrap()
            > 64
    );
    for (id, tool) in [
        (8, "query_graph"),
        (9, "check_index_coverage"),
        (10, "get_code_snippet"),
    ] {
        assert!(
            forwarded
                .iter()
                .any(|entry| entry["id"] == id && entry["tool"] == tool),
            "admitted hostile request {id} did not reach CBM backend"
        );
    }
    boundary.verify_frozen().unwrap();
    fs::write(
        candidate.join("lib.rs"),
        "pub fn example() -> usize { 2 }\n",
    )
    .unwrap();
    assert!(
        boundary.verify_frozen().is_err(),
        "candidate drift must fail closed"
    );
    fs::write(
        candidate.join("lib.rs"),
        "pub fn example() -> usize { 1 }\n",
    )
    .unwrap();
    boundary.verify_frozen().unwrap();
    let mcp_config = boundary.mcp_config_path();
    let mcp_original = fs::read(&mcp_config).unwrap();
    fs::write(&mcp_config, b"{}").unwrap();
    assert!(
        boundary.verify_frozen().is_err(),
        "MCP configuration drift must fail closed"
    );
    fs::write(&mcp_config, mcp_original).unwrap();
    let store = temporary.path().join("private/review-cache/rp0_fixture.db");
    fs::write(&store, b"mutated").unwrap();
    assert!(
        boundary.verify_frozen().is_err(),
        "index drift must fail closed"
    );
}

#[test]
fn relay_adverse_frames_fail_closed_and_preserve_frozen_store() {
    let temporary = fixture_root();
    let candidate = temporary.path().join("candidate");
    fs::create_dir(&candidate).unwrap();
    fs::write(
        candidate.join("lib.rs"),
        "pub fn example() -> usize { 1 }\n",
    )
    .unwrap();
    let boundary = CandidateBoundary::prepare(
        profile(temporary.path()),
        &candidate,
        temporary.path().join("private"),
        "rp0_fixture",
    )
    .unwrap();
    let peer = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/cbm_abuse_probe.py");
    for mode in ["oversize", "flood", "early_disconnect"] {
        eprintln!("RP0_ABUSE_MODE={mode}");
        fs::write(temporary.path().join("private/session/probe-mode"), mode).unwrap();
        let error = boundary
            .run_hostile_probe(&peer)
            .err()
            .expect("adverse relay probe must refuse");
        eprintln!("RP0_ABUSE_RESULT mode={mode} error={error:?}");
        if mode == "oversize" {
            assert!(matches!(&error, Rp0Error::OutputLimit));
        }
        boundary.verify_frozen().unwrap();
        assert!(
            !temporary
                .path()
                .join("private/proxy/analysis.sock")
                .exists()
        );
        let marker =
            fs::read_to_string(temporary.path().join("private/scratch/probe-abuse.json")).unwrap();
        let marker: Value = serde_json::from_str(&marker).unwrap();
        assert_eq!(marker["mode"], mode);
        assert_eq!(marker["stage"], "completed", "{mode} peer did not complete");
        if let Ok(evidence_dir) = std::env::var("RP0_EVIDENCE_DIR") {
            fs::create_dir_all(&evidence_dir).unwrap();
            fs::write(
                Path::new(&evidence_dir).join(format!("abuse-{mode}.json")),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "mode": mode, "marker": marker, "relay_error": format!("{error:?}"),
                    "frozen_verified": true, "socket_removed": true,
                }))
                .unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn private_root_inside_candidate_is_refused() {
    let temporary = fixture_root();
    let candidate = temporary.path().join("candidate");
    fs::create_dir(&candidate).unwrap();
    fs::write(candidate.join("lib.rs"), b"pub fn example() {}\n").unwrap();
    assert!(
        CandidateBoundary::prepare(
            profile(temporary.path()),
            &candidate,
            candidate.join("private"),
            "rp0_fixture"
        )
        .is_err()
    );
}

#[test]
fn credential_projection_uses_only_synthetic_access_token() {
    let temporary = fixture_root();
    let path = temporary.path().join("credential.json");
    fs::write(&path, r#"{"claudeAiOauth":{"accessToken":"access-canary","refreshToken":"refresh-canary","expiresAt":9000000},"other":"owner-canary"}"#).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let source = CredentialSource::new(&path).unwrap();
    let projection = source.read_access_only(1_000_000, 60_000).unwrap();
    assert_eq!(projection.expires_at_millis(), 9_000_000);
    assert_eq!(projection.with_token(str::to_owned), "access-canary");
    assert!(!format!("{}", projection.expires_at_millis()).contains("canary"));
    assert!(source.read_access_only(8_999_000, 60_000).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(source.read_access_only(1_000_000, 60_000).is_err());
    let link = temporary.path().join("link.json");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(CredentialSource::new(&link).is_err());
}

#[test]
fn credential_ancestor_replacement_is_refused_but_file_rotation_is_admitted() {
    let temporary = fixture_root();
    let admitted = temporary.path().join("admitted");
    let alternate = temporary.path().join("alternate");
    fs::create_dir(&admitted).unwrap();
    fs::create_dir(&alternate).unwrap();
    let source_path = admitted.join("credential.json");
    let alternate_path = alternate.join("credential.json");
    let value = |token: &str| {
        format!("{{\"claudeAiOauth\":{{\"accessToken\":\"{token}\",\"expiresAt\":9000000}}}}")
    };
    fs::write(&source_path, value("initial-canary")).unwrap();
    fs::write(&alternate_path, value("alternate-canary")).unwrap();
    for path in [&source_path, &alternate_path] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let source = CredentialSource::new(&source_path).unwrap();
    fs::rename(&source_path, admitted.join("rotated-away.json")).unwrap();
    fs::write(&source_path, value("rotated-canary")).unwrap();
    fs::set_permissions(&source_path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        source
            .read_access_only(1_000_000, 60_000)
            .unwrap()
            .with_token(str::to_owned),
        "rotated-canary"
    );
    fs::rename(&admitted, temporary.path().join("moved-admitted")).unwrap();
    std::os::unix::fs::symlink(&alternate, &admitted).unwrap();
    assert!(source.read_access_only(1_000_000, 60_000).is_err());
    fs::remove_file(&admitted).unwrap();
    fs::rename(&alternate, &admitted).unwrap();
    assert!(
        source.read_access_only(1_000_000, 60_000).is_err(),
        "a different real parent directory cannot replace the pinned source"
    );
}

#[test]
fn credential_fifo_at_construction_is_refused_without_writer() {
    let temporary = fixture_root();
    let fifo = temporary.path().join("credential.json");
    make_fifo(&fifo);
    assert!(CredentialSource::new(&fifo).is_err());
}

#[test]
fn credential_rotation_to_fifo_is_refused_without_writer() {
    let temporary = fixture_root();
    let path = temporary.path().join("credential.json");
    fs::write(
        &path,
        r#"{"claudeAiOauth":{"accessToken":"initial-canary","expiresAt":9000000}}"#,
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let source = CredentialSource::new(&path).unwrap();
    fs::rename(&path, temporary.path().join("rotated-away.json")).unwrap();
    make_fifo(&path);
    assert!(source.read_access_only(1_000_000, 60_000).is_err());
}

#[test]
fn pinned_program_rejects_drift_and_symlink() {
    let temporary = fixture_root();
    let executable = temporary.path().join("program");
    fs::write(&executable, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let pin = PinnedProgram::load(&executable, &digest(&executable)).unwrap();
    fs::write(&executable, b"#!/bin/sh\nexit 1\n").unwrap();
    assert!(pin.sha256() != digest(&executable));
    let link = temporary.path().join("link");
    std::os::unix::fs::symlink(&executable, &link).unwrap();
    assert!(PinnedProgram::load(&link, &digest(&executable)).is_err());
}
