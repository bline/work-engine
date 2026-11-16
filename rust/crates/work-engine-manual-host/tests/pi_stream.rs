use base64::{Engine, engine::general_purpose::STANDARD as B64};
use review_execution_evidence::pi::{Binding, LiveStore};
use serde_json::{Value, json};
use std::{
    io::{Cursor, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::Duration,
};
use work_engine_manual_host::pi_stream::{self as ps, Channel, Outcome, Upstream};
const A: &str = "SYNTHETIC_ORIGINAL_A_MUST_NEVER_BE_RETAINED";
fn binding() -> Binding {
    Binding::native(
        "11111111-1111-4111-8111-111111111111".into(),
        "test".into(),
        1,
    )
}
fn store(t: &tempfile::TempDir) -> LiveStore {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    LiveStore::create(
        &t.path().join(format!("store{n}")),
        A.as_bytes().to_vec(),
        binding(),
        u64::MAX,
    )
    .unwrap()
}
/// Scripted native consumer: pulls until `cancel_at` (then cancels).
struct Consumer {
    s: LiveStore,
    b: Binding,
    ordinal: u64,
    cancel_at: Option<u64>,
    sent: Vec<Value>,
    body: Vec<u8>,
    on_chunk: Option<mpsc::Sender<()>>,
    foreign: bool,
    /// Cancel while the pull with this ordinal is outstanding, after a delay.
    overtake: Option<(u64, std::time::Instant)>,
}
impl Consumer {
    fn new(s: LiveStore) -> Self {
        Self {
            s,
            b: binding(),
            ordinal: 0,
            cancel_at: None,
            sent: vec![],
            body: vec![],
            on_chunk: None,
            foreign: false,
            overtake: None,
        }
    }
}
impl ps::Screen for Consumer {
    fn hold(&self, p: &[u8]) -> ps::Result<usize> {
        self.s.stream_hold(p)
    }
}
impl Channel for Consumer {
    fn readable(&mut self) -> ps::Result<bool> {
        Ok(match self.overtake {
            Some((n, at)) if self.ordinal == n && std::time::Instant::now() >= at => {
                self.cancel_at = Some(n + 1);
                true
            }
            _ => false,
        })
    }
    fn send(&mut self, v: &Value) -> ps::Result<()> {
        if v["type"] == "response-chunk" {
            self.body
                .extend(B64.decode(v["body"].as_str().unwrap()).unwrap());
            if let Some(tx) = &self.on_chunk {
                let _ = tx.send(());
            }
        }
        self.sent.push(v.clone());
        Ok(())
    }
    fn receive(&mut self) -> ps::Result<Value> {
        self.ordinal += 1;
        let terminal = self
            .sent
            .last()
            .is_some_and(|v| v["type"] == "response-end" || v["type"] == "response-failure");
        let kind = if self.cancel_at == Some(self.ordinal) {
            "cancel"
        } else if terminal {
            "ack"
        } else {
            "pull"
        };
        if let Some((n, _)) = self.overtake {
            if self.ordinal == n {
                self.overtake = Some((n, std::time::Instant::now() + Duration::from_millis(200)));
            }
        }
        let seq = if self.foreign { 9 } else { 1 };
        Ok(json!({"type":kind,"binding":self.b,"sequence":seq,"stream":1,"ordinal":self.ordinal}))
    }
}
fn kinds(c: &Consumer) -> Vec<&str> {
    c.sent.iter().map(|v| v["type"].as_str().unwrap()).collect()
}
fn sse(events: &[&str]) -> String {
    events.iter().map(|e| format!("data: {e}\n\n")).collect()
}

#[test]
fn early_chunk_is_forwarded_while_upstream_is_still_open() {
    let t = tempfile::tempdir().unwrap();
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut c, _) = l.accept().unwrap();
        let first = sse(&[r#"{"type":"response.created"}"#]);
        write!(
            c,
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{first}\r\n",
            first.len()
        )
        .unwrap();
        c.flush().unwrap();
        // The endpoint withholds the remainder until the consumer received chunk 1.
        rx.recv_timeout(Duration::from_secs(10))
            .expect("early chunk not forwarded");
        let rest = sse(&[
            r#"{"type":"response.completed","response":{"id":"r1","usage":{"input_tokens":5,"output_tokens":2,"total_tokens":7,"input_tokens_details":{"cached_tokens":3}}}}"#,
        ]);
        write!(c, "{:x}\r\n{rest}\r\n0\r\n\r\n", rest.len()).unwrap();
    });
    let up = Upstream::open(TcpStream::connect(addr).unwrap()).unwrap();
    let mut c = Consumer::new(store(&t));
    c.on_chunk = Some(tx);
    let r = ps::relay(&mut c, up, || (), &binding(), 1, 1).unwrap();
    server.join().unwrap();
    assert_eq!(r.outcome, Outcome::Completed);
    assert_eq!(
        kinds(&c),
        [
            "response-head",
            "response-chunk",
            "response-chunk",
            "response-end"
        ]
    );
    assert!(r.pulls_before_upstream_end() >= 1);
    let e = r.evidence();
    assert_eq!(e["usage"]["cachedInputTokens"], 3);
    assert_eq!(e["usage"]["totalTokens"], 7);
    assert!(
        String::from_utf8(c.body)
            .unwrap()
            .contains("response.completed")
    );
}

