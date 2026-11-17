//! Closed grammar of one settled hosted analysis turn (no shell, no workspace
//! effect): the prompt, any number of provider requests, sequential calls of
//! the Host-served read-only snapshot tools, and a final text answer. Every
//! tool result the SDK recorded as executed must equal the Host delivery the
//! lifecycle gate recorded as trusted ingress (`pi.analysis_read`); a call the
//! SDK refused before execution is an error result with no Host delivery.
use crate::StoreError;
use crate::pi_coding_proof::{Parser, history, request, text};
use serde_json::{Value, json};
use work_engine_types::CodecContract;

/// The tools the analysis profile admits; any other name is an SDK refusal.
pub(crate) const ANALYSIS_TOOLS: [&str; 2] = ["snapshot_list", "snapshot_read"];
const MAX_ANSWER_BYTES: usize = 65_536;
const MAX_ROUNDS: usize = 64;

fn bad<T>() -> Result<T, StoreError> {
    Err(StoreError::ClaimConflict)
}
fn hash(b: &[u8]) -> String {
    CodecContract::BinaryArtifactV1
        .digest_binary(b)
        .expect("binary codec")
        .hex()
}
/// The Host delivery for one executed call, checked against its own bytes.
fn delivery<'a>(reads: &[&'a Value], inv: &str, call: &Value) -> Result<&'a Value, StoreError> {
    let id = call["id"].as_str().ok_or(StoreError::ClaimConflict)?;
    let matching: Vec<_> = reads.iter().filter(|r| r["call_id"] == id).collect();
    let [read] = matching.as_slice() else {
        return bad();
    };
    let raw = read["artifact_bytes"]
        .as_str()
        .ok_or(StoreError::ClaimConflict)?;
    let delivered: Value = serde_json::from_str(raw).map_err(|_| StoreError::ClaimConflict)?;
    if read["invocationId"] != inv
        || read["tool"] != call["name"]
        || read["arguments"] != call["arguments"]
        || hash(raw.as_bytes()) != read["artifact"]["sha256"]
        || delivered != read["result"]
        || delivered["type"] != "snapshot_result"
        || delivered["id"] != id
        || !delivered["ok"].is_boolean()
    {
        return bad();
    }
    Ok(read)
}
/// What the receiver returns to the SDK for a Host delivery.
fn expected_content(delivered: &Value) -> Value {
    if delivered["ok"] == true {
        delivered["result"].clone()
    } else {
        json!({"error": delivered["error"]})
    }
}

/// Validate the turn and return the final assistant text.
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate(
    values: &[Value],
    inv: &str,
    correlation: &str,
    thread: &str,
    prompt: &[u8],
    prompt_hash: &str,
) -> Result<String, StoreError> {
    if values.iter().any(|v| {
        v["type"]
            .as_str()
            .is_some_and(|t| t.starts_with("pi.coding_"))
    }) {
        return bad();
    }
    let reads: Vec<&Value> = values
        .iter()
        .filter(|v| v["type"] == "pi.analysis_read")
        .collect();
    let position = |pred: &dyn Fn(&Value) -> bool| values.iter().position(pred);
    let mut p = Parser {
        v: values
            .iter()
            .filter(|v| v["type"] != "pi.analysis_read")
            .collect(),
        i: 0,
    };
    p.take("agent_start")?;
    p.take("turn_start")?;
    let mut generated = Vec::new();
    if p.v
        .get(p.i)
        .is_some_and(|v| v["type"] == "message_start" && v["message"]["role"] == "system")
    {
        generated.push(p.pair("system")?);
    }
    let user = p.pair("user")?;
    if text(&user)?.as_bytes() != prompt {
        return bad();
    }
    generated.push(user);
    let mut used = 0usize;
    let mut rounds = 0usize;
    let final_message = loop {
        rounds += 1;
        if rounds > MAX_ROUNDS {
            return bad();
        }
        if rounds > 1 {
            p.take("turn_start")?;
        }
        request(
            p.take("pi.provider_request")?,
            inv,
            correlation,
            thread,
            prompt_hash,
            &generated,
        )?;
        let message = p.assistant()?;
        let content = message["content"]
            .as_array()
            .ok_or(StoreError::ClaimConflict)?;
        if content
            .iter()
            .any(|c| c["type"] != "text" && c["type"] != "toolCall")
        {
            return bad();
        }
        let calls: Vec<&Value> = content.iter().filter(|c| c["type"] == "toolCall").collect();
        if message["stopReason"] == "stop" && calls.is_empty() {
            break message;
        }
        if message["stopReason"] != "toolUse" || calls.is_empty() {
            return bad();
        }
        let mut results = Vec::new();
        for call in calls {
            let id = call["id"].as_str().ok_or(StoreError::ClaimConflict)?;
            let start = p.take("tool_execution_start")?;
            let end = p.take("tool_execution_end")?;
            let tool = p.pair("toolResult")?;
            if start["toolCallId"] != id
                || start["toolName"] != call["name"]
                || start["args"] != call["arguments"]
                || end["toolCallId"] != id
                || end["toolName"] != call["name"]
                || tool["toolCallId"] != id
                || tool["toolName"] != call["name"]
                || tool["isError"] != end["isError"]
                || end["result"] != json!({"content":tool["content"],"details":tool["details"]})
            {
                return bad();
            }
            let executed = end["isError"] == false;
            if executed {
                // Executed only through the Host: one delivery, recorded between
                // this call's start and end, equal to what the SDK returned.
                let read = delivery(&reads, inv, call)?;
                let delivered = &read["result"];
                let native = tool["content"]
                    .as_array()
                    .filter(|a| a.len() == 1 && a[0]["type"] == "text")
                    .and_then(|a| a[0]["text"].as_str())
                    .and_then(|t| serde_json::from_str::<Value>(t).ok());
                let at = position(&|v| v["type"] == "pi.analysis_read" && v["call_id"] == id);
                let s = position(&|v| v["type"] == "tool_execution_start" && v["toolCallId"] == id);
                let e = position(&|v| v["type"] == "tool_execution_end" && v["toolCallId"] == id);
                if !ANALYSIS_TOOLS.contains(&call["name"].as_str().unwrap_or(""))
                    || native.as_ref() != Some(&expected_content(delivered))
                    || tool["details"] != json!({"hostReportedSuccess": delivered["ok"]})
                    || !matches!((s, at, e), (Some(s), Some(at), Some(e)) if s < at && at < e)
                {
                    return bad();
                }
                used += 1;
            } else if end["isError"] != true || reads.iter().any(|r| r["call_id"] == id) {
                // An SDK refusal never reached the Host.
                return bad();
            }
            results.push(tool);
        }
        let turn = p.take("turn_end")?;
        if turn["message"] != message || turn["toolResults"] != json!(results) {
            return bad();
        }
        generated.push(message);
        generated.extend(results);
    };
    // Every Host delivery belongs to exactly one executed call.
    if used != reads.len() {
        return bad();
    }
    let answer = text(&final_message)?;
    if answer.is_empty() || answer.len() > MAX_ANSWER_BYTES {
        return bad();
    }
    let turn = p.take("turn_end")?;
    if turn["message"] != final_message || turn["toolResults"] != json!([]) {
        return bad();
    }
    generated.push(final_message);
    let end = p.take("agent_end")?;
    if end["willRetry"] != false
        || end["messages"] != json!(generated)
        || p.take("agent_settled")?["aborted"] != false
    {
        return bad();
    }
    history(p.take("pi.session_history")?, thread, &generated, None)?;
    if p.i != p.v.len() {
        return bad();
    }
    Ok(answer)
}
