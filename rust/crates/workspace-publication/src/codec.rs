use crate::{Error, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const CODEC: &str = "workspace-publication-rfc8785-v1";
const DOMAIN: &[u8] = b"work-engine/workspace-publication/v1";

pub fn raw_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let v = serde_json::to_value(value).map_err(|e| Error::Invalid(e.to_string()))?;
    fn restricted(v: &serde_json::Value, depth: usize) -> Result<()> {
        if depth > 128 {
            return Err(Error::Invalid("JSON depth".into()));
        }
        match v {
            serde_json::Value::Number(n) if n.as_i64().is_none() && n.as_u64().is_none() => {
                return Err(Error::Invalid("noninteger JSON number unsupported".into()));
            }
            serde_json::Value::Array(a) => {
                for x in a {
                    restricted(x, depth + 1)?;
                }
            }
            serde_json::Value::Object(o) => {
                for x in o.values() {
                    restricted(x, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    restricted(&v, 0)?;
    serde_json_canonicalizer::to_vec(&v).map_err(|e| Error::Invalid(e.to_string()))
}

pub fn digest<T: Serialize>(kind: &str, value: &T) -> Result<String> {
    if kind.is_empty() || kind.bytes().any(|b| b == 0 || !b.is_ascii()) {
        return Err(Error::Invalid("record kind".into()));
    }
    let mut bytes = Vec::from(DOMAIN);
    bytes.push(0);
    bytes.extend_from_slice(kind.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&canonical(value)?);
    Ok(raw_sha256(&bytes))
}
