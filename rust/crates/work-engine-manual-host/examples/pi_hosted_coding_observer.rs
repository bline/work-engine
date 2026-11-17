//! Offline synthetic Codex endpoint for the hosted Pi coding profile. Two
//! loopback routes: the primary refuses its first request with a definitive
//! 429 (exercising Host failover), the secondary serves the four-request
//! coding continuity flow. No auth or body is retained; only digests.
//!
//! `HOSTED_OBSERVER_SCENARIO` selects a fault (default `ok`):
//! - `refuse-all`: primary 429, secondary 402, for every request.
//! - `primary-500`: the primary answers 500 (not a refusal).
//! - on the secondary's first served response: `truncate` (closes mid-body),
//!   `midstream-failed` (`response.failed` after output), `late-close-short`
//!   and `late-close-long` (framed end 0.5 s or 3 s after
//!   `response.completed`), `trailing` (an extra SSE comment after
//!   `response.completed`), `trailing-data` (an extra `data:` event after
//!   it), `stall-head` (no head) and `stall-body` (two events, then
//!   silence). Stalls hold the connection until the peer closes
//!   it and report when that happened.
//! - `echo-body` (a text delta carrying the request's bearer before the
//!   terminal event) and `echo-trailing` (an SSE comment carrying it after
//!   the terminal event): the Host's access screen must stop both.
//! - A secondary fault may name the served response it applies to:
//!   `truncate@3` truncates the third (the first turn after the reset).
//! - `primary-429-echo` (a 429 whose body carries the bearer) and
//!   `primary-429-truncated` (a 429 whose body ends before its length): neither
//!   is a definitive refusal.
//!
//! After a fault the observer keeps accepting, so any later dispatch (a
//! forbidden failover or replay) is recorded as a receipt.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn events(n: usize, item: Value) -> Vec<u8> {
    let added = match item["type"].as_str() {
        Some("function_call") => {
            json!({"type":"function_call","id":item["id"],"call_id":item["call_id"],"name":item["name"],"arguments":"","status":"in_progress"})
        }
        _ => {
            json!({"type":"message","id":item["id"],"role":"assistant","status":"in_progress","content":[]})
        }
    };
    let delta = match item["type"].as_str() {
        Some("function_call") => {
            json!({"type":"response.function_call_arguments.delta","output_index":0,"delta":item["arguments"]})
        }
        _ => {
            json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":item["content"][0]["text"]})
        }
    };
    [
        json!({"type":"response.created","response":{"id":format!("resp_{n}"),"status":"in_progress"}}),
        json!({"type":"response.output_item.added","output_index":0,"item":added}),
        delta,
        json!({"type":"response.output_item.done","output_index":0,"item":item}),
        json!({"type":"response.completed","response":{"id":format!("resp_{n}"),"model":"gpt-6-luna","status":"completed","output":[item],"usage":{"input_tokens":1000,"output_tokens":20,"total_tokens":1020}}}),
    ]
    .iter()
    .map(|v| format!("data: {v}\n\n"))
    .collect::<String>()
    .into_bytes()
}
fn text(n: usize, t: &str) -> Vec<u8> {
    events(
        n,
        json!({"type":"message","id":format!("msg_{n}"),"role":"assistant","status":"completed","content":[{"type":"output_text","text":t,"annotations":[]}]}),
    )
}
fn response(n: usize) -> Vec<u8> {
    match n {
        1 => events(
            1,
            json!({"type":"function_call","id":"fc_1","call_id":"call_1","name":"workspace_exec",
                "arguments":json!({"command":"printf 'pub fn answer() -> u32 { 42 }\\n' >> src/lib.rs"}).to_string(),"status":"completed"}),
        ),
        2 => text(2, "Added answer() returning 42 to src/lib.rs."),
        3 => text(
            3,
            "Recovered the exact coding result and workspace from the checkpoint.",
        ),
        _ => text(n, "42"),
    }
}
struct Request {
    id: String,
    auth: String,
    /// Raw bearer, held only to echo it in the `echo-*` scenarios; never
    /// printed.
    bearer: String,
    body: Vec<u8>,
}
fn read(s: &mut TcpStream) -> Result<Request, Box<dyn std::error::Error>> {
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    s.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut header = vec![];
    let mut byte = [0];
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() > 16384 {
            return Err("header bound".into());
        }
        s.read_exact(&mut byte)?;
        header.push(byte[0]);
    }
    let header = std::str::from_utf8(&header)?;
    if !header.starts_with("POST /backend-api/codex/responses HTTP/1.1\r\n") {
        return Err("route".into());
    }
    let field = |name: &str| {
        header
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
            })
            .ok_or("header")
    };
    let auth = field("authorization")?;
    let auth = auth.strip_prefix("Bearer ").ok_or("auth scheme")?;
    if !auth.starts_with("synthetic-A-") || header.contains("nonsecret-marker") {
        return Err("auth profile".into());
    }
    let n: usize = field("content-length")?.parse()?;
    if n > 65536 {
        return Err("body bound".into());
    }
    let mut body = vec![0; n];
    s.read_exact(&mut body)?;
    Ok(Request {
        id: field("x-request-id")?,
        auth: hash(auth.as_bytes()),
        bearer: auth.to_string(),
        body,
    })
}
/// Hold an accepted connection without sending more, until the peer closes
/// it (or 300 s), and report when.
fn hold(s: &mut TcpStream, fault: &str, id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        json!({"type":"fault","fault":fault,"requestId":id,"phase":"holding"})
    );
    std::io::stdout().flush()?;
    let at = Instant::now();
    s.set_read_timeout(Some(Duration::from_secs(300)))?;
    let mut b = [0u8; 1];
    let closed =
        matches!(s.read(&mut b), Ok(0) | Err(_)) && at.elapsed() < Duration::from_secs(300);
    println!(
        "{}",
        json!({"type":"fault","fault":fault,"requestId":id,"phase":"released","peerClosed":closed,"heldMs":at.elapsed().as_millis() as u64})
    );
    std::io::stdout().flush()?;
    Ok(())
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let primary = TcpListener::bind("127.0.0.1:0")?;
    let secondary = TcpListener::bind("127.0.0.1:0")?;
    primary.set_nonblocking(true)?;
    secondary.set_nonblocking(true)?;
    println!(
        "{}",
        json!({"type":"ready","primary":primary.local_addr()?.port(),"secondary":secondary.local_addr()?.port(),"protocol":"offline-hosted-coding-observer-v1"})
    );
    std::io::stdout().flush()?;
    let deadline = Instant::now() + Duration::from_secs(600);
    let mut served = 0usize;
    let mut refused = 0usize;
    // Each route (payer) has its own Host-held access; it must not change.
    let mut authority: std::collections::BTreeMap<&str, String> = Default::default();
    let scenario = std::env::var("HOSTED_OBSERVER_SCENARIO").unwrap_or_else(|_| "ok".into());
    // `fault@N`: the secondary fault applies to the N-th served response.
    let (scenario, fault_at) = match scenario.split_once('@') {
        Some((f, n)) => (
            f.to_string(),
            n.parse::<usize>().map_err(|_| "fault index")?,
        ),
        None => (scenario.clone(), 1),
    };
    const SCENARIOS: [&str; 15] = [
        "echo-body",
        "echo-trailing",
        "primary-429-echo",
        "primary-429-truncated",
        "trailing-data",
        "ok",
        "refuse-all",
        "primary-500",
        "truncate",
        "midstream-failed",
        "late-close-short",
        "late-close-long",
        "trailing",
        "stall-head",
        "stall-body",
    ];
    if !SCENARIOS.contains(&scenario.as_str()) {
        return Err("unknown scenario".into());
    }
    // A clean run ends after four served responses; a faulted run keeps
    // accepting until killed, so later dispatches are visible.
    let completes = matches!(
        scenario.as_str(),
        "ok" | "late-close-short" | "late-close-long" | "trailing"
    );
    while !(completes && served >= 4) {
        if Instant::now() > deadline {
            return Err("observer deadline".into());
        }
        let mut accepted = None;
        for (route, l) in [("primary", &primary), ("secondary", &secondary)] {
            match l.accept() {
                Ok((s, _)) => {
                    accepted = Some((route, s));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.into()),
            }
        }
        let Some((route, mut s)) = accepted else {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        };
        s.set_nonblocking(false)?;
        let r = read(&mut s)?;
        if authority.get(route).is_some_and(|a| a != &r.auth) {
            return Err("authority changed".into());
        }
        if authority.iter().any(|(k, a)| *k != route && a == &r.auth) {
            return Err("one access served two payers".into());
        }
        authority.insert(route, r.auth.clone());
        let v: Value = serde_json::from_slice(&r.body)?;
        if v["model"] != "gpt-6-luna" || v["stream"] != true {
            return Err("projection".into());
        }
        let receipt = json!({"route":route,"requestId":r.id,"bodySha256":hash(&r.body)});
        if route == "primary" && scenario == "primary-500" {
            println!(
                "{}",
                json!({"type":"failed","status":500,"receipt":receipt})
            );
            std::io::stdout().flush()?;
            let body =
                json!({"error":{"code":"server_error","message":"synthetic primary failure"}})
                    .to_string();
            write!(
                s,
                "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )?;
            continue;
        }
        if route == "secondary" && scenario == "refuse-all" {
            refused += 1;
            println!(
                "{}",
                json!({"type":"refused","status":402,"receipt":receipt})
            );
            std::io::stdout().flush()?;
            let body = json!({"error":{"code":"payment_required","message":"synthetic secondary route unfunded"}}).to_string();
            write!(
                s,
                "HTTP/1.1 402 Payment Required\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )?;
            continue;
        }
        if route == "primary" && scenario == "primary-429-echo" {
            refused += 1;
            println!(
                "{}",
                json!({"type":"refused","status":429,"receipt":receipt,"fault":"echo"})
            );
            std::io::stdout().flush()?;
            let body = json!({"error":{"code":"insufficient_quota","message":format!("synthetic echo {}", r.bearer)}}).to_string();
            write!(
                s,
                "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )?;
            continue;
        }
        if route == "primary" && scenario == "primary-429-truncated" {
            refused += 1;
            println!(
                "{}",
                json!({"type":"refused","status":429,"receipt":receipt,"fault":"truncated"})
            );
            std::io::stdout().flush()?;
            let body = json!({"error":{"code":"insufficient_quota","message":"synthetic primary route quota exhausted"}}).to_string();
            write!(
                s,
                "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len() + 64,
                &body[..body.len() / 2]
            )?;
            s.flush()?;
            continue;
        }
        if route == "primary" {
            refused += 1;
            println!(
                "{}",
                json!({"type":"refused","status":429,"receipt":receipt})
            );
            std::io::stdout().flush()?;
            let body = json!({"error":{"code":"insufficient_quota","message":"synthetic primary route quota exhausted"}}).to_string();
            write!(
                s,
                "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )?;
            continue;
        }
        served += 1;
        let reply = response(served);
        println!(
            "{}",
            json!({"type":"served","effect":served,"receipt":receipt,"responseSha256":hash(&reply)})
        );
        std::io::stdout().flush()?;
        let fault = if served == fault_at {
            scenario.as_str()
        } else {
            "ok"
        };
        if fault == "stall-head" {
            hold(&mut s, "stall-head", &r.id)?;
            continue;
        }
        write!(
            s,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
        )?;
        let text = std::str::from_utf8(&reply)?;
        let mut events: Vec<String> = text.split_inclusive("\n\n").map(String::from).collect();
        if fault == "echo-body" {
            events.insert(
                2,
                format!(
                    "data: {}\n\n",
                    json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":format!("echo {}", r.bearer)})
                ),
            );
        }
        if fault == "midstream-failed" {
            events.truncate(2);
            events.push(format!(
                "data: {}\n\n",
                json!({"type":"response.failed","response":{"id":"resp_1","status":"failed","error":{"code":"insufficient_quota","message":"synthetic quota exhausted mid-stream"}}})
            ));
        }
        let mut ended = true;
        for (i, event) in events.iter().enumerate() {
            if i == 2 && matches!(fault, "truncate" | "stall-body") {
                if fault == "stall-body" {
                    hold(&mut s, "stall-body", &r.id)?;
                } else {
                    println!(
                        "{}",
                        json!({"type":"fault","fault":"truncate","requestId":r.id})
                    );
                    std::io::stdout().flush()?;
                }
                ended = false;
                break;
            }
            write!(s, "{:x}\r\n{event}\r\n", event.len())?;
            s.flush()?;
            std::thread::sleep(Duration::from_millis(20));
        }
        if !ended {
            continue;
        }
        if matches!(fault, "trailing" | "trailing-data" | "echo-trailing") {
            std::thread::sleep(Duration::from_millis(200));
            let extra = if fault == "trailing" {
                ": synthetic trailing bytes\n\n".to_string()
            } else if fault == "echo-trailing" {
                format!(": echo {}\n\n", r.bearer)
            } else {
                format!(
                    "data: {}\n\n",
                    json!({"type":"response.output_text.delta","delta":"synthetic data after the terminal event"})
                )
            };
            write!(s, "{:x}\r\n{extra}\r\n", extra.len())?;
            s.flush()?;
        }
        match fault {
            "late-close-short" => std::thread::sleep(Duration::from_millis(500)),
            "late-close-long" => std::thread::sleep(Duration::from_secs(3)),
            _ => {}
        }
        let _ = write!(s, "0\r\n\r\n").and_then(|_| s.flush());
        if fault != "ok" {
            println!("{}", json!({"type":"fault","fault":fault,"requestId":r.id}));
            std::io::stdout().flush()?;
        }
    }
    println!(
        "{}",
        json!({"type":"complete","served":served,"refused":refused,"rawAuthOrBodyRetained":false})
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("hosted_coding_observer: {e}");
        std::process::exit(1);
    }
}
