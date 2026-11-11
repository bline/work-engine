//! The restricted RFC 8785 (JCS) codec of contract section 17.5.
//!
//! The data model is: objects, arrays, strings, booleans, null (parsed so that the explicit-null
//! rule can refuse it with a position) and integers within +/-(2^53 - 1). Numbers are never
//! stored as `serde_json::Number`, so the bytes cannot depend on that type's feature-selected
//! representation (`arbitrary_precision`, `float_roundtrip`).
//!
//! Parsing refuses rather than normalizes: duplicate object keys (after unescaping), `-0`, floats,
//! exponents, out-of-range integers, unpaired surrogate escapes and trailing data. The only
//! normalization is the one RFC 8785 defines: key order by UTF-16 code unit, no insignificant
//! whitespace, minimal string escaping and integers without exponents.

use crate::digest::{Digest, DigestKind};
use crate::error::{CodecError, Position};

/// The largest accepted integer magnitude, 2^53 - 1.
pub(crate) const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// Container nesting limit. A recursion bound; the v0 schemas nest far less.
pub(crate) const MAX_DEPTH: usize = 64;

/// A JSON value restricted to the v0 data model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsonValue {
    Null,
    Bool(bool),
    Integer(i64),
    String(String),
    Array(Vec<JsonValue>),
    /// Members in authored (or insertion) order; the canonical form sorts them. Keys are unique.
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// Parses authored bytes. Accepts `null` (see [`JsonValue::refuse_null`]); refuses everything
    /// else outside the data model.
    pub(crate) fn parse(bytes: &[u8]) -> Result<JsonValue, CodecError> {
        let text = std::str::from_utf8(bytes).map_err(|error| CodecError::InvalidUtf8 {
            valid_up_to: error.valid_up_to(),
        })?;
        let mut parser = Parser {
            bytes: text.as_bytes(),
            text,
            index: 0,
        };
        parser.skip_whitespace();
        let value = parser.value(0)?;
        parser.skip_whitespace();
        if parser.index != parser.bytes.len() {
            return Err(parser.syntax("trailing data after the document"));
        }
        Ok(value)
    }

    /// Parses authored bytes under the v0 rules: [`JsonValue::parse`] followed by
    /// [`JsonValue::refuse_null`].
    pub(crate) fn parse_v0(bytes: &[u8]) -> Result<JsonValue, CodecError> {
        let value = JsonValue::parse(bytes)?;
        value.refuse_null()?;
        Ok(value)
    }

    /// Refuses an explicit `null` anywhere in the value (section 17.4: optional fields are
    /// absent, or present with a non-null value).
    pub(crate) fn refuse_null(&self) -> Result<(), CodecError> {
        fn walk(value: &JsonValue, path: &mut String) -> Result<(), CodecError> {
            match value {
                JsonValue::Null => Err(CodecError::ExplicitNull {
                    at: Position::Path(path.clone()),
                }),
                JsonValue::Array(items) => {
                    for (index, item) in items.iter().enumerate() {
                        let mark = path.len();
                        path.push_str(&format!("/{index}"));
                        walk(item, path)?;
                        path.truncate(mark);
                    }
                    Ok(())
                }
                JsonValue::Object(members) => {
                    for (key, member) in members {
                        let mark = path.len();
                        path.push('/');
                        path.push_str(&escape_pointer(key));
                        walk(member, path)?;
                        path.truncate(mark);
                    }
                    Ok(())
                }
                _ => Ok(()),
            }
        }
        walk(self, &mut String::new())
    }

    /// The RFC 8785 canonical bytes of this value. Array order is kept; object members are
    /// sorted by UTF-16 code unit. Sets are ordered by the caller before they reach this point.
    pub(crate) fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        write_value(self, &mut out);
        out
    }

    /// The framed digest of this value's canonical bytes.
    pub(crate) fn digest(&self, kind: DigestKind) -> Digest {
        Digest::frame(kind, &self.canonical_bytes())
    }
}

/// Checks the integer range of section 17.4.
pub(crate) fn check_integer(value: i64, at: Position) -> Result<i64, CodecError> {
    if value.unsigned_abs() > MAX_SAFE_INTEGER as u64 {
        Err(CodecError::IntegerOutOfRange { at })
    } else {
        Ok(value)
    }
}

