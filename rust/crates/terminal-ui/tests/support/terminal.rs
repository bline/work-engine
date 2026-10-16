use nix::{
    fcntl::{FcntlArg, OFlag, fcntl},
    pty::{Winsize, openpty},
    sys::termios::{Termios, tcgetattr},
};
use std::{
    fs::{self, File},
    io::{ErrorKind, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

/// Owns the real UI child and decodes its evolving screen, not accumulated text.
pub struct Terminal {
    child: Child,
    master: File,
    slave: File,
    original: Termios,
    parser: vt100::Parser,
    raw: Vec<u8>,
    evidence: std::path::PathBuf,
    read_deadline: Duration,
}
impl Terminal {
    pub fn launch(socket: &Path, task: &str, delivery: &str, evidence: &Path) -> Self {
        let pty = openpty(
            Some(&Winsize {
                ws_row: 60,
                ws_col: 180,
                ws_xpixel: 0,
                ws_ypixel: 0,
            }),
            None,
        )
        .unwrap();
        let master = File::from(pty.master);
        let slave = File::from(pty.slave);
        let original = tcgetattr(&slave).unwrap();
        let flags = OFlag::from_bits_truncate(fcntl(&master, FcntlArg::F_GETFL).unwrap());
        fcntl(&master, FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK)).unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_work-engine-terminal-ui"))
            .arg("--connect")
            .arg(socket)
            .args(["--task", task, "--delivery", delivery])
            .env("TERM", "xterm-256color")
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()))
            .spawn()
            .unwrap();
        Self {
            child,
            master,
            slave,
            original,
            parser: vt100::Parser::new(60, 180, 0),
            raw: vec![],
            evidence: evidence.into(),
            read_deadline: Duration::from_secs(20),
        }
    }
    #[allow(dead_code)]
    pub fn launch_history(socket: &Path, evidence: &Path, rows: u16, cols: u16) -> Self {
        let pty = openpty(
            Some(&Winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            }),
            None,
        )
        .unwrap();
        let master = File::from(pty.master);
        let slave = File::from(pty.slave);
        let original = tcgetattr(&slave).unwrap();
        let flags = OFlag::from_bits_truncate(fcntl(&master, FcntlArg::F_GETFL).unwrap());
        fcntl(&master, FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK)).unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_work-engine-terminal-ui"))
            .arg("--history-connect")
            .arg(socket)
            .env("TERM", "xterm-256color")
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave.try_clone().unwrap()))
            .spawn()
            .unwrap();
        Self {
            child,
            master,
            slave,
            original,
            parser: vt100::Parser::new(rows, cols, 0),
            raw: vec![],
            evidence: evidence.into(),
            // Historical owner reads have a 30-second server deadline and a
            // 35-second client deadline before the first frame can appear.
            read_deadline: Duration::from_secs(40),
        }
    }
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
    pub fn input(&mut self, bytes: &[u8]) {
        self.master.write_all(bytes).unwrap();
    }
    pub fn screen(&self) -> String {
        self.parser.screen().contents()
    }
    pub fn wait_for(&mut self, text: &str, stage: &str) -> Duration {
        let start = Instant::now();
        loop {
            let mut bytes = [0; 65536];
            match self.master.read(&mut bytes) {
                Ok(0) => panic!("UI PTY closed at {stage}"),
                Ok(n) => {
                    assert!(self.raw.len() + n <= 8 * 1024 * 1024, "PTY output bound");
                    self.raw.extend_from_slice(&bytes[..n]);
                    self.parser.process(&bytes[..n]);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => panic!("PTY read at {stage}: {e}"),
            }
            let screen = self.screen();
            if screen.contains(text) {
                fs::write(self.evidence.join(format!("{stage}-screen.txt")), &screen).unwrap();
                return start.elapsed();
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "UI exited at {stage}"
            );
            let _ = fs::write(self.evidence.join("latest-screen.txt"), &screen);
            assert!(
                start.elapsed() < self.read_deadline,
                "UI deadline at {stage}: expected {text:?}\n{screen}"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
    pub fn close(&mut self) {
        self.close_with(b"\x03");
    }
    #[allow(dead_code)]
    pub fn close_history(&mut self) {
        self.close_with(b"q");
    }
    fn close_with(&mut self, key: &[u8]) {
        self.input(key);
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "UI exit: {status}");
                assert_eq!(
                    tcgetattr(&self.slave).unwrap(),
                    self.original,
                    "terminal restoration"
                );
                return;
            }
            assert!(Instant::now() < deadline, "UI exit deadline");
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = fs::write(self.evidence.join("continuous-ui.raw"), &self.raw);
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
