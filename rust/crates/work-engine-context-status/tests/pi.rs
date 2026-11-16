use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const ID: &str = "01a1e497-a4bf-70ac-b6a3-7e79614ac26a";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "context-pi-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join("session.jsonl")
    }
    fn rows(&self, rows: &[Value]) {
        let mut all = vec![json!({"type":"session","version":3,"id":ID,"cwd":"/w"})];
        all.extend_from_slice(rows);
        fs::write(
            self.path(),
            all.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
    }
    fn report(&self) -> Value {
        parse(self.cmd(&["pi", "--session", ID, "--session-file"], &[]))
    }
    fn cmd(&self, head: &[&str], tail: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
            .args(head)
            .arg(self.path())
            .args(tail)
            .output()
            .unwrap()
    }
    fn control(&self, extra: &[&str]) -> Output {
        let mut tail = vec![
            "--host",
            "pi",
            "--trigger-tokens",
            "100000",
            "--band-floor-tokens",
            "150000",
        ];
        tail.extend_from_slice(extra);
        self.cmd(&["control", "--session", ID, "--rollout"], &tail)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn parse(out: Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn assistant(id: &str, parent: Option<&str>, input: u64, read: u64, write: u64) -> Value {
    json!({"type":"message","id":id,"parentId":parent,"timestamp":"2026-11-16T14:00:00.000Z",
        "message":{"role":"assistant","provider":"openai-codex","model":"gpt-6","api":"openai-codex-responses",
            "stopReason":"toolUse","usage":{"input":input,"output":50,"cacheRead":read,"cacheWrite":write,
            "totalTokens":input+read+write+50}}})
}
fn user(id: &str, parent: Option<&str>) -> Value {
    json!({"type":"message","id":id,"parentId":parent,"message":{"role":"user","content":"x"}})
}
fn compaction(id: &str, parent: &str) -> Value {
    json!({"type":"compaction","id":id,"parentId":parent,"summary":"<context_epoch/>","firstKeptEntryId":"never",
        "details":{"kind":"pi-context-epochs/epoch"}})
}

#[test]
fn reports_carried_input_from_the_latest_usable_request() {
    let f = Fixture::new();
    let mut aborted = assistant("a3", Some("a2"), 900_000, 0, 0);
    aborted["message"]["stopReason"] = json!("aborted");
    f.rows(&[
        user("u1", None),
        assistant("a1", Some("u1"), 10, 5_000, 1_000),
        assistant("a2", Some("a1"), 20, 6_000, 2_000),
        aborted,
        assistant("a4", Some("a3"), 0, 0, 0),
    ]);
    let r = f.report();
    assert_eq!(r["host"], "pi");
    assert_eq!(r["recorded_model"], "openai-codex/gpt-6");
    assert_eq!(r["latest_request_status"], "latest_valid_observation");
    assert_eq!(r["latest_request"]["input_tokens"], 8_020);
    assert_eq!(r["latest_request"]["pi_context_tokens"], 8_070);
    assert_eq!(r["latest_request"]["line"], 4);
    assert_eq!(r["recent_requests"].as_array().unwrap().len(), 2);
    assert!(r["context_window_tokens"].is_null());
}

#[test]
fn identity_and_header_are_required() {
    let f = Fixture::new();
    f.rows(&[assistant("a1", None, 1, 1, 1)]);
    let out = f.cmd(
        &[
            "pi",
            "--session",
            "11111111-1111-4111-8111-111111111111",
            "--session-file",
        ],
        &[],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("does not match"));
    fs::write(f.path(), format!("{}\n", assistant("a1", None, 1, 1, 1))).unwrap();
    let out = f.cmd(&["pi", "--session", ID, "--session-file"], &[]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("session header"));
}

#[test]
fn abandoned_branches_do_not_supply_the_reading() {
    let f = Fixture::new();
    f.rows(&[
        user("u1", None),
        assistant("a1", Some("u1"), 0, 40_000, 0),
        assistant("big", Some("a1"), 0, 140_000, 0),
        user("u2", Some("a1")),
        assistant("a2", Some("u2"), 0, 42_000, 0),
    ]);
    let r = f.report();
    assert_eq!(r["latest_request"]["input_tokens"], 42_000);
    assert_eq!(r["recent_requests"].as_array().unwrap().len(), 2);
}

#[test]
fn a_rollover_starts_an_epoch_without_borrowing_the_old_reading() {
    let f = Fixture::new();
    let before = [
        user("u1", None),
        assistant("a1", Some("u1"), 0, 120_000, 0),
        assistant("a2", Some("a1"), 0, 160_000, 0),
    ];
    f.rows(&before);
    let first = parse(f.control(&[]));
    assert_eq!(first["event"]["level"], "in_band");
    let cursor = first["next_control_cursor"].as_str().unwrap().to_owned();

    let mut rows = before.to_vec();
    rows.push(compaction("c1", "a2"));
    f.rows(&rows);
    let r = f.report();
    assert_eq!(r["epoch"], 1);
    assert_eq!(
        r["latest_request_status"],
        "awaiting_first_request_in_epoch"
    );
    assert!(r["latest_request"].is_null());
    let gap = parse(f.control(&["--cursor", &cursor]));
    assert_eq!(gap["event"]["level"], "unavailable");
    assert_eq!(gap["event"]["epoch"], 1);
    let cursor = gap["next_control_cursor"].as_str().unwrap().to_owned();

    rows.push(assistant("a3", Some("c1"), 0, 9_000, 0));
    f.rows(&rows);
    let after = parse(f.control(&["--cursor", &cursor]));
    assert_eq!(after["event"]["level"], "normal");
    assert_eq!(after["event"]["input_tokens"], 9_000);
    assert_eq!(after["event"]["epoch"], 1);
    assert_eq!(after["event"]["growth_sample_count"], 1);
}

#[test]
fn malformed_records_are_disclosed() {
    let f = Fixture::new();
    f.rows(&[user("u1", None), assistant("a1", Some("u1"), 0, 50_000, 0)]);
    let mut text = fs::read_to_string(f.path()).unwrap();
    text.push_str("{not json\n");
    fs::write(f.path(), &text).unwrap();
    assert_eq!(
        f.report()["latest_request_status"],
        "historical_observation"
    );
    let out = parse(f.control(&[]));
    assert_eq!(out["event"]["level"], "unavailable");
    text.push_str(&format!("{}\n", assistant("a2", Some("a1"), 0, 51_000, 0)));
    fs::write(f.path(), &text).unwrap();
    assert_eq!(
        f.report()["latest_request_status"],
        "latest_valid_observation_with_record_gaps"
    );
}

#[test]
fn pi_bindings_take_token_levels_and_no_agent() {
    let f = Fixture::new();
    f.rows(&[user("u1", None), assistant("a1", Some("u1"), 0, 50_000, 0)]);
    let out = f.cmd(
        &["control", "--session", ID, "--rollout"],
        &[
            "--host",
            "pi",
            "--trigger-percent",
            "50",
            "--band-floor-percent",
            "60",
        ],
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("Pi session files record no context window")
    );
    let out = f.control(&["--agent", "abc"]);
    assert!(!out.status.success());
}
