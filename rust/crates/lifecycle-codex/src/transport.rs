use std::{path::Path, time::Duration};

use lifecycle_runtime::{NativeChildExit, NativeStdio, NativeStdioError};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use thiserror::Error;
use tokio::process::Command;

use crate::{NATIVE_BINARY_SHA256, NativeEvent, NativeEventKind, NativeProtocolError, parse_event};

#[derive(Debug, Error)]
pub enum NativeHarnessError {
    #[error("installed native binary does not match the S5 pin")]
    BinaryMismatch,
    #[error("native transport failed: {0}")]
    Transport(#[from] NativeStdioError),
    #[error("native protocol failed: {0}")]
    Protocol(#[from] NativeProtocolError),
    #[error("native request serialization failed")]
    Serialization,
    #[error("native transport reached EOF")]
    Eof,
}

/// One native child and one protocol loop. Only trusted service composition
/// receives this type; it has no entry authority or store state of its own.
pub struct NativeHarness {
    pipe: NativeStdio,
    transport_session: String,
    next_rpc: u64,
}

impl NativeHarness {
    pub fn spawn(
        executable: &Path,
        work: &Path,
        home: &Path,
        codex_home: &Path,
        transport_session: String,
    ) -> Result<Self, NativeHarnessError> {
        let bytes = std::fs::read(executable).map_err(|_| NativeHarnessError::BinaryMismatch)?;
        if format!("{:x}", Sha256::digest(bytes)) != NATIVE_BINARY_SHA256
            || transport_session.is_empty()
        {
            return Err(NativeHarnessError::BinaryMismatch);
        }
        let mut command = Command::new(executable);
        let tmpdir = std::env::var_os("TMPDIR").ok_or(NativeHarnessError::BinaryMismatch)?;
        let target =
            std::env::var_os("CARGO_TARGET_DIR").ok_or(NativeHarnessError::BinaryMismatch)?;
        command
            .args(["app-server", "--strict-config", "--listen", "stdio://"])
            .env_clear()
            .env("HOME", home)
            .env("CODEX_HOME", codex_home)
            .env("PATH", "/usr/bin:/bin")
            .env("RUST_LOG", "error")
            .env("TMPDIR", tmpdir)
            .env("CARGO_TARGET_DIR", target)
            .current_dir(work);
        let pipe = NativeStdio::spawn(&mut command, 1_048_576, 4096)?;
        Ok(Self {
            pipe,
            transport_session,
            next_rpc: 1,
        })
    }

    pub fn pid(&self) -> Option<u32> {
        self.pipe.pid()
    }

    pub async fn send_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<u64, NativeHarnessError> {
        let id = self.next_rpc;
        self.next_rpc = self
            .next_rpc
            .checked_add(1)
            .ok_or(NativeHarnessError::Serialization)?;
        let raw = serde_json::to_vec(&json!({"id":id,"method":method,"params":params}))
            .map_err(|_| NativeHarnessError::Serialization)?;
        self.pipe.send(&raw).await?;
        Ok(id)
    }

    pub async fn send_notification(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(), NativeHarnessError> {
        let raw = serde_json::to_vec(&json!({"method":method,"params":params}))
            .map_err(|_| NativeHarnessError::Serialization)?;
        self.pipe.send(&raw).await?;
        Ok(())
    }

    pub async fn send_result(&mut self, id: u64, result: Value) -> Result<(), NativeHarnessError> {
        let raw = serde_json::to_vec(&json!({"id":id,"result":result}))
            .map_err(|_| NativeHarnessError::Serialization)?;
        self.pipe.send(&raw).await?;
        Ok(())
    }

    pub async fn deny_request(&mut self, id: u64) -> Result<(), NativeHarnessError> {
        let raw = serde_json::to_vec(
            &json!({"id":id,"error":{"code":-32601,"message":"unsupported native request"}}),
        )
        .map_err(|_| NativeHarnessError::Serialization)?;
        self.pipe.send(&raw).await?;
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<NativeEvent, NativeHarnessError> {
        let (sequence, raw) = self.pipe.receive().await?.ok_or(NativeHarnessError::Eof)?;
        Ok(
            match parse_event(self.transport_session.clone(), sequence, raw.clone()) {
                Ok(event) => event,
                Err(_) => NativeEvent {
                    transport_session: self.transport_session.clone(),
                    sequence,
                    sha256: format!("{:x}", Sha256::digest(&raw)),
                    raw,
                    value: Value::Null,
                    kind: NativeEventKind::Malformed,
                },
            },
        )
    }

    pub async fn terminate_and_reap(&mut self) -> Result<NativeChildExit, NativeHarnessError> {
        Ok(self.pipe.terminate_and_reap(Duration::from_secs(5)).await?)
    }
}
