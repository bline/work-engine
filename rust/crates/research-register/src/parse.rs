//! Parse layer, run before typed decoding: strict UTF-8 (R-02), JSON documents and JSON Lines
//! (R-02), duplicate object keys at any depth (F-5) and non-finite numbers (F-6).
//!
//! JSON Lines are split on `\n` only; a trailing `\r` is JSON whitespace. Lines that are empty or
//! whitespace only are skipped. Line numbers are 1-based.

use crate::rules::{Location, RuleId, Violation};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use std::collections::HashSet;
use std::fmt;

/// A parsed JSON value. Objects keep their members in document order and never hold a duplicate
/// key. Numbers are finite.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The JSON type name, for messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Json::Null => "null",
            Json::Bool(_) => "boolean",
            Json::Number(_) => "number",
            Json::String(_) => "string",
            Json::Array(_) => "array",
            Json::Object(_) => "object",
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    /// The member named `key`, if this is an object that has it.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

const DUPLICATE_KEY: &str = "duplicate object key";

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(JsonVisitor)
    }
}

struct JsonVisitor;

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Json;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }
    fn visit_unit<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }
    fn visit_none<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }
    fn visit_bool<E>(self, v: bool) -> Result<Json, E> {
        Ok(Json::Bool(v))
    }
    fn visit_i64<E>(self, v: i64) -> Result<Json, E> {
        Ok(Json::Number(v.into()))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Json, E> {
        Ok(Json::Number(v.into()))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Json, E> {
        serde_json::Number::from_f64(v)
            .map(Json::Number)
            .ok_or_else(|| E::custom("non-finite number"))
    }
    fn visit_str<E>(self, v: &str) -> Result<Json, E> {
        Ok(Json::String(v.to_owned()))
    }
    fn visit_string<E>(self, v: String) -> Result<Json, E> {
        Ok(Json::String(v))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Json::Array(items))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut seen = HashSet::new();
        let mut members = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!("{DUPLICATE_KEY} {key:?}")));
            }
            members.push((key, map.next_value()?));
        }
        Ok(Json::Object(members))
    }
}

fn violation(rule: RuleId, file: &str, line: Option<u64>, detail: String) -> Violation {
    Violation {
        rule,
        location: Location {
            file: file.to_owned(),
            line,
            pointer: String::new(),
        },
        detail,
    }
}

/// The text of `bytes`, or an R-02 violation at the line of the first invalid byte.
pub fn utf8<'a>(file: &str, bytes: &'a [u8], out: &mut Vec<Violation>) -> Option<&'a str> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Some(text),
        Err(e) => {
            let at = e.valid_up_to();
            let line = bytes[..at].iter().filter(|b| **b == b'\n').count() as u64 + 1;
            out.push(violation(
                RuleId::R02,
                file,
                Some(line),
                format!("invalid UTF-8 at byte offset {at}"),
            ));
            None
        }
    }
}

/// One JSON document. Parse failures are located by the parser's line.
pub fn document(file: &str, bytes: &[u8], out: &mut Vec<Violation>) -> Option<Json> {
    let text = utf8(file, bytes, out)?;
    parse_text(file, None, text, out)
}

/// JSON Lines: one document per non-empty line, each paired with its 1-based line number.
/// A file that is not UTF-8 yields one violation and no records.
pub fn lines(file: &str, bytes: &[u8], out: &mut Vec<Violation>) -> Vec<(u64, Json)> {
    let Some(text) = utf8(file, bytes, out) else {
        return Vec::new();
    };
    let mut records = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let n = i as u64 + 1;
        if let Some(value) = parse_text(file, Some(n), line, out) {
            records.push((n, value));
        }
    }
    records
}

fn parse_text(file: &str, line: Option<u64>, text: &str, out: &mut Vec<Violation>) -> Option<Json> {
    match serde_json::from_str::<Json>(text) {
        Ok(value) => Some(value),
        Err(e) => {
            let message = e.to_string();
            let rule = if e.classify() == serde_json::error::Category::Data
                && message.starts_with(DUPLICATE_KEY)
            {
                RuleId::F5
            } else if message.starts_with("number out of range") || has_non_finite_token(text) {
                RuleId::F6
            } else {
                RuleId::R02
            };
            out.push(violation(
                rule,
                file,
                line.or(Some(e.line() as u64)),
                message,
            ));
            None
        }
    }
}

/// Whether `NaN` or `Infinity` (which also covers `-Infinity`) occurs outside a string.
fn has_non_finite_token(text: &str) -> bool {
    let bytes = text.as_bytes();
    let (mut in_string, mut escaped) = (false, false);
    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            match (escaped, b) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
        } else if b == b'"' {
            in_string = true;
        } else if bytes[i..].starts_with(b"NaN") || bytes[i..].starts_with(b"Infinity") {
            return true;
        }
    }
    false
}
