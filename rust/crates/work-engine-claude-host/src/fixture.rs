//! Scripted Anthropic Messages SSE upstream for offline qualification.
//! A request without a tool result gets a `snapshot_read` tool_use for the
//! configured member; a request with tool results gets the final text.

use crate::http;
use serde_json::{Value, json};
use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Debug)]
pub struct Script {
    pub snapshot_id: String,
    pub member_id: String,
    pub length: u64,
    /// Name the model "calls"; a name outside the pinned set exercises refusal paths.
    pub tool_name: String,
}

pub struct Fixture {
    pub port: u16,
    pub hits: Arc<AtomicU32>,
}

fn sse(events: &[Value]) -> Vec<u8> {
    let mut out = Vec::new();
    for e in events {
        out.extend_from_slice(
            format!(
                "event: {}\ndata: {}\n\n",
                e["type"].as_str().unwrap_or(""),
                e
            )
            .as_bytes(),
        );
    }
    out
}

pub fn response_for(script: &Script, body: &Value, n: u32) -> Vec<u8> {
    let has_result = body["messages"].as_array().is_some_and(|ms| {
        ms.iter().any(|m| {
            m["role"] == "user"
                && m["content"]
                    .as_array()
                    .is_some_and(|c| c.iter().any(|b| b["type"] == "tool_result"))
        })
    });
    let start = json!({"type":"message_start","message":{"id":format!("msg_fixture_{n}"),"type":"message",
        "role":"assistant","model":body["model"],"content":[],"stop_reason":null,"stop_sequence":null,
        "usage":{"input_tokens":10,"output_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}});
    let events = if !has_result {
        let args = json!({"snapshot_id":script.snapshot_id,"member_id":script.member_id,"offset":0,"length":script.length});
        vec![
            start,
            json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":format!("toolu_fixture_{n}"),"name":script.tool_name,"input":{}}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":args.to_string()}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":20}}),
            json!({"type":"message_stop"}),
        ]
    } else {
        vec![
            start,
            json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Fixture answer: the member was read."}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":9}}),
            json!({"type":"message_stop"}),
        ]
    };
    sse(&events)
}

impl Fixture {
    pub fn start(port: u16, script: Script) -> std::io::Result<Fixture> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let port = listener.local_addr()?.port();
        let hits = Arc::new(AtomicU32::new(0));
        let h = hits.clone();
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(conn) = conn else { continue };
                let n = h.fetch_add(1, Ordering::SeqCst) + 1;
                let script = script.clone();
                std::thread::spawn(move || {
                    let _ = serve(conn, &script, n);
                });
            }
        });
        Ok(Fixture { port, hits })
    }
}

fn serve(conn: TcpStream, script: &Script, n: u32) -> std::io::Result<()> {
    let mut reader = BufReader::new(conn.try_clone()?);
    let mut w = conn;
    let Some(head) = http::read_head(&mut reader)? else {
        return Ok(());
    };
    let len = head
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let body = http::read_body(&mut reader, len, 16 * 1024 * 1024)?;
    let value: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let data = response_for(script, &value, n);
    // Chunked, as the real API answers.
    write!(
        w,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nrequest-id: req_fixture_{n}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
    )?;
    for piece in data.chunks(97) {
        http::write_chunk(&mut w, piece)?;
    }
    w.write_all(b"0\r\n\r\n")?;
    w.flush()
}
