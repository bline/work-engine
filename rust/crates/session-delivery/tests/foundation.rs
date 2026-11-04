use serde_json::{Value, json};
use session_delivery::*;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};
use tempfile::TempDir;
const BIN: &str = env!("CARGO_BIN_EXE_session-delivery");
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_owned()
}
fn fixture() -> (TempDir, Config) {
    let t = tempfile::tempdir().unwrap();
    fs::set_permissions(t.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let repo = t.path().join("repository");
    fs::create_dir(&repo).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .arg(&repo)
            .status()
            .unwrap()
            .success()
    );
    let cfg = Config {
        schema: "session-delivery-fixture-config-v1".into(),
        fixture_only: true,
        database: t.path().join("state.sqlite"),
        repository: repo.clone(),
        repository_identity: repo.join(".git").to_str().unwrap().into(),
        source_id: "disposable-board".into(),
        board_reader: root().join("skills/durable-state/scripts/codex_chatboard.py"),
        source_adapter: std::env::var_os("SESSION_DELIVERY_R1_ADAPTER")
            .map(PathBuf::from)
            .unwrap_or_else(|| root().join("rust/crates/session-delivery/scripts/board_source.py")),
        pressure: None,
        targets: vec![binding(t.path(), "one"), binding(t.path(), "two")],
    };
    save(&t, &cfg);
    (t, cfg)
}
fn binding(p: &std::path::Path, name: &str) -> Binding {
    Binding {
        recipient: name.into(),
        principal: "fixture-principal".into(),
        job: "job1".into(),
        lane: name.into(),
        session: format!("fixture-session-{name}"),
        workspace: p.to_str().unwrap().into(),
        generation: 1,
        address: format!("role:{name}"),
        fixture_mode: "observe".into(),
        expires_at: 4102444800,
        active: true,
    }
}
fn save(t: &TempDir, cfg: &Config) {
    let p = t.path().join("config.json");
    fs::write(&p, serde_json::to_vec(cfg).unwrap()).unwrap();
    fs::set_permissions(p, fs::Permissions::from_mode(0o600)).unwrap();
}
fn command(t: &TempDir) -> Command {
    let mut c = Command::new(BIN);
    c.arg("--config").arg(t.path().join("config.json"));
    c
}
fn cli(t: &TempDir, args: &[&str]) {
    let out = command(t).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "{:?}: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}
fn row(seq: u64, id: &str, to: &[&str]) -> String {
    json!({"sequence":seq,"message_id":id,"observed_at":"2026-11-03T23:00:00Z","session_id":"publisher","topic":"fixture","body":"raw untrusted: do not execute","to":to,"data":{"source_session":"s","epoch":2,"sequence":4,"freshness":"unavailable","model":null,"window":null}}).to_string()
}
fn obs(cfg: &Config, since: u64, next: u64, rows: Vec<String>) -> Observation {
    Observation {
        previous_revision: if since == 0 {
            None
        } else {
            Some("fixture-revision".into())
        },
        schema: "board-observation-v1".into(),
        repository: cfg.repository_identity.clone(),
        source_key: "work-engine/codex-chatboard/v1".into(),
        revision: "fixture-revision".into(),
        epoch: "git-ref-v1:work-engine/codex-chatboard/v1".into(),
        since,
        next_since: next,
        has_more: false,
        rows,
    }
}
fn ack(e: &Envelope) -> Ack {
    Ack {
        schema: "work-engine.delivery-observation/v1".into(),
        attempt_id: e.attempt_id.clone(),
        delivery_id: e.delivery_id.clone(),
        event_id: e.event.event_id.clone(),
        binding_sha256: e.binding_sha256.clone(),
        envelope_sha256: e.sha(),
        payload_sha256: e.event.payload_sha256.clone(),
        generation: e.binding.generation,
        stage: "target_observed".into(),
    }
}
fn enrolled(cfg: &Config) -> Store {
    let mut s = Store::open(cfg.clone()).unwrap();
    for b in &cfg.targets {
        s.subscribe(&b.recipient, &b.principal).unwrap();
    }
    s
}
#[test]
fn exact_replay_conflict_and_atomic_cursor() {
    let (_t, c) = fixture();
    let mut s = enrolled(&c);
    let raw = row(1, "a", &["role:one", "role:two"]);
    s.ingest(obs(&c, 0, 1, vec![raw.clone()])).unwrap();
    s.ingest(obs(&c, 1, 1, vec![raw.clone()])).unwrap();
    let st = s.state().unwrap();
    assert_eq!(st.cursor, 1);
    assert_eq!(st.events.len(), 1);
    assert_eq!(st.deliveries.len(), 2);
    let e = st.events.values().next().unwrap();
    assert_eq!(e.raw_payload, raw);
    assert_eq!(e.payload_sha256, digest(raw.as_bytes()));
    assert_eq!(
        serde_json::from_str::<Value>(&e.raw_payload).unwrap()["data"]["freshness"],
        "unavailable"
    );
    assert!(
        s.ingest(obs(
            &c,
            1,
            3,
            vec![row(2, "b", &["role:one"]), row(3, "a", &["role:one"])]
        ))
        .is_err()
    );
    assert_eq!(s.state().unwrap().cursor, 1);
    assert_eq!(s.state().unwrap().events.len(), 1);
}
#[test]
fn malformed_oversized_and_source_identity_do_not_advance() {
    let (_t, c) = fixture();
    let mut s = enrolled(&c);
    for raw in ["{".into(), "x".repeat(MAX_ROW + 1), row(0, "a", &[])] {
        assert!(s.ingest(obs(&c, 0, 1, vec![raw])).is_err());
        assert_eq!(s.state().unwrap().cursor, 0);
    }
    let mut o = obs(&c, 0, 1, vec![row(1, "a", &[])]);
    o.repository = "other".into();
    assert!(s.ingest(o).is_err());
}
#[test]
fn ack_identity_stages_and_expiry_are_independent() {
    let (_t, c) = fixture();
    let mut s = enrolled(&c);
    s.ingest(obs(&c, 0, 1, vec![row(1, "a", &["role:one", "role:two"])]))
        .unwrap();
    let id = s.pending_ids().unwrap()[0].clone();
    let e = s.prepare(&id, 1).unwrap().unwrap();
    s.possible_send(&id).unwrap();
    let a = ack(&e);
    for field in ["digest", "generation", "principal", "target", "event"] {
        let mut wrong = a.clone();
        let mut who = e.binding.principal.clone();
        let mut target = e.binding.recipient.clone();
        match field {
            "digest" => wrong.payload_sha256 = "0".repeat(64),
            "generation" => wrong.generation += 1,
            "principal" => who = "other".into(),
            "target" => {
                target = if target == "one" {
                    "two".into()
                } else {
                    "one".into()
                }
            }
            _ => wrong.event_id = "other".into(),
        }
        assert!(s.ack(&target, &who, wrong, 2).is_err());
    }
    assert!(
        s.ack(
            &e.binding.recipient,
            &e.binding.principal,
            a.clone(),
            4102444800
        )
        .is_err()
    );
    s.ack(&e.binding.recipient, &e.binding.principal, a.clone(), 2)
        .unwrap();
    s.ack(&e.binding.recipient, &e.binding.principal, a, 4102444801)
        .unwrap();
    let st = s.state().unwrap();
    let d = &st.deliveries[&id];
    assert!(d.target_observed);
    assert!(!d.transport_accepted);
    assert!(d.application_result.is_none());
    assert_eq!(
        st.deliveries
            .values()
            .filter(|d| d.stage == Stage::Persisted)
            .count(),
        1
    );
}
#[test]
fn expiry_revoke_and_successor_never_move_queued_work() {
    let (_t, mut c) = fixture();
    let mut s = enrolled(&c);
    s.ingest(obs(&c, 0, 1, vec![row(1, "a", &["role:one"])]))
        .unwrap();
    let id = s.pending_ids().unwrap()[0].clone();
    assert!(s.prepare(&id, 4102444800).unwrap().is_none());
    drop(s);
    c.targets[0].generation = 2;
    let mut s = Store::open(c.clone()).unwrap();
    assert!(s.subscribe("one", "fixture-principal").is_err());
    assert!(s.prepare(&id, 1).unwrap().is_none());
    assert_eq!(
        s.state().unwrap().deliveries[&id].hold.as_deref(),
        Some("current_binding_mismatch")
    );
    c.targets[0].generation = 1;
    c.targets[0].active = false;
    drop(s);
    let mut s = Store::open(c).unwrap();
    assert!(s.prepare(&id, 1).unwrap().is_none());
}
fn post(t: &TempDir, c: &Config) {
    let out = Command::new("python3")
        .arg(&c.board_reader)
        .arg("--repository")
        .arg(&c.repository)
        .args([
            "post",
            "--author",
            "fixture",
            "--session-id",
            "11111111-1111-4111-8111-111111111111",
            "--topic",
            "fixture",
            "--body",
            "non-authoritative unicode λ\nfixture bytes",
            "--to",
            "role:one",
            "--to",
            "role:two",
            "--message-id",
            "22222222-2222-4222-8222-222222222222",
        ])
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    fs::write(t.path().join("publication.json"), out.stdout).unwrap();
}
#[test]
fn disposable_git_actual_reader_store_receiver_restart() {
    let (t, c) = fixture();
    drop(enrolled(&c));
    post(&t, &c);
    cli(&t, &["observe"]);
    cli(&t, &["dispatch"]);
    let s = Store::open(c.clone()).unwrap();
    let st = s.state().unwrap();
    assert_eq!(st.cursor, 1);
    assert_eq!(st.deliveries.len(), 2);
    assert!(
        st.deliveries
            .values()
            .all(|d| d.target_observed && d.transport_accepted && d.application_result.is_none())
    );
    for b in &c.targets {
        let conn = rusqlite::Connection::open(c.database.with_extension(format!(
            "receiver-{}.sqlite",
            digest(b.recipient.as_bytes())
        )))
        .unwrap();
        let count: u64 = conn
            .query_row("SELECT count(*) FROM observations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
    if let Ok(path) = std::env::var("SESSION_DELIVERY_EVIDENCE_FILE") {
        let mut observations = Vec::new();
        for b in &c.targets {
            let conn = rusqlite::Connection::open(c.database.with_extension(format!(
                "receiver-{}.sqlite",
                digest(b.recipient.as_bytes())
            )))
            .unwrap();
            let raw: String = conn
                .query_row("SELECT raw FROM observations", [], |r| r.get(0))
                .unwrap();
            observations.push(serde_json::from_str::<Value>(&raw).unwrap());
        }
        fs::write(path,serde_json::to_vec_pretty(&json!({"fixture_only":true,"binary_sha256":digest(&fs::read(BIN).unwrap()),"publication":serde_json::from_slice::<Value>(&fs::read(t.path().join("publication.json")).unwrap()).unwrap(),"store":st,"receiver_observations":observations})).unwrap()).unwrap();
    }
    cli(&t, &["run", "2"]);
    assert_eq!(Store::open(c).unwrap().state().unwrap().deliveries.len(), 2);
}
#[test]
fn crash_cursor_inbox_and_committed_restart() {
    for point in ["after_inbox_before_cursor", "before_commit", "after_commit"] {
        let (t, c) = fixture();
        drop(enrolled(&c));
        post(&t, &c);
        let out = command(&t)
            .arg("observe")
            .env("SESSION_DELIVERY_FIXTURE_CRASH", point)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(86));
        let st = Store::open(c.clone()).unwrap().state().unwrap();
        assert_eq!(st.cursor, if point == "after_commit" { 1 } else { 0 });
        assert_eq!(st.events.len(), st.cursor as usize);
        cli(&t, &["observe"]);
        let st = Store::open(c).unwrap().state().unwrap();
        assert_eq!(st.cursor, 1);
        assert_eq!(st.deliveries.len(), 2);
    }
}
#[test]
fn crash_send_ack_and_unknown_never_auto_replayed() {
    for point in [
        "after_prepare",
        "before_send",
        "after_send",
        "before_ack_commit",
        "after_transport_ack",
        "after_ack",
    ] {
        let (t, mut c) = fixture();
        c.targets.truncate(1);
        save(&t, &c);
        drop(enrolled(&c));
        post(&t, &c);
        cli(&t, &["observe"]);

        let out = command(&t)
            .arg("dispatch")
            .env("SESSION_DELIVERY_FIXTURE_CRASH", point)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(86), "{point}");

        let before = Store::open(c.clone()).unwrap().state().unwrap();
        let d = before.deliveries.values().next().unwrap();
        assert_eq!(d.attempts, 1);
        assert_ne!(d.stage, Stage::Persisted);
        cli(&t, &["dispatch"]);
        let after = Store::open(c).unwrap().state().unwrap();
        assert_eq!(
            after.deliveries.values().next().unwrap().attempts,
            1,
            "{point}"
        );
    }
}
#[test]
fn offline_is_bounded_pre_send_busy_and_lost_ack_are_held() {
    for mode in ["offline", "busy", "lost_ack"] {
        let (t, mut c) = fixture();
        c.targets.truncate(1);
        c.targets[0].fixture_mode = mode.into();
        save(&t, &c);
        drop(enrolled(&c));
        post(&t, &c);
        cli(&t, &["observe"]);
        cli(&t, &["dispatch"]);
        let st = Store::open(c.clone()).unwrap().state().unwrap();
        let d = st.deliveries.values().next().unwrap();
        assert!(!d.target_observed);
        if mode == "offline" {
            assert_eq!(d.stage, Stage::Persisted);
            cli(&t, &["run", "5"]);
            assert_eq!(
                Store::open(c)
                    .unwrap()
                    .state()
                    .unwrap()
                    .deliveries
                    .values()
                    .next()
                    .unwrap()
                    .attempts,
                3
            );
        } else {
            assert_eq!(d.stage, Stage::Unknown);
            cli(&t, &["dispatch"]);
            assert_eq!(
                Store::open(c)
                    .unwrap()
                    .state()
                    .unwrap()
                    .deliveries
                    .values()
                    .next()
                    .unwrap()
                    .attempts,
                1
            );
        }
    }
}

#[test]
fn late_ack_cannot_attach_to_later_pre_send_attempt() {
    let (_t, c) = fixture();
    let mut s = enrolled(&c);
    s.ingest(obs(&c, 0, 1, vec![row(1, "a", &["role:one"])]))
        .unwrap();
    let id = s.pending_ids().unwrap()[0].clone();
    let old = s.prepare(&id, 1).unwrap().unwrap();
    s.pre_send_failure(&id, "proven_no_write").unwrap();
    let new = s.prepare(&id, 2).unwrap().unwrap();
    assert_ne!(old.attempt_id, new.attempt_id);
    s.possible_send(&id).unwrap();
    assert!(s.ack("one", "fixture-principal", ack(&old), 3).is_err());
    s.ack("one", "fixture-principal", ack(&new), 3).unwrap();
    let state = s.state().unwrap();
    let d = &state.deliveries[&id];
    assert_eq!(d.attempt_journal.len(), 2);
    assert!(!d.attempt_journal[0].target_observed);
    assert!(d.attempt_journal[1].target_observed);
}
#[test]
fn backlog_hold_is_atomic_and_slow_recipient_does_not_consume_other() {
    let (_t, c) = fixture();
    let mut s = enrolled(&c);
    s.ingest(obs(
        &c,
        0,
        2,
        vec![
            row(1, "a", &["role:one"]),
            row(2, "b", &["role:one", "role:two"]),
        ],
    ))
    .unwrap();
    let ids = s.pending_ids().unwrap();
    assert_eq!(ids.len(), 2);
    let first = ids
        .iter()
        .find(|id| s.envelope(id).unwrap().binding.recipient == "one")
        .unwrap();
    s.prepare(first, 1).unwrap();
    s.possible_send(first).unwrap();
    let pending = s.pending_ids().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(s.envelope(&pending[0]).unwrap().binding.recipient, "two");
    for start in [3, 503] {
        let end = (start + 499).min(1000);
        s.ingest(obs(
            &c,
            start - 1,
            end,
            (start..=end)
                .map(|n| row(n, &format!("m{n}"), &[]))
                .collect(),
        ))
        .unwrap();
    }
    assert_eq!(s.state().unwrap().events.len(), MAX_EVENTS);
    assert!(
        s.ingest(obs(&c, 1000, 1001, vec![row(1001, "overflow", &[])]))
            .is_err()
    );
    assert_eq!(s.state().unwrap().cursor, 1000);
}
#[test]
fn missing_and_rewound_board_surface_source_hold() {
    let (t, c) = fixture();
    drop(enrolled(&c));
    assert!(!command(&t).arg("observe").status().unwrap().success());
    let st = Store::open(c.clone()).unwrap().state().unwrap();
    assert_eq!(st.cursor, 0);
    assert!(st.source_hold.is_some());
    post(&t, &c);
    cli(&t, &["observe"]);
    // A disposable source reset is not treated as an empty successful read.
    let refname = format!(
        "refs/work-engine/codex-chatboard/{}",
        digest(b"work-engine/codex-chatboard/v1")
    );
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&c.repository)
            .args(["update-ref", "-d", &refname])
            .status()
            .unwrap()
            .success()
    );
    assert!(!command(&t).arg("observe").status().unwrap().success());
    let st = Store::open(c).unwrap().state().unwrap();
    assert_eq!(st.cursor, 1);
    assert!(st.source_hold.is_some());
}

