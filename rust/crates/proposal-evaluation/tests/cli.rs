//! E1-AC6: exit codes 0/1/2, reports that list every violation, and the absence of any CLI
//! path for the packet or projection (E1-AC3).

mod support;
use serde_json::{Value, json};
use support::*;

const MIN_EXIT_TABLE_ROWS: usize = 8;

/// A directory holding a packet, a projection and a set bound to them; returns the set path
/// and the set value.
fn bound(name: &str) -> (std::path::PathBuf, Value) {
    let dir = fresh_dir(name);
    let packet = dir.join("packet.json");
    let projection = dir.join("projection.json");
    let packet_sha = write_json(
        &packet,
        &json!({"schema_version": 1, "proposal_id": "alpha.beta"}),
    );
    let element = |n: u8| {
        json!({
            "revision_id": format!("rev-{n}"),
            "revision_sha256": format!("{n:x}").repeat(64),
            "claim": {"id": format!("claim-{n}"), "profile": "proposal-research-v1"}
        })
    };
    let projection_sha = write_json(
        &projection,
        &json!({
            "schema_version": 1,
            "context_kind": "rust_reviewer_exact_claim_projection_v1",
            "selection_metadata": [element(1), element(2), element(3)]
        }),
    );
    let set = valid_set(&s(&packet), &packet_sha, &s(&projection), &projection_sha);
    let path = dir.join("set.json");
    write_json(&path, &set);
    (path, set)
}

#[test]
fn a_valid_set_exits_0_with_a_pretty_report() {
    let (path, _) = bound("cli-valid");
    let run = cli(&["check", "--kind", "estimate-set", &s(&path)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stderr, "");
    assert!(run.stdout.ends_with("}\n") && !run.stdout.ends_with("\n\n"));
    let report = run.out_json();
    assert_eq!(report["valid"], json!(true));
    assert_eq!(report["kind"], json!("estimate-set"));
    assert_eq!(report["errors"], json!([]));
    assert!(report["notes"].as_array().unwrap().len() >= 2);
    // Byte-stable: a second run prints the same bytes.
    let again = cli(&["check", "--kind", "estimate-set", &s(&path)]);
    assert_eq!(run.stdout, again.stdout);
}

#[test]
fn an_invalid_set_exits_1_and_lists_every_violation() {
    let (path, mut set) = bound("cli-invalid");
    set["evidence_cutoff"] = json!("2026-10-23T10:00:00Z");
    set["authority"]["acceptance_authorized"] = json!(true);
    set["estimates"][0]["interval"] = json!({"low": {"quantity": 12}, "high": {"quantity": 5}});
    set["estimates"][2]["reason"] = json!("");
    write_json(&path, &set);
    let run = cli(&["check", "--kind", "estimate-set", &s(&path)]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stderr, "");
    let pairs = run.pairs();
    assert!(pairs.len() >= 3, "{pairs:?}");
    assert!(has(&pairs, "cutoff_after_issue", "evidence_cutoff"));
    assert!(has(
        &pairs,
        "authority_asserted",
        "authority.acceptance_authorized"
    ));
    assert!(has(&pairs, "interval_out_of_order", "estimates.0.interval"));
    assert!(has(&pairs, "empty_string", "estimates.2.reason"));
    assert_eq!(run.out_json()["valid"], json!(false));
}

#[test]
fn a_pointer_failure_is_reported_with_exit_1() {
    let (path, set) = bound("cli-pointer");
    let packet = set["proposal"]["packet"]["path"]
        .as_str()
        .unwrap()
        .to_string();
    write(std::path::Path::new(&packet), "{\"schema_version\": 1}\n");
    let run = cli(&["check", "--kind", "estimate-set", &s(&path)]);
    assert_eq!(run.code, 1);
    assert!(has(&run.pairs(), "digest_mismatch", "proposal.packet"));
}

#[test]
fn a_wrong_schema_is_a_report_not_a_failure() {
    let (path, mut set) = bound("cli-schema");
    set["schema"] = json!("other-v1");
    write_json(&path, &set);
    let run = cli(&["check", "--kind", "estimate-set", &s(&path)]);
    assert_eq!(run.code, 1);
    assert_eq!(
        run.pairs(),
        vec![("schema_mismatch".to_string(), "schema".to_string())]
    );
}

#[test]
fn exit_2_table_prints_one_error_object_and_nothing_on_stdout() {
    let (path, _) = bound("cli-exit2");
    let file = s(&path);
    let malformed = path.with_file_name("malformed.json");
    write(&malformed, "{not json");
    let missing = s(&path.with_file_name("missing.json"));
    let malformed = s(&malformed);
    let rows: Vec<(&str, Vec<&str>)> = vec![
        ("no subcommand", vec![]),
        ("unknown subcommand", vec!["frobnicate"]),
        ("missing --kind", vec!["check", &file]),
        (
            "unknown kind",
            vec!["check", "--kind", "portfolio-decision", &file],
        ),
        ("missing FILE", vec!["check", "--kind", "estimate-set"]),
        (
            "extra positional",
            vec!["check", "--kind", "estimate-set", &file, &file],
        ),
        (
            "repeated --kind",
            vec![
                "check",
                "--kind",
                "estimate-set",
                "--kind",
                "estimate-set",
                &file,
            ],
        ),
        ("--kind without value", vec!["check", "--kind"]),
        (
            "unreadable FILE",
            vec!["check", "--kind", "estimate-set", &missing],
        ),
        (
            "malformed JSON",
            vec!["check", "--kind", "estimate-set", &malformed],
        ),
    ];
    assert!(rows.len() >= MIN_EXIT_TABLE_ROWS);
    for (label, args) in rows {
        let run = cli(&args);
        assert_eq!(run.code, 2, "{label}: {}", run.stderr);
        assert_eq!(run.stdout, "", "{label}");
        let body = run.err_json();
        assert!(
            body["error"].as_str().is_some_and(|m| !m.is_empty()),
            "{label}"
        );
        assert!(body["errors"].is_array(), "{label}");
        assert!(
            body["usage"]
                .as_str()
                .unwrap()
                .contains("check --kind estimate-set FILE")
        );
    }
}

#[test]
fn no_option_names_a_packet_or_projection_path() {
    let (path, _) = bound("cli-no-path-option");
    let file = s(&path);
    for option in ["--packet", "--projection", "--claim-projection", "--out"] {
        let run = cli(&["check", "--kind", "estimate-set", &file, option, &file]);
        assert_eq!(run.code, 2, "{option}");
        assert_eq!(run.stdout, "", "{option}");
        assert!(run.err_json()["error"].as_str().unwrap().contains(option));
    }
}
