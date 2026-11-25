//! One private read surface: containerd v2.3.3 Tasks.Get / Tasks.Wait in
//! `moby`. No caller-selected method, Exec ID, retry, proxy or TCP route.
//! The caller must retain the exact original task and exclude replacement;
//! Wait alone does not identify a PID or close the runtime frontier.

use super::{docker::Endpoint, full_hex, invalid, within};
use crate::{Result, refuse};
use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::MetadataExt,
    path::Path,
    process::{Child, Command, Stdio},
    time::Instant,
};

const SOCKET: &str = "/run/containerd/containerd.sock";
const CURL: &str = "/usr/bin/curl";
const MAX: usize = 16 * 1024;

#[derive(Clone, Copy)]
enum Method {
    Get,
    Wait,
}

pub(super) struct Tasks<'a> {
    /// Resolved by the protected installation binding, not by the workload.
    endpoint: &'a Endpoint,
    curl_digest: &'a str,
}

impl<'a> Tasks<'a> {
    pub(super) fn pinned(endpoint: &'a Endpoint, curl_digest: &'a str) -> Result<Self> {
        if endpoint.path != Path::new(SOCKET)
            || endpoint.listener_uid != 0
            || endpoint.listener_gid != 0
            || endpoint.listener_pid <= 1
            || !full_hex(curl_digest)
        {
            return refuse("containerd reader endpoint or artifact not pinned");
        }
        Ok(Self {
            endpoint,
            curl_digest,
        })
    }

    fn begin(&self, method: Method, id: &str, deadline: Instant) -> Result<ReadExchange<'a>> {
        if !full_hex(id) {
            return refuse("containerd reader requires one exact full ID");
        }
        // The caller retains the original serving process through the
        // entire exchange; curl opens the same root-controlled socket path.
        // This is not per-fragment authentication or a no-other-ingress claim.
        check_socket(self.endpoint)?;
        let peer = self.endpoint.connect()?;
        let m = fs::metadata(CURL)?;
        if !m.is_file()
            || m.uid() != 0
            || m.mode() & 0o022 != 0
            || m.len() > 4 * 1024 * 1024
            || std::fs::canonicalize(CURL)? != Path::new(CURL)
            || crate::digest(&std::fs::read(CURL)?) != self.curl_digest
        {
            return refuse("containerd reader curl artifact changed");
        }
        let left = deadline.saturating_duration_since(Instant::now());
        within(deadline)?;
        let millis = left.as_millis().min(60_000);
        if millis == 0 {
            return refuse("containerd read deadline exhausted");
        }
        let timeout = format!("{}.{:03}", millis / 1000, millis % 1000);
        let grpc_timeout = format!("grpc-timeout: {millis}m");
        let url = match method {
            Method::Get => "http://localhost/containerd.services.tasks.v1.Tasks/Get",
            Method::Wait => "http://localhost/containerd.services.tasks.v1.Tasks/Wait",
        };
        let mut child = ReadExchange {
            child: Command::new(CURL)
                .env_clear()
                .args([
                    "-q",
                    "--silent",
                    "--http2-prior-knowledge",
                    "--noproxy",
                    "*",
                    "--unix-socket",
                    SOCKET,
                    "--include",
                    "--max-time",
                    &timeout,
                    "--max-filesize",
                    "16384",
                    "--header",
                    "content-type: application/grpc",
                    "--header",
                    "containerd-namespace: moby",
                    "--header",
                    "te: trailers",
                    "--header",
                    &grpc_timeout,
                    "--data-binary",
                    "@-",
                    url,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?,
            endpoint: self.endpoint,
            deadline,
        };
        drop(peer);
        // gRPC frame: uncompressed; protobuf field 1 is the full container
        // ID, field 2 (exec_id) deliberately omitted for init.
        let mut request = [0u8; 71];
        request[4..7].copy_from_slice(&[66, 10, 64]);
        request[7..].copy_from_slice(id.as_bytes());
        child
            .child
            .stdin
            .take()
            .ok_or_else(|| invalid("missing observer stdin"))?
            .write_all(&request)?;
        Ok(child)
    }

    pub(super) fn running(&self, id: &str, deadline: Instant) -> Result<ObservedRunning> {
        let wire = self.begin(Method::Get, id, deadline)?.receive()?;
        let view = running_reply(grpc(&wire)?, id)?;
        Ok(ObservedRunning { view, wire })
    }

    pub(super) fn arm_wait(&self, id: &str, deadline: Instant) -> Result<PendingWait<'a>> {
        Ok(PendingWait(self.begin(Method::Wait, id, deadline)?))
    }
}

struct ReadExchange<'a> {
    child: Child,
    endpoint: &'a Endpoint,
    deadline: Instant,
}

