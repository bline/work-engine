use lifecycle_core::{
    BuildId, Command, CommandId, CommandRequest, ContextGeneration, GrantId, ProofRunId, Revision,
    SubjectId,
};
use lifecycle_wire::{
    CommandDto, CommandOperationV1, WireError, WireErrorCode, parse_command, parse_strict_json,
};
use serde_json::{Value, json};
use work_engine_types::CodecContract;

fn input() -> Value {
    json!({
        "protocol_version": 1,
        "principal_ref": "operator:controlled",
        "command_id": "command-1",
        "subject_id": "subject-1",
        "context_generation": "context-a",
        "build_id": "build-1",
        "proof_run_id": "proof-1",
        "grant_ref": "grant-1",
        "expected_revision": "7",
        "kind": "request_replacement",
        "payload": {"reason": "controlled text"},
    })
}

fn with_digest(mut value: Value) -> Value {
    let digest = CodecContract::LifecycleCommandV1
        .digest_json(&value)
        .expect("canonical command")
        .hex();
    value["request_digest"] = Value::String(digest);
    value
}

// Test-only service-boundary conversion. The actual authenticated bridge belongs to S4.
fn to_domain(dto: CommandDto) -> Result<CommandRequest, String> {
    if !matches!(&dto.operation, CommandOperationV1::RequestReplacement(_)) {
        return Err("unsupported command".into());
    }
    let digest = CodecContract::LifecycleCommandV1
        .digest_json(&dto.digest_basis())
        .map_err(|error| error.to_string())?;
    if dto.request_digest != digest.hex() {
        return Err("digest mismatch".into());
    }
    let revision = dto
        .expected_revision
        .parse::<u64>()
        .map_err(|_| "revision is not an exact u64")?;
    CommandRequest::new(
        CommandId::parse(dto.command_id).map_err(|error| error.to_string())?,
        SubjectId::parse(dto.subject_id).map_err(|error| error.to_string())?,
        ContextGeneration::parse(dto.context_generation).map_err(|error| error.to_string())?,
        BuildId::parse(dto.build_id).map_err(|error| error.to_string())?,
        ProofRunId::parse(dto.proof_run_id).map_err(|error| error.to_string())?,
        GrantId::parse(dto.grant_ref).map_err(|error| error.to_string())?,
        Revision::new(revision),
        digest,
        Command::RequestReplacement,
    )
    .map_err(|error| error.to_string())
}

fn decode(value: &Value) -> Result<CommandRequest, String> {
    let bytes = serde_json::to_vec(value).expect("fixture JSON");
    let dto = parse_command(&bytes).map_err(|error| error.to_string())?;
    to_domain(dto)
}

#[test]
fn accepted_command_crosses_only_the_value_boundary() {
    let command = decode(&with_digest(input())).expect("typed command");
    assert_eq!(command.subject.as_str(), "subject-1");
    assert_eq!(command.context.as_str(), "context-a");
    assert_eq!(command.build.as_str(), "build-1");
    assert_eq!(command.proof_run.as_str(), "proof-1");
    assert_eq!(command.expected_revision.get(), 7);
}

#[test]
fn identity_revision_and_grant_mutations_reject_before_entry() {
    let original = with_digest(input());
    for (field, replacement) in [
        ("subject_id", "subject-2"),
        ("context_generation", "context-b"),
        ("build_id", "build-2"),
        ("proof_run_id", "proof-2"),
        ("grant_ref", "grant-2"),
        ("expected_revision", "8"),
        ("kind", "unsupported"),
    ] {
        let mut changed = original.clone();
        changed[field] = Value::String(replacement.into());
        assert!(decode(&changed).is_err(), "accepted changed {field}");
    }
    let mut changed_payload = original.clone();
    changed_payload["payload"]["reason"] = Value::String("other".into());
    assert!(decode(&changed_payload).is_err());
}

#[test]
fn strict_input_rejects_duplicates_and_unsafe_numbers() {
    assert!(parse_strict_json(br#"{"payload":{"x":1}}"#).is_ok());
    assert!(matches!(
        parse_strict_json(br#"{"payload":{"x":1,"x":2}}"#),
        Err(WireError::MalformedJson(_))
    ));
    assert!(matches!(
        parse_strict_json(br#"{"payload":{"x":9007199254740993}}"#),
        Err(WireError::MalformedJson(_))
    ));
    assert!(parse_strict_json(br#"{"payload":{"x":9007199254740991}}"#).is_ok());
    assert!(matches!(
        parse_strict_json(br#"{"payload":{"a":1,"\u0061":2}}"#),
        Err(WireError::MalformedJson(_))
    ));
    assert!(matches!(
        parse_strict_json(br#"{"payload":{"x":"\uD800"}}"#),
        Err(WireError::MalformedJson(_))
    ));
    assert!(parse_strict_json(br#"{"payload":{"x":"\uD83D\uDE00"}}"#).is_ok());
}

#[test]
fn public_error_codes_distinguish_stale_revision_and_conflict() {
    assert_eq!(
        WireError::StaleRevision.code(),
        WireErrorCode::StaleRevision
    );
    assert_eq!(WireError::Conflict.code(), WireErrorCode::Conflict);
    assert_ne!(WireError::StaleRevision.code(), WireError::Conflict.code());
}
