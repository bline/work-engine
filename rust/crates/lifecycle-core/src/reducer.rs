//! Pure admission decisions. Persistence and trusted-principal resolution belong to the store.

use serde_json::Value;
use work_engine_types::CodecContract;

use crate::{
    AttemptId, AuthorityExpiresAt, BuildId, ClockSample, Command, CommandRequest,
    ContextGeneration, ControlledTextInput, DeliveryId, EffectId, EffectInput, EffectObservation,
    EffectPlan, EffectSettlement, ExecutionOutcome, GrantId, InputId, ProofRunId, ProviderThreadId,
    ProviderTurnId, Revision, RuntimeIncarnation, SubjectId, TransitionId, TransitionStage,
    WallTimeMs,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrantScope {
    EnqueueInput,
    RequestReplacement,
    RequestInterruption,
}

impl GrantScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EnqueueInput => "enqueue_input",
            Self::RequestReplacement => "request_replacement",
            Self::RequestInterruption => "request_interruption",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "enqueue_input" => Some(Self::EnqueueInput),
            "request_replacement" => Some(Self::RequestReplacement),
            "request_interruption" => Some(Self::RequestInterruption),
            _ => None,
        }
    }
}

/// A trusted service installs this record; request bytes cannot install a grant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedGrant {
    pub id: GrantId,
    pub issuer: String,
    pub principal: String,
    pub subject: SubjectId,
    pub context: ContextGeneration,
    pub build: BuildId,
    pub proof_run: ProofRunId,
    pub scope: GrantScope,
    pub expires: AuthorityExpiresAt,
    pub revision: Revision,
    pub revoked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionOwner {
    Domain { generation: ContextGeneration },
    Transition { id: TransitionId },
    Fenced,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectState {
    pub subject: SubjectId,
    pub context: ContextGeneration,
    pub build: BuildId,
    pub proof_run: ProofRunId,
    pub revision: Revision,
    pub semantic_revision: Revision,
    pub queue_revision: Revision,
    pub last_wall: WallTimeMs,
    pub owner: AdmissionOwner,
}

/// Caller identity is supplied by trusted composition, separately from the digest basis.
/// The constructor checks that the exact basis and typed request have the same meaning.
#[derive(Clone, Debug)]
pub struct CommandAdmission {
    request: CommandRequest,
    principal: String,
    digest_basis: Value,
    input: Option<ControlledTextInput>,
    producer_ref: Option<String>,
}

impl CommandAdmission {
    pub fn new(
        request: CommandRequest,
        principal: String,
        digest_basis: Value,
        input: Option<ControlledTextInput>,
    ) -> Result<Self, AdmissionError> {
        if principal.is_empty() || principal.len() > 128 {
            return Err(AdmissionError::MalformedBasis);
        }
        request
            .request_digest
            .verify_json(CodecContract::LifecycleCommandV1, &digest_basis)
            .map_err(|_| AdmissionError::DigestMismatch)?;
        let object = digest_basis
            .as_object()
            .ok_or(AdmissionError::MalformedBasis)?;
        const BASIS_KEYS: [&str; 11] = [
            "protocol_version",
            "principal_ref",
            "command_id",
            "subject_id",
            "context_generation",
            "build_id",
            "proof_run_id",
            "grant_ref",
            "expected_revision",
            "kind",
            "payload",
        ];
        if object.len() != BASIS_KEYS.len()
            || BASIS_KEYS.iter().any(|key| !object.contains_key(*key))
        {
            return Err(AdmissionError::MalformedBasis);
        }
        let fields = [
            ("principal_ref", principal.as_str()),
            ("command_id", request.command_id.as_str()),
            ("subject_id", request.subject.as_str()),
            ("context_generation", request.context.as_str()),
            ("build_id", request.build.as_str()),
            ("proof_run_id", request.proof_run.as_str()),
            ("grant_ref", request.grant_reference.as_str()),
        ];
        if digest_basis.get("protocol_version").and_then(Value::as_u64) != Some(1)
            || digest_basis
                .get("expected_revision")
                .and_then(Value::as_str)
                != Some(request.expected_revision.get().to_string().as_str())
            || fields.iter().any(|(key, expected)| {
                digest_basis.get(*key).and_then(Value::as_str) != Some(*expected)
            })
        {
            return Err(AdmissionError::MalformedBasis);
        }
        let payload = digest_basis
            .get("payload")
            .and_then(Value::as_object)
            .ok_or(AdmissionError::MalformedBasis)?;
        let (kind, producer_ref) = match &request.command {
            Command::EnqueueInput { input: input_id } => {
                if payload.len() != 4 {
                    return Err(AdmissionError::MalformedBasis);
                }
                let controlled = input.as_ref().ok_or(AdmissionError::MalformedBasis)?;
                let producer = payload
                    .get("producer_ref")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .ok_or(AdmissionError::MalformedBasis)?;
                if controlled.input_id() != input_id
                    || payload.get("input_id").and_then(Value::as_str) != Some(input_id.as_str())
                    || payload.get("text").and_then(Value::as_str) != Some(controlled.text())
                    || payload.get("text_digest").and_then(Value::as_str)
                        != Some(controlled.digest().hex().as_str())
                {
                    return Err(AdmissionError::MalformedBasis);
                }
                ("enqueue_input", Some(producer.to_owned()))
            }
            Command::RequestReplacement => {
                if input.is_some() || payload.len() > 1 || payload.keys().any(|key| key != "reason")
                {
                    return Err(AdmissionError::MalformedBasis);
                }
                ("request_replacement", None)
            }
            Command::RequestInterruption { attempt } => {
                if input.is_some()
                    || payload.len() != 1
                    || payload.get("attempt_id").and_then(Value::as_str) != Some(attempt.as_str())
                {
                    return Err(AdmissionError::MalformedBasis);
                }
                ("request_interruption", None)
            }
        };
        if digest_basis.get("kind").and_then(Value::as_str) != Some(kind) {
            return Err(AdmissionError::MalformedBasis);
        }
        Ok(Self {
            request,
            principal,
            digest_basis,
            input,
            producer_ref,
        })
    }

    pub fn request(&self) -> &CommandRequest {
        &self.request
    }

    pub fn principal(&self) -> &str {
        &self.principal
    }

    pub fn digest_basis(&self) -> &Value {
        &self.digest_basis
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        // Construction already verified that this encoding succeeds and matches the digest.
        CodecContract::LifecycleCommandV1
            .canonical_json(&self.digest_basis)
            .expect("verified command basis remains encodable")
    }

    pub fn input(&self) -> Option<&ControlledTextInput> {
        self.input.as_ref()
    }

    pub fn producer_ref(&self) -> Option<&str> {
        self.producer_ref.as_deref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionError {
    DigestMismatch,
    MalformedBasis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectionCode {
    UnknownSubject,
    InvalidGrant,
    ExpiredGrant,
    StaleRevision,
    WrongOwner,
    WrongIdentity,
    UnsupportedCapability,
    Conflict,
    ClockRollback,
}

impl RejectionCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownSubject => "unknown_subject",
            Self::InvalidGrant => "invalid_grant",
            Self::ExpiredGrant => "expired_grant",
            Self::StaleRevision => "stale_revision",
            Self::WrongOwner => "wrong_owner",
            Self::WrongIdentity => "wrong_identity",
            Self::UnsupportedCapability => "unsupported_capability",
            Self::Conflict => "conflict",
            Self::ClockRollback => "clock_rollback",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "unknown_subject" => Some(Self::UnknownSubject),
            "invalid_grant" => Some(Self::InvalidGrant),
            "expired_grant" => Some(Self::ExpiredGrant),
            "stale_revision" => Some(Self::StaleRevision),
            "wrong_owner" => Some(Self::WrongOwner),
            "wrong_identity" => Some(Self::WrongIdentity),
            "unsupported_capability" => Some(Self::UnsupportedCapability),
            "conflict" => Some(Self::Conflict),
            "clock_rollback" => Some(Self::ClockRollback),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandOutcome {
    Enqueued {
        input: InputId,
        delivery: DeliveryId,
    },
    ReplacementRequested {
        transition: TransitionId,
    },
    InterruptionRequested {
        attempt: AttemptId,
    },
    Rejected(RejectionCode),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandResult {
    pub outcome: CommandOutcome,
    pub revision: Revision,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reduction {
    pub result: CommandResult,
    pub next_subject: SubjectState,
}

pub fn reduce_command(
    subject: &SubjectState,
    admission: &CommandAdmission,
    grant: Option<&TrustedGrant>,
    clock: ClockSample,
    delivery: DeliveryId,
    transition: TransitionId,
    target_attempt_is_entered: bool,
) -> Reduction {
    let request = admission.request();
    let reject = if subject.subject != request.subject
        || subject.context != request.context
        || subject.build != request.build
        || subject.proof_run != request.proof_run
    {
        Some(RejectionCode::WrongIdentity)
    } else if clock.wall.get() < subject.last_wall.get() {
        Some(RejectionCode::ClockRollback)
    } else if subject.revision != request.expected_revision {
        Some(RejectionCode::StaleRevision)
    } else if !matches!(subject.owner, AdmissionOwner::Domain { .. })
        && !matches!(request.command, Command::RequestInterruption { .. })
    {
        Some(RejectionCode::WrongOwner)
    } else if let Some(grant) = grant {
        if grant.id != request.grant_reference
            || grant.principal != admission.principal()
            || grant.subject != request.subject
            || grant.context != request.context
            || grant.build != request.build
            || grant.proof_run != request.proof_run
            || grant.revoked
        {
            Some(RejectionCode::InvalidGrant)
        } else if !grant.expires.allows_new_entry(clock) {
            Some(RejectionCode::ExpiredGrant)
        } else if grant.scope
            != match request.command {
                Command::EnqueueInput { .. } => GrantScope::EnqueueInput,
                Command::RequestReplacement => GrantScope::RequestReplacement,
                Command::RequestInterruption { .. } => GrantScope::RequestInterruption,
            }
        {
            Some(RejectionCode::InvalidGrant)
        } else {
            None
        }
    } else {
        Some(RejectionCode::InvalidGrant)
    };
    let outcome = match (reject, &request.command) {
        (Some(code), _) => CommandOutcome::Rejected(code),
        (None, Command::EnqueueInput { input }) => CommandOutcome::Enqueued {
            input: input.clone(),
            delivery,
        },
        (None, Command::RequestReplacement) => CommandOutcome::ReplacementRequested { transition },
        (None, Command::RequestInterruption { attempt }) if target_attempt_is_entered => {
            CommandOutcome::InterruptionRequested {
                attempt: attempt.clone(),
            }
        }
        (None, _) => CommandOutcome::Rejected(RejectionCode::Conflict),
    };
    let mut next_subject = subject.clone();
    if matches!(outcome, CommandOutcome::Enqueued { .. }) {
        next_subject.revision = Revision::new(subject.revision.get() + 1);
        next_subject.queue_revision = Revision::new(subject.queue_revision.get() + 1);
    } else if let CommandOutcome::ReplacementRequested { transition } = &outcome {
        next_subject.revision = Revision::new(subject.revision.get() + 1);
        next_subject.semantic_revision = Revision::new(subject.semantic_revision.get() + 1);
        next_subject.owner = AdmissionOwner::Transition {
            id: transition.clone(),
        };
    } else if matches!(outcome, CommandOutcome::InterruptionRequested { .. }) {
        next_subject.revision = Revision::new(subject.revision.get() + 1);
    } else if matches!(
        outcome,
        CommandOutcome::Rejected(RejectionCode::ClockRollback)
    ) {
        next_subject.revision = Revision::new(subject.revision.get() + 1);
        next_subject.owner = AdmissionOwner::Fenced;
    }
    if clock.wall.get() >= subject.last_wall.get() {
        next_subject.last_wall = clock.wall;
    }
    Reduction {
        result: CommandResult {
            outcome,
            revision: next_subject.revision,
        },
        next_subject,
    }
}

/// The oldest unsettled input is the only input eligible for preparation.
/// The store supplies the ordered snapshot; the reducer makes the custody
/// decision and returns the exact effect plan to persist or reuse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueInput {
    pub input: ControlledTextInput,
    pub delivery: DeliveryId,
    pub sequence: u64,
    pub state: CustodyState,
    pub prepared_effect: Option<(EffectId, Revision)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CustodyState {
    Queued,
    Prepared,
    Entered,
    Succeeded,
    FailedBlocked,
    Unknown,
    Retired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueFailure {
    NoInput,
    WrongOwner,
    Blocked,
    StalePrepared,
    InvalidIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueReduction {
    pub plan: EffectPlan,
    pub insert_effect: bool,
}

pub fn reduce_queue(
    subject: &SubjectState,
    queue: &[QueueInput],
) -> Result<QueueReduction, QueueFailure> {
    if !matches!(subject.owner, AdmissionOwner::Domain { .. }) {
        return Err(QueueFailure::WrongOwner);
    }
    let oldest = queue
        .iter()
        .filter(|input| !matches!(input.state, CustodyState::Succeeded | CustodyState::Retired))
        .min_by_key(|input| input.sequence)
        .ok_or(QueueFailure::NoInput)?;
    let expected = subject.semantic_revision;
    let effect = EffectId::parse(format!(
        "effect:{}:{}",
        oldest.delivery.as_str(),
        expected.get()
    ))
    .map_err(|_| QueueFailure::InvalidIdentity)?;
    let insert_effect = match oldest.state {
        CustodyState::Queued if oldest.prepared_effect.is_none() => true,
        CustodyState::Prepared
            if oldest.prepared_effect.as_ref() == Some(&(effect.clone(), expected)) =>
        {
            false
        }
        CustodyState::Prepared => return Err(QueueFailure::StalePrepared),
        _ => return Err(QueueFailure::Blocked),
    };
    Ok(QueueReduction {
        plan: EffectPlan {
            effect,
            subject: subject.subject.clone(),
            input: EffectInput::ControlledText(oldest.input.clone()),
            expected_revision: expected,
        },
        insert_effect,
    })
}

/// Only a resolved replacement boundary may release an unentered prepared
/// plan; an entered attempt remains in custody for exact settlement.
pub fn reduce_transition_custody(stage: TransitionStage, state: CustodyState) -> CustodyState {
    if matches!(
        stage,
        TransitionStage::AbortedBeforeEntry | TransitionStage::Reconciled
    ) && state == CustodyState::Prepared
    {
        CustodyState::Queued
    } else {
        state
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryFailure {
    WrongState,
    WrongOwner,
    StaleRevision,
    EarlierUnresolved,
    InvalidGrant,
    ExpiredGrant,
    ClockRollback,
}

#[derive(Clone, Copy, Debug)]
pub struct EntryFacts<'a> {
    pub effect_state: CustodyState,
    pub input_state: CustodyState,
    pub grant: &'a TrustedGrant,
    pub trusted_issuer: &'a str,
    pub clock: ClockSample,
    pub earlier_unresolved: bool,
}

/// Immutable entry facts are checked before the store inserts one attempt.
pub fn reduce_entry(
    subject: &SubjectState,
    plan: &EffectPlan,
    facts: EntryFacts<'_>,
) -> Result<(), EntryFailure> {
    if facts.effect_state != CustodyState::Prepared || facts.input_state != CustodyState::Prepared {
        return Err(EntryFailure::WrongState);
    }
    if !matches!(subject.owner, AdmissionOwner::Domain { .. }) {
        return Err(EntryFailure::WrongOwner);
    }
    if subject.semantic_revision != plan.expected_revision {
        return Err(EntryFailure::StaleRevision);
    }
    if facts.earlier_unresolved {
        return Err(EntryFailure::EarlierUnresolved);
    }
    if facts.clock.wall.get() < subject.last_wall.get() {
        return Err(EntryFailure::ClockRollback);
    }
    if facts.grant.issuer != facts.trusted_issuer
        || facts.grant.revoked
        || facts.grant.scope != GrantScope::EnqueueInput
        || facts.grant.subject != subject.subject
        || facts.grant.context != subject.context
        || facts.grant.build != subject.build
        || facts.grant.proof_run != subject.proof_run
    {
        return Err(EntryFailure::InvalidGrant);
    }
    if !facts.grant.expires.allows_new_entry(facts.clock) {
        return Err(EntryFailure::ExpiredGrant);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttemptFacts {
    pub incarnation: RuntimeIncarnation,
    pub provider_thread: Option<ProviderThreadId>,
    pub provider_turn: Option<ProviderTurnId>,
    pub outcome: ExecutionOutcome,
    pub settlement: EffectSettlement,
    pub input_state: CustodyState,
    pub attempt_entered: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationReduction {
    pub next_subject: SubjectState,
    pub provider_thread: Option<ProviderThreadId>,
    pub provider_turn: Option<ProviderTurnId>,
    pub outcome: ExecutionOutcome,
    pub settlement: EffectSettlement,
    pub input_state: CustodyState,
    pub conflict: bool,
    pub still_running: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationFailure {
    WrongIncarnation,
    RevisionOverflow,
}

/// Merge only observations for an already entered, exact attempt. The store
/// binds effect/attempt/source identities and applies this change set atomically.
pub fn reduce_observation(
    subject: &SubjectState,
    prior: &AttemptFacts,
    observation: &EffectObservation,
    clock: ClockSample,
) -> Result<ObservationReduction, ObservationFailure> {
    if prior.incarnation != observation.incarnation {
        return Err(ObservationFailure::WrongIncarnation);
    }
    let conflicting_identity = prior
        .provider_thread
        .as_ref()
        .is_some_and(|value| value != &observation.provider_thread)
        || prior
            .provider_turn
            .as_ref()
            .is_some_and(|value| value != &observation.provider_turn);
    let conflicting_outcome = !matches!(
        prior.outcome,
        ExecutionOutcome::Pending | ExecutionOutcome::Unknown
    ) && !matches!(
        observation.outcome,
        ExecutionOutcome::Pending | ExecutionOutcome::Unknown
    ) && prior.outcome != observation.outcome;
    let conflicting_settlement = match (&prior.settlement, &observation.settlement) {
        (EffectSettlement::Established(a), EffectSettlement::Established(b)) => a != b,
        (EffectSettlement::Established(_), EffectSettlement::Conflict(_)) => true,
        (EffectSettlement::Conflict(_), _) => true,
        _ => false,
    };
    let conflict = conflicting_identity
        || conflicting_outcome
        || conflicting_settlement
        || matches!(observation.settlement, EffectSettlement::Conflict(_));
    let outcome = if conflict
        || (matches!(observation.outcome, ExecutionOutcome::Pending)
            && !matches!(prior.outcome, ExecutionOutcome::Pending))
        || (matches!(observation.outcome, ExecutionOutcome::Unknown)
            && !matches!(
                prior.outcome,
                ExecutionOutcome::Pending | ExecutionOutcome::Unknown
            )) {
        prior.outcome.clone()
    } else {
        observation.outcome.clone()
    };
    let settlement = if conflict {
        EffectSettlement::Conflict(observation.source.clone())
    } else if matches!(observation.settlement, EffectSettlement::Unresolved) {
        prior.settlement.clone()
    } else {
        observation.settlement.clone()
    };
    // A contradiction fences admission. It cannot replace the previously
    // established outcome or custody with the incoming, untrusted fact.
    let input_state = if conflict {
        prior.input_state
    } else {
        match (&outcome, &settlement) {
            (ExecutionOutcome::Completed, EffectSettlement::Established(_)) => {
                CustodyState::Succeeded
            }
            (ExecutionOutcome::Failed, _) => CustodyState::FailedBlocked,
            (ExecutionOutcome::Unknown, _) => CustodyState::Unknown,
            _ => CustodyState::Entered,
        }
    };
    let still_running = if conflict {
        prior.attempt_entered
    } else {
        matches!(outcome, ExecutionOutcome::Pending)
            && matches!(settlement, EffectSettlement::Unresolved)
    };
    let mut next_subject = subject.clone();
    next_subject.revision = Revision::new(
        subject
            .revision
            .get()
            .checked_add(1)
            .ok_or(ObservationFailure::RevisionOverflow)?,
    );
    if clock.wall.get() > subject.last_wall.get() {
        next_subject.last_wall = clock.wall;
    }
    if conflict {
        next_subject.owner = AdmissionOwner::Fenced;
    }
    Ok(ObservationReduction {
        next_subject,
        provider_thread: if conflict {
            prior.provider_thread.clone()
        } else {
            Some(observation.provider_thread.clone())
        },
        provider_turn: if conflict {
            prior.provider_turn.clone()
        } else {
            Some(observation.provider_turn.clone())
        },
        outcome,
        settlement,
        input_state,
        conflict,
        still_running,
    })
}
