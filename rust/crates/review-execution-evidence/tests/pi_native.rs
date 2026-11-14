use review_execution_evidence::pi::{Binding, LiveStore, SealedReader};
use serde_json::{Value, json};
const A: &str = "SYNTHETIC_ORIGINAL_A_MUST_NEVER_BE_RETAINED";
fn binding() -> Binding {
    Binding::native(
        "11111111-1111-4111-8111-111111111111".into(),
        "test-attempt".into(),
        1,
    )
}
fn header() -> Value {
    json!({"type":"session","version":3,"id":binding().session,"cwd":"/scratch","timestamp":"2026-11-14T00:00:00Z"})
}
fn entries() -> Vec<Value> {
    vec![
        json!({"type":"message","id":"a","parentId":null,"timestamp":"2026-11-14T00:00:00Z","message":{"role":"user","timestamp":1,"content":[{"type":"text","text":"hello"}]}}),
        json!({"type":"message","id":"b","parentId":"a","timestamp":"2026-11-14T00:00:00Z","message":{"role":"assistant","timestamp":2,"api":"openai-codex-responses","provider":"openai-codex","model":"gpt-6-luna","stopReason":"stop","usage":{"input":1,"output":1,"cacheRead":0,"cacheWrite":0,"totalTokens":2,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}},"content":[{"type":"text","text":"answer"}]}}),
    ]
}
#[test]
fn exclusion_before_first_snapshot_write() {
    for kind in ["key", "value", "escape", "metadata", "header"] {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("store");
        let mut s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
        s.begin(1, 1, None).unwrap();
        let mut e = entries();
        let mut h = header();
        let mut m = json!({});
        match kind {
            "key" => e[0]["message"][A] = json!(1),
            "value" => e[0]["message"]["content"] = json!(A),
            "escape" => {
                let escaped = A
                    .chars()
                    .map(|c| format!("\\u{:04x}", c as u32))
                    .collect::<String>();
                e[0]["message"]["content"] =
                    serde_json::from_str(&format!("\"{escaped}\"")).unwrap()
            }
            "metadata" => m = json!({"error":A}),
            "header" => h["injected"] = json!(A),
            _ => unreachable!(),
        };
        assert!(s.publish(1, h, e, "b".into(), m).is_err(), "{kind}");
        assert!(!root.join("snapshot-0001.json").exists());
    }
}
#[test]
fn original_survives_source_rotation_and_removal() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("access");
    std::fs::write(&source, A).unwrap();
    let mut s = LiveStore::create(
        &t.path().join("store"),
        std::fs::read(&source).unwrap(),
        binding(),
        u64::MAX,
    )
    .unwrap();
    std::fs::write(&source, "B_REPLACEMENT").unwrap();
    std::fs::remove_file(source).unwrap();
    assert!(s.check(&json!({"leak":A})).is_err());
    assert!(s.check(&json!({"safe":"B_REPLACEMENT"})).is_ok());
    let p = s.begin(99, 1, None).unwrap();
    assert!(
        s.dispatch::<()>(p, u64::MAX, |_| panic!("expired dispatch effect"))
            .is_err()
    );
    assert!(s.check(&json!({"leak":A})).is_err());
}
#[test]
fn exclusive_writer_and_create_only() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    assert!(LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).is_err());
    assert!(s.expected_seal().is_err());
    drop(s);
}
#[test]
fn stale_duplicate_and_foreign_joins_refuse() {
    let t = tempfile::tempdir().unwrap();
    let mut s = LiveStore::create(
        &t.path().join("store"),
        A.as_bytes().to_vec(),
        binding(),
        u64::MAX,
    )
    .unwrap();
    assert!(s.begin(1, 2, None).is_err());
    let p = s.begin(1, 1, None).unwrap();
    assert!(s.begin(1, 1, None).is_err());
    s.consume(p, 1).unwrap();
    let mut h = header();
    h["id"] = json!("foreign");
    assert!(s.publish(1, h, entries(), "b".into(), json!({})).is_err());
    assert!(
        s.publish(1, header(), entries(), "a".into(), json!({}))
            .is_err()
    );
    s.publish(1, header(), entries(), "b".into(), json!({}))
        .unwrap();
    assert!(s.begin(1, 1, None).is_err());
    assert!(s.begin(1, 2, Some("forged-hash")).is_err());
}
#[test]
fn live_prefix_and_exact_sealed_read() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let mut s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    s.begin(1, 1, None).unwrap();
    s.publish(1, header(), entries(), "b".into(), json!({}))
        .unwrap();
    let old = s.head().unwrap().to_owned();
    s.begin(1, 2, Some(&old)).unwrap();
    assert!(
        s.publish(2, header(), entries(), "b".into(), json!({}))
            .is_err()
    );
    let mut e = entries();
    e.push(json!({"type":"message","id":"c","parentId":"b","timestamp":"2026-11-14T00:00:00Z","message":{"role":"user","timestamp":3,"content":[{"type":"text","text":"second"}]}}));
    let seal = s
        .publish(2, header(), e.clone(), "c".into(), json!({}))
        .unwrap()
        .digest()
        .to_owned();
    let expected = s.expected_seal().unwrap();
    drop(s);
    let reader = SealedReader::open(&root, expected.clone()).unwrap();
    assert_eq!(reader.read(2).unwrap().digest(), seal);
    assert_eq!(reader.read(2).unwrap().entries(), e);
    drop(reader);
    assert!(
        SealedReader::open(&root, expected)
            .unwrap()
            .read(1)
            .is_err()
    );
}
#[test]
fn crash_intent_cannot_resume_or_resend() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let mut s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    s.begin(1, 1, None).unwrap();
    assert!(s.expected_seal().is_err());
    drop(s);
    assert!(LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).is_err());
}
#[test]
fn forged_bytes_and_symlink_refuse() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let mut s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    s.begin(1, 1, None).unwrap();
    s.publish(1, header(), entries(), "b".into(), json!({}))
        .unwrap();
    let expected = s.expected_seal().unwrap();
    drop(s);
    let p = root.join("snapshot-0001.json");
    let mut v: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    v["payload"]["metadata"] = json!({"forged":true});
    std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(
        SealedReader::open(&root, expected.clone())
            .unwrap()
            .read(1)
            .is_err()
    );
    std::fs::remove_file(&p).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", &p).unwrap();
    assert!(
        SealedReader::open(&root, expected.clone())
            .unwrap()
            .read(1)
            .is_err()
    );
}
#[test]
fn quota_refuses_before_retention() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let mut s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    s.begin(1, 1, None).unwrap();
    assert!(
        s.publish(
            1,
            header(),
            entries(),
            "b".into(),
            json!({"large":"x".repeat(2*1024*1024)})
        )
        .is_err()
    );
    assert!(!root.join("snapshot-0001.json").exists());
}

