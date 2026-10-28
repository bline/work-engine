use serde_json::Value;
use sha2::{Digest as _, Sha256};
use thiserror::Error;

/// Observed against the installed 0.163.0 binary with an auth-free loopback
/// Responses fixture. The protocol schema hashes below were regenerated from
/// that exact binary and happen to match the previous 0.162.1 schemas.
pub const PROFILE_ID: &str = "codex-0.163.0-workspace-snapshot-read-v1";
/// Owner-selected prospective consumer rule for additive read-state presentation data.
/// The profile digest binds this rule; producer and operation eligibility remain separate.
pub const READ_STATE_CONSUMER_POLICY_ID: &str = "codex-0.163.0-read-state-opaque-scalars-v3";
pub const NATIVE_BINARY_SHA256: &str =
    "399ff69c431c066e40de318fba45c2b07f45a76b19939d247ee4828fc70c4215";
pub const NATIVE_USER_AGENT_VERSION: &str = "0.163.0";
pub const SCHEMA_THREAD_START_SHA256: &str =
    "80a40a7fac15b4bf70efb7f893fb353acc0a0d30c68f54aee4f01923deca85de";
pub const SCHEMA_TOOL_CALL_SHA256: &str =
    "401bba20cfbd95762bef0467d840430c46be53369093ad9f26425ba757e34efc";

/// The app-server prefixes its user agent with the initialize client name and
/// the native version. Require both; a substring match admits a different
/// version such as 0.163.01.
pub fn user_agent_matches(value: &str, client_name: &str) -> bool {
    value.starts_with(&format!("{client_name}/{NATIVE_USER_AGENT_VERSION} "))
}

/// 0.163.0 emits this durable-thread presentation notification immediately
/// before turn completion. It has no turn coordinate or effect authority.
/// Additive bounded scalars are opaque presentation data, not lifecycle facts.
/// The store binds the known thread ID to the owned native session. Unknown
/// notifications still fail closed.
pub fn read_state_changed_matches(value: &Value) -> bool {
    let Some(frame) = value.as_object() else {
        return false;
    };
    // The installed binary also emits a numeric timestamp on the envelope.
    // It is presentation metadata, not an effect or turn coordinate.
    if frame.get("method").and_then(Value::as_str) != Some("thread/readState/changed")
        || frame
            .keys()
            .any(|key| !matches!(key.as_str(), "method" | "params" | "emittedAtMs"))
        || frame
            .get("emittedAtMs")
            .is_some_and(|timestamp| timestamp.as_u64().is_none())
    {
        return false;
    }
    let Some(params) = frame.get("params").and_then(Value::as_object) else {
        return false;
    };
    if params.len() != 2 {
        return false;
    }
    let bounded = |field: Option<&Value>| {
        field
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty() && s.len() <= 256)
    };
    if !bounded(params.get("threadId")) {
        return false;
    }
    let Some(state) = params.get("readState").and_then(Value::as_object) else {
        return false;
    };
    if !bounded(state.get("revision")) {
        return false;
    }
    let mut extensions = 0;
    for (key, value) in state {
        if matches!(key.as_str(), "revision" | "firstUnread") {
            continue;
        }
        extensions += 1;
        if extensions > 8 || key.is_empty() || key.len() > 64 {
            return false;
        }
        match value {
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
            Value::String(text) if text.len() <= 256 => {}
            _ => return false,
        }
    }
    match state.get("firstUnread") {
        None | Some(Value::Null) => true,
        Some(Value::Object(position)) if position.len() == 1 => {
            position.get("type").and_then(Value::as_str) == Some("threadStart")
        }
        Some(Value::Object(position)) if position.len() == 2 => {
            position.get("type").and_then(Value::as_str) == Some("turn")
                && bounded(position.get("turnId"))
        }
        _ => false,
    }
}

#[derive(Debug, Error)]
pub enum NativeProtocolError {
    #[error("native JSON frame is ambiguous or malformed")]
    Malformed,
    #[error("native JSON-RPC frame shape is unsupported")]
    Unsupported,
    #[error("native tool-call identity or arguments are invalid")]
    InvalidToolCall,
}

