//! TO2 composes independently qualified reads. It owns no task or lifecycle state.
use super::*;
use lifecycle_wire::{
    CommandObservationKind, parse_command_v2, parse_grant_read_locator,
    validate_command_observation, validate_native_delivery_status,
};

#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum Component {
    Present {
        scope: Value,
        revision: Value,
        value: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        completeness: Option<Value>,
    },
    Absent {
        scope: Value,
        record_kind: &'static str,
    },
    NotRequested {
        scope: Value,
    },
    NotApplicable {
        scope: Value,
        reason: &'static str,
    },
    Unavailable {
        scope: Value,
        reason: &'static str,
    },
    Refused {
        scope: Value,
        reason: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<usize>,
    },
}

fn present(scope: Value, revision: Value, value: Value) -> Component {
    Component::Present {
        scope,
        revision,
        value,
        completeness: None,
    }
}
fn unavailable(scope: Value, reason: &'static str) -> Component {
    Component::Unavailable { scope, reason }
}
fn task_failure(scope: Value, error: TaskError) -> Component {
    match error {
        TaskError::Invalid | TaskError::Unauthorized | TaskError::Conflict | TaskError::Stale => {
            Component::Refused {
                scope,
                reason: task_code(&error),
                limit: None,
            }
        }
        _ => unavailable(scope, task_code(&error)),
    }
}

fn selected<'a>(request: &'a Value, key: &str) -> Result<Option<&'a str>, HostError> {
    let Some(value) = request.get(key) else {
        return Ok(None);
    };
    let value = value.as_str().ok_or(HostError::Configuration)?;
    work_engine_types::IdValue::parse(value.to_owned()).map_err(|_| HostError::Configuration)?;
    Ok(Some(value))
}

fn error_component(scope: Value, reply: &Value) -> Option<Component> {
    let code = reply.get("code")?.as_str();
    Some(match code {
        Some("semantic_invalidity" | "unsupported_version" | "unsupported_observation_variant") => {
            Component::Refused {
                scope,
                reason: "read_refused",
                limit: None,
            }
        }
        _ => unavailable(scope, "observation_unavailable"),
    })
}

fn native_component(scope: Value, result: Result<Value, HostError>) -> Component {
    let Ok(value) = result else {
        return unavailable(scope, "native_read_unavailable");
    };
    if value["code"] == "observation_unavailable"
        && value["reason"] == "capacity_exceeded"
        && value["limit"] == 64
    {
        return Component::Refused {
            scope,
            reason: "capacity_exceeded",
            limit: Some(64),
        };
    }
    if let Some(error) = error_component(scope.clone(), &value) {
        return error;
    }
    if scope
        .get("subject_id")
        .and_then(Value::as_str)
        .is_none_or(|subject| validate_native_delivery_status(&value, subject).is_err())
    {
        return unavailable(scope, "invalid_native_observation");
    }
    Component::Present {
        scope,
        revision: json!({"kind":"not_provided"}),
        value,
        completeness: Some(json!({"kind":"complete_for_scope","limit":64})),
    }
}

fn lifecycle_component(scope: Value, result: Result<Value, HostError>) -> Component {
    let Ok(value) = result else {
        return unavailable(scope, "lifecycle_read_unavailable");
    };
    if let Some(error) = error_component(scope.clone(), &value) {
        return error;
    }
    let Some(subject) = scope.get("subject_id").and_then(Value::as_str) else {
        return unavailable(scope, "subject_unavailable");
    };
    let valid = serde_json::to_vec(&value)
        .ok()
        .and_then(|bytes| parse_snapshot(&bytes, subject).ok());
    if valid.is_none() {
        return unavailable(scope, "invalid_lifecycle_observation");
    }
    if let (Some(expected), Some(actual)) = (
        scope.get("delivery_id").and_then(Value::as_str),
        value
            .pointer("/delivery/value/delivery_id")
            .and_then(Value::as_str),
    ) && expected != actual
    {
        return unavailable(scope, "delivery_identity_mismatch");
    }
    present(
        scope,
        json!({"kind":"lifecycle","cursor":value["cursor"]}),
        value,
    )
}

fn admission_read_request(
    bytes: &str,
    digest: &str,
    id: &str,
    subject: &str,
    principal: &str,
) -> Result<(), HostError> {
    let dto = parse_command_v2(bytes.as_bytes()).map_err(|_| HostError::Configuration)?;
    if dto.command_id != id
        || dto.subject_id != subject
        || dto.principal_ref != principal
        || dto.request_digest != digest
        || !matches!(dto.operation, CommandOperationV2::EnqueueInput(_))
    {
        return Err(HostError::Configuration);
    }
    Ok(())
}

