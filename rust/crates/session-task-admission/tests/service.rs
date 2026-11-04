use std::{
    io::Write,
    process::{Command, Stdio},
};
#[test]
fn unavailable_or_unconfigured_sources_fail_closed() {
    let binary = env!("CARGO_BIN_EXE_session-task-admission");
    let mut child = Command::new(binary)
        .args(["--config", "/nonexistent/fixture/config", "rpc"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"{}").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["protocol"], "task-admission-rpc-v1");
    assert_eq!(value["ok"], false);
    assert!(value.get("result").is_none());
}
