use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const ID: &str = "01a15f02-a935-7683-a849-9e919aecfe3c";
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(body: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "context-status-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::write(p.join(format!("rollout-{ID}.jsonl")), body).unwrap();
        Self(p)
    }
    fn path(&self) -> PathBuf {
        self.0.join(format!("rollout-{ID}.jsonl"))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn meta(id: &str) -> String {
    format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\"}}}}\n")
}
fn usage(input: &str, cached: &str, window: &str) -> String {
    format!(
        "{{\"type\":\"event_msg\",\"timestamp\":\"2026-10-21T16:00:00Z\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"last_token_usage\":{{\"input_tokens\":{input},\"cached_input_tokens\":{cached}}},\"model_context_window\":{window}}}}}}}\n"
    )
}
fn run(f: &Fixture, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("--session")
        .arg(ID)
        .arg("--rollout")
        .arg(f.path())
        .args(extra)
        .output()
        .unwrap()
}
fn json(f: &Fixture, extra: &[&str]) -> Value {
    let out = run(f, extra);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
#[test]
fn json_and_human_report_latest_not_cumulative() {
    let f = Fixture::new(&(meta(ID) + &usage("100", "20", "200") + &usage("120", "40", "200")));
    let v = json(&f, &["--json"]);
    assert_eq!(v["latest_request"]["input_tokens"], 120);
    assert_eq!(v["context_fraction_proxy"], 0.6);
    assert_eq!(v["recent_growth"]["delta_tokens"], 20);
    let text = run(&f, &[]);
    assert!(
        String::from_utf8_lossy(&text.stdout)
            .contains("Latest valid observed request input: 120 tokens")
    );
    assert!(!String::from_utf8_lossy(&text.stdout).contains("cumulative"));
}
#[test]
fn identity_missing_duplicate_and_ambiguous_fail() {
    let wrong = Fixture::new(&meta("deadbeef"));
    assert!(!run(&wrong, &["--json"]).status.success());
    let missing = Fixture::new(&usage("10", "0", "20"));
    assert!(!run(&missing, &[]).status.success());
    let duplicate = Fixture::new(&(meta(ID) + &meta(ID)));
    assert!(!run(&duplicate, &[]).status.success());
    let f = Fixture::new(&meta(ID));
    fs::create_dir(f.0.join("other")).unwrap();
    fs::write(
        f.0.join("other").join(format!("rollout-again-{ID}.jsonl")),
        meta(ID),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("--session")
        .arg(ID)
        .arg("--sessions-dir")
        .arg(&f.0)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("ambiguous"));
}
#[test]
fn malformed_partial_and_missing_usage_are_limited() {
    let f = Fixture::new(
        &(meta(ID) + "not json\n" + &usage("10", "0", "20") + "{\"type\":\"event_msg\""),
    );
    let v = json(&f, &["--json"]);
    assert_eq!(v["latest_request"]["input_tokens"], 10);
    assert_eq!(v["limitations"].as_array().unwrap().len(), 2);
    let no_data = Fixture::new(
        &(meta(ID) + "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\"}}\n"),
    );
    let v = json(&no_data, &["--json"]);
    assert!(v["latest_request"].is_null());
    assert!(v["context_fraction_proxy"].is_null());
}
#[test]
fn reset_window_change_and_invalid_cache_do_not_mix_growth() {
    let f = Fixture::new(
        &(meta(ID)
            + &usage("100", "10", "200")
            + "{\"type\":\"compacted\",\"payload\":{}}\n"
            + &usage("30", "5", "200")
            + &usage("35", "50", "200")
            + &usage("40", "10", "300")),
    );
    let v = json(&f, &["--json"]);
    assert_eq!(v["latest_request"]["input_tokens"], 40);
    assert!(v["recent_growth"].is_null());
    assert_eq!(v["context_window_tokens"], 300);
    assert_eq!(v["lineage"].as_array().unwrap().len(), 2);
}
#[test]
fn unsafe_integer_and_economics_require_explicit_components() {
    let f = Fixture::new(
        &(meta(ID) + &usage("18446744073709551616", "0", "200") + &usage("100", "50", "200")),
    );
    let v = json(
        &f,
        &[
            "--json",
            "--fresh-input-tokens",
            "20",
            "--preparation-tokens",
            "10",
            "--launch-tokens",
            "10",
            "--rehydration-tokens",
            "10",
            "--reconstruction-tokens",
            "10",
            "--remaining-calls",
            "2",
        ],
    );
    assert_eq!(v["latest_request"]["input_tokens"], 100);
    assert_eq!(v["economics"]["fixed_overhead"], 40);
    assert_eq!(v["economics"]["break_even_calls"], 2);
    assert!(!run(&f, &["--fresh-input-tokens", "20"]).status.success());
}
#[test]
fn stale_file_is_marked_without_claiming_live_state() {
    let f = Fixture::new(&(meta(ID) + &usage("10", "0", "20")));
    let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    let file = fs::File::open(f.path()).unwrap();
    file.set_modified(old).unwrap();
    let v = json(&f, &["--json"]);
    assert_eq!(v["freshness"], "stale_file_modification");
}

#[test]
fn model_change_without_new_usage_clears_old_context() {
    let f = Fixture::new(
        &(meta(ID)
            + "{\"type\":\"turn_context\",\"payload\":{\"model\":\"first\",\"effort\":\"low\"}}\n"
            + &usage("90", "20", "200")
            + "{\"type\":\"turn_context\",\"payload\":{\"model\":\"second\",\"effort\":\"high\"}}\n"),
    );
    let v = json(&f, &["--json"]);
    assert_eq!(v["recorded_model"], "second");
    assert_eq!(v["recorded_effort"], "high");
    assert!(v["latest_request"].is_null());
    assert!(v["context_fraction_proxy"].is_null());
}

#[test]
fn repeated_telemetry_with_new_timestamps_keeps_original_request_time() {
    fn at(input: u64, total: u64, minute: u32) -> String {
        format!(
            "{{\"type\":\"event_msg\",\"timestamp\":\"2026-10-21T16:{minute:02}:00Z\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"last_token_usage\":{{\"input_tokens\":{input},\"cached_input_tokens\":0}},\"total_token_usage\":{{\"input_tokens\":{total}}},\"model_context_window\":500}}}}}}\n"
        )
    }
    let mut body = meta(ID) + &at(100, 100, 0) + &at(180, 280, 1);
    for minute in 2..10 {
        body.push_str(&at(180, 280, minute));
    }
    let f = Fixture::new(&body);
    let v = json(&f, &["--json"]);
    assert_eq!(v["recent_requests"].as_array().unwrap().len(), 2);
    assert_eq!(v["recent_growth"]["delta_tokens"], 80);
    assert_eq!(v["latest_request"]["timestamp"], "2026-10-21T16:01:00Z");
    assert_eq!(v["observed_at"], "2026-10-21T16:01:00Z");
}

#[test]
fn latest_usage_requires_its_own_valid_window() {
    for value in [
        None,
        Some("0"),
        Some("\"bad\""),
        Some("18446744073709551616"),
    ] {
        let suffix = value
            .map(|v| format!(",\"model_context_window\":{v}"))
            .unwrap_or_default();
        let second = format!(
            "{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"last_token_usage\":{{\"input_tokens\":180,\"cached_input_tokens\":0}}{suffix}}}}}}}\n"
        );
        let f = Fixture::new(&(meta(ID) + &usage("100", "0", "200") + &second));
        let v = json(&f, &["--json"]);
        assert_eq!(v["latest_request"]["input_tokens"], 180);
        assert!(v["context_window_tokens"].is_null());
        assert!(v["context_fraction_proxy"].is_null());
        assert!(v["context_headroom_tokens_proxy"].is_null());
        assert_eq!(v["diagnostic_counts"]["invalid_window"], 1);
        assert_eq!(
            v["latest_request_status"],
            "latest_valid_observation_with_record_gaps"
        );
    }
}

#[test]
fn model_transition_without_effort_does_not_carry_old_effort() {
    let f = Fixture::new(
        &(meta(ID)
            + "{\"type\":\"turn_context\",\"payload\":{\"model\":\"first\",\"effort\":\"high\"}}\n"
            + &usage("100", "0", "200")
            + "{\"type\":\"turn_context\",\"payload\":{\"model\":\"second\"}}\n"),
    );
    let v = json(&f, &["--json"]);
    assert_eq!(v["recorded_model"], "second");
    assert!(v["recorded_effort"].is_null());
    assert!(v["latest_request"].is_null());
}

#[test]
fn many_malformed_records_have_bounded_output_and_exact_count() {
    let f = Fixture::new(&(meta(ID) + &"\n".repeat(100_000)));
    let output = run(&f, &["--json"]);
    assert!(output.status.success());
    assert!(
        output.stdout.len() < 12_000,
        "report bytes: {}",
        output.stdout.len()
    );
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["diagnostic_counts"]["malformed_record"], 100_000);
    assert!(v["limitations"].as_array().unwrap().len() <= 16);
    assert!(v["diagnostics_omitted"].as_u64().unwrap() >= 99_984);
    assert_eq!(v["latest_request_status"], "unavailable");
}

