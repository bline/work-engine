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

pub type Result<T> = std::result::Result<T, &'static str>;
pub const CHUNK: usize = 16 * 1024;
pub const MAX_RESPONSE: usize = 1024 * 1024;
const MAX_HEAD: usize = 8192;

/// Framed native endpoint; the probe wrapper records every received frame.
pub trait Channel {
    fn send(&mut self, v: &Value) -> Result<()>;
    fn receive(&mut self) -> Result<Value>;
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
            "usage": usage(&self.forwarded),
        })
    }
}

/// Normalized usage from a Responses SSE `response.completed` event. Missing
/// values remain null; nothing is inferred from HTTP completion.
pub fn usage(sse: &[u8]) -> Value {
    let text = String::from_utf8_lossy(sse);
    let completed = text
        .split("\n\n")
        .filter_map(|e| e.lines().find_map(|l| l.strip_prefix("data: ")))
        .filter_map(|d| serde_json::from_str::<Value>(d).ok())
        .filter(|v| v["type"] == "response.completed")
        .last();
    let Some(v) = completed else {
        return json!({"observed": false});
    };
    let u = &v["response"]["usage"];
    json!({
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

/// Relay an opened upstream response to the native consumer under the given
/// stream identity. Transport/protocol failures toward the native side are
/// returned as errors; upstream failures after the head are reported to the
/// consumer and returned as `Outcome::Interrupted`.
pub fn relay<C: Channel + Screen, R: Read>(
    chan: &mut C,
    upstream: &mut Upstream<R>,
    binding: &Binding,
    seq: u64,
    stream: u64,
) -> Result<Relay> {
    let opened_ms = now_millis();
    let mut r = Relay {
        status: upstream.status,
        outcome: Outcome::Completed,
        forwarded: Vec::new(),
        chunks: Vec::new(),
        upstream_end_ms: None,
        opened_ms,
        unforwarded_after_release: 0,
    };
    chan.send(&frame(
        "response-head",
        binding,
        seq,
        stream,
        json!({"status":upstream.status}),
    ))?;
    let mut held: Vec<u8> = Vec::new();
    let mut ordinal = 0u64;
    loop {
        let op = chan.receive()?;
        ordinal += 1;
        if op["binding"] != serde_json::to_value(binding).map_err(|_| "binding")?
            || op["sequence"] != seq
            || op["stream"] != stream
            || op["ordinal"] != ordinal
        {
            return Err("stream_foreign_frame");
        }
        let pulled = now_millis();
        let ended_at_pull = r.upstream_end_ms.is_some();
        match op["type"].as_str() {
            Some("cancel") => {
                chan.send(&frame(
                    "response-cancelled",
                    binding,
                    seq,
                    stream,
                    json!({"ordinal":ordinal}),
                ))?;
                // A consumer commonly releases right after the terminal
                // provider event. Only then, observe whether the framed end
                // follows; unforwarded bytes keep the end unobserved.
                if r.upstream_end_ms.is_none()
                    && held.is_empty()
                    && usage(&r.forwarded)["observed"] == true
                {
                    match upstream.next(CHUNK) {
                        Ok(None) => r.upstream_end_ms = Some(now_millis()),
                        Ok(Some(b)) => r.unforwarded_after_release = b.len(),
                        Err(_) => {}
                    }
                }
                r.outcome = Outcome::Cancelled {
                    upstream_end: r.upstream_end_ms.is_some() && held.is_empty(),
                };
                return Ok(r);
            }
            Some("pull") => {}
            _ => return Err("stream_interleaved_frame"),
        }
        // Read until something is releasable, the upstream ends, or it fails.
        let release = loop {
            if r.upstream_end_ms.is_some() {
                break std::mem::take(&mut held);
            }
            match upstream.next(CHUNK) {
                Ok(Some(b)) => {
                    held.extend(b);
                    if r.forwarded.len() + held.len() > MAX_RESPONSE {
                        r.outcome = Outcome::Interrupted("upstream_response_bound");
                        break Vec::new();
                    }
                    let hold = match chan.hold(&held) {
                        Ok(h) => h,
                        Err(code) => {
                            r.outcome = Outcome::Interrupted(code);
                            break Vec::new();
                        }
                    };
                    if held.len() > hold {
                        let rest = held.split_off(held.len() - hold);
                        break std::mem::replace(&mut held, rest);
                    }
                }
                Ok(None) => r.upstream_end_ms = Some(now_millis()),
                Err(code) => {
                    r.outcome = Outcome::Interrupted(code);
                    break Vec::new();
                }
            }
        };
        if let Outcome::Interrupted(code) = r.outcome {
            chan.send(&frame(
                "response-failure",
                binding,
                seq,
                stream,
                json!({"ordinal":ordinal,"code":code}),
            ))?;
            return Ok(r);
        }
        if release.is_empty() {
            chan.send(&frame(
                "response-end",
                binding,
                seq,
                stream,
                json!({"ordinal":ordinal}),
            ))?;
            return Ok(r);
        }
        chan.send(&frame(
            "response-chunk",
            binding,
            seq,
            stream,
            json!({"ordinal":ordinal,"body":B64.encode(&release)}),
        ))?;
        r.chunks
            .push((ordinal, release.len(), pulled, now_millis(), ended_at_pull));
        r.forwarded.extend(release);
    }
}
