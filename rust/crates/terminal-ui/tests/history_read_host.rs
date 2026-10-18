//! Explicit local qualification against an issued S5 inspection grant and running host.
mod support {
    #[allow(dead_code)]
    pub mod terminal;
}
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use support::terminal::Terminal;

fn local_test_dir(case: &str) -> PathBuf {
    let base = std::env::var_os("S5_TEST_SOCKET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = base.join(format!(
        "pt{}-{case}-{:x}",
        std::process::id(),
        nonce % 0x10000
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&path)
        .unwrap();
    path
}

#[test]
fn silent_owner_shows_neutral_loading_and_quit_closes_local_socket() {
    let dir = local_test_dir("silent");
    let socket = dir.join("s");
    let evidence = dir.join("evidence");
    fs::create_dir(&evidence).unwrap();
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut bytes = [0u8; 256];
        loop {
            match stream.read(&mut bytes) {
                Ok(0) => break,
                Ok(_) => (),
                Err(error) => panic!("local UI did not close socket: {error}"),
            }
        }
    });
    let mut terminal = Terminal::launch_history(&socket, &evidence, 30, 80);
    let elapsed = terminal.wait_for(
        "Local evidence read in progress; no evidence loaded",
        "loading",
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "loading frame: {elapsed:?}"
    );
    assert!(!terminal.screen().contains("ADMITTED"));
    assert!(!terminal.screen().contains("Candidate and gate"));
    let quit = Instant::now();
    terminal.close_history();
    assert!(
        quit.elapsed() < Duration::from_millis(250),
        "quit: {:?}",
        quit.elapsed()
    );
    server.join().unwrap();
    drop(terminal);
    fs::remove_file(socket).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore = "requires S5_QUALIFIED_REPLY from a private admitted owner read"]
fn repeated_retry_then_escape_closes_each_socket_without_late_installation() {
    let reply = fs::read(std::env::var("S5_QUALIFIED_REPLY").unwrap()).unwrap();
    let dir = local_test_dir("retry-esc");
    let socket = dir.join("s");
    let evidence = dir.join("evidence");
    fs::create_dir(&evidence).unwrap();
    let listener = UnixListener::bind(&socket).unwrap();
    let (sent, received) = mpsc::channel();
    let server = std::thread::spawn(move || {
        for index in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let mut request = vec![0; u32::from_be_bytes(length) as usize];
            stream.read_exact(&mut request).unwrap();
            sent.send((index, "accepted")).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut byte = [0];
            assert_eq!(stream.read(&mut byte).unwrap(), 0);
            // A complete old observation cannot install after its UI attempt
            // was abandoned. Whether the peer write itself fails is OS timing.
            let _ = stream.write_all(&(reply.len() as u32).to_be_bytes());
            let _ = stream.write_all(&reply);
            sent.send((index, "closed")).unwrap();
        }
    });
    let mut terminal = Terminal::launch_history(&socket, &evidence, 30, 80);
    let first = terminal.wait_for(
        "Local evidence read in progress; no evidence loaded",
        "retry-esc-loading",
    );
    assert!(first < Duration::from_millis(250));
    for index in 0..3 {
        assert_eq!(
            received.recv_timeout(Duration::from_secs(1)).unwrap(),
            (index, "accepted")
        );
        assert!(!terminal.screen().contains("Candidate and gate"));
        if index < 2 {
            terminal.input(b"r");
        } else {
            let exit = Instant::now();
            terminal.close_history_with(b"\x1b");
            assert!(exit.elapsed() < Duration::from_millis(250));
        }
        assert_eq!(
            received.recv_timeout(Duration::from_secs(1)).unwrap(),
            (index, "closed")
        );
    }
    server.join().unwrap();
    drop(terminal);
    fs::remove_file(socket).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_socket_never_installs_case_facts() {
    let dir = local_test_dir("missing");
    let socket = dir.join("s");
    let evidence = dir.join("evidence");
    fs::create_dir(&evidence).unwrap();
    let mut terminal = Terminal::launch_history(&socket, &evidence, 30, 80);
    terminal.wait_for("Local evidence read unavailable", "missing");
    assert!(!terminal.screen().contains("ADMITTED"));
    assert!(!terminal.screen().contains("Candidate and gate"));
    terminal.close_history();
    drop(terminal);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn refused_v2_and_v1_keep_initial_frame_unadmitted() {
    let dir = local_test_dir("refused");
    let socket = dir.join("s");
    let evidence = dir.join("evidence");
    fs::create_dir(&evidence).unwrap();
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        for attempt in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let mut request = vec![0; u32::from_be_bytes(length) as usize];
            stream.read_exact(&mut request).unwrap();
            if attempt == 0 {
                std::thread::sleep(Duration::from_millis(100));
            }
            let reply = br#"{"status":"refused","reason":"local test refusal"}"#;
            stream
                .write_all(&(reply.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(reply).unwrap();
        }
    });
    let mut terminal = Terminal::launch_history(&socket, &evidence, 30, 80);
    terminal.wait_for(
        "Local evidence read in progress; no evidence loaded",
        "refused-loading",
    );
    terminal.wait_for("Local evidence read unavailable: Refused", "refused-result");
    assert!(!terminal.screen().contains("ADMITTED"));
    assert!(!terminal.screen().contains("Candidate and gate"));
    terminal.close_history();
    server.join().unwrap();
    drop(terminal);
    fs::remove_file(socket).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore = "spends the full 35-second UI attempt budget against a silent local peer"]