fn control_read_request(
    saved: &ControlCommand,
    id: &str,
    subject: &str,
    principal: &str,
    scope: &mut Value,
) -> Result<Value, HostError> {
    if saved.command_id != id
        || saved.request_sha256.len() != 64
        || !saved.request_sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(HostError::Configuration);
    }
    match saved.kind.as_str() {
        "replacement" => {
            let dto = parse_command_v2(saved.service_command_json.as_bytes())
                .map_err(|_| HostError::Configuration)?;
            if dto.command_id != id
                || dto.subject_id != subject
                || dto.principal_ref != principal
                || !matches!(dto.operation, CommandOperationV2::RequestReplacement(_))
            {
                return Err(HostError::Configuration);
            }
            Ok(
                json!({"op":"command_result","subject_id":subject,"command_id":id,"request_digest":dto.request_digest}),
            )
        }
        "contract_grant" => {
            let dto = parse_grant_read_locator(saved.service_command_json.as_bytes())
                .map_err(|_| HostError::Configuration)?;
            if dto.command_id != id || dto.subject_id != subject || dto.authority_ref != principal {
                return Err(HostError::Configuration);
            }
            scope["grant_ref"] = json!(dto.grant_ref);
            scope["expected_revision"] = json!(dto.expected_revision);
            let command = lifecycle_wire::parse_strict_json(saved.service_command_json.as_bytes())
                .map_err(|_| HostError::Configuration)?;
            Ok(json!({"op":"grant_result","command":command}))
        }
        _ => Err(HostError::Configuration),
    }
}

fn command_component(host: &Host, command_id: Option<&str>, control: bool) -> Component {
    let mut scope = json!({"subject_id":host.startup.lifecycle.subject_id,"command_id":command_id,"task_binding":"not_established"});
    let Some(id) = command_id else {
        return Component::NotRequested { scope };
    };
    let result = if control {
        let saved = match host.store.lookup_control_command(id) {
            Ok(Some(saved)) => saved,
            Ok(None) => {
                return Component::Absent {
                    scope,
                    record_kind: "control_command_locator",
                };
            }
            Err(error) => return task_failure(scope, error),
        };
        scope["control_kind"] = json!(saved.kind);
        scope["request_sha256"] = json!(saved.request_sha256);
        let request = match control_read_request(
            &saved,
            id,
            &host.startup.lifecycle.subject_id,
            &host.startup.lifecycle.principal.principal_ref,
            &mut scope,
        ) {
            Ok(request) => request,
            Err(_) => return unavailable(scope, "invalid_control_locator"),
        };
        exchange(&host.startup.lifecycle.socket_path, &request)
    } else {
        let saved = match host.store.lookup_admission_command(id) {
            Ok(Some(saved)) => saved,
            Ok(None) => {
                return Component::Absent {
                    scope,
                    record_kind: "admission_command_locator",
                };
            }
            Err(error) => return task_failure(scope, error),
        };
        if admission_read_request(
            &saved.command_json,
            &saved.request_digest,
            id,
            &host.startup.lifecycle.subject_id,
            &host.startup.lifecycle.principal.principal_ref,
        )
        .is_err()
        {
            return unavailable(scope, "invalid_admission_locator");
        }
        scope["request_digest"] = json!(saved.request_digest);
        exchange(
            &host.startup.lifecycle.socket_path,
            &json!({"op":"command_result","subject_id":host.startup.lifecycle.subject_id,"command_id":id,"request_digest":saved.request_digest}),
        )
    };
    command_reply_component(scope, id, control, result)
}

fn command_reply_component(
    scope: Value,
    id: &str,
    control: bool,
    result: Result<Value, HostError>,
) -> Component {
    let Ok(value) = result else {
        return unavailable(scope, "command_read_unavailable");
    };
    if let Some(error) = error_component(scope.clone(), &value) {
        return error;
    }
    let kind = if control {
        CommandObservationKind::Control
    } else {
        CommandObservationKind::Admission
    };
    if scope.get("command_id").and_then(Value::as_str) != Some(id)
        || validate_command_observation(&value, &scope, kind).is_err()
    {
        return unavailable(scope, "invalid_command_observation");
    }
    present(scope, json!({"kind":"not_provided"}), value)
}

