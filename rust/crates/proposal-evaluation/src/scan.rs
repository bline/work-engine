//! A pre-parse scan of every JSON input: duplicate object keys at any depth, and numbers that
//! no 64-bit type or finite float can hold.
//!
//! `serde_json::Value` and the typed model keep the last of two equal keys without a word, so
//! a document could say one thing to a human reader and another to the checker. The scan walks
//! the document once and lists each repeated key with its dotted path, so the caller can
//! refuse the document before trusting any field of it.
//!
//! `serde_json` reports a number such as `1e400` or a 400-digit integer as a syntax failure, and
//! a syntax failure is exit 2. The document is well formed JSON, so it is a `schema` problem
//! at the number's own path instead (contract S3, E1-AC5); the scan records where it was.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;

/// One lexical problem found by [`scan`]: where it is (a dotted path, `""` for the document
/// root) and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub at: String,
    pub message: String,
}

#[derive(Default)]
struct State {
    findings: RefCell<Vec<Finding>>,
    /// The path of the value being read, so a failure inside a scalar can be placed.
    current: RefCell<String>,
}

struct Node<'a> {
    state: &'a State,
    path: String,
}

fn join(path: &str, segment: &str) -> String {
    if path.is_empty() {
        segment.to_string()
    } else {
        format!("{path}.{segment}")
    }
}

impl<'de> DeserializeSeed<'de> for Node<'_> {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        self.state.current.borrow_mut().clone_from(&self.path);
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Node<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any JSON value")
    }
    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        let mut index = 0usize;
        while seq
            .next_element_seed(Node {
                state: self.state,
                path: join(&self.path, &index.to_string()),
            })?
            .is_some()
        {
            index += 1;
        }
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut seen = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            let path = join(&self.path, &key);
            if !seen.insert(key.clone()) {
                self.state.findings.borrow_mut().push(Finding {
                    at: path.clone(),
                    message: format!("duplicate object key {key:?}"),
                });
            }
            map.next_value_seed(Node {
                state: self.state,
                path,
            })?;
        }
        Ok(())
    }
}

/// `serde_json`'s message prefix for a number that no `f64` or integer holds (pinned with the
/// crate version; a test fails if it changes).
const NUMBER_OUT_OF_RANGE: &str = "number out of range";

/// Walk `bytes` as one JSON document. `Ok` lists every problem found (empty when clean);
/// `Err` is a parse failure of the document itself, exactly as `serde_json::from_slice`
/// reports it. A number out of range is a finding, not a parse failure; the walk stops there,
/// so problems after it are not listed.
pub fn scan(bytes: &[u8]) -> Result<Vec<Finding>, serde_json::Error> {
    let state = State::default();
    let mut de = serde_json::Deserializer::from_slice(bytes);
    let walked = DeserializeSeed::deserialize(
        Node {
            state: &state,
            path: String::new(),
        },
        &mut de,
    );
    match walked {
        Ok(()) => de.end()?,
        Err(e) if e.to_string().starts_with(NUMBER_OUT_OF_RANGE) => {
            let at = state.current.borrow().clone();
            state.findings.borrow_mut().push(Finding {
                at,
                message: "number out of range: too large for a 64-bit integer or a finite float"
                    .to_string(),
            });
        }
        Err(e) => return Err(e),
    }
    Ok(state.findings.into_inner())
}

/// Where a `serde_json` data error (a type mismatch, unknown or missing field) is, as a dotted
/// path in `bytes`, which must be a document that parsed as JSON. `serde_json` reports a line
/// and a column and no path, and this crate takes no dependency that does.
///
/// The error position is the last byte the deserializer read, so the deepest key or value
/// containing it names the location. Two cases are placed by rule instead. A missing field is
/// placed at the field, under the object that lacks it. An error inside an internally tagged
/// enum is raised after the element has been read, with the position already past it (in the
/// gap before the next element or at the closing bracket), so a position in such a gap names the
/// element just before it, and only to that depth: the message holds the rest.
///
/// Never empty: the document root is `$`.
pub fn locate_data_error(bytes: &[u8], error: &serde_json::Error) -> String {
    let message = error.to_string();
    let missing = message
        .strip_prefix("missing field `")
        .and_then(|rest| rest.split_once('`'))
        .map(|(name, _)| name.to_string());
    let mut found = None;
    if let Some(offset) = offset_of(bytes, error.line(), error.column()) {
        let mut at = Locator {
            bytes,
            offset,
            missing: missing.is_some(),
            best: &mut found,
        };
        let mut i = at.skip_ws(0);
        at.value(&mut i, "");
    }
    let mut best = found.unwrap_or_default();
    if let Some(name) = missing {
        best = join(&best, &name);
    }
    if best.is_empty() {
        "$".to_string()
    } else {
        best
    }
}

