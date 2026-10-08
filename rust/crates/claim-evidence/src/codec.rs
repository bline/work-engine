//! The domain-local `claim-evidence-legacy-json-v1` codec. Strings are UTF-16 code
//! units, including unpaired surrogates; no lossy Rust `String` conversion is
//! involved in parsing or canonical serialization.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::{ClaimError, ClaimResult};

pub const CODEC_ID: &str = "claim-evidence-legacy-json-v1";
const MAX_DEPTH: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct JsString(pub Vec<u16>);

impl JsString {
    pub fn new(value: &str) -> Self {
        Self(value.encode_utf16().collect())
    }

    pub fn to_string_checked(&self) -> ClaimResult<String> {
        String::from_utf16(&self.0)
            .map_err(|_| ClaimError::new("JSON text contains unpaired UTF-16 surrogate"))
    }

    pub fn is_nonempty_text(&self) -> bool {
        // ECMAScript String.prototype.trim whitespace, over UTF-16 code units.
        // Rust's trim also removes U+0085, which JavaScript retains.
        self.0.iter().any(|unit| {
            !matches!(*unit, 0x0009..=0x000d | 0x0020 | 0x00a0 | 0x1680
                | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f
                | 0x3000 | 0xfeff)
        })
    }
}

impl From<&str> for JsString {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum JsValue {
    Null,
    Bool(bool),
    Number(f64),
    String(JsString),
    Array(Vec<JsValue>),
    Object(BTreeMap<JsString, JsValue>),
}

impl JsValue {
    pub fn object<K: AsRef<str>, I: IntoIterator<Item = (K, JsValue)>>(fields: I) -> Self {
        Self::Object(
            fields
                .into_iter()
                .map(|(key, value)| (JsString::new(key.as_ref()), value))
                .collect(),
        )
    }

    pub fn text(value: &str) -> Self {
        Self::String(JsString::new(value))
    }

    pub fn get(&self, key: &str) -> Option<&JsValue> {
        match self {
            Self::Object(map) => map.get(&JsString::new(key)),
            _ => None,
        }
    }

    pub fn as_object(&self) -> ClaimResult<&BTreeMap<JsString, JsValue>> {
        match self {
            Self::Object(map) => Ok(map),
            _ => Err(ClaimError::new("expected JSON object")),
        }
    }

    pub fn as_array(&self) -> ClaimResult<&[JsValue]> {
        match self {
            Self::Array(values) => Ok(values),
            _ => Err(ClaimError::new("expected JSON array")),
        }
    }

    pub fn as_text(&self) -> ClaimResult<&JsString> {
        match self {
            Self::String(value) => Ok(value),
            _ => Err(ClaimError::new("expected JSON string")),
        }
    }

    pub fn nonempty_text(&self, label: &str) -> ClaimResult<&JsString> {
        let value = self.as_text()?;
        if !value.is_nonempty_text() {
            return Err(ClaimError::new(format!("{label} must be non-empty text")));
        }
        Ok(value)
    }

    pub fn as_safe_positive_integer(&self, label: &str) -> ClaimResult<u64> {
        match self {
            Self::Number(number)
                if number.is_finite()
                    && *number >= 1.0
                    && *number <= 9_007_199_254_740_991.0
                    && number.fract() == 0.0 =>
            {
                Ok(*number as u64)
            }
            _ => Err(ClaimError::new(format!(
                "{label} must be a positive safe integer"
            ))),
        }
    }
}

pub fn exact_fields(value: &JsValue, fields: &[&str], label: &str) -> ClaimResult<()> {
    let map = value.as_object()?;
    if map.len() != fields.len()
        || fields
            .iter()
            .any(|field| !map.contains_key(&JsString::new(field)))
    {
        return Err(ClaimError::new(format!("{label} fields are invalid")));
    }
    Ok(())
}

pub fn field<'a>(value: &'a JsValue, key: &str) -> ClaimResult<&'a JsValue> {
    value
        .get(key)
        .ok_or_else(|| ClaimError::new(format!("missing {key}")))
}

pub fn string_is(value: &JsValue, expected: &str) -> bool {
    value == &JsValue::text(expected)
}

pub(crate) fn canonical_json_unchecked(value: &JsValue) -> String {
    let mut output = String::new();
    write_value(value, &mut output);
    output.push('\n');
    output
}

pub(crate) fn digest_unchecked(value: &JsValue) -> String {
    let bytes = Sha256::digest(canonical_json_unchecked(value).as_bytes());
    format!("{bytes:x}")
}

/// Historical claim codec, checked at the same transport and parser boundary
/// used when durable bytes are read back.
pub fn canonical_json(value: &JsValue) -> ClaimResult<String> {
    validate_transport(value)?;
    let bytes = canonical_json_unchecked(value);
    if parse_json(&bytes)? != *value {
        return Err(ClaimError::new(
            "JSON value changed during canonical round-trip",
        ));
    }
    Ok(bytes)
}

