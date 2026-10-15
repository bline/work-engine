//! Offline administration over the sole lifecycle store. These values are
//! observations and exact CAS requests; none grants a new execution entry.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, VerifyingKey};
use lifecycle_core::HandoffId;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use work_engine_types::CodecContract;

use crate::{SqliteLifecycleStore, StoreError};

// Offline custody capsules retain their actual schema across read-only inspection.
// Writable migration remains owned by SqliteLifecycleStore::open_inner.
fn supported_admin_schema(version: i64) -> bool {
    matches!(version, 4 | 5)
}

type EffectRecoveryRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);
type AdminDispositionProjection = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);
type PriorDispositionRow = (String, String, i64, Option<String>, Option<String>, String);
type HandoffCommitRow = (
    String,
    String,
    String,
    String,
    i64,
    Option<i64>,
    Option<String>,
    Option<String>,
);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct AdoptionIntent {
    capsule_sha256: String,
    source_store_id: String,
    source_revision: u64,
    handoff_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExactAttemptSelector {
    pub store_id: String,
    pub source_lineage: Option<String>,
    pub subject_id: String,
    pub input_id: Option<String>,
    pub delivery_id: Option<String>,
    pub effect_id: Option<String>,
    pub attempt_id: Option<String>,
    pub native_session_id: Option<String>,
    pub invocation_id: Option<String>,
    pub incarnation_id: Option<String>,
    pub operation_contract_id: Option<String>,
    pub operation_implementation_id: Option<String>,
    pub operation_executor_id: Option<String>,
    pub profile_digest: Option<String>,
    pub claim_revision: u64,
    pub observation_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ObservationAvailability {
    Complete,
    Partial,
    Unavailable,
    Conflicted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectCoverage {
    KnownNotEntered,
    AccountedFiniteRead,
    AccountedNativeTurn,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExactAttemptRecovery {
    pub selector: ExactAttemptSelector,
    pub classification: String,
    pub availability: ObservationAvailability,
    pub outcome: Option<String>,
    pub settlement: Option<String>,
    pub effect_coverage: EffectCoverage,
    pub provider_thread_id: Option<String>,
    pub provider_turn_id: Option<String>,
    pub transport_session: Option<String>,
    pub transport_sequence: Option<u64>,
    pub observation_revision: u64,
    pub disposition: Option<String>,
    pub disposition_command_id: Option<String>,
    pub retired_original_outcome: Option<String>,
    pub retired_original_settlement: Option<String>,
    pub gaps: Vec<String>,
    pub conflicts: Vec<String>,
    pub artifact_digests: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QualifiedRecoveryFact {
    pub command_id: String,
    pub selector: ExactAttemptSelector,
    pub expected_observation_revision: u64,
    pub source_profile: String,
    pub transport_session: String,
    pub transport_sequence: u64,
    pub provider_thread_id: Option<String>,
    pub provider_turn_id: Option<String>,
    pub native_call_id: Option<String>,
    pub raw_sha256: String,
    pub raw_artifact_sha256: Option<String>,
    pub codec: String,
    pub observed_wall_ms: i64,
    pub availability: ObservationAvailability,
    pub outcome: Option<String>,
    pub settlement: Option<String>,
    pub effect_coverage: EffectCoverage,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecoveryApplyReceipt {
    pub command_id: String,
    pub observation_revision: u64,
    pub classification: String,
    pub applied: bool,
}

/// The signature covers the exact UTF-8 JSON grant bytes, including the named
/// codec/domain. A caller cannot supply or replace the verification key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SignedDispositionRequest {
    pub command_id: String,
    pub selector: ExactAttemptSelector,
    pub grant_bytes: Vec<u8>,
    pub signature: Vec<u8>,
    pub key_id: String,
    pub clock_wall_ms: i64,
    pub expected_store_fence_revision: u64,
}

/// Named v1 test domain. These bytes are signed exactly as supplied; there is
/// no alternate-codec guessing or caller-provided trust root.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispositionGrant {
    pub codec: String,
    pub domain: String,
    pub grant_id: String,
    pub grant_revision: u64,
    pub issuer: String,
    pub principal: String,
    pub store_id: String,
    pub subject_id: String,
    pub claim_id: String,
    pub observation_revision: u64,
    pub scope: String,
    pub expires_wall_ms: i64,
    pub effect_coverage: EffectCoverage,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DispositionReceipt {
    pub command_id: String,
    pub claim_id: String,
    pub observation_revision: u64,
    pub disposition: String,
    pub original_outcome: Option<String>,
    pub original_settlement: Option<String>,
    pub grant_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapsuleArtifact {
    pub relative_path: String,
    pub digest_hex: String,
    pub byte_length: u64,
    pub codec: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapsuleReceipt {
    pub version: u16,
    pub capsule_root: PathBuf,
    pub source_store_id: String,
    pub source_revision: u64,
    pub source_schema: u32,
    pub database_sha256: String,
    pub artifacts: Vec<CapsuleArtifact>,
    pub semantic_artifact_digests: Vec<String>,
    pub native_invocation_ids: Vec<String>,
    pub capsule_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdoptionReceipt {
    pub handoff_id: String,
    pub capsule_sha256: String,
    pub source_store_id: String,
    pub source_revision: u64,
    pub destination_store_id: String,
    pub destination_root: PathBuf,
    pub destination_revision: u64,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FenceReceipt {
    pub handoff_id: String,
    pub source_store_id: String,
    pub source_root: PathBuf,
    pub destination_store_id: String,
    pub capsule_sha256: String,
    pub source_revision: u64,
}

/// Component selection proof. Its producer must hold the external launch
/// guard across source fence, manifest CAS and destination commit. A production
/// deployer/trust contract is deliberately outside this disposable profile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SelectionEvidence {
    pub handoff_id: String,
    pub source_store_id: String,
    pub destination_store_id: String,
    pub capsule_sha256: String,
    pub prior_selection_revision: u64,
    pub current_selection_revision: u64,
    pub current_selection_digest: String,
    pub launch_guard_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OwnershipReceipt {
    pub handoff_id: String,
    pub destination_store_id: String,
    pub capsule_sha256: String,
    pub ownership_revision: u64,
    pub selection_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HandoffProjection {
    pub handoff_id: String,
    pub capsule_sha256: String,
    pub source_store_id: String,
    pub destination_store_id: String,
    pub status: String,
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StoreOwnershipProjection {
    pub store_id: String,
    pub status: String,
    pub revision: u64,
    pub handoff_id: Option<String>,
    pub capsule_sha256: Option<String>,
    pub peer_store_id: Option<String>,
    pub selection_revision: Option<u64>,
}

impl SqliteLifecycleStore {
    pub fn admin_ownership(&self) -> Result<StoreOwnershipProjection, StoreError> {
        self.verify_identity()?;
        let row: (String, i64, Option<String>, Option<String>, Option<String>, Option<i64>) = self.connection.query_row(
            "SELECT status,revision,handoff_id,capsule_sha256,peer_store_id,selection_revision FROM admin_ownership WHERE singleton=1",
            [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
        ).map_err(|_| StoreError::Unavailable)?;
        Ok(StoreOwnershipProjection {
            store_id: self.admin_store_id()?,
            status: row.0,
            revision: u64::try_from(row.1).map_err(|_| StoreError::Unavailable)?,
            handoff_id: row.2,
            capsule_sha256: row.3,
            peer_store_id: row.4,
            selection_revision: row
                .5
                .map(|n| u64::try_from(n).map_err(|_| StoreError::Unavailable))
                .transpose()?,
        })
    }
    pub fn read_exact_attempt(
        &self,
        selector: &ExactAttemptSelector,
    ) -> Result<ExactAttemptRecovery, StoreError> {
        self.verify_identity()?;
        if self.admin_store_id()? != selector.store_id || selector.subject_id.is_empty() {
            return Err(StoreError::Rejected);
        }
        let source_lineage: Option<String> = self
            .connection
            .query_row(
                "SELECT source_store_id FROM admin_store_lineage WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        if selector.source_lineage != source_lineage {
            return Err(StoreError::Rejected);
        }
        let subject_revision: i64 = self
            .connection
            .query_row(
                "SELECT revision FROM subjects WHERE subject_id=?1",
                [&selector.subject_id],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Rejected)?;
        if subject_revision < 0 || subject_revision as u64 != selector.claim_revision {
            return Err(StoreError::ClaimConflict);
        }
        let claim_id = selector_claim_id(selector)?;
        let observation_revision = self.admin_observation_revision(&claim_id)?;
        if observation_revision != selector.observation_revision {
            return Err(StoreError::ClaimConflict);
        }
        let (
            disposition,
            disposition_command_id,
            retired_original_outcome,
            retired_original_settlement,
        ) = self.admin_disposition(&claim_id)?;
        if let Some(invocation) = &selector.invocation_id {
            let row: (String, String, String, String, String) = self.connection.query_row(
                "SELECT s.subject_id,s.session_id,s.incarnation_id,s.profile_digest,t.attempt_id FROM native_turns t JOIN native_sessions s ON s.session_id=t.session_id WHERE t.invocation_id=?1",
                [invocation], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
            ).map_err(|_| StoreError::Rejected)?;
            if row.0 != selector.subject_id
                || selector
                    .native_session_id
                    .as_ref()
                    .is_some_and(|id| id != &row.1)
                || selector
                    .incarnation_id
                    .as_ref()
                    .is_some_and(|id| id != &row.2)
                || selector
                    .profile_digest
                    .as_ref()
                    .is_some_and(|id| id != &row.3)
                || selector.attempt_id.as_ref().is_some_and(|id| id != &row.4)
                || selector.effect_id.is_some()
                || selector.input_id.is_some()
                || selector.delivery_id.is_some()
            {
                return Err(StoreError::Rejected);
            }
            let native = self
                .native_recovery(invocation)?
                .ok_or(StoreError::Rejected)?;
            if selector
                .operation_contract_id
                .as_ref()
                .is_some_and(|contract| {
                    native
                        .children
                        .iter()
                        .any(|child| &child.contract_id != contract)
                })
                || selector
                    .operation_implementation_id
                    .as_ref()
                    .is_some_and(|implementation| {
                        native
                            .children
                            .iter()
                            .any(|child| &child.implementation_id != implementation)
                    })
                || selector
                    .operation_executor_id
                    .as_ref()
                    .is_some_and(|executor| {
                        native
                            .children
                            .iter()
                            .any(|child| &child.executor_id != executor)
                    })
            {
                return Err(StoreError::Rejected);
            }
            let child_coverage = native.unsettled_children == 0
                && native.children.iter().all(|child| {
                    child.class == "completed_settled"
                        && child.contract_id == "workspace.snapshot.read.v1"
                        && child.implementation_id == "rust.snapshot.read.v1"
                        && child.executor_id == "lifecycle.service.snapshot"
                        && child.result_sha256.is_some()
                });
            let complete = native.class == "CompletedSettled"
                && native.history_status == "observed_complete"
                && native.final_sha256.is_some()
                && child_coverage;
            let availability = if complete {
                ObservationAvailability::Complete
            } else if native.history_status == "conflicted" {
                ObservationAvailability::Conflicted
            } else if native.history_transport_session.is_some() {
                ObservationAvailability::Partial
            } else {
                ObservationAvailability::Unavailable
            };
            let mut gaps = Vec::new();
            if native.history_status != "observed_complete" {
                gaps.push("native_history_not_complete".into());
            }
            if native.unsettled_children != 0 {
                gaps.push("native_children_unsettled".into());
            }
            if !child_coverage {
                gaps.push("unsupported_or_unaccounted_child".into());
            }
            let artifact_digests = native
                .final_sha256
                .iter()
                .cloned()
                .chain(
                    native
                        .children
                        .iter()
                        .filter_map(|child| child.result_sha256.clone()),
                )
                .collect();
            let ingress_conflicts:i64=self.connection.query_row(
                "SELECT COUNT(*) FROM native_ingress_conflicts WHERE transport_session IN (SELECT transport_session FROM native_ingress WHERE thread_id=?1)",
                [&native.thread_id],|r|r.get(0),
            ).map_err(|_| StoreError::Unavailable)?;
            return Ok(ExactAttemptRecovery {
                selector: selector.clone(),
                classification: native.class,
                availability,
                outcome: native.final_sha256.as_ref().map(|_| "completed".into()),
                settlement: if complete {
                    Some("established".into())
                } else {
                    Some("unresolved".into())
                },
                effect_coverage: if complete {
                    EffectCoverage::AccountedNativeTurn
                } else {
                    EffectCoverage::Unknown
                },
                provider_thread_id: native.thread_id,
                provider_turn_id: native.turn_id,
                transport_session: native.history_transport_session,
                transport_sequence: native.history_sequence,
                observation_revision,
                disposition,
                disposition_command_id,
                retired_original_outcome,
                retired_original_settlement,
                gaps,
                conflicts: if ingress_conflicts > 0 {
                    vec!["native_ingress_conflict".into()]
                } else {
                    Vec::new()
                },
                artifact_digests,
            });
        }
        let effect = selector.effect_id.as_ref().ok_or(StoreError::Rejected)?;
        let row: EffectRecoveryRow = self.connection.query_row(
            "SELECT i.input_id,i.delivery_id,e.state,a.attempt_id,a.incarnation_id,a.outcome,a.settlement_kind,a.provider_thread_id,a.provider_turn_id,a.settlement_evidence FROM effects e JOIN inputs i ON i.input_id=e.input_id LEFT JOIN attempts a ON a.effect_id=e.effect_id WHERE e.effect_id=?1 AND e.subject_id=?2",
            params![effect,&selector.subject_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?)),
        ).map_err(|_| StoreError::Rejected)?;
        if selector.input_id.as_ref().is_some_and(|id| id != &row.0)
            || selector.delivery_id.as_ref().is_some_and(|id| id != &row.1)
            || selector.attempt_id != row.3
            || selector.incarnation_id != row.4
            || selector.native_session_id.is_some()
            || selector.profile_digest.is_some()
            || selector.operation_contract_id.is_some()
            || selector.operation_implementation_id.is_some()
            || selector.operation_executor_id.is_some()
        {
            return Err(StoreError::Rejected);
        }
        let classification = match (row.2.as_str(), row.5.as_deref(), row.6.as_deref()) {
            ("prepared", None, None) => "PreparedNeedsGrant",
            (_, Some("completed"), Some("established")) => "CompletedSettled",
            ("entered", _, _) => "EnteredUncertain",
            _ => "BlockedOrConflicted",
        }
        .to_string();
        let complete = classification == "CompletedSettled";
        let conflict_count: i64 = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM observation_conflicts WHERE effect_id=?1",
                [effect],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        Ok(ExactAttemptRecovery {
            selector: selector.clone(),
            classification: classification.clone(),
            availability: if complete {
                ObservationAvailability::Complete
            } else {
                ObservationAvailability::Unavailable
            },
            outcome: row.5,
            settlement: row.6,
            effect_coverage: if classification == "PreparedNeedsGrant" {
                EffectCoverage::KnownNotEntered
            } else {
                EffectCoverage::Unknown
            },
            provider_thread_id: row.7,
            provider_turn_id: row.8,
            transport_session: None,
            transport_sequence: None,
            observation_revision,
            disposition,
            disposition_command_id,
            retired_original_outcome,
            retired_original_settlement,
            gaps: if complete {
                Vec::new()
            } else {
                vec!["effect_not_fully_settled".into()]
            },
            conflicts: if conflict_count > 0 {
                vec!["effect_observation_conflict".into()]
            } else {
                Vec::new()
            },
            artifact_digests: Vec::new(),
        })
    }

    pub fn admin_store_identity(&self) -> Result<String, StoreError> {
        self.verify_identity()?;
        self.admin_store_id()
    }

    fn admin_store_id(&self) -> Result<String, StoreError> {
        self.connection
            .query_row(
                "SELECT store_id FROM store_identity WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)
    }

    fn admin_observation_revision(&self, claim_id: &str) -> Result<u64, StoreError> {
        let revision: i64 = self.connection.query_row(
            "SELECT COALESCE(MAX(observation_revision),0) FROM admin_recovery_observations WHERE claim_id=?1",
            [claim_id], |r| r.get(0),
        ).map_err(|_| StoreError::Unavailable)?;
        u64::try_from(revision).map_err(|_| StoreError::Unavailable)
    }

    fn admin_disposition(&self, claim_id: &str) -> Result<AdminDispositionProjection, StoreError> {
        self.connection.query_row(
            "SELECT disposition,command_id,original_outcome,original_settlement FROM admin_dispositions WHERE claim_id=?1",
            [claim_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
        ).optional().map(|row|row.unwrap_or((None,None,None,None))).map_err(|_| StoreError::Unavailable)
    }

    pub fn apply_qualified_recovery(
        &mut self,
        fact: &QualifiedRecoveryFact,
    ) -> Result<RecoveryApplyReceipt, StoreError> {
        self.verify_identity()?;
        if fact.command_id.is_empty()
            || fact.source_profile != "s5-native-ingress"
            || fact.codec != "native-ingress.raw.v1"
            || fact.transport_sequence == 0
            || fact.selector.invocation_id.is_none()
        {
            return Err(StoreError::Rejected);
        }
        let fact_json = serde_json::to_vec(fact).map_err(|_| StoreError::Rejected)?;
        let fact_sha256 = digest(&fact_json)?;
        let prior: Option<(String, i64, String)> = self.connection.query_row(
            "SELECT fact_sha256,observation_revision,classification FROM admin_recovery_observations WHERE command_id=?1",
            [&fact.command_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        ).optional().map_err(|_| StoreError::Unavailable)?;
        if let Some((sha, revision, classification)) = prior {
            if sha != fact_sha256 {
                return Err(StoreError::ClaimConflict);
            }
            return Ok(RecoveryApplyReceipt {
                command_id: fact.command_id.clone(),
                observation_revision: u64::try_from(revision)
                    .map_err(|_| StoreError::Unavailable)?,
                classification,
                applied: true,
            });
        }
        let observed = self.read_exact_attempt(&fact.selector)?;
        if observed.observation_revision != fact.expected_observation_revision
            || observed.availability != fact.availability
            || observed.effect_coverage != fact.effect_coverage
            || observed.outcome != fact.outcome
            || observed.settlement != fact.settlement
        {
            return Err(StoreError::ClaimConflict);
        }
        let ingress: (String, Option<String>, Option<String>, Option<String>, Option<String>) = self.connection.query_row(
            "SELECT raw_sha256,thread_id,turn_id,item_id,session_id FROM native_ingress WHERE transport_session=?1 AND sequence=?2",
            params![&fact.transport_session, to_i64(fact.transport_sequence)?],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
        ).map_err(|_| StoreError::Rejected)?;
        let invocation = fact
            .selector
            .invocation_id
            .as_ref()
            .ok_or(StoreError::Rejected)?;
        let selected: (String, String, Option<String>) = self
            .connection
            .query_row(
                "SELECT session_id,thread_id,turn_id FROM native_turns WHERE invocation_id=?1",
                [invocation],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|_| StoreError::Rejected)?;
        if ingress.0 != fact.raw_sha256
            || fact.selector.native_session_id.as_ref() != Some(&selected.0)
            || ingress.4.as_ref() != Some(&selected.0)
            || fact.provider_thread_id.as_ref() != Some(&selected.1)
            || ingress.1.as_ref() != Some(&selected.1)
            || selected.2.is_none()
            || fact.provider_turn_id != selected.2
            || ingress.2 != selected.2
            || fact
                .native_call_id
                .as_ref()
                .is_some_and(|id| ingress.3.as_ref() != Some(id))
            || fact.raw_artifact_sha256.as_ref().is_some_and(|digest| {
                self.read_committed_artifact(digest)
                    .ok()
                    .flatten()
                    .is_none()
            })
        {
            return Err(StoreError::Rejected);
        }
        let claim_id = selector_claim_id(&fact.selector)?;
        let selector_sha256 =
            digest(&serde_json::to_vec(&fact.selector).map_err(|_| StoreError::Rejected)?)?;
        let next = fact
            .expected_observation_revision
            .checked_add(1)
            .ok_or(StoreError::Rejected)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let current: i64 = tx.query_row(
            "SELECT COALESCE(MAX(observation_revision),0) FROM admin_recovery_observations WHERE claim_id=?1",
            [&claim_id], |r| r.get(0),
        ).map_err(|_| StoreError::Unavailable)?;
        if current != to_i64(fact.expected_observation_revision)? {
            return Err(StoreError::ClaimConflict);
        }
        tx.execute(
            "INSERT INTO admin_recovery_observations(command_id,selector_sha256,fact_sha256,fact_json,claim_id,observation_revision,classification,availability,outcome,settlement,effect_coverage,observed_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![&fact.command_id,selector_sha256,fact_sha256,fact_json,claim_id,to_i64(next)?,
                &observed.classification,availability_name(&fact.availability),&fact.outcome,&fact.settlement,coverage_name(&fact.effect_coverage),fact.observed_wall_ms],
        ).map_err(|_| StoreError::ClaimConflict)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(RecoveryApplyReceipt {
            command_id: fact.command_id.clone(),
            observation_revision: next,
            classification: observed.classification,
            applied: true,
        })
    }

    pub fn retire_no_retry(
        &mut self,
        request: &SignedDispositionRequest,
    ) -> Result<DispositionReceipt, StoreError> {
        self.verify_identity()?;
        if request.command_id.is_empty()
            || request.key_id != "test-rfc8032-1"
            || request.grant_bytes.is_empty()
            || request.grant_bytes.len() > 4096
        {
            return Err(StoreError::Rejected);
        }
        let request_sha256 =
            digest(&serde_json::to_vec(request).map_err(|_| StoreError::Rejected)?)?;
        let prior: Option<PriorDispositionRow> = self.connection.query_row(
            "SELECT request_sha256,claim_id,observation_revision,original_outcome,original_settlement,grant_sha256 FROM admin_dispositions WHERE command_id=?1",
            [&request.command_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
        ).optional().map_err(|_| StoreError::Unavailable)?;
        if let Some((sha, claim, revision, outcome, settlement, grant_sha256)) = prior {
            if sha != request_sha256 {
                return Err(StoreError::ClaimConflict);
            }
            return Ok(DispositionReceipt {
                command_id: request.command_id.clone(),
                claim_id: claim,
                observation_revision: u64::try_from(revision)
                    .map_err(|_| StoreError::Unavailable)?,
                disposition: "retired_no_retry".into(),
                original_outcome: outcome,
                original_settlement: settlement,
                grant_sha256,
            });
        }
        const TEST_KEY: [u8; 32] = [
            0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
            0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
            0xf7, 0x07, 0x51, 0x1a,
        ];
        let key = VerifyingKey::from_bytes(&TEST_KEY).map_err(|_| StoreError::Unavailable)?;
        let signature =
            Signature::try_from(request.signature.as_slice()).map_err(|_| StoreError::Rejected)?;
        key.verify_strict(&request.grant_bytes, &signature)
            .map_err(|_| StoreError::Rejected)?;
        let grant: DispositionGrant =
            serde_json::from_slice(&request.grant_bytes).map_err(|_| StoreError::Rejected)?;
        let claim_id = selector_claim_id(&request.selector)?;
        if grant.codec != "json-exact-ed25519.v1"
            || grant.domain != "lifecycle.disposition.v1"
            || grant.scope != "retire_no_retry"
            || grant.issuer != self.trusted_issuer
            || grant.store_id != request.selector.store_id
            || grant.subject_id != request.selector.subject_id
            || grant.claim_id != claim_id
            || grant.observation_revision != request.selector.observation_revision
            || grant.expires_wall_ms <= request.clock_wall_ms
        {
            return Err(StoreError::Rejected);
        }
        let observed = self.read_exact_attempt(&request.selector)?;
        if observed.effect_coverage != grant.effect_coverage
            || matches!(observed.effect_coverage, EffectCoverage::Unknown)
            || matches!(observed.availability, ObservationAvailability::Conflicted)
            || !observed.conflicts.is_empty()
        {
            return Err(StoreError::Rejected);
        }
        let ownership = self.admin_ownership()?;
        if ownership.status != "active"
            || ownership.revision != request.expected_store_fence_revision
        {
            return Err(StoreError::ClaimConflict);
        }
        let stored_grant: (String,String,String,String,i64,i64,bool) = self.connection.query_row(
            "SELECT issuer,principal,subject_id,scope,expires_ms,revision,revoked FROM grants WHERE grant_id=?1",
            [&grant.grant_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?)),
        ).map_err(|_| StoreError::Rejected)?;
        if stored_grant.0 != grant.issuer
            || stored_grant.1 != grant.principal
            || stored_grant.2 != grant.subject_id
            || stored_grant.3 != grant.scope
            || stored_grant.4 != grant.expires_wall_ms
            || stored_grant.5 != to_i64(grant.grant_revision)?
            || stored_grant.6
            || request.clock_wall_ms >= stored_grant.4
        {
            return Err(StoreError::Rejected);
        }
        let last_wall: i64 = self
            .connection
            .query_row(
                "SELECT last_wall_ms FROM subjects WHERE subject_id=?1",
                [&grant.subject_id],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        if request.clock_wall_ms < last_wall {
            return Err(StoreError::Rejected);
        }
        let selector_sha256 =
            digest(&serde_json::to_vec(&request.selector).map_err(|_| StoreError::Rejected)?)?;
        let grant_sha256 = digest(&request.grant_bytes)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let claim_prior: Option<String> = tx
            .query_row(
                "SELECT command_id FROM admin_dispositions WHERE claim_id=?1",
                [&claim_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        if claim_prior.is_some() {
            return Err(StoreError::ClaimConflict);
        }
        let current_fence: i64 = tx
            .query_row(
                "SELECT revision FROM admin_ownership WHERE singleton=1 AND status='active'",
                [],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::ClaimConflict)?;
        let current_obs:i64=tx.query_row(
            "SELECT COALESCE(MAX(observation_revision),0) FROM admin_recovery_observations WHERE claim_id=?1", [&claim_id], |r|r.get(0),
        ).map_err(|_| StoreError::Unavailable)?;
        if current_fence != to_i64(request.expected_store_fence_revision)?
            || current_obs != to_i64(grant.observation_revision)?
        {
            return Err(StoreError::ClaimConflict);
        }
        tx.execute(
            "INSERT INTO admin_dispositions(command_id,claim_id,selector_sha256,request_sha256,grant_sha256,observation_revision,original_outcome,original_settlement,disposition,wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'retired_no_retry',?9)",
            params![&request.command_id,&claim_id,selector_sha256,request_sha256,grant_sha256,to_i64(grant.observation_revision)?,&observed.outcome,&observed.settlement,request.clock_wall_ms],
        ).map_err(|_| StoreError::ClaimConflict)?;
        tx.execute(
            "UPDATE subjects SET last_wall_ms=?1 WHERE subject_id=?2 AND last_wall_ms<=?1",
            params![request.clock_wall_ms, &grant.subject_id],
        )
        .map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(DispositionReceipt {
            command_id: request.command_id.clone(),
            claim_id,
            observation_revision: grant.observation_revision,
            disposition: "retired_no_retry".into(),
            original_outcome: observed.outcome,
            original_settlement: observed.settlement,
            grant_sha256,
        })
    }

    pub fn export_current(&self, destination: &Path) -> Result<CapsuleReceipt, StoreError> {
        self.verify_identity()?;
        let ownership = self.admin_ownership()?;
        if ownership.status != "active" {
            return Err(StoreError::Rejected);
        }
        let source_revision = self.admin_source_revision()?;
        if destination.exists() {
            let prior = Self::read_capsule(destination)?;
            if prior.source_store_id == ownership.store_id
                && prior.source_revision == source_revision
            {
                return Ok(prior);
            }
            return Err(StoreError::ClaimConflict);
        }
        let parent = destination.parent().ok_or(StoreError::Rejected)?;
        if !fs::symlink_metadata(parent)
            .map_err(|_| StoreError::Unavailable)?
            .file_type()
            .is_dir()
        {
            return Err(StoreError::Rejected);
        }
        fs::create_dir(destination).map_err(|_| StoreError::ClaimConflict)?;
        sync_dir(parent)?;
        self.snapshot_to(&destination.join("lifecycle.sqlite"))?;
        let snapshot = rusqlite::Connection::open_with_flags(
            destination.join("lifecycle.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| StoreError::Unavailable)?;
        let source_schema: i64 = snapshot
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|_| StoreError::Unavailable)?;
        if !supported_admin_schema(source_schema) {
            return Err(StoreError::Rejected);
        }
        crate::recovery::check_coherence(&snapshot)?;
        drop(snapshot);
        let database_sha256 = digest(
            &fs::read(destination.join("lifecycle.sqlite")).map_err(|_| StoreError::Unavailable)?,
        )?;
        let artifacts_dir = destination.join("artifacts");
        let blobs_dir = artifacts_dir.join("blobs");
        fs::create_dir(&artifacts_dir).map_err(|_| StoreError::Unavailable)?;
        fs::create_dir(&blobs_dir).map_err(|_| StoreError::Unavailable)?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT digest_hex,relative_path,byte_length FROM artifacts ORDER BY digest_hex",
            )
            .map_err(|_| StoreError::Unavailable)?;
        let rows = statement
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|_| StoreError::Unavailable)?;
        let mut artifacts = Vec::new();
        for row in rows {
            let (sha, path, length) = row.map_err(|_| StoreError::Unavailable)?;
            if path != format!("artifacts/blobs/{sha}") || length < 0 {
                return Err(StoreError::Rejected);
            }
            let original = self.root.join(&path);
            let meta = fs::symlink_metadata(&original).map_err(|_| StoreError::Unavailable)?;
            if !meta.file_type().is_file() {
                return Err(StoreError::Rejected);
            }
            let bytes = self
                .read_committed_artifact(&sha)?
                .ok_or(StoreError::Unavailable)?;
            if bytes.len() as i64 != length {
                return Err(StoreError::Unavailable);
            }
            let copy = blobs_dir.join(&sha);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&copy)
                .map_err(|_| StoreError::Unavailable)?;
            file.write_all(&bytes)
                .map_err(|_| StoreError::Unavailable)?;
            file.sync_all().map_err(|_| StoreError::Unavailable)?;
            artifacts.push(CapsuleArtifact {
                relative_path: path,
                digest_hex: sha,
                byte_length: length as u64,
                codec: "binary-artifact-v1".into(),
            });
        }
        sync_dir(&blobs_dir)?;
        sync_dir(&artifacts_dir)?;
        let semantic_artifact_digests = self.admin_semantic_artifacts()?;
        let native_invocation_ids = self.admin_native_invocations()?;
        let mut receipt = CapsuleReceipt {
            version: 1,
            capsule_root: destination.to_path_buf(),
            source_store_id: ownership.store_id,
            source_revision,
            source_schema: source_schema as u32,
            database_sha256,
            artifacts,
            semantic_artifact_digests,
            native_invocation_ids,
            capsule_sha256: String::new(),
        };
        receipt.capsule_sha256 = capsule_digest(&receipt)?;
        let manifest = serde_json::to_vec(&receipt).map_err(|_| StoreError::Unavailable)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination.join("manifest.json"))
            .map_err(|_| StoreError::Unavailable)?;
        file.write_all(&manifest)
            .map_err(|_| StoreError::Unavailable)?;
        file.sync_all().map_err(|_| StoreError::Unavailable)?;
        sync_dir(destination)?;
        Ok(receipt)
    }

    pub fn read_capsule(capsule_root: &Path) -> Result<CapsuleReceipt, StoreError> {
        if !fs::symlink_metadata(capsule_root)
            .map_err(|_| StoreError::Unavailable)?
            .file_type()
            .is_dir()
        {
            return Err(StoreError::Rejected);
        }
        let manifest_path = capsule_root.join("manifest.json");
        if !fs::symlink_metadata(&manifest_path)
            .map_err(|_| StoreError::Unavailable)?
            .file_type()
            .is_file()
        {
            return Err(StoreError::Rejected);
        }
        let receipt: CapsuleReceipt =
            serde_json::from_slice(&fs::read(&manifest_path).map_err(|_| StoreError::Unavailable)?)
                .map_err(|_| StoreError::Rejected)?;
        if receipt.version != 1
            || !supported_admin_schema(i64::from(receipt.source_schema))
            || receipt.capsule_root != capsule_root
            || receipt.capsule_sha256 != capsule_digest(&receipt)?
        {
            return Err(StoreError::Rejected);
        }
        let db_path = capsule_root.join("lifecycle.sqlite");
        if !fs::symlink_metadata(&db_path)
            .map_err(|_| StoreError::Unavailable)?
            .file_type()
            .is_file()
            || digest(&fs::read(&db_path).map_err(|_| StoreError::Unavailable)?)?
                != receipt.database_sha256
        {
            return Err(StoreError::Rejected);
        }
        let inspection = rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| StoreError::Unavailable)?;
        crate::recovery::check_coherence(&inspection)?;
        let version: i64 = inspection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|_| StoreError::Unavailable)?;
        if !supported_admin_schema(version) || version != i64::from(receipt.source_schema) {
            return Err(StoreError::Rejected);
        }
        let store_id: String = inspection
            .query_row(
                "SELECT store_id FROM store_identity WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        if store_id != receipt.source_store_id {
            return Err(StoreError::Rejected);
        }
        let snapshot_source_revision: i64 = inspection
            .query_row(
                "SELECT revision FROM admin_custody_revision WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        if u64::try_from(snapshot_source_revision).map_err(|_| StoreError::Unavailable)?
            != receipt.source_revision
        {
            return Err(StoreError::Rejected);
        }
        let mut statement = inspection
            .prepare(
                "SELECT digest_hex,relative_path,byte_length FROM artifacts ORDER BY digest_hex",
            )
            .map_err(|_| StoreError::Unavailable)?;
        let records = statement
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|_| StoreError::Unavailable)?;
        let mut inspected = Vec::new();
        for record in records {
            let (sha, path, length) = record.map_err(|_| StoreError::Unavailable)?;
            if length < 0 || path != format!("artifacts/blobs/{sha}") {
                return Err(StoreError::Rejected);
            }
            let blob = capsule_root.join(&path);
            if !fs::symlink_metadata(&blob)
                .map_err(|_| StoreError::Unavailable)?
                .file_type()
                .is_file()
                || digest(&fs::read(&blob).map_err(|_| StoreError::Unavailable)?)? != sha
            {
                return Err(StoreError::Rejected);
            }
            inspected.push(CapsuleArtifact {
                relative_path: path,
                digest_hex: sha,
                byte_length: length as u64,
                codec: "binary-artifact-v1".into(),
            });
        }
        if inspected != receipt.artifacts {
            return Err(StoreError::Rejected);
        }
        // Every semantic reference is a committed artifact under the same
        // database foreign keys; the manifest must also name its full closure.
        let semantic = semantic_artifacts_from(&inspection)?;
        let native = native_invocations_from(&inspection)?;
        if semantic != receipt.semantic_artifact_digests || native != receipt.native_invocation_ids
        {
            return Err(StoreError::Rejected);
        }
        Ok(receipt)
    }

    fn admin_source_revision(&self) -> Result<u64, StoreError> {
        let revision: i64 = self
            .connection
            .query_row(
                "SELECT revision FROM admin_custody_revision WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        u64::try_from(revision).map_err(|_| StoreError::Unavailable)
    }

    fn admin_semantic_artifacts(&self) -> Result<Vec<String>, StoreError> {
        semantic_artifacts_from(&self.connection)
    }

    fn admin_native_invocations(&self) -> Result<Vec<String>, StoreError> {
        native_invocations_from(&self.connection)
    }

    pub fn adopt_inactive(
        capsule: &CapsuleReceipt,
        destination: &Path,
        handoff: &HandoffId,
        trusted_issuer: String,
    ) -> Result<(SqliteLifecycleStore, AdoptionReceipt), StoreError> {
        // A configuration rejection says nothing about an existing copy.
        if trusted_issuer.is_empty() {
            return Err(StoreError::Rejected);
        }
        let checked = Self::read_capsule(&capsule.capsule_root)?;
        if &checked != capsule {
            return Err(StoreError::ClaimConflict);
        }
        let parent = destination.parent().ok_or(StoreError::Rejected)?;
        if !fs::symlink_metadata(parent)
            .map_err(|_| StoreError::Unavailable)?
            .file_type()
            .is_dir()
        {
            return Err(StoreError::Rejected);
        }
        let intent = AdoptionIntent {
            capsule_sha256: capsule.capsule_sha256.clone(),
            source_store_id: capsule.source_store_id.clone(),
            source_revision: capsule.source_revision,
            handoff_id: handoff.as_str().into(),
        };
        let intent_bytes = serde_json::to_vec(&intent).map_err(|_| StoreError::Unavailable)?;
        let intent_path = crate::sqlite::adoption_intent_path(destination)?;
        let mut copy_marker_name = destination
            .file_name()
            .ok_or(StoreError::Rejected)?
            .to_os_string();
        copy_marker_name.push(".adoption-copy-complete");
        let copy_marker = destination.with_file_name(copy_marker_name);
        // Hold the stable intent inode across copy, admission and cleanup. An
        // overlapping exact retry cannot mistake lock contention for a dead
        // adopter and remove a destination that is still being admitted.
        let _intent_lock = if intent_path.exists() {
            if !fs::symlink_metadata(&intent_path)
                .map_err(|_| StoreError::Unavailable)?
                .file_type()
                .is_file()
            {
                return Err(StoreError::Rejected);
            }
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&intent_path)
                .map_err(|_| StoreError::Unavailable)?;
            file.try_lock().map_err(|_| StoreError::Unavailable)?;
            file
        } else {
            if destination.exists() || copy_marker.exists() {
                return Err(StoreError::ClaimConflict);
            }
            // Publish only complete intent bytes. A crash during the temporary
            // write may leave an orphan, but cannot strand the destination
            // behind a truncated pending marker.
            let mut nonce = [0u8; 16];
            getrandom::fill(&mut nonce).map_err(|_| StoreError::Unavailable)?;
            let mut temporary_name = intent_path.as_os_str().to_os_string();
            temporary_name.push(format!(
                ".tmp.{}",
                nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()
            ));
            let temporary_path = PathBuf::from(temporary_name);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
                .map_err(|_| StoreError::ClaimConflict)?;
            file.write_all(&intent_bytes)
                .map_err(|_| StoreError::Unavailable)?;
            file.sync_all().map_err(|_| StoreError::Unavailable)?;
            file.try_lock().map_err(|_| StoreError::Unavailable)?;
            fs::hard_link(&temporary_path, &intent_path).map_err(|_| StoreError::ClaimConflict)?;
            sync_dir(parent)?;
            fs::remove_file(&temporary_path).map_err(|_| StoreError::Unavailable)?;
            sync_dir(parent)?;
            file
        };
        if fs::read(&intent_path).map_err(|_| StoreError::Unavailable)? != intent_bytes {
            return Err(StoreError::ClaimConflict);
        }
        if destination.exists() {
            match Self::open_adopted_inactive_pending(
                destination,
                handoff,
                &capsule.capsule_sha256,
                trusted_issuer.clone(),
            ) {
                Ok(result) => {
                    fs::remove_file(&intent_path).map_err(|_| StoreError::Unavailable)?;
                    sync_dir(parent)?;
                    if copy_marker.exists() {
                        fs::remove_file(&copy_marker).map_err(|_| StoreError::Unavailable)?;
                    }
                    sync_dir(parent)?;
                    return Ok(result);
                }
                Err(StoreError::ClaimConflict) => return Err(StoreError::ClaimConflict),
                Err(readback_error) => {
                    // A readback failure is never proof that the destination
                    // is disposable. Hold both stable locks, then require a
                    // positive pre-admission phase or an exact unadopted copy.
                    let writer_lock = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(false)
                        .open(destination.join("writer.lock"))
                        .map_err(|_| StoreError::Unavailable)?;
                    writer_lock
                        .try_lock()
                        .map_err(|_| StoreError::Unavailable)?;
                    match fs::symlink_metadata(&copy_marker) {
                        Ok(metadata) if metadata.file_type().is_file() => {
                            if !confirmed_unadopted_copy(
                                destination,
                                &capsule.source_store_id,
                                handoff,
                            )? {
                                return Err(readback_error);
                            }
                        }
                        Ok(_) => return Err(StoreError::Rejected),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            // This marker is synced before admission, and a
                            // successful admission durably removes the intent
                            // before removing the marker. An exact locked
                            // intent without a marker is pre-admission.
                        }
                        Err(_) => return Err(StoreError::Unavailable),
                    }
                    fs::remove_dir_all(destination).map_err(|_| StoreError::Unavailable)?;
                    if copy_marker.exists() {
                        fs::remove_file(&copy_marker).map_err(|_| StoreError::Unavailable)?;
                    }
                    sync_dir(parent)?;
                }
            }
        } else if copy_marker.exists() {
            // A prior retry may have removed its partial destination before
            // crashing. The intent lock proves no other adopter owns it now.
            fs::remove_file(&copy_marker).map_err(|_| StoreError::Unavailable)?;
            sync_dir(parent)?;
        }
        fs::create_dir(destination).map_err(|_| StoreError::ClaimConflict)?;
        sync_dir(parent)?;
        let source_db = capsule.capsule_root.join("lifecycle.sqlite");
        durable_copy(&source_db, &destination.join("lifecycle.sqlite"))?;
        let artifacts_dir = destination.join("artifacts");
        let blobs_dir = artifacts_dir.join("blobs");
        fs::create_dir(&artifacts_dir).map_err(|_| StoreError::Unavailable)?;
        fs::create_dir(&blobs_dir).map_err(|_| StoreError::Unavailable)?;
        for artifact in &capsule.artifacts {
            if artifact.relative_path != format!("artifacts/blobs/{}", artifact.digest_hex) {
                return Err(StoreError::Rejected);
            }
            durable_copy(
                &capsule.capsule_root.join(&artifact.relative_path),
                &destination.join(&artifact.relative_path),
            )?;
        }
        sync_dir(&blobs_dir)?;
        sync_dir(&artifacts_dir)?;
        sync_dir(destination)?;
        #[cfg(debug_assertions)]
        if std::env::var("LIFECYCLE_ADMIN_TEST_CUT").as_deref() == Ok("adoption_before_copy_marker")
        {
            let barrier =
                std::env::var("LIFECYCLE_ADMIN_TEST_BARRIER").map_err(|_| StoreError::Rejected)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&barrier)
                .map_err(|_| StoreError::Unavailable)?;
            file.write_all(b"adoption_before_copy_marker")
                .map_err(|_| StoreError::Unavailable)?;
            file.sync_all().map_err(|_| StoreError::Unavailable)?;
            let barrier_parent = Path::new(&barrier).parent().ok_or(StoreError::Rejected)?;
            sync_dir(barrier_parent)?;
            std::process::exit(87);
        }
        // The marker is published only after every copied byte is durable.
        // Without it, a failed open can only refer to an unfinished copy;
        // with it, I/O unavailability must preserve the destination.
        let marker_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&copy_marker)
            .map_err(|_| StoreError::Unavailable)?;
        marker_file
            .sync_all()
            .map_err(|_| StoreError::Unavailable)?;
        sync_dir(parent)?;
        #[cfg(debug_assertions)]
        if std::env::var("LIFECYCLE_ADMIN_TEST_CUT").as_deref() == Ok("adoption_after_db_copy") {
            let barrier =
                std::env::var("LIFECYCLE_ADMIN_TEST_BARRIER").map_err(|_| StoreError::Rejected)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&barrier)
                .map_err(|_| StoreError::Unavailable)?;
            file.write_all(b"adoption_after_db_copy")
                .map_err(|_| StoreError::Unavailable)?;
            file.sync_all().map_err(|_| StoreError::Unavailable)?;
            let barrier_parent = Path::new(&barrier).parent().ok_or(StoreError::Rejected)?;
            sync_dir(barrier_parent)?;
            std::process::exit(86);
        }
        let mut store = SqliteLifecycleStore::open_adoption_pending(destination, trusted_issuer)?;
        if store.admin_store_id()? != capsule.source_store_id {
            return Err(StoreError::Rejected);
        }
        let copied_ownership = store.admin_ownership()?;
        if copied_ownership.status != "active" {
            return Err(StoreError::ClaimConflict);
        }
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| StoreError::Unavailable)?;
        let destination_store_id = format!(
            "store:{}",
            random
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let tx = store
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        tx.execute(
            "UPDATE store_identity SET store_id=?1 WHERE singleton=1",
            [&destination_store_id],
        )
        .map_err(|_| StoreError::Unavailable)?;
        // Imported grants remain historical references. A new candidate must
        // receive a fresh trusted grant after destination ownership is current.
        tx.execute("UPDATE grants SET revoked=1 WHERE revoked=0", [])
            .map_err(|_| StoreError::Unavailable)?;
        let changed=tx.execute("UPDATE admin_ownership SET status='inactive',revision=1,handoff_id=?1,capsule_sha256=?2,peer_store_id=?3,selection_revision=NULL WHERE singleton=1 AND status='active' AND revision=?4",
            params![handoff.as_str(),&capsule.capsule_sha256,&capsule.source_store_id,to_i64(copied_ownership.revision)?])
            .map_err(|_| StoreError::Unavailable)?;
        if changed != 1 {
            return Err(StoreError::ClaimConflict);
        }
        // The copied handoff rows remain the immutable custody history. This
        // singleton records the latest import that established this instance.
        tx.execute("INSERT INTO admin_store_lineage(singleton,source_store_id,capsule_sha256,handoff_id,source_revision) VALUES (1,?1,?2,?3,?4) ON CONFLICT(singleton) DO UPDATE SET source_store_id=excluded.source_store_id,capsule_sha256=excluded.capsule_sha256,handoff_id=excluded.handoff_id,source_revision=excluded.source_revision",
            params![&capsule.source_store_id,&capsule.capsule_sha256,handoff.as_str(),to_i64(capsule.source_revision)?])
            .map_err(|_| StoreError::Unavailable)?;
        tx.execute("INSERT INTO admin_handoffs(handoff_id,capsule_sha256,source_store_id,destination_store_id,status,revision,adoption_revision,source_revision) VALUES (?1,?2,?3,?4,'accepted_inactive',1,1,?5)",
            params![handoff.as_str(),&capsule.capsule_sha256,&capsule.source_store_id,&destination_store_id,to_i64(capsule.source_revision)?])
            .map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        store.verify_identity()?;
        #[cfg(debug_assertions)]
        if std::env::var("LIFECYCLE_ADMIN_TEST_CUT").as_deref()
            == Ok("adoption_after_inactive_commit")
        {
            let barrier =
                std::env::var("LIFECYCLE_ADMIN_TEST_BARRIER").map_err(|_| StoreError::Rejected)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&barrier)
                .map_err(|_| StoreError::Unavailable)?;
            file.write_all(b"adoption_after_inactive_commit")
                .map_err(|_| StoreError::Unavailable)?;
            file.sync_all().map_err(|_| StoreError::Unavailable)?;
            sync_dir(Path::new(&barrier).parent().ok_or(StoreError::Rejected)?)?;
            std::process::exit(88);
        }
        fs::remove_file(&intent_path).map_err(|_| StoreError::Unavailable)?;
        sync_dir(parent)?;
        fs::remove_file(&copy_marker).map_err(|_| StoreError::Unavailable)?;
        sync_dir(parent)?;
        let destination_root = store.root().to_path_buf();
        Ok((
            store,
            AdoptionReceipt {
                handoff_id: handoff.as_str().into(),
                capsule_sha256: capsule.capsule_sha256.clone(),
                source_store_id: capsule.source_store_id.clone(),
                source_revision: capsule.source_revision,
                destination_store_id,
                destination_root,
                destination_revision: 1,
                status: "accepted_inactive".into(),
            },
        ))
    }

    pub fn open_adopted_inactive(
        destination: &Path,
        handoff: &HandoffId,
        capsule_sha256: &str,
        trusted_issuer: String,
    ) -> Result<(SqliteLifecycleStore, AdoptionReceipt), StoreError> {
        Self::open_adopted_inactive_inner(
            destination,
            handoff,
            capsule_sha256,
            trusted_issuer,
            false,
        )
    }

    fn open_adopted_inactive_pending(
        destination: &Path,
        handoff: &HandoffId,
        capsule_sha256: &str,
        trusted_issuer: String,
    ) -> Result<(SqliteLifecycleStore, AdoptionReceipt), StoreError> {
        Self::open_adopted_inactive_inner(
            destination,
            handoff,
            capsule_sha256,
            trusted_issuer,
            true,
        )
    }

    fn open_adopted_inactive_inner(
        destination: &Path,
        handoff: &HandoffId,
        capsule_sha256: &str,
        trusted_issuer: String,
        adoption_pending: bool,
    ) -> Result<(SqliteLifecycleStore, AdoptionReceipt), StoreError> {
        if !destination.exists() {
            return Err(StoreError::Rejected);
        }
        let store = if adoption_pending {
            SqliteLifecycleStore::open_adoption_pending(destination, trusted_issuer)?
        } else {
            SqliteLifecycleStore::open(destination, trusted_issuer)?
        };
        #[cfg(debug_assertions)]
        if adoption_pending
            && std::env::var("LIFECYCLE_ADMIN_TEST_CUT").as_deref()
                == Ok("adoption_readback_unavailable")
        {
            return Err(StoreError::Unavailable);
        }
        let row:(String,String,String,i64,i64)=store.connection.query_row(
            "SELECT capsule_sha256,source_store_id,destination_store_id,adoption_revision,source_revision FROM admin_handoffs WHERE handoff_id=?1 AND status IN ('accepted_inactive','destination_owned')",
            [handoff.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
        ).map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => StoreError::Rejected,
            _ => StoreError::Unavailable,
        })?;
        if row.0 != capsule_sha256 || row.2 != store.admin_store_id()? {
            return Err(StoreError::ClaimConflict);
        }
        let receipt = AdoptionReceipt {
            handoff_id: handoff.as_str().into(),
            capsule_sha256: row.0,
            source_store_id: row.1,
            source_revision: u64::try_from(row.4).map_err(|_| StoreError::Unavailable)?,
            destination_store_id: row.2,
            destination_root: store.root().to_path_buf(),
            destination_revision: u64::try_from(row.3).map_err(|_| StoreError::Unavailable)?,
            status: "accepted_inactive".into(),
        };
        Ok((store, receipt))
    }

    pub fn fence_source(
        &mut self,
        handoff: &HandoffId,
        adoption: &AdoptionReceipt,
    ) -> Result<FenceReceipt, StoreError> {
        self.verify_identity()?;
        let source_store_id = self.admin_store_id()?;
        if adoption.handoff_id != handoff.as_str()
            || adoption.source_store_id != source_store_id
            || adoption.status != "accepted_inactive"
            || adoption.destination_revision != 1
        {
            return Err(StoreError::Rejected);
        }
        let ownership = self.admin_ownership()?;
        let destination = inspect_peer(&adoption.destination_root)?;
        if destination.store_id != adoption.destination_store_id
            || destination.handoff_id.as_deref() != Some(handoff.as_str())
            || destination.capsule_sha256.as_deref() != Some(&adoption.capsule_sha256)
            || destination.peer_store_id.as_deref() != Some(&source_store_id)
        {
            return Err(StoreError::ClaimConflict);
        }
        if ownership.status == "fenced" {
            if ownership.handoff_id.as_deref() != Some(handoff.as_str())
                || ownership.capsule_sha256.as_deref() != Some(&adoption.capsule_sha256)
                || ownership.peer_store_id.as_deref() != Some(&adoption.destination_store_id)
                || (destination.status != "inactive" && destination.status != "active")
            {
                return Err(StoreError::ClaimConflict);
            }
            return Ok(FenceReceipt {
                handoff_id: handoff.as_str().into(),
                source_store_id,
                source_root: self.root().to_path_buf(),
                destination_store_id: adoption.destination_store_id.clone(),
                capsule_sha256: adoption.capsule_sha256.clone(),
                source_revision: adoption.source_revision,
            });
        }
        if ownership.status != "active"
            || destination.status != "inactive"
            || self.admin_source_revision()? != adoption.source_revision
        {
            return Err(StoreError::ClaimConflict);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let changed=tx.execute(
            "UPDATE admin_ownership SET status='fenced',revision=revision+1,handoff_id=?1,capsule_sha256=?2,peer_store_id=?3 WHERE singleton=1 AND status='active' AND revision=?4",
            params![handoff.as_str(),&adoption.capsule_sha256,&adoption.destination_store_id,to_i64(ownership.revision)?],
        ).map_err(|_| StoreError::Unavailable)?;
        if changed != 1 {
            return Err(StoreError::ClaimConflict);
        }
        tx.execute(
            "INSERT INTO admin_handoffs(handoff_id,capsule_sha256,source_store_id,destination_store_id,status,revision,adoption_revision,source_revision) VALUES (?1,?2,?3,?4,'source_fenced',1,?5,?6)",
            params![handoff.as_str(),&adoption.capsule_sha256,&source_store_id,&adoption.destination_store_id,
                to_i64(adoption.destination_revision)?,to_i64(adoption.source_revision)?],
        ).map_err(|_| StoreError::ClaimConflict)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(FenceReceipt {
            handoff_id: handoff.as_str().into(),
            source_store_id,
            source_root: self.root().to_path_buf(),
            destination_store_id: adoption.destination_store_id.clone(),
            capsule_sha256: adoption.capsule_sha256.clone(),
            source_revision: adoption.source_revision,
        })
    }

    pub fn commit_destination(
        &mut self,
        handoff: &HandoffId,
        fence: &FenceReceipt,
        selection: &SelectionEvidence,
    ) -> Result<OwnershipReceipt, StoreError> {
        self.verify_identity()?;
        let destination_store_id = self.admin_store_id()?;
        if fence.handoff_id != handoff.as_str()
            || selection.handoff_id != handoff.as_str()
            || fence.destination_store_id != destination_store_id
            || selection.destination_store_id != destination_store_id
            || fence.source_store_id != selection.source_store_id
            || fence.capsule_sha256 != selection.capsule_sha256
            || selection.current_selection_revision
                != selection
                    .prior_selection_revision
                    .checked_add(1)
                    .ok_or(StoreError::Rejected)?
            || selection.launch_guard_id.is_empty()
            || !hex_digest(&selection.current_selection_digest)
        {
            return Err(StoreError::Rejected);
        }
        let source = inspect_peer(&fence.source_root)?;
        if source.store_id != fence.source_store_id
            || source.status != "fenced"
            || source.handoff_id.as_deref() != Some(handoff.as_str())
            || source.capsule_sha256.as_deref() != Some(&fence.capsule_sha256)
            || source.peer_store_id.as_deref() != Some(&destination_store_id)
        {
            return Err(StoreError::ClaimConflict);
        }
        let ownership = self.admin_ownership()?;
        let row:HandoffCommitRow=self.connection.query_row(
            "SELECT capsule_sha256,source_store_id,destination_store_id,status,revision,selection_revision,selection_digest,launch_guard_id FROM admin_handoffs WHERE handoff_id=?1",
            [handoff.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)),
        ).map_err(|_| StoreError::Rejected)?;
        if row.0 != fence.capsule_sha256
            || row.1 != fence.source_store_id
            || row.2 != destination_store_id
        {
            return Err(StoreError::ClaimConflict);
        }
        if row.3 == "destination_owned" {
            if ownership.status != "active"
                || row.5 != Some(to_i64(selection.current_selection_revision)?)
                || row.6.as_deref() != Some(&selection.current_selection_digest)
                || row.7.as_deref() != Some(&selection.launch_guard_id)
            {
                return Err(StoreError::ClaimConflict);
            }
            return Ok(OwnershipReceipt {
                handoff_id: handoff.as_str().into(),
                destination_store_id,
                capsule_sha256: fence.capsule_sha256.clone(),
                ownership_revision: ownership.revision,
                selection_revision: selection.current_selection_revision,
            });
        }
        if row.3 != "accepted_inactive"
            || ownership.status != "inactive"
            || ownership.handoff_id.as_deref() != Some(handoff.as_str())
            || ownership.capsule_sha256.as_deref() != Some(&fence.capsule_sha256)
            || ownership.peer_store_id.as_deref() != Some(&fence.source_store_id)
        {
            return Err(StoreError::ClaimConflict);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let changed=tx.execute(
            "UPDATE admin_ownership SET status='active',revision=revision+1,selection_revision=?1 WHERE singleton=1 AND status='inactive' AND revision=?2 AND handoff_id=?3 AND capsule_sha256=?4",
            params![to_i64(selection.current_selection_revision)?,to_i64(ownership.revision)?,handoff.as_str(),&fence.capsule_sha256],
        ).map_err(|_| StoreError::Unavailable)?;
        if changed != 1 {
            return Err(StoreError::ClaimConflict);
        }
        tx.execute(
            "UPDATE admin_handoffs SET status='destination_owned',revision=revision+1,selection_revision=?1,selection_digest=?2,launch_guard_id=?3 WHERE handoff_id=?4 AND status='accepted_inactive'",
            params![to_i64(selection.current_selection_revision)?,&selection.current_selection_digest,&selection.launch_guard_id,handoff.as_str()],
        ).map_err(|_| StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(OwnershipReceipt {
            handoff_id: handoff.as_str().into(),
            destination_store_id,
            capsule_sha256: fence.capsule_sha256.clone(),
            ownership_revision: ownership.revision + 1,
            selection_revision: selection.current_selection_revision,
        })
    }

    pub fn read_handoff(
        &self,
        handoff: &HandoffId,
    ) -> Result<Option<HandoffProjection>, StoreError> {
        self.verify_identity()?;
        self.connection.query_row(
            "SELECT capsule_sha256,source_store_id,destination_store_id,status,revision FROM admin_handoffs WHERE handoff_id=?1",
            [handoff.as_str()],|r|Ok(HandoffProjection{
                handoff_id:handoff.as_str().into(),capsule_sha256:r.get(0)?,
                source_store_id:r.get(1)?,destination_store_id:r.get(2)?,status:r.get(3)?,
                revision:r.get::<_,i64>(4)? as u64,
            }),
        ).optional().map_err(|_| StoreError::Unavailable)
    }
}

fn selector_claim_id(selector: &ExactAttemptSelector) -> Result<String, StoreError> {
    match (&selector.effect_id, &selector.invocation_id) {
        (Some(effect), None) if !effect.is_empty() => Ok(format!("effect:{effect}")),
        (None, Some(invocation)) if !invocation.is_empty() => Ok(format!("native:{invocation}")),
        _ => Err(StoreError::Rejected),
    }
}

fn digest(bytes: &[u8]) -> Result<String, StoreError> {
    Ok(CodecContract::BinaryArtifactV1
        .digest_binary(bytes)
        .map_err(|_| StoreError::Rejected)?
        .hex())
}

fn to_i64(value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::Rejected)
}

fn availability_name(value: &ObservationAvailability) -> &'static str {
    match value {
        ObservationAvailability::Complete => "complete",
        ObservationAvailability::Partial => "partial",
        ObservationAvailability::Unavailable => "unavailable",
        ObservationAvailability::Conflicted => "conflicted",
    }
}

fn coverage_name(value: &EffectCoverage) -> &'static str {
    match value {
        EffectCoverage::KnownNotEntered => "known_not_entered",
        EffectCoverage::AccountedFiniteRead => "accounted_finite_read",
        EffectCoverage::AccountedNativeTurn => "accounted_native_turn",
        EffectCoverage::Unknown => "unknown",
    }
}

fn capsule_digest(receipt: &CapsuleReceipt) -> Result<String, StoreError> {
    let mut unsigned = receipt.clone();
    unsigned.capsule_sha256.clear();
    digest(&serde_json::to_vec(&unsigned).map_err(|_| StoreError::Unavailable)?)
}

fn sync_dir(path: &Path) -> Result<(), StoreError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| StoreError::Unavailable)
}

fn durable_copy(source: &Path, destination: &Path) -> Result<(), StoreError> {
    if !fs::symlink_metadata(source)
        .map_err(|_| StoreError::Unavailable)?
        .file_type()
        .is_file()
    {
        return Err(StoreError::Rejected);
    }
    let bytes = fs::read(source).map_err(|_| StoreError::Unavailable)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| StoreError::ClaimConflict)?;
    file.write_all(&bytes)
        .map_err(|_| StoreError::Unavailable)?;
    file.sync_all().map_err(|_| StoreError::Unavailable)
}

#[derive(Debug)]
struct PeerOwnership {
    store_id: String,
    status: String,
    handoff_id: Option<String>,
    capsule_sha256: Option<String>,
    peer_store_id: Option<String>,
}

fn inspect_peer(root: &Path) -> Result<PeerOwnership, StoreError> {
    if !root.is_absolute()
        || !fs::symlink_metadata(root)
            .map_err(|_| StoreError::Unavailable)?
            .file_type()
            .is_dir()
    {
        return Err(StoreError::Rejected);
    }
    let db = root.join("lifecycle.sqlite");
    let metadata = fs::symlink_metadata(&db).map_err(|_| StoreError::Unavailable)?;
    if !metadata.file_type().is_file() {
        return Err(StoreError::Rejected);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(StoreError::Rejected);
        }
    }
    let connection = rusqlite::Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|_| StoreError::Unavailable)?;
    crate::recovery::check_coherence(&connection)?;
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|_| StoreError::Unavailable)?;
    if !supported_admin_schema(version) {
        return Err(StoreError::Rejected);
    }
    connection.query_row(
        "SELECT s.store_id,o.status,o.handoff_id,o.capsule_sha256,o.peer_store_id FROM store_identity s JOIN admin_ownership o ON s.singleton=o.singleton WHERE s.singleton=1",
        [],|r|Ok(PeerOwnership{store_id:r.get(0)?,status:r.get(1)?,handoff_id:r.get(2)?,
            capsule_sha256:r.get(3)?,peer_store_id:r.get(4)?}),
    ).map_err(|_|StoreError::Unavailable)
}

