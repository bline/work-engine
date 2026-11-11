//! Parse layer, run before typed decoding: strict UTF-8 (R-02), JSON documents and JSON Lines
//! (R-02), duplicate object keys at any depth (F-5) and non-finite numbers (F-6).
//!
//! JSON Lines are split on `\n` only; a trailing `\r` is JSON whitespace. Lines that are empty or
//! whitespace only are skipped. Line numbers are 1-based.
//!
//! Numbers keep their exact source text ([`Number`]), so an integer of any magnitude is preserved
//! losslessly (R-07, owner decision D-CF02). serde_json (without `arbitrary_precision`) would
//! read an integer outside i64/u64 as an approximate float, and one longer than about 309 digits
//! as out of range. So before parsing, [`number_literals`] replaces each valid JSON number literal
//! outside strings with `0` padded by spaces to the literal's length (lines and columns are
//! unchanged), and the visitor pairs each placeholder it is given, in document order, with the
//! next saved literal. The pairing is checked: every visit must be a placeholder and every saved
//! literal must be used. A float literal that overflows f64 (`1e400`) is left in place, so serde
//! reports it out of range (F-6) as before; invalid literals are left for serde to reject.

use crate::rules::{Location, RuleId, Violation};
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use std::cell::Cell;
use std::collections::HashSet;
use std::fmt;

/// A JSON number: the exact text of a valid, finite JSON number literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Number(String);

impl Number {
    /// The literal as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// An integer in R-07's sense: a literal with no fraction and no exponent, of any magnitude
    /// (`-0` included; `1.0` and `1e3` are not).
    pub fn is_integer(&self) -> bool {
        !self.0.bytes().any(|b| matches!(b, b'.' | b'e' | b'E'))
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A parsed JSON value. Objects keep their members in document order and never hold a duplicate
/// key. Numbers are finite and keep their source text.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(Number),
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

const PAIRING: &str = "number literal pairing failed";

/// The saved number literals of one text, consumed in document order.
struct Literals {
    list: Vec<String>,
    next: Cell<usize>,
}

impl Literals {
    fn take<E: de::Error>(&self, placeholder: u64) -> Result<Json, E> {
        let i = self.next.get();
        match (placeholder, self.list.get(i)) {
            (0, Some(text)) => {
                self.next.set(i + 1);
                Ok(Json::Number(Number(text.clone())))
            }
            _ => Err(E::custom(PAIRING)),
        }
    }
}

#[derive(Clone, Copy)]
struct JsonSeed<'l>(&'l Literals);

impl<'de> DeserializeSeed<'de> for JsonSeed<'_> {
    type Value = Json;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Json, D::Error> {
        deserializer.deserialize_any(JsonVisitor(self.0))
    }
}

struct JsonVisitor<'l>(&'l Literals);