fn offset_of(bytes: &[u8], line: usize, column: usize) -> Option<usize> {
    if line == 0 || column == 0 {
        return None;
    }
    let mut start = 0usize;
    for _ in 1..line {
        start += bytes[start..].iter().position(|b| *b == b'\n')? + 1;
    }
    let offset = start + column - 1;
    (offset < bytes.len()).then_some(offset)
}

struct Locator<'a> {
    bytes: &'a [u8],
    offset: usize,
    /// The error is a missing field: a position on the closing brace is the object itself.
    missing: bool,
    /// The deepest key or value containing `offset`: descendants are noted first.
    best: &'a mut Option<String>,
}

impl Locator<'_> {
    fn skip_ws(&self, mut i: usize) -> usize {
        while self.bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        i
    }

    /// Advance `i` past the value that starts there, noting `path` when the position is inside it.
    fn value(&mut self, i: &mut usize, path: &str) {
        let start = *i;
        match self.bytes.get(start) {
            Some(b'{') => {
                *i += 1;
                let mut last: Option<(String, usize)> = None;
                loop {
                    *i = self.skip_ws(*i);
                    match self.bytes.get(*i) {
                        Some(b'"') => {
                            self.gap(&last, *i);
                            let key_start = *i;
                            let key = self.string(i);
                            let member = join(path, &key);
                            self.note(key_start, *i, &member);
                            *i = self.skip_ws(*i);
                            *i += 1; // ':'
                            *i = self.skip_ws(*i);
                            self.value(i, &member);
                            last = Some((member, *i));
                        }
                        Some(b',') => *i += 1,
                        _ => break,
                    }
                }
                if !self.missing {
                    self.gap(&last, *i + 1);
                }
                *i += 1; // '}'
                self.note(start, *i, path);
            }
            Some(b'[') => {
                *i += 1;
                let mut last: Option<(String, usize)> = None;
                let mut index = 0usize;
                loop {
                    *i = self.skip_ws(*i);
                    match self.bytes.get(*i) {
                        Some(b']') | None => break,
                        Some(b',') => *i += 1,
                        _ => {
                            self.gap(&last, *i);
                            let element = join(path, &index.to_string());
                            self.value(i, &element);
                            last = Some((element, *i));
                            index += 1;
                        }
                    }
                }
                self.gap(&last, *i + 1);
                *i += 1; // ']'
                self.note(start, *i, path);
            }
            Some(b'"') => {
                self.string(i);
                self.note(start, *i, path);
            }
            _ => {
                while self
                    .bytes
                    .get(*i)
                    .is_some_and(|b| !matches!(b, b',' | b'}' | b']') && !b.is_ascii_whitespace())
                {
                    *i += 1;
                }
                self.note(start, *i, path);
            }
        }
    }

    fn note(&mut self, start: usize, end: usize, path: &str) {
        if self.best.is_none() && (start..end).contains(&self.offset) {
            *self.best = Some(path.to_string());
        }
    }

    /// The position lies after the previous child and before `next` (the next child's start, or
    /// one past the closing bracket): it names the previous child.
    fn gap(&mut self, last: &Option<(String, usize)>, next: usize) {
        if let Some((path, end)) = last {
            self.note(*end, next, path);
        }
    }

    /// Decode the string at `i` and leave `i` after its closing quote.
    fn string(&self, i: &mut usize) -> String {
        let start = *i;
        *i += 1;
        while let Some(b) = self.bytes.get(*i) {
            *i += 1;
            match b {
                b'\\' => *i += 1,
                b'"' => break,
                _ => {}
            }
        }
        serde_json::from_slice::<String>(&self.bytes[start..*i]).unwrap_or_default()
    }
}
