use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::{ProofError, Result};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub linux_starttime_ticks: u64,
    pub executable: PathBuf,
    pub executable_sha256: String,
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

/// Linux `/proc/<pid>/stat` field 22, parsed after the parenthesized command.
/// Reading this and `/proc/<pid>/exe` while the child is held avoids PID-only
/// identity assertions that survive process replacement.
pub fn observe_process(pid: u32) -> Result<ProcessIdentity> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let tail = stat
        .rsplit_once(") ")
        .ok_or_else(|| ProofError::Contract("malformed proc stat".into()))?
        .1;
    let linux_starttime_ticks = tail
        .split_whitespace()
        .nth(19)
        .ok_or_else(|| ProofError::Contract("proc starttime absent".into()))?
        .parse()
        .map_err(|_| ProofError::Contract("proc starttime invalid".into()))?;
    let executable = fs::read_link(format!("/proc/{pid}/exe"))?;
    let executable_sha256 = sha256_file(&executable)?;
    Ok(ProcessIdentity {
        pid,
        linux_starttime_ticks,
        executable,
        executable_sha256,
    })
}

/// Owns the installed host child from spawn through confirmed reap.
pub struct HostProcess {
    child: Child,
    identity: ProcessIdentity,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloseEvidence {
    pub pid: u32,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub reaped: bool,
}

impl HostProcess {
    pub fn launch(executable: &Path, startup: &Path) -> Result<Self> {
        Self::launch_route(executable, startup, "--startup")
    }

    pub fn launch_selected(executable: &Path, selection: &Path) -> Result<Self> {
        Self::launch_route(executable, selection, "--selected")
    }

    fn launch_route(executable: &Path, startup: &Path, route: &str) -> Result<Self> {
        if !executable.is_absolute() || !startup.is_absolute() || !startup.is_file() {
            return Err(ProofError::Contract(
                "installed executable and startup file must be absolute".into(),
            ));
        }
        let mut child = Command::new(executable)
            .args(["serve", route])
            .arg(startup)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()?;
        let identity = match observe_process(child.id()) {
            Ok(identity) => identity,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        Ok(Self { child, identity })
    }

    pub fn identity(&self) -> &ProcessIdentity {
        &self.identity
    }

    pub fn assert_same_execution(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_some() {
            return Err(ProofError::Contract("host exited".into()));
        }
        if observe_process(self.child.id())? != self.identity {
            return Err(ProofError::Contract("host identity changed".into()));
        }
        Ok(())
    }

    pub fn terminate_and_reap(mut self, budget: Duration) -> Result<CloseEvidence> {
        let pid = self.child.id();
        let status = if let Some(status) = self.child.try_wait()? {
            status
        } else {
            let signal = Command::new("/bin/kill")
                .args(["-TERM", &pid.to_string()])
                .status()?;
            if !signal.success() {
                return Err(ProofError::Contract("host SIGTERM failed".into()));
            }
            let deadline = Instant::now() + budget;
            loop {
                if let Some(status) = self.child.try_wait()? {
                    break status;
                }
                if Instant::now() >= deadline {
                    self.child.kill()?;
                    let status = self.child.wait()?;
                    return Err(ProofError::Contract(format!(
                        "host close timed out; killed and reaped pid {pid} with {status}"
                    )));
                }
                thread::sleep(Duration::from_millis(10));
            }
        };
        Ok(close_evidence(pid, status))
    }

    /// Signal a live installed host without consuming its owned child handle.
    /// This lets a controlled proof observe shutdown fences before reaping.
    pub fn signal_terminate(&mut self) -> Result<()> {
        self.assert_same_execution()?;
        let signal = Command::new("/bin/kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()?;
        if !signal.success() {
            return Err(ProofError::Contract("host SIGTERM failed".into()));
        }
        Ok(())
    }

    pub fn wait_after_terminate_and_reap(mut self, budget: Duration) -> Result<CloseEvidence> {
        let pid = self.child.id();
        let deadline = Instant::now() + budget;
        loop {
            if let Some(status) = self.child.try_wait()? {
                return Ok(close_evidence(pid, status));
            }
            if Instant::now() >= deadline {
                self.child.kill()?;
                let status = self.child.wait()?;
                return Err(ProofError::Contract(format!(
                    "host close timed out; killed and reaped pid {pid} with {status}"
                )));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn kill_and_reap(mut self) -> Result<CloseEvidence> {
        let pid = self.child.id();
        self.child.kill()?;
        let status = self.child.wait()?;
        Ok(close_evidence(pid, status))
    }
}

fn close_evidence(pid: u32, status: ExitStatus) -> CloseEvidence {
    use std::os::unix::process::ExitStatusExt;
    CloseEvidence {
        pid,
        exit_code: status.code(),
        signal: status.signal(),
        reaped: true,
    }
}

impl Drop for HostProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub fn await_path(path: &Path, budget: Duration) -> Result<()> {
    let deadline = Instant::now() + budget;
    while !path.exists() {
        if Instant::now() >= deadline {
            return Err(ProofError::Deadline("path readiness"));
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

pub fn require_regular_file(path: &Path) -> io::Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "regular file absent",
        ))
    }
}
