//! Domain-owned historical codecs. Campaign state uses JavaScript UTF-16 key
//! ordering and no final LF. Candidate/profile producers use Python code-point
//! ordering and no final LF. Raw artifact bytes are hashed without parsing.

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{CampaignError, Result};

pub const CAMPAIGN_CODEC: &str = "slice-campaign-js-canonical-v1";
pub const HISTORICAL_ARTIFACT_CODEC: &str = "sc0-python-json-v1";

pub fn raw_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn emit(value: &Value, out: &mut String, campaign: bool, depth: usize) -> Result<()> {
    if depth > 128 {
        return Err(CampaignError::Contract(
            "JSON nesting exceeds profile limit".into(),
        ));
    }
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::String(value) => out.push_str(
            &serde_json::to_string(value).map_err(|e| CampaignError::Contract(e.to_string()))?,
        ),
        Value::Number(value) => {
            if campaign {
                let number = value
                    .as_f64()
                    .ok_or_else(|| CampaignError::Contract("unsupported campaign number".into()))?;
                if !number.is_finite() {
                    return Err(CampaignError::Contract("nonfinite campaign number".into()));
                }
                // Identity inputs above the exact JS integer range are refused by
                // their contract. Other campaign numeric projections use JS spelling.
                let mut buffer = ryu_js::Buffer::new();
                out.push_str(buffer.format(number));
            } else {
                out.push_str(&value.to_string());
            }
        }
        Value::Array(values) => {
            out.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                emit(value, out, campaign, depth + 1)?;
            }
            out.push(']');
        }
        Value::Object(values) => {
            let mut keys: Vec<_> = values.keys().collect();
            if campaign {
                keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            } else {
                keys.sort();
            }
            out.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(
                    &serde_json::to_string(key)
                        .map_err(|e| CampaignError::Contract(e.to_string()))?,
                );
                out.push(':');
                emit(&values[*key], out, campaign, depth + 1)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

pub fn canonical_campaign(value: &Value) -> Result<Vec<u8>> {
    let mut output = String::new();
    emit(value, &mut output, true, 0)?;
    Ok(output.into_bytes())
}

pub fn canonical_historical_artifact(value: &Value) -> Result<Vec<u8>> {
    let mut output = String::new();
    emit(value, &mut output, false, 0)?;
    Ok(output.into_bytes())
}

pub fn campaign_digest(value: &Value) -> Result<String> {
    Ok(raw_sha256(&canonical_campaign(value)?))
}

/// The existing campaign's published update projection spells an explicitly
/// present `revision: undefined` member. Its canonical writer emits the raw
/// token `undefined`; dropping the member changes the durable revision hash.
pub fn campaign_update_revision_digest(value_without_revision: &Value) -> Result<String> {
    let object = value_without_revision.as_object().ok_or_else(|| {
        CampaignError::Contract("campaign revision projection must be object".into())
    })?;
    if object.contains_key("revision") {
        return Err(CampaignError::Contract(
            "revision projection already has revision".into(),
        ));
    }
    let mut keys: Vec<_> = object.keys().map(String::as_str).collect();
    keys.push("revision");
    keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    let mut output = String::from("{");
    for (index, key) in keys.into_iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(
            &serde_json::to_string(key).map_err(|e| CampaignError::Contract(e.to_string()))?,
        );
        output.push(':');
        if key == "revision" {
            output.push_str("undefined");
        } else {
            output.push_str(
                &String::from_utf8(canonical_campaign(&object[key])?)
                    .map_err(|e| CampaignError::Contract(e.to_string()))?,
            );
        }
    }
    output.push('}');
    Ok(raw_sha256(output.as_bytes()))
}

pub fn historical_digest(value: &Value) -> Result<String> {
    Ok(raw_sha256(&canonical_historical_artifact(value)?))
}

/// Canonicalize a historical artifact directly from raw JSON. Numeric token
/// kind matters to Python (`-0` hashes as `0`, `-0.0` as `-0.0`), so a generic
/// `serde_json::Value` round trip cannot supply every codec oracle byte.
pub fn canonical_historical_raw(input: &[u8]) -> Result<Vec<u8>> {
    if input.len() > 1_048_576 {
        return Err(CampaignError::Contract("historical JSON too large".into()));
    }
    let mut parser = RawParser {
        input,
        position: 0,
        depth: 0,
    };
    let output = parser.value()?;
    parser.space();
    if parser.position != input.len() {
        return Err(CampaignError::Contract("trailing historical JSON".into()));
    }
    Ok(output.into_bytes())
}

pub fn historical_digest_raw(input: &[u8]) -> Result<String> {
    Ok(raw_sha256(&canonical_historical_raw(input)?))
}

