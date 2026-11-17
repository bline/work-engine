//! MCP stdio relay launched by Claude Code. Lists exactly the pinned tools and
//! forwards each call to the Host over a Unix socket. It executes nothing.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

pub const MAX_LINE: usize = 8 * 1024 * 1024;

/// Host-bound message: one relayed tool call.
pub fn call_message(seq: u64, tool_use_id: &str, tool: &str, arguments: &Value) -> Value {
    json!({"seq":seq,"tool_use_id":tool_use_id,"tool":tool,"arguments":arguments})
}

pub fn run(socket: &Path, tools_path: &Path, log_path: &Path) -> Result<(), String> {
    let tools: Vec<Value> =
        serde_json::from_slice(&std::fs::read(tools_path).map_err(|e| format!("tools spec: {e}"))?)
            .map_err(|e| format!("tools spec: {e}"))?;
    let names: Vec<String> = tools
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_owned))
        .collect();
    let host = UnixStream::connect(socket).map_err(|e| format!("host socket: {e}"))?;
    let mut host_reader = BufReader::new(host.try_clone().map_err(|e| e.to_string())?);
    let mut host_writer = host;
    let mut log = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_path)
        .map_err(|e| format!("relay log: {e}"))?;
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut seq = 0u64;
    let mut input = BufReader::new(stdin.lock());
    loop {
        let mut line = String::new();
        let n = std::io::Read::take(&mut input, MAX_LINE as u64)
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        if line.trim().is_empty() {
            continue;
        }
        let _ = writeln!(log, "{{\"in\":{}}}", line.trim_end());
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let id = req.get("id").cloned();
        let method = req["method"].as_str().unwrap_or("");
        let reply = match method {
            "initialize" => Some(
                json!({"protocolVersion":req["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
                "capabilities":{"tools":{}},"serverInfo":{"name":crate::spec::SERVER,"version":"0.1.0"}}),
            ),
            "tools/list" => Some(json!({"tools":tools})),
            "ping" => Some(json!({})),
            "tools/call" => {
                let tool = req["params"]["name"].as_str().unwrap_or("");
                if !names.iter().any(|n| n == tool) {
                    Some(json!({"content":[{"type":"text","text":"unknown tool"}],"isError":true}))
                } else {
                    seq += 1;
                    let tool_use_id = req["params"]["_meta"]["claudecode/toolUseId"]
                        .as_str()
                        .unwrap_or("");
                    let msg = call_message(seq, tool_use_id, tool, &req["params"]["arguments"]);
                    writeln!(host_writer, "{msg}").map_err(|e| e.to_string())?;
                    let mut answer = String::new();
                    std::io::Read::take(&mut host_reader, MAX_LINE as u64)
                        .read_line(&mut answer)
                        .map_err(|e| e.to_string())?;
                    let answer: Value =
                        serde_json::from_str(&answer).map_err(|_| "host answer".to_owned())?;
                    if answer["seq"] != seq {
                        return Err("host answer out of order".into());
                    }
                    Some(json!({"content":[{"type":"text","text":answer["text"]}],
                        "isError":answer["is_error"].as_bool().unwrap_or(true)}))
                }
            }
            _ => None,
        };
        let Some(id) = id else { continue };
        let msg = match reply {
            Some(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
            None => {
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"method not found"}})
            }
        };
        let _ = writeln!(log, "{{\"out\":{msg}}}");
        writeln!(stdout, "{msg}").map_err(|e| e.to_string())?;
        stdout.flush().map_err(|e| e.to_string())?;
    }
}
