use serde::{Deserialize, Serialize};
use thiserror::Error;
use work_engine_types::{CodecContract, Digest, IdError, IdValue};

macro_rules! lifecycle_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
        #[serde(transparent)]
        pub struct $name(IdValue);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, IdError> {
                IdValue::parse(value).map(Self)
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

lifecycle_id!(SubjectId);
lifecycle_id!(ContextGeneration);
lifecycle_id!(TransitionId);
lifecycle_id!(InputId);
lifecycle_id!(DeliveryId);
lifecycle_id!(EffectId);
lifecycle_id!(AttemptId);
lifecycle_id!(CheckpointId);
lifecycle_id!(GrantId);
lifecycle_id!(RuntimeIncarnation);
lifecycle_id!(ProviderThreadId);
lifecycle_id!(ProviderTurnId);
lifecycle_id!(BuildId);
lifecycle_id!(StoreId);
lifecycle_id!(ProofRunId);
lifecycle_id!(CommandId);
lifecycle_id!(EvidenceId);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(u64);

impl Revision {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WallTimeMs(i64);

impl WallTimeMs {
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaitBudgetMs(u64);

impl WaitBudgetMs {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockSample {
    pub wall: WallTimeMs,
    pub wait_budget: WaitBudgetMs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorityExpiresAt(WallTimeMs);

impl AuthorityExpiresAt {
    pub const fn new(wall: WallTimeMs) -> Self {
        Self(wall)
    }

    pub const fn allows_new_entry(self, sample: ClockSample) -> bool {
        sample.wall.get() < self.0.get()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    EnqueueInput { input: InputId },
    RequestReplacement,
    RequestInterruption { attempt: AttemptId },
}

/// A validated request value, not a grant or admission decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandRequest {
    pub command_id: CommandId,
    pub subject: SubjectId,
    pub context: ContextGeneration,
    pub build: BuildId,
    pub proof_run: ProofRunId,
    pub grant_reference: GrantId,
    pub expected_revision: Revision,
    pub request_digest: Digest,
    pub command: Command,
}

impl CommandRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_id: CommandId,
        subject: SubjectId,
        context: ContextGeneration,
        build: BuildId,
        proof_run: ProofRunId,
        grant_reference: GrantId,
        expected_revision: Revision,
        request_digest: Digest,
        command: Command,
    ) -> Result<Self, ContractError> {
        if request_digest.contract() != CodecContract::LifecycleCommandV1 {
            return Err(ContractError::WrongDigestContract);
        }
        Ok(Self {
            command_id,
            subject,
            context,
            build,
            proof_run,
            grant_reference,
            expected_revision,
            request_digest,
            command,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectKind {
    ControlledTextTurn,
}

/// Exact UTF-8 bytes and identity admitted for one controlled text effect.
/// The digest is checked against the trusted text-input contract at construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlledTextInput {
    input: InputId,
    text: String,
    digest: Digest,
}

impl ControlledTextInput {
    pub fn new(input: InputId, text: String, digest: Digest) -> Result<Self, ContractError> {
        digest
            .verify_binary(CodecContract::LifecycleTextInputV1, text.as_bytes())
            .map_err(|_| ContractError::InvalidInputDigest)?;
        Ok(Self {
            input,
            text,
            digest,
        })
    }

    pub fn input_id(&self) -> &InputId {
        &self.input
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn digest(&self) -> &Digest {
        &self.digest
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectInput {
    ControlledText(ControlledTextInput),
}

impl EffectInput {
    pub const fn kind(&self) -> EffectKind {
        match self {
            Self::ControlledText(_) => EffectKind::ControlledTextTurn,
        }
    }
}

/// Data planned by the reducer; no authority to dispatch until store claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectPlan {
    pub effect: EffectId,
    pub subject: SubjectId,
    pub input: EffectInput,
    pub expected_revision: Revision,
}

impl EffectPlan {
    pub const fn kind(&self) -> EffectKind {
        self.input.kind()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionOutcome {
    Pending,
    Completed,
    Failed,
    Interrupted,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectSettlement {
    Unresolved,
    Established(EvidenceId),
    Conflict(EvidenceId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectObservation {
    pub effect: EffectId,
    pub attempt: AttemptId,
    pub incarnation: RuntimeIncarnation,
    pub provider_thread: ProviderThreadId,
    pub provider_turn: ProviderTurnId,
    pub outcome: ExecutionOutcome,
    pub settlement: EffectSettlement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractErrorCode {
    WrongDigestContract,
    InvalidInputDigest,
    StaleRevision,
    Conflict,
    UnsupportedCapability,
    InvalidEvidence,
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ContractError {
    #[error("request digest uses the wrong trusted contract")]
    WrongDigestContract,
    #[error("controlled text input does not match its trusted digest")]
    InvalidInputDigest,
    #[error("expected revision is stale")]
    StaleRevision,
    #[error("command conflicts with committed history")]
    Conflict,
    #[error("capability is unsupported")]
    UnsupportedCapability,
    #[error("observation evidence is invalid")]
    InvalidEvidence,
}

impl ContractError {
    pub const fn code(&self) -> ContractErrorCode {
        match self {
            Self::WrongDigestContract => ContractErrorCode::WrongDigestContract,
            Self::InvalidInputDigest => ContractErrorCode::InvalidInputDigest,
            Self::StaleRevision => ContractErrorCode::StaleRevision,
            Self::Conflict => ContractErrorCode::Conflict,
            Self::UnsupportedCapability => ContractErrorCode::UnsupportedCapability,
            Self::InvalidEvidence => ContractErrorCode::InvalidEvidence,
        }
    }
}
