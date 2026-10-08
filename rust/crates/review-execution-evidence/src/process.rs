#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    SpawnFailure,
    Timeout,
    StdoutOverflow,
    StderrOverflow,
    StdinPipe,
    StdoutPipe,
    StderrPipe,
    WaitFailure,
    NonzeroExit,
    ExitSignal,
    MalformedResult,
    ResultCapacity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupObservation {
    NotNeeded,
    Reaped,
    KillFailedButReaped,
    KillFailedAndUnreaped,
    ReapFailed,
    ReapTimedOut,
    Unavailable,
}

/// Local process facts only; no provider non-entry or retry permission follows.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedFailure {
    pub schema_version: u32,
    pub kind: FailureKind,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub exit_signal: Option<i32>,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub cleanup: CleanupObservation,
}

impl ObservedFailure {
    pub(crate) fn local(
        kind: FailureKind,
        pid: Option<u32>,
        stdout_bytes: usize,
        stderr_bytes: usize,
        status: Option<std::process::ExitStatus>,
        cleanup: CleanupObservation,
    ) -> Self {
        Self {
            schema_version: 1,
            kind,
            pid,
            exit_code: status.as_ref().and_then(std::process::ExitStatus::code),
            exit_signal: status.as_ref().and_then(std::process::ExitStatus::signal),
            stdout_bytes,
            stderr_bytes,
            cleanup,
        }
    }
}

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

    pub async fn capture(mut self, input: &[u8]) -> Result<CapturedChild, ObservedFailure> {
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
        let mut stdout = self.stdout.take((STREAM_MAX_BYTES + 1) as u64);
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
        let (write, read_out, read_err, waited) =
            match tokio::time::timeout(Duration::from_secs(5), work).await {
                Ok(values) => values,
                Err(_) => {
                    let kind = if out.len() > STREAM_MAX_BYTES {
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
                        match tokio::time::timeout(Duration::from_secs(2), self.child.wait()).await
                        {
                            Ok(Ok(status)) if kill_ok => (Some(status), CleanupObservation::Reaped),
                            Ok(Ok(status)) => {
                                (Some(status), CleanupObservation::KillFailedButReaped)
                            }
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
        if out.len() > STREAM_MAX_BYTES || err.len() > STREAM_MAX_BYTES {
            let kind = if out.len() > STREAM_MAX_BYTES {
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
