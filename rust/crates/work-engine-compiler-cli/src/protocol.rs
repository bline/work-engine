use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use serde_json::{Value, json};
use work_engine_compiler::compile_skill_unverified;

pub const MAX_ENVELOPE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SOURCE_BYTES: usize = 4 * 1024 * 1024;
const MAX_DIAGNOSTIC_BYTES: usize = 1024;

#[derive(Debug, Serialize)]
pub struct ProtocolError {
    pub code: &'static str,
    pub path: Option<String>,
    pub message: String,
}

impl ProtocolError {
    fn new(code: &'static str, path: Option<&str>, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            code,
            path: path
                .filter(|value| value.len() <= MAX_DIAGNOSTIC_BYTES)
                .map(str::to_owned),
            message: if message.len() <= MAX_DIAGNOSTIC_BYTES {
                message
            } else {
                "diagnostic exceeds limit".to_owned()
            },
        }
    }
}

fn decode(value: &Value, key: &str) -> Result<Vec<u8>, ProtocolError> {
    let path = format!("/{key}");
    let encoded = value.as_str().ok_or_else(|| {
        ProtocolError::new("invalid_request", Some(&path), "must be a base64 string")
    })?;
    if encoded.len() > (MAX_SOURCE_BYTES.div_ceil(3) * 4) {
        return Err(ProtocolError::new(
            "resource_limit",
            Some(&path),
            "encoded source exceeds limit",
        ));
    }
    let bytes = STANDARD.decode(encoded).map_err(|_| {
        ProtocolError::new("invalid_encoding", Some(&path), "invalid padded base64")
    })?;
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(ProtocolError::new(
            "resource_limit",
            Some(&path),
            "decoded source exceeds limit",
        ));
    }
    std::str::from_utf8(&bytes)
        .map_err(|_| ProtocolError::new("invalid_encoding", Some(&path), "source must be UTF-8"))?;
    Ok(bytes)
}

pub fn process(request: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if request.len() > MAX_ENVELOPE_BYTES {
        return Err(ProtocolError::new(
            "resource_limit",
            None,
            "request envelope exceeds limit",
        ));
    }
    let value: Value = serde_json::from_slice(request)
        .map_err(|_| ProtocolError::new("invalid_request", None, "request must be valid JSON"))?;
    let map = value
        .as_object()
        .ok_or_else(|| ProtocolError::new("invalid_request", None, "request must be an object"))?;
    for key in map.keys() {
        if ![
            "schema_version",
            "operation",
            "structure_source_base64",
            "interface_source_base64",
        ]
        .contains(&key.as_str())
        {
            let path = if key.len() < MAX_DIAGNOSTIC_BYTES {
                Some(format!("/{key}"))
            } else {
                None
            };
            return Err(ProtocolError::new(
                "invalid_request",
                path.as_deref(),
                "unknown request field",
            ));
        }
    }
    if map.get("schema_version") != Some(&json!(1)) {
        return Err(ProtocolError::new(
            "unsupported_version",
            Some("/schema_version"),
            "schema version 1 is required",
        ));
    }
    if map.get("operation") != Some(&json!("compile_skill_unverified")) {
        return Err(ProtocolError::new(
            "unsupported_operation",
            Some("/operation"),
            "unsupported operation",
        ));
    }
    let structure = decode(
        map.get("structure_source_base64").unwrap_or(&Value::Null),
        "structure_source_base64",
    )?;
    let interface = decode(
        map.get("interface_source_base64").unwrap_or(&Value::Null),
        "interface_source_base64",
    )?;
    let compiled = compile_skill_unverified(&structure, &interface).map_err(|issue| {
        ProtocolError::new(
            match issue.code {
                work_engine_compiler::ErrorCode::InvalidYaml => "invalid_yaml",
                work_engine_compiler::ErrorCode::InvalidStructure => "invalid_structure",
                work_engine_compiler::ErrorCode::InvalidInterface => "invalid_interface",
            },
            issue.path.as_deref(),
            issue.message,
        )
    })?;
    let output = json!({"schema_version":1,"status":"ok","ir":compiled.ir,"output_base64":STANDARD.encode(compiled.output)});
    let mut bytes = serde_json::to_vec(&output)
        .map_err(|_| ProtocolError::new("internal_error", None, "failed to encode response"))?;
    if bytes.len() + 1 > MAX_ENVELOPE_BYTES {
        return Err(ProtocolError::new(
            "resource_limit",
            None,
            "result envelope exceeds limit",
        ));
    }
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn error_envelope(issue: &ProtocolError) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&json!({"schema_version":1,"status":"error","error":issue}))
        .expect("bounded static error envelope");
    if bytes.len() + 1 > MAX_ENVELOPE_BYTES {
        bytes = serde_json::to_vec(&json!({"schema_version":1,"status":"error","error":{"code":"resource_limit","path":null,"message":"error envelope exceeds limit"}})).expect("static fallback envelope");
    }
    bytes.push(b'\n');
    bytes
}
