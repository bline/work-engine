//! Disposable synthetic telemetry through the existing pinned production reader.
//! This does not qualify any live session/window or real Pi recipient.
use serde_json::{Value, json};
use session_delivery::*;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};
const BIN: &str = env!("CARGO_BIN_EXE_session-delivery");
const SESSION: &str = "11111111-1111-4111-8111-111111111111";
fn token(n: u64) -> Value {
    json!({"type":"event_msg","timestamp":"2026-11-04T00:20:00Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":n,"cached_input_tokens":0},"model_context_window":1000}}})
}
fn append(c: &Config, v: Value) {
    use std::io::Write;
    let p = &c.pressure.as_ref().unwrap().rollout;
    writeln!(fs::OpenOptions::new().append(true).open(p).unwrap(), "{v}").unwrap();
}
fn config() -> (tempfile::TempDir, Config) {
    let t = tempfile::tempdir().unwrap();
    fs::set_permissions(t.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_owned();
    let reader = PathBuf::from(
        std::env::var("PRESSURE_TEST_READER").expect("pinned existing reader required"),
    );
    let hash = std::env::var("PRESSURE_TEST_READER_SHA256").expect("reader digest required");
    assert_eq!(digest(&fs::read(&reader).unwrap()), hash);
    let rollout = t.path().join("synthetic-rollout.jsonl");
    fs::write(
        &rollout,
        format!(
            "{}\n{}\n{}\n",
            json!({"type":"session_meta","payload":{"id":SESSION}}),
            json!({"type":"turn_context","payload":{"model":"synthetic-model","effort":"medium"}}),
            token(100)
        ),
    )
    .unwrap();
    let b = Binding {
        recipient: "pressure-target".into(),
        principal: "fixture-principal".into(),
        job: "disposable-job".into(),
        lane: "isolated".into(),
        session: "isolated-fixture-session".into(),
        workspace: t.path().to_str().unwrap().into(),
        generation: 1,
        address: "role:fixture".into(),
        fixture_mode: "observe".into(),
        expires_at: 4102444800,
        active: true,
    };
    let c = Config {
        schema: "session-delivery-fixture-config-v1".into(),
        fixture_only: true,
        database: t.path().join("delivery.sqlite"),
        repository: t.path().into(),
        repository_identity: "synthetic-only".into(),
        source_id: "disposable-board-not-observed".into(),
        board_reader: root.join("skills/durable-state/scripts/codex_chatboard.py"),
        source_adapter: root.join("rust/crates/session-delivery/scripts/board_source.py"),
        targets: vec![b],
        pressure: Some(PressureConfig {
            reader,
            reader_sha256: hash,
            incarnation: "synthetic-pressure-v1".into(),
            session: SESSION.into(),
            host: "codex".into(),
            agent: None,
            rollout,
            levels: json!({"window_percent":{"trigger":58.0,"band_floor":66.0}}),
            step_percent: 5.0,
            lead_calls: 3,
            recipients: vec!["pressure-target".into()],
        }),
    };
    save(&t, &c);
    (t, c)
}
fn save(t: &tempfile::TempDir, c: &Config) {
    let p = t.path().join("config.json");
    fs::write(&p, serde_json::to_vec(c).unwrap()).unwrap();
    fs::set_permissions(p, fs::Permissions::from_mode(0o600)).unwrap();
}
fn cli(t: &tempfile::TempDir, args: &[&str], crash: Option<&str>) -> std::process::Output {
    let mut cmd = Command::new(BIN);
    cmd.arg("--config")
        .arg(t.path().join("config.json"))
        .args(args);
    if let Some(point) = crash {
        cmd.env("SESSION_DELIVERY_FIXTURE_CRASH", point);
    }
    cmd.output().unwrap()
}
fn ok(t: &tempfile::TempDir, args: &[&str]) {
    let o = cli(t, args, None);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}
fn store(c: &Config) -> Store {
    Store::open(c.clone()).unwrap()
}
fn subscribe(c: &Config) {
    store(c)
        .subscribe("pressure-target", "fixture-principal")
        .unwrap();
}
fn events(c: &Config) -> Vec<Event> {
    store(c).state().unwrap().events.into_values().collect()
}
fn event_values(c: &Config) -> Vec<Value> {
    events(c)
        .iter()
        .map(|e| serde_json::from_str::<Value>(&e.raw_payload).unwrap()["event"].clone())
        .collect()
}
#[test]
#[ignore = "requires pinned existing reader; no provider or actual recipient"]
fn actual_reader_restart_epoch_unavailable_history_and_isolated_receipt() {
    let (t, c) = config();
    subscribe(&c);
    ok(&t, &["observe-pressure"]);
    let s = store(&c).state().unwrap();
    assert!(s.events.is_empty());
    assert!(s.pressure.unwrap().cursor.is_some());
    append(&c, token(600));
    ok(&t, &["observe-pressure"]);
    let s = store(&c).state().unwrap();
    assert_eq!(s.events.len(), 1);
    assert_eq!(s.deliveries.len(), 1);
    let e = s.events.values().next().unwrap();
    assert_eq!(e.schema, "manual-context-control-v1");
    assert_eq!(e.payload_sha256, digest(e.raw_payload.as_bytes()));
    assert_eq!(event_values(&c)[0]["recorded_model"], "synthetic-model");
    ok(&t, &["observe-pressure"]);
    assert_eq!(events(&c).len(), 1);
    ok(&t, &["dispatch"]);
    assert!(
        store(&c)
            .state()
            .unwrap()
            .deliveries
            .values()
            .all(|d| d.stage == Stage::TargetObserved)
    );
    // Existing producer changes epoch from an appended native compaction marker;
    // source positions remain native line positions rather than lifecycle order.
    append(&c, json!({"type":"compacted","payload":{}}));
    append(&c, token(100));
    ok(&t, &["observe-pressure"]);
    let v = event_values(&c);
    assert!(v.iter().any(|v| v["epoch"] == 1 && v["level"] == "normal"));
    append(
        &c,
        json!({"type":"event_msg","payload":{"type":"token_count"}}),
    );
    ok(&t, &["observe-pressure"]);
    assert!(
        event_values(&c)
            .iter()
            .any(|v| v["level"] == "unavailable" && v["input_tokens"].is_null())
    );
    let before = store(&c).state().unwrap();
    fs::write(&c.pressure.as_ref().unwrap().rollout, "{}\n").unwrap();
    let o = cli(&t, &["observe-pressure"], None);
    assert!(!o.status.success());
    let after = store(&c).state().unwrap();
    assert_eq!(
        before.pressure.unwrap().cursor,
        after.pressure.as_ref().unwrap().cursor
    );
    assert_eq!(before.events, after.events);
    assert!(
        after
            .pressure
            .unwrap()
            .hold
            .unwrap()
            .contains("history_reset")
    );
    println!(
        "synthetic production-reader proof: {}",
        json!({"events":event_values(&c),"real_receiver":false,"live_telemetry":false})
    );
}
#[test]
#[ignore = "requires pinned existing reader"]
fn pressure_crash_cursor_atomicity_and_wrong_binding_or_reader_hold() {
    let (t, mut c) = config();
    subscribe(&c);
    ok(&t, &["observe-pressure"]);
    append(&c, token(700));
    let before = store(&c).state().unwrap().pressure.unwrap().cursor;
    let o = cli(
        &t,
        &["observe-pressure"],
        Some("after_pressure_inbox_before_cursor"),
    );
    assert_eq!(o.status.code(), Some(86));
    let s = store(&c).state().unwrap();
    assert!(s.events.is_empty());
    assert_eq!(s.pressure.unwrap().cursor, before);
    ok(&t, &["observe-pressure"]);
    assert_eq!(events(&c).len(), 1);
    let o = cli(&t, &["observe-pressure"], Some("after_commit"));
    assert_eq!(o.status.code(), Some(86));
    ok(&t, &["observe-pressure"]);
    assert_eq!(events(&c).len(), 1);
    c.pressure.as_mut().unwrap().session = "22222222-2222-4222-8222-222222222222".into();
    save(&t, &c);
    assert!(!cli(&t, &["observe-pressure"], None).status.success());
    // A different binding cannot silently reuse the previous producer baseline.
    assert!(Store::open(c).is_err());
    let (t, mut c) = config();
    c.pressure.as_mut().unwrap().reader_sha256 = "0".repeat(64);
    save(&t, &c);
    subscribe(&c);
    assert!(!cli(&t, &["observe-pressure"], None).status.success());
    assert!(
        store(&c)
            .state()
            .unwrap()
            .pressure
            .unwrap()
            .hold
            .unwrap()
            .contains("digest_mismatch")
    );
}
#[test]
#[ignore = "requires pinned existing reader"]
fn pressure_busy_offline_unknown_and_exact_attempt_ack() {
    for mode in ["busy", "offline", "lost_ack"] {
        let (t, mut c) = config();
        c.targets[0].fixture_mode = mode.into();
        save(&t, &c);
        subscribe(&c);
        append(&c, token(700));
        ok(&t, &["observe-pressure"]);
        ok(&t, &["dispatch"]);
        let mut s = store(&c);
        let state = s.state().unwrap();
        let d = state.deliveries.values().next().unwrap();
        assert!(!d.target_observed);
        if mode != "offline" {
            assert_eq!(d.stage, Stage::Unknown);
            let e = s.envelope(&d.delivery_id).unwrap();
            let ack = Ack {
                schema: "work-engine.delivery-observation/v1".into(),
                attempt_id: "wrong-attempt".into(),
                delivery_id: e.delivery_id.clone(),
                event_id: e.event.event_id.clone(),
                binding_sha256: e.binding_sha256.clone(),
                envelope_sha256: e.sha(),
                payload_sha256: e.event.payload_sha256.clone(),
                generation: e.binding.generation,
                stage: "target_observed".into(),
            };
            assert!(
                s.ack(&e.binding.recipient, &e.binding.principal, ack, 1)
                    .is_err()
            );
            let attempts = d.attempts;
            drop(s);
            ok(&t, &["dispatch"]);
            assert_eq!(
                store(&c)
                    .state()
                    .unwrap()
                    .deliveries
                    .values()
                    .next()
                    .unwrap()
                    .attempts,
                attempts
            );
        } else {
            assert_eq!(d.stage, Stage::Persisted);
        }
    }
}

#[test]
#[ignore = "requires pinned existing reader"]
fn pressure_stale_wrong_source_and_missing_source_remain_observations_or_holds() {
    let (t, c) = config();
    subscribe(&c);
    append(&c, token(700));
    let path = &c.pressure.as_ref().unwrap().rollout;
    fs::File::open(path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(1000))
        .unwrap();
    ok(&t, &["observe-pressure"]);
    assert_eq!(event_values(&c)[0]["freshness"], "stale_file_modification");
    // Physical source absence cannot manufacture an unavailable metric of zero.
    let before = store(&c).state().unwrap().pressure.unwrap().cursor;
    fs::remove_file(path).unwrap();
    assert!(!cli(&t, &["observe-pressure"], None).status.success());
    let s = store(&c).state().unwrap();
    assert_eq!(s.pressure.as_ref().unwrap().cursor, before);
    assert!(s.pressure.unwrap().hold.is_some());
    let (t, c) = config();
    subscribe(&c);
    // A wrong source session is diagnosed by the actual reader, retained as
    // unavailable attribution evidence, and never turned into valid pressure.
    fs::write(
        &c.pressure.as_ref().unwrap().rollout,
        format!(
            "{}\n{}\n",
            json!({"type":"session_meta","payload":{"id":"22222222-2222-4222-8222-222222222222"}}),
            token(700)
        ),
    )
    .unwrap();
    ok(&t, &["observe-pressure"]);
    let v = event_values(&c);
    assert_eq!(v[0]["level"], "unavailable");
    assert!(v[0]["input_tokens"].is_null());
    assert!(!v[0]["unavailable_detail"].is_null());
}

#[test]
#[ignore = "requires pinned existing reader; synthetic freshness only"]
fn actual_reader_freshness_aba_occurrences_survive_restart() {
    let (t, c) = config();
    subscribe(&c);
    append(&c, token(400));
    append(&c, token(600));
    let path = &c.pressure.as_ref().unwrap().rollout;
    let source = fs::read(path).unwrap();
    ok(&t, &["observe-pressure"]);
    let first = store(&c).state().unwrap();
    assert_eq!(first.events.len(), 1);
    let a = first.pressure.unwrap().cursor.unwrap();
    fs::File::open(path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(1000))
        .unwrap();
    ok(&t, &["observe-pressure"]);
    let second = store(&c).state().unwrap();
    assert_eq!(second.events.len(), 2);
    assert_ne!(second.pressure.unwrap().cursor.unwrap(), a);
    fs::File::open(path)
        .unwrap()
        .set_modified(std::time::SystemTime::now())
        .unwrap();
    ok(&t, &["observe-pressure"]);
    let third = store(&c).state().unwrap();
    assert_eq!(third.pressure.unwrap().cursor.unwrap(), a);
    assert_eq!(third.events.len(), 3);
    assert_eq!(third.deliveries.len(), 3);
    assert_eq!(fs::read(path).unwrap(), source);
    ok(&t, &["observe-pressure"]);
    assert_eq!(store(&c).state().unwrap().events.len(), 3);
    for _ in 0..3 {
        ok(&t, &["dispatch"]);
    }
    assert!(
        store(&c)
            .state()
            .unwrap()
            .deliveries
            .values()
            .all(|d| d.stage == Stage::TargetObserved)
    );
}
