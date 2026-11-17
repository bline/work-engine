//! Exact JSON integer semantics for schema-valid numbers, without any `serde_json` feature
//! change.
//!
//! Draft 2020-12 treats `1.0`, `1e0` and `4294967295.0` as the integers they denote and
//! `4294967295.0000000000000001` as a non-integer. The default `serde_json` parser collapses
//! number literals to `f64`, which rounds `18446744073709551615.0` to 2^64, accepts a fraction
//! that rounds to an integer, and rejects `1e400` outright. `parse_json` therefore reads the
//! literal with exact decimal arithmetic (never via `f64`) and stores a `Value` that preserves
//! every schema-observable fact: integral literals within `i64`/`u64` become exact integers;
//! an integral literal beyond them becomes `±1e40` (an integral number above/below every
//! published bound); a non-integral literal keeps its `f64` unless `f64` would collapse it to an
//! integer or infinity, in which case it becomes `0.5` (non-integral, below every published
//! bound). Only integer-typed locations are numerically constrained by the published schemas.
use serde_json::{Map, Number, Value};

/// Integral values wider than this are never expanded; 40 digits cover the full `i128`/`u128`
/// range that any published numeric domain uses.
const MAX_EXPANDED_DIGITS: i64 = 40;
/// Exponent magnitude beyond which an exponent literal is clamped before arithmetic.
const EXPONENT_CLAMP: i64 = 1 << 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberForm {
    /// Integral value: canonical decimal digits without sign, leading zeros or exponent.
    Integer { negative: bool, digits: String },
    /// Integral value with more than 40 decimal digits (not expanded).
    LargeInteger { negative: bool },
    /// Mathematically non-integral value.
    NonInteger,
}

fn clamp_exponent(digits: &str, negative: bool) -> i64 {
    let magnitude = digits
        .parse::<i64>()
        .map_or(EXPONENT_CLAMP, |m| m.min(EXPONENT_CLAMP));
    if negative { -magnitude } else { magnitude }
}

