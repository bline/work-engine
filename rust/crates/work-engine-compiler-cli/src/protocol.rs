use crate::{
    aeg,
    source_snapshot::{Snapshots, evidence, resolve},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use work_engine_compiler::{
    ComparedSkill, CompiledSkill, canonical_json, compile_skill_unverified, prepare_skill_verified,
    sha256_hex,
};

pub const MAX_ENVELOPE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SOURCE_BYTES: usize = 4 * 1024 * 1024;
const MAX_DIAGNOSTIC_BYTES: usize = 1024;

struct VerifiedResult {
    compiled: CompiledSkill,
    verification: Value,
}
impl VerifiedResult {
    fn from_observed(compared: ComparedSkill, sources: Value, observation: Option<Value>) -> Self {
        let mut compiled = compared.into_compiled();
        compiled.ir["runtime_requirements"]["verified_sources"] = json!(true);
        let mut unsigned = compiled.ir["runtime_requirements"].clone();
        unsigned.as_object_mut().unwrap().remove("sha256");
        compiled.ir["runtime_requirements"]["sha256"] =
            json!(sha256_hex(canonical_json(&unsigned)));
        let mut verification = json!({"mode":"verified","sources":sources});
        if let Some(observation) = observation {
            verification["python"] = observation;
        }
        Self {
            compiled,
            verification,
        }
    }
    fn encode(self, request_id: String) -> Result<Vec<u8>, ProtocolError> {
        encoded(
            json!({"schema_version":2,"request_id":request_id,"status":"ok","ir":self.compiled.ir,"output_base64":STANDARD.encode(self.compiled.output),"verification":self.verification}),
        )
    }
}

#[derive(Debug, Serialize)]
pub struct ProtocolError {
    pub code: &'static str,
    pub path: Option<String>,
    pub message: String,
}
impl ProtocolError {
    pub(crate) fn new(code: &'static str, path: Option<&str>, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            code,
            path: path
                .filter(|s| s.len() <= MAX_DIAGNOSTIC_BYTES)
                .map(str::to_owned),
            message: if message.len() <= MAX_DIAGNOSTIC_BYTES {
                message
            } else {
                "diagnostic exceeds limit".to_owned()
            },
        }
    }
    pub fn source(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, None, message)
    }
    pub fn exit_code(&self) -> i32 {
        if matches!(
            self.code,
            "invalid_request"
                | "unsupported_version"
                | "unsupported_operation"
                | "invalid_encoding"
                | "invalid_yaml"
                | "invalid_structure"
                | "invalid_interface"
                | "source_mismatch"
                | "resource_limit"
                | "invalid_manifest"
                | "invalid_requirements"
                | "requirements_unsatisfied"
        ) {
            2
        } else {
            1
        }
    }
}

struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "strict JSON value")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_none<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_some<D: de::Deserializer<'de>>(self, d: D) -> Result<Strict, D::Error> {
                Strict::deserialize(d)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = Vec::new();
                while let Some(item) = a.next_element::<Strict>()? {
                    v.push(item.0);
                }
                Ok(Strict(Value::Array(v)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = Map::new();
                while let Some((key, item)) = a.next_entry::<String, Strict>()? {
                    if v.insert(key.clone(), item.0).is_some() {
                        return Err(de::Error::custom(format!("duplicate JSON key {key}")));
                    }
                }
                Ok(Strict(Value::Object(v)))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn strict_json(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let value = Strict::deserialize(&mut d)?.0;
    d.end()?;
    Ok(value)
}

fn decode(value: &Value, key: &str) -> Result<Vec<u8>, ProtocolError> {
    let path = format!("/{key}");
    let encoded = value.as_str().ok_or_else(|| {
        ProtocolError::new("invalid_request", Some(&path), "must be a base64 string")
    })?;
    if encoded.len() > MAX_SOURCE_BYTES.div_ceil(3) * 4 {
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
fn exact(map: &Map<String, Value>, allowed: &[&str]) -> Result<(), ProtocolError> {
    if map.len() != allowed.len()
        || map.keys().any(|key| !allowed.contains(&key.as_str()))
        || allowed.iter().any(|key| !map.contains_key(*key))
    {
        return Err(ProtocolError::new(
            "invalid_request",
            None,
            "request fields differ from protocol",
        ));
    }
    Ok(())
}
fn encoded(value: Value) -> Result<Vec<u8>, ProtocolError> {
    let mut bytes = serde_json::to_vec(&value)
        .map_err(|_| ProtocolError::source("internal_error", "response encoding failed"))?;
    if bytes.len() + 1 > MAX_ENVELOPE_BYTES {
        return Err(ProtocolError::source(
            "resource_limit",
            "result envelope exceeds limit",
        ));
    }
    bytes.push(b'\n');
    Ok(bytes)
}
fn compiled_error(issue: work_engine_compiler::CompilerError) -> ProtocolError {
    use work_engine_compiler::ErrorCode as C;
    ProtocolError::new(
        match issue.code {
            C::InvalidYaml => "invalid_yaml",
            C::InvalidStructure => "invalid_structure",
            C::InvalidInterface => "invalid_interface",
            C::SourceMismatch => "source_mismatch",
            C::AegProtocol => "aeg_protocol",
        },
        issue.path.as_deref(),
        issue.message,
    )
}
fn absolute_env(key: &str) -> Result<PathBuf, ProtocolError> {
    let value = std::env::var_os(key).ok_or_else(|| {
        ProtocolError::source(
            "invalid_request",
            format!("missing host configuration {key}"),
        )
    })?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(ProtocolError::source(
            "invalid_request",
            format!("host configuration {key} must be absolute"),
        ));
    }
    Ok(path)
}
fn sha_env(key: &str) -> Result<String, ProtocolError> {
    let value = std::env::var(key).map_err(|_| {
        ProtocolError::source(
            "invalid_request",
            format!("missing host configuration {key}"),
        )
    })?;
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(ProtocolError::source(
            "invalid_request",
            format!("invalid host digest {key}"),
        ));
    }
    Ok(value)
}
fn req_id(map: &Map<String, Value>) -> Result<String, ProtocolError> {
    let id = map
        .get("request_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProtocolError::new(
                "invalid_request",
                Some("/request_id"),
                "request_id must be text",
            )
        })?;
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(ProtocolError::new(
            "invalid_request",
            Some("/request_id"),
            "invalid request_id",
        ));
    }
    Ok(id.to_owned())
}

