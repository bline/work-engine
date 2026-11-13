//! Identifier syntax (contract section 17.3).
//!
//! IDs are ASCII, case-sensitive, match `^[a-z0-9][a-z0-9._:-]{0,127}$`, are assigned by the
//! semantic authority and are part of meaning: renaming an ID changes identity.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::IdError;

/// The longest identifier, in bytes (equal to characters, since IDs are ASCII).
pub const MAX_ID_LEN: usize = 128;

/// A validated identifier. Ordering is by byte, which equals ordering by UTF-16 code unit for
/// ASCII, so it is the set key order of contract section 17.2.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(String);

impl Id {
    /// Validates `text` against the identifier syntax.
    pub fn parse(text: &str) -> Result<Id, IdError> {
        validate_id(text)?;
        Ok(Id(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Checks the identifier syntax without allocating.
pub fn validate_id(text: &str) -> Result<(), IdError> {
    let mut chars = text.char_indices();
    let Some((_, first)) = chars.next() else {
        return Err(IdError::Empty);
    };
    if text.len() > MAX_ID_LEN {
        return Err(IdError::TooLong { len: text.len() });
    }
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return Err(IdError::InvalidStart { found: first });
    }
    for (index, found) in chars {
        let allowed = found.is_ascii_lowercase()
            || found.is_ascii_digit()
            || matches!(found, '.' | '_' | ':' | '-');
        if !allowed {
            return Err(IdError::InvalidCharacter { index, found });
        }
    }
    Ok(())
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Id {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Id, D::Error> {
        let text = String::deserialize(deserializer)?;
        Id::parse(&text).map_err(serde::de::Error::custom)
    }
}
