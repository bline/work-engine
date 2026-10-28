use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const ID: &str = "11111111-1111-4111-8111-111111111111";
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "context-control-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join("rollout.jsonl")
    }
    fn write(&self, inputs: &[u64]) {
        self.write_window(inputs, 258400);
    }
    fn write_window(&self, inputs: &[u64], window: u64) {
        let mut rows = vec![json!({"type":"session_meta", "payload":{"id":ID}})];
        rows.extend(inputs.iter().map(|n| json!({"type":"event_msg", "payload":{"type":"token_count", "info":{
            "last_token_usage":{"input_tokens":n,"cached_input_tokens":0}, "model_context_window":window}}})));
        self.rows(rows);
    }
    fn rows(&self, rows: Vec<Value>) {
        fs::write(
            self.path(),
            rows.iter().map(|v| format!("{v}\n")).collect::<String>(),
        )
        .unwrap();
    }
    fn run(&self, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
            .args(["control", "--session", ID, "--rollout"])
            .arg(self.path())
            .args([
                "--trigger-tokens",
                "150000",
                "--band-floor-tokens",
                "171267",
            ])
            .args(extra)
            .output()
            .unwrap()
    }
    fn json(&self, extra: &[&str]) -> Value {
        parse(self.run(extra))
    }
    /// Runs with only the given levels, so percent and token levels can be tested.
    fn bare(&self, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
            .args(["control", "--session", ID, "--rollout"])
            .arg(self.path())
            .args(extra)
            .output()
            .unwrap()
    }
    fn percent(&self, extra: &[&str]) -> Value {
        let mut args = vec!["--trigger-percent", "58", "--band-floor-percent", "66"];
        args.extend_from_slice(extra);
        parse(self.bare(&args))
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
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn cursor(v: &Value) -> &str {
    v["next_control_cursor"].as_str().unwrap()
}

#[test]
fn pressure_transitions_deduplicate_across_processes_and_recover_after_reset() {
    let f = Fixture::new();
    f.write(&[100000]);
    let normal = f.json(&[]);
    assert!(normal["event"].is_null());
    f.write(&[100000, 155000]);
    let high = f.json(&["--cursor", cursor(&normal)]);
    assert_eq!(high["event"]["level"], "planning");
    assert_eq!(high["event"]["authority"], "observation_only");
    f.write(&[155000, 155010]);
    let repeated = f.json(&["--cursor", cursor(&high)]);
    assert!(repeated["event"].is_null());
    f.write(&[100000, 155000, 180000]);
    let critical = f.json(&["--cursor", cursor(&repeated)]);
    assert_eq!(critical["event"]["level"], "in_band");
    f.write(&[100000, 155000, 180000, 30000]);
    let reset = f.json(&["--cursor", cursor(&critical)]);
    assert_eq!(reset["event"]["level"], "normal");
    assert!(reset["event"]["estimated_calls_to_band"].is_null());
}

#[test]
fn forecast_uses_three_same_epoch_samples_and_is_not_a_reset_countdown() {
    let f = Fixture::new();
    f.write(&[100000, 120000]);
    assert!(f.json(&[])["event"].is_null());
    f.write(&[100000, 120000, 140000]);
    let v = f.json(&[]);
    assert_eq!(v["event"]["level"], "approaching_band");
    assert!(v["event"]["estimated_calls_to_band"].as_f64().unwrap() < 2.0);
    assert!(
        v["event"]["forecast_basis"]
            .as_str()
            .unwrap()
            .contains("not a reset deadline")
    );
    assert!(f.json(&["--lead-calls", "0"])["event"].is_null());
}

#[test]
fn cursor_rejects_wrong_identity_source_policy_and_shape() {
    let f = Fixture::new();
    f.write(&[160000]);
    let v = f.json(&[]);
    for extra in [
        vec!["--cursor", cursor(&v), "--trigger-tokens", "140000"],
        vec![
            "--cursor",
            cursor(&v),
            "--session",
            "22222222-2222-4222-8222-222222222222",
        ],
        vec!["--cursor", "{}"],
        vec!["--band-floor-tokens", "100000"],
    ] {
        assert!(!f.run(&extra).status.success());
    }
    let other = Fixture::new();
    other.write(&[160000]);
    assert!(!other.run(&["--cursor", cursor(&v)]).status.success());
}

#[test]
fn unavailable_is_visible_deduplicated_and_does_not_borrow_foreign_usage() {
    let f = Fixture::new();
    let missing = f.json(&[]);
    assert_eq!(missing["event"]["level"], "unavailable");
    assert!(f.json(&["--cursor", cursor(&missing)])["event"].is_null());
    f.write(&[100000]);
    let recovered = f.json(&["--cursor", cursor(&missing)]);
    assert_eq!(recovered["event"]["level"], "normal");
    let bad = f.json(&["--session", "22222222-2222-4222-8222-222222222222"]);
    assert_eq!(bad["event"]["level"], "unavailable");
    assert!(bad["event"]["input_tokens"].is_null());
    fs::write(f.path(), "{invalid}\n").unwrap();
    assert_eq!(f.json(&[])["event"]["level"], "unavailable");
}

#[test]
fn claude_requires_both_root_and_agent_identity_and_uses_total_context() {
    let f = Fixture::new();
    f.rows(vec![json!({"sessionId":ID,"agentId":"child-a","type":"assistant","message":{
        "id":"m1","model":"synthetic-model","usage":{"input_tokens":1000,
            "cache_creation_input_tokens":1000,"cache_read_input_tokens":158000,"output_tokens":1}}})]);
    let v = f.json(&["--host", "claude", "--agent", "child-a"]);
    assert_eq!(v["event"]["input_tokens"], 160000);
    assert_eq!(v["event"]["level"], "planning");
    assert!(v["topic"].as_str().unwrap().ends_with("/agent:child-a"));
    for args in [
        vec!["--host", "claude"],
        vec!["--host", "claude", "--agent", "child-b"],
    ] {
        let v = f.json(&args);
        assert_eq!(v["event"]["level"], "unavailable");
        assert!(v["event"]["input_tokens"].is_null());
    }
}

#[test]
fn observation_gaps_are_disclosed_as_unavailable() {
    let f = Fixture::new();
    f.write(&[160000]);
    use std::io::Write;
    fs::OpenOptions::new()
        .append(true)
        .open(f.path())
        .unwrap()
        .write_all(b"{partial")
        .unwrap();
    let v = f.json(&[]);
    assert_eq!(v["event"]["level"], "unavailable");
    assert!(v["event"]["estimated_calls_to_band"].is_null());
}

#[test]
fn codex_historical_gap_does_not_silence_valid_pressure_or_recent_forecasts() {
    let f = Fixture::new();
    f.write(&[100000, 120000, 140000]);
    let mut original = fs::read_to_string(f.path()).unwrap();
    original.push_str("{bad record}\n");
    fs::write(f.path(), &original).unwrap();
    let missing = f.json(&[]);
    assert_eq!(missing["event"]["level"], "unavailable");
    // Copy valid token rows after the gap, with rising inputs but no reset.
    f.write(&[150000, 155000, 160000]);
    let valid = fs::read_to_string(f.path()).unwrap();
    let rows: Vec<_> = valid.lines().skip(1).collect();
    original.push_str(rows[0]);
    original.push('\n');
    fs::write(f.path(), &original).unwrap();
    let recovered = f.json(&["--cursor", cursor(&missing)]);
    assert_eq!(recovered["event"]["level"], "planning");
    assert!(recovered["event"]["estimated_calls_to_band"].is_null());
    assert_eq!(recovered["event"]["growth_sample_count"], 1);
    assert_eq!(
        recovered["event"]["observation_status"],
        "latest_valid_observation_with_record_gaps"
    );
    original.push_str(&rows[1..].join("\n"));
    original.push('\n');
    fs::write(f.path(), &original).unwrap();
    let forecast = f.json(&["--cursor", cursor(&recovered)]);
    assert_eq!(forecast["event"]["level"], "approaching_band");
    assert_eq!(forecast["event"]["growth_sample_count"], 3);
}

#[test]
fn claude_valid_calls_after_missing_usage_restore_pressure() {
    let f = Fixture::new();
    let mut rows = vec![json!({"sessionId":ID,"type":"assistant","message":{
        "id":"missing","model":"synthetic-model"}})];
    f.rows(rows.clone());
    let missing = f.json(&["--host", "claude"]);
    assert_eq!(missing["event"]["level"], "unavailable");
    for (i, n) in [150000, 155000, 160000].iter().enumerate() {
        rows.push(json!({"sessionId":ID,"type":"assistant","message":{
            "id":format!("valid-{i}"),"model":"synthetic-model","usage":{
                "input_tokens":n,"cache_creation_input_tokens":0,
                "cache_read_input_tokens":0,"output_tokens":1}}}));
    }
    f.rows(rows);
    let recovered = f.json(&["--host", "claude", "--cursor", cursor(&missing)]);
    assert_eq!(recovered["event"]["level"], "approaching_band");
    assert_eq!(recovered["event"]["growth_sample_count"], 3);
    assert_eq!(recovered["event"]["last_record_gap_line"], 1);
}

#[test]
fn actual_codex_zero_reset_is_unavailable_then_smaller_request_recovers() {
    let f = Fixture::new();
    let reset: Value = serde_json::from_str(include_str!("fixtures/codex-reset.json")).unwrap();
    let valid = |n| {
        json!({"type":"event_msg","payload":{"type":"token_count","info":{
        "last_token_usage":{"input_tokens":n,"cached_input_tokens":0},"model_context_window":258400}}})
    };
    let mut rows = vec![
        json!({"type":"session_meta","payload":{"id":ID}}),
        valid(244784),
        json!({"type":"compacted","payload":{}}),
        reset,
    ];
    f.rows(rows.clone());
    let unavailable = f.json(&[]);
    assert_eq!(unavailable["event"]["level"], "unavailable");
    assert!(unavailable["event"]["input_tokens"].is_null());
    assert!(unavailable["event"]["estimated_calls_to_band"].is_null());
    rows.push(valid(28432));
    f.rows(rows);
    let recovered = f.json(&["--cursor", cursor(&unavailable)]);
    assert_eq!(recovered["event"]["level"], "normal");
    assert_eq!(recovered["event"]["input_tokens"], 28432);
    assert!(recovered["event"]["estimated_calls_to_band"].is_null());
}

#[test]
fn claude_usable_request_conformance_and_recovery() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/claude-usable-requests.json")).unwrap();
    for case in cases {
        let f = Fixture::new();
        let real = |id, n| {
            json!({"type":"assistant","sessionId":ID,"message":{
            "id":id,"model":"claude-opus-5-5","usage":{"input_tokens":n,
            "cache_creation_input_tokens":0,"cache_read_input_tokens":0}}})
        };
        let mut rows = vec![
            real("before", 91579),
            json!({"type":"assistant","sessionId":ID,"message":case["message"]}),
        ];
        f.rows(rows.clone());
        let v = f.json(&["--host", "claude"]);
        let state: Value = serde_json::from_str(cursor(&v)).unwrap();
        let expected = if case["context_tokens"].is_null() {
            "unavailable"
        } else {
            "normal"
        };
        assert_eq!(state["level"], expected, "{}", case["name"]);
        if expected == "unavailable" {
            assert!(v["event"]["input_tokens"].is_null(), "{}", case["name"]);
            assert!(v["event"]["headroom_to_band_tokens"].is_null());
            assert!(v["event"]["estimated_calls_to_band"].is_null());
        }
        rows.push(real("after", 25000));
        f.rows(rows);
        let recovered = f.json(&["--host", "claude", "--cursor", cursor(&v)]);
        let state: Value = serde_json::from_str(cursor(&recovered)).unwrap();
        assert_eq!(state["level"], "normal", "{}", case["name"]);
    }
}

