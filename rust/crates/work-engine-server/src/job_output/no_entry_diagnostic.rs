//! The no-entry collector record shared by the binary producer and the typed consumer.

use serde_json::{Map, Value};

pub(super) fn valid(record: &Map<String, Value>, schema: &str) -> bool {
    record.get("schema").and_then(Value::as_str) == Some(schema)
        && record.get("action").and_then(Value::as_str) == Some("run")
        && record
            .get("spawn_error")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
        && record.get("process_cleanup").and_then(Value::as_str) == Some("not_established")
        && match (
            record.get("started_unix_seconds"),
            record.get("ended_unix_seconds"),
        ) {
            (None, None) => record.len() == 4,
            (Some(start), Some(end)) => record.len() == 6 && start.is_u64() && end.is_u64(),
            _ => false,
        }
}
