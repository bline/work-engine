//! Offline synthetic Codex endpoint for the hosted analysis profile. Two
//! loopback routes: the primary refuses its first request with a definitive
//! 429 (Host failover; the second attempt must dispatch the same retained
//! body), the secondary serves a three-request analysis: a `snapshot_list`
//! call; a `snapshot_read` of the first listed member together with a call of
//! an unknown tool (refused by the SDK, never reaching the Host); then the
//! final text answer. No auth or body is retained; only digests.
//!
//! `HOSTED_OBSERVER_SCENARIO`: `ok` (default), `long-answer` (the final
//! answer is 8 000 small deltas) or `read-all` (after the list, every member
//! is read page by page, up to four reads per response, until all are read or
//! `HOSTED_OBSERVER_MAX_REQUESTS` (default 24) is one request away; then the
//! answer). `read-all` carries the whole snapshot into the context.
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
/// One response whose output items are `items`; text items stream as
/// `deltas` pieces.
fn sse(n: usize, items: &[Value], deltas: usize) -> Vec<u8> {
    let mut out = vec![
        json!({"type":"response.created","response":{"id":format!("resp_{n}"),"status":"in_progress"}}),
    ];
    for (i, item) in items.iter().enumerate() {
        if item["type"] == "function_call" {
            out.push(json!({"type":"response.output_item.added","output_index":i,"item":{"type":"function_call","id":item["id"],"call_id":item["call_id"],"name":item["name"],"arguments":"","status":"in_progress"}}));
            out.push(json!({"type":"response.function_call_arguments.delta","output_index":i,"delta":item["arguments"]}));
        } else {
            out.push(json!({"type":"response.output_item.added","output_index":i,"item":{"type":"message","id":item["id"],"role":"assistant","status":"in_progress","content":[]}}));
            let text = item["content"][0]["text"].as_str().unwrap_or("");
            let pieces = deltas.max(1);
            let size = text.len().div_ceil(pieces).max(1);
            for chunk in text.as_bytes().chunks(size) {
                out.push(json!({"type":"response.output_text.delta","output_index":i,"content_index":0,"delta":std::str::from_utf8(chunk).unwrap()}));
            }
        }
        out.push(json!({"type":"response.output_item.done","output_index":i,"item":item}));
    }
    out.push(json!({"type":"response.completed","response":{"id":format!("resp_{n}"),"model":"gpt-6-luna","status":"completed","output":items,"usage":{"input_tokens":4000,"output_tokens":200,"total_tokens":4200}}}));
    out.iter()
        .map(|v| format!("data: {v}\n\n"))
        .collect::<String>()
        .into_bytes()
}
fn call(n: usize, k: usize, name: &str, arguments: Value) -> Value {
    json!({"type":"function_call","id":format!("fc_{n}_{k}"),"call_id":format!("call_{n}_{k}"),"name":name,"arguments":arguments.to_string(),"status":"completed"})
}
fn message(n: usize, text: &str) -> Value {
    json!({"type":"message","id":format!("msg_{n}"),"role":"assistant","status":"completed","content":[{"type":"output_text","text":text,"annotations":[]}]})
}
fn response(n: usize, first_member: &str, long: bool) -> Vec<u8> {
    match n {
        1 => sse(1, &[call(1, 0, "snapshot_list", json!({}))], 1),
        2 => sse(
            2,
            &[
                call(2, 0, "snapshot_read", json!({"path":first_member})),
                call(2, 1, "workspace_exec", json!({"command":"cat /etc/passwd"})),
            ],
            1,
        ),
        _ if long => sse(n, &[message(n, &"w ".repeat(8000))], 8000),
        _ => sse(
            n,
            &[message(
                n,
                "Supported: the snapshot member was read. Unsupported: none. Unknown: none.",
            )],
            4,
        ),
    }
}
/// The next reads of `read-all`, derived only from the Host outputs already
/// in the request's input: the member list and each page's `next_offset`.
fn next_reads(input: &[Value]) -> Vec<(String, u64)> {
    let outputs: Vec<Value> = input
        .iter()
        .filter(|i| i["type"] == "function_call_output")
        .filter_map(|i| serde_json::from_str(i["output"].as_str()?).ok())
        .collect();
    let Some(list) = outputs.iter().find(|o| o["members"].is_array()) else {
        return vec![];
    };
    let mut reads = vec![];
    for m in list["members"].as_array().unwrap() {
        let path = m["path"].as_str().unwrap_or("").to_string();
        let total = m["bytes"].as_u64().unwrap_or(0);
        let mut end = 0u64;
        let mut done = false;
        for o in outputs.iter().filter(|o| o["path"] == path.as_str()) {
            let stop = o["offset"].as_u64().unwrap_or(0) + o["bytes"].as_u64().unwrap_or(0);
            if o["offset"].as_u64().unwrap_or(0) <= end && stop > end {
                end = stop;
            }
            if o["next_offset"].is_null() && o["offset"].as_u64().unwrap_or(0) <= end {
                done = true;
            }
        }
        while !done && end < total.max(1) && reads.len() < 4 {
            reads.push((path.clone(), end));
            end += 24_576;
            done = end >= total;
        }
        if reads.len() >= 4 {
            break;
        }
    }
    reads
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
    if n > 700_000 {
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
        json!({"type":"ready","primary":primary.local_addr()?.port(),"secondary":secondary.local_addr()?.port(),"protocol":"offline-hosted-analysis-observer-v1"})
    );
    std::io::stdout().flush()?;
    let scenario = std::env::var("HOSTED_OBSERVER_SCENARIO").unwrap_or_else(|_| "ok".into());
    if !matches!(scenario.as_str(), "ok" | "long-answer" | "read-all") {
        return Err("unknown scenario".into());
    }
    let long = scenario == "long-answer";
    let read_all = scenario == "read-all";
    let max_requests: usize = std::env::var("HOSTED_OBSERVER_MAX_REQUESTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(24);
    let mut finished = false;
    let deadline = Instant::now() + Duration::from_secs(600);
    let mut served = 0usize;
    let mut refused = 0usize;
    let mut first_member = String::new();
    let mut authority: std::collections::BTreeMap<&str, String> = Default::default();
    while if read_all { !finished } else { served < 3 } {
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
        authority.insert(route, r.auth.clone());
        let v: Value = serde_json::from_slice(&r.body)?;
        // Declared at the top level or as `additional_tools` input items.
        let mut tools: Vec<&str> = v["tools"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(
                v["input"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|i| i["type"] == "additional_tools")
                    .flat_map(|i| i["tools"].as_array().into_iter().flatten()),
            )
            .filter_map(|t| t["name"].as_str())
            .collect();
        tools.sort_unstable();
        if v["model"] != "gpt-6-luna"
            || v["stream"] != true
            || tools != ["snapshot_list", "snapshot_read"]
        {
            return Err("projection".into());
        }
        // The member list the Host returned to request 1 is in request 2's input.
        if first_member.is_empty()
            && let Some(input) = v["input"].as_array()
        {
            for item in input {
                if item["type"] == "function_call_output"
                    && let Some(out) = item["output"].as_str()
                    && let Ok(list) = serde_json::from_str::<Value>(out)
                    && let Some(path) = list["members"][0]["path"].as_str()
                {
                    first_member = path.to_string();
                }
            }
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
        let reply = if read_all && served > 1 {
            let reads = next_reads(v["input"].as_array().map(Vec::as_slice).unwrap_or(&[]));
            if reads.is_empty() || served >= max_requests {
                finished = true;
                let read: usize = v["input"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|i| i["type"] == "function_call_output")
                    .count();
                sse(
                    served,
                    &[message(
                        served,
                        &format!("Read {read} tool outputs; the snapshot is summarized here."),
                    )],
                    4,
                )
            } else {
                let calls: Vec<Value> = reads
                    .iter()
                    .enumerate()
                    .map(|(k, (path, offset))| {
                        call(
                            served,
                            k,
                            "snapshot_read",
                            json!({"path":path,"offset":offset}),
                        )
                    })
                    .collect();
                sse(served, &calls, 1)
            }
        } else {
            response(served, &first_member, long)
        };
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
        eprintln!("hosted_analysis_observer: {e}");
        std::process::exit(1);
    }
}