fn escape_pointer(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn write_value(value: &JsonValue, out: &mut Vec<u8>) {
    match value {
        JsonValue::Null => out.extend_from_slice(b"null"),
        JsonValue::Bool(true) => out.extend_from_slice(b"true"),
        JsonValue::Bool(false) => out.extend_from_slice(b"false"),
        JsonValue::Integer(integer) => out.extend_from_slice(integer.to_string().as_bytes()),
        JsonValue::String(text) => write_string(text, out),
        JsonValue::Array(items) => {
            out.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_value(item, out);
            }
            out.push(b']');
        }
        JsonValue::Object(members) => {
            let mut ordered: Vec<&(String, JsonValue)> = members.iter().collect();
            ordered.sort_by(|left, right| left.0.encode_utf16().cmp(right.0.encode_utf16()));
            out.push(b'{');
            for (index, (key, member)) in ordered.into_iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_string(key, out);
                out.push(b':');
                write_value(member, out);
            }
            out.push(b'}');
        }
    }
}

/// RFC 8785 string serialization: `"` and `\` and the short control escapes, every other control
/// character below U+0020 as lowercase `\u00xx`, everything else as literal UTF-8.
fn write_string(text: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for ch in text.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{08}' => out.extend_from_slice(b"\\b"),
            '\u{09}' => out.extend_from_slice(b"\\t"),
            '\u{0a}' => out.extend_from_slice(b"\\n"),
            '\u{0c}' => out.extend_from_slice(b"\\f"),
            '\u{0d}' => out.extend_from_slice(b"\\r"),
            ch if (ch as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", ch as u32).as_bytes());
            }
            ch => {
                let mut buffer = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
            }
        }
    }
    out.push(b'"');
}

struct Parser<'a> {
    bytes: &'a [u8],
    text: &'a str,
    index: usize,
}

