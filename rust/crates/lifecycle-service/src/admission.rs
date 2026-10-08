use lifecycle_core::{
    AttemptId, BuildId, Command, CommandAdmission, CommandId, CommandRequest, ContextGeneration,
    ControlledTextInput, GrantId, GrantScope, InputId, ProofRunId, Revision, SubjectId,
};
use lifecycle_wire::{
    CommandOperationV1, CommandOperationV2, WireError, parse_command, parse_command_v2,
    parse_strict_json,
};
use serde_json::Value;
use work_engine_types::CodecContract;

use crate::ServiceConfig;

#[derive(Debug)]
pub enum AdmitError {
    Wire(WireError),
    Unauthorized,
    Invalid,
}

impl From<WireError> for AdmitError {
    fn from(error: WireError) -> Self {
        Self::Wire(error)
    }
}

pub struct ParsedCommand {
    pub command_id: String,
    pub admission: CommandAdmission,
}

pub fn parse_authenticated(
    config: &ServiceConfig,
    peer_uid: u32,
    bytes: &[u8],
) -> Result<ParsedCommand, AdmitError> {
    if peer_uid != config.principal.uid {
        return Err(AdmitError::Unauthorized);
    }
    let value = parse_strict_json(bytes)?;
    let version = value
        .get("protocol_version")
        .and_then(Value::as_u64)
        .ok_or(AdmitError::Wire(WireError::UnsupportedVersion))?;
    let (
        basis,
        contract,
        principal,
        command_id,
        subject,
        context,
        build,
        proof,
        grant,
        revision,
        command,
        input,
    ) = match version {
        1 => {
            let dto = parse_command(bytes)?;
            let basis = dto.digest_basis();
            let command = match dto.operation {
                CommandOperationV1::EnqueueInput(_) => {
                    return Err(AdmitError::Wire(WireError::UnsupportedCapability));
                }
                CommandOperationV1::RequestReplacement(_) => Command::RequestReplacement,
                CommandOperationV1::RequestInterruption(payload) => Command::RequestInterruption {
                    attempt: AttemptId::parse(payload.attempt_id)
                        .map_err(|_| AdmitError::Invalid)?,
                },
            };
            (
                basis,
                CodecContract::LifecycleCommandV1,
                dto.principal_ref,
                dto.command_id,
                dto.subject_id,
                dto.context_generation,
                dto.build_id,
                dto.proof_run_id,
                dto.grant_ref,
                dto.expected_revision,
                command,
                None,
            )
        }
        2 => {
            let dto = parse_command_v2(bytes)?;
            let (command, input) = match &dto.operation {
                CommandOperationV2::EnqueueInput(payload) => {
                    if payload.producer_ref != config.principal.producer_ref {
                        return Err(AdmitError::Unauthorized);
                    }
                    let input_id = InputId::parse(payload.input_id.clone())
                        .map_err(|_| AdmitError::Invalid)?;
                    let digest = CodecContract::LifecycleTextInputV1
                        .digest_binary(payload.text.as_bytes())
                        .map_err(|_| AdmitError::Invalid)?;
                    let controlled =
                        ControlledTextInput::new(input_id.clone(), payload.text.clone(), digest)
                            .map_err(|_| AdmitError::Invalid)?;
                    (Command::EnqueueInput { input: input_id }, Some(controlled))
                }
                CommandOperationV2::RequestReplacement(_) => (Command::RequestReplacement, None),
                CommandOperationV2::RequestInterruption(payload) => (
                    Command::RequestInterruption {
                        attempt: AttemptId::parse(payload.attempt_id.clone())
                            .map_err(|_| AdmitError::Invalid)?,
                    },
                    None,
                ),
            };
            (
                dto.digest_basis(),
                CodecContract::LifecycleCommandV2,
                dto.principal_ref,
                dto.command_id,
                dto.subject_id,
                dto.context_generation,
                dto.build_id,
                dto.proof_run_id,
                dto.grant_ref,
                dto.expected_revision,
                command,
                input,
            )
        }
        _ => return Err(AdmitError::Wire(WireError::UnsupportedVersion)),
    };
    if principal != config.principal.principal_ref
        || subject != config.subject_id
        || build != config.build_id
        || proof != config.proof_run_id
    {
        return Err(AdmitError::Unauthorized);
    }
    let scope = match command {
        Command::EnqueueInput { .. } => GrantScope::EnqueueInput,
        Command::RequestReplacement => GrantScope::RequestReplacement,
        Command::RequestInterruption { .. } => GrantScope::RequestInterruption,
    };
    if !config.grants.iter().any(|item| {
        item.grant_ref == grant
            && item.context_generation == context
            && item.scope == scope.as_str()
    }) {
        return Err(AdmitError::Unauthorized);
    }
    let digest = contract
        .digest_json(&basis)
        .map_err(|_| AdmitError::Invalid)?;
    let request = CommandRequest::new(
        CommandId::parse(command_id.clone()).map_err(|_| AdmitError::Invalid)?,
        SubjectId::parse(subject).map_err(|_| AdmitError::Invalid)?,
        ContextGeneration::parse(context).map_err(|_| AdmitError::Invalid)?,
        BuildId::parse(build).map_err(|_| AdmitError::Invalid)?,
        ProofRunId::parse(proof).map_err(|_| AdmitError::Invalid)?,
        GrantId::parse(grant).map_err(|_| AdmitError::Invalid)?,
        Revision::new(revision.parse().map_err(|_| AdmitError::Invalid)?),
        digest,
        command,
    )
    .map_err(|_| AdmitError::Invalid)?;
    let admission =
        CommandAdmission::new(request, principal, basis, input).map_err(|_| AdmitError::Invalid)?;
    Ok(ParsedCommand {
        command_id,
        admission,
    })
}
