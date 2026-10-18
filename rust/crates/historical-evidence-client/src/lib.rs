//! Passive local client for the bounded historical S5 read operation.
use historical_evidence_wire::graph;
use historical_evidence_wire::{
    MAX_FRAME, Observation, ObservationV2, Reply, ReplyV2, Request, RequestV2,
};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum ReadError {
    InvalidRequest,
    Unavailable,
    Refused,
    InvalidResponse,
}

/// One live, locally cancellable inspection. No background worker owns its socket.
pub enum Inspected {
    V1(Box<Observation>),
    V2(Box<ObservationV2>),
    Graph(Box<graph::Observation>),
}

#[derive(Clone, Copy)]
enum Mode {
    Graph,
    V2,
    V1,
}

enum Phase {
    Connecting,
    Sending { frame: Vec<u8>, offset: usize },
    Length { bytes: [u8; 4], offset: usize },
    Body { bytes: Vec<u8>, offset: usize },
}

pub struct InspectAttempt {
    socket: std::path::PathBuf,
    stream: UnixStream,
    phase: Phase,
    mode: Mode,
    deadline: Instant,
}

impl InspectAttempt {
    pub fn start(socket: &Path, deadline: Instant) -> Result<Self, ReadError> {
        Self::start_mode(socket, deadline, Mode::V2)
    }

    pub fn start_graph(socket: &Path, deadline: Instant) -> Result<Self, ReadError> {
        Self::start_mode(socket, deadline, Mode::Graph)
    }

    fn start_mode(socket: &Path, deadline: Instant, mode: Mode) -> Result<Self, ReadError> {
        let now = Instant::now();
        if !socket.is_absolute() || now >= deadline {
            return Err(ReadError::InvalidRequest);
        }
        let deadline = deadline.min(now + Duration::from_secs(35));
        let (stream, connecting) = nonblocking_connect(socket)?;
        let mut attempt = Self {
            socket: socket.to_path_buf(),
            stream,
            phase: Phase::Connecting,
            mode,
            deadline,
        };
        if !connecting {
            attempt.begin_send()?;
        }
        Ok(attempt)
    }

    fn begin_send(&mut self) -> Result<(), ReadError> {
        let bytes = match self.mode {
            Mode::Graph => {
                let request = graph::Request::all();
                request.validate().map_err(|_| ReadError::InvalidRequest)?;
                serde_json::to_vec(&request).map_err(|_| ReadError::InvalidRequest)?
            }
            Mode::V2 => {
                let request = RequestV2::all();
                request.validate().map_err(|_| ReadError::InvalidRequest)?;
                serde_json::to_vec(&request).map_err(|_| ReadError::InvalidRequest)?
            }
            Mode::V1 => {
                let request = Request::all();
                request.validate().map_err(|_| ReadError::InvalidRequest)?;
                serde_json::to_vec(&request).map_err(|_| ReadError::InvalidRequest)?
            }
        };
        if bytes.len() > MAX_FRAME {
            return Err(ReadError::InvalidRequest);
        }
        let mut frame = Vec::with_capacity(bytes.len() + 4);
        frame.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        frame.extend_from_slice(&bytes);
        self.phase = Phase::Sending { frame, offset: 0 };
        Ok(())
    }

    fn fallback(&mut self) -> Result<(), ReadError> {
        if Instant::now() >= self.deadline {
            return Err(ReadError::Unavailable);
        }
        let (stream, connecting) = nonblocking_connect(&self.socket)?;
        self.stream = stream;
        self.mode = match self.mode {
            Mode::Graph => Mode::V2,
            Mode::V2 => Mode::V1,
            Mode::V1 => return Err(ReadError::Refused),
        };
        self.phase = Phase::Connecting;
        if !connecting {
            self.begin_send()?;
        }
        Ok(())
    }