#[test]
fn receiver_rejects_cross_target_and_payload_changes_before_observation() {
    use std::io::Write;
    let (t, c) = fixture();
    let mut s = enrolled(&c);
    s.ingest(obs(&c, 0, 1, vec![row(1, "a", &["role:one"])]))
        .unwrap();
    let id = s.pending_ids().unwrap()[0].clone();
    let e = s.prepare(&id, 1).unwrap().unwrap();
    for variant in ["target", "payload"] {
        let mut bad = e.clone();
        let recipient = if variant == "target" {
            "two"
        } else {
            bad.event.raw_payload.push(' ');
            "one"
        };
        let mut child = command(&t)
            .args(["fixture-receiver", recipient])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&bad).unwrap())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    for b in &c.targets {
        assert!(
            !c.database
                .with_extension(format!(
                    "receiver-{}.sqlite",
                    digest(b.recipient.as_bytes())
                ))
                .exists()
        );
    }
}
#[test]
fn status_pagination_exposes_all_retained_holds_without_payloads() {
    let (_t, c) = fixture();
    let mut s = enrolled(&c);
    s.ingest(obs(
        &c,
        0,
        60,
        (1..=60)
            .map(|n| row(n, &format!("m{n}"), &["role:one"]))
            .collect(),
    ))
    .unwrap();
    let first = s.status("one", "fixture-principal").unwrap();
    assert_eq!(first["deliveries"].as_array().unwrap().len(), 50);
    let next = first["next_after"].as_str().unwrap();
    let second = s
        .status_after("one", "fixture-principal", Some(next))
        .unwrap();
    assert_eq!(second["deliveries"].as_array().unwrap().len(), 10);
    assert_eq!(second["truncated"], false);
    assert!(first["deliveries"][0].get("raw_payload").is_none());
}

