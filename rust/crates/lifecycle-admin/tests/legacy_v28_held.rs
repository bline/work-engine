// The existing synthetic schema-12 generator remains the A test owner. This
// wrapper adds held rows without changing the translator or its fixture file.
mod synthetic_a {
    include!("legacy_v28.rs");

    pub(crate) fn held_fixture() -> Fixture {
        let fixture = fixture();
        populate_added_domain_rows(&fixture);
        fixture
    }
}

use lifecycle_admin::legacy_v28::{translate_v28_capture, verify_completed_translation};
use lifecycle_admin::legacy_v28_held::inspect_verified_held_translation;
use lifecycle_store::SqliteLifecycleStore;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn archive_verified_closure(evidence: &lifecycle_store::HeldEvidence) {
    if let Ok(root) = std::env::var("S9_B1_SYNTHETIC_CLOSURE_ROOT") {
        let root = std::path::PathBuf::from(root);
        fs::create_dir(&root).unwrap();
        for artifact in &evidence.artifacts {
            let path = root.join(&artifact.name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, &artifact.raw_bytes).unwrap();
        }
    }
}

#[test]
fn held_a_v2_production_readback_supplies_only_read_only_evidence() {
    let fixture = synthetic_a::held_fixture();
    let result = translate_v28_capture(&fixture.attempt, &fixture.output).unwrap();
    verify_completed_translation(&fixture.attempt, &fixture.output, &result).unwrap();
    assert_eq!(
        result.capsule_version.as_deref(),
        Some("s9-v28-obligations-v2")
    );
    assert_eq!(result.transfer_eligibility, "blocked_unclassified");
    let evidence =
        inspect_verified_held_translation(&fixture.attempt, &fixture.output, &result).unwrap();
    assert_eq!(evidence.artifacts.len(), 16);
    let (total, unknown) = SqliteLifecycleStore::inspect_held_evidence(&evidence).unwrap();
    assert!(total >= unknown && unknown > 0);
    let mut altered = result.clone();
    altered.transfer_eligibility = "requires_separate_B_authority".into();
    assert!(
        inspect_verified_held_translation(&fixture.attempt, &fixture.output, &altered).is_err()
    );
    let mut forged = evidence.clone();
    forged
        .artifacts
        .iter_mut()
        .find(|a| a.name == "output/inventory-private.json")
        .unwrap()
        .raw_bytes
        .push(b' ');
    assert!(SqliteLifecycleStore::inspect_held_evidence(&forged).is_err());
}