    /// Advances at most one bounded nonblocking socket operation.
    pub fn poll(&mut self) -> Option<Result<Inspected, ReadError>> {
        if Instant::now() >= self.deadline {
            return Some(Err(ReadError::Unavailable));
        }
        match &mut self.phase {
            Phase::Connecting => {
                let mut fd = libc::pollfd {
                    fd: self.stream.as_raw_fd(),
                    events: libc::POLLOUT,
                    revents: 0,
                };
                let ready = unsafe { libc::poll(&mut fd, 1, 0) };
                if ready < 0 {
                    return Some(Err(ReadError::Unavailable));
                }
                if ready == 0 {
                    return None;
                }
                match self.stream.take_error() {
                    Ok(Some(_)) | Err(_) => return Some(Err(ReadError::Unavailable)),
                    Ok(None) => (),
                }
                if self.begin_send().is_err() {
                    return Some(Err(ReadError::InvalidRequest));
                }
                None
            }
            Phase::Sending { frame, offset } => match self.stream.write(&frame[*offset..]) {
                Ok(0) => Some(Err(ReadError::Unavailable)),
                Ok(count) => {
                    *offset += count;
                    if *offset == frame.len() {
                        self.phase = Phase::Length {
                            bytes: [0; 4],
                            offset: 0,
                        };
                    }
                    None
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
                Err(_) => Some(Err(ReadError::Unavailable)),
            },
            Phase::Length { bytes, offset } => match self.stream.read(&mut bytes[*offset..]) {
                Ok(0) => Some(Err(ReadError::Unavailable)),
                Ok(count) => {
                    *offset += count;
                    if *offset == 4 {
                        let len = u32::from_be_bytes(*bytes) as usize;
                        if len == 0 || len > MAX_FRAME {
                            return Some(Err(ReadError::InvalidResponse));
                        }
                        self.phase = Phase::Body {
                            bytes: vec![0; len],
                            offset: 0,
                        };
                    }
                    None
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
                Err(_) => Some(Err(ReadError::Unavailable)),
            },
            Phase::Body { bytes, offset } => {
                let end = (*offset + 64 * 1024).min(bytes.len());
                match self.stream.read(&mut bytes[*offset..end]) {
                    Ok(0) => Some(Err(ReadError::Unavailable)),
                    Ok(count) => {
                        *offset += count;
                        if *offset != bytes.len() {
                            return None;
                        }
                        if Instant::now() >= self.deadline {
                            return Some(Err(ReadError::Unavailable));
                        }
                        if matches!(self.mode, Mode::Graph) {
                            let result: graph::Reply = match serde_json::from_slice(bytes) {
                                Ok(reply) => reply,
                                Err(_) => return Some(Err(ReadError::InvalidResponse)),
                            };
                            if Instant::now() >= self.deadline {
                                return Some(Err(ReadError::Unavailable));
                            }
                            match result {
                                graph::Reply::Observed { observation } => {
                                    if observation.validate().is_err() {
                                        Some(Err(ReadError::InvalidResponse))
                                    } else if Instant::now() >= self.deadline {
                                        Some(Err(ReadError::Unavailable))
                                    } else {
                                        Some(Ok(Inspected::Graph(Box::new(observation))))
                                    }
                                }
                                graph::Reply::Refused { .. } => match self.fallback() {
                                    Ok(()) => None,
                                    Err(error) => Some(Err(error)),
                                },
                                graph::Reply::Unavailable { .. } => {
                                    Some(Err(ReadError::Unavailable))
                                }
                            }
                        } else if matches!(self.mode, Mode::V1) {
                            let result: Reply = match serde_json::from_slice(bytes) {
                                Ok(reply) => reply,
                                Err(_) => return Some(Err(ReadError::InvalidResponse)),
                            };
                            if Instant::now() >= self.deadline {
                                return Some(Err(ReadError::Unavailable));
                            }
                            match result {
                                Reply::Observed { observation } => {
                                    if observation.validate().is_err() {
                                        Some(Err(ReadError::InvalidResponse))
                                    } else if Instant::now() >= self.deadline {
                                        Some(Err(ReadError::Unavailable))
                                    } else {
                                        Some(Ok(Inspected::V1(Box::new(observation))))
                                    }
                                }
                                Reply::Refused { .. } => Some(Err(ReadError::Refused)),
                                Reply::Unavailable { .. } => Some(Err(ReadError::Unavailable)),
                            }
                        } else {
                            let result: ReplyV2 = match serde_json::from_slice(bytes) {
                                Ok(reply) => reply,
                                Err(_) => return Some(Err(ReadError::InvalidResponse)),
                            };
                            if Instant::now() >= self.deadline {
                                return Some(Err(ReadError::Unavailable));
                            }
                            match result {
                                ReplyV2::Observed { observation } => {
                                    if observation.validate().is_err() {
                                        Some(Err(ReadError::InvalidResponse))
                                    } else if Instant::now() >= self.deadline {
                                        Some(Err(ReadError::Unavailable))
                                    } else {
                                        Some(Ok(Inspected::V2(Box::new(observation))))
                                    }
                                }
                                ReplyV2::Refused { .. } => match self.fallback() {
                                    Ok(()) => None,
                                    Err(error) => Some(Err(error)),
                                },
                                ReplyV2::Unavailable { .. } => Some(Err(ReadError::Unavailable)),
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
                    Err(_) => Some(Err(ReadError::Unavailable)),
                }
            }
        }
    }
}

fn nonblocking_connect(path: &Path) -> Result<(UnixStream, bool), ReadError> {
    let bytes = path.as_os_str().as_bytes();
    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.is_empty() || bytes.contains(&0) || bytes.len() >= addr.sun_path.len() {
        return Err(ReadError::InvalidRequest);
    }
    addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (slot, byte) in addr.sun_path.iter_mut().zip(bytes) {
        *slot = *byte as libc::c_char;
    }
    let fd = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            0,
        )
    };
    if fd < 0 {
        return Err(ReadError::Unavailable);
    }
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    let len = std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1;
    let result = unsafe {
        libc::connect(
            stream.as_raw_fd(),
            (&addr as *const libc::sockaddr_un).cast::<libc::sockaddr>(),
            len as libc::socklen_t,
        )
    };
    if result == 0 {
        return Ok((stream, false));
    }
    let error = std::io::Error::last_os_error();
    if matches!(
        error.raw_os_error(),
        Some(libc::EINPROGRESS | libc::EALREADY)
    ) {
        Ok((stream, true))
    } else {
        Err(ReadError::Unavailable)
    }
}

#[cfg(test)]
mod bounded_attempt_tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;
    use std::os::unix::net::UnixListener;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn socket(case: &str) -> (std::path::PathBuf, UnixListener) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::var_os("S5_TEST_SOCKET_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let dir = base.join(format!(
            "s5{}-{case}-{:x}",
            std::process::id(),
            nonce % 0x10000
        ));
        std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
        let path = dir.join("read.sock");
        let listener = UnixListener::bind(&path).unwrap();
        (path, listener)
    }

