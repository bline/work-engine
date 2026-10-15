use std::{
    io::{BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{ProofError, Result, identity::ProcessIdentity};

pub const MAX_HOST_FRAME: usize = 131_072;

/// Ordinary host socket transport. Command envelopes and authentication are
/// supplied by the caller from the accepted host protocol; this transport
/// never manufactures principal, grant, or task authority.
#[derive(Clone, Debug)]
pub struct HostClient {
    socket: PathBuf,
    timeout: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HostReportedIdentity {
    pub pid: u32,
    pub linux_starttime_ticks: String,
    pub incarnation: String,
    pub build_id: String,
    pub profile: String,
    pub executable_sha256: String,
    pub source_sha256: String,
    pub lock_sha256: String,
    pub toolchain_sha256: String,
    pub features: Vec<String>,
    pub provider_identity: String,
}

impl HostReportedIdentity {
    pub fn from_reply(reply: &Value) -> Result<Self> {
        Ok(serde_json::from_value(reply["host_identity"].clone())?)
    }

    pub fn bind_process(&self, observed: &ProcessIdentity, expected_build: &str) -> Result<()> {
        if self.pid != observed.pid
            || self.linux_starttime_ticks.parse::<u64>().ok()
                != Some(observed.linux_starttime_ticks)
            || self.incarnation.is_empty()
            || self.build_id != expected_build
            || self.profile != "retained-repository-analysis-v1"
            || self.executable_sha256 != observed.executable_sha256
        {
            return Err(ProofError::Contract(
                "host identity does not bind installed process".into(),
            ));
        }
        Ok(())
    }

    pub fn bind_startup(&self, startup: &Value) -> Result<()> {
        if startup["executable_sha256"].as_str() != Some(self.executable_sha256.as_str())
            || startup["source_sha256"].as_str() != Some(self.source_sha256.as_str())
            || startup["lock_sha256"].as_str() != Some(self.lock_sha256.as_str())
            || startup["toolchain_sha256"].as_str() != Some(self.toolchain_sha256.as_str())
            || serde_json::to_value(&self.features)? != startup["features"]
            || startup["provider_identity"].as_str() != Some(self.provider_identity.as_str())
        {
            return Err(ProofError::Contract(
                "host startup identity pin mismatch".into(),
            ));
        }
        Ok(())
    }
}

impl HostClient {
    pub fn new(socket: impl Into<PathBuf>, timeout: Duration) -> Result<Self> {
        let socket = socket.into();
        if !socket.is_absolute() || timeout.is_zero() || timeout > Duration::from_secs(30) {
            return Err(ProofError::Contract(
                "invalid host client configuration".into(),
            ));
        }
        Ok(Self { socket, timeout })
    }

    pub fn await_ready(&self, budget: Duration) -> Result<()> {
        let deadline = Instant::now() + budget;
        loop {
            if UnixStream::connect(&self.socket).is_ok() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(ProofError::Deadline("host socket readiness"));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn exchange(&self, command: &Value) -> Result<Value> {
        let mut bytes = serde_json::to_vec(command)?;
        if bytes.len() > MAX_HOST_FRAME - 1 {
            return Err(ProofError::Contract("host command too large".into()));
        }
        bytes.push(b'\n');
        let mut stream = UnixStream::connect(&self.socket)?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;
        stream.write_all(&bytes)?;
        stream.flush()?;
        let mut response = Vec::new();
        let reader = BufReader::new(stream);
        let read = reader
            .take(MAX_HOST_FRAME as u64 + 1)
            .read_until(b'\n', &mut response)?;
        if read == 0 || read > MAX_HOST_FRAME || response.last() != Some(&b'\n') {
            return Err(ProofError::Contract("invalid host response frame".into()));
        }
        Ok(serde_json::from_slice(&response[..response.len() - 1])?)
    }

    /// Sends one ordinary host command, then deliberately drops its reply.
    /// Callers may reconcile only through exact command-ID readback.
    pub fn send_without_reply(&self, command: &Value) -> Result<()> {
        let mut bytes = serde_json::to_vec(command)?;
        if bytes.len() > MAX_HOST_FRAME - 1 {
            return Err(ProofError::Contract("host command too large".into()));
        }
        bytes.push(b'\n');
        let mut stream = UnixStream::connect(&self.socket)?;
        stream.set_write_timeout(Some(self.timeout))?;
        stream.write_all(&bytes)?;
        stream.flush()?;
        Ok(())
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    pub fn submit(&self, payload: Value) -> Result<Value> {
        self.exchange(&json!({"op":"task.submit.v1","payload":payload}))
    }

    pub fn answer(&self, payload: Value) -> Result<Value> {
        self.exchange(&json!({"op":"task.answer.v1","payload":payload}))
    }

    pub fn observe(&self, task_id: &str, delivery_id: Option<&str>) -> Result<Value> {
        let mut request = json!({"op":"task.observe.v2","task_id":task_id});
        if let Some(delivery_id) = delivery_id {
            request["delivery_id"] = json!(delivery_id);
        }
        self.exchange(&request)
    }

    pub fn observe_command(&self, task_id: &str, command_id: &str) -> Result<Value> {
        self.exchange(&json!({
            "op":"task.observe.v2",
            "task_id":task_id,
            "command_id":command_id
        }))
    }

    pub fn request_replacement(&self, payload: Value) -> Result<Value> {
        self.exchange(&json!({"op":"task.request_replacement.v1","payload":payload}))
    }

    pub fn contract_grant(&self, payload: Value) -> Result<Value> {
        self.exchange(&json!({"op":"task.contract_grant.v1","payload":payload}))
    }
}