/// Exact stock filesystem endpoint/ancestry. This verifies access via this
/// socket only, not listener delegation or other administrative entrances.
pub(super) fn check_socket(endpoint: &Endpoint) -> Result<()> {
    endpoint.check_path()?;
    let m = fs::symlink_metadata(&endpoint.path)?;
    if (m.dev(), m.ino(), m.uid(), m.gid(), m.mode() & 0o7777)
        != (endpoint.device, endpoint.inode, 0, 0, 0o660)
    {
        return refuse("containerd filesystem socket protection changed");
    }
    for parent in endpoint.path.ancestors().skip(1) {
        let m = fs::symlink_metadata(parent)?;
        if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
            return refuse("containerd socket ancestry is not root-controlled");
        }
    }
    Ok(())
}

impl ReadExchange<'_> {
    fn receive(mut self) -> Result<Vec<u8>> {
        let mut raw = Vec::new();
        self.child
            .stdout
            .take()
            .ok_or_else(|| invalid("missing observer stdout"))?
            .take((MAX + 1) as u64)
            .read_to_end(&mut raw)?;
        if raw.len() > MAX || !self.child.wait()?.success() {
            return refuse("containerd bounded read failed or was incomplete");
        }
        check_socket(self.endpoint)?;
        within(self.deadline)?;
        Ok(raw)
    }
}

impl Drop for ReadExchange<'_> {
    fn drop(&mut self) {
        // Only this local curl subprocess, never a task PID or remote signal.
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub(super) struct PendingWait<'a>(ReadExchange<'a>);

impl PendingWait<'_> {
    pub(super) fn receive(self) -> Result<ObservedExit> {
        let wire = self.0.receive()?;
        let (status, seconds, nanos) = wait_reply(grpc(&wire)?)?;
        Ok(ObservedExit {
            status,
            seconds,
            nanos,
            wire,
        })
    }
}

#[derive(Serialize)]
pub(super) struct ObservedExit {
    pub status: u32,
    pub seconds: u64,
    pub nanos: u32,
    pub(super) wire: Vec<u8>,
}

#[derive(Serialize)]
pub(super) struct ObservedRunning {
    pub view: Running,
    wire: Vec<u8>,
}

#[derive(Serialize, PartialEq, Debug)]
pub(super) struct Running {
    pub pid: u32,
    pub stdout: String,
    pub stderr: String,
}

fn malformed<T>() -> Result<T> {
    refuse("unexpected protected containerd read response")
}

fn grpc(wire: &[u8]) -> Result<&[u8]> {
    let Some(i) = wire.windows(4).position(|w| w == b"\r\n\r\n") else {
        return malformed();
    };
    let header = std::str::from_utf8(&wire[..i]).map_err(invalid)?;
    let mut rows = header.split("\r\n");
    if rows.next().map(str::trim_end) != Some("HTTP/2 200")
        || rows
            .clone()
            .filter(|r| r.starts_with("content-type:"))
            .collect::<Vec<_>>()
            != ["content-type: application/grpc"]
        || rows.any(|r| r.starts_with("grpc-status:") || r.starts_with("content-encoding:"))
    {
        return malformed();
    }
    let frame = &wire[i + 4..];
    let Some([0, a, b, c, d]) = frame.get(..5) else {
        return malformed();
    };
    let end = 5 + u32::from_be_bytes([*a, *b, *c, *d]) as usize;
    // Retain status-only success. containerd 2.3.3's grpc-go also sends
    // grpc-message for OK(""), printed by curl as `grpc-message: <CRLF>`.
    if end > MAX
        || !matches!(
            frame.get(end..),
            Some(b"grpc-status: 0\r\n" | b"grpc-status: 0\r\ngrpc-message: \r\n")
        )
    {
        return malformed();
    }
    Ok(&frame[5..end])
}

#[derive(Clone, Copy)]
enum Field<'a> {
    Number(u64),
    Bytes(&'a [u8]),
}

fn varint(s: &mut &[u8]) -> Result<u64> {
    let mut v = 0;
    for shift in (0..64).step_by(7) {
        let Some((&b, rest)) = s.split_first() else {
            return malformed();
        };
        *s = rest;
        if shift == 63 && b > 1 {
            return malformed();
        }
        v |= u64::from(b & 127) << shift;
        if b < 128 {
            return Ok(v);
        }
    }
    malformed()
}

fn fields<const N: usize>(mut s: &[u8]) -> Result<[Option<Field<'_>>; N]> {
    let mut result = [None; N];
    while !s.is_empty() {
        let key = varint(&mut s)?;
        let n = usize::try_from(key >> 3).map_err(invalid)?;
        if n == 0 || n > N || result[n - 1].is_some() {
            return malformed();
        }
        let value = match key & 7 {
            0 => Field::Number(varint(&mut s)?),
            2 => {
                let count = usize::try_from(varint(&mut s)?).map_err(invalid)?;
                let Some((value, tail)) = s.split_at_checked(count) else {
                    return malformed();
                };
                s = tail;
                Field::Bytes(value)
            }
            _ => return malformed(),
        };
        result[n - 1] = Some(value);
    }
    Ok(result)
}

