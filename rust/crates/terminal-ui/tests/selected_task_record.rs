#![cfg(target_os = "linux")]
#[path = "support/terminal.rs"]
#[allow(dead_code)]
mod terminal;
use operator_observation::selected_task::SelectedTask;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

/// The generated consumer set (operator-observation `tests/fixture_set.rs`).
fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../operator-observation/tests/fixtures/selected-task")
        .join(format!("{name}.json"));
    let bytes = fs::read(path).unwrap();
    SelectedTask::parse(&bytes).unwrap();
    bytes
}
fn evidence(label: &str) -> PathBuf {
    let dir = std::env::var_os("U2_PTY_EVIDENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join(format!("s1-pty-{}", std::process::id())))
        .join(label);
    fs::create_dir_all(&dir).unwrap();
    dir
}
fn write(record: &std::path::Path, bytes: &[u8]) {
    fs::write(record, bytes).unwrap();
    fs::set_permissions(record, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
#[ignore = "actual PTY; run explicitly with --ignored"]
fn shared_record_detail_and_back_preserves_selection() {
    let evidence = evidence("detail-back");
    let record = evidence.join("current.json");
    let bytes = fixture("joined-fresh");
    write(&record, &bytes);
    let s = SelectedTask::parse(&bytes).unwrap();
    let mut ui =
        terminal::Terminal::launch_selected(&record, &s.endpoint, &s.request.task_id, &evidence);
    ui.wait_for("SELECTED TASK", "selected");
    ui.wait_for(&s.request.task_id, "identity");
    ui.input(b"\x1b[B\x1b[B\r");
    ui.wait_for("owner detail", "detail");
    ui.input(b"b");
    ui.wait_for("Owner observations", "back");
    ui.wait_for(&s.request.task_id, "selection-retained");
    ui.close_history();
}

#[test]
#[ignore = "actual PTY; run explicitly with --ignored"]
fn shared_record_set_failure_anchor_and_identity() {
    let evidence = evidence("transitions");
    let record = evidence.join("current.json");
    let first = fixture("joined-fresh");
    let s = SelectedTask::parse(&first).unwrap();
    write(&record, &first);
    let mut ui =
        terminal::Terminal::launch_selected(&record, &s.endpoint, &s.request.task_id, &evidence);
    ui.wait_for("Question question:u2", "pending-owner-id");
    ui.wait_for("Independent synthetic result", "published-result");
    ui.input(b"\x1b[B\x1b[B\r");
    ui.wait_for("owner detail", "initial-detail");
    write(&record, &fixture("stale-present"));
    ui.input(b"r");
    ui.wait_for("Owner observations", "revision-resets-detail");
    ui.wait_for("stale last known", "owner-failure");
    ui.wait_for("last_known_owner", "retained-basis");
    write(&record, &fixture("retained-anchor-refused"));
    ui.input(b"r");
    ui.wait_for("Owner record refused", "refused-anchor");
    // Malformed file is a failed file read, retaining prior values under the same selection.
    write(&record, b"{}");
    ui.input(b"r");
    ui.wait_for("invalid_projection", "invalid-file");
    ui.wait_for("stale last known", "file-failure-stale");
    write(&record, &fixture("unobserved"));
    ui.input(b"r");
    ui.wait_for("Compact summary unavailable", "unobserved");
    // A valid projection for a different explicit endpoint must clear all rows.
    let mut other = SelectedTask::parse(&first).unwrap();
    other.endpoint = "/other/host.sock".into();
    other.scope_id = operator_observation::digest(
        &serde_json::to_vec(&serde_json::json!([
            other.endpoint,
            other.request,
            other.source_id,
            other.host_identity
        ]))
        .unwrap(),
    );
    other.validate().unwrap();
    write(&record, &serde_json::to_vec(&other).unwrap());
    ui.input(b"r");
    ui.wait_for("selected_endpoint_or_task_mismatch", "identity-clear");
    assert!(!ui.screen().contains("Independent synthetic result"));
    assert!(!ui.screen().contains("Question question:u2"));
    ui.wait_for(&s.request.task_id, "selection-retained-after-clear");
    ui.close_history();
}

#[test]
#[ignore = "actual PTY; run explicitly with --ignored"]
fn full_u64_revision_digits_and_revision_only_reset() {
    let evidence = evidence("revision-only");
    let record = evidence.join("current.json");
    let bytes = fixture("large-revision-redacted");
    let mut p = SelectedTask::parse(&bytes).unwrap();
    write(&record, &bytes);
    let mut ui =
        terminal::Terminal::launch_selected(&record, &p.endpoint, &p.request.task_id, &evidence);
    ui.wait_for("Owner observations", "overview");
    ui.input(b"\x1b[B\x1b[B\r");
    ui.wait_for("owner detail", "detail");
    ui.wait_for("9007199254740993", "owner-revision-digits");
    ui.wait_for("18446744073709551615", "cursor-revision-digits");
    // Only the revision changes; the open inspection resets to the overview.
    p.components.get_mut("owner").unwrap().revision["owner_revision"] =
        serde_json::json!("9007199254740994");
    p.components.get_mut("owner").unwrap().last_known_revision["owner_revision"] =
        serde_json::json!("9007199254740994");
    write(&record, &serde_json::to_vec(&p).unwrap());
    ui.input(b"r");
    ui.wait_for("Owner observations", "revision-reset");
    ui.close_history();
}