pub(super) fn observe(host: &Host, request: &Value) -> Result<Value, HostError> {
    let object = request.as_object().ok_or(HostError::Configuration)?;
    if !object
        .keys()
        .all(|k| ["op", "task_id", "delivery_id", "command_id"].contains(&k.as_str()))
    {
        return Err(HostError::Configuration);
    }
    let task_id = selected(request, "task_id")?.ok_or(HostError::Configuration)?;
    if task_id.len() > 128
        || !task_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
    {
        return Err(HostError::Configuration);
    }
    let delivery = selected(request, "delivery_id")?;
    let command = selected(request, "command_id")?;
    let owner_scope = json!({"task_id":task_id});
    let (owner, subject, missing) = match host.store.observe(task_id) {
        Ok(Some(task)) if task.task_id == task_id => {
            let subject = task.subject_id.clone();
            let scope = json!({"task_id":task_id,"subject_id":subject});
            let revision = json!({"kind":"task","owner_revision":task.revision,"cursor_revision":task.cursor_revision});
            (
                present(scope, revision, serde_json::to_value(task)?),
                Some(subject),
                false,
            )
        }
        Ok(Some(_)) => (
            unavailable(owner_scope, "task_identity_mismatch"),
            None,
            false,
        ),
        Ok(None) => (
            Component::Absent {
                scope: owner_scope,
                record_kind: "task",
            },
            None,
            true,
        ),
        Err(error) => (task_failure(owner_scope, error), None, false),
    };
    let lifecycle_scope = json!({"subject_id":subject,"delivery_id":delivery});
    let native_scope =
        json!({"subject_id":subject,"collection":"native_delivery_segments","filter":"subject"});
    let (lifecycle, native) = if let Some(subject) = subject.as_deref() {
        let lifecycle = lifecycle_component(
            lifecycle_scope,
            exchange(
                &host.startup.lifecycle.socket_path,
                &json!({"op":"snapshot","subject_id":subject,"delivery_id":delivery}),
            ),
        );
        let native = native_component(
            native_scope,
            read_native_delivery_status(&host.startup.lifecycle.socket_path, subject),
        );
        (lifecycle, native)
    } else if missing {
        (
            Component::NotApplicable {
                scope: lifecycle_scope,
                reason: "task_absent",
            },
            Component::NotApplicable {
                scope: native_scope,
                reason: "task_absent",
            },
        )
    } else {
        (
            unavailable(lifecycle_scope, "task_dependency_unavailable"),
            unavailable(native_scope, "task_dependency_unavailable"),
        )
    };
    let admission = command_component(host, command, false);
    let control = command_component(host, command, true);
    let mut requested = json!({"task_id":task_id});
    if let Some(value) = delivery {
        requested["delivery_id"] = json!(value);
    }
    if let Some(value) = command {
        requested["command_id"] = json!(value);
    }
    Ok(
        json!({"protocol_version":2,"status":"observation","consistency":"independent_components","request":requested,"host_identity":host.identity,"components":{"owner":owner,"lifecycle":lifecycle,"native_delivery_status":native,"command_result":admission,"control_result":control}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_outcomes_do_not_collapse_empty_capacity_and_failure() {
        let scope = json!({"subject_id":"subject:test","collection":"native_delivery_segments","filter":"subject"});
        let value=serde_json::to_value(native_component(scope.clone(),Ok(json!({"protocol_version":1,"subject_id":"subject:test","segments":[],"recovery_only":false})))).unwrap();
        assert_eq!(value["state"], "present");
        assert_eq!(value["value"]["segments"], json!([]));
        assert_eq!(value["revision"]["kind"], "not_provided");
        let capacity = serde_json::to_value(native_component(
            scope.clone(),
            Ok(json!({"code":"observation_unavailable","reason":"capacity_exceeded","limit":64})),
        ))
        .unwrap();
        assert_eq!(capacity["state"], "refused");
        assert!(capacity.get("value").is_none());
        let failure = serde_json::to_value(native_component(
            scope,
            Ok(json!({"code":"observation_unavailable"})),
        ))
        .unwrap();
        assert_eq!(failure["state"], "unavailable");
    }
    #[test]
    fn optional_targets_are_not_null_or_arbitrary_values() {
        assert!(selected(&json!({"delivery_id":null}), "delivery_id").is_err());
        assert!(selected(&json!({"command_id":4}), "command_id").is_err());
        assert!(selected(&json!({}), "command_id").unwrap().is_none());
    }
    #[test]
    fn command_domain_unknown_and_rejected_are_present_but_read_failure_is_not() {
        let scope = json!({"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established","request_digest":"a".repeat(64)});
        for reply in [
            json!({"status":"unknown"}),
            json!({"protocol_version":2,"command_id":"command:test","accepted":false,"outcome_kind":"rejected","outcome_ref":null,"rejection_code":"rejected","resulting_revision":"1"}),
        ] {
            let component = serde_json::to_value(command_reply_component(
                scope.clone(),
                "command:test",
                false,
                Ok(reply.clone()),
            ))
            .unwrap();
            assert_eq!(component["state"], "present");
            assert_eq!(component["value"], reply);
        }
        let failed = serde_json::to_value(command_reply_component(
            scope,
            "command:test",
            false,
            Ok(json!({"code":"observation_unavailable"})),
        ))
        .unwrap();
        assert_eq!(failed["state"], "unavailable");
        assert!(failed.get("value").is_none());
    }

    #[test]
    fn malformed_command_values_cannot_be_present() {
        let scope = json!({"subject_id":"subject:test","command_id":"command:test","task_binding":"not_established","control_kind":"replacement","request_sha256":"a".repeat(64)});
        for reply in [
            json!({"protocol_version":999,"command_id":"command:test","accepted":true}),
            json!({"status":"unknown","command_id":"other"}),
            json!({"protocol_version":2,"command_id":"command:test","accepted":"yes"}),
        ] {
            let value = serde_json::to_value(command_reply_component(
                scope.clone(),
                "command:test",
                true,
                Ok(reply),
            ))
            .unwrap();
            assert_eq!(value["state"], "unavailable");
        }
    }
    #[test]
    fn bad_saved_locators_fail_before_a_read_request_exists() {
        let mut scope = json!({});
        let dto = json!({"protocol_version":1,"command_id":"c","subject_id":"s","authority_ref":"p","grant_ref":"g","expected_revision":"1"});
        let mut saved = ControlCommand {
            command_id: "c".into(),
            kind: "contract_grant".into(),
            request_sha256: "a".repeat(64),
            service_command_json: dto.to_string(),
        };
        assert!(control_read_request(&saved, "c", "s", "p", &mut scope).is_ok());
        assert_eq!(scope["grant_ref"], "g");
        assert!(control_read_request(&saved, "wrong", "s", "p", &mut scope).is_err());
        assert!(control_read_request(&saved, "c", "other", "p", &mut scope).is_err());
        assert!(control_read_request(&saved, "c", "s", "other", &mut scope).is_err());
        saved.request_sha256 = "bad".into();
        assert!(control_read_request(&saved, "c", "s", "p", &mut scope).is_err());
        assert!(admission_read_request("{}", &"a".repeat(64), "c", "s", "p").is_err());
        saved.kind = "replacement".into();
        saved.request_sha256 = "a".repeat(64);
        assert!(control_read_request(&saved, "c", "s", "p", &mut scope).is_err());
    }

    #[test]
    fn saved_v2_locator_digest_identity_and_operation_are_checked() {
        let text = "test";
        let mut command = CommandDtoV2 {
            protocol_version: 2,
            principal_ref: "p".into(),
            command_id: "c".into(),
            subject_id: "s".into(),
            context_generation: "g".into(),
            build_id: "b".into(),
            proof_run_id: "r".into(),
            grant_ref: "grant".into(),
            expected_revision: "0".into(),
            request_digest: String::new(),
            operation: CommandOperationV2::EnqueueInput(EnqueueInputPayloadV2 {
                input_id: "input".into(),
                producer_ref: "producer".into(),
                text: text.into(),
                text_digest: CodecContract::LifecycleTextInputV1
                    .digest_binary(text.as_bytes())
                    .unwrap()
                    .hex(),
            }),
        };
        command.request_digest = CodecContract::LifecycleCommandV2
            .digest_json(&command.digest_basis())
            .unwrap()
            .hex();
        let bytes = serde_json::to_string(&command).unwrap();
        assert!(admission_read_request(&bytes, &command.request_digest, "c", "s", "p").is_ok());
        for (digest, id, subject, principal) in [
            ("wrong", "c", "s", "p"),
            (command.request_digest.as_str(), "other", "s", "p"),
            (command.request_digest.as_str(), "c", "other", "p"),
            (command.request_digest.as_str(), "c", "s", "other"),
        ] {
            assert!(admission_read_request(&bytes, digest, id, subject, principal).is_err());
        }
        let mut saved = ControlCommand {
            command_id: "c".into(),
            kind: "replacement".into(),
            request_sha256: "a".repeat(64),
            service_command_json: bytes,
        };
        assert!(control_read_request(&saved, "c", "s", "p", &mut json!({})).is_err());
        command.operation = CommandOperationV2::RequestReplacement(RequestReplacementPayloadV1 {
            reason: "test".into(),
        });
        command.request_digest = CodecContract::LifecycleCommandV2
            .digest_json(&command.digest_basis())
            .unwrap()
            .hex();
        saved.service_command_json = serde_json::to_string(&command).unwrap();
        assert!(control_read_request(&saved, "c", "s", "p", &mut json!({})).is_ok());
        assert!(
            admission_read_request(
                &saved.service_command_json,
                &command.request_digest,
                "c",
                "s",
                "p"
            )
            .is_err()
        );
        command.request_digest = "b".repeat(64);
        saved.service_command_json = serde_json::to_string(&command).unwrap();
        assert!(control_read_request(&saved, "c", "s", "p", &mut json!({})).is_err());
    }
}
