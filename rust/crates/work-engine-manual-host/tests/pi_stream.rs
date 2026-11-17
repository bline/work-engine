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
fn stream(r: ps::Result<ps::Relayed>) -> ps::Result<ps::Relay> {
    r.map(|x| match x {
        ps::Relayed::Stream(r) => r,
        ps::Relayed::Refused { .. } => panic!("refused"),
    })
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
    let up = TcpStream::connect(addr).unwrap();
    let mut c = Consumer::new(store(&t));
    c.on_chunk = Some(tx);
    let r = stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).unwrap();
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
    let up = Cursor::new(raw.into_bytes());
    let mut c = Consumer::new(store(&t));
    let r = stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).unwrap();
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
    let up = Cursor::new(raw.into_bytes());
    let mut c = Consumer::new(store(&t));
    let r = stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).unwrap();
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
        let up = Cursor::new(raw.as_bytes().to_vec());
        let mut c = Consumer::new(store(&t));
        let r = stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).unwrap();
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
    let up = Cursor::new(raw.into_bytes());
    let mut c = Consumer::new(store(&t));
    c.cancel_at = Some(2);
    let r = stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).unwrap();
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
    let up = Cursor::new(raw.into_bytes());
    let mut c = Consumer::new(store(&t));
    let r = stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).unwrap();
    assert_eq!(r.outcome, Outcome::Completed);
    assert!(r.chunks.iter().all(|x| x.1 <= ps::CHUNK));
    assert!(r.chunks.len() >= 4);
    assert_eq!(r.forwarded.len(), ps::CHUNK * 4);
}

#[test]
fn foreign_or_interleaved_native_frames_refuse() {
    let t = tempfile::tempdir().unwrap();
    let up = Cursor::new(b"HTTP/1.1 200 OK\r\n\r\nx".to_vec());
    let mut c = Consumer::new(store(&t));
    c.foreign = true;
    assert_eq!(
        stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).err(),
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
    let up = Cursor::new(b"HTTP/1.1 200 OK\r\n\r\nx".to_vec());
    assert_eq!(
        stream(ps::relay(
            &mut Tool(binding(), store(&t)),
            up,
            || (),
            &binding(),
            1,
            1,
            |_| false
        ))
        .err(),
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
    let up = socket;
    let mut c = Consumer::new(store(&t));
    c.overtake = Some((2, std::time::Instant::now() + Duration::from_secs(3600)));
    let start = std::time::Instant::now();
    let r = stream(ps::relay(
        &mut c,
        up,
        move || {
            let _ = killer.shutdown(std::net::Shutdown::Both);
        },
        &binding(),
        1,
        1,
        |_| false,
    ))
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
    let up = Cursor::new(raw.into_bytes());
    let mut c = Consumer::new(store(&t));
    c.cancel_at = Some(2);
    let r = stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).unwrap();
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
    let up = Cursor::new(raw.into_bytes());
    let mut c = Consumer::new(store(&t));
    c.cancel_at = Some(2);
    let r = stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).unwrap();
    assert_eq!(r.outcome, Outcome::Cancelled { upstream_end: true });
    let u = r.evidence()["usage"].clone();
    assert_eq!(u["completed"], false);
    assert_eq!(u["failed"], "insufficient_quota");
}

#[test]
fn cancel_racing_a_terminal_frame_is_confirmed_not_left_waiting() {
    let t = tempfile::tempdir().unwrap();
    let raw = "HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc".to_string();
    let up = Cursor::new(raw.into_bytes());
    let mut c = Consumer::new(store(&t));
    // Pulls 1 (chunk) and 2 (end); the consumer's cancel 3 crossed the end.
    c.cancel_at = Some(3);
    let r = stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).unwrap();
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
    let up = Cursor::new(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx".to_vec());
    let mut c = Consumer::new(store(&t));
    let r = stream(ps::relay(&mut c, up, || (), &binding(), 1, 1, |_| false)).unwrap();
    assert_eq!(r.closing, Some("ack"));
    assert_eq!(
        kinds(&c),
        ["response-head", "response-chunk", "response-end"]
    );
}

