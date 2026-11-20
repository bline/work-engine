//! The `prepare` and `profile describe` command shapes: exit codes and the
//! refusal object on stderr (planning/pi-host-production-composition-20261120.md).
use std::process::Command;

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_work-engine-pi-host"))
        .args(args)
        .env_clear()
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn usage_exits_2_and_refusals_exit_1_with_their_class() {
    for args in [
        &["bogus"][..],
        &[
            "prepare",
            "--root",
            "/r",
            "--profile",
            "/p",
            "--snapshot-id",
            "s",
        ],
        &["profile", "describe"],
    ] {
        assert_eq!(run(args).0, 2, "{args:?}");
    }
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("absent.json");
    let absent = absent.to_str().unwrap();
    for args in [
        &["profile", "describe", "--profile", absent][..],
        &[
            "prepare",
            "--profile",
            absent,
            "--root",
            "/nonexistent/r",
            "--snapshot-id",
            "s",
        ],
    ] {
        let (code, stdout, stderr) = run(args);
        assert_eq!(code, 1, "{args:?}");
        assert!(stdout.is_empty());
        let refusal: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap();
        assert_eq!(refusal["status"], "refused");
        assert_eq!(refusal["class"], "profile");
        assert!(refusal["error"].is_string());
    }
}