pub fn digest(value: &JsValue) -> ClaimResult<String> {
    let bytes = canonical_json(value)?;
    Ok(format!("{:x}", Sha256::digest(bytes.as_bytes())))
}

fn codepoints(value: &JsString) -> Vec<u32> {
    std::char::decode_utf16(value.0.iter().copied())
        .map(|item| match item {
            Ok(ch) => ch as u32,
            Err(err) => err.unpaired_surrogate() as u32,
        })
        .collect()
}

pub fn validate_transport(value: &JsValue) -> ClaimResult<()> {
    validate_transport_at(value, 0)
}

fn validate_transport_at(value: &JsValue, depth: usize) -> ClaimResult<()> {
    if depth > MAX_DEPTH {
        return Err(ClaimError::new("JSON nesting exceeds codec limit"));
    }
    match value {
        JsValue::Number(n)
            if !n.is_finite() || (n.fract() == 0.0 && n.abs() > 9_007_199_254_740_991.0) =>
        {
            Err(ClaimError::new("unsafe JSON number"))
        }
        JsValue::Array(items) => {
            for item in items {
                validate_transport_at(item, depth + 1)?;
            }
            Ok(())
        }
        JsValue::Object(items) => {
            for item in items.values() {
                validate_transport_at(item, depth + 1)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn write_value(value: &JsValue, output: &mut String) {
    match value {
        JsValue::Null => output.push_str("null"),
        JsValue::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        JsValue::Number(number) => {
            if !number.is_finite() {
                output.push_str("null");
            } else if *number == 0.0 {
                output.push('0');
            } else {
                output.push_str(ryu_js::Buffer::new().format(*number));
            }
        }
        JsValue::String(value) => write_string(value, output),
        JsValue::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_value(value, output);
            }
            output.push(']');
        }
        JsValue::Object(values) => {
            output.push('{');
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_by(|(a, _), (b, _)| codepoints(a).cmp(&codepoints(b)));
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_string(key, output);
                output.push(':');
                write_value(value, output);
            }
            output.push('}');
        }
    }
}

fn write_string(value: &JsString, output: &mut String) {
    output.push('"');
    let mut index = 0;
    while index < value.0.len() {
        let unit = value.0[index];
        match unit {
            0x22 => output.push_str("\\\""),
            0x5c => output.push_str("\\\\"),
            0x08 => output.push_str("\\b"),
            0x09 => output.push_str("\\t"),
            0x0a => output.push_str("\\n"),
            0x0c => output.push_str("\\f"),
            0x0d => output.push_str("\\r"),
            0x0000..=0x001f => output.push_str(&format!("\\u{unit:04x}")),
            0xd800..=0xdbff
                if index + 1 < value.0.len() && (0xdc00..=0xdfff).contains(&value.0[index + 1]) =>
            {
                let low = value.0[index + 1];
                let scalar = 0x10000 + (((unit as u32 - 0xd800) << 10) | (low as u32 - 0xdc00));
                output.push(char::from_u32(scalar).expect("valid surrogate pair"));
                index += 1;
            }
            0xd800..=0xdfff => output.push_str(&format!("\\u{unit:04x}")),
            _ => output.push(char::from_u32(unit as u32).expect("valid BMP scalar")),
        }
        index += 1;
    }
    output.push('"');
}

pub fn parse_json(input: &str) -> ClaimResult<JsValue> {
    let mut parser = Parser { input, offset: 0 };
    let value = parser.value(0)?;
    parser.whitespace();
    if parser.offset != input.len() {
        return Err(ClaimError::new("trailing JSON bytes"));
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a str,
    offset: usize,
}

impl Parser<'_> {
    fn byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.offset).copied()
    }

    fn whitespace(&mut self) {
        while matches!(self.byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.offset += 1;
        }
    }

    fn take(&mut self, expected: u8) -> ClaimResult<()> {
        if self.byte() == Some(expected) {
            self.offset += 1;
            Ok(())
        } else {
            Err(ClaimError::new("invalid JSON syntax"))
        }
    }

    fn value(&mut self, depth: usize) -> ClaimResult<JsValue> {
        if depth > MAX_DEPTH {
            return Err(ClaimError::new("JSON nesting exceeds codec limit"));
        }
        self.whitespace();
        match self.byte() {
            Some(b'n') => {
                self.literal("null")?;
                Ok(JsValue::Null)
            }
            Some(b't') => {
                self.literal("true")?;
                Ok(JsValue::Bool(true))
            }
            Some(b'f') => {
                self.literal("false")?;
                Ok(JsValue::Bool(false))
            }
            Some(b'"') => Ok(JsValue::String(self.string()?)),
            Some(b'[') => self.array(depth + 1),
            Some(b'{') => self.object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(ClaimError::new("invalid JSON value")),
        }
    }

    fn literal(&mut self, literal: &str) -> ClaimResult<()> {
        if self.input[self.offset..].starts_with(literal) {
            self.offset += literal.len();
            Ok(())
        } else {
            Err(ClaimError::new("invalid JSON literal"))
        }
    }

    fn string(&mut self) -> ClaimResult<JsString> {
        self.take(b'"')?;
        let mut units = Vec::new();
        loop {
            match self.byte() {
                Some(b'"') => {
                    self.offset += 1;
                    return Ok(JsString(units));
                }
                Some(b'\\') => {
                    self.offset += 1;
                    let escaped = self
                        .byte()
                        .ok_or_else(|| ClaimError::new("incomplete JSON escape"))?;
                    self.offset += 1;
                    match escaped {
                        b'"' | b'\\' | b'/' => units.push(escaped as u16),
                        b'b' => units.push(8),
                        b'f' => units.push(12),
                        b'n' => units.push(10),
                        b'r' => units.push(13),
                        b't' => units.push(9),
                        b'u' => {
                            let end = self
                                .offset
                                .checked_add(4)
                                .ok_or_else(|| ClaimError::new("invalid JSON unicode escape"))?;
                            let hex = self
                                .input
                                .get(self.offset..end)
                                .ok_or_else(|| ClaimError::new("invalid JSON unicode escape"))?;
                            if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                                return Err(ClaimError::new("invalid JSON unicode escape"));
                            }
                            let unit = u16::from_str_radix(hex, 16)
                                .map_err(|_| ClaimError::new("invalid JSON unicode escape"))?;
                            units.push(unit);
                            self.offset = end;
                        }
                        _ => return Err(ClaimError::new("invalid JSON escape")),
                    }
                }
                Some(0..=31) => return Err(ClaimError::new("unescaped JSON control character")),
                Some(_) => {
                    let ch = self.input[self.offset..]
                        .chars()
                        .next()
                        .ok_or_else(|| ClaimError::new("incomplete JSON string"))?;
                    let mut buffer = [0u16; 2];
                    units.extend(ch.encode_utf16(&mut buffer).iter().copied());
                    self.offset += ch.len_utf8();
                }
                None => return Err(ClaimError::new("unterminated JSON string")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> ClaimResult<JsValue> {
        self.take(b'[')?;
        self.whitespace();
        let mut values = Vec::new();
        if self.byte() == Some(b']') {
            self.offset += 1;
            return Ok(JsValue::Array(values));
        }
        loop {
            values.push(self.value(depth)?);
            self.whitespace();
            match self.byte() {
                Some(b',') => self.offset += 1,
                Some(b']') => {
                    self.offset += 1;
                    return Ok(JsValue::Array(values));
                }
                _ => return Err(ClaimError::new("invalid JSON array")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> ClaimResult<JsValue> {
        self.take(b'{')?;
        self.whitespace();
        let mut values = BTreeMap::new();
        if self.byte() == Some(b'}') {
            self.offset += 1;
            return Ok(JsValue::Object(values));
        }
        loop {
            self.whitespace();
            let key = self.string()?;
            self.whitespace();
            self.take(b':')?;
            values.insert(key, self.value(depth)?);
            self.whitespace();
            match self.byte() {
                Some(b',') => self.offset += 1,
                Some(b'}') => {
                    self.offset += 1;
                    return Ok(JsValue::Object(values));
                }
                _ => return Err(ClaimError::new("invalid JSON object")),
            }
        }
    }

    fn number(&mut self) -> ClaimResult<JsValue> {
        let start = self.offset;
        if self.byte() == Some(b'-') {
            self.offset += 1;
        }
        match self.byte() {
            Some(b'0') => self.offset += 1,
            Some(b'1'..=b'9') => {
                while matches!(self.byte(), Some(b'0'..=b'9')) {
                    self.offset += 1;
                }
            }
            _ => return Err(ClaimError::new("invalid JSON number")),
        }
        if self.byte() == Some(b'.') {
            self.offset += 1;
            if !matches!(self.byte(), Some(b'0'..=b'9')) {
                return Err(ClaimError::new("invalid JSON fraction"));
            }
            while matches!(self.byte(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
        }
        if matches!(self.byte(), Some(b'e' | b'E')) {
            self.offset += 1;
            if matches!(self.byte(), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            if !matches!(self.byte(), Some(b'0'..=b'9')) {
                return Err(ClaimError::new("invalid JSON exponent"));
            }
            while matches!(self.byte(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
        }
        let number = self.input[start..self.offset]
            .parse::<f64>()
            .map_err(|_| ClaimError::new("invalid JSON number"))?;
        if !number.is_finite() {
            return Err(ClaimError::new("nonfinite JSON number is unsupported"));
        }
        Ok(JsValue::Number(number))
    }
}