#[test]
fn cancel_while_awaiting_head_is_prompt_and_closes_upstream() {
    let t = tempfile::tempdir().unwrap();
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut c, _) = l.accept().unwrap();
        // Never send a head; observe the host closing the connection.
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
    let mut c = Consumer::new(store(&t));
    // Readable after 200 ms with no pull outstanding: cancel ordinal 1.
    c.overtake = Some((0, std::time::Instant::now() + Duration::from_millis(200)));
    let start = std::time::Instant::now();
    let r = stream(ps::relay(
        &mut c,
        socket,
        move || {
            let _ = killer.shutdown(std::net::Shutdown::Both);
        },
        &binding(),
        1,
        1,
        |_| false,
    ))
    .unwrap();
    assert!(start.elapsed() < Duration::from_secs(3));
    assert_eq!(
        r.outcome,
        Outcome::Cancelled {
            upstream_end: false
        }
    );
    assert_eq!(r.status, 0);
    assert_eq!(kinds(&c), ["response-cancelled"]);
    assert_eq!(c.sent[0]["ordinal"], 1);
    let (n, waited) = server.join().unwrap();
    assert_eq!(n, 0);
    assert!(waited < Duration::from_secs(3));
}

#[test]
fn cancel_crossing_a_relayed_head_is_confirmed() {
    let t = tempfile::tempdir().unwrap();
    let s = store(&t);
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx".to_vec();
    let mut sess =
        ps::Session::start(Cursor::new(raw), || (), &binding(), 1, 1, |_| false).unwrap();
    let mut frames = vec![];
    while frames.is_empty() {
        frames = sess.poll(&s, Duration::from_millis(50)).unwrap();
    }
    assert_eq!(frames[0]["type"], "response-head");
    // The consumer had already sent cancel(1) before it saw the head.
    let cancel = json!({"type":"cancel","binding":binding(),"sequence":1,"stream":1,"ordinal":1});
    let out = sess.on_native(&cancel, &s).unwrap();
    assert_eq!(out[0]["type"], "response-cancelled");
    assert_eq!(out[0]["ordinal"], 1);
    assert_eq!(sess.progress(), &ps::Progress::Finished);
    // Pull before any head is refused in the head phase.
    let raw = b"HTTP/1.1 200 OK\r\n\r\n".to_vec();
    let slow = std::io::Read::chain(std::io::empty(), Cursor::new(raw));
    let mut early = ps::Session::start(slow, || (), &binding(), 1, 1, |_| false).unwrap();
    let pull = json!({"type":"pull","binding":binding(),"sequence":1,"stream":1,"ordinal":1});
    assert_eq!(
        early.on_native(&pull, &s).err(),
        Some("stream_frame_before_head")
    );
}

#[test]
fn refused_head_is_drained_screened_and_never_relayed() {
    let t = tempfile::tempdir().unwrap();
    let body = r#"{"error":{"code":"insufficient_quota"}}"#;
    let raw = format!(
        "HTTP/1.1 429 Too Many Requests\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut c = Consumer::new(store(&t));
    match ps::relay(
        &mut c,
        Cursor::new(raw.into_bytes()),
        || (),
        &binding(),
        1,
        1,
        |s| s == 429,
    )
    .unwrap()
    {
        ps::Relayed::Refused { status, body: b } => {
            assert_eq!(status, 429);
            assert_eq!(b, body.as_bytes());
        }
        ps::Relayed::Stream(_) => panic!("relayed a refusal"),
    }
    assert!(c.sent.is_empty(), "nothing reached the consumer");
    // A refusal body carrying the original credential is refused, not returned.
    let raw = format!("HTTP/1.1 429 X\r\nContent-Length: {}\r\n\r\n{A}", A.len());
    let mut c = Consumer::new(store(&t));
    assert_eq!(
        ps::relay(
            &mut c,
            Cursor::new(raw.into_bytes()),
            || (),
            &binding(),
            1,
            1,
            |s| s == 429
        )
        .err(),
        Some("pi_original_a_exclusion")
    );
}

#[test]
fn unreadable_head_fails_with_ordinal_zero_then_single_ack() {
    let t = tempfile::tempdir().unwrap();
    let mut c = Consumer::new(store(&t));
    let r = stream(ps::relay(
        &mut c,
        Cursor::new(b"garbage\r\n\r\n".to_vec()),
        || (),
        &binding(),
        1,
        1,
        |_| false,
    ))
    .unwrap();
    assert_eq!(r.outcome, Outcome::Interrupted("upstream_version"));
    assert_eq!(kinds(&c), ["response-failure"]);
    assert_eq!(c.sent[0]["ordinal"], 0);
    assert_eq!(r.closing, Some("ack"));
}

#[test]
fn release_after_terminal_event_drains_without_blocking_native_handling() {
    let t = tempfile::tempdir().unwrap();
    let s = store(&t);
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut c, _) = l.accept().unwrap();
        let e = sse(&[
            r#"{"type":"response.completed","response":{"id":"r","usage":{"input_tokens":1}}}"#,
        ]);
        write!(
            c,
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{e}\r\n",
            e.len()
        )
        .unwrap();
        std::thread::sleep(Duration::from_secs(3)); // framed end withheld
    });
    let mut sess = ps::Session::start(
        TcpStream::connect(addr).unwrap(),
        || (),
        &binding(),
        1,
        1,
        |_| false,
    )
    .unwrap();
    let op = |kind: &str, o: u64| json!({"type":kind,"binding":binding(),"sequence":1,"stream":1,"ordinal":o});
    while sess.poll(&s, Duration::from_millis(50)).unwrap().is_empty() {}
    let chunk = sess.on_native(&op("pull", 1), &s).unwrap();
    let chunk = if chunk.is_empty() {
        let mut f = vec![];
        while f.is_empty() {
            f = sess.poll(&s, Duration::from_millis(50)).unwrap();
        }
        f
    } else {
        chunk
    };
    assert_eq!(chunk[0]["type"], "response-chunk");
    let start = std::time::Instant::now();
    let out = sess.on_native(&op("cancel", 2), &s).unwrap();
    assert!(
        start.elapsed() < Duration::from_millis(100),
        "cancel handling blocked"
    );
    assert_eq!(out[0]["type"], "response-cancelled");
    assert!(
        !sess.accepts_native(),
        "later native frames belong to the session loop"
    );
    assert_eq!(sess.progress(), &ps::Progress::Pending);
    while sess.progress() == &ps::Progress::Pending {
        sess.poll(&s, Duration::from_millis(50)).unwrap();
    }
    assert!(start.elapsed() < Duration::from_millis(1500));
    assert_eq!(
        sess.relay().outcome,
        Outcome::Cancelled {
            upstream_end: false
        }
    );
    drop(sess);
    server.join().unwrap();
}

