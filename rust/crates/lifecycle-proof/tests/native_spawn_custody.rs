#![recursion_limit = "256"]

mod common;

use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    os::unix::{fs::OpenOptionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

use lifecycle_proof::{
    host::{HostClient, HostReportedIdentity},
    identity::{HostProcess, ProcessIdentity, observe_process, sha256_file},
    provider::{ScriptedProvider, Step},
    receipt::ArtifactBinding,
};
use serde::Serialize;
use serde_json::{Value, json};

use common::{
    BUILD_ID, NATIVE_BINARY, SUBJECT_ID, TASK_ID, create_fixture, preflight_installed_host,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Cut {
    BeforeSpawnTerm,
    AfterSpawnTerm,
    AfterSpawnKill,
}

impl Cut {
    fn from_environment() -> Self {
        match std::env::var("S8_NATIVE_SPAWN_CUT").as_deref() {
            Ok("before_spawn_term") => Self::BeforeSpawnTerm,
            Ok("after_spawn_term") => Self::AfterSpawnTerm,
            Ok("after_spawn_kill") => Self::AfterSpawnKill,
            _ => panic!("S8_NATIVE_SPAWN_CUT must name one approved T22 cut"),
        }
    }

    fn point(self) -> &'static str {
        match self {
            Self::BeforeSpawnTerm => "native_before_spawn",
            Self::AfterSpawnTerm | Self::AfterSpawnKill => "native_after_spawn_before_registration",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::BeforeSpawnTerm => "before-spawn-term",
            Self::AfterSpawnTerm => "after-spawn-term",
            Self::AfterSpawnKill => "after-spawn-kill",
        }
    }
}

struct PreserveFailure {
    root: PathBuf,
    native: Option<ProcessIdentity>,
}

impl Drop for PreserveFailure {
    fn drop(&mut self) {
        if !thread::panicking() {
            return;
        }
        if let Some(native) = &self.native {
            cleanup_exact_native(native);
        }
        if self.root.exists() {
            let retained = self
                .root
                .with_file_name(format!("s8-native-spawn-failure-{}", std::process::id()));
            match fs::rename(&self.root, &retained) {
                Ok(()) => eprintln!(
                    "S8 failed native-spawn root retained at {}",
                    retained.display()
                ),
                Err(error) => eprintln!(
                    "S8 failed native-spawn root preservation failed: original={} error={error}",
                    self.root.display()
                ),
            }
        }
    }
}

fn cleanup_exact_native(expected: &ProcessIdentity) {
    if observe_process(expected.pid).ok().as_ref() != Some(expected) {
        return;
    }
    let outcome = Command::new("/bin/kill")
        .args(["-KILL", &expected.pid.to_string()])
        .env("TMPDIR", std::env::var_os("TMPDIR").unwrap())
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR").unwrap(),
        )
        .status();
    eprintln!(
        "S8 identity-matched escaped native cleanup pid={} outcome={outcome:?}",
        expected.pid
    );
}

