//! Byte-level reproductions of the JS behaviours the revisions and validators depend on:
//! `canonical()` (sorted-key `JSON.stringify`), `Number#toString` formatting, `String#trim`,
//! `Number.isSafeInteger`, and the ES date-time string grammar behind `Date.parse`.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub(crate) const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// `sha256:<hex>` over the canonical JSON text.
pub(crate) fn revision(value: &Value) -> String {
    let digest = Sha256::digest(canonical(value).as_bytes());
    let mut out = String::with_capacity(71);
    out.push_str("sha256:");
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// The JS `canonical()`: arrays in order, object keys sorted by UTF-16 code unit, scalars as
/// `JSON.stringify` writes them.
pub(crate) fn canonical(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(number) => out.push_str(&js_number(number.as_f64().unwrap_or(f64::NAN))),
        Value::String(text) => write_quoted(text, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_quoted(key, out);
                out.push(':');
                write_canonical(&map[key], out);
            }
            out.push('}');
        }
    }
}

/// `JSON.stringify` for a string (well-formed: Rust strings hold no lone surrogates).
fn write_quoted(text: &str, out: &mut String) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

/// ES `Number::toString(10)` as `JSON.stringify` applies it; non-finite becomes `null`.
///
/// Digit selection, the choice between equally short candidates, and the exponent thresholds
/// are delegated to `ryu-js`, which implements the ECMAScript algorithm. Rust's own shortest
/// formatting breaks ties between equally short candidates differently, so it is not used.
pub(crate) fn js_number(value: f64) -> String {
    if !value.is_finite() {
        return "null".to_owned();
    }
    ryu_js::Buffer::new().format_finite(value).to_owned()
}

/// `Number.isSafeInteger` on a parsed JSON number.
pub(crate) fn safe_integer(value: &Value) -> Option<i64> {
    let number = value.as_f64()?;
    if number.is_finite() && number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER {
        Some(number as i64)
    } else {
        None
    }
}

/// JS `WhiteSpace` and `LineTerminator` code points, the set `String#trim` removes.
fn is_js_whitespace(ch: char) -> bool {
    matches!(
        ch,
        '\u{9}'..='\u{d}'
            | '\u{20}'
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

/// `value.trim() === ""`.
pub(crate) fn trims_to_empty(text: &str) -> bool {
    text.chars().all(is_js_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formatting_follows_es() {
        assert_eq!(js_number(1.0), "1");
        assert_eq!(js_number(-0.0), "0");
        assert_eq!(js_number(0.1), "0.1");
        assert_eq!(js_number(1e21), "1e+21");
        assert_eq!(js_number(1e20), "100000000000000000000");
        assert_eq!(js_number(1.5e-7), "1.5e-7");
        assert_eq!(js_number(0.000001), "0.000001");
        assert_eq!(js_number(123.456), "123.456");
        assert_eq!(js_number(-12345.5), "-12345.5");
    }

    /// Differential check against Node's `JSON.stringify` over the generator's sample of f64
    /// bit patterns (`numberCases` in `tests/golden/vectors.json`; seed, stratum sizes and edge
    /// values are documented in `tests/golden/generate.mjs`). Each entry is
    /// `[bits as 16 hex digits, expected text]`. This is a deterministic sample, not a proof over
    /// every f64.
    #[test]
    fn number_formatting_matches_node_sample() {
        let fixture: Value =
            serde_json::from_str(include_str!("../tests/golden/vectors.json")).expect("fixture");
        let cases = fixture["numberCases"].as_array().expect("numberCases");
        let declared = &fixture["header"]["numberSample"];
        let edge_count = cases.len() as u64
            - declared["uniform"].as_u64().expect("uniform count")
            - declared["tieStratum"].as_u64().expect("tie count");
        // The sample must keep its declared composition: edge values, uniform and tie strata.
        assert!(edge_count >= 20, "edge values missing: {edge_count}");
        assert!(cases.len() >= 25_000, "sample shrank to {}", cases.len());
        let mut mismatches = Vec::new();
        for case in cases {
            let bits = u64::from_str_radix(case[0].as_str().expect("bits"), 16).expect("hex bits");
            let expected = case[1].as_str().expect("text");
            let actual = js_number(f64::from_bits(bits));
            if actual != expected {
                mismatches.push(format!("{bits:016x}: node {expected} rust {actual}"));
            }
        }
        assert!(
            mismatches.is_empty(),
            "{} of {} sampled doubles differ from Node:\n{}",
            mismatches.len(),
            cases.len(),
            mismatches
                .iter()
                .take(25)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn decimal_ties_follow_ecmascript() {
        // Equally short candidates: Rust's shortest formatting picks ...3 for the first value;
        // ECMAScript Number::toString picks ...2.
        let tie: f64 = "1000000000000000.25".parse().expect("f64");
        assert_eq!(js_number(tie), "1000000000000000.2");
        let tie: f64 = "771558860699787.2".parse().expect("f64");
        assert_eq!(js_number(tie), "771558860699787.2");
        assert_eq!(js_number(-0.0), "0");
        assert_eq!(js_number(f64::NAN), "null");
        assert_eq!(js_number(f64::INFINITY), "null");
    }
}
