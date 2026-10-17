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
    let parent = Path::new("/home/bline/code/.work-engine-tmp/rp0-20261013/tests");
    fs::create_dir_all(parent).unwrap();
    tempfile::Builder::new()
        .prefix("rp0-")
        .tempdir_in(parent)
        .unwrap()
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
