//! Transport and input-bound cases that need no registry: faulty fake servers for the MCP
//! reader (CLAIMHANDOFF-003) and bounded CLI input ingestion (CLAIMHANDOFF-002). One
//! sequential test per concern keeps `live_reader_threads()` meaningful in this process.
use claim_evidence::sha256_bytes;
use repository_claims_handoff::*;
use serde_json::Value;
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

/// Tests in this file write executable scripts and spawn processes; running them one at a
/// time avoids the ETXTBSY race where a concurrent fork inherits a script's write descriptor.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const INIT_REPLY: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18"}}"#;

fn fake(dir: &Path, name: &str, body: &str) -> (TrustedServer, PathBuf) {
    let pid_file = dir.join(format!("{name}.pid"));
    let script = dir.join(format!("{name}.py"));
    let source = format!(
        "#!/usr/bin/python3\nimport os, sys, time\nopen({:?}, 'w').write(str(os.getpid()))\nsys.stdin.readline()\n{body}",
        pid_file.display().to_string()
    );
    fs::write(&script, &source).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let profile = dir.join("profile.json");
    fs::write(&profile, b"{}").unwrap();
    (
        TrustedServer {
            executable_sha256: file_sha256(&script).unwrap(),
            executable: script,
            read_profile_sha256: sha256_bytes(b"{}"),
            read_profile: profile,
        },
        pid_file,
    )
}

fn handoff() -> Handoff {
    Handoff::new(
        "root:fixture",
        "transport fault case",
        vec![ExactRef {
            claim_id: "claim-a".into(),
            revision_id: "revision-a".into(),
            revision_sha256: "a".repeat(64),
            evidence_sha256: vec![],
        }],
    )
    .unwrap()
}

fn run(server: &TrustedServer, deadline: Duration) -> (Error, Duration) {
    let start = Instant::now();
    let e = resolve_handoff(
        &handoff(),
        server,
        &Options {
            page_bytes: 4096,
            deadline,
        },
    )
    .unwrap_err();
    (e, start.elapsed())
}

fn assert_cleaned_up(pid_file: &Path) {
    let pid = fs::read_to_string(pid_file).expect("server started and recorded its pid");
    assert!(
        !Path::new(&format!("/proc/{pid}")).exists(),
        "direct child {pid} must be killed and reaped"
    );
    assert_eq!(live_reader_threads(), 0, "reader thread must terminate");
}

#[test]
fn faulty_servers_are_bounded_reaped_and_release_the_reader() {
    let _serial = serial();
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();

    // Silent after initialize: the response deadline refuses and the child is reaped.
    let (s, pid) = fake(
        d,
        "silent",
        &format!("sys.stdout.write({INIT_REPLY:?} + '\\n'); sys.stdout.flush()\ntime.sleep(60)\n"),
    );
    let (e, took) = run(&s, Duration::from_secs(1));
    assert_eq!(
        (e.code(), e.detail()),
        ("server_unavailable", "server response deadline")
    );
    assert!(took < Duration::from_secs(10), "{took:?}");
    assert_cleaned_up(&pid);

    // Silent from the start (never answers initialize).
    let (s, pid) = fake(d, "mute", "time.sleep(60)\n");
    let (e, took) = run(&s, Duration::from_secs(1));
    assert_eq!(e.code(), "server_unavailable");
    assert!(took < Duration::from_secs(10), "{took:?}");
    assert_cleaned_up(&pid);

    // One response frame larger than the per-frame bound: refused by the reader, not buffered
    // further, even though the server keeps writing after it.
    let (s, pid) = fake(
        d,
        "oversized",
        &format!(
            "sys.stdout.write({INIT_REPLY:?} + '\\n'); sys.stdout.flush()\nsys.stdin.readline()\n\
             chunk = 'x' * 65536\nfor _ in range(64):\n    sys.stdout.write(chunk); sys.stdout.flush()\nsys.stdout.write('\\n'); sys.stdout.flush()\ntime.sleep(60)\n"
        ),
    );
    let (e, _) = run(&s, Duration::from_secs(20));
    assert_eq!(
        (e.code(), e.detail()),
        ("protocol", "response line exceeds bound")
    );
    assert_cleaned_up(&pid);

    // Flood of unsolicited frames after a valid initialize. The server tries to write about
    // 2 GiB; backpressure must stop it near the pipe and queue capacity, and the verifier must
    // refuse at the first unsolicited frame.
    let progress = d.join("flood.progress");
    let (s, pid) = fake(
        d,
        "flood",
        &format!(
            "sys.stdout.write({INIT_REPLY:?} + '\\n'); sys.stdout.flush()\n\
             line = '{{\"jsonrpc\":\"2.0\",\"id\":999,\"result\":\"' + 'y' * 524288 + '\"}}\\n'\n\
             for i in range(4096):\n    sys.stdout.write(line); sys.stdout.flush()\n    open({:?}, 'w').write(str((i + 1) * len(line)))\n",
            progress.display().to_string()
        ),
    );
    let (e, took) = run(&s, Duration::from_secs(20));
    assert_eq!((e.code(), e.detail()), ("protocol", "response id mismatch"));
    assert!(took < Duration::from_secs(10), "{took:?}");
    assert_cleaned_up(&pid);
    let written: u64 = fs::read_to_string(&progress)
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);
    assert!(
        written > 0 && written < 8 * 1024 * 1024,
        "server wrote {written} bytes before being stopped; backpressure should bound this"
    );

    // Server that exits immediately after initialize closes the transport cleanly.
    let (s, pid) = fake(
        d,
        "exits",
        &format!("sys.stdout.write({INIT_REPLY:?} + '\\n'); sys.stdout.flush()\n"),
    );
    let (e, _) = run(&s, Duration::from_secs(10));
    assert_eq!(e.code(), "server_unavailable");
    assert_cleaned_up(&pid);
}

