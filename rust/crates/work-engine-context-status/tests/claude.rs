use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const SID: &str = "bf60b4c1-34d1-4900-bbe0-15042efb0e2c";
const OTHER_SID: &str = "aaaaaaaa-34d1-4900-bbe0-15042efb0e2c";
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A throwaway `projects/` root.
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "context-status-claude-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn write(&self, rel: &str, body: &str) -> PathBuf {
        let p = self.0.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, body).unwrap();
        p
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn line(v: Value) -> String {
    format!("{v}\n")
}
fn assistant(id: &str, model: &str, usage: Option<(u64, u64, u64, u64)>) -> Value {
    let mut message = json!({
        "id": id,
        "model": model,
        "role": "assistant",
        "content": [{"type": "text", "text": "SECRET-ASSISTANT-TEXT"},
                    {"type": "tool_use", "name": "Bash", "input": {"command": "SECRET-TOOL-INPUT"}}]
    });
    if let Some((i, c, r, o)) = usage {
        message["usage"] = json!({"input_tokens": i, "cache_creation_input_tokens": c,
                                  "cache_read_input_tokens": r, "output_tokens": o});
    }
    json!({"type": "assistant", "sessionId": SID, "timestamp": "2026-10-21T23:00:00Z",
           "effort": "medium", "message": message})
}
fn user() -> Value {
    json!({"type": "user", "sessionId": SID, "message": {"role": "user", "content": "SECRET-PROMPT-BODY"}})
}
fn compact(pre: u64, post: u64) -> Value {
    json!({"type": "system", "subtype": "compact_boundary", "sessionId": SID,
           "content": "SECRET-SUMMARY", "timestamp": "2026-10-21T23:10:00Z",
           "compactMetadata": {"trigger": "auto", "preTokens": pre, "postTokens": post, "durationMs": 1}})
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_work-engine-context-status"))
}
fn run(args: &[&str]) -> Output {
    bin().args(args).output().unwrap()
}
fn ok_json(args: &[&str]) -> Value {
    let out = run(args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(!text.contains("SECRET"), "content leaked: {text}");
    serde_json::from_str(&text).unwrap()
}
fn transcript_json(p: &Path) -> Value {
    ok_json(&["claude", "--transcript", p.to_str().unwrap(), "--json"])
}

#[test]
fn deduplicates_message_id_and_sums_context() {
    let root = Root::new();
    let body = [
        line(user()),
        line(assistant(
            "msg_1",
            "claude-opus-5-5",
            Some((10, 100, 1000, 7)),
        )),
        // Same response written as a second record: counted once.
        line(assistant(
            "msg_1",
            "claude-opus-5-5",
            Some((10, 100, 1000, 7)),
        )),
        line(assistant(
            "msg_2",
            "claude-opus-5-5",
            Some((2, 240, 1200, 3)),
        )),
    ]
    .concat();
    let p = root.write(&format!("proj/{SID}.jsonl"), &body);
    let v = transcript_json(&p);
    assert_eq!(v["host"], "claude-code");
    assert_eq!(v["session_id"], SID);
    assert_eq!(v["agent_id"], Value::Null);
    assert_eq!(v["latest_call"]["context_tokens"], 2 + 240 + 1200);
    assert_eq!(v["latest_call"]["line"], 4);
    assert_eq!(v["latest_model"], "claude-opus-5-5");
    assert_eq!(v["recorded_effort"], "medium");
    assert_eq!(v["context_window"], "not_recorded");
    let c = &v["cumulative"];
    assert_eq!(c["distinct_calls"], 2);
    assert_eq!(c["uncached_tokens"], 110 + 242);
    assert_eq!(c["cached_tokens"], 2200);
    assert_eq!(c["output_tokens"], 10);
    assert_eq!(c["weighted_tokens"], 352.0 + 220.0 + 50.0);
    assert_eq!(v["recent_growth"]["delta_tokens"], 1442 - 1110);
    assert!(
        v["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l.as_str().unwrap().contains("no context window recorded"))
    );
    assert!(v.get("context_fraction_proxy").is_none());
    let text = run(&["claude", "--transcript", p.to_str().unwrap()]);
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(text.contains("Latest call: context 1442 tokens"));
    assert!(!text.contains("SECRET"));
}

#[test]
fn compaction_and_model_change_start_epochs_with_lineage() {
    let root = Root::new();
    let mut later = assistant("msg_4", "claude-sonnet-5", Some((1, 1, 5000, 1)));
    later.as_object_mut().unwrap().remove("effort");
    let body = [
        line(assistant(
            "msg_1",
            "claude-opus-5-5",
            Some((1, 1, 900_000, 1)),
        )),
        line(compact(900_002, 13_000)),
        line(assistant(
            "msg_2",
            "claude-opus-5-5",
            Some((1, 1, 13_000, 1)),
        )),
        line(assistant(
            "msg_3",
            "claude-opus-5-5",
            Some((1, 1, 14_000, 1)),
        )),
        line(later),
    ]
    .concat();
    let p = root.write(&format!("proj/{SID}.jsonl"), &body);
    let v = transcript_json(&p);
    assert_eq!(v["epoch"], 2);
    let lineage = v["lineage"].as_array().unwrap();
    assert_eq!(lineage.len(), 2);
    assert_eq!(lineage[0]["kind"], "compaction");
    assert_eq!(lineage[0]["trigger"], "auto");
    assert_eq!(lineage[0]["pre_tokens"], 900_002);
    assert_eq!(lineage[0]["post_tokens"], 13_000);
    assert_eq!(lineage[0]["line"], 2);
    assert_eq!(lineage[1]["kind"], "model_change");
    assert_eq!(lineage[1]["to"], "claude-sonnet-5");
    // Growth never spans an epoch: one call in the newest epoch, no growth.
    assert_eq!(v["recent_calls"].as_array().unwrap().len(), 1);
    assert_eq!(v["recent_growth"], Value::Null);
    assert_eq!(v["latest_call"]["epoch"], 2);
    assert_eq!(
        v["observed_models"],
        json!(["claude-opus-5-5", "claude-sonnet-5"])
    );
    // A model change without a newly recorded effort leaves effort unknown.
    assert_eq!(v["recorded_effort"], Value::Null);
    // Cumulative accounting spans epochs.
    assert_eq!(v["cumulative"]["distinct_calls"], 4);

    // Compaction is the last record: no current-epoch call is borrowed.
    let p2 = root.write(
        &format!("proj2/{SID}.jsonl"),
        &[
            line(assistant("msg_1", "claude-opus-5-5", Some((1, 1, 9, 1)))),
            line(compact(11, 5)),
        ]
        .concat(),
    );
    let v = transcript_json(&p2);
    assert_eq!(v["latest_call"], Value::Null);
    assert_eq!(v["latest_call_status"], "unavailable");
}

#[test]
fn missing_usage_is_unavailable_not_zero() {
    let root = Root::new();
    let p = root.write(
        &format!("a/{SID}.jsonl"),
        &[
            line(assistant("msg_1", "claude-opus-5-5", Some((5, 5, 50, 1)))),
            line(assistant("msg_2", "claude-opus-5-5", None)),
        ]
        .concat(),
    );
    let v = transcript_json(&p);
    assert_eq!(v["latest_call"]["context_tokens"], 60);
    assert_eq!(
        v["latest_call_status"],
        "latest_valid_observation_with_record_gaps"
    );
    assert_eq!(v["cumulative"]["distinct_calls"], 1);
    assert_eq!(v["cumulative"]["calls_without_usage"], 1);
    assert_eq!(v["diagnostic_counts"]["missing_usage"], 1);

    let only = root.write(
        &format!("b/{SID}.jsonl"),
        &line(assistant("msg_1", "claude-opus-5-5", None)),
    );
    let v = transcript_json(&only);
    assert_eq!(v["latest_call"], Value::Null);
    assert_eq!(v["latest_call_status"], "unavailable");
    assert_eq!(v["cumulative"]["distinct_calls"], 0);
}

#[test]
fn partial_trailing_line_is_disclosed_and_ignored() {
    let root = Root::new();
    let body = line(assistant("msg_1", "claude-opus-5-5", Some((1, 2, 3, 4))))
        + "not json\n"
        + r#"{"type":"assistant","message":{"id":"msg_2","usage":{"input_tok"#;
    let p = root.write(&format!("a/{SID}.jsonl"), &body);
    let v = transcript_json(&p);
    assert_eq!(v["latest_call"]["context_tokens"], 6);
    assert_eq!(v["diagnostic_counts"]["partial_trailing_record"], 1);
    assert_eq!(v["diagnostic_counts"]["malformed_record"], 1);
    assert_eq!(
        v["latest_call_status"],
        "latest_valid_observation_with_record_gaps"
    );
}

#[test]
fn identity_conflict_fails() {
    let root = Root::new();
    let mut foreign = assistant("msg_2", "claude-opus-5-5", Some((1, 1, 1, 1)));
    foreign["sessionId"] = json!(OTHER_SID);
    let p = root.write(
        &format!("a/{SID}.jsonl"),
        &[
            line(assistant("msg_1", "claude-opus-5-5", Some((1, 1, 1, 1)))),
            line(foreign),
        ]
        .concat(),
    );
    let out = run(&["claude", "--transcript", p.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("sessionId mismatch"));
}

#[test]
fn session_resolution_rejects_zero_and_multiple_matches() {
    let root = Root::new();
    let dir = root.0.to_str().unwrap();
    let body = line(assistant("msg_1", "claude-opus-5-5", Some((1, 1, 1, 1))));
    let none = run(&["claude", "--session", SID, "--projects-dir", dir]);
    assert!(!none.status.success());
    assert!(String::from_utf8_lossy(&none.stderr).contains("not found"));

    root.write(&format!("p1/{SID}.jsonl"), &body);
    let mut agent_line = assistant("msg_9", "claude-opus-5-5", Some((1, 1, 40, 1)));
    agent_line["agentId"] = json!("a0e3ac7f");
    root.write(
        &format!("p1/{SID}/subagents/agent-a0e3ac7f.jsonl"),
        &line(agent_line),
    );
    let v = ok_json(&["claude", "--session", SID, "--projects-dir", dir, "--json"]);
    assert_eq!(v["latest_call"]["context_tokens"], 3);
    let v = ok_json(&[
        "claude",
        "--session",
        SID,
        "--agent",
        "a0e3ac7f",
        "--projects-dir",
        dir,
        "--json",
    ]);
    assert_eq!(v["agent_id"], "a0e3ac7f");
    assert_eq!(v["latest_call"]["context_tokens"], 42);
    let wrong_agent = run(&[
        "claude",
        "--session",
        SID,
        "--agent",
        "ffff",
        "--projects-dir",
        dir,
    ]);
    assert!(!wrong_agent.status.success());

    root.write(&format!("p2/{SID}.jsonl"), &body);
    let many = run(&["claude", "--session", SID, "--projects-dir", dir]);
    assert!(!many.status.success());
    assert!(String::from_utf8_lossy(&many.stderr).contains("ambiguous"));

    let traversal = run(&[
        "claude",
        "--session",
        SID,
        "--agent",
        "../x",
        "--projects-dir",
        dir,
    ]);
    assert!(!traversal.status.success());
}

#[test]
fn claude_agents_lists_and_isolates_one_corrupt_agent() {
    let root = Root::new();
    let dir = root.0.to_str().unwrap();
    root.write(&format!("p/{SID}.jsonl"), "");
    let mut good = assistant("msg_1", "claude-opus-5-5", Some((2, 240, 163_125, 9)));
    good["agentId"] = json!("aaa1");
    root.write(&format!("p/{SID}/subagents/agent-aaa1.jsonl"), &line(good));
    root.write(
        &format!("p/{SID}/subagents/agent-aaa1.meta.json"),
        r#"{"agentType":"general-purpose","description":"reviewer","toolUseId":"toolu_1","spawnDepth":1,"model":"opus","finished":true}"#,
    );
    // Corrupt agent: malformed meta.json and a transcript naming another session.
    let mut bad = assistant("msg_2", "claude-opus-5-5", Some((1, 1, 1, 1)));
    bad["sessionId"] = json!(OTHER_SID);
    bad["agentId"] = json!("bbb2");
    root.write(&format!("p/{SID}/subagents/agent-bbb2.jsonl"), &line(bad));
    root.write(
        &format!("p/{SID}/subagents/agent-bbb2.meta.json"),
        "{not json",
    );
    root.write(&format!("p/{SID}/subagents/notes.txt"), "x");

    let v = ok_json(&[
        "claude-agents",
        "--session",
        SID,
        "--projects-dir",
        dir,
        "--with-usage",
        "--json",
    ]);
    assert_eq!(v["agents_listed"], 2);
    assert_eq!(v["completeness"], "partial");
    assert_eq!(v["ignored_entries"], 1);
    let a = &v["agents"][0];
    assert_eq!(a["agent_id"], "aaa1");
    assert_eq!(a["agent_type"], "general-purpose");
    assert_eq!(a["description"], "reviewer");
    assert_eq!(a["finished"], true);
    assert_eq!(a["requested_model"], "opus");
    assert_eq!(a["usage"]["status"], "observed");
    assert_eq!(a["usage"]["latest_call"]["context_tokens"], 163_367);
    assert_eq!(a["usage"]["cumulative"]["distinct_calls"], 1);
    let b = &v["agents"][1];
    assert_eq!(b["agent_id"], "bbb2");
    assert_eq!(b["meta_status"], "unreadable");
    assert_eq!(b["usage"]["status"], "unreadable_transcript");
    assert!(b["usage"].get("latest_call").is_none());

    // Without --with-usage the rows carry no usage key.
    let v = ok_json(&[
        "claude-agents",
        "--session",
        SID,
        "--projects-dir",
        dir,
        "--json",
    ]);
    assert!(v["agents"][0].get("usage").is_none());
    let text = run(&[
        "claude-agents",
        "--session",
        SID,
        "--projects-dir",
        dir,
        "--with-usage",
    ]);
    assert!(text.status.success());
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(text.contains("2 subagent(s), partial"));
    assert!(text.contains("unreadable_transcript"));

    let missing = run(&[
        "claude-agents",
        "--session",
        OTHER_SID,
        "--projects-dir",
        dir,
    ]);
    assert!(!missing.status.success());
}

#[test]
fn claude_modes_offer_help_and_reject_bad_options() {
    let help = run(&["claude", "--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("claude (--transcript FILE"));
    let help = run(&["claude-agents", "--help"]);
    assert!(String::from_utf8_lossy(&help.stdout).contains("claude-agents --session ID"));
    assert!(!run(&["claude"]).status.success());
    assert!(!run(&["claude", "--session"]).status.success());
    assert!(
        !run(&["claude", "--transcript", "x", "--session", SID])
            .status
            .success()
    );
    assert!(
        !run(&["claude", "--transcript", "x", "--agent", "a"])
            .status
            .success()
    );
    assert!(!run(&["claude-agents"]).status.success());
}

#[test]
fn repeated_message_id_takes_largest_output() {
    let root = Root::new();
    let body = [
        // Streaming partial first, final value later; input/cache identical.
        line(assistant(
            "msg_1",
            "claude-opus-5-5",
            Some((1, 10, 100, 16)),
        )),
        line(assistant(
            "msg_1",
            "claude-opus-5-5",
            Some((1, 10, 100, 374)),
        )),
        line(assistant("msg_2", "claude-opus-5-5", Some((1, 20, 110, 8)))),
        line(assistant(
            "msg_2",
            "claude-opus-5-5",
            Some((1, 20, 110, 19_409)),
        )),
        // A later smaller value never lowers it.
        line(assistant("msg_2", "claude-opus-5-5", Some((1, 20, 110, 5)))),
    ]
    .concat();
    let p = root.write(&format!("a/{SID}.jsonl"), &body);
    let v = transcript_json(&p);
    let c = &v["cumulative"];
    assert_eq!(c["distinct_calls"], 2);
    assert_eq!(c["output_tokens"], 374 + 19_409);
    assert_eq!(c["uncached_tokens"], 11 + 21);
    assert_eq!(c["cached_tokens"], 210);
    assert_eq!(c["weighted_tokens"], 32.0 + 21.0 + 5.0 * 19_783.0);
    assert_eq!(v["latest_call"]["output_tokens"], 19_409);
    assert_eq!(v["latest_call"]["line"], 3);
    assert_eq!(v["recent_calls"][0]["output_tokens"], 374);
    assert!(
        v["diagnostic_counts"]
            .get("duplicate_message_usage_differs")
            .is_none()
    );

    // Differing input/cache on a repeat keeps the first record and says so.
    let q = root.write(
        &format!("b/{SID}.jsonl"),
        &[
            line(assistant(
                "msg_1",
                "claude-opus-5-5",
                Some((1, 10, 100, 16)),
            )),
            line(assistant(
                "msg_1",
                "claude-opus-5-5",
                Some((9, 10, 100, 99)),
            )),
        ]
        .concat(),
    );
    let v = transcript_json(&q);
    assert_eq!(v["cumulative"]["output_tokens"], 16);
    assert_eq!(v["diagnostic_counts"]["duplicate_message_usage_differs"], 1);
}

fn fork_ref(agent: &str) -> Value {
    json!({"type": "fork-context-ref", "agentId": agent, "parentSessionId": SID,
           "parentLastUuid": "u-1", "contextLength": 225})
}
fn inherited_call(agent: &str) -> Value {
    let mut v = assistant(
        "msg_parent",
        "claude-opus-5-5",
        Some((2, 3711, 168_055, 3886)),
    );
    v["parentUuid"] = Value::Null;
    v["agentId"] = json!(agent);
    v
}

#[test]
fn fork_excludes_inherited_parent_call() {
    let root = Root::new();
    let mut prompt = user();
    prompt["parentUuid"] = json!("u-1");
    prompt["agentId"] = json!("f0f0");
    let mut own = assistant("msg_own", "claude-opus-5-5", Some((1, 500, 171_000, 40)));
    own["parentUuid"] = json!("u-2");
    own["agentId"] = json!("f0f0");
    let body = [
        line(fork_ref("f0f0")),
        line(inherited_call("f0f0")),
        line(prompt),
        line(own),
    ]
    .concat();
    let p = root.write(&format!("p/{SID}/subagents/agent-f0f0.jsonl"), &body);
    let v = transcript_json(&p);
    assert_eq!(v["agent_id"], "f0f0");
    assert_eq!(v["lineage"][0]["kind"], "fork");
    assert_eq!(v["lineage"][0]["parent_session_id"], SID);
    assert_eq!(v["lineage"][0]["context_length"], 225);
    assert_eq!(v["lineage"][0]["line"], 1);
    assert_eq!(v["epoch"], 0);
    assert_eq!(v["cumulative"]["distinct_calls"], 1);
    assert_eq!(v["cumulative"]["output_tokens"], 40);
    assert_eq!(v["latest_call"]["context_tokens"], 171_501);
    assert_eq!(v["inherited"]["distinct_calls"], 1);
    assert_eq!(v["inherited"]["uncached_tokens"], 3713);
    assert_eq!(v["recent_calls"].as_array().unwrap().len(), 1);
    assert_eq!(v["diagnostic_counts"]["inherited_history_excluded"], 1);

    // A fork with no call of its own never reports the parent's call.
    let q = root.write(
        &format!("p/{SID}/subagents/agent-f0f1.jsonl"),
        &[line(fork_ref("f0f1")), line(inherited_call("f0f1"))].concat(),
    );
    let v = transcript_json(&q);
    assert_eq!(v["latest_call"], Value::Null);
    assert_eq!(v["latest_call_status"], "unavailable");
    assert_eq!(v["cumulative"]["distinct_calls"], 0);
    assert_eq!(v["observed_models"], json!([]));
    assert_eq!(v["inherited"]["distinct_calls"], 1);

    // A non-fork transcript has no inherited block.
    let n = root.write(
        &format!("q/{SID}.jsonl"),
        &line(assistant("msg_1", "claude-opus-5-5", Some((1, 1, 1, 1)))),
    );
    assert!(transcript_json(&n).get("inherited").is_none());

    // claude-agents reports the fork flag, requested effort and inherited calls.
    root.write(
        &format!("p/{SID}/subagents/agent-f0f0.meta.json"),
        r#"{"agentType":"fork","isFork":true,"effort":"high","description":"d","model":"inherit"}"#,
    );
    let v = ok_json(&[
        "claude-agents",
        "--session",
        SID,
        "--projects-dir",
        root.0.to_str().unwrap(),
        "--with-usage",
        "--json",
    ]);
    let a = &v["agents"][0];
    assert_eq!(a["agent_id"], "f0f0");
    assert_eq!(a["is_fork"], true);
    assert_eq!(a["requested_effort"], "high");
    assert_eq!(a["usage"]["inherited_calls"], 1);
    assert_eq!(a["usage"]["cumulative"]["distinct_calls"], 1);
}

#[cfg(unix)]
#[test]
fn symlinks_are_handled_alike_and_never_complete_silently() {
    use std::os::unix::fs::symlink;
    let root = Root::new();
    let dir = root.0.to_str().unwrap();
    let mut call = assistant("msg_1", "claude-opus-5-5", Some((1, 1, 1, 1)));
    call["agentId"] = json!("aaa1");
    let real = root.write("elsewhere/subagents/agent-aaa1.jsonl", &line(call));
    root.write(&format!("p/{SID}.jsonl"), "");
    fs::create_dir_all(root.0.join(format!("p/{SID}"))).unwrap();
    // Symlinked subagents directory: rejected by both subcommands.
    symlink(
        real.parent().unwrap(),
        root.0.join(format!("p/{SID}/subagents")),
    )
    .unwrap();
    let agent_args = [
        "claude",
        "--session",
        SID,
        "--agent",
        "aaa1",
        "--projects-dir",
        dir,
    ];
    assert!(!run(&agent_args).status.success());
    let list = [
        "claude-agents",
        "--session",
        SID,
        "--projects-dir",
        dir,
        "--json",
    ];
    let v = ok_json(&list);
    assert_eq!(v["agents_listed"], 0);
    assert_eq!(v["subagents_dir"], Value::Null);

    // A symlinked agent-named file inside a real subagents/ makes it partial.
    fs::remove_file(root.0.join(format!("p/{SID}/subagents"))).unwrap();
    root.write(&format!("p/{SID}/subagents/agent-bbb2.jsonl"), "");
    symlink(
        &real,
        root.0.join(format!("p/{SID}/subagents/agent-aaa1.jsonl")),
    )
    .unwrap();
    let v = ok_json(&list);
    assert_eq!(v["skipped_agent_entries"], 1);
    assert_eq!(v["completeness"], "partial");
    assert!(!run(&agent_args).status.success());
}

#[test]
fn absent_counters_do_not_make_listing_partial() {
    let root = Root::new();
    let mut call = assistant("msg_1", "claude-opus-5-5", None);
    call["agentId"] = json!("aaa1");
    root.write(&format!("p/{SID}/subagents/agent-aaa1.jsonl"), &line(call));
    root.write(&format!("p/{SID}/subagents/agent-aaa1.meta.json"), "{}");
    let v = ok_json(&[
        "claude-agents",
        "--session",
        SID,
        "--projects-dir",
        root.0.to_str().unwrap(),
        "--with-usage",
        "--json",
    ]);
    assert_eq!(v["agents"][0]["usage"]["status"], "absent_counters");
    assert_eq!(v["completeness"], "complete_within_subagents_directory");
}

#[test]
fn transcript_errors_use_transcript_wording() {
    let root = Root::new();
    let big = "x".repeat(8 * 1024 * 1024 + 1);
    let p = root.write(&format!("a/{SID}.jsonl"), &big);
    let out = run(&["claude", "--transcript", p.to_str().unwrap()]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("transcript line exceeds 8 MiB bound"), "{err}");
    assert!(!err.contains("rollout"));
}
