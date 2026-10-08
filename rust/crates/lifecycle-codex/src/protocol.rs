use serde_json::Value;
use sha2::{Digest as _, Sha256};
use thiserror::Error;

pub const PROFILE_ID: &str = "codex-0.160.1-workspace-snapshot-read-v1";
pub const NATIVE_BINARY_SHA256: &str =
    "f34a4d2301892ae96c90097786bfe5dc269f187b6f69faf42a7b357b8c081e35";
pub const SCHEMA_THREAD_START_SHA256: &str =
    "80a40a7fac15b4bf70efb7f893fb353acc0a0d30c68f54aee4f01923deca85de";
pub const SCHEMA_TOOL_CALL_SHA256: &str =
    "401bba20cfbd95762bef0467d840430c46be53369093ad9f26425ba757e34efc";

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
