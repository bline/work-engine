//! Passive validation of existing lifecycle read values. No I/O or authority.
use serde::Deserialize;
use serde_json::Value;
use work_engine_types::IdValue;

use crate::{CommandResultV2, WireError, parse_strict_json};

#[derive(Clone, Copy, Debug)]
pub enum CommandObservationKind {
    Admission,
    Control,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantReadLocator {
    pub protocol_version: u16,
    pub command_id: String,
    pub subject_id: String,
    pub authority_ref: String,
    pub grant_ref: String,
    pub expected_revision: String,
}

fn invalid() -> WireError {
    WireError::SemanticInvalidity
}
fn id(value: &str) -> Result<(), WireError> {
    IdValue::parse(value.to_owned())
        .map(|_| ())
        .map_err(|_| invalid())
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, WireError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(invalid)
}
fn decimal(value: &str) -> Result<u64, WireError> {
    let parsed = value.parse::<u64>().map_err(|_| invalid())?;
    if parsed.to_string() != value {
        return Err(invalid());
    }
    Ok(parsed)
}
fn digest(value: &str) -> Result<(), WireError> {
    if value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(invalid())
    }
}
fn exact_fields(value: &Value, fields: &[&str]) -> Result<(), WireError> {
    let object = value.as_object().ok_or_else(invalid)?;
    if object.len() != fields.len() || fields.iter().any(|k| !object.contains_key(*k)) {
        return Err(invalid());
    }
    Ok(())
}
fn unknown(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == 1 && o.get("status").and_then(Value::as_str) == Some("unknown"))
}

pub fn parse_grant_read_locator(bytes: &[u8]) -> Result<GrantReadLocator, WireError> {
    let value = parse_strict_json(bytes)?;
    let dto: GrantReadLocator = serde_json::from_value(value).map_err(|_| invalid())?;
    if dto.protocol_version != 1 {
        return Err(WireError::UnsupportedVersion);
    }
    for value in [
        &dto.command_id,
        &dto.subject_id,
        &dto.authority_ref,
        &dto.grant_ref,
    ] {
        id(value)?;
    }
    decimal(&dto.expected_revision)?;
    Ok(dto)
}

pub fn validate_grant_result(
    value: &Value,
    command_id: &str,
    grant_ref: &str,
    expected_revision: &str,
) -> Result<(), WireError> {
    id(command_id)?;
    id(grant_ref)?;
    let expected = decimal(expected_revision)?
        .checked_add(1)
        .ok_or_else(invalid)?;
    if unknown(value) {
        return Ok(());
    }
    exact_fields(
        value,
        &[
            "protocol_version",
            "command_id",
            "accepted",
            "grant_ref",
            "resulting_revision",
            "revoked",
        ],
    )?;
    if value["protocol_version"] != 1 {
        return Err(WireError::UnsupportedVersion);
    }
    if text(value, "command_id")? != command_id
        || text(value, "grant_ref")? != grant_ref
        || value["accepted"].as_bool() != Some(true)
        || value["revoked"].as_bool() != Some(true)
        || decimal(text(value, "resulting_revision")?)? != expected
    {
        return Err(invalid());
    }
    Ok(())
}

fn validate_v2_result(value: &Value, command_id: &str, outcome: &str) -> Result<(), WireError> {
    if unknown(value) {
        return Ok(());
    }
    exact_fields(
        value,
        &[
            "protocol_version",
            "command_id",
            "accepted",
            "outcome_kind",
            "outcome_ref",
            "rejection_code",
            "resulting_revision",
        ],
    )?;
    let dto: CommandResultV2 = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
    if dto.protocol_version != 2 {
        return Err(WireError::UnsupportedVersion);
    }
    if dto.command_id != command_id {
        return Err(invalid());
    }
    decimal(&dto.resulting_revision)?;
    if dto.accepted {
        if dto.outcome_kind != outcome || dto.rejection_code.is_some() {
            return Err(invalid());
        }
        id(dto.outcome_ref.as_deref().ok_or_else(invalid)?)?;
    } else if dto.outcome_kind != "rejected"
        || dto.outcome_ref.is_some()
        || dto.rejection_code.as_deref().is_none_or(str::is_empty)
    {
        return Err(invalid());
    }
    Ok(())
}