struct KillOnDrop(Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn cli(args: &[&str]) -> (Option<i32>, Value, Duration) {
    let start = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_repository-claims-handoff"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let status = wait_for(&mut child, Duration::from_secs(10));
    let mut out = String::new();
    if status.is_some() {
        use std::io::Read;
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut out)
            .unwrap();
    }
    (
        status.and_then(|s| s.code()),
        serde_json::from_str(out.trim()).unwrap_or(Value::Null),
        start.elapsed(),
    )
}
fn wait_for(child: &mut Child, limit: Duration) -> Option<std::process::ExitStatus> {
    let end = Instant::now() + limit;
    while Instant::now() < end {
        if let Some(s) = child.try_wait().unwrap() {
            return Some(s);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();
    None
}

#[test]
fn cli_input_is_bounded_and_non_regular_inputs_refuse_without_blocking() {
    let _serial = serial();
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let refused = |args: &[&str], detail: &str| {
        let (code, v, took) = cli(args);
        assert_eq!(code, Some(2), "{args:?}: {v}");
        assert_eq!(v["code"], "invalid_input", "{args:?}: {v}");
        assert_eq!(v["detail"], detail, "{args:?}: {v}");
        assert!(took < Duration::from_secs(5), "{args:?} took {took:?}");
    };

    // Oversized regular file: refused at the checkpoint, continuity-result and handoff bounds.
    let big = d.join("big.json");
    fs::File::create(&big)
        .unwrap()
        .set_len(16 * 1024 * 1024)
        .unwrap();
    let big = big.to_str().unwrap();
    refused(&["extract", big], "file exceeds bound");
    refused(
        &["extract", "--continuity-result", big],
        "file exceeds bound",
    );
    refused(&["verify", "--handoff", big], "file exceeds bound");

    // Regular file that reports length 0 but yields more than the bound (procfs): ingestion
    // itself must stop at max + 1 bytes rather than trusting the reported size.
    let sleeper = Command::new("python3")
        .args(["-c", "import time; time.sleep(1000)"])
        .arg("x".repeat(100_000))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let sleeper = KillOnDrop(sleeper);
    std::thread::sleep(Duration::from_millis(300));
    let cmdline = format!("/proc/{}/cmdline", sleeper.0.id());
    let reported = fs::metadata(&cmdline).unwrap();
    assert!(reported.is_file() && reported.len() == 0);
    assert!(
        fs::read(&cmdline).unwrap().len() > 65_536,
        "fixture must exceed the checkpoint bound"
    );
    refused(&["extract", &cmdline], "file exceeds bound");
    drop(sleeper);

    // Non-regular inputs: a character device that never ends, a directory, and a FIFO with no
    // writer (opening it blocking would hang).
    refused(&["extract", "/dev/zero"], "input must be a regular file");
    refused(
        &["extract", d.to_str().unwrap()],
        "input must be a regular file",
    );
    let fifo = d.join("pipe");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    refused(
        &["extract", "--continuity-result", fifo.to_str().unwrap()],
        "input must be a regular file",
    );
    refused(
        &["verify", "--handoff", fifo.to_str().unwrap()],
        "input must be a regular file",
    );

    // A symlink to a regular file is still read; an ordinary valid input is unaffected.
    let h = handoff();
    let line = d.join("handoff.txt");
    fs::File::create(&line)
        .unwrap()
        .write_all(h.line().unwrap().as_bytes())
        .unwrap();
    let link = d.join("link");
    std::os::unix::fs::symlink(&line, &link).unwrap();
    let (code, _, _) = cli(&["verify", "--handoff", link.to_str().unwrap()]);
    // Parsed fine; then refused only for the missing trusted-server arguments.
    assert_eq!(code, Some(2));
    let (_, v, _) = cli(&["verify", "--handoff", link.to_str().unwrap()]);
    assert_eq!(
        v["detail"],
        "--server, --server-sha256, --profile, --profile-sha256 required"
    );
}