#[test]
fn backdated_caller_clock_cannot_revive_exposure() {
    let t = tempfile::tempdir().unwrap();
    let mut s =
        LiveStore::create(&t.path().join("store"), A.as_bytes().to_vec(), binding(), 1).unwrap();
    assert!(s.begin(0, 1, None).is_err());
    assert!(!t.path().join("store/intent-0001.json").exists());
}

#[test]
fn recursive_compositions_duplicates_and_all_lines() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let t = tempfile::tempdir().unwrap();
    let s = LiveStore::create(
        &t.path().join("store"),
        A.as_bytes().to_vec(),
        binding(),
        u64::MAX,
    )
    .unwrap();
    let escaped = A
        .chars()
        .map(|c| format!("\\u{:04x}", c as u32))
        .collect::<String>();
    let inner = format!("{{\"x\":\"{escaped}\",\"x\":\"discarded\"}}");
    for v in [
        json!({"nested":inner}),
        json!({"nested":STANDARD.encode(STANDARD.encode(A))}),
        json!({STANDARD.encode(A):"key"}),
        json!({"nested":format!("{}\n{{\"x\":\"{escaped}\"}}",vec!["{\"ok\":true}";256].join("\n"))}),
    ] {
        assert!(s.check(&v).is_err());
    }
    assert!(s.check(&json!({"useful": "{\"a\":1}\n{\"b\":2}"})).is_ok());
    assert!(
        s.check(&json!({"nested":STANDARD.encode("{\"safe\":true}")}))
            .is_ok()
    );
    assert!(s.check(&json!({"bad":"base64:###"})).is_err());
    assert!(review_execution_evidence::pi::parse_json(br#"{"a":{"x":1,"x":2}}"#).is_err());
}
#[test]
fn generated_name_collision_and_partial_charge() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let mut s =
        LiveStore::create(&root, b"snapshot-0001.json".to_vec(), binding(), u64::MAX).unwrap();
    s.begin(1, 1, None).unwrap();
    assert!(
        s.publish(1, header(), entries(), "b".into(), json!({}))
            .is_err()
    );
    assert!(!root.join("snapshot-0001.json").exists());
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let mut s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    std::fs::write(root.join("control-created.json"), b"occupied").unwrap();
    assert!(s.record_control("created", json!({})).is_err());
    assert!(s.begin(1, 1, None).is_err());
}
#[test]
fn root_replacement_foreign_pending_and_recomputed_seal() {
    let t = tempfile::tempdir().unwrap();
    let r = t.path().join("one");
    let mut a = LiveStore::create(&r, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    let b = LiveStore::create(
        &t.path().join("two"),
        A.as_bytes().to_vec(),
        binding(),
        u64::MAX,
    )
    .unwrap();
    let p = a.begin(1, 1, None).unwrap();
    assert!(b.consume(p, 1).is_err());
    a.publish(1, header(), entries(), "b".into(), json!({}))
        .unwrap();
    let expected = a.expected_seal().unwrap();
    drop(a);
    let path = r.join("snapshot-0001.json");
    let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["payload"]["metadata"] = json!({"replacement":true});
    use sha2::{Digest, Sha256};
    v["sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&v["payload"]).unwrap())
    ));
    std::fs::write(path, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(SealedReader::open(&r, expected).unwrap().read(1).is_err());
    let r = t.path().join("root");
    let mut s = LiveStore::create(&r, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    std::fs::rename(&r, t.path().join("old")).unwrap();
    std::fs::create_dir(&r).unwrap();
    assert!(s.begin(1, 1, None).is_err());
}

#[test]
fn aggregate_includes_controls_and_reserves_closure() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let mut s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    let large = "!".repeat(1800000);
    for label in ["create-intent", "created", "ready"] {
        s.record_control(label, json!({"data":large})).unwrap();
    }
    let mut e = entries();
    let mut predecessor = None;
    for seq in 1..=2 {
        s.begin(1, seq, predecessor.as_deref()).unwrap();
        if seq > 1 {
            e.push(json!({"type":"message","id":"c","parentId":"b","timestamp":"2026-11-14T00:00:00Z","message":{"role":"user","timestamp":3,"content":[{"type":"text","text":"next"}]}}));
        }
        s.publish(
            seq,
            header(),
            e.clone(),
            if seq == 1 { "b" } else { "c" }.into(),
            json!({"data":large}),
        )
        .unwrap();
        predecessor = s.head().map(str::to_owned);
    }
    s.begin(1, 3, predecessor.as_deref()).unwrap();
    e.push(json!({"type":"message","id":"d","parentId":"c","timestamp":"2026-11-14T00:00:00Z","message":{"role":"user","timestamp":4,"content":[{"type":"text","text":"next"}]}}));
    assert!(
        s.publish(3, header(), e, "d".into(), json!({"data":large}))
            .is_err()
    );
    assert!(!root.join("snapshot-0003.json").exists());
    s.record_control("closure", json!({"data":large})).unwrap();
}
#[test]
fn complete_bounds_and_native_missing_payload_refuse() {
    let t = tempfile::tempdir().unwrap();
    let mut s = LiveStore::create(
        &t.path().join("store"),
        A.as_bytes().to_vec(),
        binding(),
        u64::MAX,
    )
    .unwrap();
    let mut nested = json!("safe");
    for _ in 0..40 {
        nested = json!([nested]);
    }
    assert!(s.check(&nested).is_err());
    assert!(
        s.check(&json!({"encoding":"rot13","data":"undeclared"}))
            .is_err()
    );
    assert!(
        s.check(&json!({"encoding":"base64","data":"bad###"}))
            .is_err()
    );
    assert!(
        s.check(&json!({"encoding":"jsonl","data":vec!["{\"safe\":true}";257].join("\n")}))
            .is_err()
    );
    assert!(
        s.check(&json!({"encoding":"json","data":"{\"safe\":true}"}))
            .is_ok()
    );
    s.begin(1, 1, None).unwrap();
    let mut e = entries();
    e[0].as_object_mut().unwrap().remove("parentId");
    assert!(s.publish(1, header(), e, "b".into(), json!({})).is_err());
    let mut e = entries();
    e[0].as_object_mut().unwrap().remove("message");
    assert!(s.publish(1, header(), e, "b".into(), json!({})).is_err());
    let mut h = header();
    h["timestamp"] = json!("9999-99-99T99:99:99Z");
    assert!(s.publish(1, h, entries(), "b".into(), json!({})).is_err());
}

#[test]
fn checked_registry_final_sqlite_images_names_and_hashes_refuse_before_write() {
    use sha2::{Digest, Sha256};
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE sample(data TEXT); INSERT INTO sample VALUES('safe')")
        .unwrap();
    let image = db.serialize(rusqlite::MAIN_DB).unwrap();
    let digest = format!("{:x}", Sha256::digest(&*image));
    for a in [
        b"SQLite format 3\0".to_vec(),
        b"registry-0001.sqlite".to_vec(),
        digest.into_bytes(),
    ] {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("store");
        let s = LiveStore::create(&root, a, binding(), u64::MAX).unwrap();
        let backend = s.create_registry(&root.join("registry")).unwrap();
        backend
            .bind(&root.join("registry"), "safe-profile")
            .unwrap();
        let c = backend.checkout().unwrap();
        c.execute_batch("CREATE TABLE sample(data TEXT); INSERT INTO sample VALUES('safe')")
            .unwrap();
        assert!(c.publish(false).is_err());
        assert_eq!(std::fs::read_dir(root.join("registry")).unwrap().count(), 0);
    }
}
#[test]
fn checked_registry_orders_writers_persists_unknown_and_refuses_generic_open() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    let backend = s.create_registry(&root.join("registry")).unwrap();
    backend
        .bind(&root.join("registry"), "safe-profile")
        .unwrap();
    assert!(rusqlite::Connection::open(root.join("registry")).is_err());
    let c = backend.checkout().unwrap();
    assert!(backend.checkout().is_err());
    c.execute_batch("CREATE TABLE sample(data TEXT); INSERT INTO sample VALUES('unknown')")
        .unwrap();
    c.publish(false).unwrap();
    c.execute_batch("BEGIN IMMEDIATE; UPDATE sample SET data='observed'")
        .unwrap();
    assert!(c.publish(false).is_err());
    drop(c);
    let c = backend.checkout().unwrap();
    assert_eq!(
        c.query_row("SELECT data FROM sample", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "unknown"
    );
    let escaped = A
        .chars()
        .map(|c| format!("\\u{:04x}", c as u32))
        .collect::<String>();
    c.execute(
        "UPDATE sample SET data=?1",
        [format!("{{\"access\":\"{escaped}\"}}")],
    )
    .unwrap();
    assert!(c.publish(false).is_err());
    assert!(!root.join("registry/registry-0002.sqlite").exists());
}

#[test]
fn authentic_custody_seal_binds_final_registry_image_and_root() {
    use review_execution_evidence::pi::ExpectedSeal;
    for substitution in ["image", "metadata", "root"] {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("store");
        let mut s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
        let backend = s.create_registry(&root.join("registry")).unwrap();
        assert!(s.create_registry(&root.join("other")).is_err());
        backend
            .bind(&root.join("registry"), "safe-profile")
            .unwrap();
        let c = backend.checkout().unwrap();
        c.execute_batch("CREATE TABLE sample(data TEXT); INSERT INTO sample VALUES('unknown')")
            .unwrap();
        c.publish(false).unwrap();
        drop(c);
        s.begin(1, 1, None).unwrap();
        s.publish(1, header(), entries(), "b".into(), json!({}))
            .unwrap();
        let fd = s.expected_seal().unwrap().custody_descriptor().unwrap();
        let expected = ExpectedSeal::from_custody_descriptor(&fd).unwrap();
        assert!(expected.custody_descriptor().is_err());
        drop(backend);
        drop(s);
        let reader = SealedReader::open(&root, expected).unwrap();
        reader.read(1).unwrap();
        match substitution {
            "image" => {
                std::fs::write(root.join("registry/registry-0001.sqlite"), b"substitution").unwrap()
            }
            "metadata" => std::fs::write(root.join("registry/registry-0001.json"), b"{}").unwrap(),
            _ => {
                std::fs::rename(root.join("registry"), root.join("old")).unwrap();
                std::fs::create_dir(root.join("registry")).unwrap();
                for name in ["registry-0001.sqlite", "registry-0001.json"] {
                    std::fs::copy(
                        root.join("old").join(name),
                        root.join("registry").join(name),
                    )
                    .unwrap();
                }
            }
        }
        assert!(reader.read(1).is_err(), "{substitution}");
    }
}

#[test]
fn inspection_work_depth_records_and_decoded_bytes_fail_as_whole_objects() {
    use review_execution_evidence::pi::{MAX_DEPTH, MAX_INSPECTED};
    let t = tempfile::tempdir().unwrap();
    let s = LiveStore::create(
        &t.path().join("store"),
        A.as_bytes().to_vec(),
        binding(),
        u64::MAX,
    )
    .unwrap();
    assert!(s.check(&json!(vec![Value::Null; 65535])).is_ok());
    assert_eq!(
        s.check(&json!(vec![Value::Null; 65536])).unwrap_err(),
        "pi_inspect_bound"
    );
    let mut depth = Value::Null;
    for _ in 0..MAX_DEPTH {
        depth = json!([depth]);
    }
    assert!(s.check(&depth).is_ok());
    assert_eq!(s.check(&json!([depth])).unwrap_err(), "pi_inspect_bound");
    assert!(
        s.check(&json!({"encoding":"jsonl","data":vec!["{\"safe\":true}";256].join("\n")}))
            .is_ok()
    );
    assert_eq!(
        s.check(&json!({"large":"!".repeat(MAX_INSPECTED+1)}))
            .unwrap_err(),
        "pi_inspect_bound"
    );
}

#[test]
fn custody_descriptor_repeated_issuance_consumes_aggregate_capacity() {
    let t = tempfile::tempdir().unwrap();
    let mut s = LiveStore::create(
        &t.path().join("store"),
        A.as_bytes().to_vec(),
        binding(),
        u64::MAX,
    )
    .unwrap();
    s.begin(1, 1, None).unwrap();
    s.publish(1, header(), entries(), "b".into(), json!({}))
        .unwrap();
    let seal = s.expected_seal().unwrap();
    let mut refused = false;
    for _ in 0..50000 {
        match seal.custody_descriptor() {
            Ok(fd) => drop(fd),
            Err(e) => {
                assert_eq!(e, "pi_aggregate");
                refused = true;
                break;
            }
        }
    }
    assert!(refused);
    assert!(seal.custody_descriptor().is_err());
}

#[test]
fn encoded_names_refuse_before_directory_creation() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let a = b"abcdefghijklmnop";
    for name in [
        STANDARD.encode(a),
        format!("base64:{}", STANDARD.encode(a)),
        "base64:%%%".into(),
    ] {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join(&name);
        assert!(LiveStore::create(&root, a.to_vec(), binding(), u64::MAX).is_err());
        assert!(!root.exists());
        let store =
            LiveStore::create(&t.path().join("safe"), a.to_vec(), binding(), u64::MAX).unwrap();
        let nested = t.path().join("safe").join(&name);
        assert!(store.create_registry(&nested).is_err());
        assert!(!nested.exists());
    }
}
#[test]
fn sdk_usage_and_tool_unions_are_validated() {
    let base = entries();
    for (key, value) in [
        ("usage", json!({})),
        ("usage", json!({"input":1})),
        ("stopReason", json!("bogus")),
        ("responseId", json!(1)),
        ("durationMs", json!("slow")),
    ] {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("store");
        let mut s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
        s.begin(1, 1, None).unwrap();
        let mut e = base.clone();
        e[1]["message"][key] = value;
        assert!(
            s.publish(1, header(), e, "b".into(), json!({})).is_err(),
            "{key}"
        );
        assert!(!root.join("snapshot-0001.json").exists());
    }
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let mut s = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    s.begin(1, 1, None).unwrap();
    let mut e = base;
    e[1]["message"]["content"] = json!([{"type":"toolCall","id":"call1","name":"write","arguments":{"path":"module.js","content":"export const x=1;"}}]);
    e[1]["message"]["stopReason"] = json!("toolUse");
    e.push(json!({"type":"message","id":"c","parentId":"b","timestamp":"2026-11-14T00:00:00Z","message":{"role":"toolResult","timestamp":3,"toolCallId":"call1","toolName":"write","isError":false,"content":[{"type":"text","text":"written"}]}}));
    s.publish(1, header(), e, "c".into(), json!({})).unwrap();
}
#[test]
fn export_inspects_name_content_and_final_envelope() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let t = tempfile::tempdir().unwrap();
    let mut s = LiveStore::create(
        &t.path().join("store"),
        A.as_bytes().to_vec(),
        binding(),
        u64::MAX,
    )
    .unwrap();
    assert!(s.export(&STANDARD.encode(A), b"clean").is_err());
    assert!(s.export("leak", A.as_bytes()).is_err());
    assert!(s.export("nested", STANDARD.encode(A).as_bytes()).is_err());
    s.export("module.mjs", b"export const add=(a,b)=>a+b;")
        .unwrap();
}