pub fn process(
    request: &[u8],
    cancelled: &AtomicBool,
) -> (u8, Option<String>, Result<Vec<u8>, ProtocolError>) {
    if request.len() > MAX_ENVELOPE_BYTES {
        return (
            1,
            None,
            Err(ProtocolError::source(
                "resource_limit",
                "request envelope exceeds limit",
            )),
        );
    }
    let value = match strict_json(request) {
        Ok(v) => v,
        Err(_) => {
            return (
                1,
                None,
                Err(ProtocolError::source(
                    "invalid_request",
                    "request must be strict JSON",
                )),
            );
        }
    };
    let map = match value.as_object() {
        Some(v) => v,
        None => {
            return (
                1,
                None,
                Err(ProtocolError::source(
                    "invalid_request",
                    "request must be an object",
                )),
            );
        }
    };
    let version = map.get("schema_version");
    if version == Some(&json!(1)) {
        return (1, None, process_v1(map));
    }
    if version == Some(&json!(3)) {
        let id = req_id(map).ok();
        return (3, id, crate::manifest_protocol::process(map, request.len()));
    }
    if version == Some(&json!(2)) {
        if map.len() == 2 && map.contains_key("operation") {
            return (
                1,
                None,
                Err(ProtocolError::new(
                    "unsupported_version",
                    Some("/schema_version"),
                    "protocol v2 requires a request_id and source fields",
                )),
            );
        }
        let id = req_id(map).ok();
        return (2, id, process_v2(map, cancelled));
    }
    if map.keys().any(|key| {
        ![
            "schema_version",
            "operation",
            "structure_source_base64",
            "interface_source_base64",
        ]
        .contains(&key.as_str())
    }) {
        return (
            1,
            None,
            Err(ProtocolError::new(
                "invalid_request",
                None,
                "unknown request field",
            )),
        );
    }
    (
        1,
        None,
        Err(ProtocolError::new(
            "unsupported_version",
            Some("/schema_version"),
            "schema version 1 or 2 is required",
        )),
    )
}
fn process_v1(map: &Map<String, Value>) -> Result<Vec<u8>, ProtocolError> {
    if map.keys().any(|key| {
        ![
            "schema_version",
            "operation",
            "structure_source_base64",
            "interface_source_base64",
        ]
        .contains(&key.as_str())
    }) {
        return Err(ProtocolError::new(
            "invalid_request",
            None,
            "unknown request field",
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
    let compiled = compile_skill_unverified(&structure, &interface).map_err(compiled_error)?;
    encoded(
        json!({"schema_version":1,"status":"ok","ir":compiled.ir,"output_base64":STANDARD.encode(compiled.output)}),
    )
}
fn process_v2(map: &Map<String, Value>, cancelled: &AtomicBool) -> Result<Vec<u8>, ProtocolError> {
    exact(
        map,
        &[
            "schema_version",
            "request_id",
            "operation",
            "structure_source_base64",
            "interface_source_base64",
        ],
    )?;
    let id = req_id(map)?;
    let operation = map["operation"].as_str().ok_or_else(|| {
        ProtocolError::new(
            "unsupported_operation",
            Some("/operation"),
            "unsupported operation",
        )
    })?;
    if !["compile_skill_unverified", "compile_skill_verified"].contains(&operation) {
        return Err(ProtocolError::new(
            "unsupported_operation",
            Some("/operation"),
            "unsupported operation",
        ));
    }
    let structure = decode(&map["structure_source_base64"], "structure_source_base64")?;
    let interface = decode(&map["interface_source_base64"], "interface_source_base64")?;
    if operation == "compile_skill_unverified" {
        let compiled = compile_skill_unverified(&structure, &interface).map_err(compiled_error)?;
        return encoded(
            json!({"schema_version":2,"request_id":id,"status":"ok","ir":compiled.ir,"output_base64":STANDARD.encode(compiled.output),"verification":{"mode":"unverified","sources":[]}}),
        );
    }
    let root = absolute_env("WORK_ENGINE_COMPILER_WORKSPACE_ROOT")?;
    let prepared = prepare_skill_verified(&structure, &interface).map_err(compiled_error)?;
    let mut snapshots = Snapshots::new();
    let mut files = HashMap::new();
    let mut seen = HashSet::new();
    for authored in prepared.paths() {
        let path = resolve(&root, &authored);
        let bytes = snapshots.capture(&path)?.to_vec();
        if seen.insert(authored.clone()) {
            files.insert(authored, bytes);
        }
    }
    let checked = prepared.check_sources(&files).map_err(compiled_error)?;
    let mut invariant_hash = None;
    let mut environment_hash = None;
    let mut script_hash = None;
    let mut observation = None;
    let aeg_envelope = if let Some(role_request) = checked.role_request() {
        let python = absolute_env("WORK_ENGINE_COMPILER_PYTHON")?;
        let script_path = absolute_env("WORK_ENGINE_COMPILER_AEG_SCRIPT")?;
        let expected = sha_env("WORK_ENGINE_COMPILER_AEG_SCRIPT_SHA256")?;
        let invariant_path = absolute_env("WORK_ENGINE_COMPILER_AEG_INVARIANTS")?;
        let environment_path = absolute_env("WORK_ENGINE_COMPILER_AEG_ENVIRONMENTS")?;
        let script = snapshots.capture(&script_path)?.to_vec();
        if sha256_hex(&script) != expected {
            return Err(ProtocolError::source(
                "source_mismatch",
                "Python script digest differs from host pin",
            ));
        }
        let invariants = snapshots.capture(&invariant_path)?.to_vec();
        let environment = snapshots.capture(&environment_path)?.to_vec();
        invariant_hash = Some(sha256_hex(&invariants));
        environment_hash = Some(sha256_hex(&environment));
        script_hash = Some(expected);
        let observed = aeg::run(
            &python,
            &script,
            &invariants,
            &environment,
            &role_request,
            cancelled,
        )?;
        observation = Some(observed.observation);
        Some(observed.envelope)
    } else {
        None
    };
    let sources = evidence(&snapshots.into_files());
    let compared = checked
        .compare_projection(
            aeg_envelope.as_ref(),
            invariant_hash.as_deref(),
            environment_hash.as_deref(),
            script_hash.as_deref(),
        )
        .map_err(compiled_error)?;
    VerifiedResult::from_observed(compared, sources, observation).encode(id)
}
pub fn error_envelope(version: u8, id: Option<&str>, issue: &ProtocolError) -> Vec<u8> {
    let value = if version == 2 {
        json!({"schema_version":2,"request_id":id,"status":"error","error":issue})
    } else {
        json!({"schema_version":1,"status":"error","error":issue})
    };
    encoded(value).unwrap_or_else(|_|b"{\"schema_version\":1,\"status\":\"error\",\"error\":{\"code\":\"resource_limit\",\"path\":null,\"message\":\"error envelope exceeds limit\"}}\n".to_vec())
}