fn running_reply(proto: &[u8], id: &str) -> Result<Running> {
    let [Some(Field::Bytes(inner))] = fields::<1>(proto)? else {
        return malformed();
    };
    // v2.3.3 local.getProcessState sets ID, not ContainerID, for Get.
    // Proto3 zero/default values can be omitted. Go's zero timestamp, if
    // encoded for a running process, is not used as exit evidence here.
    let [
        None,
        Some(Field::Bytes(found)),
        Some(Field::Number(pid)),
        Some(Field::Number(2)),
        None | Some(Field::Bytes(b"")),
        Some(Field::Bytes(out)),
        Some(Field::Bytes(err)),
        None | Some(Field::Number(0)),
        None | Some(Field::Number(0)),
        None | Some(Field::Bytes(_)),
    ] = fields::<10>(inner)?
    else {
        return malformed();
    };
    if found != id.as_bytes() || pid == 0 || out.is_empty() || err.is_empty() || out == err {
        return malformed();
    }
    Ok(Running {
        pid: u32::try_from(pid).map_err(invalid)?,
        stdout: std::str::from_utf8(out).map_err(invalid)?.into(),
        stderr: std::str::from_utf8(err).map_err(invalid)?.into(),
    })
}

fn wait_reply(proto: &[u8]) -> Result<(u32, u64, u32)> {
    let [status, Some(Field::Bytes(ts))] = fields::<2>(proto)? else {
        return malformed();
    };
    let status = match status {
        None => 0,
        Some(Field::Number(n)) => u32::try_from(n).map_err(invalid)?,
        _ => return malformed(),
    };
    let [Some(Field::Number(seconds @ 1..=253_402_300_799)), nanos] = fields::<2>(ts)? else {
        return malformed();
    };
    let nanos = match nanos {
        None => 0,
        Some(Field::Number(n @ 0..=999_999_999)) => n as u32,
        _ => return malformed(),
    };
    Ok((status, seconds, nanos))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &[u8] = b"HTTP/2 200 \r\ncontent-type: application/grpc\r\n\r\n";
    const STATUS: &[u8] = b"grpc-status: 0\r\n";
    const STOCK: &[u8] = b"grpc-status: 0\r\ngrpc-message: \r\n";

    fn wire(proto: &[u8], tail: &[u8]) -> Vec<u8> {
        [HEAD, &[0], &(proto.len() as u32).to_be_bytes(), proto, tail].concat()
    }

    #[test]
    fn natural_wait_requires_complete_success_and_runtime_exit_time() {
        let proto = [0x12, 0x02, 0x08, 0x25]; // proto3 exit status zero; nonzero timestamp
        for tail in [STATUS, STOCK] {
            assert_eq!(wait_reply(grpc(&wire(&proto, tail)).unwrap()).unwrap(), (0, 37, 0));
        }
        assert_eq!(
            wait_reply(&[0x08, 137, 1, 0x12, 2, 8, 37]).unwrap(),
            (137, 37, 0)
        );
        assert!(wait_reply(&[]).is_err());
        assert!(wait_reply(&[0x12, 0]).is_err());
        assert!(wait_reply(&[0x12, 4, 8, 37, 8, 42]).is_err());
        for tail in [
            &b""[..],
            b"grpc-message: \r\n",
            b"grpc-status: 1\r\n",
            b"grpc-status: 1\r\ngrpc-message: \r\n",
            b"grpc-status: 0\r\ngrpc-status: 0\r\n",
            b"grpc-status: 0\r\ngrpc-message: unexpected\r\n",
            b"grpc-status: 0\r\ngrpc-message: \r\ngrpc-status: 1\r\n",
            b"grpc-status: 0\r",
            b"grpc-status: 0\r\ngrpc-message: \r",
        ] {
            assert!(grpc(&wire(&proto, tail)).is_err(), "{tail:?}");
        }
        let mut incomplete = wire(&proto, STOCK);
        incomplete[HEAD.len() + 4] += 1; // body claims a byte that belongs to trailers
        assert!(grpc(&incomplete).is_err());
        let mut crossed = wire(&proto, STATUS);
        crossed.splice(HEAD.len() - 2..HEAD.len() - 2, *b"grpc-status: 1\r\n");
        assert!(grpc(&crossed).is_err());
    }

    #[test]
    fn natural_get_requires_exact_live_init_and_no_input() {
        let id = "a".repeat(64);
        let mut inner = vec![0x12, 64];
        inner.extend(id.as_bytes());
        inner.extend([0x18, 42, 0x20, 2, 0x32, 1, b'o', 0x3a, 1, b'e']);
        let mut outer = vec![0x0a, inner.len() as u8];
        outer.extend(&inner);
        for tail in [STATUS, STOCK] {
            let wire = wire(&outer, tail);
            let body = grpc(&wire).unwrap();
            assert_eq!(running_reply(body, &id).unwrap().pid, 42);
            assert!(running_reply(body, &"b".repeat(64)).is_err());
        }
        outer[71] = 3; // STOPPED instead of RUNNING
        assert!(running_reply(&outer, &id).is_err());
        outer[71] = 2;
        outer[1] += 3;
        outer.extend([0x2a, 1, b'i']);
        assert!(running_reply(&outer, &id).is_err());
    }
}
