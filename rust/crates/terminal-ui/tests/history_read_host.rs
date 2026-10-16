//! Explicit local qualification against an issued S5 inspection grant and running host.
mod support {
    #[allow(dead_code)]
    pub mod terminal;
}
use std::fs;
use std::path::PathBuf;
use support::terminal::Terminal;

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
        wait_for_pane(
            &mut terminal,
            "Candidate and gate · selected candidate 5",
            "5 / 5 top-level checks passed",
            "candidate5",
        );
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
        terminal.close_history();
    }
}
