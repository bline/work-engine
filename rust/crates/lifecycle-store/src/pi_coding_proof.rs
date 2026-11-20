//! Closed v3 (provider-disabled) and v4 (live provider) grammars over immutable
//! raw SDK and trusted Host ingress. v4 differs only where real inference
//! differs from the fixture: the model's final text is provenance, the
//! checkpoint is derived here from the exact Host result, workspace manifest
//! and settled history, and an assistant message may carry non-semantic
//! thinking blocks of the closed observed shape. v3 behaviour is unchanged.
use crate::StoreError;
use serde_json::{Value, json};
use work_engine_types::CodecContract;
/// Identifies the owner-derived v4 checkpoint document.
pub(crate) const LIVE_CHECKPOINT_PROFILE: &str = "work-engine.pi-coding-live-continuity.v1";
const MAX_CHECKPOINT_BYTES: usize = 65_536;
fn bad<T>() -> Result<T, StoreError> {
    Err(StoreError::ClaimConflict)
}
fn hash(b: &[u8]) -> String {
    CodecContract::BinaryArtifactV1
        .digest_binary(b)
        .expect("binary codec")
        .hex()
}
pub(crate) fn text(m: &Value) -> Result<String, StoreError> {
    if let Some(s) = m["content"].as_str() {
        return Ok(s.into());
    }
    let a = m["content"].as_array().ok_or(StoreError::ClaimConflict)?;
    let mut out = String::new();
    for v in a {
        if v["type"] != "text" {
            return bad();
        }
        out.push_str(v["text"].as_str().ok_or(StoreError::ClaimConflict)?);
    }
    Ok(out)
}
pub(crate) struct Parser<'a> {
    pub(crate) v: Vec<&'a Value>,
    pub(crate) i: usize,
}
impl<'a> Parser<'a> {
    pub(crate) fn take(&mut self, k: &str) -> Result<&'a Value, StoreError> {
        let v = *self.v.get(self.i).ok_or(StoreError::ClaimConflict)?;
        if v["type"] != k {
            return bad();
        }
        self.i += 1;
        Ok(v)
    }
    pub(crate) fn pair(&mut self, role: &str) -> Result<Value, StoreError> {
        let a = self.take("message_start")?;
        let b = self.take("message_end")?;
        if a["message"] != b["message"] || b["message"]["role"] != role {
            return bad();
        }
        Ok(b["message"].clone())
    }
    pub(crate) fn assistant(&mut self) -> Result<Value, StoreError> {
        let a = self.take("message_start")?;
        if a["message"]["role"] != "assistant" {
            return bad();
        }
        while self
            .v
            .get(self.i)
            .is_some_and(|v| v["type"] == "message_update")
        {
            self.take("message_update")?;
        }
        let m = self.take("message_end")?["message"].clone();
        if m["role"] != "assistant" {
            return bad();
        }
        Ok(m)
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn request(
    v: &Value,
    inv: &str,
    correlation: &str,
    thread: &str,
    prompt_hash: &str,
    expected: &[Value],
) -> Result<(), StoreError> {
    if v["producer"] != "pi-1.5.0"
        || v["sessionId"] != thread
        || v["invocationId"] != inv
        || v["turnCorrelation"] != correlation
        || v["prompt_sha256"] != prompt_hash
        || v["agentMessages"] != json!(expected)
    {
        return bad();
    }
    let input = v["messages"].as_array().ok_or(StoreError::ClaimConflict)?;
    if input.len() != expected.len()
        || input.iter().zip(expected).any(|(a, b)| {
            // Pi's system input includes sections and tool declarations outside
            // content. Preserve the exact native-owned system object, including
            // its position/count, rather than silently discarding that input.
            (b["role"] == "system" && a != b)
                || a["role"] != b["role"]
                || a["content"] != b["content"]
                || (b["role"] == "toolResult"
                    && (a["toolCallId"] != b["toolCallId"] || a["toolName"] != b["toolName"]))
        })
    {
        return bad();
    }
    Ok(())
}
pub(crate) fn history(
    v: &Value,
    thread: &str,
    full: &[Value],
    anchor: Option<&Value>,
) -> Result<(), StoreError> {
    if v["type"] != "pi.session_history"
        || v["producer"] != "pi-1.5.0"
        || v["sessionId"] != thread
        || v["header"]["id"] != thread
        || v["header"]["type"] != "session"
        || v["messages"] != json!(full)
    {
        return bad();
    }
    let branch = v["branch"].as_array().ok_or(StoreError::ClaimConflict)?;
    let mut parent = Value::Null;
    let mut ids = std::collections::BTreeSet::new();
    for row in branch {
        let id = row["id"].as_str().ok_or(StoreError::ClaimConflict)?;
        if row["parentId"] != parent
            || !ids.insert(id)
            || !matches!(
                row["type"].as_str(),
                Some("message" | "model_change" | "thinking_level_change")
            )
        {
            return bad();
        }
        parent = json!(id);
    }
    if v["leafId"] != parent
        || branch
            .iter()
            .filter(|r| r["type"] == "message")
            .map(|r| r["message"].clone())
            .collect::<Vec<_>>()
            != full
    {
        return bad();
    }
    if let Some(a) = anchor {
        let prior = a["branch"].as_array().ok_or(StoreError::ClaimConflict)?;
        if v["header"] != a["header"]
            || branch.len() < prior.len()
            || branch[..prior.len()] != prior[..]
            || branch[prior.len()..].iter().any(|r| r["type"] != "message")
        {
            return bad();
        }
    }
    Ok(())
}
fn host_result(intent: &Value, e: &Value, tool: &Value) -> Result<(), StoreError> {
    if e["intent"] != *intent
        || e["result"]["type"] != "tool_result"
        || e["result"]["id"] != intent["call_id"]
    {
        return bad();
    }
    let raw = e["artifact_bytes"]
        .as_str()
        .ok_or(StoreError::ClaimConflict)?;
    if hash(raw.as_bytes()) != e["artifact"]["sha256"]
        || serde_json::from_str::<Value>(raw).map_err(|_| StoreError::ClaimConflict)? != e["result"]
    {
        return bad();
    }
    let stdout: Vec<u8> =
        serde_json::from_value(e["stdout_bytes"].clone()).map_err(|_| StoreError::ClaimConflict)?;
    let stderr: Vec<u8> =
        serde_json::from_value(e["stderr_bytes"].clone()).map_err(|_| StoreError::ClaimConflict)?;
    let p = &e["process"];
    let argv = p["argv"].as_array().ok_or(StoreError::ClaimConflict)?;
    if stdout.len() + stderr.len() > 60000
        || p["timed_out"] != false
        || p["leftovers"] != json!([])
        || !p["pid"].is_u64()
        || !p["start"].is_string()
        || !p["status"].is_i64()
        || p["stdout_sha256"] != hash(&stdout)
        || p["stderr_sha256"] != hash(&stderr)
        || argv.len() != 8
        || argv[1..4] != [json!("exec"), json!("--workdir"), json!("/workspace")]
        || argv[5..7] != [json!("/bin/sh"), json!("-c")]
        || hash(
            argv[7]
                .as_str()
                .ok_or(StoreError::ClaimConflict)?
                .as_bytes(),
        ) != intent["command_sha256"]
        || e["tool_container"]
            .as_str()
            .is_none_or(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return bad();
    }
    let ok = p["status"] == 0;
    let output = format!(
        "Host command success: {ok}\n{}\n[stderr]\n{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
    if e["result"] != json!({"type":"tool_result","id":intent["call_id"],"ok":ok,"output":output})
        || tool["role"] != "toolResult"
        || tool["toolCallId"] != intent["call_id"]
        || tool["toolName"] != "workspace_exec"
        || tool["isError"] != false
        || tool["content"] != json!([{"type":"text","text":output}])
        || tool["details"] != json!({"hostReportedSuccess":ok})
    {
        return bad();
    }
    Ok(())
}
/// Validated outcome of one settled turn. `final_text` is always the actual
/// assistant text. `checkpoint` exists only for a tool-bearing live (v4) turn
/// and is derived from validated artifacts, never from model or caller text.
pub(super) struct Validated {
    pub final_text: String,
    pub checkpoint: Option<String>,
}
#[allow(clippy::too_many_arguments)]
pub(super) fn validate_with_thinking(
    values: &[Value],
    inv: &str,
    correlation: &str,
    thread: &str,
    prompt: &[u8],
    prompt_hash: &str,
    anchor: Option<&Value>,
    live: bool,
    thinking: bool,
    // A successor-rehydrate acknowledgment turn (operator decision
    // 2026-11-20, option B): a real reasoning model handed the owner
    // checkpoint may spend its whole output on reasoning and emit no visible
    // text (observed live: records/openrouter-lane-live-20261120/run-2). Such
    // a turn settles with an empty `final_text`; the recovery proof is the
    // settled history itself, and the follow-on continuation turn still
    // proves real work. Domain-work turns keep the non-empty-final rule.
    rehydrate: bool,
    // The successor context generation embedded in the owner-derived v4
    // checkpoint. The 4-request continuity choreography uses the original
    // literal `context:new`; the battery profile's parametrized replacement
    // cycles name their own successor generation (cycle index i: `context:newi`).
    // The checkpoint is derived, never stored, so the label must only agree
    // across the derivations of one cycle.
    successor_context: &str,
) -> Result<Validated, StoreError> {
    if thinking && !live {
        return bad();
    }
    // Non-semantic reasoning a real provider attaches to an assistant message:
    // v5's native shape, or for v4 the closed observed shape (e.g. OpenRouter
    // Completions reasoning at the operator-fixed effort). Never a tool call,
    // never final text; retained only in the settled history. v3 admits none.
    let reasoning = |c: &Value| {
        if thinking {
            valid_thinking(c)
        } else {
            live && crate::pi_analysis_proof::thinking(c)
        }
    };
    let intents: Vec<_> = values
        .iter()
        .filter(|v| v["type"] == "pi.coding_tool_intent")
        .collect();
    let results: Vec<_> = values
        .iter()
        .filter(|v| v["type"] == "pi.coding_tool_result")
        .collect();
    let workspaces: Vec<_> = values
        .iter()
        .filter(|v| v["type"] == "pi.coding_workspace")
        .collect();
    let has_tool = !intents.is_empty();
    if intents.len() > 1
        || results.len() != intents.len()
        || workspaces.len() != intents.len()
        || (has_tool && anchor.is_some())
    {
        return bad();
    }
    let mut p = Parser {
        v: values
            .iter()
            .filter(|v| {
                !matches!(
                    v["type"].as_str(),
                    Some("pi.coding_tool_intent" | "pi.coding_tool_result" | "pi.coding_workspace")
                )
            })
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
        if anchor.is_some() {
            return bad();
        }
        generated.push(p.pair("system")?);
    }
    let user = p.pair("user")?;
    if text(&user)?.as_bytes() != prompt {
        return bad();
    }
    generated.push(user);
    let mut full = anchor
        .map(|a| {
            a["messages"]
                .as_array()
                .cloned()
                .ok_or(StoreError::ClaimConflict)
        })
        .transpose()?
        .unwrap_or_default();
    full.extend(generated.clone());
    request(
        p.take("pi.provider_request")?,
        inv,
        correlation,
        thread,
        prompt_hash,
        &full,
    )?;
    let first = p.assistant()?;
    let final_message = if has_tool {
        let intent = &intents[0]["intent"];
        if intent.as_object().is_none_or(|o| o.len() != 5)
            || intent["job_id"].as_str().is_none_or(str::is_empty)
            || intent["call_id"]
                .as_str()
                .is_none_or(|s| s.is_empty() || s.len() >= 128)
            || ["input_digest", "command_sha256"].iter().any(|k| {
                intent[*k]
                    .as_str()
                    .is_none_or(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
            })
        {
            return bad();
        }

        if intents[0]["invocationId"] != inv
            || results[0]["invocationId"] != inv
            || intent["session_id"] != thread
            || first["stopReason"] != "toolUse"
        {
            return bad();
        }
        let content = first["content"]
            .as_array()
            .ok_or(StoreError::ClaimConflict)?;
        // v3: exactly the toolCall. v4/v5: real models may precede the one call
        // with text or reasoning; any other block kind or a second call is refused.
        let calls = content.iter().filter(|c| c["type"] == "toolCall").count();
        if calls != 1
            || content.last().is_none_or(|c| c["type"] != "toolCall")
            || (!live && content.len() != 1)
            || content
                .iter()
                .any(|c| c["type"] != "toolCall" && c["type"] != "text" && !reasoning(c))
        {
            return bad();
        }
        let call = &content[content.len() - 1];
        let command = call["arguments"]["command"]
            .as_str()
            .ok_or(StoreError::ClaimConflict)?;
        if call["type"] != "toolCall"
            || call["id"] != intent["call_id"]
            || call["name"] != "workspace_exec"
            || call["arguments"].as_object().is_none_or(|a| a.len() != 1)
            || hash(command.as_bytes()) != intent["command_sha256"]
        {
            return bad();
        }
        let start = p.take("tool_execution_start")?;
        let end = p.take("tool_execution_end")?;
        if start["toolCallId"] != intent["call_id"]
            || start["toolName"] != "workspace_exec"
            || start["args"] != call["arguments"]
            || end["toolCallId"] != intent["call_id"]
            || end["toolName"] != "workspace_exec"
            || end["isError"] != false
        {
            return bad();
        }
        let tool = p.pair("toolResult")?;
        host_result(intent, results[0], &tool)?;
        if end["result"] != json!({"content":tool["content"],"details":tool["details"]}) {
            return bad();
        }
        let turn = p.take("turn_end")?;
        if turn["message"] != first || turn["toolResults"] != json!([tool]) {
            return bad();
        }
        let index = |kind: &str| {
            values
                .iter()
                .position(|v| v["type"] == kind)
                .ok_or(StoreError::ClaimConflict)
        };
        if !(index("tool_execution_start")? < index("pi.coding_tool_intent")?
            && index("pi.coding_tool_intent")? < index("pi.coding_tool_result")?
            && index("pi.coding_tool_result")? < index("tool_execution_end")?
            && index("tool_execution_end")? < index("pi.coding_workspace")?)
        {
            return bad();
        }
        let requests: Vec<_> = values
            .iter()
            .enumerate()
            .filter(|(_, v)| v["type"] == "pi.provider_request")
            .map(|(i, _)| i)
            .collect();
        if requests.len() != 2 || index("pi.coding_workspace")? >= requests[1] {
            return bad();
        }
        let workspace = workspaces[0];
        let raw = workspace["manifest_bytes"]
            .as_str()
            .ok_or(StoreError::ClaimConflict)?;
        if workspace["invocationId"] != inv
            || hash(raw.as_bytes()) != workspace["manifest"]["sha256"]
            || serde_json::from_str::<Value>(raw)
                .map_err(|_| StoreError::ClaimConflict)?
                .as_object()
                .is_none()
        {
            return bad();
        }
        generated.extend([first, tool]);
        full = anchor
            .map(|a| {
                a["messages"]
                    .as_array()
                    .cloned()
                    .ok_or(StoreError::ClaimConflict)
            })
            .transpose()?
            .unwrap_or_default();
        full.extend(generated.clone());
        p.take("turn_start")?;
        request(
            p.take("pi.provider_request")?,
            inv,
            correlation,
            thread,
            prompt_hash,
            &full,
        )?;
        p.assistant()?
    } else {
        first
    };
    if final_message["stopReason"] != "stop" {
        return bad();
    }
    // The final text is the text blocks only; a reasoning-only final has none
    // and refuses below.
    let final_text = if thinking || (live && final_message["content"].is_array()) {
        let mut m = final_message.clone();
        let blocks = m["content"]
            .as_array_mut()
            .ok_or(StoreError::ClaimConflict)?;
        if blocks.iter().any(|v| v["type"] != "text" && !reasoning(v)) {
            return bad();
        }
        blocks.retain(|v| v["type"] == "text");
        text(&m)?
    } else {
        text(&final_message)?
    };
    if (final_text.is_empty() && !rehydrate) || final_text.len() > 65536 {
        return bad();
    }
    if has_tool && !live {
        let checkpoint: Value =
            serde_json::from_str(&final_text).map_err(|_| StoreError::ClaimConflict)?;
        if checkpoint["coding_result"] != results[0]["result"]
            || checkpoint["coding_result_sha256"] != results[0]["artifact"]["sha256"]
            || checkpoint["workspace_manifest_sha256"] != workspaces[0]["manifest"]["sha256"]
            || checkpoint["tool_executions"] != 1
            || checkpoint["successor_context"] != "context:new"
        {
            return bad();
        }
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
    let mut full = anchor
        .map(|a| {
            a["messages"]
                .as_array()
                .cloned()
                .ok_or(StoreError::ClaimConflict)
        })
        .transpose()?
        .unwrap_or_default();
    full.extend(generated);
    history(p.take("pi.session_history")?, thread, &full, anchor)?;
    if p.i != p.v.len() {
        return bad();
    }
    if live && has_tool {
        // Owner-derived: only validated artifacts enter. The model's text is
        // retained solely as a digest; nothing here is caller- or model-supplied.
        let checkpoint = json!({
            "version": 2,
            "profile": if thinking { "work-engine.pi-coding-sol-high-continuity.v1" } else { LIVE_CHECKPOINT_PROFILE },
            "successor_context": successor_context,
            "coding_result": results[0]["result"],
            "coding_result_sha256": results[0]["artifact"]["sha256"],
            "workspace_manifest_sha256": workspaces[0]["manifest"]["sha256"],
            "tool_executions": 1,
            "settled_turn": {
                "invocation_id": inv,
                "assistant_final_text_sha256": hash(final_text.as_bytes()),
                "history_messages": full.len(),
                "history_messages_sha256": hash(
                    &serde_json::to_vec(&full).map_err(|_| StoreError::ClaimConflict)?
                ),
            },
            "unanswered_human": {"authority": "human", "resolved": false},
        })
        .to_string();
        if checkpoint.len() > MAX_CHECKPOINT_BYTES {
            return bad();
        }
        return Ok(Validated {
            final_text,
            checkpoint: Some(checkpoint),
        });
    }
    Ok(Validated {
        final_text,
        checkpoint: None,
    })
}

// Closed native SDK reasoning shape; retain the original block in history.
fn valid_thinking(v: &Value) -> bool {
    v["type"] == "thinking"
        && v["thinking"].as_str().is_some()
        && v.as_object().is_some_and(|o| {
            o.keys()
                .all(|k| matches!(k.as_str(), "type" | "thinking" | "thinkingSignature"))
        })
        && match v.get("thinkingSignature") {
            None => true,
            Some(sig) => sig
                .as_str()
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .is_some_and(|item| item.is_object() && item["type"] == "reasoning"),
        }
}