fn silent_peer_eventually_shows_retryable_deadline_failure() {
    let dir = local_test_dir("deadline");
    let socket = dir.join("s");
    let evidence = dir.join("evidence");
    fs::create_dir(&evidence).unwrap();
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(40)))
            .unwrap();
        let mut bytes = [0u8; 256];
        loop {
            match stream.read(&mut bytes) {
                Ok(0) => break,
                Ok(_) => (),
                Err(error) => panic!("local deadline left socket open: {error}"),
            }
        }
    });
    let mut terminal = Terminal::launch_history(&socket, &evidence, 30, 80);
    terminal.wait_for(
        "Local evidence read in progress; no evidence loaded",
        "deadline-loading",
    );
    let started = Instant::now();
    terminal.wait_for(
        "Local evidence read unavailable: Unavailable",
        "deadline-failure",
    );
    assert!(started.elapsed() >= Duration::from_secs(34));
    assert!(terminal.screen().contains("press r to retry"));
    assert!(!terminal.screen().contains("Candidate and gate"));
    terminal.close_history();
    server.join().unwrap();
    drop(terminal);
    fs::remove_file(socket).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore = "requires S5_QUALIFIED_REPLY from a private admitted owner read"]
fn refresh_keeps_last_observed_frame_and_marks_failure_stale() {
    let reply = fs::read(std::env::var("S5_QUALIFIED_REPLY").unwrap()).unwrap();
    let dir = local_test_dir("refresh");
    let socket = dir.join("s");
    let evidence = dir.join("evidence");
    fs::create_dir(&evidence).unwrap();
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        for index in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let mut request = vec![0; u32::from_be_bytes(length) as usize];
            stream.read_exact(&mut request).unwrap();
            if index == 0 {
                stream
                    .write_all(&(reply.len() as u32).to_be_bytes())
                    .unwrap();
                stream.write_all(&reply).unwrap();
            }
        }
    });
    let mut terminal = Terminal::launch_history(&socket, &evidence, 30, 80);
    terminal.wait_for(
        "Candidate and gate · selected candidate 5",
        "refresh-initial",
    );
    terminal.wait_for(
        "LAST OBSERVED · refresh Unavailable; prior frame stale",
        "refresh-stale",
    );
    assert!(
        terminal
            .screen()
            .contains("Candidate and gate · selected candidate 5")
    );
    terminal.close_history();
    server.join().unwrap();
    drop(terminal);
    fs::remove_file(socket).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore = "requires S5_QUALIFIED_REPLY from a private admitted owner read"]
