//! Offline synthetic Anthropic Messages endpoint for the hosted analysis
//! profile on the `claude-sdk` provider (W4). Two loopback routes: the
//! primary refuses its first request with a definitive 429 (Host failover;
//! the second attempt must dispatch the same retained body), the secondary
//! serves a three-request analysis as Claude Code expects it: an
//! `mcp__pi__snapshot_list` call; an `mcp__pi__snapshot_read` of the first
//! listed member; then the final text answer. Each request must carry the
//! Host's synthetic access, never the container's dummy credential, and
//! `x-stainless-retry-count: 0`. No auth or body is retained; only digests
//! and the canonical digest of the declared `tools`.
//!
//! `HOSTED_OBSERVER_SCENARIO`: `ok` (default) or `retry-head` (the
//! secondary's first response is a 500, which is not a definitive refusal:
//! the attempt is uncertain and no further request may arrive), or `relay`
//! (the lifecycle relay profile's tools, Pi Host controller mode: an
//! `mcp__pi__snapshot_read`, an `mcp__pi__result_publish`, the first turn's
//! answer, then the second turn's answer), or `steps` (relay tools; the
//! responses are the steps in `HOSTED_OBSERVER_STEPS`, a JSON array of
//! `{"tool", "input"}`, `{"tools": [{"tool", "input"}, ...]}` (several
//! tool calls in one response) or `{"text"}`, one per served request).
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};
const MODEL: &str = "claude-haiku-5-5";
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
enum Block {
    Tool(String, Value),
    Text(String),
}
/// One Messages SSE response with `blocks`; text streams as a few deltas.
fn sse(n: usize, blocks: &[Block]) -> Vec<u8> {
    let mut events = vec![
        json!({"type":"message_start","message":{"id":format!("msg_{n}"),"type":"message","role":"assistant",
        "model":MODEL,"content":[],"stop_reason":null,"stop_sequence":null,
        "usage":{"input_tokens":40,"cache_creation_input_tokens":3000,"cache_read_input_tokens":1000,"output_tokens":1}}}),
    ];
    let mut tool = false;
    for (i, b) in blocks.iter().enumerate() {
        match b {
            Block::Tool(name, input) => {
                tool = true;
                events.push(json!({"type":"content_block_start","index":i,"content_block":{"type":"tool_use","id":format!("toolu_{n}_{i}"),"name":name,"input":{}}}));
                events.push(json!({"type":"content_block_delta","index":i,"delta":{"type":"input_json_delta","partial_json":input.to_string()}}));
            }
            Block::Text(text) => {
                events.push(json!({"type":"content_block_start","index":i,"content_block":{"type":"text","text":""}}));
                for chunk in text.as_bytes().chunks(text.len().div_ceil(4).max(1)) {
                    events.push(json!({"type":"content_block_delta","index":i,"delta":{"type":"text_delta","text":std::str::from_utf8(chunk).unwrap()}}));
                }
            }
        }
        events.push(json!({"type":"content_block_stop","index":i}));
    }
    events.push(json!({"type":"message_delta","delta":{"stop_reason":if tool {"tool_use"} else {"end_turn"},"stop_sequence":null},
        "usage":{"input_tokens":40,"cache_creation_input_tokens":3000,"cache_read_input_tokens":1000,"output_tokens":120}}));
    events.push(json!({"type":"message_stop"}));
    events
        .iter()
        .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap()))
        .collect::<String>()
        .into_bytes()
}
struct Request {
    id: String,
    auth: String,
    retry: String,
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
    if !header.starts_with("POST /v1/messages?beta=true HTTP/1.1\r\n")
        && !header.starts_with("POST /v1/messages HTTP/1.1\r\n")
    {
        return Err("route".into());
    }
    let field = |name: &str| {
        header
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
            })
            .ok_or(format!("header {name}"))
    };
    let auth = field("authorization")?;
    let auth = auth.strip_prefix("Bearer ").ok_or("auth scheme")?;
    // The container's dummy credential never reaches a provider.
    if !auth.starts_with("synthetic-A-") || header.contains("DDDDDDDD") {
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
        retry: field("x-stainless-retry-count").unwrap_or_default(),
        body,
    })
}
/// The first member path of the `snapshot_list` result in the request's
/// tool results (Claude Code sends MCP results as text blocks).
fn first_member(v: &Value) -> Option<String> {
    for m in v["messages"].as_array()? {
        for c in m["content"].as_array().into_iter().flatten() {
            if c["type"] != "tool_result" {
                continue;
            }
            let texts: Vec<&str> = match &c["content"] {
                Value::String(s) => vec![s.as_str()],
                Value::Array(a) => a.iter().filter_map(|b| b["text"].as_str()).collect(),
                _ => vec![],
            };
            for t in texts {
                if let Ok(list) = serde_json::from_str::<Value>(t)
                    && let Some(p) = list["members"][0]["path"].as_str()
                {
                    return Some(p.into());
                }
            }
        }
    }
    None
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let primary = TcpListener::bind("127.0.0.1:0")?;
    let secondary = TcpListener::bind("127.0.0.1:0")?;
    primary.set_nonblocking(true)?;
    secondary.set_nonblocking(true)?;
    println!(
        "{}",
        json!({"type":"ready","primary":primary.local_addr()?.port(),"secondary":secondary.local_addr()?.port(),"protocol":"offline-hosted-messages-observer-v1"})
    );
    std::io::stdout().flush()?;
    let scenario = std::env::var("HOSTED_OBSERVER_SCENARIO").unwrap_or_else(|_| "ok".into());
    if !matches!(scenario.as_str(), "ok" | "retry-head" | "relay" | "steps") {
        return Err("unknown scenario".into());
    }
    let deadline = Instant::now() + Duration::from_secs(600);
    let (mut served, mut refused) = (0usize, 0usize);
    let mut authority: std::collections::BTreeMap<&str, String> = Default::default();
    let steps: Vec<Value> = if scenario == "steps" {
        serde_json::from_slice(&std::fs::read(std::env::var("HOSTED_OBSERVER_STEPS")?)?)?
    } else {
        vec![]
    };
    let relay = scenario == "relay" || scenario == "steps";
    let total = if scenario == "steps" {
        steps.len()
    } else if relay {
        4
    } else {
        3
    };
    while served < total {
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
        let mut names: Vec<&str> = v["tools"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|t| t["name"].as_str())
            .collect();
        names.sort_unstable();
        let expected: &[&str] = if relay {
            &[
                "DeferredToolPlaceholder",
                "mcp__pi__question_open",
                "mcp__pi__result_publish",
                "mcp__pi__snapshot_read",
            ]
        } else {
            &[
                "DeferredToolPlaceholder",
                "mcp__pi__snapshot_list",
                "mcp__pi__snapshot_read",
            ]
        };
        if v["model"] != MODEL || v["stream"] != true || names != expected {
            return Err(format!("projection: {names:?}").into());
        }
        let receipt = json!({"route":route,"requestId":r.id,"bodySha256":hash(&r.body),
            "toolsSha256":hash(&serde_json::to_vec(&v["tools"])?),"retryCount":r.retry,"messages":v["messages"].as_array().map(Vec::len)});
        if route == "primary" {
            refused += 1;
            println!(
                "{}",
                json!({"type":"refused","status":429,"receipt":receipt})
            );
            std::io::stdout().flush()?;
            let body = json!({"type":"error","error":{"type":"rate_limit_error","message":"synthetic primary route limit"}}).to_string();
            write!(
                s,
                "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )?;
            continue;
        }
        served += 1;
        if scenario == "retry-head" {
            println!(
                "{}",
                json!({"type":"failed","status":500,"receipt":receipt})
            );
            std::io::stdout().flush()?;
            let body = json!({"type":"error","error":{"type":"api_error","message":"synthetic"}})
                .to_string();
            write!(
                s,
                "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )?;
            // Any later request would be a client retry reaching upstream.
            let until = Instant::now() + Duration::from_secs(20);
            while Instant::now() < until {
                for l in [&primary, &secondary] {
                    if l.accept().is_ok() {
                        println!("{}", json!({"type":"retry_reached_upstream"}));
                        return Err("retry reached upstream".into());
                    }
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            println!(
                "{}",
                json!({"type":"complete","served":served,"refused":refused,"retries":0})
            );
            return Ok(());
        }
        let reply = match served {
            n if scenario == "steps" => {
                let step = &steps[n - 1];
                let blocks = match (step["tools"].as_array(), step["tool"].as_str()) {
                    (Some(tools), None) if !tools.is_empty() => tools
                        .iter()
                        .map(|t| {
                            t["tool"]
                                .as_str()
                                .map(|name| Block::Tool(name.into(), t["input"].clone()))
                                .ok_or("step tool")
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    (None, Some(tool)) => vec![Block::Tool(tool.into(), step["input"].clone())],
                    (None, None) => vec![Block::Text(
                        step["text"].as_str().ok_or("step text")?.into(),
                    )],
                    _ => return Err("step shape".into()),
                };
                sse(n, &blocks)
            }
            1 if relay => sse(
                1,
                &[Block::Tool(
                    "mcp__pi__snapshot_read".into(),
                    json!({"snapshot_id":"snapshot:serve-qual","member_id":"docs/claim.md","offset":0,"length":4096}),
                )],
            ),
            2 if relay => sse(
                2,
                &[Block::Tool(
                    "mcp__pi__result_publish".into(),
                    json!({"result_id":"result-1","body":"The claim states a 200,000-token threshold.",
                        "source_refs":[{"id":"src-1","owner_namespace":"task","owner_ref":"owner-1","revision":1,"sha256":"1".repeat(64)}],
                        "attribution":"native:claude-haiku-5-5"}),
                )],
            ),
            n if relay => sse(
                n,
                &[Block::Text(if n == 3 {
                    "Published result-1 from the claim member.".into()
                } else {
                    "I published result-1 citing the claim member.".into()
                })],
            ),
            1 => sse(
                1,
                &[Block::Tool("mcp__pi__snapshot_list".into(), json!({}))],
            ),
            2 => sse(
                2,
                &[Block::Tool(
                    "mcp__pi__snapshot_read".into(),
                    json!({"path":first_member(&v).ok_or("member list absent")?}),
                )],
            ),
            n => sse(
                n,
                &[Block::Text(
                    "Supported: the snapshot member was read. Unsupported: none. Unknown: none."
                        .into(),
                )],
            ),
        };
        println!(
            "{}",
            json!({"type":"served","effect":served,"receipt":receipt,"responseSha256":hash(&reply)})
        );
        std::io::stdout().flush()?;
        write!(
            s,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nanthropic-ratelimit-unified-status: allowed\r\nanthropic-ratelimit-unified-overage-status: rejected\r\nrequest-id: req_{served}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
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
        eprintln!("hosted_messages_observer: {e}");
        std::process::exit(1);
    }
}
