//! The `work-engine-portfolio` binary: exit codes and output shape (contract 4.2, S2, S6, S8).

mod support;

use serde_json::Value;
use std::process::{Command, Output};
use support::*;

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_work-engine-portfolio"))
        .args(args)
        .output()
        .expect("the binary runs")
}

fn stderr_json(out: &Output) -> Value {
    serde_json::from_slice(&out.stderr).expect("stderr is one JSON object")
}

/// A valid record on disk; returns its absolute path.
fn record(name: &str) -> (World, String) {
    let w = world(name, false);
    let path = w.dir.join("portfolio.json");
    write_json(&path, &w.doc);
    let file = s(&path);
    (w, file)
}

#[test]
fn a_valid_record_exits_0_with_the_report_on_stdout() {
    let (_w, file) = record("cli-valid");
    let out = run(&["check", "--kind", "portfolio-decision", &file]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.ends_with("}\n") && !text.ends_with("\n\n"),
        "pretty plus one newline"
    );
    let report: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["valid"], true);
    assert_eq!(report["kind"], "portfolio-decision");
    assert!(
        text.starts_with("{\n  \"valid\": true,\n  \"kind\""),
        "declared field order, two-space indent: {text}"
    );
}

#[test]
fn an_invalid_record_exits_1_and_a_digest_refusal_is_not_exit_3() {
    let (w, file) = record("cli-invalid");
    w.overwrite("handoff", "{\"edited\": true}");
    let out = run(&["check", "--kind", "portfolio-decision", &file]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["valid"], false);
    assert_eq!(report["errors"][0]["code"], "digest_mismatch");
}

#[test]
fn a_relative_file_path_exits_2_with_path_not_absolute_and_no_stdout() {
    let out = run(&["check", "--kind", "portfolio-decision", "portfolio.json"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let body = stderr_json(&out);
    assert_eq!(body["errors"][0]["code"], "path_not_absolute");
    assert!(
        body["usage"]
            .as_str()
            .unwrap()
            .contains("work-engine-portfolio check")
    );
    let out = run(&[
        "check",
        "--kind",
        "portfolio-decision",
        "/tmp/../portfolio.json",
    ]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "a path with `..` is not normalised"
    );
    assert_eq!(stderr_json(&out)["errors"][0]["code"], "path_not_absolute");
}

#[test]
fn usage_errors_exit_2_and_write_nothing_to_stdout() {
    let (_w, file) = record("cli-usage");
    let cases: Vec<Vec<&str>> = vec![
        vec![],
        vec!["derive"],
        vec!["check", &file],
        vec!["check", "--kind", "comparison", &file],
        vec!["check", "--kind", "portfolio-decision"],
        vec!["check", "--kind", "portfolio-decision", &file, &file],
        vec![
            "check",
            "--kind",
            "portfolio-decision",
            "--kind",
            "portfolio-decision",
            &file,
        ],
        vec![
            "check",
            "--kind",
            "portfolio-decision",
            "--packet",
            "/x",
            &file,
        ],
        vec!["check", "--kind"],
    ];
    for args in cases {
        let out = run(&args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(stderr_json(&out)["error"].is_string(), "{args:?}");
    }
}

#[test]
fn a_file_that_is_not_json_or_not_there_exits_2() {
    let (w, _file) = record("cli-notjson");
    let text = w.dir.join("not-json.txt");
    write_text(&text, "this is not json");
    let out = run(&["check", "--kind", "portfolio-decision", &s(&text)]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let missing = w.dir.join("missing.json");
    let out = run(&["check", "--kind", "portfolio-decision", &s(&missing)]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let out = run(&["check", "--kind", "portfolio-decision", &s(&w.dir)]);
    assert_eq!(out.status.code(), Some(2), "a directory is not a record");
}

#[test]
fn a_structurally_invalid_record_exits_1() {
    let w = world("cli-structure", false);
    let mut doc = w.doc.clone();
    set(&mut doc, "non_effects.schedule_executed", Value::Bool(true));
    let path = w.dir.join("bad.json");
    write_json(&path, &doc);
    let out = run(&["check", "--kind", "portfolio-decision", &s(&path)]);
    assert_eq!(out.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["valid"], false);
}
