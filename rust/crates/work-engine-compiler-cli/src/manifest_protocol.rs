use crate::protocol::ProtocolError;
use serde_json::{Map, Value, json};
use work_engine_compiler::{project_runtime_manifest, satisfy_runtime_requirements};

const MAX_V3_BYTES: usize = 4 * 1024 * 1024;
const PRODUCER: &str = "work-engine.manifest-compiler.rust-v1";
fn exact(map: &Map<String, Value>, names: &[&str]) -> Result<(), ProtocolError> {
    if map.len() != names.len()
        || map.keys().any(|k| !names.contains(&k.as_str()))
        || names.iter().any(|k| !map.contains_key(*k))
    {
        return Err(ProtocolError::new(
            "invalid_request",
            None,
            "v3 request fields differ from protocol",
        ));
    }
    Ok(())
}
fn id(map: &Map<String, Value>) -> Result<&str, ProtocolError> {
    let s = map
        .get("request_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProtocolError::new(
                "invalid_request",
                Some("/request_id"),
                "request ID must be text",
            )
        })?;
    if s.is_empty()
        || s.len() > 128
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(ProtocolError::new(
            "invalid_request",
            Some("/request_id"),
            "invalid request ID",
        ));
    }
    Ok(s)
}
fn depth(v: &Value, n: usize) -> Result<(), ProtocolError> {
    if n > 64 {
        return Err(ProtocolError::new(
            "resource_limit",
            None,
            "JSON nesting exceeds limit",
        ));
    }
    match v {
        Value::Array(a) => {
            for x in a {
                depth(x, n + 1)?
            }
        }
        Value::Object(m) => {
            for x in m.values() {
                depth(x, n + 1)?
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn process(map: &Map<String, Value>, bytes: usize) -> Result<Vec<u8>, ProtocolError> {
    if bytes > MAX_V3_BYTES {
        return Err(ProtocolError::new(
            "resource_limit",
            None,
            "v3 request exceeds limit",
        ));
    }
    let request_id = id(map)?;
    let operation = map
        .get("operation")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProtocolError::new(
                "invalid_request",
                Some("/operation"),
                "operation must be text",
            )
        })?;
    let result = match operation {
        "project_runtime_manifest" => {
            exact(
                map,
                &[
                    "schema_version",
                    "request_id",
                    "operation",
                    "document",
                    "options",
                ],
            )?;
            depth(&map["document"], 0)?;
            depth(&map["options"], 0)?;
            project_runtime_manifest(&map["document"], &map["options"])
        }
        "satisfy_runtime_requirements" => {
            exact(
                map,
                &[
                    "schema_version",
                    "request_id",
                    "operation",
                    "manifest",
                    "role_id",
                    "requirements",
                    "skill_name",
                ],
            )?;
            depth(&map["manifest"], 0)?;
            depth(&map["requirements"], 0)?;
            let role = map["role_id"].as_str().ok_or_else(|| {
                ProtocolError::new("invalid_request", Some("/role_id"), "must be text")
            })?;
            let skill = if map["skill_name"].is_null() {
                None
            } else {
                Some(map["skill_name"].as_str().ok_or_else(|| {
                    ProtocolError::new(
                        "invalid_request",
                        Some("/skill_name"),
                        "must be text or null",
                    )
                })?)
            };
            satisfy_runtime_requirements(&map["manifest"], role, &map["requirements"], skill)
        }
        _ => {
            return Err(ProtocolError::new(
                "invalid_request",
                Some("/operation"),
                "unsupported v3 operation",
            ));
        }
    }
    .map_err(|e| ProtocolError::new(e.code, e.path.as_deref(), e.message))?;
    let mut bytes=serde_json::to_vec(&json!({"schema_version":3,"request_id":request_id,"operation":operation,"status":"ok","producer":PRODUCER,"result":result})).map_err(|_|ProtocolError::new("internal_error",None,"response encoding failed"))?;
    if bytes.len() + 1 > MAX_V3_BYTES {
        return Err(ProtocolError::new(
            "resource_limit",
            None,
            "v3 result exceeds limit",
        ));
    }
    bytes.push(b'\n');
    Ok(bytes)
}
pub fn error_envelope(id: Option<&str>, operation: Option<&str>, issue: &ProtocolError) -> Vec<u8> {
    let mut bytes=serde_json::to_vec(&json!({"schema_version":3,"request_id":id,"operation":operation,"status":"error","producer":PRODUCER,"error":issue})).unwrap_or_default();
    bytes.push(b'\n');
    bytes
}
