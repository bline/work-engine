//! Anthropic Messages SSE reading: message id, stop reason, tool_use blocks.

use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Parsed {
    pub message_id: Option<String>,
    pub stop_reason: Option<String>,
    pub tool_uses: Vec<ToolUse>,
    pub text: String,
    pub error: Option<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// Parse a complete retained SSE body. Malformed JSON in a data line, a
/// tool_use whose input is not valid JSON, or a missing `message_stop` leaves
/// `complete` false.
pub fn parse(body: &[u8]) -> Parsed {
    let mut out = Parsed::default();
    let mut blocks: BTreeMap<u64, (String, String, String)> = BTreeMap::new();
    let text = String::from_utf8_lossy(body);
    let mut ok = true;
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let Ok(event) = serde_json::from_str::<Value>(data.trim_start()) else {
            ok = false;
            continue;
        };
        match event["type"].as_str() {
            Some("message_start") => {
                out.message_id = event["message"]["id"].as_str().map(str::to_owned);
            }
            Some("content_block_start") => {
                let index = event["index"].as_u64().unwrap_or(u64::MAX);
                let block = &event["content_block"];
                if block["type"] == "tool_use" {
                    blocks.insert(
                        index,
                        (
                            block["id"].as_str().unwrap_or_default().to_owned(),
                            block["name"].as_str().unwrap_or_default().to_owned(),
                            String::new(),
                        ),
                    );
                }
            }
            Some("content_block_delta") => {
                let index = event["index"].as_u64().unwrap_or(u64::MAX);
                let delta = &event["delta"];
                match delta["type"].as_str() {
                    Some("input_json_delta") => {
                        if let Some(b) = blocks.get_mut(&index) {
                            b.2.push_str(delta["partial_json"].as_str().unwrap_or_default());
                        }
                    }
                    Some("text_delta") => out.text.push_str(delta["text"].as_str().unwrap_or("")),
                    _ => {}
                }
            }
            Some("message_delta") => {
                if let Some(r) = event["delta"]["stop_reason"].as_str() {
                    out.stop_reason = Some(r.to_owned());
                }
            }
            Some("message_stop") => out.complete = true,
            Some("error") => {
                out.error = Some(event["error"].to_string());
            }
            _ => {}
        }
    }
    for (_, (id, name, json)) in blocks {
        let input = if json.is_empty() {
            Ok(Value::Object(Default::default()))
        } else {
            serde_json::from_str(&json)
        };
        match input {
            Ok(input) if !id.is_empty() && !name.is_empty() => {
                out.tool_uses.push(ToolUse { id, name, input })
            }
            _ => ok = false,
        }
    }
    out.complete &= ok && out.message_id.is_some() && out.error.is_none();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOOL: &str = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\"}}\n\nevent: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"mcp__we__snapshot_read\",\"input\":{}}}\n\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"member_id\\\":\"}}\n\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"a\\\"}\"}}\n\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"}}\n\ndata: {\"type\":\"message_stop\"}\n\n";

    #[test]
    fn assembles_tool_use_input_and_identity() {
        let p = parse(TOOL.as_bytes());
        assert!(p.complete);
        assert_eq!(p.message_id.as_deref(), Some("msg_1"));
        assert_eq!(p.stop_reason.as_deref(), Some("tool_use"));
        assert_eq!(p.tool_uses.len(), 1);
        assert_eq!(p.tool_uses[0].id, "toolu_1");
        assert_eq!(p.tool_uses[0].input["member_id"], "a");
    }

    #[test]
    fn truncated_or_malformed_is_incomplete() {
        let cut = &TOOL[..TOOL.find("data: {\"type\":\"message_stop\"").unwrap()];
        assert!(!parse(cut.as_bytes()).complete);
        let bad = TOOL.replace("\\\"a\\\"}", "\\\"a\\\"");
        assert!(!parse(bad.as_bytes()).complete);
        let err = format!(
            "{TOOL}data: {{\"type\":\"error\",\"error\":{{\"type\":\"overloaded_error\"}}}}\n\n"
        );
        assert!(!parse(err.as_bytes()).complete);
    }

    #[test]
    fn collects_text() {
        let body = "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\"}}\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"he\"}}\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"llo\"}}\ndata: {\"type\":\"message_stop\"}\n";
        let p = parse(body.as_bytes());
        assert!(p.complete);
        assert_eq!(p.text, "hello");
    }
}