#[test]
fn percent_levels_follow_the_recorded_window_and_keep_reporting_in_band() {
    // Sol 6.1 records a 475,000-token window. On 2026-10-28 a root bound with
    // token levels from a 258,400 window reached 74% having heard nothing since
    // in_band at 171,267 (36%).
    let f = Fixture::new();
    f.write_window(&[200000], 475000);
    let normal = f.percent(&[]);
    assert!(normal["event"].is_null());
    f.write_window(&[280000], 475000);
    let planning = f.percent(&["--cursor", cursor(&normal)]);
    assert_eq!(planning["event"]["level"], "planning");
    assert_eq!(planning["event"]["level_basis"], "window_percent");
    assert_eq!(planning["event"]["trigger_tokens"], 275500);
    assert_eq!(planning["event"]["band_floor_tokens"], 313500);
    assert_eq!(planning["event"]["context_percent"], 58.9);
    assert!(planning["event"]["next_step_percent"].is_null());
    f.write_window(&[320000], 475000);
    let band = f.percent(&["--cursor", cursor(&planning)]);
    assert_eq!(band["event"]["level"], "in_band");
    assert_eq!(band["event"]["next_step_percent"], 70.0);
    assert_eq!(band["event"]["headroom_to_window_tokens"], 155000);
    f.write_window(&[330000], 475000);
    let quiet = f.percent(&["--cursor", cursor(&band)]);
    assert!(quiet["event"].is_null(), "69.5% has not crossed 70%");
    let mut last = quiet;
    for (input, next) in [(333000, 75.0), (405268, 90.0), (429188, 95.0)] {
        f.write_window(&[input], 475000);
        let step = f.percent(&["--cursor", cursor(&last)]);
        assert_eq!(step["event"]["level"], "in_band", "{input}");
        assert_eq!(step["event"]["next_step_percent"], next, "{input}");
        last = step;
    }
    // The same shares resolve against a smaller window.
    let small = Fixture::new();
    small.write_window(&[160000], 258400);
    let v = small.percent(&[]);
    assert_eq!(v["event"]["level"], "planning");
    assert_eq!(v["event"]["trigger_tokens"], 149872);
}