#[test]
fn split_original_a_is_never_forwarded() {
    let t = tempfile::tempdir().unwrap();
    // A arrives split across two upstream reads; its prefix must be withheld.
    let (p1, p2) = A.split_at(10);
    let body = format!("data: x{p1}");
    let tail = format!("{p2}\n\n");
    let raw = format!(
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{body}\r\n{:x}\r\n{tail}\r\n0\r\n\r\n",
        body.len(),
        tail.len()
    );
    let up = Upstream::open(Cursor::new(raw.into_bytes())).unwrap();
    let mut c = Consumer::new(store(&t));
    let r = ps::relay(&mut c, up, || (), &binding(), 1, 1).unwrap();
    assert_eq!(r.outcome, Outcome::Interrupted("pi_original_a_exclusion"));
    assert_eq!(c.body, b"data: x");
    assert_eq!(
        kinds(&c),
        ["response-head", "response-chunk", "response-failure"]
    );
}

#[test]
fn harmless_prefix_tail_is_released_at_end() {
    let t = tempfile::tempdir().unwrap();
    let raw = "HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\nabc SYNT".to_string();
    let up = Upstream::open(Cursor::new(raw.into_bytes())).unwrap();
    let mut c = Consumer::new(store(&t));
    let r = ps::relay(&mut c, up, || (), &binding(), 1, 1).unwrap();
    assert_eq!(r.outcome, Outcome::Completed);
    assert_eq!(c.body, b"abc SYNT");
    assert_eq!(
        kinds(&c),
        [
            "response-head",
            "response-chunk",
            "response-chunk",
            "response-end"
        ]
    );
}

#[test]
fn truncated_and_malformed_upstream_are_interrupted_not_completed() {
    let t = tempfile::tempdir().unwrap();
    for (raw, code) in [
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\nshort",
            "upstream_truncated",
        ),
        (
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n",
            "upstream_chunk_size",
        ),
        (
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhel",
            "upstream_truncated",
        ),
    ] {
        let up = Upstream::open(Cursor::new(raw.as_bytes().to_vec())).unwrap();
        let mut c = Consumer::new(store(&t));
        let r = ps::relay(&mut c, up, || (), &binding(), 1, 1).unwrap();
        assert_eq!(r.outcome, Outcome::Interrupted(code), "{raw}");
        assert_eq!(kinds(&c).last(), Some(&"response-failure"));
    }
    for (raw, code) in [
        ("HTTP/1.0 200 OK\r\n\r\n", "upstream_version"),
        (
            "HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\n\r\n",
            "upstream_content_encoding",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\n",
            "upstream_ambiguous_framing",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\n",
            "upstream_length",
        ),
        ("HTTP/1.1 200 OK\r\n", "upstream_head_eof"),
    ] {
        assert_eq!(
            Upstream::open(Cursor::new(raw.as_bytes().to_vec())).err(),
            Some(code),
            "{raw}"
        );
    }
}

