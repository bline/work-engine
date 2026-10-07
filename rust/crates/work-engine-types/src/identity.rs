use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// A bounded opaque identifier value. Domain packages wrap it in distinct types.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdValue(String);

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum IdError {
    #[error("identifier is empty or exceeds 128 bytes")]
    Length,
    #[error("identifier contains unsupported characters")]
    Characters,
}

impl IdValue {
    pub fn parse(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        if value.is_empty() || value.len() > 128 {
            return Err(IdError::Length);
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
        {
            return Err(IdError::Characters);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IdValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Serialize for IdValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for IdValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(text).map_err(serde::de::Error::custom)
    }
}