#[test]
fn token_levels_on_a_larger_window_still_report_each_step_in_band() {
    let f = Fixture::new();
    f.write_window(&[180000], 475000);
    let band = f.json(&[]);
    assert_eq!(band["event"]["level"], "in_band");
    assert_eq!(band["event"]["level_basis"], "tokens");
    assert_eq!(band["event"]["band_floor_percent"], 36.1);
    f.write_window(&[200000], 475000);
    let step = f.json(&["--cursor", cursor(&band)]);
    assert_eq!(step["event"]["next_step_percent"], 45.0);
    f.write_window(&[205000], 475000);
    assert!(f.json(&["--cursor", cursor(&step)])["event"].is_null());
    f.write_window(&[150000, 155000, 30000], 475000);
    let reset = f.json(&["--cursor", cursor(&step)]);
    assert_eq!(reset["event"]["level"], "normal");
    assert!(reset["event"]["next_step_percent"].is_null());
}

#[test]
fn percent_levels_without_a_recorded_window_are_unavailable() {
    let f = Fixture::new();
    f.rows(vec![
        json!({"type":"session_meta", "payload":{"id":ID}}),
        json!({"type":"event_msg", "payload":{"type":"token_count", "info":{
            "last_token_usage":{"input_tokens":300000,"cached_input_tokens":0}}}}),
    ]);
    let v = f.percent(&[]);
    assert_eq!(v["event"]["level"], "unavailable");
    assert!(v["event"]["input_tokens"].is_null());
    assert!(
        v["event"]["unavailable_detail"]
            .as_str()
            .unwrap()
            .contains("window")
    );
}