fn await_file(path: &Path, budget: Duration) {
    let until = Instant::now() + budget;
    while !path.is_file() {
        assert!(
            Instant::now() < until,
            "timed out waiting for {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn await_exact_process_absent(expected: &ProcessIdentity, budget: Duration) -> bool {
    let until = Instant::now() + budget;
    loop {
        if !Path::new(&format!("/proc/{}", expected.pid)).exists() {
            return true;
        }
        if let Ok(current) = observe_process(expected.pid)
            && current != *expected
        {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[derive(Clone, Debug, Serialize)]
struct ProcStat {
    state: String,
    ppid: u32,
    linux_starttime_ticks: u64,
}

#[derive(Clone, Debug, Serialize)]
struct SigkillObservation {
    state_at_five_seconds: Option<ProcStat>,
    nonexecution_elapsed_ms: u128,
    exact_absent_elapsed_ms: u128,
    last_observed: Option<ProcStat>,
}

fn read_proc_stat(pid: u32) -> Result<Option<ProcStat>, String> {
    let stat = match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("proc stat read: {error}")),
    };
    let tail = stat
        .rsplit_once(") ")
        .ok_or_else(|| "malformed proc stat".to_owned())?
        .1;
    let fields = tail.split_whitespace().collect::<Vec<_>>();
    let state = fields.first().ok_or("proc state absent")?.to_string();
    if state.len() != 1 {
        return Err("invalid proc state".into());
    }
    let ppid = fields
        .get(1)
        .ok_or("proc PPID absent")?
        .parse()
        .map_err(|_| "invalid proc PPID")?;
    let linux_starttime_ticks = fields
        .get(19)
        .ok_or("proc starttime absent")?
        .parse()
        .map_err(|_| "invalid proc starttime")?;
    Ok(Some(ProcStat {
        state,
        ppid,
        linux_starttime_ticks,
    }))
}

fn observe_sigkill_child(expected: &ProcessIdentity) -> SigkillObservation {
    let started = Instant::now();
    let mut nonexecution_elapsed_ms = None;
    let mut exact_absent_elapsed_ms = None;
    let mut last_observed = None;
    let mut state_at_five_seconds = None;
    loop {
        let elapsed = started.elapsed();
        let current = read_proc_stat(expected.pid).unwrap_or_else(|error| {
            panic!("ambiguous original native PID state: {error}; last={last_observed:?}")
        });
        match current.as_ref() {
            None => {
                nonexecution_elapsed_ms.get_or_insert(elapsed.as_millis());
                exact_absent_elapsed_ms.get_or_insert(elapsed.as_millis());
            }
            Some(sample) if sample.linux_starttime_ticks != expected.linux_starttime_ticks => {
                assert!(
                    nonexecution_elapsed_ms.is_some(),
                    "original PID identity changed without witnessed exit: {sample:?}"
                );
                exact_absent_elapsed_ms.get_or_insert(elapsed.as_millis());
            }
            Some(sample) if matches!(sample.state.as_str(), "Z" | "X") => {
                nonexecution_elapsed_ms.get_or_insert(elapsed.as_millis());
            }
            Some(sample) => {
                assert!(
                    nonexecution_elapsed_ms.is_none(),
                    "original native child re-entered active state after exit: {sample:?}"
                );
            }
        }
        if let Some(sample) = current {
            last_observed = Some(sample.clone());
            if elapsed >= Duration::from_secs(5) {
                state_at_five_seconds = Some(sample);
            }
        }
        if elapsed >= Duration::from_secs(5) {
            assert!(
                nonexecution_elapsed_ms.is_some(),
                "original pinned native child still executing at 5s: pid={} last={last_observed:?}",
                expected.pid
            );
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    while exact_absent_elapsed_ms.is_none() {
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(15),
            "original native PID not reaped by 15s: pid={} last={last_observed:?}",
            expected.pid
        );
        match read_proc_stat(expected.pid).unwrap_or_else(|error| {
            panic!(
                "ambiguous original native PID state during reap: {error}; last={last_observed:?}"
            )
        }) {
            None => exact_absent_elapsed_ms = Some(elapsed.as_millis()),
            Some(sample) if sample.linux_starttime_ticks != expected.linux_starttime_ticks => {
                exact_absent_elapsed_ms = Some(elapsed.as_millis());
                last_observed = Some(sample);
            }
            Some(sample) => {
                assert!(
                    matches!(sample.state.as_str(), "Z" | "X"),
                    "original child active after witnessed exit: {sample:?}"
                );
                last_observed = Some(sample);
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    SigkillObservation {
        state_at_five_seconds,
        nonexecution_elapsed_ms: nonexecution_elapsed_ms.unwrap(),
        exact_absent_elapsed_ms: exact_absent_elapsed_ms.unwrap(),
        last_observed,
    }
}

fn direct_child_pids(pid: u32) -> Vec<u32> {
    let mut children = std::collections::BTreeSet::new();
    for task in fs::read_dir(format!("/proc/{pid}/task")).unwrap() {
        let path = task.unwrap().path().join("children");
        for child in fs::read_to_string(path).unwrap().split_whitespace() {
            children.insert(child.parse::<u32>().unwrap());
        }
    }
    children.into_iter().collect()
}

fn rows(database: &Path, query: &str) -> Value {
    let output = Command::new("/usr/bin/sqlite3")
        .args(["-readonly", "-batch", "-json"])
        .arg(database)
        .arg(query)
        .env("TMPDIR", std::env::var_os("TMPDIR").unwrap())
        .env(
            "CARGO_TARGET_DIR",
            std::env::var_os("CARGO_TARGET_DIR").unwrap(),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "read-only SQLite query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if output.stdout.is_empty() {
        json!([])
    } else {
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

fn custody(root: &Path) -> Value {
    let database = root.join("lifecycle-store/lifecycle.sqlite");
    json!({
        "sessions":rows(&database,"SELECT session_id,incarnation_id,allocation_state,thread_id FROM native_sessions ORDER BY session_id"),
        "exits":rows(&database,"SELECT session_id,pid,exit_code,forced,outcome_kind FROM native_process_exits ORDER BY session_id"),
        "turns":rows(&database,"SELECT invocation_id,state,send_intent FROM native_turns ORDER BY invocation_id"),
        "close":rows(&database,"SELECT incarnation_id,unsafe_at_close,joined_json,unresolved_json,pending_results_json,native_close_json FROM service_closes ORDER BY wall_ms")
    })
}

fn lifecycle_status(socket: &Path) -> Value {
    let mut stream = UnixStream::connect(socket).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut frame = serde_json::to_vec(&json!({
        "op":"native_delivery_status","subject_id":SUBJECT_ID
    }))
    .unwrap();
    frame.push(b'\n');
    stream.write_all(&frame).unwrap();
    let mut line = String::new();
    assert!(BufReader::new(stream).read_line(&mut line).unwrap() > 0);
    serde_json::from_str(line.trim_end()).unwrap()
}

#[test]
fn installed_native_spawn_custody_cut() {
    let cut = Cut::from_environment();
    let build = preflight_installed_host();
    let tmp = std::env::var_os("TMPDIR").unwrap();
    let provider_root = tempfile::Builder::new()
        .prefix("s8-provider-native-spawn-")
        .tempdir_in(tmp)
        .unwrap();
    let provider = ScriptedProvider::start(
        vec![Step::text(
            "This response must never be requested by a spawn cut.",
        )],
        provider_root.path().join("provider-ledger.jsonl"),
    )
    .unwrap();
    let fixture = create_fixture(&provider.base_url(), build);
    let mut preserve = PreserveFailure {
        root: fixture.root.path().to_path_buf(),
        native: None,
    };
    let barrier = fixture.root.path().join("barrier");
    let point = cut.point();
    fs::write(barrier.join(format!("{point}.arm")), b"arm\n").unwrap();
    let mut host = HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
    await_file(
        &barrier.join(format!("{point}.reached")),
        Duration::from_secs(10),
    );
    host.assert_same_execution().unwrap();
    let initial_custody = custody(fixture.root.path());
    assert_eq!(initial_custody["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(initial_custody["sessions"][0]["allocation_state"], "intent");
    assert!(initial_custody["sessions"][0]["thread_id"].is_null());
    assert!(initial_custody["exits"].as_array().unwrap().is_empty());
    assert!(initial_custody["turns"].as_array().unwrap().is_empty());
    assert!(initial_custody["close"].as_array().unwrap().is_empty());
    assert_eq!(provider.entry_count(), 0);
    let first_process = host.identity().clone();
    let first_children = direct_child_pids(first_process.pid);
    let native = if cut == Cut::BeforeSpawnTerm {
        assert!(
            first_children.is_empty(),
            "child before native spawn: {first_children:?}"
        );
        None
    } else {
        let pid: u32 = fs::read_to_string(barrier.join(format!("{point}.pid")))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(
            first_children.contains(&pid),
            "marker PID is not host child: {first_children:?}"
        );
        let child = observe_process(pid).unwrap();
        assert_eq!(
            child.executable_sha256,
            sha256_file(Path::new(NATIVE_BINARY)).unwrap()
        );
        preserve.native = Some(child.clone());
        Some(child)
    };
    let client = HostClient::new(&fixture.host_socket, Duration::from_secs(5)).unwrap();
    client.await_ready(Duration::from_secs(10)).unwrap();
    let observed = client.observe(TASK_ID, None).unwrap();
    assert_eq!(observed["status"], "observation");
    assert_eq!(observed["components"]["owner"]["state"], "absent");
    let reported = HostReportedIdentity::from_reply(&observed).unwrap();
    reported.bind_process(&first_process, BUILD_ID).unwrap();
    reported.bind_startup(&fixture.startup).unwrap();
    let service_socket = Path::new(
        fixture.startup["lifecycle"]["socket_path"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(lifecycle_status(service_socket)["recovery_only"], false);

    let (first_close, successor, sigkill_observation) = if cut == Cut::AfterSpawnKill {
        let first_close = host.kill_and_reap().unwrap();
        assert_eq!(first_close.signal, Some(9));
        let exact = native.as_ref().unwrap();
        let observation = observe_sigkill_child(exact);
        preserve.native = None;
        let after_kill = custody(fixture.root.path());
        assert_eq!(after_kill, initial_custody);
        let mut successor =
            HostProcess::launch(&fixture.host_executable, &fixture.startup_path).unwrap();
        client.await_ready(Duration::from_secs(10)).unwrap();
        successor.assert_same_execution().unwrap();
        let successor_report = client.observe(TASK_ID, None).unwrap();
        assert_eq!(successor_report["components"]["owner"]["state"], "absent");
        let successor_identity = HostReportedIdentity::from_reply(&successor_report).unwrap();
        successor_identity
            .bind_process(successor.identity(), BUILD_ID)
            .unwrap();
        successor_identity.bind_startup(&fixture.startup).unwrap();
        assert_ne!(successor_identity.incarnation, reported.incarnation);
        assert_eq!(lifecycle_status(service_socket)["recovery_only"], true);
        let successor_children = direct_child_pids(successor.identity().pid);
        assert!(
            successor_children.is_empty(),
            "recovery-only successor spawned children: {successor_children:?}"
        );
        let refused = client.submit(common::initial_submit()).unwrap();
        assert_eq!(refused, json!({"status":"error","code":"recovery_only"}));
        assert_eq!(custody(fixture.root.path()), after_kill);
        let successor_process = successor.identity().clone();
        let successor_close = successor
            .terminate_and_reap(Duration::from_secs(15))
            .unwrap();
        (
            first_close,
            Some(
                json!({"process":successor_process,"reported":successor_identity,"observation":successor_report,"direct_children":successor_children,"refused_new_submit":refused,"close":successor_close}),
            ),
            Some(observation),
        )
    } else {
        host.signal_terminate().unwrap();
        await_file(
            &barrier.join("shutdown_gate_closed.reached"),
            Duration::from_secs(5),
        );
        assert!(
            custody(fixture.root.path())["exits"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        fs::write(barrier.join(format!("{point}.release")), b"release\n").unwrap();
        let first_close = host
            .wait_after_terminate_and_reap(Duration::from_secs(20))
            .unwrap();
        assert!(first_close.reaped);
        if let Some(exact) = &native {
            assert!(await_exact_process_absent(exact, Duration::from_secs(5)));
            preserve.native = None;
        }
        (first_close, None, None)
    };
    let final_custody = custody(fixture.root.path());
    assert_eq!(final_custody["sessions"], initial_custody["sessions"]);
    assert!(final_custody["turns"].as_array().unwrap().is_empty());
    if cut == Cut::AfterSpawnKill {
        assert_eq!(final_custody["exits"], initial_custody["exits"]);
        assert_eq!(final_custody["close"].as_array().unwrap().len(), 1);
        assert_eq!(final_custody["close"][0]["unsafe_at_close"], 1);
    } else {
        assert_eq!(final_custody["exits"].as_array().unwrap().len(), 1);
        assert_eq!(final_custody["exits"][0]["outcome_kind"], "shutdown");
        assert_eq!(
            final_custody["exits"][0]["pid"],
            json!(native.as_ref().map(|id| id.pid))
        );
        assert_eq!(final_custody["close"].as_array().unwrap().len(), 1);
        assert_eq!(final_custody["close"][0]["unsafe_at_close"], 1);
    }
    assert_eq!(provider.entry_count(), 0);
    let provider_entries = provider.entries().unwrap();
    assert!(provider_entries.is_empty());
    fixture.source_manifest.verify().unwrap();
    for source in &fixture.sources {
        source.verify().unwrap();
    }
    assert_eq!(
        sha256_file(&fixture.host_executable).unwrap(),
        fixture.build_provenance.executable_sha256
    );
    if let Some(destination) = std::env::var_os("S8_EVIDENCE_DIR") {
        let destination = PathBuf::from(destination);
        assert!(destination.is_absolute());
        fs::create_dir_all(&destination).unwrap();
        let contract = ArtifactBinding::read(Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v11.md")).unwrap();
        assert_eq!(
            contract.sha256,
            "053e80f90317a98f0711522175c44d6d18457419e17c5a1efbda10c64b41b6e3"
        );
        let predecessor_v10 = ArtifactBinding::read(Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v10.md")).unwrap();
        assert_eq!(
            predecessor_v10.sha256,
            "bc961a7c182688feedc88d52a543787b237685f3336eb11ce415bf2c8fb033ca"
        );
        let predecessor_v9 = ArtifactBinding::read(Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v9.md")).unwrap();
        assert_eq!(
            predecessor_v9.sha256,
            "d6560d2625852eff679d0a9c43265b59cb9a13ad9940694e5f985e3ae86edf2d"
        );
        let predecessor_v8 = ArtifactBinding::read(Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v8.md")).unwrap();
        assert_eq!(
            predecessor_v8.sha256,
            "149639c1a0e2777235d543cd28de3a2dd89a933ad3888dd3e93f971986fba046"
        );
        let predecessor_v7 = ArtifactBinding::read(Path::new("/home/bline/.local/state/work-engine/rust-context-lifecycle-s8-20261008/admission-contract.v7.md")).unwrap();
        assert_eq!(
            predecessor_v7.sha256,
            "cfbe11a0b6cedcc98bf4a2c2e2bd284d921f3b99dbb326622fce1e2d64ab0276"
        );
        let evidence = json!({
            "schema":"s8-installed-native-spawn-custody-v2",
            "cut":cut.label(),
            "contract":contract,
            "predecessor_v10":predecessor_v10,
            "predecessor_v9":predecessor_v9,
            "predecessor_v8":predecessor_v8,
            "predecessor_v7":predecessor_v7,
            "build_provenance":fixture.build_provenance,
            "host_executable":ArtifactBinding::read(&fixture.host_executable).unwrap(),
            "native_binary":ArtifactBinding::read(Path::new(NATIVE_BINARY)).unwrap(),
            "sqlite_read_only_tool":ArtifactBinding::read(Path::new("/usr/bin/sqlite3")).unwrap(),
            "source_manifest":fixture.source_manifest,
            "source_entries":fixture.sources,
            "startup":ArtifactBinding::read(&fixture.startup_path).unwrap(),
            "provider_ledger":ArtifactBinding::read(provider.ledger()).unwrap(),
            "provider_entries":provider_entries,
            "first":{"process":first_process,"reported":reported,"direct_children":first_children,"close":first_close},
            "native_process":native,
            "sigkill_observation":sigkill_observation,
            "initial_custody":initial_custody,
            "final_custody":final_custody,
            "successor":successor
        });
        let path = destination.join(format!(
            "s8-native-spawn-{}-{}.json",
            cut.label(),
            std::process::id()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        file.write_all(&serde_json::to_vec(&evidence).unwrap())
            .unwrap();
        file.sync_all().unwrap();
        eprintln!(
            "S8 native spawn receipt {} sha256 {}",
            path.display(),
            sha256_file(&path).unwrap()
        );
        let _ = fixture.root.keep();
        let _ = provider_root.keep();
    }
}
