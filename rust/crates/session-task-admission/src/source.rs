//! Fixed-configuration source adapters. The enclosing deployment must protect
//! binaries, dependency paths, configuration and source files against races.
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
/// Advance the caller's current UTC action time through blocking acquisition.
/// This measures elapsed work in the same time domain, including injected test
/// times; it does not relax evidence freshness or derive time from a receipt.
pub(crate) struct AcquisitionTime {
    at: time::OffsetDateTime,
    started: Instant,
}
impl AcquisitionTime {
    pub(crate) fn new(at: time::OffsetDateTime) -> Self {
        Self {
            at,
            started: Instant::now(),
        }
    }
    #[cfg(test)]
    pub(crate) fn advance(&mut self, duration: time::Duration) {
        self.at += duration;
    }
    pub(crate) fn now(&self) -> time::OffsetDateTime {
        self.at + time::Duration::try_from(self.started.elapsed()).expect("acquisition duration")
    }
}
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct Pin {
    pub path: PathBuf,
    pub sha256: String,
}
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct Program {
    pub python: Pin,
    pub script: Pin,
    pub dependencies: Vec<Pin>,
}
pub(crate) fn pinned(p: &Pin) -> bool {
    if !p.path.is_absolute() || p.sha256.len() != 64 {
        return false;
    }
    let Ok(meta) = std::fs::metadata(&p.path) else {
        return false;
    };
    if !meta.is_file() || meta.len() > 128 * 1024 * 1024 {
        return false;
    }
    let Ok(bytes) = std::fs::read(&p.path) else {
        return false;
    };
    bytes.len() <= 128 * 1024 * 1024 && format!("{:x}", Sha256::digest(bytes)) == p.sha256
}
fn run(program: &Program, args: &[&std::ffi::OsStr]) -> Option<Vec<u8>> {
    run_bounded(program, args, Duration::from_secs(10))
}
pub(crate) fn run_bounded(
    program: &Program,
    args: &[&std::ffi::OsStr],
    duration: Duration,
) -> Option<Vec<u8>> {
    use std::os::{fd::AsRawFd, unix::process::CommandExt};
    if !pinned(&program.python)
        || !pinned(&program.script)
        || !program.dependencies.iter().all(pinned)
    {
        return None;
    }
    let mut child = Command::new(&program.python.path)
        .arg("-I")
        .arg(&program.script.path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .ok()?;
    let pid = child.id() as i32;
    let Some(mut stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    };
    let fd = stdout.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    }
    let deadline = Instant::now() + duration;
    let mut data = Vec::new();
    let mut eof = false;
    let mut status = None;
    loop {
        let mut buf = [0u8; 8192];
        match stdout.read(&mut buf) {
            Ok(0) => eof = true,
            Ok(n) => {
                data.extend_from_slice(&buf[..n]);
                if data.len() > 8 * 1024 * 1024 {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(s) => status = s,
                Err(_) => break,
            }
        }
        if eof && status.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    if !eof || status.is_none() || data.len() > 8 * 1024 * 1024 {
        // Only the process group started above is targeted; no system-wide kill.
        unsafe { libc::kill(-pid, libc::SIGKILL) };
        let _ = child.wait();
        return None;
    }
    if !status?.success()
        || !pinned(&program.python)
        || !pinned(&program.script)
        || !program.dependencies.iter().all(pinned)
    {
        return None;
    }
    Some(data)
}
/// Repository and helper pins come from protected startup configuration,
/// never from MCP parameters. Source acceptance additionally requires an
/// externally qualified storage/process boundary and action-time freshness.
#[allow(dead_code)]
pub(crate) fn board(program: &Program, repository: &Path) -> Option<crate::board::CompleteBoard> {
    if !repository.is_absolute() || !repository.is_dir() {
        return None;
    }
    let raw = run(
        program,
        &[std::ffi::OsStr::new("--repository"), repository.as_os_str()],
    )?;
    crate::board::parse_transfer(&raw)
}
/// Reads an exact protected scope-budget record via the canonical pinned
/// `usage_ledger.py control` interface. Does not establish exclusive control.
#[allow(dead_code)]
pub(crate) fn budget(
    program: &Program,
    record: &Pin,
    objective: &str,
    maximum: &str,
    productive: &str,
    reserve: &str,
    now: time::OffsetDateTime,
) -> Option<crate::accounting::Observation> {
    let clock = AcquisitionTime::new(now);
    if !pinned(record) {
        return None;
    }
    let bytes = run(
        program,
        &[
            std::ffi::OsStr::new("control"),
            std::ffi::OsStr::new("--record"),
            record.path.as_os_str(),
        ],
    )?;
    if !pinned(record) {
        return None;
    }
    crate::accounting::observe(
        std::str::from_utf8(&bytes).ok()?,
        objective,
        maximum,
        productive,
        reserve,
        clock.now(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acquisition_accepts_new_snapshot_but_rejects_future_and_stale() {
        let dir = std::env::temp_dir().join(format!("budget-clock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pin = |path: PathBuf| Pin {
            sha256: format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
            path,
        };
        let record = dir.join("record");
        std::fs::write(&record, "{}").unwrap();
        let record = pin(record);
        let data = serde_json::json!({"schema":"objective-budget-control-v1","topic":"control:budget:o","closed_at":null,"snapshot":{"snapshot_at":"","status":"complete","totals":{"codex":{"quota_usd_low":1,"quota_usd_high":1,"unpriced_calls":0},"claude_all_models":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0},"claude_fable":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0}},"cost_controls":[{"pool":"codex","node_id":null,"max_usd":5,"basis":"speed_adjusted_api_usd","remaining_usd":4,"observed":{"quota_usd_low":1,"quota_usd_high":1,"unpriced_calls":0,"cost_basis":"speed_adjusted_api_usd"}}]}});
        for (offset, accepted) in [(0, true), (60, false), (-60, false)] {
            let script = dir.join("ledger.py");
            std::fs::write(&script, format!("import json,datetime,time\ntime.sleep(0.1)\nd=json.loads({:?})\nd['snapshot']['snapshot_at']=(datetime.datetime.now(datetime.timezone.utc)+datetime.timedelta(seconds={})).isoformat()\nprint(json.dumps(d))\n",data.to_string(),offset)).unwrap();
            let program = Program {
                python: pin(std::fs::canonicalize("/usr/bin/python3").unwrap()),
                script: pin(script),
                dependencies: vec![],
            };
            let start = time::OffsetDateTime::now_utc();
            let observation = budget(&program, &record, "o", "5", "4", "1", start);
            assert_eq!(observation.is_some(), accepted, "offset {offset}");
            if let Some(observation) = observation {
                assert!(observation.at > start);
            }
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn pinned_board_and_budget_sources_reject_changed_code_or_record() {
        let dir = std::env::temp_dir().join(format!("task-source-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        let python = std::fs::canonicalize("/usr/bin/python3").unwrap();
        let pin = |p: PathBuf| Pin {
            sha256: format!("{:x}", Sha256::digest(std::fs::read(&p).unwrap())),
            path: p,
        };
        let script = dir.join("source.py");
        std::fs::write(&script,"import json\nprint(json.dumps({'revision':'a'*40,'next_sequence':2,'rows':['{\\\"sequence\\\":1}']}))\n").unwrap();
        let mut program = Program {
            python: pin(python),
            script: pin(script.clone()),
            dependencies: vec![],
        };
        assert!(board(&program, &dir).is_some());
        std::fs::write(&script, "print('changed')\n").unwrap();
        assert!(board(&program, &dir).is_none());
        let data = serde_json::json!({"schema":"objective-budget-control-v1","topic":"control:budget:o","closed_at":null,"snapshot":{"snapshot_at":"2026-11-02T00:00:00Z","status":"complete","totals":{"codex":{"quota_usd_low":1,"quota_usd_high":1,"unpriced_calls":0},"claude_all_models":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0},"claude_fable":{"quota_usd_low":0,"quota_usd_high":0,"unpriced_calls":0}},"cost_controls":[{"pool":"codex","node_id":null,"max_usd":5,"basis":"speed_adjusted_api_usd","remaining_usd":4,"observed":{"quota_usd_low":1,"quota_usd_high":1,"unpriced_calls":0,"cost_basis":"speed_adjusted_api_usd"}}]}});
        std::fs::write(&script, format!("print({:?})\n", data.to_string())).unwrap();
        program.script = pin(script);
        let record = dir.join("record.json");
        std::fs::write(&record, "{}").unwrap();
        let record_pin = pin(record.clone());
        let now = time::macros::datetime!(2026-11-02 00:00:10 UTC);
        assert!(budget(&program, &record_pin, "o", "5", "4", "1", now).is_some());
        std::fs::write(record, "changed").unwrap();
        assert!(budget(&program, &record_pin, "o", "5", "4", "1", now).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
