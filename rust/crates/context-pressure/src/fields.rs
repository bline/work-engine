//! Field validators matching the JS helpers of the same names, message for message.

use crate::error::{ErrorKind, PressureError};
use crate::js;
use serde_json::{Map, Value};

pub(crate) fn record<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> Result<&'a Map<String, Value>, PressureError> {
    match value {
        Some(Value::Object(map)) => Ok(map),
        _ => Err(PressureError::new(
            ErrorKind::NotObject,
            format!("{label} must be an object"),
        )),
    }
}

pub(crate) fn reject_unknown(
    map: &Map<String, Value>,
    allowed: &[&str],
    label: &str,
) -> Result<(), PressureError> {
    let mut unknown: Vec<&String> = map
        .keys()
        .filter(|key| !allowed.contains(&key.as_str()))
        .collect();
    if unknown.is_empty() {
        return Ok(());
    }
    unknown.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    let names: Vec<&str> = unknown.iter().map(|key| key.as_str()).collect();
    Err(PressureError::new(
        ErrorKind::UnsupportedFields,
        format!("{label} contains unsupported fields: {}", names.join(", ")),
    ))
}

pub(crate) fn text<'a>(value: Option<&'a Value>, label: &str) -> Result<&'a str, PressureError> {
    match value {
        Some(Value::String(text)) if !js::trims_to_empty(text) => Ok(text),
        _ => Err(PressureError::new(
            ErrorKind::NotNonEmptyString,
            format!("{label} must be a non-empty string"),
        )),
    }
}

pub(crate) fn basis_points(value: Option<&Value>, label: &str) -> Result<u16, PressureError> {
    match value.and_then(js::safe_integer) {
        Some(points @ 0..=10_000) => Ok(points as u16),
        _ => Err(PressureError::new(
            ErrorKind::NotBasisPoints,
            format!("{label} must be an integer from 0 through 10000 basis points"),
        )),
    }
}

pub(crate) fn positive_integer(value: Option<&Value>, label: &str) -> Result<u64, PressureError> {
    match value.and_then(js::safe_integer) {
        Some(number) if number >= 1 => Ok(number as u64),
        _ => Err(PressureError::new(
            ErrorKind::NotPositiveInteger,
            format!("{label} must be a positive safe integer"),
        )),
    }
}

pub(crate) fn sha_revision<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> Result<&'a str, PressureError> {
    let revision = text(value, label)?;
    let hex = revision.strip_prefix("sha256:").unwrap_or("");
    if hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        Ok(revision)
    } else {
        Err(PressureError::new(
            ErrorKind::NotShaRevision,
            format!("{label} must be SHA-256 bound"),
        ))
    }
}

pub(crate) fn timestamp<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> Result<&'a str, PressureError> {
    let stamp = text(value, label)?;
    if crate::timestamp::is_es_iso_timestamp(stamp) {
        Ok(stamp)
    } else {
        Err(PressureError::new(
            ErrorKind::NotIsoTimestamp,
            format!("{label} must be an ISO timestamp"),
        ))
    }
}

/// `value === expected` for a JSON number literal.
pub(crate) fn is_number(value: Option<&Value>, expected: f64) -> bool {
    value.and_then(Value::as_f64) == Some(expected)
}

pub(crate) fn is_string(value: Option<&Value>, expected: &str) -> bool {
    value.and_then(Value::as_str) == Some(expected)
}