fn public_metrics_absence_renders_unavailable_but_corruption_is_rejected() {
    let original = fs::read(std::env::var("S5_QUALIFIED_REPLY").unwrap()).unwrap();
    let mut absent: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let observation = &mut absent["observation"];
    let source_index = observation["base"]["source_reads"]
        .as_array()
        .unwrap()
        .len()
        + 2;
    observation["metrics"] = serde_json::Value::Null;
    observation["metrics_unavailable"] = "Frozen metrics source unavailable under this read".into();
    observation["source_reads"][source_index]["integrity"] = "unavailable".into();
    let unavailable = serde_json::to_vec(&absent).unwrap();
    // The same absent metrics with a contradictory source proof is invalid.
    absent["observation"]["source_reads"][source_index]["integrity"] =
        "raw_git_object_and_row_digest_verified".into();
    let corrupt = serde_json::to_vec(&absent).unwrap();
    for (case, reply) in [
        ("metrics-absent", unavailable),
        ("metrics-corrupt", corrupt),
    ] {
        let dir = local_test_dir(case);
        let socket = dir.join("s");
        let evidence = dir.join("evidence");
        fs::create_dir(&evidence).unwrap();
        let listener = UnixListener::bind(&socket).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let mut request = vec![0; u32::from_be_bytes(length) as usize];
            stream.read_exact(&mut request).unwrap();
            stream
                .write_all(&(reply.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(&reply).unwrap();
        });
        let mut terminal = Terminal::launch_history(&socket, &evidence, 40, 128);
        if case == "metrics-absent" {
            terminal.wait_for(
                "Frozen metrics unavailable through this admitted read",
                case,
            );
            assert!(!terminal.screen().contains("Frozen metric stage_total 8"));
        } else {
            terminal.wait_for("Local evidence read unavailable: InvalidResponse", case);
            assert!(!terminal.screen().contains("Candidate and gate"));
        }
        terminal.close_history();
        server.join().unwrap();
        drop(terminal);
        fs::remove_file(socket).unwrap();
        fs::remove_dir_all(dir).unwrap();
    }
}

fn wait_for_pane(terminal: &mut Terminal, heading: &str, detail: &str, stage: &str) {
    terminal.wait_for(heading, &format!("{stage}-pane"));
    terminal.wait_for(detail, stage);
    assert!(terminal.screen().contains(heading), "wrong pane at {stage}");
}

