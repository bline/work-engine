use std::{
    io::{Read, Write},
    os::{fd::OwnedFd, unix::net::UnixStream},
    path::PathBuf,
    time::{Duration, Instant},
};

use lifecycle_wire::{CursorV1, LifecycleSnapshotV1, WaitTargetV1, parse_snapshot};
use serde_json::{Value, json};
use socket2::{Domain, SockAddr, Socket, Type};

use crate::{ClientError, ObservationTransport, SubjectRef, WaitHintV1};

const MAX_FRAME: usize = 131_072;

/// One bounded passive request per connection; effectful commands are deliberately absent.
pub struct UnixSocketTransport {
    path: PathBuf,
}

impl UnixSocketTransport {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn exchange(&self, request: Value, budget: Duration) -> Result<Vec<u8>, ClientError> {
        if budget.is_zero() {
            return Err(ClientError::DeadlineExpired);
        }
        let started = Instant::now();
        let socket = Socket::new(Domain::UNIX, Type::STREAM, None)
            .map_err(|_| ClientError::ObservationUnavailable)?;
        let address =
            SockAddr::unix(&self.path).map_err(|_| ClientError::ObservationUnavailable)?;
        let connect_budget = budget.saturating_sub(started.elapsed());
        if connect_budget.is_zero() {
            return Err(ClientError::DeadlineExpired);
        }
        socket
            .connect_timeout(&address, connect_budget)
            .map_err(|error| {
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) {
                    ClientError::DeadlineExpired
                } else {
                    ClientError::ObservationUnavailable
                }
            })?;
        let fd: OwnedFd = socket.into();
        let mut stream: UnixStream = fd.into();
        let remaining = budget.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return Err(ClientError::DeadlineExpired);
        }
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|_| ClientError::ObservationUnavailable)?;
        stream
            .set_write_timeout(Some(remaining))
            .map_err(|_| ClientError::ObservationUnavailable)?;
        let bytes = serde_json::to_vec(&request).map_err(|_| ClientError::InvalidObservation)?;
        if bytes.len() > MAX_FRAME {
            return Err(ClientError::InvalidObservation);
        }
        stream
            .write_all(&bytes)
            .map_err(|_| ClientError::ObservationUnavailable)?;
        stream
            .write_all(b"\n")
            .map_err(|_| ClientError::ObservationUnavailable)?;
        stream
            .flush()
            .map_err(|_| ClientError::ObservationUnavailable)?;
        let mut frame = Vec::new();
        loop {
            let remaining = budget.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err(ClientError::DeadlineExpired);
            }
            stream
                .set_read_timeout(Some(remaining))
                .map_err(|_| ClientError::ObservationUnavailable)?;
            let mut byte = [0u8; 1];
            stream.read_exact(&mut byte).map_err(|error| {
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) {
                    ClientError::DeadlineExpired
                } else {
                    ClientError::ObservationUnavailable
                }
            })?;
            if byte[0] == b'\n' {
                break;
            }
            if frame.len() == MAX_FRAME {
                return Err(ClientError::InvalidObservation);
            }
            frame.push(byte[0]);
        }
        if started.elapsed() > budget {
            return Err(ClientError::DeadlineExpired);
        }
        Ok(frame)
    }
}

impl ObservationTransport for UnixSocketTransport {
    fn snapshot(
        &self,
        subject: &SubjectRef,
        target: Option<&WaitTargetV1>,
        budget: Duration,
    ) -> Result<LifecycleSnapshotV1, ClientError> {
        let (delivery, transition) = match target {
            Some(WaitTargetV1::DeliveryOutcome { delivery_id, .. }) => {
                (Some(delivery_id.clone()), None)
            }
            Some(WaitTargetV1::TransitionStage { transition_id, .. }) => {
                (None, Some(transition_id.clone()))
            }
            None => (None, None),
        };
        let bytes = self.exchange(json!({"op":"snapshot","subject_id":subject.as_str(),"delivery_id":delivery,"transition_id":transition}),budget)?;
        parse_snapshot(&bytes, subject.as_str()).map_err(ClientError::from)
    }

    fn wait_hint(
        &self,
        subject: &SubjectRef,
        target: &WaitTargetV1,
        cursor: &CursorV1,
        budget: Duration,
    ) -> Result<WaitHintV1, ClientError> {
        let started = Instant::now();
        loop {
            let remaining = budget.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Ok(WaitHintV1::NoChange);
            }
            let millis = u64::try_from(remaining.as_millis())
                .unwrap_or(u64::MAX)
                .max(1);
            let bytes = self.exchange(json!({"op":"wait","subject_id":subject.as_str(),"target":target,"cursor":cursor,"max_wait_ms":millis.to_string()}),remaining)?;
            let value: Value =
                serde_json::from_slice(&bytes).map_err(|_| ClientError::InvalidObservation)?;
            match value.get("hint").and_then(Value::as_str) {
                Some("changed") => return Ok(WaitHintV1::Changed),
                Some("no_change") if started.elapsed() < budget => continue,
                Some("no_change") => return Ok(WaitHintV1::NoChange),
                Some("cursor_gap") => return Ok(WaitHintV1::CursorGap),
                _ => return Err(ClientError::ObservationUnavailable),
            }
        }
    }
}