#[test]
fn cancel_closes_without_reading_further_and_records_upstream_state() {
    let t = tempfile::tempdir().unwrap();
    let raw = format!("HTTP/1.1 200 OK\r\n\r\n{}", "x".repeat(ps::CHUNK * 3));
    let up = Upstream::open(Cursor::new(raw.into_bytes())).unwrap();
    let mut c = Consumer::new(store(&t));
    c.cancel_at = Some(2);
    let r = ps::relay(&mut c, up, || (), &binding(), 1, 1).unwrap();
    assert_eq!(
        r.outcome,
        Outcome::Cancelled {
            upstream_end: false
        }
    );
    assert!(r.forwarded.len() > 0 && r.forwarded.len() <= ps::CHUNK);
    assert_eq!(
        kinds(&c),
        ["response-head", "response-chunk", "response-cancelled"]
    );
    assert_eq!(r.evidence()["usage"]["observed"], false);
}

#[test]
fn slow_reader_bounds_host_buffer_to_one_chunk_per_pull() {
    let t = tempfile::tempdir().unwrap();
    let raw = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        ps::CHUNK * 4,
        "y".repeat(ps::CHUNK * 4)
    );
    let up = Upstream::open(Cursor::new(raw.into_bytes())).unwrap();
    let mut c = Consumer::new(store(&t));
    let r = ps::relay(&mut c, up, || (), &binding(), 1, 1).unwrap();
    assert_eq!(r.outcome, Outcome::Completed);
    assert!(r.chunks.iter().all(|x| x.1 <= ps::CHUNK));
    assert!(r.chunks.len() >= 4);
    assert_eq!(r.forwarded.len(), ps::CHUNK * 4);
}

#[test]
fn foreign_or_interleaved_native_frames_refuse() {
    let t = tempfile::tempdir().unwrap();
    let up = Upstream::open(Cursor::new(b"HTTP/1.1 200 OK\r\n\r\nx".to_vec())).unwrap();
    let mut c = Consumer::new(store(&t));
    c.foreign = true;
    assert_eq!(
        ps::relay(&mut c, up, || (), &binding(), 1, 1).err(),
        Some("stream_foreign_frame")
    );
    struct Tool(Binding, LiveStore);
    impl ps::Screen for Tool {
        fn hold(&self, p: &[u8]) -> ps::Result<usize> {
            self.1.stream_hold(p)
        }
    }
    impl Channel for Tool {
        fn send(&mut self, _: &Value) -> ps::Result<()> {
            Ok(())
        }
        fn readable(&mut self) -> ps::Result<bool> {
            Ok(false)
        }
        fn receive(&mut self) -> ps::Result<Value> {
            Ok(json!({"type":"tool","binding":self.0,"sequence":1,"stream":1,"ordinal":1}))
        }
    }
    let up = Upstream::open(Cursor::new(b"HTTP/1.1 200 OK\r\n\r\nx".to_vec())).unwrap();
    assert_eq!(
        ps::relay(&mut Tool(binding(), store(&t)), up, || (), &binding(), 1, 1).err(),
        Some("stream_interleaved_frame")
    );
}

#[test]
fn prefix_suffix_is_exact() {
    use review_execution_evidence::pi::prefix_suffix;
    assert_eq!(prefix_suffix(b"zzabc", b"abcd"), 3);
    assert_eq!(prefix_suffix(b"zzabcd", b"abcd"), 0);
    assert_eq!(prefix_suffix(b"zz", b"abcd"), 0);
    assert_eq!(prefix_suffix(b"a", b"a"), 0);
    assert_eq!(prefix_suffix(b"aa", b"aab"), 2);
}

#[test]
fn cancel_overtakes_pending_pull_and_aborts_stalled_upstream() {
    let t = tempfile::tempdir().unwrap();
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut c, _) = l.accept().unwrap();
        let first = sse(&[r#"{"type":"response.created"}"#]);
        write!(
            c,
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{first}\r\n",
            first.len()
        )
        .unwrap();
        // Stall; the host must close the connection on cancel.
        let mut b = [0u8; 1];
        let started = std::time::Instant::now();
        let n = std::io::Read::read(&mut c, &mut b).unwrap_or(0);
        (n, started.elapsed())
    });
    let socket = TcpStream::connect(addr).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    let killer = socket.try_clone().unwrap();
    let up = Upstream::open(socket).unwrap();
    let mut c = Consumer::new(store(&t));
    c.overtake = Some((2, std::time::Instant::now() + Duration::from_secs(3600)));
    let start = std::time::Instant::now();
    let r = ps::relay(
        &mut c,
        up,
        move || {
            let _ = killer.shutdown(std::net::Shutdown::Both);
        },
        &binding(),
        1,
        1,
    )
    .unwrap();
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "cancel not prompt"
    );
    assert_eq!(
        r.outcome,
        Outcome::Cancelled {
            upstream_end: false
        }
    );
    assert!(r.cancel_wait_ms.is_some());
    assert_eq!(
        kinds(&c),
        ["response-head", "response-chunk", "response-cancelled"]
    );
    assert_eq!(c.sent.last().unwrap()["ordinal"], 3);
    let (n, waited) = server.join().unwrap();
    assert_eq!(n, 0, "upstream connection closed by host");
    assert!(waited < Duration::from_secs(3));
}