#[test]
fn unavailable_answer_owns_only_its_crossing_cancel() {
    let (u, answer) = ps::Unavailable::answer(&binding(), 1, 2, "attempt_limit").unwrap();
    assert_eq!(answer["type"], "response-unavailable");
    assert_eq!(answer["code"], "attempt_limit");
    let cancel = json!({"type":"cancel","binding":binding(),"sequence":1,"stream":2,"ordinal":1});
    let confirm = u.on_next(&cancel).unwrap().unwrap();
    assert_eq!(
        (confirm["type"].as_str(), confirm["ordinal"].as_u64()),
        (Some("response-cancelled"), Some(1))
    );
    // Session envelopes carry the native's current stream counter.
    let snapshot = json!({"type":"snapshot","binding":binding(),"sequence":1,"stream":2});
    assert_eq!(u.on_next(&snapshot).unwrap(), None);
    let pull = json!({"type":"pull","binding":binding(),"sequence":1,"stream":2,"ordinal":1});
    assert_eq!(u.on_next(&pull).err(), Some("stream_interleaved_frame"));
}

#[test]
fn pre_dispatch_confirms_a_crossed_head_cancel_only() {
    struct One(Option<Value>, Vec<Value>);
    impl Channel for One {
        fn send(&mut self, v: &Value) -> ps::Result<()> {
            self.1.push(v.clone());
            Ok(())
        }
        fn receive(&mut self) -> ps::Result<Value> {
            self.0.take().ok_or("empty")
        }
        fn readable(&mut self) -> ps::Result<bool> {
            Ok(self.0.is_some())
        }
    }
    let mut idle = One(None, vec![]);
    assert!(!ps::pre_dispatch(&mut idle, &binding(), 1, 1).unwrap());
    let mut c = One(
        Some(json!({"type":"cancel","binding":binding(),"sequence":1,"stream":1,"ordinal":1})),
        vec![],
    );
    assert!(ps::pre_dispatch(&mut c, &binding(), 1, 1).unwrap());
    assert_eq!(c.1[0]["type"], "response-cancelled");
    let mut bad = One(
        Some(json!({"type":"tool","binding":binding(),"sequence":1,"stream":1})),
        vec![],
    );
    assert_eq!(
        ps::pre_dispatch(&mut bad, &binding(), 1, 1).err(),
        Some("stream_interleaved_frame")
    );
}

