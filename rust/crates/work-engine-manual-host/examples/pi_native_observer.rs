//! Offline root-owned observer. No auth/body logging; no live-provider route.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};
use work_engine_manual_host::pi_native;
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn response(n: usize) -> Vec<u8> {
    let call = match n {
        1 => Some((
            "write",
            json!({"path":"formula.expr","content":"sum(a,b) = a + b\n"}),
        )),
        2 => Some((
            "bash",
            json!({"command":r#"cat > build.mjs <<'BUILD'
import fs from 'node:fs';
const source=fs.readFileSync('formula.expr','utf8').trim();
const match=/^([a-z]+)\(([a-z]),([a-z])\) = ([a-z]) ([+*]) ([a-z])$/.exec(source);
if(!match||match[2]!==match[4]||match[3]!==match[6])throw Error('invalid source');
fs.writeFileSync('dist.mjs',`// compiled expression language v1\nexport function ${match[1]}(${match[2]},${match[3]}) { return Number(${match[4]}) ${match[5]} Number(${match[6]}); }\n`);
console.log('compiled formula.expr -> dist.mjs');
BUILD
cat > test.mjs <<'TEST'
import {test} from 'node:test';import assert from 'node:assert/strict';import {sum} from './dist.mjs';
test('positive addition',()=>assert.equal(sum(7,5),12));
test('signed addition',()=>assert.equal(sum(-8,3),-5));
test('coerces numeric strings',()=>assert.equal(sum('2','3'),5));
TEST
node build.mjs && node --check dist.mjs && node --test test.mjs"#,"timeout":10}),
        )),
        _ => None,
    };
    let Some((name, arguments)) = call else {
        return pi_native::sse(&json!({"schemaVersion":1,"subject":{"commit":"offline-fixture","tree":"compiled-expression-v1","patchIdentity":"native-development-v1"},"verdict":"acceptable_as_is","findings":[],"decisiveEvidence":[{"path":"dist.mjs","startLine":1,"endLine":2,"sha256":hash(b"// compiled expression language v1\nexport function sum(a,b) { return Number(a) + Number(b); }\n")}],"limitations":["Offline synthetic fixture; no production-provider qualification"]}).to_string());
    };
    let item = json!({"type":"function_call","id":format!("fc_{n}"),"call_id":format!("call_{n}"),"name":name,"arguments":arguments.to_string(),"status":"completed"});
    [json!({"type":"response.created","response":{"id":format!("resp_{n}"),"status":"in_progress"}}),
     json!({"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":format!("fc_{n}"),"call_id":format!("call_{n}"),"name":name,"arguments":"","status":"in_progress"}}),
     json!({"type":"response.function_call_arguments.delta","output_index":0,"delta":arguments.to_string()}),
     json!({"type":"response.output_item.done","output_index":0,"item":item}),
     json!({"type":"response.completed","response":{"id":format!("resp_{n}"),"status":"completed","output":[item],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}})]
    .iter().map(|v|format!("data: {v}\n\n")).collect::<String>().into_bytes()
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    println!(
        "{}",
        json!({"type":"ready","address":listener.local_addr()?.to_string(),"protocol":"offline-native-observer-v1","expectedEffects":5})
    );
    std::io::stdout().flush()?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let mut effects = BTreeMap::<String, (usize, String, Vec<u8>)>::new();
    let mut receipts = vec![];
    let mut authority = None;
    let fault = std::env::var("PI_OBSERVER_FAULT").unwrap_or_default();
    let truncate = fault == "truncate-first";
    let stall = fault == "stall-first";
    // quota-all: definitive 429 refusal for every request (a funding route
    // whose quota is exhausted); runs until the expected receipt count.
    let quota_all = fault == "quota-all";
    let quota_midstream = fault == "quota-midstream";
    let expect_receipts: usize = std::env::var("PI_OBSERVER_EXPECT_RECEIPTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    while if quota_all {
        receipts.len() < expect_receipts
    } else {
        effects.len() < 5
    } {
        let (mut s, _) = match listener.accept() {
            Ok(x) => x,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    return Err("observer deadline".into());
                }
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        s.set_read_timeout(Some(Duration::from_secs(10)))?;
        s.set_write_timeout(Some(Duration::from_secs(10)))?;
        let mut header = vec![];
        let mut byte = [0];
        while !header.ends_with(b"\r\n\r\n") {
            if header.len() > 8192 {
                return Err("header bound".into());
            }
            s.read_exact(&mut byte)?;
            header.push(byte[0]);
        }
        let header = std::str::from_utf8(&header)?;
        if !header.starts_with("POST /offline-native HTTP/1.1\r\n") {
            return Err("route".into());
        }
        let field = |name: &str| {
            header
                .lines()
                .find_map(|l| l.strip_prefix(name))
                .ok_or("header")
        };
        let auth = field("Authorization: Bearer ")?;
        if !auth.starts_with("synthetic-A-") {
            return Err("auth profile".into());
        }
        let auth = hash(auth.as_bytes());
        if authority.as_ref().is_some_and(|h| h != &auth) {
            return Err("authority changed".into());
        }
        authority = Some(auth);
        let id = field("X-Request-Id: ")?.to_string();
        if id.len() > 128 {
            return Err("request id".into());
        }
        let n: usize = field("Content-Length: ")?.parse()?;
        if n > pi_native::MAX_REQUEST {
            return Err("body bound".into());
        }
        let mut body = vec![0; n];
        s.read_exact(&mut body)?;
        let digest = hash(&body);
        let v: Value = serde_json::from_slice(&body)?;
        if v["model"] != "gpt-6-luna" || v["store"] != false {
            return Err("projection".into());
        }
        if quota_all {
            let receipt = json!({"requestId":id,"bodySha256":digest,"refused":429});
            println!("{}", json!({"type":"receipt","receipt":receipt}));
            std::io::stdout().flush()?;
            receipts.push(receipt);
            let body = json!({"error":{"code":"insufficient_quota","message":"synthetic route quota exhausted"}}).to_string();
            write!(
                s,
                "HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )?;
            continue;
        }
        let duplicate = effects.contains_key(&id);
        let next = effects.len() + 1;
        let (effect_id, prior, reply) = effects
            .entry(id.clone())
            .or_insert_with(|| (next, digest.clone(), response(next)));
        if prior != &digest {
            return Err("duplicate body mismatch".into());
        }
        let receipt = json!({"requestId":id,"bodySha256":digest,"duplicate":duplicate,"effectId":format!("effect-{effect_id}")});
        println!("{}", json!({"type":"receipt","receipt":receipt}));
        if !duplicate {
            println!(
                "{}",
                json!({"type":"effect","effectId":format!("effect-{effect_id}"),"requestId":id,"responseSha256":hash(reply)})
            );
        }
        std::io::stdout().flush()?;
        receipts.push(receipt);
        // Incremental chunked SSE: each event is flushed separately and the
        // endpoint stays open between them, so early native consumption is
        // observable before the framed end.
        write!(
            s,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
        )?;
        s.flush()?;
        let failed_reply;
        let reply: &Vec<u8> = if quota_midstream && *effect_id == 1 {
            // Quota failure after output began: two normal events, then a
            // provider failure event, then a correctly framed end.
            let t = std::str::from_utf8(reply)?;
            let mut events: Vec<&str> = t.split_inclusive("\n\n").take(2).collect();
            let failed = format!(
                "data: {}\n\n",
                json!({"type":"response.failed","response":{"id":"resp_1","status":"failed","error":{"code":"insufficient_quota","message":"synthetic quota exhausted mid-stream"}}})
            );
            events.push(&failed);
            failed_reply = events.concat().into_bytes();
            &failed_reply
        } else {
            reply
        };
        let text = std::str::from_utf8(reply)?;
        let mut sent = vec![];
        for (i, event) in text.split_inclusive("\n\n").enumerate() {
            if stall && i == 2 && !duplicate && *effect_id == 1 {
                // Fault control: hold the stream open mid-response and observe
                // when (whether) the host closes the connection.
                let at = Instant::now();
                s.set_read_timeout(Some(Duration::from_secs(20)))?;
                let mut b = [0u8; 1];
                let closed = matches!(s.read(&mut b), Ok(0) | Err(_))
                    && at.elapsed() < Duration::from_secs(20);
                println!(
                    "{}",
                    json!({"type":"fault","fault":"stall","requestId":id,"eventSentMs":sent,"stalledMs":at.elapsed().as_millis() as u64,"peerClosed":closed,"closedMs":pi_native::now()})
                );
                std::io::stdout().flush()?;
                return Err("injected stall fault".into());
            }
            if truncate && i == 2 && !duplicate && *effect_id == 1 {
                // Fault control: close mid-stream without the framed end.
                println!(
                    "{}",
                    json!({"type":"fault","fault":"truncate","requestId":id,"eventSentMs":sent})
                );
                std::io::stdout().flush()?;
                return Err("injected truncate fault".into());
            }
            if i > 0 {
                std::thread::sleep(Duration::from_millis(150));
            }
            write!(s, "{:x}\r\n{event}\r\n", event.len())?;
            s.flush()?;
            sent.push(pi_native::now());
        }
        write!(s, "0\r\n\r\n")?;
        s.flush()?;
        sent.push(pi_native::now());
        println!(
            "{}",
            json!({"type":"stream-sent","requestId":id,"duplicate":duplicate,"eventSentMs":sent})
        );
        std::io::stdout().flush()?;
    }
    println!(
        "{}",
        json!({"type":"complete","classification":"root-owned-offline-observer","requestReceipts":receipts,"receiptCount":receipts.len(),"deduplicatedEffects":effects.len(),"rawAuthOrBodyRetained":false})
    );
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!("offline_observer_unknown");
        std::process::exit(1);
    }
}
