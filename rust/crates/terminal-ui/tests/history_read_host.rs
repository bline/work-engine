//! Explicit local qualification against an issued S5 inspection grant and running host.
mod support {
    #[allow(dead_code)]
    pub mod terminal;
}
use std::fs;
use std::path::PathBuf;
use support::terminal::Terminal;

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
        terminal.wait_for("Candidate and gate", "candidate5");
        terminal.wait_for("5 / 5 top-level checks passed", "five-checks");
        terminal.input(b"1");
        terminal.wait_for("9 / 9 top-level checks passed", "candidate1");
        terminal.input(b"n");
        terminal.wait_for(
            "cbf86a8123636e0034e9b1e9af77851e7088e7e1",
            "candidate1-review",
        );
        terminal.input(b"3");
        terminal.wait_for("6 / 6 top-level checks passed", "candidate3");
        terminal.input(b"n");
        terminal.wait_for(
            "c02c6bf02836f1c0ee095d8d57335820fe60bf67",
            "candidate3-review",
        );
        assert!(
            !terminal
                .screen()
                .contains("24f3f38e5351b48bcd3e069978b22a597f590e82")
        );
        terminal.input(b"n");
        terminal.wait_for(
            "No direct original-report edge",
            "candidate3-no-direct-original",
        );
        terminal.input(b"n");
        terminal.wait_for(
            "c02c6bf02836f1c0ee095d8d57335820fe60bf67",
            "candidate3-findings",
        );
        terminal.input(b"4");
        terminal.wait_for("8 / 8 top-level checks passed", "candidate4");
        terminal.input(b"n");
        terminal.wait_for(
            "034b4df528510e5f65029aa029f3fd2f78ef39d1",
            "candidate4-review",
        );
        assert!(
            !terminal
                .screen()
                .contains("24f3f38e5351b48bcd3e069978b22a597f590e82")
        );
        terminal.input(b"5");
        terminal.wait_for("5 / 5 top-level checks passed", "candidate5-return");
        terminal.input(b"n");
        terminal.wait_for(
            "24f3f38e5351b48bcd3e069978b22a597f590e82",
            "candidate5-review",
        );
        terminal.input(b"n");
        terminal.wait_for("Original reported revision", "original");
        terminal.input(b"n");
        terminal.wait_for("Finding dispositions", "findings");
        terminal.input(b"g");
        terminal.wait_for("Evidence gaps", "gaps");
        terminal.close_history();
    }
}
