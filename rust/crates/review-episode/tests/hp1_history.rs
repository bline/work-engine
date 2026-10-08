//! Explicit, ignored HP1 diagnostic against a copied controlled HOST scratch DB.
//! The source is an immutable disposable copy, never a live Work Engine DB.
#[path = "support/mod.rs"]
#[allow(dead_code)]
mod support;

use review_episode::admission::FixtureAdmission;
use review_episode::operation::Operation;
use review_episode::protocol::{DirectRequest, RequestSelection};
use review_episode::{Application, OperationResult};
use review_episode_core::codec::{JsValue, canonical_json};
use review_episode_core::identity::Revision;
use review_episode_store::{CopyManifest, EpisodeStore, StoreOptions, import_closed_copy};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Instant;

const EXPECTED_BYTES: usize = 31_207_216;
const EXPECTED_SHA: &str = "08be22b5348c34fdf0ba9ada1b8f9e013c56597efa7920cf786ff7d7f6ed8a66";
const KEY: &str = "2e9d3722aad220efcb56def04b839b0bba776d6ec6c7a0d6224759a9d0ce9a50";

fn root() -> PathBuf {
    PathBuf::from(std::env::var("HP1_DIAGNOSTIC_ROOT").expect("HP1_DIAGNOSTIC_ROOT"))
}

#[test]
#[ignore = "explicit controlled scratch import preparation"]
fn prepare_copied_history() {
    let source = PathBuf::from(std::env::var("HP1_DIAGNOSTIC_COPY").expect("HP1_DIAGNOSTIC_COPY"));
    let hash = format!("{:x}", Sha256::digest(std::fs::read(&source).unwrap()));
    assert_eq!(
        hash,
        "a47964d5442a512f0bcacd445866b665937ed8b67c9a9da56bc650de236600b1"
    );
    let manifest = CopyManifest {
        source,
        sha256: hash,
        provenance: "controlled prior HOST scratch SQLite backup; no live database".into(),
    };
    let report = import_closed_copy(&manifest, &root()).unwrap();
    assert!(report.byte_equal);
    println!(
        "HP1_IMPORT episodes={} rows={} source_sha={} dest_sha={}",
        report.episodes, report.rows, report.source_sha256, report.destination_sha256
    );
}

#[test]
#[ignore = "explicit 30-second HP1 diagnostic watchdog applies to this read probe"]
fn direct_full_history_probe() {
    let root = root();
    let mut seed = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let current = seed.read(KEY).unwrap().current.unwrap();
    let identity = current.identity().clone();
    let observed = current.revision().0.clone();
    drop(seed);
    let request = DirectRequest::new(
        RequestSelection::Offline {
            request_id: "hp1-history".into(),
            grant_id: "hp1-history-grant".into(),
            selection_revision: Revision("a".repeat(64)),
        },
        Operation::History { identity },
    )
    .unwrap();
    let registry = root.parent().unwrap().join("hp1-history-fixture.json");
    support::write_fixture(
        &registry,
        &root,
        request.request().envelope(),
        Some(&observed),
    );
    let port = FixtureAdmission::load(&registry, &root).unwrap();
    let store = EpisodeStore::open(&root, StoreOptions::default()).unwrap();
    let mut app = Application::new(store, port, 32 * 1024 * 1024);
    let before_rss = proc_status("VmHWM");
    let start = Instant::now();
    let result = app.execute_direct(&request).unwrap();
    let elapsed = start.elapsed();
    let (rows, bytes, sha, serialize_ms) = match &result {
        OperationResult::History { history, .. } => {
            let encode = Instant::now();
            let body = canonical_json(&JsValue::Array(
                history.iter().map(|s| s.value().clone()).collect(),
            ));
            (
                history.len(),
                body.len(),
                support::raw_sha(&body),
                encode.elapsed().as_millis(),
            )
        }
        _ => panic!("history operation returned another result"),
    };
    assert_eq!(rows, 4);
    assert_eq!(bytes, EXPECTED_BYTES);
    assert_eq!(sha, EXPECTED_SHA);
    println!(
        "HP1_HISTORY rows={rows} history_bytes={bytes} history_sha={sha} direct_elapsed_ms={} separate_history_encode_ms={serialize_ms} vmhwm_before_kib={before_rss:?} vmhwm_after_kib={:?} wire_reply_bytes={}",
        elapsed.as_millis(),
        proc_status("VmHWM"),
        result.encoded_len(request.request())
    );
}

fn proc_status(name: &str) -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    text.lines().find_map(|line| {
        let remainder = line.strip_prefix(&format!("{name}:"))?;
        remainder.split_whitespace().next()?.parse().ok()
    })
}
