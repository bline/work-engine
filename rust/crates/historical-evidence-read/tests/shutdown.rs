//! The installed read host must close an active owner tree before exiting.
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RUN: &str = "skills-migration-app-server-20260828";

struct Fixture {
    directory: PathBuf,
    host: Option<Child>,
    owner_group: Option<i32>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(mut host) = self.host.take() {
            let _ = host.kill();
            let _ = host.wait();
        }
        if let Some(group) = self.owner_group.take() {
            unsafe {
                libc::kill(-group, libc::SIGKILL);
            }
        }
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn fixture() -> (Fixture, PathBuf, PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "historical-read-shutdown-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let repository = directory.join("repository");
    fs::create_dir(&repository).unwrap();
    for script in [
        "skills/slice-checkpoint/scripts/checkpoint.py",
        "skills/slice-builder/scripts/legacy_gate_inspect.py",
        "skills/independent-review-state/scripts/historical_inspect.py",
        "skills/independent-review-state/scripts/independent_review_state.py",
        "skills/durable-state/scripts/durable_state.py",
    ] {
        let path = repository.join(script);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let source = if script.ends_with("checkpoint.py") {
            "import os, subprocess, time\nchild = subprocess.Popen(['sleep', '30'])\nwith open(os.environ['HISTORICAL_SHUTDOWN_MARKER'], 'w') as output: output.write(f'{os.getpid()} {child.pid}')\ntime.sleep(30)\n"
        } else {
            "print('{}')\n"
        };
        fs::write(path, source).unwrap();
    }
    let checkpoints: Vec<Value> = (1..=6).map(|index| {
        let accepted = index == 6;
        let attempt = index.min(5);
        json!({"kind":if accepted {"accepted".to_string()} else {format!("candidate-{index}")},
            "attempt":attempt,
            "ref":format!("refs/work-engine/checkpoints/{RUN}/slice-6/{}", if accepted {"accepted".to_string()} else {format!("candidate-{index}")}),
            "oid":format!("{index:040}")})
    }).collect();
    let gates: Vec<Value> = (1..=5).map(|index| {
        let last = index == 5;
        json!({"attempt":index,"checkpoint_ref":checkpoints[index-1]["ref"],
            "checkpoint_oid":checkpoints[index-1]["oid"],
            "capture_path":directory.join("unused-capture").to_str().unwrap(),"capture_line":1,
            "capture_sha256":"a".repeat(64),"receipt_sha256":"b".repeat(64),
            "source_kind":if last {"coordinator_constructed_receipt"} else {"slice_builder_runner_output"},
            "call_id":"call","event_id":if last {Some("event")} else {None},
            "output_line":if last {Some(2)} else {None},
            "output_sha256":if last {Some("c".repeat(64))} else {None}})
    }).collect();
    let reviews = [
        ("original-generic", "512fb086b3df879ed552fe59a9513d423b5dfa63905576db73466eea5dc7912c"),
        ("instruction", "b40803c2de2d5698e7ce571505ca2dfa0407d77827cdd377857638da7cd0f539"),
        ("replacement-generic", "8c55f4e88ef8f28cff64f97cd2c8f49b37427a6b5fbab5a75afd9135814148b6"),
    ].iter().map(|(label,key)| json!({"label":label,"key":format!("independent-review-episode-v1/{key}"),"head":"d".repeat(40)})).collect::<Vec<_>>();
    let grant = json!({"schema_version":1,"profile":"historical-s5-local-inspection-v1",
        "grant_id":"shutdown-test","repository":repository.canonicalize().unwrap(),"run_id":RUN,
        "slice_number":6,"issued_at":"2026-10-16T00:00:00Z","expires_at":"2099-01-01T00:00:00Z",
        "issuer_provenance":"local-test","peer_uid":unsafe{libc::geteuid()},
        "audience":"local_terminal_inspection",
        "fields":["checkpoint","gate","metric_discrepancy","review_summary","source_limit"],
        "checkpoints":checkpoints,"gates":gates,"reviews":reviews});
    let grant_path = directory.join("grant.json");
    fs::write(&grant_path, serde_json::to_vec(&grant).unwrap()).unwrap();
    fs::set_permissions(&grant_path, fs::Permissions::from_mode(0o600)).unwrap();
    let socket = directory.join("read.sock");
    (
        Fixture {
            directory,
            host: None,
            owner_group: None,
        },
        grant_path,
        socket,
    )
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !predicate() {
        assert!(Instant::now() < deadline, "host shutdown test deadline");
        thread::sleep(Duration::from_millis(10));
    }
}

fn start(fixture: &mut Fixture, grant: &Path, socket: &Path, marker: &Path) {
    fixture.host = Some(
        Command::new(env!("CARGO_BIN_EXE_historical-evidence-read"))
            .arg("--inspection-grant")
            .arg(grant)
            .arg("--socket")
            .arg(socket)
            .env("HISTORICAL_SHUTDOWN_MARKER", marker)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    wait_until(|| socket.exists());
}

fn signal_and_wait(fixture: &mut Fixture, signal: i32) {
    let child = fixture.host.as_mut().unwrap();
    unsafe {
        libc::kill(child.id() as i32, signal);
    }
    wait_until(|| child.try_wait().unwrap().is_some());
    let mut child = fixture.host.take().unwrap();
    assert!(child.wait().unwrap().success());
}

#[test]
fn term_during_stalled_descendant_reaps_owner_and_unlinks_socket_then_restart_and_int() {
    let (mut fixture, grant, socket) = fixture();
    let marker = fixture.directory.join("owner-pids");
    start(&mut fixture, &grant, &socket, &marker);
    let mut stream = UnixStream::connect(&socket).unwrap();
    let request =
        json!({"operation":"historical_s5.inspect.v1","candidate_attempt":null,"review_key":null})
            .to_string();
    stream
        .write_all(&(request.len() as u32).to_be_bytes())
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    wait_until(|| marker.exists());
    let pids = fs::read_to_string(&marker).unwrap();
    let pids: Vec<i32> = pids
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(pids.len(), 2);
    fixture.owner_group = Some(pids[0]);
    signal_and_wait(&mut fixture, libc::SIGTERM);
    assert!(!socket.exists());
    for pid in pids {
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            -1,
            "owner process {pid} survived shutdown"
        );
    }
    fixture.owner_group = None;
    drop(stream);
    start(&mut fixture, &grant, &socket, &marker);
    signal_and_wait(&mut fixture, libc::SIGINT);
    assert!(!socket.exists());
}
