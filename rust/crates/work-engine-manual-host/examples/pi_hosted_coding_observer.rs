//! Offline synthetic Codex endpoint for the hosted Pi coding profile. Two
//! loopback routes: the primary refuses its first request with a definitive
//! 429 (exercising Host failover), the secondary serves the four-request
//! coding continuity flow. No auth or body is retained; only digests.
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
    let mut authority: Option<String> = None;
    while served < 4 {
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
        write!(
            s,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
        )?;
        for event in std::str::from_utf8(&reply)?.split_inclusive("\n\n") {
            write!(s, "{:x}\r\n{event}\r\n", event.len())?;
            s.flush()?;
            std::thread::sleep(Duration::from_millis(20));
        }
        write!(s, "0\r\n\r\n")?;
        s.flush()?;
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
