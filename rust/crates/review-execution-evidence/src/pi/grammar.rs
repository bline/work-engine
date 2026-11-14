//! Bounded native profile derived from pinned Pi 1.5.0 pi-ai/types.d.ts and
//! coding-agent/core/session-manager.d.ts. Schema is separate from exclusion.
use super::*;
fn shape(v: &Value, required: &[&str], optional: &[&str]) -> bool {
    v.as_object().is_some_and(|m| {
        required.iter().all(|k| m.contains_key(*k))
            && m.keys()
                .all(|k| required.contains(&k.as_str()) || optional.contains(&k.as_str()))
    })
}
fn opt(v: &Value, key: &str, f: impl Fn(&Value) -> bool) -> bool {
    v.get(key).is_none_or(f)
}
fn number(v: &Value) -> bool {
    v.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0)
}
fn strings(v: &Value) -> bool {
    v.as_array()
        .is_some_and(|a| a.len() <= 64 && a.iter().all(Value::is_string))
}
fn list(v: &Value, f: impl Fn(&Value) -> bool) -> bool {
    v.as_array()
        .is_some_and(|a| a.len() <= 64 && a.iter().all(f))
}
fn usage(v: &Value) -> bool {
    shape(
        v,
        &[
            "input",
            "output",
            "cacheRead",
            "cacheWrite",
            "totalTokens",
            "cost",
        ],
        &["cacheWrite1h", "reasoning"],
    ) && ["input", "output", "cacheRead", "cacheWrite", "totalTokens"]
        .iter()
        .all(|k| number(&v[k]))
        && opt(v, "cacheWrite1h", number)
        && opt(v, "reasoning", number)
        && shape(
            &v["cost"],
            &["input", "output", "cacheRead", "cacheWrite", "total"],
            &["totalFromProvider"],
        )
        && ["input", "output", "cacheRead", "cacheWrite", "total"]
            .iter()
            .all(|k| number(&v["cost"][k]))
        && opt(&v["cost"], "totalFromProvider", Value::is_boolean)
}
fn text(v: &Value) -> bool {
    shape(v, &["type", "text"], &["textSignature"])
        && v["type"] == "text"
        && v["text"].is_string()
        && opt(v, "textSignature", Value::is_string)
}
fn image(v: &Value) -> bool {
    shape(v, &["type", "data", "mimeType"], &[])
        && v["type"] == "image"
        && v["mimeType"].is_string()
        && v["data"]
            .as_str()
            .is_some_and(|s| STANDARD.decode(s).is_ok_and(|b| STANDARD.encode(b) == s))
}
fn tool(v: &Value) -> bool {
    shape(v,&["name","description","parameters"],&["constrainedSampling"])
        && v["name"].is_string() && v["description"].is_string() && v["parameters"].is_object()
        // SDK builtins declare JSON-schema sampling. Grammar variants remain
        // outside this bounded Responses profile and refuse the whole candidate.
        && opt(v,"constrainedSampling",|x| x==false || (shape(x,&["type","strict"],&[]) && x["type"]=="json_schema" && matches!(x["strict"].as_str(),Some("prefer"|"require"))))
}
fn diagnostic(v: &Value) -> bool {
    shape(v, &["type", "timestamp"], &["error", "details"])
        && v["type"].is_string()
        && number(&v["timestamp"])
        && opt(v, "details", Value::is_object)
        && opt(v, "error", |e| {
            shape(e, &["message"], &["name", "stack", "code"])
                && e["message"].is_string()
                && opt(e, "name", Value::is_string)
                && opt(e, "stack", Value::is_string)
                && opt(e, "code", |x| x.is_string() || x.is_number())
        })
}
fn deferred(v: &Value) -> bool {
    shape(
        v,
        &["provider", "modelId", "api", "id"],
        &["expiresAt", "pollAfterMs", "data"],
    ) && ["provider", "modelId", "api", "id"]
        .iter()
        .all(|k| v[k].is_string())
        && opt(v, "expiresAt", number)
        && opt(v, "pollAfterMs", number)
}
fn nested(v: &Value) -> bool {
    shape(v, &["calls", "complete"], &[])
        && v["complete"].is_boolean()
        && list(&v["calls"], |c| {
            shape(
                c,
                &["id", "name", "status"],
                &["arguments", "argumentsBytes", "durationMs", "error"],
            ) && c["id"].is_string()
                && c["name"].is_string()
                && matches!(c["status"].as_str(), Some("ok" | "error" | "unfinished"))
                && opt(c, "arguments", Value::is_object)
                && opt(c, "argumentsBytes", number)
                && opt(c, "durationMs", number)
                && opt(c, "error", Value::is_string)
        })
}
pub(super) fn message(
    v: &Value,
    pending: &mut std::collections::BTreeMap<String, String>,
) -> Result<()> {
    if !number(&v["timestamp"]) {
        return Err("pi_message_timestamp");
    }
    let good = match v["role"].as_str() {
        Some("system") => {
            shape(
                v,
                &["role", "content", "timestamp"],
                &["sections", "toolsAdded", "toolsRemoved"],
            ) && (v["content"].is_string() || list(&v["content"], text))
                && opt(v, "sections", |s| {
                    s.as_object()
                        .is_some_and(|o| o.values().all(|x| x.is_string() || x.is_null()))
                })
                && opt(v, "toolsAdded", |x| list(x, tool))
                && opt(v, "toolsRemoved", |x| {
                    list(x, |t| shape(t, &["name"], &[]) && t["name"].is_string())
                })
        }
        Some("user") => {
            shape(v, &["role", "content", "timestamp"], &[])
                && (v["content"].is_string() || list(&v["content"], |c| text(c) || image(c)))
        }
        Some("assistant") => {
            let valid = shape(
                v,
                &[
                    "role",
                    "content",
                    "timestamp",
                    "api",
                    "provider",
                    "model",
                    "usage",
                    "stopReason",
                ],
                &[
                    "responseModel",
                    "responseId",
                    "providerThinkingLevel",
                    "thinkingLevel",
                    "diagnostics",
                    "deferred",
                    "errorMessage",
                    "rawStopReason",
                    "endTurn",
                    "durationMs",
                ],
            ) && v["api"] == "openai-codex-responses"
                && v["provider"] == "openai-codex"
                && v["model"] == "gpt-6-luna"
                && usage(&v["usage"])
                && matches!(
                    v["stopReason"].as_str(),
                    Some(
                        "pending"
                            | "stop"
                            | "length"
                            | "toolUse"
                            | "error"
                            | "aborted"
                            | "deferred"
                    )
                )
                && [
                    "responseModel",
                    "responseId",
                    "providerThinkingLevel",
                    "errorMessage",
                    "rawStopReason",
                ]
                .iter()
                .all(|k| opt(v, k, Value::is_string))
                && opt(v, "thinkingLevel", |x| {
                    matches!(
                        x.as_str(),
                        Some("off" | "minimal" | "low" | "medium" | "high" | "xhigh")
                    )
                })
                && opt(v, "diagnostics", |x| list(x, diagnostic))
                && opt(v, "deferred", deferred)
                && opt(v, "endTurn", Value::is_boolean)
                && opt(v, "durationMs", number)
                && list(&v["content"], |c| {
                    text(c)
                        || (shape(c, &["type", "thinking"], &["thinkingSignature", "redacted"])
                            && c["type"] == "thinking"
                            && c["thinking"].is_string()
                            && opt(c, "thinkingSignature", Value::is_string)
                            && opt(c, "redacted", Value::is_boolean))
                        || (shape(
                            c,
                            &["type", "id", "name", "arguments"],
                            &["thoughtSignature", "namespace", "argumentsError"],
                        ) && c["type"] == "toolCall"
                            && c["id"].as_str().is_some_and(|s| !s.is_empty())
                            && c["name"].is_string()
                            && c["arguments"].is_object()
                            && ["thoughtSignature", "namespace", "argumentsError"]
                                .iter()
                                .all(|k| opt(c, k, Value::is_string)))
                });
            if valid {
                for c in v["content"].as_array().unwrap() {
                    if c["type"] == "toolCall" {
                        if pending
                            .insert(
                                c["id"].as_str().unwrap().into(),
                                c["name"].as_str().unwrap().into(),
                            )
                            .is_some()
                        {
                            return Err("pi_duplicate_tool_call");
                        }
                    }
                }
            }
            valid
        }
        Some("toolResult") => {
            let good = shape(
                v,
                &[
                    "role",
                    "toolCallId",
                    "toolName",
                    "content",
                    "isError",
                    "timestamp",
                ],
                &["details", "usage", "nestedCalls", "durationMs"],
            ) && v["toolCallId"].is_string()
                && v["toolName"].is_string()
                && v["isError"].is_boolean()
                && list(&v["content"], |c| text(c) || image(c))
                && opt(v, "usage", usage)
                && opt(v, "nestedCalls", nested)
                && opt(v, "durationMs", number);
            if good
                && pending.remove(v["toolCallId"].as_str().unwrap()).as_deref()
                    != v["toolName"].as_str()
            {
                return Err("pi_tool_result_join");
            }
            good
        }
        _ => false,
    };
    if good {
        Ok(())
    } else {
        Err("pi_native_message_schema")
    }
}