#[test]
#[ignore = "requires a current private S5 grant and an owner-backed local read host"]
fn real_owner_read_is_navigable_in_one_terminal_session() {
    let socket = PathBuf::from(std::env::var("WORK_ENGINE_S5_HISTORY_SOCKET").unwrap());
    let evidence = PathBuf::from(std::env::var("WORK_ENGINE_S5_HISTORY_EVIDENCE").unwrap());
    fs::create_dir_all(&evidence).unwrap();
    for (rows, cols) in [(30, 80), (40, 128)] {
        let evidence = evidence.join(format!("{cols}x{rows}"));
        fs::create_dir_all(&evidence).unwrap();
        let mut terminal = Terminal::launch_history(&socket, &evidence, rows, cols);
        let loading = terminal.wait_for(
            "Local evidence read in progress; no evidence loaded",
            "owner-visible-loading",
        );
        fs::write(
            evidence.join("owner-visible-loading-ms.txt"),
            format!("{:.6}\n", loading.as_secs_f64() * 1000.0),
        )
        .unwrap();
        assert!(loading < Duration::from_millis(250));
        let first_candidate = Instant::now();
        wait_for_pane(
            &mut terminal,
            "Candidate and gate · selected candidate 5",
            "5 / 5 top-level checks passed",
            "candidate5",
        );
        assert!(first_candidate.elapsed() < Duration::from_secs(6));
        for (start, route) in [
            (b'5', b"sn".as_slice()),
            (b'1', b"sn".as_slice()),
            (b'3', b"ann".as_slice()),
        ] {
            terminal.input(&[start]);
            terminal.wait_for(
                &format!("Candidate and gate · selected candidate {}", start as char),
                "shortcut-start",
            );
            for key in route {
                terminal.input(&[*key]);
            }
            wait_for_pane(
                &mut terminal,
                "Review lineage · selected candidate 4",
                "Review rev  034b4df528510e5f65029aa029f3fd2f78ef39d1",
                "shortcut-candidate4-result",
            );
            assert!(
                !terminal
                    .screen()
                    .contains("24f3f38e5351b48bcd3e069978b22a597f590e82")
            );
        }
        terminal.input(b"1");
        wait_for_pane(
            &mut terminal,
            "Candidate and gate · selected candidate 1",
            "9 / 9 top-level checks passed",
            "candidate1",
        );
        terminal.input(b"n");
        wait_for_pane(
            &mut terminal,
            "Review lineage · selected candidate 1",
            "Review rev  cbf86a8123636e0034e9b1e9af77851e7088e7e1",
            "candidate1-review",
        );
        terminal.input(b"3");
        wait_for_pane(
            &mut terminal,
            "Candidate and gate · selected candidate 3",
            "6 / 6 top-level checks passed",
            "candidate3",
        );
        terminal.input(b"n");
        wait_for_pane(
            &mut terminal,
            "Review lineage · selected candidate 3",
            "Review rev  c02c6bf02836f1c0ee095d8d57335820fe60bf67",
            "candidate3-review",
        );
        assert!(
            !terminal
                .screen()
                .contains("24f3f38e5351b48bcd3e069978b22a597f590e82")
        );
        terminal.input(b"n");
        wait_for_pane(
            &mut terminal,
            "Original reported revision · selected candidate 3",
            "No direct original-report edge",
            "candidate3-no-direct-original",
        );
        terminal.input(b"n");
        wait_for_pane(
            &mut terminal,
            "Finding dispositions · selected candidate 3",
            "c02c6bf02836f1c0ee095d8d57335820fe60bf67",
            "candidate3-findings",
        );
        terminal.input(b"4");
        wait_for_pane(
            &mut terminal,
            "Candidate and gate · selected candidate 4",
            "8 / 8 top-level checks passed",
            "candidate4",
        );
        terminal.input(b"n");
        wait_for_pane(
            &mut terminal,
            "Provider attempts · selected candidate 4",
            "Outcome    error · HTTP 429",
            "candidate4-provider-429",
        );
        terminal.wait_for("Cost       $1.107375", "candidate4-provider-cost");
        assert!(
            terminal
                .screen()
                .contains("Provider attempts · selected candidate 4")
        );
        terminal.input(b"n");
        wait_for_pane(
            &mut terminal,
            "Scheduled retry · selected candidate 4",
            "Status      acknowledged",
            "candidate4-scheduler",
        );
        terminal.wait_for("Presented   null", "candidate4-no-presentation");
        assert!(
            terminal
                .screen()
                .contains("Scheduled retry · selected candidate 4")
        );
        terminal.input(b"n");
        wait_for_pane(
            &mut terminal,
            "Review lineage · selected candidate 4",
            "Review rev  034b4df528510e5f65029aa029f3fd2f78ef39d1",
            "candidate4-review",
        );
        assert!(
            !terminal
                .screen()
                .contains("24f3f38e5351b48bcd3e069978b22a597f590e82")
        );
        terminal.input(b"5");
        wait_for_pane(
            &mut terminal,
            "Candidate and gate · selected candidate 5",
            "5 / 5 top-level checks passed",
            "candidate5-return",
        );
        terminal.input(b"n");
        wait_for_pane(
            &mut terminal,
            "Review lineage · selected candidate 5",
            "Review rev  24f3f38e5351b48bcd3e069978b22a597f590e82",
            "candidate5-review",
        );
        terminal.input(b"n");
        wait_for_pane(
            &mut terminal,
            "Original reported revision · selected candidate 5",
            "Reported    f5773967f8f6a8470161ac4b1552738d49a23d8c",
            "original",
        );
        terminal.input(b"n");
        terminal.wait_for("Finding dispositions · selected candidate 5", "findings");
        terminal.input(b"g");
        terminal.wait_for("Evidence gaps · selected candidate 5", "gaps");
        terminal.wait_for(
            "5 receipt checks / 8 frozen metric stages unresolved",
            "frozen-metrics-discrepancy",
        );
        terminal.close_history();
    }
}
