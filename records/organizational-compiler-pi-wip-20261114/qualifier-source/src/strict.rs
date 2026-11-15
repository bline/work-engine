//! Strict decoding of required JSON evidence. `serde_json::Value` silently keeps the last of
//! duplicate object keys, so a required field could be shadowed without any parse error. Every
//! required JSON or JSONL artifact goes through [`parse`], which rejects duplicate keys at any
//! depth, trailing bytes and invalid UTF-8.
use serde_json::{Map, Value};
use std::collections::BTreeSet;
pub use work_engine_review_delivery::wire::parse;

/// A JSONL artifact written as `lines.join("\n")`: every line (no empty line, no trailing
/// newline) is a strict JSON object with a string `type`. Nothing is skipped.
pub fn parse_lines(bytes: &[u8]) -> Result<Vec<Value>, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("not UTF-8: {e}"))?;
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for (n, line) in text.split('\n').enumerate() {
        let v = parse(line.as_bytes()).map_err(|e| format!("line {}: {e}", n + 1))?;
        if !v.get("type").map(Value::is_string).unwrap_or(false) {
            return Err(format!(
                "line {}: not an object with a string `type`",
                n + 1
            ));
        }
        out.push(v);
    }
    Ok(out)
}

/// Field access that distinguishes absent from wrong-typed; used by every required decoder.
pub struct Obj<'a> {
    pub what: String,
    map: &'a Map<String, Value>,
}

impl<'a> Obj<'a> {
    pub fn new(what: impl Into<String>, v: &'a Value) -> Result<Self, String> {
        let what = what.into();
        v.as_object()
            .map(|map| Obj {
                what: what.clone(),
                map,
            })
            .ok_or_else(|| format!("{what}: not an object"))
    }
    /// The object's keys must be exactly `keys` (no unknown, none missing).
    pub fn exact(&self, keys: &[&str]) -> Result<(), String> {
        let have: BTreeSet<&str> = self.map.keys().map(String::as_str).collect();
        let want: BTreeSet<&str> = keys.iter().copied().collect();
        if have != want {
            let missing: Vec<_> = want.difference(&have).collect();
            let unknown: Vec<_> = have.difference(&want).collect();
            return Err(format!(
                "{}: keys differ (missing {missing:?}, unknown {unknown:?})",
                self.what
            ));
        }
        Ok(())
    }
    pub fn get(&self, key: &str) -> Result<&'a Value, String> {
        self.map
            .get(key)
            .ok_or_else(|| format!("{}.{key}: missing", self.what))
    }
    pub fn opt(&self, key: &str) -> Option<&'a Value> {
        self.map.get(key)
    }
    pub fn str(&self, key: &str) -> Result<&'a str, String> {
        self.get(key)?
            .as_str()
            .ok_or_else(|| format!("{}.{key}: not a string", self.what))
    }
    pub fn u64(&self, key: &str) -> Result<u64, String> {
        self.get(key)?
            .as_u64()
            .ok_or_else(|| format!("{}.{key}: not an unsigned integer", self.what))
    }
    pub fn i64(&self, key: &str) -> Result<i64, String> {
        self.get(key)?
            .as_i64()
            .ok_or_else(|| format!("{}.{key}: not an integer", self.what))
    }
    pub fn bool(&self, key: &str) -> Result<bool, String> {
        self.get(key)?
            .as_bool()
            .ok_or_else(|| format!("{}.{key}: not a boolean", self.what))
    }
    pub fn arr(&self, key: &str) -> Result<&'a Vec<Value>, String> {
        self.get(key)?
            .as_array()
            .ok_or_else(|| format!("{}.{key}: not an array", self.what))
    }
    pub fn obj(&self, key: &str) -> Result<Obj<'a>, String> {
        Obj::new(format!("{}.{key}", self.what), self.get(key)?)
    }
    pub fn strings(&self, key: &str) -> Result<Vec<String>, String> {
        self.arr(key)?
            .iter()
            .map(|x| {
                x.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{}.{key}: non-string element", self.what))
            })
            .collect()
    }
    pub fn null(&self, key: &str) -> Result<(), String> {
        if self.get(key)?.is_null() {
            Ok(())
        } else {
            Err(format!("{}.{key}: not null", self.what))
        }
    }
}

pub fn is_hex64(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Strict `YYYY-MM-DDTHH:MM:SS.ffffffZ` (custody's format) to Unix seconds. Anything else is
/// `None`; callers must refuse rather than skip a dependent check.
pub fn parse_iso(t: &str) -> Option<f64> {
    let b = t.as_bytes();
    if b.len() != 27
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'.'
        || b[26] != b'Z'
    {
        return None;
    }
    let digits = |r: std::ops::Range<usize>| -> Option<i64> {
        let s = t.get(r)?;
        if s.bytes().all(|c| c.is_ascii_digit()) {
            s.parse().ok()
        } else {
            None
        }
    };
    let (y, m, d) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
    let (hh, mm, ss, us) = (
        digits(11..13)?,
        digits(14..16)?,
        digits(17..19)?,
        digits(20..26)?,
    );
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days as f64 * 86400.0 + (hh * 3600 + mm * 60 + ss) as f64 + us as f64 / 1e6)
}
