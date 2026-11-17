//! Offline synthetic Codex endpoint for the hosted Pi coding profile. Two
//! loopback routes: the primary refuses its first request with a definitive
//! 429 (exercising Host failover), the secondary serves the four-request
//! coding continuity flow. No auth or body is retained; only digests.
//!
//! `--fault <mode>:<n>` perturbs the secondary's n-th served request (piece (c)
//! qualification): `refuse`, `truncate`, `stall`, `failed`, `late-close-<ms>`,
//! `trailing`, `hold-head`, `hold-mid`. With a fault the observer keeps
//! accepting after the fault, so any later dispatch is recorded, until killed.
//!
//! `HOSTED_OBSERVER_SCENARIO` selects a fault (default `ok`):
//! - `refuse-all`: primary 429, secondary 402, for every request.
//! - `primary-500`: the primary answers 500 (not a refusal).
//! - on the secondary's first served response: `truncate` (closes mid-body),
//!   `midstream-failed` (`response.failed` after output), `delayed-close`
//!   (framed end 3 s after `response.completed`), `trailing` (an extra SSE
//!   comment after `response.completed`), `stall-head` (no head) and
//!   `stall-body` (two events, then silence). Stalls hold the connection
//!   until the peer closes it and report when that happened.
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
    let args: Vec<String> = std::env::args().collect();
    let fault: Option<(String, usize)> = match args.get(1).map(String::as_str) {
        None => None,
        Some("--fault") => {
            let spec = args.get(2).ok_or("fault spec")?;
            let (mode, n) = spec.rsplit_once(':').ok_or("fault spec")?;
            Some((mode.to_string(), n.parse()?))
        }
        Some(_) => return Err("usage: [--fault <mode>:<n>]".into()),
    };
    let deadline = Instant::now() + Duration::from_secs(600);
    let mut served = 0usize;
    let mut refused = 0usize;
    let mut authority: Option<String> = None;
    let scenario = std::env::var("HOSTED_OBSERVER_SCENARIO").unwrap_or_else(|_| "ok".into());
    const SCENARIOS: [&str; 9] = [
        "ok",
        "refuse-all",
        "primary-500",
        "truncate",
        "midstream-failed",
        "delayed-close",
        "trailing",
        "stall-head",
        "stall-body",
    ];
    if !SCENARIOS.contains(&scenario.as_str()) {
        return Err("unknown scenario".into());
    }
    while fault.is_some() || served < 4 {
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
        if authority.as_ref().is_some_and(|a| a != &r.auth) {
            return Err("authority changed".into());
        }
        authority = Some(r.auth.clone());
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
        let mode = fault
            .as_ref()
            .filter(|(_, n)| *n == served)
            .map(|(m, _)| m.as_str());
        if mode == Some("refuse") {
            refused += 1;
            println!(
                "{}",
                json!({"type":"refused","status":429,"receipt":receipt,"fault":"refuse"})
            );
            std::io::stdout().flush()?;
            let body = json!({"error":{"code":"insufficient_quota","message":"synthetic secondary route quota exhausted"}}).to_string();
            write!(
                s,
                "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )?;
            continue;
        }
        if let Some(mode) = mode {
            fault_serve(s, mode, served, receipt)?;
            continue;
        }
        let reply = response(served);
        println!(
            "{}",
            json!({"type":"served","effect":served,"receipt":receipt,"responseSha256":hash(&reply)})
        );
        std::io::stdout().flush()?;
        let fault = if served == 1 { scenario.as_str() } else { "ok" };
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
        if fault == "trailing" {
            std::thread::sleep(Duration::from_millis(200));
            let extra = ": synthetic trailing bytes\n\n";
            write!(s, "{:x}\r\n{extra}\r\n", extra.len())?;
            s.flush()?;
        }
        if fault == "delayed-close" {
            std::thread::sleep(Duration::from_secs(3));
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
fn chunk(s: &mut TcpStream, event: &str) -> std::io::Result<()> {
    write!(s, "{:x}\r\n{event}\r\n", event.len())?;
    s.flush()?;
    std::thread::sleep(Duration::from_millis(20));
    Ok(())
}
/// One faulted response. Holding modes keep the connection on a thread so
/// the accept loop keeps recording any later dispatch.
fn fault_serve(
    mut s: TcpStream,
    mode: &str,
    n: usize,
    receipt: Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reply = response(n);
    if mode == "failed" {
        let failed = json!({"type":"response.failed","response":{"id":format!("resp_{n}"),"model":"gpt-6-luna","status":"failed","error":{"code":"server_error","message":"synthetic mid-stream failure"}}});
        let text = String::from_utf8(reply)?;
        let mut events: Vec<&str> = text.split_inclusive("\n\n").collect();
        events.pop();
        reply = format!("{}data: {failed}\n\n", events.concat()).into_bytes();
    }
    println!(
        "{}",
        json!({"type":"served","effect":n,"receipt":receipt,"responseSha256":hash(&reply),"fault":mode})
    );
    std::io::stdout().flush()?;
    if mode == "hold-head" {
        println!("{}", json!({"type":"holding","effect":n,"phase":"head"}));
        std::io::stdout().flush()?;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(120));
            drop(s);
        });
        return Ok(());
    }
    write!(
        s,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
    )?;
    let text = String::from_utf8(reply)?;
    let events: Vec<&str> = text.split_inclusive("\n\n").collect();
    match mode {
        "truncate" | "stall" | "hold-mid" => {
            for e in &events[..2] {
                chunk(&mut s, e)?;
            }
            if mode == "truncate" {
                return Ok(()); // dropped: no terminal chunk
            }
            if mode == "hold-mid" {
                println!("{}", json!({"type":"holding","effect":n,"phase":"mid"}));
                std::io::stdout().flush()?;
            }
            let secs = if mode == "stall" { 40 } else { 120 };
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(secs));
                drop(s);
            });
            Ok(())
        }
        "failed" => {
            for e in &events {
                chunk(&mut s, e)?;
            }
            write!(s, "0\r\n\r\n")?;
            Ok(s.flush()?)
        }
        "trailing" => {
            for e in &events {
                chunk(&mut s, e)?;
            }
            let extra = json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":" trailing"});
            chunk(&mut s, &format!("data: {extra}\n\n"))?;
            write!(s, "0\r\n\r\n")?;
            Ok(s.flush()?)
        }
        m if m.starts_with("late-close-") => {
            let ms: u64 = m["late-close-".len()..].parse()?;
            for e in &events {
                chunk(&mut s, e)?;
            }
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(ms));
                let _ = write!(s, "0\r\n\r\n");
                let _ = s.flush();
            });
            Ok(())
        }
        _ => Err(format!("unknown fault {mode}").into()),
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("hosted_coding_observer: {e}");
        std::process::exit(1);
    }
}