    fn request(stream: &mut UnixStream) {
        let mut len = [0; 4];
        stream.read_exact(&mut len).unwrap();
        let mut body = vec![0; u32::from_be_bytes(len) as usize];
        stream.read_exact(&mut body).unwrap();
    }

    fn until_done(attempt: &mut InspectAttempt) -> Result<Inspected, ReadError> {
        let started = Instant::now();
        loop {
            if let Some(result) = attempt.poll() {
                assert!(started.elapsed() < Duration::from_secs(1));
                return result;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn silent_peer_has_absolute_deadline_and_drop_closes_socket() {
        let (path, listener) = socket("silent");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            request(&mut stream);
            let mut byte = [0];
            stream.read(&mut byte).unwrap()
        });
        let mut attempt =
            InspectAttempt::start(&path, Instant::now() + Duration::from_millis(80)).unwrap();
        assert!(matches!(
            until_done(&mut attempt),
            Err(ReadError::Unavailable)
        ));
        drop(attempt);
        assert_eq!(server.join().unwrap(), 0);
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn trickle_after_refused_v2_cannot_extend_v1_deadline() {
        let (path, listener) = socket("trickle");
        let server = std::thread::spawn(move || {
            let (mut first, _) = listener.accept().unwrap();
            request(&mut first);
            let reply = serde_json::to_vec(&ReplyV2::Refused {
                reason: "v2 not served".into(),
            })
            .unwrap();
            first
                .write_all(&(reply.len() as u32).to_be_bytes())
                .unwrap();
            first.write_all(&reply).unwrap();
            let (mut second, _) = listener.accept().unwrap();
            second
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            request(&mut second);
            let mut byte = [0];
            second.write_all(&[0, 0, 0, 32]).unwrap();
            for _ in 0..6 {
                if second.write_all(b"x").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            second.read(&mut byte).unwrap_or(0)
        });
        let mut attempt =
            InspectAttempt::start(&path, Instant::now() + Duration::from_millis(100)).unwrap();
        assert!(matches!(
            until_done(&mut attempt),
            Err(ReadError::Unavailable)
        ));
        drop(attempt);
        let _ = server.join().unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn v2_trickle_cannot_extend_absolute_deadline() {
        let (path, listener) = socket("v2-trickle");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            request(&mut stream);
            stream.write_all(&[0, 0, 0, 32]).unwrap();
            for _ in 0..6 {
                if stream.write_all(b"x").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        });
        let mut attempt =
            InspectAttempt::start(&path, Instant::now() + Duration::from_millis(80)).unwrap();
        assert!(matches!(
            until_done(&mut attempt),
            Err(ReadError::Unavailable)
        ));
        drop(attempt);
        server.join().unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    #[ignore = "requires S5_QUALIFIED_REPLY from a private admitted owner read"]
    fn valid_observed_frame_delivered_after_deadline_is_rejected() {
        let reply = std::fs::read(std::env::var("S5_QUALIFIED_REPLY").unwrap()).unwrap();
        let (path, listener) = socket("late-valid");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            request(&mut stream);
            std::thread::sleep(Duration::from_millis(90));
            stream
                .write_all(&(reply.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(&reply).unwrap();
        });
        let mut attempt =
            InspectAttempt::start(&path, Instant::now() + Duration::from_millis(50)).unwrap();
        // Drive the nonblocking connect/request before waiting for the peer.
        assert!(attempt.poll().is_none());
        server.join().unwrap();
        assert!(matches!(attempt.poll(), Some(Err(ReadError::Unavailable))));
        drop(attempt);
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn repeated_abandonment_drops_each_local_socket() {
        let (path, listener) = socket("abandon");
        let server = std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut byte = [0];
                assert_eq!(stream.read(&mut byte).unwrap(), 0);
            }
        });
        for _ in 0..3 {
            let attempt =
                InspectAttempt::start(&path, Instant::now() + Duration::from_secs(1)).unwrap();
            drop(attempt);
        }
        server.join().unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(path.parent().unwrap()).unwrap();
    }

    #[test]
    #[ignore = "requires S5_GRAPH_REPLY from a private admitted owner read"]
    fn graph_frame_is_validated_before_render_and_corruption_is_refused() {
        let raw = std::fs::read(std::env::var("S5_GRAPH_REPLY").unwrap()).unwrap();
        let mut changed_edge: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        changed_edge["observation"]["edges"][0]["owner"] = "role-scheduler".into();
        let mut missing_authority: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        missing_authority["observation"]["base"]["base"]["replacement_links"] =
            serde_json::json!([]);
        missing_authority["observation"]["edges"]
            .as_array_mut()
            .unwrap()
            .retain(|edge| edge["kind"] != "replaces_reported_episode");
        let mut public_source: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        public_source["observation"]["base"]["source_reads"][0]["confidentiality"] =
            "public".into();
        public_source["observation"]["base"]["base"]["source_reads"][0]["confidentiality"] =
            "public".into();
        let mut invalid_tree: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        invalid_tree["observation"]["base"]["base"]["candidates"][0]["tree_oid"] =
            "file:///private/not-an-oid".into();
        let mut cases: Vec<(String, Vec<u8>, bool)> = vec![
            ("graph-good".into(), raw, true),
            (
                "graph-edge".into(),
                serde_json::to_vec(&changed_edge).unwrap(),
                false,
            ),
            (
                "graph-link".into(),
                serde_json::to_vec(&missing_authority).unwrap(),
                false,
            ),
            (
                "graph-public".into(),
                serde_json::to_vec(&public_source).unwrap(),
                false,
            ),
            (
                "graph-tree".into(),
                serde_json::to_vec(&invalid_tree).unwrap(),
                false,
            ),
        ];
        for episode in 0..3 {
            for variant in ["empty", "short", "long"] {
                let mut changed: serde_json::Value = serde_json::from_slice(&cases[0].1).unwrap();
                let revisions =
                    changed["observation"]["base"]["base"]["reviews"][episode]["revisions"]
                        .as_array_mut()
                        .unwrap();
                match variant {
                    "empty" => revisions.clear(),
                    "short" => {
                        revisions.pop();
                    }
                    "long" => revisions.push(revisions.last().unwrap().clone()),
                    _ => unreachable!(),
                }
                let frame = serde_json::to_vec(&changed).unwrap();
                assert!(
                    frame.len() <= MAX_FRAME,
                    "cardinality fixture exceeds frame bound"
                );
                let label = format!("graph-r{episode}-{variant}");
                cases.push((label, frame, false));
            }
        }
        for (case, frame, valid) in cases {
            let (path, listener) = socket(&case);
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                request(&mut stream);
                stream
                    .write_all(&(frame.len() as u32).to_be_bytes())
                    .unwrap();
                stream.write_all(&frame).unwrap();
            });
            let mut attempt =
                InspectAttempt::start_graph(&path, Instant::now() + Duration::from_secs(3))
                    .unwrap();
            let observed = until_done(&mut attempt);
            assert_eq!(matches!(observed, Ok(Inspected::Graph(_))), valid);
            if !valid {
                assert!(matches!(observed, Err(ReadError::InvalidResponse)));
            }
            drop(attempt);
            server.join().unwrap();
            std::fs::remove_file(&path).unwrap();
            std::fs::remove_dir(path.parent().unwrap()).unwrap();
        }
    }
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

pub fn inspect_v2(
    socket: &Path,
    request: historical_evidence_wire::RequestV2,
) -> Result<historical_evidence_wire::ObservationV2, ReadError> {
    use historical_evidence_wire::ReplyV2;
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
    let size = u32::from_be_bytes(len) as usize;
    if size == 0 || size > MAX_FRAME {
        return Err(ReadError::InvalidResponse);
    }
    let mut response = vec![0; size];
    stream
        .read_exact(&mut response)
        .map_err(|_| ReadError::Unavailable)?;
    match serde_json::from_slice(&response).map_err(|_| ReadError::InvalidResponse)? {
        ReplyV2::Observed { observation } => {
            observation
                .validate()
                .map_err(|_| ReadError::InvalidResponse)?;
            Ok(observation)
        }
        ReplyV2::Refused { .. } => Err(ReadError::Refused),
        ReplyV2::Unavailable { .. } => Err(ReadError::Unavailable),
    }
}

/// Synchronous snapshot qualification through the same absolute-deadline,
/// nonblocking Graph → v2 → v1 attempt used by the interactive terminal.
pub fn inspect_best(socket: &Path) -> Result<Inspected, ReadError> {
    let mut attempt =
        InspectAttempt::start_graph(socket, Instant::now() + Duration::from_secs(35))?;
    loop {
        if let Some(result) = attempt.poll() {
            return result;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
mod v2_nullable_regression_tests {
    use super::*;
    use historical_evidence_wire::RequestV2;
    use std::os::unix::fs::DirBuilderExt;
    use std::os::unix::net::UnixListener;

    fn read_through_client(reply: &serde_json::Value, case: &str) -> Result<(), ReadError> {
        let base = std::env::var_os("S5_TEST_SOCKET_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let directory = base.join(format!(
            "historical-evidence-client-{}-{case}",
            std::process::id()
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .unwrap();
        let socket = directory.join("read.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let bytes = serde_json::to_vec(reply).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let mut request = vec![0; u32::from_be_bytes(length) as usize];
            stream.read_exact(&mut request).unwrap();
            stream
                .write_all(&(bytes.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(&bytes).unwrap();
        });
        let result = inspect_v2(&socket, RequestV2::all()).map(|_| ());
        server.join().unwrap();
        std::fs::remove_file(&socket).unwrap();
        std::fs::remove_dir(&directory).unwrap();
        result
    }

    // The qualified reply is private owner evidence; the fixture path is supplied
    // during local qualification and its contents never enter the repository.
    #[test]
    #[ignore = "requires S5_QUALIFIED_REPLY pointing at a private qualified reply"]
    fn qualified_reply_requires_explicit_nullable_observations() {
        let path = std::env::var("S5_QUALIFIED_REPLY").expect("qualified reply path");
        let bytes = std::fs::read(path).unwrap();
        let reply: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            reply["observation"]["scheduler"].get("presented_at"),
            Some(&serde_json::Value::Null)
        );
        assert!(read_through_client(&reply, "observed").is_ok());

        let mut omitted = reply.clone();
        omitted["observation"]["scheduler"]
            .as_object_mut()
            .unwrap()
            .remove("presented_at");
        assert!(matches!(
            read_through_client(&omitted, "missing-presentation"),
            Err(ReadError::InvalidResponse)
        ));

        let mut wrong_type = reply.clone();
        wrong_type["observation"]["scheduler"]["presented_at"] = serde_json::json!(false);
        assert!(matches!(
            read_through_client(&wrong_type, "wrong-presentation"),
            Err(ReadError::InvalidResponse)
        ));

        let mut non_null = reply.clone();
        non_null["observation"]["scheduler"]["presented_at"] =
            serde_json::json!("2026-10-16T00:00:00Z");
        assert!(matches!(
            read_through_client(&non_null, "non-null-presentation"),
            Err(ReadError::InvalidResponse)
        ));

        for field in ["metrics", "metrics_unavailable"] {
            let mut omitted = reply.clone();
            omitted["observation"]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(matches!(
                read_through_client(&omitted, field),
                Err(ReadError::InvalidResponse)
            ));
        }
        for field in ["model", "api_error_status"] {
            let mut omitted = reply.clone();
            omitted["observation"]["provider"]["attempts"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(matches!(
                read_through_client(&omitted, field),
                Err(ReadError::InvalidResponse)
            ));
        }
    }
}
