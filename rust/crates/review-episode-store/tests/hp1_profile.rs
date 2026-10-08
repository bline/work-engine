//! Isolated stage attribution for the exact controlled HOST history copy.
//! Run only with HP1_DIAGNOSTIC_ROOT; diagnostic watchdog is external.
use review_episode_core::codec::{JsValue, canonical_json};
use review_episode_store::validate_history;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use sha2::{Digest, Sha256};
use std::time::Instant;

#[test]
#[ignore = "explicit HP1 controlled-scratch diagnostic"]
fn exact_history_stages() {
    let root = std::path::PathBuf::from(std::env::var("HP1_DIAGNOSTIC_ROOT").unwrap());
    let key = "2e9d3722aad220efcb56def04b839b0bba776d6ec6c7a0d6224759a9d0ce9a50";
    let conn = Connection::open_with_flags(
        root.join("review-episodes.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let rss_start = proc_status("VmHWM");
    let fetch_start = Instant::now();
    let current: Option<(String, String)> = conn
        .query_row(
            "SELECT revision,state_json FROM review_episode_current WHERE identity_key=?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .unwrap();
    let mut statement = conn.prepare("SELECT sequence,identity_key,revision,predecessor_revision,state_json FROM review_episode_history WHERE identity_key=?1 ORDER BY sequence").unwrap();
    let rows: Vec<(i64, String, String, Option<String>, String)> = statement
        .query_map([key], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let raw_bytes: usize = rows.iter().map(|r| r.4.len()).sum();
    let fetch_ms = fetch_start.elapsed().as_millis();
    let rss_after_fetch = proc_status("VmHWM");
    let validate_start = Instant::now();
    let snapshot = validate_history(
        key,
        current.as_ref().map(|(r, j)| (r.as_str(), j.as_str())),
        rows,
    )
    .unwrap();
    let integrity_parse_ms = validate_start.elapsed().as_millis();
    let rss_after_integrity = proc_status("VmHWM");
    let clone_start = Instant::now();
    let values = JsValue::Array(
        snapshot
            .history
            .iter()
            .map(|h| h.state.value().clone())
            .collect(),
    );
    let materialize_clone_ms = clone_start.elapsed().as_millis();
    let rss_after_clone = proc_status("VmHWM");
    let encode_start = Instant::now();
    let json = canonical_json(&values);
    let serialize_ms = encode_start.elapsed().as_millis();
    let rss_after_encode = proc_status("VmHWM");
    let hash = format!("{:x}", Sha256::digest(json.as_bytes()));
    assert_eq!(json.len(), 31_207_216);
    assert_eq!(
        hash,
        "08be22b5348c34fdf0ba9ada1b8f9e013c56597efa7920cf786ff7d7f6ed8a66"
    );
    println!(
        "HP1_STAGES raw_json_bytes={raw_bytes} rows={} fetch_ms={fetch_ms} integrity_parse_ms={integrity_parse_ms} materialize_clone_ms={materialize_clone_ms} serialize_ms={serialize_ms} full_history_bytes={} sha256={hash} vmhwm_start_kib={rss_start:?} vmhwm_fetch_kib={rss_after_fetch:?} vmhwm_integrity_kib={rss_after_integrity:?} vmhwm_clone_kib={rss_after_clone:?} vmhwm_encode_kib={rss_after_encode:?}",
        snapshot.history.len(),
        json.len()
    );
}
fn proc_status(name: &str) -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    text.lines().find_map(|line| {
        line.strip_prefix(&format!("{name}:"))
            .and_then(|r| r.split_whitespace().next())
            .and_then(|s| s.parse().ok())
    })
}
