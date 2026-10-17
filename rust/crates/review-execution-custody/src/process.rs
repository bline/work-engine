use serde::{Deserialize, Serialize};
use std::os::unix::process::ExitStatusExt;

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
    pub fn local(
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