// Called only while the exact adoption intent and destination writer inode
// are both locked. A complete copied image can be discarded for retry only
// when durable owner state positively identifies the copied active source and
// no admission for this handoff. Any unreadable or inconsistent state stays
// in place for query/recovery rather than being inferred to be partial.
fn confirmed_unadopted_copy(
    destination: &Path,
    source_store_id: &str,
    handoff: &HandoffId,
) -> Result<bool, StoreError> {
    let peer = inspect_peer(destination).map_err(|_| StoreError::Unavailable)?;
    if peer.store_id != source_store_id || peer.status != "active" {
        return Ok(false);
    }
    let connection = rusqlite::Connection::open_with_flags(
        destination.join("lifecycle.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|_| StoreError::Unavailable)?;
    let admitted: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM admin_handoffs WHERE handoff_id=?1",
            [handoff.as_str()],
            |row| row.get(0),
        )
        .map_err(|_| StoreError::Unavailable)?;
    Ok(admitted == 0)
}

fn hex_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn semantic_artifacts_from(connection: &rusqlite::Connection) -> Result<Vec<String>, StoreError> {
    let mut statement=connection.prepare(
        "SELECT digest FROM (
          SELECT basis_artifact_sha256 AS digest FROM semantic_captures
          UNION SELECT result_artifact_sha256 AS digest FROM semantic_attempts WHERE result_artifact_sha256 IS NOT NULL
          UNION SELECT candidate_artifact_sha256 AS digest FROM semantic_results
          UNION SELECT candidate_record_artifact_sha256 AS digest FROM semantic_results WHERE candidate_record_artifact_sha256 IS NOT NULL
          UNION SELECT challenge_artifact_sha256 AS digest FROM semantic_results WHERE challenge_artifact_sha256 IS NOT NULL
          UNION SELECT successor_prompt_artifact_sha256 AS digest FROM semantic_results WHERE successor_prompt_artifact_sha256 IS NOT NULL
          UNION SELECT response_artifact_sha256 AS digest FROM semantic_results WHERE response_artifact_sha256 IS NOT NULL
          UNION SELECT findings_artifact_sha256 AS digest FROM semantic_results
         ) ORDER BY digest",
    ).map_err(|_| StoreError::Unavailable)?;
    statement
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| StoreError::Unavailable)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StoreError::Unavailable)
}

fn native_invocations_from(connection: &rusqlite::Connection) -> Result<Vec<String>, StoreError> {
    let mut statement = connection
        .prepare("SELECT invocation_id FROM native_turns ORDER BY invocation_id")
        .map_err(|_| StoreError::Unavailable)?;
    statement
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| StoreError::Unavailable)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StoreError::Unavailable)
}

pub(crate) fn admission_open(connection: &rusqlite::Connection) -> Result<bool, StoreError> {
    let status: String = connection
        .query_row(
            "SELECT status FROM admin_ownership WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(|_| StoreError::Unavailable)?;
    Ok(status == "active")
}