#[test]
fn held_native_capture_production_a_v2_closure() {
    const SOURCE_SHA: &str = "969e52ee4254bd9e8a1f6ad2290b77e08d30f59f880da747ba145b3835e7d5e5";
    const BINARY_SHA: &str = "f9b67b1a05b1ab3e975e9c0fe012a1909d29e7f66ca54e77c092c68e6a1eaea9";
    let fixture = synthetic_a::held_fixture();
    // These are source SQLite rows. The pinned native capture and production A
    // derive every inventory identity and projection from their copied bytes.
    // 13/7 from held_fixture plus 160 reconciled and 101 scheduled = 274/108.
    {
        let db = fixture
            .attempt
            .join("capture/image/semantic-context.sqlite3");
        let conn = rusqlite::Connection::open(&db).unwrap();
        for index in 0..261 {
            let identity = format!("synthetic-held-active-{index:03}");
            let revision = format!("sha256:{}", sha(identity.as_bytes()));
            let status = if index < 160 {
                "reconciled"
            } else {
                "scheduled"
            };
            let state = json!({"synthetic":true,"identity":identity,"status":status});
            conn.execute(
                "INSERT INTO active_turn_lifecycle (identity_key,turn_key,revision,status,next_eligible_at,state_json) VALUES (?1,?2,?3,?4,NULL,?5)",
                rusqlite::params![
                    identity,
                    format!("synthetic-held-turn-{index:03}"),
                    revision,
                    status,
                    serde_json::to_string(&state).unwrap()
                ],
            )
            .unwrap();
        }
    }
    for name in [
        "semantic-context.sqlite3",
        "role-bindings.json",
        "generation-state.json",
        "switchboard-attachment.json",
    ] {
        fs::copy(
            fixture.attempt.join("capture/image").join(name),
            fixture.source.join(name),
        )
        .unwrap();
    }
    fs::remove_dir_all(&fixture.attempt).unwrap();
    fs::create_dir(&fixture.attempt).unwrap();
    let binary = std::path::PathBuf::from(
        std::env::var("S9_CAPTURE_BIN").expect("pinned synthetic native binary"),
    );
    assert_eq!(std::env::var("S9_CAPTURE_SHA256").unwrap(), BINARY_SHA);
    assert_eq!(sha(&fs::read(&binary).unwrap()), BINARY_SHA);
    let child_root = fixture.attempt.join("capture");
    let socket = fixture
        .attempt
        .parent()
        .unwrap()
        .join("absent-synthetic.sock");
    let mut child = Command::new(&binary)
        .arg("--source-root")
        .arg(&fixture.source)
        .arg("--output-root")
        .arg(&child_root)
        .arg("--socket-path")
        .arg(&socket)
        .arg("--operator")
        .arg("s9-a-synthetic-qualification")
        .arg("--deadline-seconds")
        .arg("120")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let limit = Instant::now() + Duration::from_secs(125);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= limit {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("native synthetic capture deadline");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success());
    let child_receipt: Value =
        serde_json::from_slice(&fs::read(child_root.join("receipt.json")).unwrap()).unwrap();
    assert_eq!(child_receipt["status"], "qualified_candidate_root");
    let manifest: Value =
        serde_json::from_slice(&fs::read(child_root.join("source-manifest.json")).unwrap())
            .unwrap();
    let mut artifacts = serde_json::Map::new();
    for name in [
        "semantic-context.sqlite3",
        "role-bindings.json",
        "generation-state.json",
        "switchboard-attachment.json",
    ] {
        let bytes = fs::read(child_root.join("image").join(name)).unwrap();
        let hash = sha(&bytes);
        assert_eq!(hash, sha(&fs::read(fixture.source.join(name)).unwrap()));
        assert!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |entry| entry["copy"] == json!(child_root.join("image").join(name))
                        && entry["copy_sha256"] == hash
                )
        );
        artifacts.insert(format!("capture/image/{name}"), json!({"sha256":hash}));
    }
    let absent = json!({"semantic-context.sqlite3-wal":{"present":false},"semantic-context.sqlite3-shm":{"present":false}});
    for sidecar in [
        "semantic-context.sqlite3-wal",
        "semantic-context.sqlite3-shm",
    ] {
        assert!(!Path::new(&fixture.source).join(sidecar).exists());
    }
    let outer = json!({
        "schema_version":1,"status":"qualified_candidate_root","raw_exit_status":0,
        "primary_error":null,"cleanup_error":null,
        "child_primary_error":child_receipt["primary_error"],"child_cleanup_error":child_receipt["cleanup_error"],
        "child_status":child_receipt["status"],"child_event_ledger":child_receipt["event_ledger"],
        "preflight_source_metadata":absent,"postflight_source_metadata":absent,
        "binary":{"sha256":BINARY_SHA},"source_aggregate_sha256":SOURCE_SHA,
        "child_receipt":{"sha256":sha(&fs::read(child_root.join("receipt.json")).unwrap())},
        "child_manifest_sha256":sha(&fs::read(child_root.join("source-manifest.json")).unwrap()),
        "source_hashes_before":child_receipt["source_hashes"],"source_hashes_after":child_receipt["source_hashes"],
        "selection_claim":"current_selection_unestablished","duration_seconds":0.381,
        "argv":[binary,"--source-root",fixture.source,"--output-root",child_root,
            "--socket-path",socket,"--operator","s9-a-synthetic-qualification","--deadline-seconds","120"],
        "artifacts":artifacts
    });
    let mut outer_bytes = serde_json::to_vec_pretty(&outer).unwrap();
    outer_bytes.push(b'\n');
    fs::write(fixture.attempt.join("attempt-receipt.json"), outer_bytes).unwrap();
    let result = translate_v28_capture(&fixture.attempt, &fixture.output).unwrap();
    verify_completed_translation(&fixture.attempt, &fixture.output, &result).unwrap();
    assert_eq!(
        result.capsule_version.as_deref(),
        Some("s9-v28-obligations-v2")
    );
    assert_eq!(result.transfer_eligibility, "blocked_unclassified");
    let evidence =
        inspect_verified_held_translation(&fixture.attempt, &fixture.output, &result).unwrap();
    assert_eq!(
        SqliteLifecycleStore::inspect_held_evidence(&evidence).unwrap(),
        (274, 108)
    );
    archive_verified_closure(&evidence);
}
