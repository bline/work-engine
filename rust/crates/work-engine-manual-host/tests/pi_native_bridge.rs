use base64::{Engine, engine::general_purpose::STANDARD as B64};
use review_execution_evidence::pi::{Binding, LiveStore};
use serde_json::json;
use work_engine_manual_host::pi_native::{self as native};
const A: &str = "SYNTHETIC_ORIGINAL_A_MUST_NEVER_BE_RETAINED";
fn binding() -> Binding {
    Binding::native(
        "11111111-1111-4111-8111-111111111111".into(),
        "test".into(),
        1,
    )
}
fn request() -> serde_json::Value {
    json!({"method":"POST","url":"https://evil.invalid","headers":{"authorization":"evil","chatgpt-account-id":"evil"},"body":B64.encode(serde_json::to_vec(&json!({"model":"gpt-6-luna","stream":true,"input":[{"role":"user","content":[{"type":"input_text","text":"hello"}]}]})).unwrap())})
}
#[test]
fn request_reconstructs_authority_and_decodes_zstd() {
    let t = tempfile::tempdir().unwrap();
    let s = LiveStore::create(
        &t.path().join("store"),
        A.as_bytes().to_vec(),
        binding(),
        u64::MAX,
    )
    .unwrap();
    let mut v = request();
    let plain = native::request(&v, &s).unwrap();
    let p: serde_json::Value = serde_json::from_slice(&plain).unwrap();
    assert_eq!(p["store"], false);
    assert!(p.get("url").is_none());
    assert!(p.get("headers").is_none());
    let raw = B64.decode(v["body"].as_str().unwrap()).unwrap();
    v["headers"]["content-encoding"] = json!("zstd");
    v["body"] = json!(B64.encode(zstd::stream::encode_all(raw.as_slice(), 1).unwrap()));
    assert_eq!(native::request(&v, &s).unwrap(), plain);
    v["body"] = json!(B64.encode(
        zstd::stream::encode_all(vec![b'x'; native::MAX_REQUEST + 1].as_slice(), 1).unwrap()
    ));
    assert!(native::request(&v, &s).is_err());
    v["body"] = json!("invalid!");
    assert!(native::request(&v, &s).is_err());
}
#[test]
fn injection_and_schema_refuse_before_effect() {
    let t = tempfile::tempdir().unwrap();
    let s = LiveStore::create(
        &t.path().join("store"),
        A.as_bytes().to_vec(),
        binding(),
        u64::MAX,
    )
    .unwrap();
    for body in [
        json!({"model":"gpt-6-luna","stream":true,"input":[{"role":"user","content":[{"type":"input_text","text":A}]}]}),
        json!({"model":"foreign","stream":true,"input":[]}),
        json!({"model":"gpt-6-luna","stream":true,"tools":[{"name":"exec"}],"input":[]}),
    ] {
        let mut v = request();
        v["body"] = json!(B64.encode(serde_json::to_vec(&body).unwrap()));
        assert!(native::request(&v, &s).is_err());
    }
}
#[test]
fn joins_do_not_accept_foreign_stale_stream() {
    let b = binding();
    let mut v = json!({"type":"fetch","binding":b,"sequence":1,"stream":1});
    assert!(native::joined(&v, "fetch", &b, 1, 1).is_ok());
    v["sequence"] = json!(2);
    assert!(native::joined(&v, "fetch", &b, 1, 1).is_err());
    v["sequence"] = json!(1);
    v["binding"]["attempt"] = json!("forged");
    assert!(native::joined(&v, "fetch", &b, 1, 1).is_err());
}
#[test]
fn frame_short_oversized_aggregate_refuse() {
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    for bytes in [
        vec![0, 0],
        vec![0, 0, 0, 0],
        u32::MAX.to_be_bytes().to_vec(),
        vec![0, 0, 0, 2, b'{'],
    ] {
        let (mut a, mut b) = UnixStream::pair().unwrap();
        a.write_all(&bytes).unwrap();
        drop(a);
        assert!(native::decode(&mut b, &mut native::MAX_AGGREGATE.clone()).is_err());
    }
    let (mut a, mut b) = UnixStream::pair().unwrap();
    a.write_all(&native::encode(&json!({"ok":true})).unwrap())
        .unwrap();
    assert!(native::decode(&mut b, &mut 1).is_err());
}
