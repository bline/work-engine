use serde_json::{Value, json};
use std::{
    collections::HashSet,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, net::UnixStream},
    },
    path::Path,
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket, client::client_with_config, protocol::WebSocketConfig};

const MAX_HISTORY: usize = 10_000;
const MAX_PAGES: usize = 100;
const MAX_MESSAGE: usize = 2 * 1024 * 1024;

// Socket timeouts are per system call. Recompute them on every read and write,
// including calls made internally while tungstenite assembles fragmented frames.
struct DeadlineStream {
    stream: UnixStream,
    deadline: Instant,
}
impl DeadlineStream {
    fn remaining(&self) -> io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "native deadline"))
    }
}
impl Read for DeadlineStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        self.stream.read(buf)
    }
}
impl Write for DeadlineStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.flush()
    }
}

// A local socket connect can also block on a full listen queue. Nonblocking
// connect plus poll keeps that step inside the same deadline as the handshake.
fn connect_until(path: &Path, deadline: Instant) -> io::Result<UnixStream> {
    if Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "native connect deadline",
        ));
    }
    let bytes = path.as_os_str().as_bytes();
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.is_empty() || bytes.contains(&0) || bytes.len() >= address.sun_path.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "native socket path",
        ));
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (dst, src) in address.sun_path.iter_mut().zip(bytes) {
        *dst = *src as libc::c_char;
    }
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
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    let len =
        (std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1) as libc::socklen_t;
    let rc = unsafe { libc::connect(fd, (&address as *const libc::sockaddr_un).cast(), len) };
    if rc < 0 {
        let err = io::Error::last_os_error();
        if err.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(err);
        }
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .filter(|d| !d.is_zero())
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::TimedOut, "native connect deadline")
                })?;
            let ms = remaining
                .as_millis()
                .saturating_add(1)
                .min(i32::MAX as u128) as i32;
            let mut pfd = libc::pollfd {
                fd: stream.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut pfd, 1, ms) };
            if ready == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "native connect deadline",
                ));
            }
            if ready < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }
            if let Some(err) = stream.take_error()? {
                return Err(err);
            }
            break;
        }
    }
    if Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "native connect deadline",
        ));
    }
    stream.set_nonblocking(false)?;
    Ok(stream)
}