#[test]
fn missing_cumulative_fields_do_not_invent_requests() {
    fn record(minute: u32, input: u64, cumulative: &str) -> String {
        let total = if cumulative.is_empty() {
            String::new()
        } else {
            format!(",\"total_token_usage\":{{{cumulative}}}")
        };
        format!(
            "{{\"type\":\"event_msg\",\"timestamp\":\"2026-10-21T17:{minute:02}:00Z\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"last_token_usage\":{{\"input_tokens\":{input},\"cached_input_tokens\":0}}{total},\"model_context_window\":500}}}}}}\n"
        )
    }
    let body = meta(ID)
        + &record(0, 100, "\"input_tokens\":100,\"output_tokens\":10")
        + &record(1, 180, "\"input_tokens\":280,\"output_tokens\":20")
        + &record(2, 180, "")
        + &record(3, 180, "\"output_tokens\":20")
        + &record(4, 180, "\"input_tokens\":280")
        + &record(5, 180, "\"input_tokens\":281");
    let f = Fixture::new(&body);
    let v = json(&f, &["--json"]);
    assert_eq!(v["recent_requests"].as_array().unwrap().len(), 3);
    assert_eq!(v["recent_growth"]["delta_tokens"], 80);
    assert_eq!(v["recent_requests"][1]["timestamp"], "2026-10-21T17:01:00Z");
    assert_eq!(v["latest_request"]["timestamp"], "2026-10-21T17:05:00Z");
}