/// Classify a JSON number literal exactly.
pub fn classify_literal(text: &str) -> NumberForm {
    let (negative, rest) = text.strip_prefix('-').map_or((false, text), |r| (true, r));
    let (mantissa, exponent) = match rest.find(['e', 'E']) {
        Some(at) => {
            let tail = &rest[at + 1..];
            let (sign, digits) = match tail.strip_prefix('-') {
                Some(d) => (true, d),
                None => (false, tail.strip_prefix('+').unwrap_or(tail)),
            };
            (&rest[..at], clamp_exponent(digits, sign))
        }
        None => (rest, 0),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut digits: String = integer
        .chars()
        .chain(fraction.chars())
        .skip_while(|c| *c == '0')
        .collect();
    if digits.is_empty() {
        return NumberForm::Integer {
            negative: false,
            digits: "0".into(),
        };
    }
    let mut scale = exponent - fraction.len() as i64;
    while digits.ends_with('0') {
        digits.pop();
        scale += 1;
    }
    if scale < 0 {
        return NumberForm::NonInteger;
    }
    if digits.len() as i64 + scale > MAX_EXPANDED_DIGITS {
        return NumberForm::LargeInteger { negative };
    }
    digits.extend(std::iter::repeat_n('0', scale as usize));
    NumberForm::Integer { negative, digits }
}

/// Exact value of an integral JSON number within `u64`; `None` for every other value
/// (non-numbers, non-integers, negatives, and integers above `u64::MAX`). A `Value` built by the
/// standard parser holds `f64` for decimal/exponent literals; such a number is read at face
/// value (an integral `f64` below 2^64 is an exact integer).
pub fn exact_u64(value: &Value) -> Option<u64> {
    let Value::Number(n) = value else { return None };
    if let Some(v) = n.as_u64() {
        return Some(v);
    }
    let f = n.as_f64()?;
    (n.is_f64() && f.is_finite() && f.fract() == 0.0 && (0.0..18446744073709551616.0).contains(&f))
        .then_some(f as u64)
}

pub fn exact_u32(value: &Value) -> Option<u32> {
    exact_u64(value).and_then(|v| u32::try_from(v).ok())
}

/// Rewrite `f64`-backed integral numbers (`1.0`) to exact integers so equality and `as_u64`
/// agree with the integer the schema accepted. Everything else is preserved.
pub fn canonicalize_integers(value: &Value) -> Value {
    match value {
        Value::Number(n) if n.is_f64() => match n.as_f64() {
            Some(f) if f.is_finite() && f.fract() == 0.0 && (-9223372036854775808.0..0.0).contains(&f) => {
                Value::Number(Number::from(f as i64))
            }
            Some(f) if f.is_finite() && f.fract() == 0.0 && (0.0..18446744073709551616.0).contains(&f) => {
                Value::Number(Number::from(f as u64))
            }
            _ => value.clone(),
        },
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_integers).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), canonicalize_integers(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn sentinel(negative: bool) -> Value {
    Value::Number(Number::from_f64(if negative { -1e40 } else { 1e40 }).expect("finite"))
}

fn number_value(text: &str) -> Value {
    match classify_literal(text) {
        NumberForm::Integer { negative, digits } => {
            let parsed = if negative {
                digits.parse::<i64>().ok().map(|d| Number::from(-d))
            } else {
                digits.parse::<u64>().ok().map(Number::from)
            };
            match (parsed, negative) {
                (Some(n), _) => Value::Number(n),
                (None, _) if negative && digits == "9223372036854775808" => {
                    Value::Number(Number::from(i64::MIN))
                }
                (None, n) => sentinel(n),
            }
        }
        NumberForm::LargeInteger { negative } => sentinel(negative),
        NumberForm::NonInteger => match text.parse::<f64>().ok().and_then(Number::from_f64) {
            Some(n) if n.as_f64().is_some_and(|f| f.fract() != 0.0) => Value::Number(n),
            _ => Value::Number(Number::from_f64(0.5).expect("finite")),
        },
    }
}

/// Recursion bound identical to `serde_json`'s, so deeply nested input is a controlled error.
const MAX_DEPTH: usize = 127;

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn fail<T>(&self, why: &str) -> Result<T, String> {
        Err(format!("JSON: {why} at byte {}", self.at))
    }
    fn skip_space(&mut self) {
        while matches!(self.bytes.get(self.at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }
    fn eat(&mut self, byte: u8) -> bool {
        (self.bytes.get(self.at) == Some(&byte)).then(|| self.at += 1).is_some()
    }
    fn literal(&mut self, word: &str, value: Value) -> Result<Value, String> {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            self.fail("expected value")
        }
    }
    fn value(&mut self, depth: usize) -> Result<Value, String> {
        self.skip_space();
        match self.bytes.get(self.at) {
            Some(b'n') => self.literal("null", Value::Null),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b'[') => {
                if depth >= MAX_DEPTH {
                    return self.fail("recursion limit exceeded");
                }
                self.at += 1;
                let mut items = Vec::new();
                self.skip_space();
                if self.eat(b']') {
                    return Ok(Value::Array(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.skip_space();
                    if self.eat(b',') {
                        continue;
                    }
                    if self.eat(b']') {
                        return Ok(Value::Array(items));
                    }
                    return self.fail("expected ',' or ']'");
                }
            }
            Some(b'{') => {
                if depth >= MAX_DEPTH {
                    return self.fail("recursion limit exceeded");
                }
                self.at += 1;
                let mut map = Map::new();
                self.skip_space();
                if self.eat(b'}') {
                    return Ok(Value::Object(map));
                }
                loop {
                    self.skip_space();
                    if self.bytes.get(self.at) != Some(&b'"') {
                        return self.fail("expected string key");
                    }
                    let key = self.string()?;
                    self.skip_space();
                    if !self.eat(b':') {
                        return self.fail("expected ':'");
                    }
                    let value = self.value(depth + 1)?;
                    map.insert(key, value);
                    self.skip_space();
                    if self.eat(b',') {
                        continue;
                    }
                    if self.eat(b'}') {
                        return Ok(Value::Object(map));
                    }
                    return self.fail("expected ',' or '}'");
                }
            }
            _ => self.fail("expected value"),
        }
    }
    fn digits(&mut self) -> usize {
        let start = self.at;
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
        self.at - start
    }
    fn number(&mut self) -> Result<Value, String> {
        let start = self.at;
        self.eat(b'-');
        match self.bytes.get(self.at) {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return self.fail("invalid number"),
        }
        if self.eat(b'.') && self.digits() == 0 {
            return self.fail("invalid number fraction");
        }
        if matches!(self.bytes.get(self.at), Some(b'e' | b'E')) {
            self.at += 1;
            if !self.eat(b'+') {
                self.eat(b'-');
            }
            if self.digits() == 0 {
                return self.fail("invalid number exponent");
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.at]).expect("ascii number");
        Ok(number_value(text))
    }
    fn hex4(&mut self) -> Result<u32, String> {
        let slice = self.bytes.get(self.at..self.at + 4);
        let value = slice
            .and_then(|s| std::str::from_utf8(s).ok())
            .filter(|s| s.bytes().all(|b| b.is_ascii_hexdigit()))
            .and_then(|s| u32::from_str_radix(s, 16).ok());
        match value {
            Some(v) => {
                self.at += 4;
                Ok(v)
            }
            None => self.fail("invalid unicode escape"),
        }
    }
    fn string(&mut self) -> Result<String, String> {
        self.at += 1;
        let mut out = Vec::new();
        loop {
            let Some(&b) = self.bytes.get(self.at) else {
                return self.fail("unterminated string");
            };
            self.at += 1;
            match b {
                b'"' => {
                    return String::from_utf8(out).or_else(|_| self.fail("invalid UTF-8 in string"));
                }
                0..=0x1f => return self.fail("control character in string"),
                b'\\' => {
                    let Some(&e) = self.bytes.get(self.at) else {
                        return self.fail("unterminated escape");
                    };
                    self.at += 1;
                    let simple = match e {
                        b'"' => Some('"'),
                        b'\\' => Some('\\'),
                        b'/' => Some('/'),
                        b'b' => Some('\u{8}'),
                        b'f' => Some('\u{c}'),
                        b'n' => Some('\n'),
                        b'r' => Some('\r'),
                        b't' => Some('\t'),
                        b'u' => None,
                        _ => return self.fail("invalid escape"),
                    };
                    let ch = match simple {
                        Some(c) => c,
                        None => {
                            let first = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&first) {
                                if !(self.eat(b'\\') && self.eat(b'u')) {
                                    return self.fail("lone leading surrogate");
                                }
                                let second = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&second) {
                                    return self.fail("invalid surrogate pair");
                                }
                                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                            } else {
                                first
                            };
                            match char::from_u32(code) {
                                Some(c) => c,
                                None => return self.fail("invalid unicode scalar"),
                            }
                        }
                    };
                    let mut buffer = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
                }
                other => out.push(other),
            }
        }
    }
}

/// Parse one JSON document with exact integer semantics (see the module comment). Same grammar,
/// duplicate-key (last wins) and nesting bound as `serde_json::from_slice`.
pub fn parse_json(bytes: &[u8]) -> Result<Value, String> {
    let mut reader = Reader { bytes, at: 0 };
    let value = reader.value(0)?;
    reader.skip_space();
    if reader.at != bytes.len() {
        return reader.fail("trailing characters");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn int(negative: bool, digits: &str) -> NumberForm {
        NumberForm::Integer {
            negative,
            digits: digits.into(),
        }
    }
    #[test]
    fn classifies_exactly() {
        for (text, want) in [
            ("0", int(false, "0")),
            ("-0", int(false, "0")),
            ("-0.0e5", int(false, "0")),
            ("1.0", int(false, "1")),
            ("1e0", int(false, "1")),
            ("1E+0", int(false, "1")),
            ("10e-1", int(false, "1")),
            ("4.294967295e9", int(false, "4294967295")),
            ("4294967295.000", int(false, "4294967295")),
            ("-12e1", int(true, "120")),
            ("18446744073709551615.0", int(false, "18446744073709551615")),
            ("1.8446744073709551615e19", int(false, "18446744073709551615")),
            ("1.5", NumberForm::NonInteger),
            ("4294967295.0000000000000001", NumberForm::NonInteger),
            ("1e-999999999999999999999", NumberForm::NonInteger),
            ("0.000000000000000000000000001e27", int(false, "1")),
            ("1e40", NumberForm::LargeInteger { negative: false }),
            ("-1e999999999999999999999", NumberForm::LargeInteger { negative: true }),
            ("0e999999999999999999999", int(false, "0")),
        ] {
            assert_eq!(classify_literal(text), want, "{text}");
        }
    }
    #[test]
    fn u64_frontier_is_exact_without_float_rounding() {
        let parse = |s: &str| parse_json(s.as_bytes()).unwrap();
        assert_eq!(exact_u64(&parse("18446744073709551615")), Some(u64::MAX));
        assert_eq!(exact_u64(&parse("18446744073709551615.0")), Some(u64::MAX));
        assert_eq!(exact_u64(&parse("1.8446744073709551615e19")), Some(u64::MAX));
        assert_eq!(exact_u64(&parse("18446744073709551616")), None);
        assert_eq!(exact_u64(&parse("18446744073709551616.0")), None);
        assert_eq!(exact_u64(&parse("18446744073709551615.5")), None);
        assert_eq!(exact_u64(&parse("18446744073709551615.00000000000000000000001")), None);
        assert_eq!(exact_u64(&parse("1e400")), None);
        assert_eq!(exact_u64(&parse("-1")), None);
        assert_eq!(exact_u64(&parse("-0")), Some(0));
        assert_eq!(exact_u64(&parse("true")), None);
        assert_eq!(exact_u64(&parse("\"1\"")), None);
        assert_eq!(exact_u32(&parse("4294967295.0")), Some(u32::MAX));
        assert_eq!(exact_u32(&parse("4294967296")), None);
        // a Value from the standard parser is read at face value, never panics
        let standard: Value = serde_json::from_str("[1.0, 18446744073709551616, 1e300]").unwrap();
        assert_eq!(exact_u64(&standard[0]), Some(1));
        assert_eq!(exact_u64(&standard[1]), None);
        assert_eq!(exact_u64(&standard[2]), None);
    }
    #[test]
    fn exact_parser_matches_serde_json_on_every_non_numeric_construct() {
        let documents = [
            r#"{"a":[1,2,{"b":null}],"c":"x\u00e9\ud83d\ude00\n\"\\\/","a":true,"d":-5,"e":0.25,"f":""}"#,
            " \t\n[ ] ",
            r#"{"é":"ü","k":[[],{}]}"#,
        ];
        for text in documents {
            let ours = parse_json(text.as_bytes()).unwrap();
            let theirs: Value = serde_json::from_str(text).unwrap();
            assert_eq!(ours, theirs, "{text}");
        }
        for text in [
            "", "[", "[1,]", "{\"a\":1,}", "01", "1.", ".5", "1e", "+1", "nul", "\"a", "\"\\x\"", "\"\\ud800\"",
            "\"\\udc00\"", "[1] x", "\u{feff}[]", "\"a\tb\"", "{1:2}", "[1 2]", "tru", "-", "--1", "0x1", "NaN",
        ] {
            assert!(parse_json(text.as_bytes()).is_err(), "{text:?}");
            assert!(serde_json::from_str::<Value>(text).is_err(), "serde {text:?}");
        }
        assert!(parse_json(b"\"\xff\"").is_err());
        let deep = format!("{}{}", "[".repeat(128), "]".repeat(128));
        assert!(parse_json(deep.as_bytes()).is_err());
        assert!(serde_json::from_str::<Value>(&deep).is_err());
        let ok = format!("{}{}", "[".repeat(127), "]".repeat(127));
        assert!(parse_json(ok.as_bytes()).is_ok());
        assert!(serde_json::from_str::<Value>(&ok).is_ok());
    }
}
