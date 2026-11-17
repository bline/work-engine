//! Incremental host-to-native response relay over the framed Pi IPC.
//!
//! The host owns the upstream connection and credentials; the workload only
//! receives screened response bytes. Delivery is consumer-driven: one bounded
//! chunk is read upstream and forwarded per native `pull`, so a slow reader
//! applies backpressure instead of growing host buffers. A native `cancel`
//! closes the upstream read. Interrupted or cancelled streams return an
//! explicit non-complete outcome; this module never replays or settles them.
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use review_execution_evidence::pi::{Binding, LiveStore};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel};
use std::time::{Duration, Instant};

pub type Result<T> = std::result::Result<T, &'static str>;
pub const CHUNK: usize = 16 * 1024;
/// Relayed response bound: covers the catalog's 128k output tokens at the
/// live delta size (about 250 bytes per token, phase 6 measurement).
pub const MAX_RESPONSE: usize = 32 * 1024 * 1024;
const MAX_HEAD: usize = 8192;

/// Framed native endpoint; the probe wrapper records every received frame.
pub trait Channel {
    fn send(&mut self, v: &Value) -> Result<()>;
    fn receive(&mut self) -> Result<Value>;
    /// Whether a native frame (or EOF) can be received without blocking.
    fn readable(&mut self) -> Result<bool>;
}
/// Exclusion screen for relayed bytes. See [`LiveStore::stream_hold`].
pub trait Screen {
    fn hold(&self, pending: &[u8]) -> Result<usize>;
}
impl Screen for LiveStore {
    fn hold(&self, pending: &[u8]) -> Result<usize> {
        self.stream_hold(pending)
    }
}
pub fn now_millis() -> u64 {
    crate::pi_native::now()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Framing {
    Length(usize),
    Chunked,
    Close,
}
/// Incremental HTTP/1.1 response body reader with explicit framing checks.
pub struct Upstream<R: Read> {
    reader: BufReader<R>,
    framing: Framing,
    chunk_left: usize,
    done: bool,
    pub status: u16,
}
impl<R: Read> Upstream<R> {
    pub fn open(r: R) -> Result<Self> {
        let mut reader = BufReader::with_capacity(CHUNK, r);
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut line = Vec::new();
            let n = (&mut reader)
                .take((MAX_HEAD - head.len()) as u64 + 1)
                .read_until(b'\n', &mut line)
                .map_err(|_| "upstream_head_read")?;
            if n == 0 {
                return Err("upstream_head_eof");
            }
            head.extend(line);
            if head.len() > MAX_HEAD {
                return Err("upstream_head_bound");
            }
        }
        let text = std::str::from_utf8(&head).map_err(|_| "upstream_head_utf8")?;
        let mut lines = text.split("\r\n");
        let status_line = lines.next().ok_or("upstream_status")?;
        let mut parts = status_line.splitn(3, ' ');
        if parts.next() != Some("HTTP/1.1") {
            return Err("upstream_version");
        }
        let status: u16 = parts
            .next()
            .and_then(|s| s.parse().ok())
            .filter(|s| (100..600).contains(s))
            .ok_or("upstream_status")?;
        let mut length = None;
        let mut chunked = false;
        for line in lines.filter(|l| !l.is_empty()) {
            let (k, v) = line.split_once(':').ok_or("upstream_header")?;
            let v = v.trim();
            match k.to_ascii_lowercase().as_str() {
                "content-length" => {
                    let n: usize = v.parse().map_err(|_| "upstream_length")?;
                    if length.replace(n).is_some_and(|p| p != n) || n > MAX_RESPONSE {
                        return Err("upstream_length");
                    }
                }
                "transfer-encoding" => {
                    if !v.eq_ignore_ascii_case("chunked") {
                        return Err("upstream_transfer_encoding");
                    }
                    chunked = true;
                }
                "content-encoding" if !v.eq_ignore_ascii_case("identity") => {
                    return Err("upstream_content_encoding");
                }
                _ => {}
            }
        }
        let framing = match (chunked, length) {
            (true, Some(_)) => return Err("upstream_ambiguous_framing"),
            (true, None) => Framing::Chunked,
            (false, Some(n)) => Framing::Length(n),
            (false, None) => Framing::Close,
        };
        Ok(Self {
            reader,
            framing,
            chunk_left: 0,
            done: matches!(framing, Framing::Length(0)),
            status,
        })
    }
    /// Next body bytes (at most `max`), or `None` at a correctly framed end.
    pub fn next(&mut self, max: usize) -> Result<Option<Vec<u8>>> {
        if self.done {
            return Ok(None);
        }
        let want = match self.framing {
            Framing::Length(left) => left.min(max),
            Framing::Close => max,
            Framing::Chunked => {
                if self.chunk_left == 0 {
                    let size = self.line()?;
                    let size = size.split(';').next().unwrap_or("").trim();
                    let n = usize::from_str_radix(size, 16).map_err(|_| "upstream_chunk_size")?;
                    if n > MAX_RESPONSE {
                        return Err("upstream_chunk_size");
                    }
                    if n == 0 {
                        // Trailers are not accepted; the terminator must follow.
                        if !self.line()?.is_empty() {
                            return Err("upstream_trailer");
                        }
                        self.done = true;
                        return Ok(None);
                    }
                    self.chunk_left = n;
                }
                self.chunk_left.min(max)
            }
        };
        let mut b = vec![0; want];
        let n = self
            .reader
            .read(&mut b)
            .map_err(|_| "upstream_interrupted")?;
        b.truncate(n);
        match self.framing {
            Framing::Close if n == 0 => {
                self.done = true;
                return Ok(None);
            }
            Framing::Length(left) => {
                if n == 0 {
                    return Err("upstream_truncated");
                }
                self.framing = Framing::Length(left - n);
                self.done = left == n;
            }
            Framing::Chunked => {
                if n == 0 {
                    return Err("upstream_truncated");
                }
                self.chunk_left -= n;
                if self.chunk_left == 0 && !self.line()?.is_empty() {
                    return Err("upstream_chunk_terminator");
                }
            }
            Framing::Close => {}
        }
        Ok(Some(b))
    }
    fn line(&mut self) -> Result<String> {
        let mut l = Vec::new();
        let n = (&mut self.reader)
            .take(1026)
            .read_until(b'\n', &mut l)
            .map_err(|_| "upstream_interrupted")?;
        if n == 0 {
            return Err("upstream_truncated");
        }
        if !l.ends_with(b"\r\n") {
            return Err("upstream_chunk_line");
        }
        l.truncate(l.len() - 2);
        String::from_utf8(l).map_err(|_| "upstream_chunk_line")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Upstream framed end observed and the native consumer received `end`.
    Completed,
    /// Native consumer released the stream. `upstream_end` records whether
    /// the provider's framed end had already been observed.
    Cancelled { upstream_end: bool },
    /// Upstream failed after possible provider entry; outcome is uncertain.
    Interrupted(&'static str),
}
impl Outcome {
    pub fn label(&self) -> &'static str {
        match self {
            Outcome::Completed => "completed",
            Outcome::Cancelled { upstream_end: true } => "released-after-upstream-end",
            Outcome::Cancelled {
                upstream_end: false,
            } => "cancelled-before-upstream-end",
            Outcome::Interrupted(_) => "interrupted-uncertain",
        }
    }
}
pub struct Relay {
    pub status: u16,
    pub outcome: Outcome,
    /// Screened bytes actually forwarded to the native consumer.
    pub forwarded: Vec<u8>,
    /// Per forwarded chunk: (ordinal, bytes, pull received ms, forwarded ms,
    /// upstream framed end already observed when the pull arrived).
    pub chunks: Vec<(u64, usize, u64, u64, bool)>,
    pub upstream_end_ms: Option<u64>,
    pub opened_ms: u64,
    pub unforwarded_after_release: usize,
    /// Those bytes (read while draining, up to the framed end, the drain's
    /// time bound or [`DRAIN_MAX`]), screened like forwarded bytes, kept as
    /// evidence. Never forwarded to the consumer. A suffix the screen held
    /// back (a possible access-material prefix) is dropped, not kept.
    pub unforwarded: Vec<u8>,
    /// The release drain ran to its end (framed end, time bound, size bound
    /// or read error).
    pub drained: bool,
    /// Provider bytes read but neither forwarded nor retained: held back by
    /// the screen when the stream was released.
    pub held_dropped: usize,
    /// Read or framing error that ended the release drain.
    pub drain_error: Option<&'static str>,
    /// Time a native cancel waited behind an outstanding pull, if any.
    pub cancel_wait_ms: Option<u64>,
    /// How the consumer closed after a host terminal frame: `ack`, or a
    /// `cancel` that raced the terminal frame (then confirmed). `None` when
    /// the stream ended by cancellation.
    pub closing: Option<&'static str>,
}
impl Relay {
    pub fn sha256(&self) -> String {
        format!("{:x}", Sha256::digest(&self.forwarded))
    }
    /// Pulls (consumer demand after a consumed chunk) observed before the
    /// upstream framed end. Positive values show incremental consumption.
    pub fn pulls_before_upstream_end(&self) -> usize {
        self.chunks.iter().skip(1).filter(|c| !c.4).count()
    }
    pub fn evidence(&self) -> Value {
        json!({
            "status": self.status,
            "outcome": self.outcome.label(),
            "interruption": match self.outcome { Outcome::Interrupted(c) => json!(c), _ => json!(null) },
            "forwardedBytes": self.forwarded.len(),
            "forwardedSha256": self.sha256(),
            "chunks": self.chunks.iter().map(|(o, n, p, f, e)| json!({"ordinal":o,"bytes":n,"pullMs":p,"forwardMs":f,"upstreamEndedAtPull":e})).collect::<Vec<_>>(),
            "openedMs": self.opened_ms,
            "upstreamEndMs": self.upstream_end_ms,
            "pullsBeforeUpstreamEnd": self.pulls_before_upstream_end(),
            "unforwardedAfterRelease": self.unforwarded_after_release,
            "unforwardedSha256": (!self.unforwarded.is_empty()).then(|| format!("{:x}", Sha256::digest(&self.unforwarded))),
            "drained": self.drained,
            "heldDropped": self.held_dropped,
            "drainError": self.drain_error,
            "cancelDuringPendingPullMs": self.cancel_wait_ms,
            "closing": self.closing,
            "usage": usage(&self.forwarded),
        })
    }
}

/// Data payloads of complete SSE events. Lines end in CRLF, LF or CR; a
/// blank line dispatches; multiple `data` lines join with LF; one leading
/// space after the colon is removed. A trailing undispatched event is ignored.
pub fn sse_data(sse: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(sse);
    let mut out = Vec::new();
    let mut data: Option<String> = None;
    let mut rest: &str = &text;
    while let Some(i) = rest.find(['\r', '\n']) {
        let line = &rest[..i];
        let skip = if rest[i..].starts_with("\r\n") { 2 } else { 1 };
        rest = &rest[i + skip..];
        if line.is_empty() {
            if let Some(d) = data.take() {
                out.push(d);
            }
            continue;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        if field == "data" {
            let value = value.strip_prefix(' ').unwrap_or(value);
            match &mut data {
                Some(d) => {
                    d.push('\n');
                    d.push_str(value);
                }
                None => data = Some(value.to_owned()),
            }
        }
    }
    out
}
/// Whether the first `response.completed` is the final event of the bytes
/// the provider sent (`forwarded` then `unforwarded`): after its event block
/// only SSE comment lines and blank lines may follow. Any other line,
/// including a partial one or a second terminal event, is not clean. A hosted
/// attempt completes only on such a clean terminal event (operator decisions
/// K-a and Q-a, 2026-11-17).
pub fn terminal_is_last(forwarded: &[u8], unforwarded: &[u8]) -> bool {
    let mut all = forwarded.to_vec();
    all.extend_from_slice(unforwarded);
    let text = String::from_utf8_lossy(&all)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut terminal_end = None;
    let mut offset = 0;
    for block in text.split_inclusive("\n\n") {
        let data: Vec<&str> = block
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .map(|d| d.strip_prefix(' ').unwrap_or(d))
            .collect();
        offset += block.len();
        if terminal_end.is_none()
            && block.ends_with("\n\n")
            && serde_json::from_str::<Value>(&data.join("\n"))
                .is_ok_and(|v| v["type"] == "response.completed")
        {
            terminal_end = Some(offset);
        }
    }
    let Some(end) = terminal_end else {
        return false;
    };
    let tail = &text[end..];
    // Every tail line is complete and is a comment or blank.
    (tail.is_empty() || tail.ends_with('\n'))
        && tail.lines().all(|l| l.is_empty() || l.starts_with(':'))
}
/// Normalized usage from a Responses SSE `response.completed` event. Missing
/// values remain null; nothing is inferred from HTTP completion.
pub fn usage(sse: &[u8]) -> Value {
    let events: Vec<Value> = sse_data(sse)
        .iter()
        .filter_map(|d| serde_json::from_str::<Value>(d).ok())
        .collect();
    // A provider failure event inside a relayed stream (e.g. quota after
    // output began) is reported, never treated as completion.
    let failed = events.iter().find_map(|v| match v["type"].as_str() {
        Some("response.failed") => Some(
            v["response"]["error"]["code"]
                .as_str()
                .unwrap_or("response_failed")
                .to_owned(),
        ),
        Some("error") => Some(
            v["code"]
                .as_str()
                .or(v["error"]["code"].as_str())
                .unwrap_or("error")
                .to_owned(),
        ),
        _ => None,
    });
    // The first terminal event is the terminal; anything after it is judged
    // by `terminal_is_last`.
    let completed = events.iter().find(|v| v["type"] == "response.completed");
    let Some(v) = completed.filter(|_| failed.is_none()) else {
        return json!({"completed": false, "observed": false, "failed": failed});
    };
    let u = &v["response"]["usage"];
    json!({
        "completed": true,
        "failed": null,
        "observed": u.is_object(),
        "responseId": v["response"]["id"],
        "inputTokens": u["input_tokens"],
        "outputTokens": u["output_tokens"],
        "totalTokens": u["total_tokens"],
        "cachedInputTokens": u["input_tokens_details"]["cached_tokens"],
        "reasoningTokens": u["output_tokens_details"]["reasoning_tokens"],
    })
}

fn frame(kind: &str, b: &Binding, seq: u64, stream: u64, data: Value) -> Value {
    let mut v = json!({"type":kind,"binding":b,"sequence":seq,"stream":stream});
    if let (Some(o), Value::Object(d)) = (v.as_object_mut(), data) {
        o.extend(d);
    }
    v
}

/// Messages from the upstream reader thread.
enum Msg {
    Head(Result<u16>),
    Data(Result<Option<Vec<u8>>>),
}
/// The upstream connection is owned by a dedicated thread: it reads the
/// response head, then one bounded chunk per host request. The host therefore
/// never blocks on the provider and can act on native frames at any time.
struct Reader {
    request: SyncSender<()>,
    result: Receiver<Msg>,
    outstanding: bool,
}
impl Reader {
    fn spawn<R: Read + Send + 'static>(r: R) -> Self {
        let (request, rx) = sync_channel::<()>(1);
        let (tx, result) = sync_channel(1);
        std::thread::spawn(move || {
            let mut up = match Upstream::open(r) {
                Ok(up) => {
                    if tx.send(Msg::Head(Ok(up.status))).is_err() {
                        return;
                    }
                    up
                }
                Err(code) => {
                    let _ = tx.send(Msg::Head(Err(code)));
                    return;
                }
            };
            while rx.recv().is_ok() {
                let r = up.next(CHUNK);
                let last = !matches!(r, Ok(Some(_)));
                if tx.send(Msg::Data(r)).is_err() || last {
                    break;
                }
            }
        });
        Self {
            request,
            result,
            outstanding: false,
        }
    }
    /// Head message, if it arrives within `wait`.
    fn head(&mut self, wait: Duration) -> Option<Result<u16>> {
        match self.result.recv_timeout(wait) {
            Ok(Msg::Head(h)) => Some(h),
            Ok(Msg::Data(_)) => Some(Err("upstream_reader_order")),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => Some(Err("upstream_reader")),
        }
    }
    /// Next body bytes, if they arrive within `wait`. Requests a chunk when
    /// none is outstanding.
    fn data(&mut self, wait: Duration) -> Option<Result<Option<Vec<u8>>>> {
        if !self.outstanding {
            if self.request.send(()).is_err() {
                return Some(Err("upstream_reader"));
            }
            self.outstanding = true;
        }
        match self.result.recv_timeout(wait) {
            Ok(Msg::Data(d)) => {
                self.outstanding = false;
                Some(d)
            }
            Ok(Msg::Head(_)) => Some(Err("upstream_reader_order")),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => Some(Err("upstream_reader")),
        }
    }
}
const POLL: Duration = Duration::from_millis(10);
const DRAIN: Duration = Duration::from_secs(1);
/// Bytes the release drain reads before it stops.
pub const DRAIN_MAX: usize = 64 * 1024;
const MAX_REFUSAL: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Awaiting the upstream head; the consumer may cancel.
    Head,
    /// Head refused by policy: draining its bounded body, nothing relayed.
    Refusal,
    /// Awaiting a native `pull` or `cancel`.
    AwaitOp,
    /// A pull is outstanding; awaiting upstream bytes (or an overtaking cancel).
    Upstream {
        pulled: u64,
        ended_at_pull: bool,
    },
    /// Terminal frame sent; awaiting the single closing `ack` or `cancel`.
    Close,
    /// Consumer released after a terminal provider event; observing (polled,
    /// bounded) whether the upstream framed end follows. No native frames.
    Draining {
        until: Instant,
    },
    Done,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    Pending,
    /// The head status was refused by the session's policy before anything was
    /// relayed. The native consumer is still awaiting a head for this stream;
    /// the caller may start another attempt under the same stream identity or
    /// answer `response-unavailable`.
    Refused {
        status: u16,
        body: Vec<u8>,
    },
    Finished,
}
/// Event-driven host side of one relayed response stream.
///
/// The host loop delivers native frames with [`Session::on_native`] and
/// advances upstream progress with [`Session::poll`]; both return the frames
/// to send to the native consumer. Frames carry one fixed stream identity.
/// Protocol (ordinals count native operations from 1):
/// - head phase: host `response-head`, or `response-failure` (ordinal 0) if
///   the head cannot be read; the consumer may `cancel` (ordinal 1) at any time,
///   confirmed with `response-cancelled` even if a head crossed it.
/// - body: native `pull`/`cancel`; host `response-chunk`, `response-end`,
///   `response-failure` or `response-cancelled`. A cancel may overtake an
///   outstanding pull.
/// - closing: after a terminal frame, exactly one native `ack`, or a crossing
///   `cancel` that the host confirms.
///
/// Dropping an unfinished session closes the upstream connection.
pub struct Session {
    binding: Binding,
    bind: Value,
    seq: u64,
    stream: u64,
    reader: Reader,
    state: State,
    held: Vec<u8>,
    ordinal: u64,
    r: Relay,
    abort: Option<Box<dyn FnOnce() + Send>>,
    refuse: fn(u16) -> bool,
    refusal: (u16, Vec<u8>),
    progress: Progress,
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Some(a) = self.abort.take() {
            a();
        }
    }
}
impl Session {
    /// Start reading `upstream` (an already-sent request's response) on a
    /// dedicated thread. `abort` closes the connection; `refuse` selects head
    /// statuses that end the session as [`Progress::Refused`].
    pub fn start<R: Read + Send + 'static>(
        upstream: R,
        abort: impl FnOnce() + Send + 'static,
        binding: &Binding,
        seq: u64,
        stream: u64,
        refuse: fn(u16) -> bool,
    ) -> Result<Self> {
        Ok(Self {
            bind: serde_json::to_value(binding).map_err(|_| "binding")?,
            binding: binding.clone(),
            seq,
            stream,
            reader: Reader::spawn(upstream),
            state: State::Head,
            held: Vec::new(),
            ordinal: 0,
            r: Relay {
                status: 0,
                outcome: Outcome::Completed,
                forwarded: Vec::new(),
                chunks: Vec::new(),
                upstream_end_ms: None,
                opened_ms: now_millis(),
                unforwarded_after_release: 0,
                unforwarded: Vec::new(),
                drained: false,
                held_dropped: 0,
                drain_error: None,
                cancel_wait_ms: None,
                closing: None,
            },
            abort: Some(Box::new(abort)),
            refuse,
            refusal: (0, Vec::new()),
            progress: Progress::Pending,
        })
    }
    pub fn progress(&self) -> &Progress {
        &self.progress
    }
    pub fn relay(&self) -> &Relay {
        &self.r
    }
    /// Whether the session is idle until a native frame arrives.
    pub fn awaiting_native(&self) -> bool {
        matches!(self.state, State::AwaitOp | State::Close)
    }
    /// Whether a native frame may belong to this stream now. False once the
    /// consumer has finished with it (draining or done): later native frames
    /// belong to the session loop.
    pub fn accepts_native(&self) -> bool {
        !matches!(self.state, State::Draining { .. } | State::Done)
    }
    fn release_done(&mut self) {
        self.r.outcome = Outcome::Cancelled {
            upstream_end: self.r.upstream_end_ms.is_some() && self.held.is_empty(),
        };
        self.finish();
    }
    /// End the release drain: screen what it read like relayed bytes. Access
    /// material drops the bytes and leaves the stream interrupted; a held-back
    /// suffix (a possible access-material prefix) is dropped and counted.
    fn drain_done<S: Screen>(&mut self, screen: &S) {
        self.r.drained = true;
        match screen.hold(&self.r.unforwarded) {
            Ok(h) => {
                let keep = self.r.unforwarded.len() - h.min(self.r.unforwarded.len());
                self.r.held_dropped += self.r.unforwarded.len() - keep;
                self.r.unforwarded.truncate(keep);
                self.release_done();
            }
            Err(code) => {
                self.r.held_dropped += std::mem::take(&mut self.r.unforwarded).len();
                self.r.outcome = Outcome::Interrupted(code);
                self.finish();
            }
        }
    }
    pub fn into_relay(mut self) -> Relay {
        // Bytes an unfinished drain read were never screened, and bytes the
        // screen held back were never relayed: neither is kept or digested.
        if !self.r.drained {
            self.r.held_dropped += std::mem::take(&mut self.r.unforwarded).len();
        }
        self.r.held_dropped += std::mem::take(&mut self.held).len();
        std::mem::replace(
            &mut self.r,
            Relay {
                status: 0,
                outcome: Outcome::Completed,
                forwarded: Vec::new(),
                chunks: Vec::new(),
                upstream_end_ms: None,
                opened_ms: 0,
                unforwarded_after_release: 0,
                unforwarded: Vec::new(),
                drained: false,
                held_dropped: 0,
                drain_error: None,
                cancel_wait_ms: None,
                closing: None,
            },
        )
    }
    fn out(&self, kind: &str, data: Value) -> Value {
        frame(kind, &self.binding, self.seq, self.stream, data)
    }
    fn finish(&mut self) {
        // Bytes still held back by the screen are never retained.
        self.r.held_dropped += std::mem::take(&mut self.held).len();
        self.state = State::Done;
        if self.progress == Progress::Pending {
            self.progress = Progress::Finished;
        }
        if self.r.upstream_end_ms.is_some() {
            self.abort = None;
        } else if let Some(a) = self.abort.take() {
            a();
        }
    }
    fn terminal(&mut self, frames: &mut Vec<Value>, kind: &str, data: Value) {
        frames.push(self.out(kind, data));
        self.state = State::Close;
    }
    /// Handle one native frame for this stream.
    pub fn on_native<S: Screen>(&mut self, op: &Value, _screen: &S) -> Result<Vec<Value>> {
        if op["binding"] != self.bind || op["sequence"] != self.seq || op["stream"] != self.stream {
            return Err("stream_foreign_frame");
        }
        if op["ordinal"] != self.ordinal + 1 {
            return Err("stream_foreign_frame");
        }
        self.ordinal += 1;
        let kind = op["type"].as_str().unwrap_or("");
        let mut frames = vec![];
        match (self.state, kind) {
            (State::Head | State::Refusal, "cancel") => {
                // Cancelled before any head was relayed.
                frames.push(self.out("response-cancelled", json!({"ordinal":self.ordinal})));
                self.r.outcome = Outcome::Cancelled {
                    upstream_end: false,
                };
                self.finish();
            }
            (State::AwaitOp, "pull") => {
                self.state = State::Upstream {
                    pulled: now_millis(),
                    ended_at_pull: self.r.upstream_end_ms.is_some(),
                };
                frames.extend(self.advance(_screen, Duration::ZERO)?);
            }
            (State::AwaitOp, "cancel") => {
                frames.push(self.out("response-cancelled", json!({"ordinal":self.ordinal})));
                // Common release right after a terminal provider event: observe,
                // without blocking, whether the framed end follows within a
                // short bound. The stream is already over for the consumer.
                // Bytes the screen held back are provider bytes after the
                // forwarded ones: the drain screens them with what follows.
                let u = usage(&self.r.forwarded);
                if self.r.upstream_end_ms.is_none()
                    && (u["completed"] == true || !u["failed"].is_null())
                {
                    self.r.unforwarded = std::mem::take(&mut self.held);
                    self.state = State::Draining {
                        until: Instant::now() + DRAIN,
                    };
                } else {
                    self.release_done();
                }
            }
            (State::Upstream { pulled, .. }, "cancel") => {
                // Overtakes the outstanding pull, which this confirmation answers.
                self.r.cancel_wait_ms = Some(now_millis().saturating_sub(pulled));
                frames.push(self.out("response-cancelled", json!({"ordinal":self.ordinal})));
                self.r.outcome = Outcome::Cancelled {
                    upstream_end: false,
                };
                self.finish();
            }
            (State::Close, "ack") => {
                self.r.closing = Some("ack");
                self.finish();
            }
            (State::Close, "cancel") => {
                frames.push(self.out("response-cancelled", json!({"ordinal":self.ordinal})));
                self.r.closing = Some("cancel-after-terminal");
                self.finish();
            }
            (State::Head | State::Refusal, _) => return Err("stream_frame_before_head"),
            _ => return Err("stream_interleaved_frame"),
        }
        Ok(frames)
    }
    /// Advance upstream-driven work, waiting at most `wait` for the provider.
    pub fn poll<S: Screen>(&mut self, screen: &S, wait: Duration) -> Result<Vec<Value>> {
        match self.state {
            State::Head => {
                let mut frames = vec![];
                match self.reader.head(wait) {
                    None => {}
                    Some(Ok(status)) => {
                        self.r.status = status;
                        if (self.refuse)(status) {
                            self.refusal.0 = status;
                            self.state = State::Refusal;
                        } else {
                            frames.push(self.out("response-head", json!({"status":status})));
                            self.state = State::AwaitOp;
                        }
                    }
                    Some(Err(code)) => {
                        self.r.outcome = Outcome::Interrupted(code);
                        self.terminal(
                            &mut frames,
                            "response-failure",
                            json!({"ordinal":self.ordinal,"code":code}),
                        );
                    }
                }
                Ok(frames)
            }
            State::Refusal => {
                match self.reader.data(wait) {
                    None => {}
                    Some(Ok(Some(b))) => {
                        self.refusal.1.extend(b);
                        if self.refusal.1.len() > MAX_REFUSAL {
                            return Err("refusal_bound");
                        }
                    }
                    Some(Ok(None)) => {
                        self.r.upstream_end_ms = Some(now_millis());
                        screen.hold(&self.refusal.1)?;
                        let (status, body) = std::mem::take(&mut self.refusal);
                        self.progress = Progress::Refused { status, body };
                        self.finish();
                    }
                    Some(Err(code)) => {
                        // A refusal whose body cannot be read is not definitive.
                        self.r.outcome = Outcome::Interrupted(code);
                        let mut frames = vec![];
                        self.terminal(
                            &mut frames,
                            "response-failure",
                            json!({"ordinal":self.ordinal,"code":code}),
                        );
                        return Ok(frames);
                    }
                }
                Ok(vec![])
            }
            State::Upstream { .. } => self.advance(screen, wait),
            State::Draining { until } => {
                // Accumulate until the framed end, the time bound or the size
                // bound. At the time bound one last non-blocking read takes a
                // chunk that is already waiting.
                let now = Instant::now();
                let left = until.saturating_duration_since(now).min(wait);
                match self.reader.data(left) {
                    Some(Ok(None)) => {
                        self.r.upstream_end_ms = Some(now_millis());
                        self.drain_done(screen);
                    }
                    Some(Ok(Some(b))) => {
                        self.r.unforwarded_after_release += b.len();
                        self.r.unforwarded.extend(b);
                        if self.r.unforwarded.len() >= DRAIN_MAX || Instant::now() >= until {
                            self.drain_done(screen);
                        }
                    }
                    Some(Err(code)) => {
                        self.r.drain_error = Some(code);
                        self.drain_done(screen);
                    }
                    None if now >= until => self.drain_done(screen),
                    None => {}
                }
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }
    /// Satisfy the outstanding pull if upstream bytes are releasable.
    fn advance<S: Screen>(&mut self, screen: &S, wait: Duration) -> Result<Vec<Value>> {
        let State::Upstream {
            pulled,
            ended_at_pull,
        } = self.state
        else {
            return Ok(vec![]);
        };
        let mut frames = vec![];
        let deadline = Instant::now() + wait;
        let release = loop {
            if self.r.upstream_end_ms.is_some() {
                break std::mem::take(&mut self.held);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            match self.reader.data(left) {
                None => return Ok(frames),
                Some(Ok(Some(b))) => {
                    self.held.extend(b);
                    if self.r.forwarded.len() + self.held.len() > MAX_RESPONSE {
                        self.r.outcome = Outcome::Interrupted("upstream_response_bound");
                        break Vec::new();
                    }
                    match screen.hold(&self.held) {
                        Ok(h) if self.held.len() > h => {
                            let rest = self.held.split_off(self.held.len() - h);
                            break std::mem::replace(&mut self.held, rest);
                        }
                        Ok(_) => {}
                        Err(code) => {
                            self.r.outcome = Outcome::Interrupted(code);
                            break Vec::new();
                        }
                    }
                }
                Some(Ok(None)) => self.r.upstream_end_ms = Some(now_millis()),
                Some(Err(code)) => {
                    self.r.outcome = Outcome::Interrupted(code);
                    break Vec::new();
                }
            }
        };
        if let Outcome::Interrupted(code) = self.r.outcome {
            self.terminal(
                &mut frames,
                "response-failure",
                json!({"ordinal":self.ordinal,"code":code}),
            );
        } else if release.is_empty() {
            self.terminal(&mut frames, "response-end", json!({"ordinal":self.ordinal}));
        } else {
            frames.push(self.out(
                "response-chunk",
                json!({"ordinal":self.ordinal,"body":B64.encode(&release)}),
            ));
            self.r.chunks.push((
                self.ordinal,
                release.len(),
                pulled,
                now_millis(),
                ended_at_pull,
            ));
            self.r.forwarded.extend(release);
            self.state = State::AwaitOp;
        }
        Ok(frames)
    }
}

pub enum Relayed {
    Stream(Relay),
    Refused { status: u16, body: Vec<u8> },
}
/// Drive one [`Session`] to completion over a blocking channel. Native frames
/// are read only when available, so a cancel is acted on while the provider
/// is still silent (head or body).
pub fn relay<C: Channel + Screen, R: Read + Send + 'static>(
    chan: &mut C,
    upstream: R,
    abort: impl FnOnce() + Send + 'static,
    binding: &Binding,
    seq: u64,
    stream: u64,
    refuse: fn(u16) -> bool,
) -> Result<Relayed> {
    let mut s = Session::start(upstream, abort, binding, seq, stream, refuse)?;
    loop {
        for f in s.poll(chan, POLL)? {
            chan.send(&f)?;
        }
        match s.progress().clone() {
            Progress::Finished => return Ok(Relayed::Stream(s.into_relay())),
            Progress::Refused { status, body } => return Ok(Relayed::Refused { status, body }),
            Progress::Pending => {}
        }
        if s.awaiting_native() || (s.accepts_native() && chan.readable()?) {
            let op = chan.receive()?;
            for f in s.on_native(&op, chan)? {
                chan.send(&f)?;
            }
            if let Progress::Finished = s.progress() {
                return Ok(Relayed::Stream(s.into_relay()));
            }
        }
    }
}

/// Answer a stream with no permitted provider attempt. The consumer may have
/// sent a head-phase `cancel(1)` before reading this answer; the host must
/// offer its next native frame to [`Unavailable::on_next`], which confirms
/// such a crossing cancel and otherwise hands the frame back.
pub struct Unavailable {
    binding: Binding,
    bind: Value,
    seq: u64,
    stream: u64,
}
impl Unavailable {
    pub fn answer(binding: &Binding, seq: u64, stream: u64, code: &str) -> Result<(Self, Value)> {
        let bind = serde_json::to_value(binding).map_err(|_| "binding")?;
        Ok((
            Self {
                binding: binding.clone(),
                bind,
                seq,
                stream,
            },
            frame(
                "response-unavailable",
                binding,
                seq,
                stream,
                json!({"code":code}),
            ),
        ))
    }
    /// `Ok(Some(confirmation))` for the crossing cancel, `Ok(None)` for a
    /// frame that belongs to the session loop, error for any other frame of
    /// this stream.
    pub fn on_next(&self, op: &Value) -> Result<Option<Value>> {
        // Session envelopes (snapshot, tool, fetch, ...) carry the native's
        // current stream counter too; only stream-protocol frames are judged.
        if !matches!(op["type"].as_str(), Some("cancel" | "pull" | "ack")) {
            return Ok(None);
        }
        if op["binding"] != self.bind || op["sequence"] != self.seq || op["stream"] != self.stream {
            return Err("stream_foreign_frame");
        }
        if op["type"] == "cancel" && op["ordinal"] == 1 {
            return Ok(Some(frame(
                "response-cancelled",
                &self.binding,
                self.seq,
                self.stream,
                json!({"ordinal":1}),
            )));
        }
        Err("stream_interleaved_frame")
    }
}

/// Last check after admission and before any upstream byte is written: a
/// head-phase `cancel(1)` that arrived meanwhile is confirmed here, and the
/// attempt is then known not to have been dispatched. Returns `true` when
/// cancelled. Any other native frame while the fetch awaits its head is a
/// protocol error.
pub fn pre_dispatch<C: Channel>(
    chan: &mut C,
    binding: &Binding,
    seq: u64,
    stream: u64,
) -> Result<bool> {
    if !chan.readable()? {
        return Ok(false);
    }
    let op = chan.receive()?;
    let bind = serde_json::to_value(binding).map_err(|_| "binding")?;
    if op["binding"] != bind || op["sequence"] != seq || op["stream"] != stream {
        return Err("stream_foreign_frame");
    }
    if op["type"] != "cancel" || op["ordinal"] != 1 {
        return Err("stream_interleaved_frame");
    }
    chan.send(&frame(
        "response-cancelled",
        binding,
        seq,
        stream,
        json!({"ordinal":1}),
    ))?;
    Ok(true)
}