#[test]
fn unknown_window_continuity_splits_growth_and_tracks_valid_change() {
    let missing = "{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"last_token_usage\":{\"input_tokens\":180,\"cached_input_tokens\":0}}}}\n";
    let f =
        Fixture::new(&(meta(ID) + &usage("100", "0", "200") + missing + &usage("190", "0", "300")));
    let v = json(&f, &["--json"]);
    assert_eq!(v["latest_request"]["input_tokens"], 190);
    assert_eq!(v["context_window_tokens"], 300);
    assert!(v["recent_growth"].is_null());
    assert_eq!(v["recent_requests"].as_array().unwrap().len(), 1);
    assert_eq!(v["lineage"].as_array().unwrap().len(), 2);
    assert_eq!(v["diagnostic_counts"]["invalid_window"], 1);
}

#[test]
fn child_thread_identity_is_distinct_from_root_session_lineage() {
    const CHILD: &str = "01a15fc7-1225-7151-89df-cc883d248d21";
    let child_meta = format!(
        "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{CHILD}\",\"session_id\":\"{ID}\",\"parent_thread_id\":\"{ID}\",\"agent_path\":\"/root/child\",\"source\":{{\"subagent\":{{\"thread_spawn\":{{\"parent_thread_id\":\"{ID}\",\"agent_path\":\"/root/child\"}}}}}}}}}}\n"
    );
    let f = Fixture::new(&(child_meta.clone() + &usage("10", "0", "20")));
    let child_path = f.0.join(format!("rollout-{CHILD}.jsonl"));
    fs::rename(f.path(), &child_path).unwrap();
    let invoke = |session: &str| {
        Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
            .arg("--session")
            .arg(session)
            .arg("--rollout")
            .arg(&child_path)
            .arg("--json")
            .output()
            .unwrap()
    };
    let child = invoke(CHILD);
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    let v: Value = serde_json::from_slice(&child.stdout).unwrap();
    assert_eq!(v["session_id"], CHILD);
    assert_eq!(v["root_session_id"], ID);
    let wrong = invoke(ID);
    assert!(!wrong.status.success());
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("session_meta.id mismatch"));
    fs::write(&child_path, child_meta.clone() + &child_meta).unwrap();
    assert!(!invoke(CHILD).status.success());
    fs::write(
        &child_path,
        format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{CHILD}\",\"session_id\":123}}}}\n"
        ),
    )
    .unwrap();
    assert!(
        String::from_utf8_lossy(&invoke(CHILD).stderr)
            .contains("session_meta.session_id is malformed")
    );
}

const FORK_PARENT: &str = "01a15ef6-e6e1-7fb0-a6de-ec4868d3f075";

fn fork_fixture(records: Vec<Value>, start: Value) -> String {
    let header = serde_json::json!({"type":"session_meta", "payload":{
        "id":ID, "session_id":ID, "forked_from_id":FORK_PARENT,
        "subagent_history_start_ordinal":start
    }});
    std::iter::once(header)
        .chain(records)
        .enumerate()
        .map(|(i, mut v)| {
            v["ordinal"] = serde_json::json!(i);
            // Copied timestamps do not identify whose history this is.
            v["timestamp"] = serde_json::json!("2026-10-21T16:00:00Z");
            v.to_string() + "\n"
        })
        .collect()
}

fn usage_value(input: &str) -> Value {
    serde_json::from_str(&usage(input, "0", "2000")).unwrap()
}