// These regressions execute the actual board owner, adapter and fresh CLI
// processes. The optional adapter path is used only for a guard-disabled
// counterfactual; it changes no product configuration/authority mechanism.
fn board_update(c: &Config, action: &str, rows: &[Value]) {
    let script = r#"
import importlib.util,json,sys
from pathlib import Path
sys.path.insert(0,str(Path(sys.argv[1]).parent))
spec=importlib.util.spec_from_file_location('r1_board_owner',sys.argv[1])
owner=importlib.util.module_from_spec(spec);spec.loader.exec_module(owner)
def change(board):
    rows=json.loads(sys.argv[4])
    if sys.argv[3] in ('replace','replace_detached'): board['messages']=rows
    if sys.argv[3]=='replace_detached': board['claims']['new-lineage']={'fixture':True}
    elif sys.argv[3]=='append': board['messages'].extend(rows)
    elif sys.argv[3]=='claims': board['claims']['fixture-r1']={'note':'non-message revision'}
    board['next_sequence']=len(board['messages'])+1
owner.mutate(Path(sys.argv[2]),change)
"#;
    let out = Command::new("python3")
        .args(["-c", script])
        .arg(&c.board_reader)
        .arg(&c.repository)
        .arg(action)
        .arg(serde_json::to_string(rows).unwrap())
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "board mutation: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
fn reset_board(c: &Config) {
    let reference = format!(
        "refs/work-engine/codex-chatboard/{}",
        digest(b"work-engine/codex-chatboard/v1")
    );
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&c.repository)
            .args(["update-ref", "-d", &reference])
            .status()
            .unwrap()
            .success()
    );
}
fn r1_record(name: &str, before: &State, after: &State, cfg: &Config) {
    if let Some(dir) = std::env::var_os("SESSION_DELIVERY_R1_EVIDENCE_DIR") {
        let dir = PathBuf::from(dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{name}.json")),serde_json::to_vec_pretty(&json!({
            "fixture_only":true,"scenario":name,"adapter_sha256":digest(&fs::read(&cfg.source_adapter).unwrap()),
            "before":before,"after":after
        })).unwrap()).unwrap();
    }
}
fn assert_r1_hold(name: &str, t: &TempDir, c: &Config, before: &State) {
    // run succeeds as a bounded service operation while exposing the source hold.
    // A fresh process reconstructs the saved revision; no model/agent wait occurs.
    cli(t, &["run", "1"]);
    let after = Store::open(c.clone()).unwrap().state().unwrap();
    r1_record(name, before, &after, c);
    assert_eq!(
        after.source_hold.as_deref(),
        Some("source_history_unavailable_or_conflict"),
        "R1 source-history guard failed: replacement accepted or no history hold; cursor {} -> {}, events {} -> {}",
        before.cursor,
        after.cursor,
        before.events.len(),
        after.events.len()
    );
    assert_eq!(
        after.cursor, before.cursor,
        "R1 cursor advanced across incompatible history"
    );
    assert_eq!(
        after.source_revision, before.source_revision,
        "R1 replaced committed source revision"
    );
    assert_eq!(
        serde_json::to_value(&after.events).unwrap(),
        serde_json::to_value(&before.events).unwrap(),
        "R1 inbox changed"
    );
    assert_eq!(
        serde_json::to_value(&after.deliveries).unwrap(),
        serde_json::to_value(&before.deliveries).unwrap(),
        "R1 delivered replacement history"
    );
}
fn reset_repopulate_case(name: &str, beyond: bool, changed_id: bool) {
    let (t, c) = fixture();
    drop(enrolled(&c));
    post(&t, &c);
    cli(&t, &["observe"]);
    cli(&t, &["dispatch"]);
    let before = Store::open(c.clone()).unwrap().state().unwrap();
    let mut replacement: Value =
        serde_json::from_str(&before.events.values().next().unwrap().raw_payload).unwrap();
    if changed_id {
        replacement["message_id"] = json!("33333333-3333-4333-8333-333333333333");
    } else {
        replacement["body"] = json!("changed bytes under the original source message identity");
    }
    let mut rows = vec![replacement];
    if beyond {
        rows.push(
            serde_json::from_str(&row(
                2,
                "44444444-4444-4444-8444-444444444444",
                &["role:one", "role:two"],
            ))
            .unwrap(),
        );
    }
    reset_board(&c);
    board_update(&c, "replace", &rows);
    assert_r1_hold(name, &t, &c, &before);
}
#[test]
fn r1_reset_same_cursor_changed_id_is_held() {
    reset_repopulate_case("same-id", false, true);
}
#[test]
fn r1_reset_same_cursor_changed_bytes_is_held() {
    reset_repopulate_case("same-bytes", false, false);
}
#[test]
fn r1_reset_beyond_cursor_changed_id_is_held() {
    reset_repopulate_case("beyond-id", true, true);
}
#[test]
fn r1_reset_beyond_cursor_changed_bytes_is_held() {
    reset_repopulate_case("beyond-bytes", true, false);
}
#[test]
fn r1_missing_retained_history_is_held() {
    let (t, c) = fixture();
    drop(enrolled(&c));
    post(&t, &c);
    cli(&t, &["observe"]);
    cli(&t, &["dispatch"]);
    let before = Store::open(c.clone()).unwrap().state().unwrap();
    board_update(
        &c,
        "append",
        &[serde_json::from_str(&row(2, "next", &["role:one"])).unwrap()],
    );
    let revision = before.source_revision.as_ref().unwrap();
    let object = c
        .repository
        .join(".git/objects")
        .join(&revision[..2])
        .join(&revision[2..]);
    assert!(
        object.is_file(),
        "fixture must remove the actual retained source blob"
    );
    fs::remove_file(object).unwrap();
    assert_r1_hold("missing-history", &t, &c, &before);
}
#[test]
fn r1_identical_prefix_detached_history_is_held() {
    let (t, c) = fixture();
    drop(enrolled(&c));
    post(&t, &c);
    cli(&t, &["observe"]);
    cli(&t, &["dispatch"]);
    let before = Store::open(c.clone()).unwrap().state().unwrap();
    let original: Value =
        serde_json::from_str(&before.events.values().next().unwrap().raw_payload).unwrap();
    reset_board(&c);
    board_update(&c, "replace_detached", &[original]);
    assert_r1_hold("detached-history", &t, &c, &before);
}
#[test]
fn r1_append_reserved_rows_and_nonmessage_revisions_preserve_continuity() {
    let (t, c) = fixture();
    drop(enrolled(&c));
    let mut reserved: Value = serde_json::from_str(&row(1, "reserved", &["role:one"])).unwrap();
    reserved["topic"] = json!("control:session:fixture-source");
    let first: Value = serde_json::from_str(&row(2, "first-visible", &["role:one"])).unwrap();
    board_update(&c, "replace", &[reserved, first]);
    cli(&t, &["observe"]);
    let before = Store::open(c.clone()).unwrap().state().unwrap();
    assert_eq!(before.cursor, 2);
    assert_eq!(before.events.len(), 1);
    board_update(&c, "claims", &[]);
    cli(&t, &["observe"]);
    let nonmessage = Store::open(c.clone()).unwrap().state().unwrap();
    assert_ne!(nonmessage.source_revision, before.source_revision);
    assert_eq!(nonmessage.cursor, 2);
    assert_eq!(nonmessage.events, before.events);
    assert!(nonmessage.source_hold.is_none());
    board_update(
        &c,
        "append",
        &[serde_json::from_str(&row(3, "appended", &["role:two"])).unwrap()],
    );
    cli(&t, &["run", "2"]);
    let after = Store::open(c.clone()).unwrap().state().unwrap();
    assert_eq!(after.cursor, 3);
    assert_eq!(after.events.len(), 2);
    assert!(after.source_hold.is_none());
    assert!(after.deliveries.values().all(|d| d.target_observed));
    r1_record("legitimate-append", &before, &after, &c);
}
#[test]
fn r1_changed_consumed_reserved_bytes_are_not_hidden_by_filtering() {
    let (t, c) = fixture();
    drop(enrolled(&c));
    let mut reserved: Value = serde_json::from_str(&row(1, "reserved", &[])).unwrap();
    reserved["topic"] = json!("control:budget:fixture");
    let visible: Value = serde_json::from_str(&row(2, "visible", &["role:one"])).unwrap();
    board_update(&c, "replace", &[reserved.clone(), visible.clone()]);
    cli(&t, &["observe"]);
    cli(&t, &["dispatch"]);
    let before = Store::open(c.clone()).unwrap().state().unwrap();
    reserved["body"] = json!("changed reserved bytes");
    // Still a normal predecessor chain: prefix validation, not ancestry alone,
    // must catch the changed consumed row excluded by the existing reader.
    board_update(&c, "replace", &[reserved, visible]);
    assert_r1_hold("reserved-conflict", &t, &c, &before);
}
#[test]
fn r1_revision_changes_without_cursor_progress_reject_stale_observation() {
    let (_t, c) = fixture();
    let mut s = enrolled(&c);
    s.ingest(obs(&c, 0, 1, vec![row(1, "a", &[])])).unwrap();
    let mut stale = obs(&c, 1, 1, vec![]);
    stale.previous_revision = Some("different-history".into());
    assert!(s.ingest(stale).is_err());
    assert_eq!(
        s.state().unwrap().source_revision.as_deref(),
        Some("fixture-revision")
    );
}