impl<'de> Visitor<'de> for JsonVisitor<'_> {
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
    /// Every number serde sees is a `0` placeholder; anything else breaks the pairing.
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Json, E> {
        Err(E::custom(PAIRING))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Json, E> {
        self.0.take(v)
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Json, E> {
        Err(E::custom(PAIRING))
    }
    fn visit_str<E>(self, v: &str) -> Result<Json, E> {
        Ok(Json::String(v.to_owned()))
    }
    fn visit_string<E>(self, v: String) -> Result<Json, E> {
        Ok(Json::String(v))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element_seed(JsonSeed(self.0))? {
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
            members.push((key, map.next_value_seed(JsonSeed(self.0))?));
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

/// `text` with every valid, finite JSON number literal outside strings replaced by `0` and
/// spaces of the same length, and those literals in document order. A literal is the maximal run
/// of `0-9 + - . e E` starting at `-` or a digit; runs that are not valid JSON numbers, and float
/// literals that overflow f64, are left unchanged for serde_json to reject.
fn number_literals(text: &str) -> (String, Vec<String>) {
    let bytes = text.as_bytes();
    let mut masked = bytes.to_vec();
    let mut literals = Vec::new();
    let (mut in_string, mut escaped) = (false, false);
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            match (escaped, b) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            i += 1;
        } else if b == b'"' {
            in_string = true;
            i += 1;
        } else if b == b'-' || b.is_ascii_digit() {
            let start = i;
            while i < bytes.len()
                && matches!(bytes[i], b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E')
            {
                i += 1;
            }
            let literal = &text[start..i];
            let keep = is_json_number(literal)
                && (Number(literal.to_owned()).is_integer()
                    || literal.parse::<f64>().is_ok_and(f64::is_finite));
            if keep {
                masked[start] = b'0';
                masked[start + 1..i].fill(b' ');
                literals.push(literal.to_owned());
            }
        } else {
            i += 1;
        }
    }
    // Only ASCII bytes inside literals were replaced by ASCII, so the text stays UTF-8.
    let masked = String::from_utf8(masked).expect("ASCII replacement keeps UTF-8");
    (masked, literals)
}

/// RFC 8259 `number`: `-? (0 | [1-9][0-9]*) (. [0-9]+)? ([eE] [+-]? [0-9]+)?`, whole text.
fn is_json_number(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = usize::from(b.first() == Some(&b'-'));
    let digits = |i: &mut usize| {
        let start = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - start
    };
    match b.get(i) {
        Some(b'0') => i += 1,
        Some(b'1'..=b'9') => {
            digits(&mut i);
        }
        _ => return false,
    }
    if b.get(i) == Some(&b'.') {
        i += 1;
        if digits(&mut i) == 0 {
            return false;
        }
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        if digits(&mut i) == 0 {
            return false;
        }
    }
    i == b.len()
}

/// Parse one JSON text with numbers kept as their literals (see the module notes).
fn parse_json(text: &str) -> Result<Json, serde_json::Error> {
    let (masked, list) = number_literals(text);
    let literals = Literals {
        list,
        next: Cell::new(0),
    };
    let mut de = serde_json::Deserializer::from_str(&masked);
    let value = JsonSeed(&literals).deserialize(&mut de)?;
    de.end()?;
    if literals.next.get() != literals.list.len() {
        return Err(de::Error::custom(PAIRING));
    }
    Ok(value)
}

fn parse_text(file: &str, line: Option<u64>, text: &str, out: &mut Vec<Violation>) -> Option<Json> {
    match parse_json(text) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn numbers(v: &Json, out: &mut Vec<String>) {
        match v {
            Json::Number(n) => out.push(n.as_str().to_owned()),
            Json::Array(items) => items.iter().for_each(|i| numbers(i, out)),
            Json::Object(members) => members.iter().for_each(|(_, m)| numbers(m, out)),
            _ => {}
        }
    }

    #[test]
    fn number_literals_pair_in_document_order() {
        let text = r#"{"a\"1": "2\\", "k": -0, "12": [4.5e-1, "5\"6", 7E+2, {"x": [[8], 1e-400]}], "t": [true, false, null, 18446744073709551616]}"#;
        let v = parse_json(text).unwrap();
        let mut got = Vec::new();
        numbers(&v, &mut got);
        assert_eq!(
            got,
            [
                "-0",
                "4.5e-1",
                "7E+2",
                "8",
                "1e-400",
                "18446744073709551616"
            ]
        );
    }

    #[test]
    fn json_number_grammar() {
        for ok in [
            "0", "-0", "1", "-12", "1.5", "1e5", "1E+5", "1e-05", "-0.0e0",
        ] {
            assert!(is_json_number(ok), "{ok}");
        }
        for bad in [
            "", "-", "01", "-01", "1.", ".1", "+1", "1e", "1e+", "1.e1", "1-2", "--1",
        ] {
            assert!(!is_json_number(bad), "{bad}");
        }
    }

    /// Masking keeps serde_json's error positions: the same line and column as on the original.
    #[test]
    fn errors_keep_their_positions() {
        for text in [
            "[123, x]",
            "{\"a\": 12345 6}",
            "[1,\n 22, 333 }",
            "[01]",
            "[1e400]",
        ] {
            let ours = parse_json(text).unwrap_err();
            let theirs = serde_json::from_str::<serde_json::Value>(text).unwrap_err();
            assert_eq!(
                (ours.line(), ours.column()),
                (theirs.line(), theirs.column()),
                "{text}"
            );
        }
    }
}