#[derive(Clone, Debug)]
pub struct NativeEvent {
    pub transport_session: String,
    pub sequence: u64,
    pub raw: Vec<u8>,
    pub sha256: String,
    pub value: Value,
    pub kind: NativeEventKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeEventKind {
    Malformed,
    Response { id: u64 },
    ServerRequest { id: u64, method: String },
    Notification { method: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeToolCall {
    pub rpc_id: u64,
    pub thread_id: String,
    pub turn_id: String,
    pub call_id: String,
    pub namespace: Option<String>,
    pub tool: String,
    pub arguments: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeTurnTerminal {
    pub thread_id: String,
    pub turn_id: String,
    pub status: String,
}

pub fn parse_event(
    transport_session: String,
    sequence: u64,
    raw: Vec<u8>,
) -> Result<NativeEvent, NativeProtocolError> {
    if transport_session.is_empty() || sequence == 0 {
        return Err(NativeProtocolError::Malformed);
    }
    let value =
        lifecycle_wire::parse_strict_json(&raw).map_err(|_| NativeProtocolError::Malformed)?;
    let obj = value.as_object().ok_or(NativeProtocolError::Malformed)?;
    let kind = match (obj.get("id"), obj.get("method")) {
        (Some(id), Some(method)) => NativeEventKind::ServerRequest {
            id: id.as_u64().ok_or(NativeProtocolError::Unsupported)?,
            method: method
                .as_str()
                .ok_or(NativeProtocolError::Unsupported)?
                .to_owned(),
        },
        (Some(id), None) => NativeEventKind::Response {
            id: id.as_u64().ok_or(NativeProtocolError::Unsupported)?,
        },
        (None, Some(method)) => NativeEventKind::Notification {
            method: method
                .as_str()
                .ok_or(NativeProtocolError::Unsupported)?
                .to_owned(),
        },
        _ => return Err(NativeProtocolError::Unsupported),
    };
    let sha256 = format!("{:x}", Sha256::digest(&raw));
    Ok(NativeEvent {
        transport_session,
        sequence,
        raw,
        sha256,
        value,
        kind,
    })
}

impl NativeEvent {
    pub fn tool_call(&self) -> Result<Option<NativeToolCall>, NativeProtocolError> {
        let NativeEventKind::ServerRequest { id, method } = &self.kind else {
            return Ok(None);
        };
        if method != "item/tool/call" {
            return Ok(None);
        }
        let p = self
            .value
            .get("params")
            .ok_or(NativeProtocolError::InvalidToolCall)?;
        let text = |key: &str| {
            p.get(key)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty() && s.len() <= 256)
                .map(str::to_owned)
                .ok_or(NativeProtocolError::InvalidToolCall)
        };
        let namespace = match p.get("namespace") {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) if !value.is_empty() && value.len() <= 64 => {
                Some(value.clone())
            }
            _ => return Err(NativeProtocolError::InvalidToolCall),
        };
        let arguments = p
            .get("arguments")
            .cloned()
            .ok_or(NativeProtocolError::InvalidToolCall)?;
        Ok(Some(NativeToolCall {
            rpc_id: *id,
            thread_id: text("threadId")?,
            turn_id: text("turnId")?,
            call_id: text("callId")?,
            namespace,
            tool: text("tool")?,
            arguments,
        }))
    }

    pub fn terminal(&self) -> Result<Option<NativeTurnTerminal>, NativeProtocolError> {
        let NativeEventKind::Notification { method } = &self.kind else {
            return Ok(None);
        };
        if !matches!(
            method.as_str(),
            "turn/completed" | "turn/failed" | "turn/interrupted"
        ) {
            return Ok(None);
        }
        let p = self
            .value
            .get("params")
            .ok_or(NativeProtocolError::Unsupported)?;
        let thread_id = p
            .get("threadId")
            .and_then(Value::as_str)
            .ok_or(NativeProtocolError::Unsupported)?
            .to_owned();
        let turn = p.get("turn").ok_or(NativeProtocolError::Unsupported)?;
        let turn_id = turn
            .get("id")
            .and_then(Value::as_str)
            .ok_or(NativeProtocolError::Unsupported)?
            .to_owned();
        let status = turn
            .get("status")
            .and_then(Value::as_str)
            .ok_or(NativeProtocolError::Unsupported)?
            .to_owned();
        Ok(Some(NativeTurnTerminal {
            thread_id,
            turn_id,
            status,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bound_user_agent_refuses_old_or_ambiguous_versions() {
        assert!(user_agent_matches(
            "work_engine_s5/0.163.0 (Linux; x86_64) unknown (work_engine_s5; 0.1.0)",
            "work_engine_s5"
        ));
        for value in [
            "work_engine_s5/0.162.1 (Linux; x86_64)",
            "work_engine_s5/0.163.01 (Linux; x86_64)",
            "other_client/0.163.0 (Linux; x86_64)",
            "work_engine_s5/0.163.0-extra (Linux; x86_64)",
        ] {
            assert!(!user_agent_matches(value, "work_engine_s5"));
        }
    }

    #[test]
    fn read_state_notification_is_structural_metadata_only() {
        // Captured 0.163.0 envelope shape, including its timestamp.
        let valid = serde_json::json!({"method":"thread/readState/changed", "params":{
            "threadId":"01a14807-2044-7001-9d30-536791ac22cc", "readState":{
                "firstUnread":{"type":"turn", "turnId":"01a14807-204b-7680-b976-855e77cdcb1b"},
                "revision":"ac9e6905-b36d-4edf-8286-fdb3c1f9bda0"}},
            "emittedAtMs":1792209789026u64});
        assert!(read_state_changed_matches(&valid));
        let mut without_timestamp = valid.clone();
        without_timestamp
            .as_object_mut()
            .unwrap()
            .remove("emittedAtMs");
        assert!(read_state_changed_matches(&without_timestamp));
        for first_unread in [
            None,
            Some(serde_json::Value::Null),
            Some(serde_json::json!({"type":"threadStart"})),
        ] {
            let mut optional = valid.clone();
            if let Some(first_unread) = first_unread {
                optional["params"]["readState"]["firstUnread"] = first_unread;
            } else {
                optional["params"]["readState"]
                    .as_object_mut()
                    .unwrap()
                    .remove("firstUnread");
            }
            assert!(read_state_changed_matches(&optional));
        }
        for invalid in [
            serde_json::json!({"params":{"threadId":"thread:1","readState":{"revision":""}}}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":"thread:1","readState":{"firstUnread":{"type":"turn"},"revision":"revision:1"}}}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":"thread:1","turnId":"turn:1","readState":{"revision":"revision:1"}}}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":"thread:1","readState":{"firstUnread":{"type":"effect","turnId":"turn:1"},"revision":"revision:1"}}}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":"thread:1","readState":{"firstUnread":{"type":"threadStart","unexpected":true},"revision":"revision:1"}}}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":"thread:1","readState":{"firstUnread":{"type":"turn","turnId":"turn:1","unexpected":true},"revision":"revision:1"}}}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":"thread:1","readState":{"revision":"revision:1"}},"unexpected":true}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":"thread:1","readState":{"revision":"revision:1"}},"emittedAtMs":"1792209789026"}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":"thread:1","readState":{"revision":"revision:1"}},"emittedAtMs":-1}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":7,"readState":{"revision":"revision:1"}}}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":"thread:1","readState":{"revision":7}}}),
            serde_json::json!({"method":"thread/readState/changed","params":{"threadId":"thread:1","readState":{"revision":"revision:1","firstUnread":true}}}),
            serde_json::json!({"method":"turn/completed","params":{"threadId":"thread:1","readState":{"revision":"revision:1"}}}),
        ] {
            assert!(!read_state_changed_matches(&invalid));
        }
        for field in ["threadId", "revision", "turnId"] {
            let mut oversized = valid.clone();
            let target = match field {
                "threadId" => &mut oversized["params"]["threadId"],
                "revision" => &mut oversized["params"]["readState"]["revision"],
                _ => &mut oversized["params"]["readState"]["firstUnread"]["turnId"],
            };
            *target = serde_json::json!("x".repeat(257));
            assert!(!read_state_changed_matches(&oversized));
        }
    }

    #[test]
    fn read_state_opaque_scalar_extensions_are_bounded_and_have_no_rpc_authority() {
        let mut frame = serde_json::json!({"method":"thread/readState/changed","params":{
            "threadId":"thread:1","readState":{"revision":"revision:1","firstUnread":null}
        }});
        let state = frame["params"]["readState"].as_object_mut().unwrap();
        for (key, value) in [
            ("error", serde_json::Value::Null),
            ("needsInput", serde_json::json!(true)),
            ("completed", serde_json::json!(42)),
            ("pending", serde_json::json!("x".repeat(256))),
            ("unreadCount", serde_json::json!(0)),
            ("status", serde_json::json!(false)),
            ("answer", serde_json::json!("")),
            ("settled", serde_json::json!(-1)),
        ] {
            state.insert(key.into(), value);
        }
        assert!(read_state_changed_matches(&frame));
        let raw = serde_json::to_vec(&frame).unwrap();
        let event = parse_event("transport:1".into(), 1, raw.clone()).unwrap();
        assert_eq!(event.raw, raw);
        assert!(matches!(event.kind, NativeEventKind::Notification { .. }));
        assert!(event.terminal().unwrap().is_none());
        assert!(event.tool_call().unwrap().is_none());

        let mut max_key = frame.clone();
        max_key["params"]["readState"]
            .as_object_mut()
            .unwrap()
            .remove("error");
        max_key["params"]["readState"]["k".repeat(64).as_str()] = serde_json::Value::Null;
        assert!(read_state_changed_matches(&max_key));

        let mut ninth = frame.clone();
        ninth["params"]["readState"]["ninth"] = serde_json::Value::Null;
        assert!(!read_state_changed_matches(&ninth));
        for (key, value) in [
            ("nested", serde_json::json!({"a":1})),
            ("array", serde_json::json!([1])),
            ("long", serde_json::json!("x".repeat(257))),
        ] {
            let mut invalid = frame.clone();
            invalid["params"]["readState"]
                .as_object_mut()
                .unwrap()
                .remove("error");
            invalid["params"]["readState"][key] = value;
            assert!(!read_state_changed_matches(&invalid), "{key}");
        }
        for bad_key in [String::new(), "x".repeat(65)] {
            let mut invalid = frame.clone();
            invalid["params"]["readState"]
                .as_object_mut()
                .unwrap()
                .remove("error");
            invalid["params"]["readState"][&bad_key] = serde_json::Value::Null;
            assert!(!read_state_changed_matches(&invalid));
        }
        let mut request = frame.clone();
        request["id"] = serde_json::json!(1);
        assert!(!read_state_changed_matches(&request));
        let event = parse_event(
            "transport:1".into(),
            2,
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        assert!(matches!(event.kind, NativeEventKind::ServerRequest { .. }));
        let duplicate = br#"{"method":"thread/readState/changed","params":{"threadId":"thread:1","readState":{"revision":"r1","error":null,"error":true}}}"#;
        assert!(parse_event("transport:1".into(), 3, duplicate.to_vec()).is_err());
    }

    #[test]
    fn coordinates_distinguish_identical_bytes_and_reject_ambiguous_keys() {
        let raw = br#"{"id":0,"method":"item/tool/call","params":{"threadId":"t","turnId":"u","callId":"c","namespace":"workspace","tool":"snapshot_read","arguments":{"member":"m1"}}}"#.to_vec();
        let one = parse_event("s".into(), 1, raw.clone()).unwrap();
        let two = parse_event("s".into(), 2, raw).unwrap();
        assert_eq!(one.sha256, two.sha256);
        assert_ne!(one.sequence, two.sequence);
        assert_eq!(one.tool_call().unwrap().unwrap().call_id, "c");
        assert!(parse_event("s".into(), 1, br#"{"id":1,"id":2}"#.to_vec()).is_err());
    }

    #[test]
    fn missing_tool_identity_and_nonterminal_status_cannot_claim_success() {
        let missing=br#"{"id":0,"method":"item/tool/call","params":{"threadId":"t","turnId":"u","namespace":"workspace","tool":"snapshot_read","arguments":{}}}"#;
        let event = parse_event("s".into(), 1, missing.to_vec()).unwrap();
        assert!(matches!(
            event.tool_call(),
            Err(NativeProtocolError::InvalidToolCall)
        ));
        let raw=br#"{"method":"turn/completed","params":{"threadId":"t","turn":{"id":"u","status":"inProgress"}}}"#;
        let event = parse_event("s".into(), 2, raw.to_vec()).unwrap();
        assert_eq!(event.terminal().unwrap().unwrap().status, "inProgress");
        assert!(
            parse_event(
                "s".into(),
                3,
                br#"{"method":"unknown","params":{"message":"\uD800"}}"#.to_vec()
            )
            .is_err()
        );
    }
}