pub struct Native {
    ws: WebSocket<DeadlineStream>,
    id: u64,
    deadline: Instant,
}
impl Native {
    pub fn connect(path: &str) -> Result<Self, ()> {
        Self::connect_with_deadline(path, Instant::now() + Duration::from_secs(60))
    }
    pub fn connect_with_deadline(path: &str, deadline: Instant) -> Result<Self, ()> {
        let handshake_deadline = deadline.min(Instant::now() + Duration::from_secs(10));
        let stream = DeadlineStream {
            stream: connect_until(Path::new(path), handshake_deadline).map_err(|_| ())?,
            deadline: handshake_deadline,
        };
        let (ws, _) = client_with_config(
            "ws://localhost/",
            stream,
            Some(
                WebSocketConfig::default()
                    .max_message_size(Some(MAX_MESSAGE))
                    .max_frame_size(Some(MAX_MESSAGE)),
            ),
        )
        .map_err(|_| ())?;
        let mut me = Self {
            ws,
            id: 0,
            deadline,
        };
        me.call_until(
            "initialize",
            json!({"clientInfo":{"name":"session-bridge","version":"1"}}),
            handshake_deadline,
        )?;
        me.ws
            .send(Message::Text(
                json!({"method":"initialized"}).to_string().into(),
            ))
            .map_err(|_| ())?;
        Ok(me)
    }
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, ()> {
        self.call_until(method, params, Instant::now() + Duration::from_secs(10))
    }
    fn call_until(&mut self, method: &str, params: Value, deadline: Instant) -> Result<Value, ()> {
        self.ws.get_mut().deadline = deadline.min(self.deadline);
        self.id += 1;
        let id = self.id;
        self.ws
            .send(Message::Text(
                json!({"id":id,"method":method,"params":params})
                    .to_string()
                    .into(),
            ))
            .map_err(|_| ())?;
        for _ in 0..1024 {
            let message = self.ws.read().map_err(|_| ())?;
            if message.len() > MAX_MESSAGE {
                return Err(());
            }
            if let Message::Text(text) = message {
                let value: Value = serde_json::from_str(&text).map_err(|_| ())?;
                if value.get("id").and_then(Value::as_u64) == Some(id) {
                    if value.get("error").is_some() {
                        return Err(());
                    }
                    return value.get("result").cloned().ok_or(());
                }
            }
        }
        Err(())
    }
    fn pages(&mut self, method: &str, session: Option<&str>) -> Result<Vec<Value>, ()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut cursor: Option<String> = None;
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        let mut bytes = 0usize;
        for _ in 0..MAX_PAGES {
            let mut p = json!({"limit":100});
            if let Some(s) = session {
                p["threadId"] = json!(s);
                p["sortDirection"] = json!("asc");
                p["itemsView"] = json!("full");
            }
            if let Some(c) = &cursor {
                p["cursor"] = json!(c);
            }
            let page = self.call_until(method, p, deadline)?;
            bytes = bytes
                .checked_add(serde_json::to_vec(&page).map_err(|_| ())?.len())
                .ok_or(())?;
            if bytes > 8 * 1024 * 1024 {
                return Err(());
            }
            let data = page.get("data").and_then(Value::as_array).ok_or(())?;
            result.extend(data.iter().cloned());
            if result.len() > MAX_HISTORY {
                return Err(());
            }
            match page.get("nextCursor") {
                Some(Value::Null) => return Ok(result),
                Some(Value::String(c)) if !c.is_empty() && seen.insert(c.clone()) => {
                    cursor = Some(c.clone())
                }
                _ => return Err(()),
            }
        }
        Err(())
    }
    pub fn history(&mut self, session: &str) -> Result<Vec<Value>, ()> {
        let turns = self.pages("thread/turns/list", Some(session))?;
        let mut ids = HashSet::new();
        for turn in &turns {
            let id = turn.get("id").and_then(Value::as_str).ok_or(())?;
            let status = turn.get("status").and_then(Value::as_str).ok_or(())?;
            if !crate::wire::valid_uuid(id)
                || !ids.insert(id.to_owned())
                || turn.get("items").and_then(Value::as_array).is_none()
                || !["inProgress", "completed", "failed", "interrupted"].contains(&status)
            {
                return Err(());
            }
        }
        Ok(turns)
    }
    pub fn verify_target(&mut self, session: &str, workspace: &str) -> Result<(), ()> {
        let loaded = self.pages("thread/loaded/list", None)?;
        if loaded
            .iter()
            .filter(|v| v.as_str() == Some(session))
            .count()
            != 1
        {
            return Err(());
        }
        let response = self.call("thread/read", json!({"threadId":session}))?;
        let t = response.get("thread").ok_or(())?;
        if t.get("id").and_then(Value::as_str) != Some(session)
            || t.get("cwd").and_then(Value::as_str) != Some(workspace)
        {
            return Err(());
        }
        Ok(())
    }
    pub fn preflight(
        &mut self,
        session: &str,
        workspace: &str,
    ) -> Result<Vec<Value>, &'static str> {
        let unavailable = |_| "history_unavailable";
        let loaded = self
            .pages("thread/loaded/list", None)
            .map_err(unavailable)?;
        if loaded
            .iter()
            .filter(|v| v.as_str() == Some(session))
            .count()
            != 1
        {
            return Err("history_unavailable");
        }
        let response = self
            .call("thread/read", json!({"threadId":session}))
            .map_err(unavailable)?;
        let t = response.get("thread").ok_or("history_unavailable")?;
        if t.get("id").and_then(Value::as_str) != Some(session)
            || t.get("cwd").and_then(Value::as_str) != Some(workspace)
        {
            return Err("target_mismatch");
        }
        match t.pointer("/status/type").and_then(Value::as_str) {
            Some("idle") => (),
            Some("active") => return Err("session_busy"),
            _ => return Err("history_unavailable"),
        }
        let first = self.history(session).map_err(unavailable)?;
        if first
            .iter()
            .any(|t| t.get("status").and_then(Value::as_str) == Some("inProgress"))
        {
            return Err("session_busy");
        }
        if first != self.history(session).map_err(unavailable)? {
            return Err("history_unavailable");
        }
        Ok(first)
    }
    pub fn resume(&mut self, session: &str, workspace: &str) -> Result<(), ()> {
        let response = self.call("thread/resume", json!({"threadId":session}))?;
        let t = response.get("thread").ok_or(())?;
        if t.get("id").and_then(Value::as_str) != Some(session)
            || t.get("cwd").and_then(Value::as_str) != Some(workspace)
            || t.pointer("/status/type").and_then(Value::as_str) != Some("idle")
        {
            return Err(());
        }
        Ok(())
    }
    pub fn start(&mut self, session: &str, operation: &str, wire: &str) -> Result<String, ()> {
        let r = self.call("turn/start", json!({"threadId":session,"input":[{"type":"text","text":wire}],"clientUserMessageId":operation}))?;
        r.pointer("/turn/id")
            .and_then(Value::as_str)
            .filter(|s| crate::wire::valid_uuid(s))
            .map(str::to_owned)
            .ok_or(())
    }
}

#[cfg(test)]
mod deadline_tests {
    use super::*;
    use std::thread;

    #[test]
    fn expired_deadline_prevents_connect() {
        let err = connect_until(Path::new("/nonexistent"), Instant::now()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    }

    #[test]
    fn fragmented_reads_do_not_renew_deadline() {
        let (reader, mut writer) = UnixStream::pair().unwrap();
        let handle = thread::spawn(move || {
            for _ in 0..8 {
                if writer.write_all(b"x").is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(30));
            }
        });
        let mut reader = DeadlineStream {
            stream: reader,
            deadline: Instant::now() + Duration::from_millis(100),
        };
        let start = Instant::now();
        let mut byte = [0];
        let mut count = 0;
        while reader.read_exact(&mut byte).is_ok() {
            count += 1;
        }
        assert!(count > 0 && count < 8);
        assert!(start.elapsed() < Duration::from_millis(220));
        drop(reader);
        handle.join().unwrap();
    }
}