#[test]
fn fork_boundary_excludes_parent_usage_model_and_compactions() {
    let body = fork_fixture(
        vec![
            serde_json::from_str(&meta(FORK_PARENT)).unwrap(),
            serde_json::json!({"type":"turn_context","payload":{"model":"parent","effort":"high"}}),
            usage_value("1500"),
            serde_json::json!({"type":"compacted","payload":{}}),
            serde_json::json!({"type":"turn_context","payload":{"model":"child","effort":"medium"}}),
            usage_value("100"),
            usage_value("120"),
        ],
        serde_json::json!(5),
    );
    let f = Fixture::new(&body);
    let v = json(&f, &["--json"]);
    assert_eq!(v["session_id"], ID);
    assert_eq!(v["root_session_id"], ID);
    assert_eq!(v["forked_from_id"], FORK_PARENT);
    assert_eq!(v["own_history_start_ordinal"], 5);
    assert_eq!(v["inherited_records_skipped"], 4);
    assert_eq!(v["recorded_model"], "child");
    assert_eq!(v["recorded_effort"], "medium");
    assert_eq!(v["latest_request"]["input_tokens"], 120);
    assert_eq!(v["latest_request"]["epoch"], 0);
    assert_eq!(v["recent_growth"]["delta_tokens"], 20);
    assert_eq!(v["lineage"], serde_json::json!([]));
    assert_eq!(v["context_fraction_proxy"], 0.06);
    assert_eq!(v["latest_request_status"], "latest_valid_observation");
}

#[test]
fn fork_without_own_observations_never_borrows_parent_counters_or_model() {
    let body = fork_fixture(
        vec![
            serde_json::from_str(&meta(FORK_PARENT)).unwrap(),
            serde_json::json!({"type":"turn_context","payload":{"model":"parent","effort":"high"}}),
            usage_value("1500"),
        ],
        serde_json::json!(4),
    );
    for suffix in [
        "",
        "{\"ordinal\":4",
        "{\"ordinal\":4,\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n",
    ] {
        let f = Fixture::new(&(body.clone() + suffix));
        let v = json(&f, &["--json"]);
        for field in [
            "latest_request",
            "context_window_tokens",
            "context_fraction_proxy",
            "recorded_model",
            "recorded_effort",
            "recent_growth",
        ] {
            assert!(v[field].is_null(), "{field}: {}", v[field]);
        }
        assert_eq!(v["recent_requests"], serde_json::json!([]));
    }
}

#[test]
fn fork_boundary_rejects_missing_invalid_or_unattributed_coordinates() {
    for start in [
        Value::Null,
        serde_json::json!(0),
        serde_json::json!(-1),
        serde_json::json!("3"),
        serde_json::json!(1.5),
    ] {
        let f = Fixture::new(&fork_fixture(vec![usage_value("100")], start));
        assert!(!run(&f, &["--json"]).status.success());
    }
    let body = fork_fixture(
        vec![
            serde_json::from_str(&meta(FORK_PARENT)).unwrap(),
            usage_value("100"),
        ],
        serde_json::json!(2),
    );
    let records: Vec<Value> = body
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    for (line, field, value) in [
        (0, "ordinal", Value::Null),
        (1, "ordinal", Value::Null),
        (1, "ordinal", serde_json::json!(0)),
        (2, "ordinal", serde_json::json!(3)),
    ] {
        let mut bad = records.clone();
        bad[line][field] = value;
        let f = Fixture::new(&bad.iter().map(|v| v.to_string() + "\n").collect::<String>());
        assert!(!run(&f, &["--json"]).status.success());
    }
    for value in [Value::Null, serde_json::json!(ID), serde_json::json!(99)] {
        let mut bad = records.clone();
        bad[0]["payload"]["forked_from_id"] = value;
        let f = Fixture::new(&bad.iter().map(|v| v.to_string() + "\n").collect::<String>());
        assert!(!run(&f, &["--json"]).status.success());
    }
}

#[test]
fn fork_rejects_foreign_or_duplicate_metadata_and_foreign_own_events() {
    for records in [
        vec![serde_json::from_str(&meta(ID)).unwrap(), usage_value("100")],
        vec![
            serde_json::from_str(&meta("deadbeef")).unwrap(),
            usage_value("100"),
        ],
        vec![
            serde_json::from_str(&meta(FORK_PARENT)).unwrap(),
            serde_json::from_str(&meta(FORK_PARENT)).unwrap(),
        ],
        vec![
            serde_json::from_str(&meta(FORK_PARENT)).unwrap(),
            serde_json::from_str(&meta(ID)).unwrap(),
        ],
        vec![
            serde_json::from_str(&meta(FORK_PARENT)).unwrap(),
            serde_json::json!({"type":"token_usage_record","payload":{"thread_id":FORK_PARENT}}),
        ],
        vec![
            serde_json::from_str(&meta(FORK_PARENT)).unwrap(),
            serde_json::json!({"type":"event_msg","payload":{"type":"thread_settings_applied","thread_id":FORK_PARENT}}),
        ],
    ] {
        let f = Fixture::new(&fork_fixture(records, serde_json::json!(2)));
        assert!(!run(&f, &["--json"]).status.success());
    }
}