struct RawParser<'a> {
    input: &'a [u8],
    position: usize,
    depth: usize,
}
impl RawParser<'_> {
    fn space(&mut self) {
        while self
            .input
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }
    fn take(&mut self, byte: u8) -> Result<()> {
        self.space();
        if self.input.get(self.position) != Some(&byte) {
            return Err(CampaignError::Contract(
                "invalid historical JSON syntax".into(),
            ));
        }
        self.position += 1;
        Ok(())
    }
    fn string(&mut self) -> Result<String> {
        self.space();
        let start = self.position;
        self.take(b'"')?;
        while let Some(&byte) = self.input.get(self.position) {
            self.position += 1;
            if byte == b'\\' {
                self.position += 1;
                continue;
            }
            if byte == b'"' {
                return serde_json::from_slice::<String>(&self.input[start..self.position])
                    .map_err(|e| CampaignError::Contract(e.to_string()));
            }
        }
        Err(CampaignError::Contract("unterminated JSON string".into()))
    }
    fn value(&mut self) -> Result<String> {
        self.space();
        self.depth += 1;
        if self.depth > 128 {
            return Err(CampaignError::Contract(
                "historical JSON depth exceeds limit".into(),
            ));
        }
        let result = match self.input.get(self.position) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => serde_json::to_string(&self.string()?)
                .map_err(|e| CampaignError::Contract(e.to_string())),
            Some(b'n') | Some(b't') | Some(b'f') => {
                let start = self.position;
                while self
                    .input
                    .get(self.position)
                    .is_some_and(u8::is_ascii_alphabetic)
                {
                    self.position += 1;
                }
                let token = std::str::from_utf8(&self.input[start..self.position])
                    .map_err(|e| CampaignError::Contract(e.to_string()))?;
                if ["null", "true", "false"].contains(&token) {
                    Ok(token.into())
                } else {
                    Err(CampaignError::Contract("invalid JSON literal".into()))
                }
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(CampaignError::Contract(
                "invalid historical JSON value".into(),
            )),
        };
        self.depth -= 1;
        result
    }
    fn number(&mut self) -> Result<String> {
        let start = self.position;
        while self
            .input
            .get(self.position)
            .is_some_and(|b| matches!(b, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'))
        {
            self.position += 1;
        }
        let token = std::str::from_utf8(&self.input[start..self.position])
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        let value: Value =
            serde_json::from_str(token).map_err(|e| CampaignError::Contract(e.to_string()))?;
        if !value.is_number() {
            return Err(CampaignError::Contract("invalid JSON number".into()));
        }
        if !token.contains(['.', 'e', 'E']) {
            return Ok(if token == "-0" {
                "0".into()
            } else {
                token.into()
            });
        }
        let number = value
            .as_f64()
            .ok_or_else(|| CampaignError::Contract("historical float out of range".into()))?;
        if !number.is_finite() {
            return Err(CampaignError::Contract("nonfinite historical float".into()));
        }
        let mut output = value.to_string();
        if let Some((mantissa, exponent)) = output.split_once('e') {
            let exponent = exponent
                .parse::<i32>()
                .map_err(|e| CampaignError::Contract(e.to_string()))?;
            output = format!(
                "{mantissa}e{}{abs:02}",
                if exponent < 0 { '-' } else { '+' },
                abs = exponent.unsigned_abs()
            );
        } else if !output.contains('.') {
            output.push_str(".0");
        }
        Ok(output)
    }
    fn array(&mut self) -> Result<String> {
        self.take(b'[')?;
        let mut values = Vec::new();
        self.space();
        if self.input.get(self.position) != Some(&b']') {
            loop {
                values.push(self.value()?);
                self.space();
                if self.input.get(self.position) != Some(&b',') {
                    break;
                }
                self.position += 1;
            }
        }
        self.take(b']')?;
        Ok(format!("[{}]", values.join(",")))
    }
    fn object(&mut self) -> Result<String> {
        self.take(b'{')?;
        let mut fields = std::collections::BTreeMap::new();
        self.space();
        if self.input.get(self.position) != Some(&b'}') {
            loop {
                let key = self.string()?;
                self.take(b':')?;
                fields.insert(key, self.value()?);
                self.space();
                if self.input.get(self.position) != Some(&b',') {
                    break;
                }
                self.position += 1;
            }
        }
        self.take(b'}')?;
        let mut values = Vec::new();
        for (key, value) in fields {
            values.push(format!(
                "{}:{value}",
                serde_json::to_string(&key).map_err(|e| CampaignError::Contract(e.to_string()))?
            ));
        }
        Ok(format!("{{{}}}", values.join(",")))
    }
}
