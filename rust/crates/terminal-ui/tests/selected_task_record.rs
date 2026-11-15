#![cfg(target_os = "linux")]
#[path = "support/terminal.rs"]
#[allow(dead_code)]
mod terminal;
#[test]
#[ignore = "requires U2_SHARED_RECORD and U2_PTY_EVIDENCE"]
fn shared_record_detail_and_back_preserves_selection() {
    let record = std::path::PathBuf::from(std::env::var_os("U2_SHARED_RECORD").unwrap());
    let evidence = std::path::PathBuf::from(std::env::var_os("U2_PTY_EVIDENCE").unwrap());
    let s =
        operator_observation::selected_task::SelectedTask::parse(&std::fs::read(&record).unwrap())
            .unwrap();
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
    println!(
        "Shared Rust record rendered in actual PTY; detail/back retained explicit selection and terminal restored."
    );
}

#[test]
#[ignore = "requires Rust U2_VECTOR_DIR and U2_PTY_EVIDENCE"]
fn shared_record_changing_failure_reconnect_and_identity() {
    use operator_observation::selected_task::SelectedTask;
    use std::{fs, path::PathBuf};
    let vectors = PathBuf::from(std::env::var_os("U2_VECTOR_DIR").unwrap());
    let evidence = PathBuf::from(std::env::var_os("U2_PTY_EVIDENCE").unwrap());
    fs::create_dir_all(&evidence).unwrap();
    let record = evidence.join("current.json");
    let read = |name: &str| {
        let bytes = fs::read(vectors.join(format!("vector-{name}.json"))).unwrap();
        SelectedTask::parse(&bytes).unwrap();
        bytes
    };
    let first = read("initial");
    let s = SelectedTask::parse(&first).unwrap();
    fs::write(&record, &first).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&record, fs::Permissions::from_mode(0o600)).unwrap();
    let mut ui =
        terminal::Terminal::launch_selected(&record, &s.endpoint, &s.request.task_id, &evidence);
    ui.wait_for("Question question:u2", "pending-owner-id");
    ui.input(b"\x1b[B\x1b[B\r");
    ui.wait_for("owner detail", "initial-detail");
    fs::write(&record, read("answered")).unwrap();
    ui.input(b"r");
    ui.wait_for("Owner observations", "revision-resets-detail");
    ui.input(b"b");
    ui.wait_for("Owner observations", "changed-back");
    ui.wait_for("Yes, publish", "changed-answer");
    fs::write(&record, read("failed")).unwrap();
    ui.input(b"r");
    ui.wait_for("stale last known", "owner-failure");
    ui.wait_for("Independent synthetic result", "retained-result");
    fs::write(&record, read("reconnected")).unwrap();
    ui.input(b"r");
    ui.wait_for("read_completed", "reconnected");
    // Malformed file is a failed file read, retaining prior values under the same selection.
    fs::write(&record, b"{}").unwrap();
    ui.input(b"r");
    ui.wait_for("invalid_projection", "invalid-file");
    ui.wait_for("stale last known", "file-failure-stale");
    fs::write(&record, read("host")).unwrap();
    ui.input(b"r");
    ui.wait_for("read_completed", "new-host");
    ui.input(b"\x1b[B\x1b[B\r");
    ui.wait_for("owner detail", "new-host-detail");
    // A valid projection for a different explicit endpoint must clear all rows.
    let mut other = SelectedTask::parse(&read("source")).unwrap();
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
    fs::write(&record, serde_json::to_vec(&other).unwrap()).unwrap();
    ui.input(b"r");
    ui.wait_for("selected_endpoint_or_task_mismatch", "identity-clear");
    assert!(!ui.screen().contains("Independent synthetic result"));
    assert!(!ui.screen().contains("Question question:u2"));
    ui.wait_for(&s.request.task_id, "selection-retained-after-clear");
    ui.close_history();
}

#[test]
#[ignore = "requires R3 U2_VECTOR_DIR and U2_PTY_EVIDENCE"]
fn r3_revision_only_inspection_reset() {
    use std::{fs,path::PathBuf};
    let dir=PathBuf::from(std::env::var_os("U2_VECTOR_DIR").unwrap());
    let evidence=PathBuf::from(std::env::var_os("U2_PTY_EVIDENCE").unwrap()).join("r3-revision-only");fs::create_dir_all(&evidence).unwrap();
    for field in ["owner_revision","cursor_revision"] {
        let bytes=fs::read(dir.join(format!("r2-{field}_9007199254740993_fresh.json"))).unwrap();
        let mut p=operator_observation::selected_task::SelectedTask::parse(&bytes).unwrap();
        let record=evidence.join(format!("{field}.json"));fs::write(&record,bytes).unwrap();
        use std::os::unix::fs::PermissionsExt;fs::set_permissions(&record,fs::Permissions::from_mode(0o600)).unwrap();
        let mut ui=terminal::Terminal::launch_selected(&record,&p.endpoint,&p.request.task_id,&evidence);
        ui.wait_for("Owner observations","r3-overview");ui.input(b"\x1b[B\x1b[B\r");ui.wait_for("owner detail","r3-detail");
        // Metadata inspection scroll position is immaterial to reset: only these two strings change.
        p.components.get_mut("owner").unwrap().revision[field]=serde_json::json!("9007199254740994");
        p.components.get_mut("owner").unwrap().last_known_revision[field]=serde_json::json!("9007199254740994");
        fs::write(&record,serde_json::to_vec(&p).unwrap()).unwrap();ui.input(b"r");ui.wait_for("Owner observations","r3-revision-reset");ui.close_history();
    }
}