#[test]
fn nested_fork_prefix_is_excluded_and_child_compaction_still_resets() {
    let grandparent = "01a15b93-56c2-74f1-8c97-e471f81bc6e2";
    let body = fork_fixture(
        vec![
            serde_json::json!({"type":"session_meta","payload":{"id":FORK_PARENT,"forked_from_id":grandparent}}),
            serde_json::from_str(&meta(grandparent)).unwrap(),
            usage_value("1500"),
            usage_value("100"),
            serde_json::json!({"type":"compacted","payload":{}}),
            usage_value("30"),
        ],
        serde_json::json!(4),
    );
    let f = Fixture::new(&body);
    let v = json(&f, &["--json"]);
    assert_eq!(v["latest_request"]["input_tokens"], 30);
    assert_eq!(v["latest_request"]["epoch"], 1);
    assert_eq!(v["lineage"].as_array().unwrap().len(), 1);
    assert_eq!(v["inherited_records_skipped"], 3);
}

#[test]
fn fork_rejects_cyclic_ancestry_conflicting_roots_and_malformed_records() {
    let cyclic = fork_fixture(
        vec![
            serde_json::json!({"type":"session_meta","payload":{"id":FORK_PARENT,"forked_from_id":FORK_PARENT}}),
            serde_json::from_str(&meta(FORK_PARENT)).unwrap(),
            usage_value("100"),
        ],
        serde_json::json!(3),
    );
    let wrong_root = fork_fixture(
        vec![
            serde_json::json!({"type":"session_meta","payload":{"id":FORK_PARENT,"session_id":FORK_PARENT}}),
            usage_value("100"),
        ],
        serde_json::json!(2),
    );
    let valid = fork_fixture(
        vec![
            serde_json::from_str(&meta(FORK_PARENT)).unwrap(),
            usage_value("100"),
        ],
        serde_json::json!(2),
    );
    for body in [
        cyclic,
        wrong_root,
        valid.clone() + "not json\n",
        "not json\n".to_owned() + &meta(ID) + &usage("10", "0", "20"),
    ] {
        let f = Fixture::new(&body);
        assert!(!run(&f, &["--json"]).status.success());
    }
    // An ordinary partial tail after child usage preserves only its last
    // valid observation; it does not authorize a new request or identity.
    let f = Fixture::new(&(valid + "{\"ordinal\":3"));
    let v = json(&f, &["--json"]);
    assert_eq!(v["latest_request"]["input_tokens"], 100);
    assert_eq!(
        v["latest_request_status"],
        "latest_valid_observation_with_record_gaps"
    );
}

#[test]
fn agents_lists_direct_and_recursive_children_without_prompt_content() {
    const CHILD: &str = "01a15fc7-1225-7151-89df-cc883d248d21";
    const GRANDCHILD: &str = "01a15fc8-1225-7151-89df-cc883d248d22";
    let f = Fixture::new(&meta(ID));
    let child = format!(
        "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{CHILD}\",\"session_id\":\"{ID}\",\"parent_thread_id\":\"{ID}\",\"agent_path\":\"/root/worker\",\"source\":{{\"subagent\":{{\"thread_spawn\":{{\"parent_thread_id\":\"{ID}\",\"agent_path\":\"/root/worker\"}}}}}},\"base_instructions\":\"SECRET-PROMPT\"}}}}\n"
    );
    let grandchild = format!(
        "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{GRANDCHILD}\",\"session_id\":\"{ID}\",\"parent_thread_id\":\"{CHILD}\",\"agent_path\":\"/root/worker/nested\"}}}}\n"
    );
    fs::write(f.0.join(format!("rollout-{CHILD}.jsonl")), child).unwrap();
    fs::write(f.0.join(format!("rollout-{GRANDCHILD}.jsonl")), grandchild).unwrap();
    fs::write(f.0.join("rollout-unrelated.jsonl"), "{\"type\":\"session_meta\",\"payload\":{\"id\":\"deadbeef\",\"parent_thread_id\":\"cafebabe\"}}\n").unwrap();
    let invoke = |recursive: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"));
        command
            .arg("agents")
            .arg("--parent-thread")
            .arg(ID)
            .arg("--sessions-dir")
            .arg(&f.0)
            .arg("--json");
        if recursive {
            command.arg("--recursive");
        }
        command.output().unwrap()
    };
    let direct = invoke(false);
    assert!(
        direct.status.success(),
        "{}",
        String::from_utf8_lossy(&direct.stderr)
    );
    assert!(!String::from_utf8_lossy(&direct.stdout).contains("SECRET-PROMPT"));
    let v: Value = serde_json::from_slice(&direct.stdout).unwrap();
    assert_eq!(v["agents"].as_array().unwrap().len(), 1);
    assert_eq!(v["agents"][0]["thread_id"], CHILD);
    assert_eq!(v["agents"][0]["root_session_id"], ID);
    assert_eq!(v["completeness"], "complete_within_scanned_directory");
    let recursive = invoke(true);
    let v: Value = serde_json::from_slice(&recursive.stdout).unwrap();
    assert_eq!(v["agents"].as_array().unwrap().len(), 2);
    assert_eq!(v["agents"][1]["thread_id"], GRANDCHILD);
    let text = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--sessions-dir")
        .arg(&f.0)
        .output()
        .unwrap();
    assert!(text.status.success());
    assert!(String::from_utf8_lossy(&text.stdout).contains(CHILD));
    assert!(!String::from_utf8_lossy(&text.stdout).contains("SECRET-PROMPT"));
}

