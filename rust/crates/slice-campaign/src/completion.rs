//! Campaign-owned execution history. These records describe intent and owner
//! effects; they never attest to an episode result or a claim establishment.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::contract::{CampaignIdentity, CampaignRevision, NativeReviewRequestRef};
use crate::recovery::{OperationReceipt, RecoveryLocator};
use crate::{CampaignError, Result, require_sha, require_text};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CampaignProgress {
    pub attempts: Vec<AttemptRecord>,
    pub evaluations: Vec<EvaluationRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttemptRecord {
    pub attempt_id: String,
    pub preparation_operation_id: String,
    pub request_digest: String,
    pub predecessor: Option<String>,
    pub dispatch: Option<DispatchRecord>,
    pub completion_intent: Option<CompletionIntent>,
    pub outcome: Option<AttemptOutcome>,
    pub failure_custody: Option<FailureCustodyRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailureCustodyRef {
    pub reference: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DispatchCommand {
    pub episode_id: String,
    pub writer_actor: String,
    pub runtime_session: String,
    pub reviewer_profile: String,
    pub begin_transition_id: String,
    pub result_transition_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DispatchRecord {
    pub operation_id: String,
    pub request_digest: String,
    pub command: DispatchCommand,
    pub may_have_entered: bool,
}

/// Returned once, after the dispatch write commits. Replay and recovery return
/// data only; possession of a serialized DispatchRecord is never a permit.
///
/// ```compile_fail
/// use slice_campaign::DispatchPermit;
/// let _: DispatchPermit = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// use slice_campaign::DispatchPermit;
/// fn duplicate(permit: &DispatchPermit) -> DispatchPermit { permit.clone() }
/// ```
#[derive(Debug)]
pub struct DispatchPermit {
    pub(crate) owner_epoch: String,
    pub(crate) root_id: String,
    pub(crate) identity: CampaignIdentity,
    pub(crate) obligation_id: String,
    pub(crate) request_digest: String,
    pub(crate) dispatch_operation_id: String,
    pub(crate) dispatched_revision: CampaignRevision,
    pub(crate) command: DispatchCommand,
}

impl DispatchPermit {
    pub fn dispatch_operation_id(&self) -> &str {
        &self.dispatch_operation_id
    }
}

#[derive(Debug)]
pub struct DispatchedRequest {
    pub request: NativeReviewRequestRef,
    pub command: DispatchCommand,
}

#[derive(Debug)]
pub enum DispatchEffect {
    Applied {
        permit: DispatchPermit,
        receipt: OperationReceipt,
    },
    Replayed {
        record: DispatchRecord,
        receipt: OperationReceipt,
    },
    NoEffect(String),
    OutcomeUnknown(RecoveryLocator),
}

#[derive(Debug)]
pub struct RecoveryHandle {
    pub(crate) owner_epoch: String,
    pub(crate) root_id: String,
    pub(crate) identity: CampaignIdentity,
    pub(crate) obligation_id: String,
    pub(crate) request_digest: String,
    pub(crate) current_revision: CampaignRevision,
}

#[derive(Debug)]
pub struct RecoveredRequest {
    pub request: NativeReviewRequestRef,
    pub dispatch: Option<DispatchRecord>,
    pub completion_intent: Option<CompletionIntent>,
    /// Present only while an original, explicit preparation is still current
    /// and dispatch has never been authorized.
    pub preparation_recovery: Option<RecoveryHandle>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletionIntent {
    pub campaign_operation_id: String,
    pub registration_operation_id: String,
    pub registration_base_revision: CampaignRevision,
    pub obligation_id: String,
    pub attempt_id: String,
    pub preparation_operation_id: String,
    pub prepared_request_sha256: String,
    pub dispatch_operation_id: String,
    pub owner_selection_digest: String,
    pub execution_evidence_ref: ExecutionRefRecord,
    pub native_result_claim_sha256: String,
    pub native_result_raw_sha256: String,
    pub execution_session_id: Option<String>,
    pub observation_selection: ObservationSelection,
    pub selected_claims: Vec<SelectedClaim>,
    pub stages: Vec<StageIntent>,
    pub consumption_intents: Vec<ConsumptionIntent>,
    pub joined_readbacks: Option<JoinedReadbacks>,
    pub outcome: Option<InitialOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionRefRecord {
    pub schema_version: u32,
    pub owner: String,
    pub root_id: String,
    pub profile: String,
    pub attempt_id: String,
    pub record_id: String,
    pub revision: String,
    pub sha256: String,
}

impl From<&review_execution_evidence::ExecutionEvidenceRef> for ExecutionRefRecord {
    fn from(value: &review_execution_evidence::ExecutionEvidenceRef) -> Self {
        Self {
            schema_version: value.schema_version,
            owner: value.owner.clone(),
            root_id: value.root_id.clone(),
            profile: value.profile.clone(),
            attempt_id: value.attempt_id.clone(),
            record_id: value.record_id.clone(),
            revision: value.revision.clone(),
            sha256: value.sha256.clone(),
        }
    }
}

impl ExecutionRefRecord {
    pub(crate) fn checked(&self) -> Result<review_execution_evidence::ExecutionEvidenceRef> {
        let reference = review_execution_evidence::ExecutionEvidenceRef {
            schema_version: self.schema_version,
            owner: self.owner.clone(),
            root_id: self.root_id.clone(),
            profile: self.profile.clone(),
            attempt_id: self.attempt_id.clone(),
            record_id: self.record_id.clone(),
            revision: self.revision.clone(),
            sha256: self.sha256.clone(),
        };
        reference
            .validate()
            .map_err(|error| CampaignError::Contract(error.to_string()))?;
        Ok(reference)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationSelection {
    Present {
        observation_id: String,
        event_identity: String,
        observation_sha256: String,
    },
    Absent {
        reason: String,
        evidence_ref: ExecutionRefRecord,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectedClaim {
    pub claim_id: String,
    pub revision: String,
    pub document_sha256: String,
    pub boundary: String,
    pub consumer: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKind {
    Observation,
    EstablishmentBuilder,
    EstablishmentCampaign,
    AdmissionBuilder,
    AdmissionCampaign,
    EpisodeResultRead,
    ConsumptionBuilder,
    ConsumptionCampaign,
    Finding,
    FindingProjection,
    EvaluationReliance,
    EvaluationProjection,
    InitialOutcome,
    EvaluationCommit,
}

impl StageKind {
    pub(crate) fn is_j2(self) -> bool {
        matches!(
            self,
            Self::ConsumptionBuilder
                | Self::ConsumptionCampaign
                | Self::Finding
                | Self::FindingProjection
                | Self::EvaluationReliance
                | Self::EvaluationProjection
                | Self::InitialOutcome
                | Self::EvaluationCommit
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StageIntent {
    pub stage_id: String,
    pub kind: StageKind,
    pub registration_operation_id: String,
    pub registration_base_revision: CampaignRevision,
    pub command: Value,
    pub request_sha256: String,
    pub original_admission: Value,
    pub original_admission_sha256: String,
    pub owner_selection_digest: String,
    pub owner_locator: Value,
    pub evidence_ref: ExecutionRefRecord,
    pub progress: StageProgress,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StageProgress {
    Registered,
    OwnerCommitted {
        receipt: Value,
    },
    Checked {
        exact_refs: Vec<Value>,
        content_digests: Vec<String>,
    },
    Conflicting {
        reason: String,
    },
    Unresolved {
        reason: String,
        locator: Value,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JoinedReadbacks {
    pub builder: Value,
    pub campaign: Value,
    pub episode: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConsumptionIntent {
    pub claim_revision: String,
    pub establishment: Value,
    pub boundary: String,
    pub consumer: String,
    pub body_digest: String,
    pub reference: String,
    pub registration_operation_id: String,
    pub consumed_at_operation_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InitialOutcome {
    EvidenceUnestablished {
        pair: Value,
    },
    Reported {
        pair: Value,
        consumptions: Value,
        episode: Value,
        finding_set: Value,
        projection: Value,
    },
    AwaitingBuilder {
        pair: Value,
        consumptions: Value,
        episode: Value,
        finding_set: Value,
        projection: Value,
    },
    CorrectionRequired {
        rejection_ref: Value,
        evidence_ref: Value,
    },
}

impl CompletionIntent {
    pub(crate) fn validate_initial(
        &self,
        dispatch: &DispatchRecord,
        attempt_id: &str,
    ) -> Result<()> {
        if self.attempt_id != attempt_id
            || self.dispatch_operation_id != dispatch.operation_id
            || self.outcome.is_some()
            || self.selected_claims.len() != 2
            || self.selected_claims[0].boundary != "builder_projection"
            || self.selected_claims[1].boundary != "campaign_terminalization"
            || self.stages.iter().any(|stage| stage.kind.is_j2())
        {
            return Err(CampaignError::Contract(
                "initial completion intent invalid".into(),
            ));
        }
        for value in [
            &self.campaign_operation_id,
            &self.registration_operation_id,
            &self.obligation_id,
            &self.preparation_operation_id,
            &self.dispatch_operation_id,
            &self.owner_selection_digest,
        ] {
            require_text(value, "completion identity")?;
        }
        for value in [
            &self.prepared_request_sha256,
            &self.native_result_claim_sha256,
            &self.native_result_raw_sha256,
        ] {
            require_sha(value, "completion digest")?;
        }
        self.execution_evidence_ref.checked()?;
        if self
            .consumption_intents
            .iter()
            .any(|c| c.consumed_at_operation_id.is_some())
            || !self.consumption_intents.is_empty() && self.consumption_intents.len() != 2
        {
            return Err(CampaignError::Contract(
                "J1 consumption commitments invalid".into(),
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut kinds = std::collections::BTreeSet::new();
        for stage in &self.stages {
            require_text(&stage.stage_id, "stage ID")?;
            require_text(
                &stage.registration_operation_id,
                "stage registration operation",
            )?;
            require_sha(&stage.request_sha256, "stage request")?;
            require_sha(&stage.original_admission_sha256, "stage admission")?;
            require_sha(&stage.owner_selection_digest, "stage owner selection")?;
            stage.evidence_ref.checked()?;
            if !ids.insert(&stage.stage_id)
                || !kinds.insert(stage.kind)
                || stage.owner_selection_digest != self.owner_selection_digest
                || stage.evidence_ref != self.execution_evidence_ref
            {
                return Err(CampaignError::Contract(
                    "completion stage identity differs".into(),
                ));
            }
        }
        if self.stages.len() > 6
            || (matches!(
                self.observation_selection,
                ObservationSelection::Absent { .. }
            ) && kinds.contains(&StageKind::Observation))
            || (self.joined_readbacks.is_some()
                && ![
                    StageKind::AdmissionBuilder,
                    StageKind::AdmissionCampaign,
                    StageKind::EpisodeResultRead,
                ]
                .iter()
                .all(|kind| kinds.contains(kind)))
        {
            return Err(CampaignError::Contract(
                "completion stage set invalid".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptOutcome {
    Completed,
    FailedWithResult,
    DefinitePreEntryFailure,
}

/// Only trusted execution-custody composition may construct this token. Its
/// presence records an exact definite pre-entry observation, not a timeout.
#[derive(Debug)]
#[allow(dead_code)] // Constructed by the later HP3 custody join.
pub(crate) struct CheckedPreEntryFailure {
    pub(crate) attempt_id: String,
    pub(crate) dispatch_operation_id: String,
    pub(crate) custody_reference: String,
    pub(crate) custody_sha256: String,
    status: RetryObligationStatus,
    current: RetryRecoveryFacts,
    recorded: Option<RetryRecoveryFacts>,
    request: RetryRequestFacts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Full closed vocabulary awaits HP3 owner readback.
pub(crate) enum RetryObligationStatus {
    Executing,
    RetryableFailure,
    RetryExecuting,
    Completed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Full closed vocabulary awaits HP3 owner readback.
pub(crate) enum RetryFailureSignature {
    AuthenticationRequired,
    AuthenticationUnavailable,
    ProcessStartFailed,
    Timeout,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Full closed vocabulary awaits HP3 owner readback.
pub(crate) enum RetryProviderEntry {
    NotEntered,
    Entered,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RetryRecoveryFacts {
    pub(crate) schema_version: u32,
    pub(crate) failure_signature: RetryFailureSignature,
    pub(crate) provider_entry: RetryProviderEntry,
    pub(crate) session_available: bool,
    pub(crate) session_id: String,
    pub(crate) transport_receipt_digest: Option<String>,
    pub(crate) session_artifact_digest: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) result_digest: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct RetryRequestFacts {
    pub(crate) retry_session_id: String,
    pub(crate) continuation_session_id: Option<String>,
    pub(crate) pre_spawn_retry: bool,
}

/// Domain-only predicates. These facts must later come from a checked owner
/// readback; this function grants no public retry capability by itself.
pub(crate) fn validate_retry_recovery(
    status: RetryObligationStatus,
    current: &RetryRecoveryFacts,
    recorded: Option<&RetryRecoveryFacts>,
    request: &RetryRequestFacts,
) -> Result<()> {
    let reject = || CampaignError::Contract("pre-entry retry recovery differs".into());
    if !matches!(
        status,
        RetryObligationStatus::Executing
            | RetryObligationStatus::RetryableFailure
            | RetryObligationStatus::RetryExecuting
    ) || current.schema_version != 1
        || current.provider_entry != RetryProviderEntry::NotEntered
        || current.result_digest.is_some()
        || request.retry_session_id.trim().is_empty()
    {
        return Err(reject());
    }
    let digest = |v: &Option<String>| {
        v.as_ref()
            .is_some_and(|s| require_sha(s, "recovery digest").is_ok())
    };
    match current.failure_signature {
        RetryFailureSignature::AuthenticationRequired => {
            if !current.session_available
                || request.continuation_session_id.as_deref() != Some(current.session_id.as_str())
                || request.pre_spawn_retry
                || !digest(&current.transport_receipt_digest)
                || !digest(&current.session_artifact_digest)
                || current.error_code.is_some()
            {
                return Err(reject());
            }
        }
        RetryFailureSignature::AuthenticationUnavailable
        | RetryFailureSignature::ProcessStartFailed => {
            if current.session_available
                || current.session_id != request.retry_session_id
                || request.continuation_session_id.is_some()
                || current.transport_receipt_digest.is_some()
                || current.session_artifact_digest.is_some()
            {
                return Err(reject());
            }
            if current.failure_signature == RetryFailureSignature::ProcessStartFailed {
                if !request.pre_spawn_retry
                    || current
                        .error_code
                        .as_ref()
                        .is_none_or(|s| s.trim().is_empty())
                {
                    return Err(reject());
                }
            } else if request.pre_spawn_retry || current.error_code.is_some() {
                return Err(reject());
            }
        }
        RetryFailureSignature::Timeout | RetryFailureSignature::Unknown => return Err(reject()),
    }
    if status == RetryObligationStatus::RetryExecuting {
        let prior = recorded.ok_or_else(reject)?;
        // The only accepted drift is a renewed retained-authentication
        // readback for the same continuation session.
        if prior != current
            && (prior.failure_signature != RetryFailureSignature::AuthenticationRequired
                || current.failure_signature != RetryFailureSignature::AuthenticationRequired
                || prior.session_id != current.session_id
                || validate_retry_recovery(RetryObligationStatus::Executing, prior, None, request)
                    .is_err())
        {
            return Err(reject());
        }
    } else if recorded.is_some() {
        return Err(reject());
    }
    Ok(())
}

#[allow(dead_code)] // HP3 supplies independently checked owner custody.
impl CheckedPreEntryFailure {
    #[allow(clippy::too_many_arguments)] // Exact owner readback fields remain separate.
    pub(crate) fn from_owner_facts(
        attempt_id: String,
        dispatch_operation_id: String,
        custody_reference: String,
        custody_sha256: String,
        status: RetryObligationStatus,
        current: RetryRecoveryFacts,
        recorded: Option<RetryRecoveryFacts>,
        request: RetryRequestFacts,
    ) -> Result<Self> {
        for value in [&attempt_id, &dispatch_operation_id, &custody_reference] {
            require_text(value, "retry custody identity")?;
        }
        require_sha(&custody_sha256, "retry custody")?;
        validate_retry_recovery(status, &current, recorded.as_ref(), &request)?;
        Ok(Self {
            attempt_id,
            dispatch_operation_id,
            custody_reference,
            custody_sha256,
            status,
            current,
            recorded,
            request,
        })
    }
    pub(crate) fn validate(&self) -> Result<()> {
        validate_retry_recovery(
            self.status,
            &self.current,
            self.recorded.as_ref(),
            &self.request,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluationRecord {
    pub operation_id: String,
    pub registration_operation_id: String,
    pub finding_id: String,
    pub finding_revision: String,
    pub consumer_tree: String,
    pub decision_scope: String,
    pub valid: bool,
    pub reliance: Value,
    pub projection: Value,
    pub publication_state: String,
}

#[cfg(test)]
mod retry_predicate_tests {
    use super::*;

    #[test]
    fn closed_pre_entry_routes_and_mismatches() {
        let recovery = RetryRecoveryFacts {
            schema_version: 1,
            failure_signature: RetryFailureSignature::AuthenticationRequired,
            provider_entry: RetryProviderEntry::NotEntered,
            session_available: true,
            session_id: "session-1".into(),
            transport_receipt_digest: Some("a".repeat(64)),
            session_artifact_digest: Some("b".repeat(64)),
            error_code: None,
            result_digest: None,
        };
        let request = RetryRequestFacts {
            retry_session_id: "retry-1".into(),
            continuation_session_id: Some("session-1".into()),
            pre_spawn_retry: false,
        };
        assert!(
            validate_retry_recovery(
                RetryObligationStatus::RetryableFailure,
                &recovery,
                None,
                &request
            )
            .is_ok()
        );
        let mut invalid = recovery.clone();
        invalid.provider_entry = RetryProviderEntry::Unknown;
        assert!(
            validate_retry_recovery(
                RetryObligationStatus::RetryableFailure,
                &invalid,
                None,
                &request
            )
            .is_err()
        );
        invalid = recovery.clone();
        invalid.transport_receipt_digest = None;
        assert!(
            validate_retry_recovery(
                RetryObligationStatus::RetryableFailure,
                &invalid,
                None,
                &request
            )
            .is_err()
        );
        invalid = recovery.clone();
        invalid.schema_version = 2;
        assert!(
            validate_retry_recovery(
                RetryObligationStatus::RetryableFailure,
                &invalid,
                None,
                &request
            )
            .is_err()
        );
        invalid = recovery.clone();
        invalid.provider_entry = RetryProviderEntry::Entered;
        assert!(
            validate_retry_recovery(
                RetryObligationStatus::RetryableFailure,
                &invalid,
                None,
                &request
            )
            .is_err()
        );
        invalid = recovery.clone();
        invalid.result_digest = Some("c".repeat(64));
        assert!(
            validate_retry_recovery(
                RetryObligationStatus::RetryableFailure,
                &invalid,
                None,
                &request
            )
            .is_err()
        );
        let mut wrong_request = request.clone();
        wrong_request.continuation_session_id = Some("different".into());
        assert!(
            validate_retry_recovery(
                RetryObligationStatus::RetryableFailure,
                &recovery,
                None,
                &wrong_request
            )
            .is_err()
        );
        assert!(
            validate_retry_recovery(RetryObligationStatus::Completed, &recovery, None, &request)
                .is_err()
        );
        let mut changed = recovery.clone();
        changed.session_id = "other".into();
        assert!(
            validate_retry_recovery(
                RetryObligationStatus::RetryExecuting,
                &changed,
                Some(&recovery),
                &request
            )
            .is_err()
        );
        assert!(
            validate_retry_recovery(
                RetryObligationStatus::RetryExecuting,
                &recovery,
                None,
                &request
            )
            .is_err()
        );
        let mut renewed = recovery.clone();
        renewed.transport_receipt_digest = Some("d".repeat(64));
        assert!(
            validate_retry_recovery(
                RetryObligationStatus::RetryExecuting,
                &renewed,
                Some(&recovery),
                &request
            )
            .is_ok()
        );
        for signature in [
            RetryFailureSignature::Timeout,
            RetryFailureSignature::Unknown,
        ] {
            invalid = recovery.clone();
            invalid.failure_signature = signature;
            assert!(
                validate_retry_recovery(RetryObligationStatus::Executing, &invalid, None, &request)
                    .is_err()
            );
        }

        let mut unavailable = recovery.clone();
        unavailable.failure_signature = RetryFailureSignature::AuthenticationUnavailable;
        unavailable.session_available = false;
        unavailable.session_id = "retry-1".into();
        unavailable.transport_receipt_digest = None;
        unavailable.session_artifact_digest = None;
        let fresh = RetryRequestFacts {
            retry_session_id: "retry-1".into(),
            continuation_session_id: None,
            pre_spawn_retry: false,
        };
        assert!(
            validate_retry_recovery(RetryObligationStatus::Executing, &unavailable, None, &fresh)
                .is_ok()
        );
        invalid = unavailable.clone();
        invalid.transport_receipt_digest = Some("a".repeat(64));
        assert!(
            validate_retry_recovery(RetryObligationStatus::Executing, &invalid, None, &fresh)
                .is_err()
        );
        let mut process = unavailable.clone();
        process.failure_signature = RetryFailureSignature::ProcessStartFailed;
        process.error_code = Some("ENOENT".into());
        let mut pre_spawn = fresh.clone();
        pre_spawn.pre_spawn_retry = true;
        assert!(
            validate_retry_recovery(RetryObligationStatus::Executing, &process, None, &pre_spawn)
                .is_ok()
        );
        assert!(
            validate_retry_recovery(RetryObligationStatus::Executing, &process, None, &fresh)
                .is_err()
        );
        invalid = process.clone();
        invalid.error_code = Some(" ".into());
        assert!(
            validate_retry_recovery(RetryObligationStatus::Executing, &invalid, None, &pre_spawn)
                .is_err()
        );
    }
}
