use crate::protocol::{self, ProtocolError};
use crate::source_snapshot::{Snapshots, resolve};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::io::{self, Write};
use work_engine_compiler::{
    ExtensionSkillSource, compile_extension_bundle, preflight_extension_bundle,
    verify_extension_attachment,
};

const PRODUCER: &str = "work-engine.extension-compiler.rust-v1";
const MAX_V4_COMPILE_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const MAX_V4_COMPILE_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const MAX_MATERIALIZED_SOURCE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Serialize)]
struct FreshVerifyRequest<'a> {
    schema_version: u8,
    request_id: &'a str,
    operation: &'static str,
    attachment: &'a Value,
    host: &'a Value,
}

struct BoundedCount(usize);
impl Write for BoundedCount {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        if self.0 > protocol::MAX_ENVELOPE_BYTES {
            return Err(io::Error::other("fresh verifier envelope exceeds limit"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn require_fresh_verify_capacity(attachment: &Value, host: &Value) -> Result<(), ProtocolError> {
    // req_id admits at most 128 ASCII bytes. Measure the actual parsed values
    // under their wire serializer, including a maximal legal fresh request ID.
    let maximal_id = "x".repeat(128);
    let request = FreshVerifyRequest {
        schema_version: 4,
        request_id: &maximal_id,
        operation: "verify_extension_attachment",
        attachment,
        host,
    };
    serde_json::to_writer(BoundedCount(0), &request).map_err(|_| {
        ProtocolError::source(
            "resource_limit",
            "attachment and host exceed fresh verifier envelope",
        )
    })
}

pub fn process(map: &Map<String, Value>, bytes: usize) -> Result<Vec<u8>, ProtocolError> {
    let is_verify = map.get("operation") == Some(&json!("verify_extension_attachment"));
    if !is_verify && bytes > MAX_V4_COMPILE_REQUEST_BYTES {
        return Err(ProtocolError::source(
            "resource_limit",
            "extension request exceeds limit",
        ));
    }
    let id = protocol::req_id(map)?;
    if is_verify {
        return verify(map, id);
    }
    if map.len() != 6
        || [
            "schema_version",
            "request_id",
            "operation",
            "bundle",
            "host",
            "workspace_root",
        ]
        .iter()
        .any(|key| !map.contains_key(*key))
    {
        return Err(ProtocolError::source(
            "invalid_request",
            "extension request fields differ from protocol",
        ));
    }
    if map.get("operation") != Some(&json!("compile_extension_bundle")) {
        return Err(ProtocolError::new(
            "unsupported_operation",
            Some("/operation"),
            "unsupported operation",
        ));
    }
    let root = protocol::absolute_env("WORK_ENGINE_COMPILER_WORKSPACE_ROOT")?;
    let claimed_root = map["workspace_root"]
        .as_str()
        .ok_or_else(|| ProtocolError::source("invalid_request", "workspace root must be text"))?;
    if claimed_root != root.to_str().unwrap_or("") {
        return Err(ProtocolError::source(
            "invalid_request",
            "workspace root differs from host configuration",
        ));
    }
    preflight_extension_bundle(&map["bundle"], &map["host"])
        .map_err(|issue| ProtocolError::source(issue.code, issue.message))?;
    let skills = map["bundle"]
        .get("skills")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProtocolError::source("invalid_extension", "extension skills are invalid")
        })?;
    let mut snapshots = Snapshots::new();
    let mut sources = Vec::with_capacity(skills.len());
    let mut materialized = 0usize;
    for skill in skills {
        let structure = skill
            .get("structure")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::source("invalid_extension", "extension structure path is invalid")
            })?;
        let interface = skill
            .get("interface")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ProtocolError::source("invalid_extension", "extension interface path is invalid")
            })?;
        let structure = snapshots.capture(&resolve(&root, structure))?;
        materialized = materialized.saturating_add(structure.len());
        if materialized > MAX_MATERIALIZED_SOURCE_BYTES {
            return Err(ProtocolError::source(
                "resource_limit",
                "materialized extension sources exceed limit",
            ));
        }
        let structure = structure.to_vec();
        let interface = snapshots.capture(&resolve(&root, interface))?;
        materialized = materialized.saturating_add(interface.len());
        if materialized > MAX_MATERIALIZED_SOURCE_BYTES {
            return Err(ProtocolError::source(
                "resource_limit",
                "materialized extension sources exceed limit",
            ));
        }
        let interface = interface.to_vec();
        sources.push(ExtensionSkillSource {
            structure,
            interface,
        });
    }
    let compiled = compile_extension_bundle(&map["bundle"], &map["host"], &sources)
        .map_err(|issue| ProtocolError::source(issue.code, issue.message))?;
    let attachment = compiled.into_attachment();
    require_fresh_verify_capacity(&attachment, &map["host"])?;
    let response = protocol::encoded(
        json!({"schema_version":4,"request_id":id,"operation":"compile_extension_bundle","status":"ok","producer":PRODUCER,"attachment":attachment}),
    )?;
    if response.len() > MAX_V4_COMPILE_RESPONSE_BYTES {
        return Err(ProtocolError::source(
            "resource_limit",
            "extension attachment exceeds roundtrip limit",
        ));
    }
    Ok(response)
}

fn verify(map: &Map<String, Value>, id: String) -> Result<Vec<u8>, ProtocolError> {
    let fields = [
        "schema_version",
        "request_id",
        "operation",
        "attachment",
        "host",
    ];
    if map.len() != fields.len() || fields.iter().any(|key| !map.contains_key(*key)) {
        return Err(ProtocolError::source(
            "invalid_request",
            "extension verification fields differ from protocol",
        ));
    }
    let descriptor = verify_extension_attachment(&map["attachment"], &map["host"])
        .map_err(|issue| ProtocolError::source(issue.code, issue.message))?;
    protocol::encoded(
        json!({"schema_version":4,"request_id":id,"operation":"verify_extension_attachment",
        "status":"ok","producer":PRODUCER,"attachment_sha256":map["attachment"]["sha256"],
        "bundle_id":descriptor["bundle_id"]}),
    )
}

pub fn error_envelope(id: Option<&str>, operation: Option<&str>, issue: &ProtocolError) -> Vec<u8> {
    protocol::encoded(json!({"schema_version":4,"request_id":id,"operation":operation,"status":"error","producer":PRODUCER,"error":issue}))
        .unwrap_or_else(|_| b"{\"schema_version\":4,\"status\":\"error\"}\n".to_vec())
}