#[test]
fn agents_exposes_duplicate_id_path_and_conflicting_metadata() {
    const CHILD: &str = "01a15fc7-1225-7151-89df-cc883d248d21";
    const OTHER: &str = "01a15fc8-1225-7151-89df-cc883d248d22";
    let f = Fixture::new(&meta(ID));
    let row = |id: &str, path: &str| {
        format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\",\"parent_thread_id\":\"{ID}\",\"agent_path\":\"{path}\"}}}}\n"
        )
    };
    fs::write(f.0.join("a.jsonl"), row(CHILD, "/root/shared")).unwrap();
    fs::write(f.0.join("b.jsonl"), row(CHILD, "/root/shared")).unwrap();
    fs::write(f.0.join("c.jsonl"), row(OTHER, "/root/shared")).unwrap();
    fs::write(f.0.join("bad.jsonl"), format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"beef\",\"parent_thread_id\":\"{ID}\",\"agent_path\":\"/root/a\",\"source\":{{\"subagent\":{{\"thread_spawn\":{{\"agent_path\":\"/root/b\"}}}}}}}}}}\n")).unwrap();
    fs::write(f.0.join("malformed.jsonl"), "{\"type\":\"session_meta\"\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--sessions-dir")
        .arg(&f.0)
        .arg("--json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["completeness"], "partial_or_ambiguous");
    assert_eq!(v["agents"].as_array().unwrap().len(), 3);
    assert_eq!(v["duplicate_thread_ids"][0], CHILD);
    assert_eq!(
        v["ambiguous_agent_paths"]["/root/shared"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(v["skipped_metadata"], 2);
    assert!(!String::from_utf8_lossy(&out.stdout).contains("/root/b\"}}"));
}

#[test]
fn agents_marks_oversized_metadata_as_skipped() {
    let f = Fixture::new(&meta(ID));
    fs::write(f.0.join("oversized.jsonl"), vec![b'x'; 8 * 1024 * 1024 + 1]).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--sessions-dir")
        .arg(&f.0)
        .arg("--json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["skipped_metadata"], 1);
    assert_eq!(v["completeness"], "partial_or_ambiguous");
    assert!(out.stdout.len() < 4_000);
}

#[test]
fn agents_does_not_expand_duplicate_id_from_other_parent() {
    let f = Fixture::new(&meta(ID));
    let row = |id: &str, parent: &str, path: &str| {
        format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\",\"session_id\":\"{ID}\",\"parent_thread_id\":\"{parent}\",\"agent_path\":\"{path}\"}}}}\n"
        )
    };
    fs::write(f.0.join("first.jsonl"), row("ccc", ID, "/root/first")).unwrap();
    fs::write(f.0.join("conflict.jsonl"), row("ccc", "bbb", "/root/other")).unwrap();
    fs::write(
        f.0.join("descendant.jsonl"),
        row("ddd", "ccc", "/root/first/child"),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--recursive")
        .arg("--sessions-dir")
        .arg(&f.0)
        .arg("--json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["completeness"], "partial_or_ambiguous");
    assert_eq!(v["agents"].as_array().unwrap().len(), 1);
    assert_eq!(v["agents"][0]["thread_id"], "ccc");
    assert_eq!(v["duplicate_thread_ids"][0], "ccc");
    assert_eq!(
        v["conflicting_id_candidates"]["ccc"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        v["conflicting_id_candidates"]["ccc"]
            .to_string()
            .contains("bbb")
    );
    assert!(!v["agents"].to_string().contains("ddd"));
    let text = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--recursive")
        .arg("--sessions-dir")
        .arg(&f.0)
        .output()
        .unwrap();
    assert!(text.status.success());
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(text.contains("Ambiguous thread id: ccc"));
    assert!(text.contains("candidate parent=bbb"));
}

#[test]
fn agents_reports_self_and_ancestor_cycles_without_returning_parent() {
    let f = Fixture::new(&meta(ID));
    fs::remove_file(f.path()).unwrap();
    let row = |id: &str, parent: &str| {
        format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\",\"parent_thread_id\":\"{parent}\"}}}}\n"
        )
    };
    fs::write(f.0.join("self.jsonl"), row(ID, ID)).unwrap();
    let invoke = |recursive: bool| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"));
        cmd.arg("agents")
            .arg("--parent-thread")
            .arg(ID)
            .arg("--sessions-dir")
            .arg(&f.0)
            .arg("--json");
        if recursive {
            cmd.arg("--recursive");
        }
        let out = cmd.output().unwrap();
        assert!(out.status.success());
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let direct = invoke(false);
    assert!(direct["agents"].as_array().unwrap().is_empty());
    assert_eq!(direct["cycle_edges"].as_array().unwrap().len(), 1);
    assert_eq!(direct["completeness"], "partial_or_ambiguous");
    fs::remove_file(f.0.join("self.jsonl")).unwrap();
    fs::write(f.0.join("ancestor.jsonl"), row(ID, "ccc")).unwrap();
    fs::write(f.0.join("child.jsonl"), row("ccc", ID)).unwrap();
    let recursive = invoke(true);
    assert_eq!(recursive["agents"].as_array().unwrap().len(), 1);
    assert_eq!(recursive["agents"][0]["thread_id"], "ccc");
    assert_eq!(recursive["cycle_edges"].as_array().unwrap().len(), 1);
    assert_eq!(recursive["completeness"], "partial_or_ambiguous");
}

#[test]
fn agents_requires_option_values_and_offers_help() {
    let f = Fixture::new(&meta(ID));
    let binary = env!("CARGO_BIN_EXE_work-engine-context-status");
    let missing_dir = Command::new(binary)
        .env("CODEX_HOME", &f.0)
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--json")
        .arg("--sessions-dir")
        .output()
        .unwrap();
    assert!(!missing_dir.status.success());
    assert!(
        String::from_utf8_lossy(&missing_dir.stderr).contains("missing value for --sessions-dir")
    );
    assert!(missing_dir.stdout.is_empty());
    let missing_parent = Command::new(binary)
        .arg("agents")
        .arg("--parent-thread")
        .output()
        .unwrap();
    assert!(!missing_parent.status.success());
    assert!(
        String::from_utf8_lossy(&missing_parent.stderr)
            .contains("missing value for --parent-thread")
    );
    let next_flag = Command::new(binary)
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--sessions-dir")
        .arg("--json")
        .output()
        .unwrap();
    assert!(!next_flag.status.success());
    assert!(
        String::from_utf8_lossy(&next_flag.stderr).contains("missing value for --sessions-dir")
    );
    for args in [vec!["--help"], vec!["agents", "--help"]] {
        let out = Command::new(binary).args(args).output().unwrap();
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("agents --parent-thread ID"));
    }
}

#[test]
fn agents_path_ambiguity_changes_completeness() {
    let f = Fixture::new(&meta(ID));
    for child in ["ccc", "ddd"] {
        fs::write(f.0.join(format!("{child}.jsonl")), format!("{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{child}\",\"parent_thread_id\":\"{ID}\",\"agent_path\":\"/root/same\"}}}}\n")).unwrap();
    }
    let out = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--sessions-dir")
        .arg(&f.0)
        .arg("--json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["completeness"], "partial_or_ambiguous");
    assert_eq!(v["duplicate_thread_ids"].as_array().unwrap().len(), 0);
    assert_eq!(
        v["ambiguous_agent_paths"]["/root/same"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

const USAGE_CHILD: &str = "01a15fd1-1225-7151-89df-cc883d248d31";
const QUIET_CHILD: &str = "01a15fd2-1225-7151-89df-cc883d248d32";
const BROKEN_CHILD: &str = "01a15fd3-1225-7151-89df-cc883d248d33";

fn child_meta(id: &str, path: &str) -> String {
    format!(
        "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{id}\",\"session_id\":\"{ID}\",\"parent_thread_id\":\"{ID}\",\"agent_path\":\"{path}\"}}}}\n"
    )
}
fn agents_json(dir: &PathBuf, extra: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--sessions-dir")
        .arg(dir)
        .arg("--json")
        .args(extra)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn agents_with_usage_reports_each_child_pressure() {
    let f = Fixture::new(&meta(ID));
    fs::write(
        f.0.join(format!("rollout-{USAGE_CHILD}.jsonl")),
        child_meta(USAGE_CHILD, "/root/worker") + &usage("1200", "400", "2000"),
    )
    .unwrap();
    let v = agents_json(&f.0, &["--with-usage"]);
    let child = &v["agents"][0];
    assert_eq!(child["thread_id"], USAGE_CHILD);
    assert_eq!(child["usage"]["status"], "observed");
    assert_eq!(child["usage"]["latest_request"]["input_tokens"], 1200);
    assert_eq!(child["usage"]["latest_request"]["cached_input_tokens"], 400);
    assert_eq!(child["usage"]["context_window_tokens"], 2000);
    assert_eq!(child["usage"]["context_fraction_proxy"], 0.6);
    assert_eq!(child["usage"]["lineage_markers"], 0);
    assert!(child["usage"]["reason"].is_null());

    // The parent-facing numbers must be the same ones the single-session report
    // gives for that child, not a second derivation of them.
    let single = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("--session")
        .arg(USAGE_CHILD)
        .arg("--rollout")
        .arg(f.0.join(format!("rollout-{USAGE_CHILD}.jsonl")))
        .arg("--json")
        .output()
        .unwrap();
    let s: Value = serde_json::from_slice(&single.stdout).unwrap();
    assert_eq!(child["usage"]["latest_request"], s["latest_request"]);
    assert_eq!(
        child["usage"]["context_window_tokens"],
        s["context_window_tokens"]
    );
    assert_eq!(
        child["usage"]["context_fraction_proxy"],
        s["context_fraction_proxy"]
    );

    let text = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--sessions-dir")
        .arg(&f.0)
        .arg("--with-usage")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(text.contains("usage: observed input=1200/2000 (60.0%)"));
}

#[test]
fn agents_with_usage_distinguishes_absent_counters_from_unreadable_rollout() {
    let f = Fixture::new(&meta(ID));
    fs::write(
        f.0.join(format!("rollout-{QUIET_CHILD}.jsonl")),
        child_meta(QUIET_CHILD, "/root/quiet"),
    )
    .unwrap();
    fs::write(
        f.0.join(format!("rollout-{BROKEN_CHILD}.jsonl")),
        child_meta(BROKEN_CHILD, "/root/broken") + &child_meta(BROKEN_CHILD, "/root/broken"),
    )
    .unwrap();
    let v = agents_json(&f.0, &["--with-usage"]);
    let rows = v["agents"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let broken = rows
        .iter()
        .find(|r| r["thread_id"] == BROKEN_CHILD)
        .unwrap();
    let quiet = rows.iter().find(|r| r["thread_id"] == QUIET_CHILD).unwrap();

    // Neither case may report a zero, and each must say which kind it is.
    assert_eq!(quiet["usage"]["status"], "absent_counters");
    assert!(quiet["usage"]["latest_request"].is_null());
    assert!(quiet["usage"]["context_fraction_proxy"].is_null());
    assert!(
        quiet["usage"]["reason"]
            .as_str()
            .unwrap()
            .contains("last_token_usage")
    );
    assert_eq!(broken["usage"]["status"], "unreadable_rollout");
    assert!(broken["usage"]["latest_request"].is_null());
    assert!(!broken["usage"]["reason"].as_str().unwrap().is_empty());
    assert_ne!(quiet["usage"]["status"], broken["usage"]["status"]);

    // One unreadable child does not cost the parent its other answers.
    let text = Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
        .arg("agents")
        .arg("--parent-thread")
        .arg(ID)
        .arg("--sessions-dir")
        .arg(&f.0)
        .arg("--with-usage")
        .output()
        .unwrap();
    assert!(text.status.success());
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(text.contains("usage: absent_counters"));
    assert!(text.contains("usage: unreadable_rollout"));
}

#[test]
fn agents_without_usage_flag_keeps_its_exact_shape() {
    let f = Fixture::new(&meta(ID));
    fs::write(
        f.0.join(format!("rollout-{USAGE_CHILD}.jsonl")),
        child_meta(USAGE_CHILD, "/root/worker") + &usage("1200", "400", "2000"),
    )
    .unwrap();
    let plain = agents_json(&f.0, &[]);
    let with_usage = agents_json(&f.0, &["--with-usage"]);
    assert!(plain["agents"][0].get("usage").is_none());
    assert!(with_usage["agents"][0].get("usage").is_some());

    // Dropping the added field must return the previous object exactly, so the
    // flag adds and never rewrites.
    let mut stripped = with_usage.clone();
    for row in stripped["agents"].as_array_mut().unwrap() {
        row.as_object_mut().unwrap().remove("usage");
    }
    assert_eq!(stripped, plain);
}