#[test]
fn crlf_sse_completion_is_recognized_for_release_after_end() {
    let t = tempfile::tempdir().unwrap();
    let body = "event: response.created\r\ndata: {\"type\":\"response.created\"}\r\n\r\n\
                data: {\"type\":\"response.output_text.delta\",\r\ndata: \"delta\":\"x\"}\r\n\r\n\
                : comment\r\ndata:{\"type\":\"response.completed\",\"response\":{\"id\":\"r9\",\"usage\":{\"input_tokens\":4}}}\r\n\r\n";
    let raw = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let up = Upstream::open(Cursor::new(raw.into_bytes())).unwrap();
    let mut c = Consumer::new(store(&t));
    c.cancel_at = Some(2);
    let r = ps::relay(&mut c, up, || (), &binding(), 1, 1).unwrap();
    assert_eq!(r.outcome, Outcome::Cancelled { upstream_end: true });
    let u = r.evidence()["usage"].clone();
    assert_eq!(u["completed"], true);
    assert_eq!(u["inputTokens"], 4);
    assert_eq!(u["outputTokens"], Value::Null);
    let data = ps::sse_data(body.as_bytes());
    assert_eq!(data.len(), 3);
    assert_eq!(
        data[1],
        "{\"type\":\"response.output_text.delta\",\n\"delta\":\"x\"}"
    );
    // CR-only line endings and an undispatched trailing event.
    assert_eq!(ps::sse_data(b"data: a\r\rdata: b\r"), vec!["a".to_string()]);
}

#[test]
fn failed_terminal_event_is_reported_and_end_observed_after_release() {
    let t = tempfile::tempdir().unwrap();
    let body = sse(&[
        r#"{"type":"response.created"}"#,
        r#"{"type":"response.failed","response":{"error":{"code":"insufficient_quota"}}}"#,
    ]);
    let raw = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let up = Upstream::open(Cursor::new(raw.into_bytes())).unwrap();
    let mut c = Consumer::new(store(&t));
    c.cancel_at = Some(2);
    let r = ps::relay(&mut c, up, || (), &binding(), 1, 1).unwrap();
    assert_eq!(r.outcome, Outcome::Cancelled { upstream_end: true });
    let u = r.evidence()["usage"].clone();
    assert_eq!(u["completed"], false);
    assert_eq!(u["failed"], "insufficient_quota");
}

#[test]
fn cancel_racing_a_terminal_frame_is_confirmed_not_left_waiting() {
    let t = tempfile::tempdir().unwrap();
    let raw = "HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc".to_string();
    let up = Upstream::open(Cursor::new(raw.into_bytes())).unwrap();
    let mut c = Consumer::new(store(&t));
    // Pulls 1 (chunk) and 2 (end); the consumer's cancel 3 crossed the end.
    c.cancel_at = Some(3);
    let r = ps::relay(&mut c, up, || (), &binding(), 1, 1).unwrap();
    assert_eq!(r.outcome, Outcome::Completed);
    assert_eq!(r.closing, Some("cancel-after-terminal"));
    assert_eq!(
        kinds(&c),
        [
            "response-head",
            "response-chunk",
            "response-end",
            "response-cancelled"
        ]
    );
    assert_eq!(c.sent.last().unwrap()["ordinal"], 3);
    // Normal close: exactly one ack, nothing further sent.
    let up = Upstream::open(Cursor::new(
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx".to_vec(),
    ))
    .unwrap();
    let mut c = Consumer::new(store(&t));
    let r = ps::relay(&mut c, up, || (), &binding(), 1, 1).unwrap();
    assert_eq!(r.closing, Some("ack"));
    assert_eq!(
        kinds(&c),
        ["response-head", "response-chunk", "response-end"]
    );
}