#[test]
fn bytes_read_while_draining_after_release_are_kept_as_unforwarded_evidence() {
    // Piece (c) `trailing`: counted before, now also retained.
    let t = tempfile::tempdir().unwrap();
    let s = store(&t);
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let trailing = ": synthetic trailing bytes\n\n";
    let server = std::thread::spawn(move || {
        let (mut c, _) = l.accept().unwrap();
        let e = sse(&[
            r#"{"type":"response.completed","response":{"id":"r","usage":{"input_tokens":1}}}"#,
        ]);
        write!(
            c,
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{e}\r\n",
            e.len()
        )
        .unwrap();
        c.flush().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        write!(c, "{:x}\r\n{trailing}\r\n", trailing.len()).unwrap();
        c.flush().unwrap();
        std::thread::sleep(Duration::from_secs(2));
    });
    let mut sess = ps::Session::start(
        TcpStream::connect(addr).unwrap(),
        || (),
        &binding(),
        1,
        1,
        |_| false,
    )
    .unwrap();
    let op = |kind: &str, o: u64| json!({"type":kind,"binding":binding(),"sequence":1,"stream":1,"ordinal":o});
    while sess.poll(&s, Duration::from_millis(50)).unwrap().is_empty() {}
    let mut f = sess.on_native(&op("pull", 1), &s).unwrap();
    while f.is_empty() {
        f = sess.poll(&s, Duration::from_millis(50)).unwrap();
    }
    assert_eq!(f[0]["type"], "response-chunk");
    sess.on_native(&op("cancel", 2), &s).unwrap();
    while sess.progress() == &ps::Progress::Pending {
        sess.poll(&s, Duration::from_millis(50)).unwrap();
    }
    let r = sess.relay();
    assert_eq!(
        r.outcome,
        Outcome::Cancelled {
            upstream_end: false
        }
    );
    assert_eq!(r.unforwarded, trailing.as_bytes());
    assert_eq!(r.unforwarded_after_release, trailing.len());
    assert!(r.evidence()["unforwardedSha256"].is_string());
    assert!(!String::from_utf8_lossy(&r.forwarded).contains("trailing"));
    drop(sess);
    server.join().unwrap();
}

#[test]
fn terminal_is_last_allows_only_comments_and_blank_lines_after_the_terminal_event() {
    let done = r#"data: {"type":"response.completed","response":{"id":"r"}}"#;
    let created = r#"data: {"type":"response.created","response":{"id":"r"}}"#;
    let f = format!("{created}\n\n{done}\n\n");
    assert!(ps::terminal_is_last(f.as_bytes(), b""));
    assert!(ps::terminal_is_last(f.as_bytes(), b": keep-alive\n\n"));
    assert!(ps::terminal_is_last(
        format!("{created}\r\n\r\n{done}\r\n\r\n").as_bytes(),
        b"\r\n"
    ));
    // A data event, a partial line, or no terminal at all is not clean.
    assert!(!ps::terminal_is_last(
        f.as_bytes(),
        format!("{created}\n\n").as_bytes()
    ));
    assert!(!ps::terminal_is_last(f.as_bytes(), b": partial"));
    assert!(!ps::terminal_is_last(f.as_bytes(), b"event: x\n\n"));
    assert!(!ps::terminal_is_last(
        format!("{created}\n\n").as_bytes(),
        b""
    ));
    // The terminal event's own block must be complete.
    assert!(!ps::terminal_is_last(
        format!("{created}\n\n{done}\n").as_bytes(),
        b""
    ));
}

/// One pull, then a release right after the terminal event. The server sends
/// `first` as one chunk, then each `later` chunk after its delay, and keeps
/// the connection open past the drain bound unless `end` is set.
fn released_after(first: String, later: Vec<(u64, String)>, end: bool) -> ps::Relay {
    let t = tempfile::tempdir().unwrap();
    let s = store(&t);
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut c, _) = l.accept().unwrap();
        write!(
            c,
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{first}\r\n",
            first.len()
        )
        .unwrap();
        c.flush().unwrap();
        for (delay, b) in later {
            std::thread::sleep(Duration::from_millis(delay));
            write!(c, "{:x}\r\n{b}\r\n", b.len()).unwrap();
            c.flush().unwrap();
        }
        if end {
            write!(c, "0\r\n\r\n").unwrap();
            c.flush().unwrap();
        }
        std::thread::sleep(Duration::from_secs(2));
    });
    let mut sess = ps::Session::start(
        TcpStream::connect(addr).unwrap(),
        || (),
        &binding(),
        1,
        1,
        |_| false,
    )
    .unwrap();
    let op = |kind: &str, o: u64| json!({"type":kind,"binding":binding(),"sequence":1,"stream":1,"ordinal":o});
    while sess.poll(&s, Duration::from_millis(50)).unwrap().is_empty() {}
    let mut f = sess.on_native(&op("pull", 1), &s).unwrap();
    while f.is_empty() {
        f = sess.poll(&s, Duration::from_millis(50)).unwrap();
    }
    assert_eq!(f[0]["type"], "response-chunk");
    sess.on_native(&op("cancel", 2), &s).unwrap();
    while sess.progress() == &ps::Progress::Pending {
        sess.poll(&s, Duration::from_millis(50)).unwrap();
    }
    let r = sess.into_relay();
    server.join().unwrap();
    r
}
const DONE: &str =
    r#"{"type":"response.completed","response":{"id":"r","usage":{"input_tokens":1}}}"#;

