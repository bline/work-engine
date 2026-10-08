//! Campaign-owned execution history. These records describe intent and owner
//! effects; they never attest to an episode result or a claim establishment.

use serde::{Deserialize, Serialize};

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
#[serde(rename_all = "snake_case")]
pub enum ChildKind {
    Observation,
    Establishment,
    EpisodeResult,
    Finding,
    Reliance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChildIntent {
    pub kind: ChildKind,
    pub operation_id: String,
    pub content_digest: String,
    pub owner_root_id: String,
    pub grant_identity: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletionIntent {
    pub campaign_operation_id: String,
    pub episode_id: String,
    pub attempt_id: String,
    pub initial_episode_revision: Option<String>,
    pub result_digest: Option<String>,
    pub children: Vec<ChildIntent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResultIntent {
    pub(crate) episode_id: String,
    pub(crate) result_digest: String,
    pub(crate) episode_revision: String,
    pub(crate) children: Vec<ChildIntent>,
}

impl CompletionIntent {
    pub(crate) fn validate_initial(
        &self,
        dispatch: &DispatchRecord,
        attempt_id: &str,
    ) -> Result<()> {
        if self.episode_id != dispatch.command.episode_id
            || self.attempt_id != attempt_id
            || self.initial_episode_revision.is_some()
            || self.result_digest.is_some()
        {
            return Err(CampaignError::Contract(
                "initial completion intent does not match dispatch".into(),
            ));
        }
        if self.children.is_empty() {
            return Err(CampaignError::Contract("completion children absent".into()));
        }
        require_text(&self.campaign_operation_id, "completion intent operation")?;
        let mut ids = std::collections::BTreeSet::new();
        for child in &self.children {
            require_text(&child.operation_id, "child operation")?;
            require_sha(&child.content_digest, "child content")?;
            require_text(&child.owner_root_id, "child owner root")?;
            require_text(&child.grant_identity, "child grant")?;
            if !ids.insert(child.operation_id.as_str())
                || child.operation_id == dispatch.operation_id
            {
                return Err(CampaignError::Contract(
                    "completion child operation overlaps".into(),
                ));
            }
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
    pub finding_id: String,
    pub finding_revision: String,
    pub consumer_tree: String,
    pub decision_scope: String,
    pub valid: bool,
    pub reliance: ChildIntent,
    pub published: bool,
}

#[derive(Debug)]
#[allow(dead_code)] // Materialized only after CE finding and builder-authority readback.
pub(crate) struct CheckedBuilderEvaluation {
    pub(crate) finding_id: String,
    pub(crate) finding_revision: String,
    pub(crate) consumer_tree: String,
    pub(crate) decision_scope: String,
    pub(crate) valid: bool,
    pub(crate) reliance: ChildIntent,
}

#[derive(Debug)]
#[allow(dead_code)] // Materialized only after CE reliance and projection readback.
pub(crate) struct CheckedReliance {
    pub(crate) finding_id: String,
    pub(crate) finding_revision: String,
    pub(crate) operation_id: String,
    pub(crate) content_digest: String,
    pub(crate) owner_root_id: String,
    pub(crate) grant_identity: String,
}

#[derive(Debug)]
#[allow(dead_code)] // Constructed from exact episode and both CE owner readbacks.
pub(crate) struct CheckedInitialOutcome {
    pub(crate) episode_id: String,
    pub(crate) episode_revision: String,
    pub(crate) result_digest: String,
    pub(crate) builder_claim_id: String,
    pub(crate) terminal_claim_id: String,
    pub(crate) succeeded: bool,
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
