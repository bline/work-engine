//! A pre-parse scan of every JSON input: duplicate object keys at any depth.
//!
//! `serde_json::Value` and the typed model keep the last of two equal keys without a word, so
//! a document could say one thing to a human reader and another to the checker. The scan walks
//! the document once and lists each repeated key with its dotted path, so the caller can
//! refuse the document before trusting any field of it.

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

/// Walk `bytes` as one JSON document. `Ok` lists every problem found (empty when clean); `Err`
/// is a parse failure of the document itself, exactly as `serde_json::from_slice` reports it.
pub fn scan(bytes: &[u8]) -> Result<Vec<Finding>, serde_json::Error> {
    let state = State::default();
    let mut de = serde_json::Deserializer::from_slice(bytes);
    DeserializeSeed::deserialize(
        Node {
            state: &state,
            path: String::new(),
        },
        &mut de,
    )?;
    de.end()?;
    Ok(state.findings.into_inner())
}
