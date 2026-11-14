use base64::{Engine, engine::general_purpose::STANDARD as B64};
use review_execution_evidence::pi::{Binding, LiveStore};
use serde_json::{Value, json};
use std::{
    io::Write,
    os::unix::net::UnixStream,
    time::{Duration, Instant},
};
use work_engine_manual_host::pi_native::{self as native, MAX_AGGREGATE, MAX_FRAME};

fn frame(bytes: &[u8]) -> Vec<u8> {
    let mut out = (bytes.len() as u32).to_be_bytes().to_vec();
    out.extend(bytes);
    out
}
fn decoded(bytes: &[u8], remaining: &mut usize) -> native::Result<Value> {
    let (mut r, mut w) = UnixStream::pair().unwrap();
    w.write_all(bytes).unwrap();
    drop(w);
    native::decode(&mut r, remaining)
}
#[test]
fn frame_roundtrip_partial_eof_duplicates_and_bounds() {
    let v = json!({"type":"snapshot","data":{"nested":[1,2,3]}});
    let b = native::encode(&v).unwrap();
    assert_eq!(decoded(&b, &mut MAX_AGGREGATE).unwrap(), v);
    for n in 0..b.len() {
        assert!(decoded(&b[..n], &mut MAX_AGGREGATE).is_err(), "cut {n}");
    }
    assert!(decoded(&frame(br#"{"x":1,"x":2}"#), &mut MAX_AGGREGATE).is_err());
    assert!(decoded(&[0, 0, 0, 0], &mut MAX_AGGREGATE).is_err());
    assert!(decoded(&((MAX_FRAME + 1) as u32).to_be_bytes(), &mut MAX_AGGREGATE).is_err());
    assert!(decoded(&b, &mut (b.len() - 1)).is_err());
    assert!(native::encode(&json!("x".repeat(MAX_FRAME))).is_err());
    let mut left = b.len();
    assert!(decoded(&b, &mut left).is_ok());
    assert_eq!(left, 0);
}
#[test]
fn pipe_deadline_is_finite_without_peer_progress() {
    let (mut r, _held_writer) = UnixStream::pair().unwrap();
    let start = Instant::now();
    assert_eq!(
        native::decode(&mut r, &mut MAX_AGGREGATE),
        Err("pipe_deadline")
    );
    assert!(start.elapsed() >= Duration::from_millis(native::DEADLINE_MS - 100));
    assert!(start.elapsed() < Duration::from_millis(native::DEADLINE_MS + 3000));
}
#[test]
fn joined_refuses_foreign_stale_and_wrong_kind() {
    let binding = Binding::native(
        "11111111-1111-4111-8111-111111111111".into(),
        "attempt".into(),
        1,
    );
    let good = json!({"type":"tool","binding":binding,"sequence":2,"stream":3});
    assert!(native::joined(&good, "tool", &binding, 2, 3).is_ok());
    for (key, value) in [
        ("sequence", json!(1)),
        ("stream", json!(2)),
        ("type", json!("tool-grant")),
        ("binding", json!({})),
    ] {
        let mut bad = good.clone();
        bad[key] = value;
        assert!(native::joined(&bad, "tool", &binding, 2, 3).is_err());
    }
}
#[test]
fn request_compression_denied_channels_and_recursive_exclusion() {
    let t = tempfile::tempdir().unwrap();
    let original = "original_synthetic_access";
    let store = LiveStore::create(
        &t.path().join("store"),
        original.as_bytes().to_vec(),
        Binding::native(
            "11111111-1111-4111-8111-111111111111".into(),
            "attempt".into(),
            1,
        ),
        u64::MAX,
    )
    .unwrap();
    let body = json!({"model":"gpt-6-luna","stream":true,"instructions":"bounded","input":[{"role":"user","content":[{"type":"input_text","text":"build"}]}]});
    let make = |raw: &[u8], encoding: Option<&str>| {
        let mut headers =
            json!({"authorization":"untrusted-marker","chatgpt-account-id":"untrusted-account"});
        if let Some(e) = encoding {
            headers["content-encoding"] = json!(e);
        }
        json!({"method":"POST","url":"https://untrusted.invalid/never-used","headers":headers,"body":B64.encode(raw)})
    };
    let raw = serde_json::to_vec(&body).unwrap();
    let checked = native::request(&make(&raw, None), &store).unwrap();
    let out: Value = serde_json::from_slice(&checked).unwrap();
    assert_eq!(out["input"], body["input"]);
    assert_eq!(out["store"], false);
    assert!(!String::from_utf8(checked).unwrap().contains("untrusted"));
    let compressed = zstd::stream::encode_all(raw.as_slice(), 1).unwrap();
    assert!(native::request(&make(&compressed, Some("zstd")), &store).is_ok());
    assert!(native::request(&make(&raw, Some("gzip")), &store).is_err());
    assert!(native::request(&make(b"invalid", Some("zstd")), &store).is_err());
    let bomb = zstd::stream::encode_all(vec![b'x'; native::MAX_REQUEST + 1].as_slice(), 1).unwrap();
    assert_eq!(
        native::request(&make(&bomb, Some("zstd")), &store),
        Err("request_decompressed_bound")
    );
    assert!(native::request(&make(br#"{"model":1,"model":2}"#, None), &store).is_err());
    for text in [
        original.to_owned(),
        B64.encode(original),
        json!({"nested":original}).to_string(),
    ] {
        let mut leak = body.clone();
        leak["input"][0]["content"][0]["text"] = json!(text);
        assert!(native::request(&make(&serde_json::to_vec(&leak).unwrap(), None), &store).is_err());
    }
}
