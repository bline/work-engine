//! Passive local client for the bounded historical S5 read operation.
use historical_evidence_wire::{MAX_FRAME, Observation, Reply, Request};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
pub enum ReadError {
    InvalidRequest,
    Unavailable,
    Refused,
    InvalidResponse,
}

pub fn inspect(socket: &Path, request: Request) -> Result<Observation, ReadError> {
    request.validate().map_err(|_| ReadError::InvalidRequest)?;
    if !socket.is_absolute() {
        return Err(ReadError::InvalidRequest);
    }
    let mut stream = UnixStream::connect(socket).map_err(|_| ReadError::Unavailable)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(35)))
        .map_err(|_| ReadError::Unavailable)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(8)))
        .map_err(|_| ReadError::Unavailable)?;
    let bytes = serde_json::to_vec(&request).map_err(|_| ReadError::InvalidRequest)?;
    if bytes.len() > MAX_FRAME {
        return Err(ReadError::InvalidRequest);
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(|_| ReadError::Unavailable)?;
    stream
        .write_all(&bytes)
        .map_err(|_| ReadError::Unavailable)?;
    let mut len = [0; 4];
    stream
        .read_exact(&mut len)
        .map_err(|_| ReadError::Unavailable)?;
    let len = u32::from_be_bytes(len) as usize;
    if len == 0 || len > MAX_FRAME {
        return Err(ReadError::InvalidResponse);
    }
    let mut response = vec![0; len];
    stream
        .read_exact(&mut response)
        .map_err(|_| ReadError::Unavailable)?;
    match serde_json::from_slice(&response).map_err(|_| ReadError::InvalidResponse)? {
        Reply::Observed { observation } => {
            observation
                .validate()
                .map_err(|_| ReadError::InvalidResponse)?;
            Ok(observation)
        }
        Reply::Refused { .. } => Err(ReadError::Refused),
        Reply::Unavailable { .. } => Err(ReadError::Unavailable),
    }
}