#[test]
fn release_drain_reads_until_its_bound_not_just_one_chunk() {
    // N4: a comment followed by a data event inside the drain is not clean.
    let data = "data: {\"type\":\"response.output_text.delta\"}\n\n";
    let r = released_after(
        sse(&[DONE]),
        vec![(100, ": keep-alive\n\n".into()), (200, data.into())],
        false,
    );
    assert!(r.drained);
    assert_eq!(r.drain_error, None);
    assert_eq!(r.unforwarded, format!(": keep-alive\n\n{data}").as_bytes());
    assert_eq!(r.unforwarded_after_release, r.unforwarded.len());
    assert!(!ps::terminal_is_last(&r.forwarded, &r.unforwarded));
    // Only comments within the bound: clean, framed end not observed.
    let r = released_after(
        sse(&[DONE]),
        vec![(100, ": a\n\n".into()), (100, ": b\n\n".into())],
        false,
    );
    assert!(r.drained && r.held_dropped == 0);
    assert_eq!(
        r.outcome,
        Outcome::Cancelled {
            upstream_end: false
        }
    );
    assert!(ps::terminal_is_last(&r.forwarded, &r.unforwarded));
    // A framed end ends the drain early.
    let r = released_after(sse(&[DONE]), vec![(50, ": a\n\n".into())], true);
    assert!(r.drained);
    assert_eq!(r.outcome, Outcome::Cancelled { upstream_end: true });
}

#[test]
fn release_drain_screens_what_it_reads() {
    // N1: access material after the terminal event is neither retained nor
    // treated as a clean tail.
    let r = released_after(sse(&[DONE]), vec![(50, format!(": {A}\n\n"))], false);
    assert!(
        matches!(r.outcome, Outcome::Interrupted(_)),
        "{:?}",
        r.outcome
    );
    assert!(r.unforwarded.is_empty());
    assert!(r.held_dropped > 0);
    assert!(r.drained);
    let ev = r.evidence().to_string();
    assert!(!ev.contains(A));
}

#[test]
fn bytes_held_back_at_release_are_screened_by_the_drain_or_dropped_and_counted() {
    // N2: the screen held back a possible access-material prefix after the
    // terminal event when the consumer released.
    let first = format!("{}{}", sse(&[DONE]), &A[..3]);
    // Completed by the following bytes: access material, interrupted.
    let r = released_after(first.clone(), vec![(50, format!("{}\n\n", &A[3..]))], false);
    assert!(!String::from_utf8_lossy(&r.forwarded).contains(&A[..3]));
    assert!(
        matches!(r.outcome, Outcome::Interrupted(_)),
        "{:?}",
        r.outcome
    );
    assert!(r.unforwarded.is_empty() && r.held_dropped > 0);
    // Nothing follows: still held at the drain bound, dropped and counted.
    let r = released_after(first, vec![], false);
    assert!(r.drained);
    assert_eq!(r.held_dropped, 3);
    assert!(r.unforwarded.is_empty());
    assert_eq!(r.evidence()["heldDropped"], 3);
}

#[test]
fn the_first_response_completed_is_the_terminal_event() {
    // N5: a second terminal event is a data event after the terminal.
    let second =
        r#"{"type":"response.completed","response":{"id":"r2","usage":{"input_tokens":9}}}"#;
    let one = sse(&[DONE]);
    let two = sse(&[DONE, second]);
    assert!(!ps::terminal_is_last(two.as_bytes(), b""));
    assert!(!ps::terminal_is_last(
        one.as_bytes(),
        sse(&[second]).as_bytes()
    ));
    assert_eq!(ps::usage(two.as_bytes())["responseId"], "r");
    assert_eq!(ps::usage(two.as_bytes())["inputTokens"], 1);
}