impl Parser<'_> {
    fn at(&self) -> Position {
        Position::Offset(self.index)
    }

    fn syntax(&self, detail: &'static str) -> CodecError {
        CodecError::Syntax {
            at: self.at(),
            detail,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.index += 1;
        }
    }

    fn expect_literal(
        &mut self,
        literal: &'static [u8],
        value: JsonValue,
    ) -> Result<JsonValue, CodecError> {
        if self.bytes[self.index..].starts_with(literal) {
            self.index += literal.len();
            Ok(value)
        } else {
            Err(self.syntax("invalid literal"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<JsonValue, CodecError> {
        match self.peek() {
            None => Err(self.syntax("unexpected end of input")),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(JsonValue::String(self.string()?)),
            Some(b't') => self.expect_literal(b"true", JsonValue::Bool(true)),
            Some(b'f') => self.expect_literal(b"false", JsonValue::Bool(false)),
            Some(b'n') => self.expect_literal(b"null", JsonValue::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.syntax("unexpected character")),
        }
    }

    fn enter(&self, depth: usize) -> Result<usize, CodecError> {
        if depth >= MAX_DEPTH {
            Err(CodecError::TooDeep {
                limit: MAX_DEPTH,
                at: self.at(),
            })
        } else {
            Ok(depth + 1)
        }
    }

    fn array(&mut self, depth: usize) -> Result<JsonValue, CodecError> {
        let depth = self.enter(depth)?;
        self.index += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.index += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.value(depth)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.index += 1,
                Some(b']') => {
                    self.index += 1;
                    return Ok(JsonValue::Array(items));
                }
                _ => return Err(self.syntax("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<JsonValue, CodecError> {
        let depth = self.enter(depth)?;
        self.index += 1;
        let mut members: Vec<(String, JsonValue)> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.index += 1;
            return Ok(JsonValue::Object(members));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.syntax("expected an object key string"));
            }
            let key_at = self.at();
            let key = self.string()?;
            if members.iter().any(|(existing, _)| *existing == key) {
                return Err(CodecError::DuplicateKey { key, at: key_at });
            }
            self.skip_whitespace();
            if self.peek() != Some(b':') {
                return Err(self.syntax("expected ':' after an object key"));
            }
            self.index += 1;
            self.skip_whitespace();
            let member = self.value(depth)?;
            members.push((key, member));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.index += 1,
                Some(b'}') => {
                    self.index += 1;
                    return Ok(JsonValue::Object(members));
                }
                _ => return Err(self.syntax("expected ',' or '}'")),
            }
        }
    }

    fn number(&mut self) -> Result<JsonValue, CodecError> {
        let start = self.index;
        let negative = self.peek() == Some(b'-');
        if negative {
            self.index += 1;
        }
        let digits_start = self.index;
        match self.peek() {
            Some(b'0') => {
                self.index += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(self.syntax("leading zero in number"));
                }
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.index += 1;
                }
            }
            _ => return Err(self.syntax("expected a digit")),
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(CodecError::NonIntegerNumber {
                at: Position::Offset(start),
            });
        }
        let digits = &self.text[digits_start..self.index];
        // At most 16 digits can be in range; anything longer is out of range without parsing.
        let magnitude = digits.trim_start_matches('0');
        let in_range = magnitude.len() <= 16
            && magnitude.parse::<u64>().unwrap_or(0) <= MAX_SAFE_INTEGER as u64;
        if !in_range {
            return Err(CodecError::IntegerOutOfRange {
                at: Position::Offset(start),
            });
        }
        let magnitude = digits.parse::<i64>().unwrap_or(0);
        if negative && magnitude == 0 {
            return Err(CodecError::NegativeZero {
                at: Position::Offset(start),
            });
        }
        Ok(JsonValue::Integer(if negative {
            -magnitude
        } else {
            magnitude
        }))
    }

    fn hex4(&mut self) -> Result<u32, CodecError> {
        let mut code = 0u32;
        for _ in 0..4 {
            let digit = match self.peek() {
                Some(byte @ b'0'..=b'9') => u32::from(byte - b'0'),
                Some(byte @ b'a'..=b'f') => u32::from(byte - b'a') + 10,
                Some(byte @ b'A'..=b'F') => u32::from(byte - b'A') + 10,
                _ => return Err(self.syntax("expected four hexadecimal digits")),
            };
            code = code * 16 + digit;
            self.index += 1;
        }
        Ok(code)
    }

    fn string(&mut self) -> Result<String, CodecError> {
        self.index += 1;
        let mut out = String::new();
        loop {
            let run_start = self.index;
            while let Some(byte) = self.peek() {
                if byte == b'"' || byte == b'\\' || byte < 0x20 {
                    break;
                }
                self.index += 1;
            }
            // `text` is valid UTF-8 and the run stops only at ASCII bytes, so it ends on a
            // character boundary.
            out.push_str(&self.text[run_start..self.index]);
            match self.peek() {
                None => return Err(self.syntax("unterminated string")),
                Some(b'"') => {
                    self.index += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    let escape_at = self.at();
                    self.index += 1;
                    let Some(escape) = self.peek() else {
                        return Err(self.syntax("unterminated escape"));
                    };
                    self.index += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let first = self.hex4()?;
                            let code = match first {
                                0xD800..=0xDBFF => {
                                    if self.bytes[self.index..].starts_with(b"\\u") {
                                        self.index += 2;
                                        let second = self.hex4()?;
                                        if !(0xDC00..=0xDFFF).contains(&second) {
                                            return Err(CodecError::UnpairedSurrogate {
                                                at: escape_at,
                                            });
                                        }
                                        0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                                    } else {
                                        return Err(CodecError::UnpairedSurrogate {
                                            at: escape_at,
                                        });
                                    }
                                }
                                0xDC00..=0xDFFF => {
                                    return Err(CodecError::UnpairedSurrogate { at: escape_at });
                                }
                                other => other,
                            };
                            match char::from_u32(code) {
                                Some(ch) => out.push(ch),
                                None => {
                                    return Err(CodecError::UnpairedSurrogate { at: escape_at });
                                }
                            }
                        }
                        _ => return Err(self.syntax("invalid escape")),
                    }
                }
                Some(_) => return Err(self.syntax("unescaped control character in string")),
            }
        }
    }
}
