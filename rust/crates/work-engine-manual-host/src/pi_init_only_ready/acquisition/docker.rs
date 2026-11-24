//! Fixed local Docker HTTP/1.1 transport. No Docker context, DNS, redirects,
//! negotiation, retries, or connection reuse. A write is not an acceptance.

use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
        unix::{
            ffi::OsStrExt,
            fs::{FileTypeExt, MetadataExt},
            net::UnixStream,
        },
    },
    path::PathBuf,
    time::Instant,
};

const REPLY_MAX: usize = 48 * 1024;
const HEADER_MAX: usize = 4096;

#[derive(Clone, Debug)]
pub(crate) struct Endpoint {
    pub path: PathBuf,
    pub device: u64,
    pub inode: u64,
    pub listener_pid: i32,
    pub listener_uid: u32,
    pub listener_gid: u32,
}

fn invalid(msg: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

impl Endpoint {
    pub(super) fn check_path(&self) -> io::Result<()> {
        let m = fs::symlink_metadata(&self.path)?;
        if !self.path.is_absolute()
            || fs::canonicalize(&self.path)? != self.path
            || !m.file_type().is_socket()
            || m.dev() != self.device
            || m.ino() != self.inode
        {
            return Err(invalid("Docker endpoint identity differs"));
        }
        Ok(())
    }

    pub(super) fn connect(&self) -> io::Result<UnixStream> {
        self.check_path()?;
        // Use a nonblocking AF_UNIX connect: do not wait on a full daemon
        // accept queue or retry an in-progress connection on another socket.
        // Safety: initialize the whole address and pass only its used length.
        let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
        let path = self.path.as_os_str().as_bytes();
        if path.len() >= addr.sun_path.len() || path.contains(&0) {
            return Err(invalid("invalid Docker socket path"));
        }
        for (d, b) in addr.sun_path.iter_mut().zip(path) {
            *d = *b as libc::c_char;
        }
        let len = std::mem::offset_of!(libc::sockaddr_un, sun_path) + path.len() + 1;
        // Safety: validated domain/type; immediately take ownership on success.
        let fd = unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        if unsafe {
            libc::connect(
                fd.as_raw_fd(),
                (&raw const addr).cast(),
                len as libc::socklen_t,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        // Socket activation can report the listener's credentials, not the
        // engine generation. A separate trusted serving check is mandatory.
        let mut peer: libc::ucred = unsafe { std::mem::zeroed() };
        let mut n = std::mem::size_of_val(&peer) as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&raw mut peer).cast(),
                &mut n,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        if n as usize != std::mem::size_of_val(&peer)
            || peer.pid != self.listener_pid
            || peer.uid != self.listener_uid
            || peer.gid != self.listener_gid
        {
            return Err(invalid("Docker listener credentials differ"));
        }
        self.check_path()?;
        Ok(UnixStream::from(fd))
    }
}

pub(super) fn request(target: &str, body: &[u8]) -> Vec<u8> {
    let content_type = if body.is_empty() {
        ""
    } else {
        "Content-Type: application/json\r\n"
    };
    let mut bytes = format!("{target} HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n{content_type}Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    bytes.extend(body);
    bytes
}

pub(super) fn socket_cookie(io: &impl AsRawFd) -> io::Result<u64> {
    let mut cookie = 0_u64;
    let mut len = std::mem::size_of::<u64>() as libc::socklen_t;
    // Safety: correctly sized writable storage for Linux SO_COOKIE.
    if unsafe {
        libc::getsockopt(
            io.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_COOKIE,
            (&raw mut cookie).cast(),
            &mut len,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    if len as usize != std::mem::size_of::<u64>() || cookie == 0 {
        return Err(invalid("missing Unix socket cookie"));
    }
    Ok(cookie)
}

fn wait(io: &UnixStream, events: libc::c_short, deadline: Instant) -> io::Result<()> {
    poll(io, events, None, deadline).map(|_| ())
}

fn poll(
    io: &UnixStream,
    events: libc::c_short,
    watch: Option<RawFd>,
    deadline: Instant,
) -> io::Result<(bool, bool)> {
    loop {
        let left = deadline
            .checked_duration_since(Instant::now())
            .ok_or(io::ErrorKind::TimedOut)?;
        let mut p = [
            libc::pollfd {
                fd: io.as_raw_fd(),
                events,
                revents: 0,
            },
            libc::pollfd {
                fd: watch.unwrap_or(-1),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        // Safety: both poll records are initialized and writable.
        let n = unsafe {
            libc::poll(
                p.as_mut_ptr(),
                p.len() as libc::nfds_t,
                (left.as_millis() + 1).min(i32::MAX as u128) as i32,
            )
        };
        if n > 0 {
            return Ok((p[0].revents != 0, p[1].revents != 0)); // Includes HUP and ERR.
        }
        if n == 0 {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

pub(super) fn write(io: &mut UnixStream, mut data: &[u8], deadline: Instant) -> io::Result<()> {
    while !data.is_empty() {
        wait(io, libc::POLLOUT, deadline)?;
        match io.write(data) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(n) => data = &data[n..],
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

pub(super) fn read(io: &mut UnixStream, buf: &mut [u8], deadline: Instant) -> io::Result<usize> {
    loop {
        wait(io, libc::POLLIN, deadline)?;
        match io.read(buf) {
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            result => return result,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub(super) struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

#[derive(Serialize)]
pub(super) struct Observation {
    pub response: Option<Response>,
    raw: Vec<u8>,
    failure: Option<String>,
}

fn chunked(mut bytes: &[u8]) -> io::Result<Option<Vec<u8>>> {
    let mut out = Vec::new();
    loop {
        let Some(line) = bytes.windows(2).position(|s| s == b"\r\n") else {
            return Ok(None);
        };
        let hex = &bytes[..line];
        if hex.is_empty() || hex.len() > 8 || !hex.iter().all(u8::is_ascii_hexdigit) {
            return Err(invalid("invalid Docker chunk length"));
        }
        let n = usize::from_str_radix(std::str::from_utf8(hex).unwrap(), 16)
            .map_err(|_| invalid("invalid Docker chunk length"))?;
        if n > REPLY_MAX - out.len() {
            return Err(invalid("Docker response too large"));
        }
        bytes = &bytes[line + 2..];
        if bytes.len() < n + 2 {
            return Ok(None);
        }
        if bytes[n..n + 2] != *b"\r\n" {
            return Err(invalid("invalid Docker chunk delimiter or trailer"));
        }
        if n == 0 {
            if bytes.len() != 2 {
                return Err(invalid("extra Docker response bytes"));
            }
            return Ok(Some(out));
        }
        out.extend_from_slice(&bytes[..n]);
        bytes = &bytes[n + 2..];
    }
}

fn decode(raw: &[u8], eof: bool) -> io::Result<Option<Response>> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut res = httparse::Response::new(&mut headers);
    let end = match res
        .parse(raw)
        .map_err(|_| invalid("invalid Docker HTTP response"))?
    {
        httparse::Status::Partial if raw.len() <= HEADER_MAX => return Ok(None),
        httparse::Status::Complete(end) if end <= HEADER_MAX => end,
        _ => return Err(invalid("Docker response headers too large")),
    };
    let status = res
        .code
        .filter(|c| (200..=599).contains(c))
        .ok_or_else(|| invalid("unexpected Docker status"))?;
    if res.version != Some(1) {
        return Err(invalid("unexpected Docker HTTP version"));
    }
    let mut content_length = None;
    let mut transfer = false;
    for h in res.headers {
        if h.name.eq_ignore_ascii_case("content-length") {
            let s = std::str::from_utf8(h.value)
                .map_err(|_| invalid("invalid Docker content length"))?;
            let len = s
                .parse::<usize>()
                .ok()
                .filter(|n| *n <= REPLY_MAX)
                .ok_or_else(|| invalid("invalid Docker content length"))?;
            if content_length.replace(len).is_some() {
                return Err(invalid("duplicate Docker content length"));
            }
        } else if h.name.eq_ignore_ascii_case("transfer-encoding") {
            if transfer || !h.value.eq_ignore_ascii_case(b"chunked") {
                return Err(invalid("unsupported Docker transfer coding"));
            }
            transfer = true;
        }
    }
    let bytes = &raw[end..];
    let body = if status == 204 {
        if transfer || content_length.is_some_and(|n| n != 0) || !bytes.is_empty() {
            return Err(invalid("unexpected Docker 204 body"));
        }
        Some(Vec::new())
    } else if transfer {
        if content_length.is_some() {
            return Err(invalid("ambiguous Docker framing"));
        }
        chunked(bytes)?
    } else if let Some(n) = content_length {
        if bytes.len() > n {
            return Err(invalid("extra Docker response bytes"));
        }
        (bytes.len() == n).then(|| bytes.to_vec())
    } else {
        eof.then(|| bytes.to_vec())
    };
    Ok(body.map(|body| Response { status, body }))
}

pub(super) fn receive(io: &mut UnixStream, deadline: Instant) -> Observation {
    receive_while(io, deadline, None, || Ok(()))
}

/// Drive a response decoder while a second nonblocking stream remains live.
/// The caller holds the watched descriptor open for the entire call.
pub(super) fn receive_while(
    io: &mut UnixStream,
    deadline: Instant,
    watch: Option<RawFd>,
    mut drain: impl FnMut() -> io::Result<()>,
) -> Observation {
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    let seen = loop {
        let (reply_ready, watch_ready) = match poll(io, libc::POLLIN, watch, deadline) {
            Ok(ready) => ready,
            Err(e) => break Err(e),
        };
        if watch_ready && let Err(e) = drain() {
            break Err(e);
        }
        if !reply_ready {
            continue;
        }
        match io.read(&mut buf) {
            Ok(n) if raw.len() + n <= REPLY_MAX => {
                raw.extend_from_slice(&buf[..n]);
                match decode(&raw, n == 0) {
                    Ok(Some(r)) => break Ok(r),
                    Ok(None) if n != 0 => continue,
                    Ok(None) => break Err(invalid("incomplete Docker response")),
                    Err(e) => break Err(e),
                }
            }
            Ok(_) => break Err(invalid("Docker response too large")),
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(e) => break Err(e),
        }
    };
    match seen {
        Ok(response) => Observation {
            raw,
            response: Some(response),
            failure: None,
        },
        Err(e) => Observation {
            raw,
            response: None,
            failure: Some(e.to_string()),
        },
    }
}