#[test]
fn diagnostic_frames_use_checked_create_only_publication() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let mut store = LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
    assert!(store.record_frame(0, json!({"error":A})).is_err());
    assert!(
        store
            .record_frame(0, json!({"encoded":STANDARD.encode(A)}))
            .is_err()
    );
    assert!(!root.join("frame-0000.json").exists());
    store.record_frame(0, json!({"type":"ready"})).unwrap();
    assert!(store.record_frame(0, json!({"replace":true})).is_err());
    assert!(store.record_frame(129, json!({"outOfRange":true})).is_err());
}

#[test]
fn sdk_builtin_json_schema_declarations_and_unsupported_grammar_are_distinct() {
    for (sampling, valid) in [
        (json!(false), true),
        (json!({"type":"json_schema","strict":"prefer"}), true),
        (json!({"type":"json_schema","strict":"require"}), true),
        (json!({"type":"json_schema","strict":1}), false),
        (json!({"type":"grammar","variants":{}}), false),
        (
            json!({"type":"json_schema","strict":"prefer","unknown":true}),
            false,
        ),
    ] {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("store");
        let mut store =
            LiveStore::create(&root, A.as_bytes().to_vec(), binding(), u64::MAX).unwrap();
        store.begin(1, 1, None).unwrap();
        let mut e = entries();
        e[0]["message"] = json!({"role":"system","timestamp":1,"content":"bounded development tools","sections":{"rules":"use tools"},"toolsAdded":[{"name":"write","description":"write module","parameters":{"type":"object"},"constrainedSampling":sampling}]});
        assert_eq!(
            store.publish(1, header(), e, "b".into(), json!({})).is_ok(),
            valid
        );
        assert_eq!(root.join("snapshot-0001.json").exists(), valid);
    }
}