#[test]
fn level_binding_rules_and_earlier_cursor_versions_are_rejected() {
    let f = Fixture::new();
    f.write_window(&[300000], 475000);
    for args in [
        vec!["--trigger-percent", "58"],
        vec!["--trigger-percent", "66", "--band-floor-percent", "58"],
        vec!["--trigger-percent", "58", "--band-floor-percent", "101"],
        vec!["--trigger-percent", "0", "--band-floor-percent", "66"],
        vec!["--trigger-percent", "58", "--band-floor-percent", "NaN"],
        vec![
            "--trigger-percent",
            "58",
            "--band-floor-percent",
            "66",
            "--trigger-tokens",
            "150000",
            "--band-floor-tokens",
            "171267",
        ],
        vec![
            "--trigger-percent",
            "58",
            "--band-floor-percent",
            "66",
            "--step-percent",
            "0",
        ],
        vec![],
    ] {
        assert!(!f.bare(&args).status.success(), "{args:?}");
    }
    let claude = f.bare(&[
        "--host",
        "claude",
        "--trigger-percent",
        "58",
        "--band-floor-percent",
        "66",
    ]);
    assert!(String::from_utf8_lossy(&claude.stderr).contains("record no context window"));
    let v = f.percent(&[]);
    let mut old: Value = serde_json::from_str(cursor(&v)).unwrap();
    old["version"] = json!(1);
    let rejected = f.bare(&[
        "--trigger-percent",
        "58",
        "--band-floor-percent",
        "66",
        "--cursor",
        &old.to_string(),
    ]);
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("another listener version"));
    // A different step is a different policy.
    let other_step = f.bare(&[
        "--trigger-percent",
        "58",
        "--band-floor-percent",
        "66",
        "--step-percent",
        "10",
        "--cursor",
        cursor(&v),
    ]);
    assert!(!other_step.status.success());
}
