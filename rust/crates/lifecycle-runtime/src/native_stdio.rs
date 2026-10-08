//! A bounded, owned stdio lease for one native JSON-RPC child.
//!
//! `ProcessSupervisor::activate` speaks the controlled peer's GO protocol, so
//! it cannot be reused for app-server frames. This lease keeps the child and
//! both pipes together until an observed exit. Dropping it kills the child;
//! an entered provider turn remains uncertain in the store regardless of exit.

use std::{process::Stdio, time::Duration};

use thiserror::Error;
use tokio::process::{Child, Command};

use crate::{BoundedFrameReader, BoundedFrameWriter, FrameError};

#[derive(Debug, Error)]
pub enum NativeStdioError {
    #[error("native child pipe unavailable")]
    PipeUnavailable,
    #[error("native child frame failed: {0}")]
    Frame(#[from] FrameError),
    #[error("native child process failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("native child cleanup timed out")]
    CleanupTimeout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeChildExit {
    pub pid: Option<u32>,
    pub code: Option<i32>,
    pub success: bool,
    pub forced: bool,
}

pub struct NativeStdio {
    child: Child,
    reader: BoundedFrameReader<tokio::process::ChildStdout>,
    writer: BoundedFrameWriter<tokio::process::ChildStdin>,
    pid: Option<u32>,
    max_frames: u64,
    received: u64,
    sent: u64,
}

impl NativeStdio {
    /// Spawn a pinned command whose environment, work directory and profile
    /// were already checked by trusted composition. The child is registered
    /// before any request bytes can be sent.
    pub fn spawn(
        command: &mut Command,
        max_frame_bytes: usize,
        max_frames: u64,
    ) -> Result<Self, NativeStdioError> {
        if max_frames == 0 {
            return Err(NativeStdioError::PipeUnavailable);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let pid = child.id();
        let stdin = child
            .stdin
            .take()
            .ok_or(NativeStdioError::PipeUnavailable)?;
        let stdout = child
            .stdout
            .take()
            .ok_or(NativeStdioError::PipeUnavailable)?;
        Ok(Self {
            child,
            reader: BoundedFrameReader::new(stdout, max_frame_bytes)?,
            writer: BoundedFrameWriter::new(stdin, max_frame_bytes)?,
            pid,
            max_frames,
            received: 0,
            sent: 0,
        })
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub async fn send(&mut self, bytes: &[u8]) -> Result<u64, NativeStdioError> {
        self.sent = self
            .sent
            .checked_add(1)
            .ok_or(NativeStdioError::PipeUnavailable)?;
        if self.sent > self.max_frames {
            return Err(NativeStdioError::PipeUnavailable);
        }
        self.writer.write_frame(bytes).await?;
        Ok(self.sent)
    }

    pub async fn receive(&mut self) -> Result<Option<(u64, Vec<u8>)>, NativeStdioError> {
        let Some(frame) = self.reader.read_frame().await? else {
            return Ok(None);
        };
        self.received = self
            .received
            .checked_add(1)
            .ok_or(NativeStdioError::PipeUnavailable)?;
        if self.received > self.max_frames {
            return Err(NativeStdioError::PipeUnavailable);
        }
        Ok(Some((self.received, frame)))
    }

    pub async fn terminate_and_reap(
        &mut self,
        cleanup_budget: Duration,
    ) -> Result<NativeChildExit, NativeStdioError> {
        let mut forced = false;
        if self.child.try_wait()?.is_none() {
            self.child.start_kill()?;
            forced = true;
        }
        let status = tokio::time::timeout(cleanup_budget, self.child.wait())
            .await
            .map_err(|_| NativeStdioError::CleanupTimeout)??;
        Ok(NativeChildExit {
            pid: self.pid,
            code: status.code(),
            success: status.success(),
            forced,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bounded_native_pipe_echo_and_exact_reap() {
        let mut command = Command::new("/usr/bin/cat");
        command.env_clear();
        let mut pipe = NativeStdio::spawn(&mut command, 128, 4).unwrap();
        let pid = pipe.pid();
        pipe.send(br#"{"id":1}"#).await.unwrap();
        let (coordinate, raw) = pipe.receive().await.unwrap().unwrap();
        assert_eq!(coordinate, 1);
        assert_eq!(raw, br#"{"id":1}"#);
        assert!(matches!(
            pipe.send(&[b'a'; 129]).await,
            Err(NativeStdioError::Frame(FrameError::TooLarge))
        ));
        let exit = pipe
            .terminate_and_reap(Duration::from_secs(2))
            .await
            .unwrap();
        assert_eq!(exit.pid, pid);
        assert!(exit.forced);
    }

    #[tokio::test]
    async fn native_eof_is_observed_before_reap() {
        let mut command = Command::new("/usr/bin/true");
        command.env_clear();
        let mut pipe = NativeStdio::spawn(&mut command, 128, 4).unwrap();
        assert!(pipe.receive().await.unwrap().is_none());
        let exit = pipe
            .terminate_and_reap(Duration::from_secs(2))
            .await
            .unwrap();
        assert!(exit.success);
        assert!(!exit.forced);
    }
}
