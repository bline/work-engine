use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

use crate::producer::{ControlledScope, LaunchTicket};
use crate::{EvidenceReadError, STREAM_MAX_BYTES};

pub(crate) struct RunningChild {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
}

pub(crate) struct CapturedChild {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
    pub pid: u32,
}

pub use review_execution_custody::{CleanupObservation, FailureKind, ObservedFailure};

pub(crate) fn spawn(
    scope: &ControlledScope,
    ticket: LaunchTicket,
) -> Result<RunningChild, EvidenceReadError> {
    if !ticket.matches(scope) {
        return Err(EvidenceReadError::Conflicting(
            "launch ticket attempt mismatch",
        ));
    }
    let mut command = Command::new(&scope.executable);
    command
        .args(&scope.arguments)
        .current_dir(&scope.workspace)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    spawn_command(command)
}

pub(crate) fn spawn_command(mut command: Command) -> Result<RunningChild, EvidenceReadError> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|_| EvidenceReadError::Unresolved("spawn failed after launch intent"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or(EvidenceReadError::Unresolved("child stdin unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or(EvidenceReadError::Unresolved("child stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or(EvidenceReadError::Unresolved("child stderr unavailable"))?;
    Ok(RunningChild {
        child,
        stdin,
        stdout,
        stderr,
    })
}

impl RunningChild {
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    /// Read exactly one trusted guard handshake line before releasing prompt
    /// bytes. No buffered read may consume later diagnostic bytes.
    pub async fn await_ready(&mut self, expected: &str) -> Result<(), ObservedFailure> {
        let pid = self.child.id();
        let read = async {
            let mut line = Vec::new();
            loop {
                let mut byte = [0u8; 1];
                self.stderr.read_exact(&mut byte).await?;
                line.push(byte[0]);
                if line.len() > 4096 {
                    return Err(std::io::Error::other("guard handshake oversized"));
                }
                if byte[0] == b'\n' {
                    return Ok(line);
                }
            }
        };
        let actual = tokio::time::timeout(Duration::from_secs(5), read).await;
        match actual {
            Ok(Ok(line)) if line == expected.as_bytes() => Ok(()),
            _ => {
                let kill_ok = self.child.start_kill().is_ok();
                let (status, cleanup) =
                    match tokio::time::timeout(Duration::from_secs(5), self.child.wait()).await {
                        Ok(Ok(status)) if kill_ok => (Some(status), CleanupObservation::Reaped),
                        Ok(Ok(status)) => (Some(status), CleanupObservation::KillFailedButReaped),
                        Ok(Err(_)) if kill_ok => (None, CleanupObservation::ReapFailed),
                        Ok(Err(_)) => (None, CleanupObservation::KillFailedAndUnreaped),
                        Err(_) => (None, CleanupObservation::ReapTimedOut),
                    };
                Err(ObservedFailure::local(
                    FailureKind::StderrPipe,
                    pid,
                    0,
                    0,
                    status,
                    cleanup,
                ))
            }
        }
    }

    pub async fn capture(self, input: &[u8]) -> Result<CapturedChild, ObservedFailure> {
        self.capture_with_horizon(input, Duration::from_secs(5), Duration::from_secs(2))
            .await
    }

    pub async fn capture_with_horizon(
        self,
        input: &[u8],
        horizon: Duration,
        cleanup_horizon: Duration,
    ) -> Result<CapturedChild, ObservedFailure> {
        self.capture_with_output_limit(input, horizon, cleanup_horizon, STREAM_MAX_BYTES)
            .await
    }

    pub async fn capture_guarded_with_horizon(
        self,
        input: &[u8],
        horizon: Duration,
        cleanup_horizon: Duration,
    ) -> Result<CapturedChild, ObservedFailure> {
        const GUARD_LIMIT: usize = 2 * 16 * 1024 * 1024 + STREAM_MAX_BYTES + 1024 * 1024;
        self.capture_with_output_limit(input, horizon, cleanup_horizon, GUARD_LIMIT)
            .await
    }

    async fn capture_with_output_limit(
        mut self,
        input: &[u8],
        horizon: Duration,
        cleanup_horizon: Duration,
        stdout_limit: usize,
    ) -> Result<CapturedChild, ObservedFailure> {
        let pid = self.child.id().ok_or_else(|| {
            ObservedFailure::local(
                FailureKind::WaitFailure,
                None,
                0,
                0,
                None,
                CleanupObservation::ReapFailed,
            )
        })?;
        let mut stdin = self.stdin;
        let mut stdout = self.stdout.take((stdout_limit + 1) as u64);
        let mut stderr = self.stderr.take((STREAM_MAX_BYTES + 1) as u64);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let work = async {
            let (write, read_out, read_err, waited) = tokio::join!(
                async {
                    let result = stdin.write_all(input).await;
                    drop(stdin);
                    result
                },
                stdout.read_to_end(&mut out),
                stderr.read_to_end(&mut err),
                self.child.wait(),
            );
            (write, read_out, read_err, waited)
        };
        let (write, read_out, read_err, waited) = match tokio::time::timeout(horizon, work).await {
            Ok(values) => values,
            Err(_) => {
                let kind = if out.len() > stdout_limit {
                    FailureKind::StdoutOverflow
                } else if err.len() > STREAM_MAX_BYTES {
                    FailureKind::StderrOverflow
                } else {
                    FailureKind::Timeout
                };
                let kill_ok = self.child.start_kill().is_ok();
                #[cfg(feature = "test-faults")]
                if force_cleanup_unavailable() {
                    return Err(ObservedFailure::local(
                        kind,
                        Some(pid),
                        out.len(),
                        err.len(),
                        None,
                        CleanupObservation::Unavailable,
                    ));
                }
                let (status, cleanup) =
                    match tokio::time::timeout(cleanup_horizon, self.child.wait()).await {
                        Ok(Ok(status)) if kill_ok => (Some(status), CleanupObservation::Reaped),
                        Ok(Ok(status)) => (Some(status), CleanupObservation::KillFailedButReaped),
                        Ok(Err(_)) if kill_ok => (None, CleanupObservation::ReapFailed),
                        Ok(Err(_)) => (None, CleanupObservation::KillFailedAndUnreaped),
                        Err(_) => (None, CleanupObservation::ReapTimedOut),
                    };
                return Err(ObservedFailure::local(
                    kind,
                    Some(pid),
                    out.len(),
                    err.len(),
                    status,
                    cleanup,
                ));
            }
        };
        let wait_succeeded = waited.is_ok();
        let status = waited.ok();
        let failure_kind = if write.is_err() {
            Some(FailureKind::StdinPipe)
        } else if read_out.is_err() {
            Some(FailureKind::StdoutPipe)
        } else if read_err.is_err() {
            Some(FailureKind::StderrPipe)
        } else if status.is_none() {
            Some(FailureKind::WaitFailure)
        } else {
            None
        };
        if let Some(kind) = failure_kind {
            return Err(ObservedFailure::local(
                kind,
                Some(pid),
                out.len(),
                err.len(),
                status,
                if wait_succeeded {
                    CleanupObservation::NotNeeded
                } else {
                    CleanupObservation::ReapFailed
                },
            ));
        }
        if out.len() > stdout_limit || err.len() > STREAM_MAX_BYTES {
            let kind = if out.len() > stdout_limit {
                FailureKind::StdoutOverflow
            } else {
                FailureKind::StderrOverflow
            };
            return Err(ObservedFailure::local(
                kind,
                Some(pid),
                out.len(),
                err.len(),
                status,
                CleanupObservation::NotNeeded,
            ));
        }
        let status = status.unwrap();
        let Some(code) = status.code() else {
            return Err(ObservedFailure::local(
                FailureKind::ExitSignal,
                Some(pid),
                out.len(),
                err.len(),
                Some(status),
                CleanupObservation::NotNeeded,
            ));
        };
        if code != 0 {
            return Err(ObservedFailure::local(
                FailureKind::NonzeroExit,
                Some(pid),
                out.len(),
                err.len(),
                Some(status),
                CleanupObservation::NotNeeded,
            ));
        }
        Ok(CapturedChild {
            stdout: out,
            stderr: err,
            exit_code: code,
            pid,
        })
    }
}

#[cfg(feature = "test-faults")]
fn force_cleanup_unavailable() -> bool {
    std::env::var("WORK_ENGINE_EVIDENCE_FORCE_CLEANUP_UNAVAILABLE")
        .ok()
        .as_deref()
        == Some("1")
}