/// Validate the bindings this read actually exposes, not a task/command edge
/// or a digest preimage unavailable to the consumer.
pub fn validate_command_observation(
    value: &Value,
    scope: &Value,
    kind: CommandObservationKind,
) -> Result<(), WireError> {
    let command_id = text(scope, "command_id")?;
    id(command_id)?;
    id(text(scope, "subject_id")?)?;
    if scope["task_binding"] != "not_established" {
        return Err(invalid());
    }
    match kind {
        CommandObservationKind::Admission => {
            digest(text(scope, "request_digest")?)?;
            validate_v2_result(value, command_id, "enqueued")
        }
        CommandObservationKind::Control => {
            digest(text(scope, "request_sha256")?)?;
            match text(scope, "control_kind")? {
                "replacement" => validate_v2_result(value, command_id, "replacement_requested"),
                "contract_grant" => validate_grant_result(
                    value,
                    command_id,
                    text(scope, "grant_ref")?,
                    text(scope, "expected_revision")?,
                ),
                _ => Err(WireError::UnsupportedObservationVariant),
            }
        }
    }
}

/// The current subject-wide native query has no revision/cursor. String states
/// retain their existing opaque meaning; this validates row shape, not effects.
pub fn validate_native_delivery_status(value: &Value, subject: &str) -> Result<(), WireError> {
    id(subject)?;
    if value["protocol_version"] != 1 {
        return Err(WireError::UnsupportedVersion);
    }
    if value.get("subject_id").and_then(Value::as_str) != Some(subject)
        || !value.get("recovery_only").is_some_and(Value::is_boolean)
    {
        return Err(invalid());
    }
    let rows = value
        .get("segments")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if rows.len() > 64 {
        return Err(invalid());
    }
    for row in rows {
        for key in [
            "delivery_id",
            "input_id",
            "effect_id",
            "queue_attempt_id",
            "invocation_id",
            "native_attempt_id",
            "thread_id",
            "context_generation",
            "delivery_state",
            "segment_state",
            "native_state",
            "history_status",
        ] {
            text(row, key)?;
        }
        for key in [
            "predecessor_invocation_id",
            "transition_id",
            "turn_id",
            "final_sha256",
        ] {
            if !row.get(key).is_some_and(|v| v.is_null() || v.is_string()) {
                return Err(invalid());
            }
        }
        if !row["sequence"].is_u64()
            || !row["segment_no"].is_u64()
            || !row["send_intent"].is_boolean()
        {
            return Err(invalid());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scope() -> Value {
        json!({"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established","request_digest":"a".repeat(64)})
    }
    fn accepted(outcome: &str) -> Value {
        json!({"protocol_version":2,"command_id":"command:test","accepted":true,"outcome_kind":outcome,"outcome_ref":"result:test","rejection_code":null,"resulting_revision":"2"})
    }
    #[test]
    fn command_variants_unknown_rejection_and_malformed_values() {
        let mut scope = scope();
        let known = accepted("enqueued");
        for value in [
            known.clone(),
            json!({"status":"unknown"}),
            json!({"protocol_version":2,"command_id":"command:test","accepted":false,"outcome_kind":"rejected","outcome_ref":null,"rejection_code":"rejected","resulting_revision":"2"}),
        ] {
            assert!(
                validate_command_observation(&value, &scope, CommandObservationKind::Admission)
                    .is_ok()
            );
        }
        for (key, bad) in [
            ("accepted", json!("yes")),
            ("protocol_version", json!(999)),
            ("command_id", json!("other")),
            ("resulting_revision", json!("01")),
            ("outcome_kind", json!("replacement_requested")),
            ("rejection_code", json!("contradiction")),
        ] {
            let mut bad_value = known.clone();
            bad_value[key] = bad;
            assert!(
                validate_command_observation(&bad_value, &scope, CommandObservationKind::Admission)
                    .is_err()
            );
        }
        for key in [
            "accepted",
            "outcome_ref",
            "rejection_code",
            "resulting_revision",
        ] {
            let mut bad = known.clone();
            bad.as_object_mut().unwrap().remove(key);
            assert!(
                validate_command_observation(&bad, &scope, CommandObservationKind::Admission)
                    .is_err()
            );
        }
        assert!(
            validate_command_observation(
                &json!({"status":"unknown","command_id":"other"}),
                &scope,
                CommandObservationKind::Admission
            )
            .is_err()
        );
        scope["request_digest"] = json!("bad");
        assert!(
            validate_command_observation(&known, &scope, CommandObservationKind::Admission)
                .is_err()
        );
        scope["request_sha256"] = json!("b".repeat(64));
        scope["control_kind"] = json!("replacement");
        assert!(
            validate_command_observation(
                &accepted("replacement_requested"),
                &scope,
                CommandObservationKind::Control
            )
            .is_ok()
        );
        scope.as_object_mut().unwrap().remove("control_kind");
        assert!(
            validate_command_observation(&known, &scope, CommandObservationKind::Control).is_err()
        );
    }
    #[test]
    fn grant_binding_is_versioned_and_cannot_invent_rejection() {
        let value = json!({"protocol_version":1,"command_id":"c","accepted":true,"grant_ref":"g","resulting_revision":"2","revoked":true});
        assert!(validate_grant_result(&value, "c", "g", "1").is_ok());
        let scope = json!({"subject_id":"s","command_id":"c","task_binding":"not_established","control_kind":"contract_grant","request_sha256":"a".repeat(64),"grant_ref":"g","expected_revision":"1"});
        assert!(
            validate_command_observation(&value, &scope, CommandObservationKind::Control).is_ok()
        );
        for field in [
            "control_kind",
            "request_sha256",
            "grant_ref",
            "expected_revision",
        ] {
            let mut missing = scope.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                validate_command_observation(&value, &missing, CommandObservationKind::Control)
                    .is_err()
            );
        }
        assert!(validate_grant_result(&json!({"status":"unknown"}), "c", "g", "1").is_ok());
        for (key, bad) in [
            ("protocol_version", json!(2)),
            ("accepted", json!(false)),
            ("grant_ref", json!("other")),
            ("command_id", json!("other")),
            ("resulting_revision", json!("3")),
            ("revoked", json!(false)),
        ] {
            let mut bad_value = value.clone();
            bad_value[key] = bad;
            assert!(validate_grant_result(&bad_value, "c", "g", "1").is_err());
        }
        assert!(validate_grant_result(&value, "c", "g", &u64::MAX.to_string()).is_err());
        assert!(
            validate_grant_result(&json!({"status":"unknown","accepted":true}), "c", "g", "1")
                .is_err()
        );
        let locator = json!({"protocol_version":1,"command_id":"c","subject_id":"s","authority_ref":"a","grant_ref":"g","expected_revision":"1"});
        assert!(parse_grant_read_locator(&serde_json::to_vec(&locator).unwrap()).is_ok());
        let mut bad = locator;
        bad["expected_revision"] = json!("01");
        assert!(parse_grant_read_locator(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    #[test]
    fn native_rows_require_shape_without_inventing_revision() {
        let row = json!({"delivery_id":"d","input_id":"i","effect_id":"e","queue_attempt_id":"q","invocation_id":"v","native_attempt_id":"n","thread_id":"t","context_generation":"g","delivery_state":"active","segment_state":"entered","native_state":"entered_uncertain","history_status":"unavailable","predecessor_invocation_id":null,"transition_id":null,"turn_id":null,"final_sha256":null,"sequence":1,"segment_no":0,"send_intent":true});
        let mut value =
            json!({"protocol_version":1,"subject_id":"s","recovery_only":false,"segments":[]});
        for count in [0, 64] {
            value["segments"] = json!(vec![row.clone(); count]);
            assert!(validate_native_delivery_status(&value, "s").is_ok());
        }
        value["segments"] = json!(vec![row.clone(); 65]);
        assert!(validate_native_delivery_status(&value, "s").is_err());
        for bad in [
            json!({}),
            {
                let mut r = row.clone();
                r["send_intent"] = json!("yes");
                r
            },
            {
                let mut r = row.clone();
                r.as_object_mut().unwrap().remove("turn_id");
                r
            },
        ] {
            value["segments"] = json!([bad]);
            assert!(validate_native_delivery_status(&value, "s").is_err());
        }
        value["segments"] = json!([row]);
        assert!(validate_native_delivery_status(&value, "other").is_err());
    }
}
